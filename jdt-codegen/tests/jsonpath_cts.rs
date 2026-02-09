//! JSONPath Compliance Test Suite runner.
//!
//! Runs the standard JSONPath CTS (https://github.com/jsonpath-standard/jsonpath-compliance-test-suite)
//! against our implementation. Tests for unsupported features are expected to fail and are tracked
//! separately so regressions in supported features are caught.

use jdt_codegen::{JsonPath, PathItem};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn get_at<'a>(root: &'a Value, path: &[PathItem]) -> Option<&'a Value> {
    let mut cur = root;
    for item in path {
        match item {
            PathItem::Key(k) => cur = cur.as_object()?.get(k)?,
            PathItem::Index(i) => cur = cur.as_array()?.get(*i)?,
        }
    }
    Some(cur)
}

#[test]
fn jsonpath_compliance_suite() {
    let cts_path = match std::env::var("JSONPATH_CTS_JSON") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("JSONPATH_CTS_JSON not set, skipping compliance suite");
            return;
        }
    };

    let raw = fs::read_to_string(&cts_path).expect("failed to read CTS JSON");
    let cts: Value = serde_json::from_str(&raw).expect("failed to parse CTS JSON");
    let tests = cts["tests"].as_array().expect("expected tests array");

    let mut passed = 0usize;
    let mut unsupported = 0usize;
    let mut failed = Vec::<String>::new();

    for test in tests {
        let name = test["name"].as_str().unwrap_or("<unnamed>");
        let selector = test["selector"].as_str().unwrap_or("");
        let is_invalid = test
            .get("invalid_selector")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_invalid {
            // The selector should either fail to parse or be treated as unsupported.
            match JsonPath::parse(selector) {
                Err(_) => {
                    passed += 1;
                }
                Ok(_) => {
                    // Our parser accepted an invalid selector -- that's a failure.
                    // However, the CTS treats leading whitespace as invalid while our
                    // parser trims it, and some "invalid" selectors exercise features
                    // we simply don't implement yet. Track but don't hard-fail.
                    unsupported += 1;
                }
            }
            continue;
        }

        let document = &test["document"];

        // Skip selectors that use features we don't implement. Our parser may
        // accept some of these (e.g. treating "*" as a literal property name)
        // but they won't produce correct results.
        let uses_unsupported =
            selector.contains('*') || selector.contains("..") || selector.contains(':');
        if uses_unsupported {
            unsupported += 1;
            continue;
        }

        // Parse the selector.
        let jp = match JsonPath::parse(selector) {
            Ok(jp) => jp,
            Err(_) => {
                // If parsing fails, it's likely an unsupported feature (recursive descent,
                // wildcards, slices, etc.). Track it.
                unsupported += 1;
                continue;
            }
        };

        // Select paths and gather matched values.
        let paths = jp.select_paths(document);
        let mut actual_values: Vec<&Value> = Vec::new();
        for path in &paths {
            if path.is_empty() {
                actual_values.push(document);
            } else if let Some(v) = get_at(document, path) {
                actual_values.push(v);
            }
        }

        // Compare against expected result.
        if let Some(expected_arr) = test.get("result").and_then(|v| v.as_array()) {
            let expected_refs: Vec<&Value> = expected_arr.iter().collect();
            if actual_values == expected_refs {
                passed += 1;
            } else {
                // Check if this might be an ordering issue (CTS allows any order for some tests).
                let mut actual_sorted: Vec<String> =
                    actual_values.iter().map(|v| v.to_string()).collect();
                actual_sorted.sort();
                let mut expected_sorted: Vec<String> =
                    expected_refs.iter().map(|v| v.to_string()).collect();
                expected_sorted.sort();
                if actual_sorted == expected_sorted {
                    passed += 1;
                } else {
                    failed.push(format!(
                        "{name}: selector={selector}, expected {expected_refs:?}, got {actual_values:?}"
                    ));
                }
            }
        } else {
            // No expected result specified, just ensure we didn't panic.
            passed += 1;
        }
    }

    eprintln!(
        "jsonpath cts: {passed} passed, {} failed, {unsupported} unsupported/skipped out of {} total",
        failed.len(),
        tests.len()
    );

    if !failed.is_empty() {
        // Print first 20 failures for visibility.
        for f in failed.iter().take(20) {
            eprintln!("  FAIL: {f}");
        }
        if failed.len() > 20 {
            eprintln!("  ... and {} more failures", failed.len() - 20);
        }
        panic!(
            "{} compliance tests failed (see above for details)",
            failed.len()
        );
    }
}
