use super::dependency::DependencyGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DfsColor {
    White, // Unvisited
    Gray,  // On current search stack
    Black, // Fully processed
}

#[derive(Debug, Clone)]
pub struct CycleDetector {
    pub cycles: Vec<Vec<usize>>,
}

impl CycleDetector {
    /// Detects cycles using 3-color DFS traversal
    pub fn find_cycles(graph: &DependencyGraph) -> Self {
        let n = graph.files.len();
        let mut color = vec![DfsColor::White; n];
        let mut parent = vec![None; n];
        let mut cycles = Vec::new();

        for i in 0..n {
            if color[i] == DfsColor::White {
                let mut path = Vec::new();
                dfs_visit(i, graph, &mut color, &mut parent, &mut path, &mut cycles);
            }
        }

        Self { cycles }
    }

    /// Finds Strongly Connected Components using Tarjan's algorithm
    #[allow(dead_code)]
    pub fn find_sccs(graph: &DependencyGraph) -> Vec<Vec<usize>> {
        let n = graph.files.len();
        let mut index = 0;
        let mut indices = vec![None; n];
        let mut lowlink = vec![0; n];
        let mut on_stack = vec![false; n];
        let mut stack = Vec::new();
        let mut sccs = Vec::new();

        for i in 0..n {
            if indices[i].is_none() {
                tarjan_strongconnect(
                    i,
                    graph,
                    &mut index,
                    &mut indices,
                    &mut lowlink,
                    &mut on_stack,
                    &mut stack,
                    &mut sccs,
                );
            }
        }

        // Return only SCCs with > 1 node (or single node with self-loop)
        sccs.into_iter()
            .filter(|scc| scc.len() > 1 || (scc.len() == 1 && graph.adj[scc[0]].contains(&scc[0])))
            .collect()
    }
}

fn dfs_visit(
    u: usize,
    graph: &DependencyGraph,
    color: &mut [DfsColor],
    parent: &mut [Option<usize>],
    path: &mut Vec<usize>,
    cycles: &mut Vec<Vec<usize>>,
) {
    color[u] = DfsColor::Gray;
    path.push(u);

    for &v in &graph.adj[u] {
        if color[v] == DfsColor::Gray {
            // Found a cycle! Extract the cycle path from v to u
            if let Some(pos) = path.iter().position(|&node| node == v) {
                let cycle: Vec<usize> = path[pos..].to_vec();
                cycles.push(cycle);
            }
        } else if color[v] == DfsColor::White {
            parent[v] = Some(u);
            dfs_visit(v, graph, color, parent, path, cycles);
        }
    }

    path.pop();
    color[u] = DfsColor::Black;
}

fn tarjan_strongconnect(
    u: usize,
    graph: &DependencyGraph,
    index: &mut usize,
    indices: &mut [Option<usize>],
    lowlink: &mut [usize],
    on_stack: &mut [bool],
    stack: &mut Vec<usize>,
    sccs: &mut Vec<Vec<usize>>,
) {
    indices[u] = Some(*index);
    lowlink[u] = *index;
    *index += 1;
    stack.push(u);
    on_stack[u] = true;

    for &v in &graph.adj[u] {
        if indices[v].is_none() {
            tarjan_strongconnect(
                v,
                graph,
                index,
                indices,
                lowlink,
                on_stack,
                stack,
                sccs,
            );
            lowlink[u] = lowlink[u].min(lowlink[v]);
        } else if on_stack[v] {
            lowlink[u] = lowlink[u].min(indices[v].unwrap());
        }
    }

    if lowlink[u] == indices[u].unwrap() {
        let mut scc = Vec::new();
        while let Some(w) = stack.pop() {
            on_stack[w] = false;
            scc.push(w);
            if w == u {
                break;
            }
        }
        sccs.push(scc);
    }
}
