use serde::Deserialize;
use serde::Serialize;
use std::{env, fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub text: String,
    pub completed: bool,
    pub tags: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

pub const CMD_NAME: &str = "todo";
pub const FILE_NAME: &str = "my_todo.json";
pub const PAGE_LENGTH: usize = 10;
pub const CHECK_MARK: char = '🗹'; //✓🗹';
pub const UNCHECKED: char = '☐'; //☐';

pub fn find_file() -> String {
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

pub fn get_relative_to_todo(target_path: &std::path::Path) -> String {
    // Resolve target to an absolute path
    let abs_target = target_path
        .canonicalize()
        .unwrap_or_else(|_| target_path.to_path_buf());

    // Resolve my_todo.json to an absolute path
    let json_path = std::path::Path::new(&find_file())
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(find_file()));

    let json_dir = json_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));

    // Strip the JSON directory from the target path
    match abs_target.strip_prefix(json_dir) {
        Ok(relative_path) => relative_path.to_string_lossy().to_string(),
        Err(_) => abs_target.to_string_lossy().to_string(), // Fallback to absolute if outside project
    }
}

pub fn get_tasks() -> Vec<Task> {
    return match fs::read_to_string(find_file()) {
        Ok(content) => serde_json::from_str(&content).unwrap(),
        Err(_) => Vec::new(),
    };
}

pub fn save_tasks(tasks: Vec<Task>) {
    fs::write(find_file(), serde_json::to_string_pretty(&tasks).unwrap()).unwrap();
}

pub fn format_task(index: usize, task: &Task, show_files: bool) -> String {
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
