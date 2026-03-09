//! SafeScript - Type definitions


pub type TypeId = u32;

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
}

#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub sig: FunctionSig,
}

#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    pub extends: Vec<TypeId>,
    pub members: Vec<InterfaceMember>,
}

#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Property { name: String, ty: TypeId },
    Method { name: String, sig: FunctionSig },
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Option<TypeId>,
    pub default: Option<TypeId>,
}

#[derive(Debug, Clone)]
pub struct TypeTable {
    types: Vec<Type>,
}

impl TypeTable {
    pub fn new() -> Self {
        Self { types: Vec::new() }
    }

    pub fn add(&mut self, ty: Type) -> TypeId {
        let id = self.types.len() as TypeId;
        self.types.push(ty);
        id
    }

    pub fn get(&self, id: TypeId) -> Option<&Type> {
        self.types.get(id as usize)
    }
}
