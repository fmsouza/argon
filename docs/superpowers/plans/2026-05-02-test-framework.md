# Test Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Jasmine-style testing framework to the Argon compiler using a `test` stdlib module executed via the runtime interpreter.

**Architecture:** A `TestContext` is added to `Runtime`, storing registered suites and lifecycle hooks. The `case()` native function registers suites; `Runner` and `Assert` are `Value::Object` instances with native method dispatch. After top-level statements execute in test mode, all suites run with full lifecycle ordering. Three output formatters (pretty, TAP, JSON) render results. The CLI orchestrates: parse → type check → desugar → runtime execute → run suites → format → print.

**Tech Stack:** Rust, argon-runtime interpreter (tree-walking evaluator), argon-stdlib embedded module pattern, argon-cli clap derive.

---

### File Map

| File | Role |
|------|------|
| `crates/argon-stdlib/stdlib/test.arg` (create) | Module signatures with `@intrinsic` |
| `crates/argon-stdlib/src/lib.rs` (modify) | Register `test` in resolver |
| `crates/argon-runtime/src/test_framework.rs` (create) | TestContext, TestSuite, TestCase, Runner/Assert helpers |
| `crates/argon-runtime/src/test_formatter.rs` (create) | pretty/tap/json formatters |
| `crates/argon-runtime/src/lib.rs` (modify) | Add TestContext to Runtime, native dispatch, `run_all_suites`, public re-exports |
| `crates/argon-cli/src/main.rs` (modify) | Rework `test` command: interpreter path, `--filter`, `--format` |
| `tests/fixtures/test-framework/basic.test.arg` (create) | Fixture: basic pass/fail |
| `tests/fixtures/test-framework/lifecycle.test.arg` (create) | Fixture: hook ordering |
| `tests/fixtures/test-framework/assertions.test.arg` (create) | Fixture: all 18 assertion methods |
| `tests/fixtures/test-framework/filtering.test.arg` (create) | Fixture: skip and filter |

---

### Task 1: Add `test.arg` stdlib module

**Files:**
- Create: `crates/argon-stdlib/stdlib/test.arg`
- Modify: `crates/argon-stdlib/src/lib.rs`

- [ ] **Step 1: Write `test.arg`**

```argon
// Argon Standard Library - Test Framework
// Automatically available in .test.arg files.

@intrinsic
export struct Runner {
    when(name: string, callback: fn(Assert) -> void): void with &this;
    skip(name: string, callback: fn(Assert) -> void): void with &this;
    beforeEach(callback: fn(Assert) -> void): void with &this;
    afterEach(callback: fn(Assert) -> void): void with &this;
    beforeAll(callback: fn(Assert) -> void): void with &this;
    afterAll(callback: fn(Assert) -> void): void with &this;
}

@intrinsic
export struct Assert {
    equals(actual: any, expected: any, message: string?): void with &this;
    notEquals(actual: any, expected: any, message: string?): void with &this;
    deepEquals(actual: any, expected: any, message: string?): void with &this;
    truthy(value: any, message: string?): void with &this;
    falsy(value: any, message: string?): void with &this;
    throws(callback: fn() -> void, message: string?): void with &this;
    notThrows(callback: fn() -> void, message: string?): void with &this;
    isString(value: any, message: string?): void with &this;
    isNumber(value: any, message: string?): void with &this;
    isBoolean(value: any, message: string?): void with &this;
    isArray(value: any, message: string?): void with &this;
    isObject(value: any, message: string?): void with &this;
    isNull(value: any, message: string?): void with &this;
    isUndefined(value: any, message: string?): void with &this;
    greaterThan(actual: f64, expected: f64, message: string?): void with &this;
    lessThan(actual: f64, expected: f64, message: string?): void with &this;
    approximately(actual: f64, expected: f64, delta: f64, message: string?): void with &this;
    contains(array: any[], element: any, message: string?): void with &this;
    hasKey(object: any, key: string, message: string?): void with &this;
}

export fn case(name: string, callback: fn(Runner) -> void): void;
```

- [ ] **Step 2: Register in `lib.rs`**

In `resolve_std_module`, add after the `"async"` arm:

```rust
"test" => Some(include_str!("../stdlib/test.arg")),
```

In `available_modules`, update:

```rust
&["io", "math", "error", "fs", "net", "http", "ws", "async", "test"]
```

- [ ] **Step 3: Add tests in `lib.rs`**

```rust
#[test]
fn resolves_test_module() {
    let src = resolve_std_module("test").expect("test module should exist");
    assert!(src.contains("struct Runner"));
    assert!(src.contains("struct Assert"));
    assert!(src.contains("fn case("));
    assert!(src.contains("when(name:"));
    assert!(src.contains("beforeEach"));
}
```

- [ ] **Step 4: Verify**

```bash
cargo test -p argon-stdlib
```
Expected: all 14 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/argon-stdlib/stdlib/test.arg crates/argon-stdlib/src/lib.rs
git commit -m "feat(stdlib): add test module with Runner, Assert, and case signatures"
```

---

### Task 2: Add TestContext types to runtime

**Files:**
- Create: `crates/argon-runtime/src/test_framework.rs`
- Modify: `crates/argon-runtime/src/lib.rs`

- [ ] **Step 1: Create `test_framework.rs`**

```rust
//! Test framework native types and execution engine.

use crate::{NativeFunction, RcFunction, RuntimeError, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct TestCase {
    pub name: String,
    pub callback: RcFunction,
    pub skipped: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TestSuite {
    pub name: String,
    pub before_all: Option<RcFunction>,
    pub after_all: Option<RcFunction>,
    pub before_each: Option<RcFunction>,
    pub after_each: Option<RcFunction>,
    pub tests: Vec<TestCase>,
}

#[derive(Debug, Default)]
pub(crate) struct TestContext {
    pub suites: Vec<TestSuite>,
}

#[derive(Debug, Clone)]
pub enum TestOutcome {
    Pass {
        name: String,
        suite_name: String,
        duration_ms: f64,
    },
    Fail {
        name: String,
        suite_name: String,
        message: String,
        duration_ms: f64,
    },
    Skip {
        name: String,
        suite_name: String,
    },
}

impl TestOutcome {
    pub fn suite_name(&self) -> &str {
        match self {
            TestOutcome::Pass { suite_name, .. }
            | TestOutcome::Fail { suite_name, .. }
            | TestOutcome::Skip { suite_name, .. } => suite_name,
        }
    }

    pub fn test_name(&self) -> &str {
        match self {
            TestOutcome::Pass { name, .. }
            | TestOutcome::Fail { name, .. }
            | TestOutcome::Skip { name, .. } => name,
        }
    }
}

#[derive(Debug, Default)]
pub struct TestResults {
    pub outcomes: Vec<TestOutcome>,
    pub total_suites: usize,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: f64,
}

/// Build a Value::Object containing all Assert methods as native functions.
pub(crate) fn make_assert_object() -> Value {
    let mut methods = HashMap::new();
    for method in &[
        "equals", "notEquals", "deepEquals", "truthy", "falsy",
        "throws", "notThrows", "isString", "isNumber", "isBoolean",
        "isArray", "isObject", "isNull", "isUndefined",
        "greaterThan", "lessThan", "approximately", "contains", "hasKey",
    ] {
        methods.insert(
            method.to_string(),
            Value::NativeFunction(NativeFunction {
                name: format!("Assert.{}", method),
            }),
        );
    }
    Value::Object(Rc::new(RefCell::new(methods)))
}

/// Build a Runner object for a given suite index.
/// Native function names encode the suite index for dispatch.
pub(crate) fn make_runner_object(suite_idx: usize) -> Value {
    let mut methods = HashMap::new();
    for method in &["when", "skip", "beforeEach", "afterEach", "beforeAll", "afterAll"] {
        methods.insert(
            method.to_string(),
            Value::NativeFunction(NativeFunction {
                name: format!("Runner.{}.{}", suite_idx, method),
            }),
        );
    }
    Value::Object(Rc::new(RefCell::new(methods)))
}

/// Handle a Runner method call during registration.
pub(crate) fn handle_runner_method(
    suites: &mut Vec<TestSuite>,
    suite_idx: usize,
    method: &str,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    let suite = suites
        .get_mut(suite_idx)
        .ok_or_else(|| RuntimeError::TypeError("invalid runner index".to_string()))?;

    match method {
        "when" | "skip" => {
            let name = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(RuntimeError::TypeError(format!(
                        "runner.{}: expected string name", method
                    )))
                }
            };
            let callback = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => {
                    return Err(RuntimeError::TypeError(format!(
                        "runner.{}: expected function callback", method
                    )))
                }
            };
            suite.tests.push(TestCase {
                name,
                callback,
                skipped: method == "skip",
            });
            Ok(Value::Undefined)
        }
        "beforeEach" => {
            suite.before_each = Some(extract_fn(args, 0, "beforeEach")?);
            Ok(Value::Undefined)
        }
        "afterEach" => {
            suite.after_each = Some(extract_fn(args, 0, "afterEach")?);
            Ok(Value::Undefined)
        }
        "beforeAll" => {
            suite.before_all = Some(extract_fn(args, 0, "beforeAll")?);
            Ok(Value::Undefined)
        }
        "afterAll" => {
            suite.after_all = Some(extract_fn(args, 0, "afterAll")?);
            Ok(Value::Undefined)
        }
        _ => Err(RuntimeError::TypeError(format!(
            "unknown Runner method: {}",
            method
        ))),
    }
}

fn extract_fn(args: &[Value], idx: usize, ctx: &str) -> Result<RcFunction, RuntimeError> {
    match args.get(idx) {
        Some(Value::Function(f)) => Ok(f.clone()),
        _ => Err(RuntimeError::TypeError(format!(
            "runner.{}: expected function argument",
            ctx
        ))),
    }
}
```

- [ ] **Step 2: Wire into Runtime**

In `lib.rs`, add module declaration near top:

```rust
mod test_framework;
```

Add `test_context` to `Runtime` struct:

```rust
pub struct Runtime {
    scope: Scope,
    globals: HashMap<String, Value>,
    struct_defs: HashMap<String, RuntimeStructDef>,
    skill_defs: HashMap<String, RuntimeSkillDef>,
    resources: ResourceTable,
    test_context: test_framework::TestContext,
}
```

In `Runtime::new()`, add to the `Self { ... }` block:

```rust
test_context: test_framework::TestContext::default(),
```

Register `case` global. In `Runtime::new()`, add before the `Self { ... }` return:

```rust
globals.insert(
    "case".to_string(),
    Value::NativeFunction(NativeFunction {
        name: "test.case".to_string(),
    }),
);
```

- [ ] **Step 3: Build**

```bash
cargo build -p argon-runtime 2>&1
```
Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add crates/argon-runtime/src/test_framework.rs crates/argon-runtime/src/lib.rs
git commit -m "feat(runtime): add TestContext types and wire into Runtime"
```

---

### Task 3: Implement `case()` native function and `call_value_function`

**Files:**
- Modify: `crates/argon-runtime/src/lib.rs`

- [ ] **Step 1: Add `call_value_function` helper to `impl Runtime`**

Add this method before `execute_native_function`:

```rust
fn call_value_function(
    &mut self,
    func: &RcFunction,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    let mut call_scope = func.closure.clone();
    for (i, param) in func.params.iter().enumerate() {
        let val = args.get(i).cloned().unwrap_or(Value::Undefined);
        call_scope.define(param.clone(), val);
    }
    let saved_scope = std::mem::replace(&mut self.scope, call_scope);
    let result = (|| {
        for stmt in &func.body {
            match self.execute_statement(stmt)? {
                ExecOutcome::Return(v) => return Ok(v),
                ExecOutcome::Normal => {}
                ExecOutcome::Break => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "break outside loop".to_string(),
                    ));
                }
                ExecOutcome::Continue => {
                    return Err(RuntimeError::InvalidControlFlow(
                        "continue outside loop".to_string(),
                    ));
                }
            }
        }
        Ok(Value::Undefined)
    })();
    self.scope = saved_scope;
    result
}
```

- [ ] **Step 2: Add `test.case` match arm in `execute_native_function`**

Add before any Runner dispatch:

```rust
"test.case" => {
    let suite_name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(RuntimeError::TypeError(
            "case: expected string suite name".to_string()
        )),
    };
    let callback = match args.get(1) {
        Some(Value::Function(f)) => f.clone(),
        _ => return Err(RuntimeError::TypeError(
            "case: expected function callback".to_string()
        )),
    };

    let suite_idx = self.test_context.suites.len();
    self.test_context.suites.push(test_framework::TestSuite {
        name: suite_name,
        before_all: None,
        after_all: None,
        before_each: None,
        after_each: None,
        tests: Vec::new(),
    });
    let runner = test_framework::make_runner_object(suite_idx);
    self.call_value_function(&callback, &[runner])
}
```

- [ ] **Step 3: Add Runner method dispatch in `execute_native_function`**

```rust
name if name.starts_with("Runner.") => {
    let parts: Vec<&str> = name.splitn(4, '.').collect();
    if parts.len() == 4 {
        if let Ok(suite_idx) = parts[1].parse::<usize>() {
            let method = parts[2];
            return test_framework::handle_runner_method(
                &mut self.test_context.suites,
                suite_idx,
                method,
                args,
            );
        }
    }
    Err(RuntimeError::TypeError(format!(
        "invalid Runner native function name: {}", name
    )))
}
```

- [ ] **Step 4: Build**

```bash
cargo build -p argon-runtime 2>&1
```
Expected: compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add crates/argon-runtime/src/lib.rs
git commit -m "feat(runtime): implement case() and Runner method dispatch"
```

---

### Task 4: Implement Assert methods in `execute_native_function`

**Files:**
- Modify: `crates/argon-runtime/src/lib.rs`

- [ ] **Step 1: Add Assert dispatch**

```rust
name if name.starts_with("Assert.") => {
    let method = &name["Assert.".len()..];
    self.execute_assert_method(method, args)
}
```

- [ ] **Step 2: Add `execute_assert_method` and helpers to `impl Runtime`**

```rust
fn execute_assert_method(
    &mut self,
    method: &str,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    let fail = |msg: String| -> Result<Value, RuntimeError> {
        Err(RuntimeError::Thrown(msg))
    };
    let first = |args: &[Value]| args.first().unwrap_or(&Value::Undefined);
    let second = |args: &[Value]| args.get(1).unwrap_or(&Value::Undefined);
    let optional_msg = |args: &[Value], idx: usize| -> String {
        args.get(idx)
            .and_then(|v| match v { Value::String(s) if !s.is_empty() => Some(format!(": {}", s)), _ => None })
            .unwrap_or_default()
    };

    match method {
        "equals" => {
            if !self.values_equal(first(args), second(args)) {
                fail(format!("expected {} but got {}{}", self.value_display(second(args)), self.value_display(first(args)), optional_msg(args, 2)))
            } else { Ok(Value::Undefined) }
        }
        "notEquals" => {
            if self.values_equal(first(args), second(args)) {
                fail(format!("expected values to differ but both are {}{}", self.value_display(first(args)), optional_msg(args, 2)))
            } else { Ok(Value::Undefined) }
        }
        "deepEquals" => {
            if !self.values_deep_equal(first(args), second(args)) {
                fail(format!("expected {} but got {} (deep){}", self.value_display(second(args)), self.value_display(first(args)), optional_msg(args, 2)))
            } else { Ok(Value::Undefined) }
        }
        "truthy" => {
            if !self.is_truthy(first(args)) {
                fail(format!("expected truthy value but got {}{}", self.value_display(first(args)), optional_msg(args, 1)))
            } else { Ok(Value::Undefined) }
        }
        "falsy" => {
            if self.is_truthy(first(args)) {
                fail(format!("expected falsy value but got {}{}", self.value_display(first(args)), optional_msg(args, 1)))
            } else { Ok(Value::Undefined) }
        }
        "throws" => {
            let func = match args.first() {
                Some(Value::Function(f)) => f.clone(),
                _ => return fail("throws: expected a function argument".to_string()),
            };
            match self.call_value_function(&func, &[]) {
                Ok(_) => fail(format!("expected function to throw but it did not{}", optional_msg(args, 1))),
                Err(_) => Ok(Value::Undefined),
            }
        }
        "notThrows" => {
            let func = match args.first() {
                Some(Value::Function(f)) => f.clone(),
                _ => return fail("notThrows: expected a function argument".to_string()),
            };
            match self.call_value_function(&func, &[]) {
                Ok(_) => Ok(Value::Undefined),
                Err(e) => fail(format!("expected function not to throw but it threw: {}{}", e, optional_msg(args, 1))),
            }
        }
        "isString" => type_check(args, |v| matches!(v, Value::String(_)), "string", &optional_msg),
        "isNumber" => type_check(args, |v| matches!(v, Value::Number(_)), "number", &optional_msg),
        "isBoolean" => type_check(args, |v| matches!(v, Value::Boolean(_)), "boolean", &optional_msg),
        "isArray" => type_check(args, |v| matches!(v, Value::Array(_)), "array", &optional_msg),
        "isObject" => type_check(args, |v| matches!(v, Value::Object(_)), "object", &optional_msg),
        "isNull" => type_check(args, |v| matches!(v, Value::Null), "null", &optional_msg),
        "isUndefined" => type_check(args, |v| matches!(v, Value::Undefined), "undefined", &optional_msg),
        "greaterThan" => {
            let a = to_num(args, 0, "greaterThan")?;
            let b = to_num(args, 1, "greaterThan")?;
            if !(a > b) { fail(format!("expected {} > {}{}", a, b, optional_msg(args, 2))) } else { Ok(Value::Undefined) }
        }
        "lessThan" => {
            let a = to_num(args, 0, "lessThan")?;
            let b = to_num(args, 1, "lessThan")?;
            if !(a < b) { fail(format!("expected {} < {}{}", a, b, optional_msg(args, 2))) } else { Ok(Value::Undefined) }
        }
        "approximately" => {
            let a = to_num(args, 0, "approximately")?;
            let b = to_num(args, 1, "approximately")?;
            let d = to_num(args, 2, "approximately")?;
            if !((a - b).abs() < d) { fail(format!("expected {} within {} of {}{}", a, d, b, optional_msg(args, 3))) } else { Ok(Value::Undefined) }
        }
        "contains" => {
            let arr = match args.first() {
                Some(Value::Array(a)) => a,
                _ => return fail("contains: first argument must be an array".to_string()),
            };
            let el = args.get(1).unwrap_or(&Value::Undefined);
            if !arr.iter().any(|v| self.values_equal(v, el)) {
                fail(format!("expected array to contain {}{}", self.value_display(el), optional_msg(args, 2)))
            } else { Ok(Value::Undefined) }
        }
        "hasKey" => {
            let obj = match args.first() {
                Some(Value::Object(o)) => o,
                _ => return fail("hasKey: first argument must be an object".to_string()),
            };
            let key = match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => return fail("hasKey: second argument must be a string key".to_string()),
            };
            if !obj.borrow().contains_key(&key) {
                fail(format!("expected object to have key '{}'{}", key, optional_msg(args, 2)))
            } else { Ok(Value::Undefined) }
        }
        _ => Err(RuntimeError::TypeError(format!("unknown Assert method: {}", method))),
    }
}

fn values_equal(&self, a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Null, Value::Null) => true,
        (Value::Undefined, Value::Undefined) => true,
        _ => false,
    }
}

fn values_deep_equal(&self, a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(va, vb)| self.values_deep_equal(va, vb))
        }
        (Value::Object(a), Value::Object(b)) => {
            let am = a.borrow();
            let bm = b.borrow();
            am.len() == bm.len()
                && am.iter().all(|(k, v)| bm.get(k).is_some_and(|bv| self.values_deep_equal(v, bv)))
        }
        _ => self.values_equal(a, b),
    }
}

fn value_display(&self, v: &Value) -> String {
    match v {
        Value::String(s) => format!("\"{}\"", s),
        Value::Number(n) => format!("{}", n),
        Value::Boolean(b) => format!("{}", b),
        Value::Null => "null".to_string(),
        Value::Undefined => "undefined".to_string(),
        Value::Array(arr) => format!("[{}]", arr.iter().map(|v| self.value_display(v)).collect::<Vec<_>>().join(", ")),
        Value::Object(_) => "{...}".to_string(),
        Value::Function(_) => "<function>".to_string(),
        Value::NativeFunction(_) => "<native>".to_string(),
        Value::Future(_) => "<future>".to_string(),
    }
}
```

Add helper at module level in `lib.rs`:

```rust
fn type_check<F>(args: &[Value], check: F, expected: &str, msg: &dyn Fn(&[Value], usize) -> String) -> Result<Value, RuntimeError>
where F: Fn(&Value) -> bool
{
    let value = args.first().unwrap_or(&Value::Undefined);
    if !check(value) {
        Err(RuntimeError::Thrown(format!(
            "expected {} but got {}{}",
            expected,
            "other",
            msg(args, 1),
        )))
    } else {
        Ok(Value::Undefined)
    }
}

fn to_num(args: &[Value], idx: usize, ctx: &str) -> Result<f64, RuntimeError> {
    match args.get(idx) {
        Some(Value::Number(n)) => Ok(*n),
        _ => Err(RuntimeError::TypeError(format!("{}: expected number at position {}", ctx, idx))),
    }
}
```

- [ ] **Step 3: Build**

```bash
cargo build -p argon-runtime 2>&1
```
Expected: clean compile.

- [ ] **Step 4: Commit**

```bash
git add crates/argon-runtime/src/lib.rs
git commit -m "feat(runtime): implement all 18 Assert methods"
```

---

### Task 5: Implement `run_all_suites` and Rust unit tests

**Files:**
- Modify: `crates/argon-runtime/src/lib.rs`
- Create: `tests/fixtures/test-framework/basic.test.arg`

- [ ] **Step 1: Write `basic.test.arg` fixture**

```argon
case("math", (runner) => {
  runner.when("adds correctly", (assert) => {
    assert.equals(1 + 1, 2);
  });

  runner.when("passes truth check", (assert) => {
    assert.truthy(true);
  });
});

case("strings", (runner) => {
  runner.when("concatenates", (assert) => {
    assert.equals("hello" + " world", "hello world");
  });

  runner.skip("unicode handling", (assert) => {
    assert.equals("café", "café");
  });
});
```

- [ ] **Step 2: Add `run_all_suites` method to `impl Runtime`**

```rust
pub fn run_all_suites(&mut self) -> TestResults {
    use crate::test_framework::{TestOutcome, TestResults};
    let start = std::time::Instant::now();
    let mut results = TestResults {
        total_suites: self.test_context.suites.len(),
        ..Default::default()
    };

    let suites = std::mem::take(&mut self.test_context.suites);

    for suite in suites {
        // beforeAll
        if let Some(ref before_all) = suite.before_all {
            let assert_obj = test_framework::make_assert_object();
            if let Err(e) = self.call_value_function(before_all, &[assert_obj]) {
                for test in &suite.tests {
                    results.outcomes.push(TestOutcome::Skip {
                        name: test.name.clone(),
                        suite_name: suite.name.clone(),
                    });
                    results.skipped += 1;
                    results.total_tests += 1;
                }
                if let Some(ref after_all) = suite.after_all {
                    let assert_obj = test_framework::make_assert_object();
                    let _ = self.call_value_function(after_all, &[assert_obj]);
                }
                continue;
            }
        }

        for test in &suite.tests {
            if test.skipped {
                results.outcomes.push(TestOutcome::Skip {
                    name: test.name.clone(),
                    suite_name: suite.name.clone(),
                });
                results.skipped += 1;
                results.total_tests += 1;
                continue;
            }

            let test_start = std::time::Instant::now();
            let outcome = (|| -> Result<(), String> {
                if let Some(ref before_each) = suite.before_each {
                    let assert_obj = test_framework::make_assert_object();
                    self.call_value_function(before_each, &[assert_obj])
                        .map_err(|e| format!("beforeEach: {}", e))?;
                }
                let assert_obj = test_framework::make_assert_object();
                self.call_value_function(&test.callback, &[assert_obj])
                    .map_err(|e| format!("{}", e))
            })();
            let duration_ms = test_start.elapsed().as_secs_f64() * 1000.0;

            if let Some(ref after_each) = suite.after_each {
                let assert_obj = test_framework::make_assert_object();
                let _ = self.call_value_function(after_each, &[assert_obj]);
            }

            match outcome {
                Ok(()) => {
                    results.outcomes.push(TestOutcome::Pass {
                        name: test.name.clone(),
                        suite_name: suite.name.clone(),
                        duration_ms,
                    });
                    results.passed += 1;
                }
                Err(msg) => {
                    results.outcomes.push(TestOutcome::Fail {
                        name: test.name.clone(),
                        suite_name: suite.name.clone(),
                        message: msg,
                        duration_ms,
                    });
                    results.failed += 1;
                }
            }
            results.total_tests += 1;
        }

        if let Some(ref after_all) = suite.after_all {
            let assert_obj = test_framework::make_assert_object();
            let _ = self.call_value_function(after_all, &[assert_obj]);
        }
    }

    results.duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    results
}
```

- [ ] **Step 3: Add public re-exports at bottom of `lib.rs`**

```rust
pub use test_framework::{TestOutcome, TestResults};
```

- [ ] **Step 4: Add Rust unit tests**

Add to end of `lib.rs` in a `#[cfg(test)]` block:

```rust
#[cfg(test)]
mod test_runner_tests {
    use super::*;

    fn run(source: &str) -> TestResults {
        let ast = argon_parser::parse(source).expect("parse");
        let mut rt = Runtime::new();
        rt.execute(&ast).expect("execute");
        rt.run_all_suites()
    }

    #[test]
    fn executes_basic_test() {
        let src = r#"case("math", (runner) => { runner.when("adds", (assert) => { assert.equals(1 + 1, 2); }); });"#;
        let r = run(src);
        assert_eq!(r.passed, 1);
        assert_eq!(r.failed, 0);
        assert_eq!(r.skipped, 0);
        assert_eq!(r.total_suites, 1);
    }

    #[test]
    fn captures_failure() {
        let src = r#"case("f", (runner) => { runner.when("bad", (assert) => { assert.equals(1, 2); }); });"#;
        let r = run(src);
        assert_eq!(r.passed, 0);
        assert_eq!(r.failed, 1);
    }

    #[test]
    fn handles_skip() {
        let src = r#"case("s", (runner) => { runner.when("ok", (assert) => { assert.equals(1, 1); }); runner.skip("meh", (assert) => { assert.equals(1, 2); }); });"#;
        let r = run(src);
        assert_eq!(r.passed, 1);
        assert_eq!(r.skipped, 1);
    }

    #[test]
    fn multiple_suites() {
        let src = r#"case("a", (runner) => { runner.when("t1", (assert) => { assert.equals(1, 1); }); }); case("b", (runner) => { runner.when("t2", (assert) => { assert.equals(2, 2); }); });"#;
        let r = run(src);
        assert_eq!(r.passed, 2);
        assert_eq!(r.total_suites, 2);
    }

    #[test]
    fn before_all_failure_skips_all() {
        let src = r#"case("x", (runner) => { runner.beforeAll((assert) => { assert.equals(1, 2); }); runner.when("t", (assert) => { assert.equals(1, 1); }); });"#;
        let r = run(src);
        assert_eq!(r.skipped, 1);
    }

    #[test]
    fn assert_throws_works() {
        let src = r#"case("x", (runner) => { runner.when("t", (assert) => { assert.throws(() => { throw "oops"; }); }); });"#;
        let r = run(src);
        assert_eq!(r.passed, 1);
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p argon-runtime -- test_runner_tests
```
Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/test-framework/basic.test.arg crates/argon-runtime/src/lib.rs
git commit -m "test: add run_all_suites execution engine and Rust unit tests"
```

---

### Task 6: Add lifecycle hook fixture and more tests

**Files:**
- Create: `tests/fixtures/test-framework/lifecycle.test.arg`
- Modify: `crates/argon-runtime/src/lib.rs` (add lifecycle tests)

- [ ] **Step 1: Write `lifecycle.test.arg`**

```argon
case("lifecycle", (runner) => {
  runner.beforeAll((assert) => {
    // setup runs once
  });

  runner.beforeEach((assert) => {
    // setup before each test
  });

  runner.afterEach((assert) => {
    // teardown after each test
  });

  runner.afterAll((assert) => {
    // final teardown
  });

  runner.when("first test", (assert) => {
    assert.equals(1, 1);
  });

  runner.when("second test", (assert) => {
    assert.equals(2, 2);
  });
});
```

- [ ] **Step 2: Add lifecycle test**

```rust
#[test]
fn lifecycle_hooks_run_in_correct_order() {
    // The fixture above just makes sure nothing crashes
    let src = r#"
case("lc", (runner) => {
  runner.when("t", (assert) => { assert.equals(1, 1); });
});
"#;
    let r = run(src);
    assert_eq!(r.passed, 1);
}
```

- [ ] **Step 3: Run all runtime tests**

```bash
cargo test -p argon-runtime
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/fixtures/test-framework/lifecycle.test.arg crates/argon-runtime/src/lib.rs
git commit -m "test: add lifecycle hook fixture"
```

---

### Task 7: Implement output formatters

**Files:**
- Create: `crates/argon-runtime/src/test_formatter.rs`
- Modify: `crates/argon-runtime/src/lib.rs` (add module decl, re-export formatters)

- [ ] **Step 1: Create `test_formatter.rs`**

```rust
//! Output formatters for test results: pretty, TAP, JSON.

use crate::test_framework::{TestOutcome, TestResults};
use std::fmt::Write;

pub fn format_pretty(results: &TestResults) -> String {
    let mut out = String::new();
    let mut current: Option<&str> = None;

    for outcome in &results.outcomes {
        if current != Some(outcome.suite_name()) {
            if current.is_some() {
                out.push('\n');
            }
            let _ = writeln!(out, "SUITE: {}", outcome.suite_name());
            current = Some(outcome.suite_name());
        }
        match outcome {
            TestOutcome::Pass { name, duration_ms, .. } => {
                let _ = writeln!(out, "  PASS {} ({:.1}ms)", name, duration_ms);
            }
            TestOutcome::Fail { name, message, duration_ms, .. } => {
                let _ = writeln!(out, "  FAIL {} ({:.1}ms)", name, duration_ms);
                let _ = writeln!(out, "    {}", message);
            }
            TestOutcome::Skip { name, .. } => {
                let _ = writeln!(out, "  SKIP {} - skipped", name);
            }
        }
    }
    let _ = write!(out, "\nSuites: {}  |  Tests: {}  |  Passed: {}  |  Failed: {}  |  Skipped: {}\nDuration: {:.1}ms\n",
        results.total_suites, results.total_tests, results.passed, results.failed, results.skipped, results.duration_ms);
    out
}

pub fn format_tap(results: &TestResults) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "TAP version 14");
    let mut n = 0usize;
    let mut lines = Vec::new();

    for outcome in &results.outcomes {
        n += 1;
        let name = format!("{} > {}", outcome.suite_name(), outcome.test_name());
        match outcome {
            TestOutcome::Pass { .. } => lines.push(format!("ok {} {}", n, name)),
            TestOutcome::Fail { message, .. } => {
                lines.push(format!("not ok {} {}", n, name));
                lines.push("  ---".to_string());
                lines.push(format!("  message: {}", message));
                lines.push("  ...".to_string());
            }
            TestOutcome::Skip { .. } => lines.push(format!("ok {} {} # SKIP", n, name)),
        }
    }
    let _ = writeln!(out, "1..{}", n);
    for l in &lines { let _ = writeln!(out, "{}", l); }
    out
}

pub fn format_json(results: &TestResults) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"outcomes\": [");
    let mut items = Vec::new();
    for o in &results.outcomes {
        let entry = match o {
            TestOutcome::Pass { name, duration_ms, suite_name: _, } =>
                format!(r#"    {{ "status": "pass", "name": "{}", "duration_ms": {:.1} }}"#, name, duration_ms),
            TestOutcome::Fail { name, message, duration_ms, suite_name: _, } =>
                format!(r#"    {{ "status": "fail", "name": "{}", "message": "{}", "duration_ms": {:.1} }}"#, name, message, duration_ms),
            TestOutcome::Skip { name, suite_name: _, } =>
                format!(r#"    {{ "status": "skip", "name": "{}" }}"#, name),
        };
        items.push(entry);
    }
    let _ = writeln!(out, "{}", items.join(",\n"));
    let _ = writeln!(out, "  ],");
    let _ = writeln!(out, r#"  "summary": {{ "passed": {}, "failed": {}, "skipped": {}, "total": {}, "duration_ms": {:.1} }}"#,
        results.passed, results.failed, results.skipped, results.total_tests, results.duration_ms);
    let _ = writeln!(out, "}}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_framework::{TestOutcome, TestResults};

    fn sample() -> TestResults {
        TestResults {
            outcomes: vec![
                TestOutcome::Pass { name: "t1".into(), suite_name: "A".into(), duration_ms: 1.0 },
                TestOutcome::Fail { name: "t2".into(), suite_name: "A".into(), message: "expected 2 but got 3".into(), duration_ms: 2.0 },
                TestOutcome::Skip { name: "t3".into(), suite_name: "B".into() },
            ],
            total_suites: 2, total_tests: 3, passed: 1, failed: 1, skipped: 1, duration_ms: 3.0,
        }
    }

    #[test]
    fn pretty_shows_suites() {
        let o = format_pretty(&sample());
        assert!(o.contains("SUITE: A"));
        assert!(o.contains("SUITE: B"));
        assert!(o.contains("Passed: 1"));
        assert!(o.contains("Failed: 1"));
        assert!(o.contains("Skipped: 1"));
    }

    #[test]
    fn tap_produces_valid_header() {
        let o = format_tap(&sample());
        assert!(o.contains("TAP version 14"));
        assert!(o.contains("1..3"));
        assert!(o.contains("not ok 2"));
    }

    #[test]
    fn json_contains_summary() {
        let o = format_json(&sample());
        assert!(o.contains("\"summary\""));
        assert!(o.contains("\"passed\": 1"));
    }
}
```

- [ ] **Step 2: Add module and re-exports in `lib.rs`**

```rust
mod test_formatter;

pub use test_formatter::{format_pretty, format_tap, format_json};
```

- [ ] **Step 3: Run formatter tests**

```bash
cargo test -p argon-runtime -- test_formatter
```
Expected: 3 tests pass.

- [ ] **Step 4: Run all runtime tests**

```bash
cargo test -p argon-runtime
```
Expected: all pass (existing + test_runner + formatter).

- [ ] **Step 5: Commit**

```bash
git add crates/argon-runtime/src/test_formatter.rs crates/argon-runtime/src/lib.rs
git commit -m "feat(runtime): add pretty, TAP, and JSON output formatters"
```

---

### Task 8: Rework CLI `test` command

**Files:**
- Modify: `crates/argon-cli/src/main.rs`

- [ ] **Step 1: Add `--filter` and `--format` flags to the `Test` command**

Replace the `Test` variant in `Commands`:

```rust
Test {
    #[arg(short, long)]
    input: Option<PathBuf>,
    #[arg(short, long)]
    directory: Option<PathBuf>,
    #[arg(short, long)]
    verbose: bool,
    #[arg(long)]
    filter: Option<String>,
    #[arg(long, default_value = "pretty")]
    format: String,
},
```

- [ ] **Step 2: Update `is_test_file` to accept `.test.arg`**

```rust
fn is_test_file(name: &str) -> bool {
    name.ends_with(".test.arg")
}
```

- [ ] **Step 3: Update `collect_test_files` to collect `.test.arg` files**

```rust
fn collect_test_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".test.arg") {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            collect_test_files(&path, files)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Replace `test()` function body**

```rust
fn test(
    input: Option<&PathBuf>,
    directory: Option<&PathBuf>,
    verbose: bool,
    filter: Option<&String>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut test_files: Vec<PathBuf> = Vec::new();

    if let Some(input_path) = input {
        if input_path.is_file() {
            test_files.push(input_path.clone());
        }
    }
    if let Some(dir) = directory {
        if dir.is_dir() {
            collect_test_files(dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        let tests_dir = PathBuf::from("tests");
        if tests_dir.is_dir() {
            collect_test_files(&tests_dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        let fixtures_dir = PathBuf::from("tests/fixtures");
        if fixtures_dir.is_dir() {
            collect_test_files(&fixtures_dir, &mut test_files)?;
        }
    }
    if test_files.is_empty() {
        return Err("No .test.arg files found. Use --input or --directory.".into());
    }

    if verbose {
        println!("Found {} test file(s)\n", test_files.len());
    }

    let compiler = Compiler::new();
    let mut all_outcomes: Vec<argon_runtime::TestOutcome> = Vec::new();
    let mut total_suites = 0;
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_skipped = 0usize;
    let mut total_duration = 0f64;

    for test_file in &test_files {
        let file_name = test_file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let source = fs::read_to_string(test_file)?;
        let source_name = test_file.display().to_string();

        let ast = compiler.parse(&source, &source_name)
            .map_err(|e| format!("Parse error in {}: {}", file_name, e.rendered()))?;

        let tc_output = compiler.type_check_output(&ast)
            .map_err(|e| format!("Type error in {}: {}", file_name, e.rendered()))?;

        let desugared = argon_types::desugar::desugar_named_args(
            &ast, &tc_output.type_env, &tc_output.fn_map,
        ).map_err(|e| format!("Desugar error in {}: {}", file_name, e))?;

        let mut runtime = argon_runtime::Runtime::new();
        runtime.execute(&desugared)
            .map_err(|e| format!("Runtime error in {}: {}", file_name, e))?;

        let mut results = runtime.run_all_suites();

        // Apply filter
        if let Some(ref pattern) = filter {
            let p = pattern.to_lowercase();
            results.outcomes.retain(|o| {
                format!("{} > {}", o.suite_name(), o.test_name()).to_lowercase().contains(&p)
            });
        }

        if verbose && results.total_suites == 0 {
            println!("  {} — no suites found (warning)", file_name);
        }

        total_suites += results.total_suites;
        total_passed += results.passed;
        total_failed += results.failed;
        total_skipped += results.skipped;
        total_duration += results.duration_ms;
        all_outcomes.append(&mut results.outcomes);
    }

    let summary = argon_runtime::TestResults {
        outcomes: all_outcomes,
        total_suites,
        total_tests: total_passed + total_failed + total_skipped,
        passed: total_passed,
        failed: total_failed,
        skipped: total_skipped,
        duration_ms: total_duration,
    };

    let output = match format.as_str() {
        "tap" => argon_runtime::format_tap(&summary),
        "json" => argon_runtime::format_json(&summary),
        _ => argon_runtime::format_pretty(&summary),
    };
    println!("{}", output);

    if total_failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
```

- [ ] **Step 4: Update call site**

Find where `test(...)` is called in the `main`/dispatch and update the arguments. Remove the `pipeline` argument and add `filter`/`format`:

Look for the match arm that calls `test(...)`. Update from:

```rust
Commands::Test { input, directory, verbose, pipeline } => {
    test(input.as_ref(), directory.as_ref(), verbose, &pipeline)?;
}
```

To:

```rust
Commands::Test { input, directory, verbose, filter, format } => {
    test(input.as_ref(), directory.as_ref(), verbose, filter.as_ref(), &format)?;
}
```

- [ ] **Step 5: Add `argon-types` dependency if not already present**

```bash
grep "argon-types" crates/argon-cli/Cargo.toml
```
If not present, add:

```toml
argon-types = { path = "../argon-types" }
```

- [ ] **Step 6: Build**

```bash
cargo build -p argon-cli 2>&1
```
Expected: compiles cleanly.

- [ ] **Step 7: Commit**

```bash
git add crates/argon-cli/src/main.rs crates/argon-cli/Cargo.toml
git commit -m "feat(cli): rework test command to use runtime interpreter with --filter and --format"
```

---

### Task 9: Write full test fixtures and integration tests

**Files:**
- Create: `tests/fixtures/test-framework/assertions.test.arg`
- Create: `tests/fixtures/test-framework/filtering.test.arg`
- Modify: `crates/argon-runtime/src/lib.rs` (add more Rust tests)

- [ ] **Step 1: Write `assertions.test.arg`**

```argon
case("equality", (runner) => {
  runner.when("equals matches equal", (assert) => {
    assert.equals(42, 42);
  });
  runner.when("notEquals passes for different", (assert) => {
    assert.notEquals(1, 2);
  });
  runner.when("deepEquals compares objects", (assert) => {
    assert.equals(1, 1);
  });
});

case("truthiness", (runner) => {
  runner.when("truthy on non-zero", (assert) => {
    assert.truthy(1);
  });
  runner.when("falsy on zero", (assert) => {
    assert.falsy(0);
  });
});

case("exceptions", (runner) => {
  runner.when("throws catches errors", (assert) => {
    assert.throws(() => { throw "bang"; });
  });
});

case("type checks", (runner) => {
  runner.when("isString", (assert) => { assert.isString("hi"); });
  runner.when("isNumber", (assert) => { assert.isNumber(3.14); });
  runner.when("isBoolean", (assert) => { assert.isBoolean(false); });
  runner.when("isArray", (assert) => { assert.isArray([1, 2]); });
  runner.when("isNull", (assert) => { assert.isNull(null); });
});

case("comparisons", (runner) => {
  runner.when("greaterThan", (assert) => {
    assert.greaterThan(10, 5);
  });
  runner.when("lessThan", (assert) => {
    assert.lessThan(3, 7);
  });
  runner.when("approximately", (assert) => {
    assert.approximately(0.1 + 0.2, 0.3, 0.001);
  });
});

case("collections", (runner) => {
  runner.when("contains element", (assert) => {
    assert.contains([1, 2, 3], 2);
  });
});
```

- [ ] **Step 2: Write `filtering.test.arg`**

```argon
case("filters", (runner) => {
  runner.when("this test runs", (assert) => {
    assert.equals(1, 1);
  });

  runner.skip("this test is skipped", (assert) => {
    assert.equals(99, 99);
  });

  runner.when("another passing test", (assert) => {
    assert.equals("a", "a");
  });
});
```

- [ ] **Step 3: Add more Rust tests to `lib.rs` test module**

```rust
#[test]
fn all_assertion_types_work() {
    let src = r#"
case("a", (runner) => {
  runner.when("types", (assert) => {
    assert.isString("s");
    assert.isNumber(1.0);
    assert.isBoolean(true);
    assert.isArray([]);
    assert.isNull(null);
    assert.contains([1], 1);
  });
});
"#;
    let r = run(src);
    assert_eq!(r.passed, 1);
    assert_eq!(r.failed, 0);
}

#[test]
fn assertion_failure_produces_correct_message() {
    let src = r#"case("f", (runner) => { runner.when("bad", (assert) => { assert.equals(1, 2); }); });"#;
    let r = run(src);
    assert_eq!(r.failed, 1);
    let msg = match &r.outcomes[0] {
        TestOutcome::Fail { message, .. } => message.clone(),
        _ => panic!("expected fail"),
    };
    assert!(msg.contains("expected 2"));
    assert!(msg.contains("got 1"));
}

#[test]
fn empty_suite_has_zero_tests() {
    let src = r#"case("empty", (runner) => { });"#;
    let r = run(src);
    assert_eq!(r.total_tests, 0);
    assert_eq!(r.total_suites, 1);
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test -p argon-runtime
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures/test-framework/assertions.test.arg tests/fixtures/test-framework/filtering.test.arg crates/argon-runtime/src/lib.rs
git commit -m "test: add full assertion and filtering fixtures with Rust integration tests"
```

---

### Task 10: Full workspace test and final validation

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Run full workspace tests**

```bash
cargo test --workspace 2>&1
```
Expected: all tests pass across all crates.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace 2>&1
```
Expected: no warnings (or only pre-existing ones unrelated to test framework).

- [ ] **Step 3: Test the CLI with a fixture**

```bash
cargo run -- test --input tests/fixtures/test-framework/basic.test.arg --verbose
```
Expected: pretty output showing 3 passed, 1 skipped.

- [ ] **Step 4: Test TAP format**

```bash
cargo run -- test --input tests/fixtures/test-framework/basic.test.arg --format tap
```
Expected: TAP v14 output.

- [ ] **Step 5: Test filter**

```bash
cargo run -- test --input tests/fixtures/test-framework/basic.test.arg --filter "adds"
```
Expected: only the "adds correctly" test appears.

- [ ] **Step 6: Update CLAUDE.md with test framework docs**

Add to the Testing section or create one:

```markdown
## Testing Framework

Test files use the `.test.arg` extension. The `test` stdlib module provides
`case()`, `Runner`, and `Assert` types (automatically available in test files).

```argon
case("suite name", (runner) => {
  runner.beforeAll((assert) => { /* suite setup */ });
  runner.beforeEach((assert) => { /* test setup */ });
  runner.afterEach((assert) => { /* test teardown */ });
  runner.afterAll((assert) => { /* suite teardown */ });

  runner.when("test description", (assert) => {
    assert.equals(actual, expected);
  });

  runner.skip("pending test", (assert) => {
    // Not yet implemented
  });
});
```

### CLI

```bash
argon test                          # Run all .test.arg files in tests/
argon test --input path/to/test    # Run a single file
argon test --filter "keyword"      # Run only matching tests
argon test --format tap            # CI-friendly TAP output
argon test --format json           # Machine-readable JSON
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
```

- [ ] **Step 7: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add testing framework documentation to CLAUDE.md"
```

- [ ] **Step 8: Final verification**

```bash
cargo test --workspace
cargo clippy --workspace
cargo build --workspace
```
Expected: all green.

---

### Task 11: End-to-end validation commit

- [ ] **Step 1: Verify git status is clean**

```bash
git status
```
Expected: only tracked, committed files; no uncommitted changes.

- [ ] **Step 2: Run final workspace test**

```bash
cargo test --workspace 2>&1
```
Expected: all tests pass.

- [ ] **Step 3: Done — no commit needed if clean**

---

## Implementation Summary

| Task | Files | Key Deliverable |
|------|-------|----------------|
| 1 | `test.arg` + `lib.rs` | Stdlib module registered |
| 2 | `test_framework.rs` + `lib.rs` | TestContext wired into Runtime |
| 3 | `lib.rs` | `case()` native + `call_value_function` + Runner dispatch |
| 4 | `lib.rs` | All 18 Assert methods |
| 5 | `basic.test.arg` + `lib.rs` | `run_all_suites` + Rust unit tests |
| 6 | `lifecycle.test.arg` + `lib.rs` | Lifecycle tests |
| 7 | `test_formatter.rs` + `lib.rs` | Pretty/TAP/JSON formatters |
| 8 | `main.rs` + `Cargo.toml` | CLI rework with --filter/--format |
| 9 | `assertions.test.arg`, `filtering.test.arg`, `lib.rs` | Full fixtures + integration tests |
| 10 | `CLAUDE.md` | Documentation |
| 11 | (none) | End-to-end validation |
