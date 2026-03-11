//! Argon - Lexer

mod error;
mod token;

#[cfg(test)]
mod lexer_tests;

pub use error::*;
pub use token::*;

use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    position: usize,
    start: usize,
    in_jsx: bool,
    in_jsx_tag: bool,
    jsx_depth: usize,
    prev_token_kind: Option<TokenKind>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            position: 0,
            start: 0,
            in_jsx: false,
            in_jsx_tag: false,
            jsx_depth: 0,
            prev_token_kind: None,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            if token.kind != TokenKind::Whitespace && token.kind != TokenKind::Comment {
                self.prev_token_kind = Some(token.kind);
                tokens.push(token);
            }
        }

        tokens.push(Token::new(TokenKind::Eof, self.position..self.position + 1));

        Ok(tokens)
    }

    fn should_start_jsx_after_prev(&self) -> bool {
        // Heuristic: only treat '<' as the start of JSX in positions where an expression can begin.
        // This avoids breaking comparisons like `i < 10` (which previously flipped the lexer into JSX mode).
        match self.prev_token_kind {
            None => true,
            Some(kind) => matches!(
                kind,
                TokenKind::Equal
                    | TokenKind::OpenParen
                    | TokenKind::OpenBrace
                    | TokenKind::OpenBracket
                    | TokenKind::Comma
                    | TokenKind::Colon
                    | TokenKind::Semi
                    | TokenKind::Return
                    | TokenKind::Throw
                    | TokenKind::Await
                    | TokenKind::Yield
                    | TokenKind::Question
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::Bang
                    | TokenKind::Ampersand
                    | TokenKind::Pipe
                    | TokenKind::Caret
                    | TokenKind::Tilde
                    | TokenKind::AmpersandAmpersand
                    | TokenKind::PipePipe
                    | TokenKind::EqualEqual
                    | TokenKind::EqualEqualEqual
                    | TokenKind::BangEqual
                    | TokenKind::BangEqualEqual
                    | TokenKind::LessThan
                    | TokenKind::LessThanEqual
                    | TokenKind::GreaterThan
                    | TokenKind::GreaterThanEqual
            ),
        }
    }

    fn looks_like_jsx_start(&mut self) -> bool {
        // We are currently positioned at '<'. Peek the next char to avoid treating operators like
        // '<=', '<<', or '< 10' as JSX.
        let next = self.peek_n(1);
        match next {
            Some('>') => true, // fragment open <>
            Some('/') => true, // closing tag </...>
            Some(c) => c.is_ascii_alphabetic() || c == '_' || c == '$',
            None => false,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace_and_comments();

        self.start = self.position;

        if let Some(&ch) = self.chars.peek() {
            let token = match ch {
                // JSX self-closing tag end: `/>`
                '/' if self.in_jsx_tag && self.peek_n(1) == Some('>') => {
                    self.advance(); // '/'
                    self.advance(); // '>'
                    self.in_jsx_tag = false;
                    if self.jsx_depth > 0 {
                        self.jsx_depth -= 1;
                    }
                    self.in_jsx = self.jsx_depth > 0;
                    Token::new(TokenKind::JsxSelfClosing, self.start..self.position)
                }

                // Punctuation
                '{' => {
                    if self.in_jsx {
                        self.make_jsx_expr_token()
                    } else {
                        self.make_token(TokenKind::OpenBrace)
                    }
                }
                '}' => {
                    self.make_token(TokenKind::CloseBrace)
                }
                '(' => self.make_token(TokenKind::OpenParen),
                ')' => self.make_token(TokenKind::CloseParen),
                '[' => self.make_token(TokenKind::OpenBracket),
                ']' => self.make_token(TokenKind::CloseBracket),
                ';' => self.make_token(TokenKind::Semi),
                ',' => self.make_token(TokenKind::Comma),
                '.' => self.make_dot_token(),
                '?' => self.make_question_token(),
                ':' => self.make_token(TokenKind::Colon),
                '~' => self.make_token(TokenKind::Tilde),

                // Operators
                '+' => self.make_plus_token(),
                '-' => self.make_minus_token(),
                '*' => self.make_star_token(),
                '/' => self.make_slash_token(),
                '%' => self.make_percent_token(),
                '^' => self.make_caret_token(),
                '!' => self.make_bang_token(),
                '&' => self.make_ampersand_token(),
                '|' => self.make_pipe_token(),
                '=' => self.make_equals_token(),
                '>' => {
                    if self.in_jsx_tag {
                        self.advance();
                        self.in_jsx_tag = false;
                        // After `>`, switch into JSX children mode.
                        self.in_jsx = true;
                        Token::new(TokenKind::GreaterThan, self.start..self.position)
                    } else {
                        self.make_greater_than_token()
                    }
                }

                // JSX - handle < in JSX context
                '<' => {
                    if self.in_jsx {
                        // In JSX mode - could be nested element or closing tag.
                        if self.peek_n(1) == Some('/') {
                            self.make_jsx_closing_tag_token()
                        } else {
                            self.make_jsx_token()
                        }
                    } else if self.should_start_jsx_after_prev() && self.looks_like_jsx_start() {
                        self.make_jsx_token()
                    } else {
                        self.make_less_than_token()
                    }
                }

                // Literals
                '"' | '\'' => self.make_string_token(ch),
                '0'..='9' => {
                    if self.in_jsx && !self.in_jsx_tag {
                        self.make_jsx_child_token()
                    } else {
                        self.make_number_token()
                    }
                }

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' | '$' => {
                    if self.in_jsx && !self.in_jsx_tag {
                        self.make_jsx_child_token()
                    } else {
                        self.make_identifier_or_keyword_token()
                    }
                }

                // Template literals
                '`' => self.make_template_literal_token(),

                _ => return Err(LexerError::UnexpectedCharacter(ch, self.position)),
            };

            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.start = self.position;

            match self.chars.peek() {
                Some(&ch) if ch.is_whitespace() => {
                    self.advance();
                    while let Some(&ch) = self.chars.peek() {
                        if ch.is_whitespace() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                Some(&'/') => {
                    self.advance();
                    match self.chars.peek() {
                        Some(&'/') => {
                            self.advance();
                            while let Some(&ch) = self.chars.peek() {
                                if ch == '\n' {
                                    break;
                                }
                                self.advance();
                            }
                        }
                        Some(&'*') => {
                            self.advance();
                            let mut prev = ' ';
                            while let Some(&ch) = self.chars.peek() {
                                self.advance();
                                if prev == '*' && ch == '/' {
                                    break;
                                }
                                prev = ch;
                            }
                        }
                        _ => {
                            self.position = self.start;
                            self.chars = self.source[self.start..].chars().peekable();
                            return;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn advance(&mut self) {
        if let Some(c) = self.chars.next() {
            self.position += c.len_utf8();
        }
    }

    fn peek_n(&mut self, n: usize) -> Option<char> {
        self.chars.clone().nth(n)
    }

    fn make_token(&mut self, kind: TokenKind) -> Token {
        let span = self.start..self.position + 1;
        self.advance();
        Token::new(kind, span)
    }

    fn make_dot_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'.') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'.') => {
                        self.advance();
                        Token::new(TokenKind::DotDotDot, self.start..self.position + 1)
                    }
                    _ => Token::new(TokenKind::DotDot, self.start..self.position + 1),
                }
            }
            _ => Token::new(TokenKind::Dot, self.start..self.position + 1),
        }
    }

    fn make_question_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'?') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'.') => {
                        self.advance();
                        Token::new(
                            TokenKind::QuestionQuestionDot,
                            self.start..self.position + 1,
                        )
                    }
                    Some(&'=') => {
                        self.advance();
                        Token::new(
                            TokenKind::QuestionQuestionEqual,
                            self.start..self.position + 1,
                        )
                    }
                    _ => Token::new(TokenKind::QuestionQuestion, self.start..self.position + 1),
                }
            }
            Some(&'.') => {
                self.advance();
                Token::new(TokenKind::QuestionDot, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Question, self.start..self.position + 1),
        }
    }

    fn make_plus_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'+') => {
                self.advance();
                Token::new(TokenKind::PlusPlus, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::PlusEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Plus, self.start..self.position + 1),
        }
    }

    fn make_minus_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'-') => {
                self.advance();
                Token::new(TokenKind::MinusMinus, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::MinusEqual, self.start..self.position + 1)
            }
            Some(&'>') => {
                self.advance();
                Token::new(TokenKind::Arrow, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Minus, self.start..self.position + 1),
        }
    }

    fn make_star_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'*') => {
                self.advance();
                Token::new(TokenKind::StarStar, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::StarEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Star, self.start..self.position + 1),
        }
    }

    fn make_slash_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::SlashEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Slash, self.start..self.position + 1),
        }
    }

    fn make_percent_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::PercentEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Percent, self.start..self.position + 1),
        }
    }

    fn make_caret_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'^') => {
                self.advance();
                Token::new(TokenKind::CaretCaret, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::CaretEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Caret, self.start..self.position + 1),
        }
    }

    fn make_bang_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'=') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'=') => {
                        self.advance();
                        Token::new(TokenKind::BangEqualEqual, self.start..self.position + 1)
                    }
                    _ => Token::new(TokenKind::BangEqual, self.start..self.position + 1),
                }
            }
            _ => Token::new(TokenKind::Bang, self.start..self.position + 1),
        }
    }

    fn make_ampersand_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'&') => {
                self.advance();
                Token::new(TokenKind::AmpersandAmpersand, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::AmpersandEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Ampersand, self.start..self.position + 1),
        }
    }

    fn make_pipe_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'|') => {
                self.advance();
                Token::new(TokenKind::PipePipe, self.start..self.position + 1)
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::PipeEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Pipe, self.start..self.position + 1),
        }
    }

    fn make_equals_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'=') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'=') => {
                        self.advance();
                        Token::new(TokenKind::EqualEqualEqual, self.start..self.position + 1)
                    }
                    _ => Token::new(TokenKind::EqualEqual, self.start..self.position + 1),
                }
            }
            Some(&'>') => {
                self.advance();
                Token::new(TokenKind::FatArrow, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::Equal, self.start..self.position + 1),
        }
    }

    fn make_less_than_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'<') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'=') => {
                        self.advance();
                        Token::new(
                            TokenKind::LessThanLessThanEqual,
                            self.start..self.position + 1,
                        )
                    }
                    _ => Token::new(TokenKind::LessThanLessThan, self.start..self.position + 1),
                }
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::LessThanEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::LessThan, self.start..self.position + 1),
        }
    }

    fn make_greater_than_token(&mut self) -> Token {
        self.advance();

        match self.chars.peek() {
            Some(&'>') => {
                self.advance();
                match self.chars.peek() {
                    Some(&'>') => {
                        self.advance();
                        match self.chars.peek() {
                            Some(&'=') => {
                                self.advance();
                                Token::new(
                                    TokenKind::GreaterThanGreaterThanGreaterThanEqual,
                                    self.start..self.position + 1,
                                )
                            }
                            _ => Token::new(
                                TokenKind::GreaterThanGreaterThanGreaterThan,
                                self.start..self.position + 1,
                            ),
                        }
                    }
                    Some(&'=') => {
                        self.advance();
                        Token::new(
                            TokenKind::GreaterThanGreaterThanEqual,
                            self.start..self.position + 1,
                        )
                    }
                    _ => Token::new(
                        TokenKind::GreaterThanGreaterThan,
                        self.start..self.position + 1,
                    ),
                }
            }
            Some(&'=') => {
                self.advance();
                Token::new(TokenKind::GreaterThanEqual, self.start..self.position + 1)
            }
            _ => Token::new(TokenKind::GreaterThan, self.start..self.position + 1),
        }
    }

    fn make_string_token(&mut self, quote: char) -> Token {
        self.advance();

        let mut escaped = false;

        while let Some(&ch) = self.chars.peek() {
            self.advance();

            if escaped {
                escaped = false;
                match ch {
                    'u' => {
                        if self.chars.peek() == Some(&'{') {
                            self.advance();
                            while let Some(&c) = self.chars.peek() {
                                self.advance();
                                if c == '}' {
                                    break;
                                }
                                if !c.is_ascii_hexdigit() {
                                    break;
                                }
                            }
                        } else {
                            for _ in 0..4 {
                                if let Some(&c) = self.chars.peek() {
                                    if c.is_ascii_hexdigit() {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    'x' => {
                        for _ in 0..2 {
                            if let Some(&c) = self.chars.peek() {
                                if c.is_ascii_hexdigit() {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == quote {
                // `self.position` is already advanced past the closing quote (exclusive end).
                return Token::new(TokenKind::StringLiteral, self.start..self.position);
            }

            if ch == '\n' {
                return Token::new(TokenKind::UnterminatedString, self.start..self.position);
            }
        }

        Token::new(TokenKind::UnterminatedString, self.start..self.position)
    }

    fn make_number_token(&mut self) -> Token {
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit()
                || ch == '.'
                || ch == 'e'
                || ch == 'E'
                || ch == 'x'
                || ch == 'X'
                || ch == 'o'
                || ch == 'O'
                || ch == 'b'
                || ch == 'B'
            {
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::NumberLiteral, self.start..self.position)
    }

    fn make_identifier_or_keyword_token(&mut self) -> Token {
        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[self.start..self.position];
        let kind = TokenKind::from_keyword(text);

        Token::new(kind, self.start..self.position)
    }

    fn make_template_literal_token(&mut self) -> Token {
        self.advance();

        let mut escaped = false;
        let mut has_interpolation = false;
        let mut brace_count = 0;
        let mut start = self.start;

        while let Some(&ch) = self.chars.peek() {
            self.advance();

            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == '`' {
                if has_interpolation {
                    return Token::new(TokenKind::TemplateComplete, self.start..self.position + 1);
                }
                return Token::new(TokenKind::TemplateComplete, self.start..self.position + 1);
            }

            if ch == '{' && !has_interpolation {
                has_interpolation = true;
                brace_count += 1;
                let interp_start = self.start;
                let text_before = &self.source[start..interp_start - 1];
                if text_before.is_empty() {
                    return Token::new(TokenKind::TemplateStart, start..interp_start);
                } else {
                    return Token::new(TokenKind::TemplateStart, start..self.position);
                }
            } else if ch == '}' && has_interpolation {
                brace_count -= 1;
                if brace_count == 0 {
                    return Token::new(TokenKind::TemplateMiddle, self.start..self.position + 1);
                }
            }

            if ch == '\n' && brace_count == 0 {
                return Token::new(TokenKind::UnterminatedTemplate, self.start..self.position);
            }
        }

        Token::new(TokenKind::UnterminatedTemplate, self.start..self.position)
    }

    fn make_jsx_token(&mut self) -> Token {
        // Cursor is at '<'.
        self.advance(); // consume '<'

        match self.chars.peek() {
            // Closing tag or fragment close.
            Some(&'/') => self.make_jsx_closing_tag_token(),

            // Fragment open: `<>`
            Some(&'>') => {
                self.advance(); // consume '>'
                self.in_jsx = true;
                self.in_jsx_tag = false;
                self.jsx_depth += 1;
                Token::new(TokenKind::JsxFragmentOpen, self.start..self.position)
            }

            // Opening tag: `<div ...>`
            _ => {
                let _name = self.read_jsx_identifier();
                self.in_jsx = true;
                self.in_jsx_tag = true;
                self.jsx_depth += 1;
                Token::new(TokenKind::JsxElementOpen, self.start..self.position)
            }
        }
    }

    fn read_jsx_identifier(&mut self) -> String {
        let mut identifier = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                self.advance();
                identifier.push(ch);
            } else {
                break;
            }
        }

        identifier
    }

    fn make_jsx_child_token(&mut self) -> Token {
        let mut text = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch == '<' {
                break;
            }
            if ch == '{' {
                break;
            }
            if ch == '\n' {
                break;
            }
            self.advance();
            text.push(ch);
        }

        if text.is_empty() {
            self.advance();
            return Token::new(TokenKind::JsxChild, self.start..self.position);
        }

        Token::new(TokenKind::JsxChild, self.start..self.position)
    }

    fn make_jsx_attribute_token(&mut self) -> Token {
        let _attr_name = self.read_jsx_identifier();

        match self.chars.peek() {
            Some(&'=') => {
                self.advance();
                match self.chars.peek() {
                    Some(quote @ '"') | Some(quote @ '\'') => {
                        let quote_char = *quote;
                        self.advance();
                        while let Some(&ch) = self.chars.peek() {
                            self.advance();
                            if ch == quote_char {
                                break;
                            }
                        }
                    }
                    _ => {
                        while let Some(&ch) = self.chars.peek() {
                            if ch.is_whitespace() || ch == '>' || ch == '/' {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
                Token::new(TokenKind::JsxAttribute, self.start..self.position)
            }
            _ => Token::new(TokenKind::JsxAttribute, self.start..self.position),
        }
    }

    fn make_jsx_closing_tag_token(&mut self) -> Token {
        // Handles a closing tag like `</div>` (or `</Foo.Bar>`).
        // This can be called when the cursor is at '<' (preferred) or '/' (legacy).
        if self.chars.peek() == Some(&'<') {
            self.advance(); // consume '<'
        }
        if self.chars.peek() == Some(&'/') {
            self.advance(); // consume '/'
        }

        let name = self.read_jsx_identifier();

        // Skip any whitespace before the closing '>'
        while let Some(&ch) = self.chars.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }

        if self.chars.peek() == Some(&'>') {
            self.advance(); // consume '>'
        } else {
            return Token::new(TokenKind::Error, self.start..self.position);
        }

        // Close one JSX level and remain in JSX if we're still nested.
        if self.jsx_depth > 0 {
            self.jsx_depth -= 1;
        }
        self.in_jsx = self.jsx_depth > 0;

        if name.is_empty() {
            Token::new(TokenKind::JsxFragmentClose, self.start..self.position)
        } else {
            Token::new(TokenKind::JsxElementClose, self.start..self.position)
        }
    }

    fn make_jsx_expr_token(&mut self) -> Token {
        self.advance();
        Token::new(TokenKind::OpenBrace, self.start..self.position)
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    Lexer::new(source).tokenize()
}
