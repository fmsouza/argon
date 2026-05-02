//! Argon Standard Library
//!
//! Embeds `.arg` source files for the language's standard library.
//! The prelude is always available; other modules are imported via `std:*`.

/// The Argon source for the prelude (always-available globals).
pub fn prelude_source() -> &'static str {
    include_str!("../stdlib/prelude.arg")
}

/// Resolve a `std:*` module name to its Argon source.
/// Returns `None` if the module does not exist.
pub fn resolve_std_module(name: &str) -> Option<&'static str> {
    match name {
        "io" => Some(include_str!("../stdlib/io.arg")),
        "math" => Some(include_str!("../stdlib/math.arg")),
        "error" => Some(include_str!("../stdlib/error.arg")),
        "fs" => Some(include_str!("../stdlib/fs.arg")),
        "net" => Some(include_str!("../stdlib/net.arg")),
        "http" => Some(include_str!("../stdlib/http.arg")),
        "ws" => Some(include_str!("../stdlib/ws.arg")),
        "async" => Some(include_str!("../stdlib/async.arg")),
        "test" => Some(include_str!("../stdlib/test.arg")),
        _ => None,
    }
}

/// List all available `std:*` module names.
pub fn available_modules() -> &'static [&'static str] {
    &["io", "math", "error", "fs", "net", "http", "ws", "async", "test"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_contains_core_types() {
        let src = prelude_source();
        assert!(src.contains("struct Vec<T>"));
        assert!(src.contains("struct Some<T>"));
        assert!(src.contains("struct None"));
        assert!(src.contains("struct Ok<T, E>"));
        assert!(src.contains("struct Err<T, E>"));
        assert!(src.contains("struct Shared<T>"));
        assert!(src.contains("struct Map<K, V>"));
        assert!(src.contains("struct Set<T>"));
    }

    #[test]
    fn resolves_math_module() {
        let src = resolve_std_module("math").expect("math module should exist");
        assert!(src.contains("function sqrt("));
        assert!(src.contains("PI"));
        assert!(src.contains("function sin("));
    }

    #[test]
    fn resolves_io_module() {
        let src = resolve_std_module("io").expect("io module should exist");
        assert!(src.contains("function print("));
        assert!(src.contains("function println("));
    }

    #[test]
    fn resolves_error_module() {
        let src = resolve_std_module("error").expect("error module should exist");
        assert!(src.contains("struct IoError"));
        assert!(src.contains("code: string"));
        assert!(src.contains("message: string"));
    }

    #[test]
    fn resolves_fs_module() {
        let src = resolve_std_module("fs").expect("fs module should exist");
        assert!(src.contains("function readFile("));
        assert!(src.contains("function writeFile("));
        assert!(src.contains("function mkdir("));
        assert!(src.contains("struct File"));
        assert!(src.contains("struct FileStat"));
    }

    #[test]
    fn resolves_net_module() {
        let src = resolve_std_module("net").expect("net module should exist");
        assert!(src.contains("struct TcpListener"));
        assert!(src.contains("struct TcpStream"));
        assert!(src.contains("function bind("));
        assert!(src.contains("function connect("));
    }

    #[test]
    fn resolves_http_module() {
        let src = resolve_std_module("http").expect("http module should exist");
        assert!(src.contains("function get("));
        assert!(src.contains("function post("));
        assert!(src.contains("struct Headers"));
        assert!(src.contains("struct HttpServer"));
    }

    #[test]
    fn resolves_ws_module() {
        let src = resolve_std_module("ws").expect("ws module should exist");
        assert!(src.contains("struct WsConnection"));
        assert!(src.contains("function wsConnect("));
        assert!(src.contains("struct WsServer"));
    }

    #[test]
    fn resolves_test_module() {
        let src = resolve_std_module("test").expect("test module should exist");
        assert!(src.contains("struct Runner"));
        assert!(src.contains("struct Assert"));
        assert!(src.contains("fn case("));
        assert!(src.contains("when(name:"));
        assert!(src.contains("beforeEach"));
    }

    #[test]
    fn unknown_module_returns_none() {
        assert!(resolve_std_module("nonexistent").is_none());
    }

    #[test]
    fn available_modules_lists_all() {
        let mods = available_modules();
        assert!(mods.contains(&"io"));
        assert!(mods.contains(&"math"));
        assert!(mods.contains(&"error"));
        assert!(mods.contains(&"fs"));
        assert!(mods.contains(&"net"));
        assert!(mods.contains(&"http"));
        assert!(mods.contains(&"ws"));
        assert!(mods.contains(&"async"));
        assert!(mods.contains(&"test"));
    }
}
