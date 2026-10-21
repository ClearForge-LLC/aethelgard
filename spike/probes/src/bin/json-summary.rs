//! S1 probe: summarize a KiCad JSON report without Python.
//!
//!   json-summary REPORT.json
//!
//! Prints the top-level keys, every array of objects with its length and the
//! union of its objects' keys (recursively, by JSON path), and the number of
//! objects that carry a `severity`, which is how KiCad marks a finding.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde_json::Value;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: json-summary REPORT.json");
    let text = std::fs::read_to_string(&path).expect("read report");
    let v: Value = serde_json::from_str(&text).expect("parse report");
    if let Value::Object(map) = &v {
        println!("keys {}", map.keys().cloned().collect::<Vec<_>>().join(","));
        for (k, val) in map {
            if !val.is_array() && !val.is_object() {
                println!("scalar {k}={val}");
            }
        }
    }
    let mut arrays: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
    let mut severities: BTreeMap<String, usize> = BTreeMap::new();
    walk(&v, "$", &mut arrays, &mut severities);
    for (path, (n, keys)) in arrays {
        println!(
            "array {path} len={n} keys={}",
            keys.into_iter().collect::<Vec<_>>().join(",")
        );
    }
    let total: usize = severities.values().sum();
    println!("findings {total}");
    for (sev, n) in severities {
        println!("severity {sev}={n}");
    }
}

fn walk(
    v: &Value,
    path: &str,
    arrays: &mut BTreeMap<String, (usize, BTreeSet<String>)>,
    severities: &mut BTreeMap<String, usize>,
) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(s)) = map.get("severity") {
                *severities.entry(s.clone()).or_default() += 1;
            }
            for (k, child) in map {
                walk(child, &format!("{path}.{k}"), arrays, severities);
            }
        }
        Value::Array(items) => {
            let entry = arrays.entry(format!("{path}[]")).or_default();
            entry.0 += items.len();
            for item in items {
                if let Value::Object(m) = item {
                    entry.1.extend(m.keys().cloned());
                }
            }
            for item in items {
                walk(item, &format!("{path}[]"), arrays, severities);
            }
        }
        _ => {}
    }
}
