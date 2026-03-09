//! SafeScript - JS Codegen Tests

use crate::JsCodegen;
use safescript_parser::parse;

mod statement_codegen {
    use super::*;

    #[test]
    fn generates_variable_statement() {
        let source = "const x = 5;";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("const x = 5"));
    }

    #[test]
    fn generates_function_declaration() {
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("function add"));
    }

    #[test]
    fn generates_struct_declaration() {
        let source = "struct Point { x: f64; y: f64; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("function Point"));
    }

    #[test]
    fn generates_return_statement() {
        let source = "function foo(): i32 { return 42; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("return 42"));
    }

    #[test]
    fn generates_if_statement() {
        let source = "if (x > 0) { return 1; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("if"));
    }

    #[test]
    fn generates_while_statement() {
        let source = "while (true) { x = x + 1; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("while"));
    }

    #[test]
    fn generates_break_statement() {
        let source = "while (true) { break; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("break"));
    }

    #[test]
    fn generates_continue_statement() {
        let source = "while (true) { continue; }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("continue"));
    }
}

mod expression_codegen {
    use super::*;

    #[test]
    fn generates_binary_expression() {
        let source = "const x = 1 + 2;";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("1 + 2"));
    }

    #[test]
    fn generates_unary_expression() {
        let source = "const x = -5;";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("-5"));
    }

    #[test]
    fn generates_function_call() {
        let source = "foo(1, 2);";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("foo(1, 2)"));
    }
}

mod class_codegen {
    use super::*;

    #[test]
    fn generates_class_declaration() {
        let source = "class Point { constructor(x: i32, y: i32) { this.x = x; } }";
        let ast = parse(source).unwrap();
        let mut codegen = JsCodegen::new();

        let result = codegen.generate_from_ast(&ast);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("function Point"));
    }
}
