//! `AET-WO-2100` §1.4, tier 2: the twin boots, inside the tools image.
//!
//! - Renode's installed platform files are the ones `aethelgard-twin` pinned,
//!   and its stock table is what they declare.
//! - The emitted platform loads, and every solved peripheral is listed.
//! - The generated reference firmware builds offline for the part and boots to
//!   the boot word, with all three witnesses (ruled on PR #56, R2).
//! - Two boots give the same observation.
//! - A boot too short to reach the fixed point fails.
//! - Two firmwares from the adversarial pass fail: one linked with no vector
//!   table where the part boots (A3), one that leaves its own code after the
//!   boot word (A7).
//! - A firmware that aborts Renode's machine returns inside the wall bound
//!   plus 5 s, as a `cpu_aborted` fail (A6; ruled on PR #56, B1).
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

/// A copy of the reference firmware with `edit` applied, built like it into
/// its own target directory outside the repository.
fn mutant(name: &str, change: impl Fn(&Path)) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-2100-t2-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let src = root().join("boards/sentinel-01/firmware");
    for rel in [
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        "link.x",
        "src/main.rs",
    ] {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(src.join(rel), &to).unwrap();
    }
    let board = fs::canonicalize(root().join("boards/sentinel-01")).unwrap();
    edit(
        &dir,
        "Cargo.toml",
        "path = \"..\"",
        &format!("path = \"{}\"", board.display()),
    );
    edit(
        &dir,
        ".cargo/config.toml",
        "target-dir = \"../../../target/sentinel-01-board-firmware\"",
        &format!("target-dir = \"{}\"", dir.join("target").display()),
    );
    change(&dir);
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
    dir.join("target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware")
}

fn edit(dir: &Path, rel: &str, from: &str, to: &str) {
    let path = dir.join(rel);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{rel} lacks {from}");
    fs::write(&path, text.replace(from, to)).unwrap();
}

fn kinds(v: &Value) -> Vec<&str> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect()
}

/// One boot of `elf` on the committed platform, judged by the reference spec.
fn boot_elf(label: &str, elf: &Path, time: &str) -> (Option<i32>, Value) {
    let start = Instant::now();
    let (code, v) = aethelgard(&[
        "simulate",
        "--platform",
        platform().to_str().unwrap(),
        "--firmware",
        elf.to_str().unwrap(),
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

fn boot(label: &str, time: &str) -> (Option<i32>, Value) {
    boot_elf(label, firmware(), time)
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

#[test]
fn a_firmware_with_no_vector_table_at_the_boot_address_fails() {
    // Adversarial A3: Renode would guess the table from the ELF; the part
    // boots from FLASH_BANK1_BASE.
    let elf = mutant("flash-offset", |d| {
        edit(
            d,
            "link.x",
            "ORIGIN = 0x08000000, LENGTH = 0x100000",
            "ORIGIN = 0x08010000, LENGTH = 0xF0000",
        );
    });
    let (code, v) = boot_elf("A3 flash offset", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    let kinds = kinds(&v);
    assert!(
        kinds.contains(&"boot_word_missing") && kinds.contains(&"cpu_halted"),
        "{v}"
    );
}

#[test]
fn a_core_that_leaves_the_firmware_after_the_boot_word_fails() {
    // Adversarial A7: a jump into the FMC bank, plain zero-filled memory.
    let elf = mutant("leave", |d| {
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires",
            "    unsafe { core::arch::asm!(\"ldr r0, =0x70000001\", \"bx r0\", options(noreturn)) };\n    // No sleep: the judge requires",
        );
    });
    let (code, v) = boot_elf("A7 leave", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["pc_outside_firmware"], "{v}");
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007");
}

#[test]
fn an_aborted_machine_returns_inside_the_bound_as_cpu_aborted() {
    // Adversarial A6: a jump into a hole after the word. Renode aborts the
    // machine and `RunFor` never returns; before the bound, the call hung
    // with Renode's memory growing about 55 MB/s.
    let elf = mutant("hole", |d| {
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires",
            "    unsafe { core::arch::asm!(\"ldr r0, =0x0A000001\", \"bx r0\", options(noreturn)) };\n    // No sleep: the judge requires",
        );
    });
    let start = Instant::now();
    let (code, v) = boot_elf("A6 hole", &elf, "10ms");
    let took = start.elapsed();
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["cpu_aborted"], "{v}");
    assert!(took.as_secs() < 30 + 5, "{took:?}");
}

/// `AET-WO-2200` §3, the P2 gate's clause 2, analog half, as one test: the
/// reference specification at 2.2 kΩ passes and at 47 kΩ fails, each run
/// twice with byte-identical `analog` and `faults` arrays. The ns values are
/// `AET-WO-2101` S2's, measured with libngspice 44.2 in this image.
#[test]
fn p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes() {
    let spec_text = fs::read_to_string(spec()).unwrap();
    assert_eq!(spec_text.matches("\"pull_up_ohms\": 2200").count(), 1);
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-2200-t2-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for (ohms, rise, tolerance, faulted) in
        [(2200, 186.40, 0.05, false), (47_000, 3982.30, 0.5, true)]
    {
        let spec = dir.join(format!("spec-{ohms}.json"));
        fs::write(
            &spec,
            spec_text.replace(
                "\"pull_up_ohms\": 2200",
                &format!("\"pull_up_ohms\": {ohms}"),
            ),
        )
        .unwrap();
        let (code, v) = aethelgard(&["solve", "--file", spec.to_str().unwrap()]);
        assert_eq!(code, Some(0), "{v}");
        let topology: SolvedTopology = serde_json::from_value(v).unwrap();
        let platform = dir.join(format!("platform-{ohms}.repl"));
        fs::write(
            &platform,
            aethelgard_twin::platform::emit(&topology).unwrap(),
        )
        .unwrap();
        let run = |n: u32| {
            let start = Instant::now();
            let (code, v) = aethelgard(&[
                "simulate",
                "--platform",
                platform.to_str().unwrap(),
                "--firmware",
                firmware().to_str().unwrap(),
                "--spec",
                spec.to_str().unwrap(),
                "--for",
                "10ms",
            ]);
            answer(&format!(
                "2200 {ohms} ohm run {n}: exit={code:?} wall_s={:.2} analog={} faults={} findings={}",
                start.elapsed().as_secs_f64(),
                v["analog"],
                v["faults"],
                v["findings"]
            ));
            (code, v)
        };
        let (code, v) = run(1);
        let analog = v["analog"].as_array().unwrap_or_else(|| panic!("{v}"));
        assert_eq!(analog.len(), 2, "{v}");
        for line in analog {
            let measured = line["rise_30_70_ns"].as_f64().unwrap();
            assert!((measured - rise).abs() <= tolerance, "{ohms} ohm: {line}");
            assert_eq!(line["ceiling_ns"], 300);
            assert_eq!(line["exceeds"], faulted);
            assert_eq!(line["bus"], "SENSORS");
            assert_eq!(line["ngspice"], "44.2");
        }
        // The firmware's observation is its own record: the boot witnesses
        // hold whether or not the bus faulted (before AET-WO-2202 lands, the
        // firmware talks to no device).
        assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007", "{v}");
        assert_eq!(v["boot"]["instructions"]["observed"], 1_000_000);
        if faulted {
            // A faulted bus's BME280 is not in the platform Renode built
            // (vacuous until AET-WO-2202 declares devices: FEEDBACK F5).
            let listed = v["machine"]["peripherals"].as_array().unwrap();
            assert!(!listed.iter().any(|p| p["type"] == "BME280"), "{v}");
            assert_eq!(code, Some(1), "{v}");
            assert_eq!(v["verdict"], "fail");
            let faults = v["faults"].as_array().unwrap();
            assert_eq!(faults.len(), 1, "{v}");
            assert_eq!(faults[0]["kind"], "bridge.fault_state");
            assert_eq!(faults[0]["bus"], "SENSORS");
            for f in v["findings"].as_array().unwrap() {
                assert_eq!(f["kind"], "analog_rise_exceeds_ceiling", "{v}");
                let m = f["message"].as_str().unwrap();
                assert!(
                    m.contains("`SENSORS`") && m.contains("300 ns ceiling") && m.contains("3982.3"),
                    "{m}"
                );
            }
        } else {
            assert_eq!(code, Some(0), "{v}");
            assert_eq!(v["verdict"], "pass");
            assert_eq!(v["faults"], serde_json::json!([]));
        }
        let (_, again) = run(2);
        assert_eq!(v["analog"].to_string(), again["analog"].to_string());
        assert_eq!(v["faults"].to_string(), again["faults"].to_string());
    }
    let _ = fs::remove_dir_all(&dir);
}
