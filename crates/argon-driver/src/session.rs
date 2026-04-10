use crate::{
    CompileArtifacts, CompileOptions, CompileResult, Compiler, DriverError, ProjectCompileResult,
    Target,
};
use argon_ast::SourceFile;
use argon_codegen_js::{generate_type_declarations, JsCodegen};
use argon_codegen_native::NativeCodegen;
use argon_codegen_wasm::WasmCodegen;
use argon_ir::{IrBuilder, Module as IrModule};
use argon_target::TargetTriple;
use argon_types::TypeCheckOutput;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MAX_SOURCE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceIdentity {
    File(PathBuf),
    Inline(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceCacheKey {
    identity: SourceIdentity,
    content_hash: u64,
}

#[derive(Debug)]
struct CachedModule {
    key: SourceCacheKey,
    source_name: String,
    source: Arc<String>,
    ast: Arc<SourceFile>,
    _typed: Arc<TypeCheckOutput>,
    desugared_ast: Arc<SourceFile>,
    deps: Vec<PathBuf>,
    ir: Mutex<Option<Arc<IrModule>>>,
    optimized_ir: Mutex<Option<Arc<IrModule>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ArtifactCacheKey {
    source_key: SourceCacheKey,
    options: CompileOptions,
}

#[derive(Debug, Clone)]
struct CachedProjectGraph {
    deps: BTreeMap<PathBuf, Vec<PathBuf>>,
    file_hashes: HashMap<PathBuf, u64>,
    ordered_files: Vec<PathBuf>,
    layers: Vec<Vec<PathBuf>>,
}

#[derive(Debug, Clone)]
pub struct CheckedFile {
    pub source_name: String,
    pub ast: Arc<SourceFile>,
    pub deps: Vec<PathBuf>,
}

pub struct CompilationSession {
    compiler: Compiler,
    modules: Mutex<HashMap<SourceIdentity, Arc<CachedModule>>>,
    artifacts: Mutex<HashMap<ArtifactCacheKey, Arc<CompileArtifacts>>>,
    project_graphs: Mutex<HashMap<PathBuf, Arc<CachedProjectGraph>>>,
}

impl Default for CompilationSession {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilationSession {
    pub fn new() -> Self {
        Self {
            compiler: Compiler::new(),
            modules: Mutex::new(HashMap::new()),
            artifacts: Mutex::new(HashMap::new()),
            project_graphs: Mutex::new(HashMap::new()),
        }
    }

    pub fn compile(
        &self,
        source: &str,
        source_name: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let module = self.load_inline_module(source, source_name)?;
        self.compile_cached_module(&module, options)
    }

    pub fn compile_file(
        &self,
        path: &Path,
        options: &CompileOptions,
    ) -> Result<CompileResult, DriverError> {
        let module = self.load_file_module(path)?;
        let artifacts = self.compile_cached_module(&module, options)?;
        Ok(CompileResult {
            artifacts,
            deps: module.deps.clone(),
        })
    }

    pub fn compile_project(
        &self,
        entry: &Path,
        options: &CompileOptions,
    ) -> Result<ProjectCompileResult, DriverError> {
        let graph = self.resolve_project_graph(entry)?;
        let order_index: HashMap<PathBuf, usize> = graph
            .ordered_files
            .iter()
            .cloned()
            .enumerate()
            .map(|(idx, path)| (path, idx))
            .collect();
        let mut results = Vec::with_capacity(graph.ordered_files.len());

        for layer in &graph.layers {
            let mut layer_results = Vec::with_capacity(layer.len());
            let scoped = std::thread::scope(|scope| {
                let mut handles = Vec::with_capacity(layer.len());
                for path in layer {
                    let path = path.clone();
                    handles.push(scope.spawn(move || {
                        let result = self.compile_file(&path, options)?;
                        Ok::<_, DriverError>((path, result.artifacts))
                    }));
                }

                for handle in handles {
                    match handle.join() {
                        Ok(Ok(result)) => layer_results.push(result),
                        Ok(Err(err)) => return Err(err),
                        Err(payload) => std::panic::resume_unwind(payload),
                    }
                }

                Ok::<(), DriverError>(())
            });
            scoped?;

            layer_results
                .sort_by_key(|(path, _)| order_index.get(path).copied().unwrap_or(usize::MAX));
            results.extend(layer_results);
        }

        Ok(ProjectCompileResult { files: results })
    }

    pub fn check_file(&self, path: &Path) -> Result<CheckedFile, DriverError> {
        let module = self.load_file_module(path)?;
        Ok(CheckedFile {
            source_name: module.source_name.clone(),
            ast: Arc::clone(&module.ast),
            deps: module.deps.clone(),
        })
    }

    pub fn project_files(&self, entry: &Path) -> Result<Vec<PathBuf>, DriverError> {
        Ok(self.resolve_project_graph(entry)?.ordered_files.clone())
    }

    fn compile_cached_module(
        &self,
        module: &Arc<CachedModule>,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let cache_key = ArtifactCacheKey {
            source_key: module.key.clone(),
            options: options.clone(),
        };

        if let Some(artifacts) = self.artifacts.lock().unwrap().get(&cache_key).cloned() {
            return Ok((*artifacts).clone());
        }

        let artifacts = match options.target {
            Target::Js => self.compile_js(module, options)?,
            Target::Wasm => self.compile_wasm(module, options)?,
            Target::Native => self.compile_native(module, options)?,
        };

        self.artifacts
            .lock()
            .unwrap()
            .insert(cache_key, Arc::new(artifacts.clone()));

        Ok(artifacts)
    }

    fn compile_js(
        &self,
        module: &Arc<CachedModule>,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let mut codegen = if options.source_map {
            JsCodegen::new().with_source_map(&module.source_name)
        } else {
            JsCodegen::new()
        };

        let js = match options.pipeline {
            crate::Pipeline::Ast => codegen.generate_from_ast(&module.desugared_ast),
            crate::Pipeline::Ir => {
                let ir = self.ir_for_module(module, options.optimize)?;
                codegen.generate(ir.as_ref())
            }
        }
        .map_err(|e| {
            self.compiler.simple_error_to_driver(
                module.source.as_str(),
                &module.source_name,
                "codegen error",
                &e,
            )
        })?;

        let source_map = codegen.get_source_map();
        let declarations = options
            .declarations
            .then(|| generate_type_declarations(&module.desugared_ast));

        Ok(CompileArtifacts {
            js: Some(js),
            wasm: None,
            wat: None,
            wasm_loader_js: None,
            wasm_host_js: None,
            source_map,
            declarations,
            native_obj: None,
            native_asm: None,
        })
    }

    fn compile_wasm(
        &self,
        module: &Arc<CachedModule>,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        self.compiler.validate_wasm_imports(
            module.source.as_str(),
            &module.source_name,
            &module.ast,
        )?;

        let ir = self.ir_for_module(module, options.optimize)?;
        let mut codegen = WasmCodegen::new();
        let wasm_host_js = self.compiler.generate_wasm_host_module_from_ir(
            module.source.as_str(),
            &module.source_name,
            ir.as_ref(),
        )?;

        let wasm = match options.pipeline {
            crate::Pipeline::Ast => codegen.generate_from_ast(&module.desugared_ast),
            crate::Pipeline::Ir => codegen.generate(ir.as_ref()),
        }
        .map_err(|e| {
            self.compiler.simple_error_to_driver(
                module.source.as_str(),
                &module.source_name,
                "codegen error",
                &e,
            )
        })?;

        let wat = options
            .emit_wat
            .then(|| wasmprinter::print_bytes(&wasm).ok())
            .flatten();

        Ok(CompileArtifacts {
            js: None,
            wasm: Some(wasm),
            wat,
            wasm_loader_js: Some(
                self.compiler
                    .generate_wasm_loader("__WASM_FILE__", "__HOST_FILE__"),
            ),
            wasm_host_js: Some(wasm_host_js),
            source_map: None,
            declarations: None,
            native_obj: None,
            native_asm: None,
        })
    }

    fn compile_native(
        &self,
        module: &Arc<CachedModule>,
        options: &CompileOptions,
    ) -> Result<CompileArtifacts, DriverError> {
        let triple = match &options.target_triple {
            Some(t) => TargetTriple::parse(t).map_err(|e| {
                self.compiler.simple_error_to_driver(
                    module.source.as_str(),
                    &module.source_name,
                    "target error",
                    &e,
                )
            })?,
            None => TargetTriple::host(),
        };

        let ir = self.ir_for_module(module, options.optimize)?;
        let codegen = NativeCodegen::new(triple).with_opt_level(options.native_opt_level);
        let obj_bytes = codegen.generate(ir.as_ref()).map_err(|e| {
            self.compiler.simple_error_to_driver(
                module.source.as_str(),
                &module.source_name,
                "native codegen error",
                &e,
            )
        })?;

        Ok(CompileArtifacts {
            js: None,
            wasm: None,
            wat: None,
            wasm_loader_js: None,
            wasm_host_js: None,
            source_map: None,
            declarations: None,
            native_obj: Some(obj_bytes),
            native_asm: None,
        })
    }

    fn ir_for_module(
        &self,
        module: &Arc<CachedModule>,
        optimize: bool,
    ) -> Result<Arc<IrModule>, DriverError> {
        let slot = if optimize {
            &module.optimized_ir
        } else {
            &module.ir
        };

        let cached = {
            let guard = slot.lock().unwrap();
            guard.clone()
        };
        if let Some(ir) = cached {
            return Ok(ir);
        }

        let existing_base_ir = {
            let guard = module.ir.lock().unwrap();
            guard.clone()
        };
        let mut base_ir = if let Some(ir) = existing_base_ir {
            (*ir).clone()
        } else {
            let mut builder = IrBuilder::new();
            let ir = builder.build(&module.desugared_ast).map_err(|e| {
                self.compiler.simple_error_to_driver(
                    module.source.as_str(),
                    &module.source_name,
                    "ir error",
                    &e,
                )
            })?;
            let shared = Arc::new(ir.clone());
            {
                let mut guard = module.ir.lock().unwrap();
                *guard = Some(shared);
            }
            ir
        };

        if optimize {
            let _ = argon_ir::passes::optimize_module(&mut base_ir);
        }

        let ir = Arc::new(base_ir);
        {
            let mut guard = slot.lock().unwrap();
            *guard = Some(Arc::clone(&ir));
        }
        Ok(ir)
    }

    fn load_inline_module(
        &self,
        source: &str,
        source_name: &str,
    ) -> Result<Arc<CachedModule>, DriverError> {
        self.load_module(
            SourceIdentity::Inline(source_name.to_string()),
            source.to_string(),
            source_name.to_string(),
            None,
        )
    }

    fn load_file_module(&self, path: &Path) -> Result<Arc<CachedModule>, DriverError> {
        let canonical = std::fs::canonicalize(path).map_err(io_driver_error)?;
        let metadata = std::fs::metadata(&canonical).map_err(io_driver_error)?;
        if metadata.len() > MAX_SOURCE_SIZE {
            return Err(DriverError::WithDiagnostics {
                message: "source file exceeds 10 MB limit".to_string(),
                diagnostics: crate::Diagnostics {
                    bag: argon_diagnostics::DiagnosticBag::new(),
                    rendered: format!(
                        "error: source file '{}' is {} bytes, exceeding the 10 MB limit",
                        canonical.display(),
                        metadata.len()
                    ),
                },
            });
        }

        let source = std::fs::read_to_string(&canonical).map_err(io_driver_error)?;
        let source_name = canonical.display().to_string();
        let base_dir = canonical.parent().map(Path::to_path_buf);

        self.load_module(
            SourceIdentity::File(canonical),
            source,
            source_name,
            base_dir.as_deref(),
        )
    }

    fn load_module(
        &self,
        identity: SourceIdentity,
        source: String,
        source_name: String,
        base_dir: Option<&Path>,
    ) -> Result<Arc<CachedModule>, DriverError> {
        let key = SourceCacheKey {
            identity: identity.clone(),
            content_hash: content_hash(&source),
        };

        if let Some(module) = self.modules.lock().unwrap().get(&identity).cloned() {
            if module.key == key {
                return Ok(module);
            }
        }

        let ast = self.compiler.parse(&source, &source_name)?;
        self.compiler.validate_std_imports(&ast)?;

        let typed = Arc::new(
            self.compiler
                .type_check_output(&source, &source_name, &ast)?,
        );
        self.compiler
            .borrow_check_typed(&source, &source_name, &ast, Arc::clone(&typed))?;

        let deps = base_dir
            .map(|dir| self.compiler.collect_deps(&ast, dir))
            .unwrap_or_default();

        let mut desugared_ast = ast.clone();
        argon_types::desugar::desugar_named_args(&mut desugared_ast, &typed.env);

        let module = Arc::new(CachedModule {
            key,
            source_name,
            source: Arc::new(source),
            ast: Arc::new(ast),
            _typed: typed,
            desugared_ast: Arc::new(desugared_ast),
            deps,
            ir: Mutex::new(None),
            optimized_ir: Mutex::new(None),
        });

        self.modules
            .lock()
            .unwrap()
            .insert(identity, Arc::clone(&module));

        Ok(module)
    }

    fn resolve_project_graph(&self, entry: &Path) -> Result<Arc<CachedProjectGraph>, DriverError> {
        let entry = std::fs::canonicalize(entry).map_err(io_driver_error)?;

        if let Some(graph) = self.project_graphs.lock().unwrap().get(&entry).cloned() {
            if self.project_graph_is_valid(&graph)? {
                return Ok(graph);
            }
        }

        let graph = Arc::new(self.build_project_graph(&entry)?);
        self.project_graphs
            .lock()
            .unwrap()
            .insert(entry, Arc::clone(&graph));
        Ok(graph)
    }

    fn project_graph_is_valid(&self, graph: &CachedProjectGraph) -> Result<bool, DriverError> {
        for (path, deps) in &graph.deps {
            let module = self.load_file_module(path)?;
            if graph.file_hashes.get(path).copied() != Some(module.key.content_hash) {
                return Ok(false);
            }

            if self.normalized_file_deps(&module) != *deps {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn build_project_graph(&self, entry: &Path) -> Result<CachedProjectGraph, DriverError> {
        let mut deps = BTreeMap::new();
        let mut file_hashes = HashMap::new();
        let mut seen = HashSet::new();
        let mut queue = vec![entry.to_path_buf()];

        while let Some(path) = queue.pop() {
            if !seen.insert(path.clone()) {
                continue;
            }

            let module = self.load_file_module(&path)?;
            let normalized = self.normalized_file_deps(&module);

            file_hashes.insert(path.clone(), module.key.content_hash);
            deps.insert(path.clone(), normalized.clone());

            for dep in normalized.into_iter().rev() {
                if !seen.contains(&dep) {
                    queue.push(dep);
                }
            }
        }

        let (ordered_files, layers) = build_execution_order(&deps);

        Ok(CachedProjectGraph {
            deps,
            file_hashes,
            ordered_files,
            layers,
        })
    }

    fn normalized_file_deps(&self, module: &CachedModule) -> Vec<PathBuf> {
        let mut deps: Vec<PathBuf> = module
            .deps
            .iter()
            .filter_map(|dep| {
                if dep.exists() {
                    std::fs::canonicalize(dep)
                        .ok()
                        .or_else(|| Some(dep.clone()))
                } else {
                    None
                }
            })
            .collect();
        deps.sort();
        deps.dedup();
        deps
    }
}

fn build_execution_order(
    graph: &BTreeMap<PathBuf, Vec<PathBuf>>,
) -> (Vec<PathBuf>, Vec<Vec<PathBuf>>) {
    let components = strongly_connected_components(graph);
    let mut component_index = HashMap::new();
    for (index, component) in components.iter().enumerate() {
        for node in component {
            component_index.insert(node.clone(), index);
        }
    }

    let mut outgoing: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); components.len()];
    let mut indegree = vec![0usize; components.len()];

    for (node, deps) in graph {
        let node_component = component_index[node];
        for dep in deps {
            let dep_component = component_index[dep];
            if dep_component != node_component && outgoing[dep_component].insert(node_component) {
                indegree[node_component] += 1;
            }
        }
    }

    let mut ready: Vec<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(idx, degree)| (*degree == 0).then_some(idx))
        .collect();
    ready.sort_by_key(|idx| components[*idx].first().cloned().unwrap_or_default());

    let mut ordered_files = Vec::new();
    let mut layers = Vec::new();

    while !ready.is_empty() {
        let current = ready.clone();
        ready.clear();

        let mut layer = Vec::new();
        for component in &current {
            layer.extend(components[*component].clone());
        }
        layer.sort();
        ordered_files.extend(layer.clone());
        layers.push(layer);

        let mut next_ready = Vec::new();
        for component in current {
            for target in &outgoing[component] {
                indegree[*target] -= 1;
                if indegree[*target] == 0 {
                    next_ready.push(*target);
                }
            }
        }
        next_ready.sort_by_key(|idx| components[*idx].first().cloned().unwrap_or_default());
        ready = next_ready;
    }

    if ordered_files.len() != graph.len() {
        let mut remaining: Vec<PathBuf> = graph
            .keys()
            .filter(|path| !ordered_files.contains(path))
            .cloned()
            .collect();
        remaining.sort();
        if !remaining.is_empty() {
            ordered_files.extend(remaining.clone());
            layers.push(remaining);
        }
    }

    (ordered_files, layers)
}

fn strongly_connected_components(graph: &BTreeMap<PathBuf, Vec<PathBuf>>) -> Vec<Vec<PathBuf>> {
    fn dfs_order(
        node: &PathBuf,
        graph: &BTreeMap<PathBuf, Vec<PathBuf>>,
        visited: &mut HashSet<PathBuf>,
        order: &mut Vec<PathBuf>,
    ) {
        if !visited.insert(node.clone()) {
            return;
        }

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                dfs_order(neighbor, graph, visited, order);
            }
        }

        order.push(node.clone());
    }

    fn dfs_component(
        node: &PathBuf,
        reverse_graph: &BTreeMap<PathBuf, Vec<PathBuf>>,
        visited: &mut HashSet<PathBuf>,
        component: &mut Vec<PathBuf>,
    ) {
        if !visited.insert(node.clone()) {
            return;
        }

        component.push(node.clone());

        if let Some(neighbors) = reverse_graph.get(node) {
            for neighbor in neighbors {
                dfs_component(neighbor, reverse_graph, visited, component);
            }
        }
    }

    let mut visited = HashSet::new();
    let mut order = Vec::new();
    for node in graph.keys() {
        dfs_order(node, graph, &mut visited, &mut order);
    }

    let mut reverse_graph: BTreeMap<PathBuf, Vec<PathBuf>> = graph
        .keys()
        .cloned()
        .map(|node| (node, Vec::new()))
        .collect();
    for (node, deps) in graph {
        for dep in deps {
            reverse_graph
                .entry(dep.clone())
                .or_default()
                .push(node.clone());
        }
    }

    visited.clear();
    let mut components = Vec::new();
    while let Some(node) = order.pop() {
        if visited.contains(&node) {
            continue;
        }

        let mut component = Vec::new();
        dfs_component(&node, &reverse_graph, &mut visited, &mut component);
        component.sort();
        components.push(component);
    }

    components.sort_by_key(|component| component.first().cloned().unwrap_or_default());
    components
}

fn content_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

fn io_driver_error(err: std::io::Error) -> DriverError {
    DriverError::WithDiagnostics {
        message: format!("io error: {}", err),
        diagnostics: crate::Diagnostics {
            bag: argon_diagnostics::DiagnosticBag::new(),
            rendered: format!("io error: {}", err),
        },
    }
}
