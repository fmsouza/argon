//! Argon - Token definitions

use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Range<usize>,
}

impl Token {
    pub fn new(kind: TokenKind, span: Range<usize>) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Whitespace and comments (filtered out during tokenization)
    Whitespace,
    Comment,

    // Punctuation
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Semi,
    Comma,
    Dot,
    DotDot,
    DotDotDot,
    Question,
    QuestionDot,
    QuestionQuestion,
    QuestionQuestionDot,
    QuestionQuestionEqual,
    Colon,
    Tilde,

    // Operators
    Plus,
    PlusPlus,
    PlusEqual,
    Minus,
    MinusMinus,
    MinusEqual,
    Arrow,
    Star,
    StarStar,
    StarEqual,
    Slash,
    SlashEqual,
    Percent,
    PercentEqual,
    Caret,
    CaretCaret,
    CaretEqual,
    Bang,
    BangEqual,
    BangEqualEqual,
    Ampersand,
    AmpersandAmpersand,
    AmpersandEqual,
    Pipe,
    PipePipe,
    PipeEqual,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    FatArrow,
    LessThan,
    LessThanEqual,
    LessThanLessThan,
    LessThanLessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    GreaterThanGreaterThan,
    GreaterThanGreaterThanEqual,
    GreaterThanGreaterThanGreaterThan,
    GreaterThanGreaterThanGreaterThanEqual,

    // Literals
    NumberLiteral,
    StringLiteral,
    TemplateComplete,
    TemplateMiddle,
    TemplateStart,
    UnterminatedString,
    UnterminatedTemplate,
    True,
    False,
    Null,
    Undefined,

    // Identifiers
    Identifier,

    // Keywords
    Abstract,
    Any,
    As,
    Async,
    Await,
    NumberKw,
    StringKw,
    Boolean,
    BigInt,
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Declare,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    Finally,
    For,
    From,
    Function,
    Get,
    If,
    Implements,
    Import,
    In,
    Infer,
    Instanceof,
    Interface,
    Is,
    Keyof,
    Let,
    Module,
    Namespace,
    Never,
    New,
    Object,
    Of,
    Package,
    Private,
    Protected,
    Public,
    Readonly,
    Require,
    Return,
    Set,
    Static,
    Super,
    Switch,
    Symbol,
    This,
    Throw,
    Try,
    Type,
    Typeof,
    Unique,
    Unknown,
    Var,
    Void,
    While,
    With,
    Yield,

    // Argon-specific keywords
    Struct,
    Trait,
    Impl,
    Match,
    Shared, // Shared<T> type
    Move,   // move semantics
    Copy,   // copy trait
    Mut,    // mutable borrow
    Constructor,
    // Numeric types
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Isize,
    Usize,

    // JSX
    JsxElementOpen,
    JsxFragmentOpen,
    JsxElementClose,
    JsxFragmentClose,
    JsxSelfClosing,
    JsxAttribute,
    JsxSpreadAttribute,
    JsxChild,

    // Special
    Eof,
    Error,
}

impl TokenKind {
    pub fn from_keyword(text: &str) -> Self {
        match text {
            // Argon keywords
            "struct" => TokenKind::Struct,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "match" => TokenKind::Match,
            "with" => TokenKind::With,
            "shared" => TokenKind::Shared,
            "move" => TokenKind::Move,
            "copy" => TokenKind::Copy,
            "mut" => TokenKind::Mut,
            "constructor" => TokenKind::Constructor,

            // Numeric types
            "i8" => TokenKind::I8,
            "i16" => TokenKind::I16,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "u8" => TokenKind::U8,
            "u16" => TokenKind::U16,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "isize" => TokenKind::Isize,
            "usize" => TokenKind::Usize,

            // Standard TypeScript keywords
            "abstract" => TokenKind::Abstract,
            "any" => TokenKind::Any,
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "boolean" => TokenKind::Boolean,
            "bigint" => TokenKind::BigInt,
            "break" => TokenKind::Break,
            "case" => TokenKind::Case,
            "catch" => TokenKind::Catch,
            "class" => TokenKind::Class,
            "const" => TokenKind::Const,
            "continue" => TokenKind::Continue,
            "debugger" => TokenKind::Debugger,
            "declare" => TokenKind::Declare,
            "default" => TokenKind::Default,
            "delete" => TokenKind::Delete,
            "do" => TokenKind::Do,
            "else" => TokenKind::Else,
            "enum" => TokenKind::Enum,
            "export" => TokenKind::Export,
            "false" => TokenKind::False,
            "extends" => TokenKind::Extends,
            "finally" => TokenKind::Finally,
            "for" => TokenKind::For,
            "from" => TokenKind::From,
            "function" => TokenKind::Function,
            "get" => TokenKind::Get,
            "if" => TokenKind::If,
            "implements" => TokenKind::Implements,
            "import" => TokenKind::Import,
            "in" => TokenKind::In,
            "infer" => TokenKind::Infer,
            "instanceof" => TokenKind::Instanceof,
            "interface" => TokenKind::Interface,
            "is" => TokenKind::Is,
            "keyof" => TokenKind::Keyof,
            "let" => TokenKind::Let,
            "module" => TokenKind::Module,
            "namespace" => TokenKind::Namespace,
            "never" => TokenKind::Never,
            "new" => TokenKind::New,
            "null" => TokenKind::Null,
            "number" => TokenKind::NumberKw,
            "object" => TokenKind::Object,
            "of" => TokenKind::Of,
            "package" => TokenKind::Package,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "public" => TokenKind::Public,
            "readonly" => TokenKind::Readonly,
            "require" => TokenKind::Require,
            "return" => TokenKind::Return,
            "set" => TokenKind::Set,
            "static" => TokenKind::Static,
            "string" => TokenKind::StringKw,
            "super" => TokenKind::Super,
            "switch" => TokenKind::Switch,
            "symbol" => TokenKind::Symbol,
            "this" => TokenKind::This,
            "throw" => TokenKind::Throw,
            "true" => TokenKind::True,
            "try" => TokenKind::Try,
            "type" => TokenKind::Type,
            "typeof" => TokenKind::Typeof,
            "undefined" => TokenKind::Undefined,
            "unique" => TokenKind::Unique,
            "unknown" => TokenKind::Unknown,
            "var" => TokenKind::Var,
            "void" => TokenKind::Void,
            "while" => TokenKind::While,
            "yield" => TokenKind::Yield,

            _ => TokenKind::Identifier,
        }
    }

    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Struct
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Match
                | TokenKind::With
                | TokenKind::Shared
                | TokenKind::Move
                | TokenKind::Copy
                | TokenKind::Constructor
                | TokenKind::Abstract
                | TokenKind::Any
                | TokenKind::As
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Boolean
                | TokenKind::BigInt
                | TokenKind::Break
                | TokenKind::Case
                | TokenKind::Catch
                | TokenKind::Class
                | TokenKind::Const
                | TokenKind::Continue
                | TokenKind::Debugger
                | TokenKind::Declare
                | TokenKind::Default
                | TokenKind::Delete
                | TokenKind::Do
                | TokenKind::Else
                | TokenKind::Enum
                | TokenKind::Export
                | TokenKind::Extends
                | TokenKind::Finally
                | TokenKind::For
                | TokenKind::From
                | TokenKind::Function
                | TokenKind::Get
                | TokenKind::If
                | TokenKind::Implements
                | TokenKind::Import
                | TokenKind::In
                | TokenKind::Infer
                | TokenKind::Instanceof
                | TokenKind::Interface
                | TokenKind::Is
                | TokenKind::Keyof
                | TokenKind::Let
                | TokenKind::Module
                | TokenKind::Namespace
                | TokenKind::Never
                | TokenKind::New
                | TokenKind::Null
                | TokenKind::NumberKw
                | TokenKind::Object
                | TokenKind::Of
                | TokenKind::Package
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::Public
                | TokenKind::Readonly
                | TokenKind::Require
                | TokenKind::Return
                | TokenKind::Set
                | TokenKind::Static
                | TokenKind::StringKw
                | TokenKind::Super
                | TokenKind::Switch
                | TokenKind::Symbol
                | TokenKind::This
                | TokenKind::Throw
                | TokenKind::True
                | TokenKind::Try
                | TokenKind::Type
                | TokenKind::Typeof
                | TokenKind::Undefined
                | TokenKind::Unique
                | TokenKind::Unknown
                | TokenKind::Var
                | TokenKind::Void
                | TokenKind::While
                | TokenKind::Yield
        )
    }
}
