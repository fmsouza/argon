//! SafeScript - Borrow checker
//! Implements Rust-style ownership and borrowing

#[cfg(test)]
mod borrow_checker_tests;

use safescript_ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Ownership {
    Owned,
    Moved,
    Borrowed(BorrowKind),
    Copied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone)]
struct VariableState {
    ownership: Ownership,
    borrows: Vec<Borrow>,
}

#[derive(Debug, Clone)]
struct Borrow {
    kind: BorrowKind,
    location: Span,
}

pub struct BorrowChecker {
    locals: HashMap<String, VariableState>,
    active_borrows: HashSet<String>,
    errors: Vec<BorrowError>,
    scope_depth: usize,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
            active_borrows: HashSet::new(),
            errors: Vec::new(),
            scope_depth: 0,
        }
    }

    pub fn check(&mut self, source: &SourceFile) -> Result<(), BorrowError> {
        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(())
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), BorrowError> {
        match stmt {
            Stmt::Variable(v) => {
                self.check_variable(v)?;
            }
            Stmt::Function(f) => {
                self.check_function(f)?;
            }
            Stmt::Block(b) => {
                self.check_block(b)?;
            }
            Stmt::If(i) => {
                self.check_statement(&i.consequent)?;
                if let Some(ref alt) = i.alternate {
                    self.check_statement(alt)?;
                }
            }
            Stmt::While(w) => {
                self.check_statement(&w.body)?;
            }
            Stmt::For(f) => {
                if let Some(ref init) = f.init {
                    match init {
                        ForInit::Variable(v) => self.check_variable(v)?,
                        ForInit::Expr(e) => self.check_expression(e)?,
                    }
                }
                if let Some(ref test) = f.test {
                    self.check_expression(test)?;
                }
                if let Some(ref update) = f.update {
                    self.check_expression(update)?;
                }
                self.check_statement(&f.body)?;
            }
            Stmt::Return(r) => {
                if let Some(ref arg) = r.argument {
                    self.check_expression(arg)?;
                }
                self.check_return_clears_borrows();
            }
            Stmt::Expr(e) => {
                self.check_expression(&e.expr)?;
            }
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => {}
            Stmt::Match(m) => {
                self.check_expression(&m.discriminant)?;
                for case in &m.cases {
                    self.check_statement(&case.consequent)?;
                }
            }
            Stmt::Class(c) => {
                self.check_class(c)?;
            }
            Stmt::Struct(s) => {
                for field in &s.fields {
                    // Type annotations don't need borrow checking
                }
            }
            Stmt::Try(t) => {
                self.check_block(&t.block)?;
                if let Some(ref handler) = t.handler {
                    self.check_block(&handler.body)?;
                }
                if let Some(ref fin) = t.finalizer {
                    self.check_block(fin)?;
                }
            }
            Stmt::DoWhile(d) => {
                self.check_statement(&d.body)?;
                self.check_expression(&d.condition)?;
            }
            Stmt::Throw(t) => {
                self.check_expression(&t.argument)?;
            }
            Stmt::Switch(s) => {
                self.check_expression(&s.discriminant)?;
                for case in &s.cases {
                    for stmt in &case.consequent {
                        self.check_statement(stmt)?;
                    }
                }
            }
            Stmt::Import(_) | Stmt::Export(_) => {}
            Stmt::With(w) => {
                self.check_expression(&w.object)?;
                self.check_statement(&w.body)?;
            }
            Stmt::Labeled(l) => {
                self.check_statement(&l.body)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn check_block(&mut self, b: &BlockStmt) -> Result<(), BorrowError> {
        self.scope_depth += 1;
        for stmt in &b.statements {
            self.check_statement(stmt)?;
        }
        self.scope_depth -= 1;

        self.locals.retain(|_, state| {
            if let Ownership::Owned = state.ownership {
                true
            } else {
                false
            }
        });

        Ok(())
    }

    fn check_variable(&mut self, v: &VariableStmt) -> Result<(), BorrowError> {
        for decl in &v.declarations {
            if let Pattern::Identifier(id) = &decl.id {
                let ownership = if let Some(ref init) = decl.init {
                    self.check_moveable(init)?
                } else {
                    Ownership::Owned
                };

                self.locals.insert(
                    id.name.sym.clone(),
                    VariableState {
                        ownership,
                        borrows: Vec::new(),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_function(&mut self, f: &FunctionDecl) -> Result<(), BorrowError> {
        let old_locals = std::mem::take(&mut self.locals);

        for param in &f.params {
            if let Pattern::Identifier(id) = &param.pat {
                self.locals.insert(
                    id.name.sym.clone(),
                    VariableState {
                        ownership: Ownership::Owned,
                        borrows: Vec::new(),
                    },
                );
            }
        }

        for stmt in &f.body.statements {
            self.check_statement(stmt)?;
        }

        self.locals = old_locals;
        Ok(())
    }

    fn check_class(&mut self, c: &ClassDecl) -> Result<(), BorrowError> {
        for member in &c.body.body {
            match member {
                ClassMember::Method(m) => self.check_function(&m.value)?,
                ClassMember::Constructor(c) => {
                    for stmt in &c.body.statements {
                        self.check_statement(stmt)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_expression(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        match expr {
            Expr::Identifier(id) => {
                if let Some(state) = self.locals.get(&id.sym) {
                    if let Ownership::Moved = state.ownership {
                        self.errors.push(BorrowError::UseAfterMove {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                        });
                    }
                }
            }
            Expr::Assignment(a) => {
                self.check_assignment(&a.left, &a.right)?;
            }
            Expr::Call(c) => {
                for arg in &c.arguments {
                    if let ExprOrSpread::Expr(e) = arg {
                        self.check_expression(e)?;
                    }
                }
            }
            Expr::Binary(b) => {
                self.check_expression(&b.left)?;
                self.check_expression(&b.right)?;
            }
            Expr::Unary(u) => {
                self.check_expression(&u.argument)?;
            }
            Expr::Member(m) => {
                self.check_expression(&m.object)?;
                if m.computed {
                    self.check_expression(&m.property)?;
                }
            }
            Expr::Ref(r) => {
                self.check_borrow(&r.expr, BorrowKind::Shared)?;
            }
            Expr::MutRef(r) => {
                self.check_borrow(&r.expr, BorrowKind::Mutable)?;
            }
            Expr::New(n) => {
                self.check_expression(&n.callee)?;
                for arg in &n.arguments {
                    if let ExprOrSpread::Expr(e) = arg {
                        self.check_expression(e)?;
                    }
                }
            }
            Expr::Conditional(c) => {
                self.check_expression(&c.test)?;
                self.check_expression(&c.consequent)?;
                self.check_expression(&c.alternate)?;
            }
            Expr::Logical(l) => {
                self.check_expression(&l.left)?;
                self.check_expression(&l.right)?;
            }
            Expr::Object(o) => {
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            if let ExprOrSpread::Expr(e) = &p.value {
                                self.check_expression(e)?;
                            }
                        }
                        ObjectProperty::Shorthand(id) => {
                            if let Some(state) = self.locals.get(&id.sym) {
                                if let Ownership::Moved = state.ownership {
                                    self.errors.push(BorrowError::UseAfterMove {
                                        variable: id.sym.clone(),
                                        location: id.span.clone(),
                                    });
                                }
                            }
                        }
                        ObjectProperty::Spread(s) => {
                            self.check_expression(&s.argument)?;
                        }
                        ObjectProperty::Method(m) => {
                            self.check_function(&m.value)?;
                        }
                        _ => {}
                    }
                }
            }
            Expr::Array(a) => {
                for elem in &a.elements {
                    if let Some(ExprOrSpread::Expr(e)) = elem {
                        self.check_expression(e)?;
                    }
                }
            }
            Expr::Function(f) => {
                for stmt in &f.body.statements {
                    self.check_statement(stmt)?;
                }
            }
            Expr::ArrowFunction(a) => match &a.body {
                ArrowFunctionBody::Block(b) => {
                    for stmt in &b.statements {
                        self.check_statement(stmt)?;
                    }
                }
                ArrowFunctionBody::Expr(e) => {
                    self.check_expression(e)?;
                }
            },
            Expr::This(_) => {}
            Expr::Literal(_) => {}
            _ => {}
        }
        Ok(())
    }

    fn check_assignment(
        &mut self,
        target: &AssignmentTarget,
        value: &Expr,
    ) -> Result<(), BorrowError> {
        self.check_expression(value)?;

        match target {
            AssignmentTarget::Simple(expr) => {
                if let Expr::Identifier(id) = &**expr {
                    if let Some(state) = self.locals.get_mut(&id.sym) {
                        if let Ownership::Borrowed(BorrowKind::Mutable) = state.ownership {
                            // Can't reassign a mutable borrow
                        } else {
                            state.ownership = Ownership::Owned;
                        }
                    }
                }
            }
            AssignmentTarget::Member(member) => {
                self.check_expression(&member.object)?;
            }
            AssignmentTarget::Pattern(_) => {}
            AssignmentTarget::Pattern(_) => {}
        }
        Ok(())
    }

    fn check_borrow(&mut self, expr: &Expr, kind: BorrowKind) -> Result<(), BorrowError> {
        if let Expr::Identifier(id) = expr {
            if let Some(state) = self.locals.get_mut(&id.sym) {
                // Check for mutable borrow conflict
                if kind == BorrowKind::Mutable {
                    if let Ownership::Borrowed(BorrowKind::Shared) = &state.ownership {
                        self.errors.push(BorrowError::BorrowConflict {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                            message: "Cannot mutably borrow while shared borrow exists".to_string(),
                        });
                    }
                    if let Ownership::Borrowed(BorrowKind::Mutable) = &state.ownership {
                        self.errors.push(BorrowError::BorrowConflict {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                            message: "Cannot have multiple mutable borrows".to_string(),
                        });
                    }
                    // Mark as borrowed - can't use while mutable borrow exists
                    state.ownership = Ownership::Borrowed(kind);
                } else {
                    // Shared borrow - mark as borrowed
                    if let Ownership::Owned = state.ownership {
                        state.ownership = Ownership::Borrowed(kind);
                    }
                }

                state.borrows.push(Borrow {
                    kind,
                    location: id.span.clone(),
                });
                self.active_borrows.insert(id.sym.clone());
            }
        }
        Ok(())
    }

    fn check_moveable(&self, expr: &Expr) -> Result<Ownership, BorrowError> {
        match expr {
            Expr::Literal(_) => Ok(Ownership::Copied),
            Expr::Identifier(id) => {
                if let Some(state) = self.locals.get(&id.sym) {
                    Ok(state.ownership.clone())
                } else {
                    Ok(Ownership::Copied)
                }
            }
            Expr::Object(_) => Ok(Ownership::Owned),
            Expr::Array(_) => Ok(Ownership::Owned),
            Expr::New(_) => Ok(Ownership::Owned),
            _ => Ok(Ownership::Owned),
        }
    }

    fn check_return_clears_borrows(&mut self) {
        for (_, state) in &mut self.locals {
            state.borrows.clear();
        }
        self.active_borrows.clear();
    }
}

#[derive(Debug)]
pub enum BorrowError {
    UseAfterMove {
        variable: String,
        location: Span,
    },
    BorrowConflict {
        variable: String,
        location: Span,
        message: String,
    },
    LifetimeError(String),
    InvalidBorrow {
        variable: String,
        location: Span,
        message: String,
    },
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::UseAfterMove { variable, location } => {
                write!(
                    f,
                    "Use after move: variable '{}' at {:?}",
                    variable, location
                )
            }
            BorrowError::BorrowConflict {
                variable,
                location,
                message,
            } => {
                write!(
                    f,
                    "Borrow conflict: {} at {:?}: {}",
                    variable, location, message
                )
            }
            BorrowError::LifetimeError(msg) => write!(f, "Lifetime error: {}", msg),
            BorrowError::InvalidBorrow {
                variable,
                location,
                message,
            } => {
                write!(
                    f,
                    "Invalid borrow: {} at {:?}: {}",
                    variable, location, message
                )
            }
        }
    }
}

impl std::error::Error for BorrowError {}
