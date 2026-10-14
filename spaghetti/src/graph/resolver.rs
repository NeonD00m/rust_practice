use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::core::types::FileNode;

pub struct PathResolver {
    canonical_map: HashMap<PathBuf, usize>,
}

impl PathResolver {
    pub fn new(files: &[FileNode]) -> Self {
        let mut canonical_map = HashMap::new();
        for (idx, file) in files.iter().enumerate() {
            if let Ok(canon) = file.path.canonicalize() {
                canonical_map.insert(canon, idx);
            }
            canonical_map.insert(file.path.clone(), idx);
            canonical_map.insert(file.rel_path.clone(), idx);
        }
        Self { canonical_map }
    }

    pub fn resolve(&self, target_path: &Path) -> Option<usize> {
        if let Some(&idx) = self.canonical_map.get(target_path) {
            return Some(idx);
        }
        if let Ok(canon) = target_path.canonicalize() {
            if let Some(&idx) = self.canonical_map.get(&canon) {
                return Some(idx);
            }
        }
        None
    }
}
