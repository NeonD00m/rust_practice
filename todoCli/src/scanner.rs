use crate::core::*;
use std::fs;
use std::io::{BufRead, BufReader, Write, stdin, stdout};
use std::path::Path;

struct ScannedTodo {
    text: String,
    file_path: String,
    complete: bool,
}

fn scan_directory(dir: &Path, found: &mut Vec<ScannedTodo>, path: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let target_path = entry.path();

            // Skip common noise directories like .git, node_modules, target
            if target_path.is_dir() {
                let name = target_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                if name != ".git" && name != "target" && name != "node_modules" {
                    scan_directory(&target_path, found, path);
                }
                continue;
            }

            // Scan file contents line-by-line
            if let Ok(file) = fs::File::open(&target_path) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();

                    // Match common comment styles
                    let comment_content = if let Some(idx) = trimmed.find("// TODO:") {
                        Some(&trimmed[idx + 8..])
                    } else if let Some(idx) = trimmed.find("# TODO:") {
                        Some(&trimmed[idx + 7..])
                    } else if let Some(idx) = trimmed.find("-- TODO:") {
                        Some(&trimmed[idx + 8..])
                    } else {
                        None
                    };

                    if let Some(content) = comment_content {
                        let clean_text = content.trim();
                        if !clean_text.is_empty() {
                            found.push(ScannedTodo {
                                text: clean_text.to_string(),
                                file_path: get_relative_to_todo(&target_path, path),
                                complete: false,
                            });
                        }
                    }
                }
            }
        }
    }
}

fn scan_path(target_path: &Path, found: &mut Vec<ScannedTodo>, path: &Path) {
    if !target_path.exists() {
        println!(
            "Warning: Path '{}' does not exist. Skipping.",
            target_path.display()
        );
        return;
    }

    if target_path.is_dir() {
        scan_directory(target_path, found, path);
    } else if target_path.is_file() {
        // Scan a single file directly
        if let Ok(file) = fs::File::open(target_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                let trimmed = line.trim();

                let comment_content = if let Some(idx) = trimmed.find("// TODO:") {
                    Some(&trimmed[idx + 8..])
                } else if let Some(idx) = trimmed.find("# TODO:") {
                    Some(&trimmed[idx + 7..])
                } else if let Some(idx) = trimmed.find("-- TODO:") {
                    Some(&trimmed[idx + 8..])
                } else {
                    None
                };

                if let Some(content) = comment_content {
                    let clean_text = content.trim();
                    if !clean_text.is_empty() {
                        found.push(ScannedTodo {
                            text: clean_text.to_string(),
                            file_path: get_relative_to_todo(&target_path, path),
                            complete: false,
                        });
                    }
                }
            }
        }
    }
}

pub fn scan_tasks(args: Vec<String>, path: &Path) {
    let mut found_todos = Vec::new();

    if args.len() > 2 {
        // User provided specific files or directories: `todo scan src/main.rs tests/`
        println!("Scanning specified target(s)...");
        for path_str in &args[2..] {
            scan_path(Path::new(path_str), &mut found_todos, path);
        }
    } else {
        // Default to current directory if no args given: `todo scan`
        println!("Scanning current project directory...");
        scan_path(Path::new("."), &mut found_todos, path);
    }

    if found_todos.is_empty() {
        println!("No TODO comments found!");
        return;
    }

    let mut tasks = get_tasks(path);
    let mut added_count = 0;
    let mut ignored_files = Vec::new();

    for item in found_todos {
        if ignored_files.contains(&item.file_path) {
            continue;
        }
        let exists = tasks.iter().any(|t| t.text == item.text);
        if exists {
            continue;
        }

        println!("\nFound: \"{}\" in {}", item.text, item.file_path);
        print!("Add as task with attached file? (y/N/i to ignore file): ");
        Write::flush(&mut stdout()).expect("Error flushing stdout.");

        let mut input = String::new();
        stdin().read_line(&mut input).expect("Error reading input.");

        if input.trim().eq_ignore_ascii_case("y") {
            tasks.push(Task {
                text: item.text,
                completed: false,
                tags: vec!["code-todo".to_string()],
                files: vec![item.file_path],
            });
            added_count += 1;
        } else if input.trim().eq_ignore_ascii_case("i") {
            ignored_files.push(item.file_path);
        }
    }

    if added_count > 0 {
        save_tasks(tasks, path);
        println!("\nSuccessfully imported {} new task(s)!", added_count);
    } else {
        println!("\nNo new tasks added.");
    }
}

pub fn import_tasks(args: Vec<String>, path: &Path) {
    let mut found_todos = Vec::new();

    for path_str in &args[2..] {
        // check if file exists, scan line-by-line for "- [*]", and read until end of lin
        let p = Path::new(path_str);
        if !p.exists() {
            println!("Warning: Path '{}' does not exist. Skipping.", p.display());
            continue;
        }

        // read file line-by-line
        let file = match fs::File::open(p) {
            Ok(f) => f,
            Err(e) => {
                println!("Couldn't open file: {}", e);
                continue;
            }
        };
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if !trimmed.starts_with("- [") {
                continue;
            }
            let complete = trimmed.get(3..4).map(|s| s == "x").unwrap_or(false);
            let text = match trimmed.splitn(2, "] ").nth(1).map(|s| s.trim().to_string()) {
                Some(s) => s,
                None => {
                    println!("Warning: Malformed task line: '{}'. Skipping.", trimmed);
                    continue;
                }
            };
            found_todos.push(ScannedTodo {
                text,
                file_path: get_relative_to_todo(p, path),
                complete,
            });
        }
    }

    if found_todos.is_empty() {
        println!("No check list items found!");
        return;
    }

    let mut tasks = get_tasks(path);
    let mut updated_count = 0;

    for item in found_todos {
        println!("\nFound: \"{}\"", item.text);
        if let Some(i) = tasks.iter().position(|t| t.text == item.text) {
            let t = tasks
                .get_mut(i)
                .expect("Failed to get task at index where it was found.");
            if t.completed == item.complete {
                continue;
            }
            print!(
                "Update existing task completion status to {}? (Y/n): ",
                if item.complete { CHECK_MARK } else { UNCHECKED }
            );
            Write::flush(&mut stdout()).expect("Error flushing stdout.");

            let mut input = String::new();
            stdin().read_line(&mut input).expect("Error reading input.");

            if !input.trim().eq_ignore_ascii_case("n") {
                t.completed = item.complete;
                updated_count += 1;
            }
            continue;
        }

        print!("Add as task? (Y/n): ");
        Write::flush(&mut stdout()).expect("Error flushing stdout.");

        let mut input = String::new();
        stdin().read_line(&mut input).expect("Error reading input.");

        if !input.trim().eq_ignore_ascii_case("n") {
            tasks.push(Task {
                text: item.text,
                completed: item.complete,
                tags: vec!["md-todo".to_string()],
                files: vec![item.file_path],
            });
            updated_count += 1;
        }
    }

    if updated_count > 0 {
        save_tasks(tasks, path);
        println!("\nSuccessfully imported {} task(s)!", updated_count);
    } else {
        println!("\nNo tasks imported.");
    }
}
