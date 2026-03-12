//! Argon - Type checker

use crate::types::{
    ClassDef, FieldDef, FunctionSig, MethodDef, StructDef, Type as CompType, TypeId,
    TypeInstantiator, TypeTable,
};
use argon_ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeCheckOutput {
    pub type_table: TypeTable,
    pub env: TypeEnvironment,
    pub expr_types: HashMap<Span, TypeId>,
}

#[derive(Debug, Clone)]
struct TypeAliasDef {
    type_params: Vec<argon_ast::TypeParam>,
    type_annotation: Box<argon_ast::Type>,
}

#[derive(Debug, Clone)]
struct GenericFunctionDef {
    sig: FunctionSig,
    type_params: Vec<argon_ast::TypeParam>,
}

#[derive(Debug, Clone)]
struct GenericStructDef {
    def: StructDef,
    type_params: Vec<argon_ast::TypeParam>,
}

#[derive(Debug, Clone)]
struct GenericClassDef {
    def: ClassDef,
    type_params: Vec<argon_ast::TypeParam>,
}

#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    vars: HashMap<String, TypeId>,
    structs: HashMap<String, StructDef>,
    generic_structs: HashMap<String, GenericStructDef>,
    classes: HashMap<String, ClassDef>,
    generic_classes: HashMap<String, GenericClassDef>,
    functions: HashMap<String, FunctionSig>,
    generic_functions: HashMap<String, GenericFunctionDef>,
    type_aliases: HashMap<String, TypeAliasDef>,
    type_params: HashMap<String, TypeId>,
    this_ty: Option<TypeId>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            structs: HashMap::new(),
            generic_structs: HashMap::new(),
            classes: HashMap::new(),
            generic_classes: HashMap::new(),
            functions: HashMap::new(),
            generic_functions: HashMap::new(),
            type_aliases: HashMap::new(),
            type_params: HashMap::new(),
            this_ty: None,
        }
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

    pub fn get_generic_struct(&self, name: &str) -> Option<&GenericStructDef> {
        self.generic_structs.get(name)
    }

    pub fn add_generic_struct(
        &mut self,
        name: String,
        def: StructDef,
        type_params: Vec<argon_ast::TypeParam>,
    ) {
        self.generic_structs
            .insert(name, GenericStructDef { def, type_params });
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassDef> {
        self.classes.get(name)
    }

    pub fn add_class(&mut self, name: String, def: ClassDef) {
        self.classes.insert(name, def);
    }

    pub fn get_generic_class(&self, name: &str) -> Option<&GenericClassDef> {
        self.generic_classes.get(name)
    }

    pub fn add_generic_class(
        &mut self,
        name: String,
        def: ClassDef,
        type_params: Vec<argon_ast::TypeParam>,
    ) {
        self.generic_classes
            .insert(name, GenericClassDef { def, type_params });
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSig) {
        self.functions.insert(name, sig);
    }

    pub fn get_generic_function(&self, name: &str) -> Option<&GenericFunctionDef> {
        self.generic_functions.get(name)
    }

    pub fn add_generic_function(
        &mut self,
        name: String,
        sig: FunctionSig,
        type_params: Vec<argon_ast::TypeParam>,
    ) {
        self.generic_functions
            .insert(name, GenericFunctionDef { sig, type_params });
    }

    fn get_type_alias(&self, name: &str) -> Option<&TypeAliasDef> {
        self.type_aliases.get(name)
    }

    fn add_type_alias(&mut self, name: String, def: TypeAliasDef) {
        self.type_aliases.insert(name, def);
    }

    pub fn add_type_param(&mut self, name: String, ty: TypeId) {
        self.type_params.insert(name, ty);
    }

    pub fn get_type_param(&self, name: &str) -> Option<TypeId> {
        self.type_params.get(name).copied()
    }

    pub fn set_this(&mut self, ty: Option<TypeId>) {
        self.this_ty = ty;
    }

    pub fn get_this(&self) -> Option<TypeId> {
        self.this_ty
    }

    pub fn child(&self) -> Self {
        Self {
            vars: self.vars.clone(),
            structs: self.structs.clone(),
            generic_structs: self.generic_structs.clone(),
            classes: self.classes.clone(),
            generic_classes: self.generic_classes.clone(),
            functions: self.functions.clone(),
            generic_functions: self.generic_functions.clone(),
            type_aliases: self.type_aliases.clone(),
            type_params: self.type_params.clone(),
            this_ty: self.this_ty,
        }
    }
}

pub struct TypeChecker {
    type_table: TypeTable,
    env: TypeEnvironment,
    expr_types: HashMap<Span, TypeId>,
    type_alias_cache: HashMap<String, TypeId>,
    resolving_type_aliases: Vec<String>,
    current_return_type: Option<TypeId>,
    errors: Vec<TypeError>,
    #[allow(dead_code)]
    warnings: Vec<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut type_table = TypeTable::new_with_builtins();
        let mut env = TypeEnvironment::new();

        env.add_function(
            "console.log".to_string(),
            FunctionSig {
                params: vec![type_table.any()],
                return_type: type_table.void(),
                is_async: false,
            },
        );

        Self {
            type_table,
            env,
            expr_types: HashMap::new(),
            type_alias_cache: HashMap::new(),
            resolving_type_aliases: Vec::new(),
            current_return_type: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn collect_declarations(&mut self, source: &SourceFile) {
        for stmt in &source.statements {
            match stmt {
                Stmt::TypeAlias(a) => {
                    self.env.add_type_alias(
                        a.id.sym.clone(),
                        TypeAliasDef {
                            type_params: a.type_params.clone(),
                            type_annotation: a.type_annotation.clone(),
                        },
                    );
                }
                Stmt::Struct(s) => {
                    let mut local_env = self.env.child();
                    for type_param in &s.type_params {
                        let constraint =
                            type_param.constraint.as_ref().map(|c| self.resolve_type(c));
                        let default = type_param.default.as_ref().map(|d| self.resolve_type(d));

                        let ty = self.type_table.type_param(crate::types::TypeParam {
                            name: type_param.name.sym.clone(),
                            constraint,
                            default,
                        });
                        local_env.add_type_param(type_param.name.sym.clone(), ty);
                    }

                    let old_env = std::mem::replace(&mut self.env, local_env);
                    let fields: Vec<FieldDef> = s
                        .fields
                        .iter()
                        .map(|f| FieldDef {
                            name: f.id.sym.clone(),
                            ty: self.resolve_type(&f.type_annotation),
                        })
                        .collect();
                    let methods: Vec<MethodDef> = s
                        .methods
                        .iter()
                        .filter_map(|m| {
                            let name = match &m.key {
                                Expr::Identifier(id) => id.sym.clone(),
                                _ => return None,
                            };
                            Some(MethodDef {
                                name,
                                sig: self.resolve_function_sig(&m.value),
                            })
                        })
                        .collect();
                    self.env = old_env;

                    let struct_def = StructDef {
                        name: s.id.sym.clone(),
                        fields,
                        methods,
                    };

                    if s.type_params.is_empty() {
                        self.env.add_struct(s.id.sym.clone(), struct_def);
                    } else {
                        self.env.add_generic_struct(
                            s.id.sym.clone(),
                            struct_def,
                            s.type_params.clone(),
                        );
                    }
                }
                Stmt::Class(c) => {
                    let mut local_env = self.env.child();
                    for type_param in &c.type_params {
                        let constraint = type_param
                            .constraint
                            .as_ref()
                            .map(|ct| self.resolve_type(ct));
                        let default = type_param.default.as_ref().map(|d| self.resolve_type(d));

                        let ty = self.type_table.type_param(crate::types::TypeParam {
                            name: type_param.name.sym.clone(),
                            constraint,
                            default,
                        });
                        local_env.add_type_param(type_param.name.sym.clone(), ty);
                    }

                    let old_env = std::mem::replace(&mut self.env, local_env);

                    let mut fields: Vec<FieldDef> = Vec::new();
                    let mut methods: Vec<MethodDef> = Vec::new();

                    for member in &c.body.body {
                        match member {
                            ClassMember::Field(f) => {
                                let name = match &f.key {
                                    Expr::Identifier(id) => id.sym.clone(),
                                    _ => continue,
                                };
                                let ty = f
                                    .type_annotation
                                    .as_ref()
                                    .map(|t| self.resolve_type(t))
                                    .unwrap_or_else(|| self.type_table.unknown());
                                fields.push(FieldDef { name, ty });
                            }
                            ClassMember::Method(m) => {
                                let name = match &m.key {
                                    Expr::Identifier(id) => id.sym.clone(),
                                    _ => continue,
                                };

                                let sig = if m.value.type_params.is_empty() {
                                    self.resolve_function_sig(&m.value)
                                } else {
                                    let mut method_env = self.env.child();
                                    for tp in &m.value.type_params {
                                        let constraint =
                                            tp.constraint.as_ref().map(|c| self.resolve_type(c));
                                        let default =
                                            tp.default.as_ref().map(|d| self.resolve_type(d));
                                        let ty =
                                            self.type_table.type_param(crate::types::TypeParam {
                                                name: tp.name.sym.clone(),
                                                constraint,
                                                default,
                                            });
                                        method_env.add_type_param(tp.name.sym.clone(), ty);
                                    }
                                    let old = std::mem::replace(&mut self.env, method_env);
                                    let sig = self.resolve_function_sig(&m.value);
                                    self.env = old;
                                    sig
                                };

                                methods.push(MethodDef { name, sig });
                            }
                            _ => {}
                        }
                    }

                    self.env = old_env;

                    let class_def = ClassDef {
                        name: c.id.sym.clone(),
                        fields,
                        methods,
                    };

                    if c.type_params.is_empty() {
                        self.env.add_class(c.id.sym.clone(), class_def);
                    } else {
                        self.env.add_generic_class(
                            c.id.sym.clone(),
                            class_def,
                            c.type_params.clone(),
                        );
                    }
                }
                Stmt::Function(f) | Stmt::AsyncFunction(f) => {
                    if let Some(id) = &f.id {
                        let sig = if f.type_params.is_empty() {
                            self.resolve_function_sig(f)
                        } else {
                            let mut local_env = self.env.child();
                            for type_param in &f.type_params {
                                let constraint =
                                    type_param.constraint.as_ref().map(|c| self.resolve_type(c));
                                let default =
                                    type_param.default.as_ref().map(|d| self.resolve_type(d));

                                let ty = self.type_table.type_param(crate::types::TypeParam {
                                    name: type_param.name.sym.clone(),
                                    constraint,
                                    default,
                                });
                                local_env.add_type_param(type_param.name.sym.clone(), ty);
                            }
                            let old_env = std::mem::replace(&mut self.env, local_env);
                            let sig = self.resolve_function_sig(f);
                            self.env = old_env;
                            sig
                        };

                        if f.type_params.is_empty() {
                            self.env.add_function(id.sym.clone(), sig);
                        } else {
                            self.env.add_generic_function(
                                id.sym.clone(),
                                sig,
                                f.type_params.clone(),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_function_sig(&mut self, f: &FunctionDecl) -> FunctionSig {
        let params = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| self.type_table.unknown())
            })
            .collect();
        let return_type = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or_else(|| self.type_table.void());

        FunctionSig {
            params,
            return_type,
            is_async: f.is_async,
        }
    }

    pub fn check(&mut self, source: &SourceFile) -> Result<(), TypeError> {
        self.collect_declarations(source);

        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(())
    }

    pub fn check_with_output(&mut self, source: &SourceFile) -> Result<TypeCheckOutput, TypeError> {
        self.check(source)?;
        Ok(TypeCheckOutput {
            type_table: self.type_table.clone(),
            env: self.env.clone(),
            expr_types: self.expr_types.clone(),
        })
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
            Stmt::Interface(_) => Ok(()),
            Stmt::Enum(_) => Ok(()),
            Stmt::TypeAlias(_) => Ok(()),
            Stmt::Labeled(l) => self.check_statement(&l.body),
            Stmt::With(w) => {
                self.infer_expression(&w.object);
                self.check_statement(&w.body)
            }
            Stmt::Debugger(_) => Ok(()),
            _ => Ok(()),
        }
    }

    fn check_variable(&mut self, stmt: &VariableStmt) -> Result<(), TypeError> {
        for decl in &stmt.declarations {
            if let Pattern::Identifier(id) = &decl.id {
                let declared_ty = if let Some(ref ann) = id.type_annotation {
                    self.resolve_type(ann)
                } else {
                    self.type_table.unknown()
                };

                if let Some(init) = &decl.init {
                    let init_ty = self.infer_expression(init);
                    if declared_ty != self.type_table.unknown()
                        && declared_ty != self.type_table.any()
                    {
                        self.unify(init_ty, declared_ty);
                        self.env.add_var(id.name.sym.clone(), declared_ty);
                    } else {
                        self.env.add_var(id.name.sym.clone(), init_ty);
                    }
                } else {
                    self.env.add_var(id.name.sym.clone(), declared_ty);
                }
            }
        }
        Ok(())
    }

    fn check_function(&mut self, f: &FunctionDecl) -> Result<(), TypeError> {
        let func_name =
            f.id.as_ref()
                .map(|i| i.sym.clone())
                .unwrap_or_else(|| "anonymous".to_string());

        let mut local_env = self.env.child();

        for type_param in &f.type_params {
            let constraint = type_param.constraint.as_ref().map(|c| self.resolve_type(c));
            let default = type_param.default.as_ref().map(|d| self.resolve_type(d));

            let ty = self.type_table.type_param(crate::types::TypeParam {
                name: type_param.name.sym.clone(),
                constraint,
                default,
            });
            local_env.add_type_param(type_param.name.sym.clone(), ty);
        }

        // Resolve param/return types with type params in scope.
        let old_env = std::mem::replace(&mut self.env, local_env);

        for p in &f.params {
            let ty =
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| self.type_table.unknown());
            if let Pattern::Identifier(id) = &p.pat {
                self.env.add_var(id.name.sym.clone(), ty);
            }
        }

        let return_ty = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or_else(|| self.type_table.void());

        if let Some(ref borrow) = f.borrow_annotation {
            if let Some(target) = &borrow.target {
                if target.sym == "this" && self.env.get_this().is_none() {
                    self.env.set_this(Some(self.type_table.object()));
                }
            }
        }

        let old_return = self.current_return_type;
        self.current_return_type = Some(return_ty);

        for stmt in &f.body.statements {
            self.check_statement(stmt)?;
        }
        self.env = old_env;

        self.current_return_type = old_return;

        if return_ty != self.type_table.void() {
            let has_return = f
                .body
                .statements
                .iter()
                .any(|s| matches!(s, Stmt::Return(_)));
            if !has_return {
                self.errors.push(TypeError::MissingReturn {
                    func: func_name,
                    expected: return_ty,
                });
            }
        }

        Ok(())
    }

    fn check_struct(&mut self, s: &StructDecl) -> Result<(), TypeError> {
        if s.methods.is_empty() {
            return Ok(());
        }

        let struct_ty = self
            .env
            .get_struct(&s.id.sym)
            .map(|d| self.type_table.struct_def(d.clone()))
            .or_else(|| {
                self.env
                    .get_generic_struct(&s.id.sym)
                    .map(|g| self.type_table.struct_def(g.def.clone()))
            })
            .unwrap_or_else(|| self.type_table.object());

        for method in &s.methods {
            let mut method_env = self.env.child();
            method_env.set_this(Some(struct_ty));
            let old_env = std::mem::replace(&mut self.env, method_env);
            self.check_function(&method.value)?;
            self.env = old_env;
        }

        Ok(())
    }

    fn check_class(&mut self, c: &ClassDecl) -> Result<(), TypeError> {
        let mut class_env = self.env.child();

        for type_param in &c.type_params {
            let constraint = type_param
                .constraint
                .as_ref()
                .map(|ct| self.resolve_type(ct));
            let default = type_param.default.as_ref().map(|d| self.resolve_type(d));
            let ty = self.type_table.type_param(crate::types::TypeParam {
                name: type_param.name.sym.clone(),
                constraint,
                default,
            });
            class_env.add_type_param(type_param.name.sym.clone(), ty);
        }

        let class_ty = self
            .env
            .get_class(&c.id.sym)
            .map(|d| self.type_table.class_def(d.clone()))
            .or_else(|| {
                self.env
                    .get_generic_class(&c.id.sym)
                    .map(|g| self.type_table.class_def(g.def.clone()))
            })
            .unwrap_or_else(|| self.type_table.object());

        for member in &c.body.body {
            match member {
                ClassMember::Method(m) => {
                    let mut local_env = class_env.child();
                    local_env.set_this(Some(class_ty));
                    let old_env = std::mem::replace(&mut self.env, local_env);
                    self.check_function(&m.value)?;
                    self.env = old_env;
                }
                ClassMember::Constructor(constr) => {
                    let mut local_env = class_env.child();
                    local_env.set_this(Some(class_ty));

                    for param in &constr.params {
                        if let Pattern::Identifier(id) = &param.pat {
                            let ty = param
                                .ty
                                .as_ref()
                                .map(|t| self.resolve_type(t))
                                .unwrap_or_else(|| self.type_table.unknown());
                            local_env.add_var(id.name.sym.clone(), ty);
                        }
                    }

                    let old_env = std::mem::replace(&mut self.env, local_env);
                    for stmt in &constr.body.statements {
                        self.check_statement(stmt)?;
                    }
                    self.env = old_env;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_return(&mut self, r: &ReturnStmt) -> Result<(), TypeError> {
        if let Some(expected) = self.current_return_type {
            if let Some(ref arg) = r.argument {
                let found = self.infer_expression(arg);
                self.unify(found, expected);
            } else {
                let found = self.type_table.void();
                self.unify(found, expected);
            }
        }
        Ok(())
    }

    fn check_if(&mut self, i: &IfStmt) -> Result<(), TypeError> {
        let cond_ty = self.infer_expression(&i.condition);
        let bool_ty = self.type_table.boolean();
        self.unify(cond_ty, bool_ty);

        self.check_statement(&i.consequent)?;
        if let Some(ref alt) = i.alternate {
            self.check_statement(alt)?;
        }
        Ok(())
    }

    fn check_while(&mut self, w: &WhileStmt) -> Result<(), TypeError> {
        let cond_ty = self.infer_expression(&w.condition);
        let bool_ty = self.type_table.boolean();
        self.unify(cond_ty, bool_ty);
        self.check_statement(&w.body)?;
        Ok(())
    }

    fn check_for(&mut self, f: &ForStmt) -> Result<(), TypeError> {
        if let Some(ref init) = f.init {
            match init {
                ForInit::Variable(v) => self.check_variable(v)?,
                ForInit::Expr(e) => {
                    self.infer_expression(e);
                }
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
        let cond_ty = self.infer_expression(&d.condition);
        let bool_ty = self.type_table.boolean();
        self.unify(cond_ty, bool_ty);
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

    fn infer_expression(&mut self, expr: &Expr) -> TypeId {
        let ty = match expr {
            Expr::Literal(lit) => self.infer_literal(lit),
            Expr::Identifier(id) => self.infer_identifier(id),
            Expr::Binary(b) => self.infer_binary(b),
            Expr::Unary(u) => self.infer_unary(u),
            Expr::Call(c) => self.infer_call(c),
            Expr::Member(m) => self.infer_member(m),
            Expr::Assignment(a) => self.infer_assignment(a),
            Expr::This(_) => self.env.get_this().unwrap_or_else(|| self.type_table.any()),
            Expr::New(n) => self.infer_new(n),
            Expr::Conditional(c) => self.infer_conditional(c),
            Expr::Logical(l) => self.infer_logical(l),
            Expr::Object(o) => self.infer_object(o),
            Expr::Array(a) => self.infer_array(a),
            Expr::Template(t) => self.infer_template(t),
            Expr::Function(f) => self.infer_function_expr(f),
            Expr::ArrowFunction(a) => self.infer_arrow_function(a),
            Expr::Await(a) => self.infer_await(a),
            Expr::Yield(y) => self.infer_yield(y),
            Expr::Update(u) => self.infer_update(u),
            Expr::TypeAssertion(t) => self.infer_type_assertion(t),
            Expr::AsType(a) => self.resolve_type(&a.type_annotation),
            Expr::Ref(r) => {
                let inner = self.infer_expression(&r.expr);
                self.type_table.add(CompType::Ref(inner))
            }
            Expr::MutRef(r) => {
                let inner = self.infer_expression(&r.expr);
                self.type_table.add(CompType::MutRef(inner))
            }
            Expr::Chain(c) => self.infer_chain(c),
            Expr::OptionalCall(c) => self.infer_optional_call(c),
            Expr::OptionalMember(m) => self.infer_optional_member(m),
            _ => self.type_table.unknown(),
        };

        self.expr_types.insert(expr.span().clone(), ty);
        ty
    }

    fn infer_literal(&mut self, lit: &Literal) -> TypeId {
        match lit {
            Literal::Number(_) => self.type_table.number(),
            Literal::String(_) => self.type_table.string(),
            Literal::Boolean(_) => self.type_table.boolean(),
            Literal::Null(_) => self.type_table.null(),
            Literal::Undefined(_) => self.type_table.undefined(),
            Literal::BigInt(_) => self.type_table.add(CompType::BigInt),
            Literal::RegExp(_) => self.type_table.object(),
        }
    }

    fn infer_identifier(&mut self, id: &Ident) -> TypeId {
        if let Some(ty) = self.env.get_var(&id.sym) {
            return ty;
        }
        if let Some(func) = self.env.get_function(&id.sym) {
            return self.type_table.add(CompType::Function(func.clone()));
        }
        self.type_table.unknown()
    }

    fn infer_binary(&mut self, b: &BinaryExpr) -> TypeId {
        let _left_ty = self.infer_expression(&b.left);
        let _right_ty = self.infer_expression(&b.right);

        match b.operator {
            BinaryOperator::Plus => {
                if _left_ty == self.type_table.string() || _right_ty == self.type_table.string() {
                    self.type_table.string()
                } else if _left_ty == self.type_table.number()
                    && _right_ty == self.type_table.number()
                {
                    self.type_table.number()
                } else {
                    self.type_table.any()
                }
            }
            BinaryOperator::Minus
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo
            | BinaryOperator::LeftShift
            | BinaryOperator::RightShift
            | BinaryOperator::UnsignedRightShift => self.type_table.number(),
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::StrictEqual
            | BinaryOperator::StrictNotEqual
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::Instanceof
            | BinaryOperator::In => self.type_table.boolean(),
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOr | BinaryOperator::BitwiseXor => {
                self.type_table.number()
            }
            _ => self.type_table.unknown(),
        }
    }

    fn infer_unary(&mut self, u: &UnaryExpr) -> TypeId {
        let _arg_ty = self.infer_expression(&u.argument);

        match u.operator {
            UnaryOperator::Minus | UnaryOperator::Plus => self.type_table.number(),
            UnaryOperator::LogicalNot => self.type_table.boolean(),
            UnaryOperator::BitwiseNot => self.type_table.number(),
            UnaryOperator::Typeof => self.type_table.string(),
            UnaryOperator::Void | UnaryOperator::Delete => self.type_table.undefined(),
        }
    }

    fn infer_call(&mut self, c: &CallExpr) -> TypeId {
        let arg_tys: Vec<TypeId> = c
            .arguments
            .iter()
            .filter_map(|arg| {
                if let ExprOrSpread::Expr(e) = arg {
                    Some(self.infer_expression(e))
                } else {
                    None
                }
            })
            .collect();

        if let Expr::Identifier(id) = &*c.callee {
            if id.sym == "console" {
                return self.type_table.void();
            }

            if let Some(generic_func) = self.env.get_generic_function(&id.sym).cloned() {
                let type_args: Vec<TypeId> = if !c.type_args.is_empty() {
                    c.type_args.iter().map(|t| self.resolve_type(t)).collect()
                } else {
                    self.infer_type_args(&generic_func, &arg_tys)
                };

                self.check_generic_constraints(&generic_func.type_params, &type_args);

                let mut instantiator = TypeInstantiator::new();
                for (param, arg) in generic_func.type_params.iter().zip(type_args.iter()) {
                    instantiator.add_substitution(param.name.sym.clone(), *arg);
                }

                let instantiated_sig = FunctionSig {
                    params: generic_func
                        .sig
                        .params
                        .iter()
                        .map(|&p| instantiator.instantiate(&mut self.type_table, p))
                        .collect(),
                    return_type: instantiator
                        .instantiate(&mut self.type_table, generic_func.sig.return_type),
                    is_async: generic_func.sig.is_async,
                };

                if arg_tys.len() == instantiated_sig.params.len() {
                    for (arg_ty, param_ty) in arg_tys.iter().zip(instantiated_sig.params.iter()) {
                        self.unify(*arg_ty, *param_ty);
                    }
                }

                return instantiated_sig.return_type;
            }
        }

        let callee_ty = self.infer_expression(&c.callee);

        if let Some(CompType::Function(sig)) = self.type_table.get(callee_ty) {
            return sig.return_type;
        }

        self.type_table.unknown()
    }

    fn infer_type_args(
        &mut self,
        generic_func: &GenericFunctionDef,
        arg_tys: &[TypeId],
    ) -> Vec<TypeId> {
        let mut type_args = Vec::with_capacity(generic_func.type_params.len());

        for (i, _param) in generic_func.type_params.iter().enumerate() {
            if i < arg_tys.len() && i < generic_func.sig.params.len() {
                let arg_ty = arg_tys[i];
                let param_ty = generic_func.sig.params[i];

                if let Some(CompType::TypeParam(_)) = self.type_table.get(param_ty) {
                    type_args.push(arg_ty);
                } else {
                    type_args.push(self.type_table.any());
                }
            } else {
                type_args.push(self.type_table.any());
            }
        }

        type_args
    }

    fn infer_member(&mut self, m: &MemberExpr) -> TypeId {
        let mut obj_ty = self.infer_expression(&m.object);

        if m.computed {
            self.infer_expression(&m.property);
            return self.type_table.unknown();
        }

        // Unwrap reference-like shells.
        loop {
            match self.type_table.get(obj_ty).cloned() {
                Some(CompType::Ref(inner))
                | Some(CompType::MutRef(inner))
                | Some(CompType::Shared(inner)) => obj_ty = inner,
                _ => break,
            }
        }

        let prop_name = match &*m.property {
            Expr::Identifier(id) => id.sym.as_str(),
            _ => return self.type_table.unknown(),
        };

        match self.type_table.get(obj_ty).cloned() {
            Some(CompType::Struct(def)) => def
                .fields
                .iter()
                .find(|f| f.name == prop_name)
                .map(|f| f.ty)
                .or_else(|| {
                    def.methods
                        .iter()
                        .find(|m| m.name == prop_name)
                        .map(|m| self.type_table.add(CompType::Function(m.sig.clone())))
                })
                .unwrap_or_else(|| self.type_table.unknown()),
            Some(CompType::Class(def)) => {
                if let Some(field) = def.fields.iter().find(|f| f.name == prop_name) {
                    return field.ty;
                }
                if let Some(method) = def.methods.iter().find(|m| m.name == prop_name) {
                    return self.type_table.add(CompType::Function(method.sig.clone()));
                }
                self.type_table.unknown()
            }
            _ => self.type_table.unknown(),
        }
    }

    fn infer_assignment(&mut self, a: &AssignmentExpr) -> TypeId {
        let right_ty = self.infer_expression(&a.right);

        match &*a.left {
            AssignmentTarget::Simple(expr) => {
                if let Expr::Identifier(id) = &**expr {
                    if let Some(existing_ty) = self.env.get_var(&id.sym) {
                        self.unify(right_ty, existing_ty);
                    } else {
                        self.env.add_var(id.sym.clone(), right_ty);
                    }
                }
            }
            AssignmentTarget::Member(_) => {}
            AssignmentTarget::Pattern(_) => {}
        }

        right_ty
    }

    fn infer_new(&mut self, n: &argon_ast::NewExpr) -> TypeId {
        let arg_tys: Vec<TypeId> = n
            .arguments
            .iter()
            .filter_map(|arg| {
                if let ExprOrSpread::Expr(e) = arg {
                    Some(self.infer_expression(e))
                } else {
                    None
                }
            })
            .collect();

        if let Expr::Identifier(id) = &*n.callee {
            if let Some(generic_struct) = self.env.get_generic_struct(&id.sym).cloned() {
                let type_args: Vec<TypeId> = if !n.type_args.is_empty() {
                    n.type_args.iter().map(|t| self.resolve_type(t)).collect()
                } else {
                    arg_tys.clone()
                };

                if type_args.len() != generic_struct.type_params.len() {
                    self.errors.push(TypeError::Invalid {
                        message: format!(
                            "generic struct '{}' expects {} type arguments, got {}",
                            id.sym,
                            generic_struct.type_params.len(),
                            type_args.len()
                        ),
                    });
                    return self.type_table.unknown();
                }

                self.check_generic_constraints(&generic_struct.type_params, &type_args);

                let mut instantiator = TypeInstantiator::new();
                for (param, arg) in generic_struct.type_params.iter().zip(type_args.iter()) {
                    instantiator.add_substitution(param.name.sym.clone(), *arg);
                }

                let instantiated_fields: Vec<FieldDef> = generic_struct
                    .def
                    .fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        ty: instantiator.instantiate(&mut self.type_table, f.ty),
                    })
                    .collect();
                let instantiated_methods: Vec<MethodDef> = generic_struct
                    .def
                    .methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|p| instantiator.instantiate(&mut self.type_table, *p))
                                .collect(),
                            return_type: instantiator
                                .instantiate(&mut self.type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();

                let struct_def = StructDef {
                    name: id.sym.clone(),
                    fields: instantiated_fields,
                    methods: instantiated_methods,
                };

                return self.type_table.add(CompType::Struct(struct_def));
            }

            if let Some(struct_def) = self.env.get_struct(&id.sym).cloned() {
                return self.type_table.add(CompType::Struct(struct_def));
            }

            if let Some(generic_class) = self.env.get_generic_class(&id.sym).cloned() {
                let type_args: Vec<TypeId> = if !n.type_args.is_empty() {
                    n.type_args.iter().map(|t| self.resolve_type(t)).collect()
                } else {
                    arg_tys.clone()
                };

                if type_args.len() != generic_class.type_params.len() {
                    self.errors.push(TypeError::Invalid {
                        message: format!(
                            "generic class '{}' expects {} type arguments, got {}",
                            id.sym,
                            generic_class.type_params.len(),
                            type_args.len()
                        ),
                    });
                    return self.type_table.unknown();
                }

                self.check_generic_constraints(&generic_class.type_params, &type_args);

                let mut instantiator = TypeInstantiator::new();
                for (param, arg) in generic_class.type_params.iter().zip(type_args.iter()) {
                    instantiator.add_substitution(param.name.sym.clone(), *arg);
                }

                let fields: Vec<FieldDef> = generic_class
                    .def
                    .fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        ty: instantiator.instantiate(&mut self.type_table, f.ty),
                    })
                    .collect();

                let methods: Vec<MethodDef> = generic_class
                    .def
                    .methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|p| instantiator.instantiate(&mut self.type_table, *p))
                                .collect(),
                            return_type: instantiator
                                .instantiate(&mut self.type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();

                let class_def = ClassDef {
                    name: id.sym.clone(),
                    fields,
                    methods,
                };

                return self.type_table.add(CompType::Class(class_def));
            }

            if let Some(class_def) = self.env.get_class(&id.sym).cloned() {
                return self.type_table.add(CompType::Class(class_def));
            }
        }

        for arg in &n.arguments {
            if let ExprOrSpread::Expr(e) = arg {
                self.infer_expression(e);
            }
        }

        self.type_table.object()
    }

    fn infer_conditional(&mut self, c: &ConditionalExpr) -> TypeId {
        let test_ty = self.infer_expression(&c.test);
        let bool_ty = self.type_table.boolean();
        self.unify(test_ty, bool_ty);

        let consequent_ty = self.infer_expression(&c.consequent);
        let alternate_ty = self.infer_expression(&c.alternate);

        if consequent_ty == alternate_ty {
            consequent_ty
        } else {
            self.type_table
                .add(CompType::Union(vec![consequent_ty, alternate_ty]))
        }
    }

    fn infer_logical(&mut self, l: &LogicalExpr) -> TypeId {
        let left_ty = self.infer_expression(&l.left);
        let right_ty = self.infer_expression(&l.right);

        match l.operator {
            LogicalOperator::And | LogicalOperator::Or => {
                if left_ty == right_ty {
                    left_ty
                } else {
                    self.type_table
                        .add(CompType::Union(vec![left_ty, right_ty]))
                }
            }
            LogicalOperator::NullishCoalescing => self
                .type_table
                .add(CompType::Union(vec![left_ty, right_ty])),
        }
    }

    fn infer_object(&mut self, o: &ObjectExpression) -> TypeId {
        for prop in &o.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    if let ExprOrSpread::Expr(e) = &p.value {
                        self.infer_expression(e);
                    }
                }
                ObjectProperty::Shorthand(id) => {
                    self.infer_identifier(id);
                }
                ObjectProperty::Spread(s) => {
                    self.infer_expression(&s.argument);
                }
                ObjectProperty::Method(m) => {
                    self.check_function(&m.value).ok();
                }
                ObjectProperty::Getter(_) | ObjectProperty::Setter(_) => {}
            }
        }

        self.type_table.object()
    }

    fn infer_array(&mut self, a: &ArrayExpression) -> TypeId {
        let mut element_ty = self.type_table.unknown();

        for elem in &a.elements {
            if let Some(ExprOrSpread::Expr(e)) = elem {
                let ty = self.infer_expression(e);
                if element_ty == self.type_table.unknown() {
                    element_ty = ty;
                } else if ty != element_ty {
                    element_ty = self.type_table.any();
                }
            }
        }

        self.type_table.add(CompType::Array(element_ty))
    }

    fn infer_template(&mut self, t: &TemplateLiteral) -> TypeId {
        for expr in &t.expressions {
            self.infer_expression(expr);
        }
        self.type_table.string()
    }

    fn infer_function_expr(&mut self, _f: &FunctionExpr) -> TypeId {
        self.type_table.any()
    }

    fn infer_arrow_function(&mut self, a: &ArrowFunctionExpr) -> TypeId {
        let mut local_env = self.env.child();

        for param in &a.params {
            let ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type(t))
                .unwrap_or_else(|| self.type_table.unknown());
            if let Pattern::Identifier(id) = &param.pat {
                local_env.add_var(id.name.sym.clone(), ty);
            }
        }

        let return_ty = a
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or_else(|| self.type_table.unknown());

        let func_sig = FunctionSig {
            params: a
                .params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or_else(|| self.type_table.unknown())
                })
                .collect(),
            return_type: return_ty,
            is_async: false,
        };

        self.type_table.add(CompType::Function(func_sig))
    }

    fn infer_await(&mut self, a: &AwaitExpr) -> TypeId {
        self.infer_expression(&a.argument)
    }

    fn infer_yield(&mut self, y: &YieldExpr) -> TypeId {
        if let Some(ref arg) = y.argument {
            self.infer_expression(arg);
        }
        self.type_table.unknown()
    }

    fn infer_update(&mut self, u: &UpdateExpr) -> TypeId {
        self.infer_expression(&u.argument);
        self.type_table.number()
    }

    fn infer_type_assertion(&mut self, t: &TypeAssertionExpr) -> TypeId {
        self.infer_expression(&t.expression);
        self.resolve_type(&t.type_annotation)
    }

    fn infer_chain(&mut self, c: &ChainExpr) -> TypeId {
        for elem in &c.expressions {
            match elem {
                ChainElement::Call(call) => {
                    self.infer_call(call);
                }
                ChainElement::Member(m) => {
                    self.infer_member(m);
                }
                ChainElement::OptionalCall(c) => {
                    self.infer_optional_call(c);
                }
                ChainElement::OptionalMember(m) => {
                    self.infer_optional_member(m);
                }
            }
        }
        self.type_table.unknown()
    }

    fn infer_optional_call(&mut self, c: &OptionalCallExpr) -> TypeId {
        self.infer_call(&CallExpr {
            callee: c.callee.clone(),
            arguments: c.arguments.clone(),
            type_args: vec![],
            span: c.span.clone(),
        })
    }

    fn infer_optional_member(&mut self, m: &OptionalMemberExpr) -> TypeId {
        self.infer_member(&MemberExpr {
            object: m.object.clone(),
            property: m.property.clone(),
            computed: m.computed,
            span: m.span.clone(),
        })
    }

    fn resolve_type(&mut self, ty: &argon_ast::Type) -> TypeId {
        match ty {
            argon_ast::Type::Parenthesized(inner) => self.resolve_type(inner),
            argon_ast::Type::Number(_) => self.type_table.number(),
            argon_ast::Type::String(_) => self.type_table.string(),
            argon_ast::Type::Boolean(_) => self.type_table.boolean(),
            argon_ast::Type::BigInt(_) => self.type_table.add(CompType::BigInt),
            argon_ast::Type::Symbol(_) => self.type_table.add(CompType::Symbol),
            argon_ast::Type::ThisType(_) => self.type_table.object(),
            argon_ast::Type::Optional(o) => {
                let inner = self.resolve_type(&o.ty);
                self.type_table.option(inner)
            }
            argon_ast::Type::Primitive(p) => match p {
                PrimitiveType::Number => self.type_table.number(),
                PrimitiveType::String => self.type_table.string(),
                PrimitiveType::Boolean => self.type_table.boolean(),
                PrimitiveType::Void => self.type_table.void(),
                PrimitiveType::Null => self.type_table.null(),
                PrimitiveType::Undefined => self.type_table.undefined(),
                PrimitiveType::Any => self.type_table.any(),
                PrimitiveType::Unknown => self.type_table.unknown(),
                PrimitiveType::Never => self.type_table.never(),
                PrimitiveType::Symbol => self.type_table.add(CompType::Symbol),
                PrimitiveType::BigInt => self.type_table.add(CompType::BigInt),
                PrimitiveType::Object => self.type_table.object(),
            },
            argon_ast::Type::Reference(r) => {
                if let TypeName::Ident(id) = &r.name {
                    if let Some(ty) = self.env.get_type_param(&id.sym) {
                        return ty;
                    }

                    if !r.type_args.is_empty() {
                        if let Some(generic_struct) = self.env.get_generic_struct(&id.sym).cloned()
                        {
                            let type_args: Vec<TypeId> =
                                r.type_args.iter().map(|t| self.resolve_type(t)).collect();

                            if type_args.len() != generic_struct.type_params.len() {
                                self.errors.push(TypeError::Invalid {
                                    message: format!(
                                        "generic struct '{}' expects {} type arguments, got {}",
                                        id.sym,
                                        generic_struct.type_params.len(),
                                        type_args.len()
                                    ),
                                });
                                return self.type_table.unknown();
                            }

                            self.check_generic_constraints(&generic_struct.type_params, &type_args);

                            let mut instantiator = TypeInstantiator::new();
                            for (param, arg) in
                                generic_struct.type_params.iter().zip(type_args.iter())
                            {
                                instantiator.add_substitution(param.name.sym.clone(), *arg);
                            }

                            let fields: Vec<FieldDef> = generic_struct
                                .def
                                .fields
                                .iter()
                                .map(|f| FieldDef {
                                    name: f.name.clone(),
                                    ty: instantiator.instantiate(&mut self.type_table, f.ty),
                                })
                                .collect();
                            let methods: Vec<MethodDef> = generic_struct
                                .def
                                .methods
                                .iter()
                                .map(|m| MethodDef {
                                    name: m.name.clone(),
                                    sig: FunctionSig {
                                        params: m
                                            .sig
                                            .params
                                            .iter()
                                            .map(|p| {
                                                instantiator.instantiate(&mut self.type_table, *p)
                                            })
                                            .collect(),
                                        return_type: instantiator
                                            .instantiate(&mut self.type_table, m.sig.return_type),
                                        is_async: m.sig.is_async,
                                    },
                                })
                                .collect();

                            return self.type_table.add(CompType::Struct(StructDef {
                                name: id.sym.clone(),
                                fields,
                                methods,
                            }));
                        }

                        if let Some(generic_class) = self.env.get_generic_class(&id.sym).cloned() {
                            let type_args: Vec<TypeId> =
                                r.type_args.iter().map(|t| self.resolve_type(t)).collect();

                            if type_args.len() != generic_class.type_params.len() {
                                self.errors.push(TypeError::Invalid {
                                    message: format!(
                                        "generic class '{}' expects {} type arguments, got {}",
                                        id.sym,
                                        generic_class.type_params.len(),
                                        type_args.len()
                                    ),
                                });
                                return self.type_table.unknown();
                            }

                            self.check_generic_constraints(&generic_class.type_params, &type_args);

                            let mut instantiator = TypeInstantiator::new();
                            for (param, arg) in
                                generic_class.type_params.iter().zip(type_args.iter())
                            {
                                instantiator.add_substitution(param.name.sym.clone(), *arg);
                            }

                            let fields: Vec<FieldDef> = generic_class
                                .def
                                .fields
                                .iter()
                                .map(|f| FieldDef {
                                    name: f.name.clone(),
                                    ty: instantiator.instantiate(&mut self.type_table, f.ty),
                                })
                                .collect();

                            let methods: Vec<MethodDef> = generic_class
                                .def
                                .methods
                                .iter()
                                .map(|m| MethodDef {
                                    name: m.name.clone(),
                                    sig: FunctionSig {
                                        params: m
                                            .sig
                                            .params
                                            .iter()
                                            .map(|p| {
                                                instantiator.instantiate(&mut self.type_table, *p)
                                            })
                                            .collect(),
                                        return_type: instantiator
                                            .instantiate(&mut self.type_table, m.sig.return_type),
                                        is_async: m.sig.is_async,
                                    },
                                })
                                .collect();

                            return self.type_table.add(CompType::Class(ClassDef {
                                name: id.sym.clone(),
                                fields,
                                methods,
                            }));
                        }
                    }

                    if r.type_args.is_empty() {
                        if let Some(alias) = self.env.get_type_alias(&id.sym).cloned() {
                            return self.resolve_type_alias(&id.sym, &alias);
                        }
                    }

                    if id.sym == "Option" && r.type_args.len() == 1 {
                        let inner = self.resolve_type(&r.type_args[0]);
                        return self.type_table.option(inner);
                    }
                    if id.sym == "Result" && r.type_args.len() == 2 {
                        let ok = self.resolve_type(&r.type_args[0]);
                        let err = self.resolve_type(&r.type_args[1]);
                        return self.type_table.result(ok, err);
                    }
                    if id.sym == "Promise" && r.type_args.len() == 1 {
                        let inner = self.resolve_type(&r.type_args[0]);
                        return self.type_table.promise(inner);
                    }

                    if let Some(ty) = self.type_table.get_by_name(&id.sym) {
                        return ty;
                    }
                    if let Some(struct_def) = self.env.get_struct(&id.sym) {
                        return self.type_table.add(CompType::Struct(struct_def.clone()));
                    }
                    if let Some(class_def) = self.env.get_class(&id.sym) {
                        return self.type_table.add(CompType::Class(class_def.clone()));
                    }
                }
                self.type_table.unknown()
            }
            argon_ast::Type::Array(arr) => {
                let elem_ty = self.resolve_type(&arr.elem_type);
                self.type_table.add(CompType::Array(elem_ty))
            }
            argon_ast::Type::Ref(r) => {
                let inner = self.resolve_type(&r.ty);
                self.type_table.add(CompType::Ref(inner))
            }
            argon_ast::Type::MutRef(r) => {
                let inner = self.resolve_type(&r.ty);
                self.type_table.add(CompType::MutRef(inner))
            }
            argon_ast::Type::Shared(s) => {
                let inner = self.resolve_type(&s.ty);
                self.type_table.add(CompType::Shared(inner))
            }
            argon_ast::Type::Function(f) => {
                let params = f.params.iter().map(|p| self.resolve_type(&p.ty)).collect();
                let return_type = self.resolve_type(&f.return_type);
                self.type_table.add(CompType::Function(FunctionSig {
                    params,
                    return_type,
                    is_async: false,
                }))
            }
            argon_ast::Type::Union(u) => {
                let types = u.types.iter().map(|t| self.resolve_type(t)).collect();
                self.type_table.add(CompType::Union(types))
            }
            argon_ast::Type::Intersection(i) => {
                let types = i.types.iter().map(|t| self.resolve_type(t)).collect();
                self.type_table.add(CompType::Intersection(types))
            }
            argon_ast::Type::Tuple(t) => {
                let types = t.types.iter().map(|t| self.resolve_type(t)).collect();
                self.type_table.add(CompType::Tuple(types))
            }
            argon_ast::Type::Object(_) => self.type_table.object(),
            argon_ast::Type::Any(_) => self.type_table.any(),
            argon_ast::Type::Unknown(_) => self.type_table.unknown(),
            argon_ast::Type::Never(_) => self.type_table.never(),
            argon_ast::Type::Void(_) => self.type_table.void(),
            argon_ast::Type::Null(_) => self.type_table.null(),
            argon_ast::Type::Undefined(_) => self.type_table.undefined(),
            _ => self.type_table.unknown(),
        }
    }

    fn resolve_type_alias(&mut self, name: &str, def: &TypeAliasDef) -> TypeId {
        if let Some(id) = self.type_alias_cache.get(name).copied() {
            return id;
        }

        if self.resolving_type_aliases.iter().any(|n| n == name) {
            self.errors.push(TypeError::Invalid {
                message: format!("recursive type alias '{}'", name),
            });
            return self.type_table.unknown();
        }

        if !def.type_params.is_empty() {
            self.errors.push(TypeError::Invalid {
                message: format!(
                    "generic type alias '{}' is not supported yet (missing instantiation)",
                    name
                ),
            });
            return self.type_table.unknown();
        }

        self.resolving_type_aliases.push(name.to_string());
        let resolved = self.resolve_type(&def.type_annotation);
        let _ = self.resolving_type_aliases.pop();

        self.type_alias_cache.insert(name.to_string(), resolved);
        resolved
    }

    fn unify(&mut self, found: TypeId, expected: TypeId) {
        if found == expected {
            return;
        }

        if found == self.type_table.any() || expected == self.type_table.any() {
            return;
        }

        if found == self.type_table.unknown() || expected == self.type_table.unknown() {
            return;
        }

        let found_type = {
            let ft = self.type_table.get(found);
            ft.cloned()
        };
        let expected_type = {
            let et = self.type_table.get(expected);
            et.cloned()
        };

        match (found_type, expected_type) {
            (Some(CompType::Number), Some(CompType::Number)) => {}
            (Some(CompType::String), Some(CompType::String)) => {}
            (Some(CompType::Boolean), Some(CompType::Boolean)) => {}
            (Some(CompType::Object), Some(CompType::Object)) => {}
            (Some(CompType::Struct(fs)), Some(CompType::Struct(es))) => {
                if fs.name != es.name {
                    self.errors.push(TypeError::Mismatch { found, expected });
                }
            }
            (Some(CompType::Class(fc)), Some(CompType::Class(ec))) => {
                if fc.name != ec.name {
                    self.errors.push(TypeError::Mismatch { found, expected });
                }
            }
            (Some(CompType::Array(f)), Some(CompType::Array(e))) => {
                self.unify(f, e);
            }
            (Some(CompType::Function(fs)), Some(CompType::Function(es))) => {
                if fs.params.len() != es.params.len() {
                    self.errors.push(TypeError::Mismatch { found, expected });
                    return;
                }
                for (fp, ep) in fs.params.iter().zip(es.params.iter()) {
                    self.unify(*fp, *ep);
                }
                self.unify(fs.return_type, es.return_type);
            }
            (Some(CompType::Union(types)), _) | (_, Some(CompType::Union(types))) => {
                let types_copy = types.clone();
                for ty in types_copy {
                    self.unify(ty, if found == ty { expected } else { found });
                }
            }
            _ => {
                self.errors.push(TypeError::Mismatch { found, expected });
            }
        }
    }

    fn is_assignable(&self, found: TypeId, expected: TypeId) -> bool {
        if found == expected {
            return true;
        }

        // Treat `any` and `unknown` as permissive for now (matches existing unify behavior).
        let any = self.type_table.get_by_name("any");
        let unknown = self.type_table.get_by_name("unknown");

        if any == Some(found) || any == Some(expected) {
            return true;
        }
        if unknown == Some(found) || unknown == Some(expected) {
            return true;
        }

        match (self.type_table.get(found), self.type_table.get(expected)) {
            (Some(CompType::Number), Some(CompType::Number)) => true,
            (Some(CompType::String), Some(CompType::String)) => true,
            (Some(CompType::Boolean), Some(CompType::Boolean)) => true,
            (Some(CompType::Null), Some(CompType::Null)) => true,
            (Some(CompType::Undefined), Some(CompType::Undefined)) => true,
            (Some(CompType::Void), Some(CompType::Void)) => true,
            (Some(CompType::Array(f)), Some(CompType::Array(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Ref(f)), Some(CompType::Ref(e))) => self.is_assignable(*f, *e),
            (Some(CompType::MutRef(f)), Some(CompType::MutRef(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Shared(f)), Some(CompType::Shared(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Struct(fs)), Some(CompType::Struct(es))) => fs.name == es.name,
            (Some(CompType::Class(fc)), Some(CompType::Class(ec))) => fc.name == ec.name,
            (Some(CompType::Union(types)), _) => {
                types.iter().all(|t| self.is_assignable(*t, expected))
            }
            (_, Some(CompType::Union(types))) => {
                types.iter().any(|t| self.is_assignable(found, *t))
            }
            _ => false,
        }
    }

    fn check_generic_constraints(&mut self, params: &[argon_ast::TypeParam], type_args: &[TypeId]) {
        for (param, arg) in params.iter().zip(type_args.iter()) {
            if let Some(constraint_ast) = &param.constraint {
                let constraint = self.resolve_type(constraint_ast);
                if !self.is_assignable(*arg, constraint) {
                    self.errors.push(TypeError::ConstraintViolation {
                        param: param.name.sym.clone(),
                        found: *arg,
                        constraint,
                    });
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeError {
    Mismatch {
        found: TypeId,
        expected: TypeId,
    },
    ConstraintViolation {
        param: String,
        found: TypeId,
        constraint: TypeId,
    },
    NotFound {
        name: String,
    },
    Invalid {
        message: String,
    },
    MissingReturn {
        func: String,
        expected: TypeId,
    },
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::Mismatch { found, expected } => {
                write!(f, "Type mismatch: found {}, expected {}", found, expected)
            }
            TypeError::ConstraintViolation {
                param,
                found,
                constraint,
            } => write!(
                f,
                "Type argument for '{}' does not satisfy constraint: found {}, expected {}",
                param, found, constraint
            ),
            TypeError::NotFound { name } => write!(f, "Type not found '{}'", name),
            TypeError::Invalid { message } => write!(f, "Invalid type: {}", message),
            TypeError::MissingReturn { func, expected } => {
                write!(
                    f,
                    "Function '{}' missing return, expected {}",
                    func, expected
                )
            }
        }
    }
}

impl std::error::Error for TypeError {}
