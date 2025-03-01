use serde_metafile::{Import, Input, InputDetail, Metafile, Output};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub vmsize: u64,
    pub filesize: u64,
    pub count: u64,
    pub nodes: HashMap<String, Node>,
}

impl Node {
    fn add_path(&mut self, path: &[&str], vmsize: u64, filesize: u64) {
        let mut current = self;
        for i in 0..path.len() {
            let part = path[i];
            current.vmsize += vmsize;
            current.filesize += filesize;
            current.count += 1;
            if i == path.len() - 1 {
                current.nodes.entry(part.to_string()).or_insert(Node {
                    name: part.to_string(),
                    vmsize: vmsize,
                    filesize: filesize,
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

pub fn get_tree(csv: &str) -> Node {
    let mut tree = Node {
        name: "__ROOT__".to_string(),
        vmsize: 0,
        filesize: 0,
        count: 0,
        nodes: HashMap::new(),
    };

    for line in csv.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let section = parts[0];
        let symbols = parts[1];
        let vmsize: u64 = parts[2].parse().unwrap();
        let filesize: u64 = parts[3].parse().unwrap();
        let mut symbols_parts: Vec<&str> = symbols.split("::").collect();
        // FIXME: filter crate name
        if symbols_parts.len() == 1 || symbols_parts.get(0).unwrap_or(&"").contains("..") {
            symbols_parts.insert(0, section);
        } else {
            symbols_parts.insert(1, section);
        };

        tree.add_path(&symbols_parts, vmsize, filesize);
    }
    tree
}

pub fn from_csv(csv: &str) -> Metafile {
    let tree = get_tree(csv);
    let meta = convert_node_to_metafile(tree);
    meta
}

pub fn convert_node_to_metafile(root: Node) -> Metafile {
    let mut inputs = HashMap::new();
    for i in &root.nodes {
        traverse(&i.1, &mut inputs, None);
    }
    let entry_point_path = root.name.clone();
    let meta_name = "Binary-Size-Analyzer".to_string();
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
    let outputs = HashMap::from([(meta_name, output)]);
    Metafile { inputs, outputs }
}

fn traverse(node: &Node, inputs: &mut HashMap<String, Input>, dir: Option<String>) {
    let full_path = node.name.clone();
    let dir: String = dir.map_or(full_path.clone(), |i| i + "/" + &full_path);
    // FIXME: Add parameters to filter nodes that are too deep and reduce the size of json
    if dir.matches("/").count() >= 4 {
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
        traverse(child, inputs, Some(dir.clone()));
    }
}
