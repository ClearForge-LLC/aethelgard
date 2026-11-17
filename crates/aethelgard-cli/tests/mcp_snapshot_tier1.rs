//! Tier 1 of `AET-WO-1604` §1.2: the MCP tool list is the contract
//! (`crates/aethelgard-cli/README.md`, `### The tool list`). `tools/list`, as
//! `aethelgard mcp` serves it over stdio, must equal `tests/mcp-tools.json`
//! value for value; a difference is named by tool and field.
//!
//! After a deliberate change to a subcommand's arguments or help text,
//! `AETHELGARD_BLESS=1 cargo test -p aethelgard-cli --test mcp_snapshot_tier1`
//! rewrites the snapshot; the diff is then the contract change for review.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use common::Client;
use serde_json::{Value, json};

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/mcp-tools.json")
}

/// The tool list as a client receives it.
fn served() -> Value {
    let mut c = Client::spawn(|_| {});
    c.initialize();
    let list = c.request("tools/list", json!({}));
    let tools = list["result"]["tools"].clone();
    assert!(tools.is_array(), "{list}");
    tools
}

/// Every leaf of `v` by its JSON pointer below `at`; an empty object or
/// array is a leaf of its own.
fn leaves(v: &Value, at: String, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            for (k, child) in m {
                let k = k.replace('~', "~0").replace('/', "~1");
                leaves(child, format!("{at}/{k}"), out);
            }
        }
        Value::Array(a) if !a.is_empty() => {
            for (i, child) in a.iter().enumerate() {
                leaves(child, format!("{at}/{i}"), out);
            }
        }
        _ => {
            out.insert(at, v.clone());
        }
    }
}

fn by_name(list: &Value) -> Vec<(String, &Value)> {
    list.as_array()
        .expect("a tool list is an array")
        .iter()
        .map(|t| {
            let name = t["name"].as_str().expect("every tool has a name");
            (name.to_owned(), t)
        })
        .collect()
}

/// Every difference between the committed list and the served one, each
/// naming the tool and the field.
fn differences(committed: &Value, served: &Value) -> Vec<String> {
    let old = by_name(committed);
    let new = by_name(served);
    let mut out = Vec::new();
    for (name, _) in &old {
        if !new.iter().any(|(n, _)| n == name) {
            out.push(format!("tool `{name}`: removed"));
        }
    }
    for (name, _) in &new {
        if !old.iter().any(|(n, _)| n == name) {
            out.push(format!("tool `{name}`: added"));
        }
    }
    let order = |list: &[(String, &Value)], other: &[(String, &Value)]| -> Vec<String> {
        list.iter()
            .filter(|(n, _)| other.iter().any(|(o, _)| o == n))
            .map(|(n, _)| n.clone())
            .collect()
    };
    let (old_order, new_order) = (order(&old, &new), order(&new, &old));
    if old_order != new_order {
        out.push(format!(
            "tool order: committed {old_order:?}, served {new_order:?}"
        ));
    }
    for (name, was) in &old {
        let Some((_, now)) = new.iter().find(|(n, _)| n == name) else {
            continue;
        };
        let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
        leaves(was, String::new(), &mut a);
        leaves(now, String::new(), &mut b);
        for (field, v) in &a {
            match b.get(field) {
                None => out.push(format!("tool `{name}`: field `{field}` removed (was {v})")),
                Some(w) if w != v => out.push(format!(
                    "tool `{name}`: field `{field}` changed from {v} to {w}"
                )),
                Some(_) => {}
            }
        }
        for (field, w) in &b {
            if !a.contains_key(field) {
                out.push(format!("tool `{name}`: field `{field}` added ({w})"));
            }
        }
    }
    out
}

#[test]
fn the_served_tool_list_is_the_committed_snapshot() {
    let served = served();
    if std::env::var_os("AETHELGARD_BLESS").is_some() {
        let mut text = serde_json::to_string_pretty(&served).unwrap();
        text.push('\n');
        fs::write(snapshot_path(), text).unwrap();
    }
    let committed: Value = serde_json::from_str(
        &fs::read_to_string(snapshot_path()).expect("tests/mcp-tools.json is committed"),
    )
    .expect("tests/mcp-tools.json is JSON");
    let diff = differences(&committed, &served);
    assert!(
        diff.is_empty(),
        "the MCP tool list differs from tests/mcp-tools.json:\n{}\nIf the change is meant, \
         `AETHELGARD_BLESS=1 cargo test -p aethelgard-cli --test mcp_snapshot_tier1` rewrites it.",
        diff.join("\n")
    );
}

fn committed() -> Value {
    serde_json::from_str(&fs::read_to_string(snapshot_path()).unwrap()).unwrap()
}

fn tool_mut<'a>(list: &'a mut Value, name: &str) -> &'a mut Value {
    list.as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["name"] == name)
        .unwrap()
}

#[test]
fn an_added_required_argument_is_named_by_tool_and_field() {
    let was = committed();
    let mut now = was.clone();
    let schema = &mut tool_mut(&mut now, "verify")["inputSchema"];
    schema["properties"]["board-rev"] = json!({"type": "string", "description": "x"});
    schema["required"]
        .as_array_mut()
        .unwrap()
        .push("board-rev".into());
    let diff = differences(&was, &now);
    assert!(
        diff.iter().any(|d| d
            .starts_with("tool `verify`: field `/inputSchema/properties/board-rev/type` added")),
        "{diff:#?}"
    );
    assert!(
        diff.iter().any(
            |d| d.starts_with("tool `verify`: field `/inputSchema/required/")
                && d.ends_with("added (\"board-rev\")")
        ),
        "{diff:#?}"
    );
}

#[test]
fn an_optional_argument_made_required_is_named() {
    let was = committed();
    let mut now = was.clone();
    let schema = &mut tool_mut(&mut now, "check_drc")["inputSchema"];
    let required = schema["required"].as_array_mut().unwrap();
    required.push("spec".into());
    let diff = differences(&was, &now);
    assert_eq!(diff.len(), 1, "{diff:#?}");
    assert!(
        diff[0].starts_with("tool `check_drc`: field `/inputSchema/required/"),
        "{diff:#?}"
    );
}

#[test]
fn a_removed_tool_a_renamed_field_and_a_reordered_list_are_named() {
    let was = committed();
    let mut now = was.clone();
    let tools = now.as_array_mut().unwrap();
    let gone = tools.remove(0);
    let gone = gone["name"].as_str().unwrap().to_owned();
    tools.swap(0, 1);
    let (a, b) = (
        tools[0]["name"].as_str().unwrap().to_owned(),
        tools[1]["name"].as_str().unwrap().to_owned(),
    );
    tools[0]["description"] = "changed".into();
    let diff = differences(&was, &now);
    assert!(
        diff.contains(&format!("tool `{gone}`: removed")),
        "{diff:#?}"
    );
    assert!(
        diff.iter().any(|d| d.starts_with("tool order:")),
        "{diff:#?}"
    );
    assert!(
        diff.iter()
            .any(|d| d.starts_with(&format!("tool `{a}`: field `/description` changed"))),
        "{diff:#?}"
    );
    assert!(
        !diff
            .iter()
            .any(|d| d.starts_with(&format!("tool `{b}`: field")))
    );
}

#[test]
fn an_identical_list_has_no_difference() {
    let was = committed();
    assert_eq!(differences(&was, &was.clone()), Vec::<String>::new());
}
