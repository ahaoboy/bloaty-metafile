use serde_metafile::Metafile;
use tree::Tree;
mod packages;
mod tool;
mod tree;

pub fn from_csv(
    csv: &str,
    name: &str,
    lock: Option<String>,
    deep: usize,
    no_sections: bool,
) -> Metafile {
    Tree::new(csv, lock, no_sections).to_metafile(name, deep)
}
