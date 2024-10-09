use serde::Deserialize;
use serde::Serialize;
use std::{cmp, env, fs};

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    text: String,
    completed: bool,
    tags: Vec<String>,
}

const CMD_NAME: &str = "todo";
const FILE_NAME: &str = "my_todo.json";
const PAGE_LENGTH: usize = 8;

fn find_file() -> String {
    let file_path;
    let current_dir = env::current_dir().unwrap();
    let new_dir = &current_dir.parent().unwrap().join(FILE_NAME);
    if !fs::metadata(FILE_NAME).is_ok() && fs::metadata(new_dir).is_ok() {
        //if not found check in parent directory
        file_path = new_dir.to_str().unwrap().to_owned();
    } else {
        file_path = FILE_NAME.to_string();
    };
    return file_path;
}

fn get_tasks() -> Vec<Task> {
    return match fs::read_to_string(find_file()) {
        Ok(content) => serde_json::from_str(&content).unwrap(),
        Err(_) => Vec::new(),
    };
}

fn save_tasks(tasks: Vec<Task>) {
    fs::write(find_file(), serde_json::to_string_pretty(&tasks).unwrap()).unwrap();
}

fn format_task(index: usize, task: &Task) -> String {
    let mut tags_text: String = task.tags.get(0).unwrap_or(&"".to_string()).to_string();

    for j in 1..task.tags.len() {
        tags_text.push_str(", ");
        tags_text.push_str(task.tags.get(j).unwrap());
    }

    return format!(" ({}) [{}] {}", index, tags_text, task.text);
}

fn help_usage(cmd: &str) -> &str {
    match cmd {
        "new" => "[description of task]",
        "add" => "[task number] [tags to add...]",
        "remove" => "[task number] [tags to add...]",
        "list" => "[optional page number]",
        "search" => "[tags to search...]",
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
        "remove" => "removes a tag from a task",
        "list" => "lists tasks in pages",
        "search" => "searches tasks by tags",
        "complete" => "completes a task",
        "delete" => "deletes a task",
        "clean" => "deletes all tasks",
        &_ => "",
    }
}

fn do_help(args: &Vec<String>) {
    if args.len() > 2 {
        //user asked for help with a specific command
        println!(
            "\nNAME:\n\t\t{}-{} - {}\n",
            CMD_NAME,
            args[2],
            help_desc(args[2].as_str())
        );
        println!(
            "SYNOPSIS:\n\t\t{} {} {}\n",
            CMD_NAME,
            args[2],
            help_usage(args[2].as_str())
        );
        println!("Extended help for commands coming soon."); // TODO: help_options(), help_full()
    } else {
        //output general help and outline
        println!("Usage: {} [COMMAND]\n", CMD_NAME);

        println!("\tnew, n          {}", help_desc("new"));
        println!("\tadd, a          {}", help_desc("add"));
        println!("\tremove, r       {}", help_desc("remove"));
        println!("\tlist, l         {}", help_desc("list"));
        println!("\tsearch, s       {}", help_desc("search"));
        println!("\tcomplete, c     {}", help_desc("complete"));
        println!("\tdelete          {}", help_desc("delete")); //should there be a recovery thing??
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
        text: args.get(2).unwrap().to_string(),
        completed: false,
        tags: Vec::new(),
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
        let tag;
        match args.get(i) {
            Some(val) => tag = val,
            None => continue,
        }

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
    let tasks = get_tasks();
    let mut page: usize = 0;
    if args.len() > 2 {
        page = args[2].parse().unwrap();
    }

    let len = tasks.len();
    let maxxed = cmp::max(page, len / PAGE_LENGTH);
    let start: usize = cmp::min(maxxed * PAGE_LENGTH, 0);

    for i in start..start + 20 {
        let t = match tasks.get(i) {
            Some(v) => v,
            None => continue,
        };
        println!("{}", format_task(i, &t));
    }
    println!(
        "\nPage {} of {}. Use '{} list [PAGE NUMBER]' to for more results.",
        maxxed,
        len / PAGE_LENGTH,
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

    let tasks = get_tasks();
    let tags = &args[2..];
    for (i, v) in tasks.iter().enumerate() {
        let mut success = true;
        for tag in tags {
            if !v.tags.contains(tag) {
                success = false;
                break;
            }
        }
        if !success {
            continue;
        }
        println!("{}", format_task(i, &v));
    }
}

fn complete_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help complete' for more details.",
            CMD_NAME
        );
        return;
    }

    let task_number: usize = args[2].parse().unwrap();
    let mut tasks = get_tasks();
    tasks.get_mut(task_number).unwrap().completed = true;

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
    save_tasks(Vec::new());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's custom todo cli!");
        println!("Version - 0.1\n");
        do_help(&args);
        return;
    }

    //figure out whether user wants to
    match args[1].as_str() {
        "new" => new_task(args),
        "n" => new_task(args),
        "add" => add_task(args),
        "a" => add_task(args),
        "remove" => remove_task(args),
        "r" => remove_task(args),
        "list" => list_task(args),
        "l" => list_task(args),
        "search" => search_task(args),
        "s" => search_task(args),
        "complete" => complete_task(args),
        "c" => complete_task(args),
        "delete" => delete_task(args),
        "clean" => clean_task(),
        &_ => do_help(&args),
    }
}
