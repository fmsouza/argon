# Argon Test Framework Design

## Overview

Add a Jasmine-style testing framework to the Argon compiler. Test files use the `.test.arg` extension and are executed via the runtime interpreter. The test types (`Runner`, `Assert`) live in a dedicated `test` stdlib module, following the existing module pattern (`math`, `io`, etc.).

## Decisions

| Dimension | Decision |
|-----------|----------|
| API style | Stdlib intrinsic types/functions (`case`, `Runner`, `Assert`) |
| Assertions | Rich (18 methods) |
| Suite nesting | Flat only (no nested `case()` blocks) |
| Execution | Runtime interpreter (no `node` dependency) |
| Output | Rich hierarchical (default), TAP, JSON via `--format` |
| Filtering | CLI `--filter` pattern + inline `runner.skip()` |

## Crate-Level Architecture

Changes are concentrated in 3 crates. The rest of the pipeline is untouched.

| Crate | Changes |
|-------|---------|
| `argon-stdlib` | New `test.arg` module defining `Runner`, `Assert` signatures with `@intrinsic` |
| `argon-runtime` | Native impls for `Runner`/`Assert` types, test execution engine, lifecycle hooks, result collector, output formatter |
| `argon-cli` | Rework `test` command: use runtime interpreter instead of `node`, add `--filter` flag, add `--format` flag (pretty/tap/json) |

No changes to: lexer, parser, AST, type checker, desugar, borrow checker, IR, or any codegen backend. Since `case()` is a regular function call and `Runner`/`Assert` are regular types, they flow through the existing pipeline without new syntax or tokens.

`.test.arg` files implicitly get access to the `test` module — the runtime auto-loads it before executing the file, so no explicit `import` is needed. This mirrors how `prelude.arg` is always available to all files.

## API Surface

The `test` module exposes three items:

```
// test.arg
struct Runner { ... }  @intrinsic
struct Assert { ... }  @intrinsic
fn case(name: string, fn: fn(Runner) -> void): void  @intrinsic
```

### `case(name, callback)`

Creates a suite, invokes the callback with a fresh `Runner`, and registers the suite for execution. Returns nothing.

### `Runner` — 6 methods

All take callbacks that receive an `Assert` instance:

| Method | Purpose |
|--------|---------|
| `when(name, fn)` | Register a test case |
| `skip(name, fn)` | Register a skipped test (reported separately, callback never executes) |
| `beforeEach(fn)` | Per-test setup |
| `afterEach(fn)` | Per-test teardown |
| `beforeAll(fn)` | Per-suite setup |
| `afterAll(fn)` | Per-suite teardown |

### `Assert` — 18 methods

Each throws on failure with a descriptive message:

| Category | Methods |
|----------|---------|
| Equality | `equals`, `notEquals`, `deepEquals` |
| Truthiness | `truthy`, `falsy` |
| Exceptions | `throws`, `notThrows` |
| Types | `isString`, `isNumber`, `isBoolean`, `isArray`, `isObject`, `isNull`, `isUndefined` |
| Comparisons | `greaterThan`, `lessThan`, `approximately` |
| Collections | `contains`, `hasKey` |

### Example

```argon
case("arithmetic", (runner) => {
  runner.beforeEach((assert) => {
    // setup
  });

  runner.when("adds two positive numbers", (assert) => {
    const result = 1 + 2;
    assert.equals(result, 3);
  });

  runner.when("divides by zero", (assert) => {
    assert.throws(() => {
      const x = 1 / 0;
    });
  });

  runner.skip("floating point comparison", (assert) => {
    assert.approximately(0.1 + 0.2, 0.3, 0.0001);
  });
});
```

## Execution Flow & Lifecycle

Two-phase execution in the runtime:

### Phase 1 — Registration

The runtime executes top-level statements normally. Each `case()` call runs its callback immediately with a fresh `Runner`. The runner records all `when`/`beforeEach`/etc. calls internally. When `case()` returns, the suite is fully defined. No assertions run yet.

### Phase 2 — Execution

After all top-level statements finish, the runtime runs each suite in definition order:

```
SUITE "arithmetic"
  beforeAll()

  TEST "adds two positive numbers"
    beforeEach()
    test callback(assert)
    afterEach()
    -> PASS (or FAIL on uncaught throw)

  TEST "divides by zero"
    beforeEach()
    test callback(assert)
    afterEach()
    -> PASS

  afterAll()
```

### Lifecycle Rules

- An assertion failure throws — the test engine catches it, records the failure, and continues to `afterEach`.
- A lifecycle hook failure (e.g., `beforeEach` throws) skips the test, records the error, but still runs `afterEach`.
- Suites run sequentially; all suites run regardless of failures in prior suites.
- `afterAll` always runs, even if `beforeAll` or any test failed.
- `runner.skip()` registers the test but marks it skipped. Its callback never executes. Its `beforeEach`/`afterEach` hooks also don't run.

## Output Formats

Three modes controlled by `--format`:

### `pretty` (default)

Colored hierarchical output with suite names, test names, timing, and diff on failure:

```
SUITE: arithmetic
  v adds two positive numbers (0.5ms)
  x subtracts numbers (0.3ms)
    expected: 3
    actual:   2
  - floating point comparison SKIPPED

Suites: 1  |  Tests: 3  |  Passed: 1  |  Failed: 1  |  Skipped: 1
Duration: 1.2ms
```

### `tap`

TAP v14 for CI tooling:

```
TAP version 14
1..3
ok 1 arithmetic > adds two positive numbers
not ok 2 arithmetic > subtracts numbers
  ---
  expected: 3
  actual:   2
  ...
ok 3 arithmetic > floating point comparison # SKIP
```

### `json`

Machine-readable JSON with full suite/test/assertion details, timing, and summary.

### `--filter <pattern>`

Matches against `"suite name > test name"` (case-insensitive substring). Unmatched tests are excluded, not skipped. Filtered runs: exit 0 if all matched tests pass, exit 1 if any fail.

## Error Handling & Edge Cases

| Scenario | Behavior |
|----------|----------|
| Parse/type error | Reported as suite-level failure with location. Continue to next file. |
| No `case()` calls | Warning: "0 suites found". Does not affect exit code. |
| Empty suite (no `when()` calls) | Warning. Does not affect exit code. |
| Duplicate suite/test names | Allowed but warned. |
| Assertion failure | Throws `TestAssertionError`. Caught by engine, `afterEach` runs, test recorded as failed. |
| Unexpected throw in test | Recorded as failure with thrown value as message. |
| `beforeAll` failure | All tests in suite skipped. |
| `beforeEach` failure | That test skipped. |
| `afterEach`/`afterAll` failure | Reported as warning, no cascade. |
| `assert.throws()` with non-throwing fn | `throws` itself throws: "expected function to throw but it did not". |

### Process Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | At least one failure |
| 2 | Compilation/parse error |

## Implementation Plan (TDD)

Build order, each layer depending on the one before:

| Step | Crate | What | Tests |
|------|-------|------|-------|
| 1 | `argon-stdlib` | Add `test.arg` with `@intrinsic` signatures | `.test.arg` fixtures |
| 2 | `argon-runtime` | `Runner` native type (registration, lifecycle callbacks) | Unit tests |
| 3 | `argon-runtime` | `Assert` native type (18 assertion methods) | Unit tests per method |
| 4 | `argon-runtime` | Test engine (registration -> execution, lifecycle ordering, error capture) | `.test.arg` integration fixtures |
| 5 | `argon-runtime` | Output formatters (pretty, TAP, JSON) | Snapshot tests |
| 6 | `argon-cli` | Rework `test` command (interpreter path, `--filter`, `--format`) | Integration tests |
| 7 | Docs | Update CLAUDE.md, add testing guide | Review |

### TDD Rhythm per Step

1. Write a `.test.arg` fixture describing expected behavior
2. Run it — it fails (missing types/behavior)
3. Implement native code in the runtime
4. Run fixture — it passes
5. Write Rust `#[test]` unit tests for edge cases (hook failure ordering, empty suites, etc.)

Fixtures live in `tests/fixtures/test-framework/` and exercise the framework from the outside-in.
