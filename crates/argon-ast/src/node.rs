//! Argon - AST node definitions

use super::{Span, Spanned};

#[derive(Debug, Clone)]
pub enum Stmt {
    Empty(EmptyStmt),
    Expr(ExpressionStmt),
    Block(BlockStmt),
    If(IfStmt),
    Switch(SwitchStmt),
    For(ForStmt),
    ForIn(ForInStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Return(ReturnStmt),
    Throw(ThrowStmt),
    Try(TryStmt),
    With(WithStmt),
    Labeled(LabeledStmt),
    Debugger(DebuggerStmt),
    Variable(VariableStmt),
    Function(FunctionDecl),
    AsyncFunction(FunctionDecl),
    Class(ClassDecl),
    Struct(StructDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Interface(InterfaceDecl),
    TypeAlias(TypeAliasStmt),
    Enum(EnumDecl),
    Module(ModuleStmt),
    Import(ImportStmt),
    Export(ExportStmt),
    Match(MatchStmt),
    Loop(LoopStmt),
}

#[derive(Debug, Clone)]
pub struct EmptyStmt {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ExpressionStmt {
    pub expr: Expr,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub consequent: Box<Stmt>,
    pub alternate: Option<Box<Stmt>>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct SwitchStmt {
    pub discriminant: Expr,
    pub cases: Vec<SwitchCase>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<Expr>,
    pub consequent: Vec<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub discriminant: Expr,
    pub cases: Vec<MatchCase>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Expr,
    pub consequent: Box<Stmt>,
    pub guard: Option<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<ForInit>,
    pub test: Option<Expr>,
    pub update: Option<Expr>,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ForInit {
    Variable(VariableStmt),
    Expr(Expr),
}
#[derive(Debug, Clone)]
pub struct ForInStmt {
    pub left: ForInLeft,
    pub right: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ForInLeft {
    Pattern(Pattern),
    Variable(VariableDeclarator),
}
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct DoWhileStmt {
    pub body: Box<Stmt>,
    pub condition: Expr,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BreakStmt {
    pub label: Option<Ident>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ContinueStmt {
    pub label: Option<Ident>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct LoopStmt {
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub argument: Option<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ThrowStmt {
    pub argument: Expr,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TryStmt {
    pub block: BlockStmt,
    pub handler: Option<CatchClause>,
    pub finalizer: Option<BlockStmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<Pattern>,
    pub body: BlockStmt,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct WithStmt {
    pub object: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct LabeledStmt {
    pub label: Ident,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct DebuggerStmt {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VariableStmt {
    pub kind: VariableKind,
    pub declarations: Vec<VariableDeclarator>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq)]
pub enum VariableKind {
    Var,
    Let,
    Const,
}
#[derive(Debug, Clone)]
pub struct VariableDeclarator {
    pub id: Pattern,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub id: Option<Ident>,
    pub params: Vec<Param>,
    pub body: FunctionBody,
    pub type_params: Vec<TypeParam>,
    pub return_type: Option<Box<Type>>,
    pub is_async: bool,
    pub borrow_annotation: Option<BorrowAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BorrowAnnotation {
    pub kind: BorrowKind,
    pub target: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,
    Mutable,
}
#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub statements: Vec<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct Param {
    pub pat: Pattern,
    pub ty: Option<Box<Type>>,
    pub default: Option<Expr>,
    pub is_optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub id: Ident,
    pub type_params: Vec<TypeParam>,
    pub super_class: Option<Box<Type>>,
    pub super_type_args: Vec<Type>,
    pub implements: Vec<Type>,
    pub body: ClassBody,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ClassBody {
    pub body: Vec<ClassMember>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ClassMember {
    Method(MethodDefinition),
    Field(ClassField),
    Constructor(Constructor),
    IndexSignature(IndexSignature),
}
#[derive(Debug, Clone)]
pub struct MethodDefinition {
    pub key: Expr,
    pub value: FunctionDecl,
    pub kind: MethodKind,
    pub is_static: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum MethodKind {
    Constructor,
    Method,
    Get,
    Set,
}
#[derive(Debug, Clone)]
pub struct ClassField {
    pub key: Expr,
    pub value: Option<Expr>,
    pub type_annotation: Option<Box<Type>>,
    pub is_optional: bool,
    pub is_readonly: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct Constructor {
    pub params: Vec<Param>,
    pub body: BlockStmt,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct IndexSignature {
    pub params: Vec<Param>,
    pub return_type: Box<Type>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub id: Ident,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<StructField>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct StructField {
    pub id: Ident,
    pub type_annotation: Box<Type>,
    pub is_readonly: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub id: Ident,
    pub type_params: Vec<TypeParam>,
    pub body: TraitBody,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TraitBody {
    pub items: Vec<TraitItem>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum TraitItem {
    Method(MethodSignature),
    Property(PropertySignature),
}
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub id: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Box<Type>>,
    pub type_params: Vec<TypeParam>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct PropertySignature {
    pub id: Ident,
    pub type_annotation: Option<Box<Type>>,
    pub is_optional: bool,
    pub is_readonly: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub type_params: Vec<TypeParam>,
    pub trait_type: Option<Box<Type>>,
    pub struct_type: Box<Type>,
    pub body: ImplBody,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ImplBody {
    pub items: Vec<ImplItem>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ImplItem {
    Method(MethodDefinition),
    Field(ClassField),
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub id: Ident,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<Type>,
    pub body: InterfaceBody,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct InterfaceBody {
    pub body: Vec<InterfaceMember>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Method(MethodSignature),
    Property(PropertySignature),
    IndexSignature(IndexSignature),
}

#[derive(Debug, Clone)]
pub struct TypeAliasStmt {
    pub id: Ident,
    pub type_params: Vec<TypeParam>,
    pub type_annotation: Box<Type>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub id: Ident,
    pub members: Vec<EnumMember>,
    pub is_const: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct EnumMember {
    pub id: Ident,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModuleStmt {
    pub body: Vec<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ImportStmt {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: StringLiteral,
    pub is_type_only: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ImportSpecifier {
    Named(NamedImportSpecifier),
    Default(DefaultImportSpecifier),
    Namespace(NamespaceImportSpecifier),
}
#[derive(Debug, Clone)]
pub struct NamedImportSpecifier {
    pub imported: Ident,
    pub local: Option<Ident>,
    pub is_type: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct DefaultImportSpecifier {
    pub local: Ident,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NamespaceImportSpecifier {
    pub id: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExportStmt {
    pub declaration: Option<Box<Stmt>>,
    pub specifiers: Vec<ExportSpecifier>,
    pub source: Option<StringLiteral>,
    pub is_type_only: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ExportSpecifier {
    pub orig: Ident,
    pub exported: Option<Ident>,
    pub span: Span,
}

// Expressions

#[derive(Debug, Clone)]
pub enum Expr {
    This(ThisExpr),
    Super(SuperExpr),
    Identifier(Ident),
    Literal(Literal),
    Template(TemplateLiteral),
    Member(MemberExpr),
    Call(CallExpr),
    New(NewExpr),
    Update(UpdateExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Logical(LogicalExpr),
    Conditional(ConditionalExpr),
    Assignment(Box<AssignmentExpr>),
    ArrowFunction(Box<ArrowFunctionExpr>),
    Yield(YieldExpr),
    Await(AwaitExpr),
    AwaitPromised(AwaitExpr),
    Spread(SpreadElement),
    Array(ArrayExpression),
    Object(ObjectExpression),
    Function(FunctionExpr),
    Class(ClassExpression),
    JsxElement(JsxElement),
    JsxFragment(JsxFragment),
    TypeAssertion(TypeAssertionExpr),
    AsType(AsTypeExpr),
    NonNull(NonNullExpr),
    MetaProperty(MetaProperty),
    Chain(ChainExpr),
    OptionalCall(OptionalCallExpr),
    OptionalMember(OptionalMemberExpr),
    Ref(RefExpr),
    MutRef(MutRefExpr),
    Import(ImportExpr),
    TaggedTemplate(TaggedTemplateExpr),
    Regex(RegExpLiteral),
    Parenthesized(ParenthesizedExpr),
    AssignmentTargetPattern(AssignmentTargetPattern),
}

#[derive(Debug, Clone)]
pub struct ThisExpr {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct SuperExpr {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct Ident {
    pub sym: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(NumberLiteral),
    String(StringLiteral),
    Boolean(BooleanLiteral),
    Null(NullLiteral),
    Undefined(UndefinedLiteral),
    BigInt(BigIntLiteral),
    RegExp(RegExpLiteral),
}
#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub value: f64,
    pub raw: String,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub value: String,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BooleanLiteral {
    pub value: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NullLiteral {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct UndefinedLiteral {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BigIntLiteral {
    pub value: String,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct RegExpLiteral {
    pub pattern: String,
    pub flags: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteral {
    pub quasis: Vec<TemplateElement>,
    pub expressions: Vec<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub value: String,
    pub tail: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemberExpr {
    pub object: Box<Expr>,
    pub property: Box<Expr>,
    pub computed: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub arguments: Vec<ExprOrSpread>,
    pub type_args: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NewExpr {
    pub callee: Box<Expr>,
    pub arguments: Vec<ExprOrSpread>,
    pub type_args: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ExprOrSpread {
    Expr(Expr),
    Spread(SpreadElement),
}
#[derive(Debug, Clone)]
pub struct SpreadElement {
    pub argument: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct UpdateExpr {
    pub argument: Box<Expr>,
    pub operator: UpdateOperator,
    pub prefix: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum UpdateOperator {
    Increment,
    Decrement,
}
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub argument: Box<Expr>,
    pub operator: UnaryOperator,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Delete,
    Void,
    Typeof,
    Plus,
    Minus,
    BitwiseNot,
    LogicalNot,
}
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: BinaryOperator,
    pub right: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LeftShift,
    RightShift,
    UnsignedRightShift,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Exponent,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    In,
    Instanceof,
    As,
}
#[derive(Debug, Clone)]
pub struct LogicalExpr {
    pub left: Box<Expr>,
    pub operator: LogicalOperator,
    pub right: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum LogicalOperator {
    Or,
    And,
    NullishCoalescing,
}
#[derive(Debug, Clone)]
pub struct ConditionalExpr {
    pub test: Box<Expr>,
    pub consequent: Box<Expr>,
    pub alternate: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct AssignmentExpr {
    pub left: Box<AssignmentTarget>,
    pub operator: AssignmentOperator,
    pub right: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Simple(Box<Expr>),
    Member(MemberExpr),
    Pattern(Pattern),
}
#[derive(Debug, Clone)]
pub enum AssignmentOperator {
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    ExponentAssign,
    LeftShiftAssign,
    RightShiftAssign,
    UnsignedRightShiftAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    LogicalAndAssign,
    LogicalOrAssign,
    NullishCoalescingAssign,
}

#[derive(Debug, Clone)]
pub struct ArrowFunctionExpr {
    pub params: Vec<Param>,
    pub body: ArrowFunctionBody,
    pub type_params: Vec<TypeParam>,
    pub return_type: Option<Box<Type>>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ArrowFunctionBody {
    Block(BlockStmt),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct YieldExpr {
    pub argument: Option<Box<Expr>>,
    pub delegate: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct AwaitExpr {
    pub argument: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ArrayExpression {
    pub elements: Vec<Option<ExprOrSpread>>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ObjectExpression {
    pub properties: Vec<ObjectProperty>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ObjectProperty {
    Property(Property),
    Shorthand(Ident),
    Spread(SpreadElement),
    Method(MethodDefinition),
    Getter(MethodDefinition),
    Setter(MethodDefinition),
}
#[derive(Debug, Clone)]
pub struct Property {
    pub key: Expr,
    pub value: ExprOrSpread,
    pub kind: PropertyKind,
    pub method: bool,
    pub shorthand: bool,
    pub computed: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}
#[derive(Debug, Clone)]
pub struct FunctionExpr {
    pub id: Option<Ident>,
    pub params: Vec<Param>,
    pub body: FunctionBody,
    pub type_params: Vec<TypeParam>,
    pub return_type: Option<Box<Type>>,
    pub is_async: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ClassExpression {
    pub id: Option<Ident>,
    pub super_class: Option<Box<Type>>,
    pub body: ClassBody,
    pub type_params: Vec<TypeParam>,
    pub span: Span,
}

// JSX
#[derive(Debug, Clone)]
pub struct JsxElement {
    pub opening: JsxOpeningElement,
    pub children: Vec<JsxChild>,
    pub closing: Option<JsxClosingElement>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxFragment {
    pub opening: JsxOpeningFragment,
    pub children: Vec<JsxChild>,
    pub closing: JsxClosingFragment,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxOpeningElement {
    pub name: JsxElementName,
    pub attributes: Vec<JsxAttribute>,
    pub self_closing: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum JsxElementName {
    Identifier(Ident),
    Namespaced(JsxNamespacedName),
    Member(Box<JsxElementName>),
}
#[derive(Debug, Clone)]
pub struct JsxNamespacedName {
    pub namespace: Ident,
    pub name: Ident,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxMemberExpression {
    pub object: Box<JsxElementName>,
    pub property: Ident,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxClosingElement {
    pub name: JsxElementName,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxAttribute {
    pub name: JsxAttributeName,
    pub value: Option<JsxAttributeValue>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum JsxAttributeName {
    Identifier(Ident),
    Namespaced(JsxNamespacedName),
}
#[derive(Debug, Clone)]
pub enum JsxAttributeValue {
    String(StringLiteral),
    Expression(Expr),
    Element(JsxElement),
    Fragment(JsxFragment),
    Span(Span),
}
#[derive(Debug, Clone)]
pub enum JsxChild {
    Text(JsxText),
    Expression(Expr),
    Element(JsxElement),
    Fragment(JsxFragment),
    Spread(JsxSpreadChild),
}
#[derive(Debug, Clone)]
pub struct JsxText {
    pub value: String,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxSpreadChild {
    pub expression: Expr,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxOpeningFragment {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct JsxClosingFragment {
    pub span: Span,
}

// Type expressions
#[derive(Debug, Clone)]
pub enum Type {
    Keyword(TypeKeyword),
    Primitive(PrimitiveType),
    Reference(TypeReference),
    Array(ArrayType),
    Tuple(TupleType),
    Object(ObjectType),
    Function(FunctionType),
    Union(UnionType),
    Intersection(IntersectionType),
    Conditional(ConditionalType),
    Infer(Box<InferType>),
    Index(IndexType),
    Lookup(LookupType),
    Recursive(RecursiveType),
    Parenthesized(Box<Type>),
    Query(Box<QueryType>),
    Mapped(MappedType),
    Operator(TypeOperator),
    Template(TemplateType),
    String(StringType),
    Number(NumberType),
    Boolean(BooleanType),
    BigInt(BigIntType),
    Symbol(SymbolType),
    ThisType(ThisType),
    Unknown(UnknownType),
    Never(NeverType),
    Void(VoidType),
    Null(NullType),
    Undefined(UndefinedType),
    Any(AnyType),
    Ref(RefType),
    MutRef(MutRefType),
    Shared(SharedType),
    Optional(OptionalType),
    Predicate(PredicateType),
    TypeQuery(TypeQueryType),
}

#[derive(Debug, Clone)]
pub struct TypeKeyword {
    pub kind: TypeKeywordKind,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum TypeKeywordKind {
    Type,
    Interface,
    Class,
    Enum,
    Module,
    Namespace,
    Abstract,
    Static,
    Readonly,
    Public,
    Private,
    Protected,
    Constructor,
    Export,
    Import,
}

#[derive(Debug, Clone)]
pub enum PrimitiveType {
    Number,
    String,
    Boolean,
    Void,
    Null,
    Undefined,
    Any,
    Unknown,
    Never,
    Symbol,
    BigInt,
    Object,
}

#[derive(Debug, Clone)]
pub struct TypeReference {
    pub name: TypeName,
    pub type_args: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum TypeName {
    Ident(Ident),
    Qualified(Box<TypeName>, Ident),
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub elem_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TupleType {
    pub types: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ObjectType {
    pub members: Vec<TypeMember>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum TypeMember {
    Property(PropertySignature),
    Method(MethodSignature),
    IndexSignature(IndexSignature),
    CallSignature(CallSignatureType),
    ConstructSignature(ConstructSignatureType),
}
#[derive(Debug, Clone)]
pub struct FunctionType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct FunctionTypeParam {
    pub name: Option<Ident>,
    pub ty: Type,
    pub optional: bool,
}
#[derive(Debug, Clone)]
pub struct UnionType {
    pub types: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct IntersectionType {
    pub types: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ConditionalType {
    pub check_type: Box<Type>,
    pub extends_type: Box<Type>,
    pub true_type: Box<Type>,
    pub false_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct InferType {
    pub type_param: Box<TypeParam>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct IndexType {
    pub key_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct LookupType {
    pub object_type: Box<Type>,
    pub key_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct RecursiveType {
    pub name: Ident,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct QueryType {
    pub expr: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MappedType {
    pub type_param: TypeParam,
    pub name_type: Option<Box<Type>>,
    pub prop_type: Option<Box<Type>>,
    pub optional: MappedTypeOptional,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum MappedTypeOptional {
    Plus,
    Minus,
    None,
}
#[derive(Debug, Clone)]
pub struct TypeOperator {
    pub operator: TypeOperatorKind,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum TypeOperatorKind {
    Keyof,
    Unique,
    Readonly,
}
#[derive(Debug, Clone)]
pub struct TemplateType {
    pub text: String,
    pub types: Vec<Type>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StringType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NumberType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BooleanType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct BigIntType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct SymbolType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ThisType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct UnknownType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NeverType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct VoidType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NullType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct UndefinedType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct AnyType {
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct RefType {
    pub lifetime: Option<Ident>,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MutRefType {
    pub lifetime: Option<Ident>,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct SharedType {
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct OptionalType {
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct PredicateType {
    pub param_name: Option<Ident>,
    pub type_annotation: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TypeQueryType {
    pub type_name: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: Ident,
    pub constraint: Option<Box<Type>>,
    pub default: Option<Box<Type>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAssertionExpr {
    pub expression: Box<Expr>,
    pub type_annotation: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct AsTypeExpr {
    pub expression: Box<Expr>,
    pub type_annotation: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct NonNullExpr {
    pub expression: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MetaProperty {
    pub meta: Ident,
    pub property: Ident,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ChainExpr {
    pub expressions: Vec<ChainElement>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ChainElement {
    Call(CallExpr),
    Member(MemberExpr),
    OptionalCall(OptionalCallExpr),
    OptionalMember(OptionalMemberExpr),
}
#[derive(Debug, Clone)]
pub struct OptionalCallExpr {
    pub callee: Box<Expr>,
    pub arguments: Vec<ExprOrSpread>,
    pub type_args: Vec<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct OptionalMemberExpr {
    pub object: Box<Expr>,
    pub property: Box<Expr>,
    pub computed: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct RefExpr {
    pub expr: Box<Expr>,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MutRefExpr {
    pub expr: Box<Expr>,
    pub ty: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ParenthesizedExpr {
    pub expression: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ImportExpr {
    pub source: Box<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct TaggedTemplateExpr {
    pub tag: Box<Expr>,
    pub template: TemplateLiteral,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum AssignmentTargetPattern {
    Identifier(Ident),
    Member(MemberExpr),
    Array(ArrayAssignmentPattern),
    Object(ObjectAssignmentPattern),
    Rest(RestAssignmentTarget),
    Parenthesized(Box<AssignmentTargetPattern>),
}
#[derive(Debug, Clone)]
pub struct ArrayAssignmentPattern {
    pub elements: Vec<Option<AssignmentTargetPattern>>,
    pub rest: Option<RestAssignmentTarget>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ObjectAssignmentPattern {
    pub properties: Vec<AssignmentTargetProperty>,
    pub rest: Option<RestAssignmentTarget>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum AssignmentTargetProperty {
    Shorthand(Ident),
    KeyValue(AssignmentTargetPropertyKeyValue),
}
#[derive(Debug, Clone)]
pub struct AssignmentTargetPropertyKeyValue {
    pub key: Ident,
    pub target: AssignmentTargetPattern,
}
#[derive(Debug, Clone)]
pub struct RestAssignmentTarget {
    pub target: Option<Box<AssignmentTargetPattern>>,
    pub span: Span,
}

// Patterns
#[derive(Debug, Clone)]
pub enum Pattern {
    Identifier(IdentPattern),
    Object(Box<ObjectPattern>),
    Array(Box<ArrayPattern>),
    Rest(Box<RestElement>),
    Assignment(Box<AssignmentPattern>),
    Member(MemberPattern),
}
#[derive(Debug, Clone)]
pub struct IdentPattern {
    pub name: Ident,
    pub type_annotation: Option<Box<Type>>,
    pub default: Option<Expr>,
}
#[derive(Debug, Clone)]
pub struct ObjectPattern {
    pub properties: Vec<ObjectPatternProperty>,
    pub rest: Option<Box<RestElement>>,
}
#[derive(Debug, Clone)]
pub enum ObjectPatternProperty {
    Property(KeyValuePattern),
    Assignment(AssignmentPattern),
    Rest(RestElement),
}
#[derive(Debug, Clone)]
pub struct KeyValuePattern {
    pub key: Expr,
    pub value: Pattern,
    pub computed: bool,
}
#[derive(Debug, Clone)]
pub struct ArrayPattern {
    pub elements: Vec<Option<Pattern>>,
    pub rest: Option<Box<RestElement>>,
}
#[derive(Debug, Clone)]
pub struct RestElement {
    pub argument: Box<Pattern>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct AssignmentPattern {
    pub left: Box<Pattern>,
    pub right: Expr,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct MemberPattern {
    pub object: Box<Pattern>,
    pub property: Ident,
    pub computed: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallSignatureType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<Type>,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct ConstructSignatureType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<Type>,
    pub span: Span,
}

// Spanned implementations
impl Spanned for Expr {
    fn span(&self) -> &Span {
        match self {
            Expr::This(t) => &t.span,
            Expr::Super(s) => &s.span,
            Expr::Identifier(i) => &i.span,
            Expr::Literal(l) => match l {
                Literal::Number(n) => &n.span,
                Literal::String(s) => &s.span,
                Literal::Boolean(b) => &b.span,
                Literal::Null(n) => &n.span,
                Literal::Undefined(u) => &u.span,
                Literal::BigInt(b) => &b.span,
                Literal::RegExp(r) => &r.span,
            },
            Expr::Template(t) => &t.span,
            Expr::Member(m) => &m.span,
            Expr::Call(c) => &c.span,
            Expr::New(n) => &n.span,
            Expr::Update(u) => &u.span,
            Expr::Unary(u) => &u.span,
            Expr::Binary(b) => &b.span,
            Expr::Logical(l) => &l.span,
            Expr::Conditional(c) => &c.span,
            Expr::Assignment(a) => &a.span,
            Expr::ArrowFunction(a) => &a.span,
            Expr::Yield(y) => &y.span,
            Expr::Await(a) => &a.span,
            Expr::AwaitPromised(a) => &a.span,
            Expr::Spread(s) => &s.span,
            Expr::Array(a) => &a.span,
            Expr::Object(o) => &o.span,
            Expr::Function(f) => &f.span,
            Expr::Class(c) => &c.span,
            Expr::JsxElement(e) => &e.span,
            Expr::JsxFragment(f) => &f.span,
            Expr::TypeAssertion(t) => &t.span,
            Expr::AsType(a) => &a.span,
            Expr::NonNull(n) => &n.span,
            Expr::MetaProperty(m) => &m.span,
            Expr::Chain(c) => &c.span,
            Expr::OptionalCall(c) => &c.span,
            Expr::OptionalMember(m) => &m.span,
            Expr::Ref(r) => &r.span,
            Expr::MutRef(r) => &r.span,
            Expr::Import(i) => &i.span,
            Expr::TaggedTemplate(t) => &t.span,
            Expr::Regex(r) => &r.span,
            Expr::Parenthesized(p) => &p.span,
            Expr::AssignmentTargetPattern(a) => match a {
                AssignmentTargetPattern::Identifier(i) => &i.span,
                AssignmentTargetPattern::Member(m) => &m.span,
                AssignmentTargetPattern::Array(a) => &a.span,
                AssignmentTargetPattern::Object(o) => &o.span,
                AssignmentTargetPattern::Rest(r) => &r.span,
                AssignmentTargetPattern::Parenthesized(p) => match &**p {
                    AssignmentTargetPattern::Identifier(i) => &i.span,
                    AssignmentTargetPattern::Member(m) => &m.span,
                    AssignmentTargetPattern::Array(a) => &a.span,
                    AssignmentTargetPattern::Object(o) => &o.span,
                    AssignmentTargetPattern::Rest(r) => &r.span,
                    AssignmentTargetPattern::Parenthesized(_) => &EMPTY_SPAN,
                },
            },
        }
    }
}

macro_rules! impl_spanned {
    ($($ty:ty),*) => { $(
        impl Spanned for $ty { fn span(&self) -> &Span { &self.span } }
    )* };
}

impl_spanned!(
    EmptyStmt,
    ExpressionStmt,
    BlockStmt,
    IfStmt,
    SwitchStmt,
    SwitchCase,
    ForStmt,
    ForInStmt,
    WhileStmt,
    DoWhileStmt,
    BreakStmt,
    ContinueStmt,
    ReturnStmt,
    ThrowStmt,
    TryStmt,
    CatchClause,
    WithStmt,
    LabeledStmt,
    DebuggerStmt,
    VariableStmt,
    VariableDeclarator,
    FunctionDecl,
    FunctionBody,
    Param,
    ClassDecl,
    ClassBody,
    StructDecl,
    StructField,
    TraitDecl,
    TraitBody,
    ImplDecl,
    ImplBody,
    InterfaceDecl,
    TypeAliasStmt,
    EnumDecl,
    EnumMember,
    ModuleStmt,
    ImportStmt,
    ExportStmt,
    ThisExpr,
    SuperExpr,
    Ident,
    NumberLiteral,
    StringLiteral,
    BooleanLiteral,
    NullLiteral,
    RegExpLiteral,
    TemplateLiteral,
    TemplateElement,
    MemberExpr,
    CallExpr,
    NewExpr,
    SpreadElement,
    UpdateExpr,
    UnaryExpr,
    BinaryExpr,
    LogicalExpr,
    ConditionalExpr,
    AssignmentExpr,
    ArrowFunctionExpr,
    YieldExpr,
    AwaitExpr,
    ArrayExpression,
    ObjectExpression,
    FunctionExpr,
    ClassExpression,
    JsxElement,
    JsxFragment,
    JsxOpeningElement,
    JsxClosingElement,
    JsxAttribute,
    JsxText,
    TypeReference,
    ArrayType,
    TupleType,
    ObjectType,
    FunctionType,
    UnionType,
    IntersectionType,
    ConditionalType,
    TypeParam,
    TypeAssertionExpr,
    AsTypeExpr,
    NonNullExpr,
    ChainExpr,
    RefType,
    MutRefType,
    SharedType,
    RestElement
);

impl Spanned for JsxChild {
    fn span(&self) -> &Span {
        match self {
            JsxChild::Text(t) => &t.span,
            JsxChild::Expression(e) => e.span(),
            JsxChild::Element(e) => &e.span,
            JsxChild::Fragment(f) => &f.span,
            JsxChild::Spread(s) => &s.span,
        }
    }
}

static EMPTY_SPAN: Span = Span { start: 0, end: 0 };

impl Spanned for Type {
    fn span(&self) -> &Span {
        match self {
            Type::Keyword(k) => &k.span,
            Type::Reference(r) => &r.span,
            Type::Array(a) => &a.span,
            Type::Tuple(t) => &t.span,
            Type::Object(o) => &o.span,
            Type::Function(f) => &f.span,
            Type::Union(u) => &u.span,
            Type::Intersection(i) => &i.span,
            Type::Conditional(c) => &c.span,
            Type::Infer(i) => &i.span,
            Type::Index(i) => &i.span,
            Type::Lookup(l) => &l.span,
            Type::Recursive(r) => &r.span,
            Type::Parenthesized(p) => p.span(),
            Type::Query(q) => &q.span,
            Type::Mapped(m) => &m.span,
            Type::Operator(o) => &o.span,
            Type::Template(t) => &t.span,
            Type::Ref(r) => &r.span,
            Type::MutRef(r) => &r.span,
            Type::Shared(s) => &s.span,
            Type::Optional(o) => &o.span,
            Type::Predicate(p) => &p.span,
            Type::TypeQuery(t) => &t.span,
            _ => &EMPTY_SPAN,
        }
    }
}

impl Spanned for Pattern {
    fn span(&self) -> &Span {
        match self {
            Pattern::Identifier(i) => &i.name.span,
            _ => &EMPTY_SPAN,
        }
    }
}
