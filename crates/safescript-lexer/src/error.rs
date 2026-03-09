//! SafeScript - Lexer errors with diagnostic support

use safescript_diagnostics::{Diagnostic, DiagnosticEngine, DiagnosticLabel};
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

impl LexerError {
    pub fn to_diagnostic(&self, source: &str, source_id: &str) -> Diagnostic {
        match self {
            LexerError::UnexpectedCharacter(ch, pos) => Diagnostic::new(
                source_id.to_string(),
                *pos..*pos + 1,
                format!("unexpected character '{}'", ch),
            )
            .with_code("E001".to_string())
            .with_label(
                DiagnosticLabel::new(*pos..*pos + 1)
                    .with_message("unexpected character".to_string()),
            )
            .with_note("expected a valid token".to_string()),
            LexerError::UnterminatedString(pos) => Diagnostic::new(
                source_id.to_string(),
                *pos..source.len(),
                "unterminated string literal".to_string(),
            )
            .with_code("E002".to_string())
            .with_label(
                DiagnosticLabel::new(*pos..source.len())
                    .with_message("unterminated string".to_string()),
            )
            .with_note("string literals must be closed before end of line".to_string()),
            LexerError::UnterminatedTemplate(pos) => Diagnostic::new(
                source_id.to_string(),
                *pos..source.len(),
                "unterminated template literal".to_string(),
            )
            .with_code("E003".to_string())
            .with_label(
                DiagnosticLabel::new(*pos..source.len())
                    .with_message("unterminated template".to_string()),
            ),
            LexerError::InvalidNumber(msg) => Diagnostic::new(
                source_id.to_string(),
                0..10,
                format!("invalid number literal: {}", msg),
            )
            .with_code("E004".to_string()),
            LexerError::InvalidUnicodeEscape(pos) => Diagnostic::new(
                source_id.to_string(),
                *pos..*pos + 6,
                "invalid unicode escape sequence".to_string(),
            )
            .with_code("E005".to_string())
            .with_label(
                DiagnosticLabel::new(*pos..*pos + 6)
                    .with_message("invalid escape sequence".to_string()),
            ),
            LexerError::Io(msg) => {
                Diagnostic::new(source_id.to_string(), 0..1, format!("IO error: {}", msg))
                    .with_code("E006".to_string())
            }
        }
    }

    pub fn report(&self, source: &str, source_id: &str, source_name: &str) -> String {
        let mut engine = DiagnosticEngine::new();
        engine.add_source(safescript_diagnostics::SourceFile::new(
            source_id.to_string(),
            source_name.to_string(),
            source.to_string(),
        ));

        let diagnostic = self.to_diagnostic(source, source_id);
        engine.report(&diagnostic)
    }
}
