pub mod actions;
pub mod core;
pub mod output;
pub mod scanner;
use crate::actions::*;
use crate::core::*;
use crate::output::*;
use crate::scanner::*;
use std::env;

fn match_shortcut(cmd: &str) -> &str {
    match cmd {
        "n" => "new",
        "a" => "add",
        "r" => "remove",
        "p" => "list",
        "print" => "list",
        "l" => "list",
        "s" => "list",
        "search" => "list",
        "c" => "complete",
        "u" => "undo",
        "d" => "discover",
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
        "detach" => "detaches files from a task",
        "remove" => "removes a tag from a task",
        "print" => "prints tasks",
        "list" => "lists tasks in pages",
        "search" => "searches tasks by tags",
        "discover" => "recursively finds and prints out all todo lists",
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
        "new" => "<DESCRIPTION...> [FLAGS]",
        "add" => "<TASK_ID> <TAGS...> [FLAGS]",
        "edit" => "<TASK_ID> [FLAGS]",
        "attach" => "<TASK_ID> <FILES...> [FLAGS]",
        "detach" => "<TASK_ID> <FILES...> [FLAGS]",
        "remove" => "<TASK_ID> <TAGS...> [FLAGS]",
        "list" => "[TASK_IDs...] [FLAGS] [QUERY]",
        "discover" => "[FLAGS]",
        "scan" => "[PATHS...]",
        "import" => "[PATHS...]",
        "complete" => "[TASK_IDs...] [QUERY]",
        "undo" => "[TASK_IDs...] [QUERY]",
        "delete" => "[TASK_IDs...] [FLAGS] [QUERY]",
        "clean" => "",
        "help" => "[COMMAND]",
        &_ => "",
    }
}

fn help_flags(cmd: &str) -> Vec<(&str, &str)> {
    match cmd {
        "new" => vec![
            ("-v, --verbose", "outputs new task id"),
            ("-t, --tag <TAG>", "takes in an argument for a tag to add"),
            ("-a, --attach <FILE>", "attaches a file to the new task"),
        ],
        "add" | "attach" | "detach" => vec![
            (
                "-v, --verbose",
                "outputs numbers of operations completed successfully",
            ),
            ("-q, --query", "select tasks with a query"),
        ],
        "remove" => vec![("-q, --query", "select tasks with a query")],
        "discover" => vec![(
            "--depth <NUMBER>",
            "max directory depth to search for todo lists",
        )],
        "list" => vec![
            ("--page <NUMBER>", "displays the given page of tasks"),
            (
                "-s, --search <TERM>",
                "filter tasks that contain the search term in their description (query)",
            ),
            (
                "-t, --tag <TAG>",
                "filter tasks that have the given tag (query)",
            ),
            (
                "--not <TAG>",
                "filter tasks that do not have the given tag (query)",
            ),
            (
                "-i, --incomplete-only",
                "filter only incomplete tasks (query)",
            ),
            ("-c, --complete-only", "filter only completed tasks (query)"),
            ("-m, --markdown", "formats for Github markdown check lists"),
            (
                "--all",
                "selects all optional display flags (except markdown)",
            ),
            ("-C, --completion", "display completion status"),
            ("-n, --number", "display task numbers"),
            ("-T, --tags", "display task tags"),
            ("-f, --files", "display attached files"),
            (
                "-r, --raw",
                "outputs raw formatted task lines without paging",
            ),
            ("-d, --discover", "recursively discover all todo lists"),
            (
                "--depth <NUMBER>",
                "max directory depth to search for todo lists",
            ),
        ],
        "edit" => vec![
            (
                "-r, --rewrite",
                "retype description from blank input instead of editing existing description",
            ),
            (
                "-o, --overwrite <NEW_DESCRIPTION>",
                "overwrite existing description with description from all non-flag command line arguments",
            ),
        ],
        "delete" | "complete" | "undo" => vec![(
            "-q, --query",
            "select additional tasks with a query (see 'help list' for query flags)",
        )],
        &_ => Vec::new(),
    }
}

fn do_help(args: &[String]) {
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
        println!("FLAGS:");
        for (flag, flag_desc) in flags {
            println!("\t{:24} {}", flag, flag_desc);
        }
        let personal_display = personal_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~\\my_todo.json".to_string());
        println!(
            "\t{:24} manually select which todo list file to use (can specify multiple)\n\t{:24} uses the device user's personal todo list at {}\n\t{:24} recursively discover all todo list files\n\t{:24} max directory depth when discovering todo lists\n",
            "-p, --path <PATH>",
            "--personal",
            personal_display,
            "-d, --discover",
            "--depth <NUMBER>"
        );
    } else {
        // General top-level help
        println!("Usage: {} [COMMAND] [FLAGS]\n", CMD_NAME);

        println!(
            "Commands:\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}\n\t{:24} {}",
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
            "list, l",
            help_desc("list"),
            "discover, d",
            help_desc("discover"),
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
            "\nFlags:\n\t{:24} prints version information\n\nUse \"{} help [COMMAND]\" for more information on a specific command.",
            "-v, --version", CMD_NAME
        );
    }
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's custom todo cli!");
        println!("Version - {}\n", env!("CARGO_PKG_VERSION"));
        do_help(&args);
        return;
    }

    let has_discover_cmd = args.iter().any(|s| {
        let sc = match_shortcut(s.as_str());
        sc == "discover"
    });
    let targets = resolve_targets(&mut args, has_discover_cmd);

    if args.len() < 2 {
        if has_discover_cmd {
            discover_cmd(&targets);
            return;
        }
        println!("Welcome to Max's custom todo cli!");
        println!("Version - {}\n", env!("CARGO_PKG_VERSION"));
        do_help(&args);
        return;
    }

    let cmd = match_shortcut(args.get(1).expect("Error getting command arg.").as_str());

    // figure out whether user wants to
    match cmd {
        "init" => {
            for target in &targets {
                save_tasks(Vec::new(), &target.path);
                println!("Created empty todo list at {}.", target.path.display());
            }
        }
        "discover" => discover_cmd(&targets),
        "new" => new_task(args, &targets),
        "add" => add_tags(args, &targets),
        "edit" => edit_task(args, &targets),
        "attach" => attach_files(args, &targets),
        "detach" => detach_files(args, &targets),
        "remove" => remove_tags(args, &targets),
        "list" => list_task(args, &targets),
        "scan" => scan_tasks(args, &targets),
        "import" => import_tasks(args, &targets),
        "complete" => complete_task(args, &targets, true),
        "undo" => complete_task(args, &targets, false),
        "delete" => delete_task(args, &targets),
        "clean" => clean_task(&targets),
        "--version" => println!("Version - {}", env!("CARGO_PKG_VERSION")),
        &_ => do_help(&args),
    }
}
