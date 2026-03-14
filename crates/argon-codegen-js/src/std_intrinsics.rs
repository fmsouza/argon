//! Maps Argon `std:*` module symbols to their JavaScript equivalents.

/// Given a std module name and a symbol name, return the JS expression.
/// Returns `None` for symbols that need a polyfill (emitted separately).
pub(crate) fn js_intrinsic(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        // std:math — functions
        ("math", "abs") => Some("Math.abs"),
        ("math", "floor") => Some("Math.floor"),
        ("math", "ceil") => Some("Math.ceil"),
        ("math", "round") => Some("Math.round"),
        ("math", "trunc") => Some("Math.trunc"),
        ("math", "sign") => Some("Math.sign"),
        ("math", "min") => Some("Math.min"),
        ("math", "max") => Some("Math.max"),
        ("math", "sqrt") => Some("Math.sqrt"),
        ("math", "cbrt") => Some("Math.cbrt"),
        ("math", "pow") => Some("Math.pow"),
        ("math", "hypot") => Some("Math.hypot"),
        ("math", "sin") => Some("Math.sin"),
        ("math", "cos") => Some("Math.cos"),
        ("math", "tan") => Some("Math.tan"),
        ("math", "asin") => Some("Math.asin"),
        ("math", "acos") => Some("Math.acos"),
        ("math", "atan") => Some("Math.atan"),
        ("math", "atan2") => Some("Math.atan2"),
        ("math", "log") => Some("Math.log"),
        ("math", "log2") => Some("Math.log2"),
        ("math", "log10") => Some("Math.log10"),
        ("math", "exp") => Some("Math.exp"),

        // std:math — constants
        ("math", "PI") => Some("Math.PI"),
        ("math", "E") => Some("Math.E"),
        ("math", "LN2") => Some("Math.LN2"),
        ("math", "LN10") => Some("Math.LN10"),
        ("math", "SQRT2") => Some("Math.SQRT2"),
        ("math", "TAU") => Some("(Math.PI * 2)"),

        // clamp has no JS native equivalent
        ("math", "clamp") => None,

        // std:io — functions (provided by runtime IIFE)
        ("io", "print") => Some("print"),
        ("io", "println") => Some("println"),

        _ => None,
    }
}

/// Returns JS polyfill code for intrinsics that have no native JS equivalent.
pub(crate) fn js_polyfill(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        ("math", "clamp") => Some("function(x, lo, hi) { return Math.min(Math.max(x, lo), hi); }"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_math_functions() {
        assert_eq!(js_intrinsic("math", "sqrt"), Some("Math.sqrt"));
        assert_eq!(js_intrinsic("math", "sin"), Some("Math.sin"));
        assert_eq!(js_intrinsic("math", "PI"), Some("Math.PI"));
        assert_eq!(js_intrinsic("math", "TAU"), Some("(Math.PI * 2)"));
    }

    #[test]
    fn clamp_needs_polyfill() {
        assert_eq!(js_intrinsic("math", "clamp"), None);
        assert!(js_polyfill("math", "clamp").is_some());
    }

    #[test]
    fn maps_io_functions() {
        assert_eq!(js_intrinsic("io", "print"), Some("print"));
        assert_eq!(js_intrinsic("io", "println"), Some("println"));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(js_intrinsic("math", "nonexistent"), None);
        assert_eq!(js_intrinsic("unknown", "sqrt"), None);
    }
}
