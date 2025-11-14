use crate::packages::Packages;

pub const ROOT_NAME: &str = "ROOT";
pub const UNKNOWN_NAME: &str = "UNKNOWN";
pub const SECTIONS_NAME: &str = "SECTIONS";

/// Check if a symbol string represents a valid crate name
/// Returns false if the symbol contains invalid patterns or is a special marker
#[inline]
pub fn symbol_is_crate(s: &str) -> bool {
    // Reject symbols with invalid patterns: ".." or spaces
    // Reject special markers that start with '['
    !s.contains("..") && !s.contains(' ') && !s.starts_with('[')
}

/// Extract crate name and symbol parts from a symbol string
/// Returns the crate name and all symbol parts if valid, None otherwise
pub fn get_crate_name(symbols: &str) -> Option<(String, Vec<String>)> {
    // Split symbol string by "::" separator
    let mut parts = symbols.split("::");

    // Check if first part is a valid crate name
    let first = parts.next()?;
    if !symbol_is_crate(first) {
        return None;
    }

    // Collect all parts into a vector
    let mut symbols_parts = Vec::with_capacity(4); // Pre-allocate for typical symbol depth
    symbols_parts.push(first.to_string());
    symbols_parts.extend(parts.map(String::from));

    // Need at least 2 parts (crate::symbol)
    if symbols_parts.len() > 1 {
        Some((symbols_parts[0].clone(), symbols_parts))
    } else {
        None
    }
}

/// Build a hierarchical path from a symbol record
/// Combines package dependencies, sections, and symbol parts into a single path
pub fn get_path_from_record(symbols: String, sections: String, packages: &Packages) -> Vec<String> {
    match get_crate_name(&symbols) {
        None => {
            // No crate found: build path from sections
            // Pre-allocate capacity for sections + symbol parts
            let symbol_parts_count = symbols.matches("::").count() + 1;
            let mut path = Vec::with_capacity(2 + symbol_parts_count);
            path.push(SECTIONS_NAME.to_string());
            path.push(sections);
            path.extend(symbols.split("::").map(String::from));
            path
        }
        Some((crate_name, symbols_parts)) => {
            // Build path: crate dependency path + section + symbol parts
            // Example: .text,llrt_utils::clone::structured_clone -> llrt/llrt_utils/.text/clone/structured_clone
            let pkg_path = packages.get_path(&crate_name);
            let mut path = Vec::with_capacity(pkg_path.len() + 1 + symbols_parts.len() - 1);
            path.extend_from_slice(pkg_path);
            path.push(sections);
            path.extend_from_slice(&symbols_parts[1..]);
            path
        }
    }
}

#[cfg(test)]
mod test {
    use super::symbol_is_crate;

    #[test]
    fn test_symbol_is_crate() {
        let test_cases = [
            ("[16482 Others]", false),
            (
                "_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$",
                false,
            ),
            ("valid_crate", true),
            ("another::valid::crate", true),
            ("invalid crate", false),
            ("..invalid", false),
        ];

        for (symbol, expected) in test_cases.iter() {
            assert_eq!(
                symbol_is_crate(symbol),
                *expected,
                "Failed for symbol: {}",
                symbol
            );
        }
    }
}
