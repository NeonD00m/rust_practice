pub mod gitignore;
pub mod imports;
pub mod loc;
pub mod symbols;

use std::fs;
use std::path::Path;
use crate::core::config::Config;
use crate::core::types::{FileNode, Language};
use gitignore::GitIgnore;

pub struct ScanResult {
    pub files: Vec<FileNode>,
    pub ignored_count: usize,
}

pub fn scan_project(config: &Config) -> Result<ScanResult, std::io::Error> {
    let mut files = Vec::new();
    let mut ignored_count = 0;

    let gitignore = if config.no_ignore {
        GitIgnore::default()
    } else {
        GitIgnore::load(&config.root_dir)
    };

    scan_dir_recursive(
        &config.root_dir,
        &config.root_dir,
        0,
        config,
        &gitignore,
        &mut files,
        &mut ignored_count,
    )?;

    // Sort files deterministically by relative path
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    Ok(ScanResult {
        files,
        ignored_count,
    })
}

fn scan_dir_recursive(
    dir: &Path,
    root: &Path,
    current_depth: usize,
    config: &Config,
    gitignore: &GitIgnore,
    files: &mut Vec<FileNode>,
    ignored_count: &mut usize,
) -> Result<(), std::io::Error> {
    if let Some(max_depth) = config.max_depth {
        if current_depth > max_depth {
            return Ok(());
        }
    }

    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        let rel_path = match path.strip_prefix(root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => path.clone(),
        };

        if file_type.is_dir() {
            if !config.no_ignore && gitignore.is_ignored(&rel_path, true) {
                *ignored_count += 1;
                continue;
            }

            scan_dir_recursive(
                &path,
                root,
                current_depth + 1,
                config,
                gitignore,
                files,
                ignored_count,
            )?;
        } else if file_type.is_file() {
            if !config.no_ignore && gitignore.is_ignored(&rel_path, false) {
                *ignored_count += 1;
                continue;
            }

            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            // Language detection
            let lang = Language::from_extension(&ext);
            if let Some(lang) = lang {
                if !config.is_language_enabled(lang) || !config.is_ext_enabled(&ext) {
                    *ignored_count += 1;
                    continue;
                }

                // Read file content and extract metrics, symbols, and imports
                if let Ok(content) = fs::read_to_string(&path) {
                    let metrics = loc::count_lines(&content, lang);
                    let symbols = symbols::extract_symbols(&content, lang);
                    let imports = imports::extract_imports(&content, lang, &path, root);

                    files.push(FileNode {
                        path: path.clone(),
                        rel_path,
                        language: lang,
                        metrics,
                        symbols,
                        imports,
                    });
                }
            } else {
                // Non-code file (e.g. .json, .md, .lock, .png)
                *ignored_count += 1;
            }
        }
    }

    Ok(())
}
