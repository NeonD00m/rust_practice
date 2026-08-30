use crate::core::*;
use std::io::{IsTerminal, stdout};
use std::option;
use std::{cmp, path::Path};

fn is_piping() -> bool {
    !stdout().is_terminal()
}

struct DisplayConfig {
    markdown: bool,
    show_completion: bool,
    show_number: bool,
    show_files: bool,
    show_tags: bool,
}

struct QueryConfig {
    incomplete_only: bool,
    complete_only: bool,
    tags: Vec<String>,
    not_tags: Vec<String>,
    search_terms: Vec<String>,
}

impl DisplayConfig {
    pub const DEFAULT: Self = Self {
        markdown: false,
        show_completion: true,
        show_number: true,
        show_files: false,
        show_tags: true,
    };

    pub const ALL: Self = Self {
        markdown: false,
        show_completion: true,
        show_number: true,
        show_files: true,
        show_tags: true,
    };
}

impl QueryConfig {
    pub const DEFAULT: Self = Self {
        incomplete_only: false,
        complete_only: false,
        tags: Vec::new(),
        not_tags: Vec::new(),
        search_terms: Vec::new(),
    };
}

fn find_arg_and_remove(args: &mut Vec<String>, short_flag: &str, long_flag: &str) -> Option<usize> {
    if let Some(pos) = args
        .iter()
        .position(|arg| arg == short_flag || arg == long_flag)
    {
        args.remove(pos);
        return Some(pos);
    }
    None
}

pub fn config_display(args: &mut Vec<String>) -> DisplayConfig {
    if args.iter().any(|arg| arg == "--all" || arg == "-a") {
        return DisplayConfig::ALL;
    }

    let markdown = find_arg_and_remove(args, "-m", "--markdown").is_some();
    let show_completion = find_arg_and_remove(args, "-C", "--completion").is_some();
    let show_number = find_arg_and_remove(args, "-n", "--number").is_some();
    let show_files = find_arg_and_remove(args, "-f", "--files").is_some();
    let show_tags = find_arg_and_remove(args, "-T", "--tags").is_some();

    DisplayConfig {
        markdown,
        show_completion,
        show_number,
        show_files,
        show_tags,
    }
}

pub fn config_query(args: &mut Vec<String>) -> QueryConfig {
    QueryConfig {
        incomplete_only: find_arg_and_remove(args, "-i", "--incomplete-only").is_some(),
        complete_only: find_arg_and_remove(args, "-c", "--complete-only").is_some(),
        tags: args
            .iter()
            .enumerate()
            .filter_map(|(i, arg)| {
                if arg == "-t" || arg == "--tags" {
                    args.get(i + 1).cloned()
                } else {
                    None
                }
            })
            .collect(),
        not_tags: args
            .iter()
            .enumerate()
            .filter_map(|(i, arg)| {
                if arg == "--not" || arg == "--not" {
                    args.get(i + 1).cloned()
                } else {
                    None
                }
            })
            .collect(),
        search_terms: args
            .iter()
            .enumerate()
            .filter_map(|(i, arg)| {
                if arg == "-s" || arg == "--search" {
                    args.get(i + 1).cloned()
                } else {
                    None
                }
            })
            .collect(),
    }
}

pub fn query_tasks(tasks: Vec<Task>, conf: QueryConfig) -> Vec<(usize, Task)> {
    let mut tasks = tasks.into_iter().enumerate();

    if conf.incomplete_only {
        tasks = tasks.filter(|(_, task)| !task.completed);
    }

    if conf.complete_only {
        tasks = tasks.filter(|(_, task)| task.completed);
    }

    if !conf.tags.is_empty() {
        tasks = tasks.filter(|(_, task)| conf.tags.iter().all(|tag| task.tags.contains(tag)));
    }

    if !conf.not_tags.is_empty() {
        tasks = tasks.filter(|(_, task)| conf.not_tags.iter().all(|tag| !task.tags.contains(tag)));
    }

    if !conf.search_terms.is_empty() {
        tasks = tasks.filter(|(_, task)| {
            conf.search_terms
                .iter()
                .all(|term| task.text.contains(term))
        });
    }

    tasks.collect()
}

pub fn format_task(id: usize, task: &Task, conf: &DisplayConfig) -> String {
    let mut output = String::new();
    if conf.markdown {
        output.push_str(if task.completed { "- [x] " } else { "- [ ] " });
    } else if conf.show_completion {
        output.push(' ');
        output.push(if task.completed {
            CHECK_MARK
        } else {
            UNCHECKED
        });
        output.push(' ');
    }
    if conf.show_number {
        output.push('(');
        output.push_str(&id.to_string());
        output.push(')');
    }

    if conf.show_tags {
        output.push('[');
        output.push_str(task.tags.join(", ").as_str());
        output.push_str("] ");
    }

    output.push_str(&task.text);

    if task.files.is_empty() || !conf.show_files {
        return output;
    }

    if conf.markdown {
        output.push_str(" 📎 ");
        output.push_str(task.files.join(", ").as_str());
    } else {
        for (i, file) in task.files.iter().enumerate() {
            // Use a different arrow character for the last item
            let connector = if i == task.files.len() - 1 {
                "└──"
            } else {
                "├──"
            };
            output.push_str(&format!("\n {} {}", connector, file));
        }
    }

    output
}

pub fn list_task(mut args: Vec<String>, path: &Path) {
    args.remove(0); // remove command name
    args.remove(0); // remove subcommand name
    let display = config_display(&mut args);
    let query = config_query(&mut args);

    if is_piping() || args.iter().any(|s| s == "--raw") {
        for (original_index, task) in query_tasks(get_tasks(path), query) {
            println!("{}", format_task(original_index, &task, &display));
        }
        return;
    }

    let requested_page: usize = find_arg_and_remove(&mut args, "--page", "--page")
        .and_then(|p| args.get(p))
        .and_then(|s| {
            Some(s.parse().unwrap_or_else(|_| {
                println!("Failed to parse page argument.");
                0
            }))
        })
        .unwrap_or_default();

    let visible_tasks = query_tasks(get_tasks(path), query);
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
        println!("{}", format_task(*original_index, task, &display));
    }
    if pages <= 1 {
        return;
    }
    println!(
        "\nPage {} of {}. Use '{} list --page {}' for more results.",
        page + 1,
        pages,
        CMD_NAME,
        page + 2
    );
}

pub fn search_task(args: Vec<String>, path: &Path) {
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
