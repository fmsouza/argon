//! SafeScript CLI

use safescript_borrowck::BorrowChecker;
use safescript_codegen_js::JsCodegen;
use safescript_ir::IrBuilder;
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
    },
    Check {
        input: PathBuf,
    },
    Run {
        input: PathBuf,
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
        } => {
            compile(&input, output.as_ref(), &target, source_map)?;
        }
        Commands::Check { input } => {
            check(&input)?;
        }
        Commands::Run { input } => {
            run(&input)?;
        }
    }

    Ok(())
}

fn compile(
    input: &PathBuf,
    output: Option<&PathBuf>,
    target: &str,
    source_map: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;

    println!("Parsing {}...", input.display());
    let ast = parse(&source)?;

    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    type_checker.check(&ast)?;

    println!("Building IR...");
    let mut ir_builder = IrBuilder::new();
    let ir = ir_builder.build(&ast)?;

    if target == "js" {
        println!("Generating JavaScript...");
        let mut codegen = JsCodegen::new();
        let js = codegen
            .generate_from_ast(&ast)
            .unwrap_or_else(|_| codegen.generate(&ir).unwrap_or_default());

        let output_path = output
            .map(|p| p.clone())
            .unwrap_or_else(|| PathBuf::from("output.js"));
        fs::write(&output_path, &js)?;
        println!("Wrote {}", output_path.display());
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
