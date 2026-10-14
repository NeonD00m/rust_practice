mod core;
mod graph;
mod mcp;
mod render;
mod scanner;

use std::env;
use std::path::PathBuf;
use std::process::exit;

use core::config::Config;
use core::types::Language;
use graph::DependencyGraph;
use scanner::scan_project;

fn print_help() {
    println!(
        r#"spaghetti — Zero-Bloat Codebase Topology & Architectural Analyzer

USAGE:
    spaghetti [COMMAND] [PATH] [FLAGS]

COMMANDS:
    t, tree     Renders code filesystem tree (or dependency tree with --dep)
    l, loc      Language-grouped or flat LOC ranking view
    d, diag     Compiler-style architectural diagnostics (-Wall, -Worphan, -Wcycle, -Wgod, -Wdoc)
    p, print    Fast, unadorned code file listing (pipeable)
    help, -h    Print this help message

GLOBAL FLAGS:
    --dep               Toggle logical import dependency graph mode in 'tree'
    --loc               Display physical SLOC in aligned column
    --outline, --symbols Display extracted structs/classes/traits/interfaces
    --flat              Flatten 'loc' ranking into global table
    --limit <N>         Limit number of files printed in 'loc'
    --no-ignore         Scan everything ignoring .gitignore rules
    -l, --lang <LANG>   Filter by language (c, cpp, py, js, ts, lua, luau, rs)
    -e, --ext <EXT>     Filter by extension (e.g. -e ts,tsx)
    -d, --depth <N>     Maximum recursion depth
    --color-distinct    Enable colorblind-safe distinct palette
    --raw               Emit unformatted, pipeable lines (for todo, palmtree)
    --json              Emit structured JSON (for MCP tools)
    --no-color          Disable ANSI colors

DIAGNOSTIC FLAGS:
    -Wall               Enable all warnings (-Worphan, -Wcycle, -Wgod, -Wdoc)
    -Werror             Treat warnings as errors
    -Worphan / -Wno-orphan
    -Wcycle / -Wno-cycle
    -Wgod / -Wno-god
    -Wdoc / -Wno-doc
    --god-loc <N>       God-file SLOC threshold (default: 1000)
    --doc-ratio <N>     Doc comment ratio threshold in % (default: 5)
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        print_help();
        return;
    }

    let mut config = Config::default();
    let mut show_dep = false;
    let mut show_loc = false;
    let mut show_symbols = false;
    let mut flat = false;
    let mut limit = None;

    let cmd;
    let mut positional_path: Option<PathBuf> = None;

    let mut iter = args.into_iter();
    let first = iter.next().unwrap();

    match first.as_str() {
        "t" | "tree" => cmd = "tree".to_string(),
        "l" | "loc" => cmd = "loc".to_string(),
        "d" | "diag" => cmd = "diag".to_string(),
        "p" | "print" => cmd = "print".to_string(),
        other => {
            cmd = "tree".to_string();
            if other.starts_with('-') {
                parse_flag(other, &mut iter, &mut config, &mut show_dep, &mut show_loc, &mut show_symbols, &mut flat, &mut limit);
            } else {
                positional_path = Some(PathBuf::from(other));
            }
        }
    }

    while let Some(arg) = iter.next() {
        if arg.starts_with('-') {
            parse_flag(&arg, &mut iter, &mut config, &mut show_dep, &mut show_loc, &mut show_symbols, &mut flat, &mut limit);
        } else if positional_path.is_none() {
            positional_path = Some(PathBuf::from(arg));
        }
    }

    if let Some(p) = positional_path {
        config.root_dir = p;
    }

    // Execute scanning
    let scan_result = match scan_project(&config) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            exit(1);
        }
    };

    let needs_graph = show_dep || cmd == "diag";
    let graph = if needs_graph {
        Some(DependencyGraph::build(scan_result.files.clone()))
    } else {
        None
    };

    // JSON output branch
    if config.json {
        if cmd == "diag" {
            let g = graph.as_ref().unwrap();
            println!("{}", mcp::McpApi::diag_json(g, &config));
        } else {
            println!("{}", mcp::McpApi::tree_json(&scan_result, graph.as_ref()));
        }
        return;
    }

    // Raw piping branch
    if config.raw {
        match cmd.as_str() {
            "diag" => {
                let g = graph.as_ref().unwrap();
                render::render_raw_diagnostics(g, &config);
            }
            "loc" => {
                render::render_raw_loc(&scan_result.files, limit);
            }
            _ => {
                render::render_raw_files(&scan_result.files);
            }
        }
        return;
    }

    // Interactive / terminal presentation branch
    match cmd.as_str() {
        "tree" => {
            if show_dep {
                let g = graph.as_ref().unwrap();
                render::render_dep_tree(g, show_loc, show_symbols, &config);
            } else {
                render::render_fs_tree(
                    &scan_result.files,
                    scan_result.ignored_count,
                    show_loc,
                    show_symbols,
                    &config,
                );
            }
        }
        "loc" => {
            render::render_loc_rank(&scan_result.files, flat, show_symbols, limit, &config);
        }
        "diag" => {
            let g = graph.as_ref().unwrap();
            let success = render::render_diagnostics(g, &config);
            if !success && config.warn_error {
                exit(1);
            }
        }
        "print" => {
            render::render_raw_files(&scan_result.files);
        }
        _ => {
            print_help();
        }
    }
}

fn parse_flag<I>(
    flag: &str,
    iter: &mut I,
    config: &mut Config,
    show_dep: &mut bool,
    show_loc: &mut bool,
    show_symbols: &mut bool,
    flat: &mut bool,
    limit: &mut Option<usize>,
) where
    I: Iterator<Item = String>,
{
    match flag {
        "--dep" => *show_dep = true,
        "--loc" => *show_loc = true,
        "--outline" | "--symbols" => *show_symbols = true,
        "--flat" => *flat = true,
        "--no-ignore" => config.no_ignore = true,
        "--color-distinct" => config.color_distinct = true,
        "--no-color" => config.no_color = true,
        "--raw" => config.raw = true,
        "--json" => config.json = true,
        "--limit" => {
            if let Some(val) = iter.next() {
                *limit = val.parse().ok();
            }
        }
        "-l" | "--lang" => {
            if let Some(val) = iter.next() {
                for lang_str in val.split(',') {
                    match lang_str.trim().to_ascii_lowercase().as_str() {
                        "c" => { config.languages.insert(Language::C); }
                        "cpp" | "c++" => { config.languages.insert(Language::Cpp); }
                        "py" | "python" => { config.languages.insert(Language::Python); }
                        "js" | "javascript" => { config.languages.insert(Language::JavaScript); }
                        "ts" | "typescript" => { config.languages.insert(Language::TypeScript); }
                        "lua" => { config.languages.insert(Language::Lua); }
                        "luau" => { config.languages.insert(Language::Luau); }
                        "rs" | "rust" => { config.languages.insert(Language::Rust); }
                        _ => {}
                    }
                }
            }
        }
        "-e" | "--ext" => {
            if let Some(val) = iter.next() {
                for ext in val.split(',') {
                    config.extensions.insert(ext.trim().to_ascii_lowercase());
                }
            }
        }
        "-d" | "--depth" => {
            if let Some(val) = iter.next() {
                config.max_depth = val.parse().ok();
            }
        }
        "-Wall" => {
            config.warn_orphan = true;
            config.warn_cycle = true;
            config.warn_god = true;
            config.warn_doc = true;
        }
        "-Werror" => config.warn_error = true,
        "-Worphan" => config.warn_orphan = true,
        "-Wno-orphan" => config.warn_orphan = false,
        "-Wcycle" => config.warn_cycle = true,
        "-Wno-cycle" => config.warn_cycle = false,
        "-Wgod" => config.warn_god = true,
        "-Wno-god" => config.warn_god = false,
        "-Wdoc" => config.warn_doc = true,
        "-Wno-doc" => config.warn_doc = false,
        "--god-loc" => {
            if let Some(val) = iter.next() {
                if let Ok(num) = val.parse() {
                    config.god_loc_threshold = num;
                }
            }
        }
        "--doc-ratio" => {
            if let Some(val) = iter.next() {
                if let Ok(num) = val.parse() {
                    config.doc_ratio_threshold = num;
                }
            }
        }
        _ => {}
    }
}
