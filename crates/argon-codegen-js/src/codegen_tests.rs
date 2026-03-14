//! Argon - JS Codegen Tests

use crate::JsCodegen;
use argon_ir::IrBuilder;
use argon_parser::parse;

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

mod ir_codegen {
    use super::*;

    #[test]
    fn generates_struct_literal_via_ir() {
        let source =
            "struct Point { x: number; y: number; }\nconst p = Point { x: 1, y: 2 };\nconsole.log(p.x);\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("function Point(init)"));
        assert!(output.contains("new Point({ x: 1, y: 2 })"));
    }

    #[test]
    fn generates_async_and_await_via_ir() {
        let source = "async function greet(): string { return \"hello\"; }\nasync function main(): string { const r = await greet(); return r; }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("async function greet"));
        assert!(output.contains("async function main"));
        assert!(output.contains("await greet()"));
    }

    #[test]
    fn generates_jsx_via_ir() {
        let source = "<div className=\"test\">Hello</div>\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("React.createElement"));
        assert!(output.contains("\"div\""));
        assert!(output.contains("className: \"test\""));
        assert!(output.contains("\"Hello\""));
    }

    #[test]
    fn generates_switch_via_ir_cfg() {
        let source = "function f(x: i32): void { switch (x) { case 1: const a = 1; break; default: const b = 2; } }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("function f"));
        assert!(output.contains("switch (__bb)"));
    }

    #[test]
    fn generates_match_via_ir_cfg() {
        let source =
            "function f(x: i32): void { match (x) { 1 => const a = 1, 2 => const b = 2, } }";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("function f"));
        assert!(output.contains("switch (__bb)"));
    }

    #[test]
    fn generates_exported_function_via_ir() {
        let source = "export function foo(): i32 { return 1; }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("function foo"));
        assert!(output.contains("export { foo"));
    }

    #[test]
    fn generates_logical_and_conditional_via_ir() {
        let source = "function f(): boolean { return a && b; }\nfunction g(): i32 { return a > b ? a : b; }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("function f"));
        assert!(output.contains("&&"));
        assert!(output.contains("function g"));
        assert!(output.contains("?"));
        assert!(output.contains(":"));
    }

    #[test]
    fn generates_array_literal_via_ir() {
        let source = "function f(): i32 { const arr = [1, 2, 3]; return 0; }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("[1, 2, 3]"));
    }

    #[test]
    fn does_not_duplicate_assignment_expression_via_ir() {
        let source = "function f(): i32 { let x = 0; x = 1; return x; }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("x = 1"));
        assert!(!output.contains("\n    1;\n"));
    }

    #[test]
    fn generates_try_catch_finally_via_ir() {
        let source = "function f(): void { try { const x = 1; throw x; } catch (e) { const y = e; } finally { const z = 3; } }\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("try {"));
        assert!(output.contains("catch (e)"));
        assert!(output.contains("finally {"));
        assert!(output.contains("throw x"));
    }

    #[test]
    fn emits_module_scope_globals_via_ir() {
        let source = "const x = 1;\nlet y = x + 1;\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("const x = 1;"));
        assert!(output.contains("let y = (x + 1);"));
    }

    #[test]
    fn exports_module_scope_const_via_ir() {
        let source = "export const x = 1;\n";
        let ast = parse(source).unwrap();
        let mut builder = IrBuilder::new();
        let ir = builder.build(&ast).unwrap();
        let mut codegen = JsCodegen::new();

        let output = codegen.generate(&ir).unwrap();
        assert!(output.contains("const x = 1;"));
        assert!(output.contains("export { x"));
    }
}

mod import_path_rewriting {
    use super::*;

    #[test]
    fn rewrites_relative_import_with_dot_slash() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"./utils\""),
            "\"./utils.js\""
        );
    }

    #[test]
    fn rewrites_relative_import_with_parent_dir() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"../lib/math\""),
            "\"../lib/math.js\""
        );
    }

    #[test]
    fn does_not_double_rewrite_js_extension() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"./already.js\""),
            "\"./already.js\""
        );
    }

    #[test]
    fn does_not_rewrite_json_extension() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"./data.json\""),
            "\"./data.json\""
        );
    }

    #[test]
    fn does_not_rewrite_mjs_extension() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"./loader.mjs\""),
            "\"./loader.mjs\""
        );
    }

    #[test]
    fn does_not_rewrite_package_import() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"axios\""),
            "\"axios\""
        );
    }

    #[test]
    fn rewrites_single_quoted_relative_import() {
        assert_eq!(
            JsCodegen::rewrite_import_source("'./utils'"),
            "'./utils.js'"
        );
    }

    #[test]
    fn does_not_rewrite_bare_module() {
        assert_eq!(
            JsCodegen::rewrite_import_source("\"react\""),
            "\"react\""
        );
    }
}
