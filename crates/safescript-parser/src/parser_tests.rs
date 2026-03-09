//! SafeScript - Parser Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::parse;
use safescript_ast::*;

mod variable_statement_parsing {
    use super::*;

    #[test]
    fn parses_const_variable_declaration() {
        // Assign
        let source = "const x = 5;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Variable(var) = &ast.statements[0] {
            assert_eq!(var.kind, VariableKind::Const);
            assert_eq!(var.declarations.len(), 1);
        } else {
            panic!("Expected variable statement");
        }
    }

    #[test]
    fn parses_let_variable_declaration() {
        // Assign
        let source = "let y = 10;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Variable(var) = &ast.statements[0] {
            assert_eq!(var.kind, VariableKind::Let);
        } else {
            panic!("Expected variable statement");
        }
    }

    #[test]
    fn parses_variable_with_type_annotation() {
        // Assign
        let source = "const count: i32 = 0;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_variable_without_initializer() {
        // Assign
        let source = "let x;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod function_parsing {
    use super::*;

    #[test]
    fn parses_function_declaration() {
        // Assign
        let source = "function add(a, b) { return a + b; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Function(func) = &ast.statements[0] {
            assert_eq!(func.id.as_ref().unwrap().sym, "add");
            assert_eq!(func.params.len(), 2);
        } else {
            panic!("Expected function statement");
        }
    }

    #[test]
    fn parses_function_with_return_type() {
        // Assign
        let source = "function greet(): string { return 'hello'; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_function_with_type_annotated_parameters() {
        // Assign
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_empty_function_body() {
        // Assign
        let source = "function empty() { }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_function_with_multiple_statements() {
        // Assign
        let source = "function foo() { const x = 1; const y = 2; return x + y; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod struct_parsing {
    use super::*;

    #[test]
    fn parses_struct_declaration() {
        // Assign
        let source = "struct Point { x: f64; y: f64; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Struct(s) = &ast.statements[0] {
            assert_eq!(s.id.sym, "Point");
            assert_eq!(s.fields.len(), 2);
        } else {
            panic!("Expected struct statement");
        }
    }

    #[test]
    fn parses_struct_with_single_field() {
        // Assign
        let source = "struct Container { value: i32; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_empty_struct() {
        // Assign
        let source = "struct Empty { }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod class_parsing {
    use super::*;

    #[test]
    fn parses_class_declaration() {
        // Assign
        let source = "class Point { x: i32; y: i32; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Class(c) = &ast.statements[0] {
            assert_eq!(c.id.sym, "Point");
        } else {
            panic!("Expected class statement");
        }
    }

    #[test]
    fn parses_class_with_constructor() {
        // Assign
        let source = "class Point { constructor(x: i32, y: i32) { this.x = x; this.y = y; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_class_with_method() {
        // Assign
        let source = "class Calculator { add(a: i32, b: i32): i32 { return a + b; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_class_with_extends() {
        // Assign
        let source = "class Dog extends Animal { }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod control_flow_parsing {
    use super::*;

    #[test]
    fn parses_if_statement() {
        // Assign
        let source = "if (x > 0) { const y = 1; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_if_else_statement() {
        // Assign
        let source = "if (x > 0) { const y = 1; } else { const z = 2; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_while_loop() {
        // Assign
        let source = "while (i < 10) { i = i + 1; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_do_while_loop() {
        // Assign
        let source = "do { x = x + 1; } while (x < 10);";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_switch_statement() {
        // Assign
        let source = "switch (x) { case 1: const a = 1; break; default: const b = 2; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_match_statement() {
        // Assign
        let source = "match (x) { 1 => const a = 1, 2 => const b = 2, }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod try_catch_parsing {
    use super::*;

    #[test]
    fn parses_try_catch_statement() {
        // Assign
        let source = "try { const x = 1; } catch (e) { const y = 2; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_try_catch_finally_statement() {
        // Assign
        let source = "try { const x = 1; } catch (e) { const y = 2; } finally { const z = 3; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_try_finally_statement() {
        // Assign
        let source = "try { const x = 1; } finally { const y = 2; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod return_statement_parsing {
    use super::*;

    #[test]
    fn parses_return_with_value() {
        // Assign
        let source = "function foo() { return 42; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_return_without_value() {
        // Assign
        let source = "function foo() { return; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod import_export_parsing {
    use super::*;

    #[test]
    fn parses_export_named_declaration() {
        // Assign
        let source = "export const x = 1;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod expression_parsing {
    use super::*;

    #[test]
    fn parses_binary_expression() {
        // Assign
        let source = "const x = 1 + 2;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_unary_expression() {
        // Assign
        let source = "const x = -5;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_function_call() {
        // Assign
        let source = "foo(1, 2, 3);";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_method_call() {
        // Assign
        let source = "obj.method();";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_member_access() {
        // Assign
        let source = "const x = obj.property;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_computed_member_access() {
        // Assign
        let source = "const x = arr[0];";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_new_expression() {
        // Assign
        let source = "const obj = new Class();";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod type_annotation_parsing {
    use super::*;

    #[test]
    fn parses_primitive_type_annotation() {
        // Assign
        let source = "const x: number = 5;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_reference_type_annotation() {
        // Assign
        let source = "const x: MyType = value;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod borrow_annotation_parsing {
    use super::*;

    #[test]
    fn parses_method_with_this_borrow() {
        // Assign
        let source = "class Foo { bar(): i32 with &this { return 1; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod multiple_statements_parsing {
    use super::*;

    #[test]
    fn parses_multiple_statements() {
        // Assign
        let source = "const a = 1; const b = 2; const c = 3;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 3);
    }

    #[test]
    fn parses_mixed_declarations() {
        // Assign
        let source = "const x = 1; function foo() { } const y = 2;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn returns_error_for_unmatched_parenthesis() {
        // Assign
        let source = "(1 + 2;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_unmatched_brace() {
        // Assign
        let source = "{ const x = 1;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_err());
    }
}
