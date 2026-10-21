use file_id::{FileId, get_file_id};
use serde::Deserialize;
use serde::Serialize;
use std::{
    env, fs,
    io::{self, IsTerminal, Read, stdin},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TodoTarget {
    pub path: PathBuf,
    pub is_personal: bool,
    pub display_name: String,
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

pub fn value_flag(args: &[String], short_flag: &str, long_flag: &str) -> Option<(String, usize)> {
    args.iter()
        .position(|arg| arg == short_flag || arg == long_flag)
        .and_then(|pos| args.get(pos + 1).cloned().map(|s| (s, pos)))
}

pub fn personal_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join("my_todo.json"))
}

pub fn path_to_file(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(FILE_NAME)
    } else if path.file_name().map(|n| n == FILE_NAME).unwrap_or(false) {
        path.to_path_buf()
    } else if path.extension().is_none()
        || path.to_string_lossy().ends_with('/')
        || path.to_string_lossy().ends_with('\\')
    {
        path.join(FILE_NAME)
    } else {
        path.to_path_buf()
    }
}

pub fn make_target(path: PathBuf, is_personal: bool) -> TodoTarget {
    let file_path = path_to_file(&path);
    let display_name = if is_personal {
        "personal".to_string()
    } else if let Ok(current_dir) = env::current_dir() {
        if let Ok(rel) = file_path.strip_prefix(&current_dir) {
            let rel_str = rel.to_string_lossy().to_string();
            if rel_str.is_empty() {
                format!("./{}", FILE_NAME)
            } else {
                rel_str
            }
        } else {
            file_path.to_string_lossy().to_string()
        }
    } else {
        file_path.to_string_lossy().to_string()
    };

    TodoTarget {
        path: file_path,
        is_personal,
        display_name,
    }
}

pub fn discover_todo_files(start_path: &Path, max_depth: Option<usize>) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut visited_dirs = Vec::new();
    discover_recursive(start_path, 0, max_depth, &mut results, &mut visited_dirs);
    results.sort();
    results.dedup();
    results
}

fn discover_recursive(
    dir: &Path,
    current_depth: usize,
    max_depth: Option<usize>,
    results: &mut Vec<PathBuf>,
    visited_dirs: &mut Vec<PathBuf>,
) {
    if let Ok(canonical) = dir.canonicalize() {
        if visited_dirs.contains(&canonical) {
            return;
        }
        visited_dirs.push(canonical);
    }

    let candidate = dir.join(FILE_NAME);
    if candidate.is_file() {
        results.push(candidate);
    }

    if let Some(limit) = max_depth {
        if current_depth >= limit {
            return;
        }
    } else if current_depth >= 25 {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let child_path = entry.path();
            if child_path.is_dir() {
                let name = child_path.file_name().unwrap_or_default().to_string_lossy();
                if name.starts_with('.')
                    || name == "target"
                    || name == "node_modules"
                    || name == "vendor"
                    || name == "build"
                    || name == "dist"
                {
                    continue;
                }
                discover_recursive(
                    &child_path,
                    current_depth + 1,
                    max_depth,
                    results,
                    visited_dirs,
                );
            }
        }
    }
}

pub fn resolve_targets(args: &mut Vec<String>, is_discover_cmd: bool) -> Vec<TodoTarget> {
    let use_personal = find_arg_and_remove(args, "--personal", "--personal").is_some();
    let path_inputs = extract_flag_values(args, "-p", "--path");
    let discover_flag = find_arg_and_remove(args, "-d", "--discover").is_some();
    let depth_str = extract_flag_values(args, "--depth", "--depth").pop();
    let max_depth = depth_str.and_then(|s| s.parse::<usize>().ok());

    let do_discover = is_discover_cmd || discover_flag;

    let mut raw_targets: Vec<TodoTarget> = Vec::new();

    if do_discover {
        if path_inputs.is_empty() {
            let found = discover_todo_files(Path::new("."), max_depth);
            for p in found {
                raw_targets.push(make_target(p, false));
            }
        } else {
            for p_str in &path_inputs {
                let p = Path::new(p_str);
                if p.is_dir() {
                    let found = discover_todo_files(p, max_depth);
                    for f in found {
                        raw_targets.push(make_target(f, false));
                    }
                } else {
                    raw_targets.push(make_target(PathBuf::from(p_str), false));
                }
            }
        }
    } else if !path_inputs.is_empty() {
        for p_str in path_inputs {
            raw_targets.push(make_target(PathBuf::from(p_str), false));
        }
    }

    if use_personal {
        if let Some(personal) = personal_path() {
            raw_targets.push(make_target(personal, true));
        }
    }

    if raw_targets.is_empty() && !do_discover {
        raw_targets.push(make_target(find_file(), false));
    }

    let mut deduplicated: Vec<TodoTarget> = Vec::new();
    for target in raw_targets {
        let is_dup = deduplicated.iter().any(|existing| {
            if let (Ok(c1), Ok(c2)) = (target.path.canonicalize(), existing.path.canonicalize()) {
                c1 == c2
            } else {
                target.path == existing.path
            }
        });
        if !is_dup {
            deduplicated.push(target);
        }
    }

    deduplicated
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
                break;
            }

            // check to see if we find the target file
            if new_dir.join(FILE_NAME).exists() {
                return new_dir.join(FILE_NAME);
            }
            new_dir = match new_dir.parent() {
                Some(p) => p,
                None => {
                    break;
                }
            };
        }
    };
    default
}

pub fn get_relative_to_todo(target_path: &Path, todo_path: &Path) -> String {
    // Resolve target to an absolute path
    let abs_target = target_path
        .canonicalize()
        .unwrap_or_else(|_| target_path.to_path_buf());

    // Resolve my_todo.json to an absolute path
    let real_path = path_to_file(todo_path);
    let json_path = real_path.canonicalize().unwrap_or(real_path);

    let json_dir = json_path.parent().unwrap_or_else(|| Path::new(""));

    // Strip the JSON directory from the target path
    match abs_target.strip_prefix(json_dir) {
        Ok(relative_path) => relative_path.to_string_lossy().to_string(),
        Err(_) => abs_target.to_string_lossy().to_string(), // Fallback to absolute if outside project
    }
}

pub fn get_tasks(path: &Path) -> Vec<Task> {
    let path = path_to_file(path);
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).expect("Error reading JSON."),
        Err(_) => Vec::new(),
    }
}

pub fn save_tasks(tasks: Vec<Task>, path: &Path) {
    let path = path_to_file(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Error creating todo directories.");
    }
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
            if !buffer.trim().is_empty() {
                return Some(buffer);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_file_dir_vs_file() {
        let dir_path = Path::new("some_dir/");
        assert_eq!(path_to_file(dir_path), PathBuf::from("some_dir").join(FILE_NAME));

        let file_path = Path::new("custom_file.json");
        assert_eq!(path_to_file(file_path), PathBuf::from("custom_file.json"));

        let exact_todo = Path::new("my_todo.json");
        assert_eq!(path_to_file(exact_todo), PathBuf::from("my_todo.json"));
    }

    #[test]
    fn test_extract_and_remove_flags() {
        let mut args = vec![
            "todo".to_string(),
            "-p".to_string(),
            "path1".to_string(),
            "-p".to_string(),
            "path2".to_string(),
            "-v".to_string(),
            "list".to_string(),
        ];
        let paths = extract_flag_values(&mut args, "-p", "--path");
        assert_eq!(paths, vec!["path1", "path2"]);
        assert_eq!(find_arg_and_remove(&mut args, "-v", "--verbose"), Some(1));
        assert_eq!(args, vec!["todo", "list"]);
    }

    #[test]
    fn test_resolve_targets_fallback() {
        let mut args = vec!["todo".to_string(), "list".to_string()];
        let targets = resolve_targets(&mut args, false);
        assert_eq!(targets.len(), 1);
        assert!(!targets[0].is_personal);
    }
}
