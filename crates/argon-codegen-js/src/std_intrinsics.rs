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

        // std:net polyfills — wrap Node.js net/dgram modules
        ("net", "bind") => Some(
            "function(addr, port) { try { const net = require('net'); const server = net.createServer(); server.listen(port, addr); return { value: { __server: server, accept() { return new Promise((resolve) => { server.once('connection', (sock) => { resolve({ value: { __sock: sock, read(n) { try { const d = sock.read(n); return { value: d ? d.toString('utf8') : '', isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, write(data) { try { return { value: sock.write(data) ? data.length : 0, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, shutdown() { sock.end(); return { value: undefined, isOk: true, isErr: false }; }, close() { sock.destroy(); return { value: undefined, isOk: true, isErr: false }; }, peerAddr() { return sock.remoteAddress + ':' + sock.remotePort; } }, isOk: true, isErr: false }); }); }); }, close() { server.close(); return { value: undefined, isOk: true, isErr: false }; }, localAddr() { const a = server.address(); return a ? a.address + ':' + a.port : ''; } }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("net", "connect") => Some(
            "function(addr, port) { try { const net = require('net'); const sock = net.createConnection(port, addr); return { value: { __sock: sock, read(n) { try { const d = sock.read(n); return { value: d ? d.toString('utf8') : '', isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, write(data) { try { return { value: sock.write(data) ? data.length : 0, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, shutdown() { sock.end(); return { value: undefined, isOk: true, isErr: false }; }, close() { sock.destroy(); return { value: undefined, isOk: true, isErr: false }; }, peerAddr() { return sock.remoteAddress + ':' + sock.remotePort; } }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("net", "bindUdp") => Some(
            "function(addr, port) { try { const dgram = require('dgram'); const sock = dgram.createSocket('udp4'); sock.bind(port, addr); return { value: { __sock: sock, sendTo(data, a, p) { try { sock.send(data, p, a); return { value: data.length, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code||'EIO', message: e.message }, isOk: false, isErr: true }; } }, recvFrom(n) { return new Promise((resolve) => { sock.once('message', (msg, rinfo) => { resolve({ value: { data: msg.toString('utf8'), addr: rinfo.address, port: rinfo.port }, isOk: true, isErr: false }); }); }); }, close() { sock.close(); return { value: undefined, isOk: true, isErr: false }; } }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("net", "resolve") => Some(
            "function(hostname) { try { const dns = require('dns'); const addrs = dns.resolve4Sync ? dns.resolve4Sync(hostname) : []; return { value: addrs, isOk: true, isErr: false }; } catch(e) { try { const { execSync } = require('child_process'); const out = execSync('getent hosts ' + hostname + ' 2>/dev/null || host ' + hostname + ' 2>/dev/null', { encoding: 'utf8' }); const ips = out.match(/\\d+\\.\\d+\\.\\d+\\.\\d+/g) || []; return { value: ips, isOk: true, isErr: false }; } catch(e2) { return { error: { code: 'ENOTFOUND', message: e.message }, isOk: false, isErr: true }; } }",
        ),

        // std:http polyfills — wrap Node.js http/https + fetch
        ("http", "get") => Some(
            "function(url) { try { const h = url.startsWith('https') ? require('https') : require('http'); return new Promise((resolve) => { h.get(url, (res) => { let body = ''; res.on('data', c => body += c); res.on('end', () => { const hdrs = { get(n) { const v = res.headers[n.toLowerCase()]; return v ? { value: v, isOk: true, isErr: false } : { error: { code: 'ENOENT', message: 'header not found' }, isOk: false, isErr: true }; }, set() {}, has(n) { return n.toLowerCase() in res.headers; }, delete() {}, entries() { return Object.entries(res.headers).map(([n,v]) => ({ name: n, value: String(v) })); } }; resolve({ value: { status: res.statusCode, headers: hdrs, body }, isOk: true, isErr: false }); }); }).on('error', (e) => { resolve({ error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }); }); }); } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("http", "post") => Some(
            "function(url, body, headers) { try { const u = new (require('url').URL)(url); const h = url.startsWith('https') ? require('https') : require('http'); const opts = { hostname: u.hostname, port: u.port, path: u.pathname + u.search, method: 'POST', headers: {} }; if (headers && headers.entries) { headers.entries().forEach(e => { opts.headers[e.name] = e.value; }); } return new Promise((resolve) => { const req = h.request(opts, (res) => { let rbody = ''; res.on('data', c => rbody += c); res.on('end', () => { resolve({ value: { status: res.statusCode, headers: { get(n) { const v = res.headers[n.toLowerCase()]; return v ? { value: v, isOk: true, isErr: false } : { error: { code: 'ENOENT', message: 'not found' }, isOk: false, isErr: true }; }, has(n) { return n.toLowerCase() in res.headers; }, entries() { return Object.entries(res.headers).map(([n,v]) => ({ name: n, value: String(v) })); } }, body: rbody }, isOk: true, isErr: false }); }); }).on('error', (e) => { resolve({ error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }); }); req.write(body || ''); req.end(); }); } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("http", "request") => Some(
            "function(opts) { try { const u = new (require('url').URL)(opts.url); const h = opts.url.startsWith('https') ? require('https') : require('http'); const ropts = { hostname: u.hostname, port: u.port, path: u.pathname + u.search, method: opts.method || 'GET', headers: {}, timeout: opts.timeoutMs || 0 }; if (opts.headers && opts.headers.entries) { opts.headers.entries().forEach(e => { ropts.headers[e.name] = e.value; }); } return new Promise((resolve) => { const req = h.request(ropts, (res) => { if (opts.followRedirects !== false && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) { resolve(arguments.callee({ ...opts, url: res.headers.location })); return; } let body = ''; res.on('data', c => body += c); res.on('end', () => { resolve({ value: { status: res.statusCode, headers: { get(n) { const v = res.headers[n.toLowerCase()]; return v ? { value: v, isOk: true, isErr: false } : { error: { code: 'ENOENT', message: 'not found' }, isOk: false, isErr: true }; }, has(n) { return n.toLowerCase() in res.headers; }, entries() { return Object.entries(res.headers).map(([n,v]) => ({ name: n, value: String(v) })); } }, body }, isOk: true, isErr: false }); }); }).on('error', (e) => { resolve({ error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }); }); if (ropts.timeout) req.setTimeout(ropts.timeout, () => { req.destroy(); resolve({ error: { code: 'ETIMEDOUT', message: 'request timed out' }, isOk: false, isErr: true }); }); req.write(opts.body || ''); req.end(); }); } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("http", "del") => Some(
            "function(url) { try { const u = new (require('url').URL)(url); const h = url.startsWith('https') ? require('https') : require('http'); return new Promise((resolve) => { h.request({ hostname: u.hostname, port: u.port, path: u.pathname + u.search, method: 'DELETE' }, (res) => { let body = ''; res.on('data', c => body += c); res.on('end', () => { resolve({ value: { status: res.statusCode, body }, isOk: true, isErr: false }); }); }).on('error', (e) => { resolve({ error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }); }).end(); }); } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
        ),
        ("http", "createHeaders") => Some(
            "function() { const h = {}; return { get(n) { const v = h[n.toLowerCase()]; return v !== undefined ? { value: v, isOk: true, isErr: false } : { error: { code: 'ENOENT', message: 'header not found' }, isOk: false, isErr: true }; }, set(n, v) { h[n.toLowerCase()] = v; }, has(n) { return n.toLowerCase() in h; }, delete(n) { delete h[n.toLowerCase()]; }, entries() { return Object.entries(h).map(([n,v]) => ({ name: n, value: v })); } }; }",
        ),
        ("http", "serve") => Some(
            "function(port, handler) { try { const http = require('http'); const server = http.createServer((req, res) => { let body = ''; req.on('data', c => body += c); req.on('end', () => { const hreq = { method: req.method, url: req.url, headers: { get(n) { const v = req.headers[n.toLowerCase()]; return v ? { value: v, isOk: true, isErr: false } : { error: { code: 'ENOENT', message: 'not found' }, isOk: false, isErr: true }; }, has(n) { return n.toLowerCase() in req.headers; }, entries() { return Object.entries(req.headers).map(([n,v]) => ({ name: n, value: String(v) })); } }, body }; const hres = { __status: 200, __headers: {}, setStatus(c) { this.__status = c; }, setHeader(n, v) { this.__headers[n] = v; }, send(b) { try { res.writeHead(this.__status, this.__headers); res.end(b); return { value: undefined, isOk: true, isErr: false }; } catch(e) { return { error: { code: 'EIO', message: e.message }, isOk: false, isErr: true }; } } }; handler(hreq, hres); }); }); server.listen(port); return { value: { close() { server.close(); return { value: undefined, isOk: true, isErr: false }; }, addr() { const a = server.address(); return a ? a.address + ':' + a.port : ''; } }, isOk: true, isErr: false }; } catch(e) { return { error: { code: e.code || 'EIO', message: e.message }, isOk: false, isErr: true }; } }",
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
