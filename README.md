# Argon

A TypeScript-like language with Rust-inspired ownership and borrowing, compiling to JavaScript and WebAssembly.

Argon gives you familiar syntax — structs, interfaces, generics, async/await — while the compiler enforces memory-safety rules at compile time. Write code that looks like TypeScript, get the safety guarantees of Rust, and run it anywhere JavaScript or WebAssembly runs.

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
console.log(moved.x); // 5.0
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
console.log(counter.getValue()); // 1
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
struct Greeting {
    message: string;
}

function greet(g: &Greeting): string {
    return g.message;
}

const hello = Greeting { message: "Hello, Argon!" };
console.log(greet(&hello));
```

```bash
# Type-check and borrow-check
argon check hello.arg

# Compile to JavaScript
argon compile hello.arg --target js -o hello.js
node hello.js

# Or run directly with the built-in runtime
argon run hello.arg
```

### Try the REPL

```bash
argon repl
```

The REPL supports multi-line input and has built-in commands: `:load <file>`, `:check`, `:compile [js|wasm]`, `:show`, `:reset`.

## CLI Reference

| Command | Description |
|---------|-------------|
| `argon compile <file.arg>` | Compile to JS or WASM |
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
| `--target js\|wasm` | Output target (default: `js`) |
| `--pipeline ast\|ir` | Compilation pipeline (default: `ir`, preferred) |
| `-o <path>` | Output file path |
| `--source-map` | Generate source maps (JS target) |
| `--opt` | Enable optimization passes |
| `--declarations` | Generate TypeScript `.d.ts` declarations |

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
console.log(red.r);
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
console.log(counter.getValue()); // 1
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
// console.log(m.text);  // ERROR: use after move
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
console.log(readX(ref));
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

### Error handling

```ts
try {
    if (flag) {
        throw 7;
    }
    return 1;
} catch (err) {
    console.log(err);
    return 0;
} finally {
    console.log("cleanup");
}
```

### JavaScript interop

Declare external JS modules with `@js-interop` and export Argon functions with `@export`:

```ts
@js-interop
declare module "axios" {
    function get<T>(url: string): string;
}

@export
function processImage(data: i32, width: i32, height: i32): i32 {
    return data + width + height;
}
```

### ES modules

```ts
import axios from "axios";
import { useState } from "react";

export function readEnv(): string {
    return "/api";
}
```

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

The WASM target supports numeric operations, control flow, function calls, branching, loops, array indexing, heap-backed object access, structured `try`/`catch`/`finally`, async/await (lowered synchronously), and direct function imports. Features outside this subset fail at compile time with a clear error.

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
 Code Generator
    |── JS backend ──── ES2022 JavaScript + source maps + .d.ts
    |── WASM backend ── Native .wasm + .mjs loader + .host.mjs companion
```

The `--pipeline ir` path (default) lowers through the IR for optimization. The `--pipeline ast` path generates code directly from the AST.

For `argon run`, the type-checked and borrow-checked AST is executed directly by the built-in runtime — no Node.js required.

### Workspace layout

The compiler is organized as a Rust workspace with 14 crates:

```
crates/
  argon-cli            CLI entrypoint and commands
  argon-driver         Pipeline orchestration
  argon-lexer          Tokenization
  argon-parser         AST parsing
  argon-ast            AST node definitions
  argon-types          Type checker and type model
  argon-borrowck       Ownership and borrow checking
  argon-ir             IR representation and optimization passes
  argon-codegen-js     JavaScript code generation
  argon-codegen-wasm   WebAssembly code generation
  argon-runtime        AST interpreter for `argon run`
  argon-interop        JS/WASM interop helpers
  argon-stdlib         Runtime standard library
  argon-diagnostics    Error reporting and rendering
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

# Compile and execute
argon compile examples/control-flow.arg --target js -o /tmp/out.js && node /tmp/out.js
```

Key examples by topic:
- **Ownership/borrowing:** `ownership.arg`, `borrowing.arg`
- **Structs:** `structs.arg`, `simple_method.arg`
- **Generics:** `generic_simple.arg`, `generic_fn.arg`, `generic_struct.arg`
- **Control flow:** `control-flow.arg`, `match.arg`, `try-catch.arg`, `recursion.arg`
- **Type system:** `interface.arg`, `enum.arg`, `type_test.arg`
- **Interop:** `interop.arg`, `esm.arg`
- **WASM:** `wasm-subset.arg`

## License

MIT
