//! Maps Argon `std:*` module symbols to their JavaScript equivalents.

/// Given a std module name and a symbol name, return the JS expression.
/// Returns `None` for symbols that need a polyfill (emitted separately).
pub(crate) fn js_intrinsic(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        // std:math — functions
        ("math", "abs") => Some("Math.abs"),
        ("math", "floor") => Some("Math.floor"),
        ("math", "ceil") => Some("Math.ceil"),
        ("math", "round") => Some("Math.round"),
        ("math", "trunc") => Some("Math.trunc"),
        ("math", "sign") => Some("Math.sign"),
        ("math", "min") => Some("Math.min"),
        ("math", "max") => Some("Math.max"),
        ("math", "sqrt") => Some("Math.sqrt"),
        ("math", "cbrt") => Some("Math.cbrt"),
        ("math", "pow") => Some("Math.pow"),
        ("math", "hypot") => Some("Math.hypot"),
        ("math", "sin") => Some("Math.sin"),
        ("math", "cos") => Some("Math.cos"),
        ("math", "tan") => Some("Math.tan"),
        ("math", "asin") => Some("Math.asin"),
        ("math", "acos") => Some("Math.acos"),
        ("math", "atan") => Some("Math.atan"),
        ("math", "atan2") => Some("Math.atan2"),
        ("math", "log") => Some("Math.log"),
        ("math", "log2") => Some("Math.log2"),
        ("math", "log10") => Some("Math.log10"),
        ("math", "exp") => Some("Math.exp"),

        // std:math — constants
        ("math", "PI") => Some("Math.PI"),
        ("math", "E") => Some("Math.E"),
        ("math", "LN2") => Some("Math.LN2"),
        ("math", "LN10") => Some("Math.LN10"),
        ("math", "SQRT2") => Some("Math.SQRT2"),
        ("math", "TAU") => Some("(Math.PI * 2)"),

        // clamp has no JS native equivalent
        ("math", "clamp") => None,

        // std:io — functions (provided by runtime IIFE)
        ("io", "print") => Some("print"),
        ("io", "println") => Some("println"),

        // std:fs — file system (all polyfilled, need require('fs'))
        ("fs", "readFile") => None,
        ("fs", "readFileAsync") => None,
        ("fs", "readBytes") => None,
        ("fs", "readBytesAsync") => None,
        ("fs", "writeFile") => None,
        ("fs", "writeFileAsync") => None,
        ("fs", "writeBytes") => None,
        ("fs", "writeBytesAsync") => None,
        ("fs", "appendFile") => None,
        ("fs", "appendFileAsync") => None,
        ("fs", "readDir") => None,
        ("fs", "readDirAsync") => None,
        ("fs", "mkdir") => None,
        ("fs", "mkdirRecursive") => None,
        ("fs", "rmdir") => None,
        ("fs", "removeRecursive") => None,
        ("fs", "exists") => None,
        ("fs", "stat") => None,
        ("fs", "statAsync") => None,
        ("fs", "rename") => None,
        ("fs", "remove") => None,
        ("fs", "copy") => None,
        ("fs", "copyAsync") => None,
        ("fs", "symlink") => None,
        ("fs", "readlink") => None,
        ("fs", "tempDir") => None,
        ("fs", "open") => None,

        // std:net — networking
        ("net", "bind") => None,
        ("net", "connect") => None,
        ("net", "connectAsync") => None,
        ("net", "bindUdp") => None,
        ("net", "resolve") => None,

        // std:http — HTTP client & server
        ("http", "get") => None,
        ("http", "getAsync") => None,
        ("http", "post") => None,
        ("http", "postAsync") => None,
        ("http", "put") => None,
        ("http", "putAsync") => None,
        ("http", "del") => None,
        ("http", "delAsync") => None,
        ("http", "request") => None,
        ("http", "requestAsync") => None,
        ("http", "createHeaders") => None,
        ("http", "serve") => None,
        ("http", "serveAsync") => None,

        // std:ws — WebSocket
        ("ws", "wsConnect") => None,
        ("ws", "wsConnectAsync") => None,
        ("ws", "wsListen") => None,

        _ => None,
    }
}

/// Returns JS polyfill code for intrinsics that have no native JS equivalent.
pub(crate) fn js_polyfill(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        ("math", "clamp") => {
            Some("function(x, lo, hi) { return Math.min(Math.max(x, lo), hi); }")
        }

        // std:fs polyfills — wrap Node.js fs module with Result types
        ("fs", "readFile") => Some(
            "function(path) { try { return { value: require('fs').readFileSync(path, 'utf8'), isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "readFileAsync") => Some(
            "async function(path) { try { const fs = require('fs').promises; return { value: await fs.readFile(path, 'utf8'), isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "writeFile") => Some(
            "function(path, content) { try { require('fs').writeFileSync(path, content, 'utf8'); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "writeFileAsync") => Some(
            "async function(path, content) { try { await require('fs').promises.writeFile(path, content, 'utf8'); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "appendFile") => Some(
            "function(path, content) { try { require('fs').appendFileSync(path, content, 'utf8'); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "readDir") => Some(
            "function(path) { try { const entries = require('fs').readdirSync(path, { withFileTypes: true }); return { value: entries.map(e => ({ name: e.name, isFile: e.isFile(), isDir: e.isDirectory(), isSymlink: e.isSymbolicLink() })), isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "mkdir") => Some(
            "function(path) { try { require('fs').mkdirSync(path); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "mkdirRecursive") => Some(
            "function(path) { try { require('fs').mkdirSync(path, { recursive: true }); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "rmdir") => Some(
            "function(path) { try { require('fs').rmdirSync(path); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "removeRecursive") => Some(
            "function(path) { try { require('fs').rmSync(path, { recursive: true, force: true }); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "exists") => Some(
            "function(path) { return { value: require('fs').existsSync(path), isOk: true, isErr: false }; }",
        ),
        ("fs", "stat") => Some(
            "function(path) { try { const s = require('fs').statSync(path); return { value: { size: s.size, isFile: s.isFile(), isDir: s.isDirectory(), isSymlink: s.isSymbolicLink(), modified: s.mtimeMs, created: s.birthtimeMs }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "rename") => Some(
            "function(from, to) { try { require('fs').renameSync(from, to); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "remove") => Some(
            "function(path) { try { require('fs').unlinkSync(path); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "copy") => Some(
            "function(from, to) { try { require('fs').copyFileSync(from, to); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "symlink") => Some(
            "function(target, path) { try { require('fs').symlinkSync(target, path); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "readlink") => Some(
            "function(path) { try { return { value: require('fs').readlinkSync(path), isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("fs", "tempDir") => Some(
            "function() { return { value: require('os').tmpdir(), isOk: true, isErr: false }; }",
        ),
        ("fs", "open") => Some(
            "function(path, mode) { try { const fs = require('fs'); const flags = { Read: 'r', Write: 'w', Append: 'a', ReadWrite: 'r+', WriteAppend: 'a+' }[mode] || 'r'; const fd = fs.openSync(path, flags); return { value: { __fd: fd, read(n) { try { const buf = Buffer.alloc(n); const bytes = fs.readSync(this.__fd, buf, 0, n); return { value: buf.slice(0, bytes).toString('utf8'), isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, write(data) { try { const n = fs.writeSync(this.__fd, data); return { value: n, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, seek(offset, whence) { try { const w = { Start: 0, Current: 1, End: 2 }[whence] || 0; fs.seekSync ? fs.seekSync(this.__fd, offset, w) : null; return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, close() { try { fs.closeSync(this.__fd); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } } }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_math_functions() {
        assert_eq!(js_intrinsic("math", "sqrt"), Some("Math.sqrt"));
        assert_eq!(js_intrinsic("math", "sin"), Some("Math.sin"));
        assert_eq!(js_intrinsic("math", "PI"), Some("Math.PI"));
        assert_eq!(js_intrinsic("math", "TAU"), Some("(Math.PI * 2)"));
    }

    #[test]
    fn clamp_needs_polyfill() {
        assert_eq!(js_intrinsic("math", "clamp"), None);
        assert!(js_polyfill("math", "clamp").is_some());
    }

    #[test]
    fn maps_io_functions() {
        assert_eq!(js_intrinsic("io", "print"), Some("print"));
        assert_eq!(js_intrinsic("io", "println"), Some("println"));
    }

    #[test]
    fn fs_intrinsics_need_polyfills() {
        // All fs functions use polyfills (no direct JS equivalent)
        assert_eq!(js_intrinsic("fs", "readFile"), None);
        assert_eq!(js_intrinsic("fs", "writeFile"), None);
        assert_eq!(js_intrinsic("fs", "exists"), None);
        assert_eq!(js_intrinsic("fs", "stat"), None);
        assert_eq!(js_intrinsic("fs", "open"), None);

        // Polyfills exist for all fs functions
        assert!(js_polyfill("fs", "readFile").is_some());
        assert!(js_polyfill("fs", "writeFile").is_some());
        assert!(js_polyfill("fs", "exists").is_some());
        assert!(js_polyfill("fs", "stat").is_some());
        assert!(js_polyfill("fs", "open").is_some());
        assert!(js_polyfill("fs", "tempDir").is_some());
    }

    #[test]
    fn net_http_ws_intrinsics_need_polyfills() {
        assert_eq!(js_intrinsic("net", "bind"), None);
        assert_eq!(js_intrinsic("http", "get"), None);
        assert_eq!(js_intrinsic("ws", "wsConnect"), None);
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(js_intrinsic("math", "nonexistent"), None);
        assert_eq!(js_intrinsic("unknown", "sqrt"), None);
    }
}
