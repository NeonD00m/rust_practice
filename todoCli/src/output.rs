use crate::core::*;
use std::{cmp, path::Path};

fn format_task(index: usize, task: &Task, show_files: bool) -> String {
    let mut output = format!(
        " {} ({}) [{}] {}",
        if task.completed {
            CHECK_MARK
        } else {
            UNCHECKED
        },
        index,
        task.tags.join(", "),
        task.text
    );

    // Format files conditionally underneath using a tree structure
    if show_files && !task.files.is_empty() {
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

pub fn print_task(args: Vec<String>, path: &Path) {
    let markdown = args.iter().any(|arg| arg == "-m" || arg == "--markdown");
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
            if markdown {
                String::from(if task.completed { "- [x] " } else { "- [ ] " })
            } else if show_completion {
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

pub fn list_task(args: Vec<String>, path: &Path) {
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
