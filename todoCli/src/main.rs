pub mod core;
pub mod scanner;
use crate::core::*;
use crate::scanner::scan_tasks;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::{
    cmp, env, fs,
    path::{Path, PathBuf},
};

fn match_shortcut(cmd: &str) -> &str {
    match cmd {
        "n" => "new",
        "a" => "add",
        "r" => "remove",
        "p" => "print",
        "l" => "list",
        "s" => "search",
        "c" => "complete",
        "u" => "undo",
        "-v" => "--version",
        &_ => cmd,
    }
}

fn help_desc(cmd: &str) -> &str {
    match cmd {
        "init" => "creates an empty todo list",
        "new" => "creates a new task",
        "add" => "adds a tag to a task",
        "edit" => "edits a task's description",
        "attach" => "attaches files to a task",
        "detach" => "detaches files to a task",
        "remove" => "removes a tag from a task",
        "print" => "prints tasks",
        "list" => "lists tasks in pages",
        "search" => "searches tasks by tags",
        "scan" => "scans files and directories for TODO comments to convert",
        "complete" => "completes tasks",
        "undo" => "uncompletes tasks",
        "delete" => "deletes a task",
        "clean" => "deletes all tasks",
        "help" => "why are you doing this",
        &_ => "",
    }
}

fn help_usage(cmd: &str) -> &str {
    match cmd {
        "init" => "[FLAGS]",
        "new" => "<DESCRIPTION...>",
        "add" => "<TASK_ID> <TAGS...>",
        "edit" => "<TASK_ID> [FLAGS]",
        "attach" => "<TASK_ID> <FILES...>",
        "detach" => "<TASK_ID> <FILES...>",
        "remove" => "<TASK_ID> <TAGS...>",
        "print" => "<TASK_IDs...> [FLAGS]",
        "list" => "[PAGE] [FLAGS]",
        "search" => "<QUERY...> [FLAGS]",
        "scan" => "[PATHS...]",
        "complete" => "<TASK_IDs...>",
        "undo" => "<TASK_IDs...>",
        "delete" => "<TASK_ID>",
        "clean" => "",
        "help" => "[COMMAND]",
        &_ => "",
    }
}

fn help_flags(cmd: &str) -> Vec<(&str, &str)> {
    match cmd {
        "print" => vec![
            ("--all", "print all optional flag fields"),
            ("-c, --completion", "print completion status"),
            ("-n, --number", "print task numbers"),
            ("-t, --tags", "print task tags"),
            ("-f, --files", "print attached files"),
        ],
        "list" => vec![
            ("-i, --incomplete-only", "show only incomplete tasks"),
            ("-c, --complete-only", "show only completed tasks"),
            ("-f, --files", "display attached files underneath tasks"),
        ],
        "search" => vec![
            (
                "-i, --incomplete-only",
                "search only within incomplete tasks",
            ),
            ("-c, --complete-only", "search only within completed tasks"),
            (
                "-t, --text-only",
                "search only for terms in the tasks' text contents, not tags",
            ),
            (
                "-f, --files",
                "display attached files underneath matched tasks",
            ),
        ],
        "edit" => vec![
            (
                "-r, --rewrite",
                "retype description from blank input instead of editing existing description",
            ),
            (
                "-o, --overwrite",
                "overwrite existing description with description from all non-flag command line arguments",
            ),
        ],
        &_ => Vec::new(),
    }
}

fn do_help(args: &Vec<String>) {
    if args.len() > 2 {
        // Command-specific help
        let cmd = match_shortcut(args[2].as_str());
        let desc = help_desc(cmd);

        if desc.is_empty() {
            println!(
                "Unknown command '{}'. Run '{} help' for available commands.",
                args[2], CMD_NAME
            );
            return;
        }

        println!("\nNAME:");
        println!("\t{}-{} - {}\n", CMD_NAME, cmd, desc);

        println!("USAGE:");
        println!("\t{} {} {}\n", CMD_NAME, cmd, help_usage(cmd));

        let flags = help_flags(cmd);
        if !flags.is_empty() {
            println!("FLAGS:");
            for (flag, flag_desc) in flags {
                println!("\t{:24} {}", flag, flag_desc);
            }
            println!(
                "\t{:24} {}\n",
                "-f, --file", "manually select which todo list file to use"
            );
        }
    } else {
        // General top-level help
        println!("Usage: {} [COMMAND] [FLAGS]\n", CMD_NAME);

        println!(
            "Commands:\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}",
            "init",
            help_desc("init"),
            "new, n",
            help_desc("new"),
            "add, a",
            help_desc("add"),
            "edit",
            help_desc("edit"),
            "attach",
            help_desc("attach"),
            "detach",
            help_desc("detach"),
            "remove, r",
            help_desc("remove"),
            "print, p",
            help_desc("print"),
            "list, l",
            help_desc("list"),
            "search, s",
            help_desc("search"),
            "scan",
            help_desc("scan"),
            "complete, c",
            help_desc("complete"),
            "undo, u",
            help_desc("undo"),
            "delete",
            help_desc("delete"),
            "clean",
            help_desc("clean")
        );

        println!(
            "\nFlags:\n\t{:24} {}\n\nUse \"{} help [COMMAND]\" for more information on a specific command.",
            "-v, --version", "prints version information", CMD_NAME
        );
    }
}

fn new_task(args: Vec<String>, path: &Path) {
    if args.len() < 3 {
        println!(
            "No task description provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut tasks = get_tasks(path);

    tasks.push(Task {
        text: args[2..].join(" "),
        completed: false,
        tags: Vec::new(),
        files: Vec::new(),
    });

    save_tasks(tasks, path);
}

fn add_task(args: Vec<String>, path: &Path) {
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

fn edit_task(args: Vec<String>, path: &Path) {
    if args.len() < 3 {
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

fn remove_task(args: Vec<String>, path: &Path) {
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

fn print_task(args: Vec<String>, path: &Path) {
    let show_all = args.iter().any(|arg| arg == "--all");
    let show_completion = show_all || args.iter().any(|arg| arg == "--completion" || arg == "-c");
    let show_number = show_all || args.iter().any(|arg| arg == "--number" || arg == "-n");
    let show_files = show_all || args.iter().any(|arg| arg == "--files" || arg == "-f");
    let show_tags = show_all || args.iter().any(|arg| arg == "--tags" || arg == "-t");
    let task_ids: Vec<usize> = args[2..]
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();
    let tasks = get_tasks(path);
    let visible_tasks: Vec<(usize, &Task)> = tasks
        .iter()
        .enumerate()
        .filter(|(id, _)| task_ids.contains(id))
        .collect();

    let len = visible_tasks.len();
    if len < 1 {
        println!("No tasks found.");
        return;
    }
    for (original_index, task) in &visible_tasks {
        println!(
            "{}{}{}{}{}",
            if show_completion {
                format!(
                    " {} ",
                    if task.completed {
                        CHECK_MARK
                    } else {
                        UNCHECKED
                    }
                )
            } else {
                String::new()
            },
            if show_number {
                format!("({}) ", original_index)
            } else {
                String::new()
            },
            if show_tags {
                format!("[{}] ", task.tags.join(", "))
            } else {
                String::new()
            },
            task.text,
            if show_files && !task.files.is_empty() {
                format!(" 📎 {}", task.files.join(", "))
            } else {
                String::new()
            }
        );
    }
}

fn list_task(args: Vec<String>, path: &Path) {
    let hide_completed = args
        .iter()
        .any(|arg| arg == "--incomplete-only" || arg == "-i");
    let hide_incompleted = args
        .iter()
        .any(|arg| arg == "--complete-only" || arg == "-c");
    let show_files = args.iter().any(|arg| arg == "--files" || arg == "-f");
    let requested_page: usize = args[2..]
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .nth(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        - 1;
    // Filter tasks FIRST while preserving original indices
    let tasks = get_tasks(path);
    let visible_tasks: Vec<(usize, &Task)> = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| {
            !(hide_completed && task.completed) && !(hide_incompleted && !task.completed)
        })
        .collect();

    let len = visible_tasks.len();
    if len < 1 {
        println!("No tasks found.");
        return;
    }
    let pages = (len + PAGE_LENGTH - 1) / PAGE_LENGTH;
    let page = cmp::min(requested_page, pages - 1);
    let start: usize = page * PAGE_LENGTH;
    let end = cmp::min(start + PAGE_LENGTH, len);

    for (original_index, task) in &visible_tasks[start..end] {
        println!("{}", format_task(*original_index, task, show_files));
    }
    if pages <= 1 {
        return;
    }
    println!(
        "\nPage {} of {}. Use '{} list [PAGE NUMBER]' for more results.",
        page + 1,
        pages,
        CMD_NAME
    );
}

fn search_task(args: Vec<String>, path: &Path) {
    let search_terms: Vec<&String> = args[2..]
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .collect();
    if search_terms.len() < 1 {
        return println!(
            "No tags to search by provided.\nTry '{} help search' for more details.",
            CMD_NAME
        );
    }
    let show_files = args.iter().any(|arg| arg == "--files" || arg == "-f");
    let hide_completed = args
        .iter()
        .any(|arg| arg == "--incomplete-only" || arg == "-i");
    let hide_incompleted = args
        .iter()
        .any(|arg| arg == "--complete-only" || arg == "-c");
    let search_text = args.iter().any(|arg| arg == "--text-only" || arg == "-t");

    let tasks = get_tasks(path);
    let visible_tasks: Vec<(usize, &Task)> = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| {
            !(hide_completed && task.completed) && !(hide_incompleted && !task.completed)
        })
        .collect();
    for (i, v) in visible_tasks {
        let mut print = true;
        for tag in &search_terms {
            if search_text {
                if !v.text.contains(tag.as_str()) {
                    print = false;
                    break;
                }
            } else {
                // check instead if v has every tag
                if v.tags.iter().find(|t| t == tag).is_none() {
                    print = false;
                    break;
                }
            }
        }
        if print {
            println!("{}", format_task(i, &v, show_files));
        }
    }
}

fn attach_files(args: Vec<String>, path: &Path) {
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

fn detach_files(args: Vec<String>, path: &Path) {
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

fn complete_task(args: Vec<String>, path: &Path, mark: bool) {
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

fn delete_task(args: Vec<String>, path: &Path) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help delete' for more details.",
            CMD_NAME
        );
        return;
    }

    let task_number: usize = args[2].parse().expect("Invalid task index.");
    let mut tasks = get_tasks(path);
    if tasks.get(task_number).is_none() {
        println!("Task #{} not found.", task_number);
        return;
    }
    tasks.remove(task_number);
    save_tasks(tasks, path);
}

fn clean_task(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => println!("File deleted successfully."),
        Err(err) => println!("Failed to delete file, error: {}", err),
    };
}

fn value_flag(args: &[String], short_flag: &str, long_flag: &str) -> Option<(String, usize)> {
    args.iter()
        .position(|arg| arg == short_flag || arg == long_flag)
        .and_then(|pos| args.get(pos + 1).cloned().and_then(|s| Some((s, pos))))
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's custom todo cli!");
        println!("Version - {}\n", env!("CARGO_PKG_VERSION"));
        do_help(&args);
        return;
    }

    let todo_path_input = value_flag(&args, "-f", "--file");
    let todo_path = if let Some((p, pos)) = todo_path_input {
        // remove flag and path from args list
        if pos + 1 < args.len() {
            args.remove(pos);
        }
        args.remove(pos);

        PathBuf::from(p.as_str())
    } else {
        find_file()
    };

    //figure out whether user wants to
    match match_shortcut(args.get(1).expect("Error getting command arg.").as_str()) {
        "init" => save_tasks(Vec::new(), &todo_path),
        "new" => new_task(args, &todo_path),
        "add" => add_task(args, &todo_path),
        "edit" => edit_task(args, &todo_path),
        "attach" => attach_files(args, &todo_path),
        "detach" => detach_files(args, &todo_path),
        "remove" => remove_task(args, &todo_path),
        "print" => print_task(args, &todo_path),
        "list" => list_task(args, &todo_path),
        "search" => search_task(args, &todo_path),
        "scan" => scan_tasks(args, &todo_path),
        "complete" => complete_task(args, &todo_path, true),
        "undo" => complete_task(args, &todo_path, false),
        "delete" => delete_task(args, &todo_path),
        "clean" => clean_task(&todo_path),
        "--version" => println!("Version - {}", env!("CARGO_PKG_VERSION")),
        &_ => do_help(&args),
    }
}
