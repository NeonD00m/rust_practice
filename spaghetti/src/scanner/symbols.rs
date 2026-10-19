use crate::core::types::{Language, Symbol, SymbolKind};

pub fn extract_symbols(content: &str, lang: Language) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        // Skip obvious comments
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("--")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }

        match lang {
            Language::Rust => {
                // pub struct Foo, struct Foo
                // pub enum Foo, enum Foo
                // pub trait Foo, trait Foo
                // fn foo, pub fn foo
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(pos) = words.iter().position(|&w| w == "struct") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Struct,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "enum") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Enum,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "trait") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Trait,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "fn") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_fn_ident(name);
                        if clean_name == "main" {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Function,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            Language::C | Language::Cpp => {
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(pos) = words.iter().position(|&w| w == "class") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() && clean_name != ";" {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Class,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "struct") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() && clean_name != ";" {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Struct,
                                line: line_num,
                            });
                        }
                    }
                } else if trimmed.contains("main(") {
                    symbols.push(Symbol {
                        name: "main".to_string(),
                        kind: SymbolKind::Function,
                        line: line_num,
                    });
                }
            }
            Language::Python => {
                if trimmed.starts_with("class ") {
                    let parts: Vec<&str> = trimmed["class ".len()..]
                        .split(|c| c == ':' || c == '(')
                        .collect();
                    if let Some(first) = parts.first() {
                        let clean_name = first.trim().to_string();
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Class,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            Language::JavaScript | Language::TypeScript => {
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(pos) = words.iter().position(|&w| w == "class") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Class,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "interface") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Interface,
                                line: line_num,
                            });
                        }
                    }
                } else if let Some(pos) = words.iter().position(|&w| w == "enum") {
                    if let Some(name) = words.get(pos + 1) {
                        let clean_name = clean_ident(name);
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Enum,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            Language::Lua | Language::Luau => {
                // Luau class/type Foo = ...
                if trimmed.starts_with("type ") {
                    let parts: Vec<&str> = trimmed["type ".len()..].split('=').collect();
                    if let Some(first) = parts.first() {
                        let clean_name = first.trim().to_string();
                        if !clean_name.is_empty() {
                            symbols.push(Symbol {
                                name: clean_name,
                                kind: SymbolKind::Interface,
                                line: line_num,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    symbols
}

fn clean_ident(s: &str) -> String {
    s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
        .to_string()
}

fn clean_fn_ident(s: &str) -> String {
    let raw = s.split('(').next().unwrap_or(s);
    clean_ident(raw)
}
