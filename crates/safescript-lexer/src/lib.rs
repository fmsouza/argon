//! SafeScript - Lexer

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
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            position: 0,
            start: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            if token.kind != TokenKind::Whitespace && token.kind != TokenKind::Comment {
                tokens.push(token);
            }
        }

        tokens.push(Token::new(TokenKind::Eof, self.position..self.position + 1));

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace_and_comments();

        self.start = self.position;

        if let Some(&ch) = self.chars.peek() {
            let token = match ch {
                // Punctuation
                '{' => self.make_token(TokenKind::OpenBrace),
                '}' => self.make_token(TokenKind::CloseBrace),
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
                '<' => self.make_less_than_token(),
                '>' => self.make_greater_than_token(),

                // Literals
                '"' | '\'' => self.make_string_token(ch),
                '0'..='9' => self.make_number_token(),

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' | '$' => self.make_identifier_or_keyword_token(),

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
            Some(&'m') => {
                if self.peek_n(1) == Some('u') && self.peek_n(2) == Some('t') {
                    self.advance();
                    self.advance();
                    self.advance();
                    return Token::new(TokenKind::AmpersandMut, self.start..self.position + 1);
                }
                Token::new(TokenKind::Ampersand, self.start..self.position + 1)
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
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == quote {
                return Token::new(TokenKind::String, self.start..self.position + 1);
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

        Token::new(TokenKind::Number, self.start..self.position)
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
                    return Token::new(TokenKind::TemplateMiddle, self.start..self.position + 1);
                }
                return Token::new(TokenKind::TemplateComplete, self.start..self.position + 1);
            }

            if ch == '{' && !has_interpolation {
                has_interpolation = true;
                brace_count += 1;
            } else if ch == '}' && has_interpolation {
                brace_count -= 1;
            }

            if ch == '\n' && brace_count == 0 {
                return Token::new(TokenKind::UnterminatedTemplate, self.start..self.position);
            }
        }

        Token::new(TokenKind::UnterminatedTemplate, self.start..self.position)
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    Lexer::new(source).tokenize()
}
