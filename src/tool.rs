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
/// - Closure: `std::sys::backtrace::_print_fmt::{closure#1}::{closure#0}`
/// - Trait impl: `<&core::alloc::layout::Layout as core::fmt::Debug>::fmt`
/// - Type method: `<url::Url>::set_password`
/// - Nested: `<u8 as <[_]>::to_vec_in::ConvertVec>::to_vec::<>`
/// - Double angle: `<<Type as Trait>::method as OtherTrait>::func`
///   Returns the crate name and all symbol parts if valid, None otherwise
pub fn get_crate_name(symbols: &str) -> Option<(String, Vec<String>)> {
    // Handle angle bracket symbols (trait impls, type methods)
    if symbols.starts_with('<') {
        return parse_angle_bracket_symbol(symbols);
    }

    // Handle regular symbols (including closures like {closure#0})
    let parts = split_symbol_parts(symbols);

    if parts.is_empty() {
        return None;
    }

    let first = &parts[0];
    if !symbol_is_crate(first) {
        return None;
    }

    if parts.len() > 1 {
        Some((parts[0].clone(), parts))
    } else {
        None
    }
}

/// Parse angle bracket symbols like trait impls and type methods
/// Extracts only the innermost type path and the outermost method name
fn parse_angle_bracket_symbol(symbols: &str) -> Option<(String, Vec<String>)> {
    // Extract innermost type and outermost method
    let (inner_type, outer_method) = extract_inner_type_and_outer_method(symbols)?;

    let mut parts = Vec::with_capacity(4);

    // Add type path parts
    let type_clean = inner_type.trim_start_matches(['&', '*']);
    for part in split_symbol_parts(type_clean) {
        if !part.is_empty() && part != "<>" {
            parts.push(part);
        }
    }

    // Add outer method
    if !outer_method.is_empty() {
        for part in split_symbol_parts(&outer_method) {
            if !part.is_empty() && part != "<>" {
                parts.push(part);
            }
        }
    }

    if parts.len() > 1 {
        let crate_name = parts[0].clone();
        if symbol_is_crate(&crate_name) {
            return Some((crate_name, parts));
        }
    }
    None
}

/// Extract the innermost type and outermost method from nested angle bracket expression
/// For `<<u64 as Trait1>::method1 as Trait2>::method2` returns ("u64", "method2")
fn extract_inner_type_and_outer_method(s: &str) -> Option<(String, String)> {
    let s = s.trim();

    if !s.starts_with('<') {
        return None;
    }

    // Find matching '>' for the outermost angle bracket
    let mut depth = 0;
    let mut close_pos = None;
    for (i, c) in s.char_indices() {
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

    // Get outer method (after `>::`)
    let outer_method = s[close_pos..].strip_prefix(">::").unwrap_or("").to_string();

    // Get inner content
    let inner = &s[1..close_pos];

    // Find type part (before " as " at depth 0)
    let type_part = find_type_part(inner);

    // If type_part starts with '<', recursively extract innermost type
    if type_part.starts_with('<') {
        let (inner_type, _) = extract_inner_type_and_outer_method(type_part)?;
        Some((inner_type, outer_method))
    } else {
        Some((type_part.to_string(), outer_method))
    }
}

/// Split symbol string into parts, handling special syntax like {closure#0}, {shim:vtable#0}, ::<>
fn split_symbol_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    let mut brace_depth = 0;
    let mut angle_depth = 0;

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                brace_depth += 1;
                current.push(c);
            }
            '}' => {
                brace_depth -= 1;
                current.push(c);
            }
            '<' => {
                angle_depth += 1;
                current.push(c);
            }
            '>' => {
                angle_depth -= 1;
                current.push(c);
            }
            ':' if brace_depth == 0 && angle_depth == 0 => {
                // Check for `::`
                if chars.peek() == Some(&':') {
                    chars.next(); // consume second ':'
                    if !current.is_empty() {
                        parts.push(current);
                        current = String::new();
                    }
                } else {
                    current.push(c);
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    // Filter out empty generic markers
    parts
        .into_iter()
        .filter(|p| !p.is_empty() && p != "<>")
        .collect()
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

        // Test closure syntax
        let result = get_crate_name("std::sys::backtrace::_print_fmt::{closure#1}::{closure#0}");
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "std");
        assert_eq!(
            parts,
            vec![
                "std",
                "sys",
                "backtrace",
                "_print_fmt",
                "{closure#1}",
                "{closure#0}"
            ]
        );

        // Test shim syntax
        let result = get_crate_name(
            "<signal_hook_registry::register::<>::{closure#0} as core::ops::function::FnOnce::<>>::call_once::{shim:vtable#0}",
        );
        assert!(result.is_some());
        let (crate_name, _parts) = result.unwrap();
        assert_eq!(crate_name, "signal_hook_registry");

        // Test double angle bracket with multiple "as" - should only keep innermost type + outermost method
        let result = get_crate_name(
            "<<u64 as serde_core::de::Deserialize>::deserialize::PrimitiveVisitor as serde_core::de::Visitor>::expecting",
        );
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "u64");
        assert_eq!(parts, vec!["u64", "expecting"], "parts: {:?}", parts);

        // Test another complex case with multiple as
        let result = get_crate_name(
            "<<easy_install::manfiest::AssetKind as serde_core::de::Deserialize>::deserialize::__FieldVisitor as serde_core::de::Visitor>::expecting",
        );
        assert!(result.is_some());
        let (crate_name, parts) = result.unwrap();
        assert_eq!(crate_name, "easy_install");
        assert_eq!(
            parts,
            vec!["easy_install", "manfiest", "AssetKind", "expecting"],
            "parts: {:?}",
            parts
        );
    }
}
