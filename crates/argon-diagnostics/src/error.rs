//! Argon - Diagnostic errors

use crate::{Diagnostic, Severity};

#[derive(Debug, Clone)]
pub enum ErrorCode {
    Lexer,
    Parser,
    Type,
    Borrow,
    Codegen,
    Interop,
    Io,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::Lexer => "E001",
            ErrorCode::Parser => "E002",
            ErrorCode::Type => "E003",
            ErrorCode::Borrow => "E004",
            ErrorCode::Codegen => "E005",
            ErrorCode::Interop => "E006",
            ErrorCode::Io => "E007",
        }
    }
}

pub fn lexer_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E001"))
        .with_severity(Severity::Error)
}

pub fn parser_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E002"))
        .with_severity(Severity::Error)
}

pub fn type_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E003"))
        .with_severity(Severity::Error)
}

pub fn borrow_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E004"))
        .with_severity(Severity::Error)
}

pub fn codegen_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E005"))
        .with_severity(Severity::Error)
}

pub fn interop_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E006"))
        .with_severity(Severity::Error)
}

pub fn io_error(source_id: &str, span: std::ops::Range<usize>, message: &str) -> Diagnostic {
    Diagnostic::new(source_id.to_string(), span, message.to_string())
        .with_code(format!("E007"))
        .with_severity(Severity::Error)
}

impl crate::Diagnostic {
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}
