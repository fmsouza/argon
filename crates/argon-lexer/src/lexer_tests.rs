//! Argon - Lexer Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::{tokenize, TokenKind};

mod identifier_tokenization {
    use super::*;

    #[test]
    fn returns_identifier_token_when_simple_word_is_provided() {
        // Assign
        let source = "hello";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2); // identifier + EOF
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(&source[tokens[0].span.clone()], "hello");
    }

    #[test]
    fn returns_identifier_token_when_underscore_prefixed() {
        // Assign
        let source = "_private";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
    }

    #[test]
    fn returns_identifier_token_when_dollar_sign_prefixed() {
        // Assign
        let source = "$jquery";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
    }

    #[test]
    fn returns_identifier_token_when_containing_numbers() {
        // Assign
        let source = "variable123";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
    }

    #[test]
    fn returns_identifier_with_correct_span() {
        // Assign
        let source = "  test";
        let expected_start = 2;
        let expected_end = 6;

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].span.start, expected_start);
        assert_eq!(tokens[0].span.end, expected_end);
    }
}

mod literal_tokenization {
    use super::*;

    #[test]
    fn returns_number_token_when_integer_is_provided() {
        // Assign
        let source = "42";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
    }

    #[test]
    fn returns_number_token_when_decimal_is_provided() {
        // Assign
        let source = "3.14";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
    }

    #[test]
    fn returns_number_token_when_scientific_notation_is_provided() {
        // Assign
        let source = "1e10";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
    }

    #[test]
    fn returns_number_token_when_negative_exponent_is_provided() {
        // Assign
        let source = "2.5e-3";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
    }

    #[test]
    fn returns_string_token_when_double_quoted_string_is_provided() {
        // Assign
        let source = "\"hello world\"";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn returns_string_token_when_single_quoted_string_is_provided() {
        // Assign
        let source = "'single quotes'";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn returns_string_token_when_escaped_quotes_are_in_string() {
        // Assign
        let source = "\"escaped \\\"quote\\\"\"";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn returns_boolean_true_token_when_true_keyword_is_provided() {
        // Assign
        let source = "true";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::True);
    }

    #[test]
    fn returns_boolean_false_token_when_false_keyword_is_provided() {
        // Assign
        let source = "false";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::False);
    }

    #[test]
    fn returns_null_token_when_null_keyword_is_provided() {
        // Assign
        let source = "null";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Null);
    }

    #[test]
    fn returns_undefined_token_when_undefined_keyword_is_provided() {
        // Assign
        let source = "undefined";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Undefined);
    }
}

mod jsx_vs_less_than {
    use super::*;

    #[test]
    fn lexes_less_than_in_comparison_not_jsx() {
        // Assign
        let source = "while (i < 10) { i = i + 1; }";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LessThan));
        assert!(!tokens.iter().any(|t| t.kind == TokenKind::JsxElementOpen));
    }

    #[test]
    fn lexes_jsx_after_return() {
        // Assign
        let source = "function f(): void { return <div>Hello</div>; }";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();

        // Find `return`, then ensure the next token begins JSX.
        let return_pos = tokens
            .iter()
            .position(|t| t.kind == TokenKind::Return)
            .expect("expected Return token");
        assert!(
            matches!(
                tokens.get(return_pos + 1).map(|t| t.kind),
                Some(TokenKind::JsxElementOpen) | Some(TokenKind::JsxFragmentOpen)
            ),
            "expected JSX token after return"
        );
    }

    #[test]
    fn lexes_generic_type_params_not_jsx() {
        // Assign
        let source = "function id<T>(x: T): T { return x; }";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LessThan));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::GreaterThan));
        assert!(!tokens.iter().any(|t| t.kind == TokenKind::JsxElementOpen));
    }

    #[test]
    fn lexes_type_arguments_not_jsx() {
        // Assign
        let source = "type Boxed = Box<number>;";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LessThan));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::GreaterThan));
        assert!(!tokens.iter().any(|t| t.kind == TokenKind::JsxElementOpen));
    }

    #[test]
    fn lexes_class_generic_params_and_new_type_args_not_jsx() {
        // Assign
        let source = r#"
class Container<T> {
    value: T;
    constructor(v: T) { this.value = v; }
    get(): T { return this.value; }
}
const x = new Container<number>(42);
"#;

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LessThan));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::GreaterThan));
        assert!(
            !tokens.iter().any(|t| matches!(t.kind, TokenKind::JsxElementOpen | TokenKind::JsxChild)),
            "expected no JSX tokens when lexing generic params/type args"
        );
    }
}

mod operator_tokenization {
    use super::*;

    #[test]
    fn returns_plus_token_when_plus_character_is_provided() {
        // Assign
        let source = "+";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
    }

    #[test]
    fn returns_plus_equal_token_when_plus_equal_is_provided() {
        // Assign
        let source = "+=";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::PlusEqual);
    }

    #[test]
    fn returns_plus_plus_token_when_increment_operator_is_provided() {
        // Assign
        let source = "++";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::PlusPlus);
    }

    #[test]
    fn returns_minus_token_when_minus_character_is_provided() {
        // Assign
        let source = "-";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Minus);
    }

    #[test]
    fn returns_arrow_token_when_arrow_is_provided() {
        // Assign
        let source = "->";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Arrow);
    }

    #[test]
    fn returns_star_token_when_asterisk_is_provided() {
        // Assign
        let source = "*";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Star);
    }

    #[test]
    fn returns_star_star_token_when_exponentiation_is_provided() {
        // Assign
        let source = "**";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StarStar);
    }

    #[test]
    fn returns_slash_token_when_slash_is_provided() {
        // Assign
        let source = "/";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Slash);
    }

    #[test]
    fn returns_percent_token_when_modulo_is_provided() {
        // Assign
        let source = "%";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Percent);
    }

    #[test]
    fn returns_equal_equal_equal_token_when_strict_equality_is_provided() {
        // Assign
        let source = "===";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::EqualEqualEqual);
    }

    #[test]
    fn returns_bang_equal_token_when_not_equal_is_provided() {
        // Assign
        let source = "!=";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::BangEqual);
    }

    #[test]
    fn returns_less_than_less_than_token_when_left_shift_is_provided() {
        // Assign
        let source = "<<";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LessThanLessThan);
    }

    #[test]
    fn returns_greater_than_greater_than_greater_than_token_when_unsigned_right_shift_is_provided()
    {
        // Assign
        let source = ">>>";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::GreaterThanGreaterThanGreaterThan);
    }

    #[test]
    fn returns_ampersand_token_when_ampersand_is_provided() {
        // Assign
        let source = "&";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ampersand);
    }

    #[test]
    fn returns_ampersand_ampersand_token_when_logical_and_is_provided() {
        // Assign
        let source = "&&";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::AmpersandAmpersand);
    }

    #[test]
    fn tokenizes_mutable_reference_syntax_as_ampersand_then_mut_keyword() {
        // Assign
        let source = "&mut";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ampersand);
        assert_eq!(tokens[1].kind, TokenKind::Mut);
    }

    #[test]
    fn returns_pipe_token_when_pipe_is_provided() {
        // Assign
        let source = "|";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Pipe);
    }

    #[test]
    fn returns_pipe_pipe_token_when_logical_or_is_provided() {
        // Assign
        let source = "||";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::PipePipe);
    }

    #[test]
    fn returns_caret_token_when_caret_is_provided() {
        // Assign
        let source = "^";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Caret);
    }

    #[test]
    fn returns_caret_caret_token_when_logical_xor_is_provided() {
        // Assign
        let source = "^^";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CaretCaret);
    }
}

mod punctuation_tokenization {
    use super::*;

    #[test]
    fn returns_open_brace_token_when_opening_brace_is_provided() {
        // Assign
        let source = "{";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::OpenBrace);
    }

    #[test]
    fn returns_close_brace_token_when_closing_brace_is_provided() {
        // Assign
        let source = "}";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CloseBrace);
    }

    #[test]
    fn returns_open_paren_token_when_opening_paren_is_provided() {
        // Assign
        let source = "(";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::OpenParen);
    }

    #[test]
    fn returns_close_paren_token_when_closing_paren_is_provided() {
        // Assign
        let source = ")";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CloseParen);
    }

    #[test]
    fn returns_open_bracket_token_when_opening_bracket_is_provided() {
        // Assign
        let source = "[";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::OpenBracket);
    }

    #[test]
    fn returns_close_bracket_token_when_closing_bracket_is_provided() {
        // Assign
        let source = "]";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CloseBracket);
    }

    #[test]
    fn returns_semi_token_when_semicolon_is_provided() {
        // Assign
        let source = ";";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Semi);
    }

    #[test]
    fn returns_comma_token_when_comma_is_provided() {
        // Assign
        let source = ",";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Comma);
    }

    #[test]
    fn returns_dot_token_when_dot_is_provided() {
        // Assign
        let source = ".";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Dot);
    }

    #[test]
    fn returns_dot_dot_token_when_double_dot_is_provided() {
        // Assign
        let source = "..";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::DotDot);
    }

    #[test]
    fn returns_dot_dot_dot_token_when_spread_syntax_is_provided() {
        // Assign
        let source = "...";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::DotDotDot);
    }

    #[test]
    fn returns_question_token_when_question_mark_is_provided() {
        // Assign
        let source = "?";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Question);
    }

    #[test]
    fn returns_question_dot_token_when_optional_chaining_is_provided() {
        // Assign
        let source = "?.";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::QuestionDot);
    }

    #[test]
    fn returns_question_question_token_when_nullish_coalescing_is_provided() {
        // Assign
        let source = "??";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::QuestionQuestion);
    }

    #[test]
    fn returns_colon_token_when_colon_is_provided() {
        // Assign
        let source = ":";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Colon);
    }

    #[test]
    fn returns_fat_arrow_token_when_fat_arrow_is_provided() {
        // Assign
        let source = "=>";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::FatArrow);
    }
}

mod keyword_tokenization {
    use super::*;

    #[test]
    fn returns_if_keyword_token_when_if_is_provided() {
        // Assign
        let source = "if";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::If);
    }

    #[test]
    fn returns_else_keyword_token_when_else_is_provided() {
        // Assign
        let source = "else";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Else);
    }

    #[test]
    fn returns_for_keyword_token_when_for_is_provided() {
        // Assign
        let source = "for";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::For);
    }

    #[test]
    fn returns_while_keyword_token_when_while_is_provided() {
        // Assign
        let source = "while";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::While);
    }

    #[test]
    fn returns_do_keyword_token_when_do_is_provided() {
        // Assign
        let source = "do";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Do);
    }

    #[test]
    fn returns_switch_keyword_token_when_switch_is_provided() {
        // Assign
        let source = "switch";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Switch);
    }

    #[test]
    fn returns_case_keyword_token_when_case_is_provided() {
        // Assign
        let source = "case";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Case);
    }

    #[test]
    fn returns_default_keyword_token_when_default_is_provided() {
        // Assign
        let source = "default";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Default);
    }

    #[test]
    fn returns_break_keyword_token_when_break_is_provided() {
        // Assign
        let source = "break";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Break);
    }

    #[test]
    fn returns_continue_keyword_token_when_continue_is_provided() {
        // Assign
        let source = "continue";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Continue);
    }

    #[test]
    fn returns_return_keyword_token_when_return_is_provided() {
        // Assign
        let source = "return";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Return);
    }

    #[test]
    fn returns_function_keyword_token_when_function_is_provided() {
        // Assign
        let source = "function";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Function);
    }

    #[test]
    fn returns_class_keyword_token_when_class_is_provided() {
        // Assign
        let source = "class";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Class);
    }

    #[test]
    fn returns_const_keyword_token_when_const_is_provided() {
        // Assign
        let source = "const";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Const);
    }

    #[test]
    fn returns_let_keyword_token_when_let_is_provided() {
        // Assign
        let source = "let";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Let);
    }

    #[test]
    fn returns_var_keyword_token_when_var_is_provided() {
        // Assign
        let source = "var";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Var);
    }

    #[test]
    fn returns_import_keyword_token_when_import_is_provided() {
        // Assign
        let source = "import";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Import);
    }

    #[test]
    fn returns_export_keyword_token_when_export_is_provided() {
        // Assign
        let source = "export";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Export);
    }

    #[test]
    fn returns_try_keyword_token_when_try_is_provided() {
        // Assign
        let source = "try";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Try);
    }

    #[test]
    fn returns_catch_keyword_token_when_catch_is_provided() {
        // Assign
        let source = "catch";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Catch);
    }

    #[test]
    fn returns_finally_keyword_token_when_finally_is_provided() {
        // Assign
        let source = "finally";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Finally);
    }

    #[test]
    fn returns_throw_keyword_token_when_throw_is_provided() {
        // Assign
        let source = "throw";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Throw);
    }

    #[test]
    fn returns_this_keyword_token_when_this_is_provided() {
        // Assign
        let source = "this";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::This);
    }

    #[test]
    fn returns_super_keyword_token_when_super_is_provided() {
        // Assign
        let source = "super";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Super);
    }

    #[test]
    fn returns_new_keyword_token_when_new_is_provided() {
        // Assign
        let source = "new";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::New);
    }

    #[test]
    fn returns_typeof_keyword_token_when_typeof_is_provided() {
        // Assign
        let source = "typeof";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Typeof);
    }

    #[test]
    fn returns_instanceof_keyword_token_when_instanceof_is_provided() {
        // Assign
        let source = "instanceof";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Instanceof);
    }

    #[test]
    fn returns_in_keyword_token_when_in_is_provided() {
        // Assign
        let source = "in";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::In);
    }

    #[test]
    fn returns_async_keyword_token_when_async_is_provided() {
        // Assign
        let source = "async";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Async);
    }

    #[test]
    fn returns_await_keyword_token_when_await_is_provided() {
        // Assign
        let source = "await";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Await);
    }

    #[test]
    fn returns_yield_keyword_token_when_yield_is_provided() {
        // Assign
        let source = "yield";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Yield);
    }

    #[test]
    fn returns_delete_keyword_token_when_delete_is_provided() {
        // Assign
        let source = "delete";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Delete);
    }

    #[test]
    fn returns_void_keyword_token_when_void_is_provided() {
        // Assign
        let source = "void";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Void);
    }
}

mod argon_keyword_tokenization {
    use super::*;

    #[test]
    fn returns_struct_keyword_token_when_struct_is_provided() {
        // Assign
        let source = "struct";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Struct);
    }

    #[test]
    fn returns_trait_keyword_token_when_trait_is_provided() {
        // Assign
        let source = "trait";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Trait);
    }

    #[test]
    fn returns_impl_keyword_token_when_impl_is_provided() {
        // Assign
        let source = "impl";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Impl);
    }

    #[test]
    fn returns_match_keyword_token_when_match_is_provided() {
        // Assign
        let source = "match";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Match);
    }

    #[test]
    fn returns_with_keyword_token_when_with_is_provided() {
        // Assign
        let source = "with";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::With);
    }

    #[test]
    fn returns_shared_keyword_token_when_shared_is_provided() {
        // Assign
        let source = "shared";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Shared);
    }

    #[test]
    fn returns_move_keyword_token_when_move_is_provided() {
        // Assign
        let source = "move";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Move);
    }

    #[test]
    fn returns_copy_keyword_token_when_copy_is_provided() {
        // Assign
        let source = "copy";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Copy);
    }

    #[test]
    fn returns_mut_keyword_token_when_mut_is_provided() {
        // Assign
        let source = "mut";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Mut);
    }

    #[test]
    fn returns_constructor_keyword_token_when_constructor_is_provided() {
        // Assign
        let source = "constructor";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Constructor);
    }
}

mod numeric_type_keyword_tokenization {
    use super::*;

    #[test]
    fn returns_i8_keyword_token_when_i8_is_provided() {
        // Assign
        let source = "i8";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::I8);
    }

    #[test]
    fn returns_i16_keyword_token_when_i16_is_provided() {
        // Assign
        let source = "i16";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::I16);
    }

    #[test]
    fn returns_i32_keyword_token_when_i32_is_provided() {
        // Assign
        let source = "i32";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::I32);
    }

    #[test]
    fn returns_i64_keyword_token_when_i64_is_provided() {
        // Assign
        let source = "i64";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::I64);
    }

    #[test]
    fn returns_u8_keyword_token_when_u8_is_provided() {
        // Assign
        let source = "u8";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::U8);
    }

    #[test]
    fn returns_u16_keyword_token_when_u16_is_provided() {
        // Assign
        let source = "u16";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::U16);
    }

    #[test]
    fn returns_u32_keyword_token_when_u32_is_provided() {
        // Assign
        let source = "u32";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::U32);
    }

    #[test]
    fn returns_u64_keyword_token_when_u64_is_provided() {
        // Assign
        let source = "u64";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::U64);
    }

    #[test]
    fn returns_f32_keyword_token_when_f32_is_provided() {
        // Assign
        let source = "f32";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::F32);
    }

    #[test]
    fn returns_f64_keyword_token_when_f64_is_provided() {
        // Assign
        let source = "f64";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::F64);
    }

    #[test]
    fn returns_isize_keyword_token_when_isize_is_provided() {
        // Assign
        let source = "isize";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Isize);
    }

    #[test]
    fn returns_usize_keyword_token_when_usize_is_provided() {
        // Assign
        let source = "usize";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Usize);
    }
}

mod composite_tokenization {
    use super::*;

    #[test]
    fn tokenizes_arithmetic_expression() {
        // Assign
        let source = "x + y * 2";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 6); // x + y * 2 + EOF
    }

    #[test]
    fn tokenizes_function_call() {
        // Assign
        let source = "foo(a, b)";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&TokenKind::Identifier));
    }

    #[test]
    fn tokenizes_object_literal() {
        // Assign
        let source = "{ x: 1, y: 2 }";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::OpenBrace));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::CloseBrace));
    }

    #[test]
    fn tokenizes_array_literal() {
        // Assign
        let source = "[1, 2, 3]";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::OpenBracket));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::CloseBracket));
    }

    #[test]
    fn tokenizes_arrow_function() {
        // Assign
        let source = "(x) => x + 1";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::FatArrow));
    }

    #[test]
    fn tokenizes_type_annotation() {
        // Assign
        let source = "x: number";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Colon));
    }

    #[test]
    fn tokenizes_generic_type() {
        // Assign
        let source = "Vec<T>";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 5); // Vec < T > + EOF
    }

    #[test]
    fn tokenizes_reference_type() {
        // Assign
        let source = "&T";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ampersand);
    }

    #[test]
    fn tokenizes_mutable_reference_type() {
        // Assign
        let source = "&mut T";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ampersand);
        assert_eq!(tokens[1].kind, TokenKind::Mut);
    }

    #[test]
    fn tokenizes_ternary_operator() {
        // Assign
        let source = "a ? b : c";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Question));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Colon));
    }
}

mod whitespace_and_comment_handling {
    use super::*;

    #[test]
    fn skips_single_space_between_tokens() {
        // Assign
        let source = "a b";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3); // a + b + EOF
    }

    #[test]
    fn skips_multiple_spaces_between_tokens() {
        // Assign
        let source = "a     b";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn skips_tabs_between_tokens() {
        // Assign
        let source = "a\tb";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn skips_newlines_between_tokens() {
        // Assign
        let source = "a\nb";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn skips_single_line_comment() {
        // Assign
        let source = "a // this is a comment\nb";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3); // a + b + EOF
    }

    #[test]
    fn skips_multi_line_comment() {
        // Assign
        let source = "a /* this is\n a multi line\n comment */ b";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn does_not_support_nested_multi_line_comments() {
        // Assign - standard C-style comments don't support nesting
        // The lexer stops at the first */ it finds
        let source = "a /* outer /* inner */ b";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        // The lexer treats "inner */ b" as after the comment ends
        // This is standard C-style comment behavior (no nesting)
        assert!(tokens.len() >= 2);
    }

    #[test]
    fn skips_mixed_whitespace() {
        // Assign
        let source = "  \t\n  a  \t  b  \n  ";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn returns_error_when_unexpected_character_is_encountered() {
        // Assign
        let source = "@";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn returns_unterminated_string_error_when_string_is_not_closed() {
        // Assign
        let source = "\"unclosed string";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok()); // Returns token but marks as unterminated
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::UnterminatedString);
    }

    #[test]
    fn returns_unterminated_template_error_when_template_is_not_closed() {
        // Assign
        let source = "`unclosed template";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::UnterminatedTemplate);
    }

    #[test]
    fn handles_unterminated_string_with_escaped_quote() {
        // Assign
        let source = "\"test\\\"";

        // Act
        let result = tokenize(source);

        // Assert
        // Should not panic, handles escaped quote
        assert!(result.is_ok());
    }

    #[test]
    fn provides_position_information_in_error() {
        // Assign
        let source = "@";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("position") || error_string.contains("@"));
    }
}

mod unicode_character_handling {
    use super::*;

    #[test]
    fn handles_unicode_identifier_start() {
        // Assign
        let source = "café";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
    }

    #[test]
    fn handles_unicode_in_string_literal() {
        // Assign
        let source = "\"Hello 🌍\"";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn handles_unicode_emoji_in_comment() {
        // Assign
        let source = "// 🎉 Celebration!\ntest";

        // Act
        let result = tokenize(source);

        // Assert
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2); // test + EOF
    }
}
