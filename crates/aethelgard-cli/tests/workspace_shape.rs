//! The mechanical half of the `INV-01` dependency direction and the `unsafe`
//! forbid: every member opts into the workspace lints and states the forbid at
//! its root, the five library crates declare nothing and depend only on the
//! allowed crates, and `aethelgard-ir` depends on no other member. A library
//! crate is exactly a manifest and a `src/lib.rs`, so no other target or
//! module can hide a declaration from the check.

use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

const MEMBERS: [&str; 7] = [
    "crates/aethelgard-ir",
    "crates/aethelgard-eda",
    "crates/aethelgard-svd-hal",
    "crates/aethelgard-twin",
    "crates/aethelgard-chaos",
    "crates/aethelgard-evolve",
    "crates/aethelgard-cli",
];

/// Every member but `ir` (first) and `cli` (last); the order is pinned by
/// `members_are_exactly_the_seven_parts`.
fn libraries() -> &'static [&'static str] {
    &MEMBERS[1..6]
}

const LIBRARY_DEPENDENCIES: [&str; 3] = ["aethelgard-ir", "serde", "thiserror"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest(member: &str) -> Table {
    let path = root().join(member).join("Cargo.toml");
    fs::read_to_string(&path).unwrap().parse().unwrap()
}

/// Every dependency's package name (after any `package =` rename) in every
/// dependency table, target-specific ones included.
fn dependency_names(manifest: &Table) -> Vec<String> {
    let mut tables = vec![manifest];
    if let Some(Value::Table(targets)) = manifest.get("target") {
        tables.extend(targets.values().filter_map(Value::as_table));
    }
    let mut names = Vec::new();
    for table in tables {
        for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(Value::Table(deps)) = table.get(key) {
                names.extend(deps.iter().map(|(key, dep)| {
                    dep.get("package")
                        .and_then(Value::as_str)
                        .unwrap_or(key)
                        .to_owned()
                }));
            }
        }
    }
    names
}

/// Every file under `dir`, as paths relative to `dir`, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir).unwrap();
                found.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

fn crate_root(member: &str) -> syn::File {
    let src = root().join(member).join("src");
    let path = ["lib.rs", "main.rs"]
        .iter()
        .map(|name| src.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("{member} has neither src/lib.rs nor src/main.rs"));
    syn::parse_file(&fs::read_to_string(path).unwrap()).unwrap()
}

fn forbids_unsafe(file: &syn::File) -> bool {
    file.attrs.iter().any(|attr| {
        attr.path().is_ident("forbid")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "unsafe_code")
    })
}

#[test]
fn members_are_exactly_the_seven_parts() {
    let root = manifest("");
    let members: Vec<&str> = root["workspace"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert_eq!(members, MEMBERS);
    assert_eq!(
        root["workspace"]["lints"]["rust"]["unsafe_code"].as_str(),
        Some("forbid")
    );
}

#[test]
fn every_member_opts_into_workspace_lints() {
    for member in MEMBERS {
        let lints = manifest(member).get("lints").cloned();
        assert_eq!(
            lints.and_then(|l| l.get("workspace").and_then(Value::as_bool)),
            Some(true),
            "{member} lacks [lints] workspace = true"
        );
    }
}

#[test]
fn every_crate_root_forbids_unsafe() {
    for member in MEMBERS {
        assert!(
            forbids_unsafe(&crate_root(member)),
            "{member} lacks #![forbid(unsafe_code)]"
        );
    }
}

#[test]
fn no_member_has_a_build_script() {
    for member in MEMBERS {
        assert!(
            !root().join(member).join("build.rs").exists(),
            "{member} has build.rs"
        );
        assert!(
            manifest(member)["package"].get("build").is_none(),
            "{member} sets package.build"
        );
    }
}

#[test]
fn library_crates_declare_nothing_and_only_re_export() {
    for member in libraries() {
        for item in crate_root(member).items {
            assert!(
                matches!(item, syn::Item::Use(_)),
                "{member} declares something other than a re-export"
            );
        }
    }
}

#[test]
fn library_crates_are_only_a_manifest_and_lib_rs() {
    for member in libraries() {
        assert_eq!(
            files(&root().join(member)),
            ["Cargo.toml", "src/lib.rs"],
            "{member} has files beyond Cargo.toml and src/lib.rs"
        );
        for key in manifest(member).keys() {
            assert!(
                ["package", "dependencies", "lints"].contains(&key.as_str()),
                "{member} sets [{key}]"
            );
        }
    }
}

#[test]
fn library_crates_depend_only_on_allowed_crates() {
    for member in libraries() {
        for name in dependency_names(&manifest(member)) {
            assert!(
                LIBRARY_DEPENDENCIES.contains(&name.as_str()),
                "{member} depends on {name}"
            );
        }
    }
}

#[test]
fn ir_depends_on_no_other_member() {
    for name in dependency_names(&manifest("crates/aethelgard-ir")) {
        assert!(
            !name.starts_with("aethelgard"),
            "aethelgard-ir depends on {name}"
        );
    }
}
