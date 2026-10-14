use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_python_imports(content: &str, current_file: &Path, project_root: &Path) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        if trimmed.starts_with("import ") {
            // import os, sys or import foo.bar
            let rest = trimmed["import ".len()..].trim();
            for item in rest.split(',') {
                let module = item.trim().split_whitespace().next().unwrap_or("").trim();
                if !module.is_empty() {
                    let resolved = resolve_py_module(module, current_file, project_root, 0);
                    let is_external = resolved.is_none();
                    edges.push(ImportEdge {
                        raw_specifier: module.to_string(),
                        resolved_path: resolved,
                        is_external,
                        line: line_num,
                    });
                }
            }
        } else if trimmed.starts_with("from ") {
            // from .foo import bar, from ..parent import bar, from foo.bar import baz
            let rest = trimmed["from ".len()..].trim();
            if let Some(import_pos) = rest.find(" import ") {
                let module = rest[..import_pos].trim();
                let leading_dots = module.chars().take_while(|&c| c == '.').count();
                let clean_mod = &module[leading_dots..];

                let resolved = resolve_py_module(clean_mod, current_file, project_root, leading_dots);
                let is_external = resolved.is_none() && leading_dots == 0;
                edges.push(ImportEdge {
                    raw_specifier: module.to_string(),
                    resolved_path: resolved,
                    is_external,
                    line: line_num,
                });
            }
        }
    }

    edges
}

fn resolve_py_module(
    module: &str,
    current_file: &Path,
    project_root: &Path,
    relative_level: usize,
) -> Option<PathBuf> {
    let base_dir = if relative_level > 0 {
        let mut dir = current_file.parent()?.to_path_buf();
        for _ in 1..relative_level {
            dir = dir.parent()?.to_path_buf();
        }
        dir
    } else {
        project_root.to_path_buf()
    };

    let mod_path = module.replace('.', "/");
    let candidates = [
        base_dir.join(format!("{}.py", mod_path)),
        base_dir.join(&mod_path).join("__init__.py"),
        base_dir.join(format!("{}.pyi", mod_path)),
        // check current file's parent as well for non-relative local imports
        current_file.parent().unwrap_or(project_root).join(format!("{}.py", mod_path)),
        current_file.parent().unwrap_or(project_root).join(&mod_path).join("__init__.py"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}
