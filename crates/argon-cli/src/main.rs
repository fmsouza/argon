//! Argon CLI

use argon_borrowck::BorrowChecker;
use argon_codegen_js::{generate_type_declarations, JsCodegen};
use argon_codegen_wasm::WasmCodegen;
use argon_ir::IrBuilder;
use argon_parser::parse;
use argon_runtime::execute_ast;
use argon_types::TypeChecker;
use std::fs;
use std::path::PathBuf;

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
        #[arg(short, long, default_value = "js")]
        target: String,
        #[arg(short, long)]
        source_map: bool,
        #[arg(long)]
        declarations: bool,
        #[arg(long, default_value = "ir")]
        pipeline: String,
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
        #[arg(long, default_value = "ir")]
        pipeline: String,
    },
    Format {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Init {
        name: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli: Cli = clap::Parser::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            target,
            source_map,
            declarations,
            pipeline,
        } => {
            compile(
                &input,
                output.as_ref(),
                &target,
                source_map,
                declarations,
                &pipeline,
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
            pipeline,
        } => {
            test(input.as_ref(), directory.as_ref(), verbose, &pipeline)?;
        }
        Commands::Format { input, output } => {
            format_file(&input, output.as_ref())?;
        }
        Commands::Init { name } => {
            init_project(&name)?;
        }
    }

    Ok(())
}

fn compile(
    input: &PathBuf,
    output: Option<&PathBuf>,
    target: &str,
    source_map: bool,
    declarations: bool,
    pipeline: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    println!("Parsing {}...", input.display());
    let ast = parse(&source)?;

    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    type_checker.check(&ast)?;

    println!("Borrow checking...");
    let mut borrow_checker = BorrowChecker::new();
    borrow_checker.check(&ast)?;

    if target == "js" {
        println!("Generating JavaScript...");
        let output_path = output
            .map(|p| p.clone())
            .unwrap_or_else(|| PathBuf::from("output.js"));

        let source_name = input.display().to_string();
        let mut codegen = if source_map {
            JsCodegen::new().with_source_map(&source_name)
        } else {
            JsCodegen::new()
        };

        let mut js = if pipeline == "ast" {
            codegen.generate_from_ast(&ast)?
        } else {
            let mut builder = IrBuilder::new();
            let ir = builder.build(&ast)?;
            codegen.generate(&ir)?
        };

        if source_map {
            let ext = output_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("js");
            let map_path = output_path.with_extension(format!("{}.map", ext));
            if let Some(map) = codegen.get_source_map() {
                fs::write(&map_path, &map)?;
                let map_file = map_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output.js.map");
                js.push_str("\n//# sourceMappingURL=");
                js.push_str(map_file);
                js.push_str("\n");
                println!("Wrote {}", map_path.display());
            }
        }

        fs::write(&output_path, &js)?;
        println!("Wrote {}", output_path.display());

        if declarations {
            let dts = generate_type_declarations(&ast);
            let dts_path = output_path.with_extension("d.ts");
            fs::write(&dts_path, &dts)?;
            println!("Wrote {}", dts_path.display());
        }
    } else if target == "wasm" {
        println!("Generating WebAssembly...");
        let mut codegen = WasmCodegen::new();
        let wasm_result = if pipeline == "ast" {
            codegen.generate_from_ast(&ast)
        } else {
            let mut builder = IrBuilder::new();
            let ir = builder.build(&ast)?;
            codegen.generate(&ir)
        };

        match wasm_result {
            Ok(wasm_bytes) => {
                let output_path = output
                    .map(|p| p.clone())
                    .unwrap_or_else(|| PathBuf::from("output.wasm"));
                fs::write(&output_path, &wasm_bytes)?;
                println!("Wrote {}", output_path.display());

                match wasmprinter::print_bytes(&wasm_bytes) {
                    Ok(wat) => {
                        let wat_path = output_path.with_extension("wat");
                        fs::write(&wat_path, &wat)?;
                        println!("Wrote {}", wat_path.display());
                        println!("\nWAT output:\n{}", wat);
                    }
                    Err(e) => {
                        println!("Note: Could not generate .wat file: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("WASM generation error: {}", e);
                return Err(format!("WASM generation error: {}", e).into());
            }
        }
    } else {
        println!("Unknown target: {}", target);
    }

    println!("Done!");
    Ok(())
}

fn check(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    println!("Parsing {}...", input.display());
    let ast = parse(&source)?;

    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    type_checker.check(&ast)?;

    println!("Borrow checking...");
    let mut borrow_checker = BorrowChecker::new();
    borrow_checker.check(&ast)?;

    println!("OK!");
    Ok(())
}

fn run(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    println!("Parsing {}...", input.display());
    let ast = parse(&source)?;

    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    type_checker.check(&ast)?;

    println!("Borrow checking...");
    let mut borrow_checker = BorrowChecker::new();
    borrow_checker.check(&ast)?;

    println!("Executing...\n");
    match execute_ast(&ast) {
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
    pipeline: &str,
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
        return Err(
            "No test files found. Use --input or --directory to specify test files.".into(),
        );
    }

    println!("Running Argon tests...\n");

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for test_file in &test_files {
        let file_name = test_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if !is_test_file(file_name) {
            skipped += 1;
            if verbose {
                println!("  Skipping {} (not a test file)", file_name);
            }
            continue;
        }

        print!("  {} ... ", file_name);

        match run_test_file_with_pipeline(test_file, verbose, pipeline) {
            Ok(true) => {
                println!("ok");
                passed += 1;
            }
            Ok(false) => {
                println!("FAILED");
                failed += 1;
            }
            Err(e) => {
                println!("ERROR: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n========================================");
    println!("Test Summary:");
    println!("  Passed:  {}", passed);
    println!("  Failed:  {}", failed);
    println!("  Skipped: {}", skipped);
    println!("========================================");

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn is_test_file(name: &str) -> bool {
    // Minimal convention: any `.arg` file is runnable as a test; naming can be layered on later.
    name.ends_with(".arg")
}

fn collect_test_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "arg" || ext == "ss" {
                        files.push(path);
                    }
                }
            } else if path.is_dir() {
                collect_test_files(&path, files)?;
            }
    }
    Ok(())
}

fn run_test_file_with_pipeline(
    test_file: &PathBuf,
    verbose: bool,
    pipeline: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(test_file)?;

    let ast = parse(&source).map_err(|e| format!("Parse error: {}", e))?;

    let mut type_checker = TypeChecker::new();
    type_checker
        .check(&ast)
        .map_err(|e| format!("Type error: {}", e))?;

    let mut borrow_checker = BorrowChecker::new();
    borrow_checker
        .check(&ast)
        .map_err(|e| format!("Borrow error: {}", e))?;

    let js = if pipeline == "ast" {
        let mut codegen = JsCodegen::new();
        codegen
            .generate_from_ast(&ast)
            .map_err(|e| format!("Codegen error: {}", e))?
    } else {
        let mut builder = IrBuilder::new();
        let ir = builder
            .build(&ast)
            .map_err(|e| format!("IR error: {}", e))?;
        let mut codegen = JsCodegen::new();
        codegen
            .generate(&ir)
            .map_err(|e| format!("Codegen error: {}", e))?
    };

    let temp_file = std::env::temp_dir().join("argon_test.js");
    fs::write(&temp_file, &js)?;

    let output = std::process::Command::new("node")
        .arg(&temp_file)
        .output()
        .map_err(|e| format!("Failed to run test: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if verbose {
            println!("\n    Error output: {}", stderr);
        }
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if verbose {
        print!("\n    Output: {}", stdout);
    }

    let _ = fs::remove_file(&temp_file);

    Ok(true)
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

    let output_path = output.map(|p| p.clone()).unwrap_or_else(|| input.clone());

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
    fs::create_dir_all(&dir.join("src"))?;
    fs::create_dir_all(&dir.join("dist"))?;

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
    console.log("Hello, Argon!");
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
