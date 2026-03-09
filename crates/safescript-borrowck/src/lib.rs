//! SafeScript - Borrow checker

use safescript_ast::*;

pub struct BorrowChecker;

impl BorrowChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn check(&mut self, source: &SourceFile) -> Result<(), BorrowError> {
        for stmt in &source.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), BorrowError> {
        match stmt {
            Stmt::Function(f) => {
                for param in &f.params {
                    self.check_pattern(&param.pat)?;
                }
                self.check_block(&f.body)?;
            }
            Stmt::Block(b) => {
                for stmt in &b.statements {
                    self.check_statement(stmt)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn check_block(&mut self, body: &FunctionBody) -> Result<(), BorrowError> {
        for stmt in &body.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_pattern(&mut self, pattern: &Pattern) -> Result<(), BorrowError> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum BorrowError {
    UseAfterMove,
    BorrowConflict,
    LifetimeError(String),
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::UseAfterMove => write!(f, "Use after move"),
            BorrowError::BorrowConflict => write!(f, "Borrow conflict"),
            BorrowError::LifetimeError(msg) => write!(f, "Lifetime error: {}", msg),
        }
    }
}

impl std::error::Error for BorrowError {}
