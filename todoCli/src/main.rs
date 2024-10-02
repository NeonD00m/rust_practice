// - GET SERDE LIBRARY:

use serde::{Deserialize, Serialize};
use std::{cmp, env, fs};

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    text: String,
    completed: bool,
    tags: Vec<String>,
}

const CMD_NAME: &str = "todo";
const FILE_NAME: &str = "my_todo.json";
const PAGE_LENGTH: u8 = 8;

fn find_file() -> String {
    let mut file_path: &str = FILE_NAME;

    //check for file in current directory
    if !fs::metadata(FILE_NAME).is_ok() {
        //if not found check in parent directory
        let current_dir = env::current_dir().unwrap();
        let parent_dir = current_dir.parent().unwrap();
        file_path = parent_dir.join(FILE_NAME).to_str().unwrap();
    };
    return file_path.to_string();
}

fn get_tasks() -> Vec<Task> {
    let file_path = find_file();

    return match fs::read_to_string(file_path) {
        Ok(content) => serde_json::from_str(&content).unwrap(),
        Err(_) => Vec::new(),
    }; //.to_owned();
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

fn do_help(args: Vec<String>) {
    if args.len() > 2 {
        //user asked for help with a specific command
        println!(
            "\nNAME:\n\t\t{}-{} - {}\n",
            CMD_NAME,
            args[3],
            help_desc(args[3].as_str())
        );
        println!(
            "SYNOPSIS:\n\t\t{} {} {}\n",
            CMD_NAME,
            args[3],
            help_usage(args[3].as_str())
        );
        println!("Extended help for commands coming soon."); // TODO: help_options(), help_full()
    } else {
        //output general help and outline
        println!("Usage: {} [COMMAND]\n", CMD_NAME);

        println!("\tnew, n          {}", help_desc("new"));
        println!("\tadd, a          {}", help_desc("add"));
        println!("\tremove, r       {}", help_desc("remove"));
        println!("\tlist, l         {}", help_desc("list"));
        println!("\tsearch          {}", help_desc("search"));
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
}

fn add_task(args: Vec<String>) {
    if args.len() < 3 {
        println!(
            "No task number provided.\nTry '{} help new' for more details.",
            CMD_NAME
        );
        return;
    }
    let mut tasks = get_tasks();
}

fn list_task(args: Vec<String>) {
    let mut tasks = get_tasks();
    let mut page: u8 = 0;
    if args.len() >= 3 {
        page = args[2].parse().unwrap();
    }

    let len = tasks.len();
    let maxxed = cmp::max(page, len as u8 / PAGE_LENGTH) as u8;
    let start: u8 = cmp::min(maxxed * PAGE_LENGTH, 0);

    for i in start..start + 20 {
        let t = tasks.get(i).unwrap(); // TODO: figure out how to get the tasks from this
        println!(" ({}) [{}] {}", i, t, i);
    }
}

fn search_task(args: Vec<String>) {
    if args.len() < 3 {
        return println!(
            "No page number provided.\nTry '{} help search' for more details.",
            CMD_NAME
        );
    }
    let mut tasks = get_tasks();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's custom todo cli!");
        println!("Version - 0.1\n\n");
        do_help(args);
    }

    //figure out whether user wants to
    match args[1].as_str() {
        "new" => new_task(args),
        "n" => new_task(args),
        "add" => add_task(args),
        "a" => add_task(args),
        /*  "remove" => new_task(args),
            "r" => new_task(args),
            "list" => new_task(args),
            "l" => new_task(args),
            "search" => new_task(args),
            "complete" => new_task(args),
            "c" => new_task(args),
            "delete" => new_task(args),
            "clean" => new_task(args),
        */
        &_ => do_help(args);
    }
}
