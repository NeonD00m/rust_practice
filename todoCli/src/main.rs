pub mod actions;
pub mod core;
pub mod output;
pub mod scanner;
use crate::actions::*;
use crate::core::*;
use crate::output::*;
use crate::scanner::*;
use std::{env, path::PathBuf};

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
        "import" => "scans specified files for github markdown check lists",
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
        "import" => "[PATHS...]",
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
            ("-m, --markdown", "formats for Github markdown check lists"),
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
        "import" => import_tasks(args, &todo_path),
        "complete" => complete_task(args, &todo_path, true),
        "undo" => complete_task(args, &todo_path, false),
        "delete" => delete_task(args, &todo_path),
        "clean" => clean_task(&todo_path),
        "--version" => println!("Version - {}", env!("CARGO_PKG_VERSION")),
        &_ => do_help(&args),
    }
}
