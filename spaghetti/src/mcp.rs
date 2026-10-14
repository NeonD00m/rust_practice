use serde_json::json;
use crate::core::config::Config;
use crate::graph::DependencyGraph;
use crate::render::diag::collect_diagnostics;
use crate::scanner::ScanResult;

pub struct McpApi;

impl McpApi {
    pub fn tree_json(scan: &ScanResult, graph: Option<&DependencyGraph>) -> String {
        let mut files_json = Vec::new();

        for (idx, file) in scan.files.iter().enumerate() {
            let in_deg = graph.map(|g| g.in_degree(idx)).unwrap_or(0);
            let out_deg = graph.map(|g| g.out_degree(idx)).unwrap_or(0);

            files_json.push(json!({
                "path": file.path.to_string_lossy().replace('\\', "/"),
                "rel_path": file.rel_path.to_string_lossy().replace('\\', "/"),
                "language": file.language.name(),
                "sloc": file.metrics.sloc,
                "symbols": file.symbols.iter().map(|s| {
                    json!({
                        "name": s.name,
                        "kind": s.kind.badge(),
                        "line": s.line,
                    })
                }).collect::<Vec<_>>(),
                "in_degree": in_deg,
                "out_degree": out_deg,
            }));
        }

        json!({
            "total_files": scan.files.len(),
            "ignored_count": scan.ignored_count,
            "files": files_json,
        }).to_string()
    }

    pub fn diag_json(graph: &DependencyGraph, config: &Config) -> String {
        let diags = collect_diagnostics(graph, config);
        let diags_json: Vec<_> = diags.iter().map(|d| {
            json!({
                "tag": d.tag,
                "message": d.message,
                "file": d.file.to_string_lossy().replace('\\', "/"),
                "level": match d.level {
                    crate::core::types::WarningLevel::Warning => "warning",
                    crate::core::types::WarningLevel::Error => "error",
                }
            })
        }).collect();

        json!({
            "total_warnings": diags.len(),
            "warnings": diags_json,
        }).to_string()
    }
}
