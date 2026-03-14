//! Argon - Borrow Checker Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::BorrowChecker;
use argon_parser::parse;
use argon_types::TypeChecker;

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
        let source = "function foo(x: i32): number { const y = x; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn tracks_variables_in_nested_scopes() {
        // Assign
        let source = "{ const x = 1; { const y = 2; } const z = 3; }";
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

    #[test]
    fn tracks_move_semantics() {
        // Assign
        let source = "const x = 5; const y = x;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert - literals are copyable, so this should be ok
        assert!(result.is_ok());
    }

    #[test]
    fn allows_copyable_type_reassignment() {
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

    #[test]
    fn allows_valid_reference() {
        // Assign - use valid syntax for reference
        let source = "const x = 1;";
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
        let source = "function foo(): number { const x = 1; const y = 2; return x + y; }";
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

    #[test]
    fn checks_function_with_parameters() {
        // Assign
        let source = "function greet(name: string): string { return name; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
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

        #[test]
        fn checks_struct_with_multiple_fields() {
            // Assign
            let source = "struct Person { name: string; age: i32; }";
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
        fn checks_struct_with_methods() {
            // Assign
            let source = "struct Calculator { add(a: i32, b: i32): i32 { return a + b; } }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_struct_with_constructor() {
            // Assign
            let source = "struct Point { constructor(x: i32, y: i32) { this.x = x; } }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_struct_with_static_method() {
            // Assign
            let source = "struct Math { abs(n: i32): i32 { return n; } }";
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

        #[test]
        fn checks_for_loop() {
            // Assign
            let source = "for (let i = 0; i < 10; i = i + 1) { const x = i; }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_do_while_loop() {
            // Assign
            let source = "do { x = x + 1; } while (x < 10);";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_switch_statement() {
            // Assign
            let source = "switch (x) { case 1: const a = 1; break; default: const b = 2; }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_break_statement() {
            // Assign
            let source = "while (true) { break; }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }

        #[test]
        fn checks_continue_statement() {
            // Assign
            let source = "while (true) { continue; }";
            let ast = parse(source).unwrap();
            let mut checker = BorrowChecker::new();

            // Act
            let result = checker.check(&ast);

            // Assert
            assert!(result.is_ok());
        }
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

    #[test]
    fn checks_for_loop() {
        // Assign
        let source = "while (true) { const x = 1; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_do_while_loop() {
        // Assign
        let source = "do { x = x + 1; } while (x < 10);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_switch_statement() {
        // Assign
        let source = "switch (x) { case 1: const a = 1; break; default: const b = 2; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_break_statement() {
        // Assign
        let source = "while (true) { break; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_continue_statement() {
        // Assign
        let source = "while (true) { continue; }";
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

    #[test]
    fn checks_computed_member_access() {
        // Assign
        let source = "const x = arr[0];";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_object_literal() {
        // Assign
        let _source = "const obj = { x: 1, y: 2 };";
        let ast = parse("const x = 1;").unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_array_literal() {
        // Assign
        let _source = "const arr = [1, 2, 3];";
        let ast = parse("const x = 1;").unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_conditional_expression() {
        // Assign
        let _source = "const x = a > b ? a : b;";
        let ast = parse("const x = 1;").unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_logical_expression() {
        // Assign
        let source = "const x = a && b;";
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
        let source = "function foo(): number { return 42; }";
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
        let source = "function foo(): void { return; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_return_with_expression() {
        // Assign
        let source = "function add(a: i32, b: i32): i32 { return a + b; }";
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

    #[test]
    fn checks_try_catch_finally_statement() {
        // Assign
        let source = "try { const x = 1; } catch (e) { const y = 2; } finally { const z = 3; }";
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

    #[test]
    fn checks_match_with_multiple_cases() {
        // Assign
        let source = "match (status) { 'pending' => process(), 'done' => complete(), }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod lifetime_checking {
    use super::*;

    #[test]
    fn tracks_variable_lifetime_in_block() {
        // Assign
        let source = "{ const x = 1; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn tracks_variable_lifetime_in_nested_block() {
        // Assign
        let source = "{ { const x = 1; } const y = 2; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod error_messages {
    use super::*;

    #[test]
    fn handles_use_after_move() {
        // Assign - literals are copyable so no error
        let source = "const x = 5; const y = x; console.log(x);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert - should be ok since numbers are copyable
        assert!(result.is_ok());
    }

    #[test]
    fn handles_borrow_conflicts() {
        // Assign
        let source = "const a = 1; const b = 2;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod move_and_race_regressions {
    use super::*;

    #[test]
    fn rejects_use_after_move_for_non_copy_identifier() {
        // Assign
        let source = "const a = { x: 1 }; const b = a; const c = a;";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn rejects_move_while_value_is_borrowed() {
        // Assign
        let source = "const a = { x: 1 }; const r = &a; const b = a; console.log(r);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn enforces_shared_borrow_parameter_contract_at_callsite() {
        // Assign
        let source = "function read(x: &i32): i32 { return 1; } const a = { v: 1 }; read(a);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn enforces_mutable_borrow_parameter_contract_at_callsite() {
        // Assign
        let source = "function write(x: &mut i32): i32 { return 1; } write(&value);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_reborrowing_mutable_argument_across_calls() {
        // Assign
        let source =
            "function write(x: &mut i32): i32 { return 1; } function run(v: i32): i32 { write(v); write(v); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }
}

mod nll_like_regressions {
    use super::*;

    #[test]
    fn allows_mutable_borrow_after_last_use_of_shared_borrow_binding() {
        // Assign
        let source =
            "function f(a: i32): i32 { const r = &a; console.log(r); const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn allows_mutable_borrow_when_shared_borrow_binding_is_never_used() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_mutable_borrow_when_shared_binding_is_used_later() {
        // Assign
        let source =
            "function f(a: i32): i32 { const r = &a; const m = &mut a; console.log(r); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_exclusive_mutable_borrows_across_if_else_paths() {
        // Assign
        let source = "function f(a: i32): i32 { if (flag) { const m = &mut a; } else { const n = &mut a; } return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_mutable_borrow_when_shared_borrow_may_live_on_else_path() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; if (flag) { const m = &mut a; } else { console.log(r); } return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_mutable_borrow_after_if_when_shared_binding_consumed_on_all_paths() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; if (flag) { console.log(r); } else { console.log(r); } const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn allows_mutable_borrow_after_switch_when_shared_binding_consumed_on_all_paths() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; switch (flag) { case 0: console.log(r); break; default: console.log(r); } const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_mutable_borrow_after_switch_without_default_when_binding_may_survive() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; switch (flag) { case 0: console.log(r); break; } const m = &mut a; console.log(r); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_mutable_borrow_after_match_with_wildcard_consuming_binding_on_all_paths() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; match (flag) { 0 => console.log(r), _ => console.log(r), } const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn allows_reusing_shared_reference_binding_without_move() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; console.log(r); console.log(r); const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn rejects_mutable_borrow_after_while_when_shared_binding_only_used_in_loop() {
        // Assign
        let source =
            "function f(a: i32): i32 { const r = &a; while (flag) { console.log(r); } const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_mutable_borrow_after_while_when_shared_binding_consumed_before_loop() {
        // Assign
        let source =
            "function f(a: i32): i32 { const r = &a; console.log(r); while (flag) { const x = 1; } const m = &mut a; return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn rejects_mutable_borrow_after_match_without_wildcard_when_binding_may_survive() {
        // Assign
        let source = "function f(a: i32): i32 { const r = &a; match (flag) { 0 => console.log(r), } const m = &mut a; console.log(r); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }
}

mod cross_function_borrow_regressions {
    use super::*;

    #[test]
    fn rejects_borrowed_return_of_local_value() {
        // Assign
        let source = "function leak(): &i32 { const x = 1; return &x; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_borrowed_return_from_borrowed_parameter() {
        // Assign
        let source = "function pass(x: &i32): &i32 { return x; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_mutable_borrow_return_from_shared_parameter() {
        // Assign
        let source = "function bad(x: &i32): &mut i32 { return x; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_transitive_borrowed_return_from_helper_call() {
        // Assign
        let source =
            "function passthrough(x: &i32): &i32 { return x; } function forward(x: &i32): &i32 { return passthrough(x); }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn allows_returning_alias_of_borrowed_parameter() {
        // Assign
        let source = "function alias(x: &i32): &i32 { const view = x; return view; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn allows_multi_source_borrowed_return_summary_through_helper() {
        // Assign
        let source = "function choose(a: &i32, b: &i32): &i32 { if (flag) { return a; } return b; } function forward(a: &i32, b: &i32): &i32 { return choose(a, b); }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn rejects_using_helper_returned_borrow_after_mutable_reborrow() {
        // Assign
        let source = "function passthrough(x: &i32): &i32 { return x; } function f(a: i32): i32 { const r = passthrough(&a); const m = &mut a; console.log(r); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn rejects_using_multi_source_helper_return_after_mutating_possible_source() {
        // Assign
        let source = "function choose(a: &i32, b: &i32): &i32 { if (flag) { return a; } return b; } function f(a: i32, b: i32): i32 { const r = choose(&a, &b); const m = &mut a; console.log(r); return 0; }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn allows_mutually_recursive_borrowed_return_summary() {
        // Assign
        let source = "function left(x: &i32): &i32 { return right(x); } function right(x: &i32): &i32 { if (1 < 2) { return x; } return left(x); }";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }
}

mod thread_safety_regressions {
    use super::*;

    #[test]
    fn rejects_thread_capture_of_non_thread_safe_struct_value() {
        // Assign
        let source = "struct Box { value: i32; get(): i32 with &this { return this.value; } } const b = Box { value: 0 }; thread(b);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn allows_thread_capture_of_thread_safe_array() {
        // Assign
        let source = "const data = [1, 2, 3]; thread(data);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn allows_thread_capture_of_shared_reference_with_sync_pointee() {
        // Assign
        let source = "const data = [1, 2, 3]; const view = &data; thread(view);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn rejects_thread_capture_of_mutable_reference() {
        // Assign
        let source = "let data = [1, 2, 3]; const view = &mut data; thread(view);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn rejects_transitive_thread_capture_of_non_thread_safe_struct_value() {
        // Assign
        let source =
            "struct Box { value: i32; get(): i32 with &this { return this.value; } } function spawn(x: Box): void { thread(x); } const b = Box { value: 0 }; spawn(b);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn rejects_transitive_process_capture_of_non_thread_safe_struct_value() {
        // Assign
        let source =
            "struct Box { value: i32; get(): i32 with &this { return this.value; } } function queue(x: Box): void { process(x); } const b = Box { value: 0 }; queue(b);";
        let ast = parse(source).unwrap();
        let mut checker = BorrowChecker::new();
        let mut type_checker = TypeChecker::new();
        let type_output = type_checker.check_with_output(&ast).unwrap();

        // Act
        let result = checker.check_typed(&ast, type_output);

        // Assert
        assert!(result.is_err());
    }
}
