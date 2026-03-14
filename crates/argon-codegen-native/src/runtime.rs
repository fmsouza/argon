//! Native runtime entry point generation and C runtime compilation.
//!
//! Generates the `main` function wrapper that serves as the executable's
//! entry point. This calls `__argon_init` (the user's top-level code).
//!
//! Also provides a small C runtime with formatting helpers that avoid
//! variadic calling convention issues on aarch64.

use crate::CodegenError;
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use std::path::{Path, PathBuf};

/// Declare and define the C-ABI `main` function that wraps `__argon_init`.
pub fn define_main_wrapper<M: Module>(
    module: &mut M,
    argon_init_id: Option<FuncId>,
) -> Result<FuncId, CodegenError> {
    // Declare main(argc, argv) -> i32
    let mut main_sig = module.make_signature();
    main_sig.params.push(AbiParam::new(types::I32)); // argc
    main_sig
        .params
        .push(AbiParam::new(cranelift_codegen::ir::types::I64)); // argv
    main_sig.returns.push(AbiParam::new(types::I32)); // exit code

    let main_id = module
        .declare_function("main", Linkage::Export, &main_sig)
        .map_err(|e| CodegenError::CraneliftError(format!("failed to declare main: {}", e)))?;

    let mut ctx = module.make_context();
    ctx.func.signature = main_sig;

    let mut fbc = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fbc);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Call __argon_init if it exists
        if let Some(init_id) = argon_init_id {
            let init_ref = module.declare_func_in_func(init_id, builder.func);
            builder.ins().call(init_ref, &[]);
        }

        // return 0
        let zero = builder.ins().iconst(types::I32, 0);
        builder.ins().return_(&[zero]);

        builder.finalize();
    }

    module
        .define_function(main_id, &mut ctx)
        .map_err(|e| CodegenError::CraneliftError(format!("failed to define main: {}", e)))?;

    module.clear_context(&mut ctx);

    Ok(main_id)
}

/// C source for the Argon runtime helpers.
/// These are non-variadic wrappers that handle number formatting correctly
/// across all architectures (including aarch64 where variadic float args
/// are passed differently).
const C_RUNTIME_SOURCE: &str = r#"
#include <stdio.h>
#include <unistd.h>
#include <math.h>
#include <string.h>

void __argon_print_f64(double value) {
    char buf[64];
    int n;
    if (value == floor(value) && fabs(value) < 1e15 && fabs(value) >= 0) {
        n = snprintf(buf, sizeof(buf), "%.0f", value);
    } else {
        n = snprintf(buf, sizeof(buf), "%g", value);
    }
    write(1, buf, n);
}

void __argon_print_str(const char *s, long len) {
    write(1, s, len);
}

void __argon_print_bool(double value) {
    if (value != 0.0) {
        write(1, "true", 4);
    } else {
        write(1, "false", 5);
    }
}
"#;

/// Compile the C runtime to an object file for the given target.
/// Returns the path to the generated .o file.
pub fn compile_c_runtime(target_dir: &Path) -> Result<PathBuf, CodegenError> {
    let c_path = target_dir.join("__argon_runtime.c");
    let o_path = target_dir.join("__argon_runtime.o");

    std::fs::write(&c_path, C_RUNTIME_SOURCE)
        .map_err(|e| CodegenError::CraneliftError(format!("failed to write C runtime: {}", e)))?;

    let output = std::process::Command::new("cc")
        .args(["-c", "-O2", "-o"])
        .arg(&o_path)
        .arg(&c_path)
        .output()
        .map_err(|e| {
            CodegenError::LinkerError(format!(
                "failed to compile C runtime (is 'cc' installed?): {}",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::LinkerError(format!(
            "failed to compile C runtime:\n{}",
            stderr
        )));
    }

    // Clean up C source
    let _ = std::fs::remove_file(&c_path);

    Ok(o_path)
}
