//! Argon compiler driver.
//!
//! Centralizes pipeline orchestration so CLI/tooling (watch/REPL/LSP) can reuse it.

mod session;

use argon_ast::SourceFile;
use argon_borrowck::BorrowChecker;
use argon_codegen_js::JsCodegen;
use argon_diagnostics::{Diagnostic, DiagnosticBag, DiagnosticEngine, Severity};
use argon_parser::{parse, ParseError};
use argon_types::{TypeCheckOutput, TypeChecker};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use argon_codegen_native::NativeOptLevel;
pub use session::{CheckedFile, CompilationSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Js,
    Wasm,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmitKind {
    Exe,
    Obj,
    Asm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pipeline {
    Ast,
    Ir,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompileOptions {
    pub target: Target,
    pub pipeline: Pipeline,
    pub optimize: bool,
    pub source_map: bool,
    pub declarations: bool,
    pub emit_wat: bool,
    pub native_opt_level: NativeOptLevel,
    /// Target triple for native compilation (e.g., "x86_64-unknown-linux-gnu").
    /// When None, defaults to the host triple.
    pub target_triple: Option<String>,
    /// What to emit for native target.
    pub emit: EmitKind,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: Target::Js,
            pipeline: Pipeline::Ir,
            optimize: false,
            source_map: false,
            declarations: false,
            emit_wat: false,
            native_opt_level: NativeOptLevel::None,
            target_triple: None,
            emit: EmitKind::Exe,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompileArtifacts {
    pub js: Option<String>,
    pub wasm: Option<Vec<u8>>,
    pub wat: Option<String>,
    pub wasm_loader_js: Option<String>,
    pub wasm_host_js: Option<String>,
    pub source_map: Option<String>,
    pub declarations: Option<String>,
    /// Native object file bytes (.o / .obj).
    pub native_obj: Option<Vec<u8>>,
    /// Native assembly text (.s).
    pub native_asm: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub artifacts: CompileArtifacts,
    pub deps: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ProjectCompileResult {
    /// Each compiled file: (source path, artifacts).
    pub files: Vec<(PathBuf, CompileArtifacts)>,
}

#[derive(Debug, Clone)]
pub struct Diagnostics {
    pub bag: DiagnosticBag,
    pub rendered: String,
}

#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    #[error("{message}")]
    WithDiagnostics {
        message: String,
        diagnostics: Diagnostics,
    },
}

impl DriverError {
    pub fn diagnostics(&self) -> Option<&Diagnostics> {
        match self {
            DriverError::WithDiagnostics { diagnostics, .. } => Some(diagnostics),
        }
    }
}

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(
        &self,
        source: &str,
        source_name: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        self.new_session().compile(source, source_name, options)
    }

    pub fn compile_file(
        &self,
        path: &Path,
        options: &CompileOptions,
    ) -> Result<CompileResult, DriverError> {
        self.new_session().compile_file(path, options)
    }

    /// Compile a file and all its transitive `.arg` dependencies.
    pub fn compile_project(
        &self,
        entry: &Path,
        options: &CompileOptions,
    ) -> Result<ProjectCompileResult, DriverError> {
        self.new_session().compile_project(entry, options)
    }

    pub fn new_session(&self) -> CompilationSession {
        CompilationSession::new()
    }

    pub fn collect_deps(&self, ast: &SourceFile, base_dir: &Path) -> Vec<PathBuf> {
        use argon_ast::Stmt;

        let mut deps = Vec::new();
        for stmt in &ast.statements {
            if let Stmt::Import(import) = stmt {
                let raw = import.source.value.trim();
                let spec = raw
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim_start_matches('\'')
                    .trim_end_matches('\'');

                // std:* imports are resolved from the embedded stdlib, not the filesystem.
                if spec.starts_with("std:") {
                    continue;
                }

                if spec.starts_with("./") || spec.starts_with("../") {
                    let path = base_dir.join(spec);
                    if path.extension().is_none() {
                        // No extension means it's an argon module import.
                        deps.push(path.with_extension("arg"));
                    } else if path.extension().and_then(|e| e.to_str()) == Some("arg") {
                        deps.push(path);
                    }
                    // Skip .js/.mjs/.cjs/.json — those are external JS deps.
                }
            }
        }
        deps
    }

    /// Validate that WASM-unsupported std modules are not imported.
    pub(crate) fn validate_wasm_imports(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
    ) -> Result<(), DriverError> {
        use argon_ast::Stmt;

        for stmt in &ast.statements {
            if let Stmt::Import(import) = stmt {
                let raw = import.source.value.trim();
                let spec = raw
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim_start_matches('\'')
                    .trim_end_matches('\'');

                let (unsupported, suggestion) = match spec {
                    "std:net" => (true, "Raw sockets are not available on the WASM target. Use --target js or --target native instead."),
                    _ => (false, ""),
                };

                // Check for server-only imports in std:http and std:ws
                if spec == "std:http" || spec == "std:ws" {
                    for specifier in &import.specifiers {
                        if let argon_ast::ImportSpecifier::Named(named) = specifier {
                            let name = &named.imported.sym;
                            if matches!(name.as_str(), "serve" | "serveAsync" | "wsListen") {
                                return Err(DriverError::WithDiagnostics {
                                    message: format!(
                                        "'{}' from '{}' is not available on the WASM target. \
                                         Servers cannot listen on ports in WebAssembly. \
                                         Use --target js or --target native instead.",
                                        name, spec
                                    ),
                                    diagnostics: Diagnostics {
                                        bag: DiagnosticBag::new(),
                                        rendered: format!(
                                            "error: '{}' from '{}' is not available on the WASM target",
                                            name, spec
                                        ),
                                    },
                                });
                            }
                        }
                    }
                }

                if unsupported {
                    return Err(self.simple_error_to_driver(
                        source,
                        source_name,
                        "wasm target error",
                        &format!(
                            "'{}' is not available on the WASM target. {}",
                            spec, suggestion
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Reject programs that use async/await/spawn — `argon run` is synchronous-only.
    pub fn validate_no_async(&self, ast: &SourceFile) -> Result<(), DriverError> {
        use argon_ast::Stmt;

        for stmt in &ast.statements {
            match stmt {
                Stmt::AsyncFunction(f) => {
                    let name = f
                        .id
                        .as_ref()
                        .map(|id| id.sym.as_str())
                        .unwrap_or("<anonymous>");
                    return Err(DriverError::WithDiagnostics {
                        message: format!(
                            "async function '{}' cannot be executed with `argon run`. \
                             The interpreter is synchronous-only. \
                             Use `argon compile` and run the compiled output instead.",
                            name
                        ),
                        diagnostics: Diagnostics {
                            bag: DiagnosticBag::new(),
                            rendered: format!(
                                "error: async function '{}' is not supported by `argon run`\n\
                                 note: the Argon interpreter is synchronous-only; \
                                 compile with `argon compile` and execute the output instead",
                                name
                            ),
                        },
                    });
                }
                Stmt::Import(import) => {
                    let raw = import.source.value.trim();
                    let spec = raw
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .trim_start_matches('\'')
                        .trim_end_matches('\'');

                    if spec == "std:async" {
                        return Err(DriverError::WithDiagnostics {
                            message: "importing 'std:async' is not supported with `argon run`. \
                                     The interpreter is synchronous-only. \
                                     Use `argon compile` and run the compiled output instead."
                                .to_string(),
                            diagnostics: Diagnostics {
                                bag: DiagnosticBag::new(),
                                rendered:
                                    "error: 'std:async' is not supported by `argon run`\n\
                                     note: the Argon interpreter is synchronous-only; \
                                     compile with `argon compile` and execute the output instead"
                                        .to_string(),
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Validate that all `std:*` imports reference known modules.
    pub fn validate_std_imports(&self, ast: &SourceFile) -> Result<(), DriverError> {
        use argon_ast::Stmt;

        for stmt in &ast.statements {
            if let Stmt::Import(import) = stmt {
                let raw = import.source.value.trim();
                let spec = raw
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim_start_matches('\'')
                    .trim_end_matches('\'');

                if let Some(module_name) = spec.strip_prefix("std:") {
                    if argon_stdlib::resolve_std_module(module_name).is_none() {
                        return Err(DriverError::WithDiagnostics {
                            message: format!("unknown standard library module: std:{}", module_name),
                            diagnostics: Diagnostics {
                                bag: DiagnosticBag::new(),
                                rendered: format!(
                                    "error: unknown standard library module 'std:{}'\navailable modules: {:?}",
                                    module_name,
                                    argon_stdlib::available_modules()
                                ),
                            },
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn parse(&self, source: &str, source_name: &str) -> Result<SourceFile, DriverError> {
        parse(source).map_err(|e| self.parse_error_to_driver(source, source_name, e))
    }

    pub fn type_check(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
    ) -> Result<(), DriverError> {
        let mut type_checker = TypeChecker::new();
        type_checker
            .check(ast)
            .map_err(|e| self.simple_error_to_driver(source, source_name, "type error", &e))?;
        Ok(())
    }

    pub fn type_check_output(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
    ) -> Result<TypeCheckOutput, DriverError> {
        let mut type_checker = TypeChecker::new();
        type_checker
            .check_with_output(ast)
            .map_err(|e| self.simple_error_to_driver(source, source_name, "type error", &e))
    }

    pub fn borrow_check(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
    ) -> Result<(), DriverError> {
        let mut borrow_checker = BorrowChecker::new();
        borrow_checker
            .check(ast)
            .map_err(|e| self.simple_error_to_driver(source, source_name, "borrow error", &e))?;
        Ok(())
    }

    pub fn borrow_check_typed(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
        types: Arc<TypeCheckOutput>,
    ) -> Result<(), DriverError> {
        let mut borrow_checker = BorrowChecker::new();
        borrow_checker
            .check_typed(ast, types)
            .map_err(|e| self.simple_error_to_driver(source, source_name, "borrow error", &e))?;
        Ok(())
    }

    pub fn check_semantics(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
    ) -> Result<TypeCheckOutput, DriverError> {
        let types = Arc::new(self.type_check_output(source, source_name, ast)?);
        self.borrow_check_typed(source, source_name, ast, Arc::clone(&types))?;
        Ok((*types).clone())
    }

    pub(crate) fn generate_wasm_host_module_from_ir(
        &self,
        source: &str,
        source_name: &str,
        ir: &argon_ir::Module,
    ) -> Result<String, DriverError> {
        let mut js_codegen = JsCodegen::new();
        let host_js = js_codegen.generate(ir).map_err(|e| {
            self.simple_error_to_driver(source, source_name, "wasm host codegen error", &e)
        })?;

        let mut explicit_exports = HashSet::new();
        for export in &ir.exports {
            for specifier in &export.specifiers {
                explicit_exports.insert(specifier.orig.sym.clone());
            }
        }

        let host_only_exports: Vec<String> = ir
            .functions
            .iter()
            .filter_map(|function| {
                if function.id.is_empty()
                    || function.id == "__argon_init"
                    || explicit_exports.contains(&function.id)
                {
                    None
                } else {
                    Some(function.id.clone())
                }
            })
            .collect();

        Ok(self.append_host_exports(host_js, &host_only_exports))
    }

    pub(crate) fn append_host_exports(
        &self,
        mut host_js: String,
        host_only_exports: &[String],
    ) -> String {
        if !host_only_exports.is_empty() {
            host_js.push_str("\nexport { ");
            for (idx, export_name) in host_only_exports.iter().enumerate() {
                if idx > 0 {
                    host_js.push_str(", ");
                }
                host_js.push_str(export_name);
            }
            host_js.push_str(" };\n");
        }

        host_js
    }

    pub(crate) fn generate_wasm_loader(
        &self,
        wasm_file_name: &str,
        host_file_name: &str,
    ) -> String {
        format!(
            r#"export function createArgonEnv(overrides = {{}}) {{
  return {{
    ...overrides,
  }};
}}

export async function instantiateArgon(imports = {{}}) {{
  const fs = await import("node:fs/promises");
  const wasmUrl = new URL("./{wasm_file_name}", import.meta.url);
  const hostUrl = new URL("./{host_file_name}", import.meta.url);
  const bytes = await fs.readFile(wasmUrl);
  const hostModule = await import(hostUrl);
  const env = createArgonEnv(imports.argon_env || imports.env || {{}});
  const wasmImports = {{
    ...imports,
    argon_env: env,
    env,
  }};
  const {{ instance, module }} = await WebAssembly.instantiate(bytes, wasmImports);

  const memory = instance.exports.memory || null;
  const wasmExports = instance.exports;
  const hostExports = hostModule;
  const mergedExports = new Proxy({{}}, {{
    get(_target, prop) {{
      if (
        prop in hostExports &&
        typeof hostExports[prop] === "function" &&
        hostExports[prop].constructor &&
        hostExports[prop].constructor.name === "AsyncFunction"
      ) {{
        return hostExports[prop];
      }}
      if (prop in wasmExports) {{
        return wasmExports[prop];
      }}
      return hostExports[prop];
    }},
    has(_target, prop) {{
      return prop in wasmExports || prop in hostExports;
    }},
    ownKeys() {{
      return Array.from(new Set([
        ...Reflect.ownKeys(wasmExports),
        ...Reflect.ownKeys(hostExports),
      ]));
    }},
    getOwnPropertyDescriptor(_target, prop) {{
      if (Object.prototype.hasOwnProperty.call(wasmExports, prop)) {{
        return {{ configurable: true, enumerable: true, value: wasmExports[prop] }};
      }}
      if (Object.prototype.hasOwnProperty.call(hostExports, prop)) {{
        return {{ configurable: true, enumerable: true, value: hostExports[prop] }};
      }}
      return undefined;
    }},
  }});

  return {{
    module,
    instance,
    exports: mergedExports,
    wasmExports,
    hostExports,
    memory,
    readString(ptr) {{
      if (!memory) {{
        throw new Error("Argon wasm module does not export memory");
      }}
      const view = new DataView(memory.buffer);
      const len = view.getUint32(ptr, true);
      const bytes = new Uint8Array(memory.buffer, ptr + 4, len);
      return new TextDecoder().decode(bytes);
    }},
    readArrayI32(ptr) {{
      if (!memory) {{
        throw new Error("Argon wasm module does not export memory");
      }}
      const view = new DataView(memory.buffer);
      const len = view.getInt32(ptr, true);
      const values = [];
      for (let i = 0; i < len; i += 1) {{
        values.push(view.getInt32(ptr + 4 + (i * 4), true));
      }}
      return values;
    }},
  }};
}}
"#
        )
    }

    fn parse_error_to_driver(
        &self,
        source: &str,
        source_name: &str,
        err: ParseError,
    ) -> DriverError {
        let source_id = "source";
        let mut engine = DiagnosticEngine::new();
        engine.add_source(argon_diagnostics::SourceFile::new(
            source_id.to_string(),
            source_name.to_string(),
            source.to_string(),
        ));

        let diagnostic = err.to_diagnostic(source, source_id);
        let mut bag = DiagnosticBag::new();
        bag.add_error(diagnostic);
        let rendered = engine.render(&bag);

        DriverError::WithDiagnostics {
            message: format!("{}", err),
            diagnostics: Diagnostics { bag, rendered },
        }
    }

    pub(crate) fn simple_error_to_driver(
        &self,
        source: &str,
        source_name: &str,
        context: &str,
        err: &dyn std::fmt::Display,
    ) -> DriverError {
        let source_id = "source";
        let mut engine = DiagnosticEngine::new();
        engine.add_source(argon_diagnostics::SourceFile::new(
            source_id.to_string(),
            source_name.to_string(),
            source.to_string(),
        ));

        let diagnostic = Diagnostic::new(
            source_id.to_string(),
            0..source.len().min(1),
            format!("{}: {}", context, err),
        )
        .with_code("E000".to_string());

        let mut bag = DiagnosticBag::new();
        bag.add_error(Diagnostic {
            severity: Severity::Error,
            ..diagnostic
        });
        let rendered = engine.render(&bag);

        DriverError::WithDiagnostics {
            message: format!("{}: {}", context, err),
            diagnostics: Diagnostics { bag, rendered },
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_relative_import_deps() {
        let compiler = Compiler::new();
        let src = "from \"./foo\" import { x };\nconst y = 1;";
        let ast = compiler.parse(src, "<test>").unwrap();
        let deps = compiler.collect_deps(&ast, Path::new("/tmp/proj"));
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], PathBuf::from("/tmp/proj/foo.arg"));
    }

    #[test]
    fn compile_project_compiles_dependencies() {
        let temp_dir = tempfile::tempdir().unwrap();
        let main_path = temp_dir.path().join("main.arg");
        let utils_path = temp_dir.path().join("utils.arg");

        std::fs::write(
            &utils_path,
            "export function greet(): string { return \"hi\"; }\n",
        )
        .unwrap();
        std::fs::write(
            &main_path,
            "from \"./utils\" import { greet };\nconst msg = greet();\n",
        )
        .unwrap();

        let compiler = Compiler::new();
        let options = CompileOptions::default();
        let result = compiler.compile_project(&main_path, &options).unwrap();

        assert_eq!(result.files.len(), 2);
        // Both files should have JS output.
        for (_, artifacts) in &result.files {
            assert!(artifacts.js.is_some());
        }
    }

    #[test]
    fn compile_project_deduplicates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shared_path = temp_dir.path().join("shared.arg");
        let a_path = temp_dir.path().join("a.arg");
        let main_path = temp_dir.path().join("main.arg");

        std::fs::write(&shared_path, "export const X: i32 = 1;\n").unwrap();
        std::fs::write(
            &a_path,
            "from \"./shared\" import { X };\nexport const Y: i32 = X;\n",
        )
        .unwrap();
        std::fs::write(
            &main_path,
            "from \"./a\" import { Y };\nfrom \"./shared\" import { X };\nconst z = X;\n",
        )
        .unwrap();

        let compiler = Compiler::new();
        let options = CompileOptions::default();
        let result = compiler.compile_project(&main_path, &options).unwrap();

        // shared.arg should only appear once.
        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn compile_project_handles_circular() {
        let temp_dir = tempfile::tempdir().unwrap();
        let a_path = temp_dir.path().join("a.arg");
        let b_path = temp_dir.path().join("b.arg");

        std::fs::write(
            &a_path,
            "from \"./b\" import { Y };\nexport const X: i32 = 1;\n",
        )
        .unwrap();
        std::fs::write(
            &b_path,
            "from \"./a\" import { X };\nexport const Y: i32 = 2;\n",
        )
        .unwrap();

        let compiler = Compiler::new();
        let options = CompileOptions::default();
        let result = compiler.compile_project(&a_path, &options).unwrap();

        assert_eq!(result.files.len(), 2);
    }

    #[test]
    fn compile_project_transitive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let c_path = temp_dir.path().join("c.arg");
        let b_path = temp_dir.path().join("b.arg");
        let a_path = temp_dir.path().join("a.arg");

        std::fs::write(&c_path, "export const Z: i32 = 3;\n").unwrap();
        std::fs::write(
            &b_path,
            "from \"./c\" import { Z };\nexport const Y: i32 = Z;\n",
        )
        .unwrap();
        std::fs::write(&a_path, "from \"./b\" import { Y };\nconst x = Y;\n").unwrap();

        let compiler = Compiler::new();
        let options = CompileOptions::default();
        let result = compiler.compile_project(&a_path, &options).unwrap();

        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn session_invalidates_cached_ast_when_source_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_path = temp_dir.path().join("main.arg");
        std::fs::write(&source_path, "const x: i32 = 1;\n").unwrap();

        let session = CompilationSession::new();
        let first = session.check_file(&source_path).unwrap();
        assert_eq!(first.ast.statements.len(), 1);

        std::fs::write(&source_path, "const x: i32 = 1;\nconst y: i32 = x + 1;\n").unwrap();

        let second = session.check_file(&source_path).unwrap();
        assert_eq!(second.ast.statements.len(), 2);
    }

    #[test]
    fn session_project_files_refresh_when_missing_dep_appears() {
        let temp_dir = tempfile::tempdir().unwrap();
        let main_path = temp_dir.path().join("main.arg");
        let dep_path = temp_dir.path().join("dep.arg");

        std::fs::write(
            &main_path,
            "from \"./dep\" import { answer };\nprintln(answer);\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        // Missing deps now produce an error instead of being silently skipped.
        let first = session.project_files(&main_path);
        assert!(first.is_err(), "should error when dep.arg is missing");

        // Once the dep appears, the project graph should resolve.
        std::fs::write(&dep_path, "export const answer: i32 = 42;\n").unwrap();

        let second = session.project_files(&main_path).unwrap();
        assert_eq!(second.len(), 2);
        assert!(second.iter().any(|path| path.ends_with("dep.arg")));
    }

    #[test]
    fn compile_project_order_is_deterministic_across_runs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shared_path = temp_dir.path().join("shared.arg");
        let a_path = temp_dir.path().join("a.arg");
        let b_path = temp_dir.path().join("b.arg");
        let main_path = temp_dir.path().join("main.arg");

        std::fs::write(&shared_path, "export const X: i32 = 1;\n").unwrap();
        std::fs::write(
            &a_path,
            "from \"./shared\" import { X };\nexport const A: i32 = X;\n",
        )
        .unwrap();
        std::fs::write(
            &b_path,
            "from \"./shared\" import { X };\nexport const B: i32 = X;\n",
        )
        .unwrap();
        std::fs::write(
            &main_path,
            "from \"./a\" import { A };\nfrom \"./b\" import { B };\nprintln(A + B);\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        let options = CompileOptions::default();
        let first = session.compile_project(&main_path, &options).unwrap();
        let second = session.compile_project(&main_path, &options).unwrap();

        let first_paths: Vec<_> = first.files.iter().map(|(path, _)| path.clone()).collect();
        let second_paths: Vec<_> = second.files.iter().map(|(path, _)| path.clone()).collect();
        assert_eq!(first_paths, second_paths);
    }

    #[test]
    fn check_project_fails_on_missing_relative_module() {
        let temp_dir = tempfile::tempdir().unwrap();
        let main_path = temp_dir.path().join("main.arg");
        std::fs::write(
            &main_path,
            "from \"./missing\" import { foo };\nconst x = foo();\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        let result = session.check_project(&main_path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not found"),
            "Expected 'not found' error, got: {}",
            err_msg
        );
    }

    #[test]
    fn check_project_fails_on_missing_export() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dep_path = temp_dir.path().join("dep.arg");
        let main_path = temp_dir.path().join("main.arg");

        std::fs::write(&dep_path, "export const answer: i32 = 42;\n").unwrap();
        std::fs::write(
            &main_path,
            "from \"./dep\" import { nonexistent };\nconst x = nonexistent;\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        let result = session.check_project(&main_path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not exported"),
            "Expected 'not exported' error, got: {}",
            err_msg
        );
    }

    #[test]
    fn check_project_succeeds_with_valid_imports() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dep_path = temp_dir.path().join("dep.arg");
        let main_path = temp_dir.path().join("main.arg");

        std::fs::write(
            &dep_path,
            "export function greet(name: string): string { return name; }\n",
        )
        .unwrap();
        std::fs::write(
            &main_path,
            "from \"./dep\" import { greet };\nconst msg = greet(\"world\");\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        let result = session.check_project(&main_path);
        assert!(result.is_ok());
    }

    #[test]
    fn check_project_validates_exported_declaration_type_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let main_path = temp_dir.path().join("main.arg");
        std::fs::write(
            &main_path,
            "export function add(a: i32, b: i32): i32 { return a + b; }\nconst x = add(1, 2);\n",
        )
        .unwrap();

        let session = CompilationSession::new();
        // Should succeed — exported function is well-typed
        let result = session.check_project(&main_path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_no_async_rejects_async_function() {
        let compiler = Compiler::new();
        let ast = compiler
            .parse(
                "async function fetchData(): Future<string> { return \"hello\"; }\n",
                "test.arg",
            )
            .unwrap();
        let result = compiler.validate_no_async(&ast);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("async"));
    }

    #[test]
    fn validate_no_async_rejects_std_async_import() {
        let compiler = Compiler::new();
        let ast = compiler
            .parse(
                "from \"std:async\" import { spawn, sleep };\nconst x: i32 = 1;\n",
                "test.arg",
            )
            .unwrap();
        let result = compiler.validate_no_async(&ast);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("std:async"));
    }

    #[test]
    fn validate_no_async_allows_synchronous_code() {
        let compiler = Compiler::new();
        let ast = compiler
            .parse("function hello(): string { return \"hi\"; }\n", "test.arg")
            .unwrap();
        let result = compiler.validate_no_async(&ast);
        assert!(result.is_ok());
    }
}
