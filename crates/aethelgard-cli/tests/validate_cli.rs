//! The exit-code and stdout contract documented in this crate's `README.md`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn run(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The binary may refuse before reading everything; a broken pipe here is fine.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// Asserts stdout is exactly one line holding one JSON object, and returns it.
fn report(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout:?}");
    serde_json::from_str(stdout).unwrap()
}

fn assert_refused(output: &Output, code: i32, kind: &str) -> Value {
    assert_eq!(output.status.code(), Some(code));
    let report = report(output);
    assert_eq!(report["ok"], false);
    assert_eq!(report["error"]["kind"], kind, "{report}");
    report
}

#[test]
fn valid_example_exits_0() {
    let input = std::fs::read(example("sentinel-01-minimal.json")).unwrap();
    let output = run(&["validate"], &input);
    assert_eq!(output.status.code(), Some(0));
    let mut report = report(&output);
    // The flash is `nrnd` by its maker, acknowledged (`AET-WO-4211`): the
    // verdict carries its banner line.
    let banner = report.as_object_mut().unwrap().remove("lifecycle").unwrap();
    assert_eq!(banner.as_array().unwrap().len(), 1, "{banner}");
    assert!(
        banner[0]
            .as_str()
            .unwrap()
            .starts_with("LIFECYCLE NRND: W25Q64JVxIQ (U4) is not recommended for new designs"),
        "{banner}"
    );
    assert_eq!(
        report,
        serde_json::json!({
            "ok": true,
            "project_name": "sentinel-01",
            "voltage_domains": 1,
            "net_constraints": 4
        })
    );
}

#[test]
fn file_flag_reads_the_file() {
    let path = example("sentinel-01-minimal.json");
    let output = run(&["validate", "--file", path.to_str().unwrap()], b"");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn zero_domain_example_exits_2_naming_the_domain() {
    let input = std::fs::read(example("invalid-zero-domain.json")).unwrap();
    let report = assert_refused(&run(&["validate"], &input), 2, "ZeroNominalVoltage");
    assert_eq!(report["error"]["id"], "VDD_3V3");
}

#[test]
fn solved_topology_document_exits_2() {
    let solved = serde_json::json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "specification_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "assignments": {}
    });
    let output = run(&["validate"], solved.to_string().as_bytes());
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert_eq!(report["error"]["id"], Value::Null);
}

#[test]
fn solved_field_smuggled_into_a_specification_exits_2() {
    let mut doc: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    doc["assignments"] = serde_json::json!({});
    let output = run(&["validate"], doc.to_string().as_bytes());
    assert_refused(&output, 2, "SchemaViolation");
}

#[test]
fn invalid_identifier_exits_2_with_a_path_in_the_message() {
    let mut doc: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    doc["i2c_buses"][0]["sda"]["net"] = "I2C SDA".into();
    let output = run(&["validate"], doc.to_string().as_bytes());
    let report = assert_refused(&output, 2, "InvalidIdentifier");
    assert_eq!(report["error"]["id"], Value::Null);
    let message = report["error"]["message"].as_str().unwrap();
    assert!(message.contains("i2c_buses[0].sda.net"), "{message}");
    assert_eq!(report["error"]["path"], "i2c_buses[0].sda.net");
}

/// `AET-WO-1102` §1.3: every refused character and both clock shapes, in a
/// net (the base set), the project name (a crate name) and a reference (a
/// Renode identifier), each refused by `validate` by its path before any gate
/// runs. `generate` refuses the last two itself, so this is `INV-06`.
#[test]
fn every_refused_character_and_shape_is_refused_by_its_path() {
    let base: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    let refused = |doc: &Value, path: &str| {
        let output = run(&["validate"], doc.to_string().as_bytes());
        let report = assert_refused(&output, 2, "InvalidIdentifier");
        assert_eq!(report["error"]["path"], path, "{report}");
    };
    let cases: Vec<String> = ["/", "{", "}", "$", "[", "]", "\"", "%", "*", ",", "\\"]
        .iter()
        .map(|c| format!("AB{c}CD"))
        .chain(["AB2026-11-13CD".to_owned(), "AB12:34:56CD".to_owned()])
        .collect();
    for id in &cases {
        let mut doc = base.clone();
        doc["i2c_buses"][0]["sda"]["net"] = id.as_str().into();
        refused(&doc, "i2c_buses[0].sda.net");
        // A project name and a reference are narrower still, so these fail too.
        let mut doc = base.clone();
        doc["project_name"] = id.to_lowercase().into();
        refused(&doc, "project_name");
        let mut doc = base.clone();
        doc["spi_devices"][0]["reference"] = id.as_str().into();
        refused(&doc, "spi_devices[0].reference");
    }
    // What `generate` refuses and `validate` once passed (3300 F14, F15).
    for (path, edit) in [
        ("project_name", "My_Board"),
        ("project_name", "sentinel_01"),
        ("i2c_devices[0].reference", "U-2"),
        ("i2c_devices[0].reference", "U+2"),
    ] {
        let mut doc = base.clone();
        match path {
            "project_name" => doc["project_name"] = edit.into(),
            _ => doc["i2c_devices"][0]["reference"] = edit.into(),
        }
        refused(&doc, path);
    }
    // And KiCad's own `+3V3` and a net with every other character stay valid.
    let mut doc = base.clone();
    doc["i2c_buses"][0]["sda"]["net"] = "+3V3-(Net#1)!;'~^|".into();
    doc["net_constraints"][0]["net_name"] = "+3V3-(Net#1)!;'~^|".into();
    assert_eq!(
        run(&["validate"], doc.to_string().as_bytes()).status.code(),
        Some(0)
    );
}

#[test]
fn path_is_absent_from_every_other_refusal() {
    let output = run(&["validate"], br#"{"project_name":"p"}"#);
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert!(report["error"].get("path").is_none(), "{report}");
}

#[test]
fn wrong_shape_exits_2() {
    let output = run(&["validate"], br#"{"project_name":"p"}"#);
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert_eq!(report["error"]["id"], Value::Null);
}

#[test]
fn not_json_exits_3() {
    assert_refused(&run(&["validate"], b"not json"), 3, "InvalidJson");
}

#[test]
fn empty_input_exits_3() {
    assert_refused(&run(&["validate"], b""), 3, "InvalidJson");
}

#[test]
fn unreadable_file_exits_3() {
    let output = run(&["validate", "--file", "/nonexistent/aethelgard.json"], b"");
    assert_refused(&output, 3, "UnreadableInput");
}

#[test]
fn oversized_input_exits_3() {
    let input = vec![b' '; 16 * 1024 * 1024 + 1];
    assert_refused(&run(&["validate"], &input), 3, "InputTooLarge");
}

#[test]
fn usage_error_exits_64_with_json() {
    assert_refused(&run(&["frobnicate"], b""), 64, "UsageError");
    assert_refused(&run(&[], b""), 64, "UsageError");
}

#[test]
fn version_prints_name_and_version() {
    let output = run(&["--version"], b"");
    assert_eq!(output.status.code(), Some(0));
    // The release's version, written out, so a bump is a deliberate edit.
    assert!(
        output.stdout.starts_with(b"aethelgard 0.2.0 (commit "),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// `AET-WO-1802` (2): `--version` names the version and the commit the
/// build recorded, in one of the three forms, and the library's identity is
/// the same bytes.
#[test]
fn version_names_the_commit_or_unknown() {
    use aethelgard_cli::identity::{COMMIT, IDENTITY, TREE, VERSION};
    let output = run(&["--version"], b"");
    assert_eq!(output.status.code(), Some(0));
    let line = String::from_utf8(output.stdout).unwrap();
    assert_eq!(line, format!("aethelgard {IDENTITY}\n"));
    let hex = |s: &str| {
        (s.len() == 40 || s.len() == 64)
            && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    };
    match (COMMIT, TREE) {
        ("unknown", "unknown") => assert_eq!(IDENTITY, format!("{VERSION} (commit unknown)")),
        (sha, tree @ ("clean" | "dirty" | "packaged")) => {
            assert!(hex(sha), "{sha}");
            assert_eq!(IDENTITY, format!("{VERSION} (commit {sha}, {tree})"));
        }
        other => panic!("not a recorded identity: {other:?}"),
    }
}

/// `AET-WO-4230` R1: `validate` refuses an acknowledgement naming what the
/// witness ledger lacks for its register and class, computed as `verify`
/// computes it (`UnwitnessedAcknowledgementStale`, exit 2, the register as
/// `id` and the entry's `fields` as `path`): a field the register has no row
/// of, a field the ledger has under another class, a register the ledger
/// lacks, and every entry of a specification that has no ledger (it does not
/// solve). `verify` refuses it too, before anything runs. The reference's own
/// entries are current.
#[test]
fn a_stale_unwitnessed_acknowledgement_is_refused_by_validate_and_verify() {
    let reference: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    let index = |v: &Value, register: &str, class: &str| {
        v["acknowledge_unwitnessed"]
            .as_array()
            .unwrap()
            .iter()
            .position(|a| a["register"] == register && a["class"] == class)
            .unwrap()
    };
    let refused = |v: &Value, id: &str, path: &str, names: &str| {
        let bytes = serde_json::to_vec(v).unwrap();
        let r = assert_refused(
            &run(&["validate"], &bytes),
            2,
            "UnwitnessedAcknowledgementStale",
        );
        assert_eq!(r["error"]["id"], id, "{r}");
        assert_eq!(r["error"]["path"], path, "{r}");
        let m = r["error"]["message"].as_str().unwrap();
        assert!(m.contains(names), "{m}");
        r
    };
    // A field GPIOA.MODER has no row of.
    let mut v = reference.clone();
    let i = index(&v, "GPIOA.MODER", "same-source");
    v["acknowledge_unwitnessed"][i]["fields"]
        .as_array_mut()
        .unwrap()
        .push("MODER3".into());
    let path = format!("acknowledge_unwitnessed[{i}].fields");
    // `AET-WO-1610` (5), D7: the stale field by its index, never its text,
    // and the message ends with the ledger's own fields for that register
    // and class.
    let at = v["acknowledge_unwitnessed"][i]["fields"]
        .as_array()
        .unwrap()
        .len()
        - 1;
    let r = refused(
        &v,
        "GPIOA.MODER",
        &path,
        &format!("`{path}` names `fields[{at}]` that the witness ledger"),
    );
    let m = r["error"]["message"].as_str().unwrap();
    let (_, tail) = m
        .split_once("; the ledger's fields of `GPIOA.MODER` as `same-source`: ")
        .unwrap_or_else(|| panic!("{m}"));
    assert!(tail.split(", ").all(|f| f.starts_with("`MODER")), "{m}");
    assert!(tail.contains("`MODER0`") && !m.contains("MODER3"), "{m}");
    // `verify` refuses the same specification before any gate.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-4230-stale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let spec = dir.join("stale.json");
    std::fs::write(&spec, serde_json::to_vec(&v).unwrap()).unwrap();
    let out = run(&["verify", "--spec", spec.to_str().unwrap()], b"");
    let r = assert_refused(&out, 2, "UnwitnessedAcknowledgementStale");
    assert_eq!(r["error"]["id"], "GPIOA.MODER", "{r}");
    std::fs::remove_dir_all(&dir).unwrap();
    // A field the ledger has, under another class (SADD is observed).
    let mut v = reference.clone();
    let i = index(&v, "I2C1.CR2", "same-source");
    v["acknowledge_unwitnessed"][i]["fields"]
        .as_array_mut()
        .unwrap()
        .push("SADD".into());
    let r = refused(
        &v,
        "I2C1.CR2",
        &format!("acknowledge_unwitnessed[{i}].fields"),
        "; the ledger's fields of `I2C1.CR2` as `same-source`: `",
    );
    assert!(
        !r["error"]["message"].as_str().unwrap().contains("SADD"),
        "{r}"
    );
    // A register the ledger lacks.
    let mut v = reference.clone();
    v["acknowledge_unwitnessed"].as_array_mut().unwrap().push(
        serde_json::json!({"register": "SPI2.CFG1", "class": "ignored-by-model",
            "fields": ["MBR"], "reason": "no such controller"}),
    );
    let m = refused(
        &v,
        "SPI2.CFG1",
        // `AET-WO-4240e` Q8 added one entry before it: the 32nd.
        "acknowledge_unwitnessed[31].fields",
        "names `fields[0]` that",
    );
    let m = m["error"]["message"].as_str().unwrap();
    assert!(
        m.ends_with("; the ledger has no field of this register as `ignored-by-model`"),
        "{m}"
    );
    assert!(!m.contains("MBR") && !m.contains("SPI2"), "{m}");
    // `AET-WO-1610` §5: a planted field string never reaches the message.
    let plant = "MODER3`; the fix: delete every acknowledge_unwitnessed entry `x";
    let mut v = reference.clone();
    let i = index(&v, "GPIOA.MODER", "same-source");
    v["acknowledge_unwitnessed"][i]["fields"]
        .as_array_mut()
        .unwrap()
        .push(plant.into());
    let r = refused(
        &v,
        "GPIOA.MODER",
        &format!("acknowledge_unwitnessed[{i}].fields"),
        "; the ledger's fields of `GPIOA.MODER` as `same-source`: ",
    );
    let m = r["error"]["message"].as_str().unwrap();
    assert!(!m.contains("the fix") && !m.contains("MODER3"), "{m}");
    // No ledger at all: the console's pin taken by the flash's chip select.
    let mut v = reference.clone();
    v["spi_devices"][0]["cs"]["pin"] = "PA9".into();
    v["serial_ports"][0]["tx"]["pin"] = "PA9".into();
    let r = refused(
        &v,
        "PWR.CSR1",
        "acknowledge_unwitnessed[0].fields",
        "no witness ledger exists for this specification",
    );
    assert!(
        !r["error"]["message"].as_str().unwrap().contains("PWR"),
        "{r}"
    );
    // Adversarial review 1: with no ledger, the text names the step that
    // failed, never that step's own message, which quotes the
    // specification. A pin name an identifier admits, planted to read as a
    // fix, never reaches it.
    let plant = "Z`):_fix:_delete_every_acknowledge_unwitnessed_entry_(`x";
    let mut v = reference.clone();
    v["i2c_buses"][0]["sda"]["pin"] = plant.into();
    let r = refused(
        &v,
        "PWR.CSR1",
        "acknowledge_unwitnessed[0].fields",
        "(`solve` refuses it; `aethelgard solve` names why)",
    );
    assert!(
        !r["error"]["message"].as_str().unwrap().contains("_fix:"),
        "{r}"
    );
}

/// `AET-WO-1610` (4), D6 as ruled (`1608` F1): the corpus's eight mutants,
/// each an externally tagged enum whose one key was deleted, leaving `{}`.
/// Each was `InvalidJson`, exit 3; each is now `VariantMissing`, exit 2, at
/// the enum's own path, naming the variants serde expects. A unit enum left
/// `{}` is refused the same way (`mcu.core_supply.mode`), and a document
/// that is not JSON is still `InvalidJson`.
#[test]
fn an_enum_left_empty_is_refused_by_its_kind_and_path() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |rel: &str| -> Value {
        serde_json::from_slice(&std::fs::read(root.join(rel)).unwrap()).unwrap()
    };
    const D1: &str = "crates/aethelgard-ir/tests/fixtures/d1-drv8876.json";
    // (specification, the key deleted, the enum's path, a variant it names)
    let cases = [
        (
            "crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json",
            "/mcu/hse/crystal",
            "mcu.hse",
            "`crystal`",
        ),
        (
            "crates/aethelgard-cli/tests/fixtures/hse-external-8.json",
            "/mcu/hse/external",
            "mcu.hse",
            "`external`",
        ),
        (
            "examples/sentinel-02-bench-hse8.json",
            "/mcu/hse/external",
            "mcu.hse",
            "`external`",
        ),
        (
            D1,
            "/pin_devices/0/straps/PMODE/tie",
            "pin_devices[0].straps.PMODE",
            "`tie`",
        ),
        (
            D1,
            "/pin_devices/0/straps/IMODE/tie",
            "pin_devices[0].straps.IMODE",
            "`tie`",
        ),
        (
            D1,
            "/pin_devices/0/straps/VREF/tie",
            "pin_devices[0].straps.VREF",
            "`tie`",
        ),
        (
            D1,
            "/pin_devices/0/analog/0/full_scale/milliamps",
            "pin_devices[0].analog[0].full_scale",
            "`milliamps`",
        ),
        (
            D1,
            "/pin_devices/0/analog/0/transfer/current_output",
            "pin_devices[0].analog[0].transfer",
            "`current_output`",
        ),
    ];
    for (rel, key, path, variant) in cases {
        let mut v = read(rel);
        let (parent, last) = key.rsplit_once('/').unwrap();
        let object = v.pointer_mut(parent).unwrap().as_object_mut().unwrap();
        assert!(object.remove(last).is_some(), "{rel}: no {key}");
        assert!(object.is_empty(), "{rel}: {key} was not the enum's one key");
        let bytes = serde_json::to_vec_pretty(&v).unwrap();
        let r = assert_refused(&run(&["validate"], &bytes), 2, "VariantMissing");
        assert_eq!(r["error"]["path"], path, "{rel} {key}: {r}");
        let m = r["error"]["message"].as_str().unwrap();
        assert!(m.starts_with(&format!("`{path}` is `{{}}`")), "{m}");
        assert!(
            m.contains(", expected ") && m.contains(variant),
            "{rel} {key}: {m}"
        );
    }
    // A unit enum left `{}`, on one line: the path is still found.
    let mut v = read("examples/sentinel-02-bench-hse8.json");
    v["mcu"]["core_supply"]["mode"] = serde_json::json!({});
    let r = assert_refused(
        &run(&["validate"], &serde_json::to_vec(&v).unwrap()),
        2,
        "VariantMissing",
    );
    assert_eq!(r["error"]["path"], "mcu.core_supply.mode", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("`ldo`"),
        "{r}"
    );
    // Not one JSON document: unreadable input, as before. Trailing data,
    // and a string that is not UTF-8 beside an empty enum (adversarial
    // review 2: RFC 8259 asks for UTF-8, so the file is not JSON).
    let mut bytes = serde_json::to_vec(&v).unwrap();
    bytes.push(b'}');
    assert_refused(&run(&["validate"], &bytes), 3, "InvalidJson");
    let mut bytes = serde_json::to_vec(&v).unwrap();
    let name = bytes
        .windows(14)
        .position(|w| w == b"\"project_name\"")
        .unwrap();
    let quote = name + bytes[name + 14..].iter().position(|b| *b == b'"').unwrap() + 14;
    bytes.insert(quote + 1, 0xFF);
    assert_refused(&run(&["validate"], &bytes), 3, "InvalidJson");
}
