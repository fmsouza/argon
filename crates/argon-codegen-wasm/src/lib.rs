//! Argon - WebAssembly code generator

use argon_ir::{
    BasicBlock, BinOp, ConstValue, Function as IrFunction, Instruction as IrInstruction, LogicOp,
    Module as IrModule, Terminator, UnOp, ValueId,
};
use std::collections::{BTreeSet, HashMap};
use wasm_encoder::*;

const HEAP_PTR_GLOBAL_INDEX: u32 = 0;
const PENDING_THROW: i32 = 1;
const PENDING_RETURN: i32 = 2;
const PENDING_BREAK: i32 = 3;
const PENDING_CONTINUE: i32 = 4;

pub struct WasmCodegen {}

impl WasmCodegen {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate_from_ast(
        &mut self,
        source: &argon_ast::SourceFile,
    ) -> Result<Vec<u8>, CodegenError> {
        let mut builder = argon_ir::IrBuilder::new();
        let ir = builder
            .build(source)
            .map_err(|e| CodegenError::IrError(e.to_string()))?;
        self.generate(&ir)
    }

    pub fn generate(&self, ir_module: &IrModule) -> Result<Vec<u8>, CodegenError> {
        let mut lowerer = ModuleLowerer::new(ir_module);
        lowerer.lower_module()
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

#[derive(Debug, Clone, Copy)]
struct FunctionSignature {
    params: u32,
    returns_i32: bool,
}

#[derive(Debug, Clone)]
struct ImportPlan {
    local_name: String,
    module_name: String,
    field_name: String,
    signature: FunctionSignature,
}

#[derive(Debug, Clone)]
struct FunctionPlan {
    max_value: usize,
    var_names: BTreeSet<String>,
    max_try_depth: usize,
}

#[derive(Debug, Clone)]
struct DataSegment {
    offset: u32,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeapObjectLayout {
    properties: HashMap<String, u32>,
    field_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HeapValueShape {
    Object(HeapObjectLayout),
    Array,
}

struct ModuleLowerer<'a> {
    ir_module: &'a IrModule,
    function_indices: HashMap<String, u32>,
    import_indices: HashMap<String, u32>,
    constructor_layouts: HashMap<String, HeapObjectLayout>,
    imports: Vec<ImportPlan>,
    signatures: Vec<FunctionSignature>,
    data_segments: Vec<DataSegment>,
    string_pool: HashMap<String, u32>,
    next_data_offset: u32,
    uses_memory: bool,
}

impl<'a> ModuleLowerer<'a> {
    fn new(ir_module: &'a IrModule) -> Self {
        let mut function_indices = HashMap::new();
        let imports = Self::infer_imports(ir_module);
        let import_indices = imports
            .iter()
            .enumerate()
            .map(|(idx, import)| (import.local_name.clone(), idx as u32))
            .collect::<HashMap<_, _>>();
        let mut constructor_layouts = HashMap::new();
        let mut signatures = imports
            .iter()
            .map(|import| import.signature)
            .collect::<Vec<_>>();

        for (idx, func) in ir_module.functions.iter().enumerate() {
            function_indices.insert(func.id.clone(), (imports.len() + idx) as u32);
            signatures.push(Self::infer_signature(func));
        }

        for ty in &ir_module.types {
            if let argon_ir::TypeDef::Struct { name, fields } = ty {
                constructor_layouts.insert(name.clone(), Self::layout_from_fields(fields));
            }
        }

        Self {
            ir_module,
            function_indices,
            import_indices,
            constructor_layouts,
            imports,
            signatures,
            data_segments: Vec::new(),
            string_pool: HashMap::new(),
            next_data_offset: 16,
            uses_memory: false,
        }
    }

    fn lower_module(&mut self) -> Result<Vec<u8>, CodegenError> {
        let mut compiled_functions: Vec<Function> =
            Vec::with_capacity(self.ir_module.functions.len());
        for (idx, func) in self.ir_module.functions.iter().enumerate() {
            let signature = self.signatures[self.imports.len() + idx];
            compiled_functions.push(self.lower_function(func, signature)?);
        }

        let mut module = Module::new();

        let mut types = TypeSection::new();
        for signature in &self.signatures {
            let params = vec![ValType::I32; signature.params as usize];
            let results = if signature.returns_i32 {
                vec![ValType::I32]
            } else {
                Vec::new()
            };
            types.function(params, results);
        }
        module.section(&types);

        if !self.imports.is_empty() {
            let mut imports = ImportSection::new();
            for (idx, import) in self.imports.iter().enumerate() {
                imports.import(
                    &import.module_name,
                    &import.field_name,
                    EntityType::Function(idx as u32),
                );
            }
            module.section(&imports);
        }

        let mut funcs = FunctionSection::new();
        for (i, _) in self.ir_module.functions.iter().enumerate() {
            funcs.function((self.imports.len() + i) as u32);
        }
        module.section(&funcs);

        let heap_init = align4(self.next_data_offset.max(16));
        if self.uses_memory {
            let mut memories = MemorySection::new();
            memories.memory(MemoryType {
                minimum: 1,
                maximum: None,
                memory64: false,
                shared: false,
            });
            module.section(&memories);

            let mut globals = GlobalSection::new();
            globals.global(
                GlobalType {
                    val_type: ValType::I32,
                    mutable: true,
                },
                &ConstExpr::i32_const(heap_init as i32),
            );
            module.section(&globals);
        }

        let mut exports = ExportSection::new();
        for (i, func) in self.ir_module.functions.iter().enumerate() {
            let export_name = if func.id.is_empty() {
                format!("__anon_{}", i)
            } else {
                func.id.clone()
            };
            exports.export(
                &export_name,
                ExportKind::Func,
                (self.imports.len() + i) as u32,
            );
        }
        if self.uses_memory {
            exports.export("memory", ExportKind::Memory, 0);
        }
        module.section(&exports);

        let mut codes = CodeSection::new();
        for f in compiled_functions {
            codes.function(&f);
        }
        module.section(&codes);

        if self.uses_memory && !self.data_segments.is_empty() {
            let mut data = DataSection::new();
            for segment in &self.data_segments {
                data.active(
                    0,
                    &ConstExpr::i32_const(segment.offset as i32),
                    segment.bytes.clone(),
                );
            }
            module.section(&data);
        }

        Ok(module.finish())
    }

    fn infer_signature(func: &IrFunction) -> FunctionSignature {
        let returns_i32 = func.return_type.is_some()
            || func
                .body
                .iter()
                .any(|bb| matches!(bb.terminator, Terminator::Return(Some(_))));

        FunctionSignature {
            params: func.params.len() as u32,
            returns_i32,
        }
    }

    fn infer_imports(ir_module: &IrModule) -> Vec<ImportPlan> {
        let mut arities = HashMap::new();
        for func in &ir_module.functions {
            Self::collect_import_call_arities_from_blocks(&func.body, &mut arities);
        }

        let mut imports = Vec::new();
        for import in &ir_module.imports {
            let module_name = normalize_string_literal(&import.source.value);
            for specifier in &import.specifiers {
                match specifier {
                    argon_ast::ImportSpecifier::Default(default) => {
                        let local_name = default.local.sym.clone();
                        imports.push(ImportPlan {
                            local_name: local_name.clone(),
                            module_name: module_name.clone(),
                            field_name: "default".to_string(),
                            signature: FunctionSignature {
                                params: arities.get(&local_name).copied().unwrap_or(0),
                                returns_i32: true,
                            },
                        });
                    }
                    argon_ast::ImportSpecifier::Named(named) => {
                        let local_name = named
                            .local
                            .as_ref()
                            .map(|id| id.sym.clone())
                            .unwrap_or_else(|| named.imported.sym.clone());
                        imports.push(ImportPlan {
                            local_name: local_name.clone(),
                            module_name: module_name.clone(),
                            field_name: named.imported.sym.clone(),
                            signature: FunctionSignature {
                                params: arities.get(&local_name).copied().unwrap_or(0),
                                returns_i32: true,
                            },
                        });
                    }
                    argon_ast::ImportSpecifier::Namespace(namespace) => {
                        let local_name = namespace.id.sym.clone();
                        imports.push(ImportPlan {
                            local_name: local_name.clone(),
                            module_name: module_name.clone(),
                            field_name: "*".to_string(),
                            signature: FunctionSignature {
                                params: arities.get(&local_name).copied().unwrap_or(0),
                                returns_i32: true,
                            },
                        });
                    }
                }
            }
        }

        imports
    }

    fn collect_import_call_arities_from_blocks(
        blocks: &[BasicBlock],
        arities: &mut HashMap<String, u32>,
    ) {
        let mut refs = HashMap::new();
        for block in blocks {
            Self::collect_import_call_arities_from_instructions(
                &block.instructions,
                arities,
                &mut refs,
            );
        }
    }

    fn collect_import_call_arities_from_instructions(
        instructions: &[IrInstruction],
        arities: &mut HashMap<String, u32>,
        refs: &mut HashMap<ValueId, String>,
    ) {
        for inst in instructions {
            match inst {
                IrInstruction::VarRef { dest, name } => {
                    refs.insert(*dest, name.clone());
                }
                IrInstruction::Call { callee, args, .. } => {
                    if let Some(name) = refs.get(callee) {
                        let entry = arities.entry(name.clone()).or_insert(0);
                        *entry = (*entry).max(args.len() as u32);
                    }
                }
                IrInstruction::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    Self::collect_import_call_arities_from_instructions(then_body, arities, refs);
                    Self::collect_import_call_arities_from_instructions(else_body, arities, refs);
                }
                IrInstruction::While {
                    cond_instructions,
                    body,
                    ..
                } => {
                    Self::collect_import_call_arities_from_instructions(
                        cond_instructions,
                        arities,
                        refs,
                    );
                    Self::collect_import_call_arities_from_instructions(body, arities, refs);
                }
                IrInstruction::For {
                    init,
                    cond_instructions,
                    update,
                    body,
                    ..
                } => {
                    Self::collect_import_call_arities_from_instructions(init, arities, refs);
                    Self::collect_import_call_arities_from_instructions(
                        cond_instructions,
                        arities,
                        refs,
                    );
                    Self::collect_import_call_arities_from_instructions(update, arities, refs);
                    Self::collect_import_call_arities_from_instructions(body, arities, refs);
                }
                IrInstruction::DoWhile {
                    body,
                    cond_instructions,
                    ..
                } => {
                    Self::collect_import_call_arities_from_instructions(body, arities, refs);
                    Self::collect_import_call_arities_from_instructions(
                        cond_instructions,
                        arities,
                        refs,
                    );
                }
                IrInstruction::Loop { body } => {
                    Self::collect_import_call_arities_from_instructions(body, arities, refs);
                }
                IrInstruction::Try {
                    try_body,
                    catch,
                    finally_body,
                } => {
                    Self::collect_import_call_arities_from_instructions(try_body, arities, refs);
                    if let Some(catch) = catch {
                        Self::collect_import_call_arities_from_instructions(
                            &catch.body,
                            arities,
                            refs,
                        );
                    }
                    if let Some(finally_body) = finally_body {
                        Self::collect_import_call_arities_from_instructions(
                            finally_body,
                            arities,
                            refs,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn layout_from_fields(fields: &[argon_ir::Field]) -> HeapObjectLayout {
        let mut properties = HashMap::new();
        let mut field_order = Vec::with_capacity(fields.len());
        for (idx, field) in fields.iter().enumerate() {
            properties.insert(field.name.clone(), 4 + (idx as u32 * 4));
            field_order.push(field.name.clone());
        }

        HeapObjectLayout {
            properties,
            field_order,
        }
    }

    fn lower_function(
        &mut self,
        func: &IrFunction,
        signature: FunctionSignature,
    ) -> Result<Function, CodegenError> {
        let plan = self.plan_function(func);
        let params_count = signature.params;
        let value_local_count = (plan.max_value + 1) as u32;

        let mut named_locals = HashMap::new();
        let mut next_named_local = params_count + value_local_count;
        for name in &plan.var_names {
            if param_index(func, name).is_none() {
                named_locals.insert(name.clone(), next_named_local);
                next_named_local += 1;
            }
        }

        let bb_local = next_named_local;
        let pending_control_kind_local = bb_local + 1;
        let pending_control_value_local = bb_local + 2;
        let try_state_base_local = bb_local + 3;
        let extra_locals_count =
            value_local_count + (named_locals.len() as u32) + 3 + (plan.max_try_depth as u32 * 2);
        let mut wasm_fn = Function::new(vec![(extra_locals_count, ValType::I32)]);

        let entry_block_id = func.body.first().map(|b| b.id as i32).unwrap_or(0);
        wasm_fn.instruction(&Instruction::I32Const(entry_block_id));
        wasm_fn.instruction(&Instruction::LocalSet(bb_local));
        wasm_fn.instruction(&Instruction::I32Const(0));
        wasm_fn.instruction(&Instruction::LocalSet(pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Const(0));
        wasm_fn.instruction(&Instruction::LocalSet(pending_control_value_local));

        let mut block_ids: Vec<usize> = func.body.iter().map(|b| b.id).collect();
        block_ids.sort_unstable();

        let mut blocks = HashMap::new();
        for bb in &func.body {
            blocks.insert(bb.id, bb);
        }

        let mut ctx = FunctionCtx {
            params_count,
            signature,
            bb_local,
            pending_control_kind_local,
            pending_control_value_local,
            try_state_base_local,
            try_depth: 0,
            named_locals,
            function_refs: HashMap::new(),
            constructor_refs: HashMap::new(),
            value_shapes: HashMap::new(),
            named_shapes: HashMap::new(),
            func,
        };

        wasm_fn.instruction(&Instruction::Block(BlockType::Empty));
        wasm_fn.instruction(&Instruction::Loop(BlockType::Empty));
        self.emit_dispatch_chain(&mut wasm_fn, &block_ids, 0, &blocks, &mut ctx)?;
        wasm_fn.instruction(&Instruction::LocalGet(bb_local));
        wasm_fn.instruction(&Instruction::I32Const(-1));
        wasm_fn.instruction(&Instruction::I32Eq);
        wasm_fn.instruction(&Instruction::BrIf(1));
        wasm_fn.instruction(&Instruction::Br(0));
        wasm_fn.instruction(&Instruction::End);
        wasm_fn.instruction(&Instruction::End);

        if signature.returns_i32 {
            wasm_fn.instruction(&Instruction::I32Const(0));
        }
        wasm_fn.instruction(&Instruction::End);

        Ok(wasm_fn)
    }

    fn emit_dispatch_chain(
        &mut self,
        wasm_fn: &mut Function,
        block_ids: &[usize],
        idx: usize,
        blocks: &HashMap<usize, &BasicBlock>,
        ctx: &mut FunctionCtx<'_>,
    ) -> Result<(), CodegenError> {
        if idx >= block_ids.len() {
            wasm_fn.instruction(&Instruction::I32Const(-1));
            wasm_fn.instruction(&Instruction::LocalSet(ctx.bb_local));
            return Ok(());
        }

        let block_id = block_ids[idx] as i32;
        wasm_fn.instruction(&Instruction::LocalGet(ctx.bb_local));
        wasm_fn.instruction(&Instruction::I32Const(block_id));
        wasm_fn.instruction(&Instruction::I32Eq);
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));

        let bb = blocks
            .get(&(block_id as usize))
            .ok_or_else(|| CodegenError::IrError(format!("missing basic block {}", block_id)))?;
        self.emit_basic_block(wasm_fn, bb, ctx)?;

        wasm_fn.instruction(&Instruction::Else);
        self.emit_dispatch_chain(wasm_fn, block_ids, idx + 1, blocks, ctx)?;
        wasm_fn.instruction(&Instruction::End);
        Ok(())
    }

    fn emit_basic_block(
        &mut self,
        wasm_fn: &mut Function,
        bb: &BasicBlock,
        ctx: &mut FunctionCtx<'_>,
    ) -> Result<(), CodegenError> {
        for inst in &bb.instructions {
            self.emit_guarded_instruction(wasm_fn, inst, ctx)?;
        }

        self.emit_pending_control_transfer(wasm_fn, ctx);

        match &bb.terminator {
            Terminator::Return(Some(value)) => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(
                    ctx.params_count,
                    *value,
                )));
                wasm_fn.instruction(&Instruction::Return);
            }
            Terminator::Return(None) => {
                if ctx.signature.returns_i32 {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                }
                wasm_fn.instruction(&Instruction::Return);
            }
            Terminator::Jump(target) => {
                wasm_fn.instruction(&Instruction::I32Const(*target as i32));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.bb_local));
            }
            Terminator::Branch { cond, then, else_ } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                wasm_fn.instruction(&Instruction::I32Const(*then as i32));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.bb_local));
                wasm_fn.instruction(&Instruction::Else);
                wasm_fn.instruction(&Instruction::I32Const(*else_ as i32));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.bb_local));
                wasm_fn.instruction(&Instruction::End);
            }
            Terminator::Unreachable => {
                wasm_fn.instruction(&Instruction::Unreachable);
                wasm_fn.instruction(&Instruction::I32Const(-1));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.bb_local));
            }
        }

        Ok(())
    }

    fn emit_guarded_instruction(
        &mut self,
        wasm_fn: &mut Function,
        inst: &IrInstruction,
        ctx: &mut FunctionCtx<'_>,
    ) -> Result<(), CodegenError> {
        wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Eqz);
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));
        self.emit_instruction(wasm_fn, inst, ctx)?;
        wasm_fn.instruction(&Instruction::End);
        Ok(())
    }

    fn emit_guarded_nested_instructions(
        &mut self,
        wasm_fn: &mut Function,
        instructions: &[IrInstruction],
        ctx: &mut FunctionCtx<'_>,
    ) -> Result<(), CodegenError> {
        for inst in instructions {
            self.emit_guarded_instruction(wasm_fn, inst, ctx)?;
        }
        Ok(())
    }

    fn emit_pending_control_transfer(&self, wasm_fn: &mut Function, ctx: &FunctionCtx<'_>) {
        wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));
        wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Const(PENDING_RETURN));
        wasm_fn.instruction(&Instruction::I32Eq);
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));
        if ctx.signature.returns_i32 {
            wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_value_local));
        }
        wasm_fn.instruction(&Instruction::Return);
        wasm_fn.instruction(&Instruction::Else);
        wasm_fn.instruction(&Instruction::Unreachable);
        wasm_fn.instruction(&Instruction::End);
        wasm_fn.instruction(&Instruction::End);
    }

    fn clear_pending_control(&self, wasm_fn: &mut Function, ctx: &FunctionCtx<'_>) {
        wasm_fn.instruction(&Instruction::I32Const(0));
        wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Const(0));
        wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
    }

    fn emit_loop_control_handling(
        &self,
        wasm_fn: &mut Function,
        ctx: &FunctionCtx<'_>,
        continue_depth: u32,
        break_depth: u32,
    ) {
        wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Const(PENDING_CONTINUE));
        wasm_fn.instruction(&Instruction::I32Eq);
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));
        self.clear_pending_control(wasm_fn, ctx);
        wasm_fn.instruction(&Instruction::Br(continue_depth));
        wasm_fn.instruction(&Instruction::End);

        wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
        wasm_fn.instruction(&Instruction::I32Const(PENDING_BREAK));
        wasm_fn.instruction(&Instruction::I32Eq);
        wasm_fn.instruction(&Instruction::If(BlockType::Empty));
        self.clear_pending_control(wasm_fn, ctx);
        wasm_fn.instruction(&Instruction::Br(break_depth));
        wasm_fn.instruction(&Instruction::End);
    }

    fn emit_instruction(
        &mut self,
        wasm_fn: &mut Function,
        inst: &IrInstruction,
        ctx: &mut FunctionCtx<'_>,
    ) -> Result<(), CodegenError> {
        match inst {
            IrInstruction::Const { dest, value } => match value {
                ConstValue::Number(n) => {
                    if n.fract() != 0.0 {
                        return Err(CodegenError::Unsupported(format!(
                            "non-integer number literal '{}' is unsupported in wasm subset",
                            n
                        )));
                    }
                    wasm_fn.instruction(&Instruction::I32Const(*n as i32));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                }
                ConstValue::Bool(b) => {
                    wasm_fn.instruction(&Instruction::I32Const(if *b { 1 } else { 0 }));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                }
                ConstValue::Null => {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                }
                ConstValue::String(s) => {
                    let ptr = self.intern_string(s);
                    wasm_fn.instruction(&Instruction::I32Const(ptr as i32));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                }
            },
            IrInstruction::VarDecl { name, init, .. } => {
                let local = self.resolve_variable_local(ctx, name)?;
                if let Some(src) = init {
                    wasm_fn
                        .instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                } else {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                }
                wasm_fn.instruction(&Instruction::LocalSet(local));
                let shape = init.and_then(|src| ctx.shape_for_value(src));
                ctx.set_named_shape(name, shape);
            }
            IrInstruction::AssignVar { name, src } => {
                let local = self.resolve_variable_local(ctx, name)?;
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                wasm_fn.instruction(&Instruction::LocalSet(local));
                let shape = ctx.shape_for_value(*src);
                ctx.set_named_shape(name, shape);
            }
            IrInstruction::AssignExpr { name, src, dest } => {
                let local = self.resolve_variable_local(ctx, name)?;
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                wasm_fn.instruction(&Instruction::LocalSet(local));
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                let shape = ctx.shape_for_value(*src);
                ctx.set_named_shape(name, shape.clone());
                ctx.set_value_shape(*dest, shape);
            }
            IrInstruction::VarRef { dest, name } => {
                if let Some(param_idx) = param_index(ctx.func, name) {
                    wasm_fn.instruction(&Instruction::LocalGet(param_idx));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.set_value_shape(*dest, ctx.named_shapes.get(name).cloned());
                } else if let Some(local_idx) = ctx.named_locals.get(name).copied() {
                    wasm_fn.instruction(&Instruction::LocalGet(local_idx));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.set_value_shape(*dest, ctx.named_shapes.get(name).cloned());
                } else if let Some(func_idx) = self.function_indices.get(name).copied() {
                    wasm_fn.instruction(&Instruction::I32Const(func_idx as i32));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                    ctx.function_refs.insert(*dest, func_idx);
                } else if let Some(import_idx) = self.import_indices.get(name).copied() {
                    wasm_fn.instruction(&Instruction::I32Const(import_idx as i32));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                    ctx.function_refs.insert(*dest, import_idx);
                } else if self.constructor_layouts.contains_key(name) {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                    ctx.clear_value_metadata(*dest);
                    ctx.constructor_refs.insert(*dest, name.clone());
                } else {
                    return Err(CodegenError::Unsupported(format!(
                        "unsupported symbol reference '{}' in wasm backend",
                        name
                    )));
                }
            }
            IrInstruction::Load { dest, src } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                let shape = ctx.shape_for_value(*src);
                ctx.set_value_shape(*dest, shape);
            }
            IrInstruction::Store { dest, src } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *src)));
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                let shape = ctx.shape_for_value(*src);
                ctx.set_value_shape(*dest, shape);
            }
            IrInstruction::BinOp { op, lhs, rhs, dest } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *lhs)));
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *rhs)));
                self.emit_binop(wasm_fn, *op);
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                ctx.clear_value_metadata(*dest);
            }
            IrInstruction::UnOp { op, arg, dest } => {
                match op {
                    UnOp::Neg => {
                        wasm_fn.instruction(&Instruction::I32Const(0));
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *arg,
                        )));
                        wasm_fn.instruction(&Instruction::I32Sub);
                    }
                    UnOp::Not => {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *arg,
                        )));
                        wasm_fn.instruction(&Instruction::I32Eqz);
                    }
                }
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                ctx.clear_value_metadata(*dest);
            }
            IrInstruction::LogicalOp { op, lhs, rhs, dest } => {
                match op {
                    LogicOp::And => {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *lhs,
                        )));
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *rhs,
                        )));
                        wasm_fn.instruction(&Instruction::I32And);
                    }
                    LogicOp::Or => {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *lhs,
                        )));
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *rhs,
                        )));
                        wasm_fn.instruction(&Instruction::I32Or);
                    }
                    LogicOp::Nullish => {
                        return Err(CodegenError::Unsupported(
                            "nullish coalescing is unsupported for wasm backend".to_string(),
                        ))
                    }
                }
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                ctx.clear_value_metadata(*dest);
            }
            IrInstruction::Conditional {
                cond,
                then_value,
                else_value,
                dest,
            } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(
                    ctx.params_count,
                    *then_value,
                )));
                wasm_fn.instruction(&Instruction::LocalGet(value_local(
                    ctx.params_count,
                    *else_value,
                )));
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::Select);
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                ctx.clear_value_metadata(*dest);
            }
            IrInstruction::Call { callee, args, dest } => {
                let callee_idx = ctx.function_refs.get(callee).copied().ok_or_else(|| {
                    CodegenError::Unsupported(
                        "dynamic or interop calls are unsupported for wasm backend".to_string(),
                    )
                })?;

                for arg in args {
                    wasm_fn
                        .instruction(&Instruction::LocalGet(value_local(ctx.params_count, *arg)));
                }
                wasm_fn.instruction(&Instruction::Call(callee_idx));

                let sig = self.signatures.get(callee_idx as usize).ok_or_else(|| {
                    CodegenError::IrError(format!(
                        "missing signature for function index {}",
                        callee_idx
                    ))
                })?;
                if sig.returns_i32 {
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                } else {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn
                        .instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                }
                ctx.clear_value_metadata(*dest);
            }
            IrInstruction::ArrayLit { dest, elements } => {
                self.uses_memory = true;
                let dest_local = value_local(ctx.params_count, *dest);
                let bytes = ((elements.len() + 1) * 4) as i32;

                wasm_fn.instruction(&Instruction::GlobalGet(HEAP_PTR_GLOBAL_INDEX));
                wasm_fn.instruction(&Instruction::LocalTee(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(bytes));
                wasm_fn.instruction(&Instruction::I32Add);
                wasm_fn.instruction(&Instruction::GlobalSet(HEAP_PTR_GLOBAL_INDEX));

                wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(elements.len() as i32));
                wasm_fn.instruction(&Instruction::I32Store(memarg(0)));

                for (idx, element) in elements.iter().enumerate() {
                    wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                    if let Some(value) = element {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *value,
                        )));
                    } else {
                        wasm_fn.instruction(&Instruction::I32Const(0));
                    }
                    wasm_fn.instruction(&Instruction::I32Store(memarg(((idx + 1) * 4) as u64)));
                }

                ctx.set_value_shape(*dest, Some(HeapValueShape::Array));
            }
            IrInstruction::ObjectLit { dest, props } => {
                self.uses_memory = true;
                let dest_local = value_local(ctx.params_count, *dest);
                let bytes = ((props.len() + 1) * 4) as i32;

                wasm_fn.instruction(&Instruction::GlobalGet(HEAP_PTR_GLOBAL_INDEX));
                wasm_fn.instruction(&Instruction::LocalTee(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(bytes));
                wasm_fn.instruction(&Instruction::I32Add);
                wasm_fn.instruction(&Instruction::GlobalSet(HEAP_PTR_GLOBAL_INDEX));

                wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(props.len() as i32));
                wasm_fn.instruction(&Instruction::I32Store(memarg(0)));

                let mut properties = HashMap::new();
                let mut field_order = Vec::with_capacity(props.len());
                for (idx, prop) in props.iter().enumerate() {
                    let offset = 4 + (idx as u32 * 4);
                    properties.insert(prop.key.clone(), offset);
                    field_order.push(prop.key.clone());
                    wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                    wasm_fn.instruction(&Instruction::LocalGet(value_local(
                        ctx.params_count,
                        prop.value,
                    )));
                    wasm_fn.instruction(&Instruction::I32Store(memarg(offset as u64)));
                }

                ctx.set_value_shape(
                    *dest,
                    Some(HeapValueShape::Object(HeapObjectLayout {
                        properties,
                        field_order,
                    })),
                );
            }
            IrInstruction::Member {
                object,
                property,
                dest,
            } => {
                let shape = ctx.shape_for_value(*object).ok_or_else(|| {
                    CodegenError::Unsupported(format!(
                        "member access requires a known heap-backed shape in wasm backend: {}",
                        property
                    ))
                })?;

                match shape {
                    HeapValueShape::Object(layout) => {
                        let offset = layout.properties.get(property).copied().ok_or_else(|| {
                            CodegenError::Unsupported(format!(
                                "unknown object property '{}' in wasm backend",
                                property
                            ))
                        })?;
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *object,
                        )));
                        wasm_fn.instruction(&Instruction::I32Load(memarg(offset as u64)));
                        wasm_fn.instruction(&Instruction::LocalSet(value_local(
                            ctx.params_count,
                            *dest,
                        )));
                        ctx.clear_value_metadata(*dest);
                    }
                    HeapValueShape::Array => {
                        if property != "length" {
                            return Err(CodegenError::Unsupported(format!(
                                "unsupported array member '{}' in wasm backend",
                                property
                            )));
                        }
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *object,
                        )));
                        wasm_fn.instruction(&Instruction::I32Load(memarg(0)));
                        wasm_fn.instruction(&Instruction::LocalSet(value_local(
                            ctx.params_count,
                            *dest,
                        )));
                        ctx.clear_value_metadata(*dest);
                    }
                }
            }
            IrInstruction::MemberComputed {
                object,
                property,
                dest,
            } => {
                let shape = ctx.shape_for_value(*object).ok_or_else(|| {
                    CodegenError::Unsupported(
                        "computed member access requires a known heap-backed shape in wasm backend"
                            .to_string(),
                    )
                })?;

                match shape {
                    HeapValueShape::Array => {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *object,
                        )));
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            *property,
                        )));
                        wasm_fn.instruction(&Instruction::I32Const(4));
                        wasm_fn.instruction(&Instruction::I32Mul);
                        wasm_fn.instruction(&Instruction::I32Const(4));
                        wasm_fn.instruction(&Instruction::I32Add);
                        wasm_fn.instruction(&Instruction::I32Add);
                        wasm_fn.instruction(&Instruction::I32Load(memarg(0)));
                        wasm_fn.instruction(&Instruction::LocalSet(value_local(
                            ctx.params_count,
                            *dest,
                        )));
                        ctx.clear_value_metadata(*dest);
                    }
                    HeapValueShape::Object(_) => {
                        return Err(CodegenError::Unsupported(
                            "computed object property access is unsupported for wasm backend"
                                .to_string(),
                        ))
                    }
                }
            }
            IrInstruction::New { callee, args, dest } => {
                let constructor = ctx.constructor_refs.get(callee).cloned().ok_or_else(|| {
                    CodegenError::Unsupported(
                        "dynamic constructors are unsupported for wasm backend".to_string(),
                    )
                })?;

                if args.len() == 1 {
                    if let Some(HeapValueShape::Object(layout)) = ctx.shape_for_value(args[0]) {
                        wasm_fn.instruction(&Instruction::LocalGet(value_local(
                            ctx.params_count,
                            args[0],
                        )));
                        wasm_fn.instruction(&Instruction::LocalSet(value_local(
                            ctx.params_count,
                            *dest,
                        )));
                        ctx.set_value_shape(*dest, Some(HeapValueShape::Object(layout)));
                        return Ok(());
                    }
                }

                let layout = self
                    .constructor_layouts
                    .get(&constructor)
                    .cloned()
                    .ok_or_else(|| {
                        CodegenError::Unsupported(format!(
                            "unknown constructor '{}' in wasm backend",
                            constructor
                        ))
                    })?;

                if args.len() != layout.field_order.len() {
                    return Err(CodegenError::Unsupported(format!(
                        "constructor '{}' expects {} field value(s) in wasm backend, found {}",
                        constructor,
                        layout.field_order.len(),
                        args.len()
                    )));
                }

                self.uses_memory = true;
                let dest_local = value_local(ctx.params_count, *dest);
                let bytes = ((layout.field_order.len() + 1) * 4) as i32;

                wasm_fn.instruction(&Instruction::GlobalGet(HEAP_PTR_GLOBAL_INDEX));
                wasm_fn.instruction(&Instruction::LocalTee(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(bytes));
                wasm_fn.instruction(&Instruction::I32Add);
                wasm_fn.instruction(&Instruction::GlobalSet(HEAP_PTR_GLOBAL_INDEX));

                wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                wasm_fn.instruction(&Instruction::I32Const(layout.field_order.len() as i32));
                wasm_fn.instruction(&Instruction::I32Store(memarg(0)));

                for (idx, arg) in args.iter().enumerate() {
                    wasm_fn.instruction(&Instruction::LocalGet(dest_local));
                    wasm_fn
                        .instruction(&Instruction::LocalGet(value_local(ctx.params_count, *arg)));
                    wasm_fn
                        .instruction(&Instruction::I32Store(memarg((4 + idx as u64 * 4) as u64)));
                }

                ctx.set_value_shape(*dest, Some(HeapValueShape::Object(layout)));
            }
            IrInstruction::ExprStmt { .. } => {}
            IrInstruction::Await { arg, dest } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *arg)));
                wasm_fn.instruction(&Instruction::LocalSet(value_local(ctx.params_count, *dest)));
                let shape = ctx.shape_for_value(*arg);
                ctx.set_value_shape(*dest, shape);
            }
            IrInstruction::ThrowStmt { arg } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *arg)));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_THROW));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
            }
            IrInstruction::If {
                cond,
                then_body,
                else_body,
            } => {
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, then_body, ctx)?;
                if !else_body.is_empty() {
                    wasm_fn.instruction(&Instruction::Else);
                    self.emit_guarded_nested_instructions(wasm_fn, else_body, ctx)?;
                }
                wasm_fn.instruction(&Instruction::End);
            }
            IrInstruction::While {
                cond_instructions,
                cond,
                body,
            } => {
                wasm_fn.instruction(&Instruction::Block(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Loop(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, cond_instructions, ctx)?;
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, body, ctx)?;
                self.emit_loop_control_handling(wasm_fn, ctx, 2, 3);
                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Eqz);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Br(2));
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
            }
            IrInstruction::For {
                init,
                cond_instructions,
                cond,
                update,
                body,
            } => {
                self.emit_guarded_nested_instructions(wasm_fn, init, ctx)?;
                wasm_fn.instruction(&Instruction::Block(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Loop(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, cond_instructions, ctx)?;
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, body, ctx)?;

                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_CONTINUE));
                wasm_fn.instruction(&Instruction::I32Eq);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.clear_pending_control(wasm_fn, ctx);
                self.emit_guarded_nested_instructions(wasm_fn, update, ctx)?;
                wasm_fn.instruction(&Instruction::Br(2));
                wasm_fn.instruction(&Instruction::End);

                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_BREAK));
                wasm_fn.instruction(&Instruction::I32Eq);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.clear_pending_control(wasm_fn, ctx);
                wasm_fn.instruction(&Instruction::Br(3));
                wasm_fn.instruction(&Instruction::End);

                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Eqz);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, update, ctx)?;
                wasm_fn.instruction(&Instruction::Br(2));
                wasm_fn.instruction(&Instruction::End);

                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
            }
            IrInstruction::DoWhile {
                body,
                cond_instructions,
                cond,
            } => {
                wasm_fn.instruction(&Instruction::Block(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Loop(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, body, ctx)?;
                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_CONTINUE));
                wasm_fn.instruction(&Instruction::I32Eq);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.clear_pending_control(wasm_fn, ctx);
                wasm_fn.instruction(&Instruction::End);

                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_BREAK));
                wasm_fn.instruction(&Instruction::I32Eq);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                self.clear_pending_control(wasm_fn, ctx);
                wasm_fn.instruction(&Instruction::Br(2));
                wasm_fn.instruction(&Instruction::End);

                self.emit_guarded_nested_instructions(wasm_fn, cond_instructions, ctx)?;
                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Eqz);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                wasm_fn.instruction(&Instruction::LocalGet(value_local(ctx.params_count, *cond)));
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Br(2));
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
            }
            IrInstruction::Loop { body } => {
                wasm_fn.instruction(&Instruction::Block(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Loop(BlockType::Empty));
                self.emit_guarded_nested_instructions(wasm_fn, body, ctx)?;
                self.emit_loop_control_handling(wasm_fn, ctx, 1, 2);
                wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Eqz);
                wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                wasm_fn.instruction(&Instruction::Br(1));
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
                wasm_fn.instruction(&Instruction::End);
            }
            IrInstruction::Break => {
                wasm_fn.instruction(&Instruction::I32Const(PENDING_BREAK));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(0));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
            }
            IrInstruction::Continue => {
                wasm_fn.instruction(&Instruction::I32Const(PENDING_CONTINUE));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
                wasm_fn.instruction(&Instruction::I32Const(0));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
            }
            IrInstruction::Return { value } => {
                if let Some(value) = value {
                    wasm_fn.instruction(&Instruction::LocalGet(value_local(
                        ctx.params_count,
                        *value,
                    )));
                } else {
                    wasm_fn.instruction(&Instruction::I32Const(0));
                }
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
                wasm_fn.instruction(&Instruction::I32Const(PENDING_RETURN));
                wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
            }
            IrInstruction::Try {
                try_body,
                catch,
                finally_body,
            } => {
                self.emit_guarded_nested_instructions(wasm_fn, try_body, ctx)?;

                if let Some(catch) = catch {
                    wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::I32Const(PENDING_THROW));
                    wasm_fn.instruction(&Instruction::I32Eq);
                    wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                    if let Some(param) = &catch.param {
                        let local = self.resolve_variable_local(ctx, param)?;
                        wasm_fn
                            .instruction(&Instruction::LocalGet(ctx.pending_control_value_local));
                        wasm_fn.instruction(&Instruction::LocalSet(local));
                        ctx.set_named_shape(param, None);
                    }
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
                    self.emit_guarded_nested_instructions(wasm_fn, &catch.body, ctx)?;
                    wasm_fn.instruction(&Instruction::End);
                }

                if let Some(finally_body) = finally_body {
                    let (saved_kind_local, saved_value_local) = ctx.try_state_locals(ctx.try_depth);
                    ctx.try_depth += 1;

                    wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::LocalSet(saved_kind_local));
                    wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_value_local));
                    wasm_fn.instruction(&Instruction::LocalSet(saved_value_local));
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::I32Const(0));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));

                    self.emit_guarded_nested_instructions(wasm_fn, finally_body, ctx)?;

                    wasm_fn.instruction(&Instruction::LocalGet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::I32Eqz);
                    wasm_fn.instruction(&Instruction::If(BlockType::Empty));
                    wasm_fn.instruction(&Instruction::LocalGet(saved_kind_local));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_kind_local));
                    wasm_fn.instruction(&Instruction::LocalGet(saved_value_local));
                    wasm_fn.instruction(&Instruction::LocalSet(ctx.pending_control_value_local));
                    wasm_fn.instruction(&Instruction::End);

                    ctx.try_depth -= 1;
                }
            }
        }
        Ok(())
    }

    fn emit_binop(&self, wasm_fn: &mut Function, op: BinOp) {
        match op {
            BinOp::Add => {
                wasm_fn.instruction(&Instruction::I32Add);
            }
            BinOp::Sub => {
                wasm_fn.instruction(&Instruction::I32Sub);
            }
            BinOp::Mul => {
                wasm_fn.instruction(&Instruction::I32Mul);
            }
            BinOp::Div => {
                wasm_fn.instruction(&Instruction::I32DivS);
            }
            BinOp::Mod => {
                wasm_fn.instruction(&Instruction::I32RemS);
            }
            BinOp::Eq => {
                wasm_fn.instruction(&Instruction::I32Eq);
            }
            BinOp::Ne => {
                wasm_fn.instruction(&Instruction::I32Ne);
            }
            BinOp::Lt => {
                wasm_fn.instruction(&Instruction::I32LtS);
            }
            BinOp::Le => {
                wasm_fn.instruction(&Instruction::I32LeS);
            }
            BinOp::Gt => {
                wasm_fn.instruction(&Instruction::I32GtS);
            }
            BinOp::Ge => {
                wasm_fn.instruction(&Instruction::I32GeS);
            }
            BinOp::And => {
                wasm_fn.instruction(&Instruction::I32And);
            }
            BinOp::Or => {
                wasm_fn.instruction(&Instruction::I32Or);
            }
            BinOp::Xor => {
                wasm_fn.instruction(&Instruction::I32Xor);
            }
            BinOp::Shl => {
                wasm_fn.instruction(&Instruction::I32Shl);
            }
            BinOp::Shr => {
                wasm_fn.instruction(&Instruction::I32ShrU);
            }
            BinOp::Sar => {
                wasm_fn.instruction(&Instruction::I32ShrS);
            }
        }
    }

    fn resolve_variable_local(
        &self,
        ctx: &FunctionCtx<'_>,
        name: &str,
    ) -> Result<u32, CodegenError> {
        if let Some(param_idx) = param_index(ctx.func, name) {
            return Ok(param_idx);
        }
        ctx.named_locals
            .get(name)
            .copied()
            .ok_or_else(|| CodegenError::IrError(format!("unknown wasm local variable '{}'", name)))
    }

    fn intern_string(&mut self, s: &str) -> u32 {
        let normalized = normalize_string_literal(s);

        if let Some(ptr) = self.string_pool.get(&normalized).copied() {
            return ptr;
        }

        self.uses_memory = true;
        let offset = align4(self.next_data_offset);
        let mut bytes = Vec::with_capacity(4 + normalized.len());
        bytes.extend((normalized.len() as u32).to_le_bytes());
        bytes.extend(normalized.as_bytes());

        self.next_data_offset = align4(offset + bytes.len() as u32);
        self.data_segments.push(DataSegment {
            offset,
            bytes: bytes.clone(),
        });
        self.string_pool.insert(normalized, offset);
        offset
    }

    fn plan_function(&self, func: &IrFunction) -> FunctionPlan {
        let mut max_value: usize = 0;
        let mut var_names = BTreeSet::new();
        let mut max_try_depth = 0;

        for bb in &func.body {
            for inst in &bb.instructions {
                match inst {
                    IrInstruction::Load { dest, src } | IrInstruction::Store { dest, src } => {
                        touch_value(&mut max_value, *dest);
                        touch_value(&mut max_value, *src);
                    }
                    IrInstruction::ObjectLit { dest, props } => {
                        touch_value(&mut max_value, *dest);
                        for prop in props {
                            touch_value(&mut max_value, prop.value);
                        }
                    }
                    IrInstruction::New { callee, args, dest } => {
                        touch_value(&mut max_value, *callee);
                        touch_value(&mut max_value, *dest);
                        for arg in args {
                            touch_value(&mut max_value, *arg);
                        }
                    }
                    IrInstruction::Await { arg, dest } => {
                        touch_value(&mut max_value, *arg);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::VarDecl { name, init, .. } => {
                        var_names.insert(name.clone());
                        if let Some(init) = init {
                            touch_value(&mut max_value, *init);
                        }
                    }
                    IrInstruction::AssignVar { name, src } => {
                        var_names.insert(name.clone());
                        touch_value(&mut max_value, *src);
                    }
                    IrInstruction::AssignExpr { name, src, dest } => {
                        var_names.insert(name.clone());
                        touch_value(&mut max_value, *src);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::ThrowStmt { arg } => touch_value(&mut max_value, *arg),
                    IrInstruction::If {
                        cond,
                        then_body,
                        else_body,
                    } => {
                        touch_value(&mut max_value, *cond);
                        for inst in then_body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in else_body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                    }
                    IrInstruction::While {
                        cond_instructions,
                        cond,
                        body,
                    } => {
                        touch_value(&mut max_value, *cond);
                        for inst in cond_instructions {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                    }
                    IrInstruction::For {
                        init,
                        cond_instructions,
                        cond,
                        update,
                        body,
                    } => {
                        touch_value(&mut max_value, *cond);
                        for inst in init {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in cond_instructions {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in update {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                    }
                    IrInstruction::DoWhile {
                        body,
                        cond_instructions,
                        cond,
                    } => {
                        touch_value(&mut max_value, *cond);
                        for inst in body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                        for inst in cond_instructions {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                    }
                    IrInstruction::Loop { body } => {
                        for inst in body {
                            max_try_depth = max_try_depth.max(self.plan_nested_inst(
                                inst,
                                &mut max_value,
                                &mut var_names,
                            ));
                        }
                    }
                    IrInstruction::Break | IrInstruction::Continue => {}
                    IrInstruction::Return { value } => {
                        if let Some(value) = value {
                            touch_value(&mut max_value, *value);
                        }
                    }
                    IrInstruction::Try {
                        try_body,
                        catch,
                        finally_body,
                    } => {
                        max_try_depth = max_try_depth.max(1);
                        for i in try_body {
                            max_try_depth = max_try_depth
                                .max(self.plan_nested_inst(i, &mut max_value, &mut var_names) + 1);
                        }
                        if let Some(catch) = catch {
                            if let Some(param) = &catch.param {
                                var_names.insert(param.clone());
                            }
                            for i in &catch.body {
                                max_try_depth = max_try_depth.max(
                                    self.plan_nested_inst(i, &mut max_value, &mut var_names) + 1,
                                );
                            }
                        }
                        if let Some(finally_body) = finally_body {
                            for i in finally_body {
                                max_try_depth = max_try_depth.max(
                                    self.plan_nested_inst(i, &mut max_value, &mut var_names) + 1,
                                );
                            }
                        }
                    }
                    IrInstruction::ExprStmt { value }
                    | IrInstruction::VarRef { dest: value, .. } => {
                        touch_value(&mut max_value, *value);
                    }
                    IrInstruction::Member { object, dest, .. } => {
                        touch_value(&mut max_value, *object);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::MemberComputed {
                        object,
                        property,
                        dest,
                    } => {
                        touch_value(&mut max_value, *object);
                        touch_value(&mut max_value, *property);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::BinOp { lhs, rhs, dest, .. } => {
                        touch_value(&mut max_value, *lhs);
                        touch_value(&mut max_value, *rhs);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::UnOp { arg, dest, .. } => {
                        touch_value(&mut max_value, *arg);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::Call { callee, args, dest } => {
                        touch_value(&mut max_value, *callee);
                        for arg in args {
                            touch_value(&mut max_value, *arg);
                        }
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::ArrayLit { dest, elements } => {
                        touch_value(&mut max_value, *dest);
                        for value in elements.iter().flatten() {
                            touch_value(&mut max_value, *value);
                        }
                    }
                    IrInstruction::LogicalOp { lhs, rhs, dest, .. } => {
                        touch_value(&mut max_value, *lhs);
                        touch_value(&mut max_value, *rhs);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::Conditional {
                        cond,
                        then_value,
                        else_value,
                        dest,
                    } => {
                        touch_value(&mut max_value, *cond);
                        touch_value(&mut max_value, *then_value);
                        touch_value(&mut max_value, *else_value);
                        touch_value(&mut max_value, *dest);
                    }
                    IrInstruction::Const { dest, .. } => touch_value(&mut max_value, *dest),
                }
            }

            match &bb.terminator {
                Terminator::Return(Some(v)) | Terminator::Branch { cond: v, .. } => {
                    touch_value(&mut max_value, *v);
                }
                Terminator::Return(None) | Terminator::Jump(_) | Terminator::Unreachable => {}
            }
        }

        FunctionPlan {
            max_value,
            var_names,
            max_try_depth,
        }
    }

    fn plan_nested_inst(
        &self,
        inst: &IrInstruction,
        max_value: &mut usize,
        var_names: &mut BTreeSet<String>,
    ) -> usize {
        match inst {
            IrInstruction::VarDecl { name, init, .. } => {
                var_names.insert(name.clone());
                if let Some(value) = init {
                    touch_value(max_value, *value);
                }
                0
            }
            IrInstruction::AssignVar { name, src } => {
                var_names.insert(name.clone());
                touch_value(max_value, *src);
                0
            }
            IrInstruction::AssignExpr { name, src, dest } => {
                var_names.insert(name.clone());
                touch_value(max_value, *src);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::ExprStmt { value } | IrInstruction::VarRef { dest: value, .. } => {
                touch_value(max_value, *value);
                0
            }
            IrInstruction::Const { dest, .. } => {
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::If {
                cond,
                then_body,
                else_body,
            } => {
                touch_value(max_value, *cond);
                let mut nested_depth = 0;
                for inst in then_body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in else_body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                nested_depth
            }
            IrInstruction::While {
                cond_instructions,
                cond,
                body,
            } => {
                touch_value(max_value, *cond);
                let mut nested_depth = 0;
                for inst in cond_instructions {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                nested_depth
            }
            IrInstruction::For {
                init,
                cond_instructions,
                cond,
                update,
                body,
            } => {
                touch_value(max_value, *cond);
                let mut nested_depth = 0;
                for inst in init {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in cond_instructions {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in update {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                nested_depth
            }
            IrInstruction::DoWhile {
                body,
                cond_instructions,
                cond,
            } => {
                touch_value(max_value, *cond);
                let mut nested_depth = 0;
                for inst in body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                for inst in cond_instructions {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                nested_depth
            }
            IrInstruction::Loop { body } => {
                let mut nested_depth = 0;
                for inst in body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                nested_depth
            }
            IrInstruction::Break | IrInstruction::Continue => 0,
            IrInstruction::Return { value } => {
                if let Some(value) = value {
                    touch_value(max_value, *value);
                }
                0
            }
            IrInstruction::Try {
                try_body,
                catch,
                finally_body,
            } => {
                let mut nested_depth = 0;
                for inst in try_body {
                    nested_depth =
                        nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                }
                if let Some(catch) = catch {
                    if let Some(param) = &catch.param {
                        var_names.insert(param.clone());
                    }
                    for inst in &catch.body {
                        nested_depth =
                            nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                    }
                }
                if let Some(finally_body) = finally_body {
                    for inst in finally_body {
                        nested_depth =
                            nested_depth.max(self.plan_nested_inst(inst, max_value, var_names));
                    }
                }
                nested_depth + 1
            }
            IrInstruction::Await { arg, dest } => {
                touch_value(max_value, *arg);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::ThrowStmt { arg } => {
                touch_value(max_value, *arg);
                0
            }
            IrInstruction::Call { callee, args, dest } => {
                touch_value(max_value, *callee);
                for arg in args {
                    touch_value(max_value, *arg);
                }
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::Member { object, dest, .. } => {
                touch_value(max_value, *object);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::MemberComputed {
                object,
                property,
                dest,
            } => {
                touch_value(max_value, *object);
                touch_value(max_value, *property);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::ObjectLit { dest, props } => {
                touch_value(max_value, *dest);
                for prop in props {
                    touch_value(max_value, prop.value);
                }
                0
            }
            IrInstruction::ArrayLit { dest, elements } => {
                touch_value(max_value, *dest);
                for value in elements.iter().flatten() {
                    touch_value(max_value, *value);
                }
                0
            }
            IrInstruction::New { callee, args, dest } => {
                touch_value(max_value, *callee);
                for arg in args {
                    touch_value(max_value, *arg);
                }
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::Load { dest, src } | IrInstruction::Store { dest, src } => {
                touch_value(max_value, *dest);
                touch_value(max_value, *src);
                0
            }
            IrInstruction::BinOp { lhs, rhs, dest, .. } => {
                touch_value(max_value, *lhs);
                touch_value(max_value, *rhs);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::UnOp { arg, dest, .. } => {
                touch_value(max_value, *arg);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::LogicalOp { lhs, rhs, dest, .. } => {
                touch_value(max_value, *lhs);
                touch_value(max_value, *rhs);
                touch_value(max_value, *dest);
                0
            }
            IrInstruction::Conditional {
                cond,
                then_value,
                else_value,
                dest,
            } => {
                touch_value(max_value, *cond);
                touch_value(max_value, *then_value);
                touch_value(max_value, *else_value);
                touch_value(max_value, *dest);
                0
            }
        }
    }
}

struct FunctionCtx<'a> {
    params_count: u32,
    signature: FunctionSignature,
    bb_local: u32,
    pending_control_kind_local: u32,
    pending_control_value_local: u32,
    try_state_base_local: u32,
    try_depth: usize,
    named_locals: HashMap<String, u32>,
    function_refs: HashMap<ValueId, u32>,
    constructor_refs: HashMap<ValueId, String>,
    value_shapes: HashMap<ValueId, HeapValueShape>,
    named_shapes: HashMap<String, HeapValueShape>,
    func: &'a IrFunction,
}

impl<'a> FunctionCtx<'a> {
    fn try_state_locals(&self, depth: usize) -> (u32, u32) {
        let flag_local = self.try_state_base_local + (depth as u32 * 2);
        (flag_local, flag_local + 1)
    }

    fn clear_value_metadata(&mut self, value: ValueId) {
        self.function_refs.remove(&value);
        self.constructor_refs.remove(&value);
        self.value_shapes.remove(&value);
    }

    fn set_value_shape(&mut self, value: ValueId, shape: Option<HeapValueShape>) {
        self.clear_value_metadata(value);
        if let Some(shape) = shape {
            self.value_shapes.insert(value, shape);
        }
    }

    fn set_named_shape(&mut self, name: &str, shape: Option<HeapValueShape>) {
        if let Some(shape) = shape {
            self.named_shapes.insert(name.to_string(), shape);
        } else {
            self.named_shapes.remove(name);
        }
    }

    fn shape_for_value(&self, value: ValueId) -> Option<HeapValueShape> {
        self.value_shapes.get(&value).cloned()
    }
}

fn value_local(params_count: u32, value: ValueId) -> u32 {
    params_count + (value as u32)
}

fn memarg(offset: u64) -> MemArg {
    MemArg {
        offset,
        align: 2,
        memory_index: 0,
    }
}

fn param_index(func: &IrFunction, name: &str) -> Option<u32> {
    func.params
        .iter()
        .position(|p| p.name == name)
        .map(|i| i as u32)
}

fn touch_value(max: &mut usize, value: ValueId) {
    if value > *max {
        *max = value;
    }
}

fn align4(value: u32) -> u32 {
    (value + 3) & !3
}

fn normalize_string_literal(input: &str) -> String {
    if input.len() >= 2 {
        let first = input.chars().next().unwrap_or_default();
        let last = input.chars().next_back().unwrap_or_default();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return input[1..input.len() - 1].to_string();
        }
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use wasmparser::Validator;

    static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn compile_source(source: &str) -> Vec<u8> {
        // Assign
        let ast = argon_parser::parse(source).expect("source should parse");
        let mut codegen = WasmCodegen::new();

        // Act
        codegen
            .generate_from_ast(&ast)
            .expect("wasm generation should succeed")
    }

    fn run_node_script(wasm_bytes: &[u8], script_body: &str) -> String {
        run_node_script_with_imports(wasm_bytes, "{}", script_body)
    }

    fn run_node_script_with_imports(
        wasm_bytes: &[u8],
        imports_expr: &str,
        script_body: &str,
    ) -> String {
        // Assign
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let unique = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let wasm_path = std::env::temp_dir().join(format!(
            "argon_wasm_test_{}_{}_{}.wasm",
            std::process::id(),
            nonce,
            unique
        ));
        fs::write(&wasm_path, wasm_bytes).expect("should write wasm fixture");

        let script = format!(
            "const fs=require('fs');\
             const bytes=fs.readFileSync(process.argv[1]);\
             const imports = {};\
             WebAssembly.instantiate(bytes, imports).then(({{instance}})=>{{\
               {}\
             }}).catch((err)=>{{console.error(err); process.exit(1);}});",
            imports_expr, script_body
        );

        // Act
        let output = Command::new("node")
            .arg("-e")
            .arg(&script)
            .arg(&wasm_path)
            .output()
            .expect("node should be available");

        let _ = fs::remove_file(&wasm_path);

        // Assert
        assert!(
            output.status.success(),
            "node execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn emits_valid_wasm_for_control_flow_subset() {
        // Assign
        let source = r#"
            function sumTo(n: i32): i32 {
                let acc = 0;
                let i = 0;
                while (i <= n) {
                    acc = acc + i;
                    i = i + 1;
                }
                return acc;
            }
        "#;

        // Act
        let wasm = compile_source(source);
        let validation = Validator::new().validate_all(&wasm);

        // Assert
        assert!(validation.is_ok(), "wasm must be structurally valid");
    }

    #[test]
    fn executes_internal_function_calls_and_loops() {
        // Assign
        let source = r#"
            function add(a: i32, b: i32): i32 { return a + b; }
            function sumTo(n: i32): i32 {
                let acc = 0;
                for (let i = 0; i <= n; i = i + 1) {
                    acc = acc + i;
                }
                return add(acc, 0);
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const result = instance.exports.sumTo(4); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "10");
    }

    #[test]
    fn stores_string_constants_in_linear_memory() {
        // Assign
        let source = r#"
            function greet(): i32 {
                return "hello";
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const ptr = instance.exports.greet();\
             const view = new DataView(instance.exports.memory.buffer);\
             const len = view.getUint32(ptr, true);\
             const bytes = new Uint8Array(instance.exports.memory.buffer, ptr + 4, len);\
             console.log(new TextDecoder().decode(bytes));",
        );

        // Assert
        assert_eq!(output, "hello");
    }

    #[test]
    fn stores_array_literals_in_linear_memory() {
        // Assign
        let source = r#"
            function makeArray(): i32 {
                return [1, 2, 3];
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const ptr = instance.exports.makeArray();\
             const view = new DataView(instance.exports.memory.buffer);\
             const len = view.getInt32(ptr, true);\
             const a = view.getInt32(ptr + 4, true);\
             const b = view.getInt32(ptr + 8, true);\
             const c = view.getInt32(ptr + 12, true);\
             console.log(`${len},${a},${b},${c}`);",
        );

        // Assert
        assert_eq!(output, "3,1,2,3");
    }

    #[test]
    fn executes_object_literal_member_access() {
        // Assign
        let source = r#"
            function getValue(): i32 {
                const payload = { value: 42, other: 7 };
                return payload.value;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const result = instance.exports.getValue(); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "42");
    }

    #[test]
    fn executes_struct_literal_constructor_and_member_access() {
        // Assign
        let source = r#"
            struct Point {
                x: i32;
                y: i32;
            }

            function getX(): i32 {
                const point = Point { x: 3, y: 9 };
                return point.x;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const result = instance.exports.getX(); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "3");
    }

    #[test]
    fn executes_array_index_and_length_access() {
        // Assign
        let source = r#"
            function select(): i32 {
                const values = [5, 7, 11];
                return values[1] + values.length;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const result = instance.exports.select(); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "10");
    }

    #[test]
    fn executes_internal_async_await_flow_standalone() {
        // Assign
        let source = r#"
            async function greet(): string {
                return "hello";
            }

            async function main(): string {
                const value = await greet();
                return value;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const ptr = instance.exports.main();\
             const view = new DataView(instance.exports.memory.buffer);\
             const len = view.getUint32(ptr, true);\
             const bytes = new Uint8Array(instance.exports.memory.buffer, ptr + 4, len);\
             console.log(new TextDecoder().decode(bytes));",
        );

        // Assert
        assert_eq!(output, "hello");
    }

    #[test]
    fn executes_flat_try_catch_throw_standalone() {
        // Assign
        let source = r#"
            function recover(): i32 {
                let value = 1;
                try {
                    throw 7;
                } catch (err) {
                    value = err;
                }
                return value;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "const result = instance.exports.recover(); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "7");
    }

    #[test]
    fn executes_structured_try_catch_with_returns_standalone() {
        // Assign
        let source = r#"
            function recover(flag: bool): i32 {
                try {
                    if (flag) {
                        throw 7;
                    }
                    return 1;
                } catch (err) {
                    return err;
                }

                return 0;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "console.log(`${instance.exports.recover(0)},${instance.exports.recover(1)}`);",
        );

        // Assert
        assert_eq!(output, "1,7");
    }

    #[test]
    fn executes_loop_control_inside_try_standalone() {
        // Assign
        let source = r#"
            function countUntil(limit: i32): i32 {
                let i = 0;
                try {
                    while (i < limit) {
                        i = i + 1;
                        if (i == 2) {
                            continue;
                        }
                        if (i == 4) {
                            break;
                        }
                    }
                } finally {
                    const done = true;
                }

                return i;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "console.log(`${instance.exports.countUntil(3)},${instance.exports.countUntil(10)}`);",
        );

        // Assert
        assert_eq!(output, "3,4");
    }

    #[test]
    fn executes_for_of_inside_try_standalone() {
        // Assign
        let source = r#"
            function sumUntil(): i32 {
                let sum = 0;
                try {
                    const items = [2, 3, 4, 5];
                    for (const item of items) {
                        sum = sum + item;
                        if (sum > 6) {
                            break;
                        }
                    }
                } finally {
                    const done = true;
                }

                return sum;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(&wasm, "console.log(String(instance.exports.sumUntil()));");

        // Assert
        assert_eq!(output, "9");
    }

    #[test]
    fn executes_switch_and_match_inside_try_standalone() {
        // Assign
        let source = r#"
            function choose(x: i32): i32 {
                let value = 0;
                try {
                    switch (x) {
                        case 1:
                            value = 10;
                            break;
                        case 2:
                            value = 20;
                            break;
                        default:
                            value = 30;
                    }
                } finally {
                    const done = true;
                }

                return value;
            }

            function classify(x: i32): i32 {
                let value = 0;
                try {
                    match (x) {
                        1 => value = 100,
                        2 => value = 200,
                    }
                } finally {
                    const done = true;
                }

                return value;
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script(
            &wasm,
            "console.log(`${instance.exports.choose(1)},${instance.exports.choose(5)},${instance.exports.classify(2)},${instance.exports.classify(9)}`);",
        );

        // Assert
        assert_eq!(output, "10,30,200,0");
    }

    #[test]
    fn executes_direct_function_import_standalone() {
        // Assign
        let source = r#"
            import inc from "./dep.mjs";

            function main(): i32 {
                return inc(4);
            }
        "#;
        let wasm = compile_source(source);

        // Act
        let output = run_node_script_with_imports(
            &wasm,
            r#"{
                "./dep.mjs": {
                    default(x) { return x + 1; }
                }
            }"#,
            "const result = instance.exports.main(); console.log(String(result));",
        );

        // Assert
        assert_eq!(output, "5");
    }
}
