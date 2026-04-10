//! Native runtime entry point generation and C runtime compilation.
//!
//! Generates the `main` function wrapper that serves as the executable's
//! entry point. This calls `__argon_init` (the user's top-level code).
//!
//! Also provides a small C runtime with formatting helpers that avoid
//! variadic calling convention issues on aarch64.

use crate::CodegenError;
use argon_target::TargetTriple;
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
#include <stdlib.h>
#include <unistd.h>
#include <math.h>
#include <string.h>
#include <sys/stat.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>

/* ===== Print helpers ===== */

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

/* ===== File system helpers =====
 *
 * All fs functions return a result struct as a heap-allocated pair:
 *   [ptr_to_data, status]
 * where status: 0 = ok, non-zero = error code
 * For readFile: data is a {ptr, len} pair with the file contents.
 * For writeFile etc: data is unused (0).
 *
 * We use a simple convention:
 *   - Return pointer to a heap-allocated buffer on success
 *   - Return NULL with errno set on failure
 *   - The caller checks the return value
 */

/* Read entire file into malloc'd buffer. Returns buffer ptr, sets *out_len.
 * Returns NULL on error (check errno). */
char *__argon_fs_read_file(const char *path, long path_len, long *out_len) {
    /* Create null-terminated path */
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return NULL;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    FILE *f = fopen(cpath, "r");
    free(cpath);
    if (!f) return NULL;

    /* Get file size */
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);

    char *buf = (char *)malloc(size + 1);
    if (!buf) { fclose(f); return NULL; }

    long nread = fread(buf, 1, size, f);
    fclose(f);
    buf[nread] = '\0';
    *out_len = nread;
    return buf;
}

/* Write content to file. Returns 0 on success, -1 on error. */
int __argon_fs_write_file(const char *path, long path_len,
                          const char *data, long data_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    FILE *f = fopen(cpath, "w");
    free(cpath);
    if (!f) return -1;

    long nwritten = fwrite(data, 1, data_len, f);
    fclose(f);
    return (nwritten == data_len) ? 0 : -1;
}

/* Append content to file. Returns 0 on success, -1 on error. */
int __argon_fs_append_file(const char *path, long path_len,
                           const char *data, long data_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    FILE *f = fopen(cpath, "a");
    free(cpath);
    if (!f) return -1;

    long nwritten = fwrite(data, 1, data_len, f);
    fclose(f);
    return (nwritten == data_len) ? 0 : -1;
}

/* Check if file exists. Returns 1 if exists, 0 if not. */
int __argon_fs_exists(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return 0;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    struct stat st;
    int result = (stat(cpath, &st) == 0) ? 1 : 0;
    free(cpath);
    return result;
}

/* Get file size. Returns -1 on error. */
long __argon_fs_file_size(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    struct stat st;
    int rc = stat(cpath, &st);
    free(cpath);
    if (rc != 0) return -1;
    return (long)st.st_size;
}

/* Check if path is a regular file. Returns 1 if file, 0 otherwise. */
int __argon_fs_is_file(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return 0;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    struct stat st;
    int result = (stat(cpath, &st) == 0 && S_ISREG(st.st_mode)) ? 1 : 0;
    free(cpath);
    return result;
}

/* Check if path is a directory. Returns 1 if dir, 0 otherwise. */
int __argon_fs_is_dir(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return 0;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    struct stat st;
    int result = (stat(cpath, &st) == 0 && S_ISDIR(st.st_mode)) ? 1 : 0;
    free(cpath);
    return result;
}

/* Remove a file. Returns 0 on success, -1 on error. */
int __argon_fs_remove(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    int result = unlink(cpath);
    free(cpath);
    return result;
}

/* Create a directory. Returns 0 on success, -1 on error. */
int __argon_fs_mkdir(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    int result = mkdir(cpath, 0755);
    free(cpath);
    return result;
}

/* Remove a directory. Returns 0 on success, -1 on error. */
int __argon_fs_rmdir(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    if (!cpath) return -1;
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    int result = rmdir(cpath);
    free(cpath);
    return result;
}

/* Rename a file/directory. Returns 0 on success, -1 on error. */
int __argon_fs_rename(const char *from, long from_len,
                      const char *to, long to_len) {
    char *cfrom = (char *)malloc(from_len + 1);
    char *cto = (char *)malloc(to_len + 1);
    if (!cfrom || !cto) { free(cfrom); free(cto); return -1; }
    memcpy(cfrom, from, from_len); cfrom[from_len] = '\0';
    memcpy(cto, to, to_len); cto[to_len] = '\0';

    int result = rename(cfrom, cto);
    free(cfrom);
    free(cto);
    return result;
}

/* ===== Networking helpers ===== */

#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>

/* Create a TCP server socket, bind, and listen.
 * Returns the socket fd, or -1 on error. */
int __argon_net_tcp_bind(const char *addr, long addr_len, int port) {
    char *caddr = (char *)malloc(addr_len + 1);
    if (!caddr) return -1;
    memcpy(caddr, addr, addr_len);
    caddr[addr_len] = '\0';

    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) { free(caddr); return -1; }

    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in sa;
    memset(&sa, 0, sizeof(sa));
    sa.sin_family = AF_INET;
    sa.sin_port = htons((unsigned short)port);
    inet_pton(AF_INET, caddr, &sa.sin_addr);
    free(caddr);

    if (bind(fd, (struct sockaddr *)&sa, sizeof(sa)) < 0) { close(fd); return -1; }
    if (listen(fd, 128) < 0) { close(fd); return -1; }
    return fd;
}

/* Accept a connection on a listening socket.
 * Returns the new socket fd, or -1 on error. */
int __argon_net_tcp_accept(int listen_fd) {
    struct sockaddr_in client;
    socklen_t len = sizeof(client);
    return accept(listen_fd, (struct sockaddr *)&client, &len);
}

/* Connect to a TCP server.
 * Returns the socket fd, or -1 on error. */
int __argon_net_tcp_connect(const char *addr, long addr_len, int port) {
    char *caddr = (char *)malloc(addr_len + 1);
    if (!caddr) return -1;
    memcpy(caddr, addr, addr_len);
    caddr[addr_len] = '\0';

    struct addrinfo hints, *res;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    char port_str[16];
    snprintf(port_str, sizeof(port_str), "%d", port);

    if (getaddrinfo(caddr, port_str, &hints, &res) != 0) {
        free(caddr);
        return -1;
    }
    free(caddr);

    int fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd < 0) { freeaddrinfo(res); return -1; }

    if (connect(fd, res->ai_addr, res->ai_addrlen) < 0) {
        close(fd);
        freeaddrinfo(res);
        return -1;
    }
    freeaddrinfo(res);
    return fd;
}

/* Read from a socket. Returns bytes read, or -1 on error. */
long __argon_net_tcp_read(int fd, char *buf, long max_bytes) {
    return recv(fd, buf, max_bytes, 0);
}

/* Write to a socket. Returns bytes written, or -1 on error. */
long __argon_net_tcp_write(int fd, const char *data, long data_len) {
    return send(fd, data, data_len, 0);
}

/* Shutdown write side of socket. Returns 0 on success. */
int __argon_net_tcp_shutdown(int fd) {
    return shutdown(fd, SHUT_WR);
}

/* Close a socket. */
int __argon_net_tcp_close(int fd) {
    return close(fd);
}

/* Get the local port of a bound socket. Returns port or -1. */
int __argon_net_local_port(int fd) {
    struct sockaddr_in sa;
    socklen_t len = sizeof(sa);
    if (getsockname(fd, (struct sockaddr *)&sa, &len) < 0) return -1;
    return ntohs(sa.sin_port);
}

/* DNS resolve. Writes first resolved IP to out_buf (null-terminated).
 * Returns 0 on success, -1 on error. */
int __argon_net_resolve(const char *host, long host_len, char *out_buf, long out_buf_size) {
    char *chost = (char *)malloc(host_len + 1);
    if (!chost) return -1;
    memcpy(chost, host, host_len);
    chost[host_len] = '\0';

    struct addrinfo hints, *res;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;

    if (getaddrinfo(chost, NULL, &hints, &res) != 0) {
        free(chost);
        return -1;
    }
    free(chost);

    struct sockaddr_in *sa = (struct sockaddr_in *)res->ai_addr;
    inet_ntop(AF_INET, &sa->sin_addr, out_buf, out_buf_size);
    freeaddrinfo(res);
    return 0;
}
"#;

/// Compile the C runtime to an object file for the given target.
/// Returns the path to the generated .o file.
pub fn compile_c_runtime(
    target_dir: &Path,
    triple: &TargetTriple,
) -> Result<PathBuf, CodegenError> {
    let cache_dir = runtime_cache_dir(target_dir, triple);
    std::fs::create_dir_all(&cache_dir).map_err(|e| {
        CodegenError::CraneliftError(format!("failed to create runtime cache dir: {}", e))
    })?;

    let o_path = cache_dir.join(format!("__argon_runtime{}", triple.obj_suffix()));
    if o_path.exists() {
        return Ok(o_path);
    }

    let c_path = cache_dir.join("__argon_runtime.c");
    let tmp_o_path = cache_dir.join(format!(
        "__argon_runtime.{}.tmp{}",
        std::process::id(),
        triple.obj_suffix()
    ));

    std::fs::write(&c_path, C_RUNTIME_SOURCE)
        .map_err(|e| CodegenError::CraneliftError(format!("failed to write C runtime: {}", e)))?;

    let output = std::process::Command::new("cc")
        .args(["-c", "-O2", "-o"])
        .arg(&tmp_o_path)
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

    if let Err(err) = std::fs::rename(&tmp_o_path, &o_path) {
        if o_path.exists() {
            let _ = std::fs::remove_file(&tmp_o_path);
        } else {
            return Err(CodegenError::LinkerError(format!(
                "failed to place cached runtime object: {}",
                err
            )));
        }
    }

    Ok(o_path)
}

fn runtime_cache_dir(base_dir: &Path, triple: &TargetTriple) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    C_RUNTIME_SOURCE.hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);

    base_dir
        .join("argon-runtime-cache")
        .join(triple.triple_str())
        .join(format!("{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_c_runtime_reuses_cached_object() {
        let temp_dir = tempfile::tempdir().unwrap();
        let triple = TargetTriple::host();

        let first = compile_c_runtime(temp_dir.path(), &triple).unwrap();
        let second = compile_c_runtime(temp_dir.path(), &triple).unwrap();

        assert_eq!(first, second);
        assert!(second.exists());
    }

    #[test]
    fn compile_c_runtime_rebuilds_missing_cache_entry() {
        let temp_dir = tempfile::tempdir().unwrap();
        let triple = TargetTriple::host();

        let cached = compile_c_runtime(temp_dir.path(), &triple).unwrap();
        std::fs::remove_file(&cached).unwrap();

        let rebuilt = compile_c_runtime(temp_dir.path(), &triple).unwrap();
        assert_eq!(cached, rebuilt);
        assert!(rebuilt.exists());
    }
}
