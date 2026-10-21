use crate::core::*;
use crate::output::*;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::{
    fs,
    io::{Write, stdin, stdout},
    path::{Path, PathBuf},
};

pub fn discover_cmd(targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }
    println!("Discovered {} todo list(s):", targets.len());
    for (i, target) in targets.iter().enumerate() {
        let tasks = get_tasks(&target.path);
        let completed = tasks.iter().filter(|t| t.completed).count();
        let incomplete = tasks.len().saturating_sub(completed);
        let personal_tag = if target.is_personal { " [personal]" } else { "" };
        println!(
            "  [{}] {}{} ({} task{}, {} completed, {} incomplete)",
            i,
            target.display_name,
            personal_tag,
            tasks.len(),
            if tasks.len() == 1 { "" } else { "s" },
            completed,
            incomplete
        );
    }
}

pub fn new_task(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo file target available.");
        return;
    }

    let verbose = find_arg_and_remove(&mut args, "-v", "--verbose").is_some();
    let raw_tags = extract_flag_values(&mut args, "-t", "--tag");
    let mut tags = Vec::new();
    for t in raw_tags {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }

    let raw_files = extract_flag_values(&mut args, "-a", "--attach");
    let valid_files: Vec<PathBuf> = raw_files
        .iter()
        .filter_map(|p| {
            let file_path = Path::new(p);
            if !file_path.exists() {
                println!("Warning: File '{}' does not exist. Skipping.", p);
                None
            } else {
                Some(file_path.to_path_buf())
            }
        })
        .collect();

    // If description provided on command line (args.len() >= 3)
    if args.len() >= 3 {
        let text = args[2..].join(" ");
        for target in targets {
            let target_files: Vec<String> = valid_files
                .iter()
                .map(|f| get_relative_to_todo(f, &target.path))
                .collect();

            let mut tasks = get_tasks(&target.path);
            tasks.push(Task {
                text: text.clone(),
                completed: false,
                tags: tags.clone(),
                files: target_files,
            });
            let new_id = tasks.len() - 1;
            save_tasks(tasks, &target.path);

            if verbose {
                if targets.len() > 1 {
                    println!("[{}] New Task Id: {}", target.display_name, new_id);
                } else {
                    println!("New Task Id: {}", new_id);
                }
            }
        }
        return;
    }

    // Otherwise check for piped input
    let pipe = get_piped();
    if let Some(buffer) = pipe {
        let mut count = 0;
        let mut lines = Vec::new();
        for line in buffer.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
                count += 1;
            }
        }
        if count == 0 {
            println!(
                "No task description provided.\nTry '{} help new' for more details.",
                CMD_NAME
            );
            return;
        }

        for target in targets {
            let mut tasks = get_tasks(&target.path);
            for line in &lines {
                tasks.push(Task {
                    text: line.clone(),
                    completed: false,
                    tags: Vec::new(),
                    files: Vec::new(),
                });
            }
            save_tasks(tasks, &target.path);
        }

        if targets.len() > 1 {
            println!(
                "Added {} new task(s) from piped input across {} todo files.",
                count,
                targets.len()
            );
        } else {
            println!("Added {} new task(s) from piped input.", count);
        }
        return;
    }

    println!(
        "No task description provided.\nTry '{} help new' for more details.",
        CMD_NAME
    );
}

pub fn add_tags(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    let verbose = find_arg_and_remove(&mut args, "-v", "--verbose").is_some();
    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();

    if has_query {
        let query = config_query(&mut args);
        if args.len() < 3 {
            println!(
                "No tags provided to add.\nTry '{} help add' for more details.",
                CMD_NAME
            );
            return;
        }
        let mut tags_to_add = Vec::new();
        for tag in &args[2..] {
            if !tag.starts_with('-') && !tags_to_add.contains(tag) {
                tags_to_add.push(tag.clone());
            }
        }

        let mut total_affected = 0;
        for target in targets {
            let mut tasks = get_tasks(&target.path);
            let matching_indices: Vec<usize> =
                query_tasks(tasks.clone(), query.clone(), Vec::new())
                    .into_iter()
                    .map(|(idx, _)| idx)
                    .collect();

            if matching_indices.is_empty() {
                continue;
            }

            let mut target_added_count = 0;
            for idx in matching_indices {
                if let Some(task) = tasks.get_mut(idx) {
                    for tag in &tags_to_add {
                        if !task.tags.contains(tag) {
                            task.tags.push(tag.clone());
                            target_added_count += 1;
                        }
                    }
                    total_affected += 1;
                }
            }
            save_tasks(tasks, &target.path);
            if verbose {
                println!(
                    "[{}] Added {} unique tag(s) across matching task(s).",
                    target.display_name, target_added_count
                );
            }
        }
        println!("Added tag(s) to {} task(s).", total_affected);
        return;
    }

    // Operating by task index
    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help add' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = match args[2].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid task index: {}", args[2]);
            return;
        }
    };
    let path = &targets[0].path;
    let mut tasks = get_tasks(path);

    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    let mut count = 0;
    for i in 3..args.len() {
        if let Some(tag) = args.get(i) {
            if !tag.starts_with('-') && !task.tags.contains(tag) {
                task.tags.push(tag.to_string());
                count += 1;
            }
        }
    }

    save_tasks(tasks, path);
    if verbose {
        println!("Added {} unique tag(s) to task #{}.", count, task_number);
    }
}

pub fn remove_tags(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();
    if has_query {
        let query = config_query(&mut args);
        if args.len() < 3 {
            println!(
                "No tags provided to remove.\nTry '{} help remove' for more details.",
                CMD_NAME
            );
            return;
        }
        let tags_to_remove: Vec<String> = args[2..]
            .iter()
            .filter(|t| !t.starts_with('-'))
            .cloned()
            .collect();

        let mut total_affected = 0;
        for target in targets {
            let mut tasks = get_tasks(&target.path);
            let matching_indices: Vec<usize> =
                query_tasks(tasks.clone(), query.clone(), Vec::new())
                    .into_iter()
                    .map(|(idx, _)| idx)
                    .collect();

            if matching_indices.is_empty() {
                continue;
            }

            for idx in matching_indices {
                if let Some(task) = tasks.get_mut(idx) {
                    task.tags.retain(|t| !tags_to_remove.contains(t));
                    total_affected += 1;
                }
            }
            save_tasks(tasks, &target.path);
        }
        println!("Removed tag(s) from {} task(s).", total_affected);
        return;
    }

    // Operating by task index
    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help remove' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = match args[2].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid task index: {}", args[2]);
            return;
        }
    };
    let path = &targets[0].path;
    let mut tasks = get_tasks(path);

    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    let tags = &mut task.tags;
    for i in 3..args.len() {
        let tag = match args.get(i) {
            Some(val) => val,
            None => continue,
        };
        tags.retain(|t| t != tag);
    }

    save_tasks(tasks, path);
}

pub fn edit_task(args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot edit tasks across multiple todo files. Specify a single file with --path."
        );
        return;
    }

    let path = &targets[0].path;
    let pipe = get_piped();
    if args.len() < 3 && pipe.is_none() {
        println!(
            "No task number provided.\nTry '{} help edit' for more details.",
            CMD_NAME
        );
        return;
    }
    let overwrite = args.iter().any(|arg| arg == "--overwrite" || arg == "-o");
    let task_number: usize = args[2].parse().expect("Invalid task index.");
    let mut tasks = get_tasks(path);

    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    // rewrite from piped input
    if let Some(buffer) = pipe {
        let new_text = buffer.lines().next().unwrap_or_default().trim();
        if !new_text.is_empty() {
            task.text = new_text.to_string();
            save_tasks(tasks, path);
        } else {
            println!("Piped input is empty. Task not edited.");
        }
        return;
    }

    // overwrite from command line input
    if overwrite {
        if args.len() < 4 {
            println!(
                "No task description provided.\nTry '{} help edit' for more details.",
                CMD_NAME
            );
            return;
        }
        task.text = args[3..]
            .iter()
            .filter(|s| !s.starts_with('-'))
            .cloned()
            .collect::<Vec<String>>()
            .join(" ");
        save_tasks(tasks, path);
        return;
    }

    // get input
    let rewrite = args.iter().any(|arg| arg == "--rewrite" || arg == "-r");
    let mut rl = match DefaultEditor::new() {
        Err(e) => {
            eprintln!("Failed to load history: {}", e);
            return;
        }
        Ok(r) => r,
    };

    match rl.readline_with_initial("> ", (if rewrite { "" } else { task.text.as_str() }, "")) {
        Ok(line) => {
            if let Err(e) = rl.add_history_entry(line.as_str()) {
                eprintln!("Failed to add history entry: {}", e);
            }
            task.text = line;
            save_tasks(tasks, path);
        }
        Err(ReadlineError::Eof) => {
            println!("Input ended unexpectedly. Task not edited.");
        }
        Err(ReadlineError::Interrupted) => {
            println!("Input interrupted. Task not edited.");
        }
        Err(err) => {
            println!("Error reading input: {:?}", err);
        }
    };
}

pub fn attach_files(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    let verbose = find_arg_and_remove(&mut args, "-v", "--verbose").is_some();
    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();

    if has_query {
        let query = config_query(&mut args);
        let file_args: Vec<String> = args[2..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();
        if file_args.is_empty() {
            println!(
                "No files provided to attach.\nTry '{} help attach' for more details.",
                CMD_NAME
            );
            return;
        }

        let mut total_attached = 0;
        for target in targets {
            let mut tasks = get_tasks(&target.path);
            let matching_indices: Vec<usize> =
                query_tasks(tasks.clone(), query.clone(), Vec::new())
                    .into_iter()
                    .map(|(idx, _)| idx)
                    .collect();

            if matching_indices.is_empty() {
                continue;
            }

            let mut target_attached = 0;
            for idx in matching_indices {
                if let Some(task) = tasks.get_mut(idx) {
                    for path_str in &file_args {
                        let file_path = Path::new(path_str);
                        if !file_path.exists() {
                            println!("Warning: File '{}' does not exist. Skipping.", path_str);
                            continue;
                        }
                        let safe_path = get_relative_to_todo(file_path, &target.path);
                        if !task.files.contains(&safe_path) {
                            task.files.push(safe_path);
                            target_attached += 1;
                        }
                    }
                }
            }
            save_tasks(tasks, &target.path);
            total_attached += target_attached;
            if verbose {
                println!(
                    "[{}] Attached {} file(s) across matching task(s).",
                    target.display_name, target_attached
                );
            }
        }
        println!("Attached {} file(s) in total.", total_attached);
        return;
    }

    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if args.len() < 4 {
        println!(
            "No task number provided.\nTry '{} help attach' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = match args[2].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid task index: {}", args[2]);
            return;
        }
    };
    let path = &targets[0].path;
    let mut tasks = get_tasks(path);
    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    let mut count = 0;
    for path_str in &args[3..] {
        let file_path = Path::new(path_str);
        if !file_path.exists() {
            println!("Warning: File '{}' does not exist. Skipping.", path_str);
            continue;
        }

        let safe_path = get_relative_to_todo(file_path, path);
        if !task.files.contains(&safe_path) {
            task.files.push(safe_path);
            count += 1;
        }
    }
    let len = task.files.len();
    save_tasks(tasks, path);
    if verbose {
        println!(
            "Attached {} file(s) to task #{} ({} total files attached).",
            count, task_number, len
        );
    }
}

pub fn detach_files(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    let verbose = find_arg_and_remove(&mut args, "-v", "--verbose").is_some();
    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();

    if has_query {
        let query = config_query(&mut args);
        let file_args: Vec<String> = args[2..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();
        if file_args.is_empty() {
            println!(
                "No files provided to detach.\nTry '{} help detach' for more details.",
                CMD_NAME
            );
            return;
        }

        let mut total_detached = 0;
        for target in targets {
            let mut tasks = get_tasks(&target.path);
            let matching_indices: Vec<usize> =
                query_tasks(tasks.clone(), query.clone(), Vec::new())
                    .into_iter()
                    .map(|(idx, _)| idx)
                    .collect();

            if matching_indices.is_empty() {
                continue;
            }

            let mut target_detached = 0;
            for idx in matching_indices {
                if let Some(task) = tasks.get_mut(idx) {
                    for path_str in &file_args {
                        let file_path = Path::new(path_str);
                        let safe_path = get_relative_to_todo(file_path, &target.path);
                        if let Some(pos) = task.files.iter().position(|f| f == &safe_path) {
                            task.files.remove(pos);
                            target_detached += 1;
                        }
                    }
                }
            }
            save_tasks(tasks, &target.path);
            total_detached += target_detached;
            if verbose {
                println!(
                    "[{}] Detached {} file(s) across matching task(s).",
                    target.display_name, target_detached
                );
            }
        }
        println!("Detached {} file(s) in total.", total_detached);
        return;
    }

    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if args.len() < 4 {
        println!(
            "No task number provided.\nTry '{} help detach' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = match args[2].parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid task index: {}", args[2]);
            return;
        }
    };
    let path = &targets[0].path;
    let mut tasks = get_tasks(path);
    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    let mut count = 0;
    for path_str in &args[3..] {
        let file_path = Path::new(path_str);
        if !file_path.exists() {
            println!("Warning: File '{}' does not exist. Skipping.", path_str);
            continue;
        }

        let safe_path = get_relative_to_todo(file_path, path);
        if let Some(i) = task.files.iter().position(|f| f == &safe_path) {
            task.files.remove(i);
            count += 1;
        }
    }

    let len = task.files.len();
    save_tasks(tasks, path);
    if verbose {
        println!(
            "Detached {} file(s) to task #{} ({} total files attached).",
            count, task_number, len
        );
    }
}

pub fn complete_task(mut args: Vec<String>, targets: &[TodoTarget], mark: bool) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help complete' for more details.",
            CMD_NAME
        );
        return;
    }
    args.remove(0); // remove command name
    args.remove(0); // remove subcommand name

    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();
    let query = if has_query {
        config_query(&mut args)
    } else {
        QueryConfig::DEFAULT
    };

    let loose_indices: Vec<usize> = args
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|arg| match arg.parse::<usize>() {
            Ok(num) => Some(num),
            Err(e) => {
                println!("Arg '{}' could not be parsed into task number: {}", arg, e);
                None
            }
        })
        .collect();

    if targets.len() > 1 && !loose_indices.is_empty() {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if targets.len() == 1 {
        let path = &targets[0].path;
        let mut tasks = get_tasks(path);
        let mut to_mark: Vec<usize> = Vec::new();

        if has_query {
            let queried = query_tasks(tasks.clone(), query, Vec::new());
            to_mark.extend(queried.iter().map(|(idx, _)| *idx));
            println!("Appending {} queried tasks to mark.", queried.len());
        }
        to_mark.extend(loose_indices);

        let mut count = 0;
        for task_number in to_mark {
            if let Some(task) = tasks.get_mut(task_number) {
                task.completed = mark;
                count += 1;
            } else {
                println!("Task #{} not found.", task_number);
            }
        }
        println!(
            "Marked {} task(s) {}.",
            count,
            if mark { "complete" } else { "incomplete" }
        );
        save_tasks(tasks, path);
        return;
    }

    // Multiple targets with query
    if !has_query || query.is_empty() {
        println!(
            "No query provided across multiple todo files.\nTry '{} help complete' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut total_count = 0;
    for target in targets {
        let mut tasks = get_tasks(&target.path);
        let queried = query_tasks(tasks.clone(), query.clone(), Vec::new());
        if queried.is_empty() {
            continue;
        }

        let mut count = 0;
        for (idx, _) in queried {
            if let Some(task) = tasks.get_mut(idx) {
                task.completed = mark;
                count += 1;
            }
        }
        save_tasks(tasks, &target.path);
        println!(
            "[{}] Marked {} task(s) {}.",
            target.display_name,
            count,
            if mark { "complete" } else { "incomplete" }
        );
        total_count += count;
    }
    println!(
        "Marked {} task(s) {} across {} todo file(s).",
        total_count,
        if mark { "complete" } else { "incomplete" },
        targets.len()
    );
}

pub fn delete_task(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }

    if args.len() < 3 {
        println!(
            "No task number or query provided.\nTry '{} help delete' for more details.",
            CMD_NAME
        );
        return;
    }
    args.remove(0); // remove command name
    args.remove(0); // remove subcommand name

    let has_query = find_arg_and_remove(&mut args, "-q", "--query").is_some();
    let query = if has_query {
        config_query(&mut args)
    } else {
        QueryConfig::DEFAULT
    };

    let loose_indices: Vec<usize> = args
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|arg| match arg.parse::<usize>() {
            Ok(num) => Some(num),
            Err(e) => {
                println!("Arg '{}' could not be parsed into task number: {}", arg, e);
                None
            }
        })
        .collect();

    if targets.len() > 1 && !loose_indices.is_empty() {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    if targets.len() == 1 {
        let path = &targets[0].path;
        let mut tasks = get_tasks(path);
        let mut to_delete: Vec<usize> = Vec::new();

        if has_query {
            let queried = query_tasks(tasks.clone(), query, Vec::new());
            let mut results = queried.iter().map(|(idx, _)| *idx).collect::<Vec<usize>>();

            if get_piped().is_none() {
                loop {
                    print!(
                        "Are you sure you want to delete {} queried tasks? (y/N/i to inspect): ",
                        queried.len()
                    );
                    Write::flush(&mut stdout()).expect("Error flushing stdout.");

                    let mut input = String::new();
                    stdin().read_line(&mut input).expect("Error reading input.");

                    if input.trim().eq_ignore_ascii_case("y") {
                        to_delete.append(&mut results);
                        break;
                    } else if input.trim().eq_ignore_ascii_case("i") {
                        println!();
                        for (original_index, task) in &queried {
                            println!(
                                "{}",
                                format_task(*original_index, task, &DisplayConfig::DEFAULT)
                            );
                        }
                        println!();
                    } else {
                        println!("Deletion cancelled.");
                        return;
                    }
                }
            } else {
                to_delete.append(&mut results);
                println!("Appending {} queried tasks to delete.", queried.len());
            }
        }

        to_delete.extend(loose_indices);
        to_delete.sort_by(|a, b| b.cmp(a));
        to_delete.dedup();

        let mut count = 0;
        for task_number in to_delete {
            if task_number < tasks.len() {
                tasks.remove(task_number);
                count += 1;
            } else {
                println!("Task #{} not found.", task_number);
            }
        }
        save_tasks(tasks, path);
        println!("Deleted {} task(s).", count);
        return;
    }

    // Multiple targets with query
    if !has_query || query.is_empty() {
        println!(
            "No query provided across multiple todo files.\nTry '{} help delete' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut target_matches: Vec<(&TodoTarget, Vec<Task>, Vec<usize>)> = Vec::new();
    let mut total_matches = 0;

    for target in targets {
        let tasks = get_tasks(&target.path);
        let queried = query_tasks(tasks.clone(), query.clone(), Vec::new());
        if !queried.is_empty() {
            total_matches += queried.len();
            let mut indices: Vec<usize> = queried.iter().map(|(idx, _)| *idx).collect();
            indices.sort_by(|a, b| b.cmp(a));
            target_matches.push((target, tasks, indices));
        }
    }

    if total_matches == 0 {
        println!("No matching tasks found across todo files.");
        return;
    }

    if get_piped().is_none() {
        loop {
            print!(
                "Are you sure you want to delete {} queried task(s) across {} todo file(s)? (y/N/i to inspect): ",
                total_matches,
                target_matches.len()
            );
            Write::flush(&mut stdout()).expect("Error flushing stdout.");

            let mut input = String::new();
            stdin().read_line(&mut input).expect("Error reading input.");

            if input.trim().eq_ignore_ascii_case("y") {
                break;
            } else if input.trim().eq_ignore_ascii_case("i") {
                println!();
                for (target, tasks, indices) in &target_matches {
                    println!("=== [{}] ({}) ===", target.display_name, target.path.display());
                    for idx in indices.iter().rev() {
                        if let Some(task) = tasks.get(*idx) {
                            println!("{}", format_task(*idx, task, &DisplayConfig::DEFAULT));
                        }
                    }
                    println!();
                }
            } else {
                println!("Deletion cancelled.");
                return;
            }
        }
    }

    for (target, mut tasks, indices) in target_matches {
        for idx in indices {
            if idx < tasks.len() {
                tasks.remove(idx);
            }
        }
        save_tasks(tasks, &target.path);
    }
    println!(
        "Deleted {} task(s) across todo file(s).",
        total_matches
    );
}

pub fn clean_task(targets: &[TodoTarget]) {
    if targets.len() > 1 {
        eprintln!(
            "Error: Cannot clean multiple todo files at once to prevent accidental mass deletion."
        );
        return;
    }
    if targets.is_empty() {
        println!("No todo file to clean.");
        return;
    }

    let file_path = path_to_file(&targets[0].path);
    match fs::remove_file(file_path) {
        Ok(()) => println!("File deleted successfully."),
        Err(err) => println!("Failed to delete file, error: {}", err),
    };
}
