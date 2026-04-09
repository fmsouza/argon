use super::*;

impl JsCodegen {
    pub(crate) fn generate_ir_init(
        &mut self,
        func: &argon_ir::Function,
    ) -> Result<(), CodegenError> {
        if func.body.len() == 1
            && matches!(func.body[0].terminator, argon_ir::Terminator::Return(_))
        {
            let entry = func.body.first().ok_or_else(|| {
                CodegenError::Unsupported("init function has no body".to_string())
            })?;
            self.output.push_str("(() => {\n");
            self.generate_ir_block(entry, true)?;
            self.output.push_str("})();\n\n");
            return Ok(());
        }

        self.output.push_str("(() => {\n");
        self.generate_ir_cfg(func, true)?;
        self.output.push_str("})();\n\n");
        Ok(())
    }

    pub(crate) fn generate_ir_function(
        &mut self,
        func: &argon_ir::Function,
    ) -> Result<(), CodegenError> {
        if func.body.len() == 1
            && matches!(func.body[0].terminator, argon_ir::Terminator::Return(_))
        {
            let entry = func
                .body
                .first()
                .ok_or_else(|| CodegenError::Unsupported("function has no body".to_string()))?;

            if func.is_async {
                self.output.push_str("async ");
            }
            self.output.push_str("function ");
            self.output.push_str(&func.id);
            self.output.push('(');
            for (i, p) in func.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(&p.name);
            }
            self.output.push_str(") {\n");
            self.generate_ir_block(entry, false)?;
            self.output.push_str("}\n\n");
            return Ok(());
        }

        self.generate_ir_cfg(func, false)
    }

    fn generate_ir_cfg(
        &mut self,
        func: &argon_ir::Function,
        is_init: bool,
    ) -> Result<(), CodegenError> {
        use argon_ir::Terminator as IrTerm;

        if !is_init {
            if func.is_async {
                self.output.push_str("async ");
            }
            self.output.push_str("function ");
            self.output.push_str(&func.id);
            self.output.push('(');
            for (i, p) in func.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(&p.name);
            }
            self.output.push_str(") {\n");
        }

        let entry_id = func
            .body
            .first()
            .ok_or_else(|| CodegenError::Unsupported("function has no body".to_string()))?
            .id;

        self.output
            .push_str(&format!("    let __bb = {};\n", entry_id));
        self.output.push_str("    while (true) {\n");
        self.output.push_str("        switch (__bb) {\n");

        for block in &func.body {
            self.output
                .push_str(&format!("        case {}: {{\n", block.id));

            let mut values: std::collections::HashMap<argon_ir::ValueId, String> =
                std::collections::HashMap::new();
            self.emit_ir_instructions_cfg_with_prefix(
                &block.instructions,
                &mut values,
                "            ",
            )?;

            match &block.terminator {
                IrTerm::Jump(target) => {
                    self.output
                        .push_str(&format!("            __bb = {};\n", target));
                    self.output.push_str("            continue;\n");
                }
                IrTerm::Branch { cond, then, else_ } => {
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output
                        .push_str(&format!("            if ({}) {{\n", cond_expr));
                    self.output
                        .push_str(&format!("                __bb = {};\n", then));
                    self.output.push_str("            } else {\n");
                    self.output
                        .push_str(&format!("                __bb = {};\n", else_));
                    self.output.push_str("            }\n");
                    self.output.push_str("            continue;\n");
                }
                IrTerm::Return(Some(v)) => {
                    let expr = values
                        .get(v)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output
                        .push_str(&format!("            return {};\n", expr));
                }
                IrTerm::Return(None) => {
                    self.output.push_str("            return;\n");
                }
                IrTerm::Unreachable => {
                    self.output
                        .push_str("            throw new Error(\"unreachable\");\n");
                }
                IrTerm::EnumMatch { .. } => {
                    // Enum match is used by async state machines; JS skips the async lowering pass.
                    self.output
                        .push_str("            throw new Error(\"enum match not supported in JS codegen\");\n");
                }
            }

            self.output.push_str("        }\n");
        }

        self.output.push_str("        default: {\n");
        self.output
            .push_str("            throw new Error(\"invalid basic block\");\n");
        self.output.push_str("        }\n");
        self.output.push_str("        }\n");
        self.output.push_str("    }\n");

        if !is_init {
            self.output.push_str("}\n\n");
        }

        Ok(())
    }

    fn emit_ir_instructions_cfg_with_prefix(
        &mut self,
        instructions: &[argon_ir::Instruction],
        values: &mut std::collections::HashMap<argon_ir::ValueId, String>,
        prefix: &str,
    ) -> Result<(), CodegenError> {
        use argon_ir::Instruction as IrInst;

        for inst in instructions {
            match inst {
                IrInst::ObjectLit { dest, props } => {
                    let mut parts = Vec::new();
                    for p in props {
                        let v = values
                            .get(&p.value)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        parts.push(format!("{}: {}", p.key, v));
                    }
                    values.insert(*dest, format!("{{ {} }}", parts.join(", ")));
                }
                IrInst::New { callee, args, dest } => {
                    let callee = values
                        .get(callee)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let args = args
                        .iter()
                        .map(|a| {
                            values
                                .get(a)
                                .cloned()
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("new {}({})", callee, args));
                }
                IrInst::Await { arg, dest } => {
                    let arg = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("await {}", arg));
                }
                IrInst::AssignExpr { name, src, dest } => {
                    let expr = values
                        .get(src)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("({} = {})", name, expr));
                }
                IrInst::Const { dest, value } => {
                    values.insert(*dest, self.const_to_js(value));
                }
                IrInst::VarRef { dest, name } => {
                    values.insert(*dest, name.clone());
                }
                IrInst::Member {
                    object,
                    property,
                    dest,
                } => {
                    let obj = values
                        .get(object)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("{}.{}", obj, property));
                }
                IrInst::MemberComputed {
                    object,
                    property,
                    dest,
                } => {
                    let obj = values
                        .get(object)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let prop = values
                        .get(property)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("{}[{}]", obj, prop));
                }
                IrInst::BinOp { op, lhs, rhs, dest } => {
                    let lhs = values
                        .get(lhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let rhs = values
                        .get(rhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(
                        *dest,
                        format!("({} {} {})", lhs, self.binop_to_js(*op), rhs),
                    );
                }
                IrInst::UnOp { op, arg, dest } => {
                    let arg = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("({}{})", self.unop_to_js(*op), arg));
                }
                IrInst::Call { callee, args, dest } => {
                    let callee = values
                        .get(callee)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let args = args
                        .iter()
                        .map(|a| {
                            values
                                .get(a)
                                .cloned()
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("{}({})", callee, args));
                }
                IrInst::ArrayLit { dest, elements } => {
                    let parts = elements
                        .iter()
                        .map(|e| {
                            e.and_then(|id| values.get(&id).cloned())
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("[{}]", parts));
                }
                IrInst::LogicalOp { op, lhs, rhs, dest } => {
                    let lhs = values
                        .get(lhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let rhs = values
                        .get(rhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let op_str = match op {
                        argon_ir::LogicOp::And => "&&",
                        argon_ir::LogicOp::Or => "||",
                        argon_ir::LogicOp::Nullish => "??",
                    };
                    values.insert(*dest, format!("({} {} {})", lhs, op_str, rhs));
                }
                IrInst::Conditional {
                    cond,
                    then_value,
                    else_value,
                    dest,
                } => {
                    let cond = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let then_value = values
                        .get(then_value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let else_value = values
                        .get(else_value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(
                        *dest,
                        format!("({} ? {} : {})", cond, then_value, else_value),
                    );
                }
                IrInst::ThrowStmt { arg } => {
                    let expr = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str("throw ");
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                IrInst::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&format!("if ({}) {{\n", cond_expr));
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_cfg_with_prefix(then_body, values, &nested_prefix)?;
                    self.output.push_str(prefix);
                    self.output.push('}');
                    if !else_body.is_empty() {
                        self.output.push_str(" else {\n");
                        self.emit_ir_instructions_cfg_with_prefix(
                            else_body,
                            values,
                            &nested_prefix,
                        )?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }
                    self.output.push('\n');
                }
                IrInst::While {
                    cond_instructions,
                    cond,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_cfg_with_prefix(cond_instructions, values, prefix)?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(") {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::DoWhile {
                    body,
                    cond_instructions,
                    cond,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": do {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output
                        .push_str(&format!("}} while ({});\n", cond_expr));
                }
                IrInst::For {
                    init,
                    cond_instructions,
                    cond,
                    update,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_cfg_with_prefix(init, values, prefix)?;
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("if (!(");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(")) break ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        Some(update),
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(update, values, &nested_prefix)?;
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::Loop { body } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::Break => {
                    self.output.push_str(prefix);
                    self.output.push_str("break;\n");
                }
                IrInst::Continue => {
                    self.output.push_str(prefix);
                    self.output.push_str("continue;\n");
                }
                IrInst::Return { value } => {
                    self.output.push_str(prefix);
                    self.output.push_str("return");
                    if let Some(value) = value {
                        let expr = values
                            .get(value)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        self.output.push(' ');
                        self.output.push_str(&expr);
                    }
                    self.output.push_str(";\n");
                }
                IrInst::Try {
                    try_body,
                    catch,
                    finally_body,
                } => {
                    self.output.push_str(prefix);
                    self.output.push_str("try {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_cfg_with_prefix(try_body, values, &nested_prefix)?;
                    self.output.push_str(prefix);
                    self.output.push('}');

                    if let Some(c) = catch {
                        self.output.push_str(" catch");
                        if let Some(ref p) = c.param {
                            self.output.push_str(" (");
                            self.output.push_str(p);
                            self.output.push(')');
                        }
                        self.output.push_str(" {\n");
                        self.emit_ir_instructions_cfg_with_prefix(&c.body, values, &nested_prefix)?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }

                    if let Some(f) = finally_body {
                        self.output.push_str(" finally {\n");
                        self.emit_ir_instructions_cfg_with_prefix(f, values, &nested_prefix)?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }

                    self.output.push('\n');
                }
                IrInst::VarDecl { name, init, .. } => {
                    self.output.push_str(prefix);
                    self.output.push_str("var ");
                    self.output.push_str(name);
                    if let Some(v) = init {
                        let expr = values
                            .get(v)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        self.output.push_str(" = ");
                        self.output.push_str(&expr);
                    }
                    self.output.push_str(";\n");
                }
                IrInst::AssignVar { name, src } => {
                    let expr = values
                        .get(src)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(name);
                    self.output.push_str(" = ");
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                IrInst::ExprStmt { value } => {
                    let expr = values
                        .get(value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                _ => {
                    return Err(CodegenError::Unsupported(format!(
                        "unsupported IR instruction in CFG mode: {:?}",
                        inst
                    )));
                }
            }
        }

        Ok(())
    }

    fn emit_ir_loop_body_cfg_with_prefix(
        &mut self,
        instructions: &[argon_ir::Instruction],
        values: &mut std::collections::HashMap<argon_ir::ValueId, String>,
        prefix: &str,
        loop_label: &str,
        continue_prelude: Option<&[argon_ir::Instruction]>,
    ) -> Result<(), CodegenError> {
        use argon_ir::Instruction as IrInst;

        for inst in instructions {
            match inst {
                IrInst::Break => {
                    self.output.push_str(prefix);
                    self.output.push_str("break ");
                    self.output.push_str(loop_label);
                    self.output.push_str(";\n");
                }
                IrInst::Continue => {
                    if let Some(prelude) = continue_prelude {
                        self.emit_ir_instructions_cfg_with_prefix(prelude, values, prefix)?;
                    }
                    self.output.push_str(prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(loop_label);
                    self.output.push_str(";\n");
                }
                IrInst::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&format!("if ({}) {{\n", cond_expr));
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        then_body,
                        values,
                        &nested_prefix,
                        loop_label,
                        continue_prelude,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push('}');
                    if !else_body.is_empty() {
                        self.output.push_str(" else {\n");
                        self.emit_ir_loop_body_cfg_with_prefix(
                            else_body,
                            values,
                            &nested_prefix,
                            loop_label,
                            continue_prelude,
                        )?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }
                    self.output.push('\n');
                }
                IrInst::While {
                    cond_instructions,
                    cond,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_cfg_with_prefix(cond_instructions, values, prefix)?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(") {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::For {
                    init,
                    cond_instructions,
                    cond,
                    update,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_cfg_with_prefix(init, values, prefix)?;
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("if (!(");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(")) break ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        Some(update),
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(update, values, &nested_prefix)?;
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::DoWhile {
                    body,
                    cond_instructions,
                    cond,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": do {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_cfg_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str("} while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(");\n");
                }
                IrInst::Loop { body } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_cfg_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                _ => {
                    self.emit_ir_instructions_cfg_with_prefix(
                        std::slice::from_ref(inst),
                        values,
                        prefix,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn generate_ir_block(
        &mut self,
        block: &argon_ir::BasicBlock,
        is_init: bool,
    ) -> Result<(), CodegenError> {
        use argon_ir::Terminator as IrTerm;

        let mut values: std::collections::HashMap<argon_ir::ValueId, String> =
            std::collections::HashMap::new();

        self.emit_ir_instructions_block_with_prefix(&block.instructions, &mut values, "    ")?;

        if !is_init {
            match &block.terminator {
                IrTerm::Return(Some(v)) => {
                    let expr = values
                        .get(v)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str("    return ");
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                IrTerm::Return(None) => {
                    self.output.push_str("    return;\n");
                }
                _ => {
                    return Err(CodegenError::Unsupported(format!(
                        "unsupported IR terminator: {:?}",
                        block.terminator
                    )));
                }
            }
        }

        Ok(())
    }

    fn emit_ir_instructions_block_with_prefix(
        &mut self,
        instructions: &[argon_ir::Instruction],
        values: &mut std::collections::HashMap<argon_ir::ValueId, String>,
        prefix: &str,
    ) -> Result<(), CodegenError> {
        use argon_ir::Instruction as IrInst;

        for inst in instructions {
            match inst {
                IrInst::ObjectLit { dest, props } => {
                    let mut parts = Vec::new();
                    for p in props {
                        let v = values
                            .get(&p.value)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        parts.push(format!("{}: {}", p.key, v));
                    }
                    values.insert(*dest, format!("{{ {} }}", parts.join(", ")));
                }
                IrInst::New { callee, args, dest } => {
                    let callee = values
                        .get(callee)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let args = args
                        .iter()
                        .map(|a| {
                            values
                                .get(a)
                                .cloned()
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("new {}({})", callee, args));
                }
                IrInst::Await { arg, dest } => {
                    let arg = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("await {}", arg));
                }
                IrInst::Const { dest, value } => {
                    values.insert(*dest, self.const_to_js(value));
                }
                IrInst::VarRef { dest, name } => {
                    values.insert(*dest, name.clone());
                }
                IrInst::AssignExpr { name, src, dest } => {
                    let expr = values
                        .get(src)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("({} = {})", name, expr));
                }
                IrInst::ThrowStmt { arg } => {
                    let expr = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str("throw ");
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                IrInst::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&format!("if ({}) {{\n", cond_expr));

                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_block_with_prefix(then_body, values, &nested_prefix)?;

                    self.output.push_str(prefix);
                    self.output.push('}');
                    if !else_body.is_empty() {
                        self.output.push_str(" else {\n");
                        self.emit_ir_instructions_block_with_prefix(
                            else_body,
                            values,
                            &nested_prefix,
                        )?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }
                    self.output.push('\n');
                }
                IrInst::While {
                    cond_instructions,
                    cond,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_block_with_prefix(cond_instructions, values, prefix)?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(") {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::DoWhile {
                    body,
                    cond_instructions,
                    cond,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": do {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output
                        .push_str(&format!("}} while ({});\n", cond_expr));
                }
                IrInst::For {
                    init,
                    cond_instructions,
                    cond,
                    update,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_block_with_prefix(init, values, prefix)?;
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("if (!(");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(")) break ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        Some(update),
                    )?;
                    self.emit_ir_instructions_block_with_prefix(update, values, &nested_prefix)?;
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::Loop { body } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::Break => {
                    self.output.push_str(prefix);
                    self.output.push_str("break;\n");
                }
                IrInst::Continue => {
                    self.output.push_str(prefix);
                    self.output.push_str("continue;\n");
                }
                IrInst::Return { value } => {
                    self.output.push_str(prefix);
                    self.output.push_str("return");
                    if let Some(value) = value {
                        let expr = values
                            .get(value)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        self.output.push(' ');
                        self.output.push_str(&expr);
                    }
                    self.output.push_str(";\n");
                }
                IrInst::Try {
                    try_body,
                    catch,
                    finally_body,
                } => {
                    self.output.push_str(prefix);
                    self.output.push_str("try {\n");

                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_block_with_prefix(try_body, values, &nested_prefix)?;

                    self.output.push_str(prefix);
                    self.output.push('}');

                    if let Some(c) = catch {
                        self.output.push_str(" catch");
                        if let Some(ref p) = c.param {
                            self.output.push_str(" (");
                            self.output.push_str(p);
                            self.output.push(')');
                        }
                        self.output.push_str(" {\n");
                        self.emit_ir_instructions_block_with_prefix(
                            &c.body,
                            values,
                            &nested_prefix,
                        )?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }

                    if let Some(f) = finally_body {
                        self.output.push_str(" finally {\n");
                        self.emit_ir_instructions_block_with_prefix(f, values, &nested_prefix)?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }

                    self.output.push('\n');
                }
                IrInst::Member {
                    object,
                    property,
                    dest,
                } => {
                    let obj = values
                        .get(object)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("{}.{}", obj, property));
                }
                IrInst::MemberComputed {
                    object,
                    property,
                    dest,
                } => {
                    let obj = values
                        .get(object)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let prop = values
                        .get(property)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("{}[{}]", obj, prop));
                }
                IrInst::BinOp { op, lhs, rhs, dest } => {
                    let lhs = values
                        .get(lhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let rhs = values
                        .get(rhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(
                        *dest,
                        format!("({} {} {})", lhs, self.binop_to_js(*op), rhs),
                    );
                }
                IrInst::UnOp { op, arg, dest } => {
                    let arg = values
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(*dest, format!("({}{})", self.unop_to_js(*op), arg));
                }
                IrInst::Call { callee, args, dest } => {
                    let callee = values
                        .get(callee)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let args = args
                        .iter()
                        .map(|a| {
                            values
                                .get(a)
                                .cloned()
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("{}({})", callee, args));
                }
                IrInst::ArrayLit { dest, elements } => {
                    let parts = elements
                        .iter()
                        .map(|e| {
                            e.and_then(|id| values.get(&id).cloned())
                                .unwrap_or_else(|| "undefined".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    values.insert(*dest, format!("[{}]", parts));
                }
                IrInst::LogicalOp { op, lhs, rhs, dest } => {
                    let lhs = values
                        .get(lhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let rhs = values
                        .get(rhs)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let op_str = match op {
                        argon_ir::LogicOp::And => "&&",
                        argon_ir::LogicOp::Or => "||",
                        argon_ir::LogicOp::Nullish => "??",
                    };
                    values.insert(*dest, format!("({} {} {})", lhs, op_str, rhs));
                }
                IrInst::Conditional {
                    cond,
                    then_value,
                    else_value,
                    dest,
                } => {
                    let cond = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let then_value = values
                        .get(then_value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    let else_value = values
                        .get(else_value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    values.insert(
                        *dest,
                        format!("({} ? {} : {})", cond, then_value, else_value),
                    );
                }
                IrInst::VarDecl { kind, name, init } => {
                    self.output.push_str(prefix);
                    self.output.push_str(match kind {
                        argon_ir::VarKind::Var => "var ",
                        argon_ir::VarKind::Let => "let ",
                        argon_ir::VarKind::Const => "const ",
                    });
                    self.output.push_str(name);
                    if let Some(v) = init {
                        let expr = values
                            .get(v)
                            .cloned()
                            .unwrap_or_else(|| "undefined".to_string());
                        self.output.push_str(" = ");
                        self.output.push_str(&expr);
                    }
                    self.output.push_str(";\n");
                }
                IrInst::AssignVar { name, src } => {
                    let expr = values
                        .get(src)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(name);
                    self.output.push_str(" = ");
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                IrInst::ExprStmt { value } => {
                    let expr = values
                        .get(value)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&expr);
                    self.output.push_str(";\n");
                }
                // Legacy/control-flow IR is not emitted by the current builder in IR pipeline.
                _ => {
                    return Err(CodegenError::Unsupported(format!(
                        "unsupported IR instruction: {:?}",
                        inst
                    )));
                }
            }
        }

        Ok(())
    }

    fn emit_ir_loop_body_block_with_prefix(
        &mut self,
        instructions: &[argon_ir::Instruction],
        values: &mut std::collections::HashMap<argon_ir::ValueId, String>,
        prefix: &str,
        loop_label: &str,
        continue_prelude: Option<&[argon_ir::Instruction]>,
    ) -> Result<(), CodegenError> {
        use argon_ir::Instruction as IrInst;

        for inst in instructions {
            match inst {
                IrInst::Break => {
                    self.output.push_str(prefix);
                    self.output.push_str("break ");
                    self.output.push_str(loop_label);
                    self.output.push_str(";\n");
                }
                IrInst::Continue => {
                    if let Some(prelude) = continue_prelude {
                        self.emit_ir_instructions_block_with_prefix(prelude, values, prefix)?;
                    }
                    self.output.push_str(prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(loop_label);
                    self.output.push_str(";\n");
                }
                IrInst::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&format!("if ({}) {{\n", cond_expr));
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        then_body,
                        values,
                        &nested_prefix,
                        loop_label,
                        continue_prelude,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push('}');
                    if !else_body.is_empty() {
                        self.output.push_str(" else {\n");
                        self.emit_ir_loop_body_block_with_prefix(
                            else_body,
                            values,
                            &nested_prefix,
                            loop_label,
                            continue_prelude,
                        )?;
                        self.output.push_str(prefix);
                        self.output.push('}');
                    }
                    self.output.push('\n');
                }
                IrInst::While {
                    cond_instructions,
                    cond,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_block_with_prefix(cond_instructions, values, prefix)?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(") {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::For {
                    init,
                    cond_instructions,
                    cond,
                    update,
                    body,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.emit_ir_instructions_block_with_prefix(init, values, prefix)?;
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("if (!(");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(")) break ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        Some(update),
                    )?;
                    self.emit_ir_instructions_block_with_prefix(update, values, &nested_prefix)?;
                    self.output.push_str(&nested_prefix);
                    self.output.push_str("continue ");
                    self.output.push_str(&nested_label);
                    self.output.push_str(";\n");
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                IrInst::DoWhile {
                    body,
                    cond_instructions,
                    cond,
                } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": do {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.emit_ir_instructions_block_with_prefix(
                        cond_instructions,
                        values,
                        &nested_prefix,
                    )?;
                    let cond_expr = values
                        .get(cond)
                        .cloned()
                        .unwrap_or_else(|| "undefined".to_string());
                    self.output.push_str(prefix);
                    self.output.push_str("} while (");
                    self.output.push_str(&cond_expr);
                    self.output.push_str(");\n");
                }
                IrInst::Loop { body } => {
                    let nested_label = format!("__argon_loop_{}", self.next_temp_id());
                    self.output.push_str(prefix);
                    self.output.push_str(&nested_label);
                    self.output.push_str(": while (true) {\n");
                    let mut nested_prefix = String::new();
                    nested_prefix.push_str(prefix);
                    nested_prefix.push_str("    ");
                    self.emit_ir_loop_body_block_with_prefix(
                        body,
                        values,
                        &nested_prefix,
                        &nested_label,
                        None,
                    )?;
                    self.output.push_str(prefix);
                    self.output.push_str("}\n");
                }
                _ => {
                    self.emit_ir_instructions_block_with_prefix(
                        std::slice::from_ref(inst),
                        values,
                        prefix,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn const_to_js(&self, value: &argon_ir::ConstValue) -> String {
        match value {
            argon_ir::ConstValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            argon_ir::ConstValue::String(s) => s.clone(),
            argon_ir::ConstValue::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            argon_ir::ConstValue::Null => "null".to_string(),
        }
    }

    pub(crate) fn binop_to_js(&self, op: argon_ir::BinOp) -> &'static str {
        match op {
            argon_ir::BinOp::Add => "+",
            argon_ir::BinOp::Sub => "-",
            argon_ir::BinOp::Mul => "*",
            argon_ir::BinOp::Div => "/",
            argon_ir::BinOp::Mod => "%",
            argon_ir::BinOp::Eq => "===",
            argon_ir::BinOp::Ne => "!==",
            argon_ir::BinOp::Lt => "<",
            argon_ir::BinOp::Le => "<=",
            argon_ir::BinOp::Gt => ">",
            argon_ir::BinOp::Ge => ">=",
            argon_ir::BinOp::And => "&",
            argon_ir::BinOp::Or => "|",
            argon_ir::BinOp::Xor => "^",
            argon_ir::BinOp::Shl => "<<",
            argon_ir::BinOp::Shr => ">>",
            argon_ir::BinOp::Sar => ">>",
        }
    }

    pub(crate) fn unop_to_js(&self, op: argon_ir::UnOp) -> &'static str {
        match op {
            argon_ir::UnOp::Neg => "-",
            argon_ir::UnOp::Not => "!",
        }
    }
}
