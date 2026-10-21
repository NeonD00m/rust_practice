use crate::core::*;
use std::io::{IsTerminal, stdout};
use std::cmp;

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

#[derive(Clone, Debug)]
pub struct QueryConfig {
    pub incomplete_only: bool,
    pub complete_only: bool,
    pub tags: Vec<String>,
    pub not_tags: Vec<String>,
    pub search_terms: Vec<String>,
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

    pub fn is_empty(&self) -> bool {
        !self.incomplete_only
            && !self.complete_only
            && self.tags.is_empty()
            && self.not_tags.is_empty()
            && self.search_terms.is_empty()
    }
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
    if conf.is_empty() && !exceptions.is_empty() {
        return tasks
            .into_iter()
            .enumerate()
            .filter(|(id, _)| exceptions.contains(id))
            .collect();
    }
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

pub fn list_task(mut args: Vec<String>, targets: &[TodoTarget]) {
    if targets.is_empty() {
        println!("No todo files found.");
        return;
    }
    args.remove(0); // remove command name
    args.remove(0); // remove subcommand name
    let display = config_display(&mut args);
    let query = config_query(&mut args);
    let raw = find_arg_and_remove(&mut args, "-r", "--raw").is_some();

    let requested_page: usize = extract_flag_values(&mut args, "--page", "--page")
        .pop()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1_usize)
        .saturating_sub(1);

    let exceptions: Vec<usize> = args
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|arg| match arg.parse::<usize>() {
            Ok(num) => Some(num),
            Err(e) => {
                println!("Arg '{}' could not be parsed into task number: {}", arg, e);
                None
            }
        })
        .collect();

    if targets.len() > 1 && !exceptions.is_empty() {
        eprintln!(
            "Error: Cannot use task indices across multiple todo files. Use a single file with --path or use --query instead."
        );
        return;
    }

    let piping = is_piping() || raw;

    if targets.len() == 1 {
        let path = &targets[0].path;
        let visible_tasks = query_tasks(get_tasks(path), query, exceptions);
        if piping {
            for (original_index, task) in visible_tasks {
                println!("{}", format_task(original_index, &task, &display));
            }
            return;
        }

        let len = visible_tasks.len();
        if len < 1 {
            println!("No tasks found.");
            return;
        }
        let pages = len.div_ceil(PAGE_LENGTH);
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
        return;
    }

    // Multiple targets
    let mut total_tasks = 0;
    for target in targets {
        let visible_tasks =
            query_tasks(get_tasks(&target.path), query.clone(), exceptions.clone());
        if visible_tasks.is_empty() {
            continue;
        }
        total_tasks += visible_tasks.len();
        if !piping {
            println!(
                "=== [{}] ({}) ===",
                target.display_name,
                target.path.display()
            );
        }
        for (original_index, task) in visible_tasks {
            println!("{}", format_task(original_index, &task, &display));
        }
        if !piping {
            println!();
        }
    }
    if total_tasks == 0 {
        println!("No tasks found.");
    }
}
