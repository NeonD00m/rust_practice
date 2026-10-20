use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_go_imports(
    content: &str,
    current_file: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();
    let mut in_import_block = false;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        if in_import_block {
            if trimmed.starts_with(')') {
                in_import_block = false;
                continue;
            }
            if let Some(specifier) = extract_quoted_string(trimmed) {
                let resolved = resolve_go_import(&specifier, current_file, project_root);
                let is_external = resolved.is_none();
                edges.push(ImportEdge {
                    raw_specifier: specifier,
                    resolved_path: resolved,
                    is_external,
                    line: line_num,
                });
            }
            continue;
        }

        if trimmed.starts_with("import (") || trimmed == "import (" {
            in_import_block = true;
            continue;
        }

        if trimmed.starts_with("import ") {
            let rest = trimmed["import ".len()..].trim();
            if let Some(specifier) = extract_quoted_string(rest) {
                let resolved = resolve_go_import(&specifier, current_file, project_root);
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

fn extract_quoted_string(line: &str) -> Option<String> {
    for quote in &['"', '`'] {
        if let Some(start) = line.find(*quote) {
            if let Some(end) = line[start + 1..].find(*quote) {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn resolve_go_import(
    specifier: &str,
    current_file: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    // Relative imports: ./foo or ../foo
    if specifier.starts_with("./") || specifier.starts_with("../") {
        if let Some(parent) = current_file.parent() {
            let candidate_dir = parent.join(specifier);
            if candidate_dir.is_dir() {
                return Some(candidate_dir);
            }
            let candidate_file = parent.join(format!("{}.go", specifier));
            if candidate_file.is_file() {
                return Some(candidate_file);
            }
        }
    }

    // Local module paths relative to project root
    let root_cand = project_root.join(specifier);
    if root_cand.is_dir() {
        return Some(root_cand);
    }
    let root_file = project_root.join(format!("{}.go", specifier));
    if root_file.is_file() {
        return Some(root_file);
    }

    None
}
