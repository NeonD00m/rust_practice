# Contributing Language Support to Spaghetti

`spaghetti` is designed around the UNIX philosophy: fast, zero-bloat codebase topology and architectural analysis. It avoids heavy parser generators, AST runtimes, and regex compilation overhead in favor of fast, linear, zero-allocation token scanning.

Adding support for a new language in `spaghetti` is modular and structured into **three distinct tiers**:

```
┌────────────────────────────────────────────────────────┐
│ Tier 1: Basic Scanning, Tree Listing, SLOC & Palette   │ (~2 min)
├────────────────────────────────────────────────────────┤
│ Tier 2: Symbol Extraction (--symbols / --outline)      │ (~5 min)
├────────────────────────────────────────────────────────┤
│ Tier 3: Dependency & Import Resolution (--dep / diag)  │ (~15 min)
└────────────────────────────────────────────────────────┘
```

You can contribute any tier independently! For instance, adding Tier 1 alone immediately lets files be discovered in trees, line-counted, and piped to other CLI tools.

---

## Tier 1: Basic File Scanning & Tree Listing

This tier makes files discoverable, color-coded, counted in SLOC/comment rankings, and pipeable.

### 1. Register Language & File Extensions
**File:** `src/core/types.rs`

1. Add your language variant to the `Language` enum:
   ```rust
   pub enum Language {
       // ...
       Zig,
       Other,
   }
   ```
2. Return its display name in `Language::name`:
   ```rust
   impl Language {
       pub fn name(&self) -> &'static str {
           match self {
               // ...
               Language::Zig => "Zig",
               Language::Other => "Other",
           }
       }
   ```
3. Map file extension(s) in `Language::from_extension`:
   ```rust
       pub fn from_extension(ext: &str) -> Option<Self> {
           match ext.to_ascii_lowercase().as_str() {
               // ...
               "zig" | "zon" => Some(Language::Zig),
               _ => None,
           }
       }
   }
   ```

### 2. Configure Comment Syntax & SLOC Counting
**File:** `src/scanner/loc.rs`

Specify how line comments and block comments are recognized in `count_lines`:
```rust
match lang {
    // Languages with standard C-style // and /* */ comments
    Language::C
    | Language::Cpp
    | Language::Rust
    | Language::Zig => {
        if in_block_comment {
            metrics.comments += 1;
            if trimmed.contains("*/") { in_block_comment = false; }
            continue;
        }
        if trimmed.starts_with("//") {
            metrics.comments += 1;
            continue;
        }
        if trimmed.starts_with("/*") {
            metrics.comments += 1;
            if !trimmed.contains("*/") { in_block_comment = true; }
            continue;
        }
        metrics.sloc += 1;
    }
    // ...
}
```

### 3. Assign Colorblind-Safe Terminal Palette Color
**File:** `src/core/palette.rs`

Map your language to a color in `language_color`:
```rust
pub fn language_color(lang_name: &str) -> Color {
    match lang_name.to_ascii_lowercase().as_str() {
        "rust" => Color::AnsiValue(214), // Orange
        "c" | "cpp" | "go" => Color::AnsiValue(75), // Cyan
        "python" | "zig" => Color::AnsiValue(220), // Yellow
        "javascript" | "typescript" => Color::AnsiValue(33), // Blue
        // ...
        _ => Color::AnsiValue(72), // Bluish green
    }
}
```

### 4. Enable CLI Language Filtering
**File:** `src/main.rs`

Add language aliases to `-l / --lang` in `parse_flag`:
```rust
"-l" | "--lang" => {
    if let Some(val) = iter.next() {
        for lang_str in val.split(',') {
            match lang_str.trim().to_ascii_lowercase().as_str() {
                // ...
                "zig" => { config.languages.insert(Language::Zig); }
                _ => {}
            }
        }
    }
}
```
Update `print_help()` in `src/main.rs` to document your new language option in `-l, --lang`.

---

## Tier 2: Symbol Extraction (`--symbols` / `--outline`)

Extract top-level architectural definitions (structs, classes, traits, interfaces, enums, functions) for display in tree views and LOC rankings.

**File:** `src/scanner/symbols.rs`

Symbols use the `SymbolKind` enum:
- `SymbolKind::Struct` (`struct`)
- `SymbolKind::Class` (`class`)
- `SymbolKind::Trait` (`trait`)
- `SymbolKind::Interface` (`interface`)
- `SymbolKind::Enum` (`enum`)
- `SymbolKind::Function` (`fn`)

Add a branch to `extract_symbols`:
```rust
match lang {
    Language::Zig => {
        // Example: const Foo = struct { ... } or const Bar = enum { ... }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() >= 4 && words[0] == "const" && words[2] == "=" {
            let name = clean_ident(words[1]);
            if words[3].starts_with("struct") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Struct,
                    line: line_num,
                });
            } else if words[3].starts_with("enum") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Enum,
                    line: line_num,
                });
            }
        } else if trimmed.starts_with("pub fn main(") || trimmed.starts_with("fn main(") {
            symbols.push(Symbol {
                name: "main".to_string(),
                kind: SymbolKind::Function,
                line: line_num,
            });
        }
    }
    // ...
}
```

### Parsing Guidelines:
1. **Never use heavy dependencies:** Do not add regex or grammar parser crates.
2. **Handle comments:** `extract_symbols` already skips common comment prefixes automatically before dispatching to the match arm.
3. **Use sanitizers:** Use `clean_ident()` or `clean_fn_ident()` to strip trailing punctuation like `{`, `(`, or `;`.

---

## Tier 3: Dependency Graph & Import Resolution (`--dep` / `diag`)

Enables topology visualization, root entrypoint detection, circular dependency detection (`-Wcycle`), and dead file detection (`-Worphan`).

### 1. Create Module Extractor
**File:** `src/scanner/imports/<language>.rs`

Implement import extraction returning `Vec<ImportEdge>`:
```rust
use std::path::{Path, PathBuf};
use crate::core::types::ImportEdge;

pub fn extract_zig_imports(
    content: &str,
    current_file: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = idx + 1;

        // Skip comments
        if trimmed.starts_with("//") { continue; }

        // Match: @import("module")
        if let Some(pos) = trimmed.find("@import(") {
            let rest = &trimmed[pos + "@import(".len()..];
            if let Some(quote_char) = rest.chars().next().filter(|&c| c == '"') {
                if let Some(end) = rest[1..].find(quote_char) {
                    let specifier = &rest[1..1 + end];
                    let resolved = resolve_zig_module(specifier, current_file, project_root);
                    let is_external = resolved.is_none();

                    edges.push(ImportEdge {
                        raw_specifier: specifier.to_string(),
                        resolved_path: resolved,
                        is_external,
                        line: line_num,
                    });
                }
            }
        }
    }

    edges
}

fn resolve_zig_module(specifier: &str, current_file: &Path, project_root: &Path) -> Option<PathBuf> {
    // Relative import: @import("utils.zig") or @import("../common.zig")
    if specifier.ends_with(".zig") {
        if let Some(parent) = current_file.parent() {
            let candidate = parent.join(specifier);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // Root-relative candidate
    let root_cand = project_root.join(specifier);
    if root_cand.is_file() {
        return Some(root_cand);
    }

    None // External package (e.g., @import("std"))
}
```

### 2. Register Extractor in Scanner
**File:** `src/scanner/imports/mod.rs`

1. Expose the submodule:
   ```rust
   pub mod zig;
   ```
2. Call it in `extract_imports`:
   ```rust
   pub fn extract_imports(
       content: &str,
       lang: Language,
       current_file: &Path,
       project_root: &Path,
   ) -> Vec<ImportEdge> {
       match lang {
           // ...
           Language::Zig => zig::extract_zig_imports(content, current_file, project_root),
           _ => Vec::new(),
       }
   }
   ```

---

## Ad-Hoc / Custom Unsupported Extensions

If you just want `spaghetti` to track custom or non-standard file extensions in your project without editing source code:

Use the `--include` flag:
```bash
spaghetti tree --include proto,graphql,zig
```

- Any matched file is categorized as `Language::Other`.
- All non-empty lines are counted as SLOC.
- Files appear in `tree`, `loc`, `print`, and JSON output.

---

## Testing Your Changes

Add unit and integration tests in `tests/integration_tests.rs`:
```rust
#[test]
fn test_loc_counter_zig() {
    let code = "// comment\nconst x: i32 = 42;\n";
    let metrics = count_lines(code, Language::Zig);
    assert_eq!(metrics.comments, 1);
    assert_eq!(metrics.sloc, 1);
}
```

Run test suite:
```bash
cargo test
```
