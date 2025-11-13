use crate::{tool::get_crate_name, tree::SectionRecord};
use cargo_lock::dependency::{
    Tree,
    graph::{Graph, NodeIndex},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Package dependency resolver
/// Maps crate names to their dependency paths in the dependency tree
#[derive(Debug, Default, Clone)]
pub struct Packages {
    parent: HashMap<String, Vec<String>>,
}

/// Helper function to normalize crate names by replacing hyphens with underscores
#[inline]
fn normalize_crate_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Node used in breadth-first search traversal of the dependency graph
struct BfsNode {
    name: Box<str>,
    path: Vec<String>,
    index: NodeIndex,
}

impl BfsNode {
    /// Create a new BFS node from a graph index
    fn new(g: &Graph, index: NodeIndex) -> Self {
        let name = normalize_crate_name(g[index].name.as_str());
        let name_boxed: Box<str> = name.as_str().into();
        Self {
            name: name_boxed,
            path: vec![name],
            index,
        }
    }

    /// Create a child BFS node with an extended path
    fn child(g: &Graph, index: NodeIndex, mut path: Vec<String>) -> Self {
        let name = normalize_crate_name(g[index].name.as_str());
        let name_boxed: Box<str> = name.as_str().into();
        path.push(name);
        Self {
            name: name_boxed,
            path,
            index,
        }
    }
}

impl Packages {
    /// Create a new Packages resolver from a dependency tree and section records
    /// Uses BFS to find the shortest path to each crate in the dependency graph
    pub fn new(tree: &Tree, records: &[SectionRecord]) -> Self {
        // Build set of crate names from records
        let crates: HashSet<String> = records
            .iter()
            .filter_map(|record| get_crate_name(&record.symbols))
            .map(|(name, _)| name)
            .collect();

        let g = tree.graph();
        let roots = tree.roots().to_vec();

        // Pre-allocate collections with estimated capacity
        let estimated_nodes = g.node_count();
        let mut visited = HashSet::with_capacity(estimated_nodes);
        let mut queue = VecDeque::with_capacity(estimated_nodes / 4);
        let mut parent: HashMap<String, Vec<String>> = HashMap::with_capacity(crates.len());

        // Initialize queue with root nodes
        for &start in &roots {
            queue.push_back(BfsNode::new(g, start));
        }

        // BFS traversal to find shortest paths
        while let Some(BfsNode { name, path, index }) = queue.pop_front() {
            if visited.contains(&index) {
                continue;
            }

            let name_str = name.as_ref();
            if crates.contains(name_str) {
                // Use entry API to avoid double lookup
                parent
                    .entry(name_str.to_string())
                    .and_modify(|entry| {
                        // Keep shorter path
                        if entry.len() > path.len() {
                            *entry = path.clone();
                        }
                    })
                    .or_insert_with(|| path.clone());
            } else {
                parent.insert(name_str.to_string(), path.clone());
            }

            visited.insert(index);

            // Add unvisited neighbors to queue
            for neighbor in g.neighbors(index) {
                if !visited.contains(&neighbor) {
                    queue.push_back(BfsNode::child(g, neighbor, path.clone()));
                }
            }
        }

        // Ensure standard library crates (std, alloc) have entries
        for crate_name in crates {
            parent
                .entry(crate_name.clone())
                .or_insert_with(|| vec![crate_name]);
        }

        Self { parent }
    }

    /// Get the dependency path for a crate by ID
    /// Returns a reference to avoid cloning when possible
    pub fn get_path(&self, id: &str) -> &[String] {
        self.parent.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
