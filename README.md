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

For the locked scope in [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md), the compiler now has truthful default-path support for the currently supported targets:
- The preferred JS path (`argon compile --target js --pipeline ir`) handles the README declaration syntax examples, including enums, interfaces, type aliases, and interop declarations.
- `argon run` executes the host-independent core runtime examples and reports JSX/ESM/interop constructs as compile-only features instead of failing later with generic runtime errors.
- Native WASM compilation only succeeds for the validated native subset. Unsupported cases now fail clearly instead of emitting placeholder `.wasm` files.
- Memory-safety baseline is enforced (moves, borrows, NLL-style loop analysis, thread/process capture checks).
- Post-scope type-system hardening is implemented for interfaces/enums, structural object shapes, and recursive generic inference.
- Post-scope borrow analysis now computes global interprocedural borrow summaries across the call graph, including alias-aware borrowed returns, multi-source returned-borrow provenance, and transitive thread/process captures.

Known boundaries:
- `argon run` is intentionally scoped to host-independent execution. JSX, ESM imports, and JS interop declarations are compile-only features.
- Raw standalone `.wasm` covers the validated native subset directly. If native lowering is unsupported, `argon compile --target wasm` now fails instead of falling back to a compatibility placeholder binary.

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

# Compile to WebAssembly (native wasm + host-ABI sidecars)
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
   - global call-graph borrowed-return and escape summaries, including recursive SCC convergence
   - alias-aware borrowed-return tracking across local bindings and helper-returned references
   - Send/Sync-style thread/process capture checks
5. **Lowering to IR** (`argon-ir`)  
   Produces a control-flow-oriented IR with optional optimization passes.
6. **Code Generation**
   - `argon-codegen-js`: ES2022 JS output (+ experimental source maps and `.d.ts`)
   - `argon-codegen-wasm`: native `.wasm` + generated `.mjs` loader + `.host.mjs` host companion for the validated native subset

For `argon run`, the checked AST is executed by `argon-runtime` directly (no Node fallback as the primary path).

## Pipelines and Targets

The CLI supports two internal pipelines:
- `--pipeline ast` (direct AST codegen path)
- `--pipeline ir` (default, preferred; enables IR lowering/optimization flow)

Supported targets:
- `--target js`
- `--target wasm`

Target support matrix:
- `argon check`: full parser/type-checker/borrow-checker surface, including declaration-only syntax such as interfaces, type aliases, enums, and interop module declarations.
- `argon compile --target js --pipeline ir`: preferred JS path; supports the README declaration examples plus executable core language features.
- `argon run`: host-independent runtime subset only; rejects JSX, ESM imports, and interop declarations as compile-only features.
- `argon compile --target wasm --pipeline ir`: validated native subset only; unsupported native cases fail clearly.
- Generated WASM sidecars (`.mjs` + `.host.mjs`): JS-host convenience layer for supported native wasm builds.

WASM notes:
- `.wasm` is the binary output format.
- `argon compile --target wasm ... -o out.wasm` also writes `out.mjs` and `out.host.mjs`.
- Native standalone wasm covers the validated numeric/control-flow subset used by the current standalone tests: locals/ops, calls, branching, loops, array indexing, heap-backed object/field access for local shapes, internal async/await lowered synchronously, structured `try/catch/finally`, loop control, `for`/`for..of`, nested `switch`/`match`, and direct function imports supplied by the embedder.
- Unsupported native wasm cases fail at compile time; successful compilation now means the emitted `.wasm` is a real native backend artifact.
- The loader merges native wasm exports with the generated host companion for JS-host convenience on supported native builds.
- Linear-memory helpers still exist for native wasm strings, arrays, object literals, and struct-literal constructor lowering.

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
- Global interprocedural borrow-summary propagation for returned borrows, alias bindings, and thread/process captures.
- Data-race style checks for thread/process captures.
- Send/Sync-style typed capture constraints.

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
- README example compilation on the preferred JS IR path
- Runtime execution paths and compile-only runtime diagnostics
- WASM compile/execute paths, including non-placeholder native artifact checks, standalone async/import coverage, flat and structured standalone try/catch coverage, loop-control/for/switch/match-in-try standalone coverage, loader-sidecar host-ABI coverage, and native heap-backed object/member cases

## Roadmap Beyond Current Scope

- Additional host-side conveniences around JS-heavy module resolution and promise-backed interop without relying on generated host sidecars.

## License

MIT
