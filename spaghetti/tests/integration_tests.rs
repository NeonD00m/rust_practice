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
    fn test_loc_counter_new_languages() {
        // Haskell
        let hs_code = "{- block -}\n-- line\nmain = putStrLn \"hi\"";
        let hs_metrics = count_lines(hs_code, Language::Haskell);
        assert_eq!(hs_metrics.comments, 2);
        assert_eq!(hs_metrics.sloc, 1);

        // HTML
        let html_code = "<!-- comment -->\n<div>Hello</div>";
        let html_metrics = count_lines(html_code, Language::HTML);
        assert_eq!(html_metrics.comments, 1);
        assert_eq!(html_metrics.sloc, 1);

        // PHP
        let php_code = "<?php\n// comment\n# hash comment\n/* block */\necho 'hi';";
        let php_metrics = count_lines(php_code, Language::PHP);
        assert_eq!(php_metrics.comments, 3);
        assert_eq!(php_metrics.sloc, 2);
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
    fn test_symbol_extraction_new_languages() {
        // Go
        let go_code = "type Server struct {}\ntype Handler interface {}\nfunc main() {}";
        let syms_go = extract_symbols(go_code, Language::Go);
        assert_eq!(syms_go.len(), 3);
        assert_eq!(syms_go[0].name, "Server");
        assert_eq!(syms_go[0].kind, SymbolKind::Struct);
        assert_eq!(syms_go[1].name, "Handler");
        assert_eq!(syms_go[1].kind, SymbolKind::Interface);
        assert_eq!(syms_go[2].name, "main");
        assert_eq!(syms_go[2].kind, SymbolKind::Function);

        // Haskell
        let hs_code = "data Tree = Leaf | Node\ntype Name = String\nclass Printable a";
        let syms_hs = extract_symbols(hs_code, Language::Haskell);
        assert_eq!(syms_hs.len(), 3);
        assert_eq!(syms_hs[0].name, "Tree");
        assert_eq!(syms_hs[0].kind, SymbolKind::Struct);
        assert_eq!(syms_hs[1].name, "Name");
        assert_eq!(syms_hs[1].kind, SymbolKind::Interface);
        assert_eq!(syms_hs[2].name, "Printable");
        assert_eq!(syms_hs[2].kind, SymbolKind::Trait);
    }

    #[test]
    fn test_custom_extensions() {
        use spaghetti::core::config::Config;
        let mut config = Config::default();
        config.custom_extensions.insert("proto".to_string());
        assert!(config.is_custom_ext("proto"));
        assert!(config.is_custom_ext("PROTO"));
        assert!(!config.is_custom_ext("graphql"));

        assert_eq!(Language::from_extension("php"), Some(Language::PHP));
        assert_eq!(Language::from_extension("hs"), Some(Language::Haskell));
        assert_eq!(Language::from_extension("html"), Some(Language::HTML));
        assert_eq!(Language::from_extension("css"), Some(Language::CSS));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("cs"), Some(Language::CSharp));
        assert_eq!(Language::from_extension("proto"), None);
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
