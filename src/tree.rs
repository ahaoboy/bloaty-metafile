use crate::{
    packages::Packages,
    tool::{ROOT_NAME, SECTIONS_NAME, UNKNOWN_NAME, get_path_from_record},
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
    pub fn new(csv: &str, lock: Option<String>, no_sections: bool) -> Tree {
        let mut tree = Tree {
            root: Node {
                name: ROOT_NAME.to_string(),
                vmsize: 0,
                filesize: 0,
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

        for record in records {
            let sym = if record.symbols.is_empty() {
                UNKNOWN_NAME.to_string()
            } else {
                record.symbols
            };
            let path = get_path_from_record(sym, record.sections, &packages);
            if no_sections && path[0] == SECTIONS_NAME {
                continue;
            }
            tree.add_path(&path, record.vmsize, record.filesize);
        }

        tree
    }

    pub fn to_metafile(&self, name: &str, deep: usize) -> Metafile {
        let root = &self.root;
        let mut inputs = HashMap::new();
        for i in root.nodes.values() {
            i.traverse(&mut inputs, None, deep);
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

    fn add_path(&mut self, path: &[String], vmsize: u64, filesize: u64) {
        let mut current = &mut self.root;
        for (i, part) in path.iter().enumerate() {
            current.total_vmsize += vmsize;
            current.total_filesize += filesize;
            if i == path.len() - 1 {
                let n = current.nodes.entry(part.clone()).or_insert(Node {
                    name: part.clone(),
                    vmsize,
                    filesize,
                    nodes: HashMap::new(),
                    total_filesize: 0,
                    total_vmsize: 0,
                });
                n.vmsize = vmsize;
                n.filesize = filesize;
            } else {
                current = current.nodes.entry(part.clone()).or_insert(Node {
                    name: part.clone(),
                    vmsize: 0,
                    filesize: 0,
                    nodes: HashMap::new(),
                    total_filesize: 0,
                    total_vmsize: 0,
                });
            }
        }
    }
}

impl Node {
    fn traverse(&self, inputs: &mut HashMap<String, Input>, dir: Option<String>, deep: usize) {
        let full_path = self.name.clone();
        let dir: String = dir.map_or(full_path.clone(), |i| i + "/" + &full_path);
        if deep != 0 && dir.matches("/").count() >= deep {
            return;
        }

        let imports = self
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
            bytes: self.filesize,
            imports,
            format: None,
            with: None,
        };
        inputs.insert(dir.clone(), input);
        for child in self.nodes.values() {
            child.traverse(inputs, Some(dir.clone()), deep);
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
            let tree = Tree::new(csv, None, false);
            assert_eq!(tree.root.nodes.len(), 1)
        }
    }
}
