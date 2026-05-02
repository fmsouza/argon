//! Test framework native types and execution engine.

// TODO: types and helpers will be used in Tasks 3-5 (case dispatch, assertion impl, execution engine)
#![allow(dead_code)]

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

#[derive(Debug, Clone, Default)]
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
    pub(crate) fn suite_name(&self) -> &str {
        match self {
            TestOutcome::Pass { suite_name, .. }
            | TestOutcome::Fail { suite_name, .. }
            | TestOutcome::Skip { suite_name, .. } => suite_name,
        }
    }

    pub(crate) fn test_name(&self) -> &str {
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
        "equals",
        "notEquals",
        "deepEquals",
        "truthy",
        "falsy",
        "throws",
        "notThrows",
        "isString",
        "isNumber",
        "isBoolean",
        "isArray",
        "isObject",
        "isNull",
        "isUndefined",
        "greaterThan",
        "lessThan",
        "approximately",
        "contains",
        "hasKey",
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
    for method in &[
        "when",
        "skip",
        "beforeEach",
        "afterEach",
        "beforeAll",
        "afterAll",
    ] {
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
                        "runner.{}: expected string name",
                        method
                    )))
                }
            };
            let callback = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => {
                    return Err(RuntimeError::TypeError(format!(
                        "runner.{}: expected function callback",
                        method
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
