use crate::{
    packages::Packages,
    tool::{ROOT_NAME, UNKNOWN_NAME, get_path_from_record},
};
use cargo_lock::Lockfile;
use serde::Deserialize;
use serde_metafile::{Import, Input, InputDetail, Metafile, Output};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub vmsize: u64,
    pub filesize: u64,
    pub total_vmsize: u64,
    pub total_filesize: u64,
    // pub count: u64,
    pub nodes: HashMap<String, Node>,
}

#[derive(Debug, Deserialize)]
pub struct SectionRecord {
    pub sections: String,
    pub symbols: String,
    pub vmsize: u64,
    pub filesize: u64,
}

pub struct Tree {
    root: Node,
}

impl Tree {
    pub fn new(csv: &str, lock: Option<String>) -> Tree {
        let mut tree = Tree {
            root: Node {
                name: ROOT_NAME.to_string(),
                vmsize: 0,
                filesize: 0,
                // count: 0,
                nodes: HashMap::new(),
                total_filesize: 0,
                total_vmsize: 0,
            },
        };

        let mut rdr = csv::Reader::from_reader(csv.as_bytes());

        let records: Vec<_> = rdr
            .deserialize::<SectionRecord>()
            .flat_map(|i| i.ok())
            .collect();
        let packages = Lockfile::load(lock.unwrap_or("Cargo.lock".to_string()))
            .map(|lock| Packages::new(lock, &records))
            .unwrap_or_default();

        let mut records_paths = vec![];
        for SectionRecord {
            sections,
            symbols,
            vmsize,
            filesize,
        } in records
        {
            // symbols maybe empty
            let sym = if symbols.is_empty() {
                UNKNOWN_NAME.to_string()
            } else {
                symbols
            };
            let path = get_path_from_record(sym, sections, &packages);
            records_paths.push(path.join("/"));
            tree.add_path(&path, vmsize, filesize);
        }

        // let mut cur = vec![];
        // let mut paths = vec![];
        // Tree::calc_size(&mut tree.root, &mut cur, &mut paths);
        tree
    }

    pub fn to_metafile(&self, name: &str, deep: usize) -> Metafile {
        let root = &self.root;
        let mut inputs = HashMap::new();
        for i in root.nodes.values() {
            self.traverse(i, &mut inputs, None, deep);
            self.traverse(i, &mut inputs, None, deep);
        }
        let output_inputs: HashMap<_, _> = inputs
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
            bytes: root.total_filesize,
            inputs: output_inputs,
            imports: vec![],
            exports: vec![],
            entry_point: None,
            css_bundle: None,
        };
        let outputs = HashMap::from([(name.to_string(), output)]);
        Metafile { inputs, outputs }
    }

    fn add_path(&mut self, path: &[String], vmsize: u64, filesize: u64) -> &Node {
        let mut current = &mut self.root;
        for i in 0..path.len() {
            let part = path[i].clone();
            current.total_vmsize += vmsize;
            current.total_filesize += filesize;
            // current.count += 1;
            if i == path.len() - 1 {
                let n = current.nodes.entry(part.to_string()).or_insert(Node {
                    name: part.to_string(),
                    vmsize,
                    filesize,
                    // count: 1,
                    nodes: HashMap::new(),
                    total_filesize: 0,
                    total_vmsize: 0,
                });
                n.vmsize = vmsize;
                n.filesize = filesize;
            } else {
                current = current.nodes.entry(part.to_string()).or_insert(Node {
                    name: part.to_string(),
                    vmsize: 0,
                    filesize: 0,
                    // count: 0,
                    nodes: HashMap::new(),
                    total_filesize: 0,
                    total_vmsize: 0,
                });
            }
        }

        current
    }

    // fn calc_size(tree: &mut Node, cur: &mut Vec<String>, paths: &mut Vec<String>) {
    //     for i in tree.nodes.values_mut() {
    //         cur.push(i.name.clone());
    //         Tree::calc_size(i, cur, paths);
    //         cur.pop();
    //     }
    //     // if tree.filesize > 0 || tree.vmsize > 0{
    //     paths.push(cur.join("/"));
    //     // }

    //     let count = tree.count + tree.nodes.values().fold(0, |pre, cur| pre + cur.count);
    //     let filesize = tree.total_filesize
    //         + tree
    //             .nodes
    //             .values()
    //             .fold(0, |pre, cur| pre + cur.filesize + cur.total_filesize);
    //     let vmsize = tree.total_vmsize
    //         + tree
    //             .nodes
    //             .values()
    //             .fold(0, |pre, cur| pre + cur.vmsize + cur.total_vmsize);
    //     tree.total_filesize = filesize;
    //     tree.total_vmsize = vmsize;
    //     tree.count = count;
    // }

    fn traverse(
        &self,
        node: &Node,
        inputs: &mut HashMap<String, Input>,
        dir: Option<String>,
        deep: usize,
    ) {
        let full_path = node.name.clone();
        let dir: String = dir.map_or(full_path.clone(), |i| i + "/" + &full_path);
        if deep != 0 && dir.matches("/").count() >= deep {
            return;
        }

        let imports = node
            .nodes
            .values()
            .map(|child| Import {
                path: dir.clone() + "/" + &child.name + "__",
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
            self.traverse(child, inputs, Some(dir.clone()), deep);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tree::Tree;

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
            let tree = Tree::new(csv, None);
            assert_eq!(tree.root.nodes.len(), 1)
        }
    }
}
