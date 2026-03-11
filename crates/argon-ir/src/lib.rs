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
    Branch {
        cond: ValueId,
        then: BlockId,
        else_: BlockId,
    },
    Jump {
        target: BlockId,
    },
    Return {
        value: Option<ValueId>,
    },
    Const {
        dest: ValueId,
        value: ConstValue,
    },
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
        let mut init_instructions = Vec::new();

        for stmt in &source.statements {
            match stmt {
                Stmt::Function(f) => self.translate_function(f)?,
                Stmt::Struct(s) => self.translate_struct(s)?,
                Stmt::Variable(v) => self.translate_variable_stmt(v, &mut init_instructions)?,
                Stmt::Expr(e) => {
                    let value = self.translate_expression(&e.expr, &mut init_instructions)?;
                    init_instructions.push(Instruction::ExprStmt { value });
                }
                Stmt::Import(i) => self.imports.push(i.clone()),
                Stmt::Export(e) => self.exports.push(e.clone()),
                _ => {}
            }
        }

        if !init_instructions.is_empty() {
            let block_id = self.new_block();
            self.functions.push(Function {
                id: "__argon_init".to_string(),
                params: Vec::new(),
                return_type: None,
                body: vec![BasicBlock {
                    id: block_id,
                    instructions: init_instructions,
                    terminator: Terminator::Return(None),
                }],
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

    fn translate_function(&mut self, f: &FunctionDecl) -> Result<(), IrError> {
        let func_name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();

        let mut params = Vec::new();
        for p in &f.params {
            if let Pattern::Identifier(id) = &p.pat {
                let ty = 0;
                params.push(Param {
                    name: id.name.sym.clone(),
                    ty,
                });
            }
        }

        let mut body = Vec::new();
        let block_id = self.new_block();
        let mut instructions = Vec::new();

        for stmt in &f.body.statements {
            self.translate_statement_to_instructions(stmt, &mut instructions)?;
        }

        // Add implicit return if needed
        if instructions.is_empty()
            || !matches!(instructions.last(), Some(Instruction::Return { .. }))
        {
            instructions.push(Instruction::Return { value: None });
        }

        let terminator = if let Some(last) = instructions.pop() {
            match last {
                Instruction::Return { value } => Terminator::Return(value),
                _ => {
                    instructions.push(last);
                    Terminator::Return(None)
                }
            }
        } else {
            Terminator::Return(None)
        };

        body.push(BasicBlock {
            id: block_id,
            instructions,
            terminator,
        });

        self.functions.push(Function {
            id: func_name,
            params,
            return_type: None,
            body,
        });

        Ok(())
    }

    fn translate_statement_to_instructions(
        &mut self,
        stmt: &Stmt,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), IrError> {
        match stmt {
            Stmt::Variable(v) => {
                self.translate_variable_stmt(v, instructions)?;
            }
            Stmt::Return(r) => {
                let value = if let Some(arg) = &r.argument {
                    Some(self.translate_expression(arg, instructions)?)
                } else {
                    None
                };
                instructions.push(Instruction::Return { value });
            }
            Stmt::Expr(e) => {
                let value = self.translate_expression(&e.expr, instructions)?;
                instructions.push(Instruction::ExprStmt { value });
            }
            Stmt::Block(b) => {
                for s in &b.statements {
                    self.translate_statement_to_instructions(s, instructions)?;
                }
            }
            Stmt::If(_) => return Err(IrError::Unsupported("if statement".to_string())),
            Stmt::While(_) => return Err(IrError::Unsupported("while loop".to_string())),
            Stmt::For(_) => return Err(IrError::Unsupported("for loop".to_string())),
            Stmt::DoWhile(_) => return Err(IrError::Unsupported("do-while loop".to_string())),
            _ => {}
        }
        Ok(())
    }

    fn translate_expression(
        &mut self,
        expr: &Expr,
        instructions: &mut Vec<Instruction>,
    ) -> Result<ValueId, IrError> {
        match expr {
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
