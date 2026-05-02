//! Argon CLI

use argon_driver::{
    CompilationSession, CompileOptions, Compiler, EmitKind, NativeOptLevel, Pipeline, Target,
};
use argon_runtime::{execute_ast, format_json, format_pretty, format_tap, Runtime, TestOutcome, TestResults};
use argon_types::desugar::desugar_named_args;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(clap::Parser)]
#[command(name = "argon")]
#[command(about = "Argon compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(short, long, default_value = "js")]
        target: String,
        #[arg(short, long)]
        source_map: bool,
        #[arg(long)]
        opt: bool,
        #[arg(long)]
        declarations: bool,
        #[arg(long, default_value = "ir")]
        pipeline: String,
        /// Target triple for native compilation (e.g., x86_64-unknown-linux-gnu).
        /// Implies --target native.
        #[arg(long)]
        triple: Option<String>,
        /// What to emit for native target: exe (default), obj, asm.
        /// For the wasm target, use `wat` to additionally write a `.wat` sidecar.
        #[arg(long, default_value = "exe")]
        emit: String,
        /// Native codegen optimization level: none or speed.
        #[arg(long)]
        native_opt: Option<String>,
    },
    Check {
        input: PathBuf,
    },
    Run {
        input: PathBuf,
    },
    Test {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        directory: Option<PathBuf>,
        #[arg(short, long)]
        verbose: bool,
        #[arg(long)]
        filter: Option<String>,
        #[arg(long, default_value = "pretty")]
        format: String,
    },
    Format {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Init {
        name: String,
    },
    Watch {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long, default_value = "js")]
        target: String,
        #[arg(short, long)]
        source_map: bool,
        #[arg(long)]
        opt: bool,
        #[arg(long)]
        declarations: bool,
        #[arg(long, default_value = "ir")]
        pipeline: String,
        #[arg(long)]
        check_only: bool,
        /// Native codegen optimization level: none or speed.
        #[arg(long)]
        native_opt: Option<String>,
    },
    Repl {},
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli: Cli = clap::Parser::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            out_dir,
            target,
            source_map,
            opt,
            declarations,
            pipeline,
            triple,
            emit,
            native_opt,
        } => {
            compile(
                &input,
                output.as_ref(),
                out_dir.as_ref(),
                &target,
                source_map,
                opt,
                declarations,
                &pipeline,
                triple.as_deref(),
                &emit,
                native_opt.as_deref(),
            )?;
        }
        Commands::Check { input } => {
            check(&input)?;
        }
        Commands::Run { input } => {
            run(&input)?;
        }
        Commands::Test {
            input,
            directory,
            verbose,
            filter,
            format,
        } => {
            test(input.as_ref(), directory.as_ref(), verbose, filter.as_ref(), &format)?;
        }
        Commands::Format { input, output } => {
            format_file(&input, output.as_ref())?;
        }
        Commands::Init { name } => {
            init_project(&name)?;
        }
        Commands::Watch {
            input,
            output,
            target,
            source_map,
            opt,
            declarations,
            pipeline,
            check_only,
            native_opt,
        } => {
            watch(
                &input,
                output.as_ref(),
                &target,
                source_map,
                opt,
                declarations,
                &pipeline,
                check_only,
                native_opt.as_deref(),
            )?;
        }
        Commands::Repl {} => {
            repl()?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile(
    input: &PathBuf,
    output: Option<&PathBuf>,
    out_dir: Option<&PathBuf>,
    target: &str,
    source_map: bool,
    opt: bool,
    declarations: bool,
    pipeline: &str,
    triple: Option<&str>,
    emit: &str,
    native_opt: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = CompilationSession::new();
    compile_with_session(
        &session,
        input,
        output,
        out_dir,
        target,
        source_map,
        opt,
        declarations,
        pipeline,
        triple,
        emit,
        native_opt,
    )
}

#[allow(clippy::too_many_arguments)]
fn compile_with_session(
    session: &CompilationSession,
    input: &PathBuf,
    output: Option<&PathBuf>,
    out_dir: Option<&PathBuf>,
    target: &str,
    source_map: bool,
    opt: bool,
    declarations: bool,
    pipeline: &str,
    triple: Option<&str>,
    emit: &str,
    native_opt: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing {}...", input.display());
    println!("Type checking...");
    println!("Borrow checking...");

    // --triple implies --target native
    let target = if triple.is_some() {
        Target::Native
    } else {
        match target {
            "js" => Target::Js,
            "wasm" => Target::Wasm,
            "native" => Target::Native,
            other => {
                return Err(format!("Unknown target: {}", other).into());
            }
        }
    };

    let pipeline = match pipeline {
        "ast" => Pipeline::Ast,
        "ir" => Pipeline::Ir,
        other => {
            return Err(format!("Unknown pipeline: {}", other).into());
        }
    };

    let native_opt_level = match native_opt {
        Some("none") => NativeOptLevel::None,
        Some("speed") => NativeOptLevel::Speed,
        Some(other) => {
            return Err(format!(
                "Unknown native optimization level: {}. Use none or speed.",
                other
            )
            .into());
        }
        None if opt => NativeOptLevel::Speed,
        None => NativeOptLevel::None,
    };

    let (emit, emit_wat) = match target {
        Target::Native => match emit {
            "exe" => (EmitKind::Exe, false),
            "obj" => (EmitKind::Obj, false),
            "asm" => (EmitKind::Asm, false),
            other => {
                return Err(format!("Unknown emit kind: {}. Use exe, obj, or asm.", other).into());
            }
        },
        Target::Wasm => match emit {
            "wat" => (EmitKind::Exe, true),
            "exe" | "wasm" => (EmitKind::Exe, false),
            other => {
                return Err(format!(
                    "Unknown emit kind for wasm: {}. Use wat or omit --emit.",
                    other
                )
                .into());
            }
        },
        Target::Js => (EmitKind::Exe, false),
    };

    let options = CompileOptions {
        target,
        pipeline,
        optimize: opt,
        source_map,
        declarations,
        emit_wat,
        native_opt_level,
        target_triple: triple.map(|s| s.to_string()),
        emit,
    };

    // Multi-file project compilation: compile entry + all dependencies.
    let result = match session.compile_file(input, &options) {
        Ok(r) => r,
        Err(e) => {
            if let Some(diag) = e.diagnostics() {
                eprintln!("{}", diag.rendered);
            }
            return Err(e.into());
        }
    };

    // If there are dependencies, compile the full project.
    if !result.deps.is_empty() {
        let project = match session.compile_project(input, &options) {
            Ok(p) => p,
            Err(e) => {
                if let Some(diag) = e.diagnostics() {
                    eprintln!("{}", diag.rendered);
                }
                return Err(e.into());
            }
        };

        let entry_dir = input
            .parent()
            .and_then(|p| std::fs::canonicalize(p).ok())
            .unwrap_or_else(|| PathBuf::from("."));

        for (source_path, artifacts) in &project.files {
            let relative = source_path.strip_prefix(&entry_dir).unwrap_or(source_path);
            let js_relative = relative.with_extension("js");

            let out_path = if let Some(dir) = out_dir {
                dir.join(&js_relative)
            } else if let Some(single_output) = output {
                // For single -o with deps, put deps alongside the output.
                let out_parent = single_output.parent().unwrap_or(Path::new("."));
                if source_path == &std::fs::canonicalize(input).unwrap_or(input.clone()) {
                    single_output.clone()
                } else {
                    out_parent.join(&js_relative)
                }
            } else {
                js_relative
            };

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            if let Some(ref js) = artifacts.js {
                fs::write(&out_path, js)?;
                println!("Wrote {}", out_path.display());
            }

            if let Some(ref dts) = artifacts.declarations {
                let dts_path = out_path.with_extension("d.ts");
                fs::write(&dts_path, dts)?;
                println!("Wrote {}", dts_path.display());
            }
        }

        println!("Done!");
        return Ok(());
    }

    let artifacts = result.artifacts;

    match target {
        Target::Js => {
            println!("Generating JavaScript...");
            let output_path = output
                .cloned()
                .unwrap_or_else(|| PathBuf::from("output.js"));

            let mut js = artifacts.js.unwrap_or_default();

            if source_map {
                let ext = output_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("js");
                let map_path = output_path.with_extension(format!("{}.map", ext));
                if let Some(map) = artifacts.source_map {
                    fs::write(&map_path, &map)?;
                    let map_file = map_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output.js.map");
                    js.push_str("\n//# sourceMappingURL=");
                    js.push_str(map_file);
                    js.push('\n');
                    println!("Wrote {}", map_path.display());
                }
            }

            fs::write(&output_path, &js)?;
            println!("Wrote {}", output_path.display());

            if let Some(dts) = artifacts.declarations {
                let dts_path = output_path.with_extension("d.ts");
                fs::write(&dts_path, &dts)?;
                println!("Wrote {}", dts_path.display());
            }
        }
        Target::Wasm => {
            println!("Generating WebAssembly...");
            let wasm_bytes = artifacts.wasm.unwrap_or_default();
            let output_path = output
                .cloned()
                .unwrap_or_else(|| PathBuf::from("output.wasm"));
            fs::write(&output_path, &wasm_bytes)?;
            println!("Wrote {}", output_path.display());

            let host_path = output_path.with_extension("host.mjs");
            if let Some(host_js) = artifacts.wasm_host_js {
                fs::write(&host_path, host_js)?;
                println!("Wrote {}", host_path.display());
            }

            if let Some(loader) = artifacts.wasm_loader_js {
                let wasm_file = output_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output.wasm");
                let host_file = host_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output.host.mjs");
                let loader_path = output_path.with_extension("mjs");
                let loader_contents = loader
                    .replace("__WASM_FILE__", wasm_file)
                    .replace("__HOST_FILE__", host_file);
                fs::write(&loader_path, loader_contents)?;
                println!("Wrote {}", loader_path.display());
            }

            if let Some(wat) = artifacts.wat {
                let wat_path = output_path.with_extension("wat");
                fs::write(&wat_path, &wat)?;
                println!("Wrote {}", wat_path.display());
                println!("\nWAT output:\n{}", wat);
            }
        }
        Target::Native => {
            println!("Generating native binary...");

            let triple = match &options.target_triple {
                Some(t) => argon_target::TargetTriple::parse(t)
                    .map_err(|e| format!("Invalid target triple: {}", e))?,
                None => argon_target::TargetTriple::host(),
            };

            let obj_bytes = artifacts.native_obj.unwrap_or_default();

            match emit {
                EmitKind::Obj => {
                    let output_path = output.cloned().unwrap_or_else(|| {
                        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                        PathBuf::from(format!("{}{}", stem, triple.obj_suffix()))
                    });
                    fs::write(&output_path, &obj_bytes)?;
                    println!("Wrote {}", output_path.display());
                }
                EmitKind::Asm => {
                    let output_path = output.cloned().unwrap_or_else(|| {
                        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                        PathBuf::from(format!("{}.s", stem))
                    });
                    if let Some(asm) = artifacts.native_asm {
                        fs::write(&output_path, &asm)?;
                        println!("Wrote {}", output_path.display());
                    } else {
                        return Err("assembly output not available".into());
                    }
                }
                EmitKind::Exe => {
                    let output_path = output.cloned().unwrap_or_else(|| {
                        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                        PathBuf::from(format!("{}{}", stem, triple.exe_suffix()))
                    });

                    let tmp_dir = std::env::temp_dir();

                    // Write object file to temp location
                    let obj_path = tmp_dir.join(format!(
                        "argon_{}{}",
                        std::process::id(),
                        triple.obj_suffix()
                    ));
                    fs::write(&obj_path, &obj_bytes)?;

                    // Compile the C runtime helpers
                    let runtime_obj_path =
                        argon_codegen_native::compile_c_runtime(&tmp_dir, &triple)
                            .map_err(|e| format!("{}", e))?;

                    // Link both objects
                    let linker_config = argon_codegen_native::LinkerConfig {
                        triple: triple.clone(),
                        output: output_path.clone(),
                        objects: vec![obj_path.clone(), runtime_obj_path.clone()],
                    };
                    argon_codegen_native::link(&linker_config).map_err(|e| {
                        let _ = fs::remove_file(&obj_path);
                        let _ = fs::remove_file(&runtime_obj_path);
                        format!("{}", e)
                    })?;

                    let _ = fs::remove_file(&obj_path);
                    let _ = fs::remove_file(&runtime_obj_path);

                    // Set executable permission on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755))?;
                    }

                    println!("Wrote {}", output_path.display());
                }
            }
        }
    }

    println!("Done!");
    Ok(())
}

fn check(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let session = CompilationSession::new();
    check_with_session(&session, input)
}

fn check_with_session(
    session: &CompilationSession,
    input: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing {}...", input.display());
    println!("Type checking...");
    println!("Borrow checking...");

    if let Err(e) = session.check_file(input) {
        if let Some(diag) = e.diagnostics() {
            eprintln!("{}", diag.rendered);
        }
        return Err(e.into());
    }

    println!("OK!");
    Ok(())
}

fn run(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let session = CompilationSession::new();
    run_with_session(&session, input)
}

fn run_with_session(
    session: &CompilationSession,
    input: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing {}...", input.display());
    println!("Type checking...");
    println!("Borrow checking...");

    let checked = session.check_file(input).map_err(|e| {
        if let Some(diag) = e.diagnostics() {
            eprintln!("{}", diag.rendered);
        }
        e
    })?;

    execute_checked_ast(&checked.ast)
}

fn execute_checked_ast(ast: &argon_ast::SourceFile) -> Result<(), Box<dyn std::error::Error>> {
    println!("Executing...\n");
    match execute_ast(ast) {
        Ok(value) => {
            println!("=> {:?}", value);
        }
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            return Err(format!("Runtime error: {}", e).into());
        }
    }
    Ok(())
}

fn test(
    input: Option<&PathBuf>,
    directory: Option<&PathBuf>,
    verbose: bool,
    filter: Option<&String>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut test_files: Vec<PathBuf> = Vec::new();

    if let Some(input_path) = input {
        if input_path.is_file() {
            test_files.push(input_path.clone());
        }
    }
    if let Some(dir) = directory {
        if dir.is_dir() {
            collect_test_files(dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        let tests_dir = PathBuf::from("tests");
        if tests_dir.is_dir() {
            collect_test_files(&tests_dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        let fixtures_dir = PathBuf::from("tests/fixtures");
        if fixtures_dir.is_dir() {
            collect_test_files(&fixtures_dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        return Err("No .test.arg files found. Use --input or --directory.".into());
    }

    if verbose {
        println!("Found {} test file(s)\n", test_files.len());
    }

    let compiler = Compiler::new();
    let mut all_outcomes: Vec<TestOutcome> = Vec::new();
    let mut total_suites = 0;
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_skipped = 0usize;
    let mut total_duration = 0f64;

    for test_file in &test_files {
        let file_name = test_file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let source = fs::read_to_string(test_file)?;
        let source_name = test_file.display().to_string();

        let mut ast = compiler.parse(&source, &source_name)
            .map_err(|e| {
                if let Some(diag) = e.diagnostics() {
                    format!("Parse error in {}: {}", file_name, diag.rendered)
                } else {
                    format!("Parse error in {}: {}", file_name, e)
                }
            })?;

        let tc_output = compiler.type_check_output(&source, &source_name, &ast)
            .map_err(|e| {
                if let Some(diag) = e.diagnostics() {
                    format!("Type error in {}: {}", file_name, diag.rendered)
                } else {
                    format!("Type error in {}: {}", file_name, e)
                }
            })?;

        // Desugar named args (mutates ast in-place)
        desugar_named_args(&mut ast, &tc_output.env);

        let mut runtime = Runtime::new();
        runtime.execute(&ast)
            .map_err(|e| format!("Runtime error in {}: {}", file_name, e))?;

        let mut results = runtime.run_all_suites();

        // Apply filter
        if let Some(ref pattern) = filter {
            let p = pattern.to_lowercase();
            results.outcomes.retain(|o| {
                format!("{} > {}", o.suite_name(), o.test_name())
                    .to_lowercase()
                    .contains(&p)
            });
            // Recompute counts after filtering
            results.total_tests = results.outcomes.len();
            results.passed = results.outcomes.iter().filter(|o| matches!(o, TestOutcome::Pass { .. })).count();
            results.failed = results.outcomes.iter().filter(|o| matches!(o, TestOutcome::Fail { .. })).count();
            results.skipped = results.outcomes.iter().filter(|o| matches!(o, TestOutcome::Skip { .. })).count();
            results.total_suites = results.outcomes.iter()
                .map(|o| o.suite_name())
                .collect::<std::collections::HashSet<_>>()
                .len();
        }

        if verbose && results.total_suites == 0 {
            println!("  {} — no suites found (warning)", file_name);
        }

        total_suites += results.total_suites;
        total_passed += results.passed;
        total_failed += results.failed;
        total_skipped += results.skipped;
        total_duration += results.duration_ms;
        all_outcomes.append(&mut results.outcomes);
    }

    let summary = TestResults {
        outcomes: all_outcomes,
        total_suites,
        total_tests: total_passed + total_failed + total_skipped,
        passed: total_passed,
        failed: total_failed,
        skipped: total_skipped,
        duration_ms: total_duration,
    };

    let output = match format {
        "tap" => format_tap(&summary),
        "json" => format_json(&summary),
        _ => format_pretty(&summary),
    };
    println!("{}", output);

    if total_failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn collect_test_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".test.arg") {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            collect_test_files(&path, files)?;
        }
    }
    Ok(())
}

fn format_file(
    input: &PathBuf,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    let mut formatted = String::new();
    let mut indent_level: i32 = 0;
    let mut in_string = false;
    let mut prev_char = ' ';

    for c in source.chars() {
        if c == '"' && prev_char != '\\' {
            in_string = !in_string;
        }

        if !in_string {
            if c == '{' {
                formatted.push(c);
                formatted.push('\n');
                indent_level += 1;
                for _ in 0..indent_level {
                    formatted.push_str("    ");
                }
                prev_char = c;
                continue;
            }
            if c == '}' {
                indent_level = indent_level.saturating_sub(1);
                formatted.push('\n');
                for _ in 0..indent_level {
                    formatted.push_str("    ");
                }
                formatted.push(c);
                prev_char = c;
                continue;
            }
            if c == '\n' {
                formatted.push(c);
                for _ in 0..indent_level {
                    formatted.push_str("    ");
                }
                prev_char = c;
                continue;
            }
            if c == ';' {
                formatted.push(c);
                formatted.push('\n');
                for _ in 0..indent_level {
                    formatted.push_str("    ");
                }
                prev_char = c;
                continue;
            }
        }

        formatted.push(c);
        prev_char = c;
    }

    let output_path = output.cloned().unwrap_or_else(|| input.clone());

    fs::write(&output_path, &formatted)?;
    println!("Formatted {}", output_path.display());
    Ok(())
}

fn init_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::current_dir()?.join(name);

    if dir.exists() {
        return Err("Directory already exists".into());
    }

    fs::create_dir_all(&dir)?;
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("dist"))?;

    fs::write(
        dir.join("package.json"),
        format!(
            r#"{{
  "name": "{}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "argon compile src/main.arg -o dist/main.js",
    "dev": "argon watch src/main.arg"
  }},
  "devDependencies": {{
    "argon": "^0.1.0"
  }}
}}"#,
            name
        ),
    )?;

    fs::write(
        dir.join("src/main.arg"),
        r#"// Welcome to Argon!

function main(): void {
    println("Hello, Argon!");
}

main();
"#,
    )?;

    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "strict": true,
    "esModuleInterop": true
  }
}
"#,
    )?;

    println!("Initialized Argon project in {}", name);
    println!("Run 'cd {} && npm install' to get started", name);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn watch(
    input: &PathBuf,
    output: Option<&PathBuf>,
    target: &str,
    source_map: bool,
    opt: bool,
    declarations: bool,
    pipeline: &str,
    check_only: bool,
    native_opt: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use notify::{Config, RecommendedWatcher, Watcher};
    use std::sync::mpsc;
    use std::time::Duration;

    let session = CompilationSession::new();
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    let mut watched_paths = HashSet::new();
    sync_watch_paths(&session, &mut watcher, input, &mut watched_paths)?;

    println!("Watching {} (Ctrl-C to stop)", input.display());

    // Run once immediately.
    if check_only {
        let _ = check_with_session(&session, input);
    } else {
        let _ = compile_with_session(
            &session,
            input,
            output,
            None,
            target,
            source_map,
            opt,
            declarations,
            pipeline,
            None,
            "exe",
            native_opt,
        );
    }
    sync_watch_paths(&session, &mut watcher, input, &mut watched_paths)?;

    loop {
        // Wait for at least one event, then debounce by draining the queue.
        let _ = rx.recv()?;
        std::thread::sleep(Duration::from_millis(75));
        while rx.try_recv().is_ok() {}

        println!("\nChange detected. Rebuilding...\n");
        if check_only {
            if let Err(e) = check_with_session(&session, input) {
                eprintln!("{}", e);
            }
        } else if let Err(e) = compile_with_session(
            &session,
            input,
            output,
            None,
            target,
            source_map,
            opt,
            declarations,
            pipeline,
            None,
            "exe",
            native_opt,
        ) {
            eprintln!("{}", e);
        }
        sync_watch_paths(&session, &mut watcher, input, &mut watched_paths)?;
    }
}

fn sync_watch_paths(
    session: &CompilationSession,
    watcher: &mut notify::RecommendedWatcher,
    entry: &Path,
    watched_paths: &mut HashSet<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use notify::{RecursiveMode, Watcher};

    let mut desired = HashSet::new();
    if let Ok(canonical) = std::fs::canonicalize(entry) {
        desired.insert(canonical);
    } else {
        desired.insert(entry.to_path_buf());
    }

    if let Ok(files) = session.project_files(entry) {
        desired.extend(files);
    }

    for path in watched_paths.difference(&desired) {
        let _ = watcher.unwatch(path);
    }

    for path in desired.difference(watched_paths) {
        watcher.watch(path, RecursiveMode::NonRecursive)?;
    }

    *watched_paths = desired;
    Ok(())
}

fn repl() -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::error::ReadlineError;
    use rustyline::DefaultEditor;

    let mut rl = DefaultEditor::new()?;
    let compiler = Compiler::new();

    let mut buffer = String::new();
    println!("Argon REPL");
    println!("Commands: :load <file>, :show, :reset, :check, :compile [js|wasm], :exit");

    loop {
        let line = match rl.readline("argon> ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(e.into()),
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        rl.add_history_entry(trimmed)?;

        if let Some(cmd) = trimmed.strip_prefix(':') {
            let mut parts = cmd.split_whitespace();
            let name = parts.next().unwrap_or("");
            match name {
                "exit" | "quit" => break,
                "reset" => {
                    buffer.clear();
                    println!("(buffer cleared)");
                }
                "show" => {
                    println!("{}", buffer);
                }
                "load" => {
                    let path = parts.next().ok_or("usage: :load <file>")?;
                    buffer = fs::read_to_string(path)?;
                    println!("Loaded {}", path);
                }
                "check" => {
                    let source_name = "<repl>";
                    match compiler.parse(&buffer, source_name) {
                        Ok(ast) => {
                            if let Err(e) = compiler.check_semantics(&buffer, source_name, &ast) {
                                if let Some(diag) = e.diagnostics() {
                                    eprintln!("{}", diag.rendered);
                                }
                                continue;
                            }
                            println!("OK");
                        }
                        Err(e) => {
                            if let Some(diag) = e.diagnostics() {
                                eprintln!("{}", diag.rendered);
                            } else {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
                "compile" => {
                    let target = parts.next().unwrap_or("js");
                    let target = match target {
                        "js" => Target::Js,
                        "wasm" => Target::Wasm,
                        other => {
                            eprintln!("Unknown target: {}", other);
                            continue;
                        }
                    };
                    let options = CompileOptions {
                        target,
                        pipeline: Pipeline::Ir,
                        optimize: false,
                        source_map: false,
                        declarations: false,
                        emit_wat: target == Target::Wasm,
                        ..Default::default()
                    };

                    let source_name = "<repl>";
                    match compiler.compile(&buffer, source_name, &options) {
                        Ok(artifacts) => match target {
                            Target::Js => {
                                let js = artifacts.js.unwrap_or_default();
                                println!("{}", js);
                            }
                            Target::Wasm => {
                                let wat =
                                    artifacts.wat.unwrap_or_else(|| "<wat unavailable>".into());
                                println!("{}", wat);
                            }
                            Target::Native => {
                                if let Some(obj) = &artifacts.native_obj {
                                    println!("<native object: {} bytes>", obj.len());
                                } else {
                                    println!("<no native output>");
                                }
                            }
                        },
                        Err(e) => {
                            if let Some(diag) = e.diagnostics() {
                                eprintln!("{}", diag.rendered);
                            } else {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Unknown command: :{}", name);
                }
            }
            continue;
        }

        buffer.push_str(trimmed);
        buffer.push('\n');
    }

    Ok(())
}
