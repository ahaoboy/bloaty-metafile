use cargo_lock::Lockfile;
use serde::Deserialize;
use serde_metafile::{Import, Input, InputDetail, Metafile, Output};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub vmsize: u64,
    pub filesize: u64,
    pub count: u64,
    pub nodes: HashMap<String, Node>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct SectionRecord {
    sections: String,
    symbols: String,
    vmsize: u64,
    filesize: u64,
}

impl Node {
    fn add_path(&mut self, path: &[String], vmsize: u64, filesize: u64) {
        let mut current = self;
        for i in 0..path.len() {
            let part = path[i].clone();
            current.vmsize += vmsize;
            current.filesize += filesize;
            current.count += 1;
            if i == path.len() - 1 {
                current.nodes.entry(part.to_string()).or_insert(Node {
                    name: part.to_string(),
                    vmsize,
                    filesize,
                    count: 1,
                    nodes: HashMap::new(),
                });
            } else {
                current = current.nodes.entry(part.to_string()).or_insert(Node {
                    name: part.to_string(),
                    vmsize: 0,
                    filesize: 0,
                    count: 0,
                    nodes: HashMap::new(),
                });
            }
        }
    }
}

fn symbol_is_crate(s: &str) -> bool {
    if s.contains("..") {
        return false;
    }
    let mut chars = s.chars();
    if chars.next() == Some('[') && chars.last() == Some(']') {
        return false;
    }
    true
}

pub fn get_tree(csv: &str, lock: Option<String>) -> Node {
    let mut tree = Node {
        name: "__ROOT__".to_string(),
        vmsize: 0,
        filesize: 0,
        count: 0,
        nodes: HashMap::new(),
    };

    let mut parent: HashMap<String, String> = HashMap::new();
    if let Ok(lock) = Lockfile::load(lock.unwrap_or("Cargo.lock".to_string())) {
        for pkg in lock.packages {
            let name = pkg.name.as_str().replace("-", "_");
            if !parent.contains_key(&name) {
                parent.insert(name.clone(), name.clone());
            }
            for dep in pkg.dependencies {
                let dep_name = dep.name.as_str().replace("-", "_");
                parent.insert(dep_name.clone(), name.clone());
            }
        }
    }

    // monorepo may have multiple roots
    let root_crates: HashSet<_> = parent
        .keys()
        .filter(|i| parent.get(&i.to_string()) == Some(i))
        .collect();
    let mut rdr = csv::Reader::from_reader(csv.as_bytes());

    for SectionRecord {
        sections,
        symbols,
        vmsize,
        filesize,
    } in rdr.deserialize::<SectionRecord>().flat_map(|i| i.ok())
    {
        let mut symbols_parts: Vec<String> = symbols.split("::").map(|i| i.to_string()).collect();
        if symbols_parts.len() == 1 || !symbol_is_crate(&symbols_parts[0]) {
            symbols_parts.insert(0, sections.to_string());
            // FIXME: Put all unknown data into sections and distinguish them from crates
            symbols_parts.insert(0, "sections".to_string());
        } else {
            // crate
            symbols_parts.insert(1, sections.to_string());
            let mut prefix = vec![];
            let crate_name = &symbols_parts[0];
            let mut top = crate_name;
            loop {
                if root_crates.contains(crate_name) {
                    break;
                }
                let Some(p) = parent.get(top) else {
                    break;
                };
                prefix.push(top.clone());
                if p == top {
                    break;
                }
                top = p;
            }
            for i in prefix {
                symbols_parts.insert(0, i.clone());
            }
        };

        tree.add_path(&symbols_parts, vmsize, filesize);
    }
    tree
}

pub fn from_csv(csv: &str, name: &str, lock: Option<String>, deep: usize) -> Metafile {
    let tree = get_tree(csv, lock);

    convert_node_to_metafile(tree, name, deep)
}

pub fn convert_node_to_metafile(root: Node, name: &str, deep: usize) -> Metafile {
    let mut inputs = HashMap::new();
    for i in &root.nodes {
        traverse(i.1, &mut inputs, None, deep);
        traverse(i.1, &mut inputs, None, deep);
    }
    let entry_point_path = root.name.clone();
    let output_inputs = inputs
        .iter()
        .map(|(path, input)| {
            (
                path.clone(),
                InputDetail {
                    bytes_in_output: input.bytes,
                },
            )
        })
        .collect();
    let output = Output {
        bytes: root.filesize,
        inputs: output_inputs,
        imports: vec![],
        exports: vec![],
        entry_point: Some(entry_point_path),
        css_bundle: None,
    };
    let outputs = HashMap::from([(name.to_string(), output)]);
    Metafile { inputs, outputs }
}

fn traverse(node: &Node, inputs: &mut HashMap<String, Input>, dir: Option<String>, deep: usize) {
    let full_path = node.name.clone();
    let dir: String = dir.map_or(full_path.clone(), |i| i + "/" + &full_path);
    // FIXME: Add parameters to filter nodes that are too deep and reduce the size of json
    if deep != 0 && dir.matches("/").count() >= deep {
        return;
    }
    let imports = node
        .nodes
        .values()
        .map(|child| Import {
            path: dir.clone() + "/" + &child.name,
            kind: None,
            external: false,
            original: None,
            with: None,
        })
        .collect();

    let input = Input {
        bytes: node.filesize,
        imports,
        format: None,
        with: None,
    };

    inputs.insert(dir.clone(), input);

    for child in node.nodes.values() {
        traverse(child, inputs, Some(dir.clone()), deep);
    }
}

#[cfg(test)]
mod test {
    use crate::{get_tree, symbol_is_crate};

    #[test]
    fn test_symbol_is_crate() {
        for (a, b) in [
            ("[16482 Others]", false),
            (
                "_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$",
                false,
            ),
        ] {
            assert_eq!(symbol_is_crate(a), b);
        }
    }

    #[test]
    fn test_get_tree() {
        for csv in [
            r#"
sections,symbols,vmsize,filesize
"__TEXT,__text",[1848 Others],918108,918108
"#,
            r#"
sections,symbols,vmsize,filesize
.text,[1843 Others],1086372,1086372
"#,
        ] {
            let tree = get_tree(csv, None);
            assert_eq!(tree.nodes.len(), 1)
        }
    }
}
