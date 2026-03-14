//! Native implementations of Argon stdlib intrinsics.
//!
//! Maps `std:io` and `std:math` intrinsics to libc calls or
//! native Cranelift instructions.

use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, Type};
use cranelift_module::{Linkage, Module};

/// Declare libc functions that intrinsics depend on.
pub fn declare_libc_functions<M: Module>(
    module: &mut M,
    pointer_type: Type,
) -> Result<LibcFunctions, cranelift_module::ModuleError> {
    // int write(int fd, const void *buf, size_t count)
    let mut write_sig = module.make_signature();
    write_sig.params.push(AbiParam::new(types::I32)); // fd
    write_sig.params.push(AbiParam::new(pointer_type)); // buf
    write_sig.params.push(AbiParam::new(pointer_type)); // count
    write_sig.returns.push(AbiParam::new(pointer_type)); // return
    let write_fn = module.declare_function("write", Linkage::Import, &write_sig)?;

    // void *malloc(size_t size)
    let mut malloc_sig = module.make_signature();
    malloc_sig.params.push(AbiParam::new(pointer_type)); // size
    malloc_sig.returns.push(AbiParam::new(pointer_type)); // ptr
    let malloc_fn = module.declare_function("malloc", Linkage::Import, &malloc_sig)?;

    // void free(void *ptr)
    let mut free_sig = module.make_signature();
    free_sig.params.push(AbiParam::new(pointer_type)); // ptr
    let free_fn = module.declare_function("free", Linkage::Import, &free_sig)?;

    // void *realloc(void *ptr, size_t size)
    let mut realloc_sig = module.make_signature();
    realloc_sig.params.push(AbiParam::new(pointer_type)); // ptr
    realloc_sig.params.push(AbiParam::new(pointer_type)); // size
    realloc_sig.returns.push(AbiParam::new(pointer_type)); // ptr
    let realloc_fn = module.declare_function("realloc", Linkage::Import, &realloc_sig)?;

    // double sin(double x)
    let mut sin_sig = module.make_signature();
    sin_sig.params.push(AbiParam::new(types::F64));
    sin_sig.returns.push(AbiParam::new(types::F64));
    let sin_fn = module.declare_function("sin", Linkage::Import, &sin_sig)?;

    // double cos(double x)
    let mut cos_sig = module.make_signature();
    cos_sig.params.push(AbiParam::new(types::F64));
    cos_sig.returns.push(AbiParam::new(types::F64));
    let cos_fn = module.declare_function("cos", Linkage::Import, &cos_sig)?;

    // double tan(double x)
    let mut tan_sig = module.make_signature();
    tan_sig.params.push(AbiParam::new(types::F64));
    tan_sig.returns.push(AbiParam::new(types::F64));
    let tan_fn = module.declare_function("tan", Linkage::Import, &tan_sig)?;

    // double pow(double base, double exp)
    let mut pow_sig = module.make_signature();
    pow_sig.params.push(AbiParam::new(types::F64));
    pow_sig.params.push(AbiParam::new(types::F64));
    pow_sig.returns.push(AbiParam::new(types::F64));
    let pow_fn = module.declare_function("pow", Linkage::Import, &pow_sig)?;

    // double log(double x)
    let mut log_sig = module.make_signature();
    log_sig.params.push(AbiParam::new(types::F64));
    log_sig.returns.push(AbiParam::new(types::F64));
    let log_fn = module.declare_function("log", Linkage::Import, &log_sig)?;

    // double exp(double x)
    let mut exp_sig = module.make_signature();
    exp_sig.params.push(AbiParam::new(types::F64));
    exp_sig.returns.push(AbiParam::new(types::F64));
    let exp_fn = module.declare_function("exp", Linkage::Import, &exp_sig)?;

    // int snprintf(char *str, size_t size, const char *format, ...)
    // We declare this with a variadic-like signature for formatting numbers.
    let mut snprintf_sig = module.make_signature();
    snprintf_sig.params.push(AbiParam::new(pointer_type)); // buf
    snprintf_sig.params.push(AbiParam::new(pointer_type)); // size
    snprintf_sig.params.push(AbiParam::new(pointer_type)); // format
    snprintf_sig.params.push(AbiParam::new(types::F64)); // value (we format f64)
    snprintf_sig.returns.push(AbiParam::new(types::I32)); // chars written
    let snprintf_fn = module.declare_function("snprintf", Linkage::Import, &snprintf_sig)?;

    // void __argon_print_f64(double value) — non-variadic, from C runtime
    let mut print_f64_sig = module.make_signature();
    print_f64_sig.params.push(AbiParam::new(types::F64)); // value
    let print_f64_fn =
        module.declare_function("__argon_print_f64", Linkage::Import, &print_f64_sig)?;

    // void __argon_print_str(const char *s, long len) — from C runtime
    let mut print_str_sig = module.make_signature();
    print_str_sig.params.push(AbiParam::new(pointer_type)); // str ptr
    print_str_sig.params.push(AbiParam::new(pointer_type)); // len
    let print_str_fn =
        module.declare_function("__argon_print_str", Linkage::Import, &print_str_sig)?;

    // void __argon_print_bool(double value) — from C runtime
    let mut print_bool_sig = module.make_signature();
    print_bool_sig.params.push(AbiParam::new(types::F64)); // value
    let print_bool_fn =
        module.declare_function("__argon_print_bool", Linkage::Import, &print_bool_sig)?;

    Ok(LibcFunctions {
        write: write_fn,
        malloc: malloc_fn,
        free: free_fn,
        realloc: realloc_fn,
        sin: sin_fn,
        cos: cos_fn,
        tan: tan_fn,
        pow: pow_fn,
        log: log_fn,
        exp: exp_fn,
        snprintf: snprintf_fn,
        print_f64: print_f64_fn,
        print_str: print_str_fn,
        print_bool: print_bool_fn,
    })
}

/// Handles to declared libc functions.
#[allow(dead_code)]
pub struct LibcFunctions {
    pub write: cranelift_module::FuncId,
    pub malloc: cranelift_module::FuncId,
    pub free: cranelift_module::FuncId,
    pub realloc: cranelift_module::FuncId,
    pub sin: cranelift_module::FuncId,
    pub cos: cranelift_module::FuncId,
    pub tan: cranelift_module::FuncId,
    pub pow: cranelift_module::FuncId,
    pub log: cranelift_module::FuncId,
    pub exp: cranelift_module::FuncId,
    pub snprintf: cranelift_module::FuncId,
    pub print_f64: cranelift_module::FuncId,
    pub print_str: cranelift_module::FuncId,
    pub print_bool: cranelift_module::FuncId,
}

/// Check if a function name is a known intrinsic.
#[allow(dead_code)]
pub fn is_intrinsic(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "println"
            | "sqrt"
            | "cbrt"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "log"
            | "log2"
            | "log10"
            | "exp"
            | "pow"
            | "hypot"
            | "abs"
            | "floor"
            | "ceil"
            | "round"
            | "trunc"
            | "sign"
            | "min"
            | "max"
            | "clamp"
    )
}
