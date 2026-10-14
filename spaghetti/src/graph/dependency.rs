use std::collections::HashSet;
use crate::core::types::FileNode;
use super::resolver::PathResolver;

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Adjacency list: u -> vec of v where u imports v
    pub adj: Vec<Vec<usize>>,
    /// Reverse adjacency list: v -> vec of u where u imports v (incoming edges)
    pub rev_adj: Vec<Vec<usize>>,
    pub files: Vec<FileNode>,
}

impl DependencyGraph {
    pub fn build(files: Vec<FileNode>) -> Self {
        let n = files.len();
        let resolver = PathResolver::new(&files);

        let mut adj = vec![Vec::new(); n];
        let mut rev_adj = vec![Vec::new(); n];

        for (u_idx, file) in files.iter().enumerate() {
            let mut seen = HashSet::new();
            for import in &file.imports {
                if let Some(ref target_path) = import.resolved_path {
                    if let Some(v_idx) = resolver.resolve(target_path) {
                        if u_idx != v_idx && seen.insert(v_idx) {
                            adj[u_idx].push(v_idx);
                            rev_adj[v_idx].push(u_idx);
                        }
                    }
                }
            }
        }

        Self {
            adj,
            rev_adj,
            files,
        }
    }

    pub fn in_degree(&self, idx: usize) -> usize {
        self.rev_adj[idx].len()
    }

    pub fn out_degree(&self, idx: usize) -> usize {
        self.adj[idx].len()
    }
}
