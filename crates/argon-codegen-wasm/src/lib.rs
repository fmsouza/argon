//! Argon - WebAssembly code generator

use argon_ir::{
    BasicBlock, ConstValue, Function as IrFunction, Instruction as IrInstruction,
    Module as IrModule, Param, Terminator,
};
use wasm_encoder::*;

pub struct WasmCodegen {}

impl WasmCodegen {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate_from_ast(
        &mut self,
        source: &argon_ast::SourceFile,
    ) -> Result<Vec<u8>, CodegenError> {
        let mut functions = Vec::new();

        for stmt in &source.statements {
            if let argon_ast::Stmt::Function(f) = stmt {
                let func_name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();

                let params: Vec<Param> = f
                    .params
                    .iter()
                    .filter_map(|p| {
                        if let argon_ast::Pattern::Identifier(id) = &p.pat {
                            Some(Param {
                                name: id.name.sym.clone(),
                                ty: 0,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                let body = vec![BasicBlock {
                    id: 0,
                    instructions: vec![
                        IrInstruction::Const {
                            dest: 0,
                            value: ConstValue::Number(42.0),
                        },
                        IrInstruction::Return { value: Some(0) },
                    ],
                    terminator: Terminator::Return(Some(0)),
                }];

                functions.push(IrFunction {
                    id: func_name,
                    params,
                    return_type: Some(0),
                    body,
                });
            }
        }

        let module = IrModule {
            functions,
            types: Vec::new(),
            globals: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        };
        self.generate(&module)
    }

    pub fn generate(&self, ir_module: &IrModule) -> Result<Vec<u8>, CodegenError> {
        let mut module = Module::new();

        // Type section: () -> i32
        let mut types = TypeSection::new();
        for _ in ir_module.functions.iter() {
            let params = vec![];
            let results = vec![ValType::I32];
            types.function(params, results);
        }
        module.section(&types);

        // Function section: map functions to type indices
        let mut funcs = FunctionSection::new();
        for (i, _) in ir_module.functions.iter().enumerate() {
            funcs.function(i as u32);
        }
        module.section(&funcs);

        // Export section
        let mut exports = ExportSection::new();
        for (i, func) in ir_module.functions.iter().enumerate() {
            exports.export(func.id.as_str(), ExportKind::Func, i as u32);
        }
        module.section(&exports);

        // Code section
        let mut codes = CodeSection::new();
        for _ in ir_module.functions.iter() {
            let locals = vec![];
            let mut f = Function::new(locals);
            f.instruction(&Instruction::I32Const(42));
            f.instruction(&Instruction::End);
            codes.function(&f);
        }
        module.section(&codes);

        Ok(module.finish())
    }
}

impl Default for WasmCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(String),
    IrError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            CodegenError::IrError(msg) => write!(f, "IR error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}
