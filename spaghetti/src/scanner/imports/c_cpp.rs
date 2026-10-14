use std::path::Path;
use crate::core::types::ImportEdge;

pub fn extract_c_cpp_imports(content: &str, current_file: &Path, _project_root: &Path) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("#include") {
            continue;
        }

        let line_num = idx + 1;
        let rest = trimmed["#include".len()..].trim();

        if rest.starts_with('"') {
            if let Some(end_idx) = rest[1..].find('"') {
                let specifier = &rest[1..1 + end_idx];
                let resolved = current_file.parent().map(|p| p.join(specifier));

                edges.push(ImportEdge {
                    raw_specifier: specifier.to_string(),
                    resolved_path: resolved,
                    is_external: false,
                    line: line_num,
                });
            }
        } else if rest.starts_with('<') {
            if let Some(end_idx) = rest[1..].find('>') {
                let specifier = &rest[1..1 + end_idx];
                // Local include paths or system include
                let local_candidate = current_file.parent().map(|p| p.join(specifier));
                let resolved = if let Some(ref cand) = local_candidate {
                    if cand.exists() {
                        local_candidate
                    } else {
                        None
                    }
                } else {
                    None
                };

                edges.push(ImportEdge {
                    raw_specifier: specifier.to_string(),
                    resolved_path: resolved,
                    is_external: true,
                    line: line_num,
                });
            }
        }
    }

    edges
}
