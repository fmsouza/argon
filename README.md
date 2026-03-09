# Argon Compiler Implementation Guide

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)
[![Lines of Code](https://img.shields.io/badge/loc-15%2C844-blue.svg)](#)
[![Coverage](https://img.shields.io/badge/coverage-70%25-yellow.svg)](#)

**Version:** 1.0  
**Date:** March 9, 2026  
**Status:** Implementation In Progress (Phases 1-8 Active, WASM Pending)

---

## Quick Start

```bash
# Clone and build
cargo build --release

# Run tests
cargo test --all

# Run examples directly (uses AST interpreter)
argon run examples/structs.arg

# Type check
argon check input.arg

# Compile to JavaScript
argon compile input.arg --target js -o output.js

# Run tests
argon test examples/option-result.arg

# Initialize new project
argon init my-project
```

---

## Implementation Status Summary (as of March 2026)

> **Note:** This file reflects the current implementation state. Items marked [x] are implemented unless otherwise noted in comments.

**Phase 1 (Foundation):** ~75% complete ✓

- Project setup, CLI, diagnostics: Complete
- Lexer: 75% (JSX tokenization partial)
- Parser: 80% (comprehensive)
- Error reporting: Complete

**Phase 2 (Type System):** ~50% complete

- Type definitions, primitives, references: Complete
- Type inference: Basic (40%)
- Generics: Defined but not instantiated
- Type interop with JS: Partial

**Phase 3 (Borrow Checker):** ~45% complete

- Ownership tracking, moves, copies: Complete
- Borrow validation: Complete
- Lifetime inference: Basic scope-based only
- Send + Sync checking: Not started
- Async safety: Not started

**Phase 4 (IR & Optimization):** ~35% complete

- IR definitions: Complete
- AST → IR lowering: Basic
- SSA construction: Not started
- Optimizations: Not started

**Phase 5 (JS Backend):** ~60% complete ✓

- IR → JavaScript: Basic
- ES2022 output: Working
- .d.ts generation: Working
- Source maps: Stub
- Full ES modules: In progress
- JSX transformation: Not started

**Phase 6 (WASM Backend):** ~5% complete

- IR → WASM: Stub only
- Linear memory: Not started
- Type mapping: Not started
- WASM/JS interop: Not started

**Phase 7 (Stdlib & Runtime):** ~75% complete ✓

- Runtime interpreter: Complete
- Vec<T>, Option<T>, Result<T,E>: Complete
- Shared<T>, Map<K,V>, Set<T>: Complete
- Native functions (console, Math): Complete
- Test utilities: Complete

**Phase 8 (CLI & Tooling):** ~70% complete ✓

- compile, check, run, test: Working
- format, init: Working
- Watch mode: Not started
- REPL: Not started
- LSP server: Not started

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Implementation Status Summary](#implementation-status-summary-as-of-march-2026)
3. [Executive Summary](#executive-summary)
4. [Project Architecture](#project-architecture)
5. [Working Examples](#working-examples)
6. [Language Specification](#language-specification)
7. [Next Steps](#next-steps)
8. [Compiler Phases](#compiler-phases)
9. [Implementation Phases](#implementation-phases)
10. [Testing Strategy](#testing-strategy)
11. [Success Criteria](#success-criteria)

---

## Executive Summary

Argon is a programming language that combines TypeScript's familiar syntax with Rust's memory safety guarantees. The compiler is written in Rust and targets both JavaScript (ES2022+) and WebAssembly.

### Core Goals

1. **TypeScript-like syntax** - TypeScript developers feel at home
2. **Rust memory safety** - Ownership, borrowing, lifetimes enforced at compile time
3. **JavaScript interop** - Seamless integration with existing JS ecosystem
4. **WASM support** - High-performance compiled code for performance-critical paths
5. **Multi-threading** - Web Workers + SharedArrayBuffer support

### Target Use Cases

- Frontend: React/Vue apps with memory-safe components
- Backend: Express/Node.js services with deterministic resource cleanup
- Performance: WASM hot paths for compute-intensive operations
- Systems: Safe concurrent code with data race prevention

---

## Project Architecture

### Workspace Structure

```
argon/
├── Cargo.toml              # Workspace manifest
├── rust-toolchain.toml     # Rust version specification
├── crates/
│   ├── argon-lexer/          # Tokenization
│   ├── argon-parser/        # AST generation
│   ├── argon-ast/           # AST definitions
│   ├── argon-types/         # Type system
│   ├── argon-borrowck/      # Borrow checker
│   ├── argon-ir/            # Intermediate representation
│   ├── argon-codegen-js/    # JavaScript codegen
│   ├── argon-codegen-wasm/  # WebAssembly codegen
│   ├── argon-interop/       # JS interop layer
│   ├── argon-stdlib/        # Standard library (JS runtime)
│   ├── argon-runtime/       # AST interpreter
│   ├── argon-cli/           # CLI interface
│   └── argon-diagnostics/   # Error reporting
├── tests/
│   ├── integration/               # End-to-end tests
│   └── fixtures/                  # Test programs
├── examples/                       # Example programs
└── docs/                          # Documentation
```

### Dependencies (Key Crates)

- **ariadne** - Beautiful error diagnostics
- **bumpalo** - Arena allocation
- **indexmap** - Ordered maps
- **smallvec** - Small vector optimization
- **lasso** - String interning
- **cranelift** - WASM codegen
- **wasm-encoder** - WASM binary encoding
- **proptest** - Property-based testing
- **criterion** - Benchmarking
- **insta** - Snapshot testing

---

## Working Examples

The following 16 example programs are implemented and working:

| Example | Description | Status |
|---------|-------------|--------|
| `arithmetic.arg` | Basic math operations | ✓ |
| `boolean.arg` | Boolean operations | ✓ |
| `borrowing.arg` | Borrow checking demos | ✓ |
| `classes.arg` | Class definitions and methods | ✓ |
| `collections.arg` | Vec, Map, Set usage | ✓ |
| `control-flow.arg` | if/else, switch, match | ✓ |
| `function.arg` | Function definitions | ✓ |
| `functions.arg` | Function with ownership | ✓ |
| `match.arg` | Pattern matching | ✓ |
| `numeric-types.arg` | Number type conversions | ✓ |
| `object.arg` | Object literals | ✓ |
| `option-result.arg` | Option<T> and Result<T,E> | ✓ |
| `ownership.arg` | Ownership and moves | ✓ |
| `recursion.arg` | Recursive functions | ✓ |
| `strings.arg` | String operations | ✓ |
| `structs.arg` | Struct definitions | ✓ |

### Running Examples

```bash
# Run with interpreter
argon run examples/structs.arg

# Compile to JavaScript
argon compile examples/classes.arg --target js -o out.js
node out.js

# Type check
argon check examples/borrowing.arg

# Run tests
argon test examples/option-result.arg
```

---

## Language Specification

### 1. Syntax (TypeScript-like)

```typescript
// Structs: stack-allocated, copied by default
struct Point {
    x: f64;
    y: f64;
}

// Classes: heap-allocated, moved by default
class Canvas {
    private pixels: Vec<u8>;

    constructor(width: u32, height: u32) {
        this.pixels = new Vec(width * height * 4);
    }

    // &mut this = exclusive mutable borrow
    drawPixel(x: u32, y: u32, color: Color): void with &mut this {
        // ...
    }

    // &this = shared immutable borrow
    getPixel(x: u32, y: u32): Color with &this {
        // ...
    }
}

// Ownership transfer (move)
function renderLoop(canvas: Canvas): never {
    loop {
        canvas.drawPixel(0, 0, RED);
    }
}

// Borrowing
function render(canvas: &mut Canvas, scene: &Scene): void {
    for (const obj of scene.objects) {
        canvas.drawPixel(obj.x, obj.y, obj.color);
    }
}

// Option type
function findUser(id: u64): User? {
    // ...
}

// Pattern matching
function describe(val: string | number | null): string {
    match val {
        null        => "nothing",
        n: number   => `the number ${n}`,
        s: string   => `"${s}"`,
    }
}

// Async
async function fetchData(url: &str): Promise<Data> {
    const response = await fetch(url.toString());
    return response.json();
}

// Generics
interface Drawable {
    draw(canvas: &mut Canvas): void;
}

function renderAll<T extends Drawable>(items: &[T], canvas: &mut Canvas): void {
    for (const item of items) {
        item.draw(canvas);
    }
}
```

### 2. Ownership Model

| Concept          | Syntax      | Description                           |
| ---------------- | ----------- | ------------------------------------- |
| Owned            | `T`         | Unique owner, dropped at end of scope |
| Shared borrow    | `&T`        | Immutable, many can coexist           |
| Mutable borrow   | `&mut T`    | Exclusive, only one at a time         |
| Shared ownership | `Shared<T>` | Reference counted (Arc-like)          |

### 3. Memory Safety Guarantees

- **No use-after-free**: References validated at compile time
- **No data races**: Borrow checker prevents concurrent &mut access
- **No null dereference**: Option<T> requires exhaustive handling
- **Deterministic cleanup**: RAII pattern, no GC pause times

### 4. JavaScript Interop

```typescript
// Import JS library (automatic Shared<T> wrapping)
import { useState, useEffect } from "react";

// Export Argon to JS
@export
function processImage(data: &[u8], width: u32, height: u32): Vec<u8> {
    // Fully borrow-checked implementation
}

// JS interop annotation
@js-interop
declare module "axios" {
    function get<T>(url: string): Promise<AxiosResponse<T>>;
}
```

---

## Next Steps: Completing the MVP

### Priority 1: Production-Ready JS Compilation

| Feature | Status | Notes |
|---------|--------|-------|
| JSX Parsing | Partial | Basic elements work, nested needs fixing |
| JSX Codegen | Partial | Simple React.createElement output |
| Async/Await | Working | Basic async functions and await work |
| Source Maps | Basic | Stub structure, needs full token tracking |
| ES Modules | Partial | Codegen added, parser has issues |

### Priority 2: Type System Completion

| Feature | Status | Notes |
|---------|--------|-------|
| Bidirectional Type Checking | Not started | Better error messages |
| Constraint Generation | Not started | Full generic support |
| Lifetime Bounds | Not started | Better borrow checking |
| Type Interop with JS | Partial | Needs completion |

### Priority 3: Developer Experience

| Feature | Status | Notes |
|---------|--------|-------|
| Watch Mode | Not started | Incremental rebuilds |
| REPL | Not started | Interactive development |
| LSP Server | Not started | IDE integration |
| Incremental Compilation | Not started | Sub-second rebuilds |

### Priority 4: WASM Backend (Future)

| Feature | Status | Notes |
|---------|--------|-------|
| IR → WASM Translation | Stub | Only returns 42 |
| Linear Memory Layout | Not started | Memory management |
| Cranelift Integration | Not started | Performance |
| WASM/JS Interop | Not started | Typed arrays, promises |

---

## Compiler Phases

### Phase 1: Lexing

**Input:** Source code string  
**Output:** Token stream with spans

```
Source → Lexer → Tokens
```

Key responsibilities:

- Unicode-aware tokenization
- JSX parsing
- Template literals
- Error recovery

### Phase 2: Parsing

**Input:** Token stream  
**Output:** Typed AST

```
Tokens → Parser → AST
```

Key responsibilities:

- Recursive descent parsing
- Error recovery
- Incremental parsing support

### Phase 3: Type Checking

**Input:** Typed AST  
**Output:** Type-annotated AST

```
AST → TypeChecker → Annotated AST
```

Key responsibilities:

- Structural typing
- Type inference
- Generic constraints
- Ownership types

### Phase 4: Borrow Checking

**Input:** Type-annotated AST  
**Output:** Validated AST (ownership constraints satisfied)

```
Annotated AST → BorrowChecker → Validated AST
```

Key responsibilities:

- Move/copy analysis
- Borrow validation
- Lifetime inference
- Data race detection

### Phase 5: IR Generation

**Input:** Validated AST  
**Output:** SSA-based IR

```
Validated AST → IRBuilder → IR
```

Key responsibilities:

- SSA construction
- Control flow graph
- Ownership-aware instructions

### Phase 6: Code Generation

**Input:** IR  
**Output:** Target code (JS or WASM)

```
IR → Codegen → JavaScript
IR → Codegen → WebAssembly
```

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-4)

#### Week 1: Project Setup

- [x] Create workspace structure
- [x] Set up CI/CD
- [x] Configure logging and error handling
- [x] Basic CLI skeleton

#### Week 2: Lexer

- [x] Token kinds definition
- [x] Unicode handling
- [x] JSX tokenization (partial)
- [x] Template literals

#### Week 3: Parser

- [x] AST node definitions
- [x] Recursive descent parser
- [x] Expression parsing
- [x] Statement parsing
- [x] Pattern matching
- [x] Import/export

#### Week 4: Error Reporting

- [x] Diagnostic definitions
- [x] Span tracking
- [x] Error message formatting
- [x] Colorized output

### Phase 2: Type System (Weeks 5-8)

#### Week 5: Type Representation

- [x] Primitive types
- [x] Compound types (struct, class, array)
- [x] Reference types

#### Week 6: Type Inference

- [x] Unification algorithm
- [ ] Bidirectional checking
- [ ] Constraint generation

#### Week 7: Advanced Types

- [x] Generics (definition, not instantiation)
- [x] Union/intersection types
- [x] Option/Result types

#### Week 8: Ownership Types

- [x] Owned vs borrowed distinction
- [x] Shared<T> type
- [x] Type interop with JS (partial)

### Phase 3: Borrow Checker (Weeks 9-12)

#### Week 9: Ownership Tracking

- [x] Move detection
- [x] Copy inference
- [x] Drop analysis

#### Week 10: Borrow Validation

- [x] Shared borrow rules
- [x] Mutable borrow rules
- [x] Borrow conflicts

#### Week 11: Lifetime Inference

- [x] Elision rules
- [ ] Lifetime bounds
- [ ] Return lifetime analysis

#### Week 12: Advanced Checking

- [x] Data race detection
- [ ] Async safety
- [ ] Thread safety (Send + Sync)

### Phase 4: IR & Optimization (Weeks 13-15)

#### Week 13: IR Definition

- [x] Instruction set
- [x] Basic blocks
- [x] Control flow graph

#### Week 14: IR Generation

- [x] AST → IR lowering (basic)
- [ ] Function inlining
- [ ] Constant folding
- [ ] SSA construction

#### Week 15: Optimizations

- [ ] Common subexpression elimination
- [ ] Dead code elimination
- [ ] Copy propagation

### Phase 5: JavaScript Backend (Weeks 16-18)

#### Week 16: JS Codegen

- [x] IR → JavaScript (basic)
- [x] ES2022 output
- [x] Module generation

#### Week 17: Source Maps & Types

- [ ] Source map generation
- [x] .d.ts output
- [x] TypeScript interop (basic)

#### Week 18: JSX & Interop

- [ ] JSX transformation
- [ ] React support
- [ ] JS library integration

### Phase 6: WASM Backend (Weeks 19-22)

#### Week 19: WASM Setup

- [ ] Linear memory layout
- [ ] Type mapping (Argon → WASM)
- [ ] Cranelift integration

#### Week 20: WASM Codegen

- [ ] IR → WASM (stub: returns 42)
- [ ] Function calls
- [ ] Memory management

#### Week 21: WASM/JS Interop

- [ ] Import/export handling
- [ ] Typed array marshalling
- [ ] Promise integration

#### Week 22: Optimization

- [ ] SIMD support
- [ ] Loop optimizations
- [ ] Performance tuning

### Phase 7: Runtime & Stdlib (Weeks 23-25)

#### Week 23: Runtime Interpreter

- [x] AST interpreter
- [x] Value types (Number, String, Boolean, etc.)
- [x] Native functions (console.log, Math.*)
- [x] Scope management
- [x] Function calls with closures

#### Week 24: Standard Library (JS Runtime)

- [x] Vec<T>
- [x] Option<T>
- [x] Result<T, E>
- [x] Shared<T>
- [x] String, &str
- [x] Map<K, V>
- [x] Set<T>
- [x] Test utilities

#### Week 25: Async Runtime

- [ ] Promise integration
- [ ] Async/await lowering
- [ ] Future implementation

### Phase 8: Tooling & CLI (Weeks 26-27)

#### Week 26: CLI Commands

- [x] compile - Compile to JS/WASM
- [x] check - Type and borrow check
- [x] run - Execute with interpreter
- [x] test - Run test files
- [x] format - Basic formatter
- [x] init - Create new project

#### Week 27: Developer Experience

- [ ] Watch mode
- [ ] REPL
- [ ] Incremental compilation
- [ ] LSP server

### Phase 9: Advanced Features (Weeks 28-30)

#### Week 28: Advanced Type System

- [ ] Bidirectional type checking
- [ ] Constraint generation
- [ ] Full generic instantiation

#### Week 29: JSX & Interop

- [ ] JSX parsing
- [ ] React component support
- [ ] JS library integration

#### Week 30: Polish

- [ ] Documentation
- [ ] Examples
- [ ] Performance finalization
- [ ] Performance finalization

---

## Testing Strategy

### Test Categories

1. **Unit Tests** - Per-crate functionality
2. **Integration Tests** - Full compilation pipeline
3. **Property-Based Tests** - Invariant verification
4. **Fuzzing** - Input validation
5. **Runtime Tests** - Executed output verification
6. **Snapshots** - Expected output matching

### Coverage Targets

| Metric          | Target |
| --------------- | ------ |
| Line Coverage   | ≥90%   |
| Branch Coverage | ≥85%   |
| Fuzz Iterations | 1M+    |
| Property Tests  | 10k+   |

---

## Success Criteria

### Must Have (MVP)

1. **JavaScript Compilation**
   - [x] Argon → ES2022 JavaScript (basic)
   - [x] Correct type erasure
   - [x] .d.ts output
   - [ ] ES modules output

2. **WASM Compilation**
   - [ ] Argon → WebAssembly (stub only)
   - [ ] Linear memory management
   - [ ] No runtime memory errors

3. **JS Interop**
   - [x] Import existing JS libraries (basic)
   - [x] Type-safe boundary (partial)
   - [x] Shared<T> wrapping

4. **Core Language Features**
   - [x] Structs and classes
   - [x] Functions with ownership
   - [x] Basic borrow checking
   - [x] Type inference (basic)
   - [x] Pattern matching
   - [x] Option/Result types
   - [x] Runtime interpreter

### Should Have

- [x] Full borrow checker (basic)
- [x] Lifetime inference (basic)
- [x] Standard library (JS runtime)
- [ ] Async/await
- [ ] JSX support
- [ ] Source maps
- [ ] Full ES modules

### Nice to Have

- [ ] Multi-threading
- [ ] Incremental compilation
- [ ] LSP server
- [ ] WASM backend

---

## Example Programs

### Building

```bash
# Clone and build
cargo build --release

# Run tests
cargo test --all

# Run benchmarks
cargo bench
```

### Using the Compiler

```bash
# Compile to JavaScript
argon compile input.arg --target js -o output.js

# Compile to WASM
argon compile input.arg --target wasm -o output.wasm

# Type check only
argon check input.arg

# Run with interpreter
argon run input.arg
```

### Example: Hello World

```typescript
// hello.arg
struct Greeter {
    name: string;

    greet(): string with &this {
        return `Hello, ${this.name}!`;
    }
}

const greeter = Greeter { name: "World" };
console.log(greeter.greet());
```

```bash
# Compile and run
argon compile hello.arg --target js -o hello.js
node hello.js
# Output: Hello, World!
```

### Example: JS Interop

```typescript
// app.arg
import { useState } from "react";

@export
function counter(): number {
    const [count, setCount] = useState(0);
    return count;
}
```

```typescript
// React component using Argon
import { counter } from "./app.arg";

function App() {
    return <div>Count: {counter()}</div>;
}
```

---

## Implementation Notes

### Key Design Decisions

1. **Parser**: Handwritten recursive descent (not LALR) for better error recovery
2. **Type System**: Unification-based inference (bidirectional pending)
3. **Borrow Checker**: Scope-based lifetime tracking
4. **IR**: Basic SSA-like (not yet full SSA)
5. **WASM**: Stub backend (cranelift not yet integrated)

### Runtime Interpreter

The compiler includes an AST interpreter for direct execution:

- Direct AST execution without code generation
- Useful for testing and rapid prototyping
- Supports all core language features
- Run with `argon run <file>`

### Incremental Compilation

Incremental compilation is planned but not yet implemented:

- Planned: Parsed ASTs cached
- Planned: Type information cached
- Planned: Only affected functions recompiled
- Target: <100ms for small changes

### Debugging

The compiler includes debugging features:

- Source locations for all errors
- Type inference debug output
- Borrow checking debug output
- IR visualization (basic)

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to catch common mistakes
- Write documentation for public APIs
- Add tests for bug fixes

---

## License

MIT License - See LICENSE file for details.

---

## Appendix A: Crate Dependencies

```
argon-cli
    ├── argon-codegen-js
    │   ├── argon-ir
    │   │   ├── argon-borrowck
    │   │   │   ├── argon-types
    │   │   │   │   ├── argon-ast
    │   │   │   │   │   ├── argon-parser
    │   │   │   │   │   │   ├── argon-lexer
    │   │   │   │   │   └── argon-diagnostics
    │   │   │   │   └── (dependencies)
    │   │   │   └── (dependencies)
    │   │   └── (dependencies)
    │   └── (dependencies)
    ├── argon-codegen-wasm
    │   └── (similar structure, stub)
    ├── argon-interop
    │   └── (dependencies)
    ├── argon-stdlib
    │   └── (dependencies)
    ├── argon-runtime
    │   ├── argon-ast
    │   ├── argon-types
    │   └── (dependencies)
    └── (dependencies)
```

## Appendix B: File Extensions

| Extension   | Description                         |
| ----------- | ----------------------------------- |
| `.arg`      | Argon source file                   |
| `.arg.d.ts` | Argon declaration file (TypeScript) |
| `.wat`      | WebAssembly text format             |

## Appendix C: CLI Commands

```
argon <command> [options]

Commands:
    compile    Compile source file(s) to JS/WASM
    check      Type and borrow check without emitting code
    run        Execute source file(s) with interpreter
    test       Run test file(s)
    format     Format source files
    init       Initialize new Argon project

Options:
    --target    Target: js, wasm (default: js)
    --output    Output file path
    --source-map    Generate source maps
    --declarations  Generate .d.ts files
    --optimize  Optimization level: 0-3
```
