//! CFG Flattening Pass
//!
//! Converts nested control flow (If, While, For, Loop, DoWhile, Try) into
//! a flat basic block CFG with explicit Branch/Jump terminators.
//! Required before async lowering, which needs to split at Await points.

use crate::*;

/// Flatten a function's nested IR into a proper CFG.
pub fn flatten_cfg(func: &Function) -> Function {
    let mut flattener = CfgFlattener::new(func);
    flattener.flatten_function(func)
}

struct CfgFlattener {
    blocks: Vec<BasicBlock>,
    next_block_id: BlockId,
    next_value_id: ValueId,
    /// Current block's instructions being accumulated.
    current_insts: Vec<Instruction>,
}

impl CfgFlattener {
    fn new(func: &Function) -> Self {
        let max_val = Self::max_value_id(func);
        let max_block = func.body.iter().map(|b| b.id).max().unwrap_or(0);
        Self {
            blocks: Vec::new(),
            next_block_id: max_block + 100,
            next_value_id: max_val + 1,
            current_insts: Vec::new(),
        }
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }

    #[allow(dead_code)]
    fn alloc_value(&mut self) -> ValueId {
        let id = self.next_value_id;
        self.next_value_id += 1;
        id
    }

    fn flatten_function(&mut self, func: &Function) -> Function {
        // Process all original blocks. Typically there's just one block
        // with nested instructions.
        for block in &func.body {
            for inst in &block.instructions {
                self.flatten_instruction(inst);
            }
            // Close the current block with the original terminator
            self.close_block(block.terminator.clone());
        }

        // If there are leftover instructions (shouldn't happen normally), close them
        if !self.current_insts.is_empty() {
            self.close_block(Terminator::Return(None));
        }

        // Re-number blocks sequentially from 0
        let mut body = std::mem::take(&mut self.blocks);
        for (i, block) in body.iter_mut().enumerate() {
            block.id = i;
        }

        Function {
            id: func.id.clone(),
            params: func.params.clone(),
            return_type: func.return_type,
            is_async: func.is_async,
            body,
        }
    }

    /// Close the current accumulation buffer as a block with the given terminator.
    fn close_block(&mut self, term: Terminator) {
        let id = self.blocks.len(); // sequential
        self.blocks.push(BasicBlock {
            id,
            instructions: std::mem::take(&mut self.current_insts),
            terminator: term,
        });
    }

    fn flatten_instruction(&mut self, inst: &Instruction) {
        match inst {
            Instruction::If {
                cond,
                then_body,
                else_body,
            } => {
                let _then_id = self.blocks.len() + 1;
                let _else_id = _then_id + 1;

                // We don't know exact IDs yet since then/else may generate more blocks.
                // Use placeholder IDs that we'll fix via renumbering.
                let then_placeholder = self.alloc_block();
                let else_placeholder = self.alloc_block();
                let merge_placeholder = self.alloc_block();

                // Close current block with branch
                self.close_block(Terminator::Branch {
                    cond: *cond,
                    then: then_placeholder,
                    else_: else_placeholder,
                });

                // Then block
                let then_start = self.blocks.len();
                for i in then_body {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(merge_placeholder));

                // Else block
                let else_start = self.blocks.len();
                if else_body.is_empty() {
                    // Empty else — just jump to merge
                    self.close_block(Terminator::Jump(merge_placeholder));
                } else {
                    for i in else_body {
                        self.flatten_instruction(i);
                    }
                    self.close_block(Terminator::Jump(merge_placeholder));
                }

                // Merge block (empty, subsequent instructions go here)
                let merge_start = self.blocks.len();

                // Fix up the placeholder IDs
                self.fixup_block_id(then_placeholder, then_start);
                self.fixup_block_id(else_placeholder, else_start);
                self.fixup_block_id(merge_placeholder, merge_start);

                // The merge block doesn't exist yet as a BasicBlock —
                // subsequent instructions will accumulate in current_insts
                // and be closed later.
            }

            Instruction::While {
                cond_instructions,
                cond,
                body,
            } => {
                let header_placeholder = self.alloc_block();
                let body_placeholder = self.alloc_block();
                let exit_placeholder = self.alloc_block();

                // Close current block → jump to header
                self.close_block(Terminator::Jump(header_placeholder));

                // Header: evaluate condition
                let header_start = self.blocks.len();
                for i in cond_instructions {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Branch {
                    cond: *cond,
                    then: body_placeholder,
                    else_: exit_placeholder,
                });

                // Body
                let body_start = self.blocks.len();
                for i in body {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(header_placeholder));

                let exit_start = self.blocks.len();

                self.fixup_block_id(header_placeholder, header_start);
                self.fixup_block_id(body_placeholder, body_start);
                self.fixup_block_id(exit_placeholder, exit_start);
            }

            Instruction::For {
                init,
                cond_instructions,
                cond,
                update,
                body,
            } => {
                let header_placeholder = self.alloc_block();
                let body_placeholder = self.alloc_block();
                let update_placeholder = self.alloc_block();
                let exit_placeholder = self.alloc_block();

                // Init
                for i in init {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(header_placeholder));

                // Header
                let header_start = self.blocks.len();
                for i in cond_instructions {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Branch {
                    cond: *cond,
                    then: body_placeholder,
                    else_: exit_placeholder,
                });

                // Body
                let body_start = self.blocks.len();
                for i in body {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(update_placeholder));

                // Update
                let update_start = self.blocks.len();
                for i in update {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(header_placeholder));

                let exit_start = self.blocks.len();

                self.fixup_block_id(header_placeholder, header_start);
                self.fixup_block_id(body_placeholder, body_start);
                self.fixup_block_id(update_placeholder, update_start);
                self.fixup_block_id(exit_placeholder, exit_start);
            }

            Instruction::DoWhile {
                body,
                cond_instructions,
                cond,
            } => {
                let body_placeholder = self.alloc_block();
                let cond_placeholder = self.alloc_block();
                let exit_placeholder = self.alloc_block();

                self.close_block(Terminator::Jump(body_placeholder));

                let body_start = self.blocks.len();
                for i in body {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Jump(cond_placeholder));

                let cond_start = self.blocks.len();
                for i in cond_instructions {
                    self.flatten_instruction(i);
                }
                self.close_block(Terminator::Branch {
                    cond: *cond,
                    then: body_placeholder,
                    else_: exit_placeholder,
                });

                let exit_start = self.blocks.len();

                self.fixup_block_id(body_placeholder, body_start);
                self.fixup_block_id(cond_placeholder, cond_start);
                self.fixup_block_id(exit_placeholder, exit_start);
            }

            Instruction::Loop { body } => {
                let loop_placeholder = self.alloc_block();
                let exit_placeholder = self.alloc_block();

                self.close_block(Terminator::Jump(loop_placeholder));

                let loop_start = self.blocks.len();
                for i in body {
                    self.flatten_instruction(i);
                }
                // Infinite loop — jump back to loop header
                self.close_block(Terminator::Jump(loop_placeholder));

                let exit_start = self.blocks.len();

                self.fixup_block_id(loop_placeholder, loop_start);
                self.fixup_block_id(exit_placeholder, exit_start);
            }

            Instruction::Try {
                try_body,
                catch,
                finally_body,
            } => {
                // For CFG flattening, try/catch is complex.
                // For now, keep the Try instruction as-is (it's handled by the
                // codegen backends directly). The async lowering pass will need
                // special handling for await inside try/catch.
                self.current_insts.push(Instruction::Try {
                    try_body: try_body.clone(),
                    catch: catch.clone(),
                    finally_body: finally_body.clone(),
                });
            }

            // Non-control-flow instructions pass through unchanged
            _ => {
                self.current_insts.push(inst.clone());
            }
        }
    }

    /// Fix all references to `placeholder_id` in block terminators to `real_id`.
    fn fixup_block_id(&mut self, placeholder: BlockId, real: BlockId) {
        for block in &mut self.blocks {
            match &mut block.terminator {
                Terminator::Jump(t) => {
                    if *t == placeholder {
                        *t = real;
                    }
                }
                Terminator::Branch { then, else_, .. } => {
                    if *then == placeholder {
                        *then = real;
                    }
                    if *else_ == placeholder {
                        *else_ = real;
                    }
                }
                Terminator::EnumMatch { arms, default, .. } => {
                    for (_, _, target) in arms {
                        if *target == placeholder {
                            *target = real;
                        }
                    }
                    if let Some(d) = default {
                        if *d == placeholder {
                            *d = real;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn max_value_id(func: &Function) -> ValueId {
        let mut max = 0usize;
        for block in &func.body {
            for inst in &block.instructions {
                Self::collect_max_value_id(inst, &mut max);
            }
        }
        max
    }

    fn collect_max_value_id(inst: &Instruction, max: &mut ValueId) {
        if let Some(d) = Self::inst_dest(inst) {
            *max = (*max).max(d);
        }
        // Recurse into nested instructions
        match inst {
            Instruction::If {
                then_body,
                else_body,
                ..
            } => {
                for i in then_body {
                    Self::collect_max_value_id(i, max);
                }
                for i in else_body {
                    Self::collect_max_value_id(i, max);
                }
            }
            Instruction::While {
                cond_instructions,
                body,
                ..
            } => {
                for i in cond_instructions {
                    Self::collect_max_value_id(i, max);
                }
                for i in body {
                    Self::collect_max_value_id(i, max);
                }
            }
            Instruction::For {
                init,
                cond_instructions,
                update,
                body,
                ..
            } => {
                for i in init {
                    Self::collect_max_value_id(i, max);
                }
                for i in cond_instructions {
                    Self::collect_max_value_id(i, max);
                }
                for i in update {
                    Self::collect_max_value_id(i, max);
                }
                for i in body {
                    Self::collect_max_value_id(i, max);
                }
            }
            Instruction::DoWhile {
                body,
                cond_instructions,
                ..
            } => {
                for i in body {
                    Self::collect_max_value_id(i, max);
                }
                for i in cond_instructions {
                    Self::collect_max_value_id(i, max);
                }
            }
            Instruction::Loop { body } => {
                for i in body {
                    Self::collect_max_value_id(i, max);
                }
            }
            Instruction::Try {
                try_body,
                catch,
                finally_body,
            } => {
                for i in try_body {
                    Self::collect_max_value_id(i, max);
                }
                if let Some(c) = catch {
                    for i in &c.body {
                        Self::collect_max_value_id(i, max);
                    }
                }
                if let Some(f) = finally_body {
                    for i in f {
                        Self::collect_max_value_id(i, max);
                    }
                }
            }
            _ => {}
        }
    }

    fn inst_dest(inst: &Instruction) -> Option<ValueId> {
        match inst {
            Instruction::Const { dest, .. }
            | Instruction::VarRef { dest, .. }
            | Instruction::Member { dest, .. }
            | Instruction::MemberComputed { dest, .. }
            | Instruction::BinOp { dest, .. }
            | Instruction::UnOp { dest, .. }
            | Instruction::Call { dest, .. }
            | Instruction::New { dest, .. }
            | Instruction::Await { dest, .. }
            | Instruction::ObjectLit { dest, .. }
            | Instruction::ArrayLit { dest, .. }
            | Instruction::LogicalOp { dest, .. }
            | Instruction::Conditional { dest, .. }
            | Instruction::Load { dest, .. }
            | Instruction::AssignExpr { dest, .. }
            | Instruction::EnumConstruct { dest, .. }
            | Instruction::EnumField { dest, .. } => Some(*dest),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_if_else() {
        let func = Function {
            id: "test".to_string(),
            params: vec![],
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::Const {
                        dest: 0,
                        value: ConstValue::Bool(true),
                    },
                    Instruction::If {
                        cond: 0,
                        then_body: vec![
                            Instruction::Const {
                                dest: 1,
                                value: ConstValue::Number(1.0),
                            },
                            Instruction::ExprStmt { value: 1 },
                        ],
                        else_body: vec![
                            Instruction::Const {
                                dest: 2,
                                value: ConstValue::Number(2.0),
                            },
                            Instruction::ExprStmt { value: 2 },
                        ],
                    },
                ],
                terminator: Terminator::Return(None),
            }],
        };

        let flattened = flatten_cfg(&func);
        // entry (branch) + then (jump to merge) + else (jump to merge) + merge (return)
        assert!(
            flattened.body.len() >= 3,
            "expected at least 3 blocks, got {}",
            flattened.body.len()
        );
        // Entry block should end with Branch
        assert!(
            matches!(flattened.body[0].terminator, Terminator::Branch { .. }),
            "entry block should end with Branch, got {:?}",
            flattened.body[0].terminator
        );
    }

    #[test]
    fn flattens_while_loop() {
        let func = Function {
            id: "test_while".to_string(),
            params: vec![],
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![Instruction::While {
                    cond_instructions: vec![Instruction::Const {
                        dest: 0,
                        value: ConstValue::Bool(true),
                    }],
                    cond: 0,
                    body: vec![
                        Instruction::Const {
                            dest: 1,
                            value: ConstValue::Number(1.0),
                        },
                        Instruction::ExprStmt { value: 1 },
                    ],
                }],
                terminator: Terminator::Return(None),
            }],
        };

        let flattened = flatten_cfg(&func);
        // entry (jump to header) + header (branch) + body (jump to header) + exit (return)
        assert!(
            flattened.body.len() >= 3,
            "expected at least 3 blocks, got {}",
            flattened.body.len()
        );
    }

    #[test]
    fn preserves_non_control_flow() {
        let func = Function {
            id: "simple".to_string(),
            params: vec![],
            return_type: None,
            is_async: false,
            body: vec![BasicBlock {
                id: 0,
                instructions: vec![
                    Instruction::Const {
                        dest: 0,
                        value: ConstValue::Number(42.0),
                    },
                    Instruction::ExprStmt { value: 0 },
                ],
                terminator: Terminator::Return(Some(0)),
            }],
        };

        let flattened = flatten_cfg(&func);
        assert_eq!(flattened.body.len(), 1);
        assert_eq!(flattened.body[0].instructions.len(), 2);
    }
}
