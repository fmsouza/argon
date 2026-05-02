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
            TestOutcome::Pass {
                name, duration_ms, ..
            } => {
                let _ = writeln!(out, "  PASS {} ({:.1}ms)", name, duration_ms);
            }
            TestOutcome::Fail {
                name,
                message,
                duration_ms,
                ..
            } => {
                let _ = writeln!(out, "  FAIL {} ({:.1}ms)", name, duration_ms);
                let _ = writeln!(out, "    {}", message);
            }
            TestOutcome::Skip { name, .. } => {
                let _ = writeln!(out, "  SKIP {} - skipped", name);
            }
        }
    }
    let _ = write!(
        out,
        "\nSuites: {}  |  Tests: {}  |  Passed: {}  |  Failed: {}  |  Skipped: {}\nDuration: {:.1}ms\n",
        results.total_suites,
        results.total_tests,
        results.passed,
        results.failed,
        results.skipped,
        results.duration_ms
    );
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
    for l in &lines {
        let _ = writeln!(out, "{}", l);
    }
    out
}

pub fn format_json(results: &TestResults) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"outcomes\": [");
    let mut items = Vec::new();
    for o in &results.outcomes {
        let entry = match o {
            TestOutcome::Pass {
                name,
                duration_ms,
                suite_name: _,
            } => format!(
                r#"    {{ "status": "pass", "name": "{}", "duration_ms": {:.1} }}"#,
                name, duration_ms
            ),
            TestOutcome::Fail {
                name,
                message,
                duration_ms,
                suite_name: _,
            } => format!(
                r#"    {{ "status": "fail", "name": "{}", "message": "{}", "duration_ms": {:.1} }}"#,
                name, message, duration_ms
            ),
            TestOutcome::Skip {
                name,
                suite_name: _,
            } => format!(r#"    {{ "status": "skip", "name": "{}" }}"#, name),
        };
        items.push(entry);
    }
    let _ = writeln!(out, "{}", items.join(",\n"));
    let _ = writeln!(out, "  ],");
    let _ = writeln!(
        out,
        r#"  "summary": {{ "passed": {}, "failed": {}, "skipped": {}, "total": {}, "duration_ms": {:.1} }}"#,
        results.passed, results.failed, results.skipped, results.total_tests, results.duration_ms
    );
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
                TestOutcome::Pass {
                    name: "t1".into(),
                    suite_name: "A".into(),
                    duration_ms: 1.0,
                },
                TestOutcome::Fail {
                    name: "t2".into(),
                    suite_name: "A".into(),
                    message: "expected 2 but got 3".into(),
                    duration_ms: 2.0,
                },
                TestOutcome::Skip {
                    name: "t3".into(),
                    suite_name: "B".into(),
                },
            ],
            total_suites: 2,
            total_tests: 3,
            passed: 1,
            failed: 1,
            skipped: 1,
            duration_ms: 3.0,
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
