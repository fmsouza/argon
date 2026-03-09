# Argon Compiler Implementation Guide

**Version:** 1.0  
**Date:** March 9, 2026  
**Status:** Implementation In Progress (Phase 1-4 Complete, Phase 5-9 Pending)

---

## Implementation Status Summary (as of March 2026)

> **Note:** This file reflects the current implementation state. Items marked [x] are implemented unless otherwise noted in comments.

**Phase 1 (Foundation):** ~75% complete
- Project setup, CLI, diagnostics: Complete
- Lexer: 70% (missing JSX tokenization, template interpolation)
- Parser: 50% (missing advanced expressions)
- Error reporting: Complete

**Phase 2 (Type System):** ~55% complete
- Type definitions, primitives, references: Complete
- Type inference: Basic (40%)
- Generics: Defined but not instantiated

**Phase 3 (Borrow Checker):** ~50% complete
- Ownership tracking, moves, copies: Complete
- Borrow validation: Complete
- Lifetime inference: Basic scope-based only

**Phase 4 (IR & Optimization):** ~40% complete
- IR definitions: Complete
- AST → IR lowering: Basic
- Optimizations: Not started

**Phase 5-9 (Backends, Async, Stdlib, Tooling):** Not started

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Architecture](#project-architecture)
3. [Language Specification](#language-specification)
4. [Compiler Phases](#compiler-phases)
5. [Implementation Phases](#implementation-phases)
6. [Testing Strategy](#testing-strategy)
7. [Success Criteria](#success-criteria)
8. [Quick Start](#quick-start)

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
│   ├── argon-parser/         # AST generation
│   ├── argon-ast/            # AST definitions
│   ├── argon-types/          # Type system
│   ├── argon-borrowck/       # Borrow checker
│   ├── argon-ir/             # Intermediate representation
│   ├── argon-codegen-js/     # JavaScript codegen
│   ├── argon-codegen-wasm/   # WebAssembly codegen
│   ├── argon-interop/        # JS interop layer
│   ├── argon-stdlib/         # Standard library
│   ├── argon-cli/            # CLI interface
│   └── argon-diagnostics/    # Error reporting
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

| Concept | Syntax | Description |
|---------|--------|-------------|
| Owned | `T` | Unique owner, dropped at end of scope |
| Shared borrow | `&T` | Immutable, many can coexist |
| Mutable borrow | `&mut T` | Exclusive, only one at a time |
| Shared ownership | `Shared<T>` | Reference counted (Arc-like) |

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
- [ ] JSX tokenization
- [x] Template literals

#### Week 3: Parser
- [x] AST node definitions
- [x] Recursive descent parser
- [x] Expression parsing
- [x] Statement parsing

#### Week 4: Error Reporting
- [x] Diagnostic definitions
- [x] Span tracking
- [x] Error message formatting

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
- [ ] Generics with constraints
- [x] Union/intersection types
- [x] Option/Result types

#### Week 8: Ownership Types
- [x] Owned vs borrowed distinction
- [x] Shared<T> type
- [ ] Type interop with JS

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
- [x] AST → IR lowering
- [ ] Function inlining
- [ ] Constant folding

#### Week 15: Optimizations
- [ ] Common subexpression elimination
- [ ] Dead code elimination
- [ ] Copy propagation

### Phase 5: JavaScript Backend (Weeks 16-18)

#### Week 16: JS Codegen
- [ ] IR → JavaScript
- [x] ES2022 output
- [ ] Module generation

#### Week 17: Source Maps & Types
- [ ] Source map generation
- [x] .d.ts output
- [ ] TypeScript interop

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
- [ ] IR → WASM
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

### Phase 7: Async & Threading (Weeks 23-25)

#### Week 23: Async Runtime
- [ ] Promise integration
- [ ] Async/await lowering
- [ ] Future implementation

#### Week 24: Multi-threading
- [ ] Web Workers support
- [ ] SharedArrayBuffer
- [ ] Thread spawning

#### Week 25: Concurrency Safety
- [ ] Channel implementation
- [ ] Mutex/RwLock
- [ ] Data race prevention

### Phase 8: Standard Library (Weeks 26-27)

#### Week 26: Core Types
- [ ] Vec<T>
- [ ] Option<T>
- [ ] Result<T, E>
- [ ] String, &str

#### Week 27: Collections & Async
- [ ] Map<K, V>
- [ ] Set<T>
- [ ] Iterator traits

### Phase 9: Tooling (Weeks 28-30)

#### Week 28: Incremental Compilation
- [ ] Parse caching
- [ ] Type cache
- [ ] Sub-second rebuilds

#### Week 29: CLI & Debugging
- [ ] Complete CLI
- [ ] Debug symbols
- [ ] Error improvements

#### Week 30: Polish
- [ ] Documentation
- [ ] Examples
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

| Metric | Target |
|--------|--------|
| Line Coverage | ≥90% |
| Branch Coverage | ≥85% |
| Fuzz Iterations | 1M+ |
| Property Tests | 10k+ |

---

## Success Criteria

### Must Have (MVP)

1. **JavaScript Compilation**
   - [x] Argon → ES2022 JavaScript
   - [x] Correct type erasure
   - [ ] ES modules output

2. **WASM Compilation**
   - [ ] Argon → WebAssembly
   - [ ] Linear memory management
   - [ ] No runtime memory errors

3. **JS Interop**
   - [ ] Import existing JS libraries
   - [ ] Type-safe boundary
   - [x] Shared<T> wrapping

4. **Core Language Features**
   - [x] Structs and classes
   - [x] Functions with ownership
   - [x] Basic borrow checking
   - [x] Type inference

### Should Have

- [x] Full borrow checker (basic)
- [x] Lifetime inference (basic)
- [ ] Async/await
- [ ] JSX support
- [ ] Standard library

### Nice to Have

- [ ] Multi-threading
- [ ] Incremental compilation
- [ ] LSP server

---

## Quick Start

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
argon compile input.ss --target js -o output.js

# Compile to WASM
argon compile input.ss --target wasm -o output.wasm

# Type check only
argon check input.ss

# Start REPL
argon repl
```

### Example: Hello World

```typescript
// hello.ss
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
argon compile hello.ss --target js -o hello.js
node hello.js
# Output: Hello, World!
```

### Example: JS Interop

```typescript
// app.ss
import { useState } from "react";

@export
function counter(): number {
    const [count, setCount] = useState(0);
    return count;
}
```

```typescript
// React component using Argon
import { counter } from "./app.ss";

function App() {
    return <div>Count: {counter()}</div>;
}
```

---

## Implementation Notes

### Key Design Decisions

1. **Parser**: Handwritten recursive descent (not LALR) for better error recovery
2. **Type System**: Bidirectional checking for better error messages
3. **Borrow Checker**: Based on Polonius (Rust's next-gen checker)
4. **IR**: SSA-based for optimization clarity
5. **WASM**: Cranelift backend for fast compilation and good performance

### Incremental Compilation

The compiler supports incremental compilation:
- Parsed ASTs are cached
- Type information is cached
- Only affected functions are recompiled
- Target: <100ms for small changes

### Debugging

The compiler includes debugging features:
- Source locations for all errors
- Type inference debug output
- Borrow checking debug output
- IR visualization

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
    │   └── (similar structure)
    ├── argon-interop
    │   └── (dependencies)
    ├── argon-stdlib
    └── (dependencies)
```

## Appendix B: File Extensions

| Extension | Description |
|-----------|-------------|
| `.ss` | Argon source file |
| `.ssd.ts` | Argon declaration file (TypeScript) |
| `.wat` | WebAssembly text format |

## Appendix C: CLI Commands

```
argon <command> [options]

Commands:
    compile    Compile source file(s)
    check      Type check without emitting code
    watch      Watch mode for incremental compilation
    repl       Start REPL
    format     Format source files
    init       Initialize new Argon project

Options:
    --target    Target: js, wasm (default: js)
    --output    Output file path
    --source-map    Generate source maps
    --declarations  Generate .d.ts files
    --optimize  Optimization level: 0-3
```
