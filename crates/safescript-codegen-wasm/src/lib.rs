//! SafeScript - WebAssembly code generator

use safescript_ir::*;

pub struct WasmCodegen {
    module: wasm_encoder::Module,
}

impl WasmCodegen {
    pub fn new() -> Self {
        Self {
            module: wasm_encoder::Module::new(),
        }
    }

    pub fn generate(&mut self, module: &Module) -> Result<Vec<u8>, CodegenError> {
        Ok(vec![])
    }
}

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(String),
}
