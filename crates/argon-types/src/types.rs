//! Argon - Type definitions

use std::collections::HashMap;
use std::fmt;

pub type TypeId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Never,
    Any,
    Unknown,
    Void,
    Null,
    Undefined,
    Boolean,
    Number,
    BigInt,
    String,
    Symbol,
    Object,
    Array(TypeId),
    Tuple(Vec<TypeId>),
    Function(FunctionSig),
    Union(Vec<TypeId>),
    Intersection(Vec<TypeId>),
    Struct(StructDef),
    Skill(SkillTypeDef),
    Interface(InterfaceDef),
    ObjectShape(ObjectShapeDef),
    Enum(EnumDef),
    Ref(TypeId),
    MutRef(TypeId),
    Shared(TypeId),
    Generic(String),
    TypeParam(TypeParam),
    Error,
    Option(TypeId),
    Result(TypeId, TypeId),
    Promise(TypeId),
}

impl Type {
    pub fn is_truthy(&self) -> bool {
        matches!(
            self,
            Type::Boolean | Type::Number | Type::String | Type::Object | Type::ObjectShape(_)
        )
    }

    pub fn is_subtype_of(&self, _other: &Type) -> bool {
        true
    }

    pub fn is_copyable(&self) -> bool {
        matches!(
            self,
            Type::Never
                | Type::Boolean
                | Type::Number
                | Type::BigInt
                | Type::String
                | Type::Symbol
                | Type::Null
                | Type::Undefined
                | Type::Void
                | Type::Ref(_)
                | Type::Shared(_)
        )
    }

    pub fn is_moveable(&self) -> bool {
        !self.is_copyable()
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Never => write!(f, "never"),
            Type::Any => write!(f, "any"),
            Type::Unknown => write!(f, "unknown"),
            Type::Void => write!(f, "void"),
            Type::Null => write!(f, "null"),
            Type::Undefined => write!(f, "undefined"),
            Type::Boolean => write!(f, "boolean"),
            Type::Number => write!(f, "number"),
            Type::BigInt => write!(f, "bigint"),
            Type::String => write!(f, "string"),
            Type::Symbol => write!(f, "symbol"),
            Type::Object => write!(f, "object"),
            Type::Array(id) => write!(f, "{}[]", id),
            Type::Tuple(types) => {
                write!(f, "[")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            }
            Type::Function(sig) => write!(f, "{}", sig),
            Type::Union(types) => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            Type::Intersection(types) => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            Type::Struct(def) => write!(f, "struct {}", def.name),
            Type::Skill(def) => write!(f, "skill {}", def.name),
            Type::Interface(def) => write!(f, "interface {}", def.name),
            Type::ObjectShape(_) => write!(f, "object"),
            Type::Enum(def) => write!(f, "enum {}", def.name),
            Type::Ref(id) => write!(f, "&{}", id),
            Type::MutRef(id) => write!(f, "&mut {}", id),
            Type::Shared(id) => write!(f, "Shared<{}>", id),
            Type::Generic(name) => write!(f, "{}", name),
            Type::TypeParam(param) => write!(f, "{}", param.name),
            Type::Option(inner) => write!(f, "{}?", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Promise(inner) => write!(f, "Promise<{}>", inner),
            Type::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSig {
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
    pub is_async: bool,
}

impl fmt::Display for FunctionSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, ") => {}", self.return_type)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
    pub constructor_params: Option<Vec<(String, TypeId)>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDef {
    pub name: String,
    pub sig: FunctionSig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillTypeDef {
    pub name: String,
    pub required_fields: Vec<FieldDef>,
    pub concrete_methods: Vec<MethodDef>,
    pub abstract_methods: Vec<MethodDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectShapeDef {
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceDef {
    pub name: String,
    pub extends: Vec<TypeId>,
    pub members: Vec<InterfaceMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InterfaceMember {
    Property { name: String, ty: TypeId },
    Method { name: String, sig: FunctionSig },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Option<TypeId>,
    pub default: Option<TypeId>,
}

#[derive(Debug, Clone, Default)]
pub struct TypeTable {
    types: Vec<Type>,
    names: HashMap<String, TypeId>,
}

impl TypeTable {
    pub fn new_with_builtins() -> Self {
        let mut table = Self {
            types: Vec::new(),
            names: HashMap::new(),
        };
        table.init_builtins();
        table
    }

    fn init_builtins(&mut self) {
        self.add_named("never", Type::Never);
        self.add_named("any", Type::Any);
        self.add_named("unknown", Type::Unknown);
        self.add_named("void", Type::Void);
        self.add_named("null", Type::Null);
        self.add_named("undefined", Type::Undefined);
        self.add_named("boolean", Type::Boolean);
        self.add_named("number", Type::Number);
        self.add_named("bigint", Type::BigInt);
        self.add_named("string", Type::String);
        self.add_named("symbol", Type::Symbol);
        self.add_named("object", Type::Object);
    }

    fn add_named(&mut self, name: &str, ty: Type) {
        let id = self.types.len() as TypeId;
        self.names.insert(name.to_string(), id);
        self.types.push(ty);
    }

    pub fn add(&mut self, ty: Type) -> TypeId {
        let id = self.types.len() as TypeId;
        self.types.push(ty);
        id
    }

    pub fn get(&self, id: TypeId) -> Option<&Type> {
        self.types.get(id as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<TypeId> {
        self.names.get(name).copied()
    }

    pub fn get_or_add(&mut self, ty: Type) -> TypeId {
        if let Some(id) = self.types.iter().position(|t| t == &ty) {
            return id as TypeId;
        }
        self.add(ty)
    }

    pub fn error(&mut self) -> TypeId {
        self.get_or_add(Type::Error)
    }

    pub fn unknown(&mut self) -> TypeId {
        self.get_or_add(Type::Unknown)
    }

    pub fn any(&mut self) -> TypeId {
        self.get_or_add(Type::Any)
    }

    pub fn number(&mut self) -> TypeId {
        self.get_or_add(Type::Number)
    }

    pub fn string(&mut self) -> TypeId {
        self.get_or_add(Type::String)
    }

    pub fn boolean(&mut self) -> TypeId {
        self.get_or_add(Type::Boolean)
    }

    pub fn void(&mut self) -> TypeId {
        self.get_or_add(Type::Void)
    }

    pub fn null(&mut self) -> TypeId {
        self.get_or_add(Type::Null)
    }

    pub fn undefined(&mut self) -> TypeId {
        self.get_or_add(Type::Undefined)
    }

    pub fn never(&mut self) -> TypeId {
        self.get_or_add(Type::Never)
    }

    pub fn object(&mut self) -> TypeId {
        self.get_or_add(Type::Object)
    }

    pub fn option(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::Option(inner))
    }

    pub fn result(&mut self, ok: TypeId, err: TypeId) -> TypeId {
        self.get_or_add(Type::Result(ok, err))
    }

    pub fn promise(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::Promise(inner))
    }

    pub fn array(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::Array(inner))
    }

    pub fn tuple(&mut self, types: Vec<TypeId>) -> TypeId {
        self.get_or_add(Type::Tuple(types))
    }

    pub fn union(&mut self, types: Vec<TypeId>) -> TypeId {
        self.get_or_add(Type::Union(types))
    }

    pub fn intersection(&mut self, types: Vec<TypeId>) -> TypeId {
        self.get_or_add(Type::Intersection(types))
    }

    pub fn ref_type(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::Ref(inner))
    }

    pub fn mut_ref(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::MutRef(inner))
    }

    pub fn shared(&mut self, inner: TypeId) -> TypeId {
        self.get_or_add(Type::Shared(inner))
    }

    pub fn function(&mut self, sig: FunctionSig) -> TypeId {
        self.get_or_add(Type::Function(sig))
    }

    pub fn struct_def(&mut self, def: StructDef) -> TypeId {
        self.get_or_add(Type::Struct(def))
    }

    pub fn skill_def(&mut self, def: SkillTypeDef) -> TypeId {
        self.get_or_add(Type::Skill(def))
    }

    pub fn interface_def(&mut self, def: InterfaceDef) -> TypeId {
        self.get_or_add(Type::Interface(def))
    }

    pub fn object_shape(&mut self, def: ObjectShapeDef) -> TypeId {
        self.get_or_add(Type::ObjectShape(def))
    }

    pub fn enum_def(&mut self, def: EnumDef) -> TypeId {
        self.get_or_add(Type::Enum(def))
    }

    pub fn type_param(&mut self, param: TypeParam) -> TypeId {
        self.get_or_add(Type::TypeParam(param))
    }

    pub fn generic(&mut self, name: String) -> TypeId {
        self.get_or_add(Type::Generic(name))
    }
}

pub struct TypeInstantiator {
    substitutions: HashMap<String, TypeId>,
}

impl TypeInstantiator {
    pub fn new() -> Self {
        Self {
            substitutions: HashMap::new(),
        }
    }

    pub fn add_substitution(&mut self, param_name: String, concrete_type: TypeId) {
        self.substitutions.insert(param_name, concrete_type);
    }

    pub fn instantiate(&self, type_table: &mut TypeTable, type_id: TypeId) -> TypeId {
        let ty = type_table.get(type_id).cloned().unwrap_or(Type::Never);

        match ty {
            Type::TypeParam(param) => self
                .substitutions
                .get(&param.name)
                .copied()
                .unwrap_or(type_id),
            Type::Generic(name) => self.substitutions.get(&name).copied().unwrap_or(type_id),
            Type::Function(sig) => {
                let new_params: Vec<_> = sig
                    .params
                    .iter()
                    .map(|&p| self.instantiate(type_table, p))
                    .collect();
                let new_return = self.instantiate(type_table, sig.return_type);
                type_table.function(FunctionSig {
                    params: new_params,
                    return_type: new_return,
                    is_async: sig.is_async,
                })
            }
            Type::Array(elem) => {
                let new_elem = self.instantiate(type_table, elem);
                type_table.array(new_elem)
            }
            Type::Tuple(types) => {
                let new_types: Vec<_> = types
                    .iter()
                    .map(|&t| self.instantiate(type_table, t))
                    .collect();
                type_table.tuple(new_types)
            }
            Type::Option(inner) => {
                let new_inner = self.instantiate(type_table, inner);
                type_table.option(new_inner)
            }
            Type::Result(ok, err) => {
                let new_ok = self.instantiate(type_table, ok);
                let new_err = self.instantiate(type_table, err);
                type_table.result(new_ok, new_err)
            }
            Type::Ref(inner) => {
                let new_inner = self.instantiate(type_table, inner);
                type_table.ref_type(new_inner)
            }
            Type::MutRef(inner) => {
                let new_inner = self.instantiate(type_table, inner);
                type_table.mut_ref(new_inner)
            }
            Type::Shared(inner) => {
                let new_inner = self.instantiate(type_table, inner);
                type_table.shared(new_inner)
            }
            Type::Union(types) => {
                let new_types: Vec<_> = types
                    .iter()
                    .map(|&t| self.instantiate(type_table, t))
                    .collect();
                type_table.union(new_types)
            }
            Type::Struct(def) => {
                let fields = def
                    .fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        ty: self.instantiate(type_table, f.ty),
                    })
                    .collect();
                let methods = def
                    .methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|&p| self.instantiate(type_table, p))
                                .collect(),
                            return_type: self.instantiate(type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();
                let constructor_params = def.constructor_params.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|(name, ty)| (name.clone(), self.instantiate(type_table, *ty)))
                        .collect()
                });
                type_table.struct_def(StructDef {
                    name: def.name,
                    fields,
                    methods,
                    constructor_params,
                })
            }
            Type::Skill(def) => {
                let required_fields = def
                    .required_fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        ty: self.instantiate(type_table, f.ty),
                    })
                    .collect();
                let concrete_methods = def
                    .concrete_methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|&p| self.instantiate(type_table, p))
                                .collect(),
                            return_type: self.instantiate(type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();
                let abstract_methods = def
                    .abstract_methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|&p| self.instantiate(type_table, p))
                                .collect(),
                            return_type: self.instantiate(type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();
                type_table.skill_def(SkillTypeDef {
                    name: def.name,
                    required_fields,
                    concrete_methods,
                    abstract_methods,
                })
            }
            Type::Interface(def) => {
                let extends = def
                    .extends
                    .iter()
                    .map(|&t| self.instantiate(type_table, t))
                    .collect();
                let members = def
                    .members
                    .iter()
                    .map(|member| match member {
                        InterfaceMember::Property { name, ty } => InterfaceMember::Property {
                            name: name.clone(),
                            ty: self.instantiate(type_table, *ty),
                        },
                        InterfaceMember::Method { name, sig } => InterfaceMember::Method {
                            name: name.clone(),
                            sig: FunctionSig {
                                params: sig
                                    .params
                                    .iter()
                                    .map(|&p| self.instantiate(type_table, p))
                                    .collect(),
                                return_type: self.instantiate(type_table, sig.return_type),
                                is_async: sig.is_async,
                            },
                        },
                    })
                    .collect();
                type_table.interface_def(InterfaceDef {
                    name: def.name,
                    extends,
                    members,
                })
            }
            Type::ObjectShape(def) => {
                let fields = def
                    .fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        ty: self.instantiate(type_table, f.ty),
                    })
                    .collect();
                let methods = def
                    .methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        sig: FunctionSig {
                            params: m
                                .sig
                                .params
                                .iter()
                                .map(|&p| self.instantiate(type_table, p))
                                .collect(),
                            return_type: self.instantiate(type_table, m.sig.return_type),
                            is_async: m.sig.is_async,
                        },
                    })
                    .collect();
                type_table.object_shape(ObjectShapeDef { fields, methods })
            }
            Type::Enum(def) => type_table.enum_def(def),
            _ => type_id,
        }
    }
}

impl Default for TypeInstantiator {
    fn default() -> Self {
        Self::new()
    }
}
