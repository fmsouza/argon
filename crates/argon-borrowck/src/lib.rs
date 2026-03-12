//! Argon - Borrow checker
//! Implements Rust-style ownership and borrowing with lifetimes

#[cfg(test)]
mod borrow_checker_tests;

use argon_ast::*;
use argon_types::{Type as CheckedType, TypeCheckOutput, TypeId as CheckedTypeId};
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

#[derive(Debug, Clone)]
struct FunctionBorrowContract {
    param_borrows: Vec<Option<BorrowKind>>,
}

#[derive(Debug, Clone)]
struct FunctionBorrowContext {
    return_borrow: Option<BorrowKind>,
    param_borrows: HashMap<String, Option<BorrowKind>>,
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
    function_contracts: HashMap<String, FunctionBorrowContract>,
    current_function_context: Option<FunctionBorrowContext>,
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
            function_contracts: HashMap::new(),
            current_function_context: None,
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
        self.function_contracts.clear();
        self.collect_function_contracts(source);

        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }

        self.check_data_race()?;

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(())
    }

    fn collect_function_contracts(&mut self, source: &SourceFile) {
        for stmt in &source.statements {
            self.collect_function_contracts_from_stmt(stmt);
        }
    }

    fn collect_function_contracts_from_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Function(f) | Stmt::AsyncFunction(f) => {
                if let Some(id) = &f.id {
                    self.function_contracts.insert(
                        id.sym.clone(),
                        FunctionBorrowContract {
                            param_borrows: f
                                .params
                                .iter()
                                .map(|p| self.param_borrow_kind(p))
                                .collect(),
                        },
                    );
                }
            }
            Stmt::Block(b) => {
                for nested in &b.statements {
                    self.collect_function_contracts_from_stmt(nested);
                }
            }
            _ => {}
        }
    }

    fn param_borrow_kind(&self, param: &Param) -> Option<BorrowKind> {
        let ty = param.ty.as_ref()?;
        match ty.as_ref() {
            Type::Ref(_) => Some(BorrowKind::Shared),
            Type::MutRef(_) => Some(BorrowKind::Mutable),
            _ => None,
        }
    }

    fn return_borrow_kind(&self, return_type: &Option<Box<Type>>) -> Option<BorrowKind> {
        let ty = return_type.as_ref()?;
        match ty.as_ref() {
            Type::Ref(_) => Some(BorrowKind::Shared),
            Type::MutRef(_) => Some(BorrowKind::Mutable),
            _ => None,
        }
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
                if let Some(ref init) = decl.init {
                    self.check_expression(init)?;
                }

                let mut ownership = if let Some(ref init) = decl.init {
                    self.check_moveable(init)?
                } else {
                    Ownership::Owned
                };

                if let Some(Expr::Identifier(src)) = &decl.init {
                    self.move_identifier_if_needed(src)?;
                }

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
        let old_context = self.current_function_context.take();

        let mut param_borrows = HashMap::new();
        for param in &f.params {
            if let Pattern::Identifier(id) = &param.pat {
                param_borrows.insert(id.name.sym.clone(), self.param_borrow_kind(param));
            }
        }
        self.current_function_context = Some(FunctionBorrowContext {
            return_borrow: self.return_borrow_kind(&f.return_type),
            param_borrows,
        });

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
        self.current_function_context = old_context;
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
            self.check_return_borrow(arg)?;
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

        if let Expr::Identifier(src) = value {
            self.move_identifier_if_needed(src)?;
        }

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

        let param_contracts = if let Expr::Identifier(id) = &*c.callee {
            self.function_contracts
                .get(&id.sym)
                .map(|contract| contract.param_borrows.clone())
        } else {
            None
        };

        let mut temporary_borrow_counts: HashMap<String, usize> = HashMap::new();
        let mut ownership_snapshots: HashMap<String, Ownership> = HashMap::new();

        for (arg_index, arg) in c.arguments.iter().enumerate() {
            if let ExprOrSpread::Expr(e) = arg {
                let expected_borrow = param_contracts
                    .as_ref()
                    .and_then(|params| params.get(arg_index))
                    .copied()
                    .flatten();

                if let Some(kind) = expected_borrow {
                    let borrow_name = self.borrowed_argument_name(e);
                    let borrow_count_before = borrow_name
                        .as_ref()
                        .map(|name| self.active_borrows.get(name).map_or(0, Vec::len));

                    if let Some(name) = borrow_name.as_ref() {
                        if !ownership_snapshots.contains_key(name) {
                            if let Some(state) = self.locals.get(name) {
                                ownership_snapshots.insert(name.clone(), state.ownership.clone());
                            }
                        }
                    }

                    self.check_borrow_argument(e, kind)?;

                    if let (Some(name), Some(before_count)) = (borrow_name, borrow_count_before) {
                        let after_count = self.active_borrows.get(&name).map_or(0, Vec::len);
                        let added = after_count.saturating_sub(before_count);
                        if added > 0 {
                            *temporary_borrow_counts.entry(name).or_insert(0) += added;
                        }
                    }
                } else {
                    self.check_expression(e)?;
                    if let Expr::Identifier(id) = e {
                        self.move_identifier_if_needed(id)?;
                    }
                }
            }
        }

        if let Expr::Identifier(id) = &*c.callee {
            if id.sym == "process" || id.sym == "thread" {
                for arg in &c.arguments {
                    if let ExprOrSpread::Expr(e) = arg {
                        if let Some(name) = self.borrowed_argument_name(e) {
                            self.thread_access.insert(name.clone());
                            let has_mutable_borrow = self
                                .active_borrows
                                .get(&name)
                                .map(|borrows| {
                                    borrows.iter().any(|b| b.kind == BorrowKind::Mutable)
                                })
                                .unwrap_or(false);
                            if has_mutable_borrow {
                                self.errors.push(BorrowError::DataRace { variable: name });
                            }
                        }
                        self.check_thread_safe_argument(e)?;
                    }
                }
            }
        }

        for (name, count) in temporary_borrow_counts {
            if let Some(borrows) = self.active_borrows.get_mut(&name) {
                for _ in 0..count {
                    if borrows.pop().is_none() {
                        break;
                    }
                }
                if borrows.is_empty() {
                    self.active_borrows.remove(&name);
                }
            }

            if let Some(ownership) = ownership_snapshots.get(&name) {
                if let Some(state) = self.locals.get_mut(&name) {
                    state.ownership = ownership.clone();
                }
            }
        }

        Ok(())
    }

    fn check_borrow_argument(&mut self, expr: &Expr, kind: BorrowKind) -> Result<(), BorrowError> {
        match expr {
            Expr::Identifier(id) => self.check_borrow(&Expr::Identifier(id.clone()), kind),
            Expr::Ref(_) => {
                if kind == BorrowKind::Mutable {
                    self.errors.push(BorrowError::InvalidBorrow {
                        variable: "<arg>".to_string(),
                        location: expr.span().clone(),
                        message: "expected mutable borrow argument".to_string(),
                    });
                    return Ok(());
                }
                self.check_expression(expr)
            }
            Expr::MutRef(_) => self.check_expression(expr),
            _ => {
                self.errors.push(BorrowError::InvalidBorrow {
                    variable: "<arg>".to_string(),
                    location: expr.span().clone(),
                    message: "borrowed parameter expects identifier/reference argument".to_string(),
                });
                Ok(())
            }
        }
    }

    fn borrowed_argument_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Identifier(id) => Some(id.sym.clone()),
            Expr::Ref(r) => match &*r.expr {
                Expr::Identifier(id) => Some(id.sym.clone()),
                _ => None,
            },
            Expr::MutRef(r) => match &*r.expr {
                Expr::Identifier(id) => Some(id.sym.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn move_identifier_if_needed(&mut self, id: &Ident) -> Result<(), BorrowError> {
        if let Some(state) = self.locals.get_mut(&id.sym) {
            if state.is_copyable {
                return Ok(());
            }

            match state.ownership {
                Ownership::Moved => {
                    self.errors.push(BorrowError::UseAfterMove {
                        variable: id.sym.clone(),
                        location: id.span.clone(),
                    });
                }
                Ownership::Borrowed(_) => {
                    self.errors.push(BorrowError::CannotMove {
                        variable: id.sym.clone(),
                        location: id.span.clone(),
                    });
                }
                _ => {
                    state.ownership = Ownership::Moved;
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
                    if let Ownership::Borrowed(BorrowKind::Mutable) = &state.ownership {
                        self.errors.push(BorrowError::BorrowConflict {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                            message: "Cannot immutably borrow while mutable borrow exists"
                                .to_string(),
                        });
                    }
                    if let Ownership::Owned = state.ownership {
                        state.ownership = Ownership::Borrowed(kind);
                    }
                    if let Ownership::Copied = state.ownership {
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

    fn check_return_borrow(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        let Some(ctx) = &self.current_function_context else {
            return Ok(());
        };
        let Some(expected_kind) = ctx.return_borrow else {
            return Ok(());
        };

        match expr {
            Expr::Identifier(id) => {
                let Some(found_kind) = ctx.param_borrows.get(&id.sym).copied().flatten() else {
                    self.errors.push(BorrowError::LifetimeError(format!(
                        "borrowed return value '{}' does not outlive function",
                        id.sym
                    )));
                    return Ok(());
                };

                if !Self::borrow_kind_satisfies(found_kind, expected_kind) {
                    self.errors.push(BorrowError::InvalidBorrow {
                        variable: id.sym.clone(),
                        location: id.span.clone(),
                        message: "return borrow kind does not match function return type"
                            .to_string(),
                    });
                }
            }
            Expr::Ref(r) => {
                self.check_return_borrow_reference(&r.expr, BorrowKind::Shared, expected_kind)?;
            }
            Expr::MutRef(r) => {
                self.check_return_borrow_reference(&r.expr, BorrowKind::Mutable, expected_kind)?;
            }
            _ => {
                self.errors.push(BorrowError::LifetimeError(
                    "borrowed return requires identifier/reference expression".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn borrow_kind_satisfies(found: BorrowKind, expected: BorrowKind) -> bool {
        found == expected || (found == BorrowKind::Mutable && expected == BorrowKind::Shared)
    }

    fn check_return_borrow_reference(
        &mut self,
        inner: &Expr,
        found_kind: BorrowKind,
        expected_kind: BorrowKind,
    ) -> Result<(), BorrowError> {
        if !Self::borrow_kind_satisfies(found_kind, expected_kind) {
            self.errors.push(BorrowError::InvalidBorrow {
                variable: "<return>".to_string(),
                location: inner.span().clone(),
                message: "return borrow kind does not match function return type".to_string(),
            });
            return Ok(());
        }

        let Some(ctx) = &self.current_function_context else {
            return Ok(());
        };

        let Expr::Identifier(id) = inner else {
            self.errors.push(BorrowError::LifetimeError(
                "borrowed return must reference a function parameter".to_string(),
            ));
            return Ok(());
        };

        let Some(param_kind) = ctx.param_borrows.get(&id.sym).copied().flatten() else {
            self.errors.push(BorrowError::LifetimeError(format!(
                "borrowed return '{}' references non-borrowed parameter/local",
                id.sym
            )));
            return Ok(());
        };

        if !Self::borrow_kind_satisfies(param_kind, found_kind) {
            self.errors.push(BorrowError::InvalidBorrow {
                variable: id.sym.clone(),
                location: id.span.clone(),
                message: "cannot reborrow with stronger mutability than parameter".to_string(),
            });
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

    fn check_thread_safe_argument(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        if self.is_thread_safe_expression(expr) {
            return Ok(());
        }

        self.errors.push(BorrowError::ThreadSafetyViolation {
            location: expr.span().clone(),
            message: "value captured by thread/process is not thread-safe".to_string(),
        });
        Ok(())
    }

    fn is_thread_safe_expression(&self, expr: &Expr) -> bool {
        if let Some(info) = &self.type_info {
            if let Some(ty) = info.expr_types.get(expr.span()) {
                let mut visited = HashSet::new();
                return self.is_thread_safe_type(&info.type_table, *ty, &mut visited);
            }
        }

        match expr {
            Expr::Literal(lit) => matches!(
                lit,
                Literal::Number(_)
                    | Literal::Boolean(_)
                    | Literal::String(_)
                    | Literal::BigInt(_)
                    | Literal::Null(_)
                    | Literal::Undefined(_)
            ),
            Expr::Array(arr) => arr.elements.iter().all(|elem| {
                if let Some(ExprOrSpread::Expr(e)) = elem {
                    self.is_thread_safe_expression(e)
                } else {
                    true
                }
            }),
            Expr::Object(obj) => obj.properties.iter().all(|prop| match prop {
                ObjectProperty::Property(p) => match &p.value {
                    ExprOrSpread::Expr(e) => self.is_thread_safe_expression(e),
                    _ => true,
                },
                ObjectProperty::Shorthand(id) => self
                    .locals
                    .get(&id.sym)
                    .map(|v| v.is_copyable)
                    .unwrap_or(false),
                ObjectProperty::Spread(s) => self.is_thread_safe_expression(&s.argument),
                _ => false,
            }),
            Expr::Ref(_) | Expr::MutRef(_) => false,
            Expr::Identifier(id) => self
                .locals
                .get(&id.sym)
                .map(|v| v.is_copyable)
                .unwrap_or(false),
            _ => false,
        }
    }

    fn is_thread_safe_type(
        &self,
        type_table: &argon_types::TypeTable,
        ty: CheckedTypeId,
        visited: &mut HashSet<CheckedTypeId>,
    ) -> bool {
        if !visited.insert(ty) {
            return true;
        }

        match type_table.get(ty) {
            Some(
                CheckedType::Never
                | CheckedType::Boolean
                | CheckedType::Number
                | CheckedType::BigInt
                | CheckedType::String
                | CheckedType::Symbol
                | CheckedType::Null
                | CheckedType::Undefined
                | CheckedType::Void
                | CheckedType::Enum(_),
            ) => true,
            Some(CheckedType::Array(inner))
            | Some(CheckedType::Option(inner))
            | Some(CheckedType::Promise(inner))
            | Some(CheckedType::Shared(inner)) => {
                self.is_thread_safe_type(type_table, *inner, visited)
            }
            Some(CheckedType::Tuple(types))
            | Some(CheckedType::Union(types))
            | Some(CheckedType::Intersection(types)) => types
                .iter()
                .all(|inner| self.is_thread_safe_type(type_table, *inner, visited)),
            Some(CheckedType::Result(ok, err)) => {
                self.is_thread_safe_type(type_table, *ok, visited)
                    && self.is_thread_safe_type(type_table, *err, visited)
            }
            Some(CheckedType::Struct(def)) => def
                .fields
                .iter()
                .all(|field| self.is_thread_safe_type(type_table, field.ty, visited)),
            Some(CheckedType::Ref(_))
            | Some(CheckedType::MutRef(_))
            | Some(CheckedType::Class(_))
            | Some(CheckedType::Interface(_))
            | Some(CheckedType::Function(_))
            | Some(CheckedType::Object)
            | Some(CheckedType::Any)
            | Some(CheckedType::Unknown)
            | Some(CheckedType::Generic(_))
            | Some(CheckedType::TypeParam(_))
            | Some(CheckedType::Error)
            | None => false,
        }
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
    ThreadSafetyViolation {
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
            BorrowError::ThreadSafetyViolation { location, message } => {
                write!(f, "thread safety violation at {:?}: {}", location, message)
            }
        }
    }
}

impl std::error::Error for BorrowError {}
