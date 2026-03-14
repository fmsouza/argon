//! Argon Standard Library
//!
//! Embeds `.arg` source files for the language's standard library.
//! The prelude is always available; other modules are imported via `std:*`.

/// The Argon source for the prelude (always-available globals).
pub fn prelude_source() -> &'static str {
    include_str!("../stdlib/prelude.arg")
}

/// Resolve a `std:*` module name to its Argon source.
/// Returns `None` if the module does not exist.
pub fn resolve_std_module(name: &str) -> Option<&'static str> {
    match name {
        "io" => Some(include_str!("../stdlib/io.arg")),
        "math" => Some(include_str!("../stdlib/math.arg")),
        _ => None,
    }
}

/// List all available `std:*` module names.
pub fn available_modules() -> &'static [&'static str] {
    &["io", "math"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_contains_core_types() {
        let src = prelude_source();
        assert!(src.contains("struct Vec<T>"));
        assert!(src.contains("struct Some<T>"));
        assert!(src.contains("struct None"));
        assert!(src.contains("struct Ok<T, E>"));
        assert!(src.contains("struct Err<T, E>"));
        assert!(src.contains("struct Shared<T>"));
        assert!(src.contains("struct Map<K, V>"));
        assert!(src.contains("struct Set<T>"));
    }

    #[test]
    fn resolves_math_module() {
        let src = resolve_std_module("math").expect("math module should exist");
        assert!(src.contains("function sqrt("));
        assert!(src.contains("PI"));
        assert!(src.contains("function sin("));
    }

    #[test]
    fn resolves_io_module() {
        let src = resolve_std_module("io").expect("io module should exist");
        assert!(src.contains("function print("));
        assert!(src.contains("function println("));
    }

    #[test]
    fn unknown_module_returns_none() {
        assert!(resolve_std_module("nonexistent").is_none());
    }

    #[test]
    fn available_modules_lists_all() {
        let mods = available_modules();
        assert!(mods.contains(&"io"));
        assert!(mods.contains(&"math"));
    }
}
