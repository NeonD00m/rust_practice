use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_php_imports(
    content: &str,
    current_file: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
            continue;
        }

        // require, require_once, include, include_once
        let is_include = trimmed.starts_with("require ")
            || trimmed.starts_with("require_once ")
            || trimmed.starts_with("include ")
            || trimmed.starts_with("include_once ")
            || trimmed.starts_with("require(")
            || trimmed.starts_with("require_once(")
            || trimmed.starts_with("include(")
            || trimmed.starts_with("include_once(");

        if is_include {
            if let Some(specifier) = extract_quoted_string(trimmed) {
                let resolved = resolve_php_path(&specifier, current_file, project_root);
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
    for quote in &['\'', '"'] {
        if let Some(start) = line.find(*quote) {
            if let Some(end) = line[start + 1..].find(*quote) {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn resolve_php_path(
    specifier: &str,
    current_file: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    if let Some(parent) = current_file.parent() {
        let local_cand = parent.join(specifier);
        if local_cand.is_file() {
            return Some(local_cand);
        }
    }

    let root_cand = project_root.join(specifier);
    if root_cand.is_file() {
        return Some(root_cand);
    }

    None
}
