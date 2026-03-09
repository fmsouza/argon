//! SafeScript - Borrow Checker Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::BorrowChecker;
use safescript_parser::parse;

mod ownership_tracking {
    use super::*;

    #[test]
    fn tracks_variable_ownership() {
        // Assign
        let source = "const x = 5;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn tracks_multiple_variables() {
        // Assign
        let source = "const a = 1; const b = 2; const c = 3;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn tracks_function_parameters_as_owned() {
        // Assign
        let source = "function foo(x: i32) { const y = x; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod move_detection {
    use super::*;

    #[test]
    fn allows_assignment() {
        // Assign
        let source = "let x = 5; x = 10;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn allows_variable_usage() {
        // Assign
        let source = "const x = 5; const y = x;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod borrow_validation {
    use super::*;

    #[test]
    fn allows_shared_borrow() {
        // Assign
        let source = "const ref = &value;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod function_checking {
    use super::*;

    #[test]
    fn checks_simple_function() {
        // Assign
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_function_with_local_variables() {
        // Assign
        let source = "function foo() { const x = 1; const y = 2; return x + y; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_nested_function_calls() {
        // Assign
        let source =
            "function inner(): i32 { return 42; } function outer(): i32 { return inner(); }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod struct_checking {
    use super::*;

    #[test]
    fn checks_struct_declaration() {
        // Assign
        let source = "struct Point { x: f64; y: f64; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod class_checking {
    use super::*;

    #[test]
    fn checks_class_with_methods() {
        // Assign
        let source = "class Calculator { add(a: i32, b: i32): i32 { return a + b; } }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_class_with_constructor() {
        // Assign
        let source = "class Point { constructor(x: i32, y: i32) { this.x = x; } }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod control_flow_checking {
    use super::*;

    #[test]
    fn checks_if_statement() {
        // Assign
        let source = "if (x > 0) { const y = 1; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_if_else_statement() {
        // Assign
        let source = "if (x > 0) { const y = 1; } else { const z = 2; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_while_loop() {
        // Assign
        let source = "while (i < 10) { i = i + 1; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod expression_checking {
    use super::*;

    #[test]
    fn checks_binary_expression() {
        // Assign
        let source = "const x = 1 + 2;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_function_call_expression() {
        // Assign
        let source = "foo(1, 2, 3);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_member_access() {
        // Assign
        let source = "const x = obj.property;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod return_statement_checking {
    use super::*;

    #[test]
    fn checks_return_with_value() {
        // Assign
        let source = "function foo() { return 42; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_return_without_value() {
        // Assign
        let source = "function foo() { return; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod try_catch_checking {
    use super::*;

    #[test]
    fn checks_try_catch_statement() {
        // Assign
        let source = "try { const x = 1; } catch (e) { const y = 2; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_try_finally_statement() {
        // Assign
        let source = "try { const x = 1; } finally { const y = 2; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod match_statement_checking {
    use super::*;

    #[test]
    fn checks_match_statement() {
        // Assign
        let source = "match (x) { 1 => const a = 1, 2 => const b = 2, }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}
