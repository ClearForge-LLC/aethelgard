//! `AET-WO-4412` D1 (a), ruled 2026-11-26: a specification's
//! `mcu.foreign_clock` is ArduPilot's tree for the declared oscillator, and a
//! generated run drops it.
//!
//! - `validate` refuses a declaration that is not ArduPilot's tree for
//!   `mcu.hse`'s frequency (`ForeignClockNotArduPilots`), and an oscillator
//!   ArduPilot's `stm32h7_mcuconf.h` has no tree for
//!   (`ForeignClockOscillatorUnsupported`).
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

/// Sentinel-03's 16 MHz crystal: its own row validates; the HSI's values,
/// which `4400h` once copied, do not, and the refusal names the field and
/// both figures; another scale is refused too. An external oscillator at
/// 12 MHz has no tree, and one at 8 MHz has one.
#[test]
fn a_foreign_clock_that_is_not_ardupilots_tree_for_the_oscillator_is_refused() {
    let dir = common::private_tmp("ap-clock");
    let mut s03 = read(&common::example("sentinel-03.json"));
    s03["mcu"]["foreign_clock"] = declared(16_000_000);
    let p = write(&dir.0, "s03.json", &s03);
    let (code, v) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
    assert_eq!((code, &v["ok"]), (Some(0), &Value::Bool(true)), "{v}");

    s03["mcu"]["foreign_clock"] = declared(0);
    let p = write(&dir.0, "s03-hsi.json", &s03);
    let (code, v) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "ForeignClockNotArduPilots", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("usarts.UART4 declares 72000000 Hz")
            && m.contains("16000000 Hz oscillator")
            && m.contains("at 100000000 Hz")
            && m.contains("L132-138"),
        "{m}"
    );

    s03["mcu"]["foreign_clock"] = declared(16_000_000);
    s03["mcu"]["foreign_clock"]["vos"]["value"] = "scale2".into();
    let p = write(&dir.0, "s03-vos.json", &s03);
    let (code, v) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "ForeignClockNotArduPilots", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mcu.foreign_clock.vos"),
        "{v}"
    );

    let mut ext = read(&fixture("hse-external-8.json"));
    ext["mcu"]["foreign_clock"] = declared(8_000_000);
    let p = write(&dir.0, "ext8.json", &ext);
    let (code, v) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{v}");
    ext["mcu"]["hse"]["external"]["hz_num"] = 12_000_000.into();
    let p = write(&dir.0, "ext12.json", &ext);
    let (code, v) = aethelgard(&dir.0, &["validate", "--file", p.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(
        v["error"]["kind"], "ForeignClockOscillatorUnsupported",
        "{v}"
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
