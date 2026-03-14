//! Argon - Native code generator using Cranelift.
//!
//! Translates Argon IR to native machine code via Cranelift, producing
//! object files that can be linked into executables.

mod intrinsics;
mod linker;
mod lower;
mod runtime;
mod types;

use argon_ir::Module as IrModule;
use argon_target::TargetTriple;
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_object::{ObjectBuilder, ObjectModule};

pub use linker::{link, LinkerConfig};
pub use runtime::compile_c_runtime;

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(String),
    IrError(String),
    CraneliftError(String),
    LinkerError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Unsupported(msg) => write!(f, "unsupported for native target: {}", msg),
            CodegenError::IrError(msg) => write!(f, "IR error: {}", msg),
            CodegenError::CraneliftError(msg) => write!(f, "codegen error: {}", msg),
            CodegenError::LinkerError(msg) => write!(f, "linker error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

pub struct NativeCodegen {
    triple: TargetTriple,
}

impl NativeCodegen {
    pub fn new(triple: TargetTriple) -> Self {
        Self { triple }
    }

    /// Generate an object file (.o) from the Argon IR module.
    pub fn generate(&self, ir_module: &IrModule) -> Result<Vec<u8>, CodegenError> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;
        flag_builder
            .set("is_pic", "true")
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        let isa_builder = isa::lookup(self.triple.triple.clone())
            .map_err(|e| CodegenError::CraneliftError(format!("unsupported target: {}", e)))?;

        let flags = settings::Flags::new(flag_builder);
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        let obj_builder = ObjectBuilder::new(
            isa,
            "argon_module",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        let mut object_module = ObjectModule::new(obj_builder);

        let mut lowerer = lower::ModuleLowerer::new(&mut object_module, &self.triple);
        lowerer.lower_module(ir_module)?;

        let product = object_module.finish();
        product
            .emit()
            .map_err(|e| CodegenError::CraneliftError(format!("failed to emit object: {}", e)))
    }

    /// Get the target triple.
    pub fn triple(&self) -> &TargetTriple {
        &self.triple
    }
}
