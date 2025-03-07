use crate::packages::Packages;

pub const ROOT_NAME: &str = "ROOT";
pub const UNKNOWN_NAME: &str = "UNKNOWN";
pub const SECTIONS_NAME: &str = "SECTIONS";

pub fn symbol_is_crate(s: &str) -> bool {
    if ["..", " "].iter().any(|i| s.contains(i)) {
        return false;
    }
    !s.starts_with('[')
}

pub fn get_crate_name(symbols: &str) -> Option<(String, Vec<String>)> {
    let symbols_parts: Vec<String> = symbols
        .trim_end_matches(":")
        .split("::")
        .map(|i| i.to_string())
        .collect();
    if symbols_parts.len() == 1 || !symbol_is_crate(&symbols_parts[0]) {
        return None;
    }
    Some((symbols_parts[0].clone(), symbols_parts.clone()))
}

pub fn get_path_from_record(symbols: String, sections: String, packages: &Packages) -> Vec<String> {
    match get_crate_name(&symbols) {
        None => vec![SECTIONS_NAME.to_string(), sections.to_string()]
            .into_iter()
            .chain(symbols.trim_end_matches(':').split("::").map(String::from))
            .collect(),
        Some((crate_name, symbols_parts)) => {
            // crate
            // .text,llrt_utils::clone::structured_clone -> llrt/llrt_utils/.text/clone/structured_clone
            let mut prefix = packages.get_path(&crate_name);
            prefix.reverse();
            prefix.push(crate_name.to_string());
            prefix.push(sections.to_string());
            prefix.extend_from_slice(&symbols_parts[1..]);
            prefix
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tool::symbol_is_crate;

    #[test]
    fn test_symbol_is_crate() {
        for (a, b) in [
            ("[16482 Others]", false),
            (
                "_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$",
                false,
            ),
        ] {
            assert_eq!(symbol_is_crate(a), b);
        }
    }
}
