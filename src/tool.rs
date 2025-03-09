use crate::packages::Packages;

pub const ROOT_NAME: &str = "ROOT";
pub const UNKNOWN_NAME: &str = "UNKNOWN";
pub const SECTIONS_NAME: &str = "SECTIONS";

pub fn symbol_is_crate(s: &str) -> bool {
    if ["..", " "].iter().any(|i| s.contains(i)) {
        return false;
    }

    // FIXME: Maybe there are other cases
    !["[", "std::", "core::", "alloc::"]
        .iter()
        .any(|i| s.starts_with(i))
}

pub fn get_crate_name(symbols: &str) -> Option<(String, Vec<String>)> {
    let symbols_parts: Vec<String> = symbols.split("::").map(String::from).collect();
    if symbols_parts.len() > 1 && symbol_is_crate(&symbols_parts[0]) {
        Some((symbols_parts[0].clone(), symbols_parts))
    } else {
        None
    }
}

pub fn get_path_from_record(symbols: String, sections: String, packages: &Packages) -> Vec<String> {
    match get_crate_name(&symbols) {
        None => {
            let mut path = vec![SECTIONS_NAME.to_string(), sections];
            path.extend(symbols.split("::").map(String::from));
            path
        }
        Some((crate_name, symbols_parts)) => {
            // crate
            // .text,llrt_utils::clone::structured_clone -> llrt/llrt_utils/.text/clone/structured_clone
            let mut prefix = packages.get_path(&crate_name);
            prefix.reverse();
            prefix.push(crate_name);
            prefix.push(sections);
            prefix.extend_from_slice(&symbols_parts[1..]);
            prefix
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
