# Argon

A TypeScript-like language with Rust-inspired ownership and borrowing, compiling to JavaScript, WebAssembly, and native binaries.

Argon gives you familiar syntax — structs, interfaces, generics, async/await — while the compiler enforces memory-safety rules at compile time. Write code that looks like TypeScript, get the safety guarantees of Rust, and compile to JavaScript, WebAssembly, or native executables.

```ts
struct Point {
    x: f64;
    y: f64;
}

function translate(p: &Point, dx: f64): Point {
    return Point { x: p.x + dx, y: p.y };
}

const origin = Point { x: 0.0, y: 0.0 };
const moved = translate(&origin, 5.0);
println(moved.x); // 5.0
```

```ts
struct Counter {
    value: i32;

    constructor(initial: i32) {
        this.value = initial;
    }

    increment(): void with &mut this {
        this.value = this.value + 1;
    }

    getValue(): i32 with &this {
        return this.value;
    }
}

const counter = Counter { initial: 0 };
counter.increment();
println(counter.getValue()); // 1
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Build from source

```bash
cargo build --release
```

The `argon` binary will be at `target/release/argon`.

### Your first program

Create a file called `hello.arg`:

```ts
from "std:io" import { println };

struct Greeting {
    message: string;
}

function greet(g: &Greeting): string {
    return g.message;
}

const hello = Greeting { message: "Hello, Argon!" };
println(greet(&hello));
```

```bash
# Type-check and borrow-check
argon check hello.arg

# Compile to JavaScript
argon compile hello.arg --target js -o hello.js
node hello.js

# Compile to a native executable
argon compile hello.arg --target native -o hello
./hello

# Or run directly with the built-in runtime
argon run hello.arg
```

### Try the REPL

```bash
argon repl
```

The REPL supports multi-line input and has built-in commands: `:load <file>`, `:check`, `:compile [js|wasm|native]`, `:show`, `:reset`.

## CLI Reference

| Command | Description |
|---------|-------------|
| `argon compile <file.arg>` | Compile to JS, WASM, or native binary |
| `argon check <file.arg>` | Type-check and borrow-check without emitting code |
| `argon run <file.arg>` | Execute on the built-in AST runtime |
| `argon test [--input file] [--directory path]` | Run test suites |
| `argon format <file.arg>` | Format source code |
| `argon init <project-name>` | Scaffold a new project |
| `argon watch <file.arg>` | Rebuild on file changes |
| `argon repl` | Interactive REPL |

### Key flags for `argon compile`

| Flag | Description |
|------|-------------|
| `--target js\|wasm\|native` | Output target (default: `js`) |
| `--pipeline ast\|ir` | Compilation pipeline (default: `ir`, preferred) |
| `-o <path>` | Output file path |
| `--out-dir <path>` | Output directory (for multi-file projects) |
| `--source-map` | Generate source maps (JS target) |
| `--opt` | Enable optimization passes |
| `--declarations` | Generate TypeScript `.d.ts` declarations |
| `--triple <triple>` | Target triple for native compilation (implies `--target native`) |
| `--emit exe\|obj\|asm` | Native emit format (default: `exe`) |

## Language Features

### Types and primitives

Argon has sized numeric types, plus the familiar JS types:

```ts
const count: i32 = 42;
const ratio: f64 = 3.14;
const name: string = "argon";
const active: bool = true;
```

Available numeric types: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `isize`, `usize`.

### Structs

Plain structs hold data with named fields:

```ts
struct Color {
    r: u8;
    g: u8;
    b: u8;
}

const red = Color { r: 255, g: 0, b: 0 };
println(red.r);
```

Structs can also have constructors, methods, and `implements`:

```ts
struct Counter {
    value: i32;

    constructor(initial: i32) {
        this.value = initial;
    }

    increment(): void with &mut this {
        this.value = this.value + 1;
    }

    getValue(): i32 with &this {
        return this.value;
    }
}

const counter = Counter { initial: 0 };
counter.increment();
println(counter.getValue()); // 1
```

The `with &this` and `with &mut this` annotations declare whether a method borrows `this` as shared or mutable — the compiler enforces these at call sites.

### Ownership and move semantics

Non-copy types are moved on assignment. The compiler rejects use-after-move:

```ts
struct Message {
    text: string;
}

function consume(msg: Message): string {
    return msg.text;
}

const m = Message { text: "hello" };
consume(m);
// println(m.text);  // ERROR: use after move
```

Primitives (`i32`, `f64`, `bool`, etc.) are copy types and can be freely reused.

### Borrowing

Shared references (`&T`) allow read-only access. Mutable references (`&mut T`) give exclusive access. You can have multiple shared references or one mutable reference, but not both at the same time:

```ts
function readX(p: &Point): f64 {
    return p.x;
}

const point = Point { x: 10.0, y: 20.0 };
const ref = &point;
println(readX(ref));
```

### Generics and type aliases

```ts
struct Container<T> {
    value: T;
}

type NumberContainer = Container<i32>;
const box: NumberContainer = { value: 42 };
```

Generic functions:

```ts
function identity<T>(x: T): T {
    return x;
}
```

### Interfaces

```ts
interface Reader {
    read(key: string): string;
}

function useReader(reader: Reader): string {
    return reader.read("config");
}
```

### Skills (mixins)

Skills are reusable behavior bundles that structs can embody. They can contain concrete methods (with a body), abstract methods (signature only), and required fields:

```ts
skill Bark {
    bark(): void {
        print("Woof!");
    }
}

skill Greeter {
    name: string;  // required field

    greet(): void with &this {
        print("Hi! My name is ");
        println(this.name);
    }
}

struct Dog embodies Bark {
    breed: string;
}

struct Person embodies Bark, Greeter {
    name: string;
    age: i32;
}

const dog = Dog { breed: "Labrador" };
dog.bark();  // "Woof!"

const person = Person { name: "Alice", age: 30 };
person.greet();  // "Hi! My name is Alice"
```

When a struct embodies a skill, concrete methods are automatically mixed in. The struct can override any skill method by providing its own implementation.

### Enums

```ts
enum Mode { Dev, Test, Prod }

const mode = Mode.Dev;
```

### Function types

```ts
type Reducer = (acc: i32, value: i32) => i32;

function reduceThree(a: i32, b: i32, c: i32, reducer: Reducer): i32 {
    let acc = reducer(0, a);
    acc = reducer(acc, b);
    return reducer(acc, c);
}
```

### Control flow

Argon supports `if`/`else`, `for`, `while`, `loop`, `switch`, `match`, `break`, `continue`:

```ts
match (n) {
    0 => return "zero",
    1 => return "one",
    2 => return "two",
}

loop {
    count = count + 1;
    if (count >= 10) {
        break;
    }
}
```

### Async/await

```ts
async function fetchLabel(id: i32): string {
    return `user-${id}`;
}

async function main(): string {
    const label = await fetchLabel(7);
    return label;
}
```

Async works with the standard library's I/O functions:

```ts
from "std:fs" import { readFileAsync, writeFileAsync };
from "std:async" import { sleep };

async function main(): void {
    await writeFileAsync("/tmp/hello.txt", "async I/O!");
    const content = await readFileAsync("/tmp/hello.txt");
    println(content.value);    // "async I/O!"
    await sleep(100);          // sleep 100ms
}
```

### Error handling

```ts
function divide(a: i32, b: i32): Result<i32, string> {
    if (b == 0) {
        return Err { error: "division by zero" };
    }
    return Ok { value: a / b };
}

const result = divide(10, 2);
match (result) {
    Ok(value) => println(value),
    Err(error) => println(error),
}
```

### JavaScript interop

Declare external JS modules with `@js-interop` and export Argon functions with `@export`:

```ts
@js-interop
declare module "axios" {
    function get<T>(url: string): string;
}

from "axios" import Axios;

@export
function processImage(data: i32, width: i32, height: i32): i32 {
    return data + width + height;
}
```

### Imports and modules

Argon uses a `from ... import` syntax for all imports:

```ts
// Import from the standard library
from "std:io" import { println };
from "std:math" import { sqrt, PI, sin, cos };

// Import from other Argon files
from "./math" import { add, multiply };

// Namespace import
from "./utils" import Utils;

// Import from external packages (e.g. npm)
from "axios" import Axios;

// Side-effect import
from "reflect-metadata" import;

// Re-export
export { add, multiply } from "./math";
```

Multi-file projects can be compiled with `--out-dir`:

```bash
argon compile src/main.arg --target js --out-dir dist/
```

## Standard Library

Argon ships with a standard library written in `.arg` files, imported via `std:` module paths:

### `std:io`

```ts
from "std:io" import { print, println };

print("hello ");   // outputs without trailing newline
println("world");  // outputs with trailing newline
```

### `std:math`

```ts
from "std:math" import { sqrt, PI, abs, floor, ceil, sin, cos, pow, max, min, clamp };

println(sqrt(16.0));       // 4
println(PI);               // 3.14159...
println(abs(-42.0));       // 42
println(max(10.0, 20.0));  // 20
println(clamp(15.0, 0.0, 10.0)); // 10
```

Available functions: `abs`, `floor`, `ceil`, `round`, `trunc`, `sign`, `min`, `max`, `clamp`, `sqrt`, `cbrt`, `pow`, `hypot`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log`, `log2`, `log10`, `exp`.

Constants: `PI`, `E`, `TAU`, `LN2`, `LN10`, `SQRT2`.

### `std:error`

```ts
from "std:error" import { IoError };
```

Unified error type used across all I/O modules. `IoError` has `code` (e.g., `"ENOENT"`, `"ECONNREFUSED"`) and `message` fields.

### `std:fs`

```ts
from "std:fs" import { readFile, writeFile, exists, stat, mkdir, readDir, open };

const wr = writeFile("/tmp/hello.txt", "Hello!");
const rd = readFile("/tmp/hello.txt");
println(rd.value);  // Hello!

const st = stat("/tmp/hello.txt");
println(st.value.size);    // 6
println(st.value.isFile);  // true

// Streaming I/O
const f = open("/tmp/data.txt", "Write");
f.value.write("streaming");
f.value.close();
```

All functions return `Result<T, IoError>`. Async variants are available: `readFileAsync`, `writeFileAsync`, etc.

Available functions: `readFile`, `writeFile`, `readBytes`, `writeBytes`, `appendFile`, `readDir`, `mkdir`, `mkdirRecursive`, `rmdir`, `removeRecursive`, `exists`, `stat`, `rename`, `remove`, `copy`, `symlink`, `readlink`, `tempDir`, `open`.

### `std:net`

```ts
from "std:net" import { bind, connect, resolve };

// TCP server
const server = bind("127.0.0.1", 8080);
const stream = server.value.accept();

// TCP client
const client = connect("127.0.0.1", 8080);
client.value.write("hello");

// DNS
const ips = resolve("localhost");
println(ips.value);  // [::1, 127.0.0.1]
```

Includes `TcpListener`, `TcpStream`, `UdpSocket`, and DNS resolution. Not available on the WASM target.

### `std:http`

```ts
from "std:http" import { get, post, createHeaders };

const resp = get("https://api.example.com/data");
println(resp.value.status);  // 200
println(resp.value.body);

const headers = createHeaders();
headers.set("Content-Type", "application/json");
const resp2 = post("https://api.example.com/data", "{}", headers);
```

HTTP client with `get`, `post`, `put`, `del`, `request`. Async variants available (`getAsync`, `postAsync`, etc.). Includes `Headers`, `Response`, `RequestOptions` types. Server support via `serve(port, handler)`.

### `std:ws`

```ts
from "std:ws" import { wsConnect, wsListen };

// Client
const conn = wsConnect("ws://localhost:8080");
conn.value.send("hello");
const msg = conn.value.recv();
println(msg.value.data);

// Server
const server = wsListen("127.0.0.1", 9090);
const client = server.value.accept();
```

WebSocket client and server with text/binary message support, ping/pong, and close codes.

### `std:async`

```ts
from "std:async" import { spawn, sleep };

await sleep(1000);  // sleep 1 second
```

### Prelude types

These types are available without an import: `Vec<T>`, `Some<T>`, `None`, `Ok<T,E>`, `Err<T,E>`, `Shared<T>`, `Map<K,V>`, `Set<T>`, `Future<T>`, `Task<T>`, `Waker`.

## Compilation Targets

### JavaScript

The default target. Emits ES2022 JavaScript:

```bash
argon compile app.arg --target js -o app.js
argon compile app.arg --target js -o app.js --source-map --declarations
```

With `--source-map`, generates `app.js.map` for debugger support. With `--declarations`, generates `app.d.ts` for TypeScript consumers.

### WebAssembly

Compiles the validated native subset to `.wasm` with sidecar loaders:

```bash
argon compile math.arg --target wasm -o math.wasm
```

This produces three files:
- `math.wasm` — binary WebAssembly module
- `math.mjs` — module loader that merges native exports with host companions
- `math.host.mjs` — JS host implementations for features that need runtime support

The WASM target supports numeric operations, control flow, function calls, branching, loops, array indexing, heap-backed object access, `Result`-based error handling, async/await (lowered synchronously), and direct function imports. `std:fs` works via WASI. `std:http` and `std:ws` clients work via host companion JS. `std:net` (raw sockets), HTTP servers, and WebSocket servers are not available on WASM and produce a compile-time error. Features outside the supported subset fail at compile time with a clear error.

### Native

Produces OS-executable binaries using Cranelift as the backend. Works like Rust's compilation model — target triples specify the OS/arch:

```bash
# Compile for the current platform
argon compile app.arg --target native -o app
./app

# Cross-compile (generates .o, requires manual linking)
argon compile app.arg --target native --triple x86_64-unknown-linux-gnu --emit obj

# Emit assembly
argon compile app.arg --target native --emit asm -o app.s
```

When no `--triple` is specified, the compiler targets the host OS and architecture. The native target supports:
- Arithmetic, comparisons, and logical operators
- Variables, constants, and global initialization
- Functions, recursion, and return values
- Control flow: `if`/`else`, `while`, `for`, `loop`, `break`, `continue`, `switch`
- `print`/`println` with numbers, booleans, and strings
- Math intrinsics via libc (`sin`, `cos`, `sqrt`, `pow`, etc.)
- File system operations (`readFile`, `writeFile`, `exists`, `stat`, `mkdir`, `remove`, etc.)
- Networking operations (TCP bind/connect/accept, UDP, DNS)

Supported platforms: macOS (aarch64, x86_64), Linux (aarch64, x86_64), Windows (x86_64). A C compiler (`cc` or `link.exe`) is required for linking.

## How the Compiler Works

Argon runs a fixed multi-stage pipeline:

```
Source (.arg)
    |
    v
 Lexer ──────── Tokenize source text
    |
    v
 Parser ─────── Build abstract syntax tree
    |
    v
 Type Checker ── Resolve types, validate generics, check interfaces/enums
    |
    v
 Borrow Checker ── Enforce ownership, move, and borrow rules
    |
    v
 IR Builder ──── Lower to control-flow IR with optional optimization passes
    |
    v
 Async Lowering ── Transform async functions into state machines (native/WASM only)
    |
    v
 Code Generator
    |── JS backend ────── ES2022 JavaScript + source maps + .d.ts
    |── WASM backend ──── .wasm + .mjs loader + .host.mjs companion
    |── Native backend ── Machine code via Cranelift + system linker
```

The `--pipeline ir` path (default) lowers through the IR for optimization. The `--pipeline ast` path generates code directly from the AST.

For `argon run`, the type-checked and borrow-checked AST is executed directly by the built-in runtime — no Node.js required.

### Workspace layout

The compiler is organized as a Rust workspace with 18 crates:

```
crates/
  argon-cli              CLI entrypoint and commands
  argon-driver           Pipeline orchestration
  argon-lexer            Tokenization
  argon-parser           AST parsing
  argon-ast              AST node definitions
  argon-types            Type checker and type model
  argon-borrowck         Ownership and borrow checking
  argon-ir               IR representation and optimization passes
  argon-codegen-js       JavaScript code generation
  argon-codegen-wasm     WebAssembly code generation
  argon-codegen-native   Native code generation via Cranelift
  argon-target           Target triple abstraction and host detection
  argon-runtime          AST interpreter for `argon run`
  argon-interop          JS/WASM interop helpers
  argon-stdlib           Standard library (.arg source files)
  argon-diagnostics      Error reporting and rendering
  argon-backend-traits   Backend trait abstractions for I/O operations
  argon-async            Async runtime (work-stealing scheduler, mio reactor)
```

## Contributing

### Build and test

```bash
cargo build                                     # Debug build
cargo test --workspace --all-targets            # All tests
cargo test --workspace --doc                    # Doc tests
cargo fmt --all -- --check                      # Format check
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

### Explore the language

The `examples/` directory contains `.arg` files covering every major feature:

```bash
# Check all examples
for f in examples/*.arg; do argon check "$f"; done

# Run an example
argon run examples/collections.arg

# Compile to JS and execute
argon compile examples/control-flow.arg --target js -o /tmp/out.js && node /tmp/out.js

# Compile to native and execute
argon compile examples/arithmetic.arg --target native -o /tmp/arithmetic && /tmp/arithmetic
```

Key examples by topic:
- **Ownership/borrowing:** `ownership.arg`, `borrowing.arg`
- **Structs:** `structs.arg`, `simple_method.arg`, `native_structs.arg`
- **Generics:** `generic_simple.arg`, `generic_fn.arg`, `generic_struct.arg`
- **Control flow:** `control-flow.arg`, `match.arg`, `result-match.arg`, `recursion.arg`
- **Type system:** `interface.arg`, `enum.arg`, `type_test.arg`
- **Skills:** `skills.arg`
- **Standard library:** `std_math.arg`, `arithmetic.arg`
- **File system:** `fs_test.arg`, `fs_file_handle.arg`, `fs_native_test.arg`
- **Networking:** `net_test.arg`
- **HTTP:** `http_test.arg`
- **WebSocket:** `ws_test.arg`
- **Async:** `async.arg`, `async_sleep.arg`, `async_fs.arg`, `async_http.arg`
- **Multi-file:** `modules/main.arg`
- **Interop:** `interop.arg`
- **WASM:** `wasm-subset.arg`

## License

MIT
