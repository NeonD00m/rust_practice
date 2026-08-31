use crate::core::*;
use std::io::{IsTerminal, stdout};
use std::{cmp, path::Path};

fn is_piping() -> bool {
    !stdout().is_terminal()
}

pub struct DisplayConfig {
    markdown: bool,
    show_completion: bool,
    show_number: bool,
    show_files: bool,
    show_tags: bool,
}

pub struct QueryConfig {
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

pub fn find_arg_and_remove(
    args: &mut Vec<String>,
    short_flag: &str,
    long_flag: &str,
) -> Option<usize> {
    if let Some(pos) = args
        .iter()
        .position(|arg| arg == short_flag || arg == long_flag)
    {
        args.remove(pos);
        return Some(pos);
    }
    None
}

pub fn extract_flag_values(
    args: &mut Vec<String>,
    short_flag: &str,
    long_flag: &str,
) -> Vec<String> {
    let mut values = Vec::new();
    while let Some(pos) = args
        .iter()
        .position(|arg| arg == short_flag || arg == long_flag)
    {
        args.remove(pos);
        if pos < args.len() {
            values.push(args.remove(pos));
        }
    }
    values
}

pub fn config_display(args: &mut Vec<String>) -> DisplayConfig {
    if find_arg_and_remove(args, "-a", "--all").is_some() {
        return DisplayConfig::ALL;
    }

    let markdown = find_arg_and_remove(args, "-m", "--markdown").is_some();
    let show_completion = find_arg_and_remove(args, "-C", "--completion").is_some();
    let show_number = find_arg_and_remove(args, "-n", "--number").is_some();
    let show_files = find_arg_and_remove(args, "-f", "--files").is_some();
    let show_tags = find_arg_and_remove(args, "-T", "--tags").is_some();

    if !markdown && !show_completion && !show_number && !show_files && !show_tags {
        return DisplayConfig::DEFAULT;
    }

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
        tags: extract_flag_values(args, "-t", "--tag"),
        not_tags: extract_flag_values(args, "--not", "--not"),
        search_terms: extract_flag_values(args, "-s", "--search"),
    }
}

pub fn query_tasks(
    tasks: Vec<Task>,
    conf: QueryConfig,
    exceptions: Vec<usize>,
) -> Vec<(usize, Task)> {
    tasks
        .into_iter()
        .enumerate()
        .filter(|(id, task)| {
            if exceptions.contains(id) {
                return true;
            }

            if task.completed && conf.incomplete_only {
                return false;
            }
            if !task.completed && conf.complete_only {
                return false;
            }
            if !conf.search_terms.is_empty()
                && !conf
                    .search_terms
                    .iter()
                    .all(|term| task.text.contains(term))
            {
                return false;
            }
            if !conf.tags.is_empty() && !conf.tags.iter().all(|tag| task.tags.contains(tag)) {
                return false;
            }
            if !conf.not_tags.is_empty()
                && !conf.not_tags.iter().all(|tag| !task.tags.contains(tag))
            {
                return false;
            }

            true
        })
        .collect()
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
        output.push_str(") ");
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
        for (original_index, task) in query_tasks(get_tasks(path), query, Vec::new()) {
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

    let exceptions: Vec<usize> = args
        .iter()
        .filter_map(|arg| match arg.parse::<usize>() {
            Ok(num) => Some(num),
            Err(e) => {
                println!("Arg '{}' could not be parsed into task number: {}", arg, e);
                None
            }
        })
        .collect();

    let visible_tasks = query_tasks(get_tasks(path), query, exceptions);
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
