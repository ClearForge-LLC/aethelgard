//! `AET-WO-4412` D1, ruled 2026-11-26: an ArduPilot firmware's
//! `mcu.foreign_clock` is ArduPilot's tree for the declared oscillator, and a
//! generated run drops the declaration.
//!
//! - `simulate --foreign` refuses, for an ELF that names `AP_HAL::panic`, a
//!   declaration that is not ArduPilot's tree for `mcu.hse`'s frequency
//!   (`ForeignClockNotArduPilots`), and an oscillator ArduPilot's
//!   `stm32h7_mcuconf.h` has no tree for (`ForeignClockOscillatorUnsupported`);
//!   `validate` holds it to its own consistency only (D1 (b), ruled
//!   2026-11-26T11:26Z).
//! - `generate` writes the same tree with the declaration and without it, but
//!   for the specification's hash, plus the foreign platform beside the
//!   generated one (`<project>.foreign.repl`).

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn aethelgard(dir: &Path, args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .env("TMPDIR", dir)
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    (out.status.code(), v)
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn write(dir: &Path, name: &str, v: &Value) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    p
}

/// ArduPilot's tree for `oscillator_hz` as a declaration.
fn declared(oscillator_hz: u64) -> Value {
    let t = aethelgard_ir::ardupilot_clock::tree(oscillator_hz).unwrap();
    let hz = |hz: u64| serde_json::json!({"value": hz, "source": "Copter-4.7.2 stm32h7_mcuconf.h (aethelgard-ir data/ardupilot-h7-clock.tsv)"});
    let usarts: serde_json::Map<String, Value> = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), hz(t.usarts_hz)))
        .collect();
    serde_json::json!({
        "apb1_timers_hz": hz(t.timers_hz),
        "apb2_timers_hz": hz(t.timers_hz),
        "usarts": usarts,
        "vos": {"value": "scale1", "source": "Copter-4.7.2 stm32h7_mcuconf.h STM32_VOS_SCALE1 (L109)"},
    })
}

/// `hold` on `spec`'s solved topology: what `simulate --foreign` asks of an
/// ArduPilot ELF.
fn held(spec: &Value) -> Result<(), aethelgard_ir::IrValidationError> {
    let s: aethelgard_ir::Specification = serde_json::from_value(spec.clone()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&s.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&s, &table, &"0".repeat(64)).unwrap();
    aethelgard_ir::ardupilot_clock::hold(
        t.foreign_clock.as_ref().unwrap(),
        t.clock.hse.as_ref().map(|h| &h.declared),
    )
}

/// D1 (b), ruled 2026-11-26T11:26Z: `validate` holds a declared foreign
/// clock to its own consistency only, so each variant below validates; the
/// tree is held at the run, for an ELF that names `AP_HAL::panic`
/// (`boot_tier1`'s planted pair). Sentinel-03's 16 MHz crystal holds its own
/// row; the HSI's values, which `4400h` once copied, are refused, naming the
/// field and both figures; so is another scale. An external oscillator at
/// 12 MHz has no row, and one at 8 MHz has one.
#[test]
fn validate_holds_consistency_and_the_run_holds_ardupilots_tree() {
    let dir = common::private_tmp("ap-clock");
    let validates = |name: &str, v: &Value| {
        let p = write(&dir.0, name, v);
        let (code, r) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
        assert_eq!(
            (code, &r["ok"]),
            (Some(0), &Value::Bool(true)),
            "{name}: {r}"
        );
    };
    let mut s03 = read(&common::example("sentinel-03.json"));
    s03["mcu"]["foreign_clock"] = declared(16_000_000);
    validates("s03.json", &s03);
    held(&s03).unwrap();

    s03["mcu"]["foreign_clock"] = declared(0);
    validates("s03-hsi.json", &s03);
    let e = held(&s03).unwrap_err();
    assert_eq!(e.kind(), "ForeignClockNotArduPilots");
    let m = e.to_string();
    assert!(
        m.contains("usarts.UART4 declares 72000000 Hz")
            && m.contains("16000000 Hz oscillator")
            && m.contains("at 100000000 Hz")
            && m.contains("L132-138"),
        "{m}"
    );

    s03["mcu"]["foreign_clock"] = declared(16_000_000);
    s03["mcu"]["foreign_clock"]["vos"]["value"] = "scale2".into();
    validates("s03-vos.json", &s03);
    let e = held(&s03).unwrap_err();
    assert_eq!(e.kind(), "ForeignClockNotArduPilots");
    assert!(e.to_string().contains("mcu.foreign_clock.vos"), "{e}");

    let mut ext = read(&fixture("hse-external-8.json"));
    ext["mcu"]["foreign_clock"] = declared(8_000_000);
    validates("ext8.json", &ext);
    held(&ext).unwrap();
    ext["mcu"]["hse"]["external"]["hz_num"] = 12_000_000.into();
    validates("ext12.json", &ext);
    assert_eq!(
        held(&ext).unwrap_err().kind(),
        "ForeignClockOscillatorUnsupported"
    );
}

/// Every file `generate` wrote under `root`, by path, with the
/// specification's hash masked.
fn tree(root: &Path, sha: &str) -> BTreeMap<String, String> {
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files
        .into_iter()
        .map(|p| {
            let text = String::from_utf8(std::fs::read(&p).unwrap()).unwrap();
            (
                p.strip_prefix(root).unwrap().display().to_string(),
                text.replace(sha, "<spec>"),
            )
        })
        .collect()
}

/// `sentinel-02-own` with its declaration and without it: `generate` writes
/// the same files, byte for byte but the specification's hash, and with it
/// adds the foreign platform, which is not the generated one.
#[test]
fn a_generated_tree_is_the_same_with_or_without_a_foreign_clock() {
    let dir = common::private_tmp("ap-gen");
    let own = read(&fixture("sentinel-02-own.json"));
    let mut plain = own.clone();
    plain["mcu"]
        .as_object_mut()
        .unwrap()
        .remove("foreign_clock")
        .unwrap();
    let mut shas = Vec::new();
    for (name, v) in [("declared", &own), ("plain", &plain)] {
        let spec = write(&dir.0, &format!("{name}.json"), v);
        let out = dir.0.join(format!("gen-{name}"));
        let (code, r) = aethelgard(
            &dir.0,
            &[
                "generate",
                "--spec",
                spec.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ],
        );
        assert_eq!(code, Some(0), "{r}");
        shas.push(r["specification_sha256"].as_str().unwrap().to_owned());
    }
    let mut declared = tree(&dir.0.join("gen-declared"), &shas[0]);
    let plain = tree(&dir.0.join("gen-plain"), &shas[1]);
    let foreign = declared
        .remove("board/sentinel-02-own.foreign.repl")
        .expect("the foreign platform beside the generated one");
    assert_eq!(declared, plain);
    let generated = &plain["board/sentinel-02-own.repl"];
    assert_ne!(&foreign, generated);
    assert!(
        foreign.contains("mcu.foreign_clock") && !generated.contains("mcu.foreign_clock"),
        "the foreign platform builds the declared clock; the generated one the solved"
    );
}
