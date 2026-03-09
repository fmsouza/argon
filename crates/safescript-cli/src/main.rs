//! SafeScript CLI

use safescript_borrowck::BorrowChecker;
use safescript_codegen_js::{generate_type_declarations, JsCodegen};
use safescript_parser::parse;
use safescript_types::TypeChecker;
use std::fs;
use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(name = "safescript")]
#[command(about = "SafeScript compiler", long_about = None)]
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
    },
    Check {
        input: PathBuf,
    },
    Run {
        input: PathBuf,
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
            source_map: _,
            declarations,
        } => {
            compile(&input, output.as_ref(), &target, declarations)?;
        }
        Commands::Check { input } => {
            check(&input)?;
        }
        Commands::Run { input } => {
            run(&input)?;
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
    declarations: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    println!("Parsing {}...", input.display());
    let ast = parse(&source)?;

    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    type_checker.check(&ast)?;

    if target == "js" {
        println!("Generating JavaScript...");
        let mut codegen = JsCodegen::new();
        let js = codegen.generate_from_ast(&ast).unwrap_or_default();

        let output_path = output
            .map(|p| p.clone())
            .unwrap_or_else(|| PathBuf::from("output.js"));
        fs::write(&output_path, &js)?;
        println!("Wrote {}", output_path.display());

        if declarations {
            let dts = generate_type_declarations(&ast);
            let dts_path = output_path.with_extension("d.ts");
            fs::write(&dts_path, &dts)?;
            println!("Wrote {}", dts_path.display());
        }
    } else if target == "wasm" {
        println!("WASM generation not yet implemented");
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
    compile(input, None, "js", false)?;
    println!("\nRunning output.js...");
    println!("(Execution not implemented - run the generated JS file with Node.js)");
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
    "build": "safescript compile src/main.ss -o dist/main.js",
    "dev": "safescript watch src/main.ss"
  }},
  "devDependencies": {{
    "safescript": "^0.1.0"
  }}
}}"#,
            name
        ),
    )?;

    fs::write(
        dir.join("src/main.ss"),
        r#"// Welcome to SafeScript!

function main(): void {
    console.log("Hello, SafeScript!");
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

    println!("Initialized SafeScript project in {}", name);
    println!("Run 'cd {} && npm install' to get started", name);

    Ok(())
}
