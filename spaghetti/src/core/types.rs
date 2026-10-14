use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    C,
    Cpp,
    Python,
    JavaScript,
    TypeScript,
    Lua,
    Luau,
    Rust,
    Other,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Lua => "Lua",
            Language::Luau => "Luau",
            Language::Rust => "Rust",
            Language::Other => "Other",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "c" | "h" => Some(Language::C),
            "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "inl" => Some(Language::Cpp),
            "py" | "pyi" => Some(Language::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" | "tsx" => Some(Language::TypeScript),
            "lua" => Some(Language::Lua),
            "luau" => Some(Language::Luau),
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Struct,
    Class,
    Trait,
    Interface,
    Enum,
    Function,
}

impl SymbolKind {
    pub fn badge(&self) -> &'static str {
        match self {
            SymbolKind::Struct => "struct",
            SymbolKind::Class => "class",
            SymbolKind::Trait => "trait",
            SymbolKind::Interface => "interface",
            SymbolKind::Enum => "enum",
            SymbolKind::Function => "fn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub physical_loc: usize,
    pub sloc: usize,
    pub comments: usize,
    pub blanks: usize,
}

impl CodeMetrics {
    pub fn comment_ratio(&self) -> f64 {
        if self.physical_loc == 0 {
            0.0
        } else {
            (self.comments as f64) / (self.physical_loc as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEdge {
    pub raw_specifier: String,
    pub resolved_path: Option<PathBuf>,
    pub is_external: bool,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub language: Language,
    pub metrics: CodeMetrics,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticWarning {
    pub tag: String,
    pub message: String,
    pub file: PathBuf,
    pub related_files: Vec<PathBuf>,
    pub level: WarningLevel,
}
