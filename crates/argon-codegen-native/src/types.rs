//! Argon type to Cranelift type mapping.

use cranelift_codegen::ir::types;
use cranelift_codegen::ir::Type;

/// Returns the Cranelift type for a pointer on the given pointer size.
pub fn pointer_type(pointer_bytes: u8) -> Type {
    match pointer_bytes {
        8 => types::I64,
        4 => types::I32,
        _ => types::I64,
    }
}
