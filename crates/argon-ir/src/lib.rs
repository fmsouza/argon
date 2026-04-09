//! Argon - Intermediate representation

use argon_ast::*;

pub type BlockId = usize;
pub type ValueId = usize;

pub mod passes;

#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    pub types: Vec<TypeDef>,
    pub globals: Vec<Global>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportStmt>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeId>,
    pub is_async: bool,
    pub body: Vec<BasicBlock>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone, Copy)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Load {
        dest: ValueId,
        src: ValueId,
    },
    Store {
        dest: ValueId,
        src: ValueId,
    },
    ObjectLit {
        dest: ValueId,
        props: Vec<ObjectProp>,
    },
    New {
        callee: ValueId,
        args: Vec<ValueId>,
        dest: ValueId,
    },
    Await {
        arg: ValueId,
        dest: ValueId,
    },
    VarDecl {
        kind: VarKind,
        name: String,
        init: Option<ValueId>,
    },
    AssignVar {
        name: String,
        src: ValueId,
    },
    AssignExpr {
        name: String,
        src: ValueId,
        dest: ValueId,
    },
    ThrowStmt {
        arg: ValueId,
    },
    If {
        cond: ValueId,
        then_body: Vec<Instruction>,
        else_body: Vec<Instruction>,
    },
    While {
        cond_instructions: Vec<Instruction>,
        cond: ValueId,
        body: Vec<Instruction>,
    },
    For {
        init: Vec<Instruction>,
        cond_instructions: Vec<Instruction>,
        cond: ValueId,
        update: Vec<Instruction>,
        body: Vec<Instruction>,
    },
    DoWhile {
        body: Vec<Instruction>,
        cond_instructions: Vec<Instruction>,
        cond: ValueId,
    },
    Loop {
        body: Vec<Instruction>,
    },
    Break,
    Continue,
    Return {
        value: Option<ValueId>,
    },
    Try {
        try_body: Vec<Instruction>,
        catch: Option<TryCatch>,
        finally_body: Option<Vec<Instruction>>,
    },
    ExprStmt {
        value: ValueId,
    },
    VarRef {
        dest: ValueId,
        name: String,
    },
    Member {
        object: ValueId,
        property: String,
        dest: ValueId,
    },
    MemberComputed {
        object: ValueId,
        property: ValueId,
        dest: ValueId,
    },
    BinOp {
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
        dest: ValueId,
    },
    UnOp {
        op: UnOp,
        arg: ValueId,
        dest: ValueId,
    },
    Call {
        callee: ValueId,
        args: Vec<ValueId>,
        dest: ValueId,
    },
    ArrayLit {
        dest: ValueId,
        elements: Vec<Option<ValueId>>,
    },
    LogicalOp {
        op: LogicOp,
        lhs: ValueId,
        rhs: ValueId,
        dest: ValueId,
    },
    Conditional {
        cond: ValueId,
        then_value: ValueId,
        else_value: ValueId,
        dest: ValueId,
    },
    Const {
        dest: ValueId,
        value: ConstValue,
    },
    /// Construct an enum variant value (for async state machines).
    EnumConstruct {
        dest: ValueId,
        enum_name: String,
        variant: String,
        fields: Vec<(String, ValueId)>,
    },
    /// Read a field from the current enum variant.
    EnumField {
        dest: ValueId,
        value: ValueId,
        field: String,
    },
    /// Mutate the state enum in place (change variant + payload).
    EnumMutate {
        target: ValueId,
        variant: String,
        fields: Vec<(String, ValueId)>,
    },
}

#[derive(Debug, Clone)]
pub struct TryCatch {
    pub param: Option<String>,
    pub body: Vec<Instruction>,
}

#[derive(Debug, Clone, Copy)]
pub enum LogicOp {
    And,
    Or,
    Nullish,
}

#[derive(Debug, Clone)]
pub struct ObjectProp {
    // JS-ready key: either an identifier like `x` or a literal like `"x"`.
    pub key: String,
    pub value: ValueId,
}

#[derive(Debug, Clone)]
pub enum ConstValue {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Option<ValueId>),
    Branch {
        cond: ValueId,
        then: BlockId,
        else_: BlockId,
    },
    Jump(BlockId),
    Unreachable,
    /// Match on an enum discriminant, jump to the arm's block.
    /// Used by async state machine poll functions.
    EnumMatch {
        value: ValueId,
        /// (variant_name, field_bindings, target_block)
        arms: Vec<(String, Vec<String>, BlockId)>,
        default: Option<BlockId>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Sar,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct {
        name: String,
        fields: Vec<Field>,
    },
    Array {
        element_type: TypeId,
        length: usize,
    },
    Pointer(TypeId),
    /// Enum type for async state machines. Each variant carries named fields.
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

/// A variant of an enum type, carrying named fields.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<(String, Option<TypeId>)>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub kind: VarKind,
    pub name: String,
    pub ty: TypeId,
    // Straight-line instructions to compute `init` (if present).
    pub init_insts: Vec<Instruction>,
    pub init: Option<ValueId>,
}

pub type TypeId = usize;

pub struct IrBuilder {
    next_value: ValueId,
    next_block: BlockId,
    functions: Vec<Function>,
    types: Vec<TypeDef>,
    globals: Vec<Global>,
    imports: Vec<ImportStmt>,
    exports: Vec<ExportStmt>,
}

impl Default for IrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            functions: Vec::new(),
            types: Vec::new(),
            globals: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    pub fn build(&mut self, source: &SourceFile) -> Result<Module, IrError> {
        // Lower all executable top-level statements into `__argon_init` so that examples like
        // `if (...) { ... }` and `while (...) { ... }` at module scope compile correctly.
        let mut init_stmts: Vec<Stmt> = Vec::new();

        for stmt in &source.statements {
            match stmt {
                Stmt::Function(f) if !f.is_intrinsic => self.translate_function(f, false)?,
                Stmt::AsyncFunction(f) if !f.is_intrinsic => self.translate_function(f, true)?,
                Stmt::Struct(s) if !s.is_intrinsic => self.translate_struct(s)?,
                Stmt::Function(_) | Stmt::AsyncFunction(_) | Stmt::Struct(_) => {
                    // Skip intrinsic declarations - they have no body to lower
                }
                Stmt::Enum(e) => self.translate_enum(e)?,
                Stmt::Variable(v) => {
                    self.translate_global_variable_stmt(v)?;
                }
                Stmt::Import(i) => self.imports.push(i.clone()),
                Stmt::Export(e) => {
                    // IR pipeline treats exports as module metadata; any exported declaration must
                    // also be lowered into IR so codegen can emit it.
                    if let Some(ref decl) = e.declaration {
                        if e.is_type_only {
                            self.translate_erased_export_declaration(decl.as_ref())?;
                            self.exports.push(e.clone());
                            continue;
                        }

                        let exported_syms = self.translate_export_declaration(decl.as_ref())?;

                        // Convert `export <decl>` into `export { name }` so IR codegen has a single
                        // export emission path.
                        if !exported_syms.is_empty() || !e.specifiers.is_empty() {
                            let mut rewritten = e.clone();
                            rewritten.declaration = None;
                            if rewritten.specifiers.is_empty() {
                                rewritten.specifiers = exported_syms
                                    .into_iter()
                                    .map(|sym| ExportSpecifier {
                                        orig: Ident { sym, span: 0..0 },
                                        exported: None,
                                        span: 0..0,
                                    })
                                    .collect();
                            }
                            self.exports.push(rewritten);
                        }
                    } else {
                        self.exports.push(e.clone());
                    }
                }
                Stmt::Interface(_)
                | Stmt::TypeAlias(_)
                | Stmt::Module(_)
                | Stmt::Empty(_)
                | Stmt::Debugger(_) => {}
                _ => init_stmts.push(stmt.clone()),
            }
        }

        if !init_stmts.is_empty() {
            let entry = self.new_block();
            let mut lowerer = FunctionLowerer::new(self, entry);
            let terminated = lowerer.lower_stmt_list(&init_stmts)?;
            if !terminated {
                lowerer.finish_current(Terminator::Return(None))?;
            }
            let body = lowerer.into_blocks();

            self.functions.push(Function {
                id: "__argon_init".to_string(),
                params: Vec::new(),
                return_type: None,
                is_async: false,
                body,
            });
        }

        Ok(Module {
            functions: self.functions.clone(),
            types: self.types.clone(),
            globals: self.globals.clone(),
            imports: self.imports.clone(),
            exports: self.exports.clone(),
        })
    }

    fn translate_export_declaration(&mut self, decl: &Stmt) -> Result<Vec<String>, IrError> {
        match decl {
            Stmt::Function(f) => {
                let name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();
                if name.is_empty() {
                    return Err(IrError::Unsupported(
                        "exported function must have a name".to_string(),
                    ));
                }
                if !f.is_intrinsic {
                    self.translate_function(f, false)?;
                }
                Ok(vec![name])
            }
            Stmt::AsyncFunction(f) => {
                let name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();
                if name.is_empty() {
                    return Err(IrError::Unsupported(
                        "exported function must have a name".to_string(),
                    ));
                }
                if !f.is_intrinsic {
                    self.translate_function(f, true)?;
                }
                Ok(vec![name])
            }
            Stmt::Struct(s) => {
                let name = s.id.sym.clone();
                if s.is_intrinsic {
                    return Ok(vec![name]);
                }
                self.translate_struct(s)?;
                Ok(vec![name])
            }
            Stmt::Variable(v) => {
                let names = self.translate_global_variable_stmt(v)?;
                Ok(names)
            }
            Stmt::Enum(e) => {
                let name = e.id.sym.clone();
                self.translate_enum(e)?;
                Ok(vec![name])
            }
            Stmt::Interface(_) | Stmt::TypeAlias(_) | Stmt::Module(_) => Ok(Vec::new()),
            _ => Err(IrError::Unsupported(format!(
                "unsupported export declaration in IR pipeline: {:?}",
                decl
            ))),
        }
    }

    fn translate_erased_export_declaration(&mut self, decl: &Stmt) -> Result<(), IrError> {
        match decl {
            Stmt::Interface(_) | Stmt::TypeAlias(_) | Stmt::Module(_) | Stmt::Enum(_) => Ok(()),
            _ => Err(IrError::Unsupported(format!(
                "unsupported type-only export declaration in IR pipeline: {:?}",
                decl
            ))),
        }
    }

    fn new_value(&mut self) -> ValueId {
        let v = self.next_value;
        self.next_value += 1;
        v
    }

    fn new_block(&mut self) -> BlockId {
        let b = self.next_block;
        self.next_block += 1;
        b
    }

    fn translate_function(&mut self, f: &FunctionDecl, is_async: bool) -> Result<(), IrError> {
        let func_name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();

        let mut params = Vec::new();
        for p in &f.params {
            if let Pattern::Identifier(id) = &p.pat {
                params.push(Param {
                    name: id.name.sym.clone(),
                    ty: 0,
                });
            }
        }

        let entry = self.new_block();
        let mut lowerer = FunctionLowerer::new(self, entry);
        let terminated = lowerer.lower_stmt_list(&f.body.statements)?;
        if !terminated {
            lowerer.finish_current(Terminator::Return(None))?;
        }
        let body = lowerer.into_blocks();

        self.functions.push(Function {
            id: func_name,
            params,
            return_type: None,
            is_async,
            body,
        });

        Ok(())
    }

    fn translate_expression(
        &mut self,
        expr: &Expr,
        instructions: &mut Vec<Instruction>,
    ) -> Result<ValueId, IrError> {
        match expr {
            Expr::Object(o) => {
                let dest = self.new_value();
                let mut props = Vec::new();
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            let key = match &p.key {
                                Expr::Identifier(id) => id.sym.clone(),
                                Expr::Literal(Literal::String(s)) => s.value.clone(),
                                _ => {
                                    return Err(IrError::Unsupported(format!(
                                        "object literal key: {:?}",
                                        p.key
                                    )))
                                }
                            };

                            let value_expr = match &p.value {
                                ExprOrSpread::Expr(e) => e,
                                ExprOrSpread::Named { value, .. } => value,
                                ExprOrSpread::Spread(_) => {
                                    return Err(IrError::Unsupported(
                                        "object literal spread properties".to_string(),
                                    ))
                                }
                            };
                            let value = self.translate_expression(value_expr, instructions)?;
                            props.push(ObjectProp { key, value });
                        }
                        ObjectProperty::Shorthand(id) => {
                            let value = self.translate_expression(
                                &Expr::Identifier(id.clone()),
                                instructions,
                            )?;
                            props.push(ObjectProp {
                                key: id.sym.clone(),
                                value,
                            });
                        }
                        _ => {
                            return Err(IrError::Unsupported(
                                "unsupported object literal property".to_string(),
                            ))
                        }
                    }
                }
                instructions.push(Instruction::ObjectLit { dest, props });
                Ok(dest)
            }
            Expr::Literal(lit) => {
                let dest = self.new_value();
                let value = match lit {
                    Literal::Number(n) => ConstValue::Number(n.value),
                    Literal::String(s) => ConstValue::String(s.value.clone()),
                    Literal::Boolean(b) => ConstValue::Bool(b.value),
                    Literal::Null(_) => ConstValue::Null,
                    _ => ConstValue::Null,
                };
                instructions.push(Instruction::Const { dest, value });
                Ok(dest)
            }
            Expr::Identifier(id) => {
                let dest = self.new_value();
                instructions.push(Instruction::VarRef {
                    dest,
                    name: id.sym.clone(),
                });
                Ok(dest)
            }
            Expr::Member(m) => {
                let object = self.translate_expression(&m.object, instructions)?;
                let dest = self.new_value();

                if !m.computed {
                    if let Expr::Identifier(prop) = &*m.property {
                        instructions.push(Instruction::Member {
                            object,
                            property: prop.sym.clone(),
                            dest,
                        });
                        return Ok(dest);
                    }
                }

                let property = self.translate_expression(&m.property, instructions)?;
                instructions.push(Instruction::MemberComputed {
                    object,
                    property,
                    dest,
                });
                Ok(dest)
            }
            Expr::Binary(b) => {
                let lhs = self.translate_expression(&b.left, instructions)?;
                let rhs = self.translate_expression(&b.right, instructions)?;
                let dest = self.new_value();
                let op = match b.operator {
                    BinaryOperator::Plus => BinOp::Add,
                    BinaryOperator::Minus => BinOp::Sub,
                    BinaryOperator::Multiply => BinOp::Mul,
                    BinaryOperator::Divide => BinOp::Div,
                    BinaryOperator::Modulo => BinOp::Mod,
                    BinaryOperator::Equal => BinOp::Eq,
                    BinaryOperator::NotEqual => BinOp::Ne,
                    BinaryOperator::LessThan => BinOp::Lt,
                    BinaryOperator::LessThanOrEqual => BinOp::Le,
                    BinaryOperator::GreaterThan => BinOp::Gt,
                    BinaryOperator::GreaterThanOrEqual => BinOp::Ge,
                    BinaryOperator::BitwiseAnd => BinOp::And,
                    BinaryOperator::BitwiseOr => BinOp::Or,
                    BinaryOperator::BitwiseXor => BinOp::Xor,
                    BinaryOperator::LeftShift => BinOp::Shl,
                    BinaryOperator::RightShift | BinaryOperator::UnsignedRightShift => BinOp::Shr,
                    _ => BinOp::Add,
                };
                instructions.push(Instruction::BinOp { op, lhs, rhs, dest });
                Ok(dest)
            }
            Expr::Unary(u) => {
                let arg = self.translate_expression(&u.argument, instructions)?;
                let dest = self.new_value();
                let op = match u.operator {
                    UnaryOperator::Minus => UnOp::Neg,
                    UnaryOperator::LogicalNot => UnOp::Not,
                    _ => UnOp::Neg,
                };
                instructions.push(Instruction::UnOp { op, arg, dest });
                Ok(dest)
            }
            Expr::Call(c) => {
                let mut args = Vec::new();
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => {
                            let a = self.translate_expression(e, instructions)?;
                            args.push(a);
                        }
                        ExprOrSpread::Named { value, .. } => {
                            let a = self.translate_expression(value, instructions)?;
                            args.push(a);
                        }
                        ExprOrSpread::Spread(_) => {}
                    }
                }
                let dest = self.new_value();
                let callee = self.translate_expression(&c.callee, instructions)?;
                instructions.push(Instruction::Call { callee, args, dest });
                Ok(dest)
            }
            Expr::New(n) => {
                let mut args = Vec::new();
                for arg in &n.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => {
                            let a = self.translate_expression(e, instructions)?;
                            args.push(a);
                        }
                        ExprOrSpread::Named { value, .. } => {
                            let a = self.translate_expression(value, instructions)?;
                            args.push(a);
                        }
                        ExprOrSpread::Spread(_) => {
                            return Err(IrError::Unsupported(
                                "new expression spread arguments".to_string(),
                            ));
                        }
                    }
                }
                let dest = self.new_value();
                let callee = self.translate_expression(&n.callee, instructions)?;
                instructions.push(Instruction::New { callee, args, dest });
                Ok(dest)
            }
            Expr::Assignment(a) => {
                match &a.operator {
                    AssignmentOperator::Assign => {}
                    op => {
                        return Err(IrError::Unsupported(format!(
                            "assignment operator: {:?}",
                            op
                        )))
                    }
                }

                let src = self.translate_expression(&a.right, instructions)?;
                let dest = self.new_value();

                match &*a.left {
                    AssignmentTarget::Simple(target) => match &**target {
                        Expr::Identifier(id) => {
                            instructions.push(Instruction::AssignExpr {
                                name: id.sym.clone(),
                                src,
                                dest,
                            });
                            Ok(dest)
                        }
                        _ => Err(IrError::Unsupported(
                            "assignment target (simple)".to_string(),
                        )),
                    },
                    AssignmentTarget::Member(_) => Err(IrError::Unsupported(
                        "assignment target (member)".to_string(),
                    )),
                    AssignmentTarget::Pattern(_) => Err(IrError::Unsupported(
                        "assignment target (pattern)".to_string(),
                    )),
                }
            }
            Expr::Logical(l) => {
                let lhs = self.translate_expression(&l.left, instructions)?;
                let rhs = self.translate_expression(&l.right, instructions)?;
                let dest = self.new_value();
                let op = match &l.operator {
                    LogicalOperator::And => LogicOp::And,
                    LogicalOperator::Or => LogicOp::Or,
                    LogicalOperator::NullishCoalescing => LogicOp::Nullish,
                };
                instructions.push(Instruction::LogicalOp { op, lhs, rhs, dest });
                Ok(dest)
            }
            Expr::Conditional(c) => {
                let cond = self.translate_expression(&c.test, instructions)?;
                let then_value = self.translate_expression(&c.consequent, instructions)?;
                let else_value = self.translate_expression(&c.alternate, instructions)?;
                let dest = self.new_value();
                instructions.push(Instruction::Conditional {
                    cond,
                    then_value,
                    else_value,
                    dest,
                });
                Ok(dest)
            }
            Expr::Array(a) => {
                let dest = self.new_value();
                let mut elements = Vec::new();
                for el in &a.elements {
                    match el {
                        None => elements.push(None),
                        Some(ExprOrSpread::Expr(e)) => {
                            elements.push(Some(self.translate_expression(e, instructions)?));
                        }
                        Some(ExprOrSpread::Named { value, .. }) => {
                            elements.push(Some(self.translate_expression(value, instructions)?));
                        }
                        Some(ExprOrSpread::Spread(_)) => {
                            return Err(IrError::Unsupported(
                                "array literal spread element".to_string(),
                            ));
                        }
                    }
                }
                instructions.push(Instruction::ArrayLit { dest, elements });
                Ok(dest)
            }
            Expr::This(_) => {
                let dest = self.new_value();
                instructions.push(Instruction::VarRef {
                    dest,
                    name: "this".to_string(),
                });
                Ok(dest)
            }
            Expr::Await(a) | Expr::AwaitPromised(a) => {
                let arg = self.translate_expression(&a.argument, instructions)?;
                let dest = self.new_value();
                instructions.push(Instruction::Await { arg, dest });
                Ok(dest)
            }
            Expr::JsxElement(e) => self.translate_jsx_element(e, instructions),
            Expr::JsxFragment(f) => self.translate_jsx_fragment(f, instructions),
            _ => {
                let dest = self.new_value();
                Ok(dest)
            }
        }
    }

    fn translate_variable_stmt(
        &mut self,
        v: &VariableStmt,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), IrError> {
        for decl in &v.declarations {
            if let Pattern::Identifier(id) = &decl.id {
                let init = if let Some(init) = &decl.init {
                    Some(self.translate_expression(init, instructions)?)
                } else {
                    None
                };

                let kind = match v.kind {
                    VariableKind::Var => VarKind::Var,
                    VariableKind::Let => VarKind::Let,
                    VariableKind::Const => VarKind::Const,
                };

                instructions.push(Instruction::VarDecl {
                    kind,
                    name: id.name.sym.clone(),
                    init,
                });
            }
        }
        Ok(())
    }

    fn translate_global_variable_stmt(&mut self, v: &VariableStmt) -> Result<Vec<String>, IrError> {
        let mut names = Vec::new();
        for decl in &v.declarations {
            let (name, init_expr) = match &decl.id {
                Pattern::Identifier(id) => (id.name.sym.clone(), decl.init.as_ref()),
                _ => {
                    return Err(IrError::Unsupported(
                        "module-scope destructuring declarations".to_string(),
                    ));
                }
            };

            let kind = match v.kind {
                VariableKind::Var => VarKind::Var,
                VariableKind::Let => VarKind::Let,
                VariableKind::Const => VarKind::Const,
            };

            let mut init_insts = Vec::new();
            let init = if let Some(expr) = init_expr {
                Some(self.translate_expression(expr, &mut init_insts)?)
            } else {
                None
            };

            self.globals.push(Global {
                kind,
                name: name.clone(),
                ty: 0,
                init_insts,
                init,
            });
            names.push(name);
        }
        Ok(names)
    }

    fn translate_struct(&mut self, s: &StructDecl) -> Result<(), IrError> {
        let fields = s
            .fields
            .iter()
            .map(|f| Field {
                name: f.id.sym.clone(),
                ty: 0,
            })
            .collect();

        self.types.push(TypeDef::Struct {
            name: s.id.sym.clone(),
            fields,
        });

        Ok(())
    }

    fn translate_enum(&mut self, e: &EnumDecl) -> Result<(), IrError> {
        let mut init_insts = Vec::new();
        let mut props = Vec::with_capacity(e.members.len());
        let mut next_numeric = Some(0.0);

        for member in &e.members {
            let value = if let Some(init) = &member.init {
                let value = self.translate_expression(init, &mut init_insts)?;
                next_numeric = match init {
                    Expr::Literal(Literal::Number(n)) => Some(n.value + 1.0),
                    _ => None,
                };
                value
            } else if let Some(current) = next_numeric {
                let dest = self.new_value();
                init_insts.push(Instruction::Const {
                    dest,
                    value: ConstValue::Number(current),
                });
                next_numeric = Some(current + 1.0);
                dest
            } else {
                return Err(IrError::Unsupported(format!(
                    "enum member '{}' requires an explicit initializer after a non-numeric member",
                    member.id.sym
                )));
            };

            props.push(ObjectProp {
                key: member.id.sym.clone(),
                value,
            });
        }

        let dest = self.new_value();
        init_insts.push(Instruction::ObjectLit { dest, props });
        self.globals.push(Global {
            kind: VarKind::Const,
            name: e.id.sym.clone(),
            ty: 0,
            init_insts,
            init: Some(dest),
        });

        Ok(())
    }

    fn translate_jsx_element(
        &mut self,
        elem: &JsxElement,
        instructions: &mut Vec<Instruction>,
    ) -> Result<ValueId, IrError> {
        // React.createElement(tag, props, ...children)
        let react = self.new_value();
        instructions.push(Instruction::VarRef {
            dest: react,
            name: "React".to_string(),
        });
        let create_element = self.new_value();
        instructions.push(Instruction::Member {
            object: react,
            property: "createElement".to_string(),
            dest: create_element,
        });

        let tag = match &elem.opening.name {
            JsxElementName::Identifier(id) => {
                let dest = self.new_value();
                instructions.push(Instruction::Const {
                    dest,
                    value: ConstValue::String(format!("\"{}\"", id.sym)),
                });
                dest
            }
            _ => {
                return Err(IrError::Unsupported(format!(
                    "jsx element name: {:?}",
                    elem.opening.name
                )))
            }
        };

        let props_value = if elem.opening.attributes.is_empty() {
            let dest = self.new_value();
            instructions.push(Instruction::Const {
                dest,
                value: ConstValue::Null,
            });
            dest
        } else {
            let dest = self.new_value();
            let mut props = Vec::new();
            for attr in &elem.opening.attributes {
                let key = match &attr.name {
                    JsxAttributeName::Identifier(id) => id.sym.clone(),
                    JsxAttributeName::Namespaced(ns) => {
                        format!("\"{}:{}\"", ns.namespace.sym, ns.name.sym)
                    }
                };

                let value = if let Some(v) = attr.value.as_ref() {
                    match v {
                        JsxAttributeValue::String(s) => {
                            let id = self.new_value();
                            instructions.push(Instruction::Const {
                                dest: id,
                                value: ConstValue::String(s.value.clone()),
                            });
                            id
                        }
                        JsxAttributeValue::Expression(e) => {
                            self.translate_expression(e, instructions)?
                        }
                        _ => return Err(IrError::Unsupported("jsx attribute value".to_string())),
                    }
                } else {
                    let id = self.new_value();
                    instructions.push(Instruction::Const {
                        dest: id,
                        value: ConstValue::Bool(true),
                    });
                    id
                };

                props.push(ObjectProp { key, value });
            }
            instructions.push(Instruction::ObjectLit { dest, props });
            dest
        };

        let mut args = vec![tag, props_value];
        if elem.children.is_empty() {
            let dest = self.new_value();
            instructions.push(Instruction::Const {
                dest,
                value: ConstValue::Null,
            });
            args.push(dest);
        } else {
            for child in &elem.children {
                match child {
                    JsxChild::Text(t) => {
                        let dest = self.new_value();
                        let escaped = t.value.replace('\\', "\\\\").replace('\"', "\\\"");
                        instructions.push(Instruction::Const {
                            dest,
                            value: ConstValue::String(format!("\"{}\"", escaped)),
                        });
                        args.push(dest);
                    }
                    JsxChild::Expression(e) => {
                        args.push(self.translate_expression(e, instructions)?)
                    }
                    JsxChild::Element(e) => args.push(self.translate_jsx_element(e, instructions)?),
                    JsxChild::Fragment(f) => {
                        args.push(self.translate_jsx_fragment(f, instructions)?)
                    }
                    JsxChild::Spread(_) => {
                        return Err(IrError::Unsupported("jsx spread children".to_string()))
                    }
                }
            }
        }

        let dest = self.new_value();
        instructions.push(Instruction::Call {
            callee: create_element,
            args,
            dest,
        });
        Ok(dest)
    }

    fn translate_jsx_fragment(
        &mut self,
        frag: &JsxFragment,
        instructions: &mut Vec<Instruction>,
    ) -> Result<ValueId, IrError> {
        // React.createElement(React.Fragment, null, ...children)
        let react = self.new_value();
        instructions.push(Instruction::VarRef {
            dest: react,
            name: "React".to_string(),
        });
        let create_element = self.new_value();
        instructions.push(Instruction::Member {
            object: react,
            property: "createElement".to_string(),
            dest: create_element,
        });

        let frag_member = self.new_value();
        instructions.push(Instruction::Member {
            object: react,
            property: "Fragment".to_string(),
            dest: frag_member,
        });

        let null_dest = self.new_value();
        instructions.push(Instruction::Const {
            dest: null_dest,
            value: ConstValue::Null,
        });

        let mut args = vec![frag_member, null_dest];
        for child in &frag.children {
            match child {
                JsxChild::Text(t) => {
                    let dest = self.new_value();
                    let escaped = t.value.replace('\\', "\\\\").replace('\"', "\\\"");
                    instructions.push(Instruction::Const {
                        dest,
                        value: ConstValue::String(format!("\"{}\"", escaped)),
                    });
                    args.push(dest);
                }
                JsxChild::Expression(e) => args.push(self.translate_expression(e, instructions)?),
                JsxChild::Element(e) => args.push(self.translate_jsx_element(e, instructions)?),
                JsxChild::Fragment(f) => args.push(self.translate_jsx_fragment(f, instructions)?),
                JsxChild::Spread(_) => {
                    return Err(IrError::Unsupported("jsx spread children".to_string()))
                }
            }
        }

        let dest = self.new_value();
        instructions.push(Instruction::Call {
            callee: create_element,
            args,
            dest,
        });
        Ok(dest)
    }
}

#[derive(Clone, Copy)]
struct BreakableContext {
    break_target: BlockId,
    // Only loops provide a continue target.
    continue_target: Option<BlockId>,
}

struct FunctionLowerer<'a> {
    builder: &'a mut IrBuilder,
    blocks: Vec<BasicBlock>,
    current_id: BlockId,
    current_instructions: Vec<Instruction>,
    breakable_stack: Vec<BreakableContext>,
}

impl<'a> FunctionLowerer<'a> {
    fn new(builder: &'a mut IrBuilder, entry: BlockId) -> Self {
        Self {
            builder,
            blocks: Vec::new(),
            current_id: entry,
            current_instructions: Vec::new(),
            breakable_stack: Vec::new(),
        }
    }

    fn into_blocks(self) -> Vec<BasicBlock> {
        self.blocks
    }

    fn finish_current(&mut self, terminator: Terminator) -> Result<(), IrError> {
        let block = BasicBlock {
            id: self.current_id,
            instructions: std::mem::take(&mut self.current_instructions),
            terminator,
        };
        self.blocks.push(block);
        Ok(())
    }

    fn start_block(&mut self, id: BlockId) -> Result<(), IrError> {
        self.current_id = id;
        self.current_instructions.clear();
        Ok(())
    }

    fn lower_stmt_list(&mut self, stmts: &[Stmt]) -> Result<bool, IrError> {
        for stmt in stmts {
            if self.lower_stmt(stmt)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Result<bool, IrError> {
        match stmt {
            Stmt::Variable(v) => {
                self.builder
                    .translate_variable_stmt(v, &mut self.current_instructions)?;
                Ok(false)
            }
            Stmt::Expr(e) => {
                let value = self
                    .builder
                    .translate_expression(&e.expr, &mut self.current_instructions)?;
                self.current_instructions
                    .push(Instruction::ExprStmt { value });
                Ok(false)
            }
            Stmt::Return(r) => {
                let value = if let Some(arg) = &r.argument {
                    Some(
                        self.builder
                            .translate_expression(arg, &mut self.current_instructions)?,
                    )
                } else {
                    None
                };
                self.finish_current(Terminator::Return(value))?;
                Ok(true)
            }
            Stmt::Block(b) => self.lower_stmt_list(&b.statements),
            Stmt::If(i) => self.lower_if(i),
            Stmt::While(w) => self.lower_while(w),
            Stmt::Loop(l) => self.lower_loop(l),
            Stmt::For(f) => self.lower_for(f),
            Stmt::ForIn(fi) => self.lower_for_in(fi),
            Stmt::DoWhile(d) => self.lower_do_while(d),
            Stmt::Switch(s) => self.lower_switch(s),
            Stmt::Match(m) => self.lower_match(m),
            Stmt::Break(_) => self.lower_break(),
            Stmt::Continue(_) => self.lower_continue(),
            Stmt::Empty(_) => Ok(false),
            _ => Err(IrError::Unsupported(format!("statement: {:?}", stmt))),
        }
    }

    fn lower_break(&mut self) -> Result<bool, IrError> {
        let ctx = self
            .breakable_stack
            .last()
            .copied()
            .ok_or_else(|| IrError::InvalidAst("break outside of loop/switch".to_string()))?;
        self.finish_current(Terminator::Jump(ctx.break_target))?;
        Ok(true)
    }

    fn lower_continue(&mut self) -> Result<bool, IrError> {
        // Continue targets the nearest loop, skipping intervening switch contexts.
        let target = self
            .breakable_stack
            .iter()
            .rev()
            .find_map(|c| c.continue_target)
            .ok_or_else(|| IrError::InvalidAst("continue outside of loop".to_string()))?;
        self.finish_current(Terminator::Jump(target))?;
        Ok(true)
    }

    fn lower_if(&mut self, i: &IfStmt) -> Result<bool, IrError> {
        let cond = self
            .builder
            .translate_expression(&i.condition, &mut self.current_instructions)?;

        let then_id = self.builder.new_block();
        let else_id = self.builder.new_block();
        let join_id = self.builder.new_block();

        self.finish_current(Terminator::Branch {
            cond,
            then: then_id,
            else_: else_id,
        })?;

        self.start_block(then_id)?;
        let then_terminated = self.lower_stmt(&i.consequent)?;
        if !then_terminated {
            self.finish_current(Terminator::Jump(join_id))?;
        }

        self.start_block(else_id)?;
        let else_terminated = if let Some(alt) = &i.alternate {
            self.lower_stmt(alt)?
        } else {
            false
        };
        if !else_terminated {
            self.finish_current(Terminator::Jump(join_id))?;
        }

        self.start_block(join_id)?;
        Ok(false)
    }

    fn lower_while(&mut self, w: &WhileStmt) -> Result<bool, IrError> {
        let cond_id = self.builder.new_block();
        let body_id = self.builder.new_block();
        let after_id = self.builder.new_block();

        self.finish_current(Terminator::Jump(cond_id))?;

        self.start_block(cond_id)?;
        let cond = self
            .builder
            .translate_expression(&w.condition, &mut self.current_instructions)?;
        self.finish_current(Terminator::Branch {
            cond,
            then: body_id,
            else_: after_id,
        })?;

        self.start_block(body_id)?;
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: Some(cond_id),
        });
        let body_terminated = self.lower_stmt(&w.body)?;
        self.breakable_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(cond_id))?;
        }

        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_loop(&mut self, l: &LoopStmt) -> Result<bool, IrError> {
        let body_id = self.builder.new_block();
        let after_id = self.builder.new_block();

        self.finish_current(Terminator::Jump(body_id))?;

        self.start_block(body_id)?;
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: Some(body_id),
        });
        let body_terminated = self.lower_stmt(&l.body)?;
        self.breakable_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(body_id))?;
        }

        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_for(&mut self, f: &ForStmt) -> Result<bool, IrError> {
        if let Some(init) = &f.init {
            match init {
                ForInit::Variable(v) => self
                    .builder
                    .translate_variable_stmt(v, &mut self.current_instructions)?,
                ForInit::Expr(e) => {
                    let v = self
                        .builder
                        .translate_expression(e, &mut self.current_instructions)?;
                    self.current_instructions
                        .push(Instruction::ExprStmt { value: v });
                }
            }
        }

        let cond_id = self.builder.new_block();
        let body_id = self.builder.new_block();
        let update_id = self.builder.new_block();
        let after_id = self.builder.new_block();

        self.finish_current(Terminator::Jump(cond_id))?;

        self.start_block(cond_id)?;
        let cond_val = if let Some(test) = &f.test {
            self.builder
                .translate_expression(test, &mut self.current_instructions)?
        } else {
            let dest = self.builder.new_value();
            self.current_instructions.push(Instruction::Const {
                dest,
                value: ConstValue::Bool(true),
            });
            dest
        };
        self.finish_current(Terminator::Branch {
            cond: cond_val,
            then: body_id,
            else_: after_id,
        })?;

        self.start_block(body_id)?;
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: Some(update_id),
        });
        let body_terminated = self.lower_stmt(&f.body)?;
        self.breakable_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(update_id))?;
        }

        self.start_block(update_id)?;
        if let Some(update) = &f.update {
            let v = self
                .builder
                .translate_expression(update, &mut self.current_instructions)?;
            self.current_instructions
                .push(Instruction::ExprStmt { value: v });
        }
        self.finish_current(Terminator::Jump(cond_id))?;

        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_for_in(&mut self, f: &ForInStmt) -> Result<bool, IrError> {
        let bound_name = match &f.left {
            ForInLeft::Pattern(Pattern::Identifier(id)) => id.name.sym.clone(),
            ForInLeft::Variable(v) => match &v.id {
                Pattern::Identifier(id) => id.name.sym.clone(),
                _ => return Err(IrError::Unsupported("for..of left pattern".to_string())),
            },
            _ => return Err(IrError::Unsupported("for..of left pattern".to_string())),
        };

        let iter_name = format!("__argon_forin_iter_{}", self.builder.new_value());
        let idx_name = format!("__argon_forin_idx_{}", self.builder.new_value());

        let iter_value = self
            .builder
            .translate_expression(&f.right, &mut self.current_instructions)?;
        self.current_instructions.push(Instruction::VarDecl {
            kind: VarKind::Const,
            name: iter_name.clone(),
            init: Some(iter_value),
        });

        let zero = self.builder.new_value();
        self.current_instructions.push(Instruction::Const {
            dest: zero,
            value: ConstValue::Number(0.0),
        });
        self.current_instructions.push(Instruction::VarDecl {
            kind: VarKind::Let,
            name: idx_name.clone(),
            init: Some(zero),
        });

        let cond_id = self.builder.new_block();
        let body_id = self.builder.new_block();
        let update_id = self.builder.new_block();
        let after_id = self.builder.new_block();

        self.finish_current(Terminator::Jump(cond_id))?;

        self.start_block(cond_id)?;
        let idx_val = self.builder.new_value();
        self.current_instructions.push(Instruction::VarRef {
            dest: idx_val,
            name: idx_name.clone(),
        });
        let iter_ref = self.builder.new_value();
        self.current_instructions.push(Instruction::VarRef {
            dest: iter_ref,
            name: iter_name.clone(),
        });
        let len_val = self.builder.new_value();
        self.current_instructions.push(Instruction::Member {
            object: iter_ref,
            property: "length".to_string(),
            dest: len_val,
        });
        let cond_val = self.builder.new_value();
        self.current_instructions.push(Instruction::BinOp {
            op: BinOp::Lt,
            lhs: idx_val,
            rhs: len_val,
            dest: cond_val,
        });
        self.finish_current(Terminator::Branch {
            cond: cond_val,
            then: body_id,
            else_: after_id,
        })?;

        self.start_block(body_id)?;
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: Some(update_id),
        });

        let iter_obj = self.builder.new_value();
        self.current_instructions.push(Instruction::VarRef {
            dest: iter_obj,
            name: iter_name.clone(),
        });
        let idx_obj = self.builder.new_value();
        self.current_instructions.push(Instruction::VarRef {
            dest: idx_obj,
            name: idx_name.clone(),
        });
        let element = self.builder.new_value();
        self.current_instructions.push(Instruction::MemberComputed {
            object: iter_obj,
            property: idx_obj,
            dest: element,
        });
        self.current_instructions.push(Instruction::VarDecl {
            kind: VarKind::Let,
            name: bound_name,
            init: Some(element),
        });

        let body_terminated = self.lower_stmt(&f.body)?;
        self.breakable_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(update_id))?;
        }

        self.start_block(update_id)?;
        let cur_idx = self.builder.new_value();
        self.current_instructions.push(Instruction::VarRef {
            dest: cur_idx,
            name: idx_name.clone(),
        });
        let one = self.builder.new_value();
        self.current_instructions.push(Instruction::Const {
            dest: one,
            value: ConstValue::Number(1.0),
        });
        let next_idx = self.builder.new_value();
        self.current_instructions.push(Instruction::BinOp {
            op: BinOp::Add,
            lhs: cur_idx,
            rhs: one,
            dest: next_idx,
        });
        self.current_instructions.push(Instruction::AssignVar {
            name: idx_name,
            src: next_idx,
        });
        self.finish_current(Terminator::Jump(cond_id))?;

        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_do_while(&mut self, d: &DoWhileStmt) -> Result<bool, IrError> {
        let body_id = self.builder.new_block();
        let cond_id = self.builder.new_block();
        let after_id = self.builder.new_block();

        self.finish_current(Terminator::Jump(body_id))?;

        self.start_block(body_id)?;
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: Some(cond_id),
        });
        let body_terminated = self.lower_stmt(&d.body)?;
        self.breakable_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(cond_id))?;
        }

        self.start_block(cond_id)?;
        let cond = self
            .builder
            .translate_expression(&d.condition, &mut self.current_instructions)?;
        self.finish_current(Terminator::Branch {
            cond,
            then: body_id,
            else_: after_id,
        })?;

        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_switch(&mut self, s: &SwitchStmt) -> Result<bool, IrError> {
        // Lower `switch (discriminant) { case ... }` into a chain of comparisons and case blocks.
        //
        // Note: currently re-translates the discriminant in each check block to avoid cross-block
        // ValueId dependencies in the non-SSA IR.
        let after_id = self.builder.new_block();
        self.breakable_stack.push(BreakableContext {
            break_target: after_id,
            continue_target: None,
        });

        let case_block_ids: Vec<BlockId> = (0..s.cases.len())
            .map(|_| self.builder.new_block())
            .collect();
        let default_case = s.cases.iter().position(|c| c.test.is_none());
        let non_default: Vec<usize> = s
            .cases
            .iter()
            .enumerate()
            .filter_map(|(i, c)| if c.test.is_some() { Some(i) } else { None })
            .collect();

        if non_default.is_empty() {
            // No tests; jump straight to default or after.
            let target = default_case.map(|i| case_block_ids[i]).unwrap_or(after_id);
            self.finish_current(Terminator::Jump(target))?;
        } else {
            // Build check blocks.
            let check_ids: Vec<BlockId> = (0..non_default.len())
                .map(|_| self.builder.new_block())
                .collect();
            self.finish_current(Terminator::Jump(check_ids[0]))?;

            for (j, check_id) in check_ids.iter().enumerate() {
                self.start_block(*check_id)?;
                let discr = self
                    .builder
                    .translate_expression(&s.discriminant, &mut self.current_instructions)?;
                let case_idx = non_default[j];
                let test_expr = s.cases[case_idx]
                    .test
                    .as_ref()
                    .expect("non-default case has test");
                let test_val = self
                    .builder
                    .translate_expression(test_expr, &mut self.current_instructions)?;
                let cond = self.builder.new_value();
                self.current_instructions.push(Instruction::BinOp {
                    op: BinOp::Eq,
                    lhs: discr,
                    rhs: test_val,
                    dest: cond,
                });

                let else_target = if j + 1 < check_ids.len() {
                    check_ids[j + 1]
                } else {
                    default_case.map(|i| case_block_ids[i]).unwrap_or(after_id)
                };
                self.finish_current(Terminator::Branch {
                    cond,
                    then: case_block_ids[case_idx],
                    else_: else_target,
                })?;
            }
        }

        // Lower case bodies with fallthrough semantics.
        for (i, case) in s.cases.iter().enumerate() {
            self.start_block(case_block_ids[i])?;
            let terminated = self.lower_stmt_list(&case.consequent)?;
            if !terminated {
                let next = if i + 1 < case_block_ids.len() {
                    case_block_ids[i + 1]
                } else {
                    after_id
                };
                self.finish_current(Terminator::Jump(next))?;
            }
        }

        // Continue after the switch.
        self.breakable_stack.pop();
        self.start_block(after_id)?;
        Ok(false)
    }

    fn lower_match(&mut self, m: &MatchStmt) -> Result<bool, IrError> {
        // Lower `match (x) { pat => stmt, ... }` into a chain of comparisons and arms.
        //
        // Note: currently re-translates the discriminant in each check block to avoid cross-block
        // ValueId dependencies in the non-SSA IR.
        let after_id = self.builder.new_block();

        let arm_ids: Vec<BlockId> = (0..m.cases.len())
            .map(|_| self.builder.new_block())
            .collect();
        let check_ids: Vec<BlockId> = (0..m.cases.len())
            .map(|_| self.builder.new_block())
            .collect();

        if m.cases.is_empty() {
            self.finish_current(Terminator::Jump(after_id))?;
            self.start_block(after_id)?;
            return Ok(false);
        }

        self.finish_current(Terminator::Jump(check_ids[0]))?;

        for (i, check_id) in check_ids.iter().enumerate() {
            self.start_block(*check_id)?;
            let discr = self
                .builder
                .translate_expression(&m.discriminant, &mut self.current_instructions)?;
            let cond = match &m.cases[i].pattern {
                MatchPattern::Expr(pattern) => {
                    let pat = self
                        .builder
                        .translate_expression(pattern, &mut self.current_instructions)?;
                    let cond = self.builder.new_value();
                    self.current_instructions.push(Instruction::BinOp {
                        op: BinOp::Eq,
                        lhs: discr,
                        rhs: pat,
                        dest: cond,
                    });
                    cond
                }
                MatchPattern::Result(pattern) => {
                    let cond = self.builder.new_value();
                    self.current_instructions.push(Instruction::Member {
                        object: discr,
                        property: match pattern.kind {
                            ResultPatternKind::Ok => "isOk".to_string(),
                            ResultPatternKind::Err => "isErr".to_string(),
                        },
                        dest: cond,
                    });
                    cond
                }
            };

            let else_target = if i + 1 < check_ids.len() {
                check_ids[i + 1]
            } else {
                after_id
            };

            self.finish_current(Terminator::Branch {
                cond,
                then: arm_ids[i],
                else_: else_target,
            })?;
        }

        for (i, case) in m.cases.iter().enumerate() {
            self.start_block(arm_ids[i])?;
            if let MatchPattern::Result(pattern) = &case.pattern {
                let discr = self
                    .builder
                    .translate_expression(&m.discriminant, &mut self.current_instructions)?;
                let payload = self.builder.new_value();
                self.current_instructions.push(Instruction::Member {
                    object: discr,
                    property: match pattern.kind {
                        ResultPatternKind::Ok => "value".to_string(),
                        ResultPatternKind::Err => "error".to_string(),
                    },
                    dest: payload,
                });
                self.current_instructions.push(Instruction::VarDecl {
                    kind: VarKind::Const,
                    name: pattern.binding.sym.clone(),
                    init: Some(payload),
                });
            }
            let terminated = self.lower_stmt(&case.consequent)?;
            if !terminated {
                self.finish_current(Terminator::Jump(after_id))?;
            }
        }

        self.start_block(after_id)?;
        Ok(false)
    }
}

#[derive(Debug)]
pub enum IrError {
    InvalidAst(String),
    Unsupported(String),
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrError::InvalidAst(msg) => write!(f, "Invalid AST: {}", msg),
            IrError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

impl std::error::Error for IrError {}

#[cfg(test)]
mod ir_builder_tests;
#[cfg(test)]
mod passes_tests;
