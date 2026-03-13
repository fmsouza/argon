//! Argon Runtime - Direct execution of Argon code

use argon_ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Function(RcFunction),
    NativeFunction(NativeFunction),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Array(Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct RcFunction {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub closure: Scope,
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    pub name: String,
}

#[derive(Debug, Clone)]
struct RuntimeStructDef {
    fields: Vec<String>,
    methods: HashMap<String, FunctionDecl>,
}

#[derive(Debug, Clone)]
struct RuntimeClassDef {
    fields: Vec<String>,
    methods: HashMap<String, FunctionDecl>,
    constructor: Option<Constructor>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    values: HashMap<String, Value>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.values.get(name).cloned()
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }
}

enum ExecOutcome {
    Normal,
    Return(Value),
    Break,
    Continue,
}

pub struct Runtime {
    scope: Scope,
    globals: HashMap<String, Value>,
    struct_defs: HashMap<String, RuntimeStructDef>,
    class_defs: HashMap<String, RuntimeClassDef>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let mut globals = HashMap::new();

        let mut console_obj = HashMap::new();
        console_obj.insert(
            "log".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "console.log".to_string(),
            }),
        );
        globals.insert(
            "console".to_string(),
            Value::Object(Rc::new(RefCell::new(console_obj))),
        );

        let mut math_obj = HashMap::new();
        math_obj.insert(
            "abs".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.abs".to_string(),
            }),
        );
        math_obj.insert(
            "floor".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.floor".to_string(),
            }),
        );
        math_obj.insert(
            "ceil".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.ceil".to_string(),
            }),
        );
        math_obj.insert(
            "round".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.round".to_string(),
            }),
        );
        math_obj.insert(
            "sqrt".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.sqrt".to_string(),
            }),
        );
        math_obj.insert(
            "pow".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.pow".to_string(),
            }),
        );
        math_obj.insert(
            "max".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.max".to_string(),
            }),
        );
        math_obj.insert(
            "min".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "Math.min".to_string(),
            }),
        );
        globals.insert(
            "Math".to_string(),
            Value::Object(Rc::new(RefCell::new(math_obj))),
        );

        Self {
            scope: Scope::new(),
            globals,
            struct_defs: HashMap::new(),
            class_defs: HashMap::new(),
        }
    }

    pub fn execute(&mut self, source: &SourceFile) -> Result<Value, RuntimeError> {
        if let Some(feature) = detect_compile_only_feature_in_source(source) {
            return Err(RuntimeError::CompileOnlyFeature(feature.to_string()));
        }

        for stmt in &source.statements {
            match self.execute_statement(stmt)? {
                ExecOutcome::Normal => {}
                ExecOutcome::Return(value) => return Ok(value),
                ExecOutcome::Break => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "break outside loop".to_string(),
                    ))
                }
                ExecOutcome::Continue => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "continue outside loop".to_string(),
                    ))
                }
            }
        }
        Ok(Value::Undefined)
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<ExecOutcome, RuntimeError> {
        match stmt {
            Stmt::Variable(v) => {
                for decl in &v.declarations {
                    if let Pattern::Identifier(id) = &decl.id {
                        let value = match &decl.init {
                            Some(expr) => self.evaluate_expression(expr)?,
                            None => Value::Undefined,
                        };
                        self.scope.define(id.name.sym.clone(), value);
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Function(f) | Stmt::AsyncFunction(f) => {
                if let Some(id) = &f.id {
                    let func = self.function_value(f, self.scope.clone());
                    self.scope.define(id.sym.clone(), Value::Function(func));
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Struct(s) => {
                self.register_struct(s);
                Ok(ExecOutcome::Normal)
            }
            Stmt::Class(c) => {
                self.register_class(c);
                Ok(ExecOutcome::Normal)
            }
            Stmt::Enum(e) => {
                self.register_enum(e)?;
                Ok(ExecOutcome::Normal)
            }
            Stmt::Expr(e) => {
                let _ = self.evaluate_expression(&e.expr)?;
                Ok(ExecOutcome::Normal)
            }
            Stmt::Return(r) => match &r.argument {
                Some(expr) => Ok(ExecOutcome::Return(self.evaluate_expression(expr)?)),
                None => Ok(ExecOutcome::Return(Value::Undefined)),
            },
            Stmt::If(i) => {
                let cond = self.evaluate_expression(&i.condition)?;
                if self.is_truthy(&cond) {
                    self.execute_statement(&i.consequent)
                } else if let Some(alt) = &i.alternate {
                    self.execute_statement(alt)
                } else {
                    Ok(ExecOutcome::Normal)
                }
            }
            Stmt::While(w) => {
                loop {
                    let condition = self.evaluate_expression(&w.condition)?;
                    if !self.is_truthy(&condition) {
                        break;
                    }

                    match self.execute_statement(&w.body)? {
                        ExecOutcome::Normal => {}
                        ExecOutcome::Break => break,
                        ExecOutcome::Continue => continue,
                        ExecOutcome::Return(v) => return Ok(ExecOutcome::Return(v)),
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Loop(l) => {
                loop {
                    match self.execute_statement(&l.body)? {
                        ExecOutcome::Normal => {}
                        ExecOutcome::Break => break,
                        ExecOutcome::Continue => continue,
                        ExecOutcome::Return(v) => return Ok(ExecOutcome::Return(v)),
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::For(f) => {
                if let Some(init) = &f.init {
                    match init {
                        ForInit::Variable(v) => {
                            let _ = self.execute_statement(&Stmt::Variable(v.clone()))?;
                        }
                        ForInit::Expr(e) => {
                            let _ = self.evaluate_expression(e)?;
                        }
                    }
                }

                loop {
                    if let Some(test) = &f.test {
                        let test_value = self.evaluate_expression(test)?;
                        if !self.is_truthy(&test_value) {
                            break;
                        }
                    }

                    match self.execute_statement(&f.body)? {
                        ExecOutcome::Normal => {}
                        ExecOutcome::Break => break,
                        ExecOutcome::Continue => {}
                        ExecOutcome::Return(v) => return Ok(ExecOutcome::Return(v)),
                    }

                    if let Some(update) = &f.update {
                        let _ = self.evaluate_expression(update)?;
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::ForIn(f) => {
                let iterable = self.evaluate_expression(&f.right)?;
                let values = self.to_iter_values(&iterable)?;
                for value in values {
                    match &f.left {
                        ForInLeft::Pattern(Pattern::Identifier(id)) => {
                            self.scope.set(id.name.sym.clone(), value);
                        }
                        ForInLeft::Variable(v) => {
                            if let Pattern::Identifier(id) = &v.id {
                                self.scope.set(id.name.sym.clone(), value);
                            }
                        }
                        _ => {
                            return Err(RuntimeError::Unsupported(
                                "unsupported for..of binding pattern".to_string(),
                            ));
                        }
                    }

                    match self.execute_statement(&f.body)? {
                        ExecOutcome::Normal => {}
                        ExecOutcome::Break => break,
                        ExecOutcome::Continue => continue,
                        ExecOutcome::Return(v) => return Ok(ExecOutcome::Return(v)),
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Block(b) => {
                for stmt in &b.statements {
                    match self.execute_statement(stmt)? {
                        ExecOutcome::Normal => {}
                        non_normal => return Ok(non_normal),
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Break(_) => Ok(ExecOutcome::Break),
            Stmt::Continue(_) => Ok(ExecOutcome::Continue),
            Stmt::Match(m) => {
                let disc = self.evaluate_expression(&m.discriminant)?;
                for case in &m.cases {
                    let pat = self.evaluate_expression(&case.pattern)?;
                    if self.values_equal(&disc, &pat) {
                        return self.execute_statement(&case.consequent);
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Switch(s) => {
                let disc = self.evaluate_expression(&s.discriminant)?;
                let mut matched = false;
                for case in &s.cases {
                    if !matched {
                        matched = if let Some(test) = &case.test {
                            let test_value = self.evaluate_expression(test)?;
                            self.values_equal(&disc, &test_value)
                        } else {
                            true
                        };
                    }
                    if matched {
                        for stmt in &case.consequent {
                            match self.execute_statement(stmt)? {
                                ExecOutcome::Normal => {}
                                ExecOutcome::Break => return Ok(ExecOutcome::Normal),
                                non_normal => return Ok(non_normal),
                            }
                        }
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Throw(t) => {
                let value = self.evaluate_expression(&t.argument)?;
                Err(RuntimeError::Thrown(self.value_to_string(&value)))
            }
            Stmt::Try(t) => {
                let try_result = self.execute_statement(&Stmt::Block(t.block.clone()));
                let outcome = match try_result {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        if let Some(handler) = &t.handler {
                            if let Some(Pattern::Identifier(id)) = &handler.param {
                                self.scope
                                    .set(id.name.sym.clone(), Value::String(err.to_string()));
                            }
                            self.execute_statement(&Stmt::Block(handler.body.clone()))?
                        } else {
                            return Err(err);
                        }
                    }
                };

                if let Some(finalizer) = &t.finalizer {
                    let _ = self.execute_statement(&Stmt::Block(finalizer.clone()))?;
                }
                Ok(outcome)
            }
            Stmt::Export(_)
            | Stmt::Interface(_)
            | Stmt::TypeAlias(_)
            | Stmt::Empty(_)
            | Stmt::Debugger(_) => Ok(ExecOutcome::Normal),
            _ => Err(RuntimeError::Unsupported(format!(
                "unsupported statement at runtime: {:?}",
                stmt
            ))),
        }
    }

    fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Number(n) => Ok(Value::Number(n.value)),
                Literal::String(s) => Ok(Value::String(s.value.clone())),
                Literal::Boolean(b) => Ok(Value::Boolean(b.value)),
                Literal::Null(_) => Ok(Value::Null),
                _ => Ok(Value::Undefined),
            },
            Expr::This(_) => Ok(self.scope.get("this").unwrap_or(Value::Undefined)),
            Expr::Identifier(id) => self
                .scope
                .get(&id.sym)
                .or_else(|| self.globals.get(&id.sym).cloned())
                .ok_or_else(|| RuntimeError::UndefinedVariable(id.sym.clone())),
            Expr::Template(t) => {
                let mut out = String::new();
                for (idx, quasi) in t.quasis.iter().enumerate() {
                    out.push_str(&quasi.value);
                    if let Some(expr) = t.expressions.get(idx) {
                        let value = self.evaluate_expression(expr)?;
                        out.push_str(&self.value_to_string(&value));
                    }
                }
                Ok(Value::String(out))
            }
            Expr::Object(o) => {
                let mut map = HashMap::new();
                for prop in &o.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            let key = self.property_key(&p.key)?;
                            let value = match &p.value {
                                ExprOrSpread::Expr(e) => self.evaluate_expression(e)?,
                                ExprOrSpread::Spread(_) => {
                                    return Err(RuntimeError::Unsupported(
                                        "spread in object literals is not supported".to_string(),
                                    ))
                                }
                            };
                            map.insert(key, value);
                        }
                        ObjectProperty::Shorthand(id) => {
                            let value = self
                                .scope
                                .get(&id.sym)
                                .or_else(|| self.globals.get(&id.sym).cloned())
                                .unwrap_or(Value::Undefined);
                            map.insert(id.sym.clone(), value);
                        }
                        _ => {
                            return Err(RuntimeError::Unsupported(
                                "unsupported object literal property".to_string(),
                            ))
                        }
                    }
                }
                Ok(Value::Object(Rc::new(RefCell::new(map))))
            }
            Expr::Array(a) => {
                let mut values = Vec::new();
                for el in &a.elements {
                    match el {
                        Some(ExprOrSpread::Expr(e)) => values.push(self.evaluate_expression(e)?),
                        Some(ExprOrSpread::Spread(_)) => {
                            return Err(RuntimeError::Unsupported(
                                "array spread is not supported".to_string(),
                            ))
                        }
                        None => values.push(Value::Undefined),
                    }
                }
                Ok(Value::Array(values))
            }
            Expr::Member(m) => {
                let object = self.evaluate_expression(&m.object)?;
                if let Value::Object(map) = object {
                    let property = if m.computed {
                        let key = self.evaluate_expression(&m.property)?;
                        self.value_to_string(&key)
                    } else {
                        self.property_key(&m.property)?
                    };
                    Ok(map
                        .borrow()
                        .get(&property)
                        .cloned()
                        .unwrap_or(Value::Undefined))
                } else if let Value::Array(values) = object {
                    if m.computed {
                        let key = self.evaluate_expression(&m.property)?;
                        match key {
                            Value::Number(n) if n >= 0.0 && n.fract() == 0.0 => {
                                Ok(values.get(n as usize).cloned().unwrap_or(Value::Undefined))
                            }
                            Value::String(s) => Ok(s
                                .parse::<usize>()
                                .ok()
                                .and_then(|idx| values.get(idx).cloned())
                                .unwrap_or(Value::Undefined)),
                            _ => Err(RuntimeError::TypeError(
                                "array index must be numeric".to_string(),
                            )),
                        }
                    } else {
                        let property = self.property_key(&m.property)?;
                        match property.as_str() {
                            "length" => Ok(Value::Number(values.len() as f64)),
                            _ => Ok(Value::Undefined),
                        }
                    }
                } else {
                    Err(RuntimeError::TypeError(
                        "member access on non-object value".to_string(),
                    ))
                }
            }
            Expr::Assignment(a) => {
                let value = self.evaluate_expression(&a.right)?;
                match &*a.left {
                    AssignmentTarget::Simple(target) => match &**target {
                        Expr::Identifier(id) => {
                            self.scope.set(id.sym.clone(), value.clone());
                            Ok(value)
                        }
                        Expr::Member(member) => {
                            self.assign_member(member, value.clone())?;
                            Ok(value)
                        }
                        _ => Err(RuntimeError::Unsupported(
                            "unsupported assignment target".to_string(),
                        )),
                    },
                    AssignmentTarget::Member(member) => {
                        self.assign_member(member, value.clone())?;
                        Ok(value)
                    }
                    AssignmentTarget::Pattern(_) => Err(RuntimeError::Unsupported(
                        "pattern assignment is not supported".to_string(),
                    )),
                }
            }
            Expr::Binary(b) => {
                let left = self.evaluate_expression(&b.left)?;
                let right = self.evaluate_expression(&b.right)?;
                self.eval_binary(b.operator.clone(), left, right)
            }
            Expr::Unary(u) => {
                let value = self.evaluate_expression(&u.argument)?;
                match u.operator {
                    UnaryOperator::Minus => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::TypeError(
                            "unary '-' expects a number".to_string(),
                        )),
                    },
                    UnaryOperator::LogicalNot => Ok(Value::Boolean(!self.is_truthy(&value))),
                    UnaryOperator::Plus => match value {
                        Value::Number(n) => Ok(Value::Number(n)),
                        _ => Err(RuntimeError::TypeError(
                            "unary '+' expects a number".to_string(),
                        )),
                    },
                    _ => Err(RuntimeError::Unsupported(
                        "unsupported unary operator".to_string(),
                    )),
                }
            }
            Expr::Call(c) => {
                let callee = self.evaluate_expression(&c.callee)?;
                let mut args = Vec::new();
                for arg in &c.arguments {
                    match arg {
                        ExprOrSpread::Expr(e) => args.push(self.evaluate_expression(e)?),
                        ExprOrSpread::Spread(_) => {
                            return Err(RuntimeError::Unsupported(
                                "spread arguments are not supported".to_string(),
                            ))
                        }
                    }
                }
                self.call_value(callee, &args)
            }
            Expr::New(n) => {
                if let Expr::Identifier(id) = &*n.callee {
                    if self.struct_defs.contains_key(&id.sym) {
                        return self.instantiate_struct(&id.sym, &n.arguments);
                    }
                    if self.class_defs.contains_key(&id.sym) {
                        return self.instantiate_class(&id.sym, &n.arguments);
                    }
                }

                Err(RuntimeError::Unsupported(
                    "unsupported constructor target".to_string(),
                ))
            }
            Expr::Conditional(c) => {
                let test = self.evaluate_expression(&c.test)?;
                if self.is_truthy(&test) {
                    self.evaluate_expression(&c.consequent)
                } else {
                    self.evaluate_expression(&c.alternate)
                }
            }
            Expr::Logical(l) => {
                let left = self.evaluate_expression(&l.left)?;
                match l.operator {
                    LogicalOperator::And => {
                        if self.is_truthy(&left) {
                            self.evaluate_expression(&l.right)
                        } else {
                            Ok(left)
                        }
                    }
                    LogicalOperator::Or => {
                        if self.is_truthy(&left) {
                            Ok(left)
                        } else {
                            self.evaluate_expression(&l.right)
                        }
                    }
                    LogicalOperator::NullishCoalescing => {
                        if matches!(left, Value::Null | Value::Undefined) {
                            self.evaluate_expression(&l.right)
                        } else {
                            Ok(left)
                        }
                    }
                }
            }
            Expr::Update(u) => {
                if let Expr::Identifier(id) = &*u.argument {
                    let current = self.scope.get(&id.sym).unwrap_or(Value::Undefined);
                    let Value::Number(n) = current else {
                        return Err(RuntimeError::TypeError(
                            "update operator expects numeric identifier".to_string(),
                        ));
                    };
                    let next = match u.operator {
                        UpdateOperator::Increment => n + 1.0,
                        UpdateOperator::Decrement => n - 1.0,
                    };
                    self.scope.set(id.sym.clone(), Value::Number(next));
                    if u.prefix {
                        Ok(Value::Number(next))
                    } else {
                        Ok(Value::Number(n))
                    }
                } else {
                    Err(RuntimeError::Unsupported(
                        "update target must be an identifier".to_string(),
                    ))
                }
            }
            Expr::Await(a) | Expr::AwaitPromised(a) => self.evaluate_expression(&a.argument),
            Expr::Ref(r) => self.evaluate_expression(&r.expr),
            Expr::MutRef(r) => self.evaluate_expression(&r.expr),
            _ => Err(RuntimeError::Unsupported(format!(
                "unsupported expression at runtime: {:?}",
                expr
            ))),
        }
    }

    fn call_value(&mut self, callee: Value, args: &[Value]) -> Result<Value, RuntimeError> {
        match callee {
            Value::Function(func) => {
                let mut call_scope = func.closure.clone();
                if let Some(name) = &func.name {
                    call_scope.define(name.clone(), Value::Function(func.clone()));
                }
                for (i, param) in func.params.iter().enumerate() {
                    call_scope.define(
                        param.clone(),
                        args.get(i).cloned().unwrap_or(Value::Undefined),
                    );
                }

                let mut rt = Runtime {
                    scope: call_scope,
                    globals: self.globals.clone(),
                    struct_defs: self.struct_defs.clone(),
                    class_defs: self.class_defs.clone(),
                };
                rt.execute_function_body(&func.body)
            }
            Value::NativeFunction(nf) => self.execute_native_function(&nf.name, args),
            _ => Err(RuntimeError::TypeError(
                "attempted to call a non-function value".to_string(),
            )),
        }
    }

    fn register_struct(&mut self, s: &StructDecl) {
        let fields = s.fields.iter().map(|f| f.id.sym.clone()).collect();
        let methods = s
            .methods
            .iter()
            .filter_map(|m| match &m.key {
                Expr::Identifier(id) => Some((id.sym.clone(), m.value.clone())),
                _ => None,
            })
            .collect();
        self.struct_defs
            .insert(s.id.sym.clone(), RuntimeStructDef { fields, methods });
    }

    fn register_class(&mut self, c: &ClassDecl) {
        let mut fields = Vec::new();
        let mut methods = HashMap::new();
        let mut constructor = None;

        for member in &c.body.body {
            match member {
                ClassMember::Field(f) => {
                    if let Expr::Identifier(id) = &f.key {
                        fields.push(id.sym.clone());
                    }
                }
                ClassMember::Method(m) => {
                    if let Expr::Identifier(id) = &m.key {
                        methods.insert(id.sym.clone(), m.value.clone());
                    }
                }
                ClassMember::Constructor(cons) => {
                    constructor = Some(cons.clone());
                }
                _ => {}
            }
        }

        self.class_defs.insert(
            c.id.sym.clone(),
            RuntimeClassDef {
                fields,
                methods,
                constructor,
            },
        );
    }

    fn register_enum(&mut self, e: &EnumDecl) -> Result<(), RuntimeError> {
        let mut values = HashMap::new();
        let mut next_numeric = Some(0.0);

        for member in &e.members {
            let value = if let Some(init) = &member.init {
                let value = self.evaluate_enum_initializer(init)?;
                next_numeric = match value {
                    Value::Number(n) => Some(n + 1.0),
                    _ => None,
                };
                value
            } else if let Some(current) = next_numeric {
                next_numeric = Some(current + 1.0);
                Value::Number(current)
            } else {
                return Err(RuntimeError::Unsupported(format!(
                    "enum member '{}' requires an explicit initializer after a non-numeric member",
                    member.id.sym
                )));
            };

            values.insert(member.id.sym.clone(), value);
        }

        self.scope.define(
            e.id.sym.clone(),
            Value::Object(Rc::new(RefCell::new(values))),
        );
        Ok(())
    }

    fn instantiate_struct(
        &mut self,
        name: &str,
        args: &[ExprOrSpread],
    ) -> Result<Value, RuntimeError> {
        let def = self
            .struct_defs
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))?;
        let obj = Rc::new(RefCell::new(HashMap::new()));

        if let Some(ExprOrSpread::Expr(init_expr)) = args.first() {
            let init = self.evaluate_expression(init_expr)?;
            if let Value::Object(init_map) = init {
                for (k, v) in init_map.borrow().iter() {
                    obj.borrow_mut().insert(k.clone(), v.clone());
                }
            }
        }

        for field in def.fields {
            obj.borrow_mut().entry(field).or_insert(Value::Undefined);
        }
        for (method_name, method_decl) in def.methods {
            let method_fn = self.bound_method_function(&method_decl, obj.clone());
            obj.borrow_mut().insert(method_name, method_fn);
        }

        Ok(Value::Object(obj))
    }

    fn instantiate_class(
        &mut self,
        name: &str,
        args: &[ExprOrSpread],
    ) -> Result<Value, RuntimeError> {
        let def = self
            .class_defs
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))?;
        let obj = Rc::new(RefCell::new(HashMap::new()));

        for field in def.fields {
            obj.borrow_mut().entry(field).or_insert(Value::Undefined);
        }
        for (method_name, method_decl) in def.methods {
            let method_fn = self.bound_method_function(&method_decl, obj.clone());
            obj.borrow_mut().insert(method_name, method_fn);
        }

        if let Some(constructor) = def.constructor {
            let mut ctor_scope = Scope::new();
            ctor_scope.define("this".to_string(), Value::Object(obj.clone()));
            for (idx, param) in constructor.params.iter().enumerate() {
                if let Pattern::Identifier(id) = &param.pat {
                    let value = match args.get(idx) {
                        Some(ExprOrSpread::Expr(e)) => self.evaluate_expression(e)?,
                        _ => Value::Undefined,
                    };
                    ctor_scope.define(id.name.sym.clone(), value);
                }
            }

            let mut ctor_rt = Runtime {
                scope: ctor_scope,
                globals: self.globals.clone(),
                struct_defs: self.struct_defs.clone(),
                class_defs: self.class_defs.clone(),
            };
            for stmt in &constructor.body.statements {
                match ctor_rt.execute_statement(stmt)? {
                    ExecOutcome::Normal => {}
                    ExecOutcome::Return(_) => break,
                    ExecOutcome::Break | ExecOutcome::Continue => {
                        return Err(RuntimeError::InvalidControlFlow(
                            "break/continue inside constructor".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(Value::Object(obj))
    }

    fn bound_method_function(
        &self,
        decl: &FunctionDecl,
        this_obj: Rc<RefCell<HashMap<String, Value>>>,
    ) -> Value {
        let mut closure = Scope::new();
        closure.define("this".to_string(), Value::Object(this_obj));
        Value::Function(self.function_value(decl, closure))
    }

    fn function_value(&self, decl: &FunctionDecl, closure: Scope) -> RcFunction {
        RcFunction {
            name: decl.id.as_ref().map(|id| id.sym.clone()),
            params: function_params(decl),
            body: decl.body.statements.clone(),
            closure,
        }
    }

    fn evaluate_enum_initializer(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(Literal::Number(n)) => Ok(Value::Number(n.value)),
            Expr::Literal(Literal::String(s)) => Ok(Value::String(s.value.clone())),
            Expr::Literal(Literal::Boolean(b)) => Ok(Value::Boolean(b.value)),
            Expr::Literal(Literal::Null(_)) => Ok(Value::Null),
            _ => Err(RuntimeError::Unsupported(
                "enum initializers must be literal values at runtime".to_string(),
            )),
        }
    }

    fn assign_member(&mut self, member: &MemberExpr, value: Value) -> Result<(), RuntimeError> {
        let object = self.evaluate_expression(&member.object)?;
        let property = if member.computed {
            let key = self.evaluate_expression(&member.property)?;
            self.value_to_string(&key)
        } else {
            self.property_key(&member.property)?
        };

        match object {
            Value::Object(map) => {
                map.borrow_mut().insert(property, value);
                Ok(())
            }
            Value::Array(_) => Err(RuntimeError::Unsupported(
                "array element assignment is not supported at runtime".to_string(),
            )),
            _ => Err(RuntimeError::TypeError(
                "member assignment on non-object value".to_string(),
            )),
        }
    }

    fn execute_function_body(&mut self, body: &[Stmt]) -> Result<Value, RuntimeError> {
        for stmt in body {
            match self.execute_statement(stmt)? {
                ExecOutcome::Normal => {}
                ExecOutcome::Return(v) => return Ok(v),
                ExecOutcome::Break => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "break outside loop".to_string(),
                    ))
                }
                ExecOutcome::Continue => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "continue outside loop".to_string(),
                    ))
                }
            }
        }
        Ok(Value::Undefined)
    }

    fn execute_native_function(&self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "console.log" => {
                let output: Vec<String> = args.iter().map(|v| self.value_to_string(v)).collect();
                println!("{}", output.join(" "));
                Ok(Value::Undefined)
            }
            "Math.abs" => one_number(args, |n| Value::Number(n.abs())),
            "Math.floor" => one_number(args, |n| Value::Number(n.floor())),
            "Math.ceil" => one_number(args, |n| Value::Number(n.ceil())),
            "Math.round" => one_number(args, |n| Value::Number(n.round())),
            "Math.sqrt" => one_number(args, |n| Value::Number(n.sqrt())),
            "Math.pow" => two_numbers(args, |a, b| Value::Number(a.powf(b))),
            "Math.max" => two_numbers(args, |a, b| Value::Number(a.max(b))),
            "Math.min" => two_numbers(args, |a, b| Value::Number(a.min(b))),
            _ => Ok(Value::Undefined),
        }
    }

    fn eval_binary(
        &self,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    ) -> Result<Value, RuntimeError> {
        match operator {
            BinaryOperator::Plus => match (&left, &right) {
                (Value::String(s), v) => Ok(Value::String(s.clone() + &self.value_to_string(v))),
                (v, Value::String(s)) => Ok(Value::String(self.value_to_string(v) + s)),
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                _ => Err(RuntimeError::TypeError(
                    "operator '+' expects numbers or strings".to_string(),
                )),
            },
            BinaryOperator::Minus => numeric_binop(left, right, |a, b| Value::Number(a - b)),
            BinaryOperator::Multiply => numeric_binop(left, right, |a, b| Value::Number(a * b)),
            BinaryOperator::Divide => numeric_binop(left, right, |a, b| Value::Number(a / b)),
            BinaryOperator::Modulo => numeric_binop(left, right, |a, b| Value::Number(a % b)),
            BinaryOperator::Equal | BinaryOperator::StrictEqual => {
                Ok(Value::Boolean(self.values_equal(&left, &right)))
            }
            BinaryOperator::NotEqual | BinaryOperator::StrictNotEqual => {
                Ok(Value::Boolean(!self.values_equal(&left, &right)))
            }
            BinaryOperator::LessThan => compare_numbers(left, right, |a, b| a < b),
            BinaryOperator::LessThanOrEqual => compare_numbers(left, right, |a, b| a <= b),
            BinaryOperator::GreaterThan => compare_numbers(left, right, |a, b| a > b),
            BinaryOperator::GreaterThanOrEqual => compare_numbers(left, right, |a, b| a >= b),
            _ => Err(RuntimeError::Unsupported(
                "unsupported binary operator".to_string(),
            )),
        }
    }

    fn to_iter_values(&self, value: &Value) -> Result<Vec<Value>, RuntimeError> {
        match value {
            Value::Array(values) => Ok(values.clone()),
            Value::Object(map) => Ok(map.borrow().values().cloned().collect()),
            _ => Err(RuntimeError::TypeError(
                "for..of expects an array or object value".to_string(),
            )),
        }
    }

    fn property_key(&mut self, expr: &Expr) -> Result<String, RuntimeError> {
        match expr {
            Expr::Identifier(id) => Ok(id.sym.clone()),
            Expr::Literal(Literal::String(s)) => Ok(s.value.clone()),
            Expr::Literal(Literal::Number(n)) => Ok(n.value.to_string()),
            _ => {
                let value = self.evaluate_expression(expr)?;
                Ok(self.value_to_string(&value))
            }
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            _ => false,
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Null | Value::Undefined => false,
            Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::NativeFunction(_) => {
                true
            }
        }
    }

    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Undefined => "undefined".to_string(),
            Value::Object(_) => "[object Object]".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.value_to_string(v)).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Function(_) => "[function]".to_string(),
            Value::NativeFunction(nf) => format!("[function: {}]", nf.name),
        }
    }
}

fn function_params(f: &FunctionDecl) -> Vec<String> {
    f.params
        .iter()
        .filter_map(|p| {
            if let Pattern::Identifier(id) = &p.pat {
                Some(id.name.sym.clone())
            } else {
                None
            }
        })
        .collect()
}

fn one_number<F>(args: &[Value], f: F) -> Result<Value, RuntimeError>
where
    F: Fn(f64) -> Value,
{
    if let Some(Value::Number(n)) = args.first() {
        Ok(f(*n))
    } else {
        Err(RuntimeError::TypeError(
            "expected a numeric argument".to_string(),
        ))
    }
}

fn two_numbers<F>(args: &[Value], f: F) -> Result<Value, RuntimeError>
where
    F: Fn(f64, f64) -> Value,
{
    if let (Some(Value::Number(a)), Some(Value::Number(b))) = (args.first(), args.get(1)) {
        Ok(f(*a, *b))
    } else {
        Err(RuntimeError::TypeError(
            "expected two numeric arguments".to_string(),
        ))
    }
}

fn numeric_binop<F>(left: Value, right: Value, f: F) -> Result<Value, RuntimeError>
where
    F: Fn(f64, f64) -> Value,
{
    if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
        Ok(f(*a, *b))
    } else {
        Err(RuntimeError::TypeError(
            "numeric binary operator expects numbers".to_string(),
        ))
    }
}

fn compare_numbers<F>(left: Value, right: Value, f: F) -> Result<Value, RuntimeError>
where
    F: Fn(f64, f64) -> bool,
{
    if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
        Ok(Value::Boolean(f(*a, *b)))
    } else {
        Err(RuntimeError::TypeError(
            "comparison operator expects numbers".to_string(),
        ))
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(String),
    TypeError(String),
    Unsupported(String),
    CompileOnlyFeature(String),
    Thrown(String),
    InvalidControlFlow(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
            RuntimeError::Unsupported(msg) => write!(f, "Unsupported runtime feature: {}", msg),
            RuntimeError::CompileOnlyFeature(msg) => write!(f, "Compile-only feature: {}", msg),
            RuntimeError::Thrown(msg) => write!(f, "Thrown value: {}", msg),
            RuntimeError::InvalidControlFlow(msg) => write!(f, "Invalid control flow: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub fn execute_ast(ast: &SourceFile) -> Result<Value, RuntimeError> {
    let mut runtime = Runtime::new();
    runtime.execute(ast)
}

fn detect_compile_only_feature_in_source(source: &SourceFile) -> Option<&'static str> {
    for stmt in &source.statements {
        if let Some(feature) = detect_compile_only_feature_in_stmt(stmt) {
            return Some(feature);
        }
    }
    None
}

fn detect_compile_only_feature_in_stmt(stmt: &Stmt) -> Option<&'static str> {
    match stmt {
        Stmt::Import(_) => Some(
            "ES module imports are not supported by `argon run`; use `argon compile --target js` instead",
        ),
        Stmt::Module(_) => Some(
            "interop module declarations are not supported by `argon run`; use `argon check` or `argon compile` instead",
        ),
        Stmt::Expr(e) => detect_compile_only_feature_in_expr(&e.expr),
        Stmt::Block(b) => b
            .statements
            .iter()
            .find_map(detect_compile_only_feature_in_stmt),
        Stmt::If(i) => detect_compile_only_feature_in_expr(&i.condition)
            .or_else(|| detect_compile_only_feature_in_stmt(&i.consequent))
            .or_else(|| i.alternate.as_deref().and_then(detect_compile_only_feature_in_stmt)),
        Stmt::Switch(s) => detect_compile_only_feature_in_expr(&s.discriminant).or_else(|| {
            s.cases.iter().find_map(|case| {
                case.test
                    .as_ref()
                    .and_then(detect_compile_only_feature_in_expr)
                    .or_else(|| case.consequent.iter().find_map(detect_compile_only_feature_in_stmt))
            })
        }),
        Stmt::For(f) => f
            .init
            .as_ref()
            .and_then(|init| match init {
                ForInit::Variable(v) => v
                    .declarations
                    .iter()
                    .find_map(|decl| decl.init.as_ref().and_then(detect_compile_only_feature_in_expr)),
                ForInit::Expr(expr) => detect_compile_only_feature_in_expr(expr),
            })
            .or_else(|| f.test.as_ref().and_then(detect_compile_only_feature_in_expr))
            .or_else(|| f.update.as_ref().and_then(detect_compile_only_feature_in_expr))
            .or_else(|| detect_compile_only_feature_in_stmt(&f.body)),
        Stmt::ForIn(f) => detect_compile_only_feature_in_expr(&f.right)
            .or_else(|| detect_compile_only_feature_in_stmt(&f.body)),
        Stmt::While(w) => detect_compile_only_feature_in_expr(&w.condition)
            .or_else(|| detect_compile_only_feature_in_stmt(&w.body)),
        Stmt::DoWhile(d) => detect_compile_only_feature_in_stmt(&d.body)
            .or_else(|| detect_compile_only_feature_in_expr(&d.condition)),
        Stmt::Loop(l) => detect_compile_only_feature_in_stmt(&l.body),
        Stmt::Return(r) => r
            .argument
            .as_ref()
            .and_then(detect_compile_only_feature_in_expr),
        Stmt::Throw(t) => detect_compile_only_feature_in_expr(&t.argument),
        Stmt::Try(t) => t
            .block
            .statements
            .iter()
            .find_map(detect_compile_only_feature_in_stmt)
            .or_else(|| {
                t.handler.as_ref().and_then(|handler| {
                    handler
                        .body
                        .statements
                        .iter()
                        .find_map(detect_compile_only_feature_in_stmt)
                })
            })
            .or_else(|| {
                t.finalizer.as_ref().and_then(|finalizer| {
                    finalizer
                        .statements
                        .iter()
                        .find_map(detect_compile_only_feature_in_stmt)
                })
            }),
        Stmt::Variable(v) => v
            .declarations
            .iter()
            .find_map(|decl| decl.init.as_ref().and_then(detect_compile_only_feature_in_expr)),
        Stmt::Function(f) | Stmt::AsyncFunction(f) => f
            .body
            .statements
            .iter()
            .find_map(detect_compile_only_feature_in_stmt),
        Stmt::Class(c) => c.body.body.iter().find_map(|member| match member {
            ClassMember::Field(field) => field
                .value
                .as_ref()
                .and_then(detect_compile_only_feature_in_expr),
            ClassMember::Method(method) => method
                .value
                .body
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt),
            ClassMember::Constructor(cons) => cons
                .body
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt),
            ClassMember::IndexSignature(_) => None,
        }),
        Stmt::Struct(s) => s.methods.iter().find_map(|method| {
            method
                .value
                .body
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt)
        }),
        Stmt::Export(e) => e
            .declaration
            .as_deref()
            .and_then(detect_compile_only_feature_in_stmt),
        _ => None,
    }
}

fn detect_compile_only_feature_in_expr(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::JsxElement(_) | Expr::JsxFragment(_) => Some(
            "JSX is not supported by `argon run`; use `argon compile --target js` instead",
        ),
        Expr::Import(_) => Some(
            "dynamic imports are not supported by `argon run`; use `argon compile --target js` instead",
        ),
        Expr::Template(t) => t
            .expressions
            .iter()
            .find_map(detect_compile_only_feature_in_expr),
        Expr::Member(m) => detect_compile_only_feature_in_expr(&m.object)
            .or_else(|| detect_compile_only_feature_in_expr(&m.property)),
        Expr::Call(c) => detect_compile_only_feature_in_expr(&c.callee).or_else(|| {
            c.arguments.iter().find_map(|arg| match arg {
                ExprOrSpread::Expr(expr) => detect_compile_only_feature_in_expr(expr),
                ExprOrSpread::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
            })
        }),
        Expr::New(n) => detect_compile_only_feature_in_expr(&n.callee).or_else(|| {
            n.arguments.iter().find_map(|arg| match arg {
                ExprOrSpread::Expr(expr) => detect_compile_only_feature_in_expr(expr),
                ExprOrSpread::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
            })
        }),
        Expr::Update(u) => detect_compile_only_feature_in_expr(&u.argument),
        Expr::Unary(u) => detect_compile_only_feature_in_expr(&u.argument),
        Expr::Binary(b) => detect_compile_only_feature_in_expr(&b.left)
            .or_else(|| detect_compile_only_feature_in_expr(&b.right)),
        Expr::Logical(l) => detect_compile_only_feature_in_expr(&l.left)
            .or_else(|| detect_compile_only_feature_in_expr(&l.right)),
        Expr::Conditional(c) => detect_compile_only_feature_in_expr(&c.test)
            .or_else(|| detect_compile_only_feature_in_expr(&c.consequent))
            .or_else(|| detect_compile_only_feature_in_expr(&c.alternate)),
        Expr::Assignment(a) => match &*a.left {
            AssignmentTarget::Simple(expr) => detect_compile_only_feature_in_expr(expr),
            AssignmentTarget::Member(member) => detect_compile_only_feature_in_expr(&Expr::Member(member.clone())),
            AssignmentTarget::Pattern(_) => None,
        }
        .or_else(|| detect_compile_only_feature_in_expr(&a.right)),
        Expr::Array(a) => a.elements.iter().find_map(|element| {
            element.as_ref().and_then(|element| match element {
                ExprOrSpread::Expr(expr) => detect_compile_only_feature_in_expr(expr),
                ExprOrSpread::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
            })
        }),
        Expr::Object(o) => o.properties.iter().find_map(|prop| match prop {
            ObjectProperty::Property(p) => detect_compile_only_feature_in_expr(&p.key).or_else(|| {
                match &p.value {
                    ExprOrSpread::Expr(expr) => detect_compile_only_feature_in_expr(expr),
                    ExprOrSpread::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
                }
            }),
            ObjectProperty::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
            ObjectProperty::Method(method)
            | ObjectProperty::Getter(method)
            | ObjectProperty::Setter(method) => method
                .value
                .body
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt),
            ObjectProperty::Shorthand(_) => None,
        }),
        Expr::ArrowFunction(arrow) => match &arrow.body {
            ArrowFunctionBody::Block(block) => block
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt),
            ArrowFunctionBody::Expr(expr) => detect_compile_only_feature_in_expr(expr),
        },
        Expr::Await(a)
        | Expr::AwaitPromised(a) => detect_compile_only_feature_in_expr(&a.argument),
        Expr::Ref(r) => detect_compile_only_feature_in_expr(&r.expr),
        Expr::MutRef(r) => detect_compile_only_feature_in_expr(&r.expr),
        Expr::TaggedTemplate(t) => detect_compile_only_feature_in_expr(&t.tag).or_else(|| {
            t.template
                .expressions
                .iter()
                .find_map(detect_compile_only_feature_in_expr)
        }),
        Expr::Parenthesized(p) => detect_compile_only_feature_in_expr(&p.expression),
        Expr::AsType(a) => detect_compile_only_feature_in_expr(&a.expression),
        Expr::TypeAssertion(a) => detect_compile_only_feature_in_expr(&a.expression),
        Expr::NonNull(n) => detect_compile_only_feature_in_expr(&n.expression),
        Expr::Chain(chain) => chain.expressions.iter().find_map(|element| match element {
            ChainElement::Call(call) => detect_compile_only_feature_in_expr(&Expr::Call(call.clone())),
            ChainElement::Member(member) => {
                detect_compile_only_feature_in_expr(&Expr::Member(member.clone()))
            }
            ChainElement::OptionalCall(call) => {
                detect_compile_only_feature_in_expr(&Expr::OptionalCall(call.clone()))
            }
            ChainElement::OptionalMember(member) => {
                detect_compile_only_feature_in_expr(&Expr::OptionalMember(member.clone()))
            }
        }),
        Expr::OptionalCall(call) => detect_compile_only_feature_in_expr(&call.callee).or_else(|| {
            call.arguments.iter().find_map(|arg| match arg {
                ExprOrSpread::Expr(expr) => detect_compile_only_feature_in_expr(expr),
                ExprOrSpread::Spread(spread) => detect_compile_only_feature_in_expr(&spread.argument),
            })
        }),
        Expr::OptionalMember(member) => detect_compile_only_feature_in_expr(&member.object)
            .or_else(|| detect_compile_only_feature_in_expr(&member.property)),
        _ => None,
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;
    use argon_parser::parse;

    #[test]
    fn executes_struct_method_call() {
        let source = r#"
struct Greeter {
    name: string;
    greet(): string with &this { return "Hello"; }
}
const g = Greeter { name: "World" };
const out = g.greet();
"#;
        let ast = parse(source).expect("parse should succeed");
        let mut runtime = Runtime::new();
        let result = runtime.execute(&ast);
        assert!(result.is_ok());
        assert!(matches!(runtime.scope.get("out"), Some(Value::String(_))));
    }

    #[test]
    fn executes_for_of_loop() {
        let source = r#"
const items = [1, 2, 3];
let sum = 0;
for (const item of items) {
    sum = sum + item;
}
"#;
        let ast = parse(source).expect("parse should succeed");
        let mut runtime = Runtime::new();
        let result = runtime.execute(&ast);
        assert!(result.is_ok());
        match runtime.scope.get("sum") {
            Some(Value::Number(n)) => assert_eq!(n, 6.0),
            _ => panic!("expected numeric sum"),
        }
    }

    #[test]
    fn executes_loop_with_break_and_continue() {
        let source = r#"
let i = 0;
let sum = 0;
loop {
    if (i >= 3) { break; }
    i = i + 1;
    if (i == 2) { continue; }
    sum = sum + i;
}
"#;
        let ast = parse(source).expect("parse should succeed");
        let mut runtime = Runtime::new();
        let result = runtime.execute(&ast);
        assert!(result.is_ok());
        match runtime.scope.get("sum") {
            Some(Value::Number(n)) => assert_eq!(n, 4.0),
            _ => panic!("expected numeric sum"),
        }
    }

    #[test]
    fn executes_match_statement() {
        let source = r#"
const x = 2;
let out = 0;
match (x) {
    1 => out = 10,
    2 => out = 20,
}
"#;
        let ast = parse(source).expect("parse should succeed");
        let mut runtime = Runtime::new();
        let result = runtime.execute(&ast);
        assert!(result.is_ok());
        match runtime.scope.get("out") {
            Some(Value::Number(n)) => assert_eq!(n, 20.0),
            _ => panic!("expected numeric match output"),
        }
    }

    #[test]
    fn executes_template_literal_interpolation() {
        let source = r#"
const name = "Argon";
const msg = `Hello ${name}`;
"#;
        let ast = parse(source).expect("parse should succeed");
        let mut runtime = Runtime::new();
        let result = runtime.execute(&ast);
        assert!(result.is_ok());
        match runtime.scope.get("msg") {
            Some(Value::String(s)) => assert_eq!(s, "Hello \"Argon\""),
            _ => panic!("expected interpolated template string"),
        }
    }
}
