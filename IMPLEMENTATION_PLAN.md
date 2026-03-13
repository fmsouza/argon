# Argon Completion Rebaseline

**Date:** March 13, 2026  
**Status:** Scope-Complete + Post-Scope Type Hardening/Global Borrow Solver/WASM Host-ABI Parity Implemented  
**Scope:** README parity + WASM core subset + runtime execution + memory-safety baseline

---

## Completion Definition (Locked)

The compiler is considered **complete** for this scope only when all gates pass:

1. **README parity gate (language + interop subset)**
   - README annotation syntax (`@export`, `@js-interop`, `declare module`) parses and type-checks.
   - README-style loop/for-of/struct-method/object-literal snippets compile through `--pipeline ir`.

2. **Runtime gate (`argon run`)**
   - Internal AST runtime executes runnable README-style snippets (no Node fallback as primary path).
   - Unsupported runtime features must produce structured runtime errors (no silent fallback).

3. **WASM gate (documented core subset)**
   - `.wasm` is the required binary format.
   - IR-driven WASM lowering supports constants, locals, arithmetic/comparison, calls, branching, loops.
   - Minimal linear-memory model exists for subset strings and arrays.
   - Unsupported JS-only/interop paths produce explicit diagnostics.

4. **Memory-safety gate**
   - Move tracking and use-after-move must be enforced.
   - Borrow-conflict and data-race checks must be invoked in normal check flow.
   - NLL, cross-function/reborrow, and thread-safety trait-like checks must be implemented to reach full completion.

5. **Testing gate**
   - `cargo test --all` stays green.
   - README parity tests + WASM subset execution tests + runtime tests are in CI.

---

## Audited Status Matrix (March 13, 2026)

| Area | Status | Notes |
|---|---|---|
| Lexer core + decorator tokenization | **Implemented** | `@` tokenized, `loop` keyword tokenized, template interpolation panic fixed. |
| Parser frontend parity (decorators/declare/loop/for-of/object literals/struct methods/templates) | **Implemented (core)** | Core syntax paths now parse, including template interpolation expressions. |
| Type checker for README-required constructs | **Implemented (post-scope hardening)** | Added interface/enum registration, structural object-shape typing, generic interface/generic alias resolution, enum member typing, and recursive generic inference across nested parameter shapes. |
| Borrow checker move/use-after-move | **Implemented (baseline)** | Move state now transitions to `Moved`; use-after-move and move-while-borrowed regressions covered; shared-reference bindings (`&T`) now treated as copyable in move analysis. |
| Borrow checker NLL / cross-function / reborrow / Send+Sync-style checks | **Implemented (global solver)** | Added call-site borrow contracts, alias-aware borrowed-return validation, persistent helper-return borrow bindings, call-graph SCC summary convergence, multi-source returned-borrow provenance, statement-level end-of-use release, branch-aware merges, loop fixed-point NLL-style analysis, and Send/Sync-style thread/process capture validation (`Send` for owned capture, `Sync` for shared-reference pointees). |
| Data-race check invocation | **Implemented (baseline)** | Wired into normal check path; model still basic. |
| IR lowering for `loop` and `for..of` | **Implemented (core)** | Lowered into CFG blocks with index-based `for..of` expansion. |
| JS codegen + source maps | **Implemented** | IR path and source map generation are working and tested in CLI integration tests. |
| Runtime execution (`argon run`) for core control flow + structs/classes + `new` + match + for-of + loop | **Implemented (core)** | Added behavior and tests; template interpolation supported at runtime AST level. |
| WASM backend | **Implemented (host-ABI parity + complete native standalone subset)** | Replaced hardcoded `42` with IR-driven lowering + memory model for strings/arrays/object literals, heap-backed member access, struct-literal constructor lowering, direct wasm imports for embedder-supplied functions, native synchronous lowering for internal async/await, native flat and structured `try/catch/finally` lowering for nested `if`/`return`, loop-control flow, `for`/`for..of`, and nested `switch`/`match`, generated `.mjs` loader + `.host.mjs` companion sidecars, and host fallbacks for JS-heavy async/interop paths. |
| WASM validation/execution tests | **Implemented (host-ABI parity + complete native standalone subset)** | Structural validation + Node execution tests cover native numeric control flow, string/array memory, object/member access, struct-literal lowering, raw standalone wasm execution for async/await, flat and structured try/catch, loop control, `for`/`for..of`, and nested `switch`/`match` inside exception regions, and direct imports, plus CLI host-ABI execution for loader, async/await, try/catch, and module imports. |
| Example suite parity with README/plan-required language features | **Implemented** | Rewrote and expanded `examples/*.arg` to include decorators/declare-module interop, loop/for-of, struct methods, object literals, template interpolation, async/await, try/catch/finally, interfaces/enums, generics, and a wasm-subset fixture. |
| `argon-interop` crate tests | **Implemented** | Added crate-local tests for import/export/declaration behavior. |
| `argon-stdlib` crate tests | **Implemented** | Added crate-local tests for primitives/runtime/definitions. |
| Completion CI job (README parity + runtime + WASM subset) | **Implemented** | Added dedicated `completion` workflow job. |

---

## Active Phase: Completion Hardening

### Milestone 1: Frontend Syntax Parity
- [x] Decorator parsing for `@export` and `@js-interop`.
- [x] `declare module "..." { ... }` parsing.
- [x] Struct methods in parser + type model.
- [x] Object literals in expression position.
- [x] `loop {}` and `for (const x of y)` parser support.
- [x] Source parser support for template literals with interpolation.
- [x] Panic-to-diagnostic hardening for new syntax paths.

### Milestone 2: Type + Borrow Safety Baseline
- [x] Struct/generic method type resolution baseline.
- [x] Generic struct alias assignment baseline for object literals.
- [x] Union assignability baseline for struct/class variant returns (`A | B`).
- [x] Move tracking transitions (`Owned` -> `Moved`) and use-after-move enforcement.
- [x] Shared-reference (`&T`) copy semantics in borrow checker move analysis.
- [x] Data-race check integrated into default borrow check flow.
- [x] NLL-like end-of-use borrow release baseline (statement/use-count driven, function + top-level scopes).
- [x] Branch-aware `if/else` borrow-state merge to prevent path-leak false positives.
- [x] Branch-aware `switch`/`match` borrow-state merge with conservative no-match handling.
- [x] CFG-based NLL end-of-use borrow analysis (loop fixed-point baseline).
- [x] Cross-function borrow/reborrow validation baseline (call-site param + borrowed return constraints).
- [x] Thread/process typed argument safety checks (Send-like baseline).
- [x] Send/Sync-like trait constraints and thread/process capture enforcement baseline.

### Milestone 3: Runtime Completion (`argon run`)
- [x] Struct/class/new/member semantics.
- [x] `match`, `loop`, and `for..of`.
- [x] Runtime tests for struct methods, loop controls, match, for-of.
- [x] Structured runtime errors for unsupported runtime constructs.
- [x] Template interpolation execution support at AST runtime level.
- [x] Parser support for template literal source syntax.

### Milestone 4: WASM Backend (Core Subset)
- [x] IR-driven lowering for constants/locals/ops/calls/branching/loops.
- [x] Minimal linear memory model for strings and arrays.
- [x] Heap-backed object literals, field access, and array indexing for local wasm shapes.
- [x] Struct-literal constructor lowering (`Point { ... }` -> `new Point({ ... })`) in the wasm subset.
- [x] Generated `.mjs` loader sidecar next to `.wasm` outputs.
- [x] Generated `.host.mjs` host companion with merged-loader exports.
- [x] Host-ABI fallback path for async/await, try/throw, and import/interop-heavy modules when native wasm lowering is unavailable.
- [x] Native standalone lowering for internal async/await, flat try/catch/throw, and direct function imports supplied at instantiation time.
- [x] Native standalone lowering for structured `try/catch/finally` control flow with nested `if`/`return`.
- [x] Native standalone lowering for loop control inside exception regions (`while`/`do while`/`loop` with `break`/`continue`).
- [x] Native standalone lowering for `for`/`for..of` inside exception regions.
- [x] Native standalone lowering for nested `switch`/`match` inside exception regions.
- [x] Explicit unsupported diagnostics for non-subset operations.
- [x] Structural validation + execution tests for subset fixtures.
- [x] Full README parity on WASM target via host ABI sidecars.

### Milestone 5: Example Suite Parity + Coverage
- [x] Rewrite stale examples to match current Argon syntax and implementation-plan gates.
- [x] Add missing examples for interop annotations (`@js-interop`, `declare module`, `@export`).
- [x] Add dedicated examples for `try/catch/finally`, interface/enum declarations, and WASM core subset fixture.
- [x] Ensure full `examples/*.arg` suite passes `argon check`.
- [x] Spot-validate gate paths with examples (`argon run`, `compile --pipeline ir`, `compile --target wasm --pipeline ir`).

---

## Remaining Gaps (Post-Scope Completion)

1. **Host integration boundary**: the standalone `.wasm` artifact now covers the full native compiler subset, including structured `try/catch/finally` with nested `if`/`return`, loop control, `for`/`for..of`, and nested `switch`/`match`. The generated sidecars remain for JS-host conveniences like module resolution and promise-backed interop that plain `.wasm` imports cannot automate by themselves.

---

## Post-Scope Hardening

### Milestone 6: Type-System Hardening
- [x] Register `interface` and `enum` declarations in the type environment.
- [x] Add structural object-shape typing for object literals and object-type annotations.
- [x] Support interface/class/object-shape member resolution.
- [x] Support generic interface references and generic type-alias instantiation.
- [x] Replace positional generic inference with recursive shape-aware inference for nested parameter types.
- [x] Add crate-local regressions for interfaces, enums, structural object literals, and generic inference.

---

## Test Baseline (Current)

- `cargo test --all` must remain green.
- Crate-local tests now cover:
  - `argon-codegen-wasm`
  - `argon-runtime`
  - `argon-interop`
  - `argon-stdlib`
- CLI integration now covers:
  - README-style interop annotation `check`
  - IR compile + execute of `for..of` snippet
  - `argon run` runtime execution snippet
  - WASM subset compile + execute
  - Raw standalone WASM async/await execution
  - Raw standalone WASM flat try/catch execution
  - Raw standalone WASM structured try/catch execution
  - Raw standalone WASM loop-control-in-try execution
  - Raw standalone WASM for-of-in-try execution
  - Raw standalone WASM switch/match-in-try execution
  - Raw standalone WASM direct-import execution
  - WASM loader-sidecar generation/import
  - WASM host-sidecar generation/import
  - WASM async/await host-ABI execution
  - WASM try/catch host-ABI execution
  - WASM import/interoperability host-ABI execution
  - WASM struct-literal/object member execution
- Example validation now includes:
  - Full `examples/*.arg` suite passing `argon check`
  - Coverage examples for: interop annotations (`interop.arg`, `test_lexer.arg`), loop/for-of (`control-flow.arg`, `collections.arg`), template interpolation (`strings.arg`), struct methods (`simple_method.arg`), object literals (`object.arg`), try/catch/finally (`try-catch.arg`), interface/enum (`interface.arg`, `enum.arg`), and wasm subset (`wasm-subset.arg`)
- Borrow-check regressions include:
  - borrowed-return escape checks
  - helper-mediated borrowed-return propagation
  - alias-aware borrowed-return propagation through local bindings
  - multi-source returned-borrow summary propagation
  - mutually recursive borrowed-return summary convergence
  - thread/process typed capture safety
  - helper-mediated thread/process capture propagation
  - helper-mediated process capture propagation
  - NLL-like last-use borrow-release scenarios
  - loop fixed-point NLL scenarios (zero-iteration path + iterative convergence)
  - `if/else` exclusive-path borrow merge scenarios
  - `switch`/`match` branch-merge borrow liveness scenarios
  - shared-reference binding reuse without move
- Type-check regressions now include:
  - structural object-literal assignment to interfaces
  - generic interface member resolution
  - generic type-alias instantiation with explicit type arguments
  - enum member typing
  - recursive generic inference for nested parameter shapes
