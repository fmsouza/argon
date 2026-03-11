//! Argon - Intermediate representation

use argon_ast::*;

pub type BlockId = usize;
pub type ValueId = usize;

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
    Const {
        dest: ValueId,
        value: ConstValue,
    },
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
    Struct { name: String, fields: Vec<Field> },
    Array { element_type: TypeId, length: usize },
    Pointer(TypeId),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub name: String,
    pub ty: TypeId,
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
                Stmt::Function(f) => self.translate_function(f, false)?,
                Stmt::AsyncFunction(f) => self.translate_function(f, true)?,
                Stmt::Struct(s) => self.translate_struct(s)?,
                Stmt::Class(c) => self.translate_class(c)?,
                Stmt::Import(i) => self.imports.push(i.clone()),
                Stmt::Export(e) => self.exports.push(e.clone()),
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
                                _ => {
                                    return Err(IrError::Unsupported(
                                        "object literal spread properties".to_string(),
                                    ))
                                }
                            };
                            let value = self.translate_expression(value_expr, instructions)?;
                            props.push(ObjectProp { key, value });
                        }
                        ObjectProperty::Shorthand(id) => {
                            let value = self.translate_expression(&Expr::Identifier(id.clone()), instructions)?;
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
                    if let ExprOrSpread::Expr(e) = arg {
                        let a = self.translate_expression(e, instructions)?;
                        args.push(a);
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
                    if let ExprOrSpread::Expr(e) = arg {
                        let a = self.translate_expression(e, instructions)?;
                        args.push(a);
                    } else {
                        return Err(IrError::Unsupported(
                            "new expression spread arguments".to_string(),
                        ));
                    }
                }
                let dest = self.new_value();
                let callee = self.translate_expression(&n.callee, instructions)?;
                instructions.push(Instruction::New { callee, args, dest });
                Ok(dest)
            }
            Expr::Assignment(a) => {
                let src = self.translate_expression(&a.right, instructions)?;
                if let AssignmentTarget::Simple(target) = &*a.left {
                    if let Expr::Identifier(id) = &**target {
                        instructions.push(Instruction::AssignVar {
                            name: id.sym.clone(),
                            src,
                        });
                    }
                }
                Ok(src)
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

    fn translate_class(&mut self, c: &ClassDecl) -> Result<(), IrError> {
        // For now, classes lower to the same runtime representation as structs: a constructor
        // that copies fields from an initializer object. This matches the current parser
        // lowering of `Foo { x: 1 }` into `new Foo({ x: 1 })`.
        let mut fields = Vec::new();
        for member in &c.body.body {
            if let ClassMember::Field(f) = member {
                if let Expr::Identifier(id) = &f.key {
                    fields.push(Field {
                        name: id.sym.clone(),
                        ty: 0,
                    });
                }
            }
        }

        self.types.push(TypeDef::Struct {
            name: c.id.sym.clone(),
            fields,
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
                        _ => {
                            return Err(IrError::Unsupported("jsx attribute value".to_string()))
                        }
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
                    JsxChild::Expression(e) => args.push(self.translate_expression(e, instructions)?),
                    JsxChild::Element(e) => args.push(self.translate_jsx_element(e, instructions)?),
                    JsxChild::Fragment(f) => args.push(self.translate_jsx_fragment(f, instructions)?),
                    JsxChild::Spread(_) => {
                        return Err(IrError::Unsupported(
                            "jsx spread children".to_string(),
                        ))
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
                    return Err(IrError::Unsupported(
                        "jsx spread children".to_string(),
                    ))
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
struct LoopContext {
    break_target: BlockId,
    continue_target: BlockId,
}

struct FunctionLowerer<'a> {
    builder: &'a mut IrBuilder,
    blocks: Vec<BasicBlock>,
    current_id: BlockId,
    current_instructions: Vec<Instruction>,
    loop_stack: Vec<LoopContext>,
}

impl<'a> FunctionLowerer<'a> {
    fn new(builder: &'a mut IrBuilder, entry: BlockId) -> Self {
        Self {
            builder,
            blocks: Vec::new(),
            current_id: entry,
            current_instructions: Vec::new(),
            loop_stack: Vec::new(),
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
                self.current_instructions.push(Instruction::ExprStmt { value });
                Ok(false)
            }
            Stmt::Return(r) => {
                let value = if let Some(arg) = &r.argument {
                    Some(self.builder.translate_expression(
                        arg,
                        &mut self.current_instructions,
                    )?)
                } else {
                    None
                };
                self.finish_current(Terminator::Return(value))?;
                Ok(true)
            }
            Stmt::Block(b) => self.lower_stmt_list(&b.statements),
            Stmt::If(i) => self.lower_if(i),
            Stmt::While(w) => self.lower_while(w),
            Stmt::For(f) => self.lower_for(f),
            Stmt::DoWhile(d) => self.lower_do_while(d),
            Stmt::Break(_) => self.lower_break(),
            Stmt::Continue(_) => self.lower_continue(),
            Stmt::Empty(_) => Ok(false),
            _ => Err(IrError::Unsupported(format!("statement: {:?}", stmt))),
        }
    }

    fn lower_break(&mut self) -> Result<bool, IrError> {
        let ctx = self
            .loop_stack
            .last()
            .copied()
            .ok_or_else(|| IrError::InvalidAst("break outside of loop".to_string()))?;
        self.finish_current(Terminator::Jump(ctx.break_target))?;
        Ok(true)
    }

    fn lower_continue(&mut self) -> Result<bool, IrError> {
        let ctx = self
            .loop_stack
            .last()
            .copied()
            .ok_or_else(|| IrError::InvalidAst("continue outside of loop".to_string()))?;
        self.finish_current(Terminator::Jump(ctx.continue_target))?;
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
        self.loop_stack.push(LoopContext {
            break_target: after_id,
            continue_target: cond_id,
        });
        let body_terminated = self.lower_stmt(&w.body)?;
        self.loop_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(cond_id))?;
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
                    self.current_instructions.push(Instruction::ExprStmt { value: v });
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
        self.loop_stack.push(LoopContext {
            break_target: after_id,
            continue_target: update_id,
        });
        let body_terminated = self.lower_stmt(&f.body)?;
        self.loop_stack.pop();
        if !body_terminated {
            self.finish_current(Terminator::Jump(update_id))?;
        }

        self.start_block(update_id)?;
        if let Some(update) = &f.update {
            let v = self
                .builder
                .translate_expression(update, &mut self.current_instructions)?;
            self.current_instructions.push(Instruction::ExprStmt { value: v });
        }
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
        self.loop_stack.push(LoopContext {
            break_target: after_id,
            continue_target: cond_id,
        });
        let body_terminated = self.lower_stmt(&d.body)?;
        self.loop_stack.pop();
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
