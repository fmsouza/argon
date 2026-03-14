//! Argon compiler driver.
//!
//! Centralizes pipeline orchestration so CLI/tooling (watch/REPL/LSP) can reuse it.

use argon_ast::SourceFile;
use argon_borrowck::BorrowChecker;
use argon_codegen_js::{generate_type_declarations, JsCodegen};
use argon_codegen_wasm::WasmCodegen;
use argon_diagnostics::{Diagnostic, DiagnosticBag, DiagnosticEngine, Severity};
use argon_ir::IrBuilder;
use argon_parser::{parse, ParseError};
use argon_types::{TypeCheckOutput, TypeChecker};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Js,
    Wasm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pipeline {
    Ast,
    Ir,
}

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub target: Target,
    pub pipeline: Pipeline,
    pub optimize: bool,
    pub source_map: bool,
    pub declarations: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: Target::Js,
            pipeline: Pipeline::Ir,
            optimize: false,
            source_map: false,
            declarations: false,
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
        let ast = self.parse(source, source_name)?;
        let _types = self.check_semantics(source, source_name, &ast)?;

        match options.target {
            Target::Js => self.compile_js(source, source_name, &ast, options),
            Target::Wasm => self.compile_wasm(source, source_name, &ast, options),
        }
    }

    pub fn compile_file(
        &self,
        path: &Path,
        options: &CompileOptions,
    ) -> Result<CompileResult, DriverError> {
        let source = std::fs::read_to_string(path).map_err(|e| DriverError::WithDiagnostics {
            message: format!("io error: {}", e),
            diagnostics: Diagnostics {
                bag: DiagnosticBag::new(),
                rendered: format!("io error: {}", e),
            },
        })?;

        let source_name = path.display().to_string();
        let ast = self.parse(&source, &source_name)?;
        let deps = self.collect_deps(&ast, path.parent().unwrap_or(Path::new(".")));

        let _types = self.check_semantics(&source, &source_name, &ast)?;

        let artifacts = match options.target {
            Target::Js => self.compile_js(&source, &source_name, &ast, options)?,
            Target::Wasm => self.compile_wasm(&source, &source_name, &ast, options)?,
        };

        Ok(CompileResult { artifacts, deps })
    }

    /// Compile a file and all its transitive `.arg` dependencies.
    pub fn compile_project(
        &self,
        entry: &Path,
        options: &CompileOptions,
    ) -> Result<ProjectCompileResult, DriverError> {
        let entry_canonical =
            std::fs::canonicalize(entry).map_err(|e| DriverError::WithDiagnostics {
                message: format!("io error: {}", e),
                diagnostics: Diagnostics {
                    bag: DiagnosticBag::new(),
                    rendered: format!("io error: {}", e),
                },
            })?;

        let mut compiled: HashSet<PathBuf> = HashSet::new();
        let mut results: Vec<(PathBuf, CompileArtifacts)> = Vec::new();
        let mut queue: Vec<PathBuf> = vec![entry_canonical];

        while let Some(path) = queue.pop() {
            if compiled.contains(&path) {
                continue;
            }
            compiled.insert(path.clone());

            let result = self.compile_file(&path, options)?;

            for dep in &result.deps {
                if dep.exists() {
                    if let Ok(canonical) = std::fs::canonicalize(dep) {
                        if !compiled.contains(&canonical) {
                            queue.push(canonical);
                        }
                    }
                }
            }

            results.push((path, result.artifacts));
        }

        Ok(ProjectCompileResult { files: results })
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
        types: TypeCheckOutput,
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
        let types = self.type_check_output(source, source_name, ast)?;
        self.borrow_check_typed(source, source_name, ast, types.clone())?;
        Ok(types)
    }

    fn compile_js(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let mut codegen = if options.source_map {
            JsCodegen::new().with_source_map(source_name)
        } else {
            JsCodegen::new()
        };

        let js = match options.pipeline {
            Pipeline::Ast => codegen.generate_from_ast(ast),
            Pipeline::Ir => {
                let mut builder = IrBuilder::new();
                let mut ir = builder.build(ast).map_err(|e| {
                    self.simple_error_to_driver(source, source_name, "ir error", &e)
                })?;
                if options.optimize {
                    let _ = argon_ir::passes::optimize_module(&mut ir);
                }
                codegen.generate(&ir)
            }
        }
        .map_err(|e| self.simple_error_to_driver(source, source_name, "codegen error", &e))?;

        let source_map = codegen.get_source_map();
        let declarations = options
            .declarations
            .then(|| generate_type_declarations(ast));

        Ok(CompileArtifacts {
            js: Some(js),
            wasm: None,
            wat: None,
            wasm_loader_js: None,
            wasm_host_js: None,
            source_map,
            declarations,
        })
    }

    fn compile_wasm(
        &self,
        source: &str,
        source_name: &str,
        ast: &SourceFile,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let mut codegen = WasmCodegen::new();
        let mut builder = IrBuilder::new();
        let mut ir = builder
            .build(ast)
            .map_err(|e| self.simple_error_to_driver(source, source_name, "ir error", &e))?;

        if options.optimize {
            let _ = argon_ir::passes::optimize_module(&mut ir);
        }

        let wasm_host_js = self.generate_wasm_host_module_from_ir(source, source_name, &ir)?;
        let wasm = match options.pipeline {
            Pipeline::Ast => codegen.generate_from_ast(ast),
            Pipeline::Ir => codegen.generate(&ir),
        }
        .map_err(|e| self.simple_error_to_driver(source, source_name, "codegen error", &e))?;

        let wat = wasmprinter::print_bytes(&wasm).ok();

        Ok(CompileArtifacts {
            js: None,
            wasm: Some(wasm),
            wat,
            wasm_loader_js: Some(self.generate_wasm_loader("__WASM_FILE__", "__HOST_FILE__")),
            wasm_host_js: Some(wasm_host_js),
            source_map: None,
            declarations: None,
        })
    }

    fn generate_wasm_host_module_from_ir(
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

    fn append_host_exports(&self, mut host_js: String, host_only_exports: &[String]) -> String {
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

    fn generate_wasm_loader(&self, wasm_file_name: &str, host_file_name: &str) -> String {
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

    fn simple_error_to_driver(
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

        std::fs::write(&utils_path, "export function greet(): string { return \"hi\"; }\n")
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
        std::fs::write(
            &a_path,
            "from \"./b\" import { Y };\nconst x = Y;\n",
        )
        .unwrap();

        let compiler = Compiler::new();
        let options = CompileOptions::default();
        let result = compiler.compile_project(&a_path, &options).unwrap();

        assert_eq!(result.files.len(), 3);
    }
}
