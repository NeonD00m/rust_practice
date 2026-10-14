use crate::core::config::Config;
use crate::core::types::{DiagnosticWarning, WarningLevel};
use crate::graph::{CycleDetector, DependencyGraph};

pub fn collect_diagnostics(
    graph: &DependencyGraph,
    config: &Config,
) -> Vec<DiagnosticWarning> {
    let mut diags = Vec::new();

    // 1. -Wcycle: Circular Dependencies
    if config.warn_cycle {
        let detector = CycleDetector::find_cycles(graph);
        for cycle in detector.cycles {
            if cycle.len() >= 2 {
                let mut path_names = Vec::new();
                for &node_idx in &cycle {
                    path_names.push(graph.files[node_idx].rel_path.to_string_lossy().replace('\\', "/"));
                }
                let cycle_str = path_names.join(" -> ");
                let file = graph.files[cycle[0]].path.clone();
                let related = cycle.iter().skip(1).map(|&i| graph.files[i].path.clone()).collect();

                diags.push(DiagnosticWarning {
                    tag: "-Wcycle".into(),
                    message: format!("{} ({} files in cycle loop)", cycle_str, cycle.len()),
                    file,
                    related_files: related,
                    level: if config.warn_error { WarningLevel::Error } else { WarningLevel::Warning },
                });
            }
        }
    }

    // 2. -Worphan: Internal source files with 0 incoming imports that are not entry points
    if config.warn_orphan {
        for (idx, file) in graph.files.iter().enumerate() {
            let in_deg = graph.in_degree(idx);
            let path_str = file.rel_path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
            let is_likely_entry = path_str.contains("main.")
                || path_str.contains("index.")
                || path_str.contains("lib.")
                || path_str.contains("app.")
                || path_str.contains("mod.");

            if in_deg == 0 && !is_likely_entry {
                diags.push(DiagnosticWarning {
                    tag: "-Worphan".into(),
                    message: "0 incoming imports, not an obvious entrypoint".into(),
                    file: file.path.clone(),
                    related_files: Vec::new(),
                    level: if config.warn_error { WarningLevel::Error } else { WarningLevel::Warning },
                });
            }
        }
    }

    // 3. -Wgod: God files exceeding SLOC threshold
    if config.warn_god {
        for file in &graph.files {
            if file.metrics.sloc > config.god_loc_threshold {
                diags.push(DiagnosticWarning {
                    tag: "-Wgod".into(),
                    message: format!(
                        "{} SLOC, {} imports (exceeds threshold of {})",
                        file.metrics.sloc,
                        file.imports.len(),
                        config.god_loc_threshold
                    ),
                    file: file.path.clone(),
                    related_files: Vec::new(),
                    level: if config.warn_error { WarningLevel::Error } else { WarningLevel::Warning },
                });
            }
        }
    }

    // 4. -Wdoc: Under-documented files
    if config.warn_doc {
        for file in &graph.files {
            let ratio = file.metrics.comment_ratio();
            if ratio < config.doc_ratio_threshold && file.metrics.physical_loc > 20 {
                diags.push(DiagnosticWarning {
                    tag: "-Wdoc".into(),
                    message: format!(
                        "{} SLOC, {} comment lines [{:.1}% < {:.1}%]",
                        file.metrics.sloc,
                        file.metrics.comments,
                        ratio,
                        config.doc_ratio_threshold
                    ),
                    file: file.path.clone(),
                    related_files: Vec::new(),
                    level: if config.warn_error { WarningLevel::Error } else { WarningLevel::Warning },
                });
            }
        }
    }

    diags
}

pub fn render_diagnostics(
    graph: &DependencyGraph,
    config: &Config,
) -> bool {
    println!("[spaghetti diagnostics] Scanning {} code files...\n", graph.files.len());

    let diags = collect_diagnostics(graph, config);

    if diags.is_empty() {
        println!("No architectural warnings found! Codebase is clean.");
        return true;
    }

    // Group by tag
    let mut cycles = Vec::new();
    let mut orphans = Vec::new();
    let mut gods = Vec::new();
    let mut docs = Vec::new();

    for d in &diags {
        match d.tag.as_str() {
            "-Wcycle" => cycles.push(d),
            "-Worphan" => orphans.push(d),
            "-Wgod" => gods.push(d),
            "-Wdoc" => docs.push(d),
            _ => {}
        }
    }

    if !cycles.is_empty() {
        println!("[-Wcycle] Circular Dependencies:");
        for d in &cycles {
            println!("  ! {}", d.message);
        }
        println!();
    }

    if !orphans.is_empty() {
        println!("[-Worphan] Dead / Unimported Code Files:");
        for d in &orphans {
            let _rel = d.file.file_name().unwrap_or_default().to_string_lossy();
            println!("  ? {} ({})", d.file.display(), d.message);
        }
        println!();
    }

    if !gods.is_empty() {
        println!("[-Wgod] God Files (> {} SLOC):", config.god_loc_threshold);
        for d in &gods {
            println!("  ! {} ({})", d.file.display(), d.message);
        }
        println!();
    }

    if !docs.is_empty() {
        println!("[-Wdoc] Under-documented Files (< {}% comments):", config.doc_ratio_threshold);
        for d in &docs {
            println!("  ! {} ({})", d.file.display(), d.message);
        }
        println!();
    }

    println!(
        "Found {} warning(s) across categories (use -Wno-<tag> to suppress).",
        diags.len()
    );

    let has_error = diags.iter().any(|d| d.level == WarningLevel::Error);
    !has_error
}
