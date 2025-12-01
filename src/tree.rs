use crate::{
    error::{BloatyError, Result},
    packages::Packages,
    tool::{ROOT_NAME, SECTIONS_NAME, UNKNOWN_NAME, get_path_from_record},
};
use cargo_lock::Lockfile;
use serde::Deserialize;
use serde_metafile::{Import, Input, InputDetail, Metafile, Output};
use std::collections::HashMap;

/// Tree node representing a symbol or section in the binary
/// Contains size information and child nodes
#[derive(Debug, Clone)]
pub struct Node {
    pub name: Box<str>,
    pub vmsize: u64,
    pub filesize: u64,
    pub total_vmsize: u64,
    pub total_filesize: u64,
    pub nodes: HashMap<Box<str>, Node>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: String::new().into_boxed_str(),
            vmsize: 0,
            filesize: 0,
            total_vmsize: 0,
            total_filesize: 0,
            nodes: HashMap::new(),
        }
    }
}

/// CSV record from bloaty output
/// Contains section name, symbol name, virtual memory size, and file size
#[derive(Debug, Deserialize)]
pub struct SectionRecord {
    pub sections: String,
    pub symbols: String,
    pub vmsize: u64,
    pub filesize: u64,
}

/// Hierarchical tree structure for organizing binary symbols and sections
pub struct Tree {
    root: Node,
}

impl Tree {
    /// Create a new tree from CSV data and optional Cargo.lock file
    /// Parses CSV records and builds a hierarchical structure
    pub fn new(csv: &str, lock: Option<String>, no_sections: bool) -> Result<Tree> {
        let mut tree = Tree {
            root: Node {
                name: ROOT_NAME.to_string().into_boxed_str(),
                vmsize: 0,
                filesize: 0,
                nodes: HashMap::new(),
                total_filesize: 0,
                total_vmsize: 0,
            },
        };

        // Parse CSV records
        let mut rdr = csv::Reader::from_reader(csv.as_bytes());
        let records: Vec<_> = rdr
            .deserialize::<SectionRecord>()
            .collect::<std::result::Result<Vec<_>, csv::Error>>()
            .map_err(BloatyError::CsvParse)?;

        // Load Cargo.lock and resolve package dependencies
        let lock_path = lock.unwrap_or_else(|| "Cargo.lock".to_string());
        let packages = Lockfile::load(&lock_path)
            .map_err(|source| BloatyError::LockfileLoad {
                path: lock_path.clone(),
                source,
            })
            .and_then(|lock| {
                lock.dependency_tree()
                    .map_err(|source| BloatyError::LockfileLoad {
                        path: lock_path.clone(),
                        source,
                    })
            })
            .map(|dep_tree| Packages::new(&dep_tree, &records))
            .unwrap_or_default();

        // Build tree from records
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

        Ok(tree)
    }

    /// Convert the tree to an esbuild metafile format
    /// Traverses the tree and generates the metafile structure
    pub fn to_metafile(&self, name: &str, deep: usize) -> Metafile {
        let root = &self.root;

        // Pre-allocate HashMap with estimated capacity
        let mut inputs = HashMap::with_capacity(root.nodes.len() * 4);

        // Traverse all root nodes to build inputs
        for node in root.nodes.values() {
            node.traverse(&mut inputs, None, deep);
        }

        // Build output_inputs using iterator chain
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

    /// Add a path to the tree with associated size information
    /// Creates intermediate nodes as needed
    fn add_path(&mut self, path: &[String], vmsize: u64, filesize: u64) {
        let mut current = &mut self.root;
        let last_idx = path.len() - 1;

        for (i, part) in path.iter().enumerate() {
            current.total_vmsize += vmsize;
            current.total_filesize += filesize;

            let is_leaf = i == last_idx;
            let part_boxed: Box<str> = part.as_str().into();

            // Use entry API to avoid double lookup
            current = current.nodes.entry(part_boxed.clone()).or_insert_with(|| {
                Node::create_node(
                    part_boxed.clone(),
                    0, // Initialize with 0, will be accumulated below
                    0,
                    is_leaf,
                )
            });

            // Accumulate leaf node values (don't overwrite)
            if is_leaf {
                current.vmsize += vmsize;
                current.filesize += filesize;
            }
        }
    }
}

impl Node {
    /// Helper function to create a new node with given parameters
    #[inline]
    fn create_node(name: Box<str>, vmsize: u64, filesize: u64, is_leaf: bool) -> Self {
        Self {
            name,
            vmsize,
            filesize,
            nodes: if is_leaf {
                HashMap::new()
            } else {
                HashMap::with_capacity(4) // Pre-allocate for intermediate nodes
            },
            total_filesize: 0,
            total_vmsize: 0,
        }
    }

    /// Recursively traverse the tree to build metafile inputs
    /// Respects the depth limit if specified
    fn traverse(&self, inputs: &mut HashMap<String, Input>, dir: Option<String>, deep: usize) {
        // Build directory path with capacity pre-allocation
        let dir: String = match &dir {
            Some(parent) => {
                let mut path = String::with_capacity(parent.len() + 1 + self.name.len());
                path.push_str(parent);
                path.push('/');
                path.push_str(&self.name);
                path
            }
            None => self.name.to_string(),
        };

        let current_depth = dir.matches('/').count();

        // Check if we're at the depth limit
        let at_depth_limit = deep != 0 && current_depth >= deep;

        // Build imports (only if not at depth limit)
        let imports: Vec<Import> = if at_depth_limit {
            vec![]
        } else {
            self.nodes
                .values()
                .map(|child| {
                    let mut import_path = String::with_capacity(dir.len() + 1 + child.name.len());
                    import_path.push_str(&dir);
                    import_path.push('/');
                    import_path.push_str(&child.name);
                    Import {
                        path: import_path,
                        kind: None,
                        external: false,
                        original: None,
                        with: None,
                    }
                })
                .collect()
        };

        // Use total_filesize when at depth limit to include all children's sizes
        let bytes = if at_depth_limit {
            self.total_filesize
        } else {
            self.filesize
        };

        let input = Input {
            bytes,
            imports,
            format: None,
            with: None,
        };

        inputs.insert(dir.clone(), input);

        // Recurse into children only if not at depth limit
        if !at_depth_limit && !self.nodes.is_empty() {
            let dir_ref = Some(dir);
            for child in self.nodes.values() {
                child.traverse(inputs, dir_ref.clone(), deep);
            }
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
            let tree = Tree::new(csv, None, false).expect("Failed to create tree");
            assert_eq!(tree.root.nodes.len(), 1)
        }
    }
}
