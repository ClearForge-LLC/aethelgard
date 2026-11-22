//! `AET-WO-1215`: an older specification is refused with every change it
//! lacks, from `versions::CHANGES`, and the table is held to the version
//! literal, to the schema (by serde, on every committed current-version
//! specification); `readme_versions.rs` holds it to the README's prose. Uses only `aethelgard_ir`,
//! `serde_json` and `std` (`workspace_shape`).

use aethelgard_ir::versions::{self, CHANGES, VersionChange};
use aethelgard_ir::{IrValidationError as E, SPECIFICATION_SCHEMA_VERSION, Specification};
use serde_json::{Value, json};

fn repo() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example() -> Value {
    serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json")).unwrap()
}

fn refused(version: u32) -> E {
    let mut v = example();
    v["schema_version"] = json!(version);
    serde_json::from_value::<Specification>(v)
        .unwrap()
        .validate()
        .unwrap_err()
}

fn changes(e: &E) -> &'static [VersionChange] {
    match e {
        E::UnsupportedSchemaVersion { changes, .. } => changes,
        other => panic!("not UnsupportedSchemaVersion: {other}"),
    }
}

fn versions_of(c: &[VersionChange]) -> Vec<u32> {
    c.iter().map(|c| c.version).collect()
}

#[test]
fn the_last_row_is_the_version_literal_and_none_is_skipped() {
    assert_eq!(
        CHANGES.last().map(|c| c.version),
        Some(SPECIFICATION_SCHEMA_VERSION),
        "a bump needs its row in versions::CHANGES"
    );
    assert_eq!(
        versions_of(CHANGES),
        (2..=SPECIFICATION_SCHEMA_VERSION).collect::<Vec<_>>()
    );
}

#[test]
fn a_version_20_file_lists_21_to_26_with_their_work_orders() {
    let e = refused(20);
    assert_eq!(e.kind(), "UnsupportedSchemaVersion");
    assert_eq!(e.id(), None);
    let c = changes(&e);
    assert_eq!(versions_of(c), [21, 22, 23, 24, 25, 26]);
    assert_eq!(
        c.iter().map(|c| c.work_order).collect::<Vec<_>>(),
        [
            "AET-WO-1210",
            "AET-WO-1211a",
            "AET-WO-4240b",
            "AET-WO-1211e",
            "AET-WO-1216b",
            "AET-WO-1217"
        ]
    );
    let m = e.to_string();
    assert!(m.starts_with("schema_version is 20, not 26; "), "{m}");
    for (v, wo) in [
        (21, "AET-WO-1210"),
        (22, "AET-WO-1211a"),
        (23, "AET-WO-4240b"),
        (24, "AET-WO-1211e"),
        (25, "AET-WO-1216b"),
        (26, "AET-WO-1217"),
    ] {
        assert!(m.contains(&format!(" {v} ({wo})")), "{m}");
    }
    assert!(m.contains("motor_outputs[].pwm.idle"), "{m}");
    assert!(m.contains("mcu.hse.crystal.startup_max_us"), "{m}");
    assert!(m.contains("pin_devices[].bulk"), "{m}");
    assert!(m.contains("refuses by [OffDomainAboveLimit]"), "{m}");
    assert!(m.contains("refuses by [AnalogPadPin]"), "{m}");
    assert!(!m.contains(" 20 ("), "{m}");
}

#[test]
fn a_version_1_file_lists_2_to_26() {
    let c = changes(&refused(1));
    assert_eq!(versions_of(c), (2..=26).collect::<Vec<_>>());
    assert_eq!(c, CHANGES);
}

#[test]
fn a_newer_file_and_version_0_name_the_current_version_only() {
    for v in [SPECIFICATION_SCHEMA_VERSION + 1, 0, u32::MAX] {
        let e = refused(v);
        assert!(changes(&e).is_empty(), "{v}");
        assert_eq!(e.to_string(), format!("schema_version is {v}, not 26"));
    }
}

#[test]
fn each_older_version_lacks_exactly_the_rows_after_it() {
    for v in 1..SPECIFICATION_SCHEMA_VERSION {
        assert_eq!(
            versions_of(versions::lacked_by(v)),
            (v + 1..=SPECIFICATION_SCHEMA_VERSION).collect::<Vec<_>>(),
            "{v}"
        );
    }
    assert!(versions::lacked_by(SPECIFICATION_SCHEMA_VERSION).is_empty());
}

/// A message names paths and kinds, never a value: no digit but the
/// versions, and none of the values the README restates.
#[test]
fn the_message_suggests_no_value() {
    let m = refused(1).to_string();
    for word in [
        "null",
        "true",
        "false",
        "pulse_us\":",
        "1,000,000",
        "= ",
        "e.g.",
    ] {
        assert!(!m.contains(word), "{word}: {m}");
    }
}

// The schema, by serde on every committed current-version specification.

/// Every committed specification at the current version.
fn current_specifications() -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for dir in [
        "examples",
        "crates/aethelgard-cli/tests/fixtures",
        "crates/aethelgard-ir/tests/fixtures",
    ] {
        let mut paths: Vec<_> = std::fs::read_dir(repo().join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        paths.sort();
        for p in paths {
            let Ok(v) = serde_json::from_str::<Value>(&std::fs::read_to_string(&p).unwrap()) else {
                continue;
            };
            if v.get("mcu").is_some_and(Value::is_object)
                && v["schema_version"] == json!(SPECIFICATION_SCHEMA_VERSION)
                && serde_json::from_value::<Specification>(v.clone()).is_ok()
            {
                out.push((p.display().to_string(), v));
            }
        }
    }
    assert!(out.len() >= 20, "{}", out.len());
    out
}

/// The JSON pointers of every object `path` less its last key reaches in
/// `v`, and that key. `[]` steps into each element.
fn parents(v: &Value, path: &str) -> (Vec<String>, String) {
    let mut steps: Vec<&str> = path.split('.').collect();
    let key = steps.pop().unwrap().to_owned();
    let mut at = vec![(String::new(), v)];
    for step in steps {
        let (name, each) = match step.strip_suffix("[]") {
            Some(n) => (n, true),
            None => (step, false),
        };
        let mut next = Vec::new();
        for (ptr, node) in at {
            let Some(child) = node.get(name) else {
                continue;
            };
            let ptr = format!("{ptr}/{name}");
            if each {
                for (i, e) in child.as_array().into_iter().flatten().enumerate() {
                    next.push((format!("{ptr}/{i}"), e));
                }
            } else if child.is_object() {
                next.push((ptr, child));
            }
        }
        at = next;
    }
    (at.into_iter().map(|(p, _)| p).collect(), key)
}

/// The parse error of `v` with `edit` applied at `ptr`, or `None` if it parses.
fn parse_with(
    v: &Value,
    ptr: &str,
    edit: impl FnOnce(&mut serde_json::Map<String, Value>),
) -> Option<String> {
    let mut v = v.clone();
    edit(v.pointer_mut(ptr).unwrap().as_object_mut().unwrap());
    serde_json::from_value::<Specification>(v)
        .err()
        .map(|e| e.to_string())
}

/// Each `(file, pointer)` under which `path`'s parent exists, across `specs`.
fn sites<'a>(
    specs: &'a [(String, Value)],
    path: &str,
) -> (Vec<(&'a str, &'a Value, String)>, String) {
    let mut out = Vec::new();
    let mut key = String::new();
    for (f, v) in specs {
        let (ptrs, k) = parents(v, path);
        key = k;
        out.extend(ptrs.into_iter().map(|p| (f.as_str(), v, p)));
    }
    (out, key)
}

const PROBE: &str = "\u{1}probe";

/// A path a later row replaced or removed is judged as such, not as a key
/// of today's schema.
fn superseded() -> Vec<&'static str> {
    CHANGES
        .iter()
        .flat_map(|c| {
            c.replaced
                .iter()
                .map(|r| r.path)
                .chain(c.removed.iter().copied())
        })
        .collect()
}

#[test]
fn every_required_path_is_a_key_the_current_schema_requires() {
    let specs = current_specifications();
    let gone = superseded();
    for c in CHANGES {
        for path in c.requires.iter().filter(|p| !gone.contains(p)) {
            let (at, key) = sites(&specs, path);
            let held: Vec<_> = at
                .iter()
                .filter(|(_, v, p)| v.pointer(p).unwrap().get(&key).is_some())
                .collect();
            assert!(
                !held.is_empty(),
                "{}: no committed specification holds `{path}`",
                c.version
            );
            for (f, v, p) in held {
                let err = parse_with(v, p, |o| {
                    o.remove(&key);
                })
                .unwrap_or_else(|| panic!("{f}: `{path}` removed at {p} still parses"));
                assert!(
                    err.contains(&format!("missing field `{key}`")),
                    "{f} {p}: {err}"
                );
            }
        }
    }
}

#[test]
fn every_optional_path_is_a_key_the_current_schema_admits_absent() {
    let specs = current_specifications();
    for c in CHANGES {
        for path in c.optional {
            let (at, key) = sites(&specs, path);
            assert!(
                !at.is_empty(),
                "{}: no committed specification holds `{path}`'s parent",
                c.version
            );
            for (f, v, p) in &at {
                // Absent parses (the key is optional) ...
                assert_eq!(
                    parse_with(v, p, |o| {
                        o.remove(&key);
                    }),
                    None,
                    "{f} {p}: `{path}`"
                );
                // ... and the key is the schema's: a wrong value is not an unknown field.
                let err = parse_with(v, p, |o| {
                    o.insert(key.clone(), json!(PROBE));
                })
                .unwrap_or_else(|| panic!("{f}: `{path}` takes any string"));
                assert!(!err.contains("unknown field"), "{f} {p}: {err}");
            }
        }
    }
}

#[test]
fn every_replaced_or_removed_path_is_refused_today() {
    let specs = current_specifications();
    let mut seen = 0;
    for path in superseded() {
        let (at, key) = sites(&specs, path);
        assert!(
            !at.is_empty(),
            "no committed specification holds `{path}`'s parent"
        );
        for (f, v, p) in at {
            if path == "mcu.hse_hz" {
                hse_hz_is_the_version_19_spelling_only(f, v, &p);
            } else {
                let err = parse_with(v, &p, |o| {
                    o.insert(key.clone(), json!(PROBE));
                })
                .unwrap_or_else(|| panic!("{f}: `{path}` still parses"));
                assert!(
                    err.contains(&format!("unknown field `{key}`")),
                    "{f} {p}: {err}"
                );
            }
            seen += 1;
        }
    }
    assert!(seen > 0);
}

/// The one replaced key the parser still reads (`AET-WO-4240a` §1.2, README
/// version 20): `hse_hz: 0` alone is the version-19 spelling of `hse: null`.
/// Any other value, beside `hse` or not, is refused, and `0` beside `hse` is
/// refused, so no file states both.
fn hse_hz_is_the_version_19_spelling_only(f: &str, v: &Value, p: &str) {
    let alone = |n: u32| {
        parse_with(v, p, |o| {
            o.remove("hse");
            o.insert("hse_hz".into(), json!(n));
        })
    };
    let beside = |n: u32| {
        parse_with(v, p, |o| {
            o.insert("hse".into(), Value::Null);
            o.insert("hse_hz".into(), json!(n));
        })
    };
    if v.pointer(p).unwrap().get("hse") == Some(&Value::Null)
        || v.pointer(p).unwrap().get("hse_hz") == Some(&json!(0))
    {
        assert_eq!(alone(0), None, "{f}");
    }
    assert!(alone(8_000_000).is_some(), "{f}");
    assert!(beside(0).is_some(), "{f}");
    assert!(beside(8_000_000).is_some(), "{f}");
}

#[test]
fn every_rule_is_a_kind_this_crate_returns() {
    let mut src = String::new();
    for e in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap() != "versions.rs" {
            src += &std::fs::read_to_string(p).unwrap();
        }
    }
    for c in CHANGES {
        for kind in c.rules {
            assert!(
                src.contains(&format!("=> \"{kind}\"")),
                "{}: `{kind}`",
                c.version
            );
        }
    }
}
