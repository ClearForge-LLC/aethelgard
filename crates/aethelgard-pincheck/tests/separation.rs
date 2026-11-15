//! Ruling D7 on PR #130, mechanically: the check and the generator live in
//! crates that do not depend on each other. The proof is every workspace
//! member's declared dependencies, as Cargo reads the manifests (`cargo
//! metadata --no-deps`, which needs no download, so it runs under `--network
//! none`). The check reaches exactly one workspace crate, `aethelgard-ir`,
//! through any dependency kind, dev included; so it reaches neither
//! `aethelgard-svd-hal` (the board crate generator) nor any future `hwdef.dat`
//! generator. And only the CLI, which wires the check, reaches it through what
//! a crate links: normal and build dependencies. (`svd-hal`, `twin` and
//! `evolve` each have a test-only dev-dependency on the CLI, which links
//! nothing into them.) A workspace crate can reach another only through
//! workspace crates, so the member graph is the whole path.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

/// Each workspace member to the members it declares as dependencies, dev
/// dependencies included when `dev`.
fn graph(dev: bool) -> BTreeMap<String, Vec<String>> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: Value = serde_json::from_slice(&out.stdout).unwrap();
    let packages = meta["packages"].as_array().unwrap();
    let members: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    packages
        .iter()
        .map(|p| {
            let deps = p["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| dev || d["kind"] != "dev")
                .map(|d| d["name"].as_str().unwrap())
                .filter(|n| members.contains(n))
                .map(str::to_owned)
                .collect();
            (p["name"].as_str().unwrap().to_owned(), deps)
        })
        .collect()
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
    let graph = graph(true);
    let reached: Vec<String> = reach(&graph, "aethelgard-pincheck").into_iter().collect();
    assert_eq!(reached, ["aethelgard-ir"]);
}

#[test]
fn only_the_cli_links_the_check() {
    let graph = graph(false);
    let reaching: Vec<&String> = graph
        .keys()
        .filter(|m| reach(&graph, m).contains("aethelgard-pincheck"))
        .collect();
    assert_eq!(reaching, ["aethelgard-cli"]);
}
