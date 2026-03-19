//! Argon Runtime - Direct execution of Argon code

use argon_ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read as IoRead, Seek as IoSeek, Write as IoWrite};
use std::pin::Pin;
use std::rc::Rc;

/// A boxed future that resolves to a Value. Stored inside Rc<RefCell<Option<...>>>
/// so it can be taken once and awaited.
type BoxFuture = Pin<Box<dyn std::future::Future<Output = Value>>>;

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
    /// An async future that resolves to a Value.
    Future(Rc<RefCell<Option<BoxFuture>>>),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "Number({})", n),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Boolean(b) => write!(f, "Boolean({})", b),
            Value::Null => write!(f, "Null"),
            Value::Undefined => write!(f, "Undefined"),
            Value::Function(func) => write!(f, "Function({:?})", func.name),
            Value::NativeFunction(nf) => write!(f, "NativeFunction({})", nf.name),
            Value::Object(_) => write!(f, "Object(...)"),
            Value::Array(arr) => write!(f, "Array(len={})", arr.len()),
            Value::Future(_) => write!(f, "Future(...)"),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Number(n) => Value::Number(*n),
            Value::String(s) => Value::String(s.clone()),
            Value::Boolean(b) => Value::Boolean(*b),
            Value::Null => Value::Null,
            Value::Undefined => Value::Undefined,
            Value::Function(f) => Value::Function(f.clone()),
            Value::NativeFunction(nf) => Value::NativeFunction(nf.clone()),
            Value::Object(o) => Value::Object(o.clone()),
            Value::Array(a) => Value::Array(a.clone()),
            // Futures cannot be cloned — cloning produces Undefined
            Value::Future(_) => Value::Undefined,
        }
    }
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
    constructor: Option<Constructor>,
    #[allow(dead_code)]
    embodies: Vec<String>,
}

#[derive(Debug, Clone)]
struct RuntimeSkillDef {
    concrete_methods: HashMap<String, FunctionDecl>,
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

/// Shared resource table for OS handles (files, sockets, etc.)
/// that need to persist across function call boundaries.
#[derive(Debug, Default, Clone)]
struct ResourceTable {
    inner: Rc<RefCell<ResourceTableInner>>,
}

/// WebSocket connection variants (client uses MaybeTlsStream, server uses raw TcpStream).
enum WsConn {
    Client(tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>),
    Server(tungstenite::WebSocket<std::net::TcpStream>),
}

impl std::fmt::Debug for WsConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsConn::Client(_) => write!(f, "WsConn::Client(...)"),
            WsConn::Server(_) => write!(f, "WsConn::Server(...)"),
        }
    }
}

#[allow(clippy::result_large_err)]
impl WsConn {
    fn send(&mut self, msg: tungstenite::Message) -> Result<(), tungstenite::Error> {
        match self {
            WsConn::Client(ws) => ws.send(msg),
            WsConn::Server(ws) => ws.send(msg),
        }
    }
    fn read(&mut self) -> Result<tungstenite::Message, tungstenite::Error> {
        match self {
            WsConn::Client(ws) => ws.read(),
            WsConn::Server(ws) => ws.read(),
        }
    }
    fn close(
        &mut self,
        frame: Option<tungstenite::protocol::CloseFrame>,
    ) -> Result<(), tungstenite::Error> {
        match self {
            WsConn::Client(ws) => ws.close(frame),
            WsConn::Server(ws) => ws.close(frame),
        }
    }
    fn can_write(&self) -> bool {
        match self {
            WsConn::Client(ws) => ws.can_write(),
            WsConn::Server(ws) => ws.can_write(),
        }
    }
}

#[derive(Debug, Default)]
struct ResourceTableInner {
    file_handles: HashMap<u64, std::fs::File>,
    tcp_listeners: HashMap<u64, std::net::TcpListener>,
    tcp_streams: HashMap<u64, std::net::TcpStream>,
    udp_sockets: HashMap<u64, std::net::UdpSocket>,
    ws_connections: HashMap<u64, WsConn>,
    next_id: u64,
}

impl ResourceTable {
    fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ResourceTableInner {
                file_handles: HashMap::new(),
                tcp_listeners: HashMap::new(),
                tcp_streams: HashMap::new(),
                udp_sockets: HashMap::new(),
                ws_connections: HashMap::new(),
                next_id: 1,
            })),
        }
    }

    fn insert_file(&self, file: std::fs::File) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.file_handles.insert(id, file);
        id
    }

    fn with_file<F, R>(&self, id: u64, f: F) -> Result<R, RuntimeError>
    where
        F: FnOnce(&mut std::fs::File) -> Result<R, RuntimeError>,
    {
        let mut inner = self.inner.borrow_mut();
        let file = inner
            .file_handles
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid file handle".to_string()))?;
        f(file)
    }

    fn remove_file(&self, id: u64) -> Result<std::fs::File, RuntimeError> {
        self.inner
            .borrow_mut()
            .file_handles
            .remove(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid file handle".to_string()))
    }

    fn insert_tcp_listener(&self, listener: std::net::TcpListener) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.tcp_listeners.insert(id, listener);
        id
    }

    fn with_tcp_listener<F, R>(&self, id: u64, f: F) -> Result<R, RuntimeError>
    where
        F: FnOnce(&mut std::net::TcpListener) -> Result<R, RuntimeError>,
    {
        let mut inner = self.inner.borrow_mut();
        let listener = inner
            .tcp_listeners
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid tcp listener handle".to_string()))?;
        f(listener)
    }

    fn remove_tcp_listener(&self, id: u64) -> Result<std::net::TcpListener, RuntimeError> {
        self.inner
            .borrow_mut()
            .tcp_listeners
            .remove(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid tcp listener handle".to_string()))
    }

    fn insert_tcp_stream(&self, stream: std::net::TcpStream) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.tcp_streams.insert(id, stream);
        id
    }

    fn with_tcp_stream<F, R>(&self, id: u64, f: F) -> Result<R, RuntimeError>
    where
        F: FnOnce(&mut std::net::TcpStream) -> Result<R, RuntimeError>,
    {
        let mut inner = self.inner.borrow_mut();
        let stream = inner
            .tcp_streams
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid tcp stream handle".to_string()))?;
        f(stream)
    }

    fn remove_tcp_stream(&self, id: u64) -> Result<std::net::TcpStream, RuntimeError> {
        self.inner
            .borrow_mut()
            .tcp_streams
            .remove(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid tcp stream handle".to_string()))
    }

    fn insert_udp_socket(&self, socket: std::net::UdpSocket) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.udp_sockets.insert(id, socket);
        id
    }

    fn with_udp_socket<F, R>(&self, id: u64, f: F) -> Result<R, RuntimeError>
    where
        F: FnOnce(&mut std::net::UdpSocket) -> Result<R, RuntimeError>,
    {
        let mut inner = self.inner.borrow_mut();
        let socket = inner
            .udp_sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid udp socket handle".to_string()))?;
        f(socket)
    }

    fn remove_udp_socket(&self, id: u64) -> Result<std::net::UdpSocket, RuntimeError> {
        self.inner
            .borrow_mut()
            .udp_sockets
            .remove(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid udp socket handle".to_string()))
    }

    fn insert_ws(&self, ws: WsConn) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.ws_connections.insert(id, ws);
        id
    }

    fn with_ws<F, R>(&self, id: u64, f: F) -> Result<R, RuntimeError>
    where
        F: FnOnce(&mut WsConn) -> Result<R, RuntimeError>,
    {
        let mut inner = self.inner.borrow_mut();
        let ws = inner
            .ws_connections
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid websocket handle".to_string()))?;
        f(ws)
    }

    fn remove_ws(&self, id: u64) -> Result<WsConn, RuntimeError> {
        self.inner
            .borrow_mut()
            .ws_connections
            .remove(&id)
            .ok_or_else(|| RuntimeError::TypeError("invalid websocket handle".to_string()))
    }
}

pub struct Runtime {
    scope: Scope,
    globals: HashMap<String, Value>,
    struct_defs: HashMap<String, RuntimeStructDef>,
    skill_defs: HashMap<String, RuntimeSkillDef>,
    resources: ResourceTable,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let mut globals = HashMap::new();

        globals.insert(
            "print".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "print".to_string(),
            }),
        );
        globals.insert(
            "println".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "println".to_string(),
            }),
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

        // Register std:fs native functions
        for name in &[
            "readFile",
            "writeFile",
            "readBytes",
            "writeBytes",
            "appendFile",
            "readDir",
            "mkdir",
            "mkdirRecursive",
            "rmdir",
            "removeRecursive",
            "exists",
            "stat",
            "rename",
            "remove",
            "copy",
            "symlink",
            "readlink",
            "tempDir",
            "open",
        ] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("fs.{}", name),
                }),
            );
        }

        // Register async variants of std:fs, std:net, std:http
        for name in &[
            "readFileAsync",
            "writeFileAsync",
            "readBytesAsync",
            "writeBytesAsync",
            "appendFileAsync",
            "readDirAsync",
            "statAsync",
            "copyAsync",
        ] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("fs.{}", name),
                }),
            );
        }
        globals.insert(
            "connectAsync".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "net.connectAsync".to_string(),
            }),
        );
        for name in &[
            "getAsync",
            "postAsync",
            "putAsync",
            "delAsync",
            "requestAsync",
        ] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("http.{}", name),
                }),
            );
        }
        globals.insert(
            "wsConnectAsync".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "ws.wsConnectAsync".to_string(),
            }),
        );

        // Register std:net native functions
        for name in &["bind", "connect", "bindUdp", "resolve"] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("net.{}", name),
                }),
            );
        }

        // Register std:http native functions
        for name in &[
            "get",
            "post",
            "put",
            "del",
            "request",
            "createHeaders",
            "serve",
        ] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("http.{}", name),
                }),
            );
        }

        // Register std:async native functions
        globals.insert(
            "sleep".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "async.sleep".to_string(),
            }),
        );
        globals.insert(
            "spawn".to_string(),
            Value::NativeFunction(NativeFunction {
                name: "async.spawn".to_string(),
            }),
        );

        // Register std:ws native functions
        for name in &["wsConnect", "wsListen"] {
            globals.insert(
                name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("ws.{}", name),
                }),
            );
        }

        Self {
            scope: Scope::new(),
            globals,
            struct_defs: HashMap::new(),
            skill_defs: HashMap::new(),
            resources: ResourceTable::new(),
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
                if !f.is_intrinsic {
                    if let Some(id) = &f.id {
                        let func = self.function_value(f, self.scope.clone());
                        self.scope.define(id.sym.clone(), Value::Function(func));
                    }
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Struct(s) => {
                if !s.is_intrinsic {
                    self.register_struct(s);
                }
                Ok(ExecOutcome::Normal)
            }
            Stmt::Skill(sk) => {
                self.register_skill(sk);
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
                    if let Some(outcome) = self.try_execute_match_case(case, &disc)? {
                        return Ok(outcome);
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
            Stmt::Import(_)
            | Stmt::Export(_)
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
                // For method calls on objects (e.g., file.read(1024)),
                // pass the receiver object as the first arg to native functions.
                let (callee, receiver) = if let Expr::Member(m) = &*c.callee {
                    let obj = self.evaluate_expression(&m.object)?;
                    let property = if m.computed {
                        let key = self.evaluate_expression(&m.property)?;
                        self.value_to_string(&key)
                    } else {
                        self.property_key(&m.property)?
                    };
                    let callee_val = match &obj {
                        Value::Object(map) => map
                            .borrow()
                            .get(&property)
                            .cloned()
                            .unwrap_or(Value::Undefined),
                        Value::Array(values) => {
                            if let Ok(idx) = property.parse::<usize>() {
                                values.get(idx).cloned().unwrap_or(Value::Undefined)
                            } else {
                                Value::Undefined
                            }
                        }
                        _ => Value::Undefined,
                    };
                    (callee_val, Some(obj))
                } else {
                    (self.evaluate_expression(&c.callee)?, None)
                };

                let mut args = Vec::new();
                // For native function method calls, prepend the receiver as first arg
                if let (Value::NativeFunction(_), Some(recv)) = (&callee, receiver) {
                    args.push(recv);
                }
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
            Expr::Await(a) | Expr::AwaitPromised(a) => {
                let val = self.evaluate_expression(&a.argument)?;
                match val {
                    Value::Future(future_cell) => {
                        let future = future_cell.borrow_mut().take().ok_or_else(|| {
                            RuntimeError::TypeError("future already consumed".to_string())
                        })?;
                        // Block on the future using tokio if available
                        match tokio::runtime::Handle::try_current() {
                            Ok(h) => Ok(h.block_on(future)),
                            Err(_) => Ok(Value::Undefined), // no tokio runtime, fall back
                        }
                    }
                    other => Ok(other), // sync value, pass through
                }
            }
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
                    skill_defs: self.skill_defs.clone(),
                    resources: self.resources.clone(),
                };
                rt.execute_function_body(&func.body)
            }
            Value::NativeFunction(nf) => self.execute_native_function(&nf.name, args),
            _ => Err(RuntimeError::TypeError(
                "attempted to call a non-function value".to_string(),
            )),
        }
    }

    fn register_skill(&mut self, sk: &SkillDecl) {
        let concrete_methods = sk
            .items
            .iter()
            .filter_map(|item| match item {
                SkillItem::ConcreteMethod(m) => match &m.key {
                    Expr::Identifier(id) => Some((id.sym.clone(), m.value.clone())),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        self.skill_defs
            .insert(sk.id.sym.clone(), RuntimeSkillDef { concrete_methods });
    }

    fn register_struct(&mut self, s: &StructDecl) {
        let fields = s.fields.iter().map(|f| f.id.sym.clone()).collect();
        let mut methods: HashMap<String, FunctionDecl> = s
            .methods
            .iter()
            .filter_map(|m| match &m.key {
                Expr::Identifier(id) => Some((id.sym.clone(), m.value.clone())),
                _ => None,
            })
            .collect();

        // Merge concrete methods from embodied skills
        let embodies: Vec<String> = s.embodies.iter().map(|id| id.sym.clone()).collect();
        for skill_name in &embodies {
            if let Some(skill_def) = self.skill_defs.get(skill_name) {
                for (name, decl) in &skill_def.concrete_methods {
                    // Struct's own methods take priority over skill methods
                    methods.entry(name.clone()).or_insert_with(|| decl.clone());
                }
            }
        }

        let constructor = s.constructor.clone();
        self.struct_defs.insert(
            s.id.sym.clone(),
            RuntimeStructDef {
                fields,
                methods,
                constructor,
                embodies,
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

        if let Some(constructor) = &def.constructor {
            // Get the init object from args
            let init_value = if let Some(ExprOrSpread::Expr(arg)) = args.first() {
                self.evaluate_expression(arg)?
            } else {
                Value::Undefined
            };

            // Set up constructor scope with params extracted from init object
            let mut ctor_scope = Scope::new();
            ctor_scope.define("this".to_string(), Value::Object(obj.clone()));

            if let Value::Object(init_obj) = &init_value {
                for param in &constructor.params {
                    if let Pattern::Identifier(id) = &param.pat {
                        let val = init_obj
                            .borrow()
                            .get(&id.name.sym)
                            .cloned()
                            .unwrap_or(Value::Undefined);
                        ctor_scope.define(id.name.sym.clone(), val);
                    }
                }
            }

            let mut ctor_rt = Runtime {
                scope: ctor_scope,
                globals: self.globals.clone(),
                struct_defs: self.struct_defs.clone(),
                skill_defs: self.skill_defs.clone(),
                resources: self.resources.clone(),
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
        } else {
            // No constructor - unpack init object fields (original struct behavior)
            if let Some(ExprOrSpread::Expr(init_expr)) = args.first() {
                let init = self.evaluate_expression(init_expr)?;
                if let Value::Object(init_map) = init {
                    for (k, v) in init_map.borrow().iter() {
                        obj.borrow_mut().insert(k.clone(), v.clone());
                    }
                }
            }

            for field in &def.fields {
                obj.borrow_mut()
                    .entry(field.clone())
                    .or_insert(Value::Undefined);
            }
        }

        // Attach methods
        for (method_name, method_decl) in &def.methods {
            let method_fn = self.bound_method_function(method_decl, obj.clone());
            obj.borrow_mut().insert(method_name.clone(), method_fn);
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

    fn execute_native_function(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        match name {
            "print" => {
                let output: Vec<String> = args.iter().map(|v| self.value_to_string(v)).collect();
                print!("{}", output.join(" "));
                Ok(Value::Undefined)
            }
            "println" => {
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

            // --- std:fs ---
            "fs.readFile" => {
                let path = expect_string(args, 0, "readFile: path")?;
                match std::fs::read_to_string(&path) {
                    Ok(content) => Ok(make_ok(Value::String(content))),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.writeFile" => {
                let path = expect_string(args, 0, "writeFile: path")?;
                let content = expect_string(args, 1, "writeFile: content")?;
                match std::fs::write(&path, &content) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.readBytes" => {
                let path = expect_string(args, 0, "readBytes: path")?;
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let arr = bytes.into_iter().map(|b| Value::Number(b as f64)).collect();
                        Ok(make_ok(Value::Array(arr)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.writeBytes" => {
                let path = expect_string(args, 0, "writeBytes: path")?;
                let data = expect_byte_array(args, 1, "writeBytes: data")?;
                match std::fs::write(&path, &data) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.appendFile" => {
                let path = expect_string(args, 0, "appendFile: path")?;
                let content = expect_string(args, 1, "appendFile: content")?;
                match std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                {
                    Ok(mut file) => {
                        match std::io::Write::write_all(&mut file, content.as_bytes()) {
                            Ok(()) => Ok(make_ok(Value::Undefined)),
                            Err(e) => Ok(make_err(io_error_from(&e))),
                        }
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.readDir" => {
                let path = expect_string(args, 0, "readDir: path")?;
                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let mut result = Vec::new();
                        for entry in entries {
                            match entry {
                                Ok(e) => {
                                    let mut obj = HashMap::new();
                                    obj.insert(
                                        "name".to_string(),
                                        Value::String(e.file_name().to_string_lossy().to_string()),
                                    );
                                    let ft = e.file_type().ok();
                                    obj.insert(
                                        "isFile".to_string(),
                                        Value::Boolean(ft.as_ref().is_some_and(|t| t.is_file())),
                                    );
                                    obj.insert(
                                        "isDir".to_string(),
                                        Value::Boolean(ft.as_ref().is_some_and(|t| t.is_dir())),
                                    );
                                    obj.insert(
                                        "isSymlink".to_string(),
                                        Value::Boolean(ft.as_ref().is_some_and(|t| t.is_symlink())),
                                    );
                                    result.push(Value::Object(Rc::new(RefCell::new(obj))));
                                }
                                Err(e) => return Ok(make_err(io_error_from(&e))),
                            }
                        }
                        Ok(make_ok(Value::Array(result)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.mkdir" => {
                let path = expect_string(args, 0, "mkdir: path")?;
                match std::fs::create_dir(&path) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.mkdirRecursive" => {
                let path = expect_string(args, 0, "mkdirRecursive: path")?;
                match std::fs::create_dir_all(&path) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.rmdir" => {
                let path = expect_string(args, 0, "rmdir: path")?;
                match std::fs::remove_dir(&path) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.removeRecursive" => {
                let path = expect_string(args, 0, "removeRecursive: path")?;
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.exists" => {
                let path = expect_string(args, 0, "exists: path")?;
                Ok(make_ok(Value::Boolean(
                    std::path::Path::new(&path).exists(),
                )))
            }
            "fs.stat" => {
                let path = expect_string(args, 0, "stat: path")?;
                match std::fs::metadata(&path) {
                    Ok(meta) => {
                        let mut obj = HashMap::new();
                        obj.insert("size".to_string(), Value::Number(meta.len() as f64));
                        obj.insert("isFile".to_string(), Value::Boolean(meta.is_file()));
                        obj.insert("isDir".to_string(), Value::Boolean(meta.is_dir()));
                        obj.insert("isSymlink".to_string(), Value::Boolean(meta.is_symlink()));
                        obj.insert(
                            "modified".to_string(),
                            Value::Number(
                                meta.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map_or(0.0, |d| d.as_millis() as f64),
                            ),
                        );
                        obj.insert(
                            "created".to_string(),
                            Value::Number(
                                meta.created()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map_or(0.0, |d| d.as_millis() as f64),
                            ),
                        );
                        Ok(make_ok(Value::Object(Rc::new(RefCell::new(obj)))))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.rename" => {
                let from = expect_string(args, 0, "rename: from")?;
                let to = expect_string(args, 1, "rename: to")?;
                match std::fs::rename(&from, &to) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.remove" => {
                let path = expect_string(args, 0, "remove: path")?;
                match std::fs::remove_file(&path) {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.copy" => {
                let from = expect_string(args, 0, "copy: from")?;
                let to = expect_string(args, 1, "copy: to")?;
                match std::fs::copy(&from, &to) {
                    Ok(_) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.symlink" => {
                let target = expect_string(args, 0, "symlink: target")?;
                let path = expect_string(args, 1, "symlink: path")?;
                #[cfg(unix)]
                let result = std::os::unix::fs::symlink(&target, &path);
                #[cfg(windows)]
                let result = std::os::windows::fs::symlink_file(&target, &path);
                match result {
                    Ok(()) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.readlink" => {
                let path = expect_string(args, 0, "readlink: path")?;
                match std::fs::read_link(&path) {
                    Ok(target) => Ok(make_ok(Value::String(target.to_string_lossy().to_string()))),
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "fs.tempDir" => Ok(make_ok(Value::String(
                std::env::temp_dir().to_string_lossy().to_string(),
            ))),
            "fs.open" => {
                let path = expect_string(args, 0, "open: path")?;
                let mode =
                    expect_string(args, 1, "open: mode").unwrap_or_else(|_| "Read".to_string());
                let result = match mode.as_str() {
                    "Read" => std::fs::File::open(&path),
                    "Write" => std::fs::File::create(&path),
                    "Append" => std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&path),
                    "ReadWrite" => std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(&path),
                    "WriteAppend" => std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&path),
                    _ => {
                        return Ok(make_err(make_io_error(
                            "EINVAL",
                            &format!("invalid file mode: {}", mode),
                        )));
                    }
                };
                match result {
                    Ok(file) => {
                        let handle_id = self.resources.insert_file(file);
                        Ok(make_ok(self.make_file_object(handle_id)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }

            // --- File handle methods ---
            "File.read" => {
                let handle_id = expect_handle_id(args, 0)?;
                let max_bytes = expect_usize(args, 1, "File.read: maxBytes")?;
                self.resources.with_file(handle_id, |file| {
                    let mut buf = vec![0u8; max_bytes];
                    match file.read(&mut buf) {
                        Ok(n) => {
                            buf.truncate(n);
                            match String::from_utf8(buf) {
                                Ok(s) => Ok(make_ok(Value::String(s))),
                                Err(e) => Ok(make_err(make_io_error("EILSEQ", &e.to_string()))),
                            }
                        }
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "File.readBytes" => {
                let handle_id = expect_handle_id(args, 0)?;
                let max_bytes = expect_usize(args, 1, "File.readBytes: maxBytes")?;
                self.resources.with_file(handle_id, |file| {
                    let mut buf = vec![0u8; max_bytes];
                    match file.read(&mut buf) {
                        Ok(n) => {
                            buf.truncate(n);
                            let arr = buf.into_iter().map(|b| Value::Number(b as f64)).collect();
                            Ok(make_ok(Value::Array(arr)))
                        }
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "File.write" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_string(args, 1, "File.write: data")?;
                self.resources
                    .with_file(handle_id, |file| match file.write(data.as_bytes()) {
                        Ok(n) => Ok(make_ok(Value::Number(n as f64))),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    })
            }
            "File.writeBytes" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_byte_array(args, 1, "File.writeBytes: data")?;
                self.resources
                    .with_file(handle_id, |file| match file.write(&data) {
                        Ok(n) => Ok(make_ok(Value::Number(n as f64))),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    })
            }
            "File.seek" => {
                let handle_id = expect_handle_id(args, 0)?;
                let offset = expect_number(args, 1, "File.seek: offset")? as i64;
                let whence = expect_string(args, 2, "File.seek: whence")
                    .unwrap_or_else(|_| "Start".to_string());
                let seek_from = match whence.as_str() {
                    "Start" => std::io::SeekFrom::Start(offset as u64),
                    "Current" => std::io::SeekFrom::Current(offset),
                    "End" => std::io::SeekFrom::End(offset),
                    _ => {
                        return Ok(make_err(make_io_error(
                            "EINVAL",
                            &format!("invalid seek whence: {}", whence),
                        )));
                    }
                };
                self.resources
                    .with_file(handle_id, |file| match file.seek(seek_from) {
                        Ok(_) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    })
            }
            "File.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                match self.resources.remove_file(handle_id) {
                    Ok(_file) => Ok(make_ok(Value::Undefined)), // file dropped, closes fd
                    Err(e) => Err(e),
                }
            }

            // --- std:net ---
            "net.bind" => {
                let addr = expect_string(args, 0, "bind: addr")?;
                let port = expect_number(args, 1, "bind: port")? as u16;
                match std::net::TcpListener::bind(format!("{}:{}", addr, port)) {
                    Ok(listener) => {
                        let handle_id = self.resources.insert_tcp_listener(listener);
                        Ok(make_ok(self.make_tcp_listener_object(handle_id)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "net.connect" => {
                let addr = expect_string(args, 0, "connect: addr")?;
                let port = expect_number(args, 1, "connect: port")? as u16;
                match std::net::TcpStream::connect(format!("{}:{}", addr, port)) {
                    Ok(stream) => {
                        let handle_id = self.resources.insert_tcp_stream(stream);
                        Ok(make_ok(self.make_tcp_stream_object(handle_id)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "net.bindUdp" => {
                let addr = expect_string(args, 0, "bindUdp: addr")?;
                let port = expect_number(args, 1, "bindUdp: port")? as u16;
                match std::net::UdpSocket::bind(format!("{}:{}", addr, port)) {
                    Ok(socket) => {
                        let handle_id = self.resources.insert_udp_socket(socket);
                        Ok(make_ok(self.make_udp_socket_object(handle_id)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }
            "net.resolve" => {
                let hostname = expect_string(args, 0, "resolve: hostname")?;
                match std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", hostname)) {
                    Ok(addrs) => {
                        let ips: Vec<Value> =
                            addrs.map(|a| Value::String(a.ip().to_string())).collect();
                        Ok(make_ok(Value::Array(ips)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }

            // --- TcpListener methods ---
            "TcpListener.accept" => {
                let handle_id = expect_handle_id(args, 0)?;
                // Accept outside the borrow so we can insert the stream
                let accept_result = {
                    let mut inner = self.resources.inner.borrow_mut();
                    let listener = inner.tcp_listeners.get_mut(&handle_id).ok_or_else(|| {
                        RuntimeError::TypeError("invalid tcp listener handle".to_string())
                    })?;
                    listener
                        .accept()
                        .map_err(|e| RuntimeError::TypeError(e.to_string()))
                };
                match accept_result {
                    Ok((stream, _addr)) => {
                        let stream_id = self.resources.insert_tcp_stream(stream);
                        Ok(make_ok(self.make_tcp_stream_object(stream_id)))
                    }
                    Err(e) => Ok(make_err(make_io_error("EIO", &e.to_string()))),
                }
            }
            "TcpListener.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                match self.resources.remove_tcp_listener(handle_id) {
                    Ok(_) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Err(e),
                }
            }
            "TcpListener.localAddr" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources.with_tcp_listener(handle_id, |listener| {
                    match listener.local_addr() {
                        Ok(addr) => Ok(Value::String(addr.to_string())),
                        Err(e) => Err(RuntimeError::TypeError(e.to_string())),
                    }
                })
            }

            // --- TcpStream methods ---
            "TcpStream.read" => {
                let handle_id = expect_handle_id(args, 0)?;
                let max_bytes = expect_usize(args, 1, "TcpStream.read: maxBytes")?;
                self.resources.with_tcp_stream(handle_id, |stream| {
                    let mut buf = vec![0u8; max_bytes];
                    match stream.read(&mut buf) {
                        Ok(n) => {
                            buf.truncate(n);
                            match String::from_utf8(buf) {
                                Ok(s) => Ok(make_ok(Value::String(s))),
                                Err(e) => Ok(make_err(make_io_error("EILSEQ", &e.to_string()))),
                            }
                        }
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "TcpStream.write" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_string(args, 1, "TcpStream.write: data")?;
                self.resources.with_tcp_stream(handle_id, |stream| {
                    match stream.write(data.as_bytes()) {
                        Ok(n) => Ok(make_ok(Value::Number(n as f64))),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "TcpStream.shutdown" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources.with_tcp_stream(handle_id, |stream| {
                    match stream.shutdown(std::net::Shutdown::Write) {
                        Ok(()) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "TcpStream.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                match self.resources.remove_tcp_stream(handle_id) {
                    Ok(_) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Err(e),
                }
            }
            "TcpStream.peerAddr" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources
                    .with_tcp_stream(handle_id, |stream| match stream.peer_addr() {
                        Ok(addr) => Ok(Value::String(addr.to_string())),
                        Err(e) => Err(RuntimeError::TypeError(e.to_string())),
                    })
            }

            // --- UdpSocket methods ---
            "UdpSocket.sendTo" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_string(args, 1, "UdpSocket.sendTo: data")?;
                let addr = expect_string(args, 2, "UdpSocket.sendTo: addr")?;
                let port = expect_number(args, 3, "UdpSocket.sendTo: port")? as u16;
                self.resources.with_udp_socket(handle_id, |socket| {
                    match socket.send_to(data.as_bytes(), format!("{}:{}", addr, port)) {
                        Ok(n) => Ok(make_ok(Value::Number(n as f64))),
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "UdpSocket.recvFrom" => {
                let handle_id = expect_handle_id(args, 0)?;
                let max_bytes = expect_usize(args, 1, "UdpSocket.recvFrom: maxBytes")?;
                self.resources.with_udp_socket(handle_id, |socket| {
                    let mut buf = vec![0u8; max_bytes];
                    match socket.recv_from(&mut buf) {
                        Ok((n, addr)) => {
                            buf.truncate(n);
                            let mut msg = HashMap::new();
                            msg.insert(
                                "data".to_string(),
                                Value::String(String::from_utf8_lossy(&buf).to_string()),
                            );
                            msg.insert("addr".to_string(), Value::String(addr.ip().to_string()));
                            msg.insert("port".to_string(), Value::Number(addr.port() as f64));
                            Ok(make_ok(Value::Object(Rc::new(RefCell::new(msg)))))
                        }
                        Err(e) => Ok(make_err(io_error_from(&e))),
                    }
                })
            }
            "UdpSocket.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                match self.resources.remove_udp_socket(handle_id) {
                    Ok(_) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Err(e),
                }
            }

            // --- std:http ---
            "http.get" => {
                let url = expect_string(args, 0, "get: url")?;
                match ureq::get(&url).call() {
                    Ok(resp) => Ok(make_ok(self.ureq_response_to_value(resp))),
                    Err(e) => Ok(make_err(http_error_from(&e))),
                }
            }
            "http.post" => {
                let url = expect_string(args, 0, "post: url")?;
                let body = expect_string(args, 1, "post: body")?;
                let req = ureq::post(&url);
                let req = self.apply_headers_arg(req, args, 2);
                match req.send_string(&body) {
                    Ok(resp) => Ok(make_ok(self.ureq_response_to_value(resp))),
                    Err(e) => Ok(make_err(http_error_from(&e))),
                }
            }
            "http.put" => {
                let url = expect_string(args, 0, "put: url")?;
                let body = expect_string(args, 1, "put: body")?;
                let req = ureq::put(&url);
                let req = self.apply_headers_arg(req, args, 2);
                match req.send_string(&body) {
                    Ok(resp) => Ok(make_ok(self.ureq_response_to_value(resp))),
                    Err(e) => Ok(make_err(http_error_from(&e))),
                }
            }
            "http.del" => {
                let url = expect_string(args, 0, "del: url")?;
                match ureq::delete(&url).call() {
                    Ok(resp) => Ok(make_ok(self.ureq_response_to_value(resp))),
                    Err(e) => Ok(make_err(http_error_from(&e))),
                }
            }
            "http.request" => {
                // args[0] is a RequestOptions object
                if let Some(Value::Object(opts)) = args.first() {
                    let opts = opts.borrow();
                    let method = opts
                        .get("method")
                        .and_then(|v| {
                            if let Value::String(s) = v {
                                Some(strip_quotes(s))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| "GET".to_string());
                    let url = opts
                        .get("url")
                        .and_then(|v| {
                            if let Value::String(s) = v {
                                Some(strip_quotes(s))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    let body = opts
                        .get("body")
                        .and_then(|v| {
                            if let Value::String(s) = v {
                                Some(strip_quotes(s))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    let req = ureq::request(&method, &url);
                    let result = if body.is_empty() {
                        req.call()
                    } else {
                        req.send_string(&body)
                    };
                    match result {
                        Ok(resp) => Ok(make_ok(self.ureq_response_to_value(resp))),
                        Err(e) => Ok(make_err(http_error_from(&e))),
                    }
                } else {
                    Ok(make_err(make_io_error(
                        "EINVAL",
                        "request expects RequestOptions object",
                    )))
                }
            }
            "http.createHeaders" => {
                let obj = HashMap::new();
                Ok(self.make_headers_object(obj))
            }
            "http.serve" => {
                let port = expect_number(args, 0, "serve: port")? as u16;
                let handler = args.get(1).cloned().unwrap_or(Value::Undefined);

                let addr = format!("0.0.0.0:{}", port);
                match tiny_http::Server::http(&addr) {
                    Ok(server) => {
                        let bound_addr = server
                            .server_addr()
                            .to_ip()
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| addr.clone());

                        // Process requests in a loop until close is called.
                        // For now, process one request then return (blocking server
                        // needs threading which is out of scope for the interpreter).
                        // We'll handle a batch of requests then return the server handle.
                        // Store the server so it can be closed.
                        let server_rc = Rc::new(RefCell::new(Some(server)));
                        // Spawn a blocking request-handling loop
                        // For the runtime interpreter, we process requests synchronously
                        if let Value::Function(ref _func) = handler {
                            let srv = server_rc.clone();
                            // Process a single request (blocking server model)
                            let maybe_req = {
                                let guard = srv.borrow();
                                if let Some(ref server) = *guard {
                                    server
                                        .recv_timeout(std::time::Duration::from_millis(100))
                                        .ok()
                                        .flatten()
                                } else {
                                    None
                                }
                            };
                            {
                                if let Some(request) = maybe_req {
                                    // Build HttpRequest value
                                    let mut req_obj = HashMap::new();
                                    req_obj.insert(
                                        "method".to_string(),
                                        Value::String(request.method().to_string()),
                                    );
                                    req_obj.insert(
                                        "url".to_string(),
                                        Value::String(request.url().to_string()),
                                    );
                                    req_obj
                                        .insert("body".to_string(), Value::String(String::new()));

                                    let mut req_headers = HashMap::new();
                                    for header in request.headers() {
                                        req_headers.insert(
                                            header.field.as_str().as_str().to_lowercase(),
                                            Value::String(header.value.as_str().to_string()),
                                        );
                                    }
                                    req_obj.insert(
                                        "headers".to_string(),
                                        self.make_headers_object(req_headers),
                                    );

                                    // Build HttpResponse value
                                    let response_data = Rc::new(RefCell::new((
                                        200u16,
                                        HashMap::<String, String>::new(),
                                        String::new(),
                                    )));
                                    let rd = response_data.clone();

                                    let mut res_obj = HashMap::new();
                                    res_obj
                                        .insert("__response_data".to_string(), Value::Number(0.0)); // placeholder
                                    res_obj.insert(
                                        "setStatus".to_string(),
                                        Value::NativeFunction(NativeFunction {
                                            name: "HttpResponse.setStatus".to_string(),
                                        }),
                                    );
                                    res_obj.insert(
                                        "setHeader".to_string(),
                                        Value::NativeFunction(NativeFunction {
                                            name: "HttpResponse.setHeader".to_string(),
                                        }),
                                    );
                                    res_obj.insert(
                                        "send".to_string(),
                                        Value::NativeFunction(NativeFunction {
                                            name: "HttpResponse.send".to_string(),
                                        }),
                                    );

                                    let req_val = Value::Object(Rc::new(RefCell::new(req_obj)));
                                    let res_val = Value::Object(Rc::new(RefCell::new(res_obj)));

                                    // Call the handler
                                    let _ = self.call_value(handler.clone(), &[req_val, res_val]);

                                    // Send the response
                                    let (status, ref hdrs, ref body) = *rd.borrow();
                                    let mut response =
                                        tiny_http::Response::from_string(body.clone())
                                            .with_status_code(tiny_http::StatusCode(status));
                                    for (k, v) in hdrs {
                                        if let Ok(header) = tiny_http::Header::from_bytes(
                                            k.as_bytes(),
                                            v.as_bytes(),
                                        ) {
                                            response = response.with_header(header);
                                        }
                                    }
                                    let _ = request.respond(response);
                                }
                            }
                        }

                        // Return server handle
                        let mut obj = HashMap::new();
                        obj.insert("__addr".to_string(), Value::String(bound_addr));
                        obj.insert(
                            "close".to_string(),
                            Value::NativeFunction(NativeFunction {
                                name: "HttpServer.close".to_string(),
                            }),
                        );
                        obj.insert(
                            "addr".to_string(),
                            Value::NativeFunction(NativeFunction {
                                name: "HttpServer.addr".to_string(),
                            }),
                        );
                        let server_val = Value::Object(Rc::new(RefCell::new(obj)));
                        Ok(make_ok(server_val))
                    }
                    Err(e) => Ok(make_err(make_io_error("EADDRINUSE", &e.to_string()))),
                }
            }
            "HttpServer.close" => {
                // Server close is a no-op in the simple runtime model
                // (the server goes out of scope when the handle is dropped)
                Ok(make_ok(Value::Undefined))
            }
            "HttpServer.addr" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let map = obj.borrow();
                    match map.get("__addr") {
                        Some(Value::String(a)) => Ok(Value::String(a.clone())),
                        _ => Ok(Value::String(String::new())),
                    }
                } else {
                    Ok(Value::String(String::new()))
                }
            }

            // --- Headers methods ---
            "Headers.get" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let name = expect_string(args, 1, "Headers.get: name")?;
                    let map = obj.borrow();
                    match map.get(&name.to_lowercase()) {
                        Some(Value::String(v)) => Ok(make_ok(Value::String(v.clone()))),
                        _ => Ok(make_err(make_io_error("ENOENT", "header not found"))),
                    }
                } else {
                    Ok(make_err(make_io_error("EINVAL", "invalid headers object")))
                }
            }
            "Headers.set" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let name = expect_string(args, 1, "Headers.set: name")?;
                    let value = expect_string(args, 2, "Headers.set: value")?;
                    obj.borrow_mut()
                        .insert(name.to_lowercase(), Value::String(value));
                }
                Ok(Value::Undefined)
            }
            "Headers.has" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let name = expect_string(args, 1, "Headers.has: name")?;
                    Ok(Value::Boolean(
                        obj.borrow().contains_key(&name.to_lowercase()),
                    ))
                } else {
                    Ok(Value::Boolean(false))
                }
            }
            "Headers.delete" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let name = expect_string(args, 1, "Headers.delete: name")?;
                    obj.borrow_mut().remove(&name.to_lowercase());
                }
                Ok(Value::Undefined)
            }
            "Headers.entries" => {
                if let Some(Value::Object(obj)) = args.first() {
                    let entries: Vec<Value> = obj
                        .borrow()
                        .iter()
                        .filter(|(k, _)| {
                            !k.starts_with("__")
                                && !k.starts_with("get")
                                && !k.starts_with("set")
                                && !k.starts_with("has")
                                && !k.starts_with("delete")
                                && !k.starts_with("entries")
                        })
                        .map(|(k, v)| {
                            let mut entry = HashMap::new();
                            entry.insert("name".to_string(), Value::String(k.clone()));
                            entry.insert("value".to_string(), v.clone());
                            Value::Object(Rc::new(RefCell::new(entry)))
                        })
                        .collect();
                    Ok(Value::Array(entries))
                } else {
                    Ok(Value::Array(Vec::new()))
                }
            }

            // --- std:ws ---
            "ws.wsConnect" => {
                let url = expect_string(args, 0, "wsConnect: url")?;
                match tungstenite::connect(&url) {
                    Ok((ws, _response)) => {
                        let handle_id = self.resources.insert_ws(WsConn::Client(ws));
                        Ok(make_ok(self.make_ws_connection_object(handle_id)))
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        Ok(make_err(make_io_error("WS_ERROR", &msg)))
                    }
                }
            }
            "ws.wsListen" => {
                let addr = expect_string(args, 0, "wsListen: addr")?;
                let port = expect_number(args, 1, "wsListen: port")? as u16;
                match std::net::TcpListener::bind(format!("{}:{}", addr, port)) {
                    Ok(listener) => {
                        let handle_id = self.resources.insert_tcp_listener(listener);
                        Ok(make_ok(self.make_ws_server_object(handle_id)))
                    }
                    Err(e) => Ok(make_err(io_error_from(&e))),
                }
            }

            // --- WsConnection methods ---
            "WsConnection.send" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_string(args, 1, "WsConnection.send: data")?;
                self.resources.with_ws(handle_id, |ws| {
                    match ws.send(tungstenite::Message::Text(data.clone())) {
                        Ok(()) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(ws_error_from(&e))),
                    }
                })
            }
            "WsConnection.sendBytes" => {
                let handle_id = expect_handle_id(args, 0)?;
                let data = expect_byte_array(args, 1, "WsConnection.sendBytes: data")?;
                self.resources.with_ws(handle_id, |ws| {
                    match ws.send(tungstenite::Message::Binary(data.clone())) {
                        Ok(()) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(ws_error_from(&e))),
                    }
                })
            }
            "WsConnection.recv" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources.with_ws(handle_id, |ws| match ws.read() {
                    Ok(msg) => {
                        let mut obj = HashMap::new();
                        match msg {
                            tungstenite::Message::Text(text) => {
                                obj.insert("data".to_string(), Value::String(text.to_string()));
                                obj.insert("bytes".to_string(), Value::Array(Vec::new()));
                                obj.insert("isText".to_string(), Value::Boolean(true));
                                obj.insert("isBinary".to_string(), Value::Boolean(false));
                            }
                            tungstenite::Message::Binary(bytes) => {
                                obj.insert("data".to_string(), Value::String(String::new()));
                                let arr = bytes.iter().map(|b| Value::Number(*b as f64)).collect();
                                obj.insert("bytes".to_string(), Value::Array(arr));
                                obj.insert("isText".to_string(), Value::Boolean(false));
                                obj.insert("isBinary".to_string(), Value::Boolean(true));
                            }
                            tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => {
                                obj.insert("data".to_string(), Value::String(String::new()));
                                obj.insert("bytes".to_string(), Value::Array(Vec::new()));
                                obj.insert("isText".to_string(), Value::Boolean(false));
                                obj.insert("isBinary".to_string(), Value::Boolean(false));
                            }
                            tungstenite::Message::Close(_) => {
                                return Ok(make_err(make_io_error(
                                    "WS_CLOSED",
                                    "connection closed",
                                )));
                            }
                            _ => {
                                obj.insert("data".to_string(), Value::String(String::new()));
                                obj.insert("bytes".to_string(), Value::Array(Vec::new()));
                                obj.insert("isText".to_string(), Value::Boolean(false));
                                obj.insert("isBinary".to_string(), Value::Boolean(false));
                            }
                        }
                        Ok(make_ok(Value::Object(Rc::new(RefCell::new(obj)))))
                    }
                    Err(e) => Ok(make_err(ws_error_from(&e))),
                })
            }
            "WsConnection.ping" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources.with_ws(handle_id, |ws| {
                    match ws.send(tungstenite::Message::Ping(Vec::new())) {
                        Ok(()) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(ws_error_from(&e))),
                    }
                })
            }
            "WsConnection.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                let code = args
                    .get(1)
                    .and_then(|v| {
                        if let Value::Number(n) = v {
                            Some(*n as u16)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(1000);
                let reason = args
                    .get(2)
                    .and_then(|v| {
                        if let Value::String(s) = v {
                            Some(strip_quotes(s))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let close_frame = tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::from(code),
                    reason: reason.into(),
                };
                self.resources.with_ws(handle_id, |ws| {
                    match ws.close(Some(close_frame.clone())) {
                        Ok(()) => Ok(make_ok(Value::Undefined)),
                        Err(e) => Ok(make_err(ws_error_from(&e))),
                    }
                })?;
                // Remove from resource table
                let _ = self.resources.remove_ws(handle_id);
                Ok(make_ok(Value::Undefined))
            }
            "WsConnection.isOpen" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources
                    .with_ws(handle_id, |ws| Ok(Value::Boolean(ws.can_write())))
            }

            // --- WsServer methods ---
            "WsServer.accept" => {
                let handle_id = expect_handle_id(args, 0)?;
                // Accept TCP connection, then upgrade to WebSocket
                let accept_result = {
                    let mut inner = self.resources.inner.borrow_mut();
                    let listener = inner.tcp_listeners.get_mut(&handle_id).ok_or_else(|| {
                        RuntimeError::TypeError("invalid ws server handle".to_string())
                    })?;
                    listener
                        .accept()
                        .map_err(|e| RuntimeError::TypeError(e.to_string()))
                };
                match accept_result {
                    Ok((stream, _addr)) => match tungstenite::accept(stream) {
                        Ok(ws) => {
                            let ws_id = self.resources.insert_ws(WsConn::Server(ws));
                            Ok(make_ok(self.make_ws_connection_object(ws_id)))
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            Ok(make_err(make_io_error("WS_HANDSHAKE_ERROR", &msg)))
                        }
                    },
                    Err(e) => Ok(make_err(make_io_error("EIO", &e.to_string()))),
                }
            }
            "WsServer.close" => {
                let handle_id = expect_handle_id(args, 0)?;
                match self.resources.remove_tcp_listener(handle_id) {
                    Ok(_) => Ok(make_ok(Value::Undefined)),
                    Err(e) => Err(e),
                }
            }
            // --- std:async ---
            "async.sleep" => {
                let ms = expect_number(args, 0, "sleep: ms")? as u64;
                let future: BoxFuture = Box::pin(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    Value::Undefined
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "async.spawn" => {
                // In the single-threaded interpreter, spawn just runs the future immediately
                // (no true concurrency). Returns a Task-like value.
                Ok(Value::Undefined)
            }

            // --- Async fs ---
            "fs.readFileAsync" => {
                let path = expect_string(args, 0, "readFileAsync: path")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::fs::read_to_string(&path).await {
                        Ok(content) => make_ok(Value::String(content)),
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "fs.writeFileAsync" => {
                let path = expect_string(args, 0, "writeFileAsync: path")?;
                let content = expect_string(args, 1, "writeFileAsync: content")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::fs::write(&path, &content).await {
                        Ok(()) => make_ok(Value::Undefined),
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "fs.readBytesAsync" => {
                let path = expect_string(args, 0, "readBytesAsync: path")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::fs::read(&path).await {
                        Ok(bytes) => {
                            let arr = bytes.into_iter().map(|b| Value::Number(b as f64)).collect();
                            make_ok(Value::Array(arr))
                        }
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "fs.writeBytesAsync" => {
                let path = expect_string(args, 0, "writeBytesAsync: path")?;
                let data = expect_byte_array(args, 1, "writeBytesAsync: data")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::fs::write(&path, &data).await {
                        Ok(()) => make_ok(Value::Undefined),
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "fs.appendFileAsync" => {
                let path = expect_string(args, 0, "appendFileAsync: path")?;
                let content = expect_string(args, 1, "appendFileAsync: content")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&path)
                        .await
                    {
                        Ok(mut file) => {
                            use tokio::io::AsyncWriteExt;
                            match file.write_all(content.as_bytes()).await {
                                Ok(()) => make_ok(Value::Undefined),
                                Err(e) => make_err(io_error_from(&e)),
                            }
                        }
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "fs.readDirAsync" | "fs.statAsync" | "fs.copyAsync" => {
                // These async variants fall back to sync for now
                // (tokio::fs covers read/write but readdir/stat/copy are less common)
                let sync_name = name.trim_end_matches("Async");
                self.execute_native_function(sync_name, args)
            }

            // --- Async net ---
            "net.connectAsync" => {
                let addr = expect_string(args, 0, "connectAsync: addr")?;
                let port = expect_number(args, 1, "connectAsync: port")? as u16;
                let resources = self.resources.clone();
                let future: BoxFuture = Box::pin(async move {
                    let addr_str = format!("{}:{}", addr, port);
                    match tokio::net::TcpStream::connect(&addr_str).await {
                        Ok(stream) => {
                            // Convert tokio stream to std stream for our resource table
                            match stream.into_std() {
                                Ok(std_stream) => {
                                    let handle_id = resources.insert_tcp_stream(std_stream);
                                    let mut obj = HashMap::new();
                                    obj.insert(
                                        "__handle".to_string(),
                                        Value::Number(handle_id as f64),
                                    );
                                    for method in
                                        &["read", "write", "shutdown", "close", "peerAddr"]
                                    {
                                        obj.insert(
                                            method.to_string(),
                                            Value::NativeFunction(NativeFunction {
                                                name: format!("TcpStream.{}", method),
                                            }),
                                        );
                                    }
                                    make_ok(Value::Object(Rc::new(RefCell::new(obj))))
                                }
                                Err(e) => make_err(io_error_from(&e)),
                            }
                        }
                        Err(e) => make_err(io_error_from(&e)),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }

            // --- Async http ---
            "http.getAsync" => {
                let url = expect_string(args, 0, "getAsync: url")?;
                let future: BoxFuture = Box::pin(async move {
                    // Use blocking ureq in a spawn_blocking context
                    match tokio::task::spawn_blocking(move || {
                        ureq::get(&url).call().map_err(|e| ureq_error_to_pair(&e))
                    })
                    .await
                    {
                        Ok(Ok(resp)) => {
                            let status = resp.status() as f64;
                            let body = resp.into_string().unwrap_or_default();
                            let mut obj = HashMap::new();
                            obj.insert("status".to_string(), Value::Number(status));
                            obj.insert("body".to_string(), Value::String(body));
                            obj.insert(
                                "headers".to_string(),
                                Value::Object(Rc::new(RefCell::new(HashMap::new()))),
                            );
                            make_ok(Value::Object(Rc::new(RefCell::new(obj))))
                        }
                        Ok(Err((code, msg))) => make_err(make_io_error(&code, &msg)),
                        Err(e) => make_err(make_io_error("EIO", &e.to_string())),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "http.postAsync" => {
                let url = expect_string(args, 0, "postAsync: url")?;
                let body = expect_string(args, 1, "postAsync: body")?;
                let future: BoxFuture = Box::pin(async move {
                    match tokio::task::spawn_blocking(move || {
                        ureq::post(&url)
                            .send_string(&body)
                            .map_err(|e| ureq_error_to_pair(&e))
                    })
                    .await
                    {
                        Ok(Ok(resp)) => {
                            let status = resp.status() as f64;
                            let rbody = resp.into_string().unwrap_or_default();
                            let mut obj = HashMap::new();
                            obj.insert("status".to_string(), Value::Number(status));
                            obj.insert("body".to_string(), Value::String(rbody));
                            obj.insert(
                                "headers".to_string(),
                                Value::Object(Rc::new(RefCell::new(HashMap::new()))),
                            );
                            make_ok(Value::Object(Rc::new(RefCell::new(obj))))
                        }
                        Ok(Err((code, msg))) => make_err(make_io_error(&code, &msg)),
                        Err(e) => make_err(make_io_error("EIO", &e.to_string())),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }
            "http.putAsync" | "http.delAsync" | "http.requestAsync" => {
                // Fall back to sync for less common methods
                let sync_name = name.trim_end_matches("Async");
                self.execute_native_function(sync_name, args)
            }

            // --- Async ws ---
            "ws.wsConnectAsync" => {
                let url = expect_string(args, 0, "wsConnectAsync: url")?;
                let resources = self.resources.clone();
                let future: BoxFuture = Box::pin(async move {
                    match tokio::task::spawn_blocking(move || {
                        tungstenite::connect(&url).map_err(|e| e.to_string())
                    })
                    .await
                    {
                        Ok(Ok((ws, _response))) => {
                            let handle_id = resources.insert_ws(WsConn::Client(ws));
                            let mut obj = HashMap::new();
                            obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
                            for method in &["send", "sendBytes", "recv", "ping", "close", "isOpen"]
                            {
                                obj.insert(
                                    method.to_string(),
                                    Value::NativeFunction(NativeFunction {
                                        name: format!("WsConnection.{}", method),
                                    }),
                                );
                            }
                            make_ok(Value::Object(Rc::new(RefCell::new(obj))))
                        }
                        Ok(Err(e)) => make_err(make_io_error("WS_ERROR", &e)),
                        Err(e) => make_err(make_io_error("EIO", &e.to_string())),
                    }
                });
                Ok(Value::Future(Rc::new(RefCell::new(Some(future)))))
            }

            "WsServer.addr" => {
                let handle_id = expect_handle_id(args, 0)?;
                self.resources.with_tcp_listener(handle_id, |listener| {
                    match listener.local_addr() {
                        Ok(addr) => Ok(Value::String(addr.to_string())),
                        Err(e) => Err(RuntimeError::TypeError(e.to_string())),
                    }
                })
            }

            _ => Ok(Value::Undefined),
        }
    }

    /// Convert a ureq Response to an Argon Value.
    fn ureq_response_to_value(&self, resp: ureq::Response) -> Value {
        let status = resp.status() as f64;
        // Collect headers
        let mut headers_map = HashMap::new();
        for name in resp.headers_names() {
            if let Some(value) = resp.header(&name) {
                headers_map.insert(name.to_lowercase(), Value::String(value.to_string()));
            }
        }
        let headers = self.make_headers_object(headers_map);
        // Read body
        let body = resp.into_string().unwrap_or_default();

        let mut obj = HashMap::new();
        obj.insert("status".to_string(), Value::Number(status));
        obj.insert("headers".to_string(), headers);
        obj.insert("body".to_string(), Value::String(body));
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Apply headers from an argument (if present) to a ureq request.
    fn apply_headers_arg(
        &self,
        mut req: ureq::Request,
        args: &[Value],
        idx: usize,
    ) -> ureq::Request {
        if let Some(Value::Object(headers_obj)) = args.get(idx) {
            let map = headers_obj.borrow();
            for (key, val) in map.iter() {
                if !key.starts_with("__")
                    && !matches!(key.as_str(), "get" | "set" | "has" | "delete" | "entries")
                {
                    if let Value::String(v) = val {
                        req = req.set(key, &strip_quotes(v));
                    }
                }
            }
        }
        req
    }

    /// Create WsConnection intrinsic struct object.
    fn make_ws_connection_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
        for method in &["send", "sendBytes", "recv", "ping", "close", "isOpen"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("WsConnection.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create WsServer intrinsic struct object.
    fn make_ws_server_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
        for method in &["accept", "close", "addr"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("WsServer.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create a Headers intrinsic struct object.
    fn make_headers_object(&self, initial: HashMap<String, Value>) -> Value {
        let mut obj = initial;
        for method in &["get", "set", "has", "delete", "entries"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("Headers.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create TcpListener intrinsic struct object.
    fn make_tcp_listener_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
        for method in &["accept", "close", "localAddr"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("TcpListener.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create TcpStream intrinsic struct object.
    fn make_tcp_stream_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
        for method in &["read", "write", "shutdown", "close", "peerAddr"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("TcpStream.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create UdpSocket intrinsic struct object.
    fn make_udp_socket_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));
        for method in &["sendTo", "recvFrom", "close"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("UdpSocket.{}", method),
                }),
            );
        }
        Value::Object(Rc::new(RefCell::new(obj)))
    }

    /// Create a File intrinsic struct object with native method functions.
    /// The object carries a `__handle` field for resource lookup.
    fn make_file_object(&self, handle_id: u64) -> Value {
        let mut obj = HashMap::new();
        obj.insert("__handle".to_string(), Value::Number(handle_id as f64));

        for method in &["read", "readBytes", "write", "writeBytes", "seek", "close"] {
            obj.insert(
                method.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: format!("File.{}", method),
                }),
            );
        }

        Value::Object(Rc::new(RefCell::new(obj)))
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
            Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::NativeFunction(_)
            | Value::Future(_) => true,
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
            Value::Future(_) => "[future]".to_string(),
        }
    }

    fn try_execute_match_case(
        &mut self,
        case: &MatchCase,
        discriminant: &Value,
    ) -> Result<Option<ExecOutcome>, RuntimeError> {
        match &case.pattern {
            MatchPattern::Expr(pattern) => {
                let pat = self.evaluate_expression(pattern)?;
                if !self.values_equal(discriminant, &pat) {
                    return Ok(None);
                }
                if let Some(guard) = &case.guard {
                    let guard_value = self.evaluate_expression(guard)?;
                    if !self.is_truthy(&guard_value) {
                        return Ok(None);
                    }
                }
                Ok(Some(self.execute_statement(&case.consequent)?))
            }
            MatchPattern::Result(pattern) => {
                let Some(payload) = self.result_pattern_payload(discriminant, pattern.kind) else {
                    return Ok(None);
                };

                let binding_name = pattern.binding.sym.clone();
                let previous = self.scope.get(&binding_name);
                self.scope.define(binding_name.clone(), payload);

                let outcome = (|| -> Result<Option<ExecOutcome>, RuntimeError> {
                    if let Some(guard) = &case.guard {
                        let guard_value = self.evaluate_expression(guard)?;
                        if !self.is_truthy(&guard_value) {
                            return Ok(None);
                        }
                    }
                    Ok(Some(self.execute_statement(&case.consequent)?))
                })();

                match previous {
                    Some(value) => self.scope.set(binding_name, value),
                    None => {
                        self.scope.values.remove(&binding_name);
                    }
                }

                outcome
            }
        }
    }

    fn result_pattern_payload(
        &self,
        value: &Value,
        kind: ResultPatternKind,
    ) -> Option<Value> {
        let Value::Object(map) = value else {
            return None;
        };
        let map = map.borrow();
        let tag = match map.get("__tag") {
            Some(Value::String(tag)) => Some(tag.as_str()),
            _ => None,
        };

        match kind {
            ResultPatternKind::Ok => {
                if tag == Some("Ok")
                    || matches!(map.get("isOk"), Some(Value::Boolean(true)))
                {
                    map.get("value").cloned()
                } else {
                    None
                }
            }
            ResultPatternKind::Err => {
                if tag == Some("Err")
                    || matches!(map.get("isErr"), Some(Value::Boolean(true)))
                {
                    map.get("error").cloned()
                } else {
                    None
                }
            }
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

// --- fs helper functions ---

fn expect_string(args: &[Value], idx: usize, context: &str) -> Result<String, RuntimeError> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(strip_quotes(s)),
        Some(v) => Err(RuntimeError::TypeError(format!(
            "{} expects a string, got {:?}",
            context, v
        ))),
        None => Err(RuntimeError::TypeError(format!(
            "{} missing argument at index {}",
            context, idx
        ))),
    }
}

/// Strip surrounding quotes from a string value.
/// The Argon parser preserves quotes in StringLiteral values.
fn strip_quotes(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn expect_number(args: &[Value], idx: usize, context: &str) -> Result<f64, RuntimeError> {
    match args.get(idx) {
        Some(Value::Number(n)) => Ok(*n),
        Some(v) => Err(RuntimeError::TypeError(format!(
            "{} expects a number, got {:?}",
            context, v
        ))),
        None => Err(RuntimeError::TypeError(format!(
            "{} missing argument at index {}",
            context, idx
        ))),
    }
}

fn expect_usize(args: &[Value], idx: usize, context: &str) -> Result<usize, RuntimeError> {
    expect_number(args, idx, context).map(|n| n as usize)
}

fn expect_byte_array(args: &[Value], idx: usize, context: &str) -> Result<Vec<u8>, RuntimeError> {
    match args.get(idx) {
        Some(Value::Array(arr)) => {
            let mut bytes = Vec::with_capacity(arr.len());
            for v in arr {
                match v {
                    Value::Number(n) => bytes.push(*n as u8),
                    _ => {
                        return Err(RuntimeError::TypeError(format!(
                            "{} expects an array of numbers",
                            context
                        )))
                    }
                }
            }
            Ok(bytes)
        }
        _ => Err(RuntimeError::TypeError(format!(
            "{} expects a byte array",
            context
        ))),
    }
}

/// Extract the __handle ID from a File-method call's first arg (the `this` object).
fn expect_handle_id(args: &[Value], idx: usize) -> Result<u64, RuntimeError> {
    match args.get(idx) {
        Some(Value::Number(n)) => Ok(*n as u64),
        Some(Value::Object(obj)) => {
            let map = obj.borrow();
            match map.get("__handle") {
                Some(Value::Number(n)) => Ok(*n as u64),
                _ => Err(RuntimeError::TypeError(
                    "expected object with __handle".to_string(),
                )),
            }
        }
        _ => Err(RuntimeError::TypeError(
            "expected handle ID or object with __handle".to_string(),
        )),
    }
}

/// Create an Ok result value using the shared Result object shape.
fn make_ok(value: Value) -> Value {
    let mut obj = HashMap::new();
    obj.insert("__tag".to_string(), Value::String("Ok".to_string()));
    obj.insert("value".to_string(), value);
    obj.insert("isOk".to_string(), Value::Boolean(true));
    obj.insert("isErr".to_string(), Value::Boolean(false));
    Value::Object(Rc::new(RefCell::new(obj)))
}

/// Create an Err result value using the shared Result object shape.
fn make_err(error: Value) -> Value {
    let mut obj = HashMap::new();
    obj.insert("__tag".to_string(), Value::String("Err".to_string()));
    obj.insert("error".to_string(), error);
    obj.insert("isOk".to_string(), Value::Boolean(false));
    obj.insert("isErr".to_string(), Value::Boolean(true));
    Value::Object(Rc::new(RefCell::new(obj)))
}

/// Create an IoError value object from a Rust std::io::Error.
fn io_error_from(e: &std::io::Error) -> Value {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        std::io::ErrorKind::AlreadyExists => "EEXIST",
        std::io::ErrorKind::ConnectionRefused => "ECONNREFUSED",
        std::io::ErrorKind::ConnectionReset => "ECONNRESET",
        std::io::ErrorKind::AddrInUse => "EADDRINUSE",
        std::io::ErrorKind::TimedOut => "ETIMEDOUT",
        std::io::ErrorKind::BrokenPipe => "EPIPE",
        _ => "EIO",
    };
    make_io_error(code, &e.to_string())
}

/// Create an IoError from a tungstenite error.
fn ws_error_from(e: &tungstenite::Error) -> Value {
    let (code, message) = match e {
        tungstenite::Error::ConnectionClosed => ("WS_CLOSED", "connection closed".to_string()),
        tungstenite::Error::AlreadyClosed => ("WS_CLOSED", "already closed".to_string()),
        tungstenite::Error::Io(io_err) => {
            let code = match io_err.kind() {
                std::io::ErrorKind::ConnectionRefused => "ECONNREFUSED",
                std::io::ErrorKind::ConnectionReset => "ECONNRESET",
                _ => "EIO",
            };
            (code, io_err.to_string())
        }
        tungstenite::Error::Protocol(p) => ("WS_PROTOCOL_ERROR", p.to_string()),
        _ => ("WS_ERROR", e.to_string()),
    };
    make_io_error(code, &message)
}

/// Create an IoError from a ureq error.
fn http_error_from(e: &ureq::Error) -> Value {
    let (code, message) = ureq_error_to_pair(e);
    make_io_error(&code, &message)
}

/// Convert a ureq error to a Send-safe (String, String) pair.
/// Used inside spawn_blocking closures where Value (containing Rc) can't be sent.
fn ureq_error_to_pair(e: &ureq::Error) -> (String, String) {
    match e {
        ureq::Error::Status(status, resp) => {
            let msg = format!("HTTP {}: {}", status, resp.status_text());
            (format!("HTTP_{}", status), msg)
        }
        ureq::Error::Transport(t) => {
            let code = match t.kind() {
                ureq::ErrorKind::Dns => "ENOTFOUND",
                ureq::ErrorKind::ConnectionFailed => "ECONNREFUSED",
                ureq::ErrorKind::Io => "EIO",
                _ => "HTTP_ERROR",
            };
            (code.to_string(), t.to_string())
        }
    }
}

/// Create an IoError value object with given code and message.
fn make_io_error(code: &str, message: &str) -> Value {
    let mut obj = HashMap::new();
    obj.insert("code".to_string(), Value::String(code.to_string()));
    obj.insert("message".to_string(), Value::String(message.to_string()));
    Value::Object(Rc::new(RefCell::new(obj)))
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
    // Create a multi-threaded tokio runtime with 1 worker thread.
    // This allows us to call Handle::block_on from the main thread
    // to await futures without the "nested block_on" panic that
    // current_thread triggers.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|e| RuntimeError::Unsupported(format!("failed to create async runtime: {}", e)))?;

    // Enter the runtime context so Handle::try_current() works
    let _guard = rt.enter();

    let mut runtime = Runtime::new();
    let result = runtime.execute(ast);

    drop(_guard);
    drop(rt);

    result
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
        Stmt::Import(i) => {
            // std:* imports are no-ops at runtime (symbols are built-in globals)
            let source = i.source.value.trim_matches('"').trim_matches('\'');
            if source.starts_with("std:") {
                None
            } else {
                Some("ES module imports are not supported by `argon run`; use `argon compile --target js` instead")
            }
        }
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
        Stmt::Variable(v) => v
            .declarations
            .iter()
            .find_map(|decl| decl.init.as_ref().and_then(detect_compile_only_feature_in_expr)),
        Stmt::Function(f) | Stmt::AsyncFunction(f) => f
            .body
            .statements
            .iter()
            .find_map(detect_compile_only_feature_in_stmt),
        Stmt::Struct(s) => s
            .methods
            .iter()
            .find_map(|method| {
                method
                    .value
                    .body
                    .statements
                    .iter()
                    .find_map(detect_compile_only_feature_in_stmt)
            })
            .or_else(|| {
                s.constructor.as_ref().and_then(|cons| {
                    cons.body
                        .statements
                        .iter()
                        .find_map(detect_compile_only_feature_in_stmt)
                })
            }),
        Stmt::Skill(sk) => sk.items.iter().find_map(|item| match item {
            SkillItem::ConcreteMethod(m) => m
                .value
                .body
                .statements
                .iter()
                .find_map(detect_compile_only_feature_in_stmt),
            _ => None,
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
