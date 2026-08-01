use crate::core::*;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

struct ScannedTodo {
    text: String,
    file_path: String,
}

fn scan_directory(dir: &Path, found: &mut Vec<ScannedTodo>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Skip common noise directories like .git, node_modules, target
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name != ".git" && name != "target" && name != "node_modules" {
                    scan_directory(&path, found);
                }
                continue;
            }

            // Scan file contents line-by-line
            if let Ok(file) = fs::File::open(&path) {
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
                                file_path: get_relative_to_todo(&path),
                            });
                        }
                    }
                }
            }
        }
    }
}
fn scan_path(path: &Path, found: &mut Vec<ScannedTodo>) {
    if !path.exists() {
        println!(
            "Warning: Path '{}' does not exist. Skipping.",
            path.display()
        );
        return;
    }

    if path.is_dir() {
        scan_directory(path, found);
    } else if path.is_file() {
        // Scan a single file directly
        if let Ok(file) = fs::File::open(path) {
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
                            file_path: get_relative_to_todo(&path),
                        });
                    }
                }
            }
        }
    }
}

pub fn scan_tasks(args: Vec<String>) {
    let mut found_todos = Vec::new();

    if args.len() > 2 {
        // User provided specific files or directories: `todo scan src/main.rs tests/`
        println!("Scanning specified target(s)...");
        for path_str in &args[2..] {
            scan_path(Path::new(path_str), &mut found_todos);
        }
    } else {
        // Default to current directory if no args given: `todo scan`
        println!("Scanning current project directory...");
        scan_path(Path::new("."), &mut found_todos);
    }

    if found_todos.is_empty() {
        println!("No TODO comments found!");
        return;
    }

    let mut tasks = get_tasks();
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
        print!("Add as task with attached file? (y/n or i to ignore file): ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

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
        save_tasks(tasks);
        println!("\nSuccessfully imported {} new task(s)!", added_count);
    } else {
        println!("\nNo new tasks added.");
    }
}
