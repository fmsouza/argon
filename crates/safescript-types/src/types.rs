//! SafeScript - Type definitions

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
    Class(ClassDef),
    Interface(InterfaceDef),
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
            Type::Boolean | Type::Number | Type::String | Type::Object
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
            Type::Class(def) => write!(f, "class {}", def.name),
            Type::Interface(def) => write!(f, "interface {}", def.name),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDef {
    pub name: String,
    pub sig: FunctionSig,
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

#[derive(Debug, Clone)]
pub struct TypeTable {
    types: Vec<Type>,
    names: HashMap<String, TypeId>,
}

impl Default for TypeTable {
    fn default() -> Self {
        Self {
            types: Vec::new(),
            names: HashMap::new(),
        }
    }
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

    pub fn class_def(&mut self, def: ClassDef) -> TypeId {
        self.get_or_add(Type::Class(def))
    }

    pub fn type_param(&mut self, param: TypeParam) -> TypeId {
        self.get_or_add(Type::TypeParam(param))
    }

    pub fn generic(&mut self, name: String) -> TypeId {
        self.get_or_add(Type::Generic(name))
    }
}
