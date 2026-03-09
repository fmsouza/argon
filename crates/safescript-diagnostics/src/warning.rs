//! SafeScript - Diagnostic warnings

use crate::{Warning, WarningLabel};

pub fn unused_variable(source_id: &str, span: std::ops::Range<usize>, name: &str) -> Warning {
    Warning::new(
        source_id.to_string(),
        span,
        format!("unused variable `{}`", name),
    )
    .with_label(WarningLabel {
        span,
        message: "consider using `_` if intentionally unused".to_string(),
    })
}

pub fn unreachable_code(source_id: &str, span: std::ops::Range<usize>) -> Warning {
    Warning::new(source_id.to_string(), span, "unreachable code".to_string())
}

pub fn dead_code(source_id: &str, span: std::ops::Range<usize>) -> Warning {
    Warning::new(source_id.to_string(), span, "dead code".to_string())
}

pub fn implicit_copy(source_id: &str, span: std::ops::Range<usize>, type_name: &str) -> Warning {
    Warning::new(
        source_id.to_string(),
        span,
        format!("implicit copy of `{}`", type_name),
    )
}

pub fn implicit_box(source_id: &str, span: std::ops::Range<usize>, type_name: &str) -> Warning {
    Warning::new(
        source_id.to_string(),
        span,
        format!("implicit box of `{}`", type_name),
    )
}
