//! Argon - Parser Tests
//!
//! Tests organized with AAA pattern: Assign, Act, Assert

use crate::parse;
use argon_ast::*;

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
        let source = "function add(a: number, b: number): number { return a + b; }";

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
        let source = "function empty(): void { }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_function_with_multiple_statements() {
        // Assign
        let source = "function foo(): number { const x = 1; const y = 2; return x + y; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod loop_parsing {
    use super::*;

    #[test]
    fn parses_while_loop_with_less_than_condition() {
        // Assign
        let source = "while (i < 10) { i = i + 1; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_for_loop_with_less_than_condition() {
        // Assign
        let source = "for (let i = 0; i < 10; i = i + 1) { const x = i; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_do_while_loop_with_less_than_condition() {
        // Assign
        let source = "do { x = x + 1; } while (x < 10);";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod jsx_parsing {
    use super::*;

    #[test]
    fn parses_jsx_expression_after_return() {
        // Assign
        let source = "function f(): void { return <div>Hello</div>; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        let func = match &ast.statements[0] {
            Stmt::Function(f) => f,
            _ => panic!("expected function statement"),
        };

        let ret = func
            .body
            .statements
            .iter()
            .find_map(|s| match s {
                Stmt::Return(r) => Some(r),
                _ => None,
            })
            .expect("expected return statement");

        let arg = ret.argument.as_ref().expect("expected return value");
        assert!(
            matches!(arg, Expr::JsxElement(_) | Expr::JsxFragment(_)),
            "expected JSX expression"
        );
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

mod struct_constructor_parsing {
    use super::*;

    #[test]
    fn parses_struct_with_fields() {
        // Assign
        let source = "struct Point { x: i32; y: i32; }";

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
    fn parses_struct_with_constructor() {
        // Assign
        let source = "struct Point { constructor(x: i32, y: i32) { this.x = x; this.y = y; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Struct(s) = &ast.statements[0] {
            assert!(s.constructor.is_some());
        } else {
            panic!("Expected struct statement");
        }
    }

    #[test]
    fn parses_struct_with_method_and_constructor() {
        // Assign
        let source = "struct Calculator { add(a: i32, b: i32): i32 { return a + b; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Struct(s) = &ast.statements[0] {
            assert_eq!(s.id.sym, "Calculator");
        } else {
            panic!("Expected struct statement");
        }
    }

    #[test]
    fn parses_generic_struct_with_constructor_and_method() {
        // Assign
        let source = "struct Container<T> { value: T; constructor(v: T) { this.value = v; } get(): T { return this.value; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok(), "parse error: {:?}", result.err());
        let ast = result.unwrap();

        if let Stmt::Struct(s) = &ast.statements[0] {
            assert!(!s.type_params.is_empty());
            assert!(s.constructor.is_some());
            assert!(!s.methods.is_empty());
        } else {
            panic!("Expected struct statement");
        }
    }

    #[test]
    fn parses_struct_with_implements() {
        // Assign
        let source = "struct Foo implements Bar { x: i32; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok(), "parse error: {:?}", result.err());
        let ast = result.unwrap();

        if let Stmt::Struct(s) = &ast.statements[0] {
            assert!(!s.implements.is_empty());
        } else {
            panic!("Expected struct statement");
        }
    }

    #[test]
    fn class_keyword_is_rejected() {
        // Assign
        let source = "class Foo { }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_err());
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
        let source = "function foo(): number { return 42; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_return_without_value() {
        // Assign
        let source = "function foo(): void { return; }";

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

mod advanced_type_parsing {
    use super::*;

    #[test]
    fn parses_union_and_optional_types() {
        // Assign
        let source = "type Maybe = number | null;\nconst x: User? = value;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_array_and_tuple_types() {
        // Assign
        let source = "type A = number[];\ntype T = [number, string];";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_function_type() {
        // Assign
        let source = "type F = (a: number, b?: string) => number;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_type_arguments() {
        // Assign
        let source = "type Boxed = Box<number>;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod generic_declaration_parsing {
    use super::*;

    #[test]
    fn parses_generic_function_declaration() {
        // Assign
        let source = "function id<T>(x: T): T { return x; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_generic_struct_declaration() {
        // Assign
        let source = "struct Box<T> { value: T; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod generic_instantiation_parsing {
    use super::*;

    #[test]
    fn parses_generic_function_call_with_explicit_type_args() {
        // Assign
        let source = "function identity<T>(x: T): T { return x; }\nconst n = identity<number>(1);";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_generic_new_expression_with_explicit_type_args() {
        // Assign
        let source = "struct Container<T> { value: T; }\nconst c = new Container<number>(42);";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_nested_generic_type_arguments_with_double_greater_than() {
        // Assign
        let source = "type T = Map<string, Vec<number>>;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }
}

mod interface_enum_parsing {
    use super::*;

    #[test]
    fn parses_interface_declaration() {
        // Assign
        let source = "interface Drawable { draw(canvas: &mut Canvas): void; value?: number; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_enum_declaration() {
        // Assign
        let source = "enum Color { Red, Green, Blue }";

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
        let source = "struct Foo { bar(): i32 with &this { return 1; } }";

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
        let source = "const x = 1; function foo(): void { } const y = 2;";

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

mod span_parsing {
    use super::*;

    #[test]
    fn variable_statement_spans_cover_expected_substrings() {
        // Assign
        let source = "const x = 1 + 2;";

        // Act
        let ast = parse(source).unwrap();

        // Assert
        let var = match &ast.statements[0] {
            Stmt::Variable(v) => v,
            _ => panic!("expected variable statement"),
        };
        assert_eq!(&source[var.span.clone()], source);

        let decl = &var.declarations[0];
        assert_eq!(&source[decl.span.clone()], "x = 1 + 2");

        let init = decl.init.as_ref().expect("expected initializer");
        assert_eq!(&source[init.span().clone()], "1 + 2");
    }

    #[test]
    fn function_and_return_spans_are_precise() {
        // Assign
        let source = "function add(a: number, b: number): number { return a + b; }";

        // Act
        let ast = parse(source).unwrap();

        // Assert
        let func = match &ast.statements[0] {
            Stmt::Function(f) => f,
            _ => panic!("expected function statement"),
        };
        assert_eq!(&source[func.span.clone()], source);
        assert_eq!(&source[func.body.span.clone()], "{ return a + b; }");

        let ret = func
            .body
            .statements
            .iter()
            .find_map(|s| match s {
                Stmt::Return(r) => Some(r),
                _ => None,
            })
            .expect("expected return statement");
        assert_eq!(&source[ret.span.clone()], "return a + b;");
    }

    #[test]
    fn if_else_and_jsx_spans_cover_full_constructs() {
        // Assign
        let source = "function f(): void { if (x > 0) { return <div>Hello</div>; } else { return <span>Bye</span>; } }";

        // Act
        let ast = parse(source).unwrap();

        // Assert
        let func = match &ast.statements[0] {
            Stmt::Function(f) => f,
            _ => panic!("expected function statement"),
        };

        let if_stmt = func
            .body
            .statements
            .iter()
            .find_map(|s| match s {
                Stmt::If(i) => Some(i),
                _ => None,
            })
            .expect("expected if statement");

        assert_eq!(
            &source[if_stmt.span.clone()],
            "if (x > 0) { return <div>Hello</div>; } else { return <span>Bye</span>; }"
        );

        let first_return = match &*if_stmt.consequent {
            Stmt::Block(b) => b
                .statements
                .iter()
                .find_map(|s| match s {
                    Stmt::Return(r) => Some(r),
                    _ => None,
                })
                .expect("expected return in consequent"),
            _ => panic!("expected block consequent"),
        };

        let jsx = first_return
            .argument
            .as_ref()
            .expect("expected return value");
        assert_eq!(&source[jsx.span().clone()], "<div>Hello</div>");
    }
}

mod completion_frontend_parsing {
    use super::*;

    #[test]
    fn parses_loop_statement() {
        // Assign
        let source = "loop { break; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert!(matches!(ast.statements[0], Stmt::Loop(_)));
    }

    #[test]
    fn parses_for_of_statement() {
        // Assign
        let source = "for (const item of items) { println(item); }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert!(matches!(ast.statements[0], Stmt::ForIn(_)));
    }

    #[test]
    fn parses_struct_with_method() {
        // Assign
        let source = "struct Point { x: f64; getX(): f64 with &this { return this.x; } }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        let s = match &ast.statements[0] {
            Stmt::Struct(s) => s,
            _ => panic!("expected struct statement"),
        };
        assert_eq!(s.methods.len(), 1);
    }

    #[test]
    fn parses_object_literal_in_initializer() {
        // Assign
        let source = "const data = { value: 42 };";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn parses_export_decorator() {
        // Assign
        let source = "@export function f(): i32 { return 1; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert!(matches!(ast.statements[0], Stmt::Export(_)));
    }

    #[test]
    fn parses_js_interop_declare_module_block() {
        // Assign
        let source =
            "@js-interop declare module \"axios\" { function get<T>(url: string): string; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert!(matches!(ast.statements[0], Stmt::Module(_)));
    }

    #[test]
    fn parses_template_literal_with_interpolation() {
        // Assign
        let source = "const msg = `Hello ${name}`;";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        let var = match &ast.statements[0] {
            Stmt::Variable(v) => v,
            _ => panic!("expected variable statement"),
        };
        let init = var.declarations[0]
            .init
            .as_ref()
            .expect("expected initializer");
        match init {
            Expr::Template(t) => {
                assert_eq!(t.expressions.len(), 1);
                assert_eq!(t.quasis.len(), 2);
            }
            _ => panic!("expected template literal expression"),
        }
    }
}

mod import_parsing {
    use super::*;

    #[test]
    fn parses_from_import_named() {
        // Assign
        let source = r#"from "./math" import { add, multiply };"#;

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Import(import) = &ast.statements[0] {
            assert_eq!(import.specifiers.len(), 2);
            assert!(
                matches!(&import.specifiers[0], ImportSpecifier::Named(n) if n.imported.sym == "add")
            );
            assert!(
                matches!(&import.specifiers[1], ImportSpecifier::Named(n) if n.imported.sym == "multiply")
            );
            assert!(import.source.value.contains("./math"));
        } else {
            panic!("Expected import statement");
        }
    }

    #[test]
    fn parses_from_import_namespace() {
        // Assign
        let source = r#"from "./math" import Math;"#;

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Import(import) = &ast.statements[0] {
            assert_eq!(import.specifiers.len(), 1);
            assert!(
                matches!(&import.specifiers[0], ImportSpecifier::Namespace(n) if n.id.sym == "Math")
            );
            assert!(import.source.value.contains("./math"));
        } else {
            panic!("Expected import statement");
        }
    }

    #[test]
    fn parses_from_import_side_effect() {
        // Assign
        let source = r#"from "reflect-metadata" import;"#;

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Import(import) = &ast.statements[0] {
            assert!(import.specifiers.is_empty());
            assert!(import.source.value.contains("reflect-metadata"));
        } else {
            panic!("Expected import statement");
        }
    }

    #[test]
    fn parses_from_import_aliased() {
        // Assign
        let source = r#"from "./math" import { add as plus };"#;

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Import(import) = &ast.statements[0] {
            assert_eq!(import.specifiers.len(), 1);
            if let ImportSpecifier::Named(n) = &import.specifiers[0] {
                assert_eq!(n.imported.sym, "add");
                assert_eq!(n.local.as_ref().unwrap().sym, "plus");
            } else {
                panic!("Expected named import specifier");
            }
        } else {
            panic!("Expected import statement");
        }
    }
}

mod export_parsing {
    use super::*;

    #[test]
    fn parses_export_function() {
        // Assign
        let source = "export function add(a: i32, b: i32): i32 { return a + b; }";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();
        assert_eq!(ast.statements.len(), 1);

        if let Stmt::Export(export) = &ast.statements[0] {
            assert!(export.declaration.is_some());
            assert!(!export.is_type_only);
        } else {
            panic!("Expected export statement");
        }
    }

    #[test]
    fn parses_export_named_specifiers() {
        // Assign
        let source = "export { add, multiply };";

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Export(export) = &ast.statements[0] {
            assert_eq!(export.specifiers.len(), 2);
            assert_eq!(export.specifiers[0].orig.sym, "add");
            assert_eq!(export.specifiers[1].orig.sym, "multiply");
            assert!(export.source.is_none());
        } else {
            panic!("Expected export statement");
        }
    }

    #[test]
    fn parses_export_reexport_from_source() {
        // Assign
        let source = r#"export { add, multiply } from "./math";"#;

        // Act
        let result = parse(source);

        // Assert
        assert!(result.is_ok());
        let ast = result.unwrap();

        if let Stmt::Export(export) = &ast.statements[0] {
            assert_eq!(export.specifiers.len(), 2);
            assert!(export.source.is_some());
            assert!(export.source.as_ref().unwrap().value.contains("./math"));
        } else {
            panic!("Expected export statement");
        }
    }

    #[test]
    fn rejects_export_default() {
        // Assign
        let source = "export default function foo() {}";

        // Act
        let result = parse(source);

        // Assert — export default is not supported, should fail
        assert!(result.is_err());
    }
}
