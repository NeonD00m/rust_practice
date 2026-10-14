use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct GitIgnore {
    patterns: Vec<IgnorePattern>,
}

#[derive(Debug, Clone)]
struct IgnorePattern {
    pattern: String,
    negated: bool,
    dir_only: bool,
}

impl GitIgnore {
    pub fn load(root: &Path) -> Self {
        let mut patterns = Vec::new();

        // Default ignored directories always skipped unless --no-ignore
        for default_dir in &[
            "target",
            "node_modules",
            ".git",
            ".venv",
            "venv",
            "dist",
            "build",
            ".idea",
            ".vscode",
            ".gemini",
        ] {
            patterns.push(IgnorePattern {
                pattern: default_dir.to_string(),
                negated: false,
                dir_only: true,
            });
        }

        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = fs::read_to_string(&gitignore_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let negated = trimmed.starts_with('!');
                    let clean = if negated { &trimmed[1..] } else { trimmed };
                    let dir_only = clean.ends_with('/');
                    let pattern = clean.trim_matches('/').replace('\\', "/");

                    patterns.push(IgnorePattern {
                        pattern,
                        negated,
                        dir_only,
                    });
                }
            }
        }

        Self { patterns }
    }

    pub fn is_ignored(&self, rel_path: &Path, is_dir: bool) -> bool {
        let normalized = rel_path.to_string_lossy().replace('\\', "/");
        let components: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

        let mut ignored = false;

        for rule in &self.patterns {
            if rule.dir_only && !is_dir {
                continue;
            }

            let matches = if rule.pattern.contains('/') {
                // Exact or prefix path match
                normalized == rule.pattern || normalized.starts_with(&(rule.pattern.clone() + "/"))
            } else {
                // Component name match or glob wildcard
                components.iter().any(|c| simple_wildcard_match(c, &rule.pattern))
            };

            if matches {
                ignored = !rule.negated;
            }
        }

        ignored
    }
}

fn simple_wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return text.starts_with(prefix);
    }
    text == pattern
}
