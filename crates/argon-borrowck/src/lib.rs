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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReturnBorrowSource {
    param_index: usize,
    kind: BorrowKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionBorrowSummary {
    param_borrows: Vec<Option<BorrowKind>>,
    return_borrow: Option<BorrowKind>,
    return_sources: Vec<ReturnBorrowSource>,
    escaped_params: HashSet<usize>,
    thread_captured_params: HashSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BorrowBindingSource {
    source: String,
    kind: BorrowKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BorrowBinding {
    sources: Vec<BorrowBindingSource>,
}

#[derive(Debug, Clone)]
struct FunctionBorrowContext {
    return_borrow: Option<BorrowKind>,
    param_borrows: HashMap<String, Option<BorrowKind>>,
}

#[derive(Debug, Clone, Default)]
struct SummaryState {
    bindings: HashMap<String, Vec<ReturnBorrowSource>>,
    return_sources: HashSet<ReturnBorrowSource>,
    thread_captured_params: HashSet<usize>,
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
    function_summaries: HashMap<String, FunctionBorrowSummary>,
    current_function_context: Option<FunctionBorrowContext>,
    scope_stack: Vec<Vec<String>>,
    remaining_identifier_uses: Vec<HashMap<String, usize>>,
    borrow_bindings: HashMap<String, BorrowBinding>,
}

impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
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
            function_summaries: HashMap::new(),
            current_function_context: None,
            scope_stack: vec![Vec::new()],
            remaining_identifier_uses: Vec::new(),
            borrow_bindings: HashMap::new(),
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
        self.function_summaries.clear();
        self.collect_function_summaries(source);
        self.scope_stack.clear();
        self.scope_stack.push(Vec::new());
        self.borrow_bindings.clear();
        self.remaining_identifier_uses.clear();
        self.remaining_identifier_uses
            .push(self.count_identifier_uses_in_statements(&source.statements));

        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }

        self.check_data_race()?;

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(())
    }

    fn collect_function_summaries(&mut self, source: &SourceFile) {
        let mut functions = HashMap::new();
        self.collect_functions(source, &mut functions);
        let call_graph = self.build_call_graph(&functions);
        let components = self.condense_call_graph(&functions, &call_graph);

        self.function_summaries = functions
            .iter()
            .map(|(name, function)| {
                (
                    name.clone(),
                    FunctionBorrowSummary {
                        param_borrows: function
                            .params
                            .iter()
                            .map(|p| self.param_borrow_kind(p))
                            .collect(),
                        return_borrow: self.return_borrow_kind(&function.return_type),
                        return_sources: Vec::new(),
                        escaped_params: HashSet::new(),
                        thread_captured_params: HashSet::new(),
                    },
                )
            })
            .collect();

        for component in components {
            loop {
                let previous = self.function_summaries.clone();
                let mut changed = false;
                for name in &component {
                    if let Some(function) = functions.get(name) {
                        let summary = self.compute_function_summary(function, &previous);
                        if previous.get(name) != Some(&summary) {
                            self.function_summaries.insert(name.clone(), summary);
                            changed = true;
                        }
                    }
                }

                if !changed {
                    break;
                }
            }
        }
    }

    fn collect_functions(
        &self,
        source: &SourceFile,
        functions: &mut HashMap<String, FunctionDecl>,
    ) {
        for stmt in &source.statements {
            self.collect_functions_from_stmt(stmt, functions);
        }
    }

    fn collect_functions_from_stmt(
        &self,
        stmt: &Stmt,
        functions: &mut HashMap<String, FunctionDecl>,
    ) {
        match stmt {
            Stmt::Function(f) | Stmt::AsyncFunction(f) => {
                if let Some(id) = &f.id {
                    functions.insert(id.sym.clone(), f.clone());
                }
            }
            Stmt::Block(b) => {
                for nested in &b.statements {
                    self.collect_functions_from_stmt(nested, functions);
                }
            }
            _ => {}
        }
    }

    fn build_call_graph(
        &self,
        functions: &HashMap<String, FunctionDecl>,
    ) -> HashMap<String, HashSet<String>> {
        functions
            .iter()
            .map(|(name, function)| {
                (
                    name.clone(),
                    self.collect_called_functions(&function.body.statements, functions),
                )
            })
            .collect()
    }

    fn collect_called_functions(
        &self,
        statements: &[Stmt],
        functions: &HashMap<String, FunctionDecl>,
    ) -> HashSet<String> {
        let mut called = HashSet::new();
        for stmt in statements {
            self.collect_called_functions_from_stmt(stmt, functions, &mut called);
        }
        called
    }

    fn collect_called_functions_from_stmt(
        &self,
        stmt: &Stmt,
        functions: &HashMap<String, FunctionDecl>,
        called: &mut HashSet<String>,
    ) {
        match stmt {
            Stmt::Variable(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.collect_called_functions_from_expr(init, functions, called);
                    }
                }
            }
            Stmt::Expr(e) => self.collect_called_functions_from_expr(&e.expr, functions, called),
            Stmt::Return(r) => {
                if let Some(arg) = &r.argument {
                    self.collect_called_functions_from_expr(arg, functions, called);
                }
            }
            Stmt::Throw(t) => {
                self.collect_called_functions_from_expr(&t.argument, functions, called)
            }
            Stmt::Block(b) => {
                for nested in &b.statements {
                    self.collect_called_functions_from_stmt(nested, functions, called);
                }
            }
            Stmt::If(i) => {
                self.collect_called_functions_from_expr(&i.condition, functions, called);
                self.collect_called_functions_from_stmt(&i.consequent, functions, called);
                if let Some(alternate) = &i.alternate {
                    self.collect_called_functions_from_stmt(alternate, functions, called);
                }
            }
            Stmt::While(w) => {
                self.collect_called_functions_from_expr(&w.condition, functions, called);
                self.collect_called_functions_from_stmt(&w.body, functions, called);
            }
            Stmt::Loop(l) => self.collect_called_functions_from_stmt(&l.body, functions, called),
            Stmt::DoWhile(d) => {
                self.collect_called_functions_from_stmt(&d.body, functions, called);
                self.collect_called_functions_from_expr(&d.condition, functions, called);
            }
            Stmt::For(f) => {
                if let Some(init) = &f.init {
                    match init {
                        ForInit::Variable(v) => {
                            for decl in &v.declarations {
                                if let Some(init) = &decl.init {
                                    self.collect_called_functions_from_expr(
                                        init, functions, called,
                                    );
                                }
                            }
                        }
                        ForInit::Expr(e) => {
                            self.collect_called_functions_from_expr(e, functions, called)
                        }
                    }
                }
                if let Some(test) = &f.test {
                    self.collect_called_functions_from_expr(test, functions, called);
                }
                if let Some(update) = &f.update {
                    self.collect_called_functions_from_expr(update, functions, called);
                }
                self.collect_called_functions_from_stmt(&f.body, functions, called);
            }
            Stmt::ForIn(f) => {
                if let ForInLeft::Variable(v) = &f.left {
                    if let Some(init) = &v.init {
                        self.collect_called_functions_from_expr(init, functions, called);
                    }
                }
                self.collect_called_functions_from_expr(&f.right, functions, called);
                self.collect_called_functions_from_stmt(&f.body, functions, called);
            }
            Stmt::Switch(s) => {
                self.collect_called_functions_from_expr(&s.discriminant, functions, called);
                for case in &s.cases {
                    if let Some(test) = &case.test {
                        self.collect_called_functions_from_expr(test, functions, called);
                    }
                    for stmt in &case.consequent {
                        self.collect_called_functions_from_stmt(stmt, functions, called);
                    }
                }
            }
            Stmt::Match(m) => {
                self.collect_called_functions_from_expr(&m.discriminant, functions, called);
                for case in &m.cases {
                    self.collect_called_functions_from_expr(&case.pattern, functions, called);
                    if let Some(guard) = &case.guard {
                        self.collect_called_functions_from_expr(guard, functions, called);
                    }
                    self.collect_called_functions_from_stmt(&case.consequent, functions, called);
                }
            }
            Stmt::Try(t) => {
                for nested in &t.block.statements {
                    self.collect_called_functions_from_stmt(nested, functions, called);
                }
                if let Some(handler) = &t.handler {
                    for nested in &handler.body.statements {
                        self.collect_called_functions_from_stmt(nested, functions, called);
                    }
                }
                if let Some(finalizer) = &t.finalizer {
                    for nested in &finalizer.statements {
                        self.collect_called_functions_from_stmt(nested, functions, called);
                    }
                }
            }
            Stmt::With(w) => {
                self.collect_called_functions_from_expr(&w.object, functions, called);
                self.collect_called_functions_from_stmt(&w.body, functions, called);
            }
            Stmt::Labeled(l) => self.collect_called_functions_from_stmt(&l.body, functions, called),
            Stmt::Function(_)
            | Stmt::AsyncFunction(_)
            | Stmt::Class(_)
            | Stmt::Struct(_)
            | Stmt::Trait(_)
            | Stmt::Impl(_)
            | Stmt::Interface(_)
            | Stmt::TypeAlias(_)
            | Stmt::Enum(_)
            | Stmt::Module(_)
            | Stmt::Import(_)
            | Stmt::Export(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Empty(_)
            | Stmt::Debugger(_) => {}
        }
    }

    fn collect_called_functions_from_expr(
        &self,
        expr: &Expr,
        functions: &HashMap<String, FunctionDecl>,
        called: &mut HashSet<String>,
    ) {
        match expr {
            Expr::Assignment(a) => {
                self.collect_called_functions_from_expr(&a.right, functions, called)
            }
            Expr::Call(c) => {
                if let Expr::Identifier(id) = &*c.callee {
                    if functions.contains_key(&id.sym) {
                        called.insert(id.sym.clone());
                    }
                }
                self.collect_called_functions_from_expr(&c.callee, functions, called);
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => {
                            self.collect_called_functions_from_expr(e, functions, called)
                        }
                        ExprOrSpread::Spread(s) => {
                            self.collect_called_functions_from_expr(&s.argument, functions, called)
                        }
                    }
                }
            }
            Expr::OptionalCall(c) => {
                if let Expr::Identifier(id) = &*c.callee {
                    if functions.contains_key(&id.sym) {
                        called.insert(id.sym.clone());
                    }
                }
                self.collect_called_functions_from_expr(&c.callee, functions, called);
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => {
                            self.collect_called_functions_from_expr(e, functions, called)
                        }
                        ExprOrSpread::Spread(s) => {
                            self.collect_called_functions_from_expr(&s.argument, functions, called)
                        }
                    }
                }
            }
            Expr::Binary(b) => {
                self.collect_called_functions_from_expr(&b.left, functions, called);
                self.collect_called_functions_from_expr(&b.right, functions, called);
            }
            Expr::Unary(u) => {
                self.collect_called_functions_from_expr(&u.argument, functions, called)
            }
            Expr::Member(m) => {
                self.collect_called_functions_from_expr(&m.object, functions, called);
                self.collect_called_functions_from_expr(&m.property, functions, called);
            }
            Expr::OptionalMember(m) => {
                self.collect_called_functions_from_expr(&m.object, functions, called);
                self.collect_called_functions_from_expr(&m.property, functions, called);
            }
            Expr::Ref(r) => self.collect_called_functions_from_expr(&r.expr, functions, called),
            Expr::MutRef(r) => self.collect_called_functions_from_expr(&r.expr, functions, called),
            Expr::New(n) => {
                self.collect_called_functions_from_expr(&n.callee, functions, called);
                for arg in &n.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => {
                            self.collect_called_functions_from_expr(e, functions, called)
                        }
                        ExprOrSpread::Spread(s) => {
                            self.collect_called_functions_from_expr(&s.argument, functions, called)
                        }
                    }
                }
            }
            Expr::Conditional(c) => {
                self.collect_called_functions_from_expr(&c.test, functions, called);
                self.collect_called_functions_from_expr(&c.consequent, functions, called);
                self.collect_called_functions_from_expr(&c.alternate, functions, called);
            }
            Expr::Logical(l) => {
                self.collect_called_functions_from_expr(&l.left, functions, called);
                self.collect_called_functions_from_expr(&l.right, functions, called);
            }
            Expr::Object(o) => {
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => match &p.value {
                            ExprOrSpread::Expr(e) => {
                                self.collect_called_functions_from_expr(e, functions, called)
                            }
                            ExprOrSpread::Spread(s) => self.collect_called_functions_from_expr(
                                &s.argument,
                                functions,
                                called,
                            ),
                        },
                        ObjectProperty::Spread(s) => {
                            self.collect_called_functions_from_expr(&s.argument, functions, called)
                        }
                        ObjectProperty::Method(_)
                        | ObjectProperty::Getter(_)
                        | ObjectProperty::Setter(_)
                        | ObjectProperty::Shorthand(_) => {}
                    }
                }
            }
            Expr::Array(a) => {
                for elem in a.elements.iter().flatten() {
                    match elem {
                        ExprOrSpread::Expr(e) => {
                            self.collect_called_functions_from_expr(e, functions, called)
                        }
                        ExprOrSpread::Spread(s) => {
                            self.collect_called_functions_from_expr(&s.argument, functions, called)
                        }
                    }
                }
            }
            Expr::Spread(s) => {
                self.collect_called_functions_from_expr(&s.argument, functions, called)
            }
            Expr::Await(a) | Expr::AwaitPromised(a) => {
                self.collect_called_functions_from_expr(&a.argument, functions, called)
            }
            Expr::Yield(y) => {
                if let Some(argument) = &y.argument {
                    self.collect_called_functions_from_expr(argument, functions, called);
                }
            }
            Expr::Update(u) => {
                self.collect_called_functions_from_expr(&u.argument, functions, called)
            }
            Expr::Chain(c) => {
                for elem in &c.expressions {
                    match elem {
                        ChainElement::Call(call) => {
                            if let Expr::Identifier(id) = &*call.callee {
                                if functions.contains_key(&id.sym) {
                                    called.insert(id.sym.clone());
                                }
                            }
                            self.collect_called_functions_from_expr(
                                &call.callee,
                                functions,
                                called,
                            );
                            for arg in &call.arguments {
                                match arg {
                                    ExprOrSpread::Expr(e) => self
                                        .collect_called_functions_from_expr(e, functions, called),
                                    ExprOrSpread::Spread(s) => self
                                        .collect_called_functions_from_expr(
                                            &s.argument,
                                            functions,
                                            called,
                                        ),
                                }
                            }
                        }
                        ChainElement::OptionalCall(call) => {
                            if let Expr::Identifier(id) = &*call.callee {
                                if functions.contains_key(&id.sym) {
                                    called.insert(id.sym.clone());
                                }
                            }
                            self.collect_called_functions_from_expr(
                                &call.callee,
                                functions,
                                called,
                            );
                            for arg in &call.arguments {
                                match arg {
                                    ExprOrSpread::Expr(e) => self
                                        .collect_called_functions_from_expr(e, functions, called),
                                    ExprOrSpread::Spread(s) => self
                                        .collect_called_functions_from_expr(
                                            &s.argument,
                                            functions,
                                            called,
                                        ),
                                }
                            }
                        }
                        ChainElement::Member(m) => {
                            self.collect_called_functions_from_expr(&m.object, functions, called);
                            self.collect_called_functions_from_expr(&m.property, functions, called);
                        }
                        ChainElement::OptionalMember(m) => {
                            self.collect_called_functions_from_expr(&m.object, functions, called);
                            self.collect_called_functions_from_expr(&m.property, functions, called);
                        }
                    }
                }
            }
            Expr::Template(t) => {
                for expr in &t.expressions {
                    self.collect_called_functions_from_expr(expr, functions, called);
                }
            }
            Expr::TypeAssertion(t) => {
                self.collect_called_functions_from_expr(&t.expression, functions, called)
            }
            Expr::AsType(a) => {
                self.collect_called_functions_from_expr(&a.expression, functions, called)
            }
            Expr::NonNull(n) => {
                self.collect_called_functions_from_expr(&n.expression, functions, called)
            }
            Expr::Parenthesized(p) => {
                self.collect_called_functions_from_expr(&p.expression, functions, called)
            }
            Expr::Import(i) => {
                self.collect_called_functions_from_expr(&i.source, functions, called)
            }
            Expr::TaggedTemplate(t) => {
                self.collect_called_functions_from_expr(&t.tag, functions, called);
                for expr in &t.template.expressions {
                    self.collect_called_functions_from_expr(expr, functions, called);
                }
            }
            Expr::Identifier(_)
            | Expr::Function(_)
            | Expr::ArrowFunction(_)
            | Expr::This(_)
            | Expr::Literal(_)
            | Expr::Super(_)
            | Expr::JsxElement(_)
            | Expr::JsxFragment(_)
            | Expr::Class(_)
            | Expr::MetaProperty(_)
            | Expr::Regex(_)
            | Expr::AssignmentTargetPattern(_) => {}
        }
    }

    fn condense_call_graph(
        &self,
        functions: &HashMap<String, FunctionDecl>,
        call_graph: &HashMap<String, HashSet<String>>,
    ) -> Vec<Vec<String>> {
        let components = self.compute_sccs(functions, call_graph);
        let mut component_index = HashMap::new();
        for (index, component) in components.iter().enumerate() {
            for name in component {
                component_index.insert(name.clone(), index);
            }
        }

        let mut indegree = vec![0usize; components.len()];
        let mut edges = vec![HashSet::new(); components.len()];
        for (name, callees) in call_graph {
            let Some(&from) = component_index.get(name) else {
                continue;
            };
            for callee in callees {
                let Some(&to) = component_index.get(callee) else {
                    continue;
                };
                if from != to && edges[from].insert(to) {
                    indegree[to] += 1;
                }
            }
        }

        let mut ready: Vec<usize> = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, degree)| (*degree == 0).then_some(index))
            .collect();
        let mut ordered = Vec::new();
        while let Some(index) = ready.pop() {
            ordered.push(components[index].clone());
            let outgoing: Vec<usize> = edges[index].iter().copied().collect();
            for next in outgoing {
                indegree[next] -= 1;
                if indegree[next] == 0 {
                    ready.push(next);
                }
            }
        }

        if ordered.len() == components.len() {
            ordered
        } else {
            components
        }
    }

    fn compute_sccs(
        &self,
        functions: &HashMap<String, FunctionDecl>,
        call_graph: &HashMap<String, HashSet<String>>,
    ) -> Vec<Vec<String>> {
        #[derive(Default)]
        struct TarjanState {
            index: usize,
            stack: Vec<String>,
            on_stack: HashSet<String>,
            indices: HashMap<String, usize>,
            lowlinks: HashMap<String, usize>,
            components: Vec<Vec<String>>,
        }

        fn strong_connect(
            node: &str,
            graph: &HashMap<String, HashSet<String>>,
            state: &mut TarjanState,
        ) {
            let node_name = node.to_string();
            state.indices.insert(node_name.clone(), state.index);
            state.lowlinks.insert(node_name.clone(), state.index);
            state.index += 1;
            state.stack.push(node_name.clone());
            state.on_stack.insert(node_name.clone());

            let neighbors = graph.get(node).cloned().unwrap_or_default();
            for neighbor in neighbors {
                if !state.indices.contains_key(&neighbor) {
                    strong_connect(&neighbor, graph, state);
                    let neighbor_low = state.lowlinks.get(&neighbor).copied().unwrap_or(usize::MAX);
                    if let Some(lowlink) = state.lowlinks.get_mut(node) {
                        *lowlink = (*lowlink).min(neighbor_low);
                    }
                } else if state.on_stack.contains(&neighbor) {
                    let neighbor_index =
                        state.indices.get(&neighbor).copied().unwrap_or(usize::MAX);
                    if let Some(lowlink) = state.lowlinks.get_mut(node) {
                        *lowlink = (*lowlink).min(neighbor_index);
                    }
                }
            }

            let is_root = state.lowlinks.get(node) == state.indices.get(node);
            if is_root {
                let mut component = Vec::new();
                while let Some(entry) = state.stack.pop() {
                    state.on_stack.remove(&entry);
                    component.push(entry.clone());
                    if entry == node {
                        break;
                    }
                }
                component.sort();
                state.components.push(component);
            }
        }

        let mut state = TarjanState::default();
        let mut names: Vec<String> = functions.keys().cloned().collect();
        names.sort();
        for name in names {
            if !state.indices.contains_key(&name) {
                strong_connect(&name, call_graph, &mut state);
            }
        }

        state.components
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

    fn compute_function_summary(
        &self,
        function: &FunctionDecl,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
    ) -> FunctionBorrowSummary {
        let param_borrows: Vec<_> = function
            .params
            .iter()
            .map(|p| self.param_borrow_kind(p))
            .collect();
        let return_borrow = self.return_borrow_kind(&function.return_type);
        let param_indices: HashMap<String, usize> = function
            .params
            .iter()
            .enumerate()
            .filter_map(|(index, param)| match &param.pat {
                Pattern::Identifier(id) => Some((id.name.sym.clone(), index)),
                _ => None,
            })
            .collect();
        let mut state = SummaryState::default();
        for (name, index) in &param_indices {
            if let Some(kind) = param_borrows[*index] {
                state.bindings.insert(
                    name.clone(),
                    vec![ReturnBorrowSource {
                        param_index: *index,
                        kind,
                    }],
                );
            }
        }

        self.collect_summary_from_statements(
            &function.body.statements,
            &param_indices,
            return_borrow,
            known_summaries,
            &mut state,
        );

        let mut return_sources: Vec<_> = state.return_sources.into_iter().collect();
        return_sources.sort_by_key(|source| (source.param_index, source.kind as usize));
        let mut escaped_params = state.thread_captured_params.clone();
        escaped_params.extend(return_sources.iter().map(|source| source.param_index));

        FunctionBorrowSummary {
            param_borrows,
            return_borrow,
            return_sources,
            escaped_params,
            thread_captured_params: state.thread_captured_params,
        }
    }

    fn collect_summary_from_statements(
        &self,
        statements: &[Stmt],
        param_indices: &HashMap<String, usize>,
        expected_return_borrow: Option<BorrowKind>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        state: &mut SummaryState,
    ) {
        for stmt in statements {
            self.collect_summary_from_stmt(
                stmt,
                param_indices,
                expected_return_borrow,
                known_summaries,
                state,
            );
        }
    }

    fn collect_summary_from_stmt(
        &self,
        stmt: &Stmt,
        param_indices: &HashMap<String, usize>,
        expected_return_borrow: Option<BorrowKind>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        state: &mut SummaryState,
    ) {
        match stmt {
            Stmt::Return(r) => {
                if let Some(expected_kind) = expected_return_borrow {
                    if let Some(expr) = &r.argument {
                        for source in self.return_sources_from_expr(
                            expr,
                            expected_kind,
                            param_indices,
                            known_summaries,
                            &state.bindings,
                        ) {
                            if Self::borrow_kind_satisfies(source.kind, expected_kind) {
                                state.return_sources.insert(source);
                            }
                        }
                    }
                }
            }
            Stmt::Expr(expr_stmt) => {
                self.collect_thread_captures_from_expr(
                    &expr_stmt.expr,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                self.update_summary_binding_from_expr_stmt(
                    &expr_stmt.expr,
                    param_indices,
                    known_summaries,
                    &mut state.bindings,
                );
            }
            Stmt::Variable(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.collect_thread_captures_from_expr(
                            init,
                            param_indices,
                            known_summaries,
                            &state.bindings,
                            &mut state.thread_captured_params,
                        );
                    }
                    if let Pattern::Identifier(id) = &decl.id {
                        if let Some(init) = &decl.init {
                            if let Some(binding) = self.summary_binding_sources_from_expr(
                                init,
                                param_indices,
                                known_summaries,
                                &state.bindings,
                            ) {
                                state.bindings.insert(id.name.sym.clone(), binding);
                            } else {
                                state.bindings.remove(&id.name.sym);
                            }
                        } else {
                            state.bindings.remove(&id.name.sym);
                        }
                    }
                }
            }
            Stmt::Block(b) => {
                let mut block_state = state.clone();
                self.collect_summary_from_statements(
                    &b.statements,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut block_state,
                );
                state.return_sources.extend(block_state.return_sources);
                state
                    .thread_captured_params
                    .extend(block_state.thread_captured_params);
            }
            Stmt::If(i) => {
                self.collect_thread_captures_from_expr(
                    &i.condition,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                let mut then_state = state.clone();
                self.collect_summary_from_stmt(
                    &i.consequent,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut then_state,
                );
                let mut branch_states = vec![then_state];
                if let Some(alternate) = &i.alternate {
                    let mut else_state = state.clone();
                    self.collect_summary_from_stmt(
                        alternate,
                        param_indices,
                        expected_return_borrow,
                        known_summaries,
                        &mut else_state,
                    );
                    branch_states.push(else_state);
                }
                self.merge_summary_branch_states(state, &branch_states);
            }
            Stmt::While(w) => {
                self.collect_thread_captures_from_expr(
                    &w.condition,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                let mut loop_state = state.clone();
                self.collect_summary_from_stmt(
                    &w.body,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut loop_state,
                );
                self.merge_summary_branch_states(state, &[loop_state]);
            }
            Stmt::DoWhile(d) => {
                let mut loop_state = state.clone();
                self.collect_summary_from_stmt(
                    &d.body,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut loop_state,
                );
                self.collect_thread_captures_from_expr(
                    &d.condition,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                self.merge_summary_branch_states(state, &[loop_state]);
            }
            Stmt::For(f) => {
                let mut loop_state = state.clone();
                if let Some(init) = &f.init {
                    match init {
                        ForInit::Variable(v) => {
                            for decl in &v.declarations {
                                if let Some(init) = &decl.init {
                                    self.collect_thread_captures_from_expr(
                                        init,
                                        param_indices,
                                        known_summaries,
                                        &loop_state.bindings,
                                        &mut loop_state.thread_captured_params,
                                    );
                                }
                                if let Pattern::Identifier(id) = &decl.id {
                                    if let Some(init) = &decl.init {
                                        if let Some(binding) = self
                                            .summary_binding_sources_from_expr(
                                                init,
                                                param_indices,
                                                known_summaries,
                                                &loop_state.bindings,
                                            )
                                        {
                                            loop_state
                                                .bindings
                                                .insert(id.name.sym.clone(), binding);
                                        }
                                    }
                                }
                            }
                        }
                        ForInit::Expr(e) => self.collect_thread_captures_from_expr(
                            e,
                            param_indices,
                            known_summaries,
                            &loop_state.bindings,
                            &mut loop_state.thread_captured_params,
                        ),
                    }
                }
                if let Some(test) = &f.test {
                    self.collect_thread_captures_from_expr(
                        test,
                        param_indices,
                        known_summaries,
                        &loop_state.bindings,
                        &mut loop_state.thread_captured_params,
                    );
                }
                if let Some(update) = &f.update {
                    self.collect_thread_captures_from_expr(
                        update,
                        param_indices,
                        known_summaries,
                        &loop_state.bindings,
                        &mut loop_state.thread_captured_params,
                    );
                    self.update_summary_binding_from_expr_stmt(
                        update,
                        param_indices,
                        known_summaries,
                        &mut loop_state.bindings,
                    );
                }
                self.collect_summary_from_stmt(
                    &f.body,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut loop_state,
                );
                self.merge_summary_branch_states(state, &[loop_state]);
            }
            Stmt::ForIn(f) => {
                self.collect_thread_captures_from_expr(
                    &f.right,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                let mut loop_state = state.clone();
                self.collect_summary_from_stmt(
                    &f.body,
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut loop_state,
                );
                self.merge_summary_branch_states(state, &[loop_state]);
            }
            Stmt::Switch(s) => {
                self.collect_thread_captures_from_expr(
                    &s.discriminant,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                let mut branch_states = Vec::new();
                for case in &s.cases {
                    let mut case_state = state.clone();
                    if let Some(test) = &case.test {
                        self.collect_thread_captures_from_expr(
                            test,
                            param_indices,
                            known_summaries,
                            &case_state.bindings,
                            &mut case_state.thread_captured_params,
                        );
                    }
                    self.collect_summary_from_statements(
                        &case.consequent,
                        param_indices,
                        expected_return_borrow,
                        known_summaries,
                        &mut case_state,
                    );
                    branch_states.push(case_state);
                }
                self.merge_summary_branch_states(state, &branch_states);
            }
            Stmt::Match(m) => {
                self.collect_thread_captures_from_expr(
                    &m.discriminant,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
                let mut branch_states = Vec::new();
                for case in &m.cases {
                    let mut case_state = state.clone();
                    self.collect_thread_captures_from_expr(
                        &case.pattern,
                        param_indices,
                        known_summaries,
                        &case_state.bindings,
                        &mut case_state.thread_captured_params,
                    );
                    self.collect_summary_from_stmt(
                        &case.consequent,
                        param_indices,
                        expected_return_borrow,
                        known_summaries,
                        &mut case_state,
                    );
                    branch_states.push(case_state);
                }
                self.merge_summary_branch_states(state, &branch_states);
            }
            Stmt::Try(t) => {
                let mut branch_states = Vec::new();
                let mut block_state = state.clone();
                self.collect_summary_from_stmt(
                    &Stmt::Block(t.block.clone()),
                    param_indices,
                    expected_return_borrow,
                    known_summaries,
                    &mut block_state,
                );
                branch_states.push(block_state);
                if let Some(handler) = &t.handler {
                    let mut handler_state = state.clone();
                    self.collect_summary_from_statements(
                        &handler.body.statements,
                        param_indices,
                        expected_return_borrow,
                        known_summaries,
                        &mut handler_state,
                    );
                    branch_states.push(handler_state);
                }
                if let Some(finalizer) = &t.finalizer {
                    let mut finalizer_state = state.clone();
                    self.collect_summary_from_statements(
                        &finalizer.statements,
                        param_indices,
                        expected_return_borrow,
                        known_summaries,
                        &mut finalizer_state,
                    );
                    branch_states.push(finalizer_state);
                }
                self.merge_summary_branch_states(state, &branch_states);
            }
            Stmt::Throw(t) => {
                self.collect_thread_captures_from_expr(
                    &t.argument,
                    param_indices,
                    known_summaries,
                    &state.bindings,
                    &mut state.thread_captured_params,
                );
            }
            _ => {}
        }
    }

    fn merge_summary_branch_states(
        &self,
        state: &mut SummaryState,
        branch_states: &[SummaryState],
    ) {
        for branch in branch_states {
            state.return_sources.extend(branch.return_sources.clone());
            state
                .thread_captured_params
                .extend(branch.thread_captured_params.iter().copied());
        }

        let mut merged = state.bindings.clone();
        for branch in branch_states {
            for (name, sources) in &branch.bindings {
                let entry = merged.entry(name.clone()).or_default();
                entry.extend(sources.clone());
                Self::dedupe_return_sources(entry);
            }
        }
        state.bindings = merged;
    }

    fn return_sources_from_expr(
        &self,
        expr: &Expr,
        expected_kind: BorrowKind,
        param_indices: &HashMap<String, usize>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
    ) -> Vec<ReturnBorrowSource> {
        match expr {
            Expr::Identifier(id) => bindings
                .get(&id.sym)
                .cloned()
                .unwrap_or_else(|| self.return_sources_for_identifier(id, param_indices)),
            Expr::Ref(r) => self.return_sources_from_param_expr(
                &r.expr,
                BorrowKind::Shared,
                expected_kind,
                param_indices,
                bindings,
            ),
            Expr::MutRef(r) => self.return_sources_from_param_expr(
                &r.expr,
                BorrowKind::Mutable,
                expected_kind,
                param_indices,
                bindings,
            ),
            Expr::Call(c) => self.return_sources_from_call(
                c,
                expected_kind,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::Parenthesized(p) => self.return_sources_from_expr(
                &p.expression,
                expected_kind,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::TypeAssertion(t) => self.return_sources_from_expr(
                &t.expression,
                expected_kind,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::AsType(a) => self.return_sources_from_expr(
                &a.expression,
                expected_kind,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::NonNull(n) => self.return_sources_from_expr(
                &n.expression,
                expected_kind,
                param_indices,
                known_summaries,
                bindings,
            ),
            _ => Vec::new(),
        }
    }

    fn return_sources_for_identifier(
        &self,
        id: &Ident,
        param_indices: &HashMap<String, usize>,
    ) -> Vec<ReturnBorrowSource> {
        let Some(&param_index) = param_indices.get(&id.sym) else {
            return Vec::new();
        };
        vec![ReturnBorrowSource {
            param_index,
            kind: BorrowKind::Shared,
        }]
    }

    fn return_sources_from_param_expr(
        &self,
        expr: &Expr,
        found_kind: BorrowKind,
        expected_kind: BorrowKind,
        param_indices: &HashMap<String, usize>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
    ) -> Vec<ReturnBorrowSource> {
        if !Self::borrow_kind_satisfies(found_kind, expected_kind) {
            return Vec::new();
        }

        let sources = match expr {
            Expr::Identifier(id) => bindings
                .get(&id.sym)
                .cloned()
                .unwrap_or_else(|| self.return_sources_for_identifier(id, param_indices)),
            _ => Vec::new(),
        };

        sources
            .into_iter()
            .filter_map(|mut source| {
                if !Self::borrow_kind_satisfies(source.kind, found_kind) {
                    return None;
                }
                source.kind = found_kind;
                Some(source)
            })
            .collect()
    }

    fn return_sources_from_call(
        &self,
        call: &CallExpr,
        expected_kind: BorrowKind,
        param_indices: &HashMap<String, usize>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
    ) -> Vec<ReturnBorrowSource> {
        let Expr::Identifier(id) = &*call.callee else {
            return Vec::new();
        };
        let Some(summary) = known_summaries.get(&id.sym) else {
            return Vec::new();
        };
        let mut mapped = Vec::new();
        for source in &summary.return_sources {
            if !Self::borrow_kind_satisfies(source.kind, expected_kind) {
                continue;
            }
            let Some(ExprOrSpread::Expr(arg_expr)) = call.arguments.get(source.param_index) else {
                continue;
            };
            let mut sources = self.return_sources_from_param_expr(
                arg_expr,
                source.kind,
                expected_kind,
                param_indices,
                bindings,
            );
            mapped.append(&mut sources);
        }
        Self::dedupe_return_sources(&mut mapped);
        mapped
    }

    fn collect_thread_captures_from_expr(
        &self,
        expr: &Expr,
        param_indices: &HashMap<String, usize>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
        thread_captured_params: &mut HashSet<usize>,
    ) {
        match expr {
            Expr::Call(c) => {
                if let Expr::Identifier(id) = &*c.callee {
                    if id.sym == "thread" || id.sym == "process" {
                        for arg in &c.arguments {
                            if let ExprOrSpread::Expr(e) = arg {
                                for index in self.param_indices_for_expr(e, param_indices, bindings)
                                {
                                    thread_captured_params.insert(index);
                                }
                            }
                        }
                    } else if let Some(summary) = known_summaries.get(&id.sym) {
                        for param_index in &summary.thread_captured_params {
                            if let Some(ExprOrSpread::Expr(e)) = c.arguments.get(*param_index) {
                                for index in self.param_indices_for_expr(e, param_indices, bindings)
                                {
                                    thread_captured_params.insert(index);
                                }
                            }
                        }
                    }
                }

                self.collect_thread_captures_from_expr(
                    &c.callee,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
                for arg in &c.arguments {
                    if let ExprOrSpread::Expr(e) = arg {
                        self.collect_thread_captures_from_expr(
                            e,
                            param_indices,
                            known_summaries,
                            bindings,
                            thread_captured_params,
                        );
                    }
                }
            }
            Expr::Member(m) => {
                self.collect_thread_captures_from_expr(
                    &m.object,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
                self.collect_thread_captures_from_expr(
                    &m.property,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
            }
            Expr::Array(a) => {
                for element in &a.elements {
                    if let Some(ExprOrSpread::Expr(e)) = element {
                        self.collect_thread_captures_from_expr(
                            e,
                            param_indices,
                            known_summaries,
                            bindings,
                            thread_captured_params,
                        );
                    }
                }
            }
            Expr::Object(o) => {
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            if let ExprOrSpread::Expr(e) = &p.value {
                                self.collect_thread_captures_from_expr(
                                    e,
                                    param_indices,
                                    known_summaries,
                                    bindings,
                                    thread_captured_params,
                                );
                            }
                        }
                        ObjectProperty::Method(m) => {
                            let mut nested_state = SummaryState {
                                bindings: bindings.clone(),
                                return_sources: HashSet::new(),
                                thread_captured_params: HashSet::new(),
                            };
                            self.collect_summary_from_statements(
                                &m.value.body.statements,
                                param_indices,
                                None,
                                known_summaries,
                                &mut nested_state,
                            );
                            thread_captured_params
                                .extend(nested_state.thread_captured_params.into_iter());
                        }
                        _ => {}
                    }
                }
            }
            Expr::Await(a) | Expr::AwaitPromised(a) => self.collect_thread_captures_from_expr(
                &a.argument,
                param_indices,
                known_summaries,
                bindings,
                thread_captured_params,
            ),
            Expr::Assignment(a) => self.collect_thread_captures_from_expr(
                &a.right,
                param_indices,
                known_summaries,
                bindings,
                thread_captured_params,
            ),
            Expr::Binary(b) => {
                self.collect_thread_captures_from_expr(
                    &b.left,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
                self.collect_thread_captures_from_expr(
                    &b.right,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
            }
            Expr::Unary(u) => self.collect_thread_captures_from_expr(
                &u.argument,
                param_indices,
                known_summaries,
                bindings,
                thread_captured_params,
            ),
            Expr::Conditional(c) => {
                self.collect_thread_captures_from_expr(
                    &c.test,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
                self.collect_thread_captures_from_expr(
                    &c.consequent,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
                self.collect_thread_captures_from_expr(
                    &c.alternate,
                    param_indices,
                    known_summaries,
                    bindings,
                    thread_captured_params,
                );
            }
            _ => {}
        }
    }

    fn update_summary_binding_from_expr_stmt(
        &self,
        expr: &Expr,
        param_indices: &HashMap<String, usize>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        bindings: &mut HashMap<String, Vec<ReturnBorrowSource>>,
    ) {
        let Expr::Assignment(assignment) = expr else {
            return;
        };
        let AssignmentTarget::Simple(target) = &*assignment.left else {
            return;
        };
        let Expr::Identifier(id) = &**target else {
            return;
        };

        if let Some(new_binding) = self.summary_binding_sources_from_expr(
            &assignment.right,
            param_indices,
            known_summaries,
            bindings,
        ) {
            bindings.insert(id.sym.clone(), new_binding);
        } else {
            bindings.remove(&id.sym);
        }
    }

    fn summary_binding_sources_from_expr(
        &self,
        expr: &Expr,
        param_indices: &HashMap<String, usize>,
        known_summaries: &HashMap<String, FunctionBorrowSummary>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
    ) -> Option<Vec<ReturnBorrowSource>> {
        match expr {
            Expr::Identifier(id) => bindings.get(&id.sym).cloned().or_else(|| {
                param_indices.get(&id.sym).map(|param_index| {
                    vec![ReturnBorrowSource {
                        param_index: *param_index,
                        kind: BorrowKind::Shared,
                    }]
                })
            }),
            Expr::Ref(r) => Some(self.return_sources_from_param_expr(
                &r.expr,
                BorrowKind::Shared,
                BorrowKind::Shared,
                param_indices,
                bindings,
            )),
            Expr::MutRef(r) => Some(self.return_sources_from_param_expr(
                &r.expr,
                BorrowKind::Mutable,
                BorrowKind::Mutable,
                param_indices,
                bindings,
            )),
            Expr::Call(c) => {
                let Expr::Identifier(id) = &*c.callee else {
                    return None;
                };
                let summary = known_summaries.get(&id.sym)?;
                if summary.return_sources.is_empty() {
                    return None;
                }
                let mut result = Vec::new();
                for source in &summary.return_sources {
                    let ExprOrSpread::Expr(arg_expr) = c.arguments.get(source.param_index)? else {
                        continue;
                    };
                    let mut sources = self.return_sources_from_param_expr(
                        arg_expr,
                        source.kind,
                        source.kind,
                        param_indices,
                        bindings,
                    );
                    result.append(&mut sources);
                }
                Self::dedupe_return_sources(&mut result);
                Some(result)
            }
            Expr::Parenthesized(p) => self.summary_binding_sources_from_expr(
                &p.expression,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::TypeAssertion(t) => self.summary_binding_sources_from_expr(
                &t.expression,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::AsType(a) => self.summary_binding_sources_from_expr(
                &a.expression,
                param_indices,
                known_summaries,
                bindings,
            ),
            Expr::NonNull(n) => self.summary_binding_sources_from_expr(
                &n.expression,
                param_indices,
                known_summaries,
                bindings,
            ),
            _ => None,
        }
    }

    fn param_indices_for_expr(
        &self,
        expr: &Expr,
        param_indices: &HashMap<String, usize>,
        bindings: &HashMap<String, Vec<ReturnBorrowSource>>,
    ) -> Vec<usize> {
        match expr {
            Expr::Identifier(id) => {
                if let Some(bound) = bindings.get(&id.sym) {
                    let mut indices: Vec<_> =
                        bound.iter().map(|source| source.param_index).collect();
                    indices.sort_unstable();
                    indices.dedup();
                    indices
                } else {
                    param_indices.get(&id.sym).copied().into_iter().collect()
                }
            }
            Expr::Ref(r) => self.param_indices_for_expr(&r.expr, param_indices, bindings),
            Expr::MutRef(r) => self.param_indices_for_expr(&r.expr, param_indices, bindings),
            Expr::Parenthesized(p) => {
                self.param_indices_for_expr(&p.expression, param_indices, bindings)
            }
            Expr::TypeAssertion(t) => {
                self.param_indices_for_expr(&t.expression, param_indices, bindings)
            }
            Expr::AsType(a) => self.param_indices_for_expr(&a.expression, param_indices, bindings),
            Expr::NonNull(n) => self.param_indices_for_expr(&n.expression, param_indices, bindings),
            _ => Vec::new(),
        }
    }

    fn dedupe_return_sources(sources: &mut Vec<ReturnBorrowSource>) {
        sources.sort_by_key(|source| (source.param_index, source.kind as usize));
        sources.dedup();
    }

    fn record_scope_local(&mut self, name: String) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.push(name);
        }
    }

    fn cleanup_local(&mut self, name: &str) -> Result<(), BorrowError> {
        self.release_borrow_binding(name)?;

        if let Some(state) = self.locals.remove(name) {
            self.check_drop(name, &state)?;
        }

        Ok(())
    }

    fn register_borrow_binding(
        &mut self,
        binding_name: &str,
        init: &Option<Expr>,
    ) -> Result<(), BorrowError> {
        self.release_borrow_binding(binding_name)?;

        let Some(expr) = init else {
            return Ok(());
        };
        let Some(binding) = self.binding_from_expr(expr) else {
            return Ok(());
        };

        if Self::expr_activates_borrow_during_evaluation(expr) {
            self.borrow_bindings
                .insert(binding_name.to_string(), binding);
            Ok(())
        } else {
            self.install_borrow_binding(binding_name, binding, expr.span().clone())
        }
    }

    fn release_dead_borrow_binding(&mut self, binding_name: &str) -> Result<(), BorrowError> {
        if !self.has_remaining_identifier_uses(binding_name) {
            self.release_borrow_binding(binding_name)?;
        }
        Ok(())
    }

    fn has_remaining_identifier_uses(&self, name: &str) -> bool {
        self.remaining_identifier_uses
            .last()
            .and_then(|uses| uses.get(name))
            .copied()
            .unwrap_or(0)
            > 0
    }

    fn consume_identifier_use(&mut self, name: &str) -> Result<(), BorrowError> {
        let mut reached_zero = false;
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            if let Some(count) = remaining.get_mut(name) {
                if *count > 0 {
                    *count -= 1;
                }
                if *count == 0 {
                    remaining.remove(name);
                    reached_zero = true;
                }
            }
        }

        if reached_zero {
            self.release_borrow_binding(name)?;
        }

        Ok(())
    }

    fn release_borrow_binding(&mut self, binding_name: &str) -> Result<(), BorrowError> {
        let Some(binding) = self.borrow_bindings.remove(binding_name) else {
            return Ok(());
        };

        for source in &binding.sources {
            self.release_source_borrow(&source.source, source.kind)?;
        }
        Ok(())
    }

    fn install_borrow_binding(
        &mut self,
        binding_name: &str,
        mut binding: BorrowBinding,
        location: Span,
    ) -> Result<(), BorrowError> {
        binding.sources.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then((left.kind as usize).cmp(&(right.kind as usize)))
        });
        binding.sources.dedup();

        for source in &binding.sources {
            self.activate_binding_source_borrow(&source.source, source.kind, location.clone())?;
        }

        self.borrow_bindings
            .insert(binding_name.to_string(), binding);
        Ok(())
    }

    fn activate_binding_source_borrow(
        &mut self,
        source: &str,
        kind: BorrowKind,
        location: Span,
    ) -> Result<(), BorrowError> {
        if let Some(state) = self.locals.get_mut(source) {
            if kind == BorrowKind::Mutable {
                if let Ownership::Borrowed(BorrowKind::Shared) = &state.ownership {
                    self.errors.push(BorrowError::BorrowConflict {
                        variable: source.to_string(),
                        location: location.clone(),
                        message: "Cannot mutably borrow while shared borrow exists".to_string(),
                    });
                }
                if let Ownership::Borrowed(BorrowKind::Mutable) = &state.ownership {
                    self.errors.push(BorrowError::BorrowConflict {
                        variable: source.to_string(),
                        location: location.clone(),
                        message: "Cannot have multiple mutable borrows".to_string(),
                    });
                }
                if self.loop_scope > 0 {
                    self.errors.push(BorrowError::MutableBorrowInLoop {
                        variable: source.to_string(),
                        location: location.clone(),
                    });
                }
                state.ownership = Ownership::Borrowed(BorrowKind::Mutable);
            } else {
                if let Ownership::Borrowed(BorrowKind::Mutable) = &state.ownership {
                    self.errors.push(BorrowError::BorrowConflict {
                        variable: source.to_string(),
                        location: location.clone(),
                        message: "Cannot immutably borrow while mutable borrow exists".to_string(),
                    });
                }
                if matches!(state.ownership, Ownership::Owned | Ownership::Copied) {
                    state.ownership = Ownership::Borrowed(BorrowKind::Shared);
                }
            }
        }

        let borrow = Borrow {
            kind,
            location,
            lifetime: self.generate_lifetime(),
            from: source.to_string(),
        };
        self.active_borrows
            .entry(source.to_string())
            .or_default()
            .push(borrow);
        Ok(())
    }

    fn binding_from_expr(&self, expr: &Expr) -> Option<BorrowBinding> {
        let mut sources = self.binding_sources_from_expr(expr)?;
        sources.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then((left.kind as usize).cmp(&(right.kind as usize)))
        });
        sources.dedup();
        Some(BorrowBinding { sources })
    }

    fn binding_sources_from_expr(&self, expr: &Expr) -> Option<Vec<BorrowBindingSource>> {
        match expr {
            Expr::Identifier(id) => {
                if let Some(binding) = self.borrow_bindings.get(&id.sym) {
                    return Some(binding.sources.clone());
                }

                let kind = self
                    .current_function_context
                    .as_ref()
                    .and_then(|ctx| ctx.param_borrows.get(&id.sym))
                    .copied()
                    .flatten()?;
                Some(vec![BorrowBindingSource {
                    source: id.sym.clone(),
                    kind,
                }])
            }
            Expr::Ref(r) => self.direct_binding_sources_from_expr(&r.expr, BorrowKind::Shared),
            Expr::MutRef(r) => self.direct_binding_sources_from_expr(&r.expr, BorrowKind::Mutable),
            Expr::Call(c) => self.binding_sources_from_call(c),
            Expr::OptionalCall(c) => self.binding_sources_from_optional_call(c),
            Expr::Parenthesized(p) => self.binding_sources_from_expr(&p.expression),
            Expr::TypeAssertion(t) => self.binding_sources_from_expr(&t.expression),
            Expr::AsType(a) => self.binding_sources_from_expr(&a.expression),
            Expr::NonNull(n) => self.binding_sources_from_expr(&n.expression),
            _ => None,
        }
    }

    fn direct_binding_sources_from_expr(
        &self,
        expr: &Expr,
        kind: BorrowKind,
    ) -> Option<Vec<BorrowBindingSource>> {
        match expr {
            Expr::Identifier(id) => Some(vec![BorrowBindingSource {
                source: id.sym.clone(),
                kind,
            }]),
            Expr::Parenthesized(p) => self.direct_binding_sources_from_expr(&p.expression, kind),
            Expr::TypeAssertion(t) => self.direct_binding_sources_from_expr(&t.expression, kind),
            Expr::AsType(a) => self.direct_binding_sources_from_expr(&a.expression, kind),
            Expr::NonNull(n) => self.direct_binding_sources_from_expr(&n.expression, kind),
            _ => None,
        }
    }

    fn binding_sources_from_call(&self, call: &CallExpr) -> Option<Vec<BorrowBindingSource>> {
        let Expr::Identifier(callee) = &*call.callee else {
            return None;
        };
        let summary = self.function_summaries.get(&callee.sym)?;
        if summary.return_sources.is_empty() {
            return None;
        }

        let mut result = Vec::new();
        for source in &summary.return_sources {
            let ExprOrSpread::Expr(argument) = call.arguments.get(source.param_index)? else {
                continue;
            };
            let mut mapped = self.binding_sources_from_expr(argument)?;
            mapped.retain(|candidate| Self::borrow_kind_satisfies(candidate.kind, source.kind));
            for candidate in &mut mapped {
                candidate.kind = source.kind;
            }
            result.extend(mapped);
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn binding_sources_from_optional_call(
        &self,
        call: &OptionalCallExpr,
    ) -> Option<Vec<BorrowBindingSource>> {
        let Expr::Identifier(callee) = &*call.callee else {
            return None;
        };
        let summary = self.function_summaries.get(&callee.sym)?;
        if summary.return_sources.is_empty() {
            return None;
        }

        let mut result = Vec::new();
        for source in &summary.return_sources {
            let ExprOrSpread::Expr(argument) = call.arguments.get(source.param_index)? else {
                continue;
            };
            let mut mapped = self.binding_sources_from_expr(argument)?;
            mapped.retain(|candidate| Self::borrow_kind_satisfies(candidate.kind, source.kind));
            for candidate in &mut mapped {
                candidate.kind = source.kind;
            }
            result.extend(mapped);
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn expr_activates_borrow_during_evaluation(expr: &Expr) -> bool {
        matches!(expr, Expr::Ref(_) | Expr::MutRef(_))
    }

    fn release_source_borrow(&mut self, source: &str, kind: BorrowKind) -> Result<(), BorrowError> {
        let mut has_remaining_borrow = false;

        if let Some(borrows) = self.active_borrows.get_mut(source) {
            if let Some(index) = borrows.iter().rposition(|b| b.kind == kind) {
                borrows.remove(index);
            }
            has_remaining_borrow = !borrows.is_empty();
        }

        if !has_remaining_borrow {
            self.active_borrows.remove(source);
        }

        if let Some(state) = self.locals.get_mut(source) {
            if has_remaining_borrow {
                if let Some(kind) = self
                    .active_borrows
                    .get(source)
                    .and_then(|borrows| borrows.last().map(|borrow| borrow.kind))
                {
                    state.ownership = Ownership::Borrowed(kind);
                }
            } else if matches!(state.ownership, Ownership::Borrowed(_)) {
                state.ownership = if state.is_copyable {
                    Ownership::Copied
                } else {
                    Ownership::Owned
                };
            }
        }

        Ok(())
    }

    fn merge_branch_locals(
        &self,
        base: &HashMap<String, VariableState>,
        then_locals: &HashMap<String, VariableState>,
        else_locals: &HashMap<String, VariableState>,
    ) -> HashMap<String, VariableState> {
        self.merge_branch_locals_many(base, &[then_locals, else_locals])
    }

    fn merge_branch_locals_many(
        &self,
        base: &HashMap<String, VariableState>,
        branch_locals: &[&HashMap<String, VariableState>],
    ) -> HashMap<String, VariableState> {
        if branch_locals.is_empty() {
            return base.clone();
        }

        let mut merged = HashMap::new();

        for (name, base_state) in base {
            let mut merged_ownership = base_state.ownership.clone();
            for branch in branch_locals {
                let branch_state = branch.get(name).unwrap_or(base_state);
                merged_ownership =
                    Self::merge_ownership(&merged_ownership, &branch_state.ownership);
            }

            let mut state = base_state.clone();
            state.ownership = merged_ownership;
            merged.insert(name.clone(), state);
        }

        merged
    }

    fn merge_ownership(then_ownership: &Ownership, else_ownership: &Ownership) -> Ownership {
        if matches!(then_ownership, Ownership::Moved) || matches!(else_ownership, Ownership::Moved)
        {
            return Ownership::Moved;
        }

        let has_mut_borrow = matches!(then_ownership, Ownership::Borrowed(BorrowKind::Mutable))
            || matches!(else_ownership, Ownership::Borrowed(BorrowKind::Mutable));
        if has_mut_borrow {
            return Ownership::Borrowed(BorrowKind::Mutable);
        }

        let has_shared_borrow = matches!(then_ownership, Ownership::Borrowed(BorrowKind::Shared))
            || matches!(else_ownership, Ownership::Borrowed(BorrowKind::Shared));
        if has_shared_borrow {
            return Ownership::Borrowed(BorrowKind::Shared);
        }

        if matches!(then_ownership, Ownership::SharedOwner)
            || matches!(else_ownership, Ownership::SharedOwner)
        {
            return Ownership::SharedOwner;
        }

        if matches!(then_ownership, Ownership::Copied)
            || matches!(else_ownership, Ownership::Copied)
        {
            return Ownership::Copied;
        }

        Ownership::Owned
    }

    fn merge_branch_borrows(
        &self,
        then_borrows: &HashMap<String, Vec<Borrow>>,
        else_borrows: &HashMap<String, Vec<Borrow>>,
    ) -> HashMap<String, Vec<Borrow>> {
        self.merge_branch_borrows_many(&[then_borrows, else_borrows])
    }

    fn merge_branch_borrows_many(
        &self,
        branch_borrows: &[&HashMap<String, Vec<Borrow>>],
    ) -> HashMap<String, Vec<Borrow>> {
        let mut merged = HashMap::new();

        let mut names: HashSet<String> = HashSet::new();
        for borrows in branch_borrows {
            names.extend(borrows.keys().cloned());
        }

        for name in names {
            let mut list = Vec::new();
            for borrows in branch_borrows {
                let branch_list = borrows.get(&name).cloned().unwrap_or_default();
                if list.is_empty() {
                    list = branch_list;
                } else {
                    list = Self::merge_borrow_lists(&list, &branch_list);
                }
            }
            if !list.is_empty() {
                merged.insert(name, list);
            }
        }

        merged
    }

    fn merge_borrow_lists(then_list: &[Borrow], else_list: &[Borrow]) -> Vec<Borrow> {
        let pick_mut = then_list
            .iter()
            .find(|b| b.kind == BorrowKind::Mutable)
            .cloned()
            .or_else(|| {
                else_list
                    .iter()
                    .find(|b| b.kind == BorrowKind::Mutable)
                    .cloned()
            });

        if let Some(borrow) = pick_mut {
            return vec![borrow];
        }

        if then_list.len() >= else_list.len() {
            then_list.to_vec()
        } else {
            else_list.to_vec()
        }
    }

    fn merge_branch_bindings(
        &self,
        merged_locals: &HashMap<String, VariableState>,
        then_bindings: &HashMap<String, BorrowBinding>,
        else_bindings: &HashMap<String, BorrowBinding>,
        merged_remaining_uses: &HashMap<String, usize>,
    ) -> HashMap<String, BorrowBinding> {
        self.merge_branch_bindings_many(
            merged_locals,
            &[then_bindings, else_bindings],
            merged_remaining_uses,
        )
    }

    fn merge_branch_bindings_many(
        &self,
        merged_locals: &HashMap<String, VariableState>,
        branch_bindings: &[&HashMap<String, BorrowBinding>],
        merged_remaining_uses: &HashMap<String, usize>,
    ) -> HashMap<String, BorrowBinding> {
        let mut merged_bindings = HashMap::new();

        for (name, count) in merged_remaining_uses {
            if *count == 0 || !merged_locals.contains_key(name) {
                continue;
            }

            let mut binding: Option<BorrowBinding> = None;
            let mut conflicted = false;
            for branch in branch_bindings {
                if let Some(candidate) = branch.get(name) {
                    if let Some(existing) = &binding {
                        if existing != candidate {
                            conflicted = true;
                            break;
                        }
                    } else {
                        binding = Some(candidate.clone());
                    }
                }
            }

            if !conflicted {
                if let Some(binding) = binding {
                    merged_bindings.insert(name.clone(), binding);
                }
            }
        }

        merged_bindings
    }

    fn expired_branch_bindings(
        &self,
        then_bindings: &HashMap<String, BorrowBinding>,
        else_bindings: &HashMap<String, BorrowBinding>,
        merged_remaining_uses: &HashMap<String, usize>,
    ) -> Vec<BorrowBinding> {
        self.expired_branch_bindings_many(&[then_bindings, else_bindings], merged_remaining_uses)
    }

    fn expired_branch_bindings_many(
        &self,
        branch_bindings: &[&HashMap<String, BorrowBinding>],
        merged_remaining_uses: &HashMap<String, usize>,
    ) -> Vec<BorrowBinding> {
        let mut names: HashSet<String> = HashSet::new();
        for bindings in branch_bindings {
            names.extend(bindings.keys().cloned());
        }

        let mut seen: HashSet<Vec<(String, BorrowKind)>> = HashSet::new();
        let mut expired = Vec::new();

        for name in names {
            if merged_remaining_uses.get(&name).copied().unwrap_or(0) > 0 {
                continue;
            }

            for bindings in branch_bindings {
                if let Some(binding) = bindings.get(&name) {
                    let key: Vec<_> = binding
                        .sources
                        .iter()
                        .map(|source| (source.source.clone(), source.kind))
                        .collect();
                    if seen.insert(key) {
                        expired.push(binding.clone());
                    }
                }
            }
        }

        expired
    }

    fn merge_remaining_uses_after_if(
        &self,
        base_remaining_uses: &HashMap<String, usize>,
        then_remaining_uses: &HashMap<String, usize>,
        else_remaining_uses: &HashMap<String, usize>,
    ) -> HashMap<String, usize> {
        self.merge_remaining_uses_after_exclusive_branches(
            base_remaining_uses,
            &[then_remaining_uses, else_remaining_uses],
        )
    }

    fn merge_remaining_uses_after_exclusive_branches(
        &self,
        base_remaining_uses: &HashMap<String, usize>,
        branch_remaining_uses: &[&HashMap<String, usize>],
    ) -> HashMap<String, usize> {
        if branch_remaining_uses.is_empty() {
            return base_remaining_uses.clone();
        }

        let mut merged = HashMap::new();

        let mut names: HashSet<String> = HashSet::new();
        names.extend(base_remaining_uses.keys().cloned());
        for branch in branch_remaining_uses {
            names.extend(branch.keys().cloned());
        }

        let branch_count = branch_remaining_uses.len();
        for name in names {
            let base = base_remaining_uses.get(&name).copied().unwrap_or(0);
            let mut total_remaining = 0usize;
            for branch in branch_remaining_uses {
                let count = branch.get(&name).copied().unwrap_or(0);
                total_remaining = total_remaining.saturating_add(count);
            }

            let baseline_overlap = base.saturating_mul(branch_count.saturating_sub(1));
            let merged_count = total_remaining.saturating_sub(baseline_overlap);
            if merged_count > 0 {
                merged.insert(name, merged_count);
            }
        }

        merged
    }

    fn is_wildcard_match_pattern(pattern: &Expr) -> bool {
        matches!(pattern, Expr::Identifier(id) if id.sym == "_")
    }

    #[allow(clippy::too_many_arguments)]
    fn merge_branch_outcomes(
        &mut self,
        base_locals: &HashMap<String, VariableState>,
        base_borrows: &HashMap<String, Vec<Borrow>>,
        base_bindings: &HashMap<String, BorrowBinding>,
        base_remaining_uses: &HashMap<String, usize>,
        branch_locals: &[HashMap<String, VariableState>],
        branch_borrows: &[HashMap<String, Vec<Borrow>>],
        branch_bindings: &[HashMap<String, BorrowBinding>],
        branch_remaining_uses: &[HashMap<String, usize>],
    ) -> Result<(), BorrowError> {
        if branch_locals.is_empty() {
            self.locals = base_locals.clone();
            self.active_borrows = base_borrows.clone();
            self.borrow_bindings = base_bindings.clone();
            if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
                *remaining = base_remaining_uses.clone();
            }
            return Ok(());
        }

        let branch_local_refs: Vec<&HashMap<String, VariableState>> =
            branch_locals.iter().collect();
        let branch_borrow_refs: Vec<&HashMap<String, Vec<Borrow>>> =
            branch_borrows.iter().collect();
        let branch_binding_refs: Vec<&HashMap<String, BorrowBinding>> =
            branch_bindings.iter().collect();
        let branch_remaining_refs: Vec<&HashMap<String, usize>> =
            branch_remaining_uses.iter().collect();

        self.locals = self.merge_branch_locals_many(base_locals, &branch_local_refs);
        let merged_remaining_uses = self.merge_remaining_uses_after_exclusive_branches(
            base_remaining_uses,
            &branch_remaining_refs,
        );
        let expired_branch_bindings =
            self.expired_branch_bindings_many(&branch_binding_refs, &merged_remaining_uses);
        let merged_bindings = self.merge_branch_bindings_many(
            &self.locals,
            &branch_binding_refs,
            &merged_remaining_uses,
        );
        self.active_borrows = self.merge_branch_borrows_many(&branch_borrow_refs);
        self.borrow_bindings = merged_bindings;
        for binding in expired_branch_bindings {
            for source in binding.sources {
                self.release_source_borrow(&source.source, source.kind)?;
            }
        }
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = merged_remaining_uses;
        }

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn evaluate_match_case_branch(
        &mut self,
        case: &MatchCase,
        base_locals: &HashMap<String, VariableState>,
        base_borrows: &HashMap<String, Vec<Borrow>>,
        base_bindings: &HashMap<String, BorrowBinding>,
        base_remaining_uses: &HashMap<String, usize>,
    ) -> Result<
        (
            HashMap<String, VariableState>,
            HashMap<String, Vec<Borrow>>,
            HashMap<String, BorrowBinding>,
            HashMap<String, usize>,
        ),
        BorrowError,
    > {
        self.locals = base_locals.clone();
        self.active_borrows = base_borrows.clone();
        self.borrow_bindings = base_bindings.clone();
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = base_remaining_uses.clone();
        }

        self.check_expression(&case.pattern)?;
        if let Some(guard) = &case.guard {
            self.check_expression(guard)?;
        }
        self.check_statement(&case.consequent)?;

        Ok((
            self.locals.clone(),
            self.active_borrows.clone(),
            self.borrow_bindings.clone(),
            self.remaining_identifier_uses
                .last()
                .cloned()
                .unwrap_or_default(),
        ))
    }

    #[allow(clippy::type_complexity)]
    fn evaluate_switch_case_branch(
        &mut self,
        case: &SwitchCase,
        base_locals: &HashMap<String, VariableState>,
        base_borrows: &HashMap<String, Vec<Borrow>>,
        base_bindings: &HashMap<String, BorrowBinding>,
        base_remaining_uses: &HashMap<String, usize>,
    ) -> Result<
        (
            HashMap<String, VariableState>,
            HashMap<String, Vec<Borrow>>,
            HashMap<String, BorrowBinding>,
            HashMap<String, usize>,
        ),
        BorrowError,
    > {
        self.locals = base_locals.clone();
        self.active_borrows = base_borrows.clone();
        self.borrow_bindings = base_bindings.clone();
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = base_remaining_uses.clone();
        }

        if let Some(test) = &case.test {
            self.check_expression(test)?;
        }
        for stmt in &case.consequent {
            self.check_statement(stmt)?;
        }

        Ok((
            self.locals.clone(),
            self.active_borrows.clone(),
            self.borrow_bindings.clone(),
            self.remaining_identifier_uses
                .last()
                .cloned()
                .unwrap_or_default(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn push_base_branch_outcome(
        branch_locals: &mut Vec<HashMap<String, VariableState>>,
        branch_borrows: &mut Vec<HashMap<String, Vec<Borrow>>>,
        branch_bindings: &mut Vec<HashMap<String, BorrowBinding>>,
        branch_remaining_uses: &mut Vec<HashMap<String, usize>>,
        base_locals: &HashMap<String, VariableState>,
        base_borrows: &HashMap<String, Vec<Borrow>>,
        base_bindings: &HashMap<String, BorrowBinding>,
        base_remaining_uses: &HashMap<String, usize>,
    ) {
        branch_locals.push(base_locals.clone());
        branch_borrows.push(base_borrows.clone());
        branch_bindings.push(base_bindings.clone());
        branch_remaining_uses.push(base_remaining_uses.clone());
    }

    fn count_identifier_uses_in_statements(&self, statements: &[Stmt]) -> HashMap<String, usize> {
        let mut uses = HashMap::new();
        for stmt in statements {
            self.count_identifier_uses_in_statement(stmt, &mut uses);
        }
        uses
    }

    fn count_identifier_uses_in_statement(&self, stmt: &Stmt, uses: &mut HashMap<String, usize>) {
        match stmt {
            Stmt::Variable(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.count_identifier_uses_in_expr(init, uses);
                    }
                }
            }
            Stmt::Expr(e) => self.count_identifier_uses_in_expr(&e.expr, uses),
            Stmt::Return(r) => {
                if let Some(arg) = &r.argument {
                    self.count_identifier_uses_in_expr(arg, uses);
                }
            }
            Stmt::Throw(t) => self.count_identifier_uses_in_expr(&t.argument, uses),
            Stmt::If(i) => {
                self.count_identifier_uses_in_expr(&i.condition, uses);
                self.count_identifier_uses_in_statement(&i.consequent, uses);
                if let Some(alt) = &i.alternate {
                    self.count_identifier_uses_in_statement(alt, uses);
                }
            }
            Stmt::While(w) => {
                self.count_identifier_uses_in_expr(&w.condition, uses);
                self.count_identifier_uses_in_statement(&w.body, uses);
            }
            Stmt::Loop(l) => self.count_identifier_uses_in_statement(&l.body, uses),
            Stmt::DoWhile(d) => {
                self.count_identifier_uses_in_statement(&d.body, uses);
                self.count_identifier_uses_in_expr(&d.condition, uses);
            }
            Stmt::For(f) => {
                if let Some(init) = &f.init {
                    match init {
                        ForInit::Variable(v) => {
                            for decl in &v.declarations {
                                if let Some(init) = &decl.init {
                                    self.count_identifier_uses_in_expr(init, uses);
                                }
                            }
                        }
                        ForInit::Expr(e) => self.count_identifier_uses_in_expr(e, uses),
                    }
                }
                if let Some(test) = &f.test {
                    self.count_identifier_uses_in_expr(test, uses);
                }
                if let Some(update) = &f.update {
                    self.count_identifier_uses_in_expr(update, uses);
                }
                self.count_identifier_uses_in_statement(&f.body, uses);
            }
            Stmt::ForIn(f) => {
                if let ForInLeft::Variable(v) = &f.left {
                    if let Some(init) = &v.init {
                        self.count_identifier_uses_in_expr(init, uses);
                    }
                }
                self.count_identifier_uses_in_expr(&f.right, uses);
                self.count_identifier_uses_in_statement(&f.body, uses);
            }
            Stmt::Switch(s) => {
                self.count_identifier_uses_in_expr(&s.discriminant, uses);
                for case in &s.cases {
                    if let Some(test) = &case.test {
                        self.count_identifier_uses_in_expr(test, uses);
                    }
                    for stmt in &case.consequent {
                        self.count_identifier_uses_in_statement(stmt, uses);
                    }
                }
            }
            Stmt::Try(t) => {
                self.count_identifier_uses_in_block(&t.block, uses);
                if let Some(handler) = &t.handler {
                    self.count_identifier_uses_in_block(&handler.body, uses);
                }
                if let Some(finalizer) = &t.finalizer {
                    self.count_identifier_uses_in_block(finalizer, uses);
                }
            }
            Stmt::Match(m) => {
                self.count_identifier_uses_in_expr(&m.discriminant, uses);
                for case in &m.cases {
                    self.count_identifier_uses_in_expr(&case.pattern, uses);
                    if let Some(guard) = &case.guard {
                        self.count_identifier_uses_in_expr(guard, uses);
                    }
                    self.count_identifier_uses_in_statement(&case.consequent, uses);
                }
            }
            Stmt::Block(b) => self.count_identifier_uses_in_block(b, uses),
            Stmt::With(w) => {
                self.count_identifier_uses_in_expr(&w.object, uses);
                self.count_identifier_uses_in_statement(&w.body, uses);
            }
            Stmt::Labeled(l) => self.count_identifier_uses_in_statement(&l.body, uses),
            Stmt::Function(_)
            | Stmt::AsyncFunction(_)
            | Stmt::Class(_)
            | Stmt::Struct(_)
            | Stmt::Trait(_)
            | Stmt::Impl(_)
            | Stmt::Interface(_)
            | Stmt::TypeAlias(_)
            | Stmt::Enum(_)
            | Stmt::Module(_)
            | Stmt::Import(_)
            | Stmt::Export(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Empty(_)
            | Stmt::Debugger(_) => {}
        }
    }

    fn count_identifier_uses_in_block(&self, block: &BlockStmt, uses: &mut HashMap<String, usize>) {
        for stmt in &block.statements {
            self.count_identifier_uses_in_statement(stmt, uses);
        }
    }

    fn count_identifier_uses_in_expr(&self, expr: &Expr, uses: &mut HashMap<String, usize>) {
        match expr {
            Expr::Identifier(id) => {
                *uses.entry(id.sym.clone()).or_insert(0) += 1;
            }
            Expr::Assignment(a) => {
                self.count_identifier_uses_in_assignment_target(&a.left, uses);
                self.count_identifier_uses_in_expr(&a.right, uses);
            }
            Expr::Call(c) => {
                self.count_identifier_uses_in_expr(&c.callee, uses);
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => self.count_identifier_uses_in_expr(e, uses),
                        ExprOrSpread::Spread(s) => {
                            self.count_identifier_uses_in_expr(&s.argument, uses)
                        }
                    }
                }
            }
            Expr::Binary(b) => {
                self.count_identifier_uses_in_expr(&b.left, uses);
                self.count_identifier_uses_in_expr(&b.right, uses);
            }
            Expr::Unary(u) => self.count_identifier_uses_in_expr(&u.argument, uses),
            Expr::Member(m) => {
                self.count_identifier_uses_in_expr(&m.object, uses);
                if m.computed {
                    self.count_identifier_uses_in_expr(&m.property, uses);
                }
            }
            Expr::Ref(r) => self.count_identifier_uses_in_expr(&r.expr, uses),
            Expr::MutRef(r) => self.count_identifier_uses_in_expr(&r.expr, uses),
            Expr::New(n) => {
                self.count_identifier_uses_in_expr(&n.callee, uses);
                for arg in &n.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => self.count_identifier_uses_in_expr(e, uses),
                        ExprOrSpread::Spread(s) => {
                            self.count_identifier_uses_in_expr(&s.argument, uses)
                        }
                    }
                }
            }
            Expr::Conditional(c) => {
                self.count_identifier_uses_in_expr(&c.test, uses);
                self.count_identifier_uses_in_expr(&c.consequent, uses);
                self.count_identifier_uses_in_expr(&c.alternate, uses);
            }
            Expr::Logical(l) => {
                self.count_identifier_uses_in_expr(&l.left, uses);
                self.count_identifier_uses_in_expr(&l.right, uses);
            }
            Expr::Object(o) => {
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            if p.computed {
                                self.count_identifier_uses_in_expr(&p.key, uses);
                            }
                            match &p.value {
                                ExprOrSpread::Expr(e) => {
                                    self.count_identifier_uses_in_expr(e, uses)
                                }
                                ExprOrSpread::Spread(s) => {
                                    self.count_identifier_uses_in_expr(&s.argument, uses)
                                }
                            }
                        }
                        ObjectProperty::Shorthand(id) => {
                            *uses.entry(id.sym.clone()).or_insert(0) += 1;
                        }
                        ObjectProperty::Spread(s) => {
                            self.count_identifier_uses_in_expr(&s.argument, uses)
                        }
                        ObjectProperty::Method(_)
                        | ObjectProperty::Getter(_)
                        | ObjectProperty::Setter(_) => {}
                    }
                }
            }
            Expr::Array(a) => {
                for elem in a.elements.iter().flatten() {
                    match elem {
                        ExprOrSpread::Expr(e) => self.count_identifier_uses_in_expr(e, uses),
                        ExprOrSpread::Spread(s) => {
                            self.count_identifier_uses_in_expr(&s.argument, uses)
                        }
                    }
                }
            }
            Expr::Function(_)
            | Expr::ArrowFunction(_)
            | Expr::This(_)
            | Expr::Literal(_)
            | Expr::Super(_) => {}
            Expr::Spread(s) => self.count_identifier_uses_in_expr(&s.argument, uses),
            Expr::Await(a) | Expr::AwaitPromised(a) => {
                self.count_identifier_uses_in_expr(&a.argument, uses)
            }
            Expr::Yield(y) => {
                if let Some(arg) = &y.argument {
                    self.count_identifier_uses_in_expr(arg, uses);
                }
            }
            Expr::Update(u) => self.count_identifier_uses_in_expr(&u.argument, uses),
            Expr::Chain(c) => {
                for elem in &c.expressions {
                    match elem {
                        ChainElement::Call(call) => {
                            self.count_identifier_uses_in_expr(&call.callee, uses);
                            for arg in &call.arguments {
                                match arg {
                                    ExprOrSpread::Expr(e) => {
                                        self.count_identifier_uses_in_expr(e, uses)
                                    }
                                    ExprOrSpread::Spread(s) => {
                                        self.count_identifier_uses_in_expr(&s.argument, uses)
                                    }
                                }
                            }
                        }
                        ChainElement::Member(m) => {
                            self.count_identifier_uses_in_expr(&m.object, uses);
                            if m.computed {
                                self.count_identifier_uses_in_expr(&m.property, uses);
                            }
                        }
                        ChainElement::OptionalCall(c) => {
                            self.count_identifier_uses_in_expr(&c.callee, uses);
                            for arg in &c.arguments {
                                match arg {
                                    ExprOrSpread::Expr(e) => {
                                        self.count_identifier_uses_in_expr(e, uses)
                                    }
                                    ExprOrSpread::Spread(s) => {
                                        self.count_identifier_uses_in_expr(&s.argument, uses)
                                    }
                                }
                            }
                        }
                        ChainElement::OptionalMember(m) => {
                            self.count_identifier_uses_in_expr(&m.object, uses);
                            if m.computed {
                                self.count_identifier_uses_in_expr(&m.property, uses);
                            }
                        }
                    }
                }
            }
            Expr::OptionalCall(c) => {
                self.count_identifier_uses_in_expr(&c.callee, uses);
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => self.count_identifier_uses_in_expr(e, uses),
                        ExprOrSpread::Spread(s) => {
                            self.count_identifier_uses_in_expr(&s.argument, uses)
                        }
                    }
                }
            }
            Expr::OptionalMember(m) => {
                self.count_identifier_uses_in_expr(&m.object, uses);
                if m.computed {
                    self.count_identifier_uses_in_expr(&m.property, uses);
                }
            }
            Expr::Template(t) => {
                for expr in &t.expressions {
                    self.count_identifier_uses_in_expr(expr, uses);
                }
            }
            Expr::TypeAssertion(t) => self.count_identifier_uses_in_expr(&t.expression, uses),
            Expr::AsType(a) => self.count_identifier_uses_in_expr(&a.expression, uses),
            Expr::NonNull(n) => self.count_identifier_uses_in_expr(&n.expression, uses),
            Expr::Parenthesized(p) => self.count_identifier_uses_in_expr(&p.expression, uses),
            Expr::Import(i) => self.count_identifier_uses_in_expr(&i.source, uses),
            Expr::TaggedTemplate(t) => {
                self.count_identifier_uses_in_expr(&t.tag, uses);
                for expr in &t.template.expressions {
                    self.count_identifier_uses_in_expr(expr, uses);
                }
            }
            Expr::JsxElement(_)
            | Expr::JsxFragment(_)
            | Expr::Class(_)
            | Expr::MetaProperty(_)
            | Expr::Regex(_)
            | Expr::AssignmentTargetPattern(_) => {}
        }
    }

    fn count_identifier_uses_in_assignment_target(
        &self,
        target: &AssignmentTarget,
        uses: &mut HashMap<String, usize>,
    ) {
        match target {
            AssignmentTarget::Simple(expr) => {
                if let Expr::Member(m) = &**expr {
                    self.count_identifier_uses_in_expr(&m.object, uses);
                    if m.computed {
                        self.count_identifier_uses_in_expr(&m.property, uses);
                    }
                }
            }
            AssignmentTarget::Member(member) => {
                self.count_identifier_uses_in_expr(&member.object, uses);
                if member.computed {
                    self.count_identifier_uses_in_expr(&member.property, uses);
                }
            }
            AssignmentTarget::Pattern(_) => {}
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
        self.scope_stack.push(Vec::new());

        for stmt in &b.statements {
            self.check_statement(stmt)?;
        }

        if let Some(scope_names) = self.scope_stack.pop() {
            for name in scope_names {
                self.cleanup_local(&name)?;
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
                self.record_scope_local(id.name.sym.clone());
                self.register_borrow_binding(&id.name.sym, &decl.init)?;
                self.release_dead_borrow_binding(&id.name.sym)?;
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

        if let Some(expr) = init {
            if let Some(binding) = self.binding_from_expr(expr) {
                return binding
                    .sources
                    .iter()
                    .all(|source| source.kind == BorrowKind::Shared);
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
                Expr::Ref(_) => true,
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
        let old_scopes = std::mem::take(&mut self.scope_stack);
        let old_remaining_uses = std::mem::take(&mut self.remaining_identifier_uses);
        let old_borrow_bindings = std::mem::take(&mut self.borrow_bindings);

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
        self.scope_stack = vec![Vec::new()];
        self.remaining_identifier_uses =
            vec![self.count_identifier_uses_in_statements(&f.body.statements)];

        for param in &f.params {
            if let Pattern::Identifier(id) = &param.pat {
                let lifetime = self.generate_lifetime();
                self.locals.insert(
                    id.name.sym.clone(),
                    VariableState {
                        name: id.name.sym.clone(),
                        ownership: if self.param_borrow_kind(param).is_some() {
                            Ownership::Copied
                        } else {
                            Ownership::Owned
                        },
                        borrows: Vec::new(),
                        lifetime: Some(lifetime),
                        is_copyable: self.param_borrow_kind(param) == Some(BorrowKind::Shared),
                        drop_scope: 0,
                    },
                );
                self.record_scope_local(id.name.sym.clone());
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
        self.scope_stack = old_scopes;
        self.remaining_identifier_uses = old_remaining_uses;
        self.borrow_bindings = old_borrow_bindings;
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

        let base_locals = self.locals.clone();
        let base_borrows = self.active_borrows.clone();
        let base_bindings = self.borrow_bindings.clone();
        let base_remaining_uses = self
            .remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default();

        self.locals = base_locals.clone();
        self.active_borrows = base_borrows.clone();
        self.borrow_bindings = base_bindings.clone();
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = base_remaining_uses.clone();
        }
        self.check_statement(&i.consequent)?;
        let then_locals = self.locals.clone();
        let then_borrows = self.active_borrows.clone();
        let then_bindings = self.borrow_bindings.clone();
        let then_remaining_uses = self
            .remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default();

        self.locals = base_locals.clone();
        self.active_borrows = base_borrows.clone();
        self.borrow_bindings = base_bindings.clone();
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = base_remaining_uses.clone();
        }
        if let Some(ref alt) = i.alternate {
            self.check_statement(alt)?;
        }
        let else_locals = self.locals.clone();
        let else_borrows = self.active_borrows.clone();
        let else_bindings = self.borrow_bindings.clone();
        let else_remaining_uses = self
            .remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default();

        self.locals = self.merge_branch_locals(&base_locals, &then_locals, &else_locals);
        let merged_remaining_uses = self.merge_remaining_uses_after_if(
            &base_remaining_uses,
            &then_remaining_uses,
            &else_remaining_uses,
        );
        let expired_branch_bindings =
            self.expired_branch_bindings(&then_bindings, &else_bindings, &merged_remaining_uses);
        let merged_bindings = self.merge_branch_bindings(
            &self.locals,
            &then_bindings,
            &else_bindings,
            &merged_remaining_uses,
        );
        self.active_borrows = self.merge_branch_borrows(&then_borrows, &else_borrows);
        self.borrow_bindings = merged_bindings;
        for binding in expired_branch_bindings {
            for source in binding.sources {
                self.release_source_borrow(&source.source, source.kind)?;
            }
        }
        if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
            *remaining = merged_remaining_uses;
        }

        Ok(())
    }

    fn current_remaining_uses(&self) -> HashMap<String, usize> {
        self.remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default()
    }

    fn merge_remaining_uses_after_loop(
        &self,
        base_remaining_uses: &HashMap<String, usize>,
        iter_remaining_uses: &HashMap<String, usize>,
    ) -> HashMap<String, usize> {
        let mut merged = HashMap::new();

        let mut names: HashSet<String> = HashSet::new();
        names.extend(base_remaining_uses.keys().cloned());
        names.extend(iter_remaining_uses.keys().cloned());

        for name in names {
            let base = base_remaining_uses.get(&name).copied().unwrap_or(0);
            let iter = iter_remaining_uses.get(&name).copied().unwrap_or(0);
            let count = base.max(iter);
            if count > 0 {
                merged.insert(name, count);
            }
        }

        merged
    }

    fn loop_state_fingerprint(
        locals: &HashMap<String, VariableState>,
        borrows: &HashMap<String, Vec<Borrow>>,
        bindings: &HashMap<String, BorrowBinding>,
        remaining: &HashMap<String, usize>,
    ) -> String {
        let mut local_parts: Vec<String> = locals
            .iter()
            .map(|(name, state)| format!("{}:{:?}", name, state.ownership))
            .collect();
        local_parts.sort();

        let mut borrow_parts: Vec<String> = borrows
            .iter()
            .map(|(name, borrows)| {
                let mut kinds: Vec<String> = borrows
                    .iter()
                    .map(|b| match b.kind {
                        BorrowKind::Shared => "S".to_string(),
                        BorrowKind::Mutable => "M".to_string(),
                    })
                    .collect();
                kinds.sort();
                format!("{}:{}", name, kinds.join(","))
            })
            .collect();
        borrow_parts.sort();

        let mut binding_parts: Vec<String> = bindings
            .iter()
            .map(|(name, binding)| {
                let sources: Vec<String> = binding
                    .sources
                    .iter()
                    .map(|source| format!("{}:{:?}", source.source, source.kind))
                    .collect();
                format!("{}:{}", name, sources.join("+"))
            })
            .collect();
        binding_parts.sort();

        let mut remaining_parts: Vec<String> = remaining
            .iter()
            .map(|(name, count)| format!("{}:{}", name, count))
            .collect();
        remaining_parts.sort();

        format!(
            "{}|{}|{}|{}",
            local_parts.join(";"),
            borrow_parts.join(";"),
            binding_parts.join(";"),
            remaining_parts.join(";")
        )
    }

    fn analyze_loop_fixed_point<F>(&mut self, mut run_iteration: F) -> Result<(), BorrowError>
    where
        F: FnMut(&mut BorrowChecker) -> Result<(), BorrowError>,
    {
        const MAX_LOOP_FIXPOINT_ITERS: usize = 8;

        let base_locals = self.locals.clone();
        let base_borrows = self.active_borrows.clone();
        let base_bindings = self.borrow_bindings.clone();
        let base_remaining_uses = self.current_remaining_uses();

        let mut candidate_locals = base_locals.clone();
        let mut candidate_borrows = base_borrows.clone();
        let mut candidate_bindings = base_bindings.clone();
        let mut candidate_remaining_uses = base_remaining_uses.clone();
        let mut last_fingerprint: Option<String> = None;

        for _ in 0..MAX_LOOP_FIXPOINT_ITERS {
            self.locals = candidate_locals.clone();
            self.active_borrows = candidate_borrows.clone();
            self.borrow_bindings = candidate_bindings.clone();
            if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
                *remaining = candidate_remaining_uses.clone();
            }

            run_iteration(self)?;

            let iter_locals = self.locals.clone();
            let iter_borrows = self.active_borrows.clone();
            let iter_bindings = self.borrow_bindings.clone();
            let iter_remaining_uses = self.current_remaining_uses();

            self.locals = self.merge_branch_locals(&base_locals, &iter_locals, &base_locals);
            let merged_remaining_uses =
                self.merge_remaining_uses_after_loop(&base_remaining_uses, &iter_remaining_uses);
            let expired_branch_bindings = self.expired_branch_bindings(
                &iter_bindings,
                &base_bindings,
                &merged_remaining_uses,
            );
            let merged_bindings = self.merge_branch_bindings(
                &self.locals,
                &iter_bindings,
                &base_bindings,
                &merged_remaining_uses,
            );
            self.active_borrows = self.merge_branch_borrows(&iter_borrows, &base_borrows);
            self.borrow_bindings = merged_bindings;
            for binding in expired_branch_bindings {
                for source in binding.sources {
                    self.release_source_borrow(&source.source, source.kind)?;
                }
            }
            if let Some(remaining) = self.remaining_identifier_uses.last_mut() {
                *remaining = merged_remaining_uses;
            }

            let fingerprint = Self::loop_state_fingerprint(
                &self.locals,
                &self.active_borrows,
                &self.borrow_bindings,
                &self.current_remaining_uses(),
            );
            if last_fingerprint.as_ref() == Some(&fingerprint) {
                break;
            }

            last_fingerprint = Some(fingerprint);
            candidate_locals = self.locals.clone();
            candidate_borrows = self.active_borrows.clone();
            candidate_bindings = self.borrow_bindings.clone();
            candidate_remaining_uses = self.current_remaining_uses();
        }

        Ok(())
    }

    fn check_while(&mut self, w: &WhileStmt) -> Result<(), BorrowError> {
        self.check_expression(&w.condition)?;
        self.loop_scope += 1;
        let result = self.analyze_loop_fixed_point(|checker| {
            checker.check_statement(&w.body)?;
            checker.check_expression(&w.condition)?;
            Ok(())
        });
        self.loop_scope -= 1;
        result
    }

    fn check_loop(&mut self, l: &LoopStmt) -> Result<(), BorrowError> {
        self.loop_scope += 1;
        let result = self.analyze_loop_fixed_point(|checker| checker.check_statement(&l.body));
        self.loop_scope -= 1;
        result
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

        self.loop_scope += 1;
        let result = self.analyze_loop_fixed_point(|checker| {
            checker.check_statement(&f.body)?;
            if let Some(ref update) = f.update {
                checker.check_expression(update)?;
            }
            if let Some(ref test) = f.test {
                checker.check_expression(test)?;
            }
            Ok(())
        });
        self.loop_scope -= 1;
        result
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
                    self.record_scope_local(id.name.sym.clone());
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

        self.loop_scope += 1;
        let result = self.analyze_loop_fixed_point(|checker| checker.check_statement(&f.body));
        self.loop_scope -= 1;
        result
    }

    fn check_return(&mut self, r: &ReturnStmt) -> Result<(), BorrowError> {
        if let Some(ref arg) = r.argument {
            self.check_return_borrow(arg)?;
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
        let base_locals = self.locals.clone();
        let base_borrows = self.active_borrows.clone();
        let base_bindings = self.borrow_bindings.clone();
        let base_remaining_uses = self
            .remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default();

        let mut branch_locals = Vec::new();
        let mut branch_borrows = Vec::new();
        let mut branch_bindings = Vec::new();
        let mut branch_remaining_uses = Vec::new();
        let mut has_wildcard_case = false;

        for case in &m.cases {
            has_wildcard_case |= Self::is_wildcard_match_pattern(&case.pattern);
            let (locals, borrows, bindings, remaining) = self.evaluate_match_case_branch(
                case,
                &base_locals,
                &base_borrows,
                &base_bindings,
                &base_remaining_uses,
            )?;
            branch_locals.push(locals);
            branch_borrows.push(borrows);
            branch_bindings.push(bindings);
            branch_remaining_uses.push(remaining);
        }

        if !has_wildcard_case {
            Self::push_base_branch_outcome(
                &mut branch_locals,
                &mut branch_borrows,
                &mut branch_bindings,
                &mut branch_remaining_uses,
                &base_locals,
                &base_borrows,
                &base_bindings,
                &base_remaining_uses,
            );
        }

        self.merge_branch_outcomes(
            &base_locals,
            &base_borrows,
            &base_bindings,
            &base_remaining_uses,
            &branch_locals,
            &branch_borrows,
            &branch_bindings,
            &branch_remaining_uses,
        )
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
        let base_locals = self.locals.clone();
        let base_borrows = self.active_borrows.clone();
        let base_bindings = self.borrow_bindings.clone();
        let base_remaining_uses = self
            .remaining_identifier_uses
            .last()
            .cloned()
            .unwrap_or_default();

        let mut branch_locals = Vec::new();
        let mut branch_borrows = Vec::new();
        let mut branch_bindings = Vec::new();
        let mut branch_remaining_uses = Vec::new();
        let mut has_default_case = false;

        for case in &s.cases {
            has_default_case |= case.test.is_none();
            let (locals, borrows, bindings, remaining) = self.evaluate_switch_case_branch(
                case,
                &base_locals,
                &base_borrows,
                &base_bindings,
                &base_remaining_uses,
            )?;
            branch_locals.push(locals);
            branch_borrows.push(borrows);
            branch_bindings.push(bindings);
            branch_remaining_uses.push(remaining);
        }

        if !has_default_case {
            Self::push_base_branch_outcome(
                &mut branch_locals,
                &mut branch_borrows,
                &mut branch_bindings,
                &mut branch_remaining_uses,
                &base_locals,
                &base_borrows,
                &base_bindings,
                &base_remaining_uses,
            );
        }

        self.merge_branch_outcomes(
            &base_locals,
            &base_borrows,
            &base_bindings,
            &base_remaining_uses,
            &branch_locals,
            &branch_borrows,
            &branch_bindings,
            &branch_remaining_uses,
        )
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
        self.consume_identifier_use(&id.sym)?;
        Ok(())
    }

    fn check_assignment(
        &mut self,
        target: &AssignmentTarget,
        value: &Expr,
    ) -> Result<(), BorrowError> {
        self.check_expression(value)?;
        let mut value_ownership = self.check_moveable(value)?;

        if let Expr::Identifier(src) = value {
            self.move_identifier_if_needed(src)?;
        }

        match target {
            AssignmentTarget::Simple(expr) => {
                if let Expr::Identifier(id) = &**expr {
                    if self
                        .active_borrows
                        .get(&id.sym)
                        .map(|borrows| !borrows.is_empty())
                        .unwrap_or(false)
                    {
                        self.errors.push(BorrowError::BorrowConflict {
                            variable: id.sym.clone(),
                            location: id.span.clone(),
                            message: "Cannot assign while active borrows exist".to_string(),
                        });
                    }

                    self.release_borrow_binding(&id.sym)?;
                    let is_copyable = self.is_copyable_variable(&id.sym, &Some(value.clone()));
                    if is_copyable && matches!(value_ownership, Ownership::Owned) {
                        value_ownership = Ownership::Copied;
                    }
                    if let Some(state) = self.locals.get_mut(&id.sym) {
                        state.ownership = value_ownership;
                        state.is_copyable = is_copyable;
                    }
                    self.register_borrow_binding(&id.sym, &Some(value.clone()))?;
                    self.release_dead_borrow_binding(&id.sym)?;
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

        let function_summary = if let Expr::Identifier(id) = &*c.callee {
            self.function_summaries.get(&id.sym).cloned()
        } else {
            None
        };

        let mut temporary_borrow_counts: HashMap<String, usize> = HashMap::new();
        let mut ownership_snapshots: HashMap<String, Ownership> = HashMap::new();

        for (arg_index, arg) in c.arguments.iter().enumerate() {
            if let ExprOrSpread::Expr(e) = arg {
                let expected_borrow = function_summary
                    .as_ref()
                    .and_then(|summary| summary.param_borrows.get(arg_index))
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
                        self.check_thread_capture_argument(e)?;
                    }
                }
            } else if let Some(summary) = function_summary.as_ref() {
                for param_index in &summary.thread_captured_params {
                    if let Some(ExprOrSpread::Expr(e)) = c.arguments.get(*param_index) {
                        self.check_thread_capture_argument(e)?;
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

    fn check_thread_capture_argument(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        if let Some(name) = self.borrowed_argument_name(expr) {
            self.thread_access.insert(name.clone());
            let has_mutable_borrow = self
                .active_borrows
                .get(&name)
                .map(|borrows| borrows.iter().any(|b| b.kind == BorrowKind::Mutable))
                .unwrap_or(false);
            if has_mutable_borrow {
                self.errors.push(BorrowError::DataRace { variable: name });
            }
        }
        self.check_thread_safe_argument(expr)
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
                    .or_default()
                    .push(borrow);
            }
            self.consume_identifier_use(&id.sym)?;
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
                self.validate_returned_identifier_or_binding(id, expected_kind)?;
            }
            Expr::Ref(r) => {
                self.check_return_borrow_reference(&r.expr, BorrowKind::Shared, expected_kind)?;
            }
            Expr::MutRef(r) => {
                self.check_return_borrow_reference(&r.expr, BorrowKind::Mutable, expected_kind)?;
            }
            Expr::Call(c) => {
                self.check_return_borrow_call(c, expected_kind)?;
            }
            _ => {
                self.errors.push(BorrowError::LifetimeError(
                    "borrowed return requires identifier/reference expression".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn check_return_borrow_call(
        &mut self,
        call: &CallExpr,
        expected_kind: BorrowKind,
    ) -> Result<(), BorrowError> {
        let Expr::Identifier(callee) = &*call.callee else {
            self.errors.push(BorrowError::LifetimeError(
                "borrowed return call must target a named function".to_string(),
            ));
            return Ok(());
        };

        let Some(summary) = self.function_summaries.get(&callee.sym).cloned() else {
            self.errors.push(BorrowError::LifetimeError(format!(
                "borrowed return call '{}' has no borrow summary",
                callee.sym
            )));
            return Ok(());
        };
        if summary.return_sources.is_empty() {
            self.errors.push(BorrowError::LifetimeError(format!(
                "borrowed return call '{}' does not map to a borrowed parameter",
                callee.sym
            )));
            return Ok(());
        }

        let mut seen = HashSet::new();
        for source in summary.return_sources {
            if !seen.insert((source.param_index, source.kind)) {
                continue;
            }

            if !Self::borrow_kind_satisfies(source.kind, expected_kind) {
                self.errors.push(BorrowError::InvalidBorrow {
                    variable: callee.sym.clone(),
                    location: callee.span.clone(),
                    message: format!(
                        "returned borrow from '{}' parameter {} does not match function return type",
                        callee.sym, source.param_index
                    ),
                });
                continue;
            }

            let Some(ExprOrSpread::Expr(arg_expr)) = call.arguments.get(source.param_index) else {
                self.errors.push(BorrowError::LifetimeError(format!(
                    "borrowed return call '{}' is missing source parameter {}",
                    callee.sym, source.param_index
                )));
                continue;
            };

            match arg_expr {
                Expr::Identifier(id) => {
                    self.validate_returned_identifier_or_binding(id, source.kind)?;
                }
                Expr::Ref(r) => {
                    self.check_return_borrow_reference(&r.expr, BorrowKind::Shared, expected_kind)?;
                }
                Expr::MutRef(r) => {
                    self.check_return_borrow_reference(
                        &r.expr,
                        BorrowKind::Mutable,
                        expected_kind,
                    )?;
                }
                _ => {
                    self.errors.push(BorrowError::LifetimeError(format!(
                        "borrowed return call '{}' source parameter {} must be identifier/reference expression",
                        callee.sym, source.param_index
                    )));
                }
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

        let Expr::Identifier(id) = inner else {
            self.errors.push(BorrowError::LifetimeError(
                "borrowed return must reference a function parameter".to_string(),
            ));
            return Ok(());
        };

        self.validate_returned_identifier_or_binding(id, found_kind)?;
        if !Self::borrow_kind_satisfies(found_kind, expected_kind) {
            self.errors.push(BorrowError::InvalidBorrow {
                variable: id.sym.clone(),
                location: id.span.clone(),
                message: "return borrow kind does not match function return type".to_string(),
            });
        }
        Ok(())
    }

    fn validate_returned_identifier_or_binding(
        &mut self,
        id: &Ident,
        found_kind: BorrowKind,
    ) -> Result<(), BorrowError> {
        if self.validate_returned_param_reference(id, found_kind)? {
            return Ok(());
        }

        if let Some(binding) = self.borrow_bindings.get(&id.sym).cloned() {
            let mut saw_valid_param = false;
            for source in binding.sources {
                if let Some(ctx) = &self.current_function_context {
                    if let Some(param_kind) =
                        ctx.param_borrows.get(&source.source).copied().flatten()
                    {
                        saw_valid_param = true;
                        if !Self::borrow_kind_satisfies(param_kind, source.kind)
                            || !Self::borrow_kind_satisfies(source.kind, found_kind)
                        {
                            self.errors.push(BorrowError::InvalidBorrow {
                                variable: id.sym.clone(),
                                location: id.span.clone(),
                                message: format!(
                                    "borrowed return '{}' reborrows '{}' with incompatible mutability",
                                    id.sym, source.source
                                ),
                            });
                        }
                    } else {
                        self.errors.push(BorrowError::LifetimeError(format!(
                            "borrowed return '{}' references local '{}' that does not outlive function",
                            id.sym, source.source
                        )));
                    }
                }
            }

            if saw_valid_param {
                return Ok(());
            }
        }

        self.errors.push(BorrowError::LifetimeError(format!(
            "borrowed return value '{}' does not outlive function",
            id.sym
        )));
        Ok(())
    }

    fn validate_returned_param_reference(
        &mut self,
        id: &Ident,
        found_kind: BorrowKind,
    ) -> Result<bool, BorrowError> {
        let Some(ctx) = &self.current_function_context else {
            return Ok(false);
        };

        let Some(param_kind) = ctx.param_borrows.get(&id.sym).copied().flatten() else {
            return Ok(false);
        };

        if !Self::borrow_kind_satisfies(param_kind, found_kind) {
            self.errors.push(BorrowError::InvalidBorrow {
                variable: id.sym.clone(),
                location: id.span.clone(),
                message: "cannot reborrow with stronger mutability than parameter".to_string(),
            });
        }

        Ok(true)
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
            Expr::Ref(_) => Ok(Ownership::Copied),
            Expr::Identifier(id) => {
                if let Some(binding) = self.borrow_bindings.get(&id.sym) {
                    return Ok(
                        if binding
                            .sources
                            .iter()
                            .all(|source| source.kind == BorrowKind::Shared)
                        {
                            Ownership::Copied
                        } else {
                            Ownership::Owned
                        },
                    );
                }
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
        for state in self.locals.values_mut() {
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
        if let Some((is_safe, message)) = self.thread_safety_by_type(expr) {
            if is_safe {
                return Ok(());
            }
            self.errors.push(BorrowError::ThreadSafetyViolation {
                location: expr.span().clone(),
                message,
            });
            return Ok(());
        }

        if self.is_thread_safe_expression(expr) {
            return Ok(());
        }

        self.errors.push(BorrowError::ThreadSafetyViolation {
            location: expr.span().clone(),
            message: "value captured by thread/process is not thread-safe".to_string(),
        });
        Ok(())
    }

    fn thread_safety_by_type(&self, expr: &Expr) -> Option<(bool, String)> {
        let info = self.type_info.as_ref()?;
        let ty = *info.expr_types.get(expr.span())?;
        let ty_kind = info.type_table.get(ty)?;

        let (is_safe, reason) = match ty_kind {
            CheckedType::Ref(inner) => {
                let mut visited = HashSet::new();
                (
                    self.is_sync_type(&info.type_table, *inner, &mut visited),
                    "shared reference capture requires Sync pointee".to_string(),
                )
            }
            CheckedType::MutRef(_) => (
                false,
                "mutable reference capture is not Send across thread/process boundaries"
                    .to_string(),
            ),
            CheckedType::Shared(inner) => {
                let mut send_visited = HashSet::new();
                let mut sync_visited = HashSet::new();
                (
                    self.is_send_type(&info.type_table, *inner, &mut send_visited)
                        && self.is_sync_type(&info.type_table, *inner, &mut sync_visited),
                    "Shared<T> capture requires Send + Sync inner type".to_string(),
                )
            }
            _ => {
                let mut visited = HashSet::new();
                (
                    self.is_send_type(&info.type_table, ty, &mut visited),
                    "thread/process capture requires Send type".to_string(),
                )
            }
        };

        Some((is_safe, reason))
    }

    fn is_thread_safe_expression(&self, expr: &Expr) -> bool {
        if let Some((is_safe, _)) = self.thread_safety_by_type(expr) {
            return is_safe;
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

    fn is_send_type(
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
            | Some(CheckedType::Promise(inner)) => self.is_send_type(type_table, *inner, visited),
            Some(CheckedType::Shared(inner)) => {
                let mut sync_visited = HashSet::new();
                self.is_send_type(type_table, *inner, visited)
                    && self.is_sync_type(type_table, *inner, &mut sync_visited)
            }
            Some(CheckedType::Tuple(types))
            | Some(CheckedType::Union(types))
            | Some(CheckedType::Intersection(types)) => types
                .iter()
                .all(|inner| self.is_send_type(type_table, *inner, visited)),
            Some(CheckedType::Result(ok, err)) => {
                self.is_send_type(type_table, *ok, visited)
                    && self.is_send_type(type_table, *err, visited)
            }
            Some(CheckedType::Struct(def)) => def
                .fields
                .iter()
                .all(|field| self.is_send_type(type_table, field.ty, visited)),
            Some(CheckedType::ObjectShape(def)) => {
                def.methods.is_empty()
                    && def
                        .fields
                        .iter()
                        .all(|field| self.is_send_type(type_table, field.ty, visited))
            }
            Some(CheckedType::Ref(inner)) => {
                let mut sync_visited = HashSet::new();
                self.is_sync_type(type_table, *inner, &mut sync_visited)
            }
            Some(CheckedType::MutRef(_))
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

    fn is_sync_type(
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
            | Some(CheckedType::Shared(inner))
            | Some(CheckedType::Ref(inner)) => self.is_sync_type(type_table, *inner, visited),
            Some(CheckedType::Tuple(types))
            | Some(CheckedType::Union(types))
            | Some(CheckedType::Intersection(types)) => types
                .iter()
                .all(|inner| self.is_sync_type(type_table, *inner, visited)),
            Some(CheckedType::Result(ok, err)) => {
                self.is_sync_type(type_table, *ok, visited)
                    && self.is_sync_type(type_table, *err, visited)
            }
            Some(CheckedType::Struct(def)) => def
                .fields
                .iter()
                .all(|field| self.is_sync_type(type_table, field.ty, visited)),
            Some(CheckedType::ObjectShape(def)) => {
                def.methods.is_empty()
                    && def
                        .fields
                        .iter()
                        .all(|field| self.is_sync_type(type_table, field.ty, visited))
            }
            Some(CheckedType::MutRef(_))
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
