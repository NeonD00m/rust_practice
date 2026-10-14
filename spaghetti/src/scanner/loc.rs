use crate::core::types::{CodeMetrics, Language};

pub fn count_lines(content: &str, lang: Language) -> CodeMetrics {
    let mut metrics = CodeMetrics::default();
    let mut in_block_comment = false;

    for line in content.lines() {
        metrics.physical_loc += 1;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            metrics.blanks += 1;
            continue;
        }

        match lang {
            Language::C | Language::Cpp | Language::JavaScript | Language::TypeScript | Language::Rust => {
                if in_block_comment {
                    metrics.comments += 1;
                    if trimmed.contains("*/") {
                        in_block_comment = false;
                    }
                    continue;
                }

                if trimmed.starts_with("//") {
                    metrics.comments += 1;
                    continue;
                }

                if trimmed.starts_with("/*") {
                    metrics.comments += 1;
                    if !trimmed.contains("*/") {
                        in_block_comment = true;
                    }
                    continue;
                }

                metrics.sloc += 1;
            }
            Language::Python => {
                if in_block_comment {
                    metrics.comments += 1;
                    if trimmed.contains("\"\"\"") || trimmed.contains("'''") {
                        in_block_comment = false;
                    }
                    continue;
                }

                if trimmed.starts_with('#') {
                    metrics.comments += 1;
                    continue;
                }

                if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                    metrics.comments += 1;
                    let quote = &trimmed[..3];
                    if trimmed[3..].contains(quote) {
                        // closed on same line
                    } else {
                        in_block_comment = true;
                    }
                    continue;
                }

                metrics.sloc += 1;
            }
            Language::Lua | Language::Luau => {
                if in_block_comment {
                    metrics.comments += 1;
                    if trimmed.contains("]]") {
                        in_block_comment = false;
                    }
                    continue;
                }

                if trimmed.starts_with("--[[") {
                    metrics.comments += 1;
                    if !trimmed.contains("]]") {
                        in_block_comment = true;
                    }
                    continue;
                }

                if trimmed.starts_with("--") {
                    metrics.comments += 1;
                    continue;
                }

                metrics.sloc += 1;
            }
            Language::Other => {
                metrics.sloc += 1;
            }
        }
    }

    metrics
}
