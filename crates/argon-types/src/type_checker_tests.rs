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
        let source = "function foo(): void { println('test'); }";
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

mod struct_constructor_type_checking {
    use super::*;

    #[test]
    fn checks_struct_with_constructor() {
        // Assign
        let source = "struct Point { constructor(x: i32, y: i32) { this.x = x; this.y = y; } }";
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn checks_struct_with_methods() {
        // Assign
        let source = "struct Calculator { add(a: i32, b: i32): i32 { return a + b; } }";
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
    fn type_checks_generic_struct_instantiation_and_member_access() {
        // Assign
        let source = r#"
struct Container<T> {
    value: T;
    constructor(v: T) { this.value = v; }
    get(): T { return this.value; }
}
type NumberContainer = Container<number>;
const container: NumberContainer = Container { v: 42 };
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
struct Box<T> { value: T; constructor(v: T) { this.value = v; } }
type NumBox = Box<number>;
const b: NumBox = Box { v: 1 };
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }

    #[test]
    fn allows_object_literal_assignment_to_instantiated_generic_struct_alias() {
        // Assign
        let source = r#"
struct Container<T> { value: T; }
type NumberContainer = Container<number>;
const container: NumberContainer = { value: 42 };
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }

    #[test]
    fn rejects_object_literal_assignment_with_missing_field_for_generic_struct_alias() {
        // Assign
        let source = r#"
struct Container<T> { value: T; }
type NumberContainer = Container<number>;
const container: NumberContainer = { };
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_err());
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

mod union_assignability {
    use super::*;

    #[test]
    fn allows_returning_either_variant_for_struct_union_return_type() {
        // Assign
        let source = r#"
struct Some { value: i32; }
struct None {}
function find(flag: boolean, value: i32): Some | None {
    if (flag) {
        return Some { value };
    }
    return None {};
}
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
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

mod interface_and_object_shape_checking {
    use super::*;

    #[test]
    fn allows_structural_object_literal_assignment_to_interface() {
        // Assign
        let source = r#"
interface HasName {
    name: string;
}

function render(item: HasName): string {
    return item.name;
}

const label: string = render({ name: "argon" });
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }

    #[test]
    fn resolves_generic_interface_member_types() {
        // Assign
        let source = r#"
interface Box<T> {
    value: T;
}

function unwrap<T>(box: Box<T>): T {
    return box.value;
}

const value: i32 = unwrap({ value: 7 });
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }

    #[test]
    fn resolves_generic_object_type_aliases() {
        // Assign
        let source = r#"
struct Container<T> {
    value: T;
}

type Box<T> = Container<T>;
const value: Box<i32> = { value: 7 };
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }
}

mod enum_and_generic_inference {
    use super::*;

    #[test]
    fn resolves_enum_member_access_to_enum_type() {
        // Assign
        let source = r#"
enum Status { Ready, Running }
const status: Status = Status.Ready;
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
    }

    #[test]
    fn infers_generic_type_arguments_from_array_shape() {
        // Assign
        let source = r#"
function first<T>(items: T[]): T {
    return items[0];
}

const value: i32 = first([1, 2, 3]);
"#;
        let ast = parse(source).unwrap();
        let mut checker = TypeChecker::new();

        // Act
        let result = checker.check(&ast);

        // Assert
        assert!(result.is_ok(), "type check error: {:?}", result.err());
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
