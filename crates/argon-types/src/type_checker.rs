//! Argon - Type checker

use crate::types::{
    ClassDef, EnumDef, FieldDef, FunctionSig, InterfaceDef, InterfaceMember, MethodDef,
    ObjectShapeDef, StructDef, Type as CompType, TypeId, TypeInstantiator, TypeTable,
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
struct GenericInterfaceDef {
    def: InterfaceDef,
    type_params: Vec<argon_ast::TypeParam>,
}

#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    vars: HashMap<String, TypeId>,
    structs: HashMap<String, StructDef>,
    generic_structs: HashMap<String, GenericStructDef>,
    classes: HashMap<String, ClassDef>,
    generic_classes: HashMap<String, GenericClassDef>,
    interfaces: HashMap<String, InterfaceDef>,
    generic_interfaces: HashMap<String, GenericInterfaceDef>,
    enums: HashMap<String, EnumDef>,
    functions: HashMap<String, FunctionSig>,
    generic_functions: HashMap<String, GenericFunctionDef>,
    type_aliases: HashMap<String, TypeAliasDef>,
    type_params: HashMap<String, TypeId>,
    this_ty: Option<TypeId>,
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            structs: HashMap::new(),
            generic_structs: HashMap::new(),
            classes: HashMap::new(),
            generic_classes: HashMap::new(),
            interfaces: HashMap::new(),
            generic_interfaces: HashMap::new(),
            enums: HashMap::new(),
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

    fn get_generic_struct(&self, name: &str) -> Option<&GenericStructDef> {
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

    fn get_generic_class(&self, name: &str) -> Option<&GenericClassDef> {
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

    pub fn get_interface(&self, name: &str) -> Option<&InterfaceDef> {
        self.interfaces.get(name)
    }

    pub fn add_interface(&mut self, name: String, def: InterfaceDef) {
        self.interfaces.insert(name, def);
    }

    fn get_generic_interface(&self, name: &str) -> Option<&GenericInterfaceDef> {
        self.generic_interfaces.get(name)
    }

    pub fn add_generic_interface(
        &mut self,
        name: String,
        def: InterfaceDef,
        type_params: Vec<argon_ast::TypeParam>,
    ) {
        self.generic_interfaces
            .insert(name, GenericInterfaceDef { def, type_params });
    }

    pub fn get_enum(&self, name: &str) -> Option<&EnumDef> {
        self.enums.get(name)
    }

    pub fn add_enum(&mut self, name: String, def: EnumDef) {
        self.enums.insert(name, def);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSig) {
        self.functions.insert(name, sig);
    }

    fn get_generic_function(&self, name: &str) -> Option<&GenericFunctionDef> {
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
            interfaces: self.interfaces.clone(),
            generic_interfaces: self.generic_interfaces.clone(),
            enums: self.enums.clone(),
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

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
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
                Stmt::Interface(i) => {
                    let mut local_env = self.env.child();
                    for type_param in &i.type_params {
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
                    let interface_def = self.build_interface_def(Some(i.id.sym.clone()), i);
                    self.env = old_env;

                    if i.type_params.is_empty() {
                        self.env.add_interface(i.id.sym.clone(), interface_def);
                    } else {
                        self.env.add_generic_interface(
                            i.id.sym.clone(),
                            interface_def,
                            i.type_params.clone(),
                        );
                    }
                }
                Stmt::Enum(e) => {
                    let enum_def = EnumDef {
                        name: e.id.sym.clone(),
                        variants: e.members.iter().map(|m| m.id.sym.clone()).collect(),
                    };
                    let enum_ty = self.type_table.enum_def(enum_def.clone());
                    self.env.add_enum(e.id.sym.clone(), enum_def);
                    self.env.add_var(e.id.sym.clone(), enum_ty);
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

    fn build_interface_def(
        &mut self,
        name: Option<String>,
        interface: &InterfaceDecl,
    ) -> InterfaceDef {
        let extends = interface
            .extends
            .iter()
            .map(|t| self.resolve_type(t))
            .collect();
        let members = interface
            .body
            .body
            .iter()
            .filter_map(|member| self.resolve_interface_member(member))
            .collect();

        InterfaceDef {
            name: name.unwrap_or_else(|| "<anonymous>".to_string()),
            extends,
            members,
        }
    }

    fn resolve_interface_member(
        &mut self,
        member: &argon_ast::InterfaceMember,
    ) -> Option<InterfaceMember> {
        match member {
            argon_ast::InterfaceMember::Property(p) => Some(InterfaceMember::Property {
                name: p.id.sym.clone(),
                ty: p
                    .type_annotation
                    .as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| self.type_table.any()),
            }),
            argon_ast::InterfaceMember::Method(m) => {
                let params = m
                    .params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(|t| self.resolve_type(t))
                            .unwrap_or_else(|| self.type_table.any())
                    })
                    .collect();
                let return_type = m
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| self.type_table.void());

                Some(InterfaceMember::Method {
                    name: m.id.sym.clone(),
                    sig: FunctionSig {
                        params,
                        return_type,
                        is_async: false,
                    },
                })
            }
            argon_ast::InterfaceMember::IndexSignature(_) => None,
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
                        if let Expr::Object(obj) = init {
                            if self.supports_object_literal_target(declared_ty) {
                                self.check_object_literal_assignment(obj, declared_ty);
                                self.env.add_var(id.name.sym.clone(), declared_ty);
                                continue;
                            }
                        }

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

    fn supports_object_literal_target(&self, ty: TypeId) -> bool {
        matches!(
            self.type_table.get(ty),
            Some(CompType::Struct(_))
                | Some(CompType::Class(_))
                | Some(CompType::Interface(_))
                | Some(CompType::ObjectShape(_))
        )
    }

    fn shape_members(&self, ty: TypeId) -> Option<(String, Vec<FieldDef>, Vec<MethodDef>)> {
        match self.type_table.get(ty).cloned() {
            Some(CompType::Struct(def)) => Some((def.name, def.fields, def.methods)),
            Some(CompType::Class(def)) => Some((def.name, def.fields, def.methods)),
            Some(CompType::Interface(def)) => {
                let (fields, methods) = self.flatten_interface_shape(&def);
                Some((def.name, fields, methods))
            }
            Some(CompType::ObjectShape(def)) => {
                Some(("<object>".to_string(), def.fields, def.methods))
            }
            _ => None,
        }
    }

    fn flatten_interface_shape(&self, def: &InterfaceDef) -> (Vec<FieldDef>, Vec<MethodDef>) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for extends in &def.extends {
            if let Some((_, extend_fields, extend_methods)) = self.shape_members(*extends) {
                fields.extend(extend_fields);
                methods.extend(extend_methods);
            }
        }

        for member in &def.members {
            match member {
                InterfaceMember::Property { name, ty } => fields.push(FieldDef {
                    name: name.clone(),
                    ty: *ty,
                }),
                InterfaceMember::Method { name, sig } => methods.push(MethodDef {
                    name: name.clone(),
                    sig: sig.clone(),
                }),
            }
        }

        (fields, methods)
    }

    fn check_object_literal_assignment(&mut self, obj: &ObjectExpression, expected_ty: TypeId) {
        let Some((target_name, expected_fields, expected_methods)) =
            self.shape_members(expected_ty)
        else {
            return;
        };

        let mut provided_values: HashMap<String, TypeId> = HashMap::new();
        let mut provided_methods: HashMap<String, FunctionSig> = HashMap::new();
        for prop in &obj.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    if p.computed {
                        self.errors.push(TypeError::Invalid {
                            message: format!(
                                "computed object keys are not supported when assigning to '{}'",
                                target_name
                            ),
                        });
                        continue;
                    }

                    let key = match &p.key {
                        Expr::Identifier(id) => id.sym.clone(),
                        Expr::Literal(Literal::String(s)) => s.value.clone(),
                        _ => {
                            self.errors.push(TypeError::Invalid {
                                message: format!(
                                    "unsupported object property key when assigning to '{}'",
                                    target_name
                                ),
                            });
                            continue;
                        }
                    };

                    let value_ty = match &p.value {
                        ExprOrSpread::Expr(e) => self.infer_expression(e),
                        ExprOrSpread::Spread(_) => {
                            self.errors.push(TypeError::Invalid {
                                message: format!(
                                    "spread properties are not supported when assigning to '{}'",
                                    target_name
                                ),
                            });
                            continue;
                        }
                    };
                    provided_values.insert(key, value_ty);
                }
                ObjectProperty::Shorthand(id) => {
                    let value_ty = self.infer_identifier(id);
                    provided_values.insert(id.sym.clone(), value_ty);
                }
                ObjectProperty::Spread(_) => {
                    self.errors.push(TypeError::Invalid {
                        message: format!(
                            "spread properties are not supported when assigning to '{}'",
                            target_name
                        ),
                    });
                }
                ObjectProperty::Method(m) => {
                    let name = match &m.key {
                        Expr::Identifier(id) => id.sym.clone(),
                        _ => {
                            self.errors.push(TypeError::Invalid {
                                message: format!(
                                    "unsupported object method key when assigning to '{}'",
                                    target_name
                                ),
                            });
                            continue;
                        }
                    };
                    provided_methods.insert(name, self.resolve_function_sig(&m.value));
                }
                ObjectProperty::Getter(_) | ObjectProperty::Setter(_) => {
                    self.errors.push(TypeError::Invalid {
                        message: format!(
                            "getter/setter properties are not supported when assigning to '{}'",
                            target_name
                        ),
                    });
                }
            }
        }

        for expected in expected_fields {
            if let Some(found_ty) = provided_values.get(&expected.name).copied() {
                self.unify(found_ty, expected.ty);
            } else {
                self.errors.push(TypeError::Invalid {
                    message: format!(
                        "missing field '{}' when assigning to '{}'",
                        expected.name, target_name
                    ),
                });
            }
        }

        for expected in expected_methods {
            if let Some(found_sig) = provided_methods.get(&expected.name).cloned() {
                let found_ty = self.type_table.function(found_sig);
                let expected_ty = self.type_table.function(expected.sig);
                self.unify(found_ty, expected_ty);
            } else {
                self.errors.push(TypeError::Invalid {
                    message: format!(
                        "missing method '{}' when assigning to '{}'",
                        expected.name, target_name
                    ),
                });
            }
        }
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

        for implemented in &c.implements {
            let interface_ty = self.resolve_type(implemented);
            if !self.is_assignable(class_ty, interface_ty) {
                self.errors.push(TypeError::Invalid {
                    message: format!(
                        "class '{}' does not satisfy implemented interface",
                        c.id.sym
                    ),
                });
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
        let mut inferred = HashMap::new();

        for (arg_ty, param_ty) in arg_tys.iter().zip(generic_func.sig.params.iter()) {
            self.collect_type_arg_bindings(*param_ty, *arg_ty, &mut inferred);
        }

        generic_func
            .type_params
            .iter()
            .map(|param| {
                inferred
                    .get(&param.name.sym)
                    .copied()
                    .or_else(|| param.default.as_ref().map(|d| self.resolve_type(d)))
                    .unwrap_or_else(|| self.type_table.any())
            })
            .collect()
    }

    fn collect_type_arg_bindings(
        &self,
        param_ty: TypeId,
        arg_ty: TypeId,
        inferred: &mut HashMap<String, TypeId>,
    ) {
        let Some(param_kind) = self.type_table.get(param_ty).cloned() else {
            return;
        };

        match param_kind {
            CompType::TypeParam(param) => {
                inferred
                    .entry(param.name)
                    .and_modify(|existing| {
                        if *existing != arg_ty {
                            *existing = self
                                .common_assignable_type(*existing, arg_ty)
                                .unwrap_or_else(|| {
                                    self.type_table.get_by_name("any").unwrap_or(*existing)
                                });
                        }
                    })
                    .or_insert(arg_ty);
            }
            CompType::Array(inner) => {
                if let Some(CompType::Array(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::Tuple(param_items) => {
                if let Some(CompType::Tuple(arg_items)) = self.type_table.get(arg_ty).cloned() {
                    for (param_item, arg_item) in param_items.iter().zip(arg_items.iter()) {
                        self.collect_type_arg_bindings(*param_item, *arg_item, inferred);
                    }
                }
            }
            CompType::Ref(inner) => {
                if let Some(CompType::Ref(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::MutRef(inner) => {
                if let Some(CompType::MutRef(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::Shared(inner) => {
                if let Some(CompType::Shared(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::Option(inner) => {
                if let Some(CompType::Option(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::Promise(inner) => {
                if let Some(CompType::Promise(arg_inner)) = self.type_table.get(arg_ty).cloned() {
                    self.collect_type_arg_bindings(inner, arg_inner, inferred);
                }
            }
            CompType::Result(param_ok, param_err) => {
                if let Some(CompType::Result(arg_ok, arg_err)) =
                    self.type_table.get(arg_ty).cloned()
                {
                    self.collect_type_arg_bindings(param_ok, arg_ok, inferred);
                    self.collect_type_arg_bindings(param_err, arg_err, inferred);
                }
            }
            CompType::Function(sig) => {
                if let Some(CompType::Function(arg_sig)) = self.type_table.get(arg_ty).cloned() {
                    for (param_item, arg_item) in sig.params.iter().zip(arg_sig.params.iter()) {
                        self.collect_type_arg_bindings(*param_item, *arg_item, inferred);
                    }
                    self.collect_type_arg_bindings(sig.return_type, arg_sig.return_type, inferred);
                }
            }
            CompType::Struct(param_def) => {
                if let Some((_, arg_fields, _)) = self.shape_members(arg_ty) {
                    for field in param_def.fields {
                        if let Some(arg_field) = arg_fields.iter().find(|f| f.name == field.name) {
                            self.collect_type_arg_bindings(field.ty, arg_field.ty, inferred);
                        }
                    }
                }
            }
            CompType::Class(param_def) => {
                if let Some((_, arg_fields, _)) = self.shape_members(arg_ty) {
                    for field in param_def.fields {
                        if let Some(arg_field) = arg_fields.iter().find(|f| f.name == field.name) {
                            self.collect_type_arg_bindings(field.ty, arg_field.ty, inferred);
                        }
                    }
                }
            }
            CompType::Interface(param_def) => {
                if let Some((_, arg_fields, arg_methods)) = self.shape_members(arg_ty) {
                    for member in param_def.members {
                        match member {
                            InterfaceMember::Property { name, ty } => {
                                if let Some(arg_field) = arg_fields.iter().find(|f| f.name == name)
                                {
                                    self.collect_type_arg_bindings(ty, arg_field.ty, inferred);
                                }
                            }
                            InterfaceMember::Method { name, sig } => {
                                if let Some(arg_method) =
                                    arg_methods.iter().find(|m| m.name == name)
                                {
                                    for (param_item, arg_item) in
                                        sig.params.iter().zip(arg_method.sig.params.iter())
                                    {
                                        self.collect_type_arg_bindings(
                                            *param_item,
                                            *arg_item,
                                            inferred,
                                        );
                                    }
                                    self.collect_type_arg_bindings(
                                        sig.return_type,
                                        arg_method.sig.return_type,
                                        inferred,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn common_assignable_type(&self, left: TypeId, right: TypeId) -> Option<TypeId> {
        if left == right {
            Some(left)
        } else if self.is_assignable(left, right) {
            Some(right)
        } else if self.is_assignable(right, left) {
            Some(left)
        } else {
            None
        }
    }

    fn infer_member(&mut self, m: &MemberExpr) -> TypeId {
        let mut obj_ty = self.infer_expression(&m.object);

        if m.computed {
            self.infer_expression(&m.property);
            return self.type_table.unknown();
        }

        // Unwrap reference-like shells.
        while let Some(CompType::Ref(inner))
        | Some(CompType::MutRef(inner))
        | Some(CompType::Shared(inner)) = self.type_table.get(obj_ty).cloned()
        {
            obj_ty = inner;
        }

        let prop_name = match &*m.property {
            Expr::Identifier(id) => id.sym.as_str(),
            _ => return self.type_table.unknown(),
        };

        self.lookup_member_type(obj_ty, prop_name)
            .unwrap_or_else(|| self.type_table.unknown())
    }

    fn lookup_member_type(&mut self, obj_ty: TypeId, prop_name: &str) -> Option<TypeId> {
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
                        .map(|m| self.type_table.function(m.sig.clone()))
                }),
            Some(CompType::Class(def)) => def
                .fields
                .iter()
                .find(|f| f.name == prop_name)
                .map(|f| f.ty)
                .or_else(|| {
                    def.methods
                        .iter()
                        .find(|m| m.name == prop_name)
                        .map(|m| self.type_table.function(m.sig.clone()))
                }),
            Some(CompType::ObjectShape(def)) => def
                .fields
                .iter()
                .find(|f| f.name == prop_name)
                .map(|f| f.ty)
                .or_else(|| {
                    def.methods
                        .iter()
                        .find(|m| m.name == prop_name)
                        .map(|m| self.type_table.function(m.sig.clone()))
                }),
            Some(CompType::Interface(def)) => {
                for member in &def.members {
                    match member {
                        InterfaceMember::Property { name, ty } if name == prop_name => {
                            return Some(*ty);
                        }
                        InterfaceMember::Method { name, sig } if name == prop_name => {
                            return Some(self.type_table.function(sig.clone()));
                        }
                        _ => {}
                    }
                }

                for extends in &def.extends {
                    if let Some(member_ty) = self.lookup_member_type(*extends, prop_name) {
                        return Some(member_ty);
                    }
                }

                None
            }
            Some(CompType::Enum(def)) => def
                .variants
                .iter()
                .find(|variant| variant.as_str() == prop_name)
                .map(|_| obj_ty),
            Some(CompType::Union(types)) => {
                let mut member_types = Vec::new();
                for ty in types {
                    let member_ty = self.lookup_member_type(ty, prop_name)?;
                    member_types.push(member_ty);
                }
                Some(self.union_or_single(member_types))
            }
            Some(CompType::Intersection(types)) => {
                let member_types: Vec<_> = types
                    .iter()
                    .filter_map(|ty| self.lookup_member_type(*ty, prop_name))
                    .collect();
                if member_types.is_empty() {
                    None
                } else {
                    Some(self.union_or_single(member_types))
                }
            }
            _ => None,
        }
    }

    fn union_or_single(&mut self, mut tys: Vec<TypeId>) -> TypeId {
        tys.dedup();
        if tys.len() == 1 {
            tys[0]
        } else {
            self.type_table.union(tys)
        }
    }

    fn object_property_name(&self, key: &Expr) -> Option<String> {
        match key {
            Expr::Identifier(id) => Some(id.sym.clone()),
            Expr::Literal(Literal::String(s)) => Some(s.value.clone()),
            _ => None,
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
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for prop in &o.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    let Some(key) = self.object_property_name(&p.key) else {
                        continue;
                    };
                    if let ExprOrSpread::Expr(e) = &p.value {
                        let ty = self.infer_expression(e);
                        fields.push(FieldDef { name: key, ty });
                    }
                }
                ObjectProperty::Shorthand(id) => {
                    let ty = self.infer_identifier(id);
                    fields.push(FieldDef {
                        name: id.sym.clone(),
                        ty,
                    });
                }
                ObjectProperty::Spread(s) => {
                    self.infer_expression(&s.argument);
                }
                ObjectProperty::Method(m) => {
                    self.check_function(&m.value).ok();
                    if let Expr::Identifier(id) = &m.key {
                        methods.push(MethodDef {
                            name: id.sym.clone(),
                            sig: self.resolve_function_sig(&m.value),
                        });
                    }
                }
                ObjectProperty::Getter(_) | ObjectProperty::Setter(_) => {}
            }
        }

        self.type_table
            .object_shape(ObjectShapeDef { fields, methods })
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

    fn instantiate_interface_def(
        &mut self,
        def: &InterfaceDef,
        type_params: &[argon_ast::TypeParam],
        type_args: &[TypeId],
    ) -> InterfaceDef {
        let mut instantiator = TypeInstantiator::new();
        for (param, arg) in type_params.iter().zip(type_args.iter()) {
            instantiator.add_substitution(param.name.sym.clone(), *arg);
        }

        let extends = def
            .extends
            .iter()
            .map(|&t| instantiator.instantiate(&mut self.type_table, t))
            .collect();
        let members = def
            .members
            .iter()
            .map(|member| match member {
                InterfaceMember::Property { name, ty } => InterfaceMember::Property {
                    name: name.clone(),
                    ty: instantiator.instantiate(&mut self.type_table, *ty),
                },
                InterfaceMember::Method { name, sig } => InterfaceMember::Method {
                    name: name.clone(),
                    sig: FunctionSig {
                        params: sig
                            .params
                            .iter()
                            .map(|&p| instantiator.instantiate(&mut self.type_table, p))
                            .collect(),
                        return_type: instantiator
                            .instantiate(&mut self.type_table, sig.return_type),
                        is_async: sig.is_async,
                    },
                },
            })
            .collect();

        InterfaceDef {
            name: def.name.clone(),
            extends,
            members,
        }
    }

    fn resolve_type_alias_with_args(
        &mut self,
        name: &str,
        def: &TypeAliasDef,
        type_args: &[TypeId],
    ) -> TypeId {
        if type_args.len() != def.type_params.len() {
            self.errors.push(TypeError::Invalid {
                message: format!(
                    "generic type alias '{}' expects {} type arguments, got {}",
                    name,
                    def.type_params.len(),
                    type_args.len()
                ),
            });
            return self.type_table.unknown();
        }

        self.check_generic_constraints(&def.type_params, type_args);

        let mut alias_env = self.env.child();
        for (param, arg) in def.type_params.iter().zip(type_args.iter()) {
            alias_env.add_type_param(param.name.sym.clone(), *arg);
        }

        let old_env = std::mem::replace(&mut self.env, alias_env);
        let resolved = self.resolve_type(&def.type_annotation);
        self.env = old_env;
        resolved
    }

    fn resolve_object_type(&mut self, object: &ObjectType) -> TypeId {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for member in &object.members {
            match member {
                TypeMember::Property(p) => {
                    let ty = p
                        .type_annotation
                        .as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or_else(|| self.type_table.any());
                    fields.push(FieldDef {
                        name: p.id.sym.clone(),
                        ty,
                    });
                }
                TypeMember::Method(m) => {
                    let params = m
                        .params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(|t| self.resolve_type(t))
                                .unwrap_or_else(|| self.type_table.any())
                        })
                        .collect();
                    let return_type = m
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or_else(|| self.type_table.void());
                    methods.push(MethodDef {
                        name: m.id.sym.clone(),
                        sig: FunctionSig {
                            params,
                            return_type,
                            is_async: false,
                        },
                    });
                }
                _ => {}
            }
        }

        self.type_table
            .object_shape(ObjectShapeDef { fields, methods })
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
                        if let Some(alias) = self.env.get_type_alias(&id.sym).cloned() {
                            let type_args: Vec<TypeId> =
                                r.type_args.iter().map(|t| self.resolve_type(t)).collect();
                            return self.resolve_type_alias_with_args(&id.sym, &alias, &type_args);
                        }

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

                        if let Some(generic_interface) =
                            self.env.get_generic_interface(&id.sym).cloned()
                        {
                            let type_args: Vec<TypeId> =
                                r.type_args.iter().map(|t| self.resolve_type(t)).collect();

                            if type_args.len() != generic_interface.type_params.len() {
                                self.errors.push(TypeError::Invalid {
                                    message: format!(
                                        "generic interface '{}' expects {} type arguments, got {}",
                                        id.sym,
                                        generic_interface.type_params.len(),
                                        type_args.len()
                                    ),
                                });
                                return self.type_table.unknown();
                            }

                            self.check_generic_constraints(
                                &generic_interface.type_params,
                                &type_args,
                            );

                            let interface_def = self.instantiate_interface_def(
                                &generic_interface.def,
                                &generic_interface.type_params,
                                &type_args,
                            );
                            return self.type_table.interface_def(interface_def);
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
                    if let Some(interface_def) = self.env.get_interface(&id.sym) {
                        return self.type_table.interface_def(interface_def.clone());
                    }
                    if let Some(enum_def) = self.env.get_enum(&id.sym) {
                        return self.type_table.enum_def(enum_def.clone());
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
            argon_ast::Type::Object(object) => self.resolve_object_type(object),
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
            (Some(CompType::Union(_)), _) | (_, Some(CompType::Union(_))) => {
                if !self.is_assignable(found, expected) {
                    self.errors.push(TypeError::Mismatch { found, expected });
                }
            }
            _ => {
                if !self.is_assignable(found, expected) {
                    self.errors.push(TypeError::Mismatch { found, expected });
                }
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
            (Some(CompType::Enum(found_enum)), Some(CompType::Enum(expected_enum))) => {
                found_enum.name == expected_enum.name
            }
            (Some(CompType::Array(f)), Some(CompType::Array(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Tuple(found_types)), Some(CompType::Tuple(expected_types))) => {
                found_types.len() == expected_types.len()
                    && found_types
                        .iter()
                        .zip(expected_types.iter())
                        .all(|(f, e)| self.is_assignable(*f, *e))
            }
            (Some(CompType::Ref(f)), Some(CompType::Ref(e))) => self.is_assignable(*f, *e),
            (Some(CompType::MutRef(f)), Some(CompType::MutRef(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Shared(f)), Some(CompType::Shared(e))) => self.is_assignable(*f, *e),
            (Some(CompType::Struct(fs)), Some(CompType::Struct(es))) => fs.name == es.name,
            (Some(CompType::Class(fc)), Some(CompType::Class(ec))) => fc.name == ec.name,
            (Some(CompType::Object), Some(CompType::Object)) => true,
            (_, Some(CompType::Object)) => matches!(
                self.type_table.get(found),
                Some(CompType::Object)
                    | Some(CompType::Struct(_))
                    | Some(CompType::Class(_))
                    | Some(CompType::Interface(_))
                    | Some(CompType::ObjectShape(_))
                    | Some(CompType::Array(_))
                    | Some(CompType::Tuple(_))
            ),
            (Some(CompType::Struct(_)), Some(CompType::Interface(_)))
            | (Some(CompType::Class(_)), Some(CompType::Interface(_)))
            | (Some(CompType::ObjectShape(_)), Some(CompType::Interface(_)))
            | (Some(CompType::Interface(_)), Some(CompType::Interface(_)))
            | (Some(CompType::Struct(_)), Some(CompType::ObjectShape(_)))
            | (Some(CompType::Class(_)), Some(CompType::ObjectShape(_)))
            | (Some(CompType::ObjectShape(_)), Some(CompType::ObjectShape(_)))
            | (Some(CompType::Interface(_)), Some(CompType::ObjectShape(_))) => {
                self.shape_assignable(found, expected)
            }
            (Some(CompType::Union(types)), _) => {
                types.iter().all(|t| self.is_assignable(*t, expected))
            }
            (_, Some(CompType::Union(types))) => {
                types.iter().any(|t| self.is_assignable(found, *t))
            }
            (Some(CompType::Intersection(types)), _) => {
                types.iter().all(|t| self.is_assignable(*t, expected))
            }
            (_, Some(CompType::Intersection(types))) => {
                types.iter().all(|t| self.is_assignable(found, *t))
            }
            _ => false,
        }
    }

    fn shape_assignable(&self, found: TypeId, expected: TypeId) -> bool {
        let Some((_, found_fields, found_methods)) = self.shape_members(found) else {
            return false;
        };
        let Some((_, expected_fields, expected_methods)) = self.shape_members(expected) else {
            return false;
        };

        expected_fields.iter().all(|expected_field| {
            found_fields
                .iter()
                .find(|field| field.name == expected_field.name)
                .map(|field| self.is_assignable(field.ty, expected_field.ty))
                .unwrap_or(false)
        }) && expected_methods.iter().all(|expected_method| {
            found_methods
                .iter()
                .find(|method| method.name == expected_method.name)
                .map(|method| self.function_sig_assignable(&method.sig, &expected_method.sig))
                .unwrap_or(false)
        })
    }

    fn function_sig_assignable(&self, found: &FunctionSig, expected: &FunctionSig) -> bool {
        found.params.len() == expected.params.len()
            && found
                .params
                .iter()
                .zip(expected.params.iter())
                .all(|(f, e)| self.is_assignable(*f, *e))
            && self.is_assignable(found.return_type, expected.return_type)
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
