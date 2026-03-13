# Argon Compiler

Argon is a TypeScript-like language with Rust-inspired ownership and borrowing checks, implemented in Rust.

This repository contains the full compiler toolchain:
- Frontend (`lexer` + `parser`)
- Type checker
- Borrow checker
- IR builder + optimization passes
- JavaScript codegen
- WebAssembly codegen (core subset)
- AST runtime (`argon run`)
- CLI + test tooling

## Status (March 13, 2026)

For the locked scope in [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md), the compiler is **scope-complete**:
- README syntax parity (including interop decorators/declarations) is implemented.
- `argon run` executes core runtime features in the internal AST runtime.
- WASM compilation works for a documented core subset.
- Memory-safety baseline is enforced (moves, borrows, NLL-style loop analysis, thread/process capture checks).

Known boundaries:
- Full README parity on the WASM target is intentionally out of scope (core subset only).
- Interprocedural lifetime solving is not yet a full global solver.
- Some advanced type-inference/object-compatibility edge cases remain.

## Quick Start

```bash
# Build
cargo build

# Run all tests
cargo test --all

# Type + borrow check
argon check examples/interop.arg

# Run on internal AST runtime
argon run examples/collections.arg

# Compile to JavaScript
argon compile examples/control-flow.arg --target js -o out.js
node out.js

# Compile to WebAssembly (core subset)
argon compile examples/wasm-subset.arg --target wasm --pipeline ir -o out.wasm
```

## How the Compiler Works

Argon runs a fixed pipeline:

1. **Lexing** (`argon-lexer`)  
   Converts source text into tokens (including JSX/template/decorator tokens).
2. **Parsing** (`argon-parser`)  
   Builds AST nodes for declarations, expressions, control flow, interop syntax, and templates.
3. **Type Checking** (`argon-types`)  
   Resolves and validates types, generics baseline, unions, structs/classes/interfaces/enums.
4. **Borrow Checking** (`argon-borrowck`)  
   Enforces ownership and borrow rules, including:
   - use-after-move checks
   - borrow conflict checks
   - branch-aware borrow-state merges (`if`/`switch`/`match`)
   - loop fixed-point NLL-style analysis
   - cross-function borrowed return baseline checks
   - Send/Sync-style thread/process capture checks
5. **Lowering to IR** (`argon-ir`)  
   Produces a control-flow-oriented IR with optional optimization passes.
6. **Code Generation**
   - `argon-codegen-js`: ES2022 JS output (+ optional source maps and `.d.ts`)
   - `argon-codegen-wasm`: `.wasm` for the supported core subset

For `argon run`, the checked AST is executed by `argon-runtime` directly (no Node fallback as the primary path).

## Pipelines and Targets

The CLI supports two internal pipelines:
- `--pipeline ast` (direct AST codegen path)
- `--pipeline ir` (default, preferred; enables IR lowering/optimization flow)

Supported targets:
- `--target js`
- `--target wasm`

WASM notes:
- `.wasm` is the binary output format.
- Core subset includes numeric locals/ops, calls, branching, and loops.
- String/array subset memory support exists.
- JS-only/interop-heavy constructs produce explicit unsupported diagnostics.

## CLI Commands

```bash
argon compile <file.arg> [--target js|wasm] [--pipeline ast|ir] [-o output]
argon check <file.arg>
argon run <file.arg>
argon test [--input file.arg] [--directory path] [--pipeline ast|ir]
argon format <file.arg>
argon init <project-name>
argon watch <file.arg> [--check-only]
argon repl
```

## Language Feature Examples

The `examples/` directory is organized as executable/checkable feature coverage:

- Ownership/borrowing: `ownership.arg`, `borrowing.arg`
- Structs/classes/methods/generics: `structs.arg`, `classes.arg`, `simple_method.arg`, `generic_*.arg`
- Control flow: `control-flow.arg`, `match.arg`, `try-catch.arg`, `recursion.arg`
- Interop syntax: `interop.arg`, `test_lexer.arg`, `esm.arg`
- Type declarations: `interface.arg`, `enum.arg`, `type_test.arg`
- Runtime/basic language: `arithmetic.arg`, `boolean.arg`, `strings.arg`, `functions.arg`, `object.arg`
- WASM subset fixture: `wasm-subset.arg`

Check all examples:

```bash
for f in examples/*.arg; do argon check "$f"; done
```

## Memory Safety Model (Current Baseline)

Argon currently enforces:
- Move tracking and use-after-move rejection.
- Shared vs mutable borrow conflict rules.
- Borrow release based on use/liveness heuristics.
- Loop fixed-point borrow-state convergence.
- Borrowed return validation (including reborrow mutability constraints).
- Data-race style checks for thread/process captures.
- Send/Sync-style typed capture constraints.

Not yet a full production solver:
- No complete interprocedural lifetime graph solver across all call chains.

## Workspace Layout

```text
crates/
  argon-cli          # CLI entrypoint and commands
  argon-driver       # Pipeline orchestration
  argon-lexer        # Tokenization
  argon-parser       # AST parsing
  argon-ast          # AST definitions
  argon-types        # Type checker/type model
  argon-borrowck     # Ownership/borrow checking
  argon-ir           # IR + optimization passes
  argon-codegen-js   # JavaScript backend
  argon-codegen-wasm # WebAssembly backend (core subset)
  argon-runtime      # AST interpreter for `argon run`
  argon-interop      # Interop surface helpers
  argon-stdlib       # Runtime stdlib assets
  argon-diagnostics  # Error rendering infrastructure
```

## Testing and Quality Gates

Main validation commands:

```bash
cargo test --all
argon check examples/interop.arg
argon compile examples/control-flow.arg --pipeline ir -o /tmp/out.js
argon run examples/collections.arg
argon compile examples/wasm-subset.arg --target wasm --pipeline ir -o /tmp/out.wasm
```

CI includes completion-focused coverage for:
- README parity checks
- Runtime execution paths
- WASM subset compile/execute paths

## Roadmap Beyond Current Scope

- Full-language WASM parity (currently core subset by design).
- Deeper global/interprocedural lifetime analysis.
- Broader type-system edge-case coverage for advanced generics/object compatibility.

## License

MIT
