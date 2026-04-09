//! Desugaring pass: rewrites named arguments into positional order.
//!
//! After the type checker validates named arguments (names, types, arity),
//! this pass normalizes them so downstream phases (borrow checker, IR, codegen)
//! only see positional arguments.

use crate::type_checker::TypeEnvironment;
use crate::types::FunctionSig;
use argon_ast::*;

/// Rewrite all named arguments in `source` into positional order
/// using function signatures from `env`.
pub fn desugar_named_args(source: &mut SourceFile, env: &TypeEnvironment) {
    for stmt in &mut source.statements {
        desugar_stmt(stmt, env);
    }
}

fn desugar_stmt(stmt: &mut Stmt, env: &TypeEnvironment) {
    match stmt {
        Stmt::Expr(e) => desugar_expr(&mut e.expr, env),
        Stmt::Block(b) => {
            for s in &mut b.statements {
                desugar_stmt(s, env);
            }
        }
        Stmt::If(i) => {
            desugar_expr(&mut i.condition, env);
            desugar_stmt(&mut i.consequent, env);
            if let Some(alt) = &mut i.alternate {
                desugar_stmt(alt, env);
            }
        }
        Stmt::Switch(s) => {
            desugar_expr(&mut s.discriminant, env);
            for case in &mut s.cases {
                if let Some(test) = &mut case.test {
                    desugar_expr(test, env);
                }
                for s in &mut case.consequent {
                    desugar_stmt(s, env);
                }
            }
        }
        Stmt::For(f) => {
            if let Some(ForInit::Variable(v)) = &mut f.init {
                desugar_variable_stmt(v, env);
            } else if let Some(ForInit::Expr(e)) = &mut f.init {
                desugar_expr(e, env);
            }
            if let Some(test) = &mut f.test {
                desugar_expr(test, env);
            }
            if let Some(update) = &mut f.update {
                desugar_expr(update, env);
            }
            desugar_stmt(&mut f.body, env);
        }
        Stmt::ForIn(f) => {
            desugar_expr(&mut f.right, env);
            desugar_stmt(&mut f.body, env);
        }
        Stmt::While(w) => {
            desugar_expr(&mut w.condition, env);
            desugar_stmt(&mut w.body, env);
        }
        Stmt::DoWhile(d) => {
            desugar_expr(&mut d.condition, env);
            desugar_stmt(&mut d.body, env);
        }
        Stmt::Return(r) => {
            if let Some(arg) = &mut r.argument {
                desugar_expr(arg, env);
            }
        }
        Stmt::Variable(v) => desugar_variable_stmt(v, env),
        Stmt::Function(f) | Stmt::AsyncFunction(f) => {
            desugar_function_body(&mut f.body, env);
        }
        Stmt::Struct(s) => {
            for method in &mut s.methods {
                desugar_function_body(&mut method.value.body, env);
            }
            if let Some(ctor) = &mut s.constructor {
                for s in &mut ctor.body.statements {
                    desugar_stmt(s, env);
                }
            }
        }
        Stmt::Export(e) => {
            if let Some(decl) = &mut e.declaration {
                desugar_stmt(decl, env);
            }
        }
        Stmt::Match(m) => {
            desugar_expr(&mut m.discriminant, env);
            for case in &mut m.cases {
                desugar_stmt(&mut case.consequent, env);
                if let Some(guard) = &mut case.guard {
                    desugar_expr(guard, env);
                }
            }
        }
        Stmt::Loop(l) => {
            desugar_stmt(&mut l.body, env);
        }
        Stmt::Labeled(l) => {
            desugar_stmt(&mut l.body, env);
        }
        Stmt::Empty(_)
        | Stmt::Break(_)
        | Stmt::Continue(_)
        | Stmt::With(_)
        | Stmt::Debugger(_)
        | Stmt::Skill(_)
        | Stmt::Impl(_)
        | Stmt::Interface(_)
        | Stmt::TypeAlias(_)
        | Stmt::Enum(_)
        | Stmt::Module(_)
        | Stmt::Import(_) => {}
    }
}

fn desugar_variable_stmt(v: &mut VariableStmt, env: &TypeEnvironment) {
    for decl in &mut v.declarations {
        if let Some(init) = &mut decl.init {
            desugar_expr(init, env);
        }
    }
}

fn desugar_function_body(body: &mut FunctionBody, env: &TypeEnvironment) {
    for stmt in &mut body.statements {
        desugar_stmt(stmt, env);
    }
}

fn desugar_expr(expr: &mut Expr, env: &TypeEnvironment) {
    match expr {
        Expr::Call(c) => {
            // First, recurse into callee and argument expressions
            desugar_expr(&mut c.callee, env);
            for arg in &mut c.arguments {
                desugar_expr_or_spread(arg, env);
            }
            // Then reorder named args if present
            if c.arguments
                .iter()
                .any(|a| matches!(a, ExprOrSpread::Named { .. }))
            {
                if let Some(sig) = resolve_callee_sig(&c.callee, env) {
                    reorder_args(&mut c.arguments, &sig);
                }
            }
        }
        Expr::New(n) => {
            desugar_expr(&mut n.callee, env);
            for arg in &mut n.arguments {
                desugar_expr_or_spread(arg, env);
            }
            if n.arguments
                .iter()
                .any(|a| matches!(a, ExprOrSpread::Named { .. }))
            {
                if let Some(sig) = resolve_callee_sig(&n.callee, env) {
                    reorder_args(&mut n.arguments, &sig);
                }
            }
        }
        Expr::OptionalCall(c) => {
            desugar_expr(&mut c.callee, env);
            for arg in &mut c.arguments {
                desugar_expr_or_spread(arg, env);
            }
            if c.arguments
                .iter()
                .any(|a| matches!(a, ExprOrSpread::Named { .. }))
            {
                if let Some(sig) = resolve_callee_sig(&c.callee, env) {
                    reorder_args(&mut c.arguments, &sig);
                }
            }
        }
        Expr::Binary(b) => {
            desugar_expr(&mut b.left, env);
            desugar_expr(&mut b.right, env);
        }
        Expr::Logical(l) => {
            desugar_expr(&mut l.left, env);
            desugar_expr(&mut l.right, env);
        }
        Expr::Unary(u) => desugar_expr(&mut u.argument, env),
        Expr::Update(u) => desugar_expr(&mut u.argument, env),
        Expr::Assignment(a) => {
            if let AssignmentTarget::Simple(e) = &mut *a.left {
                desugar_expr(e, env);
            } else if let AssignmentTarget::Member(m) = &mut *a.left {
                desugar_expr(&mut m.object, env);
                desugar_expr(&mut m.property, env);
            }
            desugar_expr(&mut a.right, env);
        }
        Expr::Conditional(c) => {
            desugar_expr(&mut c.test, env);
            desugar_expr(&mut c.consequent, env);
            desugar_expr(&mut c.alternate, env);
        }
        Expr::Member(m) => {
            desugar_expr(&mut m.object, env);
            desugar_expr(&mut m.property, env);
        }
        Expr::OptionalMember(m) => {
            desugar_expr(&mut m.object, env);
            desugar_expr(&mut m.property, env);
        }
        Expr::Array(a) => {
            for elem in a.elements.iter_mut().flatten() {
                desugar_expr_or_spread(elem, env);
            }
        }
        Expr::Object(o) => {
            for prop in &mut o.properties {
                match prop {
                    ObjectProperty::Property(p) => {
                        desugar_expr_or_spread(&mut p.value, env);
                    }
                    ObjectProperty::Spread(s) => {
                        desugar_expr(&mut s.argument, env);
                    }
                    ObjectProperty::Method(m)
                    | ObjectProperty::Getter(m)
                    | ObjectProperty::Setter(m) => {
                        desugar_function_body(&mut m.value.body, env);
                    }
                    ObjectProperty::Shorthand(_) => {}
                }
            }
        }
        Expr::ArrowFunction(f) => match &mut f.body {
            ArrowFunctionBody::Block(b) => {
                for s in &mut b.statements {
                    desugar_stmt(s, env);
                }
            }
            ArrowFunctionBody::Expr(e) => desugar_expr(e, env),
        },
        Expr::Function(f) => {
            desugar_function_body(&mut f.body, env);
        }
        Expr::Await(a) | Expr::AwaitPromised(a) => desugar_expr(&mut a.argument, env),
        Expr::Yield(y) => {
            if let Some(arg) = &mut y.argument {
                desugar_expr(arg, env);
            }
        }
        Expr::Spread(s) => desugar_expr(&mut s.argument, env),
        Expr::Template(t) => {
            for e in &mut t.expressions {
                desugar_expr(e, env);
            }
        }
        Expr::TaggedTemplate(t) => {
            desugar_expr(&mut t.tag, env);
            for e in &mut t.template.expressions {
                desugar_expr(e, env);
            }
        }
        Expr::Parenthesized(p) => desugar_expr(&mut p.expression, env),
        Expr::Ref(r) => desugar_expr(&mut r.expr, env),
        Expr::MutRef(r) => desugar_expr(&mut r.expr, env),
        Expr::Chain(c) => {
            for elem in &mut c.expressions {
                match elem {
                    ChainElement::Call(call) => {
                        desugar_expr(&mut call.callee, env);
                        for arg in &mut call.arguments {
                            desugar_expr_or_spread(arg, env);
                        }
                        if call
                            .arguments
                            .iter()
                            .any(|a| matches!(a, ExprOrSpread::Named { .. }))
                        {
                            if let Some(sig) = resolve_callee_sig(&call.callee, env) {
                                reorder_args(&mut call.arguments, &sig);
                            }
                        }
                    }
                    ChainElement::OptionalCall(call) => {
                        desugar_expr(&mut call.callee, env);
                        for arg in &mut call.arguments {
                            desugar_expr_or_spread(arg, env);
                        }
                        if call
                            .arguments
                            .iter()
                            .any(|a| matches!(a, ExprOrSpread::Named { .. }))
                        {
                            if let Some(sig) = resolve_callee_sig(&call.callee, env) {
                                reorder_args(&mut call.arguments, &sig);
                            }
                        }
                    }
                    ChainElement::Member(m) => {
                        desugar_expr(&mut m.object, env);
                        desugar_expr(&mut m.property, env);
                    }
                    ChainElement::OptionalMember(m) => {
                        desugar_expr(&mut m.object, env);
                        desugar_expr(&mut m.property, env);
                    }
                }
            }
        }
        Expr::TypeAssertion(t) => desugar_expr(&mut t.expression, env),
        Expr::AsType(a) => desugar_expr(&mut a.expression, env),
        Expr::NonNull(n) => desugar_expr(&mut n.expression, env),
        // Leaf expressions — no sub-expressions to walk
        Expr::This(_)
        | Expr::Super(_)
        | Expr::Identifier(_)
        | Expr::Literal(_)
        | Expr::MetaProperty(_)
        | Expr::JsxElement(_)
        | Expr::JsxFragment(_)
        | Expr::Import(_)
        | Expr::Regex(_)
        | Expr::AssignmentTargetPattern(_) => {}
    }
}

fn desugar_expr_or_spread(arg: &mut ExprOrSpread, env: &TypeEnvironment) {
    match arg {
        ExprOrSpread::Expr(e) => desugar_expr(e, env),
        ExprOrSpread::Named { value, .. } => desugar_expr(value, env),
        ExprOrSpread::Spread(s) => desugar_expr(&mut s.argument, env),
    }
}

/// Look up the function signature for a callee expression.
fn resolve_callee_sig(callee: &Expr, env: &TypeEnvironment) -> Option<FunctionSig> {
    match callee {
        Expr::Identifier(id) => env.get_function(&id.sym).cloned(),
        _ => None,
    }
}

/// Rewrite `arguments` so that named args are placed in parameter order,
/// converting `Named { name, value }` into `Expr(*value)`.
fn reorder_args(arguments: &mut Vec<ExprOrSpread>, sig: &FunctionSig) {
    // Separate positional and named args
    let mut positional: Vec<ExprOrSpread> = Vec::new();
    let mut named: Vec<(String, ExprOrSpread)> = Vec::new();

    for arg in arguments.drain(..) {
        match arg {
            ExprOrSpread::Named { name, value } => {
                named.push((name.sym.clone(), ExprOrSpread::Expr(*value)));
            }
            other => positional.push(other),
        }
    }

    // Build result in parameter order
    for (i, param) in sig.params.iter().enumerate() {
        if i < positional.len() {
            arguments.push(positional[i].clone());
        } else if let Some(pos) = named.iter().position(|(n, _)| n == &param.name) {
            let (_, arg) = named.remove(pos);
            arguments.push(arg);
        }
        // If neither positional nor named covers this param, the param must have
        // a default — the type checker already validated this. We leave a gap so
        // the codegen's default parameter handling kicks in.
    }
}
