use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_js_ts_imports(content: &str, current_file: &Path, _project_root: &Path) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // import ... from '...' or import '...'
        // export ... from '...'
        // require('...')
        // import('...')
        if let Some(specifier) = extract_string_literal(trimmed) {
            let is_relative = specifier.starts_with('.') || specifier.starts_with('/');
            let resolved = if is_relative {
                resolve_js_ts_module(&specifier, current_file)
            } else {
                None
            };

            let is_external = !is_relative || resolved.is_none();

            edges.push(ImportEdge {
                raw_specifier: specifier,
                resolved_path: resolved,
                is_external,
                line: line_num,
            });
        }
    }

    edges
}

fn extract_string_literal(line: &str) -> Option<String> {
    let is_import_or_require = line.contains("import") || line.contains("require(") || line.contains("export ");
    if !is_import_or_require {
        return None;
    }

    for quote in &['\'', '"', '`'] {
        if let Some(first) = line.find(*quote) {
            if let Some(last) = line[first + 1..].find(*quote) {
                let candidate = &line[first + 1..first + 1 + last];
                // basic verification that this isn't random string
                if line.contains("from") || line.contains("require") || line.starts_with("import") || line.contains("export") {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

fn resolve_js_ts_module(specifier: &str, current_file: &Path) -> Option<PathBuf> {
    let parent = current_file.parent()?;
    let direct = parent.join(specifier);

    if direct.is_file() {
        return Some(direct);
    }

    let extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];
    for ext in &extensions {
        let with_ext = parent.join(format!("{}.{}", specifier, ext));
        if with_ext.is_file() {
            return Some(with_ext);
        }
    }

    // Index files in directories
    for ext in &extensions {
        let index_file = direct.join(format!("index.{}", ext));
        if index_file.is_file() {
            return Some(index_file);
        }
    }

    None
}
