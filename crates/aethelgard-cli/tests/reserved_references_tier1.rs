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
    let (code, stdout, created) = generate_raw(label, edit);
    (code, serde_json::from_str(&stdout).unwrap(), created)
}

/// [`generate`], with stdout exactly as printed.
fn generate_raw(label: &str, edit: impl FnOnce(&mut Value)) -> (Option<i32>, String, bool) {
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
    (run.status.code(), stdout, out.exists())
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

/// `<command> --file` (`validate`, `solve`) on the reference with `edit`
/// applied: the exit code and stdout exactly as printed.
fn run_file(command: &str, label: &str, edit: impl FnOnce(&mut Value)) -> (Option<i32>, String) {
    let dir = scratch(label);
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    edit(&mut spec);
    let path = dir.join("spec.json");
    fs::write(&path, spec.to_string()).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([command, "--file"])
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
        // Two of the specification's references with one KiCad key, by a
        // leading zero (the adversarial pass's F1); `generate`'s board writer
        // refuses the pair.
        (
            "a4-key-u02",
            |s| s["i2c_devices"][1]["reference"] = "U02".into(),
            "DuplicateReference",
            json!("U02 and U2"),
        ),
        (
            "a4-key-j01",
            |s| s["i2c_devices"][0]["reference"] = "J01".into(),
            "DuplicateReference",
            json!("J01 and J1"),
        ),
        // A serial line the emitter never compared with its own nets: before,
        // `generate` accepted this and drew J1 pad 1 and U1 pad 101 onto the
        // MCU's VCAP1 net (FEEDBACK, measured); now both refuse it.
        (
            "a5-serial-vcap",
            |s| rename(s, "CONSOLE_TX", "U1_VCAP1"),
            "ReservedName",
            json!("U1_VCAP1"),
        ),
        // §1.8 (amendment 2): a part with no row in the parts table.
        (
            "part-no-row",
            |s| rename(s, "TMP117", "LM75"),
            "UnknownPart",
            json!("U3"),
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

/// `AET-WO-1206`: `validate` refuses each edit `generate` refuses, exit 2,
/// printing byte for byte what `generate` prints, before anything is written.
/// The commit before the change (`58c2438`) held the old behaviour here:
/// `validate` exit 0 on every row, `generate` refusing with this kind and `id`.
#[test]
fn aet_1206_validate_refuses_what_generate_refuses_byte_for_byte() {
    for (label, edit, kind, id) in moved() {
        let (code, generated, created) = generate_raw(&format!("1206-g-{label}"), edit);
        let v: Value = serde_json::from_str(&generated).unwrap();
        assert_eq!(code, Some(2), "{label}: {v}");
        assert_eq!(v["error"]["kind"], kind, "{label}: {v}");
        assert_eq!(v["error"]["id"], id, "{label}: {v}");
        assert!(!created, "{label}: generate created its output directory");
        let (code, validated) = run_file("validate", &format!("1206-v-{label}"), edit);
        assert_eq!(code, Some(2), "{label}: {validated}");
        assert_eq!(validated, generated, "{label}");
    }
}

/// `MissingBusConstraint` keeps `solve`'s `claimants` from every command:
/// before `AET-WO-1206`, `solve`, `emit` and `generate` printed exactly this.
#[test]
fn aet_1206_a_missing_bus_constraint_prints_solves_bytes_from_every_command() {
    let (_, edit, _, _) = moved()
        .into_iter()
        .find(|m| m.0 == "bus-constraint")
        .unwrap();
    let expected = "{\"ok\":false,\"error\":{\"kind\":\"MissingBusConstraint\",\"id\":\"SENSORS\",\"claimants\":[\"SENSORS\"],\"message\":\"I2C bus `SENSORS` has no net constraint on `I2C_SCL`, so it has no rise time to bound its speed\"}}\n";
    for command in ["validate", "solve"] {
        let (code, out) = run_file(command, &format!("1206-bus-{command}"), edit);
        assert_eq!((code, out.as_str()), (Some(2), expected), "{command}");
    }
    let (code, out, _) = generate_raw("1206-bus-generate", edit);
    assert_eq!((code, out.as_str()), (Some(2), expected));
}

/// `AET-WO-1206` §1.5 (`AET-WO-1205` F1): `solve`'s three bus refusals are
/// validation's variants now, and print exactly what `solve` printed before,
/// each pinned from the binary at `8030fa5`, but for the pull-up's floor:
/// `AET-WO-4230` §1.5 (review A8) takes it at the domain's maximum, 3465 mV,
/// so 1022 ohms, where `8030fa5` took the nominal rail's 967.
#[test]
fn aet_1206_one_error_set_keeps_solves_bytes() {
    type Pin = (&'static str, fn(&mut Value), &'static str);
    let pins: [Pin; 3] = [
        (
            "too-fast",
            |s| s["i2c_buses"][0]["speed_hz"] = 1_000_000.into(),
            r#"{"ok":false,"error":{"kind":"BusTooFast","id":"SENSORS","claimants":["SENSORS"],"message":"I2C bus `SENSORS` runs at 1000000 Hz, above its rise-time ceiling of 400000 Hz"}}"#,
        ),
        (
            "pull-up",
            |s| s["i2c_buses"][0]["pull_up_ohms"] = 100.into(),
            r#"{"ok":false,"error":{"kind":"PullUpBelowSinkFloor","id":"SENSORS","claimants":["SENSORS"],"message":"I2C bus `SENSORS` has a pull-up of 100 ohms, below the 1022 ohm floor its rail and mode set (UM10204 sink current)"}}"#,
        ),
        (
            "capacitance",
            |s| {
                for c in s["net_constraints"].as_array_mut().unwrap() {
                    if c["net_name"] == "I2C_SDA" {
                        c["max_capacitance_femtofarads"] = 500_000.into();
                    }
                }
            },
            r#"{"ok":false,"error":{"kind":"BusCapacitanceOverLimit","id":"I2C_SDA","claimants":["I2C_SDA"],"message":"I2C line `I2C_SDA` has a capacitance of 500000 fF, over its bus's limit of 400000 fF (UM10204)"}}"#,
        ),
    ];
    for (label, edit, expected) in pins {
        let (code, out) = run_file("solve", &format!("1206-f1-{label}"), edit);
        assert_eq!((code, out.trim_end()), (Some(2), expected), "{label}");
        // `validate` alone does not solve, so it still passes these, for a
        // specification that acknowledges nothing.
        let (code, out) = run_file("validate", &format!("1206-f1v-{label}"), |s| {
            edit(s);
            s["acknowledge_unwitnessed"] = Value::Array(Vec::new());
        });
        assert_eq!(code, Some(0), "{label}: {out}");
        // `AET-WO-4230` R1: the reference's acknowledgements stand only for a
        // witness ledger, which a topology that does not solve has none of,
        // so `validate` refuses them as stale, by that name, never by the
        // bus refusal `solve` makes.
        let (code, out) = run_file("validate", &format!("1206-f1a-{label}"), edit);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(code, Some(2), "{label}: {out}");
        assert_eq!(
            v["error"]["kind"], "UnwitnessedAcknowledgementStale",
            "{label}: {out}"
        );
    }
}

/// `AET-WO-1206` §1.6: eda's `UnsupportedAddress` is kept. `validate`
/// refuses a strap the part does not offer (`I2cAddressNotOffered`, step
/// 14), so `generate` never reaches it; a library caller of
/// `aethelgard_eda::schematic::emit` handing it a deserialized, unvalidated
/// specification does, so the emitter still refuses rather than draw a strap
/// the part cannot take.
#[test]
fn aet_1206_the_emitters_address_guard_is_reachable_by_a_library_caller() {
    use aethelgard_ir::Specification;
    let mut v: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    let valid: Specification = serde_json::from_value(v.clone()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&valid.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&valid, &table, &"0".repeat(64)).unwrap();
    for (index, part, address) in [(0, "BME280", 0x50), (1, "TMP117", 0x50)] {
        v["i2c_devices"][index]["address"] = address.into();
        let spec: Specification = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(spec.validate().unwrap_err().kind(), "I2cAddressNotOffered");
        let e = aethelgard_eda::schematic::emit(&spec, &solved, &table).unwrap_err();
        assert_eq!(e.kind(), "UnsupportedAddress", "{part}: {e}");
        v = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    }
}
