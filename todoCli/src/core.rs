use file_id::{FileId, get_file_id};
use serde::Deserialize;
use serde::Serialize;
use std::{
    env, fs,
    io::{self, IsTerminal, Read, stdin},
    path::{Path, PathBuf},
};

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
pub const CHECK_MARK: char = '🗹';
pub const UNCHECKED: char = '☐';

fn path_to_file(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(FILE_NAME)
    } else {
        path.to_path_buf()
    }
}

fn get_volume_id(path: &Path) -> io::Result<u64> {
    let id = get_file_id(path)?;

    // destructure enum to get device/volume serial identifier
    Ok(match id {
        FileId::Inode { device_id, .. } => device_id,
        FileId::LowRes {
            volume_serial_number,
            ..
        } => volume_serial_number as u64,
        FileId::HighRes {
            volume_serial_number,
            ..
        } => volume_serial_number,
    })
}

pub fn find_file() -> PathBuf {
    if let Ok(s) = env::var("TODO_FILE")
        && let p = PathBuf::from(s)
        && p.exists()
    {
        return p;
    }
    let current_dir = env::current_dir().expect("Error getting current directory.");
    let default = PathBuf::from(FILE_NAME.to_string());
    let initial_id = match get_volume_id(&current_dir) {
        Ok(id) => id,
        Err(_) => {
            println!("Warning: No Initial Volume Id");
            return default;
        } // Skip files where metadata permissions are blocked
    };
    let mut new_dir = current_dir.as_path();
    if !default.exists() {
        // recursively check parent directory until we find a file or there is no parent
        loop {
            // end search if we have changed devices
            if let Ok(id) = get_volume_id(new_dir)
                && id != initial_id
            {
                println!("Warning: Changed Devices");
                break;
            }

            // check to see if we find the target file
            if new_dir.join(FILE_NAME).exists() {
                return new_dir.join(FILE_NAME);
            }
            new_dir = match new_dir.parent() {
                Some(p) => p,
                None => {
                    println!("Warning: No parent");
                    break;
                }
            };
        }
    };
    return default;
}

pub fn get_relative_to_todo(target_path: &Path, todo_path: &Path) -> String {
    // Resolve target to an absolute path
    let abs_target = target_path
        .canonicalize()
        .unwrap_or_else(|_| target_path.to_path_buf());

    // Resolve my_todo.json to an absolute path
    let json_path = path_to_file(todo_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(todo_path));

    let json_dir = json_path.parent().unwrap_or_else(|| Path::new(""));

    // Strip the JSON directory from the target path
    match abs_target.strip_prefix(json_dir) {
        Ok(relative_path) => relative_path.to_string_lossy().to_string(),
        Err(_) => abs_target.to_string_lossy().to_string(), // Fallback to absolute if outside project
    }
}

pub fn get_tasks(path: &Path) -> Vec<Task> {
    let path = path_to_file(path);
    return match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).expect("Error reading JSON."),
        Err(_) => Vec::new(),
    };
}

pub fn save_tasks(tasks: Vec<Task>, path: &Path) {
    if !path.exists()
        && let Some(parent) = path.parent()
    {
        println!("Creating directories...");
        let _ = fs::create_dir_all(parent);
    }
    let path = path_to_file(path);
    fs::write(
        path,
        serde_json::to_string_pretty(&tasks).expect("Error formatting JSON."),
    )
    .expect("Error writing to todo file.");
}

pub fn get_piped() -> Option<String> {
    let stdin = stdin();
    if !stdin.is_terminal() {
        let mut buffer = String::new();
        if stdin.lock().read_to_string(&mut buffer).is_ok() {
            // let line = buffer.lines().next().unwrap_or("").trim().to_string();
            // if !line.is_empty() {
            return Some(buffer);
            // }
        }
    }
    None
}
