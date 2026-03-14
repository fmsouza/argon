//! Argon - Parser

#[cfg(test)]
mod parser_tests;

use argon_ast::SourceFile;
use argon_ast::Span;
use argon_ast::Spanned;
use argon_diagnostics::{Diagnostic, DiagnosticEngine, DiagnosticLabel};
use argon_lexer::Token as LexerToken;
use argon_lexer::{tokenize, LexerError, TokenKind};

#[derive(Debug, Clone)]
pub enum ParseError {
    Lexer(LexerError),
    Parser {
        msg: String,
        span: Span,
    },
    UnexpectedToken {
        msg: String,
        span: Span,
    },
    ExpectedToken(String, usize),
    MissingParameterType {
        param_name: String,
        func_name: String,
        span: Span,
    },
    MissingReturnType {
        func_name: String,
        span: Span,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Lexer(err) => write!(f, "Lexer error: {}", err),
            ParseError::Parser { msg, .. } => write!(f, "Parse error: {}", msg),
            ParseError::UnexpectedToken { msg, .. } => write!(f, "Unexpected token: {}", msg),
            ParseError::ExpectedToken(msg, pos) => {
                write!(f, "Expected {} at position {}", msg, pos)
            }
            ParseError::MissingParameterType {
                param_name,
                func_name,
                ..
            } => {
                write!(
                    f,
                    "Parameter '{}' in function '{}' is missing type annotation. Example: {}: number",
                    param_name, func_name, param_name
                )
            }
            ParseError::MissingReturnType { func_name, .. } => {
                write!(
                    f,
                    "Function '{}' is missing return type annotation. Example: function {}(): number {{ ... }}",
                    func_name, func_name
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn to_diagnostic(&self, source: &str, source_id: &str) -> Diagnostic {
        match self {
            ParseError::Lexer(err) => err.to_diagnostic(source, source_id),
            ParseError::Parser { msg, span } => {
                Diagnostic::new(source_id.to_string(), span.clone(), msg.clone())
                    .with_code("P000".to_string())
            }
            ParseError::UnexpectedToken { msg, span } => Diagnostic::new(
                source_id.to_string(),
                span.clone(),
                format!("unexpected token: {}", msg),
            )
            .with_code("P001".to_string()),
            ParseError::ExpectedToken(msg, pos) => Diagnostic::new(
                source_id.to_string(),
                *pos..*pos + 1,
                format!("expected {}", msg),
            )
            .with_code("P002".to_string())
            .with_label(
                DiagnosticLabel::new(*pos..*pos + 1).with_message(format!("expected {}", msg)),
            ),
            ParseError::MissingParameterType {
                param_name,
                func_name,
                span,
            } => Diagnostic::new(
                source_id.to_string(),
                span.clone(),
                format!(
                    "Parameter '{}' in function '{}' is missing type annotation. Example: {}: number",
                    param_name, func_name, param_name
                ),
            )
            .with_code("P003".to_string()),
            ParseError::MissingReturnType { func_name, span } => Diagnostic::new(
                source_id.to_string(),
                span.clone(),
                format!(
                    "Function '{}' is missing return type annotation. Example: function {}(): number {{ ... }}",
                    func_name, func_name
                ),
            )
            .with_code("P004".to_string()),
        }
    }

    pub fn report(&self, source: &str, source_id: &str, source_name: &str) -> String {
        let mut engine = DiagnosticEngine::new();
        engine.add_source(argon_diagnostics::SourceFile::new(
            source_id.to_string(),
            source_name.to_string(),
            source.to_string(),
        ));

        let diagnostic = self.to_diagnostic(source, source_id);
        engine.report(&diagnostic)
    }
}

pub fn parse(source: &str) -> Result<SourceFile, ParseError> {
    let tokens = tokenize(source).map_err(ParseError::Lexer)?;
    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse()
}

pub struct Parser {
    tokens: Vec<LexerToken>,
    current: usize,
    source: String,
}

impl Parser {
    pub fn new(tokens: Vec<LexerToken>, source: String) -> Self {
        Self {
            tokens,
            current: 0,
            source,
        }
    }

    pub fn parse(&mut self) -> Result<SourceFile, ParseError> {
        let statements = self.parse_statements()?;
        let eof_span = self.tokens.last().map(|t| t.span.clone()).unwrap_or(0..0);
        Ok(SourceFile {
            path: String::new(),
            statements,
            eof_span,
        })
    }

    pub fn parse_module(&mut self) -> Result<Vec<argon_ast::Stmt>, ParseError> {
        self.parse_statements()
    }

    fn parse_statements(&mut self) -> Result<Vec<argon_ast::Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            if self.check(&TokenKind::Eof) {
                break;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        if self.match_one(&[TokenKind::At]) {
            return self.parse_decorated_statement();
        }
        if self.match_one(&[TokenKind::Declare]) {
            return self.parse_declare();
        }
        if self.match_one(&[TokenKind::Const, TokenKind::Let, TokenKind::Var]) {
            return self.parse_variable();
        }
        if self.match_one(&[TokenKind::Function]) {
            return self.parse_function();
        }
        if self.match_one(&[TokenKind::Async]) {
            return self.parse_async_function();
        }
        if self.match_one(&[TokenKind::Return]) {
            return self.parse_return();
        }
        if self.match_one(&[TokenKind::If]) {
            return self.parse_if();
        }
        if self.match_one(&[TokenKind::While]) {
            return self.parse_while();
        }
        if self.match_one(&[TokenKind::Loop]) {
            return self.parse_loop();
        }
        if self.match_one(&[TokenKind::For]) {
            return self.parse_for();
        }
        if self.match_one(&[TokenKind::Do]) {
            return self.parse_do_while();
        }
        if self.match_one(&[TokenKind::Switch]) {
            return self.parse_switch();
        }
        if self.match_one(&[TokenKind::Try]) {
            return self.parse_try();
        }
        if self.match_one(&[TokenKind::Break]) {
            return self.parse_break();
        }
        if self.match_one(&[TokenKind::Continue]) {
            return self.parse_continue();
        }
        if self.match_one(&[TokenKind::Throw]) {
            return self.parse_throw();
        }
        if self.match_one(&[TokenKind::Struct]) {
            return self.parse_struct();
        }
        if self.match_one(&[TokenKind::Skill]) {
            return self.parse_skill();
        }
        if self.check(&TokenKind::Class) {
            let span = self.peek().span.clone();
            return Err(ParseError::UnexpectedToken {
                msg: "the `class` keyword is not supported; use `struct` instead".to_string(),
                span,
            });
        }
        if self.match_one(&[TokenKind::Match]) {
            return self.parse_match();
        }
        if self.match_one(&[TokenKind::From]) {
            return self.parse_from_import();
        }
        if self.match_one(&[TokenKind::Export]) {
            return self.parse_export();
        }
        if self.match_one(&[TokenKind::Interface]) {
            return self.parse_interface();
        }
        if self.match_one(&[TokenKind::Enum]) {
            return self.parse_enum();
        }
        if self.match_one(&[TokenKind::Type]) {
            return self.parse_type_alias();
        }
        if self.match_one(&[TokenKind::OpenBrace]) {
            return self.parse_block();
        }

        let expr = self.parse_expression()?;
        let start = expr.span().start;
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;
        Ok(argon_ast::Stmt::Expr(argon_ast::ExpressionStmt {
            expr,
            span: start..end,
        }))
    }

    fn parse_decorated_statement(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let decorator = self.parse_decorator_name()?;

        match decorator.as_str() {
            "export" => {
                let declaration = self.parse_statement()?;
                let end = self.previous().span.end;
                Ok(Stmt::Export(ExportStmt {
                    declaration: Some(Box::new(declaration)),
                    specifiers: vec![],
                    source: None,
                    is_type_only: false,
                    span: start..end,
                }))
            }
            "js-interop" => {
                if self.match_one(&[TokenKind::Declare]) {
                    self.parse_declare()
                } else {
                    Err(self.parser_error_here("Expected 'declare' after '@js-interop' decorator"))
                }
            }
            "intrinsic" => {
                let stmt = self.parse_statement()?;
                match stmt {
                    Stmt::Function(mut f) => {
                        f.is_intrinsic = true;
                        Ok(Stmt::Function(f))
                    }
                    Stmt::Struct(mut s) => {
                        s.is_intrinsic = true;
                        Ok(Stmt::Struct(s))
                    }
                    Stmt::Export(mut e) => {
                        if let Some(ref mut decl) = e.declaration {
                            match **decl {
                                Stmt::Function(ref mut f) => f.is_intrinsic = true,
                                Stmt::Struct(ref mut s) => s.is_intrinsic = true,
                                _ => {}
                            }
                        }
                        Ok(Stmt::Export(e))
                    }
                    _ => Err(self.parser_error_prev(
                        "@intrinsic can only be applied to functions or structs",
                    )),
                }
            }
            other => Err(self.parser_error_prev(format!("Unsupported decorator '@{}'", other))),
        }
    }

    fn parse_decorator_name(&mut self) -> Result<String, ParseError> {
        let mut parts = Vec::new();
        parts.push(self.parse_decorator_part()?);

        while self.match_one(&[TokenKind::Minus]) {
            parts.push(self.parse_decorator_part()?);
        }

        Ok(parts.join("-"))
    }

    fn parse_decorator_part(&mut self) -> Result<String, ParseError> {
        if self.is_at_end() {
            return Err(self.parser_error_here("Expected decorator name"));
        }

        let token = self.peek();
        if token.kind == TokenKind::Identifier || token.kind.is_keyword() {
            let span = token.span.clone();
            self.advance();
            return Ok(self.source[span].to_string());
        }

        Err(self.parser_error_here("Expected decorator name"))
    }

    fn parse_declare(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        if self.match_one(&[TokenKind::Module]) {
            // `declare module "name" { ... }` declarations are currently metadata-only.
            let _module_name = self.parse_literal_string()?;
            self.expect(TokenKind::OpenBrace)?;
            let mut depth: usize = 1;
            while depth > 0 && !self.is_at_end() {
                if self.match_one(&[TokenKind::OpenBrace]) {
                    depth += 1;
                } else if self.match_one(&[TokenKind::CloseBrace]) {
                    depth = depth.saturating_sub(1);
                } else {
                    self.advance();
                }
            }

            let end = self.previous().span.end;
            return Ok(Stmt::Module(ModuleStmt {
                body: vec![],
                span: start..end,
            }));
        }

        Err(self.parser_error_here("Only 'declare module \"...\" { ... }' is supported"))
    }

    fn parse_pattern(&mut self) -> Result<argon_ast::Pattern, ParseError> {
        use argon_ast::*;

        if self.match_one(&[TokenKind::OpenBracket]) {
            let mut elements: Vec<Option<argon_ast::Pattern>> = Vec::new();

            while !self.check(&TokenKind::CloseBracket) && !self.is_at_end() {
                if self.match_one(&[TokenKind::DotDotDot]) {
                    let pat = self.parse_pattern()?;
                    elements.push(Some(pat));
                } else if !self.check(&TokenKind::Comma) {
                    elements.push(Some(self.parse_pattern()?));
                } else {
                    elements.push(None);
                }

                if !self.check(&TokenKind::CloseBracket) {
                    self.expect_comma()?;
                }
            }

            self.expect(TokenKind::CloseBracket)?;
            return Ok(Pattern::Array(Box::new(ArrayPattern {
                elements,
                rest: None,
            })));
        }

        if self.match_one(&[TokenKind::OpenBrace]) {
            let mut properties = Vec::new();

            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                if self.match_one(&[TokenKind::DotDotDot]) {
                    let start = self.previous().span.start;
                    let pat = self.parse_pattern()?;
                    properties.push(ObjectPatternProperty::Rest(RestElement {
                        argument: Box::new(pat),
                        span: self.span_since(start),
                    }));
                    if !self.check(&TokenKind::CloseBrace) {
                        self.expect_comma()?;
                    }
                    continue;
                }

                let key = self.parse_expression()?;

                let value = if self.match_one(&[TokenKind::Colon]) {
                    self.parse_pattern()?
                } else {
                    match key.clone() {
                        Expr::Identifier(id) => Pattern::Identifier(IdentPattern {
                            name: id,
                            type_annotation: None,
                            default: None,
                        }),
                        _ => return Err(self.parser_error_prev("Expected identifier in pattern")),
                    }
                };

                properties.push(ObjectPatternProperty::Property(KeyValuePattern {
                    key,
                    value,
                    computed: false,
                }));

                if !self.check(&TokenKind::CloseBrace) {
                    self.expect_comma()?;
                }
            }

            self.expect(TokenKind::CloseBrace)?;
            return Ok(Pattern::Object(Box::new(ObjectPattern {
                properties,
                rest: None,
            })));
        }

        let name = self.expect_identifier()?;
        Ok(Pattern::Identifier(IdentPattern {
            name,
            type_annotation: None,
            default: None,
        }))
    }

    fn parse_variable(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let stmt_start = self.previous().span.start;
        let kind = match self.previous().kind {
            TokenKind::Const => VariableKind::Const,
            TokenKind::Var => VariableKind::Var,
            _ => VariableKind::Let,
        };

        let decl_start = self.peek().span.start;
        let mut id = self.parse_pattern()?;

        if let Pattern::Identifier(ref mut ident) = id {
            if self.match_one(&[TokenKind::Colon]) {
                ident.type_annotation = Some(Box::new(self.parse_type()?));
            }
        }

        let init = if self.match_one(&[TokenKind::Equal]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let decl_end = self.previous().span.end;
        self.match_one(&[TokenKind::Semi]);
        let stmt_end = self.previous().span.end;

        Ok(Stmt::Variable(VariableStmt {
            kind,
            declarations: vec![VariableDeclarator {
                id,
                init,
                span: decl_start..decl_end,
            }],
            span: stmt_start..stmt_end,
        }))
    }

    fn validate_function_types(
        &self,
        params: &[argon_ast::Param],
        return_type: &Option<Box<argon_ast::Type>>,
        func_name: &str,
        func_span: Span,
        is_constructor: bool,
    ) -> Result<(), ParseError> {
        for param in params {
            if let argon_ast::Pattern::Identifier(id) = &param.pat {
                if id.type_annotation.is_none() {
                    return Err(ParseError::MissingParameterType {
                        param_name: id.name.sym.clone(),
                        func_name: func_name.to_string(),
                        span: id.name.span.clone(),
                    });
                }
            }
        }

        if !is_constructor && return_type.is_none() {
            return Err(ParseError::MissingReturnType {
                func_name: func_name.to_string(),
                span: func_span,
            });
        }

        Ok(())
    }

    fn parse_function(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let stmt_start = self.previous().span.start;
        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        self.expect(TokenKind::OpenParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let param_start = name.span.start;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            let param_end = self.previous().span.end;

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty.clone(),
                    default: None,
                }),
                ty,
                default: None,
                is_optional: false,
                span: param_start..param_end,
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;

        let return_type = if self.match_one(&[TokenKind::Colon]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let func_name = id.sym.clone();
        self.validate_function_types(&params, &return_type, &func_name, id.span.clone(), false)?;

        let borrow_annotation = self.parse_borrow_annotation()?;

        // Allow bodyless functions terminated by `;` (used by @intrinsic declarations)
        if self.match_one(&[TokenKind::Semi]) {
            let stmt_end = self.previous().span.end;
            let body = FunctionBody {
                statements: vec![],
                span: stmt_end..stmt_end,
            };
            return Ok(Stmt::Function(FunctionDecl {
                id: Some(id),
                params,
                body,
                type_params,
                return_type,
                is_async: false,
                is_intrinsic: false,
                borrow_annotation,
                span: stmt_start..stmt_end,
            }));
        }

        self.expect(TokenKind::OpenBrace)?;
        let body_start = self.previous().span.start;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect(TokenKind::CloseBrace)?;
        let stmt_end = self.previous().span.end;

        let body = FunctionBody {
            statements,
            span: body_start..stmt_end,
        };

        Ok(Stmt::Function(FunctionDecl {
            id: Some(id),
            params,
            body,
            type_params,
            return_type,
            is_async: false,
            is_intrinsic: false,
            borrow_annotation,
            span: stmt_start..stmt_end,
        }))
    }

    fn parse_async_function(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let stmt_start = self.previous().span.start;
        // Expect 'function' keyword after 'async'
        self.expect(TokenKind::Function)?;

        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        self.expect(TokenKind::OpenParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let param_start = name.span.start;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            let param_end = self.previous().span.end;

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty.clone(),
                    default: None,
                }),
                ty,
                default: None,
                is_optional: false,
                span: param_start..param_end,
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;

        let return_type = if self.match_one(&[TokenKind::Colon]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let func_name = id.sym.clone();
        self.validate_function_types(&params, &return_type, &func_name, id.span.clone(), false)?;

        let borrow_annotation = self.parse_borrow_annotation()?;

        self.expect(TokenKind::OpenBrace)?;
        let body_start = self.previous().span.start;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect(TokenKind::CloseBrace)?;
        let stmt_end = self.previous().span.end;

        let body = FunctionBody {
            statements,
            span: body_start..stmt_end,
        };

        Ok(Stmt::AsyncFunction(FunctionDecl {
            id: Some(id),
            params,
            body,
            type_params,
            return_type,
            is_async: true,
            is_intrinsic: false,
            borrow_annotation,
            span: stmt_start..stmt_end,
        }))
    }

    fn parse_function_expression(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let span_start = self.previous().span.start;

        self.expect(TokenKind::OpenParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let param_start = name.span.start;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            let param_end = self.previous().span.end;

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty.clone(),
                    default: None,
                }),
                ty,
                default: None,
                is_optional: false,
                span: param_start..param_end,
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;

        let return_type = if self.match_one(&[TokenKind::Colon]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        self.validate_function_types(
            &params,
            &return_type,
            "anonymous",
            self.previous().span.clone(),
            false,
        )?;

        let body = if self.match_one(&[TokenKind::OpenBrace]) {
            self.current -= 1;
            match self.parse_statement()? {
                Stmt::Block(b) => ArrowFunctionBody::Block(b),
                _ => return Err(self.parser_error_prev("Function expression body must be a block")),
            }
        } else if self.match_one(&[TokenKind::FatArrow]) {
            let expr = self.parse_expression()?;
            let expr_span = expr.span().clone();
            let block = BlockStmt {
                statements: vec![Stmt::Return(ReturnStmt {
                    argument: Some(expr),
                    span: expr_span.clone(),
                })],
                span: expr_span,
            };
            ArrowFunctionBody::Block(block)
        } else {
            return Err(self.parser_error_here("Expected function body"));
        };

        let span = span_start..self.previous().span.end;

        Ok(Expr::ArrowFunction(Box::new(ArrowFunctionExpr {
            params,
            body,
            type_params: vec![],
            return_type,
            span,
        })))
    }

    fn parse_borrow_annotation(
        &mut self,
    ) -> Result<Option<argon_ast::BorrowAnnotation>, ParseError> {
        if !self.match_one(&[TokenKind::With]) {
            return Ok(None);
        }

        let span_start = self.previous().span.start;

        // Handle `with &this` / `with &mut this`
        if !self.match_one(&[TokenKind::Ampersand]) {
            return Err(self.parser_error_here("Expected '&' after 'with'"));
        }

        let kind = if self.match_one(&[TokenKind::Mut]) {
            argon_ast::BorrowKind::Mutable
        } else {
            argon_ast::BorrowKind::Shared
        };

        let target = if self.match_one(&[TokenKind::This]) {
            Some(argon_ast::Ident {
                sym: "this".to_string(),
                span: self.previous().span.clone(),
            })
        } else if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let span = span_start..self.previous().span.end;

        Ok(Some(argon_ast::BorrowAnnotation { kind, target, span }))
    }

    fn parse_return(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let argument = if self.check(&TokenKind::Semi) || self.check(&TokenKind::OpenBrace) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::Return(ReturnStmt {
            argument,
            span: start..end,
        }))
    }

    fn parse_if(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let consequent = Box::new(self.parse_statement()?);
        let alternate = if self.match_one(&[TokenKind::Else]) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        let end = self.previous().span.end;
        Ok(Stmt::If(IfStmt {
            condition,
            consequent,
            alternate,
            span: start..end,
        }))
    }

    fn parse_block(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let span_start = self.previous().span.start;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Block(BlockStmt {
            statements,
            span: span_start..self.previous().span.end,
        }))
    }

    fn parse_while(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let body = Box::new(self.parse_statement()?);
        let end = self.previous().span.end;

        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: start..end,
        }))
    }

    fn parse_loop(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let body = Box::new(self.parse_statement()?);
        let end = self.previous().span.end;

        Ok(Stmt::Loop(LoopStmt {
            body,
            span: start..end,
        }))
    }

    fn parse_for(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        self.expect(TokenKind::OpenParen)?;
        if self.match_one(&[TokenKind::Const, TokenKind::Let, TokenKind::Var]) {
            let kind = match self.previous().kind {
                TokenKind::Const => VariableKind::Const,
                TokenKind::Var => VariableKind::Var,
                _ => VariableKind::Let,
            };

            let decl_start = self.peek().span.start;
            let id = self.parse_pattern()?;
            let init = if self.match_one(&[TokenKind::Equal]) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let decl_end = self.previous().span.end;

            let decl = VariableDeclarator {
                id: id.clone(),
                init: init.clone(),
                span: decl_start..decl_end,
            };

            if self.match_one(&[TokenKind::Of]) {
                let right = self.parse_expression()?;
                self.expect(TokenKind::CloseParen)?;
                let body = Box::new(self.parse_statement()?);
                let end = self.previous().span.end;
                return Ok(Stmt::ForIn(ForInStmt {
                    left: ForInLeft::Variable(decl),
                    right,
                    body,
                    span: start..end,
                }));
            }

            self.expect(TokenKind::Semi)?;
            let test = if !self.check(&TokenKind::Semi) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::Semi)?;

            let update = if !self.check(&TokenKind::CloseParen) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::CloseParen)?;

            let body = Box::new(self.parse_statement()?);
            let end = self.previous().span.end;

            return Ok(Stmt::For(ForStmt {
                init: Some(ForInit::Variable(VariableStmt {
                    kind,
                    declarations: vec![decl],
                    span: decl_start..decl_end,
                })),
                test,
                update,
                body,
                span: start..end,
            }));
        }

        if self.match_one(&[TokenKind::Semi]) {
            let test = if !self.check(&TokenKind::Semi) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::Semi)?;
            let update = if !self.check(&TokenKind::CloseParen) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::CloseParen)?;
            let body = Box::new(self.parse_statement()?);
            let end = self.previous().span.end;
            return Ok(Stmt::For(ForStmt {
                init: None,
                test,
                update,
                body,
                span: start..end,
            }));
        }

        let expr = self.parse_expression()?;
        if self.match_one(&[TokenKind::Of]) {
            let left = match expr {
                Expr::Identifier(id) => ForInLeft::Pattern(Pattern::Identifier(IdentPattern {
                    name: id,
                    type_annotation: None,
                    default: None,
                })),
                _ => return Err(self.parser_error_prev("Invalid for..of left-hand side")),
            };
            let right = self.parse_expression()?;
            self.expect(TokenKind::CloseParen)?;
            let body = Box::new(self.parse_statement()?);
            let end = self.previous().span.end;
            return Ok(Stmt::ForIn(ForInStmt {
                left,
                right,
                body,
                span: start..end,
            }));
        }

        self.expect(TokenKind::Semi)?;
        let test = if !self.check(&TokenKind::Semi) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Semi)?;
        let update = if !self.check(&TokenKind::CloseParen) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::CloseParen)?;
        let body = Box::new(self.parse_statement()?);
        let end = self.previous().span.end;

        Ok(Stmt::For(ForStmt {
            init: Some(ForInit::Expr(expr)),
            test,
            update,
            body,
            span: start..end,
        }))
    }

    fn parse_do_while(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let body = Box::new(self.parse_statement()?);

        self.expect(TokenKind::While)?;
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::DoWhile(DoWhileStmt {
            body,
            condition,
            span: start..end,
        }))
    }

    fn parse_switch(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        self.expect(TokenKind::OpenParen)?;
        let discriminant = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Case]) {
                let case_start = self.previous().span.start;
                let test = Some(self.parse_expression()?);
                self.expect(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::CloseBrace)
                {
                    consequent.push(self.parse_statement()?);
                }

                let case_end = self.previous().span.end;
                cases.push(SwitchCase {
                    test,
                    consequent,
                    span: case_start..case_end,
                });
            } else if self.match_one(&[TokenKind::Default]) {
                let case_start = self.previous().span.start;
                self.expect(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case) && !self.check(&TokenKind::CloseBrace) {
                    consequent.push(self.parse_statement()?);
                }

                let case_end = self.previous().span.end;
                cases.push(SwitchCase {
                    test: None,
                    consequent,
                    span: case_start..case_end,
                });
            } else {
                self.advance();
            }
        }

        self.expect(TokenKind::CloseBrace)?;
        let end = self.previous().span.end;

        Ok(Stmt::Switch(SwitchStmt {
            discriminant,
            cases,
            span: start..end,
        }))
    }

    fn parse_try(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let block = match self.parse_statement()? {
            Stmt::Block(b) => b,
            _ => return Err(self.parser_error_prev("try block must be a block")),
        };

        let handler = if self.match_one(&[TokenKind::Catch]) {
            let catch_start = self.previous().span.start;
            self.expect(TokenKind::OpenParen)?;
            let param = if self.check(&TokenKind::Identifier) {
                Some(self.parse_pattern()?)
            } else {
                None
            };
            self.expect(TokenKind::CloseParen)?;

            let body = match self.parse_statement()? {
                Stmt::Block(b) => b,
                _ => return Err(self.parser_error_prev("catch block must be a block")),
            };

            let catch_end = self.previous().span.end;
            Some(CatchClause {
                param,
                body,
                span: catch_start..catch_end,
            })
        } else {
            None
        };

        let finalizer = if self.match_one(&[TokenKind::Finally]) {
            match self.parse_statement()? {
                Stmt::Block(b) => Some(b),
                _ => return Err(self.parser_error_prev("finally block must be a block")),
            }
        } else {
            None
        };

        let end = self.previous().span.end;
        Ok(Stmt::Try(TryStmt {
            block,
            handler,
            finalizer,
            span: start..end,
        }))
    }

    fn parse_break(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let label = if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::Break(BreakStmt {
            label,
            span: start..end,
        }))
    }

    fn parse_continue(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let label = if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::Continue(ContinueStmt {
            label,
            span: start..end,
        }))
    }

    fn parse_throw(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let argument = self.parse_expression()?;
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::Throw(ThrowStmt {
            argument,
            span: start..end,
        }))
    }

    fn parse_struct(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        // Parse optional `embodies` clause (skills)
        let embodies = if self.match_one(&[TokenKind::Embodies]) {
            let mut skills = vec![self.expect_identifier()?];
            while self.match_one(&[TokenKind::Comma]) {
                if self.check(&TokenKind::OpenBrace) {
                    break;
                }
                skills.push(self.expect_identifier()?);
            }
            skills
        } else {
            vec![]
        };

        // Parse optional `implements` clause
        let implements = if self.match_one(&[TokenKind::Implements]) {
            let mut types = vec![self.parse_type()?];
            while self.match_one(&[TokenKind::Comma]) {
                types.push(self.parse_type()?);
            }
            types
        } else {
            vec![]
        };

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructor = None;
        self.expect(TokenKind::OpenBrace)?;

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Semi, TokenKind::Comma]) {
                continue;
            }

            // Check for constructor
            if self.match_one(&[TokenKind::Constructor]) {
                constructor = Some(self.parse_constructor()?);
                self.match_one(&[TokenKind::Semi]);
                continue;
            }

            let member_id = self.expect_identifier()?;
            let member_start = member_id.span.start;
            let key = Expr::Identifier(member_id.clone());

            if self.check(&TokenKind::OpenParen) || self.check(&TokenKind::LessThan) {
                let method = self.parse_method(key)?;
                methods.push(method);
                self.match_one(&[TokenKind::Semi]);
                continue;
            }

            self.expect(TokenKind::Colon)?;
            let type_annotation = Box::new(self.parse_type()?);
            let field_end = self.previous().span.end;

            fields.push(StructField {
                id: member_id,
                type_annotation,
                is_readonly: false,
                span: member_start..field_end,
            });

            self.match_one(&[TokenKind::Semi, TokenKind::Comma]);
        }

        self.expect(TokenKind::CloseBrace)?;
        let end = self.previous().span.end;

        Ok(Stmt::Struct(StructDecl {
            id,
            type_params,
            fields,
            methods,
            constructor,
            implements,
            embodies,
            is_intrinsic: false,
            span: start..end,
        }))
    }

    fn parse_skill(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        let mut items = Vec::new();
        self.expect(TokenKind::OpenBrace)?;

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Semi, TokenKind::Comma]) {
                continue;
            }

            let member_id = self.expect_identifier()?;
            let key = Expr::Identifier(member_id.clone());

            if self.check(&TokenKind::OpenParen) || self.check(&TokenKind::LessThan) {
                // Method — check if it has a body (concrete) or just a signature (abstract)
                let type_params = self.parse_type_params_opt()?;

                let mut params = Vec::new();
                self.expect(TokenKind::OpenParen)?;

                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    let name = self.expect_identifier()?;
                    let param_start = name.span.start;
                    let ty = if self.match_one(&[TokenKind::Colon]) {
                        Some(Box::new(self.parse_type()?))
                    } else {
                        None
                    };
                    let param_end = self.previous().span.end;

                    params.push(Param {
                        pat: Pattern::Identifier(IdentPattern {
                            name: name.clone(),
                            type_annotation: ty.clone(),
                            default: None,
                        }),
                        ty,
                        default: None,
                        is_optional: false,
                        span: param_start..param_end,
                    });

                    if !self.check(&TokenKind::CloseParen) {
                        self.expect_comma()?;
                    }
                }

                self.expect(TokenKind::CloseParen)?;

                let return_type = if self.match_one(&[TokenKind::Colon]) {
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };

                // Check for borrow annotation before body
                let borrow_annotation = self.parse_borrow_annotation()?;

                if self.check(&TokenKind::OpenBrace) {
                    // Concrete method (has body)
                    let body_stmt = match self.parse_statement()? {
                        Stmt::Block(b) => b,
                        _ => return Err(self.parser_error_prev("method body must be a block")),
                    };

                    let body = FunctionBody {
                        statements: body_stmt.statements,
                        span: body_stmt.span,
                    };

                    let end = self.previous().span.end;
                    items.push(SkillItem::ConcreteMethod(MethodDefinition {
                        key,
                        value: FunctionDecl {
                            id: None,
                            params,
                            body,
                            type_params,
                            return_type,
                            is_async: false,
                            is_intrinsic: false,
                            borrow_annotation,
                            span: start..end,
                        },
                        kind: MethodKind::Method,
                        is_static: false,
                        span: start..end,
                    }));
                } else {
                    // Abstract method (signature only)
                    let end = self.previous().span.end;
                    self.match_one(&[TokenKind::Semi]);
                    items.push(SkillItem::AbstractMethod(MethodSignature {
                        id: member_id,
                        params,
                        return_type,
                        type_params,
                        span: start..end,
                    }));
                }
            } else {
                // Required field: `name: type;`
                self.expect(TokenKind::Colon)?;
                let type_annotation = Some(Box::new(self.parse_type()?));
                let field_end = self.previous().span.end;
                self.match_one(&[TokenKind::Semi, TokenKind::Comma]);

                items.push(SkillItem::RequiredField(PropertySignature {
                    id: member_id,
                    type_annotation,
                    is_optional: false,
                    is_readonly: false,
                    span: start..field_end,
                }));
            }
        }

        self.expect(TokenKind::CloseBrace)?;
        let end = self.previous().span.end;

        Ok(Stmt::Skill(SkillDecl {
            id,
            type_params,
            items,
            span: start..end,
        }))
    }

    fn parse_interface(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        let mut extends = Vec::new();
        if self.match_one(&[TokenKind::Extends]) {
            loop {
                extends.push(self.parse_type()?);
                if !self.match_one(&[TokenKind::Comma]) {
                    break;
                }
            }
        }

        self.expect(TokenKind::OpenBrace)?;
        let body_start = self.previous().span.start;

        let mut members = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Semi, TokenKind::Comma]) {
                continue;
            }

            if self.match_one(&[TokenKind::OpenBracket]) {
                let member_start = self.previous().span.start;
                // Index signature: `[key: string]: number;`
                let param_name = self.expect_identifier()?;
                let param_start = param_name.span.start;
                self.expect(TokenKind::Colon)?;
                let param_ty = Box::new(self.parse_type()?);
                let param_end = self.previous().span.end;
                self.expect(TokenKind::CloseBracket)?;
                self.expect(TokenKind::Colon)?;
                let return_type = Box::new(self.parse_type()?);
                self.match_one(&[TokenKind::Semi]);
                let member_end = self.previous().span.end;

                members.push(InterfaceMember::IndexSignature(IndexSignature {
                    params: vec![Param {
                        pat: Pattern::Identifier(IdentPattern {
                            name: param_name.clone(),
                            type_annotation: Some(param_ty.clone()),
                            default: None,
                        }),
                        ty: Some(param_ty),
                        default: None,
                        is_optional: false,
                        span: param_start..param_end,
                    }],
                    return_type,
                    span: member_start..member_end,
                }));
                continue;
            }

            let member_start = self.peek().span.start;
            let is_readonly = self.match_one(&[TokenKind::Readonly]);
            let member_id = self.expect_identifier()?;

            // Method signature: `name<T>(...): R;`
            if self.check(&TokenKind::LessThan) || self.check(&TokenKind::OpenParen) {
                let method_type_params = self.parse_type_params_opt()?;
                self.expect(TokenKind::OpenParen)?;

                let mut params = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    let name = self.expect_identifier()?;
                    let param_start = name.span.start;
                    let is_optional = self.match_one(&[TokenKind::Question]);
                    let ty = if self.match_one(&[TokenKind::Colon]) {
                        Some(Box::new(self.parse_type()?))
                    } else {
                        None
                    };
                    let param_end = self.previous().span.end;

                    params.push(Param {
                        pat: Pattern::Identifier(IdentPattern {
                            name: name.clone(),
                            type_annotation: ty.clone(),
                            default: None,
                        }),
                        ty,
                        default: None,
                        is_optional,
                        span: param_start..param_end,
                    });

                    if !self.check(&TokenKind::CloseParen) {
                        self.expect_comma()?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;

                let return_type = if self.match_one(&[TokenKind::Colon]) {
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };
                self.match_one(&[TokenKind::Semi]);
                let member_end = self.previous().span.end;

                members.push(InterfaceMember::Method(MethodSignature {
                    id: member_id,
                    params,
                    return_type,
                    type_params: method_type_params,
                    span: member_start..member_end,
                }));
                continue;
            }

            // Property signature: `name?: T;`
            let is_optional = self.match_one(&[TokenKind::Question]);
            let type_annotation = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            self.match_one(&[TokenKind::Semi]);
            let member_end = self.previous().span.end;

            members.push(InterfaceMember::Property(PropertySignature {
                id: member_id,
                type_annotation,
                is_optional,
                is_readonly,
                span: member_start..member_end,
            }));
        }

        self.expect(TokenKind::CloseBrace)?;
        let span = body_start..self.previous().span.end;

        Ok(Stmt::Interface(InterfaceDecl {
            id,
            type_params,
            extends,
            body: InterfaceBody {
                body: members,
                span: span.clone(),
            },
            span: start..span.end,
        }))
    }

    fn parse_enum(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let id = self.expect_identifier()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut members = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Semi, TokenKind::Comma]) {
                continue;
            }
            let member_id = self.expect_identifier()?;
            let member_start = member_id.span.start;
            let init = if self.match_one(&[TokenKind::Equal]) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let member_end = self.previous().span.end;
            members.push(EnumMember {
                id: member_id,
                init,
                span: member_start..member_end,
            });
            self.match_one(&[TokenKind::Comma, TokenKind::Semi]);
        }

        self.expect(TokenKind::CloseBrace)?;
        let end = self.previous().span.end;

        Ok(Stmt::Enum(EnumDecl {
            id,
            members,
            is_const: false,
            span: start..end,
        }))
    }

    fn parse_type_alias(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let id = self.expect_identifier()?;
        let type_params = self.parse_type_params_opt()?;

        self.expect(TokenKind::Equal)?;
        let type_annotation = Box::new(self.parse_type()?);
        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::TypeAlias(TypeAliasStmt {
            id,
            type_params,
            type_annotation,
            span: start..end,
        }))
    }

    fn parse_constructor(&mut self) -> Result<argon_ast::Constructor, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let mut params = Vec::new();
        self.expect(TokenKind::OpenParen)?;

        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let param_start = name.span.start;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            let param_end = self.previous().span.end;

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty.clone(),
                    default: None,
                }),
                ty,
                default: None,
                is_optional: false,
                span: param_start..param_end,
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;

        self.validate_function_types(
            &params,
            &None,
            "constructor",
            self.previous().span.clone(),
            true,
        )?;

        let body = match self.parse_statement()? {
            Stmt::Block(b) => b,
            _ => return Err(self.parser_error_prev("constructor body must be a block")),
        };

        let end = self.previous().span.end;
        Ok(Constructor {
            params,
            body,
            span: start..end,
        })
    }

    fn parse_method(
        &mut self,
        key: argon_ast::Expr,
    ) -> Result<argon_ast::MethodDefinition, ParseError> {
        use argon_ast::*;

        let start = key.span().start;
        let type_params = self.parse_type_params_opt()?;

        let mut params = Vec::new();
        self.expect(TokenKind::OpenParen)?;

        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let param_start = name.span.start;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            let param_end = self.previous().span.end;

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty.clone(),
                    default: None,
                }),
                ty,
                default: None,
                is_optional: false,
                span: param_start..param_end,
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;

        let return_type = if self.match_one(&[TokenKind::Colon]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let method_name = match &key {
            argon_ast::Expr::Identifier(id) => id.sym.clone(),
            _ => "method".to_string(),
        };
        self.validate_function_types(
            &params,
            &return_type,
            &method_name,
            key.span().clone(),
            false,
        )?;

        // Parse borrow annotation (e.g., "with &mut this")
        let borrow_annotation = self.parse_borrow_annotation()?;

        // Allow bodyless methods terminated by `;` (used by @intrinsic struct declarations)
        let (body, end) = if self.match_one(&[TokenKind::Semi]) {
            let end = self.previous().span.end;
            (
                argon_ast::FunctionBody {
                    statements: vec![],
                    span: end..end,
                },
                end,
            )
        } else {
            let body_stmt = match self.parse_statement()? {
                Stmt::Block(b) => b,
                _ => return Err(self.parser_error_prev("method body must be a block")),
            };
            let end = self.previous().span.end;
            (
                argon_ast::FunctionBody {
                    statements: body_stmt.statements,
                    span: body_stmt.span,
                },
                end,
            )
        };

        Ok(MethodDefinition {
            key,
            value: FunctionDecl {
                id: None,
                params,
                body,
                type_params,
                return_type,
                is_async: false,
                is_intrinsic: false,
                borrow_annotation,
                span: start..end,
            },
            kind: MethodKind::Method,
            is_static: false,
            span: start..end,
        })
    }

    fn parse_match(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        self.expect(TokenKind::OpenParen)?;
        let discriminant = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let pattern = self.parse_expression()?;
            let case_start = pattern.span().start;
            self.expect(TokenKind::FatArrow)?;
            let consequent = Box::new(self.parse_statement()?);
            let case_end = self.previous().span.end;

            cases.push(MatchCase {
                pattern,
                consequent,
                guard: None,
                span: case_start..case_end,
            });

            self.match_one(&[TokenKind::Comma]);
        }

        self.expect(TokenKind::CloseBrace)?;
        let end = self.previous().span.end;

        Ok(Stmt::Match(MatchStmt {
            discriminant,
            cases,
            span: start..end,
        }))
    }

    /// Parse argon-style import: `from "path" import { a, b }` or `from "path" import Name`
    fn parse_from_import(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;
        let source = self.parse_literal_string()?;
        self.expect(TokenKind::Import)?;

        // Side-effect import: `from "path" import;`
        if self.check(&TokenKind::Semi) || self.is_at_end() {
            self.match_one(&[TokenKind::Semi]);
            let end = self.previous().span.end;
            return Ok(Stmt::Import(ImportStmt {
                specifiers: vec![],
                source,
                is_type_only: false,
                span: start..end,
            }));
        }

        let specifiers = if self.match_one(&[TokenKind::OpenBrace]) {
            // Named imports: `from "path" import { a, b as c }`
            let mut specs = Vec::new();
            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                let imported = self.expect_identifier()?;
                let spec_start = imported.span.start;
                let local: Option<Ident> = if self.match_one(&[TokenKind::As]) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                specs.push(ImportSpecifier::Named(NamedImportSpecifier {
                    imported,
                    local,
                    is_type: false,
                    span: spec_start..self.previous().span.end,
                }));
                if !self.check(&TokenKind::CloseBrace) {
                    self.expect_comma()?;
                }
            }
            self.expect(TokenKind::CloseBrace)?;
            specs
        } else if self.check(&TokenKind::Identifier) {
            // Namespace import: `from "path" import Name`
            let id = self.expect_identifier()?;
            vec![ImportSpecifier::Namespace(NamespaceImportSpecifier {
                span: id.span.clone(),
                id,
            })]
        } else {
            return Err(self.parser_error_here("Expected import specifier or '{'"));
        };

        self.match_one(&[TokenKind::Semi]);
        let end = self.previous().span.end;

        Ok(Stmt::Import(ImportStmt {
            specifiers,
            source,
            is_type_only: false,
            span: start..end,
        }))
    }

    fn parse_export(&mut self) -> Result<argon_ast::Stmt, ParseError> {
        use argon_ast::*;

        let start = self.previous().span.start;

        if self.check(&TokenKind::OpenBrace) {
            self.expect(TokenKind::OpenBrace)?;
            let mut specs = Vec::new();
            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                let orig = self.expect_identifier()?;
                let spec_start = orig.span.start;
                let exported = if self.match_one(&[TokenKind::As]) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                specs.push(ExportSpecifier {
                    orig,
                    exported,
                    span: spec_start..self.previous().span.end,
                });
                if !self.check(&TokenKind::CloseBrace) {
                    self.expect_comma()?;
                }
            }
            self.expect(TokenKind::CloseBrace)?;

            // Re-export: `export { x, y } from "./module";`
            let source = if self.match_one(&[TokenKind::From]) {
                Some(self.parse_literal_string()?)
            } else {
                None
            };

            self.match_one(&[TokenKind::Semi]);
            let end = self.previous().span.end;
            return Ok(Stmt::Export(ExportStmt {
                declaration: None,
                specifiers: specs,
                source,
                is_type_only: false,
                span: start..end,
            }));
        }

        let declaration = self.parse_statement()?;
        let end = self.previous().span.end;
        Ok(Stmt::Export(ExportStmt {
            declaration: Some(Box::new(declaration)),
            specifiers: vec![],
            source: None,
            is_type_only: false,
            span: start..end,
        }))
    }

    fn parse_literal_string(&mut self) -> Result<argon_ast::StringLiteral, ParseError> {
        if self.match_one(&[TokenKind::StringLiteral]) {
            let span = self.previous().span.clone();
            let value = self.source[span.clone()].to_string();
            Ok(argon_ast::StringLiteral { value, span })
        } else {
            Err(self.parser_error_here("Expected string"))
        }
    }

    fn parse_type_params_opt(&mut self) -> Result<Vec<argon_ast::TypeParam>, ParseError> {
        use argon_ast::TypeParam;

        if !self.match_one(&[TokenKind::LessThan]) {
            return Ok(Vec::new());
        }

        let start = self.previous().span.start;
        let mut params = Vec::new();

        while !self.check(&TokenKind::GreaterThan) && !self.is_at_end() {
            let name = self.expect_identifier()?;

            let constraint = if self.match_one(&[TokenKind::Extends]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            let default = if self.match_one(&[TokenKind::Equal]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            let span = start..self.previous().span.end;
            params.push(TypeParam {
                name,
                constraint,
                default,
                span,
            });

            if !self.check(&TokenKind::GreaterThan) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::GreaterThan)?;
        Ok(params)
    }

    fn expect_identifier(&mut self) -> Result<argon_ast::Ident, ParseError> {
        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let sym = self.source[span.clone()].to_string();
            return Ok(argon_ast::Ident { sym, span });
        }
        Err(self.parser_error_here("Expected identifier"))
    }

    fn parse_expression(&mut self) -> Result<argon_ast::Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let expr = self.parse_conditional()?;

        if self.match_one(&[TokenKind::Equal]) {
            let right = Box::new(self.parse_assignment()?);
            let span = expr.span().start..right.span().end;
            return Ok(Expr::Assignment(Box::new(AssignmentExpr {
                left: Box::new(AssignmentTarget::Simple(Box::new(expr))),
                operator: AssignmentOperator::Assign,
                right,
                span,
            })));
        }

        if self.match_one(&[
            TokenKind::PlusEqual,
            TokenKind::MinusEqual,
            TokenKind::StarEqual,
            TokenKind::SlashEqual,
            TokenKind::PercentEqual,
            TokenKind::LessThanLessThanEqual,
            TokenKind::GreaterThanGreaterThanEqual,
            TokenKind::GreaterThanGreaterThanGreaterThanEqual,
            TokenKind::AmpersandEqual,
            TokenKind::PipeEqual,
            TokenKind::CaretEqual,
            TokenKind::QuestionQuestionEqual,
        ]) {
            let operator = match self.previous().kind {
                TokenKind::PlusEqual => AssignmentOperator::PlusAssign,
                TokenKind::MinusEqual => AssignmentOperator::MinusAssign,
                TokenKind::StarEqual => AssignmentOperator::MultiplyAssign,
                TokenKind::SlashEqual => AssignmentOperator::DivideAssign,
                TokenKind::PercentEqual => AssignmentOperator::ModuloAssign,
                TokenKind::LessThanLessThanEqual => AssignmentOperator::LeftShiftAssign,
                TokenKind::GreaterThanGreaterThanEqual => AssignmentOperator::RightShiftAssign,
                TokenKind::GreaterThanGreaterThanGreaterThanEqual => {
                    AssignmentOperator::UnsignedRightShiftAssign
                }
                TokenKind::AmpersandEqual => AssignmentOperator::BitwiseAndAssign,
                TokenKind::PipeEqual => AssignmentOperator::BitwiseOrAssign,
                TokenKind::CaretEqual => AssignmentOperator::BitwiseXorAssign,
                TokenKind::QuestionQuestionEqual => AssignmentOperator::NullishCoalescingAssign,
                _ => AssignmentOperator::Assign,
            };
            let right = Box::new(self.parse_assignment()?);
            let span = expr.span().start..right.span().end;
            return Ok(Expr::Assignment(Box::new(AssignmentExpr {
                left: Box::new(AssignmentTarget::Simple(Box::new(expr))),
                operator,
                right,
                span,
            })));
        }

        Ok(expr)
    }

    fn parse_conditional(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let expr = self.parse_or()?;

        if self.match_one(&[TokenKind::Question]) {
            let consequent = Box::new(self.parse_assignment()?);
            self.expect_token(TokenKind::Colon, "ternary expression")?;
            let alternate = Box::new(self.parse_assignment()?);
            let span = expr.span().start..alternate.span().end;
            return Ok(Expr::Conditional(ConditionalExpr {
                test: Box::new(expr),
                consequent,
                alternate,
                span,
            }));
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_and()?;

        while self.match_one(&[TokenKind::PipePipe]) {
            let right = Box::new(self.parse_and()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator: LogicalOperator::Or,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_equality()?;

        while self.match_one(&[TokenKind::AmpersandAmpersand]) {
            let right = Box::new(self.parse_equality()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator: LogicalOperator::And,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_bitwise_and()?;

        while self.match_one(&[
            TokenKind::BangEqual,
            TokenKind::EqualEqual,
            TokenKind::BangEqualEqual,
            TokenKind::EqualEqualEqual,
        ]) {
            let operator = match self.previous().kind {
                TokenKind::BangEqual => BinaryOperator::NotEqual,
                TokenKind::EqualEqual => BinaryOperator::Equal,
                TokenKind::BangEqualEqual => BinaryOperator::StrictNotEqual,
                TokenKind::EqualEqualEqual => BinaryOperator::StrictEqual,
                _ => BinaryOperator::Equal,
            };
            let right = Box::new(self.parse_bitwise_and()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_bitwise_xor()?;

        while self.match_one(&[TokenKind::Ampersand]) {
            let right = Box::new(self.parse_bitwise_xor()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator: BinaryOperator::BitwiseAnd,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_bitwise_xor(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_bitwise_or()?;

        while self.match_one(&[TokenKind::Caret]) {
            let right = Box::new(self.parse_bitwise_or()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator: BinaryOperator::BitwiseXor,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_comparison()?;

        while self.match_one(&[TokenKind::Pipe]) {
            let right = Box::new(self.parse_comparison()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator: BinaryOperator::BitwiseOr,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_term()?;

        while self.match_one(&[
            TokenKind::LessThan,
            TokenKind::LessThanEqual,
            TokenKind::GreaterThan,
            TokenKind::GreaterThanEqual,
        ]) {
            let operator = match self.previous().kind {
                TokenKind::LessThan => BinaryOperator::LessThan,
                TokenKind::LessThanEqual => BinaryOperator::LessThanOrEqual,
                TokenKind::GreaterThan => BinaryOperator::GreaterThan,
                TokenKind::GreaterThanEqual => BinaryOperator::GreaterThanOrEqual,
                _ => BinaryOperator::Equal,
            };
            let right = Box::new(self.parse_term()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_factor()?;

        while self.match_one(&[TokenKind::Plus, TokenKind::Minus]) {
            let operator = if self.previous().kind == TokenKind::Plus {
                BinaryOperator::Plus
            } else {
                BinaryOperator::Minus
            };
            let right = Box::new(self.parse_factor()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_unary()?;

        while self.match_one(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let operator = match self.previous().kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => BinaryOperator::Multiply,
            };
            let right = Box::new(self.parse_unary()?);
            let span = expr.span().start..right.span().end;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span,
            });
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        if self.match_one(&[TokenKind::Await]) {
            let start = self.previous().span.start;
            let argument = Box::new(self.parse_unary()?);
            let span = start..argument.span().end;
            return Ok(Expr::Await(AwaitExpr { argument, span }));
        }

        if self.match_one(&[TokenKind::PlusPlus, TokenKind::MinusMinus]) {
            let start = self.previous().span.start;
            let operator = match self.previous().kind {
                TokenKind::PlusPlus => UpdateOperator::Increment,
                TokenKind::MinusMinus => UpdateOperator::Decrement,
                _ => UpdateOperator::Increment,
            };
            let argument = Box::new(self.parse_unary()?);
            let span = start..argument.span().end;
            return Ok(Expr::Update(UpdateExpr {
                argument,
                operator,
                prefix: true,
                span,
            }));
        }

        if self.match_one(&[TokenKind::Bang, TokenKind::Minus, TokenKind::Plus]) {
            let start = self.previous().span.start;
            let operator = match self.previous().kind {
                TokenKind::Bang => UnaryOperator::LogicalNot,
                TokenKind::Minus => UnaryOperator::Minus,
                TokenKind::Plus => UnaryOperator::Plus,
                _ => UnaryOperator::LogicalNot,
            };
            let argument = Box::new(self.parse_unary()?);
            let span = start..argument.span().end;
            return Ok(Expr::Unary(UnaryExpr {
                argument,
                operator,
                span,
            }));
        }

        if self.match_one(&[TokenKind::Ampersand]) {
            let start = self.previous().span.start;
            let is_mut = self.match_one(&[TokenKind::Mut]);
            let argument = Box::new(self.parse_unary()?);
            let span = start..argument.span().end;

            return if is_mut {
                Ok(Expr::MutRef(MutRefExpr {
                    expr: argument,
                    ty: Box::new(Type::Any(AnyType { span: span.clone() })),
                    span,
                }))
            } else {
                Ok(Expr::Ref(RefExpr {
                    expr: argument,
                    ty: Box::new(Type::Any(AnyType { span: span.clone() })),
                    span,
                }))
            };
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let mut expr = self.parse_primary()?;

        loop {
            if self.match_one(&[TokenKind::Dot]) {
                let property = Box::new(Expr::Identifier(self.expect_identifier()?));
                let span = expr.span().start..property.span().end;
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    computed: false,
                    span,
                });
            } else if self.match_one(&[TokenKind::PlusPlus, TokenKind::MinusMinus]) {
                let operator = match self.previous().kind {
                    TokenKind::PlusPlus => UpdateOperator::Increment,
                    TokenKind::MinusMinus => UpdateOperator::Decrement,
                    _ => UpdateOperator::Increment,
                };
                let span = expr.span().start..self.previous().span.end;
                expr = Expr::Update(UpdateExpr {
                    argument: Box::new(expr),
                    operator,
                    prefix: false,
                    span,
                });
            } else if self.match_one(&[TokenKind::OpenBracket]) {
                let property = Box::new(self.parse_expression()?);
                self.expect(TokenKind::CloseBracket)?;
                let span = expr.span().start..self.previous().span.end;
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    computed: true,
                    span,
                });
            } else if self.check(&TokenKind::LessThan) {
                // Potential generic call: `callee<T>(args)`.
                // Must not consume `<` when this is actually a comparison (`a < b`), so we
                // backtrack unless we can parse `<...>` and the next token is `(`.
                let saved_pos = self.current;
                let type_args = match self.parse_type_args_opt() {
                    Ok(args) if !args.is_empty() => args,
                    Ok(_) => {
                        self.current = saved_pos;
                        vec![]
                    }
                    Err(_) => {
                        self.current = saved_pos;
                        vec![]
                    }
                };

                if !type_args.is_empty() && self.match_one(&[TokenKind::OpenParen]) {
                    let mut args = Vec::new();
                    let mut first = true;
                    while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                        if !first {
                            if !self.check(&TokenKind::CloseParen) {
                                self.expect_comma()?;
                            }
                        } else {
                            first = false;
                        }
                        args.push(ExprOrSpread::Expr(self.parse_expression()?));
                    }
                    self.expect(TokenKind::CloseParen)?;
                    let span = expr.span().start..self.previous().span.end;
                    expr = Expr::Call(CallExpr {
                        callee: Box::new(expr),
                        arguments: args,
                        type_args,
                        span,
                    });
                } else {
                    // Not a generic call; restore cursor so `<` can be parsed as a binary operator.
                    self.current = saved_pos;
                    break;
                }
            } else if self.match_one(&[TokenKind::OpenParen]) {
                let mut args = Vec::new();
                let mut first = true;
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    if !first {
                        if !self.check(&TokenKind::CloseParen) {
                            self.expect_comma()?;
                        }
                    } else {
                        first = false;
                    }
                    args.push(ExprOrSpread::Expr(self.parse_expression()?));
                }
                self.expect(TokenKind::CloseParen)?;
                let span = expr.span().start..self.previous().span.end;
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    arguments: args,
                    type_args: vec![],
                    span,
                });
            } else if self.match_one(&[TokenKind::OpenBrace]) {
                // Check if this is a struct literal (Identifier { key: value }) vs object literal
                let is_struct_literal = matches!(expr, Expr::Identifier(_));

                // Parse the object properties (key: value pairs)
                let span_start = self.previous().span.start;
                let mut properties = Vec::new();

                while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                    if self.check(&TokenKind::CloseBrace) {
                        break;
                    }

                    let key = self.parse_expression()?;

                    let (value, shorthand) = if self.match_one(&[TokenKind::Colon]) {
                        (ExprOrSpread::Expr(self.parse_expression()?), false)
                    } else {
                        (ExprOrSpread::Expr(key.clone()), true)
                    };

                    let prop_start = key.span().start;
                    let prop_end = self.previous().span.end;
                    properties.push(ObjectProperty::Property(Property {
                        key,
                        value,
                        kind: PropertyKind::Init,
                        method: false,
                        computed: false,
                        shorthand,
                        span: prop_start..prop_end,
                    }));

                    if !self.check(&TokenKind::CloseBrace) {
                        self.match_one(&[TokenKind::Comma]);
                        self.match_one(&[TokenKind::Semi]);
                    }
                }

                self.expect(TokenKind::CloseBrace)?;
                let obj_span = span_start..self.previous().span.end;

                if is_struct_literal {
                    // Struct literal: Person { x: 10, y: 20 } -> new Person({ x: 10, y: 20 })
                    let obj_expr = Expr::Object(ObjectExpression {
                        properties,
                        span: obj_span.clone(),
                    });
                    let span = expr.span().start..obj_span.end;
                    expr = Expr::New(NewExpr {
                        callee: Box::new(expr),
                        arguments: vec![ExprOrSpread::Expr(obj_expr)],
                        type_args: vec![],
                        span,
                    });
                } else {
                    // Regular object literal
                    expr = Expr::Object(ObjectExpression {
                        properties,
                        span: obj_span,
                    });
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_jsx_element(&mut self) -> Result<argon_ast::Expr, ParseError> {
        #[allow(unused_imports)]
        use argon_ast::*;

        if self.match_one(&[TokenKind::JsxFragmentOpen]) {
            return self.parse_jsx_fragment();
        }

        if self.check(&TokenKind::JsxElementOpen) {
            return self.parse_jsx_element_inner();
        }

        Err(self.parser_error_here("Expected JSX element"))
    }

    fn parse_jsx_fragment(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let opening_span = self.previous().span.clone();
        let start = opening_span.start;
        let children = self.parse_jsx_children()?;

        self.expect_token(TokenKind::JsxFragmentClose, "JSX fragment")?;
        let closing_span = self.previous().span.clone();
        let span = start..closing_span.end;

        Ok(Expr::JsxFragment(JsxFragment {
            opening: JsxOpeningFragment { span: opening_span },
            children,
            closing: JsxClosingFragment { span: closing_span },
            span,
        }))
    }

    fn parse_jsx_element_inner(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let start = self.peek().span.start;
        let name = self.parse_jsx_element_name()?;
        let attributes = self.parse_jsx_attributes()?;

        let is_self_closing = self.match_one(&[TokenKind::JsxSelfClosing]);
        if !is_self_closing {
            // End of opening tag.
            self.expect(TokenKind::GreaterThan)?;
        }

        let opening_span = start..self.previous().span.end;
        let opening = JsxOpeningElement {
            name,
            attributes,
            self_closing: is_self_closing,
            span: opening_span,
        };

        let mut children = Vec::new();

        if !is_self_closing && !self.check(&TokenKind::JsxElementClose) {
            children = self.parse_jsx_children()?;
        }

        let closing = if !is_self_closing && self.match_one(&[TokenKind::JsxElementClose]) {
            let close_span = self.previous().span.clone();
            Some(JsxClosingElement {
                name: self.parse_jsx_closing_element_name()?,
                span: close_span,
            })
        } else {
            None
        };

        let end = closing
            .as_ref()
            .map(|c| c.span.end)
            .unwrap_or(opening.span.end);
        Ok(Expr::JsxElement(JsxElement {
            opening,
            children,
            closing,
            span: start..end,
        }))
    }

    fn parse_jsx_element_name(&mut self) -> Result<argon_ast::JsxElementName, ParseError> {
        use argon_ast::*;

        // Check if we have a JsxElementOpen token - extract name from its span
        let current_token = &self.peek().kind;

        if let TokenKind::JsxElementOpen = current_token {
            let token = self.peek();
            let span = &token.span;
            // Extract the element name from the source (skip <)
            let name_str = &self.source[span.start + 1..span.end];
            // Remove any trailing > or whitespace
            let name_str = name_str.trim_end_matches(&['>', ' ', '\t', '\n'][..]);

            let name = Ident {
                sym: name_str.to_string(),
                span: span.clone(),
            };
            self.advance(); // consume the JsxElementOpen token
            return Ok(JsxElementName::Identifier(name));
        }

        let name = self.expect_identifier()?;
        let mut result = JsxElementName::Identifier(name);

        while self.match_one(&[TokenKind::Dot]) {
            let member = self.expect_identifier()?;
            result = JsxElementName::Member(Box::new(JsxElementName::Identifier(member)));
        }

        Ok(result)
    }

    fn parse_jsx_closing_element_name(&mut self) -> Result<argon_ast::JsxElementName, ParseError> {
        use argon_ast::*;

        // The JsxElementClose token span includes `</Name>`; extract the name from source.
        let span = self.previous().span.clone();
        let raw = self.source[span.clone()].trim();
        let name_str = raw
            .strip_prefix("</")
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or("")
            .trim();

        Ok(JsxElementName::Identifier(Ident {
            sym: name_str.to_string(),
            span,
        }))
    }

    fn parse_jsx_attributes(&mut self) -> Result<Vec<argon_ast::JsxAttribute>, ParseError> {
        use argon_ast::*;

        let mut attributes = Vec::new();

        while !self.check(&TokenKind::GreaterThan)
            && !self.check(&TokenKind::JsxSelfClosing)
            && !self.is_at_end()
        {
            if self.check(&TokenKind::Identifier) {
                let name = JsxAttributeName::Identifier(self.expect_identifier()?);
                let start = match &name {
                    JsxAttributeName::Identifier(id) => id.span.start,
                    JsxAttributeName::Namespaced(ns) => ns.name.span.start,
                };
                let value = if self.match_one(&[TokenKind::Equal]) {
                    Some(self.parse_jsx_attribute_value()?)
                } else {
                    None
                };
                let end = self.previous().span.end;
                attributes.push(JsxAttribute {
                    name,
                    value,
                    span: start..end,
                });
                continue;
            }

            if self.match_one(&[TokenKind::DotDotDot]) {
                // Spread attributes like `{...props}`. Not fully supported yet; parse as an expression.
                let start = self.previous().span.start;
                let expr = self.parse_expression()?;
                let end = self.previous().span.end;
                attributes.push(JsxAttribute {
                    name: JsxAttributeName::Identifier(Ident {
                        sym: "spread".to_string(),
                        span: start..start,
                    }),
                    value: Some(JsxAttributeValue::Expression(expr)),
                    span: start..end,
                });
                continue;
            }

            break;
        }

        Ok(attributes)
    }

    fn parse_jsx_attribute_value(&mut self) -> Result<argon_ast::JsxAttributeValue, ParseError> {
        use argon_ast::*;

        if self.match_one(&[TokenKind::OpenBrace]) {
            let expr = self.parse_expression()?;
            self.expect(TokenKind::CloseBrace)?;
            return Ok(JsxAttributeValue::Expression(expr));
        }

        if self.match_one(&[TokenKind::StringLiteral]) {
            let span = self.previous().span.clone();
            let value = self.source[span.clone()].to_string();
            return Ok(JsxAttributeValue::String(StringLiteral { value, span }));
        }

        if self.match_one(&[TokenKind::JsxElementOpen]) {
            let elem = self.parse_jsx_element()?;
            if let Expr::JsxElement(e) = elem {
                return Ok(JsxAttributeValue::Element(e));
            }
        }

        Err(self.parser_error_here("Expected JSX attribute value"))
    }

    fn parse_jsx_children(&mut self) -> Result<Vec<argon_ast::JsxChild>, ParseError> {
        use argon_ast::*;

        let mut children = Vec::new();

        while !self.check(&TokenKind::JsxElementClose)
            && !self.check(&TokenKind::JsxFragmentClose)
            && !self.is_at_end()
        {
            if self.check(&TokenKind::OpenBrace) {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::CloseBrace)?;
                children.push(JsxChild::Expression(expr));
            } else if self.check(&TokenKind::JsxElementOpen) {
                let elem = self.parse_jsx_element()?;
                if let Expr::JsxElement(e) = elem {
                    children.push(JsxChild::Element(e));
                }
            } else if self.check(&TokenKind::JsxFragmentOpen) {
                let elem = self.parse_jsx_element()?;
                if let Expr::JsxFragment(f) = elem {
                    children.push(JsxChild::Fragment(f));
                }
            } else if self.match_one(&[TokenKind::JsxChild]) {
                let span = self.previous().span.clone();
                let value = self.source[span.clone()].to_string();
                children.push(JsxChild::Text(JsxText { value, span }));
            } else {
                break;
            }
        }

        Ok(children)
    }

    fn parse_primary(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        if self.match_one(&[TokenKind::JsxElementOpen])
            || self.match_one(&[TokenKind::JsxFragmentOpen])
        {
            self.current -= 1;
            return self.parse_jsx_element();
        }

        if self.match_one(&[TokenKind::True]) {
            let span = self.previous().span.clone();
            return Ok(Expr::Literal(Literal::Boolean(BooleanLiteral {
                value: true,
                span,
            })));
        }

        if self.match_one(&[TokenKind::False]) {
            let span = self.previous().span.clone();
            return Ok(Expr::Literal(Literal::Boolean(BooleanLiteral {
                value: false,
                span,
            })));
        }

        if self.match_one(&[TokenKind::Null]) {
            return Ok(Expr::Literal(Literal::Null(NullLiteral {
                span: self.previous().span.clone(),
            })));
        }

        if self.match_one(&[TokenKind::NumberLiteral]) {
            let span = self.previous().span.clone();
            let raw = self.source[span.clone()].to_string();
            return Ok(Expr::Literal(Literal::Number(NumberLiteral {
                value: raw.parse().unwrap_or(0.0),
                raw,
                span,
            })));
        }

        if self.match_one(&[TokenKind::StringLiteral]) {
            let span = self.previous().span.clone();
            let value = self.source[span.clone()].to_string();
            return Ok(Expr::Literal(Literal::String(StringLiteral {
                value,
                span,
            })));
        }

        if self.match_one(&[TokenKind::TemplateComplete]) {
            return self.parse_template_literal();
        }

        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let sym = self.source[span.clone()].to_string();
            return Ok(Expr::Identifier(Ident { sym, span }));
        }

        if self.match_one(&[TokenKind::New]) {
            let start = self.previous().span.start;
            // Parse new expression - just parse the class name as identifier
            let ident = self.expect_identifier()?;
            let callee = argon_ast::Expr::Identifier(ident);

            let type_args = self.parse_type_args_opt()?;

            let mut arguments = Vec::new();
            if self.match_one(&[TokenKind::OpenParen]) {
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    arguments.push(ExprOrSpread::Expr(self.parse_expression()?));
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect_comma()?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
            }
            let end = self.previous().span.end;
            return Ok(Expr::New(argon_ast::NewExpr {
                callee: Box::new(callee),
                arguments,
                type_args,
                span: start..end,
            }));
        }

        if self.match_one(&[TokenKind::OpenParen]) {
            let expr = self.parse_expression()?;
            self.expect(TokenKind::CloseParen)?;
            return Ok(expr);
        }

        if self.match_one(&[TokenKind::OpenBrace]) {
            let span_start = self.previous().span.start;
            let mut properties = Vec::new();

            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                if self.match_one(&[TokenKind::Comma, TokenKind::Semi]) {
                    continue;
                }

                let key = self.parse_expression()?;
                let (value, shorthand) = if self.match_one(&[TokenKind::Colon]) {
                    (ExprOrSpread::Expr(self.parse_expression()?), false)
                } else {
                    (ExprOrSpread::Expr(key.clone()), true)
                };

                let prop_start = key.span().start;
                let prop_end = self.previous().span.end;
                properties.push(ObjectProperty::Property(Property {
                    key,
                    value,
                    kind: PropertyKind::Init,
                    method: false,
                    computed: false,
                    shorthand,
                    span: prop_start..prop_end,
                }));

                if !self.check(&TokenKind::CloseBrace) {
                    self.match_one(&[TokenKind::Comma, TokenKind::Semi]);
                }
            }

            self.expect(TokenKind::CloseBrace)?;
            let span = span_start..self.previous().span.end;
            return Ok(Expr::Object(ObjectExpression { properties, span }));
        }

        if self.match_one(&[TokenKind::OpenBracket]) {
            let span_start = self.previous().span.start;
            let mut elements: Vec<Option<argon_ast::ExprOrSpread>> = Vec::new();

            while !self.check(&TokenKind::CloseBracket) && !self.is_at_end() {
                if self.match_one(&[TokenKind::DotDotDot]) {
                    let start = self.previous().span.start;
                    let expr = self.parse_expression()?;
                    let end = self.previous().span.end;
                    let spread = SpreadElement {
                        argument: Box::new(expr),
                        span: start..end,
                    };
                    elements.push(Some(ExprOrSpread::Spread(spread)));
                } else {
                    elements.push(Some(ExprOrSpread::Expr(self.parse_expression()?)));
                }

                if !self.check(&TokenKind::CloseBracket) {
                    self.expect_comma()?;
                }
            }

            self.expect(TokenKind::CloseBracket)?;
            let span = span_start..self.previous().span.end;
            return Ok(Expr::Array(ArrayExpression { elements, span }));
        }

        if self.match_one(&[TokenKind::This]) {
            return Ok(Expr::This(ThisExpr {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Function]) {
            return self.parse_function_expression();
        }

        Err(self.unexpected_here(format!("Unexpected token at position {}", self.current)))
    }

    fn parse_template_literal(&mut self) -> Result<argon_ast::Expr, ParseError> {
        use argon_ast::*;

        let span = self.previous().span.clone();
        let raw = self.source[span.clone()].to_string();
        if raw.len() < 2 || !raw.starts_with('`') || !raw.ends_with('`') {
            return Err(self.parser_error_prev("Invalid template literal token"));
        }

        let inner = &raw[1..raw.len() - 1];
        let (quasis, expressions) = self.split_template_parts(inner)?;

        Ok(Expr::Template(TemplateLiteral {
            quasis,
            expressions,
            span,
        }))
    }

    fn split_template_parts(
        &self,
        inner: &str,
    ) -> Result<(Vec<argon_ast::TemplateElement>, Vec<argon_ast::Expr>), ParseError> {
        use argon_ast::*;

        let mut quasis = Vec::new();
        let mut expressions = Vec::new();

        let bytes = inner.as_bytes();
        let mut i = 0usize;
        let mut last_quasi_start = 0usize;

        while i < bytes.len() {
            let ch = bytes[i] as char;

            if ch == '\\' {
                i = (i + 2).min(bytes.len());
                continue;
            }

            if ch == '$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                let quasi_value = inner[last_quasi_start..i].to_string();
                quasis.push(TemplateElement {
                    value: quasi_value,
                    tail: false,
                    span: 0..0,
                });

                i += 2;
                let expr_start = i;
                let mut depth = 1usize;
                let mut escaped = false;
                let mut in_single = false;
                let mut in_double = false;
                let mut in_backtick = false;

                while i < bytes.len() {
                    let c = bytes[i] as char;

                    if escaped {
                        escaped = false;
                        i += 1;
                        continue;
                    }

                    if c == '\\' {
                        escaped = true;
                        i += 1;
                        continue;
                    }

                    if in_single {
                        if c == '\'' {
                            in_single = false;
                        }
                        i += 1;
                        continue;
                    }
                    if in_double {
                        if c == '"' {
                            in_double = false;
                        }
                        i += 1;
                        continue;
                    }
                    if in_backtick {
                        if c == '`' {
                            in_backtick = false;
                        }
                        i += 1;
                        continue;
                    }

                    match c {
                        '\'' => in_single = true,
                        '"' => in_double = true,
                        '`' => in_backtick = true,
                        '{' => depth += 1,
                        '}' => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }

                if depth != 0 || i >= bytes.len() {
                    return Err(
                        self.parser_error_prev("Unterminated template interpolation expression")
                    );
                }

                let expr_src = inner[expr_start..i].trim();
                let expr = self.parse_template_expression(expr_src)?;
                expressions.push(expr);

                i += 1; // Skip closing `}`.
                last_quasi_start = i;
                continue;
            }

            i += 1;
        }

        quasis.push(TemplateElement {
            value: inner[last_quasi_start..].to_string(),
            tail: true,
            span: 0..0,
        });

        Ok((quasis, expressions))
    }

    fn parse_template_expression(&self, expr_src: &str) -> Result<argon_ast::Expr, ParseError> {
        if expr_src.is_empty() {
            return Err(self.parser_error_prev("Empty template interpolation expression"));
        }

        let mut lexer = argon_lexer::Lexer::new(expr_src);
        let tokens = lexer.tokenize().map_err(|e| {
            self.parser_error_prev(format!("Invalid template interpolation expression: {}", e))
        })?;

        let mut parser = Parser::new(tokens, expr_src.to_string());
        let expr = parser.parse_expression().map_err(|e| {
            self.parser_error_prev(format!("Invalid template interpolation expression: {}", e))
        })?;

        if !parser.is_at_end() && !parser.check(&TokenKind::Eof) {
            return Err(
                self.parser_error_prev("Template interpolation expression has trailing tokens")
            );
        }

        Ok(expr)
    }

    fn parse_type(&mut self) -> Result<argon_ast::Type, ParseError> {
        self.parse_type_union()
    }

    fn parse_type_union(&mut self) -> Result<argon_ast::Type, ParseError> {
        use argon_ast::*;

        let start = self.peek().span.start;
        let mut types = vec![self.parse_type_intersection()?];

        while self.match_one(&[TokenKind::Pipe]) {
            types.push(self.parse_type_intersection()?);
        }

        if types.len() == 1 {
            return Ok(types.remove(0));
        }

        let span = start..self.previous().span.end;
        Ok(Type::Union(UnionType { types, span }))
    }

    fn parse_type_intersection(&mut self) -> Result<argon_ast::Type, ParseError> {
        use argon_ast::*;

        let start = self.peek().span.start;
        let mut types = vec![self.parse_type_postfix()?];

        while self.match_one(&[TokenKind::Ampersand]) {
            types.push(self.parse_type_postfix()?);
        }

        if types.len() == 1 {
            return Ok(types.remove(0));
        }

        let span = start..self.previous().span.end;
        Ok(Type::Intersection(IntersectionType { types, span }))
    }

    fn parse_type_postfix(&mut self) -> Result<argon_ast::Type, ParseError> {
        use argon_ast::*;

        let start = self.peek().span.start;
        let mut ty = self.parse_type_primary()?;

        loop {
            if self.match_one(&[TokenKind::OpenBracket]) {
                // Postfix array type: `T[]`
                self.expect(TokenKind::CloseBracket)?;
                let span = start..self.previous().span.end;
                ty = Type::Array(ArrayType {
                    elem_type: Box::new(ty),
                    span,
                });
                continue;
            }

            if self.match_one(&[TokenKind::Question]) {
                let span = start..self.previous().span.end;
                ty = Type::Optional(OptionalType {
                    ty: Box::new(ty),
                    span,
                });
                continue;
            }

            break;
        }

        Ok(ty)
    }

    fn parse_type_primary(&mut self) -> Result<argon_ast::Type, ParseError> {
        use argon_ast::*;

        if self.match_one(&[TokenKind::Ampersand]) {
            let start = self.previous().span.start;
            let is_mut = self.match_one(&[TokenKind::Mut]);
            let inner = Box::new(self.parse_type_primary()?);
            let span = start..self.previous().span.end;
            return Ok(if is_mut {
                Type::MutRef(MutRefType {
                    lifetime: None,
                    ty: inner,
                    span,
                })
            } else {
                Type::Ref(RefType {
                    lifetime: None,
                    ty: inner,
                    span,
                })
            });
        }

        if self.match_one(&[TokenKind::NumberKw]) {
            return Ok(Type::Number(NumberType {
                span: self.previous().span.clone(),
            }));
        }

        // Numeric type keywords map to number for now.
        if self.match_one(&[
            TokenKind::I8,
            TokenKind::I16,
            TokenKind::I32,
            TokenKind::I64,
            TokenKind::U8,
            TokenKind::U16,
            TokenKind::U32,
            TokenKind::U64,
            TokenKind::F32,
            TokenKind::F64,
            TokenKind::Isize,
            TokenKind::Usize,
        ]) {
            return Ok(Type::Number(NumberType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::StringKw]) {
            return Ok(Type::String(StringType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Boolean]) {
            return Ok(Type::Boolean(BooleanType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Any]) {
            return Ok(Type::Any(AnyType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Unknown]) {
            return Ok(Type::Unknown(UnknownType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Never]) {
            return Ok(Type::Never(NeverType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Void]) {
            return Ok(Type::Void(VoidType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Null]) {
            return Ok(Type::Null(NullType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Undefined]) {
            return Ok(Type::Undefined(UndefinedType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::Object]) {
            return Ok(Type::Primitive(PrimitiveType::Object));
        }

        if self.match_one(&[TokenKind::Symbol]) {
            return Ok(Type::Symbol(SymbolType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::BigInt]) {
            return Ok(Type::BigInt(BigIntType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::This]) {
            return Ok(Type::ThisType(ThisType {
                span: self.previous().span.clone(),
            }));
        }

        if self.match_one(&[TokenKind::OpenBracket]) {
            let start = self.previous().span.start;
            let mut types = Vec::new();
            while !self.check(&TokenKind::CloseBracket) && !self.is_at_end() {
                types.push(self.parse_type()?);
                if !self.check(&TokenKind::CloseBracket) {
                    self.expect_comma()?;
                }
            }
            self.expect(TokenKind::CloseBracket)?;
            let span = start..self.previous().span.end;
            return Ok(Type::Tuple(TupleType { types, span }));
        }

        if self.match_one(&[TokenKind::OpenParen]) {
            let start = self.previous().span.start;

            // Empty parameter list: `() => T`
            if self.check(&TokenKind::CloseParen) {
                self.advance();
                self.expect(TokenKind::FatArrow)?;
                let return_type = Box::new(self.parse_type()?);
                let span = start..self.previous().span.end;
                return Ok(Type::Function(FunctionType {
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type,
                    span,
                }));
            }

            let next_kind = self
                .tokens
                .get(self.current + 1)
                .map(|t| t.kind)
                .unwrap_or(TokenKind::Eof);

            if self.check(&TokenKind::Identifier)
                && matches!(next_kind, TokenKind::Colon | TokenKind::Question)
            {
                // Function type: `(a: T, b?: U) => R`
                let mut params = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    let name = self.expect_identifier()?;
                    let optional = self.match_one(&[TokenKind::Question]);
                    self.expect(TokenKind::Colon)?;
                    let ty = self.parse_type()?;
                    params.push(FunctionTypeParam {
                        name: Some(name),
                        ty,
                        optional,
                    });
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect_comma()?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
                self.expect(TokenKind::FatArrow)?;
                let return_type = Box::new(self.parse_type()?);
                let span = start..self.previous().span.end;
                return Ok(Type::Function(FunctionType {
                    type_params: Vec::new(),
                    params,
                    return_type,
                    span,
                }));
            }

            // Parenthesized type: `(T)`
            let inner = self.parse_type()?;
            self.expect(TokenKind::CloseParen)?;
            return Ok(Type::Parenthesized(Box::new(inner)));
        }

        if self.check(&TokenKind::Identifier) {
            let start = self.peek().span.start;
            let name = self.parse_type_name()?;
            let type_args = self.parse_type_args_opt()?;
            let span = start..self.previous().span.end;

            // Desugar `Shared<T>` into a dedicated AST node so later stages don't need to special-case.
            if let (TypeName::Ident(id), [inner]) = (&name, type_args.as_slice()) {
                if id.sym == "Shared" {
                    return Ok(Type::Shared(SharedType {
                        ty: Box::new(inner.clone()),
                        span,
                    }));
                }
            }

            return Ok(Type::Reference(TypeReference {
                name,
                type_args,
                span,
            }));
        }

        Err(self.parser_error_here("Expected type"))
    }

    fn parse_type_name(&mut self) -> Result<argon_ast::TypeName, ParseError> {
        use argon_ast::*;

        let first = self.expect_identifier()?;
        let mut name = TypeName::Ident(first);

        while self.match_one(&[TokenKind::Dot]) {
            let member = self.expect_identifier()?;
            name = TypeName::Qualified(Box::new(name), member);
        }

        Ok(name)
    }

    fn parse_type_args_opt(&mut self) -> Result<Vec<argon_ast::Type>, ParseError> {
        if !self.match_one(&[TokenKind::LessThan]) {
            return Ok(Vec::new());
        }

        let mut args = Vec::new();
        while !self.is_type_arg_close() && !self.is_at_end() {
            args.push(self.parse_type()?);
            if !self.is_type_arg_close() {
                self.expect_comma()?;
            }
        }
        self.consume_type_arg_close()?;
        Ok(args)
    }

    #[allow(dead_code)]
    fn parse_type_args(&mut self) -> Result<Vec<argon_ast::Type>, ParseError> {
        self.parse_type_args_opt()
    }

    fn is_type_arg_close(&self) -> bool {
        self.check(&TokenKind::GreaterThan)
            || self.check(&TokenKind::GreaterThanGreaterThan)
            || self.check(&TokenKind::GreaterThanGreaterThanGreaterThan)
    }

    fn consume_type_arg_close(&mut self) -> Result<(), ParseError> {
        if self.match_one(&[TokenKind::GreaterThan]) {
            return Ok(());
        }

        // Split `>>` / `>>>` into individual `>` tokens when parsing type arguments.
        let kind = self.peek().kind;
        if matches!(
            kind,
            TokenKind::GreaterThanGreaterThan | TokenKind::GreaterThanGreaterThanGreaterThan
        ) {
            let span = self.peek().span.clone();
            let split_count = match kind {
                TokenKind::GreaterThanGreaterThan => 2,
                TokenKind::GreaterThanGreaterThanGreaterThan => 3,
                _ => 1,
            };

            // Replace the current token with a single `>`, then insert the remaining `>` tokens.
            self.tokens[self.current].kind = TokenKind::GreaterThan;
            for _ in 1..split_count {
                self.tokens.insert(
                    self.current + 1,
                    LexerToken::new(TokenKind::GreaterThan, span.clone()),
                );
            }

            self.advance();
            return Ok(());
        }

        Err(self.parser_error_here("Expected '>'"))
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.match_one(&[kind]) {
            Ok(())
        } else {
            Err(self.parser_error_here(format!("Expected {:?}", kind)))
        }
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        self.expect(TokenKind::Comma)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        !self.is_at_end() && &self.peek().kind == kind
    }

    fn match_one(&mut self, kinds: &[TokenKind]) -> bool {
        if self.is_at_end() {
            return false;
        }
        for kind in kinds {
            if &self.peek().kind == kind {
                self.advance();
                return true;
            }
        }
        false
    }

    fn expect_token(&mut self, kind: TokenKind, context: &str) -> Result<(), ParseError> {
        if self.match_one(&[kind]) {
            Ok(())
        } else {
            Err(ParseError::ExpectedToken(
                format!("{:?} in {}", kind, context),
                self.peek().span.start,
            ))
        }
    }

    fn parser_error_here(&self, msg: impl Into<String>) -> ParseError {
        ParseError::Parser {
            msg: msg.into(),
            span: self.peek().span.clone(),
        }
    }

    fn parser_error_prev(&self, msg: impl Into<String>) -> ParseError {
        ParseError::Parser {
            msg: msg.into(),
            span: self.previous().span.clone(),
        }
    }

    fn unexpected_here(&self, msg: impl Into<String>) -> ParseError {
        ParseError::UnexpectedToken {
            msg: msg.into(),
            span: self.peek().span.clone(),
        }
    }

    fn span_since(&self, start: usize) -> Span {
        start..self.previous().span.end
    }

    fn peek(&self) -> LexerToken {
        self.tokens
            .get(self.current)
            .cloned()
            .unwrap_or(LexerToken::new(TokenKind::Eof, 0..0))
    }

    fn previous(&self) -> LexerToken {
        self.tokens
            .get(self.current.saturating_sub(1))
            .cloned()
            .unwrap_or(LexerToken::new(TokenKind::Eof, 0..0))
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }
}
