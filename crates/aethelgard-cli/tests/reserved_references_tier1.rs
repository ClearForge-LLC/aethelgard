//! `AET-WO-2204` D5, tier 1: `generate` refuses a device reference the Renode
//! platform would refuse, and two references that differ only in case, before
//! it writes anything. No tool is needed: each call runs with an empty `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const SPEC: &str = "examples/sentinel-01-minimal.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-2204-d5-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `generate` on the reference spec with `edit` applied, from the
/// repository root, with an empty `PATH`; returns the exit code, the one-line
/// JSON report, and whether the output directory exists afterwards.
fn generate(label: &str, edit: impl FnOnce(&mut Value)) -> (Option<i32>, Value, bool) {
    let dir = scratch(label);
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    edit(&mut spec);
    let path = dir.join("spec.json");
    fs::write(&path, spec.to_string()).unwrap();
    let out = dir.join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["generate", "--spec"])
        .arg(&path)
        .arg("--out")
        .arg(&out)
        .current_dir(root())
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(run.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    let report = serde_json::from_str(&stdout).unwrap();
    (run.status.code(), report, out.exists())
}

#[test]
fn a_reserved_reference_is_refused_before_any_output() {
    for (list, name) in [
        ("i2c_devices", "nvic"),
        ("i2c_devices", "NVIC"),
        ("i2c_devices", "True"),
        ("spi_devices", "sysbus"),
        ("spi_devices", "crypto"),
    ] {
        let (code, v, created) = generate(&format!("reserved-{list}-{name}"), |spec| {
            spec[list][0]["reference"] = name.into();
        });
        assert_eq!(code, Some(2), "{name}: {v}");
        assert_eq!(v["ok"], false, "{v}");
        assert_eq!(v["error"]["kind"], "InvalidIdentifier", "{name}: {v}");
        assert!(
            v["error"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("already holds")),
            "{name}: {v}"
        );
        assert!(!created, "{name}: generate created its output directory");
    }
}

#[test]
fn references_that_differ_only_in_case_are_refused_before_any_output() {
    let (code, v, created) = generate("case", |spec| {
        spec["i2c_devices"][1]["reference"] = "u2".into();
    });
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "DuplicateReference", "{v}");
    assert!(!created, "generate created its output directory");
}
