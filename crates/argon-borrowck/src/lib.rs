//! Argon - Borrow checker
//! Implements Rust-style ownership and borrowing with lifetimes

#[cfg(test)]
mod borrow_checker_tests;

use argon_ast::*;
use argon_types::TypeCheckOutput;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Ownership {
    Owned,
    Moved,
    Borrowed(BorrowKind),
    Copied,
    SharedOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VariableState {
    name: String,
    ownership: Ownership,
    borrows: Vec<Borrow>,
    lifetime: Option<Lifetime>,
    is_copyable: bool,
    drop_scope: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Borrow {
    kind: BorrowKind,
    location: Span,
    lifetime: Lifetime,
    from: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub id: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LifetimeScope {
    id: usize,
    parent: Option<usize>,
    variables: HashSet<String>,
    children: Vec<usize>,
}

#[allow(dead_code)]
pub struct BorrowChecker {
    locals: HashMap<String, VariableState>,
    active_borrows: HashMap<String, Vec<Borrow>>,
    errors: Vec<BorrowError>,
    warnings: Vec<String>,
    scope_depth: usize,
    lifetime_counter: usize,
    lifetimes: HashMap<usize, LifetimeScope>,
    parent_lifetime: Option<usize>,
    returned_borrows: Vec<String>,
    loop_scope: usize,
    in_unsafe: bool,
    thread_access: HashSet<String>,
    type_info: Option<TypeCheckOutput>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
            active_borrows: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            scope_depth: 0,
            lifetime_counter: 0,
            lifetimes: HashMap::new(),
            parent_lifetime: None,
            returned_borrows: Vec::new(),
            loop_scope: 0,
            in_unsafe: false,
            thread_access: HashSet::new(),
            type_info: None,
        }
    }

    pub fn check(&mut self, source: &SourceFile) -> Result<(), BorrowError> {
        self.type_info = None;
        self.check_impl(source)
    }

    pub fn check_typed(
        &mut self,
        source: &SourceFile,
        type_info: TypeCheckOutput,
    ) -> Result<(), BorrowError> {
        self.type_info = Some(type_info);
        self.check_impl(source)
    }

    fn check_impl(&mut self, source: &SourceFile) -> Result<(), BorrowError> {
        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(())
    }

    fn generate_lifetime(&mut self) -> Lifetime {
        let id = self.lifetime_counter;
        self.lifetime_counter += 1;
        Lifetime {
            id,
            start: self.scope_depth,
            end: self.scope_depth,
        }
    }

    #[allow(dead_code)]
    fn extend_lifetime(&mut self, lifetime: &mut Lifetime) {
        lifetime.end = self.scope_depth;
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), BorrowError> {
        match stmt {
            Stmt::Variable(v) => {
                self.check_variable(v)?;
            }
            Stmt::Function(f) => {
                self.check_function(f)?;
            }
            Stmt::AsyncFunction(f) => {
                self.check_function(f)?;
            }
            Stmt::Block(b) => {
                self.check_block(b)?;
            }
            Stmt::If(i) => {
                self.check_if(i)?;
            }
            Stmt::While(w) => {
                self.check_while(w)?;
            }
            Stmt::Loop(l) => {
                self.check_loop(l)?;
            }
            Stmt::For(f) => {
                self.check_for(f)?;
            }
            Stmt::ForIn(fi) => {
                self.check_for_in(fi)?;
            }
            Stmt::Return(r) => {
                self.check_return(r)?;
            }
            Stmt::Expr(e) => {
                self.check_expression(&e.expr)?;
            }
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => {}
            Stmt::Match(m) => {
                self.check_match(m)?;
            }
            Stmt::Class(c) => {
                self.check_class(c)?;
            }
            Stmt::Struct(s) => for _field in &s.fields {},
            Stmt::Try(t) => {
                self.check_try(t)?;
            }
            Stmt::DoWhile(d) => {
                self.check_do_while(d)?;
            }
            Stmt::Throw(t) => {
                self.check_expression(&t.argument)?;
            }
            Stmt::Switch(s) => {
                self.check_switch(s)?;
            }
            Stmt::Import(_) | Stmt::Export(_) => {}
            Stmt::With(w) => {
                self.check_with(w)?;
            }
            Stmt::Labeled(l) => {
                self.check_statement(&l.body)?;
            }
            Stmt::Debugger(_) => {}
            _ => {}
        }
        Ok(())
    }

    fn check_block(&mut self, b: &BlockStmt) -> Result<(), BorrowError> {
        self.scope_depth += 1;

        let initial_locals = self.locals.clone();

        for stmt in &b.statements {
            self.check_statement(stmt)?;
        }

        let current_scope = self.scope_depth;
        self.locals.retain(|_name, state| {
            if state.drop_scope > current_scope {
                false
            } else {
                true
            }
        });

        if self.locals.len() < initial_locals.len() {
            for (name, state) in initial_locals {
                if !self.locals.contains_key(&name) {
                    self.check_drop(&name, &state)?;
                }
            }
        }

        self.scope_depth -= 1;
        Ok(())
    }

    fn check_drop(&mut self, name: &str, state: &VariableState) -> Result<(), BorrowError> {
        if let Some(borrows) = self.active_borrows.get(name) {
            for borrow in borrows {
                if borrow.lifetime.end >= state.drop_scope {
                    self.errors.push(BorrowError::BorrowedValueDropped {
                        variable: name.to_string(),
                        location: borrow.location.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn check_variable(&mut self, v: &VariableStmt) -> Result<(), BorrowError> {
        for decl in &v.declarations {
            if let Pattern::Identifier(id) = &decl.id {
                let mut ownership = if let Some(ref init) = decl.init {
                    self.check_moveable(init)?
                } else {
                    Ownership::Owned
                };

                let is_copyable = self.is_copyable_variable(&id.name.sym, &decl.init);
                if is_copyable && matches!(ownership, Ownership::Owned) {
                    ownership = Ownership::Copied;
                }
                let lifetime = self.generate_lifetime();

                self.locals.insert(
                    id.name.sym.clone(),
                    VariableState {
                        name: id.name.sym.clone(),
                        ownership,
                        borrows: Vec::new(),
                        lifetime: Some(lifetime),
                        is_copyable,
                        drop_scope: self.scope_depth,
                    },
                );
            }
        }
        Ok(())
    }

    fn is_copyable_variable(&self, name: &str, init: &Option<Expr>) -> bool {
        if let Some(info) = &self.type_info {
            if let Some(expr) = init {
                if let Some(ty) = info.expr_types.get(expr.span()) {
                    if let Some(t) = info.type_table.get(*ty) {
                        return t.is_copyable();
                    }
                }
            }

            if let Some(ty) = info.env.get_var(name) {
                if let Some(t) = info.type_table.get(ty) {
                    return t.is_copyable();
                }
            }
        }

        self.is_copyable_type_heuristic(init)
    }

    fn is_copyable_type_heuristic(&self, init: &Option<Expr>) -> bool {
        if let Some(expr) = init {
            match expr {
                Expr::Literal(lit) => matches!(
                    lit,
                    Literal::Number(_) | Literal::Boolean(_) | Literal::String(_)
                ),
                _ => false,
            }
        } else {
            false
        }
    }

    fn check_function(&mut self, f: &FunctionDecl) -> Result<(), BorrowError> {
        let old_locals = std::mem::take(&mut self.locals);
        let old_borrows = std::mem::take(&mut self.active_borrows);
        let old_returned = std::mem::take(&mut self.returned_borrows);

        for param in &f.params {
            if let Pattern::Identifier(id) = &param.pat {
                let lifetime = self.generate_lifetime();
                self.locals.insert(
                    id.name.sym.clone(),
                    VariableState {
                        name: id.name.sym.clone(),
                        ownership: Ownership::Owned,
                        borrows: Vec::new(),
                        lifetime: Some(lifetime),
                        is_copyable: false,
                        drop_scope: 0,
                    },
                );
            }
        }

        for stmt in &f.body.statements {
            self.check_statement(stmt)?;
        }

        self.check_no_active_borrows()?;

        self.locals = old_locals;
        self.active_borrows = old_borrows;
        self.returned_borrows = old_returned;
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

    fn check_if(&mut self, i: &IfStmt) -> Result<(), BorrowError> {
        self.check_expression(&i.condition)?;

        let then_locals = self.locals.clone();
        self.check_statement(&i.consequent)?;
        let then_borrows = self.active_borrows.clone();
        let _ = then_borrows;

        self.locals = then_locals;

        if let Some(ref alt) = i.alternate {
            self.check_statement(alt)?;
        }

        Ok(())
    }

    fn check_while(&mut self, w: &WhileStmt) -> Result<(), BorrowError> {
        self.loop_scope += 1;
        self.check_expression(&w.condition)?;
        self.check_statement(&w.body)?;
        self.loop_scope -= 1;
        Ok(())
    }

    fn check_loop(&mut self, l: &LoopStmt) -> Result<(), BorrowError> {
        self.loop_scope += 1;
        self.check_statement(&l.body)?;
        self.loop_scope -= 1;
        Ok(())
    }

    fn check_for(&mut self, f: &ForStmt) -> Result<(), BorrowError> {
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
        Ok(())
    }

    fn check_for_in(&mut self, f: &ForInStmt) -> Result<(), BorrowError> {
        self.check_expression(&f.right)?;

        match &f.left {
            ForInLeft::Pattern(pat) => {
                if let Pattern::Identifier(id) = pat {
                    let lifetime = self.generate_lifetime();
                    self.locals.insert(
                        id.name.sym.clone(),
                        VariableState {
                            name: id.name.sym.clone(),
                            ownership: Ownership::Owned,
                            borrows: Vec::new(),
                            lifetime: Some(lifetime),
                            is_copyable: false,
                            drop_scope: self.scope_depth,
                        },
                    );
                }
            }
            ForInLeft::Variable(v) => {
                self.check_variable(&VariableStmt {
                    kind: VariableKind::Let,
                    declarations: vec![v.clone()],
                    span: 0..0,
                })?;
            }
        }

        self.check_statement(&f.body)?;
        Ok(())
    }

    fn check_return(&mut self, r: &ReturnStmt) -> Result<(), BorrowError> {
        if let Some(ref arg) = r.argument {
            self.check_expression(arg)?;
            self.check_return_clears_borrows();
        }
        Ok(())
    }

    fn check_with(&mut self, w: &WithStmt) -> Result<(), BorrowError> {
        self.check_expression(&w.object)?;
        self.check_statement(&w.body)?;
        Ok(())
    }

    fn check_match(&mut self, m: &MatchStmt) -> Result<(), BorrowError> {
        self.check_expression(&m.discriminant)?;
        for case in &m.cases {
            self.check_statement(&case.consequent)?;
        }
        Ok(())
    }

    fn check_try(&mut self, t: &TryStmt) -> Result<(), BorrowError> {
        self.check_block(&t.block)?;
        if let Some(ref handler) = t.handler {
            self.check_block(&handler.body)?;
        }
        if let Some(ref fin) = t.finalizer {
            self.check_block(fin)?;
        }
        Ok(())
    }

    fn check_do_while(&mut self, d: &DoWhileStmt) -> Result<(), BorrowError> {
        self.loop_scope += 1;
        self.check_statement(&d.body)?;
        self.check_expression(&d.condition)?;
        self.loop_scope -= 1;
        Ok(())
    }

    fn check_switch(&mut self, s: &SwitchStmt) -> Result<(), BorrowError> {
        self.check_expression(&s.discriminant)?;
        for case in &s.cases {
            for stmt in &case.consequent {
                self.check_statement(stmt)?;
            }
        }
        Ok(())
    }

    fn check_expression(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        match expr {
            Expr::Identifier(id) => {
                self.check_identifier(id)?;
            }
            Expr::Assignment(a) => {
                self.check_assignment(&a.left, &a.right)?;
            }
            Expr::Call(c) => {
                self.check_call(c)?;
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
                self.check_object(o)?;
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
            Expr::Super(_) => {}
            Expr::Spread(s) => {
                self.check_expression(&s.argument)?;
            }
            Expr::Await(a) => {
                self.check_expression(&a.argument)?;
            }
            Expr::Yield(y) => {
                if let Some(ref arg) = y.argument {
                    self.check_expression(arg)?;
                }
            }
            Expr::Update(u) => {
                self.check_expression(&u.argument)?;
            }
            Expr::Chain(c) => {
                for elem in &c.expressions {
                    match elem {
                        ChainElement::Call(call) => self.check_call(call)?,
                        ChainElement::Member(m) => {
                            self.check_expression(&m.object)?;
                        }
                        ChainElement::OptionalCall(c) => {
                            self.check_expression(&c.callee)?;
                            for arg in &c.arguments {
                                if let ExprOrSpread::Expr(e) = arg {
                                    self.check_expression(e)?;
                                }
                            }
                        }
                        ChainElement::OptionalMember(m) => {
                            self.check_expression(&m.object)?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn check_identifier(&mut self, id: &Ident) -> Result<(), BorrowError> {
        if let Some(state) = self.locals.get(&id.sym) {
            if let Ownership::Moved = state.ownership {
                self.errors.push(BorrowError::UseAfterMove {
                    variable: id.sym.clone(),
                    location: id.span.clone(),
                });
            }

            if let Some(borrows) = self.active_borrows.get(&id.sym) {
                for borrow in borrows {
                    if borrow.kind == BorrowKind::Mutable && self.loop_scope > 0 {
                        self.errors.push(BorrowError::MutableBorrowInLoop {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                        });
                    }
                }
            }
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
                        match state.ownership {
                            Ownership::Borrowed(BorrowKind::Mutable) => {}
                            _ => {
                                state.ownership = Ownership::Owned;
                            }
                        }
                    }
                }
            }
            AssignmentTarget::Member(member) => {
                self.check_expression(&member.object)?;
            }
            AssignmentTarget::Pattern(_) => {}
        }
        Ok(())
    }

    fn check_call(&mut self, c: &CallExpr) -> Result<(), BorrowError> {
        self.check_expression(&c.callee)?;

        for arg in &c.arguments {
            if let ExprOrSpread::Expr(e) = arg {
                self.check_expression(e)?;
            }
        }

        if let Expr::Identifier(id) = &*c.callee {
            if id.sym == "process" || id.sym == "thread" {
                for arg in &c.arguments {
                    if let ExprOrSpread::Expr(Expr::Identifier(id)) = arg {
                        self.thread_access.insert(id.sym.clone());
                    }
                }
            }
        }

        Ok(())
    }

    fn check_borrow(&mut self, expr: &Expr, kind: BorrowKind) -> Result<(), BorrowError> {
        if let Expr::Identifier(id) = expr {
            if let Some(state) = self.locals.get_mut(&id.sym) {
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
                    if self.loop_scope > 0 {
                        self.errors.push(BorrowError::MutableBorrowInLoop {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                        });
                    }
                    state.ownership = Ownership::Borrowed(kind);
                } else {
                    if let Ownership::Owned = state.ownership {
                        state.ownership = Ownership::Borrowed(kind);
                    }
                }

                let borrow = Borrow {
                    kind,
                    location: id.span.clone(),
                    lifetime: self.generate_lifetime(),
                    from: id.sym.clone(),
                };

                self.active_borrows
                    .entry(id.sym.clone())
                    .or_insert_with(Vec::new)
                    .push(borrow);
            }
        }
        Ok(())
    }

    fn check_object(&mut self, o: &ObjectExpression) -> Result<(), BorrowError> {
        for prop in &o.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    if let ExprOrSpread::Expr(e) = &p.value {
                        self.check_expression(e)?;
                    }
                }
                ObjectProperty::Shorthand(id) => {
                    self.check_identifier(id)?;
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
        Ok(())
    }

    fn check_moveable(&self, expr: &Expr) -> Result<Ownership, BorrowError> {
        match expr {
            Expr::Literal(_) => Ok(Ownership::Copied),
            Expr::Identifier(id) => {
                if let Some(state) = self.locals.get(&id.sym) {
                    if state.is_copyable {
                        Ok(Ownership::Copied)
                    } else {
                    Ok(state.ownership.clone())
                    }
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
        self.returned_borrows.clear();
    }

    fn check_no_active_borrows(&mut self) -> Result<(), BorrowError> {
        for (name, borrows) in &self.active_borrows {
            for borrow in borrows {
                if borrow.lifetime.end > self.scope_depth {
                    self.errors.push(BorrowError::LifetimeError(format!(
                        "borrow of '{}' does not live long enough",
                        name
                    )));
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn check_data_race(&mut self) -> Result<(), BorrowError> {
        let mutable_borrowed: HashSet<_> = self
            .active_borrows
            .iter()
            .filter_map(|(name, borrows)| {
                if borrows.iter().any(|b| b.kind == BorrowKind::Mutable) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for var in &self.thread_access {
            if mutable_borrowed.contains(var) {
                self.errors.push(BorrowError::DataRace {
                    variable: var.clone(),
                });
            }
        }

        Ok(())
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
    BorrowedValueDropped {
        variable: String,
        location: Span,
    },
    MutableBorrowInLoop {
        variable: String,
        location: Span,
    },
    DataRace {
        variable: String,
    },
    CannotMove {
        variable: String,
        location: Span,
    },
    DropOfBorrowedValue {
        variable: String,
        location: Span,
    },
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::UseAfterMove { variable, location } => {
                write!(
                    f,
                    "use after move: variable '{}' at {:?}",
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
                    "borrow conflict: {} at {:?}: {}",
                    variable, location, message
                )
            }
            BorrowError::LifetimeError(msg) => write!(f, "lifetime error: {}", msg),
            BorrowError::InvalidBorrow {
                variable,
                location,
                message,
            } => {
                write!(
                    f,
                    "invalid borrow: {} at {:?}: {}",
                    variable, location, message
                )
            }
            BorrowError::BorrowedValueDropped { variable, location } => {
                write!(
                    f,
                    "borrowed value dropped: '{}' at {:?}",
                    variable, location
                )
            }
            BorrowError::MutableBorrowInLoop { variable, location } => {
                write!(
                    f,
                    "mutable borrow in loop: '{}' at {:?}",
                    variable, location
                )
            }
            BorrowError::DataRace { variable } => {
                write!(f, "potential data race on '{}'", variable)
            }
            BorrowError::CannotMove { variable, location } => {
                write!(f, "cannot move: '{}' at {:?}", variable, location)
            }
            BorrowError::DropOfBorrowedValue { variable, location } => {
                write!(
                    f,
                    "drop of borrowed value: '{}' at {:?}",
                    variable, location
                )
            }
        }
    }
}

impl std::error::Error for BorrowError {}
