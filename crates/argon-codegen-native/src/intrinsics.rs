//! Native implementations of Argon stdlib intrinsics.
//!
//! Maps `std:io` and `std:math` intrinsics to libc calls or
//! native Cranelift instructions.

use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, Type};
use cranelift_module::{Linkage, Module};

/// Declare libc functions that intrinsics depend on.
#[allow(clippy::result_large_err)]
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

    // --- File system helpers from C runtime ---

    // char *__argon_fs_read_file(const char *path, long path_len, long *out_len)
    let mut fs_read_file_sig = module.make_signature();
    fs_read_file_sig.params.push(AbiParam::new(pointer_type)); // path
    fs_read_file_sig.params.push(AbiParam::new(pointer_type)); // path_len
    fs_read_file_sig.params.push(AbiParam::new(pointer_type)); // out_len ptr
    fs_read_file_sig.returns.push(AbiParam::new(pointer_type)); // buf ptr (NULL on error)
    let fs_read_file_fn =
        module.declare_function("__argon_fs_read_file", Linkage::Import, &fs_read_file_sig)?;

    // int __argon_fs_write_file(path, path_len, data, data_len)
    let mut fs_write_file_sig = module.make_signature();
    fs_write_file_sig.params.push(AbiParam::new(pointer_type)); // path
    fs_write_file_sig.params.push(AbiParam::new(pointer_type)); // path_len
    fs_write_file_sig.params.push(AbiParam::new(pointer_type)); // data
    fs_write_file_sig.params.push(AbiParam::new(pointer_type)); // data_len
    fs_write_file_sig.returns.push(AbiParam::new(types::I32)); // 0=ok, -1=err
    let fs_write_file_fn =
        module.declare_function("__argon_fs_write_file", Linkage::Import, &fs_write_file_sig)?;

    // int __argon_fs_append_file(path, path_len, data, data_len)
    let mut fs_append_file_sig = module.make_signature();
    fs_append_file_sig.params.push(AbiParam::new(pointer_type));
    fs_append_file_sig.params.push(AbiParam::new(pointer_type));
    fs_append_file_sig.params.push(AbiParam::new(pointer_type));
    fs_append_file_sig.params.push(AbiParam::new(pointer_type));
    fs_append_file_sig.returns.push(AbiParam::new(types::I32));
    let fs_append_file_fn = module.declare_function(
        "__argon_fs_append_file",
        Linkage::Import,
        &fs_append_file_sig,
    )?;

    // int __argon_fs_exists(path, path_len)
    let mut fs_exists_sig = module.make_signature();
    fs_exists_sig.params.push(AbiParam::new(pointer_type));
    fs_exists_sig.params.push(AbiParam::new(pointer_type));
    fs_exists_sig.returns.push(AbiParam::new(types::I32));
    let fs_exists_fn =
        module.declare_function("__argon_fs_exists", Linkage::Import, &fs_exists_sig)?;

    // long __argon_fs_file_size(path, path_len)
    let mut fs_file_size_sig = module.make_signature();
    fs_file_size_sig.params.push(AbiParam::new(pointer_type));
    fs_file_size_sig.params.push(AbiParam::new(pointer_type));
    fs_file_size_sig.returns.push(AbiParam::new(pointer_type));
    let fs_file_size_fn =
        module.declare_function("__argon_fs_file_size", Linkage::Import, &fs_file_size_sig)?;

    // int __argon_fs_is_file(path, path_len)
    let mut fs_is_file_sig = module.make_signature();
    fs_is_file_sig.params.push(AbiParam::new(pointer_type));
    fs_is_file_sig.params.push(AbiParam::new(pointer_type));
    fs_is_file_sig.returns.push(AbiParam::new(types::I32));
    let fs_is_file_fn =
        module.declare_function("__argon_fs_is_file", Linkage::Import, &fs_is_file_sig)?;

    // int __argon_fs_is_dir(path, path_len)
    let mut fs_is_dir_sig = module.make_signature();
    fs_is_dir_sig.params.push(AbiParam::new(pointer_type));
    fs_is_dir_sig.params.push(AbiParam::new(pointer_type));
    fs_is_dir_sig.returns.push(AbiParam::new(types::I32));
    let fs_is_dir_fn =
        module.declare_function("__argon_fs_is_dir", Linkage::Import, &fs_is_dir_sig)?;

    // int __argon_fs_remove(path, path_len)
    let mut fs_remove_sig = module.make_signature();
    fs_remove_sig.params.push(AbiParam::new(pointer_type));
    fs_remove_sig.params.push(AbiParam::new(pointer_type));
    fs_remove_sig.returns.push(AbiParam::new(types::I32));
    let fs_remove_fn =
        module.declare_function("__argon_fs_remove", Linkage::Import, &fs_remove_sig)?;

    // int __argon_fs_mkdir(path, path_len)
    let mut fs_mkdir_sig = module.make_signature();
    fs_mkdir_sig.params.push(AbiParam::new(pointer_type));
    fs_mkdir_sig.params.push(AbiParam::new(pointer_type));
    fs_mkdir_sig.returns.push(AbiParam::new(types::I32));
    let fs_mkdir_fn =
        module.declare_function("__argon_fs_mkdir", Linkage::Import, &fs_mkdir_sig)?;

    // int __argon_fs_rmdir(path, path_len)
    let mut fs_rmdir_sig = module.make_signature();
    fs_rmdir_sig.params.push(AbiParam::new(pointer_type));
    fs_rmdir_sig.params.push(AbiParam::new(pointer_type));
    fs_rmdir_sig.returns.push(AbiParam::new(types::I32));
    let fs_rmdir_fn =
        module.declare_function("__argon_fs_rmdir", Linkage::Import, &fs_rmdir_sig)?;

    // int __argon_fs_rename(from, from_len, to, to_len)
    let mut fs_rename_sig = module.make_signature();
    fs_rename_sig.params.push(AbiParam::new(pointer_type));
    fs_rename_sig.params.push(AbiParam::new(pointer_type));
    fs_rename_sig.params.push(AbiParam::new(pointer_type));
    fs_rename_sig.params.push(AbiParam::new(pointer_type));
    fs_rename_sig.returns.push(AbiParam::new(types::I32));
    let fs_rename_fn =
        module.declare_function("__argon_fs_rename", Linkage::Import, &fs_rename_sig)?;

    // --- Networking helpers from C runtime ---

    // int __argon_net_tcp_bind(addr, addr_len, port)
    let mut net_tcp_bind_sig = module.make_signature();
    net_tcp_bind_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_bind_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_bind_sig.params.push(AbiParam::new(types::I32));
    net_tcp_bind_sig.returns.push(AbiParam::new(types::I32));
    let net_tcp_bind_fn =
        module.declare_function("__argon_net_tcp_bind", Linkage::Import, &net_tcp_bind_sig)?;

    // int __argon_net_tcp_connect(addr, addr_len, port)
    let mut net_tcp_connect_sig = module.make_signature();
    net_tcp_connect_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_connect_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_connect_sig.params.push(AbiParam::new(types::I32));
    net_tcp_connect_sig.returns.push(AbiParam::new(types::I32));
    let net_tcp_connect_fn = module.declare_function(
        "__argon_net_tcp_connect",
        Linkage::Import,
        &net_tcp_connect_sig,
    )?;

    // int __argon_net_tcp_accept(listen_fd)
    let mut net_tcp_accept_sig = module.make_signature();
    net_tcp_accept_sig.params.push(AbiParam::new(types::I32));
    net_tcp_accept_sig.returns.push(AbiParam::new(types::I32));
    let net_tcp_accept_fn = module.declare_function(
        "__argon_net_tcp_accept",
        Linkage::Import,
        &net_tcp_accept_sig,
    )?;

    // long __argon_net_tcp_read(fd, buf, max_bytes)
    let mut net_tcp_read_sig = module.make_signature();
    net_tcp_read_sig.params.push(AbiParam::new(types::I32));
    net_tcp_read_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_read_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_read_sig.returns.push(AbiParam::new(pointer_type));
    let net_tcp_read_fn =
        module.declare_function("__argon_net_tcp_read", Linkage::Import, &net_tcp_read_sig)?;

    // long __argon_net_tcp_write(fd, data, data_len)
    let mut net_tcp_write_sig = module.make_signature();
    net_tcp_write_sig.params.push(AbiParam::new(types::I32));
    net_tcp_write_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_write_sig.params.push(AbiParam::new(pointer_type));
    net_tcp_write_sig.returns.push(AbiParam::new(pointer_type));
    let net_tcp_write_fn =
        module.declare_function("__argon_net_tcp_write", Linkage::Import, &net_tcp_write_sig)?;

    // int __argon_net_tcp_close(fd)
    let mut net_tcp_close_sig = module.make_signature();
    net_tcp_close_sig.params.push(AbiParam::new(types::I32));
    net_tcp_close_sig.returns.push(AbiParam::new(types::I32));
    let net_tcp_close_fn =
        module.declare_function("__argon_net_tcp_close", Linkage::Import, &net_tcp_close_sig)?;

    // int __argon_net_resolve(host, host_len, out_buf, out_buf_size)
    let mut net_resolve_sig = module.make_signature();
    net_resolve_sig.params.push(AbiParam::new(pointer_type));
    net_resolve_sig.params.push(AbiParam::new(pointer_type));
    net_resolve_sig.params.push(AbiParam::new(pointer_type));
    net_resolve_sig.params.push(AbiParam::new(pointer_type));
    net_resolve_sig.returns.push(AbiParam::new(types::I32));
    let net_resolve_fn =
        module.declare_function("__argon_net_resolve", Linkage::Import, &net_resolve_sig)?;

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
        fs_read_file: fs_read_file_fn,
        fs_write_file: fs_write_file_fn,
        fs_append_file: fs_append_file_fn,
        fs_exists: fs_exists_fn,
        fs_file_size: fs_file_size_fn,
        fs_is_file: fs_is_file_fn,
        fs_is_dir: fs_is_dir_fn,
        fs_remove: fs_remove_fn,
        fs_mkdir: fs_mkdir_fn,
        fs_rmdir: fs_rmdir_fn,
        fs_rename: fs_rename_fn,
        // Networking
        net_tcp_bind: net_tcp_bind_fn,
        net_tcp_connect: net_tcp_connect_fn,
        net_tcp_accept: net_tcp_accept_fn,
        net_tcp_read: net_tcp_read_fn,
        net_tcp_write: net_tcp_write_fn,
        net_tcp_close: net_tcp_close_fn,
        net_resolve: net_resolve_fn,
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
    // File system
    pub fs_read_file: cranelift_module::FuncId,
    pub fs_write_file: cranelift_module::FuncId,
    pub fs_append_file: cranelift_module::FuncId,
    pub fs_exists: cranelift_module::FuncId,
    pub fs_file_size: cranelift_module::FuncId,
    pub fs_is_file: cranelift_module::FuncId,
    pub fs_is_dir: cranelift_module::FuncId,
    pub fs_remove: cranelift_module::FuncId,
    pub fs_mkdir: cranelift_module::FuncId,
    pub fs_rmdir: cranelift_module::FuncId,
    pub fs_rename: cranelift_module::FuncId,
    // Networking
    pub net_tcp_bind: cranelift_module::FuncId,
    pub net_tcp_connect: cranelift_module::FuncId,
    pub net_tcp_accept: cranelift_module::FuncId,
    pub net_tcp_read: cranelift_module::FuncId,
    pub net_tcp_write: cranelift_module::FuncId,
    pub net_tcp_close: cranelift_module::FuncId,
    pub net_resolve: cranelift_module::FuncId,
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
            // std:fs
            | "readFile"
            // std:net
            | "bind"
            | "connect"
            | "bindUdp"
            | "resolve"
            // std:http
            | "get"
            | "post"
            | "put"
            | "del"
            | "request"
            | "createHeaders"
            | "serve"
            // std:ws
            | "wsConnect"
            | "wsListen"
            // std:async
            | "sleep"
            | "spawn"
            // Async variants
            | "readFileAsync"
            | "writeFileAsync"
            | "readBytesAsync"
            | "writeBytesAsync"
            | "appendFileAsync"
            | "readDirAsync"
            | "statAsync"
            | "copyAsync"
            | "connectAsync"
            | "getAsync"
            | "postAsync"
            | "putAsync"
            | "delAsync"
            | "requestAsync"
            | "wsConnectAsync"
            | "serveAsync"
            // std:fs (continued)
            | "writeFile"
            | "appendFile"
            | "exists"
            | "stat"
            | "rename"
            | "remove"
            | "mkdir"
            | "mkdirRecursive"
            | "rmdir"
            | "removeRecursive"
            | "copy"
            | "readDir"
            | "tempDir"
            | "open"
    )
}
