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
        // `AET-WO-4300c` F21: the emitter declares the SPI controllers.
        ("spi_devices", "SPI1"),
        ("i2c_devices", "spi6"),
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

/// Every string in `v` equal to `old` becomes `new`, so whatever refers to a
/// renamed net or domain (a constraint, a connector pin) follows it.
fn rename(v: &mut Value, old: &str, new: &str) {
    match v {
        Value::String(s) if s == old => *s = new.to_owned(),
        Value::Array(items) => items.iter_mut().for_each(|x| rename(x, old, new)),
        Value::Object(map) => map.values_mut().for_each(|x| rename(x, old, new)),
        _ => {}
    }
}

/// `validate --file` on the reference with `edit` applied: the exit code and
/// stdout exactly as printed.
fn validate(label: &str, edit: impl FnOnce(&mut Value)) -> (Option<i32>, String) {
    let dir = scratch(label);
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    edit(&mut spec);
    let path = dir.join("spec.json");
    fs::write(&path, spec.to_string()).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["validate", "--file"])
        .arg(&path)
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (run.status.code(), String::from_utf8(run.stdout).unwrap())
}

/// An edit of the reference, and the kind and `id` `generate` refuses it with.
type Moved = (&'static str, fn(&mut Value), &'static str, Value);

/// `AET-WO-1206`: each refusal `generate` makes for a name or a missing bus
/// constraint, with the `id` it prints. One row per rule and spelling.
fn moved() -> Vec<Moved> {
    use serde_json::json;
    vec![
        // A2: a solved port, any case; the twin prints no `id`.
        (
            "a2-gpioa",
            |s| s["i2c_devices"][0]["reference"] = "gpioa".into(),
            "InvalidDeviceName",
            Value::Null,
        ),
        (
            "a2-GpioB",
            |s| s["spi_devices"][0]["reference"] = "GpioB".into(),
            "InvalidDeviceName",
            Value::Null,
        ),
        // A4: the emitter's own `U<n>`, `R<n>`, `C<n>` in another spelling.
        (
            "a4-u1",
            |s| s["i2c_devices"][0]["reference"] = "u1".into(),
            "DuplicateReference",
            json!("U1 and u1"),
        ),
        (
            "a4-r1",
            |s| s["i2c_devices"][1]["reference"] = "r1".into(),
            "DuplicateReference",
            json!("R1 and r1"),
        ),
        (
            "a4-c5",
            |s| s["spi_devices"][0]["reference"] = "c5".into(),
            "DuplicateReference",
            json!("C5 and c5"),
        ),
        (
            "a4-r01",
            |s| s["i2c_devices"][0]["reference"] = "r01".into(),
            "DuplicateReference",
            json!("R1 and r01"),
        ),
        // A5: the emitter's nets, as a net or a domain.
        (
            "a5-vcap",
            |s| rename(s, "I2C_SDA", "U1_VCAP1"),
            "ReservedName",
            json!("U1_VCAP1"),
        ),
        (
            "a5-vcap-case",
            |s| rename(s, "I2C_SDA", "u1_vcap1"),
            "ReservedName",
            json!("U1_VCAP1 and u1_vcap1"),
        ),
        (
            "a5-sdo",
            |s| rename(s, "I2C_SCL", "U2_SDO"),
            "ReservedName",
            json!("U2_SDO"),
        ),
        (
            "a5-hold",
            |s| rename(s, "FLASH_CS", "U4_HOLD"),
            "ReservedName",
            json!("U4_HOLD"),
        ),
        (
            "a5-unconnected",
            |s| rename(s, "SPI_MISO", "unconnected-x"),
            "ReservedName",
            json!("unconnected-x"),
        ),
        (
            "a5-gnd-domain",
            |s| rename(s, "VDD_3V3", "GND"),
            "ReservedName",
            json!("GND"),
        ),
        (
            "a5-wp-domain",
            |s| rename(s, "VDD_3V3", "U4_WP"),
            "ReservedName",
            json!("U4_WP"),
        ),
        // `4100` F1: an I2C line with no net constraint.
        (
            "bus-constraint",
            |s| {
                s["net_constraints"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|c| c["net_name"] != "I2C_SCL");
            },
            "MissingBusConstraint",
            json!("SENSORS"),
        ),
    ]
}

/// `AET-WO-1206`, the behaviour before it: `validate` passes each edit and
/// `generate` refuses it, exit 2, before writing anything.
#[test]
fn aet_1206_validate_passes_what_generate_refuses() {
    for (label, edit, kind, id) in moved() {
        let (code, v, created) = generate(&format!("1206-g-{label}"), edit);
        assert_eq!(code, Some(2), "{label}: {v}");
        assert_eq!(v["error"]["kind"], kind, "{label}: {v}");
        assert_eq!(v["error"]["id"], id, "{label}: {v}");
        assert!(!created, "{label}: generate created its output directory");
        let (code, out) = validate(&format!("1206-v-{label}"), edit);
        assert_eq!(code, Some(0), "{label}: {out}");
    }
}
