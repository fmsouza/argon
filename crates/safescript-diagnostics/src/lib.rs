//! SafeScript - Error reporting and diagnostics

use std::ops::Range;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: String,
    pub name: String,
    pub content: String,
}

impl SourceFile {
    pub fn new(id: String, name: String, content: String) -> Self {
        Self { id, name, content }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    errors: Vec<Diagnostic>,
    warnings: Vec<Warning>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn take_errors(self) -> Vec<Diagnostic> {
        self.errors
    }

    pub fn take_warnings(self) -> Vec<Warning> {
        self.warnings
    }

    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }
}

pub struct DiagnosticEngine {
    sources: indexmap::IndexMap<String, SourceFile>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            sources: indexmap::IndexMap::new(),
        }
    }

    pub fn add_source(&mut self, source: SourceFile) {
        self.sources.insert(source.id.clone(), source);
    }

    pub fn report(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "error[{}]: {}\n",
            diagnostic.code.as_deref().unwrap_or("E000"),
            diagnostic.message
        ));

        if let Some(source) = self.sources.get(&diagnostic.source_id) {
            output.push_str(&format!(
                "  --> {}:{}\n",
                source.name, diagnostic.span.start
            ));
        }

        for label in &diagnostic.labels {
            if let Some(msg) = &label.message {
                output.push_str(&format!("    {}\n", msg));
            }
        }

        if let Some(note) = &diagnostic.note {
            output.push_str(&format!("    = {}\n", note));
        }

        output
    }

    pub fn render(&self, bag: &DiagnosticBag) -> String {
        let mut output = String::new();

        for error in &bag.errors {
            output.push_str(&self.report(error));
            output.push('\n');
        }

        for warning in &bag.warnings {
            output.push_str(&self.report_warning(warning));
            output.push('\n');
        }

        output
    }

    fn report_warning(&self, warning: &Warning) -> String {
        let mut output = String::new();

        output.push_str(&format!("warning: {}\n", warning.message));

        if let Some(source) = self.sources.get(&warning.source_id) {
            output.push_str(&format!("  --> {}:{}\n", source.name, warning.span.start));
        }

        for label in &warning.labels {
            output.push_str(&format!("    {}\n", label.message));
        }

        output
    }
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Suggestion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticColor {
    Red,
    Yellow,
    Green,
    Blue,
    Cyan,
    Magenta,
}

#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub span: Range<usize>,
    pub message: Option<String>,
    pub color: Option<DiagnosticColor>,
}

impl DiagnosticLabel {
    pub fn new(span: Range<usize>) -> Self {
        Self {
            span,
            message: None,
            color: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_color(mut self, color: DiagnosticColor) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub source_id: String,
    pub span: Range<usize>,
    pub message: String,
    pub code: Option<String>,
    pub note: Option<String>,
    pub labels: Vec<DiagnosticLabel>,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn new(source_id: String, span: Range<usize>, message: String) -> Self {
        Self {
            source_id,
            span,
            message,
            code: None,
            note: None,
            labels: Vec::new(),
            severity: Severity::Error,
        }
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_labels(mut self, labels: Vec<DiagnosticLabel>) -> Self {
        self.labels = labels;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Warning {
    pub source_id: String,
    pub span: Range<usize>,
    pub message: String,
    pub code: Option<String>,
    pub labels: Vec<WarningLabel>,
}

impl Warning {
    pub fn new(source_id: String, span: Range<usize>, message: String) -> Self {
        Self {
            source_id,
            span,
            message,
            code: None,
            labels: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: WarningLabel) -> Self {
        self.labels.push(label);
        self
    }
}

#[derive(Debug, Clone)]
pub struct WarningLabel {
    pub span: Range<usize>,
    pub message: String,
}
