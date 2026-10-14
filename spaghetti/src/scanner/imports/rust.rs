use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_rust_imports(content: &str, current_file: &Path, project_root: &Path) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // 1. mod foo; or pub mod foo;
        if let Some(mod_name) = parse_mod_declaration(trimmed) {
            let resolved = resolve_rust_mod(&mod_name, current_file);
            let is_external = resolved.is_none();
            edges.push(ImportEdge {
                raw_specifier: format!("mod {}", mod_name),
                resolved_path: resolved,
                is_external,
                line: line_num,
            });
            continue;
        }

        // 2. use crate::path; or use super::path; or use self::path;
        if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
            if let Some(specifier) = parse_use_statement(trimmed) {
                let resolved = resolve_rust_use(&specifier, current_file, project_root);
                let is_external = resolved.is_none();
                edges.push(ImportEdge {
                    raw_specifier: specifier,
                    resolved_path: resolved,
                    is_external,
                    line: line_num,
                });
            }
        }
    }

    edges
}

fn parse_mod_declaration(line: &str) -> Option<String> {
    let cleaned = line.trim_end_matches(';').trim();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    if words[0] == "mod" && words.len() == 2 {
        return Some(words[1].to_string());
    }
    if words.len() >= 3 && words[0] == "pub" && words[1] == "mod" {
        return Some(words[2].to_string());
    }
    None
}

fn parse_use_statement(line: &str) -> Option<String> {
    let clean = line.trim_end_matches(';').trim();
    let words: Vec<&str> = clean.split_whitespace().collect();
    let pos = words.iter().position(|&w| w == "use")?;
    let path_part = words.get(pos + 1)?;
    Some(path_part.to_string())
}

fn resolve_rust_mod(mod_name: &str, current_file: &Path) -> Option<PathBuf> {
    let current_dir = current_file.parent()?;
    let stem = current_file.file_stem()?.to_string_lossy();

    if stem == "mod" || stem == "main" || stem == "lib" {
        let file_cand = current_dir.join(format!("{}.rs", mod_name));
        if file_cand.is_file() {
            return Some(file_cand);
        }
        let dir_cand = current_dir.join(mod_name).join("mod.rs");
        if dir_cand.is_file() {
            return Some(dir_cand);
        }
    } else {
        // e.g. scanner.rs declaring mod gitignore; looks in scanner/gitignore.rs
        let sub_dir = current_dir.join(&*stem);
        let file_cand = sub_dir.join(format!("{}.rs", mod_name));
        if file_cand.is_file() {
            return Some(file_cand);
        }
        let dir_cand = sub_dir.join(mod_name).join("mod.rs");
        if dir_cand.is_file() {
            return Some(dir_cand);
        }
        // or sibling file_cand
        let sib_cand = current_dir.join(format!("{}.rs", mod_name));
        if sib_cand.is_file() {
            return Some(sib_cand);
        }
    }

    None
}

fn resolve_rust_use(specifier: &str, current_file: &Path, project_root: &Path) -> Option<PathBuf> {
    let first_part = specifier.split("::").next()?;

    if specifier.starts_with("crate::") {
        let sub = &specifier["crate::".len()..];
        let segments: Vec<&str> = sub.split("::").collect();
        if segments.is_empty() {
            return None;
        }

        // Find crate root (src/lib.rs or src/main.rs)
        let src_dir = find_src_dir(current_file, project_root)?;
        return resolve_in_src_dir(&segments, &src_dir);
    } else if specifier.starts_with("super::") {
        let parent = current_file.parent()?.parent()?;
        let sub = &specifier["super::".len()..];
        let segments: Vec<&str> = sub.split("::").collect();
        return resolve_in_src_dir(&segments, parent);
    }

    // external crate dependency
    if first_part != "std" && first_part != "core" && first_part != "alloc" {
        // Check if there is a local module matching first_part
        if let Some(src_dir) = find_src_dir(current_file, project_root) {
            let direct = src_dir.join(format!("{}.rs", first_part));
            if direct.is_file() {
                return Some(direct);
            }
        }
    }

    None
}

fn find_src_dir(file: &Path, _project_root: &Path) -> Option<PathBuf> {
    let mut curr = file.parent();
    while let Some(dir) = curr {
        if dir.file_name().map_or(false, |name| name == "src") {
            return Some(dir.to_path_buf());
        }
        curr = dir.parent();
    }
    None
}

fn resolve_in_src_dir(segments: &[&str], base_dir: &Path) -> Option<PathBuf> {
    let first = segments.first()?;
    let as_file = base_dir.join(format!("{}.rs", first));
    if as_file.is_file() {
        return Some(as_file);
    }
    let as_mod = base_dir.join(first).join("mod.rs");
    if as_mod.is_file() {
        return Some(as_mod);
    }
    None
}
