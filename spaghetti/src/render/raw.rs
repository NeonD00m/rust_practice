use crate::core::types::FileNode;
use crate::graph::DependencyGraph;
use super::diag::collect_diagnostics;
use crate::core::config::Config;

pub fn render_raw_files(files: &[FileNode]) {
    for file in files {
        let path_str = file.rel_path.to_string_lossy().replace('\\', "/");
        println!("{}", path_str);
    }
}

pub fn render_raw_diagnostics(graph: &DependencyGraph, config: &Config) {
    let diags = collect_diagnostics(graph, config);
    for d in diags {
        let path_str = d.file.to_string_lossy().replace('\\', "/");
        println!("{}", path_str);
    }
}

pub fn render_raw_loc(files: &[FileNode], limit: Option<usize>) {
    let mut sorted: Vec<&FileNode> = files.iter().collect();
    sorted.sort_by(|a, b| b.metrics.sloc.cmp(&a.metrics.sloc));
    let count = limit.unwrap_or(sorted.len());
    for file in sorted.into_iter().take(count) {
        let path_str = file.rel_path.to_string_lossy().replace('\\', "/");
        println!("{}", path_str);
    }
}
