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
    use crate::Instruction;

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

    #[test]
    fn translates_logical_expression() {
        let source = "function foo(): boolean { return a && b; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let foo = module.functions.iter().find(|f| f.id == "foo").unwrap();

        let mut saw_logical = false;
        for block in &foo.body {
            for inst in &block.instructions {
                if matches!(inst, Instruction::LogicalOp { .. }) {
                    saw_logical = true;
                }
            }
        }
        assert!(saw_logical);
    }

    #[test]
    fn translates_conditional_expression() {
        let source = "function foo(): i32 { return a > b ? a : b; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let foo = module.functions.iter().find(|f| f.id == "foo").unwrap();

        let mut saw_conditional = false;
        for block in &foo.body {
            for inst in &block.instructions {
                if matches!(inst, Instruction::Conditional { .. }) {
                    saw_conditional = true;
                }
            }
        }
        assert!(saw_conditional);
    }

    #[test]
    fn translates_array_literal() {
        let source = "function foo(): i32 { const arr = [1, 2, 3]; return 0; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let foo = module.functions.iter().find(|f| f.id == "foo").unwrap();

        let mut saw_array = false;
        for block in &foo.body {
            for inst in &block.instructions {
                if matches!(inst, Instruction::ArrayLit { .. }) {
                    saw_array = true;
                }
            }
        }
        assert!(saw_array);
    }

    #[test]
    fn translates_assignment_as_expression() {
        let source = "function foo(): i32 { let x = 0; x = 1; return x; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let foo = module.functions.iter().find(|f| f.id == "foo").unwrap();

        let mut saw_assign_expr = false;
        for block in &foo.body {
            for inst in &block.instructions {
                if matches!(inst, Instruction::AssignExpr { .. }) {
                    saw_assign_expr = true;
                }
            }
        }
        assert!(saw_assign_expr);
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

mod new_and_object_translation {
    use super::*;
    use crate::Instruction;

    #[test]
    fn translates_struct_literal_to_object_and_new() {
        let source = "struct Point { x: number; y: number; }\nconst p = Point { x: 1, y: 2 };";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let p = module.globals.iter().find(|g| g.name == "p").expect("expected global p");
        let mut saw_object = false;
        let mut saw_new = false;
        for inst in &p.init_insts {
            match inst {
                Instruction::ObjectLit { .. } => saw_object = true,
                Instruction::New { .. } => saw_new = true,
                _ => {}
            }
        }

        assert!(saw_object);
        assert!(saw_new);
    }
}

mod async_translation {
    use super::*;
    use crate::Instruction;

    #[test]
    fn translates_async_function_and_await() {
        let source = "async function greet(): string { return \"hello\"; }\nasync function main(): string { const r = await greet(); return r; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let greet = module
            .functions
            .iter()
            .find(|f| f.id == "greet")
            .expect("expected greet");
        let main = module
            .functions
            .iter()
            .find(|f| f.id == "main")
            .expect("expected main");

        assert!(greet.is_async);
        assert!(main.is_async);

        let mut saw_await = false;
        for block in &main.body {
            for inst in &block.instructions {
                if matches!(inst, Instruction::Await { .. }) {
                    saw_await = true;
                }
            }
        }
        assert!(saw_await);
    }
}

mod jsx_translation {
    use super::*;
    use crate::Instruction;

    #[test]
    fn translates_jsx_element_with_attributes_and_children() {
        let source = "<div className=\"test\">Hello</div>";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let init = module
            .functions
            .iter()
            .find(|f| f.id == "__argon_init")
            .expect("expected __argon_init");

        let mut saw_create_element = false;
        let mut saw_class_name_prop = false;
        let mut saw_hello = false;

        for block in &init.body {
            for inst in &block.instructions {
                match inst {
                    Instruction::Member { property, .. } if property == "createElement" => {
                        saw_create_element = true
                    }
                    Instruction::ObjectLit { props, .. } => {
                        if props.iter().any(|p| p.key == "className") {
                            saw_class_name_prop = true;
                        }
                    }
                    Instruction::Const { value, .. } => {
                        if let crate::ConstValue::String(s) = value {
                            if s.contains("Hello") {
                                saw_hello = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        assert!(saw_create_element);
        assert!(saw_class_name_prop);
        assert!(saw_hello);
    }
}

mod switch_and_match_translation {
    use super::*;

    #[test]
    fn translates_switch_statement() {
        let source = "function f(x: i32): void { switch (x) { case 1: const a = 1; break; default: const b = 2; } }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let f = module.functions.iter().find(|f| f.id == "f").unwrap();
        assert!(f.body.len() > 1);
    }

    #[test]
    fn translates_match_statement() {
        let source = "function f(x: i32): void { match (x) { 1 => const a = 1, 2 => const b = 2, } }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let f = module.functions.iter().find(|f| f.id == "f").unwrap();
        assert!(f.body.len() > 1);
    }
}

mod export_translation {
    use super::*;

    #[test]
    fn lowers_exported_function_declaration() {
        let source = "export function foo(): i32 { return 1; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        assert!(module.functions.iter().any(|f| f.id == "foo"));
        assert!(module
            .exports
            .iter()
            .any(|e| e.declaration.is_none()
                && e.specifiers.iter().any(|s| s.orig.sym == "foo")));
    }

    #[test]
    fn lowers_exported_async_function_declaration() {
        let source = "export async function foo(): i32 { return 1; }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let foo = module.functions.iter().find(|f| f.id == "foo").unwrap();
        assert!(foo.is_async);
        assert!(module
            .exports
            .iter()
            .any(|e| e.declaration.is_none()
                && e.specifiers.iter().any(|s| s.orig.sym == "foo")));
    }
}

mod try_catch_translation {
    use super::*;
    use crate::Instruction;

    #[test]
    fn translates_try_catch_finally() {
        let source = "function f(): void { try { const x = 1; throw x; } catch (e) { const y = e; } finally { const z = 3; } }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();

        let module = builder.build(&ast).unwrap();
        let f = module.functions.iter().find(|f| f.id == "f").unwrap();

        let mut saw_try = false;
        let mut saw_throw = false;
        for block in &f.body {
            for inst in &block.instructions {
                match inst {
                    Instruction::Try { try_body, .. } => {
                        saw_try = true;
                        if try_body.iter().any(|i| matches!(i, Instruction::ThrowStmt { .. })) {
                            saw_throw = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        assert!(saw_try);
        assert!(saw_throw);
    }
}
