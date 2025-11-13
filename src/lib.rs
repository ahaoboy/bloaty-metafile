use serde_metafile::Metafile;
use tree::Tree;

mod error;
mod packages;
mod tool;
mod tree;

pub use error::{BloatyError, Result};

/// Convert bloaty CSV output to esbuild metafile format
///
/// # Arguments
///
/// * `csv` - CSV string containing bloaty output with sections, symbols, vmsize, and filesize columns
/// * `name` - Name for the output binary in the metafile
/// * `lock` - Optional path to Cargo.lock file for dependency resolution (defaults to "Cargo.lock")
/// * `deep` - Maximum depth for tree traversal (0 means unlimited)
/// * `no_sections` - If true, exclude section-level entries from the output
///
/// # Returns
///
/// Returns a `Result` containing the generated `Metafile` or a `BloatyError` if parsing fails
///
/// # Example
///
/// ```no_run
/// use bloaty_metafile::from_csv;
///
/// let csv = "sections,symbols,vmsize,filesize\n.text,main,1000,1000";
/// let metafile = from_csv(csv, "binary", None, 0, false)?;
/// # Ok::<(), bloaty_metafile::BloatyError>(())
/// ```
pub fn from_csv(
    csv: &str,
    name: &str,
    lock: Option<String>,
    deep: usize,
    no_sections: bool,
) -> Result<Metafile> {
    let tree = Tree::new(csv, lock, no_sections)?;
    Ok(tree.to_metafile(name, deep))
}
