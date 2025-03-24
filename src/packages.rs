use crate::{tool::get_crate_name, tree::SectionRecord};
use cargo_lock::Lockfile;
use std::collections::{HashMap, HashSet};

fn find_shortest_parents(
    parents: &HashMap<String, HashSet<String>>,
    dependencies: &HashMap<String, HashSet<String>>,
) -> HashMap<String, String> {
    let mut cache: HashMap<String, (usize, usize)> = HashMap::new();
    fn compute_shortest_path(
        start: &str,
        parents: &HashMap<String, HashSet<String>>,
        cache: &mut HashMap<String, (usize, usize)>,
    ) -> (usize, usize) {
        if let Some(&cached) = cache.get(start) {
            return cached;
        }
        let mut min_path = None;
        if let Some(nodes) = parents.get(start) {
            for parent in nodes {
                let (path_len, name_len) = compute_shortest_path(parent, parents, cache);
                let current_path = (path_len + 1, name_len + start.len());
                if min_path.is_none() || current_path < min_path.unwrap() {
                    min_path = Some(current_path);
                }
            }
        }
        let min_path = min_path.unwrap_or((0, 0));
        cache.insert(start.to_string(), min_path);
        min_path
    }

    let parent_map: HashMap<String, String> = parents
        .keys()
        .map(|name| {
            let shortest_parent = parents[name]
                .iter()
                .filter(|p| dependencies[*p].contains(name))
                .min_by(|&a, &b| {
                    let a_path = compute_shortest_path(a, parents, &mut cache);
                    let b_path = compute_shortest_path(b, parents, &mut cache);
                    if name == "tokio" {
                        eprintln!("{:?} {:?}", a_path, b_path);
                    }
                    a_path.cmp(&b_path)
                })
                .unwrap_or(name);
            (name.clone(), shortest_parent.clone())
        })
        .collect();

    parent_map
}

#[derive(Debug, Default, Clone)]
pub struct Packages {
    parent: HashMap<String, String>,
}

impl Packages {
    pub fn new(lock: Lockfile, records: &[SectionRecord]) -> Self {
        let crates: HashSet<String> = records
            .iter()
            .filter_map(|i| get_crate_name(&i.symbols))
            .map(|(name, _)| name)
            .collect();

        let (mut parents, dependencies) = lock.packages.into_iter().fold(
            (HashMap::new(), HashMap::new()),
            |(mut parents, mut deps), pkg| {
                let name = pkg.name.as_str().replace("-", "_");
                parents.entry(name.clone()).or_insert_with(HashSet::new);

                let pkg_deps: HashSet<String> = pkg
                    .dependencies
                    .iter()
                    .map(|dep| dep.name.as_str().replace("-", "_"))
                    .collect();

                for dep in &pkg_deps {
                    parents
                        .entry(dep.clone())
                        .or_insert_with(HashSet::new)
                        .insert(name.clone());
                }
                deps.insert(name, pkg_deps);
                (parents, deps)
            },
        );

        let mut roots = HashSet::new();
        while let Some(root) = parents
            .iter()
            .find_map(|(k, v)| v.is_empty().then(|| k.clone()))
        {
            let is_real_root = crates.contains(&root);
            parents.remove(&root);
            if is_real_root {
                roots.insert(root.clone());
            }
            for p in parents.values_mut() {
                if p.remove(&root) && is_real_root {
                    p.insert(root.clone());
                }
            }
        }
        for i in roots {
            parents.insert(i, HashSet::new());
        }
        let parent = find_shortest_parents(&parents, &dependencies);

        eprintln!("{:?}", parent.get("tokio"));
        // eprintln!("{}", serde_json::to_string(&parent).unwrap());
        Self { parent }
    }

    pub fn get_path(&self, id: &str) -> Vec<String> {
        let mut path = vec![id.to_string()];
        let mut cur = id;
        while let Some(parent) = self.parent.get(cur) {
            if parent == cur {
                break;
            }
            path.push(parent.clone());
            cur = parent;
        }
        path
    }
}
