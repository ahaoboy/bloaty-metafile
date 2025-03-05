use cargo_lock::Lockfile;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct Packages {
    dependencies: HashMap<String, HashSet<String>>,
    parent: HashMap<String, String>,
}

impl Packages {
    pub fn from_lock(lock: Lockfile) -> Self {
        let mut dependencies = HashMap::new();
        let mut parent = HashMap::new();
        for pkg in lock.packages {
            let name = pkg.name.as_str().replace("-", "_");
            if !parent.contains_key(&name) {
                parent.insert(name.clone(), name.clone());
            }
            for dep in &pkg.dependencies {
                let dep_name = dep.name.as_str().replace("-", "_");
                parent.insert(dep_name.clone(), name.clone());
            }

            let deps: HashSet<String> = HashSet::from_iter(
                pkg.dependencies
                    .into_iter()
                    .map(|i| i.name.as_str().replace("-", "_")),
            );

            dependencies.insert(name, deps);
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
