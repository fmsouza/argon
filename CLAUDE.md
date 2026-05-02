# Argon Compiler

Multi-target compiler (JavaScript, WebAssembly, native) written in Rust.

## Architecture

```
Source Code -> Lexer -> Parser -> AST -> Type Checker -> Borrow Checker -> IR -> Codegen
                                                                              |-> JS (ESM)
                                                                              |-> WASM
                                                                              |-> Native (Cranelift)
```

Two compilation pipelines:
- **AST Pipeline** (`Pipeline::Ast`): Direct AST-to-JS codegen
- **IR Pipeline** (`Pipeline::Ir`): AST -> IR -> optimizations -> codegen (all targets)

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `argon-cli` | CLI entry point (compile, check, run, test, format, watch, repl) |
| `argon-driver` | Compiler pipeline orchestration |
| `argon-lexer` | Tokenization |
| `argon-parser` | Recursive descent parser |
| `argon-ast` | AST node definitions |
| `argon-types` | Type checker + desugaring |
| `argon-borrowck` | Borrow checker (Rust-style ownership) |
| `argon-ir` | Intermediate representation + optimization passes |
| `argon-codegen-js` | JavaScript code generation |
| `argon-codegen-wasm` | WebAssembly code generation |
| `argon-codegen-native` | Native code generation (Cranelift) |
| `argon-diagnostics` | Error reporting |
| `argon-runtime` | AST interpreter for REPL |
| `argon-stdlib` | Embedded standard library (.arg files) |
| `argon-target` | Target triple abstraction |
| `argon-backend-traits` | Abstract I/O traits |
| `argon-interop` | JS interop layer |
| `argon-async` | Async runtime (work-stealing scheduler) |

## Build & Test

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests
cargo clippy --workspace         # Lint
cargo fmt --all -- --check       # Format check
```

## Conventions

- Error handling: use `Result<T, E>` everywhere. No `panic!()` in production code.
- Tests follow AAA pattern (Assign, Act, Assert).
- Parser has a MAX_RECURSION_DEPTH limit (50) to prevent stack overflow on malicious input.
- Driver enforces a 10MB source file size limit.
- All `#[allow(dead_code)]` annotations should have a `// TODO:` comment explaining planned use.

## Testing Framework

Test files use the `.test.arg` extension. The `test` stdlib module provides
`case()`, `Runner`, and `Assert` types (automatically available in test files).

```argon
case("suite name", function(runner: any): any {
  runner.beforeAll(function(assert: any): any { /* suite setup */ });
  runner.beforeEach(function(assert: any): any { /* test setup */ });
  runner.afterEach(function(assert: any): any { /* test teardown */ });
  runner.afterAll(function(assert: any): any { /* suite teardown */ });

  runner.when("test description", function(assert: any): any {
    assert.equals(actual, expected);
  });

  runner.skip("pending test", function(assert: any): any {
    // Not yet implemented
  });
});
```

### CLI

```bash
argon test                          # Run all .test.arg files in tests/
argon test --input path/to/test    # Run a single file
argon test --directory path/       # Run all .test.arg files in directory
argon test --filter "keyword"      # Run only matching tests
argon test --format tap            # CI-friendly TAP output
argon test --format json           # Machine-readable JSON
argon test --verbose               # Show detailed output
```

### Assert Methods

| Category | Methods |
|----------|---------|
| Equality | `equals`, `notEquals`, `deepEquals` |
| Truthiness | `truthy`, `falsy` |
| Exceptions | `throws`, `notThrows` |
| Types | `isString`, `isNumber`, `isBoolean`, `isArray`, `isObject`, `isNull`, `isUndefined` |
| Comparisons | `greaterThan`, `lessThan`, `approximately` |
| Collections | `contains`, `hasKey` |

All assert methods accept an optional `message` parameter for custom failure text.
