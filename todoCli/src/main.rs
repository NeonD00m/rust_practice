pub mod core;
pub mod scanner;
use crate::core::*;
use crate::scanner::scan_tasks;
use std::{cmp, env, fs};

fn match_shortcut(cmd: &str) -> &str {
    match cmd {
        "n" => "new",
        "a" => "add",
        "r" => "remove",
        "l" => "list",
        "s" => "search",
        "c" => "complete",
        &_ => cmd,
    }
}

fn help_usage(cmd: &str) -> &str {
    match cmd {
        "new" => "[description of task...]",
        "add" => "[task number] [tags to add...]",
        "attach" => "[task number] [files to attach...]",
        "remove" => "[task number] [tags to remove...]",
        "list" => "[optional: page number] [optional: -i|--incomplete-only]",
        "search" => "[tags to search...]",
        "scan" => "[optional: directories/files to scan...]",
        "complete" => "[task numbers...]",
        "delete" => "[task numbers...]",
        "clean" => "",
        &_ => "",
    }
}

fn help_desc(cmd: &str) -> &str {
    match cmd {
        "new" => "creates a new task",
        "add" => "adds a tag to a task",
        "attach" => "attaches files to a task",
        "remove" => "removes a tag from a task",
        "list" => "lists tasks in pages",
        "search" => "searches tasks by tags",
        "scan" => "scans files and directories for TODO comments to convert",
        "complete" => "completes/uncompletes tasks",
        "delete" => "deletes tasks",
        "clean" => "deletes all tasks",
        &_ => "",
    }
}

fn do_help(args: &Vec<String>) {
    if args.len() > 2 {
        //user asked for help with a specific command
        let cmd = match_shortcut(args[2].as_str());
        println!("\nNAME:\n\t\t{}-{} - {}\n", CMD_NAME, cmd, help_desc(cmd));
        println!("SYNOPSIS:\n\t\t{} {} {}\n", CMD_NAME, cmd, help_usage(cmd));
        // println!("Extended help for commands coming soon."); // TODO: help_options(), help_full()
    } else {
        //output general help and outline
        println!("Usage: {} [COMMAND]\n", CMD_NAME);

        println!("\tnew, n          {}", help_desc("new"));
        println!("\tadd, a          {}", help_desc("add"));
        println!("\tattach, a       {}", help_desc("attach"));
        println!("\tremove, r       {}", help_desc("remove"));
        println!("\tlist, l         {}", help_desc("list"));
        println!("\tsearch, s       {}", help_desc("search"));
        println!("\tscan            {}", help_desc("scan"));
        println!("\tcomplete, c     {}", help_desc("complete"));
        println!("\tdelete          {}", help_desc("delete"));
        println!("\tclean           {}", help_desc("clean"));
    }
}

fn new_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task description provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut tasks = get_tasks();

    tasks.push(Task {
        text: args[2..].join(" "),
        completed: false,
        tags: Vec::new(),
        files: Vec::new(),
    });

    save_tasks(tasks);
}

fn add_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = args[2].parse().unwrap();
    let mut tasks = get_tasks();

    let tags = &mut tasks.get_mut(task_number).unwrap().tags;

    for i in 3..args.len() {
        match args.get(i) {
            Some(tag) => tags.push(tag.to_string()),
            None => (),
        }
    }

    save_tasks(tasks);
}

fn remove_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }
    let task_number: usize = args[2].parse().unwrap();
    let mut tasks = get_tasks();

    let tags = &mut tasks.get_mut(task_number).unwrap().tags;

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

    save_tasks(tasks);
}

fn list_task(args: Vec<String>) {
    let hide_completed = args
        .iter()
        .any(|arg| arg == "--incomplete-only" || arg == "-i");
    let show_files = args.iter().any(|arg| arg == "--files" || arg == "-f");
    let requested_page: usize = args[2..]
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .nth(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // Filter tasks FIRST while preserving original indices
    let tasks = get_tasks();
    let visible_tasks: Vec<(usize, &Task)> = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| !(hide_completed && task.completed))
        .collect();

    let len = visible_tasks.len();
    if len == 0 {
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
        page,
        pages - 1,
        CMD_NAME
    );
}

fn search_task(args: Vec<String>) {
    if args.len() < 3 {
        return println!(
            "No tags to search by provided.\nTry '{} help search' for more details.",
            CMD_NAME
        );
    }
    let show_files = args.iter().any(|arg| arg == "--files" || arg == "-f");

    let tasks = get_tasks();
    let tags = &args[2..];
    for (i, v) in tasks.iter().enumerate() {
        for tag in tags {
            // check instead if any tag in v contains an arg inside of it
            if v.text.contains(tag) || v.tags.iter().filter(|t| t.contains(tag)).count() > 0 {
                println!("{}", format_task(i, &v, show_files));
                break;
            }
        }
    }
}

fn attach_files(args: Vec<String>) {
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
    let mut tasks = get_tasks();
    let task = match tasks.get_mut(task_number) {
        Some(t) => t,
        None => {
            println!("Task #{} not found.", task_number);
            return;
        }
    };

    let mut count = 0;
    for path_str in &args[3..] {
        let path = std::path::Path::new(path_str);
        if !path.exists() {
            println!("Warning: File '{}' does not exist. Skipping.", path_str);
            continue;
        }

        // Convert to stable relative path
        let safe_path = get_relative_to_todo(path);

        if !task.files.contains(&safe_path) {
            task.files.push(safe_path);
            count += 1;
        }
    }

    save_tasks(tasks);
    println!("Attached {} file(s) to task #{}.", count, task_number);
}

fn complete_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help complete' for more details.",
            CMD_NAME
        );
        return;
    }

    let mut tasks = get_tasks();
    //let task_number: usize = args[2].parse().unwrap();
    //tasks.get_mut(task_number).unwrap().completed = true;

    for i in 2..args.len() {
        let task_number: usize = args.get(i).unwrap().parse().unwrap();
        let task = tasks.get_mut(task_number).unwrap();
        task.completed = !task.completed;
    }

    save_tasks(tasks);
}

fn delete_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task bumber provided.\nTry '{} help delete' for more details.",
            CMD_NAME
        );
        return;
    }

    let task_number: usize = args[2].parse().unwrap();
    let mut tasks = get_tasks();

    tasks.remove(task_number);
    save_tasks(tasks);
}

fn clean_task() {
    if let Err(err) = fs::remove_file(find_file()) {
        println!("Failed to delete file, error: {}", err);
    }
    //fs::remove_file(find_file());
    //save_tasks(Vec::new());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's custom todo cli!");
        println!("Version - {}\n", env!("CARGO_PKG_VERSION"));
        do_help(&args);
        return;
    }

    //figure out whether user wants to
    match match_shortcut(args[1].as_str()) {
        "new" => new_task(args),
        "add" => add_task(args),
        "attach" => attach_files(args),
        "remove" => remove_task(args),
        "list" => list_task(args),
        "search" => search_task(args),
        "scan" => scan_tasks(args),
        "complete" => complete_task(args),
        "delete" => delete_task(args),
        "clean" => clean_task(),
        &_ => do_help(&args),
    }
}
