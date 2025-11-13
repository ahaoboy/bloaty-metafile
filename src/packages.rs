use crate::{tool::get_crate_name, tree::SectionRecord};
use cargo_lock::dependency::{
    Tree,
    graph::{Graph, NodeIndex},
};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Default, Clone)]
pub struct Packages {
    parent: HashMap<String, Vec<String>>,
}

struct BfsNode {
    name: String,
    path: Vec<String>,
    index: NodeIndex,
}

impl BfsNode {
    fn new(g: &Graph, index: NodeIndex) -> Self {
        let name = g[index].name.as_str().to_string().replace("-", "_");
        Self {
            name: name.clone(),
            path: vec![name],
            index,
        }
    }

    fn child(g: &Graph, index: NodeIndex, mut path: Vec<String>) -> Self {
        let name = g[index].name.as_str().to_string().replace("-", "_");
        path.push(name.clone());
        Self { name, path, index }
    }
}

impl Packages {
    pub fn new(tree: &Tree, records: &[SectionRecord]) -> Self {
        let crates: HashSet<String> = records
            .iter()
            .filter_map(|i| get_crate_name(&i.symbols))
            .map(|(name, _)| name)
            .collect();

        let g = tree.graph();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        let mut parent: HashMap<String, Vec<String>> = HashMap::new();

        for start in tree.roots() {
            queue.push_back(BfsNode::new(g, start));
        }

        while let Some(BfsNode { name, path, index }) = queue.pop_front() {
            if visited.contains(&index) {
                continue;
            }

            if crates.contains(&name)
                && let Some(entry) = parent.get_mut(&name)
            {
                if entry.len() > path.len() {
                    *entry = path.clone();
                }
            } else {
                parent.insert(name, path.clone());
            }

            visited.insert(index);
            for neighbor in g.neighbors(index) {
                if !visited.contains(&neighbor) {
                    queue.push_back(BfsNode::child(g, neighbor, path.clone()));
                }
            }
        }

        // std, alloc
        for i in crates {
            parent.entry(i.clone()).or_insert(vec![i]);
        }

        Self { parent }
    }

    pub fn get_path(&self, id: &str) -> Vec<String> {
        self.parent.get(id).cloned().unwrap_or_default()
    }
}
