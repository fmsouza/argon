//! SafeScript - Intermediate representation

use indexmap::IndexMap;
use safescript_ast::*;
use std::collections::HashMap;

pub type BlockId = usize;
pub type ValueId = usize;

#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    pub types: Vec<TypeDef>,
    pub globals: Vec<Global>,
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
        func: String,
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
    locals: HashMap<String, ValueId>,
    functions: Vec<Function>,
    types: Vec<TypeDef>,
    globals: Vec<Global>,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            locals: HashMap::new(),
            functions: Vec::new(),
            types: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn build(&mut self, source: &SourceFile) -> Result<Module, IrError> {
        for stmt in &source.statements {
            self.translate_statement(stmt)?;
        }
        Ok(Module {
            functions: self.functions.clone(),
            types: self.types.clone(),
            globals: self.globals.clone(),
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

    fn translate_statement(&mut self, stmt: &Stmt) -> Result<(), IrError> {
        match stmt {
            Stmt::Function(f) => self.translate_function(f),
            Stmt::Variable(v) => self.translate_variable(v),
            Stmt::Struct(s) => self.translate_struct(s),
            _ => Ok(()),
        }
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
                let v = self.new_value();
                self.locals.insert(id.name.sym.clone(), v);
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

        self.locals.clear();

        Ok(())
    }

    fn translate_statement_to_instructions(
        &mut self,
        stmt: &Stmt,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), IrError> {
        match stmt {
            Stmt::Variable(v) => {
                for decl in &v.declarations {
                    if let Pattern::Identifier(id) = &decl.id {
                        let dest = self.new_value();
                        if let Some(init) = &decl.init {
                            let _src = self.translate_expression(init, instructions)?;
                        }
                        self.locals.insert(id.name.sym.clone(), dest);
                    }
                }
            }
            Stmt::Return(r) => {
                let value = r
                    .argument
                    .as_ref()
                    .and_then(|e| self.translate_expression(e, instructions).ok());
                instructions.push(Instruction::Return { value });
            }
            Stmt::Expr(e) => {
                self.translate_expression(&e.expr, instructions)?;
            }
            Stmt::Block(b) => {
                for s in &b.statements {
                    self.translate_statement_to_instructions(s, instructions)?;
                }
            }
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
                if let Some(&v) = self.locals.get(&id.sym) {
                    Ok(v)
                } else {
                    let dest = self.new_value();
                    Ok(dest)
                }
            }
            Expr::Binary(b) => {
                let lhs = self.translate_expression(&b.left, instructions)?;
                let rhs = self.translate_expression(&b.right, instructions)?;
                let dest = self.new_value();
                let op = BinOp::Add; // Simplified
                instructions.push(Instruction::BinOp { op, lhs, rhs, dest });
                Ok(dest)
            }
            Expr::Unary(u) => {
                let arg = self.translate_expression(&u.argument, instructions)?;
                let dest = self.new_value();
                let op = UnOp::Neg;
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
                let func = "call".to_string();
                instructions.push(Instruction::Call { func, args, dest });
                Ok(dest)
            }
            Expr::Assignment(a) => {
                let src = self.translate_expression(&a.right, instructions)?;
                Ok(src)
            }
            _ => {
                let dest = self.new_value();
                Ok(dest)
            }
        }
    }

    fn translate_variable(&mut self, v: &VariableStmt) -> Result<(), IrError> {
        for decl in &v.declarations {
            if let Pattern::Identifier(id) = &decl.id {
                let dest = self.new_value();
                if let Some(init) = &decl.init {
                    let src = self.translate_expression(init, &mut vec![])?;
                    self.globals.push(Global {
                        name: id.name.sym.clone(),
                        ty: 0,
                        init: Some(src),
                    });
                }
                self.locals.insert(id.name.sym.clone(), dest);
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
