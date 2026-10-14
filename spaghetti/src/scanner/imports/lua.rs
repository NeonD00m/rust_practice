use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_lua_imports(content: &str, current_file: &Path, project_root: &Path) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        if trimmed.starts_with("--") {
            continue;
        }

        // require("module") or require 'module'
        if let Some(pos) = trimmed.find("require") {
            let rest = &trimmed[pos + "require".len()..];
            for quote in &['"', '\''] {
                if let Some(start) = rest.find(*quote) {
                    if let Some(end) = rest[start + 1..].find(*quote) {
                        let module = &rest[start + 1..start + 1 + end];
                        let resolved = resolve_lua_module(module, current_file, project_root);
                        let is_external = resolved.is_none();

                        edges.push(ImportEdge {
                            raw_specifier: module.to_string(),
                            resolved_path: resolved,
                            is_external,
                            line: line_num,
                        });
                        break;
                    }
                }
            }
        }
    }

    edges
}

fn resolve_lua_module(module: &str, current_file: &Path, project_root: &Path) -> Option<PathBuf> {
    let path_style = module.replace('.', "/");
    let current_dir = current_file.parent().unwrap_or(project_root);

    let candidates = [
        current_dir.join(format!("{}.lua", path_style)),
        current_dir.join(format!("{}.luau", path_style)),
        current_dir.join(&path_style).join("init.lua"),
        current_dir.join(&path_style).join("init.luau"),
        project_root.join(format!("{}.lua", path_style)),
        project_root.join(format!("{}.luau", path_style)),
        project_root.join(&path_style).join("init.lua"),
        project_root.join(&path_style).join("init.luau"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}
