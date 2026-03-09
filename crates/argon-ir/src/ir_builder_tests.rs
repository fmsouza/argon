//! Argon - IR Builder Tests

use crate::IrBuilder;
use argon_parser::parse;

mod function_translation {
    use super::*;

    #[test]
    fn translates_simple_function() {
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].id, "add");
    }

    #[test]
    fn translates_function_with_variables() {
        let source = "function foo(): i32 { const x = 1; const y = 2; return x + y; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_multiple_functions() {
        let source = "function foo(): i32 { return 1; } function bar(): i32 { return 2; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.functions.len(), 2);
    }
}

mod struct_translation {
    use super::*;

    #[test]
    fn translates_struct() {
        let source = "struct Point { x: f64; y: f64; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.types.len(), 1);
    }
}

mod expression_translation {
    use super::*;

    #[test]
    fn translates_number_literal() {
        let source = "function foo(): i32 { return 42; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_string_literal() {
        let source = "function foo(): string { return \"hello\"; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_boolean_literals() {
        let source = "function foo(): boolean { return true; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_binary_expression() {
        let source = "function foo(): i32 { return 1 + 2; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_unary_expression() {
        let source = "function foo(): i32 { return -5; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_function_call() {
        let source = "function bar(): i32 { return 1; } function foo(): i32 { return bar(); }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }
}

mod variable_translation {
    use super::*;

    #[test]
    fn translates_const_variable() {
        let source = "const x = 5;";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }

    #[test]
    fn translates_let_variable() {
        let source = "let y = 10;";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let result = builder.build(&ast);

        assert!(result.is_ok());
    }
}
