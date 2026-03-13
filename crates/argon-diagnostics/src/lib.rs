//! Argon - Error reporting and diagnostics
//!
//! Provides beautiful error messages using ariadne

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

    pub fn get_line_at(&self, pos: usize) -> Option<(usize, &str)> {
        let mut current_pos = 0;
        for (i, line) in self.content.lines().enumerate() {
            let line_end = current_pos + line.len() + 1; // +1 for newline
            if current_pos <= pos && pos < line_end {
                return Some((i + 1, line));
            }
            current_pos = line_end;
        }
        None
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
    use_colors: bool,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            sources: indexmap::IndexMap::new(),
            use_colors: atty::is(atty::Stream::Stdout),
        }
    }

    pub fn add_source(&mut self, source: SourceFile) {
        self.sources.insert(source.id.clone(), source);
    }

    fn colorize(&self, text: &str, color: &str) -> String {
        if !self.use_colors {
            return text.to_string();
        }
        match color {
            "red" => format!("\x1b[31m{}\x1b[0m", text),
            "green" => format!("\x1b[32m{}\x1b[0m", text),
            "yellow" => format!("\x1b[33m{}\x1b[0m", text),
            "blue" => format!("\x1b[34m{}\x1b[0m", text),
            "cyan" => format!("\x1b[36m{}\x1b[0m", text),
            "magenta" => format!("\x1b[35m{}\x1b[0m", text),
            "bold" => format!("\x1b[1m{}\x1b[0m", text),
            _ => text.to_string(),
        }
    }

    pub fn report(&self, diagnostic: &Diagnostic) -> String {
        let source = self.sources.get(&diagnostic.source_id);

        let severity_str = match diagnostic.severity {
            Severity::Error => self.colorize("error", "red"),
            Severity::Warning => self.colorize("warning", "yellow"),
            Severity::Hint => self.colorize("hint", "cyan"),
        };

        let code_str = diagnostic
            .code
            .as_ref()
            .map(|c| format!("[{}]", c))
            .unwrap_or_default();

        let mut output = format!("{} {}: {}\n", severity_str, code_str, diagnostic.message);

        if let Some(source) = source {
            if let Some((line_num, line_content)) = source.get_line_at(diagnostic.span.start) {
                output.push_str(&format!("  --> {}:{}\n", source.name, line_num));

                // Show the line
                output.push_str("   |\n");
                output.push_str(&format!("{:>4} | {}\n", line_num, line_content));

                // Show pointer to the span
                let col = diagnostic.span.start
                    - source.content[..diagnostic.span.start]
                        .rfind('\n')
                        .map(|p| diagnostic.span.start - p - 1)
                        .unwrap_or(diagnostic.span.start);

                let _pointer = " ".repeat(5 + col) + "^";
                let span_len = diagnostic.span.len().max(1);
                let underline = " ".repeat(5 + col) + &"~".repeat(span_len);

                output.push_str(&format!("   | {}\n", underline));

                for label in &diagnostic.labels {
                    if let Some(label_msg) = &label.message {
                        let label_col = label.span.start
                            - source.content[..label.span.start]
                                .rfind('\n')
                                .map(|p| label.span.start - p - 1)
                                .unwrap_or(label.span.start);
                        output.push_str(&format!(
                            "   | {}{}\n",
                            " ".repeat(5 + label_col),
                            self.colorize(&format!("- {}", label_msg), "cyan")
                        ));
                    }
                }
            } else {
                output.push_str(&format!(
                    "  --> {}:{}\n",
                    source.name, diagnostic.span.start
                ));
            }
        }

        if let Some(note) = &diagnostic.note {
            output.push_str(&format!("   = {}\n", note));
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
        let source = self.sources.get(&warning.source_id);

        let severity_str = self.colorize("warning", "yellow");

        let mut output = format!("{}: {}\n", severity_str, warning.message);

        if let Some(source) = source {
            if let Some((line_num, line_content)) = source.get_line_at(warning.span.start) {
                output.push_str(&format!("  --> {}:{}\n", source.name, line_num));
                output.push_str("   |\n");
                output.push_str(&format!("{:>4} | {}\n", line_num, line_content));
            }
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
    Warning,
    Hint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub message: Option<String>,
}

impl WarningLabel {
    pub fn new(span: Range<usize>) -> Self {
        Self {
            span,
            message: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}
