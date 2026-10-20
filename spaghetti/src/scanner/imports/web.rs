use std::path::{Path, PathBuf};
use crate::core::types::{ImportEdge, Language};

pub fn extract_web_imports(
    content: &str,
    lang: Language,
    current_file: &Path,
    _project_root: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        match lang {
            Language::HTML => {
                // <script ... src="..." ...>
                // <link ... href="..." ...>
                if (trimmed.contains("<script") && trimmed.contains("src="))
                    || (trimmed.contains("<link") && trimmed.contains("href="))
                {
                    if let Some(target) = extract_attribute_val(trimmed, "src=")
                        .or_else(|| extract_attribute_val(trimmed, "href="))
                    {
                        if !target.starts_with("http://")
                            && !target.starts_with("https://")
                            && !target.starts_with("//")
                        {
                            let resolved = resolve_relative(&target, current_file);
                            let is_external = resolved.is_none();
                            edges.push(ImportEdge {
                                raw_specifier: target,
                                resolved_path: resolved,
                                is_external,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            Language::CSS => {
                // @import "style.css"; or @import url("style.css");
                if trimmed.starts_with("@import") {
                    if let Some(target) = extract_css_import_url(trimmed) {
                        if !target.starts_with("http://")
                            && !target.starts_with("https://")
                            && !target.starts_with("//")
                        {
                            let resolved = resolve_relative(&target, current_file);
                            let is_external = resolved.is_none();
                            edges.push(ImportEdge {
                                raw_specifier: target,
                                resolved_path: resolved,
                                is_external,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    edges
}

fn extract_attribute_val(line: &str, attr: &str) -> Option<String> {
    let pos = line.find(attr)?;
    let rest = &line[pos + attr.len()..];
    for quote in &['"', '\''] {
        if let Some(start) = rest.find(*quote) {
            if let Some(end) = rest[start + 1..].find(*quote) {
                return Some(rest[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn extract_css_import_url(line: &str) -> Option<String> {
    for quote in &['"', '\''] {
        if let Some(start) = line.find(*quote) {
            if let Some(end) = line[start + 1..].find(*quote) {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn resolve_relative(target: &str, current_file: &Path) -> Option<PathBuf> {
    let clean = target.split('?').next().unwrap_or(target).split('#').next().unwrap_or(target);
    if let Some(parent) = current_file.parent() {
        let cand = parent.join(clean);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}
