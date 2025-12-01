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
/// Supports both regular symbols and angle bracket symbols:
/// - Regular: `crate::module::func`
/// - Trait impl: `<&core::alloc::layout::Layout as core::fmt::Debug>::fmt`
/// - Type method: `<url::Url>::set_password`
/// - Nested: `<u8 as <[_]>::to_vec_in::ConvertVec>::to_vec::<>`
///   Returns the crate name and all symbol parts if valid, None otherwise
pub fn get_crate_name(symbols: &str) -> Option<(String, Vec<String>)> {
    // Handle angle bracket symbols (trait impls, type methods)
    if symbols.starts_with('<') {
        // Find matching '>' for the outer angle bracket
        let mut depth = 0;
        let mut close_pos = None;
        for (i, c) in symbols.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let close_pos = close_pos?;
        let inner = &symbols[1..close_pos];

        // Check for trait impl pattern: `<Type as Trait>` or simple type: `<Type>`
        // For nested patterns like `<u8 as <[_]>::to_vec_in::ConvertVec>`, find " as " not inside nested <>
        let type_part = find_type_part(inner);

        // Extract crate name from type part (skip leading & or *)
        let type_clean = type_part.trim_start_matches(['&', '*']);
        let type_crate = type_clean.split("::").next()?;

        if !symbol_is_crate(type_crate) {
            return None;
        }

        // Build symbol parts: crate + type path + method
        let mut parts = Vec::with_capacity(4);
        parts.push(type_crate.to_string());
        parts.extend(type_clean.split("::").skip(1).map(String::from));

        // Add method name after `>::`
        if let Some(method) = symbols[close_pos..].strip_prefix(">::") {
            // Filter out empty parts and generic markers like `<>`
            parts.extend(
                method
                    .split("::")
                    .filter(|s| !s.is_empty() && *s != "<>")
                    .map(|s| s.trim_end_matches("::<>").to_string()),
            );
        }

        // For primitive types like u8, we need at least the type + method
        if parts.len() > 1 {
            return Some((parts[0].clone(), parts));
        }
        return None;
    }

    // Handle regular symbols
    let mut parts_iter = symbols.split("::");
    let first = parts_iter.next()?;

    if !symbol_is_crate(first) {
        return None;
    }

    let mut parts = Vec::with_capacity(4);
    parts.push(first.to_string());
    parts.extend(parts_iter.map(String::from));

    if parts.len() > 1 {
        Some((parts[0].clone(), parts))
    } else {
        None
    }
}

/// Find the type part in an angle bracket expression
/// Handles nested angle brackets like `<u8 as <[_]>::to_vec_in::ConvertVec>`
fn find_type_part(inner: &str) -> &str {
    // Find " as " that is not inside nested angle brackets
    let mut depth = 0;
    let bytes = inner.as_bytes();
    let as_pattern = b" as ";

    for i in 0..inner.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => depth -= 1,
            b' ' if depth == 0 && i + 4 <= inner.len() && &bytes[i..i + 4] == as_pattern => {
                return &inner[..i];
            }
            _ => {}
        }
    }
    inner
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
    use super::{get_crate_name, symbol_is_crate};

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

    #[test]
    fn test_get_crate_name_angle_bracket() {
        // Test trait impl: <&core::alloc::layout::Layout as core::fmt::Debug>::fmt
        let result = get_crate_name("<&core::alloc::layout::Layout as core::fmt::Debug>::fmt");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "core");
        assert_eq!(parts, vec!["core", "alloc", "layout", "Layout", "fmt"]);

        // Test type method: <url::Url>::set_password
        let result = get_crate_name("<url::Url>::set_password");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "url");
        assert_eq!(parts, vec!["url", "Url", "set_password"]);

        // Test: <core::alloc::layout::LayoutError as core::fmt::Debug>::fmt
        let result = get_crate_name("<core::alloc::layout::LayoutError as core::fmt::Debug>::fmt");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "core");
        assert_eq!(parts, vec!["core", "alloc", "layout", "LayoutError", "fmt"]);

        // Test regular symbol still works
        let result = get_crate_name("llrt_utils::clone::structured_clone");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "llrt_utils");
        assert_eq!(parts, vec!["llrt_utils", "clone", "structured_clone"]);

        // Test nested angle brackets: <u8 as <[_]>::to_vec_in::ConvertVec>::to_vec::<>
        let result = get_crate_name("<u8 as <[_]>::to_vec_in::ConvertVec>::to_vec::<>");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "u8");
        assert_eq!(parts, vec!["u8", "to_vec"]);
    }
}
