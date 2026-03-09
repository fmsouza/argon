//! SafeScript - Type checker

use crate::types::{FunctionSig, StructDef, Type as CompType, TypeId, TypeTable};
use safescript_ast::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TypeEnvironment {
    vars: HashMap<String, TypeId>,
    structs: HashMap<String, StructDef>,
    functions: HashMap<String, FunctionSig>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        let mut env = Self {
            vars: HashMap::new(),
            structs: HashMap::new(),
            functions: HashMap::new(),
        };
        env
    }

    pub fn get_var(&self, name: &str) -> Option<TypeId> {
        self.vars.get(name).copied()
    }

    pub fn add_var(&mut self, name: String, ty: TypeId) {
        self.vars.insert(name, ty);
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructDef> {
        self.structs.get(name)
    }

    pub fn add_struct(&mut self, name: String, def: StructDef) {
        self.structs.insert(name, def);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSig) {
        self.functions.insert(name, sig);
    }
}

pub struct TypeChecker {
    type_table: TypeTable,
    env: TypeEnvironment,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            type_table: TypeTable::new(),
            env: TypeEnvironment::new(),
        }
    }

    pub fn check(&mut self, source: &SourceFile) -> Result<(), TypeError> {
        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::Variable(v) => self.check_variable(v),
            Stmt::Function(f) => self.check_function(f),
            Stmt::Struct(s) => self.check_struct(s),
            Stmt::Class(c) => self.check_class(c),
            Stmt::Return(r) => self.check_return(r),
            Stmt::If(i) => self.check_if(i),
            Stmt::While(w) => self.check_while(w),
            Stmt::For(f) => self.check_for(f),
            Stmt::Block(b) => self.check_block(b),
            Stmt::Expr(e) => {
                self.infer_expression(&e.expr);
                Ok(())
            }
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => Ok(()),
            Stmt::Switch(s) => self.check_switch(s),
            Stmt::Try(t) => self.check_try(t),
            Stmt::Throw(t) => {
                self.infer_expression(&t.argument);
                Ok(())
            }
            Stmt::DoWhile(d) => self.check_do_while(d),
            Stmt::Match(m) => self.check_match(m),
            Stmt::Import(_) | Stmt::Export(_) => Ok(()),
            _ => Ok(()),
        }
    }

    fn check_variable(&mut self, stmt: &VariableStmt) -> Result<(), TypeError> {
        for decl in &stmt.declarations {
            let type_id = self.type_table.add(CompType::Unknown);

            if let Pattern::Identifier(id) = &decl.id {
                self.env.add_var(id.name.sym.clone(), type_id);
            }

            if let Some(init) = &decl.init {
                self.infer_expression(init);
            }
        }
        Ok(())
    }

    fn check_function(&mut self, f: &FunctionDecl) -> Result<(), TypeError> {
        for stmt in &f.body.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_struct(&mut self, s: &StructDecl) -> Result<(), TypeError> {
        let fields: Vec<crate::types::FieldDef> = s
            .fields
            .iter()
            .map(|f| crate::types::FieldDef {
                name: f.id.sym.clone(),
                ty: self.type_table.add(CompType::Unknown),
            })
            .collect();

        let struct_def = StructDef {
            name: s.id.sym.clone(),
            fields,
        };

        self.env.add_struct(s.id.sym.clone(), struct_def);
        Ok(())
    }

    fn check_class(&mut self, c: &ClassDecl) -> Result<(), TypeError> {
        for member in &c.body.body {
            if let ClassMember::Method(m) = member {
                self.check_function(&m.value)?;
            }
        }
        Ok(())
    }

    fn check_return(&mut self, r: &ReturnStmt) -> Result<(), TypeError> {
        if let Some(ref arg) = r.argument {
            self.infer_expression(arg);
        }
        Ok(())
    }

    fn check_if(&mut self, i: &IfStmt) -> Result<(), TypeError> {
        self.infer_expression(&i.condition);
        self.check_statement(&i.consequent)?;
        if let Some(ref alt) = i.alternate {
            self.check_statement(alt)?;
        }
        Ok(())
    }

    fn check_while(&mut self, w: &WhileStmt) -> Result<(), TypeError> {
        self.infer_expression(&w.condition);
        self.check_statement(&w.body)?;
        Ok(())
    }

    fn check_for(&mut self, f: &ForStmt) -> Result<(), TypeError> {
        if let Some(ref init) = f.init {
            match init {
                ForInit::Variable(v) => self.check_variable(v)?,
                ForInit::Expr(e) => self.infer_expression(e),
            }
        }
        if let Some(ref test) = f.test {
            self.infer_expression(test);
        }
        if let Some(ref update) = f.update {
            self.infer_expression(update);
        }
        self.check_statement(&f.body)?;
        Ok(())
    }

    fn check_block(&mut self, b: &BlockStmt) -> Result<(), TypeError> {
        for stmt in &b.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_switch(&mut self, s: &SwitchStmt) -> Result<(), TypeError> {
        self.infer_expression(&s.discriminant);
        for case in &s.cases {
            for stmt in &case.consequent {
                self.check_statement(stmt)?;
            }
        }
        Ok(())
    }

    fn check_try(&mut self, t: &TryStmt) -> Result<(), TypeError> {
        self.check_block(&t.block)?;
        if let Some(ref handler) = t.handler {
            self.check_block(&handler.body)?;
        }
        if let Some(ref fin) = t.finalizer {
            self.check_block(fin)?;
        }
        Ok(())
    }

    fn check_do_while(&mut self, d: &DoWhileStmt) -> Result<(), TypeError> {
        self.check_statement(&d.body)?;
        self.infer_expression(&d.condition);
        Ok(())
    }

    fn check_match(&mut self, m: &MatchStmt) -> Result<(), TypeError> {
        self.infer_expression(&m.discriminant);
        for case in &m.cases {
            self.infer_expression(&case.pattern);
            self.check_statement(&case.consequent)?;
        }
        Ok(())
    }

    fn infer_expression(&mut self, _expr: &Expr) {
        // Type inference - simplified for now
        // The actual implementation would traverse the AST and build type information
    }

    fn infer_literal(&mut self, lit: &Literal) -> TypeId {
        match lit {
            Literal::Number(_) => self.type_table.add(CompType::Number),
            Literal::String(_) => self.type_table.add(CompType::String),
            Literal::Boolean(_) => self.type_table.add(CompType::Boolean),
            Literal::Null(_) => self.type_table.add(CompType::Null),
            Literal::RegExp(_) => self.type_table.add(CompType::Object),
        }
    }

    fn infer_identifier(&mut self, id: &Ident) -> TypeId {
        if let Some(ty) = self.env.get_var(&id.sym) {
            return ty;
        }
        self.type_table.add(CompType::Unknown)
    }
}

#[derive(Debug)]
pub enum TypeError {
    Mismatch(String),
    NotFound(String),
    Invalid(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::Mismatch(msg) => write!(f, "Type mismatch: {}", msg),
            TypeError::NotFound(msg) => write!(f, "Type not found: {}", msg),
            TypeError::Invalid(msg) => write!(f, "Invalid type: {}", msg),
        }
    }
}

impl std::error::Error for TypeError {}
