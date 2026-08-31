use crate::core::*;
use crate::output::*;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::{
    fs,
    io::{Write, stdin, stdout},
    path::Path,
};

pub fn new_task(args: Vec<String>, path: &Path) {
    let pipe = get_piped();
    if args.len() < 3 && pipe.is_none() {
        println!(
            "No task description provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut count = 0;
    let mut tasks = get_tasks(path);
    if let Some(buffer) = pipe {
        for line in buffer.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            tasks.push(Task {
                text: trimmed.to_string(),
                completed: false,
                tags: Vec::new(),
                files: Vec::new(),
            });
            count += 1;
        }
    } else {
        tasks.push(Task {
            text: args[2..].join(" "),
            completed: false,
            tags: Vec::new(),
            files: Vec::new(),
        });
    }

    save_tasks(tasks, path);
    if count > 0 {
        println!("Added {} new task(s) from piped input.", count);
    }
}

pub fn add_task(args: Vec<String>, path: &Path) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = args[2].parse().expect("Invalid task index.");
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
        match args.get(i) {
            Some(tag) => tags.push(tag.to_string()),
            None => (),
        }
    }

    save_tasks(tasks, path);
}

pub fn edit_task(args: Vec<String>, path: &Path) {
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

pub fn remove_task(args: Vec<String>, path: &Path) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = args[2].parse().expect("Invalid task index.");
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

        for i in 0..tags.len() {
            match tags.get(i) {
                Some(val) => {
                    if tag == val {
                        tags.swap_remove(i);
                    }
                }
                None => (),
            }
        }
    }

    save_tasks(tasks, path);
}

pub fn attach_files(args: Vec<String>, path: &Path) {
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

        // Convert to stable relative path
        let safe_path = get_relative_to_todo(file_path, path);

        if !task.files.contains(&safe_path) {
            task.files.push(safe_path);
            count += 1;
        }
    }

    save_tasks(tasks, path);
    println!("Attached {} file(s) to task #{}.", count, task_number);
}

pub fn detach_files(args: Vec<String>, path: &Path) {
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

        // Convert to stable relative path
        let safe_path = get_relative_to_todo(file_path, path);

        if let Some(i) = task.files.iter().position(|f| f == &safe_path) {
            task.files.remove(i);
            count += 1;
        }
    }

    save_tasks(tasks, path);
    println!("detached {} file(s) to task #{}.", count, task_number);
}

pub fn complete_task(args: Vec<String>, path: &Path, mark: bool) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help complete' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut tasks = get_tasks(path);
    let mut count = 0;
    for i in 2..args.len() {
        let task_number: usize = match args.get(i).expect("Error getting arg.").parse() {
            Ok(num) => num,
            Err(_) => {
                println!(
                    "Invalid task index: {}",
                    args.get(i).expect("Error getting arg.")
                );
                continue;
            }
        };
        let task = match tasks.get_mut(task_number) {
            Some(t) => t,
            None => {
                println!("Task #{} not found.", task_number);
                continue;
            }
        };
        task.completed = mark;
        count += 1;
    }
    println!(
        "{} {} task(s).",
        if mark { "Completed" } else { "Uncompleted" },
        count
    );
    save_tasks(tasks, path);
}

pub fn delete_task(mut args: Vec<String>, path: &Path) {
    if args.len() < 3 {
        println!(
            "No task number or query provided.\nTry '{} help delete' for more details.",
            CMD_NAME
        );
        return;
    }
    args.remove(0); // remove command name
    args.remove(0); // remove subcommand name

    let mut tasks = get_tasks(path);
    let mut to_delete: Vec<usize> = Vec::new();

    // add tasks that have been queried for
    if find_arg_and_remove(&mut args, "--query", "--query").is_some() {
        let queried = query_tasks(get_tasks(path), config_query(&mut args), Vec::new());

        let mut results = queried
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<usize>>();

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
                            format_task(*original_index, &task, &DisplayConfig::DEFAULT)
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

    // add loose task number args
    to_delete.append(
        &mut args
            .iter()
            .filter_map(|arg| match arg.parse::<usize>() {
                Ok(num) => Some(num),
                Err(e) => {
                    println!("Arg '{}' could not be parsed into task number: {}", arg, e);
                    None
                }
            })
            .collect::<Vec<usize>>(),
    );

    // sort to_delete from highest to lowest value
    to_delete.sort_by(|a, b| b.cmp(a));
    let mut count = 0;
    for task_number in to_delete {
        if tasks.get(task_number).is_none() {
            println!("Task #{} not found.", task_number);
            continue;
        }
        tasks.remove(task_number);
        count += 1;
    }
    save_tasks(tasks, path);
    println!("Deleted {} task(s).", count);
}

pub fn clean_task(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => println!("File deleted successfully."),
        Err(err) => println!("Failed to delete file, error: {}", err),
    };
}
