//! Argon IR to Cranelift IR translation.
//!
//! Translates `argon_ir::Module` into Cranelift functions within an ObjectModule.

use crate::intrinsics::{self, LibcFunctions};
use crate::runtime;
use crate::types::pointer_type;
use crate::CodegenError;
use argon_ir::{
    BasicBlock, BinOp, ConstValue, Function as IrFunction, Instruction as IrInstruction, LogicOp,
    Module as IrModule, Terminator, TypeDef, UnOp, ValueId,
};
use argon_target::TargetTriple;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use std::collections::HashMap;

pub struct ModuleLowerer<'a> {
    module: &'a mut ObjectModule,
    triple: &'a TargetTriple,
    ptr_type: cranelift_codegen::ir::Type,
    libc: Option<LibcFunctions>,
    /// Maps function names to their FuncIds in the Cranelift module.
    func_ids: HashMap<String, FuncId>,
    /// Maps data names (string constants) to their DataId.
    data_ids: HashMap<String, cranelift_module::DataId>,
    /// Counter for generating unique data names.
    data_counter: u32,
    /// Struct type definitions from the IR module, used for field layout resolution.
    struct_layouts: Vec<Vec<String>>,
}

impl<'a> ModuleLowerer<'a> {
    pub fn new(module: &'a mut ObjectModule, triple: &'a TargetTriple) -> Self {
        let ptr_type = pointer_type(triple.pointer_bytes());
        Self {
            module,
            triple,
            ptr_type,
            libc: None,
            func_ids: HashMap::new(),
            data_ids: HashMap::new(),
            data_counter: 0,
            struct_layouts: Vec::new(),
        }
    }

    pub fn lower_module(&mut self, ir_module: &IrModule) -> Result<(), CodegenError> {
        // Collect struct field layouts from the IR module's type definitions.
        for ty in &ir_module.types {
            if let TypeDef::Struct { fields, .. } = ty {
                let layout: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                self.struct_layouts.push(layout);
            }
        }

        // Declare libc functions
        self.libc = Some(
            intrinsics::declare_libc_functions(self.module, self.ptr_type)
                .map_err(|e| CodegenError::CraneliftError(e.to_string()))?,
        );

        // If there are globals with init instructions, prepend them to __argon_init.
        // This ensures global variables are initialized before any user code runs.
        let mut patched_functions = ir_module.functions.clone();
        if !ir_module.globals.is_empty() {
            let mut global_inits: Vec<IrInstruction> = Vec::new();
            for g in &ir_module.globals {
                // Lower the global's init instructions
                for inst in &g.init_insts {
                    global_inits.push(inst.clone());
                }
                // Create a VarDecl for the global
                global_inits.push(IrInstruction::VarDecl {
                    kind: g.kind,
                    name: g.name.clone(),
                    init: g.init,
                });
            }

            // Find __argon_init and prepend global inits
            if let Some(init_func) = patched_functions
                .iter_mut()
                .find(|f| f.id == "__argon_init")
            {
                if let Some(first_block) = init_func.body.first_mut() {
                    let mut combined = global_inits;
                    combined.append(&mut first_block.instructions);
                    first_block.instructions = combined;
                }
            } else if !global_inits.is_empty() {
                // No __argon_init exists, create one with globals
                patched_functions.push(IrFunction {
                    id: "__argon_init".to_string(),
                    params: Vec::new(),
                    return_type: None,
                    is_async: false,
                    body: vec![BasicBlock {
                        id: 0,
                        instructions: global_inits,
                        terminator: Terminator::Return(None),
                    }],
                });
            }
        }

        // First pass: declare all functions
        for func in &patched_functions {
            self.declare_function(func)?;
        }

        // Second pass: define all functions
        for func in &patched_functions {
            self.define_function(func)?;
        }

        // Generate main wrapper
        let init_id = self.func_ids.get("__argon_init").copied();
        runtime::define_main_wrapper(self.module, init_id)?;

        Ok(())
    }

    /// Check if a function returns a value by scanning its terminators.
    fn function_has_return_value(func: &IrFunction) -> bool {
        if func.return_type.is_some() {
            return true;
        }
        // Scan terminators for Return(Some(...))
        func.body
            .iter()
            .any(|block| matches!(&block.terminator, Terminator::Return(Some(_))))
    }

    fn declare_function(&mut self, func: &IrFunction) -> Result<(), CodegenError> {
        let mut sig = self.module.make_signature();

        for _param in &func.params {
            sig.params.push(AbiParam::new(types::F64));
        }

        if Self::function_has_return_value(func) {
            sig.returns.push(AbiParam::new(types::F64));
        }

        let linkage = if func.id == "__argon_init" {
            Linkage::Local
        } else {
            Linkage::Export
        };

        let func_id = self
            .module
            .declare_function(&func.id, linkage, &sig)
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        self.func_ids.insert(func.id.clone(), func_id);
        Ok(())
    }

    fn define_function(&mut self, func: &IrFunction) -> Result<(), CodegenError> {
        if func.is_async {
            return Err(CodegenError::Unsupported(
                "async functions are not supported for the native target".to_string(),
            ));
        }

        let func_id = *self
            .func_ids
            .get(&func.id)
            .ok_or_else(|| CodegenError::IrError(format!("undeclared function: {}", func.id)))?;

        let mut ctx = self.module.make_context();

        // Build signature
        for _param in &func.params {
            ctx.func.signature.params.push(AbiParam::new(types::F64));
        }
        if Self::function_has_return_value(func) {
            ctx.func.signature.returns.push(AbiParam::new(types::F64));
        }

        let mut fbc = FunctionBuilderContext::new();
        {
            let builder = FunctionBuilder::new(&mut ctx.func, &mut fbc);
            let func_lowerer = FunctionLowerer::new(
                builder,
                self.module,
                self.triple,
                &self.func_ids,
                &self.libc,
                &mut self.data_ids,
                &mut self.data_counter,
                self.ptr_type,
                &self.struct_layouts,
            );
            func_lowerer.lower(func)?;
        }

        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| CodegenError::CraneliftError(format!("define {}: {}", func.id, e)))?;

        self.module.clear_context(&mut ctx);
        Ok(())
    }
}

struct StringConstInfo {
    len: usize,
}

/// Lowers a single Argon IR function into Cranelift IR.
struct FunctionLowerer<'a, 'b> {
    builder: FunctionBuilder<'b>,
    module: &'a mut ObjectModule,
    #[allow(dead_code)]
    triple: &'a TargetTriple,
    func_ids: &'a HashMap<String, FuncId>,
    libc: &'a Option<LibcFunctions>,
    data_ids: &'a mut HashMap<String, cranelift_module::DataId>,
    data_counter: &'a mut u32,
    ptr_type: cranelift_codegen::ir::Type,
    /// Maps Argon IR ValueIds to Cranelift Values.
    values: HashMap<ValueId, Value>,
    /// Maps variable names to Cranelift Variables.
    variables: HashMap<String, Variable>,
    /// Next variable index.
    next_var: usize,
    /// Stack of (break_block, continue_block) for loop handling.
    loop_stack: Vec<(cranelift_codegen::ir::Block, cranelift_codegen::ir::Block)>,
    /// Maps ValueIds to function names (populated by VarRef for call resolution).
    callee_names: HashMap<ValueId, String>,
    /// Maps ValueIds to string constant info (for print calls).
    string_constants: HashMap<ValueId, StringConstInfo>,
    /// Maps ValueIds to their struct field layout (ordered field names).
    /// Used to compute field offsets for Member access.
    field_layouts: HashMap<ValueId, Vec<String>>,
    /// Maps variable names to their struct field layout.
    var_field_layouts: HashMap<String, Vec<String>>,
    /// Tracks which ValueIds originated from boolean values.
    bool_values: std::collections::HashSet<ValueId>,
    /// Tracks which variable names hold boolean values.
    bool_vars: std::collections::HashSet<String>,
    /// Known struct field layouts from the IR module's type definitions.
    struct_layouts: &'a [Vec<String>],
}

impl<'a, 'b> FunctionLowerer<'a, 'b> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        builder: FunctionBuilder<'b>,
        module: &'a mut ObjectModule,
        triple: &'a TargetTriple,
        func_ids: &'a HashMap<String, FuncId>,
        libc: &'a Option<LibcFunctions>,
        data_ids: &'a mut HashMap<String, cranelift_module::DataId>,
        data_counter: &'a mut u32,
        ptr_type: cranelift_codegen::ir::Type,
        struct_layouts: &'a [Vec<String>],
    ) -> Self {
        Self {
            builder,
            module,
            triple,
            func_ids,
            libc,
            data_ids,
            data_counter,
            ptr_type,
            values: HashMap::new(),
            variables: HashMap::new(),
            next_var: 0,
            loop_stack: Vec::new(),
            callee_names: HashMap::new(),
            string_constants: HashMap::new(),
            field_layouts: HashMap::new(),
            var_field_layouts: HashMap::new(),
            bool_values: std::collections::HashSet::new(),
            bool_vars: std::collections::HashSet::new(),
            struct_layouts,
        }
    }

    fn lower(mut self, func: &IrFunction) -> Result<(), CodegenError> {
        if func.body.is_empty() {
            let entry_block = self.builder.create_block();
            self.builder
                .append_block_params_for_function_params(entry_block);
            self.builder.switch_to_block(entry_block);
            self.builder.seal_block(entry_block);
            if ModuleLowerer::function_has_return_value(func) {
                let zero = self.builder.ins().f64const(0.0);
                self.builder.ins().return_(&[zero]);
            } else {
                self.builder.ins().return_(&[]);
            }
            self.builder.finalize();
            return Ok(());
        }

        // Map IR block IDs to Cranelift blocks.
        // IR blocks may not be stored in ID order, so we use a HashMap.
        let mut block_map: HashMap<usize, cranelift_codegen::ir::Block> = HashMap::new();
        for ir_block in &func.body {
            let cl_block = self.builder.create_block();
            block_map.insert(ir_block.id, cl_block);
        }

        // The first IR block (by position) is the entry.
        let entry_id = func.body[0].id;
        let entry_cl = block_map[&entry_id];

        self.builder
            .append_block_params_for_function_params(entry_cl);
        self.builder.switch_to_block(entry_cl);

        for (i, param) in func.params.iter().enumerate() {
            let val = self.builder.block_params(entry_cl)[i];
            let var = self.declare_variable(&param.name, types::F64);
            self.builder.def_var(var, val);
        }

        // Lower each IR block: instructions first, then terminator.
        for (bi, ir_block) in func.body.iter().enumerate() {
            if bi > 0 {
                self.builder.switch_to_block(block_map[&ir_block.id]);
            }

            self.lower_instructions(&ir_block.instructions)?;

            match &ir_block.terminator {
                Terminator::Return(Some(val_id)) => {
                    if let Some(&val) = self.values.get(val_id) {
                        self.builder.ins().return_(&[val]);
                    } else {
                        let zero = self.builder.ins().f64const(0.0);
                        self.builder.ins().return_(&[zero]);
                    }
                }
                Terminator::Return(None) => {
                    self.builder.ins().return_(&[]);
                }
                Terminator::Branch {
                    cond,
                    then: then_id,
                    else_: else_id,
                } => {
                    let cond_val = self.get_value(*cond)?;
                    let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);
                    self.builder.ins().brif(
                        is_true,
                        block_map[then_id],
                        &[],
                        block_map[else_id],
                        &[],
                    );
                }
                Terminator::Jump(target_id) => {
                    self.builder.ins().jump(block_map[target_id], &[]);
                }
                Terminator::Unreachable => {
                    self.builder
                        .ins()
                        .trap(cranelift_codegen::ir::TrapCode::unwrap_user(0));
                }
            }
        }

        // Seal all blocks (all predecessors are now known).
        for cl_block in block_map.values() {
            self.builder.seal_block(*cl_block);
        }

        self.builder.finalize();
        Ok(())
    }

    fn lower_instructions(&mut self, instructions: &[IrInstruction]) -> Result<(), CodegenError> {
        for inst in instructions {
            self.lower_instruction(inst)?;
        }
        Ok(())
    }

    fn lower_instruction(&mut self, inst: &IrInstruction) -> Result<(), CodegenError> {
        match inst {
            IrInstruction::Const { dest, value } => {
                let val = match value {
                    ConstValue::Number(n) => self.builder.ins().f64const(*n),
                    ConstValue::Bool(b) => {
                        self.bool_values.insert(*dest);
                        let i = if *b { 1i64 } else { 0i64 };
                        let ival = self.builder.ins().iconst(types::I64, i);
                        self.builder.ins().fcvt_from_sint(types::F64, ival)
                    }
                    ConstValue::String(s) => {
                        // The parser includes surrounding quotes in string values;
                        // strip them for native output.
                        let stripped = s
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .or_else(|| s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                            .unwrap_or(s);
                        let len = stripped.len();
                        let ptr = self.create_data_string(&format!("{}\0", stripped))?;
                        self.string_constants.insert(*dest, StringConstInfo { len });
                        ptr
                    }
                    ConstValue::Null => self.builder.ins().f64const(0.0),
                };
                self.values.insert(*dest, val);
            }

            IrInstruction::BinOp { op, lhs, rhs, dest } => {
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = self.lower_binop(*op, lhs_val, rhs_val)?;
                // Mark comparison results as booleans for proper true/false printing
                if matches!(
                    op,
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                ) {
                    self.bool_values.insert(*dest);
                }
                self.values.insert(*dest, result);
            }

            IrInstruction::UnOp { op, arg, dest } => {
                if matches!(op, UnOp::Not) {
                    self.bool_values.insert(*dest);
                }
                let arg_val = self.get_value(*arg)?;
                let result = match op {
                    UnOp::Neg => self.builder.ins().fneg(arg_val),
                    UnOp::Not => {
                        // Convert to int, xor with 1, convert back
                        let ival = self.builder.ins().fcvt_to_sint(types::I64, arg_val);
                        let one = self.builder.ins().iconst(types::I64, 1);
                        let result = self.builder.ins().bxor(ival, one);
                        self.builder.ins().fcvt_from_sint(types::F64, result)
                    }
                };
                self.values.insert(*dest, result);
            }

            IrInstruction::VarDecl { name, init, .. } => {
                let var = self.declare_variable(name, types::F64);
                if let Some(init_id) = init {
                    let val = self.get_value(*init_id)?;
                    self.builder.def_var(var, val);
                    // Propagate field layout to the variable
                    if let Some(layout) = self.field_layouts.get(init_id).cloned() {
                        self.var_field_layouts.insert(name.clone(), layout);
                    }
                    // Propagate boolean tracking
                    if self.bool_values.contains(init_id) {
                        self.bool_vars.insert(name.clone());
                    }
                } else {
                    let zero = self.builder.ins().f64const(0.0);
                    self.builder.def_var(var, zero);
                }
            }

            IrInstruction::VarRef { dest, name } => {
                // Track name -> ValueId for resolving call targets
                self.callee_names.insert(*dest, name.clone());

                if let Some(&var) = self.variables.get(name) {
                    let val = self.builder.use_var(var);
                    self.values.insert(*dest, val);
                    // Propagate field layout from variable to this value
                    if let Some(layout) = self.var_field_layouts.get(name).cloned() {
                        self.field_layouts.insert(*dest, layout);
                    }
                    // Propagate boolean tracking
                    if self.bool_vars.contains(name) {
                        self.bool_values.insert(*dest);
                    }
                } else {
                    // Could be a function reference or unknown variable
                    let zero = self.builder.ins().f64const(0.0);
                    self.values.insert(*dest, zero);
                }
            }

            IrInstruction::AssignVar { name, src } => {
                let val = self.get_value(*src)?;
                if let Some(&var) = self.variables.get(name) {
                    self.builder.def_var(var, val);
                } else {
                    let var = self.declare_variable(name, types::F64);
                    self.builder.def_var(var, val);
                }
                // Propagate field layout
                if let Some(layout) = self.field_layouts.get(src).cloned() {
                    self.var_field_layouts.insert(name.clone(), layout);
                }
            }

            IrInstruction::AssignExpr { name, src, dest } => {
                let val = self.get_value(*src)?;
                if let Some(&var) = self.variables.get(name) {
                    self.builder.def_var(var, val);
                } else {
                    let var = self.declare_variable(name, types::F64);
                    self.builder.def_var(var, val);
                }
                // Propagate field layout
                if let Some(layout) = self.field_layouts.get(src).cloned() {
                    self.var_field_layouts.insert(name.clone(), layout);
                }
                self.values.insert(*dest, val);
            }

            IrInstruction::Call { callee, args, dest } => {
                self.lower_call(*callee, args, *dest)?;
            }

            IrInstruction::If {
                cond,
                then_body,
                else_body,
            } => {
                self.lower_if(*cond, then_body, else_body)?;
            }

            IrInstruction::While {
                cond_instructions,
                cond,
                body,
            } => {
                self.lower_while(cond_instructions, *cond, body)?;
            }

            IrInstruction::For {
                init,
                cond_instructions,
                cond,
                update,
                body,
            } => {
                self.lower_for(init, cond_instructions, *cond, update, body)?;
            }

            IrInstruction::DoWhile {
                body,
                cond_instructions,
                cond,
            } => {
                self.lower_do_while(body, cond_instructions, *cond)?;
            }

            IrInstruction::Loop { body } => {
                self.lower_loop(body)?;
            }

            IrInstruction::Break => {
                if let Some(&(break_block, _)) = self.loop_stack.last() {
                    self.builder.ins().jump(break_block, &[]);
                    // Create an unreachable block to continue emitting after break
                    let dead = self.builder.create_block();
                    self.builder.switch_to_block(dead);
                    self.builder.seal_block(dead);
                }
            }

            IrInstruction::Continue => {
                if let Some(&(_, continue_block)) = self.loop_stack.last() {
                    self.builder.ins().jump(continue_block, &[]);
                    let dead = self.builder.create_block();
                    self.builder.switch_to_block(dead);
                    self.builder.seal_block(dead);
                }
            }

            IrInstruction::Return { value } => {
                if let Some(val_id) = value {
                    let val = self.get_value(*val_id)?;
                    self.builder.ins().return_(&[val]);
                } else {
                    self.builder.ins().return_(&[]);
                }
                // Create dead block after return
                let dead = self.builder.create_block();
                self.builder.switch_to_block(dead);
                self.builder.seal_block(dead);
            }

            IrInstruction::LogicalOp { op, lhs, rhs, dest } => {
                // If both operands are boolean, the result is boolean
                if self.bool_values.contains(lhs) && self.bool_values.contains(rhs) {
                    self.bool_values.insert(*dest);
                }
                let lhs_val = self.get_value(*lhs)?;
                let rhs_val = self.get_value(*rhs)?;
                let result = match op {
                    LogicOp::And => {
                        // Short-circuit: if lhs is falsy, return lhs, else return rhs
                        let lhs_int = self.builder.ins().fcvt_to_sint(types::I64, lhs_val);
                        let zero_i = self.builder.ins().iconst(types::I64, 0);
                        let is_zero = self.builder.ins().icmp(IntCC::Equal, lhs_int, zero_i);
                        self.builder.ins().select(is_zero, lhs_val, rhs_val)
                    }
                    LogicOp::Or => {
                        let lhs_int = self.builder.ins().fcvt_to_sint(types::I64, lhs_val);
                        let zero_i = self.builder.ins().iconst(types::I64, 0);
                        let is_nonzero = self.builder.ins().icmp(IntCC::NotEqual, lhs_int, zero_i);
                        self.builder.ins().select(is_nonzero, lhs_val, rhs_val)
                    }
                    LogicOp::Nullish => {
                        // For native, null is represented as 0.0, so same as Or
                        let lhs_int = self.builder.ins().fcvt_to_sint(types::I64, lhs_val);
                        let zero_i = self.builder.ins().iconst(types::I64, 0);
                        let is_nonzero = self.builder.ins().icmp(IntCC::NotEqual, lhs_int, zero_i);
                        self.builder.ins().select(is_nonzero, lhs_val, rhs_val)
                    }
                };
                self.values.insert(*dest, result);
            }

            IrInstruction::Conditional {
                cond,
                then_value,
                else_value,
                dest,
            } => {
                let cond_val = self.get_value(*cond)?;
                let then_val = self.get_value(*then_value)?;
                let else_val = self.get_value(*else_value)?;
                let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
                let zero = self.builder.ins().iconst(types::I64, 0);
                let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);
                let result = self.builder.ins().select(is_true, then_val, else_val);
                self.values.insert(*dest, result);
            }

            IrInstruction::ExprStmt { value: _ } => {
                // Expression statement - value is discarded
            }

            IrInstruction::Try { .. } => {
                return Err(CodegenError::Unsupported(
                    "try/catch is not supported for the native target; use Result types instead"
                        .to_string(),
                ));
            }

            IrInstruction::ThrowStmt { .. } => {
                return Err(CodegenError::Unsupported(
                    "throw is not supported for the native target; use Result types instead"
                        .to_string(),
                ));
            }

            IrInstruction::Await { .. } => {
                return Err(CodegenError::Unsupported(
                    "await is not supported for the native target".to_string(),
                ));
            }

            IrInstruction::Load { dest, src } => {
                let val = self.get_value(*src)?;
                self.values.insert(*dest, val);
            }

            IrInstruction::Store { dest, src } => {
                let val = self.get_value(*src)?;
                self.values.insert(*dest, val);
            }

            IrInstruction::Member {
                object,
                property,
                dest,
            } => {
                let obj_f64 = self.get_value(*object)?;
                // Try local layout first, then fall back to module-level struct definitions.
                let layout = self.field_layouts.get(object).cloned().or_else(|| {
                    self.struct_layouts
                        .iter()
                        .find(|sl| sl.iter().any(|f| f == property))
                        .cloned()
                });
                if let Some(layout) = layout {
                    // Bitcast F64 back to pointer for memory access
                    let obj_ptr =
                        self.builder
                            .ins()
                            .bitcast(self.ptr_type, MemFlags::new(), obj_f64);
                    if let Some(field_idx) = layout.iter().position(|k| k == property) {
                        let offset = (field_idx * 8) as i32;
                        let val =
                            self.builder
                                .ins()
                                .load(types::F64, MemFlags::new(), obj_ptr, offset);
                        self.values.insert(*dest, val);
                    } else {
                        let zero = self.builder.ins().f64const(0.0);
                        self.values.insert(*dest, zero);
                    }
                } else {
                    // No layout info — fallback to 0
                    let zero = self.builder.ins().f64const(0.0);
                    self.values.insert(*dest, zero);
                }
            }

            IrInstruction::MemberComputed {
                object,
                property: _,
                dest,
            } => {
                let obj_val = self.get_value(*object)?;
                self.values.insert(*dest, obj_val);
            }

            IrInstruction::ObjectLit { dest, props } => {
                let num_fields = props.len();
                if num_fields == 0 {
                    // Empty struct — return 0.0 as null
                    let zero = self.builder.ins().f64const(0.0);
                    self.values.insert(*dest, zero);
                } else {
                    // Allocate: malloc(num_fields * 8)
                    let libc = self.libc.as_ref().unwrap();
                    let malloc_ref = self
                        .module
                        .declare_func_in_func(libc.malloc, self.builder.func);
                    let alloc_size = self
                        .builder
                        .ins()
                        .iconst(self.ptr_type, (num_fields * 8) as i64);
                    let call = self.builder.ins().call(malloc_ref, &[alloc_size]);
                    let base_ptr = self.builder.inst_results(call)[0];

                    // Store each field value at its offset
                    let mut layout = Vec::with_capacity(num_fields);
                    for (i, prop) in props.iter().enumerate() {
                        let field_val = self.get_value(prop.value)?;
                        let offset = (i * 8) as i32;
                        self.builder
                            .ins()
                            .store(MemFlags::new(), field_val, base_ptr, offset);
                        layout.push(prop.key.clone());
                    }

                    self.field_layouts.insert(*dest, layout);
                    // Bitcast the pointer to F64 so all values have uniform type.
                    // Member access will bitcast back to pointer for loads.
                    let ptr_as_f64 =
                        self.builder
                            .ins()
                            .bitcast(types::F64, MemFlags::new(), base_ptr);
                    self.values.insert(*dest, ptr_as_f64);
                }
            }

            IrInstruction::ArrayLit { dest, elements } => {
                // Layout: [length (f64), elem0 (f64), elem1 (f64), ...]
                let num_elems = elements.len();
                let total_slots = num_elems + 1; // +1 for length
                let libc = self.libc.as_ref().unwrap();
                let malloc_ref = self
                    .module
                    .declare_func_in_func(libc.malloc, self.builder.func);
                let alloc_size = self
                    .builder
                    .ins()
                    .iconst(self.ptr_type, (total_slots * 8) as i64);
                let call = self.builder.ins().call(malloc_ref, &[alloc_size]);
                let base_ptr = self.builder.inst_results(call)[0];

                // Store length at offset 0
                let len_val = self.builder.ins().f64const(num_elems as f64);
                self.builder
                    .ins()
                    .store(MemFlags::new(), len_val, base_ptr, 0);

                // Store each element at offset (i+1)*8
                for (i, elem) in elements.iter().enumerate() {
                    let val = if let Some(elem_id) = elem {
                        self.get_value(*elem_id)?
                    } else {
                        self.builder.ins().f64const(0.0)
                    };
                    let offset = ((i + 1) * 8) as i32;
                    self.builder
                        .ins()
                        .store(MemFlags::new(), val, base_ptr, offset);
                }

                // Bitcast pointer to F64 for uniform value representation
                let ptr_as_f64 = self
                    .builder
                    .ins()
                    .bitcast(types::F64, MemFlags::new(), base_ptr);
                self.values.insert(*dest, ptr_as_f64);
            }

            IrInstruction::New { callee, args, dest } => {
                // For struct instantiation, the pattern is:
                //   ObjectLit { init_obj } → VarRef("StructName") → New(callee, [init_obj])
                // The ObjectLit already has the fields allocated on the heap.
                // We forward its pointer and layout to New's dest.
                if let Some(&first_arg) = args.first() {
                    if self.field_layouts.contains_key(&first_arg) {
                        // The init object is a struct with known layout — forward it
                        let ptr = self.get_value(first_arg)?;
                        self.values.insert(*dest, ptr);
                        if let Some(layout) = self.field_layouts.get(&first_arg).cloned() {
                            self.field_layouts.insert(*dest, layout);
                        }
                    } else {
                        // Try calling as a regular function
                        self.lower_call(*callee, args, *dest)?;
                    }
                } else {
                    // No args — try calling as a regular function
                    self.lower_call(*callee, args, *dest)?;
                }
            }
        }
        Ok(())
    }

    fn lower_binop(&mut self, op: BinOp, lhs: Value, rhs: Value) -> Result<Value, CodegenError> {
        let result = match op {
            BinOp::Add => self.builder.ins().fadd(lhs, rhs),
            BinOp::Sub => self.builder.ins().fsub(lhs, rhs),
            BinOp::Mul => self.builder.ins().fmul(lhs, rhs),
            BinOp::Div => self.builder.ins().fdiv(lhs, rhs),
            BinOp::Mod => {
                // fmod: a - floor(a/b) * b
                let div = self.builder.ins().fdiv(lhs, rhs);
                let floored = self.builder.ins().floor(div);
                let product = self.builder.ins().fmul(floored, rhs);
                self.builder.ins().fsub(lhs, product)
            }
            BinOp::Eq => {
                let cmp = self.builder.ins().fcmp(FloatCC::Equal, lhs, rhs);
                // Convert bool (i8) to f64: 0.0 or 1.0
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::Ne => {
                let cmp = self.builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs);
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::Lt => {
                let cmp = self.builder.ins().fcmp(FloatCC::LessThan, lhs, rhs);
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::Le => {
                let cmp = self.builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs);
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::Gt => {
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs);
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::Ge => {
                let cmp = self
                    .builder
                    .ins()
                    .fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs);
                let ext = self.builder.ins().uextend(types::I64, cmp);
                self.builder.ins().fcvt_from_sint(types::F64, ext)
            }
            BinOp::And => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().band(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
            BinOp::Or => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().bor(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
            BinOp::Xor => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().bxor(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
            BinOp::Shl => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().ishl(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
            BinOp::Shr => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().ushr(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
            BinOp::Sar => {
                let lhs_i = self.builder.ins().fcvt_to_sint(types::I64, lhs);
                let rhs_i = self.builder.ins().fcvt_to_sint(types::I64, rhs);
                let result = self.builder.ins().sshr(lhs_i, rhs_i);
                self.builder.ins().fcvt_from_sint(types::F64, result)
            }
        };
        Ok(result)
    }

    fn lower_call(
        &mut self,
        callee: ValueId,
        args: &[ValueId],
        dest: ValueId,
    ) -> Result<(), CodegenError> {
        // Resolve the callee name from the values or a VarRef
        // The IR uses ValueIds for callees, but the callee is typically a VarRef
        // pointing to a function name. We need to look up the function by name.

        // Check if this is a known intrinsic or function call.
        // First, try to find the callee name from a prior VarRef instruction.
        let callee_name = self.find_callee_name(callee);

        if let Some(name) = callee_name {
            // Check for print/println intrinsics
            if name == "print" || name == "println" {
                return self.lower_print_call(&name, args, dest);
            }

            // Check for math intrinsics
            if let Some(result) = self.try_lower_math_intrinsic(&name, args)? {
                self.values.insert(dest, result);
                return Ok(());
            }

            // Check for fs intrinsics
            if let Some(result) = self.try_lower_fs_intrinsic(&name, args, dest)? {
                self.values.insert(dest, result);
                return Ok(());
            }

            // Check for net/http/ws intrinsics — return 0 for now
            // (full native net/http/ws requires struct return values
            // which need further infrastructure work)
            if matches!(
                name.as_str(),
                "bind" | "connect" | "bindUdp" | "resolve"
                    | "get" | "post" | "put" | "del" | "request"
                    | "createHeaders" | "serve"
                    | "wsConnect" | "wsListen"
            ) {
                // These intrinsics are recognized but produce a stub return value
                // in native mode. Full implementation requires struct/object
                // return values via heap allocation, which is a future enhancement.
                let zero = self.builder.ins().f64const(0.0);
                self.values.insert(dest, zero);
                return Ok(());
            }

            // Regular function call
            if let Some(&func_id) = self.func_ids.get(&name) {
                let func_ref = self.module.declare_func_in_func(func_id, self.builder.func);
                let mut arg_vals = Vec::new();
                for arg_id in args {
                    arg_vals.push(self.get_value(*arg_id)?);
                }
                let call = self.builder.ins().call(func_ref, &arg_vals);
                let results = self.builder.inst_results(call);
                if !results.is_empty() {
                    self.values.insert(dest, results[0]);
                } else {
                    let zero = self.builder.ins().f64const(0.0);
                    self.values.insert(dest, zero);
                }
                return Ok(());
            }
        }

        // Fallback: unknown call, return 0
        let zero = self.builder.ins().f64const(0.0);
        self.values.insert(dest, zero);
        Ok(())
    }

    fn find_callee_name(&self, callee_id: ValueId) -> Option<String> {
        self.callee_names.get(&callee_id).cloned()
    }

    fn lower_print_call(
        &mut self,
        name: &str,
        args: &[ValueId],
        dest: ValueId,
    ) -> Result<(), CodegenError> {
        let libc = self.libc.as_ref().unwrap();

        if let Some(&arg_id) = args.first() {
            let arg_val = self.get_value(arg_id)?;

            // Check if this is a string constant (pointer)
            if let Some(str_data) = self.string_constants.get(&arg_id) {
                // Write the string using the C runtime helper
                let print_str_ref = self
                    .module
                    .declare_func_in_func(libc.print_str, self.builder.func);
                let len = self
                    .builder
                    .ins()
                    .iconst(self.ptr_type, str_data.len as i64);
                self.builder.ins().call(print_str_ref, &[arg_val, len]);
            } else if self.bool_values.contains(&arg_id) {
                // Print boolean as "true"/"false"
                let print_bool_ref = self
                    .module
                    .declare_func_in_func(libc.print_bool, self.builder.func);
                self.builder.ins().call(print_bool_ref, &[arg_val]);
            } else {
                // Format and print the number using the C runtime helper.
                // This avoids the variadic calling convention issue with snprintf
                // on aarch64 where variadic float args are passed on the stack.
                let print_f64_ref = self
                    .module
                    .declare_func_in_func(libc.print_f64, self.builder.func);
                self.builder.ins().call(print_f64_ref, &[arg_val]);
            }
        }

        // Print newline for println
        if name == "println" {
            let libc = self.libc.as_ref().unwrap();
            let nl_ptr = self.create_data_string("\n\0")?;
            let print_str_ref = self
                .module
                .declare_func_in_func(libc.print_str, self.builder.func);
            let one = self.builder.ins().iconst(self.ptr_type, 1);
            self.builder.ins().call(print_str_ref, &[nl_ptr, one]);
        }

        let zero = self.builder.ins().f64const(0.0);
        self.values.insert(dest, zero);
        Ok(())
    }

    fn try_lower_math_intrinsic(
        &mut self,
        name: &str,
        args: &[ValueId],
    ) -> Result<Option<Value>, CodegenError> {
        let libc = self.libc.as_ref().unwrap();

        match name {
            "sqrt" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().sqrt(arg)))
            }
            "abs" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().fabs(arg)))
            }
            "floor" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().floor(arg)))
            }
            "ceil" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().ceil(arg)))
            }
            "trunc" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().trunc(arg)))
            }
            "round" => {
                let arg = self.get_value(args[0])?;
                Ok(Some(self.builder.ins().nearest(arg)))
            }
            "min" => {
                let a = self.get_value(args[0])?;
                let b = self.get_value(args[1])?;
                Ok(Some(self.builder.ins().fmin(a, b)))
            }
            "max" => {
                let a = self.get_value(args[0])?;
                let b = self.get_value(args[1])?;
                Ok(Some(self.builder.ins().fmax(a, b)))
            }
            "sin" | "cos" | "tan" | "log" | "exp" => {
                let func_id = match name {
                    "sin" => libc.sin,
                    "cos" => libc.cos,
                    "tan" => libc.tan,
                    "log" => libc.log,
                    "exp" => libc.exp,
                    _ => unreachable!(),
                };
                let func_ref = self.module.declare_func_in_func(func_id, self.builder.func);
                let arg = self.get_value(args[0])?;
                let call = self.builder.ins().call(func_ref, &[arg]);
                Ok(Some(self.builder.inst_results(call)[0]))
            }
            "pow" => {
                let func_ref = self
                    .module
                    .declare_func_in_func(libc.pow, self.builder.func);
                let base = self.get_value(args[0])?;
                let exp = self.get_value(args[1])?;
                let call = self.builder.ins().call(func_ref, &[base, exp]);
                Ok(Some(self.builder.inst_results(call)[0]))
            }
            _ => Ok(None),
        }
    }

    /// Try to lower a std:fs intrinsic call.
    /// Returns Some(result_value) if the name matches, None otherwise.
    ///
    /// For fs operations that return Result types, the native codegen uses a
    /// simplified approach: functions that return data (readFile) print/store the
    /// result directly. Functions that return status (writeFile, mkdir, etc.)
    /// return 0.0 on success or a negative value on error.
    ///
    /// The full Result<T, IoError> struct semantics are handled by the runtime
    /// and JS targets. Native target provides direct access to the underlying
    /// operations for now.
    fn try_lower_fs_intrinsic(
        &mut self,
        name: &str,
        args: &[ValueId],
        _dest: ValueId,
    ) -> Result<Option<Value>, CodegenError> {
        let libc = self.libc.as_ref().unwrap();

        match name {
            "readFile" => {
                // readFile(path) -> calls __argon_fs_read_file, prints result
                // In native mode, readFile returns the content ptr as a value
                // that can be passed to println.
                if let Some(&arg_id) = args.first() {
                    let path_val = self.get_value(arg_id)?;
                    if let Some(str_data) = self.string_constants.get(&arg_id) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_read_file, self.builder.func);
                        let path_len = self
                            .builder
                            .ins()
                            .iconst(self.ptr_type, str_data.len as i64);
                        // Allocate stack slot for out_len
                        let out_len_slot = self.builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                8,
                                0,
                            ),
                        );
                        let out_len_ptr =
                            self.builder.ins().stack_addr(self.ptr_type, out_len_slot, 0);
                        let call =
                            self.builder
                                .ins()
                                .call(func_ref, &[path_val, path_len, out_len_ptr]);
                        let buf_ptr = self.builder.inst_results(call)[0];
                        // Return the buffer pointer — native callers use this
                        // as an opaque result. Full Result<> wrapping is a
                        // future enhancement.
                        return Ok(Some(buf_ptr));
                    }
                }
                Ok(Some(self.builder.ins().iconst(self.ptr_type, 0)))
            }
            "writeFile" => {
                if args.len() >= 2 {
                    let path_val = self.get_value(args[0])?;
                    let data_val = self.get_value(args[1])?;
                    let path_info = self.string_constants.get(&args[0]).map(|s| s.len);
                    let data_info = self.string_constants.get(&args[1]).map(|s| s.len);
                    if let (Some(path_len), Some(data_len)) = (path_info, data_info) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_write_file, self.builder.func);
                        let pl = self.builder.ins().iconst(self.ptr_type, path_len as i64);
                        let dl = self.builder.ins().iconst(self.ptr_type, data_len as i64);
                        let call =
                            self.builder
                                .ins()
                                .call(func_ref, &[path_val, pl, data_val, dl]);
                        let result = self.builder.inst_results(call)[0];
                        // Convert i32 result to f64 (0.0 on success)
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "appendFile" => {
                if args.len() >= 2 {
                    let path_val = self.get_value(args[0])?;
                    let data_val = self.get_value(args[1])?;
                    let path_info = self.string_constants.get(&args[0]).map(|s| s.len);
                    let data_info = self.string_constants.get(&args[1]).map(|s| s.len);
                    if let (Some(path_len), Some(data_len)) = (path_info, data_info) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_append_file, self.builder.func);
                        let pl = self.builder.ins().iconst(self.ptr_type, path_len as i64);
                        let dl = self.builder.ins().iconst(self.ptr_type, data_len as i64);
                        let call =
                            self.builder
                                .ins()
                                .call(func_ref, &[path_val, pl, data_val, dl]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "exists" => {
                if let Some(&arg_id) = args.first() {
                    let path_val = self.get_value(arg_id)?;
                    if let Some(str_data) = self.string_constants.get(&arg_id) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_exists, self.builder.func);
                        let path_len = self
                            .builder
                            .ins()
                            .iconst(self.ptr_type, str_data.len as i64);
                        let call = self.builder.ins().call(func_ref, &[path_val, path_len]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "remove" => {
                if let Some(&arg_id) = args.first() {
                    let path_val = self.get_value(arg_id)?;
                    if let Some(str_data) = self.string_constants.get(&arg_id) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_remove, self.builder.func);
                        let path_len = self
                            .builder
                            .ins()
                            .iconst(self.ptr_type, str_data.len as i64);
                        let call = self.builder.ins().call(func_ref, &[path_val, path_len]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "mkdir" => {
                if let Some(&arg_id) = args.first() {
                    let path_val = self.get_value(arg_id)?;
                    if let Some(str_data) = self.string_constants.get(&arg_id) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_mkdir, self.builder.func);
                        let path_len = self
                            .builder
                            .ins()
                            .iconst(self.ptr_type, str_data.len as i64);
                        let call = self.builder.ins().call(func_ref, &[path_val, path_len]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "rmdir" => {
                if let Some(&arg_id) = args.first() {
                    let path_val = self.get_value(arg_id)?;
                    if let Some(str_data) = self.string_constants.get(&arg_id) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_rmdir, self.builder.func);
                        let path_len = self
                            .builder
                            .ins()
                            .iconst(self.ptr_type, str_data.len as i64);
                        let call = self.builder.ins().call(func_ref, &[path_val, path_len]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            "rename" => {
                if args.len() >= 2 {
                    let from_val = self.get_value(args[0])?;
                    let to_val = self.get_value(args[1])?;
                    let from_info = self.string_constants.get(&args[0]).map(|s| s.len);
                    let to_info = self.string_constants.get(&args[1]).map(|s| s.len);
                    if let (Some(from_len), Some(to_len)) = (from_info, to_info) {
                        let func_ref = self
                            .module
                            .declare_func_in_func(libc.fs_rename, self.builder.func);
                        let fl = self.builder.ins().iconst(self.ptr_type, from_len as i64);
                        let tl = self.builder.ins().iconst(self.ptr_type, to_len as i64);
                        let call =
                            self.builder
                                .ins()
                                .call(func_ref, &[from_val, fl, to_val, tl]);
                        let result = self.builder.inst_results(call)[0];
                        let result_f64 = self.builder.ins().fcvt_from_sint(types::F64, result);
                        return Ok(Some(result_f64));
                    }
                }
                Ok(Some(self.builder.ins().f64const(0.0)))
            }
            _ => Ok(None),
        }
    }

    // --- Control flow ---

    fn lower_if(
        &mut self,
        cond: ValueId,
        then_body: &[IrInstruction],
        else_body: &[IrInstruction],
    ) -> Result<(), CodegenError> {
        let cond_val = self.get_value(cond)?;

        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let merge_block = self.builder.create_block();

        // Convert f64 condition to boolean
        let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);

        self.builder
            .ins()
            .brif(is_true, then_block, &[], else_block, &[]);

        // Then block
        self.builder.switch_to_block(then_block);
        self.builder.seal_block(then_block);
        self.lower_instructions(then_body)?;
        self.builder.ins().jump(merge_block, &[]);

        // Else block
        self.builder.switch_to_block(else_block);
        self.builder.seal_block(else_block);
        if !else_body.is_empty() {
            self.lower_instructions(else_body)?;
        }
        self.builder.ins().jump(merge_block, &[]);

        // Merge block
        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(merge_block);

        Ok(())
    }

    fn lower_while(
        &mut self,
        cond_instructions: &[IrInstruction],
        cond: ValueId,
        body: &[IrInstruction],
    ) -> Result<(), CodegenError> {
        let header_block = self.builder.create_block();
        let body_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        self.builder.ins().jump(header_block, &[]);

        // Header: evaluate condition
        self.builder.switch_to_block(header_block);
        self.lower_instructions(cond_instructions)?;
        let cond_val = self.get_value(cond)?;
        let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);
        self.builder
            .ins()
            .brif(is_true, body_block, &[], exit_block, &[]);

        // Body
        self.builder.switch_to_block(body_block);
        self.builder.seal_block(body_block);
        self.loop_stack.push((exit_block, header_block));
        self.lower_instructions(body)?;
        self.loop_stack.pop();
        self.builder.ins().jump(header_block, &[]);

        // Seal header after body (back edge)
        self.builder.seal_block(header_block);

        // Exit
        self.builder.switch_to_block(exit_block);
        self.builder.seal_block(exit_block);

        Ok(())
    }

    fn lower_for(
        &mut self,
        init: &[IrInstruction],
        cond_instructions: &[IrInstruction],
        cond: ValueId,
        update: &[IrInstruction],
        body: &[IrInstruction],
    ) -> Result<(), CodegenError> {
        // Init
        self.lower_instructions(init)?;

        let header_block = self.builder.create_block();
        let body_block = self.builder.create_block();
        let update_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        self.builder.ins().jump(header_block, &[]);

        // Header: evaluate condition
        self.builder.switch_to_block(header_block);
        self.lower_instructions(cond_instructions)?;
        let cond_val = self.get_value(cond)?;
        let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);
        self.builder
            .ins()
            .brif(is_true, body_block, &[], exit_block, &[]);

        // Body
        self.builder.switch_to_block(body_block);
        self.builder.seal_block(body_block);
        self.loop_stack.push((exit_block, update_block));
        self.lower_instructions(body)?;
        self.loop_stack.pop();
        self.builder.ins().jump(update_block, &[]);

        // Update
        self.builder.switch_to_block(update_block);
        self.builder.seal_block(update_block);
        self.lower_instructions(update)?;
        self.builder.ins().jump(header_block, &[]);

        // Seal header after back edge
        self.builder.seal_block(header_block);

        // Exit
        self.builder.switch_to_block(exit_block);
        self.builder.seal_block(exit_block);

        Ok(())
    }

    fn lower_do_while(
        &mut self,
        body: &[IrInstruction],
        cond_instructions: &[IrInstruction],
        cond: ValueId,
    ) -> Result<(), CodegenError> {
        let body_block = self.builder.create_block();
        let cond_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        self.builder.ins().jump(body_block, &[]);

        // Body (executes at least once)
        self.builder.switch_to_block(body_block);
        self.loop_stack.push((exit_block, cond_block));
        self.lower_instructions(body)?;
        self.loop_stack.pop();
        self.builder.ins().jump(cond_block, &[]);

        // Condition
        self.builder.switch_to_block(cond_block);
        self.builder.seal_block(cond_block);
        self.lower_instructions(cond_instructions)?;
        let cond_val = self.get_value(cond)?;
        let cond_int = self.builder.ins().fcvt_to_sint(types::I64, cond_val);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_true = self.builder.ins().icmp(IntCC::NotEqual, cond_int, zero);
        self.builder
            .ins()
            .brif(is_true, body_block, &[], exit_block, &[]);

        // Seal body after back edge
        self.builder.seal_block(body_block);

        // Exit
        self.builder.switch_to_block(exit_block);
        self.builder.seal_block(exit_block);

        Ok(())
    }

    fn lower_loop(&mut self, body: &[IrInstruction]) -> Result<(), CodegenError> {
        let body_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        self.builder.ins().jump(body_block, &[]);

        self.builder.switch_to_block(body_block);
        self.loop_stack.push((exit_block, body_block));
        self.lower_instructions(body)?;
        self.loop_stack.pop();
        self.builder.ins().jump(body_block, &[]);

        self.builder.seal_block(body_block);

        self.builder.switch_to_block(exit_block);
        self.builder.seal_block(exit_block);

        Ok(())
    }

    // --- Helpers ---

    fn get_value(&self, id: ValueId) -> Result<Value, CodegenError> {
        self.values
            .get(&id)
            .copied()
            .ok_or_else(|| CodegenError::IrError(format!("undefined value: v{}", id)))
    }

    fn declare_variable(&mut self, name: &str, ty: cranelift_codegen::ir::Type) -> Variable {
        if let Some(&var) = self.variables.get(name) {
            return var;
        }
        let var = Variable::from_u32(self.next_var as u32);
        self.next_var += 1;
        self.builder.declare_var(var, ty);
        self.variables.insert(name.to_string(), var);
        var
    }

    fn create_data_string(&mut self, s: &str) -> Result<Value, CodegenError> {
        let name = if let Some(id) = self.data_ids.get(s) {
            let gv = self.module.declare_data_in_func(*id, self.builder.func);
            return Ok(self.builder.ins().global_value(self.ptr_type, gv));
        } else {
            let name = format!("__str_{}", self.data_counter);
            *self.data_counter += 1;
            name
        };

        let data_id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        let mut desc = DataDescription::new();
        desc.define(s.as_bytes().to_vec().into_boxed_slice());

        self.module
            .define_data(data_id, &desc)
            .map_err(|e| CodegenError::CraneliftError(e.to_string()))?;

        self.data_ids.insert(s.to_string(), data_id);

        let gv = self.module.declare_data_in_func(data_id, self.builder.func);
        Ok(self.builder.ins().global_value(self.ptr_type, gv))
    }
}
