use crate::{tool::get_crate_name, tree::SectionRecord};
use cargo_lock::Lockfile;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Default, Clone)]
pub struct Packages {
    dependencies: HashMap<String, HashSet<String>>,
    parent: HashMap<String, HashSet<String>>,
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
            parent.entry(name.clone()).or_insert(HashSet::new());

            let deps: HashSet<String> = pkg
                .dependencies
                .iter()
                .map(|dep| dep.name.as_str().replace("-", "_"))
                .collect();
            for dep in &deps {
                parent
                    .entry(dep.clone())
                    .or_insert(HashSet::new())
                    .insert(name.clone());
            }

            dependencies.insert(name, deps);
        }

        let mut removed_roots = HashSet::new();
        loop {
            let roots: HashSet<String> = parent
                .iter()
                .filter(|(k, v)| v.is_empty() && !removed_roots.contains(*k))
                .map(|(k, _)| k.clone())
                .collect();
            if roots.is_empty() {
                break;
            }

            if let Some(root) = roots.iter().find(|r| crates.contains(*r)) {
                // find the real root, all nodes pointing to other roots need to be pointed to this root.
                for p in parent.values_mut() {
                    let common: HashSet<_> = p.intersection(&roots).cloned().collect();
                    if common.is_empty() {
                        continue;
                    }
                    for i in common {
                        p.remove(&i);
                    }
                    p.insert(root.clone());
                }
                break;
            } else {
                // remove all fake roots in tree
                for p in parent.values_mut() {
                    let common: HashSet<_> = p.intersection(&roots).cloned().collect();
                    for i in common {
                        p.remove(&i);
                    }
                }
            }

            for i in &roots {
                removed_roots.insert(i.clone());
            }
        }

        Packages {
            dependencies,
            parent,
        }
    }

    pub fn is_root(&self, id: &str) -> bool {
        self.parent.get(id).is_some_and(|i| i.is_empty())
    }

    // Find the parent node closest to the root node.
    pub fn get_short_parent(&self, id: &str) -> Option<String> {
        let mut q = VecDeque::from_iter(
            self.parent
                .get(id)?
                .iter()
                .map(|i| (vec![i.clone()], self.is_root(i))),
        );
        while let Some((path, end)) = q.pop_front() {
            if end {
                for i in path.iter().rev() {
                    let is_direct_dep = self
                        .dependencies
                        .get(i)
                        .is_some_and(|deps| deps.contains(id));
                    if is_direct_dep {
                        return Some(i.to_string());
                    }
                }
                continue;
            }

            if let Some(p) = path.last().and_then(|i| self.parent.get(i)) {
                for i in p {
                    let mut next = path.clone();
                    next.push(i.to_string());
                    q.push_back((next, self.is_root(i)));
                }
            }
        }
        None
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
