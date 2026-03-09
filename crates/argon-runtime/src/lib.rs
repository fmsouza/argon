//! Argon Runtime - Direct execution of Argon code

use argon_ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Function(RcFunction),
    NativeFunction(NativeFunction),
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct RcFunction {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub closure: Scope,
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Scope {
    values: HashMap<String, Value>,
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

pub struct Runtime {
    scope: Scope,
    globals: HashMap<String, Value>,
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
        globals.insert("console".to_string(), Value::Object(console_obj));

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
        globals.insert("Math".to_string(), Value::Object(math_obj));

        Self {
            scope: Scope::new(),
            globals,
        }
    }

    pub fn execute(&mut self, source: &SourceFile) -> Result<Value, RuntimeError> {
        for stmt in &source.statements {
            self.execute_statement(stmt)?;
        }
        Ok(Value::Undefined)
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<Value, RuntimeError> {
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
                Ok(Value::Undefined)
            }
            Stmt::Function(f) => {
                if let Some(id) = &f.id {
                    let func = RcFunction {
                        params: f
                            .params
                            .iter()
                            .filter_map(|p| {
                                if let Pattern::Identifier(id) = &p.pat {
                                    Some(id.name.sym.clone())
                                } else {
                                    None
                                }
                            })
                            .collect(),
                        body: f.body.statements.clone(),
                        closure: self.scope.clone(),
                    };
                    self.scope.define(id.sym.clone(), Value::Function(func));
                }
                Ok(Value::Undefined)
            }
            Stmt::Expr(e) => self.evaluate_expression(&e.expr),
            Stmt::Return(r) => match &r.argument {
                Some(expr) => self.evaluate_expression(expr),
                None => Ok(Value::Undefined),
            },
            Stmt::If(i) => {
                let cond = self.evaluate_expression(&i.condition)?;
                if self.is_truthy(&cond) {
                    self.execute_statement(&i.consequent)?;
                } else if let Some(alt) = &i.alternate {
                    self.execute_statement(alt)?;
                }
                Ok(Value::Undefined)
            }
            Stmt::While(w) => {
                while self.is_truthy(&self.evaluate_expression(&w.condition)?) {
                    self.execute_statement(&w.body)?;
                }
                Ok(Value::Undefined)
            }
            Stmt::Block(b) => {
                for stmt in &b.statements {
                    self.execute_statement(stmt)?;
                }
                Ok(Value::Undefined)
            }
            _ => Ok(Value::Undefined),
        }
    }

    fn evaluate_expression(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Number(n) => Ok(Value::Number(n.value)),
                Literal::String(s) => Ok(Value::String(s.value.clone())),
                Literal::Boolean(b) => Ok(Value::Boolean(b.value)),
                Literal::Null(_) => Ok(Value::Null),
                _ => Ok(Value::Undefined),
            },
            Expr::Identifier(id) => self
                .scope
                .get(&id.sym)
                .or_else(|| self.globals.get(&id.sym).cloned())
                .ok_or_else(|| RuntimeError::UndefinedVariable(id.sym.clone())),
            Expr::Binary(b) => {
                let left = self.evaluate_expression(&b.left)?;
                let right = self.evaluate_expression(&b.right)?;
                match b.operator {
                    BinaryOperator::Plus => match (&left, &right) {
                        (Value::String(s), v) => {
                            Ok(Value::String(s.clone() + &self.value_to_string(v)))
                        }
                        (v, Value::String(s)) => Ok(Value::String(self.value_to_string(v) + s)),
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                        _ => Ok(Value::Undefined),
                    },
                    BinaryOperator::Minus => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Number(a - b))
                        } else {
                            Ok(Value::Undefined)
                        }
                    }
                    BinaryOperator::Multiply => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Number(a * b))
                        } else {
                            Ok(Value::Undefined)
                        }
                    }
                    BinaryOperator::Divide => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Number(a / b))
                        } else {
                            Ok(Value::Undefined)
                        }
                    }
                    BinaryOperator::Equal => Ok(Value::Boolean(false)),
                    BinaryOperator::NotEqual => Ok(Value::Boolean(true)),
                    BinaryOperator::LessThan => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Boolean(a < b))
                        } else {
                            Ok(Value::Boolean(false))
                        }
                    }
                    BinaryOperator::LessThanOrEqual => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Boolean(a <= b))
                        } else {
                            Ok(Value::Boolean(false))
                        }
                    }
                    BinaryOperator::GreaterThan => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Boolean(a > b))
                        } else {
                            Ok(Value::Boolean(false))
                        }
                    }
                    BinaryOperator::GreaterThanOrEqual => {
                        if let (Value::Number(a), Value::Number(b)) = (&left, &right) {
                            Ok(Value::Boolean(a >= b))
                        } else {
                            Ok(Value::Boolean(false))
                        }
                    }
                    _ => Ok(Value::Undefined),
                }
            }
            Expr::Unary(u) => {
                let value = self.evaluate_expression(&u.argument)?;
                match u.operator {
                    UnaryOperator::Minus => {
                        if let Value::Number(n) = value {
                            Ok(Value::Number(-n))
                        } else {
                            Ok(Value::Undefined)
                        }
                    }
                    UnaryOperator::LogicalNot => Ok(Value::Boolean(!self.is_truthy(&value))),
                    _ => Ok(Value::Undefined),
                }
            }
            Expr::Call(c) => {
                let callee = self.evaluate_expression(&c.callee)?;
                let args: Result<Vec<_>, _> = c
                    .arguments
                    .iter()
                    .filter_map(|a| {
                        if let ExprOrSpread::Expr(e) = a {
                            Some(self.evaluate_expression(e))
                        } else {
                            None
                        }
                    })
                    .collect();
                let args = args?;

                match callee {
                    Value::Function(func) => {
                        let mut call_scope = Scope::new();
                        for (i, param) in func.params.iter().enumerate() {
                            call_scope.define(
                                param.clone(),
                                args.get(i).cloned().unwrap_or(Value::Undefined),
                            );
                        }
                        let mut rt = Runtime {
                            scope: call_scope,
                            globals: self.globals.clone(),
                        };
                        rt.execute_function_body(&func.body)
                    }
                    Value::NativeFunction(nf) => self.execute_native_function(&nf.name, &args),
                    _ => Ok(Value::Undefined),
                }
            }
            Expr::Member(m) => {
                let obj = self.evaluate_expression(&m.object)?;
                if let Value::Object(map) = obj {
                    match &*m.property {
                        Expr::Identifier(id) => {
                            Ok(map.get(&id.sym).cloned().unwrap_or(Value::Undefined))
                        }
                        _ => Ok(Value::Undefined),
                    }
                } else {
                    Ok(Value::Undefined)
                }
            }
            _ => Ok(Value::Undefined),
        }
    }

    fn execute_function_body(&mut self, body: &[Stmt]) -> Result<Value, RuntimeError> {
        for stmt in body {
            match stmt {
                Stmt::Return(r) => {
                    return match &r.argument {
                        Some(expr) => self.evaluate_expression(expr),
                        None => Ok(Value::Undefined),
                    };
                }
                _ => {
                    self.execute_statement(stmt)?;
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
            "Math.abs" => {
                if let Some(Value::Number(n)) = args.get(0) {
                    Ok(Value::Number(n.abs()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.floor" => {
                if let Some(Value::Number(n)) = args.get(0) {
                    Ok(Value::Number(n.floor()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.ceil" => {
                if let Some(Value::Number(n)) = args.get(0) {
                    Ok(Value::Number(n.ceil()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.round" => {
                if let Some(Value::Number(n)) = args.get(0) {
                    Ok(Value::Number(n.round()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.sqrt" => {
                if let Some(Value::Number(n)) = args.get(0) {
                    Ok(Value::Number(n.sqrt()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.pow" => {
                if let (Some(Value::Number(a)), Some(Value::Number(b))) = (args.get(0), args.get(1))
                {
                    Ok(Value::Number(a.powf(*b)))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.max" => {
                if let (Some(Value::Number(a)), Some(Value::Number(b))) = (args.get(0), args.get(1))
                {
                    Ok(Value::Number(a.max(*b)))
                } else {
                    Ok(Value::Undefined)
                }
            }
            "Math.min" => {
                if let (Some(Value::Number(a)), Some(Value::Number(b))) = (args.get(0), args.get(1))
                {
                    Ok(Value::Number(a.min(*b)))
                } else {
                    Ok(Value::Undefined)
                }
            }
            _ => Ok(Value::Undefined),
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Undefined => false,
            _ => true,
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

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(String),
    TypeError(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub fn execute_ast(ast: &SourceFile) -> Result<Value, RuntimeError> {
    let mut runtime = Runtime::new();
    runtime.execute(ast)
}
