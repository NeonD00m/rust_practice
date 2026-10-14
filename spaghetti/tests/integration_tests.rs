#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use spaghetti::core::types::{CodeMetrics, Language, SymbolKind, FileNode};
    use spaghetti::scanner::loc::count_lines;
    use spaghetti::scanner::symbols::extract_symbols;
    use spaghetti::graph::dependency::DependencyGraph;
    use spaghetti::graph::cycles::CycleDetector;

    #[test]
    fn test_loc_counter_rust() {
        let code = r#"
        // Line 1 is a comment
        /* Block comment */
        fn hello() {
            println!("Hello world");
        }
        "#;
        let metrics = count_lines(code, Language::Rust);
        assert_eq!(metrics.comments, 2);
        assert_eq!(metrics.sloc, 3);
    }

    #[test]
    fn test_symbol_extraction() {
        let rust_code = r#"
        pub struct MyEngine {
            val: usize,
        }
        pub enum Mode { Active, Inactive }
        pub trait Worker {}
        fn main() {}
        "#;
        let syms = extract_symbols(rust_code, Language::Rust);
        assert_eq!(syms.len(), 4);
        assert_eq!(syms[0].name, "MyEngine");
        assert_eq!(syms[0].kind, SymbolKind::Struct);
        assert_eq!(syms[1].name, "Mode");
        assert_eq!(syms[1].kind, SymbolKind::Enum);
        assert_eq!(syms[2].name, "Worker");
        assert_eq!(syms[2].kind, SymbolKind::Trait);
        assert_eq!(syms[3].name, "main");
        assert_eq!(syms[3].kind, SymbolKind::Function);
    }

    #[test]
    fn test_cycle_detection() {
        // Construct 2 synthetic nodes with A -> B and B -> A
        let node_a = FileNode {
            path: PathBuf::from("a.rs"),
            rel_path: PathBuf::from("a.rs"),
            language: Language::Rust,
            metrics: CodeMetrics::default(),
            symbols: vec![],
            imports: vec![spaghetti::core::types::ImportEdge {
                raw_specifier: "b".into(),
                resolved_path: Some(PathBuf::from("b.rs")),
                is_external: false,
                line: 1,
            }],
        };
        let node_b = FileNode {
            path: PathBuf::from("b.rs"),
            rel_path: PathBuf::from("b.rs"),
            language: Language::Rust,
            metrics: CodeMetrics::default(),
            symbols: vec![],
            imports: vec![spaghetti::core::types::ImportEdge {
                raw_specifier: "a".into(),
                resolved_path: Some(PathBuf::from("a.rs")),
                is_external: false,
                line: 1,
            }],
        };

        let graph = DependencyGraph::build(vec![node_a, node_b]);
        let detector = CycleDetector::find_cycles(&graph);
        assert!(!detector.cycles.is_empty(), "Should detect cycle between a and b");
    }
}
