use cargo_lock::Lockfile;
use std::collections::{HashMap, HashSet};

use crate::{tool::get_crate_name, tree::SectionRecord};

#[derive(Debug, Default, Clone)]
pub struct Packages {
    dependencies: HashMap<String, HashSet<String>>,
    parent: HashMap<String, String>,
}

impl Packages {
    pub fn new(lock: Lockfile, records: &[SectionRecord]) -> Self {
        let mut dependencies = HashMap::new();
        let mut parent = HashMap::new();
        let crates: HashSet<String> = records
            .iter()
            .filter_map(|i| get_crate_name(&i.symbols))
            .map(|(name, _)| name)
            .collect();

        for pkg in lock.packages {
            let name = pkg.name.as_str().replace("-", "_");
            // Some packages in llrt have no code, but we need these empty packages to maintain the dependency tree
            // So we cannot use crates to filter them.
            parent.entry(name.clone()).or_insert(name.clone());

            let deps: HashSet<String> = pkg
                .dependencies
                .iter()
                .map(|dep| dep.name.as_str().replace("-", "_"))
                .collect();
            for dep in &deps {
                parent.insert(dep.clone(), name.clone());
            }

            dependencies.insert(name, deps);
        }

        let roots: Vec<String> = parent
            .iter()
            .filter(|(k, v)| k == v)
            .map(|(k, _)| k.clone())
            .collect();

        // Since the order is random, and the union-find set can only have one parent, all nodes pointing to other roots need to be pointed to the binary root.
        if let Some(root) = roots.iter().find(|r| crates.contains(*r)) {
            for (_, p) in parent.iter_mut() {
                if roots.contains(p) && p != root {
                    *p = root.clone();
                }
            }
        }

        Packages {
            dependencies,
            parent,
        }
    }

    pub fn is_root(&self, id: &str) -> bool {
        self.parent.get(id).is_some_and(|i| i == id)
    }

    // Find the parent node closest to the root node.
    pub fn get_short_parent(&self, id: &str) -> Option<String> {
        let mut path = vec![];
        let mut cur = id;
        while let Some(p) = self.parent.get(cur) {
            if p == cur {
                break;
            }
            path.push(p);
            cur = p;
        }
        for i in path.iter().rev() {
            let is_direct_dep = self
                .dependencies
                .get(*i)
                .is_some_and(|deps| deps.contains(id));
            if is_direct_dep {
                return Some(i.to_string());
            }
        }
        self.parent.get(id).cloned()
    }

    pub fn get_path(&self, id: &str) -> Vec<String> {
        let mut path = vec![id.to_string()];
        if self.is_root(id) {
            return path;
        }
        let mut cur = id.to_string();
        while let Some(parent) = self.get_short_parent(&cur) {
            if parent == cur {
                break;
            }
            path.push(parent.clone());
            cur = parent;
        }
        path
    }
}
