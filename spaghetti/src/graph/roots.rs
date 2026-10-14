use super::dependency::DependencyGraph;

pub struct RootDetector;

impl RootDetector {
    /// Identifies root entrypoint nodes (in-degree == 0), ranked by heuristics
    pub fn find_roots(graph: &DependencyGraph) -> Vec<usize> {
        let n = graph.files.len();
        if n == 0 {
            return Vec::new();
        }

        let mut candidate_roots: Vec<usize> = (0..n)
            .filter(|&i| graph.in_degree(i) == 0)
            .collect();

        // If all nodes have in-degree > 0 (e.g. pure cycle), take highest out-degree
        if candidate_roots.is_empty() {
            let mut all_nodes: Vec<usize> = (0..n).collect();
            all_nodes.sort_by(|&a, &b| graph.out_degree(b).cmp(&graph.out_degree(a)));
            return vec![all_nodes[0]];
        }

        // Rank roots:
        // 1. Primary: Standard entrypoint naming (main.*, index.*, lib.*, app.*, mod.*)
        // 2. Secondary: High out-degree
        // 3. Tertiary: Shortest relative path
        candidate_roots.sort_by(|&a, &b| {
            let a_score = entrypoint_score(&graph.files[a].rel_path.to_string_lossy());
            let b_score = entrypoint_score(&graph.files[b].rel_path.to_string_lossy());

            b_score.cmp(&a_score)
                .then_with(|| graph.out_degree(b).cmp(&graph.out_degree(a)))
                .then_with(|| graph.files[a].rel_path.cmp(&graph.files[b].rel_path))
        });

        candidate_roots
    }
}

fn entrypoint_score(path_str: &str) -> usize {
    let lower = path_str.to_ascii_lowercase().replace('\\', "/");
    let file_name = lower.split('/').last().unwrap_or("");

    if file_name.starts_with("main.") {
        100
    } else if file_name.starts_with("index.") {
        90
    } else if file_name.starts_with("app.") {
        85
    } else if file_name.starts_with("lib.") {
        80
    } else if file_name.starts_with("mod.") {
        70
    } else if lower.contains("bin/") || lower.contains("cmd/") {
        60
    } else {
        10
    }
}
