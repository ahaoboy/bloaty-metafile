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

    fn is_direct_dep(&self, id: &str, dep: &str) -> bool {
        self.dependencies.get(id).is_some_and(|i| i.contains(dep))
    }

    pub fn get_path(&self, id: &str) -> Vec<String> {
        if self.is_root(id) {
            return vec![];
        }
        let mut path = vec![];
        let mut cur = &id.to_string();
        while let Some(parent) = self.parent.get(cur) {
            path.push(cur.clone());
            if parent == cur {
                break;
            }
            cur = parent;
        }

        let mut post_path = vec![];
        for i in path.iter().rev() {
            if self.is_direct_dep(i, id) {
                let mut short = vec![id.to_string(), i.to_string()];
                short.extend(post_path.into_iter().rev());
                return short;
            }
            post_path.push(i.clone());
        }
        path
    }
}
