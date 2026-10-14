use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use crate::core::config::Config;
use crate::core::types::FileNode;
use crate::graph::{DependencyGraph, RootDetector};
use super::columns::{format_with_commas, ColumnFormatter};

// --- Filesystem Tree ---

enum FsTreeNode {
    Dir(BTreeMap<String, FsTreeNode>),
    File(usize), // index into files
}

pub fn render_fs_tree(
    files: &[FileNode],
    ignored_count: usize,
    show_loc: bool,
    show_symbols: bool,
    config: &Config,
) {
    if files.is_empty() {
        println!("No code files found.");
        return;
    }

    let root_name = config
        .root_dir
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "project".into());

    println!("{}/", root_name);

    // Build hierarchy
    let mut root_node = BTreeMap::new();
    for (idx, file) in files.iter().enumerate() {
        insert_into_tree(&mut root_node, &file.rel_path, idx);
    }

    let mut total_sloc = 0;
    let mut total_types = 0;
    for file in files {
        total_sloc += file.metrics.sloc;
        total_types += file.symbols.len();
    }

    render_fs_node(&root_node, "", files, show_loc, show_symbols, config);

    // Footer summary
    println!();
    let file_label = if files.len() == 1 { "code file" } else { "code files" };
    if show_loc || show_symbols {
        println!(
            "Total: {} {} | {} SLOC | {} types defined",
            files.len(),
            file_label,
            format_with_commas(total_sloc),
            total_types
        );
    } else {
        println!(
            "{} {} found. Ignored {} non-code/ignored files.",
            files.len(),
            file_label,
            ignored_count
        );
    }
}

fn insert_into_tree(tree: &mut BTreeMap<String, FsTreeNode>, rel_path: &Path, file_idx: usize) {
    let components: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components.is_empty() {
        return;
    }

    let mut current = tree;
    for (i, comp) in components.iter().enumerate() {
        if i == components.len() - 1 {
            current.insert(comp.clone(), FsTreeNode::File(file_idx));
        } else {
            let entry = current
                .entry(comp.clone())
                .or_insert_with(|| FsTreeNode::Dir(BTreeMap::new()));
            if let FsTreeNode::Dir(ref mut sub) = entry {
                current = sub;
            } else {
                break;
            }
        }
    }
}

fn render_fs_node(
    tree: &BTreeMap<String, FsTreeNode>,
    prefix: &str,
    files: &[FileNode],
    show_loc: bool,
    show_symbols: bool,
    config: &Config,
) {
    let count = tree.len();
    for (idx, (name, node)) in tree.iter().enumerate() {
        let is_last = idx + 1 == count;
        let branch = if is_last { "└── " } else { "├── " };
        let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        match node {
            FsTreeNode::Dir(sub) => {
                println!("{}{}{}/", prefix, branch, name);
                render_fs_node(sub, &next_prefix, files, show_loc, show_symbols, config);
            }
            FsTreeNode::File(file_idx) => {
                let file = &files[*file_idx];
                let display_branch = format!("{}{}{}", prefix, branch, name);

                let loc_str = if show_loc {
                    Some(ColumnFormatter::format_loc(file.metrics.sloc))
                } else {
                    None
                };

                let sym_str = if show_symbols {
                    Some(ColumnFormatter::format_symbols(&file.symbols))
                } else {
                    None
                };

                let line = ColumnFormatter::format_tree_line(
                    &display_branch,
                    36,
                    loc_str.as_deref(),
                    sym_str.as_deref(),
                );

                println!("{}", line);
            }
        }
    }
}

// --- Dependency Tree ---

pub fn render_dep_tree(
    graph: &DependencyGraph,
    show_loc: bool,
    show_symbols: bool,
    config: &Config,
) {
    if graph.files.is_empty() {
        println!("No code files found.");
        return;
    }

    let roots = RootDetector::find_roots(graph);
    let root_idx = roots[0];
    let root_file = &graph.files[root_idx];

    let root_path_str = root_file.rel_path.to_string_lossy().replace('\\', "/");
    println!("Project Dependency Map (Root: {})", root_path_str);

    // Root line
    let loc_str = if show_loc {
        Some(ColumnFormatter::format_loc(root_file.metrics.sloc))
    } else {
        None
    };
    let sym_str = if show_symbols {
        Some(ColumnFormatter::format_symbols(&root_file.symbols))
    } else {
        None
    };

    let root_line = ColumnFormatter::format_tree_line(
        &root_path_str,
        36,
        loc_str.as_deref(),
        sym_str.as_deref(),
    );
    println!("{}", root_line);

    let mut visited_global = HashSet::new();
    visited_global.insert(root_idx);

    let mut current_path = Vec::new();
    current_path.push(root_idx);

    render_dep_children(
        root_idx,
        "",
        graph,
        &mut visited_global,
        &mut current_path,
        show_loc,
        show_symbols,
        config,
    );

    // Render any disjoint root components if present
    for &extra_root in roots.iter().skip(1) {
        if !visited_global.contains(&extra_root) {
            println!();
            let extra_file = &graph.files[extra_root];
            let extra_str = extra_file.rel_path.to_string_lossy().replace('\\', "/");
            println!("{}", extra_str);
            visited_global.insert(extra_root);
            current_path.clear();
            current_path.push(extra_root);

            render_dep_children(
                extra_root,
                "",
                graph,
                &mut visited_global,
                &mut current_path,
                show_loc,
                show_symbols,
                config,
            );
        }
    }

    println!();
    println!(
        "Found {} root entrypoint(s), {} files in dependency graph.",
        roots.len(),
        graph.files.len()
    );
}

fn render_dep_children(
    u: usize,
    prefix: &str,
    graph: &DependencyGraph,
    visited_global: &mut HashSet<usize>,
    current_path: &mut Vec<usize>,
    show_loc: bool,
    show_symbols: bool,
    config: &Config,
) {
    let children = &graph.adj[u];
    let count = children.len();

    for (idx, &v) in children.iter().enumerate() {
        let is_last = idx + 1 == count;
        let branch = if is_last { "└── " } else { "├── " };
        let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        let child_file = &graph.files[v];
        let file_path = child_file.rel_path.to_string_lossy().replace('\\', "/");
        let display_branch = format!("{}{}{}", prefix, branch, file_path);

        // Cycle check: is v currently on the active stack?
        if current_path.contains(&v) {
            let cycle_line = format!(
                "{} (cycle ⟲) [WARNING: -Wcycle]",
                display_branch
            );
            println!("{}", cycle_line);
            continue;
        }

        // Dedup check: was v already expanded in a previous branch?
        if visited_global.contains(&v) {
            let dedup_line = format!("{} (dedup ↷)", display_branch);
            println!("{}", dedup_line);
            continue;
        }

        let loc_str = if show_loc {
            Some(ColumnFormatter::format_loc(child_file.metrics.sloc))
        } else {
            None
        };
        let sym_str = if show_symbols {
            Some(ColumnFormatter::format_symbols(&child_file.symbols))
        } else {
            None
        };

        let line = ColumnFormatter::format_tree_line(
            &display_branch,
            36,
            loc_str.as_deref(),
            sym_str.as_deref(),
        );
        println!("{}", line);

        visited_global.insert(v);
        current_path.push(v);

        render_dep_children(
            v,
            &next_prefix,
            graph,
            visited_global,
            current_path,
            show_loc,
            show_symbols,
            config,
        );

        current_path.pop();
    }
}
