//! SafeScript - Parser

#[cfg(test)]
mod parser_tests;

use safescript_ast::SourceFile;
use safescript_diagnostics::{Diagnostic, DiagnosticEngine, DiagnosticLabel};
use safescript_lexer::Token as LexerToken;
use safescript_lexer::{tokenize, LexerError, TokenKind};

#[derive(Debug, Clone)]
pub enum ParseError {
    Lexer(LexerError),
    Parser(String),
    UnexpectedToken(String),
    ExpectedToken(String, usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Lexer(err) => write!(f, "Lexer error: {}", err),
            ParseError::Parser(msg) => write!(f, "Parse error: {}", msg),
            ParseError::UnexpectedToken(msg) => write!(f, "Unexpected token: {}", msg),
            ParseError::ExpectedToken(msg, pos) => {
                write!(f, "Expected {} at position {}", msg, pos)
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn to_diagnostic(&self, source: &str, source_id: &str) -> Diagnostic {
        match self {
            ParseError::Lexer(err) => err.to_diagnostic(source, source_id),
            ParseError::Parser(msg) => Diagnostic::new(source_id.to_string(), 0..1, msg.clone())
                .with_code("P000".to_string()),
            ParseError::UnexpectedToken(msg) => Diagnostic::new(
                source_id.to_string(),
                0..10,
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

    pub fn parse_module(&mut self) -> Result<Vec<safescript_ast::Stmt>, ParseError> {
        self.parse_statements()
    }

    fn parse_statements(&mut self) -> Result<Vec<safescript_ast::Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            if self.check(&TokenKind::Eof) {
                break;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        if self.match_one(&[TokenKind::Const, TokenKind::Let, TokenKind::Var]) {
            return self.parse_variable();
        }
        if self.match_one(&[TokenKind::Function]) {
            return self.parse_function();
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
        if self.match_one(&[TokenKind::Class]) {
            return self.parse_class();
        }
        if self.match_one(&[TokenKind::Match]) {
            return self.parse_match();
        }
        if self.match_one(&[TokenKind::Import]) {
            return self.parse_import();
        }
        if self.match_one(&[TokenKind::Export]) {
            return self.parse_export();
        }
        if self.match_one(&[TokenKind::OpenBrace]) {
            return self.parse_block();
        }

        let expr = self.parse_expression()?;
        self.match_one(&[TokenKind::Semi]);
        Ok(safescript_ast::Stmt::Expr(safescript_ast::ExpressionStmt {
            expr,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_variable(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let kind = if self.previous().kind == TokenKind::Const {
            VariableKind::Const
        } else {
            VariableKind::Let
        };

        let name = self.expect_identifier()?;

        let ty = if self.match_one(&[TokenKind::Colon]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let init = if self.match_one(&[TokenKind::Equal]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::Variable(VariableStmt {
            kind,
            declarations: vec![VariableDeclarator {
                id: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty,
                    default: init.clone(),
                }),
                init,
                span: 0..10,
            }],
            span: 0..10,
        }))
    }

    fn parse_function(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let id = self.expect_identifier()?;

        self.expect(TokenKind::OpenParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty,
                    default: None,
                }),
                ty: None,
                default: None,
                is_optional: false,
                span: 0..5,
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

        let borrow_annotation = self.parse_borrow_annotation()?;

        self.expect(TokenKind::OpenBrace)?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect(TokenKind::CloseBrace)?;

        let body = FunctionBody {
            statements,
            span: 0..10,
        };

        Ok(Stmt::Function(FunctionDecl {
            id: Some(id),
            params,
            body,
            type_params: vec![],
            return_type,
            is_async: false,
            borrow_annotation,
            span: 0..10,
        }))
    }

    fn parse_function_expression(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let span_start = self.previous().span.start;

        self.expect(TokenKind::OpenParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty,
                    default: None,
                }),
                ty: None,
                default: None,
                is_optional: false,
                span: 0..5,
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

        let body = if self.match_one(&[TokenKind::OpenBrace]) {
            self.current -= 1;
            match self.parse_statement()? {
                Stmt::Block(b) => ArrowFunctionBody::Block(b),
                _ => {
                    return Err(ParseError::Parser(
                        "Function expression body must be a block".to_string(),
                    ))
                }
            }
        } else if self.match_one(&[TokenKind::FatArrow]) {
            let expr = self.parse_expression()?;
            let block = BlockStmt {
                statements: vec![Stmt::Return(ReturnStmt {
                    argument: Some(expr),
                    span: 0..10,
                })],
                span: 0..10,
            };
            ArrowFunctionBody::Block(block)
        } else {
            return Err(ParseError::Parser("Expected function body".to_string()));
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
    ) -> Result<Option<safescript_ast::BorrowAnnotation>, ParseError> {
        if !self.match_one(&[TokenKind::With]) {
            return Ok(None);
        }

        let span_start = self.previous().span.start;

        // Handle both & and &mut
        let is_mut = self.match_one(&[TokenKind::AmpersandMut]);
        if !is_mut && !self.match_one(&[TokenKind::Ampersand]) {
            return Err(ParseError::Parser("Expected '&' after 'with'".to_string()));
        }

        let kind = if is_mut {
            safescript_ast::BorrowKind::Mutable
        } else if self.match_one(&[TokenKind::Mut]) {
            safescript_ast::BorrowKind::Mutable
        } else {
            safescript_ast::BorrowKind::Shared
        };

        let target = if self.match_one(&[TokenKind::This]) {
            Some(safescript_ast::Ident {
                sym: "this".to_string(),
                span: self.previous().span.clone(),
            })
        } else if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let span = span_start..self.previous().span.end;

        Ok(Some(safescript_ast::BorrowAnnotation {
            kind,
            target,
            span,
        }))
    }

    fn parse_return(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let argument = if self.check(&TokenKind::Semi) || self.check(&TokenKind::OpenBrace) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::Return(ReturnStmt {
            argument,
            span: 0..10,
        }))
    }

    fn parse_if(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let consequent = Box::new(self.parse_statement()?);
        let alternate = if self.match_one(&[TokenKind::Else]) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If(IfStmt {
            condition,
            consequent,
            alternate,
            span: 0..10,
        }))
    }

    fn parse_block(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let span_start = self.previous().span.start;

        if self.match_one(&[TokenKind::CloseBrace]) {
            self.current -= 1;
            return Ok(Stmt::Block(BlockStmt {
                statements: vec![],
                span: span_start..span_start + 1,
            }));
        }

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

    fn parse_while(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_for(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        self.expect(TokenKind::OpenParen)?;

        let init = if self.match_one(&[TokenKind::Semi]) {
            None
        } else if self.match_one(&[TokenKind::Const, TokenKind::Let, TokenKind::Var]) {
            let var_stmt = self.parse_variable()?;
            match var_stmt {
                safescript_ast::Stmt::Variable(v) => Some(ForInit::Variable(v)),
                _ => None,
            }
        } else {
            let expr = self.parse_expression()?;
            self.match_one(&[TokenKind::Semi]);
            Some(ForInit::Expr(expr))
        };

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

        Ok(Stmt::For(ForStmt {
            init,
            test,
            update,
            body,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_do_while(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let body = Box::new(self.parse_statement()?);

        self.expect(TokenKind::While)?;
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::DoWhile(DoWhileStmt {
            body,
            condition,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_switch(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        self.expect(TokenKind::OpenParen)?;
        let discriminant = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Case]) {
                let test = Some(self.parse_expression()?);
                self.expect(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::CloseBrace)
                {
                    consequent.push(self.parse_statement()?);
                }

                cases.push(SwitchCase {
                    test,
                    consequent,
                    span: self.previous().span.clone(),
                });
            } else if self.match_one(&[TokenKind::Default]) {
                self.expect(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case) && !self.check(&TokenKind::CloseBrace) {
                    consequent.push(self.parse_statement()?);
                }

                cases.push(SwitchCase {
                    test: None,
                    consequent,
                    span: self.previous().span.clone(),
                });
            } else {
                self.advance();
            }
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Switch(SwitchStmt {
            discriminant,
            cases,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_try(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let block = match self.parse_statement()? {
            Stmt::Block(b) => b,
            _ => return Err(ParseError::Parser("try block must be a block".to_string())),
        };

        let handler = if self.match_one(&[TokenKind::Catch]) {
            self.expect(TokenKind::OpenParen)?;
            let param = if self.check(&TokenKind::Identifier) {
                Some(self.parse_pattern()?)
            } else {
                None
            };
            self.expect(TokenKind::CloseParen)?;

            let body = match self.parse_statement()? {
                Stmt::Block(b) => b,
                _ => {
                    return Err(ParseError::Parser(
                        "catch block must be a block".to_string(),
                    ))
                }
            };

            Some(CatchClause {
                param,
                body,
                span: self.previous().span.clone(),
            })
        } else {
            None
        };

        let finalizer = if self.match_one(&[TokenKind::Finally]) {
            match self.parse_statement()? {
                Stmt::Block(b) => Some(b),
                _ => {
                    return Err(ParseError::Parser(
                        "finally block must be a block".to_string(),
                    ))
                }
            }
        } else {
            None
        };

        Ok(Stmt::Try(TryStmt {
            block,
            handler,
            finalizer,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_break(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let label = if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::Break(BreakStmt {
            label,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_continue(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let label = if self.check(&TokenKind::Identifier) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::Continue(ContinueStmt {
            label,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_throw(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let argument = self.parse_expression()?;
        self.match_one(&[TokenKind::Semi]);

        Ok(Stmt::Throw(ThrowStmt {
            argument,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_struct(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let id = self.expect_identifier()?;

        let mut fields = Vec::new();
        self.expect(TokenKind::OpenBrace)?;

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let field_id = self.expect_identifier()?;

            self.expect(TokenKind::Colon)?;
            let type_annotation = Box::new(self.parse_type()?);

            fields.push(StructField {
                id: field_id,
                type_annotation,
                is_readonly: false,
                span: self.previous().span.clone(),
            });

            if !self.check(&TokenKind::CloseBrace) {
                self.match_one(&[TokenKind::Semi, TokenKind::Comma]);
            }
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Struct(StructDecl {
            id,
            type_params: vec![],
            fields,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_class(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let id = self.expect_identifier()?;

        let super_class = if self.match_one(&[TokenKind::Extends]) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let mut body_items = Vec::new();
        self.expect(TokenKind::OpenBrace)?;

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if self.match_one(&[TokenKind::Constructor]) {
                body_items.push(ClassMember::Constructor(self.parse_constructor()?));
            } else if self.check(&TokenKind::Identifier) {
                // Check if this is a method (identifier followed by parenthesis)
                let saved_pos = self.current;
                let _ = self.expect_identifier()?;
                let is_method_call = self.check(&TokenKind::OpenParen);
                self.current = saved_pos;

                if is_method_call {
                    // It's a method - parse identifier then call parse_method
                    let key_ident = self.expect_identifier()?;
                    let key = safescript_ast::Expr::Identifier(key_ident);
                    let method = self.parse_method(key)?;
                    body_items.push(ClassMember::Method(method));
                } else {
                    // It's a field - parse the full expression
                    let key = self.parse_expression()?;
                    if self.match_one(&[TokenKind::OpenParen]) {
                        // Actually it's a method after all
                        self.current -= 1;
                        let method = self.parse_method(key)?;
                        body_items.push(ClassMember::Method(method));
                    } else if self.match_one(&[TokenKind::Colon]) {
                        // It's a typed field
                        let type_annotation = Some(Box::new(self.parse_type()?));
                        let value = if self.match_one(&[TokenKind::Equal]) {
                            Some(self.parse_expression()?)
                        } else {
                            None
                        };
                        body_items.push(ClassMember::Field(ClassField {
                            key,
                            value,
                            type_annotation,
                            is_optional: false,
                            is_readonly: false,
                            span: self.previous().span.clone(),
                        }));
                    } else {
                        // Just an expression statement
                        body_items.push(ClassMember::Field(ClassField {
                            key,
                            value: None,
                            type_annotation: None,
                            is_optional: false,
                            is_readonly: false,
                            span: self.previous().span.clone(),
                        }));
                    }
                }
                self.match_one(&[TokenKind::Semi]);
            } else if self.check(&TokenKind::String) {
                // String key for computed property - treat as field
                let key = self.parse_expression()?;
                self.expect(TokenKind::Colon)?;
                let type_annotation = Some(Box::new(self.parse_type()?));
                let value = if self.match_one(&[TokenKind::Equal]) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                body_items.push(ClassMember::Field(ClassField {
                    key,
                    value,
                    type_annotation,
                    is_optional: false,
                    is_readonly: false,
                    span: self.previous().span.clone(),
                }));
                self.match_one(&[TokenKind::Semi]);
            } else {
                self.advance();
            }
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Class(ClassDecl {
            id,
            type_params: vec![],
            super_class,
            super_type_args: vec![],
            implements: vec![],
            body: ClassBody {
                body: body_items,
                span: self.previous().span.clone(),
            },
            span: self.previous().span.clone(),
        }))
    }

    fn parse_constructor(&mut self) -> Result<safescript_ast::Constructor, ParseError> {
        use safescript_ast::*;

        let mut params = Vec::new();
        self.expect(TokenKind::OpenParen)?;

        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty,
                    default: None,
                }),
                ty: None,
                default: None,
                is_optional: false,
                span: self.previous().span.clone(),
            });

            if !self.check(&TokenKind::CloseParen) {
                self.expect_comma()?;
            }
        }

        self.expect(TokenKind::CloseParen)?;
        let body = match self.parse_statement()? {
            Stmt::Block(b) => b,
            _ => {
                return Err(ParseError::Parser(
                    "constructor body must be a block".to_string(),
                ))
            }
        };

        Ok(Constructor {
            params,
            body,
            span: self.previous().span.clone(),
        })
    }

    fn parse_method(
        &mut self,
        key: safescript_ast::Expr,
    ) -> Result<safescript_ast::MethodDefinition, ParseError> {
        use safescript_ast::*;

        let mut params = Vec::new();
        self.expect(TokenKind::OpenParen)?;

        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let ty = if self.match_one(&[TokenKind::Colon]) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            params.push(Param {
                pat: Pattern::Identifier(IdentPattern {
                    name: name.clone(),
                    type_annotation: ty,
                    default: None,
                }),
                ty: None,
                default: None,
                is_optional: false,
                span: self.previous().span.clone(),
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

        // Parse borrow annotation (e.g., "with &mut this")
        let borrow_annotation = self.parse_borrow_annotation()?;

        let body_stmt = match self.parse_statement()? {
            Stmt::Block(b) => b,
            _ => {
                return Err(ParseError::Parser(
                    "method body must be a block".to_string(),
                ))
            }
        };

        let body = safescript_ast::FunctionBody {
            statements: body_stmt.statements,
            span: body_stmt.span,
        };

        Ok(MethodDefinition {
            key,
            value: FunctionDecl {
                id: None,
                params,
                body,
                type_params: vec![],
                return_type,
                is_async: false,
                borrow_annotation,
                span: self.previous().span.clone(),
            },
            kind: MethodKind::Method,
            is_static: false,
            span: self.previous().span.clone(),
        })
    }

    fn parse_match(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        self.expect(TokenKind::OpenParen)?;
        let discriminant = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let pattern = self.parse_expression()?;
            self.expect(TokenKind::FatArrow)?;
            let consequent = Box::new(self.parse_statement()?);

            cases.push(MatchCase {
                pattern,
                consequent,
                guard: None,
                span: self.previous().span.clone(),
            });

            self.match_one(&[TokenKind::Comma]);
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Match(MatchStmt {
            discriminant,
            cases,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_import(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        let specifiers = if self.match_one(&[TokenKind::OpenBrace]) {
            let mut specs = Vec::new();
            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                let imported = self.expect_identifier()?;
                let local: Option<Ident> = if self.match_one(&[TokenKind::As]) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                specs.push(ImportSpecifier::Named(NamedImportSpecifier {
                    imported,
                    local,
                    is_type: false,
                    span: self.previous().span.clone(),
                }));
                if !self.check(&TokenKind::CloseBrace) {
                    self.expect_comma()?;
                }
            }
            self.expect(TokenKind::CloseBrace)?;
            specs
        } else if self.check(&TokenKind::Identifier) {
            vec![ImportSpecifier::Default(DefaultImportSpecifier {
                local: self.expect_identifier()?,
                span: self.previous().span.clone(),
            })]
        } else {
            vec![ImportSpecifier::Default(DefaultImportSpecifier {
                local: Ident {
                    sym: "default".to_string(),
                    span: 0..0,
                },
                span: self.previous().span.clone(),
            })]
        };

        self.expect(TokenKind::From)?;
        let source = self.parse_literal_string()?;

        Ok(Stmt::Import(ImportStmt {
            specifiers,
            source,
            is_type_only: false,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_export(&mut self) -> Result<safescript_ast::Stmt, ParseError> {
        use safescript_ast::*;

        if self.match_one(&[TokenKind::Default]) {
            let declaration = self.parse_statement()?;
            return Ok(Stmt::Export(ExportStmt {
                declaration: Some(Box::new(declaration)),
                specifiers: vec![],
                source: None,
                is_type_only: true,
                span: self.previous().span.clone(),
            }));
        }

        if self.check(&TokenKind::OpenBrace) {
            self.expect(TokenKind::OpenBrace)?;
            let mut specs = Vec::new();
            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                let orig = self.expect_identifier()?;
                let exported = if self.match_one(&[TokenKind::As]) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                specs.push(ExportSpecifier {
                    orig,
                    exported,
                    span: self.previous().span.clone(),
                });
                if !self.check(&TokenKind::CloseBrace) {
                    self.expect_comma()?;
                }
            }
            self.expect(TokenKind::CloseBrace)?;
            return Ok(Stmt::Export(ExportStmt {
                declaration: None,
                specifiers: specs,
                source: None,
                is_type_only: false,
                span: self.previous().span.clone(),
            }));
        }

        let declaration = self.parse_statement()?;
        Ok(Stmt::Export(ExportStmt {
            declaration: Some(Box::new(declaration)),
            specifiers: vec![],
            source: None,
            is_type_only: false,
            span: self.previous().span.clone(),
        }))
    }

    fn parse_literal_string(&mut self) -> Result<safescript_ast::StringLiteral, ParseError> {
        if let TokenKind::String = &self.peek().kind {
            let span = self.previous().span.clone();
            let value = self.source[span.clone()].to_string();
            self.advance();
            Ok(safescript_ast::StringLiteral { value, span })
        } else {
            Err(ParseError::Parser("Expected string".to_string()))
        }
    }

    fn parse_pattern(&mut self) -> Result<safescript_ast::Pattern, ParseError> {
        use safescript_ast::*;

        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let name = self.source[span.clone()].to_string();
            Ok(Pattern::Identifier(IdentPattern {
                name: Ident { sym: name, span },
                type_annotation: None,
                default: None,
            }))
        } else {
            Err(ParseError::Parser("Expected pattern".to_string()))
        }
    }

    fn expect_identifier(&mut self) -> Result<safescript_ast::Ident, ParseError> {
        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let sym = self.source[span.clone()].to_string();
            return Ok(safescript_ast::Ident { sym, span });
        }
        Err(ParseError::Parser("Expected identifier".to_string()))
    }

    fn parse_expression(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let expr = self.parse_or()?;

        if self.match_one(&[TokenKind::Equal]) {
            let right = Box::new(self.parse_assignment()?);
            return Ok(Expr::Assignment(Box::new(AssignmentExpr {
                left: Box::new(AssignmentTarget::Simple(Box::new(expr))),
                operator: AssignmentOperator::Assign,
                right,
                span: 0..10,
            })));
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_and()?;

        while self.match_one(&[TokenKind::PipePipe]) {
            let right = Box::new(self.parse_and()?);
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator: LogicalOperator::Or,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_equality()?;

        while self.match_one(&[TokenKind::AmpersandAmpersand]) {
            let right = Box::new(self.parse_equality()?);
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator: LogicalOperator::And,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_comparison()?;

        while self.match_one(&[TokenKind::BangEqual, TokenKind::EqualEqual]) {
            let operator = if self.previous().kind == TokenKind::BangEqual {
                BinaryOperator::NotEqual
            } else {
                BinaryOperator::Equal
            };
            let right = Box::new(self.parse_comparison()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

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
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_factor()?;

        while self.match_one(&[TokenKind::Plus, TokenKind::Minus]) {
            let operator = if self.previous().kind == TokenKind::Plus {
                BinaryOperator::Plus
            } else {
                BinaryOperator::Minus
            };
            let right = Box::new(self.parse_factor()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_unary()?;

        while self.match_one(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let operator = match self.previous().kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => BinaryOperator::Multiply,
            };
            let right = Box::new(self.parse_unary()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
                span: 0..10,
            });
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        if self.match_one(&[TokenKind::Bang, TokenKind::Minus, TokenKind::Plus]) {
            let operator = match self.previous().kind {
                TokenKind::Bang => UnaryOperator::LogicalNot,
                TokenKind::Minus => UnaryOperator::Minus,
                TokenKind::Plus => UnaryOperator::Plus,
                _ => UnaryOperator::LogicalNot,
            };
            let argument = Box::new(self.parse_unary()?);
            return Ok(Expr::Unary(UnaryExpr {
                argument,
                operator,
                span: 0..10,
            }));
        }

        if self.match_one(&[TokenKind::Ampersand]) {
            let is_mut = self.match_one(&[TokenKind::Mut]);
            let argument = Box::new(self.parse_unary()?);
            let span = self.previous().span.start..argument.span().end;

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

    fn parse_postfix(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        let mut expr = self.parse_primary()?;

        loop {
            if self.match_one(&[TokenKind::Dot]) {
                let property = Box::new(Expr::Identifier(self.expect_identifier()?));
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    computed: false,
                    span: 0..10,
                });
            } else if self.match_one(&[TokenKind::OpenBracket]) {
                let property = Box::new(self.parse_expression()?);
                self.expect(TokenKind::CloseBracket)?;
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    computed: true,
                    span: 0..10,
                });
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
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    arguments: args,
                    span: 0..10,
                });
            } else if self.match_one(&[TokenKind::OpenBrace]) {
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

                    properties.push(ObjectProperty::Property(Property {
                        key,
                        value,
                        kind: PropertyKind::Init,
                        method: false,
                        computed: false,
                        shorthand,
                        span: 0..10,
                    }));

                    if !self.check(&TokenKind::CloseBrace) {
                        self.match_one(&[TokenKind::Comma]);
                        self.match_one(&[TokenKind::Semi]);
                    }
                }

                let span = span_start..self.previous().span.end;
                self.expect(TokenKind::CloseBrace)?;

                expr = Expr::Object(ObjectExpression { properties, span });
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<safescript_ast::Expr, ParseError> {
        use safescript_ast::*;

        if self.match_one(&[TokenKind::True]) {
            return Ok(Expr::Literal(Literal::Boolean(BooleanLiteral {
                value: true,
                span: 0..4,
            })));
        }

        if self.match_one(&[TokenKind::False]) {
            return Ok(Expr::Literal(Literal::Boolean(BooleanLiteral {
                value: false,
                span: 0..5,
            })));
        }

        if self.match_one(&[TokenKind::Null]) {
            return Ok(Expr::Literal(Literal::Null(NullLiteral { span: 0..4 })));
        }

        if self.match_one(&[TokenKind::Number]) {
            let span = self.previous().span.clone();
            let raw = self.source[span.clone()].to_string();
            return Ok(Expr::Literal(Literal::Number(NumberLiteral {
                value: raw.parse().unwrap_or(0.0),
                raw,
                span,
            })));
        }

        if self.match_one(&[TokenKind::String]) {
            let span = self.previous().span.clone();
            let value = self.source[span.clone()].to_string();
            return Ok(Expr::Literal(Literal::String(StringLiteral {
                value,
                span,
            })));
        }

        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let sym = self.source[span.clone()].to_string();
            return Ok(Expr::Identifier(Ident { sym, span }));
        }

        if self.match_one(&[TokenKind::New]) {
            // Parse new expression - just parse the class name as identifier
            let ident = self.expect_identifier()?;
            let callee = safescript_ast::Expr::Identifier(ident);

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
            return Ok(Expr::New(safescript_ast::NewExpr {
                callee: Box::new(callee),
                arguments,
                span: 0..10,
            }));
        }

        if self.match_one(&[TokenKind::OpenParen]) {
            let expr = self.parse_expression()?;
            self.expect(TokenKind::CloseParen)?;
            return Ok(expr);
        }

        if self.match_one(&[TokenKind::This]) {
            return Ok(Expr::This(ThisExpr { span: 0..4 }));
        }

        if self.match_one(&[TokenKind::Function]) {
            return self.parse_function_expression();
        }

        Err(ParseError::UnexpectedToken(format!(
            "Unexpected token at position {}",
            self.current
        )))
    }

    fn parse_type(&mut self) -> Result<safescript_ast::Type, ParseError> {
        use safescript_ast::*;

        if self.match_one(&[TokenKind::Ampersand]) {
            let span_start = self.previous().span.start;
            let inner = Box::new(self.parse_type()?);
            let span = span_start..self.previous().span.end;
            if self.match_one(&[TokenKind::AmpersandMut]) {
                return Ok(Type::MutRef(MutRefType {
                    lifetime: None,
                    ty: inner,
                    span,
                }));
            }
            return Ok(Type::Ref(RefType {
                lifetime: None,
                ty: inner,
                span,
            }));
        }

        if self.match_one(&[TokenKind::Number]) {
            return Ok(Type::Primitive(PrimitiveType::Number));
        }

        // Numeric types
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
            return Ok(Type::Primitive(PrimitiveType::Number));
        }

        if self.match_one(&[TokenKind::String]) {
            return Ok(Type::Primitive(PrimitiveType::String));
        }

        if self.match_one(&[TokenKind::Boolean]) {
            return Ok(Type::Primitive(PrimitiveType::Boolean));
        }

        if self.match_one(&[TokenKind::Any]) {
            return Ok(Type::Any(AnyType { span: 0..3 }));
        }

        if self.match_one(&[TokenKind::Void]) {
            return Ok(Type::Void(VoidType { span: 0..4 }));
        }

        if self.match_one(&[TokenKind::Identifier]) {
            let span = self.previous().span.clone();
            let sym = self.source[span.clone()].to_string();
            return Ok(Type::Reference(TypeReference {
                name: TypeName::Ident(Ident {
                    sym,
                    span: span.clone(),
                }),
                type_args: vec![],
                span,
            }));
        }

        Err(ParseError::Parser("Expected type".to_string()))
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.match_one(&[kind]) {
            Ok(())
        } else {
            Err(ParseError::Parser(format!("Expected {:?}", kind)))
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
