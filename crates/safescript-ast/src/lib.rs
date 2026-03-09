//! SafeScript - Abstract Syntax Tree

mod node;
mod visit;

pub use node::*;
pub use visit::*;

use std::ops::Range;

pub type Span = Range<usize>;

pub trait Spanned: Sized {
    fn span(&self) -> &Span;
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub statements: Vec<Stmt>,
    pub eof_span: Span,
}

impl SourceFile {
    pub fn new(path: String, statements: Vec<Stmt>, eof_span: Span) -> Self {
        Self {
            path,
            statements,
            eof_span,
        }
    }
}
