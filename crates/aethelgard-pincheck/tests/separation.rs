//! Ruling D7 on PR #130, mechanically: the check and the generator live in
//! crates that do not depend on each other. Cargo's own resolve is the proof.
//! The check reaches exactly one workspace crate, `aethelgard-ir`, through any
//! dependency kind, dev included; so it reaches neither `aethelgard-svd-hal`
//! (the board crate generator) nor any future `hwdef.dat` generator. And only
//! the CLI, which wires the check, reaches it through what a crate links:
//! normal and build dependencies. (`svd-hal`, `twin` and `evolve` each have a
//! test-only dev-dependency on the CLI, which links nothing into them.)

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

/// Package name to the names of everything it depends on, per Cargo's resolve,
/// through dev-dependencies too when `dev`.
fn graph(dev: bool) -> (BTreeMap<String, Vec<String>>, BTreeSet<String>) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: Value = serde_json::from_slice(&out.stdout).unwrap();
    let name = |id: &Value| {
        let id = id.as_str().unwrap();
        meta["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()["name"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let members = meta["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(name)
        .collect();
    let graph = meta["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            let deps = n["deps"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| {
                    d["dep_kinds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|k| dev || k["kind"] != "dev")
                })
                .map(|d| name(&d["pkg"]))
                .collect();
            (name(&n["id"]), deps)
        })
        .collect();
    (graph, members)
}

fn reach(graph: &BTreeMap<String, Vec<String>>, from: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![from.to_owned()];
    while let Some(p) = todo.pop() {
        for d in &graph[&p] {
            if seen.insert(d.clone()) {
                todo.push(d.clone());
            }
        }
    }
    seen
}

#[test]
fn the_check_reaches_only_the_ir_among_workspace_crates() {
    let (graph, members) = graph(true);
    let reached: Vec<String> = reach(&graph, "aethelgard-pincheck")
        .intersection(&members)
        .cloned()
        .collect();
    assert_eq!(reached, ["aethelgard-ir"]);
}

#[test]
fn only_the_cli_links_the_check() {
    let (graph, members) = graph(false);
    let reaching: Vec<&String> = members
        .iter()
        .filter(|m| reach(&graph, m).contains("aethelgard-pincheck"))
        .collect();
    assert_eq!(reaching, ["aethelgard-cli"]);
}
