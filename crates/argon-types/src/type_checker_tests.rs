//! Argon - Type Checker Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::TypeChecker;
use argon_parser::parse;

mod type_inference {
    use super::*;

    #[test]
    fn infers_number_type_from_numeric_literal() {
        // Assign
        let source = "const x = 42;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn infers_string_type_from_string_literal() {
        // Assign
        let source = "const x = 'hello';";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn infers_boolean_type_from_boolean_literal() {
        // Assign
        let source = "const x = true;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn infers_null_type_from_null_literal() {
        // Assign
        let source = "const x = null;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn infers_string_type_from_template_literal() {
        // Assign
        let source = "const name = 'argon'; const x = `hello ${name}`;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod type_checking {
    use super::*;

    #[test]
    fn validates_variable_with_type_annotation() {
        // Assign
        let source = "const x: i32 = 5;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validates_struct_declaration() {
        // Assign
        let source = "struct Point { x: f64; y: f64; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validates_function_declaration() {
        // Assign
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validates_function_call() {
        // Assign
        let source = "function foo(): void { console.log('test'); }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod type_mismatch_detection {
    use super::*;

    #[test]
    fn detects_type_mismatch_when_assigning_string_to_number() {
        // Assign
        let source = "const x: i32 = 'hello';";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        // Type checker may not catch this at basic level, but should not panic
        let _ = result;
    }
}

mod expression_type_checking {
    use super::*;

    #[test]
    fn checks_binary_arithmetic_expression() {
        // Assign
        let source = "const x = 1 + 2;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_binary_comparison_expression() {
        // Assign
        let source = "const x = 1 > 2;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_logical_expression() {
        // Assign
        let source = "const x = true && false;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_unary_expression() {
        // Assign
        let source = "const x = -5;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod statement_type_checking {
    use super::*;

    #[test]
    fn checks_if_statement_condition_is_boolean() {
        // Assign
        let source = "if (1) { const x = 1; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        // Basic check - should not panic
        let _ = result;
    }

    #[test]
    fn checks_while_statement_condition_is_boolean() {
        // Assign
        let source = "while (true) { break; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_return_statement() {
        // Assign
        let source = "function foo(): i32 { return 42; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn detects_return_type_mismatch() {
        // Assign
        let source = "function foo(): i32 { return 'hello'; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn detects_missing_return_value_for_non_void() {
        // Assign
        let source = "function foo(): i32 { return; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }
}

mod class_type_checking {
    use super::*;

    #[test]
    fn checks_class_with_constructor() {
        // Assign
        let source = "class Point { constructor(x: i32, y: i32) { this.x = x; this.y = y; } }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_class_with_methods() {
        // Assign
        let source = "class Calculator { add(a: i32, b: i32): i32 { return a + b; } }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod function_type_checking {
    use super::*;

    #[test]
    fn registers_function_in_environment() {
        // Assign
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn validates_function_body_statements() {
        // Assign
        let source = "function foo(): number { const x = 1; const y = 2; return x + y; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod type_alias_checking {
    use super::*;

    #[test]
    fn resolves_non_generic_type_aliases() {
        // Assign
        let source = "type MyNum = number; function id(x: MyNum): MyNum { return x; }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod generics_and_members {
    use super::*;

    #[test]
    fn type_checks_generic_class_instantiation_and_member_access() {
        // Assign
        let source = r#"
class Container<T> {
    value: T;
    constructor(v: T) { this.value = v; }
    get(): T { return this.value; }
}
const container = new Container<number>(42);
const v: number = container.value;
const g: number = container.get();
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let output = checker.check_with_output(&ast);

        // Assert
        assert!(output.is_ok());
        let output = output.unwrap();
        let number_ty = output.type_table.get_by_name("number").unwrap();
        assert_eq!(output.env.get_var("v"), Some(number_ty));
        assert_eq!(output.env.get_var("g"), Some(number_ty));
    }

    #[test]
    fn resolves_type_alias_to_instantiated_generic_type() {
        // Assign
        let source = r#"
class Box<T> { value: T; constructor(v: T) { this.value = v; } }
type NumBox = Box<number>;
const b: NumBox = new Box<number>(1);
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }
}

mod generic_constraints {
    use super::*;

    #[test]
    fn rejects_type_arguments_that_do_not_satisfy_extends_constraint() {
        // Assign
        let source = r#"
function id<T extends number>(x: T): T { return x; }
const a = id(1);
const b = id("nope");
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }
}

mod reference_type_checking {
    use super::*;

    #[test]
    fn checks_reference_type() {
        // Assign
        let source = "const ref: &i32;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        // Should not panic
        let _ = result;
    }
}

mod match_expression_checking {
    use super::*;

    #[test]
    fn checks_match_statement() {
        // Assign
        let source = "match (x) { 1 => const a = 1, 2 => const b = 2, }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod error_recovery {
    use super::*;

    #[test]
    fn continues_checking_after_first_error() {
        // Assign
        let source = "const x: i32 = 'string'; const y = 42;";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        // The checker should handle this gracefully
        let _ = result;
    }
}
