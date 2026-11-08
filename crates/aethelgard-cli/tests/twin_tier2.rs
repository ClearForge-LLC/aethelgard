//! `AET-WO-2100` §1.4, tier 2: the twin boots, inside the tools image.
//!
//! - Renode's installed platform files are the ones `aethelgard-twin` pinned,
//!   and its stock table is what they declare.
//! - The emitted platform loads, and every solved peripheral is listed.
//! - The generated reference firmware builds offline for the part and boots to
//!   the boot word, with all three witnesses (ruled on PR #56, R2).
//! - Two boots give the same observation.
//! - A boot too short to reach the fixed point fails.
//!
//! (Two emissions being byte-identical is tier 1: `aethelgard-twin`'s unit
//! tests and the regeneration guard in `aethelgard-svd-hal`.) Each boot writes
//! an `ANSWER 2100` line to `/dev/stderr`.

#![cfg(feature = "with-tools")]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;

use aethelgard_ir::SolvedTopology;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec() -> PathBuf {
    root().join("examples/sentinel-01-minimal.json")
}

fn platform() -> PathBuf {
    root().join("boards/sentinel-01/sentinel-01.repl")
}

fn answer(line: &str) {
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(format!("ANSWER 2100: {line}\n").as_bytes())
        .unwrap();
}

fn aethelgard(args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{e}: {stdout} / {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.code(), value)
}

fn topology() -> SolvedTopology {
    let (code, v) = aethelgard(&["solve", "--file", spec().to_str().unwrap()]);
    assert_eq!(code, Some(0), "{v}");
    serde_json::from_value(v).unwrap()
}

/// The reference firmware, built once, offline and locked, by the image's
/// toolchain. Its `.cargo/config.toml` puts the build in the top-level
/// `target/`, never under `boards/`.
fn firmware() -> &'static Path {
    static ELF: OnceLock<PathBuf> = OnceLock::new();
    ELF.get_or_init(|| {
        let dir = root().join("boards/sentinel-01/firmware");
        let start = Instant::now();
        let out = Command::new(env!("CARGO"))
            .args(["build", "--release", "--offline", "--locked"])
            .current_dir(&dir)
            .env_remove("CARGO_TARGET_DIR")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let elf = root().join(
            "target/sentinel-01-board-firmware/thumbv7em-none-eabihf/release/sentinel-01-board-firmware",
        );
        answer(&format!(
            "firmware build wall_s={:.2} bytes={}",
            start.elapsed().as_secs_f64(),
            fs::metadata(&elf).unwrap().len()
        ));
        elf
    })
}

fn boot(label: &str, time: &str) -> (Option<i32>, Value) {
    let start = Instant::now();
    let (code, v) = aethelgard(&[
        "simulate",
        "--platform",
        platform().to_str().unwrap(),
        "--firmware",
        firmware().to_str().unwrap(),
        "--spec",
        spec().to_str().unwrap(),
        "--for",
        time,
    ]);
    answer(&format!(
        "{label}: exit={code:?} wall_s={:.2} verdict={v}",
        start.elapsed().as_secs_f64()
    ));
    (code, v)
}

#[test]
fn renode_s_files_are_the_pinned_ones_and_declare_the_stock_table() {
    let renode = std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("renode"))
                .find(|f| f.is_file())
        })
        .expect("renode on PATH");
    let tree = fs::canonicalize(renode).unwrap();
    let tree = tree.parent().unwrap();
    for (file, sha) in aethelgard_twin::platform::RENODE_FILES {
        let bytes = fs::read(tree.join(file)).unwrap();
        let got: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(got, sha, "{file}");
    }
    let text = fs::read_to_string(tree.join(aethelgard_twin::platform::STOCK_FILE)).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    for (_, name, model, address) in aethelgard_twin::platform::STOCK {
        let head = format!("{name}: {model} @ ");
        let at = lines
            .iter()
            .position(|l| l.starts_with(&head))
            .unwrap_or_else(|| panic!("{name} is not declared as {model}"));
        let sysbus = lines[at..at + 3]
            .iter()
            .find_map(|l| {
                l.split_once("sysbus ")
                    .map(|(_, r)| r.trim_start_matches('<'))
            })
            .unwrap_or_else(|| panic!("{name} has no sysbus address"));
        let hex: String = sysbus
            .trim_start_matches("0x")
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        assert_eq!(u32::from_str_radix(&hex, 16).unwrap(), address, "{name}");
    }
}

#[test]
fn the_emitted_platform_loads_and_lists_every_solved_peripheral() {
    let (code, v) = aethelgard(&["simulate", "--platform", platform().to_str().unwrap()]);
    answer(&format!("platform: exit={code:?} verdict={v}"));
    assert_eq!(code, Some(0), "{v}");
    let listed: Vec<(&str, &str)> = v["machine"]["peripherals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap()))
        .collect();
    let solved = aethelgard_twin::platform::instances(&topology()).unwrap();
    assert!(!solved.is_empty());
    for i in solved {
        assert!(
            listed.contains(&(i.name.as_str(), i.model.as_str())),
            "{} ({}) is not listed",
            i.name,
            i.model
        );
    }
}

#[test]
fn the_reference_firmware_boots_and_two_boots_agree() {
    let (code, first) = boot("boot 1", "10ms");
    assert_eq!(code, Some(0), "{first}");
    assert_eq!(first["verdict"], "pass");
    let b = &first["boot"];
    assert_eq!(b["boot_word"]["observed"], "0x2100b007");
    assert_eq!(b["instructions"]["observed"], 1_000_000);
    assert_eq!(b["seed"], 2100);
    for r in b["registers"].as_array().unwrap() {
        assert_eq!(r["observed"], r["expected"], "{r}");
    }
    let (code, second) = boot("boot 2", "10ms");
    assert_eq!(code, Some(0), "{second}");
    assert_eq!(first["boot"], second["boot"]);
    assert_eq!(first["machine"], second["machine"]);
}

#[test]
fn a_boot_too_short_to_reach_the_fixed_point_fails() {
    // The spike measured the fixed point between 1,000 and 2,000 instructions.
    let (code, v) = boot("short", "10us");
    assert_eq!(code, Some(1), "{v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"boot_word_missing"), "{v}");
    assert!(!kinds.contains(&"cpu_halted"), "{v}");
    assert_eq!(v["boot"]["instructions"]["observed"], 1000);
}
