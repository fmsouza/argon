//! SafeScript - Lexer errors

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LexerError {
    #[error("unexpected character: `{0}` at position {1}")]
    UnexpectedCharacter(char, usize),

    #[error("unterminated string literal")]
    UnterminatedString(usize),

    #[error("unterminated template literal")]
    UnterminatedTemplate(usize),

    #[error("invalid number literal: {0}")]
    InvalidNumber(String),

    #[error("invalid unicode escape sequence")]
    InvalidUnicodeEscape(usize),

    #[error("IO error: {0}")]
    Io(String),
}
