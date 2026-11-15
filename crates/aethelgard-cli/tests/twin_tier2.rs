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
//! `AET-WO-2202`: the firmware reads the BME280's ID register through the
//! board crate's transfer and the judge checks it against the part table.
//! - U2 (BME280, 0x76) reads 0x60 and passes; two boots agree; U3 (TMP117) is
//!   reported `device_unmodelled` and does not fail the run.
//! - Negatives: a read of an address no model answers fails `i2c_timeout` at
//!   TXIS; a record whose value is one bit off fails `i2c_identity_mismatch`;
//!   and, with Renode run directly, a hand-built platform without the BME280
//!   declaration leaves the TXIS timeout in the record.
//!
//! (Two emissions being byte-identical is tier 1: `aethelgard-twin`'s unit
//! tests and the regeneration guard in `aethelgard-svd-hal`.) Each boot writes
//! an `ANSWER 2100` line to `/dev/stderr`.

#![cfg(feature = "with-tools")]

mod common;

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
    assert_eq!(code, Some(0), "solving the spec exits 0: {v}");
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
            "the reference firmware builds offline and locked: {}",
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
    mutant_of(&root().join("boards/sentinel-01"), name, change)
}

/// As [`mutant`], from the firmware crate of the board crate at `board`.
fn mutant_of(board: &Path, name: &str, change: impl Fn(&Path)) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-2100-t2-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let src = board.join("firmware");
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
    let board = fs::canonicalize(board).unwrap();
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
        "the mutant firmware builds offline: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    dir.join("target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware")
}

fn edit(dir: &Path, rel: &str, from: &str, to: &str) {
    let path = dir.join(rel);
    let text = fs::read_to_string(&path).unwrap();
    // Exactly once, as the seeded set's mutants: a find that matches two
    // sites mutates both (`AET-WO-4200`: the SPI read's fold matched U2's).
    assert_eq!(text.matches(from).count(), 1, "{rel} holds {from} not once");
    fs::write(&path, text.replace(from, to)).unwrap();
}

/// The kinds of the findings that fail a boot, not the `info` ones.
fn kinds(v: &Value) -> Vec<&str> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] != "info")
        .map(|f| f["kind"].as_str().unwrap())
        .collect()
}

/// One boot of `elf` on the committed platform, judged by the reference spec.
fn boot_elf(label: &str, elf: &Path, time: &str) -> (Option<i32>, Value) {
    boot_on(label, &platform(), &spec(), elf, time)
}

/// One boot of `elf` on `platform`, judged by `spec`.
fn boot_on(
    label: &str,
    platform: &Path,
    spec: &Path,
    elf: &Path,
    time: &str,
) -> (Option<i32>, Value) {
    let start = Instant::now();
    let (code, v) = aethelgard(&[
        "simulate",
        "--platform",
        platform.to_str().unwrap(),
        "--firmware",
        elf.to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
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
        assert_eq!(
            got, sha,
            "each Renode file matches its pinned sha256: {file}"
        );
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
        assert_eq!(
            u32::from_str_radix(&hex, 16).unwrap(),
            address,
            "each stock peripheral sits at its declared sysbus address: {name}"
        );
    }
}

#[test]
fn the_emitted_platform_loads_and_lists_every_solved_peripheral() {
    let (code, v) = aethelgard(&["simulate", "--platform", platform().to_str().unwrap()]);
    answer(&format!("platform: exit={code:?} verdict={v}"));
    assert_eq!(code, Some(0), "the emitted platform loads and exits 0: {v}");
    let listed: Vec<(&str, &str)> = v["machine"]["peripherals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap()))
        .collect();
    let solved =
        aethelgard_twin::platform::instances(&topology(), &Default::default(), &Default::default())
            .unwrap();
    assert!(
        !solved.is_empty(),
        "the topology solves to at least one peripheral"
    );
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
    assert_eq!(
        code,
        Some(0),
        "the reference firmware boots and passes the spec: {first}"
    );
    assert_eq!(first["verdict"], "pass", "the first boot's verdict is pass");
    let b = &first["boot"];
    assert_eq!(
        b["boot_word"]["observed"], "0x2100b007",
        "the boot word reads 0x2100b007 after the first boot"
    );
    assert_eq!(
        b["instructions"]["observed"], 1_000_000,
        "the first boot executes exactly 1,000,000 instructions"
    );
    assert_eq!(b["seed"], 2100, "the boot runs with the pinned seed 2100");
    let word = |v: &Value| u32::from_str_radix(&v.as_str().unwrap()[2..], 16).unwrap();
    for r in b["registers"].as_array().unwrap() {
        assert_eq!(
            word(&r["observed"]) & word(&r["mask"]),
            word(&r["expected"]),
            "every register at the end of the boot equals its expected value under its mask: {r}"
        );
    }
    // AET-WO-2202: U2's ID register reads 0x60, from the part table; U3 is
    // reported and does not fail the run.
    assert_eq!(
        b["i2c"],
        serde_json::json!([{
            "reference": "U2", "address": "0x76", "step": "identity", "register": "0xd0",
            "width": 1, "expected": "0x60", "mask": "0xff", "observed": "0x60", "status": "ok"
        }]),
        "the i2c record is U2 reading 0x60 at register 0xd0 with status ok"
    );
    let unmodelled: Vec<&Value> = first["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "device_unmodelled")
        .collect();
    assert_eq!(
        unmodelled.len(),
        1,
        "exactly one device is reported unmodelled: {first}"
    );
    assert_eq!(
        unmodelled[0]["severity"], "info",
        "the unmodelled-device finding is informational"
    );
    // AET-WO-2204 D1: Renode's BME280 logged the read the record claims,
    // exactly; D3: the core ended in thread mode.
    assert_eq!(
        b["i2c_witness"],
        serde_json::json!([{
            "peripheral": "I2C1", "reference": "U2", "step": "identity", "device": "i2c1.u2",
            "expected": WITNESS, "observed": WITNESS, "absent": []
        }]),
        "Renode's BME280 logged exactly the read the record claims"
    );
    assert_eq!(
        b["icsr"], "0x00000800",
        "the core ended the boot in thread mode"
    );
    assert!(
        unmodelled[0]["message"]
            .as_str()
            .unwrap()
            .contains("U3 (TMP117)"),
        "the unmodelled-device message names U3 (TMP117)"
    );
    assert_eq!(
        first["findings"].as_array().unwrap().len(),
        1,
        "the unmodelled-device report is the boot's only finding: {first}"
    );
    let (code, second) = boot("boot 2", "10ms");
    assert_eq!(code, Some(0), "the second boot also passes: {second}");
    assert_eq!(
        first["boot"], second["boot"],
        "two boots of the reference agree on the boot record"
    );
    assert_eq!(
        first["machine"], second["machine"],
        "two boots of the reference agree on the machine record"
    );
}

#[test]
fn a_boot_too_short_to_reach_the_fixed_point_fails() {
    // The spike measured the fixed point between 1,000 and 2,000 instructions.
    let (code, v) = boot("short", "10us");
    assert_eq!(
        code,
        Some(1),
        "a boot too short for the fixed point fails the spec: {v}"
    );
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"boot_word_missing"),
        "a boot too short for the fixed point reports the boot word missing: {v}"
    );
    assert!(
        !kinds.contains(&"cpu_halted"),
        "a boot too short for the fixed point has not halted the cpu: {v}"
    );
    assert_eq!(
        v["boot"]["instructions"]["observed"], 1000,
        "a 10 us boot executes exactly 1000 instructions"
    );
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
    assert_eq!(
        code,
        Some(1),
        "a flash offset reports boot_word_missing and cpu_halted: {v}"
    );
    let kinds = kinds(&v);
    assert!(
        kinds.contains(&"boot_word_missing") && kinds.contains(&"cpu_halted"),
        "a flash offset reports boot_word_missing and cpu_halted: {v}"
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
    assert_eq!(code, Some(1), "a core leaving the firmware exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["pc_outside_firmware"],
        "a core leaving the firmware reports exactly pc_outside_firmware: {v}"
    );
    assert_eq!(
        v["boot"]["boot_word"]["observed"], "0x2100b007",
        "the boot word is still set when the core leaves the firmware"
    );
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
    assert_eq!(code, Some(1), "an aborted machine exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["cpu_aborted"],
        "an aborted machine reports exactly cpu_aborted: {v}"
    );
    assert!(
        took.as_secs() < 30 + 5,
        "an aborted machine returns inside the bound: {took:?}"
    );
}

/// `AET-WO-2200` §3, the P2 gate's clause 2, as one test: the reference
/// specification at 2.2 kΩ passes, with U2 reading 0x60, and at 47 kΩ fails,
/// with three separate records: the analog evidence (`analog`), the bridge's
/// fault decision (`faults`), and the firmware's own observation, U2 timing
/// out at TXIS (`boot.i2c`, `AET-WO-2202`'s judge). Each case runs twice with
/// byte-identical records. The ns values are `AET-WO-2101` S2's.
#[test]
fn p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes() {
    let spec_text = fs::read_to_string(spec()).unwrap();
    assert_eq!(
        spec_text.matches("\"pull_up_ohms\": 2200").count(),
        1,
        "the spec declares the 2200 ohm pull-up exactly once"
    );
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-2200-t2-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for (ohms, rise, tolerance, faulted) in
        [(2200, 186.40, 0.05, false), (47_000, 3982.30, 0.5, true)]
    {
        let text = spec_text.replace(
            "\"pull_up_ohms\": 2200",
            &format!("\"pull_up_ohms\": {ohms}"),
        );
        let spec = dir.join(format!("spec-{ohms}.json"));
        fs::write(&spec, &text).unwrap();
        let (code, v) = aethelgard(&["solve", "--file", spec.to_str().unwrap()]);
        assert_eq!(
            code,
            Some(0),
            "the spec with each pull-up value solves: {v}"
        );
        let topology: SolvedTopology = serde_json::from_value(v).unwrap();
        // The caller's platform is the unfaulted emission, U2 declared (F6).
        let platform = dir.join(format!("platform-{ohms}.repl"));
        let emitted = aethelgard_twin::platform::emit(&topology).unwrap();
        assert!(
            emitted.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"),
            "the emitted platform declares the BME280 u2 at i2c1 0x76"
        );
        fs::write(&platform, emitted).unwrap();
        let run = |n: u32| {
            let start = Instant::now();
            // 100 ms, so a timed-out read still reaches the boot word.
            let (code, v) = aethelgard(&[
                "simulate",
                "--platform",
                platform.to_str().unwrap(),
                "--firmware",
                firmware().to_str().unwrap(),
                "--spec",
                spec.to_str().unwrap(),
                "--for",
                "100ms",
            ]);
            answer(&format!(
                "2200 {ohms} ohm run {n}: exit={code:?} wall_s={:.2} analog={} faults={} i2c={} findings={}",
                start.elapsed().as_secs_f64(),
                v["analog"],
                v["faults"],
                v["boot"]["i2c"],
                v["findings"]
            ));
            (code, v)
        };
        let (code, v) = run(1);
        let analog = v["analog"]
            .as_array()
            .unwrap_or_else(|| panic!("the run reports its analog records: {v}"));
        assert_eq!(
            analog.len(),
            2,
            "the run reports analog records for two lines: {v}"
        );
        for line in analog {
            let measured = line["rise_30_70_ns"].as_f64().unwrap();
            assert!(
                (measured - rise).abs() <= tolerance,
                "the measured 30-70 rise matches the SPICE value within tolerance: {ohms} ohm: {line}"
            );
            assert_eq!(
                line["ceiling_ns"], 300,
                "each analog line carries the 300 ns ceiling"
            );
            assert_eq!(
                line["complete"], true,
                "each analog line is a complete measurement"
            );
            assert_eq!(
                line["exceeds"], faulted,
                "an analog line exceeds the ceiling exactly when the pull-up is 47k"
            );
            assert_eq!(
                line["bus"], "SENSORS",
                "each analog line names the bus SENSORS"
            );
            assert_eq!(
                line["ngspice"], "44.2",
                "each analog line records ngspice 44.2"
            );
        }
        // The boot completed either way: the word is set and the core ran.
        assert_eq!(
            v["boot"]["boot_word"]["observed"], "0x2100b007",
            "the boot completes either way: the boot word is set: {v}"
        );
        assert_eq!(
            v["boot"]["instructions"]["observed"], 10_000_000,
            "the core ran 10,000,000 instructions either way"
        );
        let u2 = &v["boot"]["i2c"][0];
        assert_eq!(u2["reference"], "U2", "the i2c record is U2's: {v}");
        assert_eq!(u2["address"], "0x76", "the i2c record is for address 0x76");
        assert_eq!(u2["expected"], "0x60", "the i2c record expects 0x60");
        let listed = v["machine"]["peripherals"].as_array().unwrap();
        let bme = listed.iter().any(|p| p["type"] == "BME280");
        let kinds: Vec<&str> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == "error")
            .map(|f| f["kind"].as_str().unwrap())
            .collect();
        if faulted {
            assert_eq!(code, Some(1), "the 47k run fails: {v}");
            assert_eq!(v["verdict"], "fail", "the 47k run's verdict is fail");
            // The fault reached the platform: Renode built no BME280.
            assert!(
                !bme,
                "the fault reached the platform: Renode built no BME280: {v}"
            );
            let faults = v["faults"].as_array().unwrap();
            assert_eq!(
                faults.len(),
                1,
                "the 47k run reports exactly one fault: {v}"
            );
            assert_eq!(
                faults[0]["kind"], "bridge.fault_state",
                "the fault is the bridge's fault state"
            );
            assert_eq!(faults[0]["bus"], "SENSORS", "the fault is on bus SENSORS");
            // The firmware's observation, its own record: U2 timed out at TXIS.
            assert_eq!(
                u2["status"], "timeout_txis",
                "the firmware's own record shows U2 timed out at TXIS: {v}"
            );
            assert_eq!(
                kinds,
                [
                    "analog_rise_exceeds_ceiling",
                    "analog_rise_exceeds_ceiling",
                    "i2c_timeout"
                ],
                "the 47k run's error findings are two slow rises and an I2C timeout: {v}"
            );
            for f in &v["findings"].as_array().unwrap()[..2] {
                let m = f["message"].as_str().unwrap();
                assert!(
                    m.contains("`SENSORS`") && m.contains("300 ns ceiling") && m.contains("3982.3"),
                    "each rise finding names the bus, the 300 ns ceiling and the measured 3982.3 ns: {m}"
                );
            }
        } else {
            assert_eq!(code, Some(0), "the 2k2 run passes: {v}");
            assert_eq!(v["verdict"], "pass", "the 2k2 run's verdict is pass");
            assert_eq!(
                v["faults"],
                serde_json::json!([]),
                "the 2k2 run reports no faults"
            );
            assert!(bme, "the unfaulted platform gives Renode a BME280: {v}");
            assert_eq!(
                (&u2["status"], &u2["observed"]),
                (&"ok".into(), &"0x60".into()),
                "the 2k2 run's U2 record is ok reading 0x60"
            );
            assert!(kinds.is_empty(), "the 2k2 run has no error findings: {v}");
        }
        let (_, again) = run(2);
        for record in ["analog", "faults"] {
            assert_eq!(
                v[record].to_string(),
                again[record].to_string(),
                "a second run reproduces the same record: {record}"
            );
        }
        assert_eq!(
            v["boot"]["i2c"].to_string(),
            again["boot"]["i2c"].to_string(),
            "a second run reproduces the same i2c record"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_read_no_model_answers_fails_as_a_timeout_at_txis() {
    // AET-WO-2202: the firmware reads address 0x77, where the emitted platform
    // declares nothing. Renode's controller never raises TXIS for an absent
    // device (AET-WO-2101 S3), so the bounded wait expires.
    let elf = mutant("absent", |d| {
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x76,",
            "i2c1.transfer(&mut r, 0x77,",
        );
    });
    let (code, v) = boot_elf("absent device", &elf, "100ms");
    assert_eq!(code, Some(1), "an absent device exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["i2c_timeout"],
        "an absent device reports exactly i2c_timeout: {v}"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["status"], "timeout_txis",
        "the absent device's I2C status is timeout_txis"
    );
    let f = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == "i2c_timeout")
        .unwrap();
    let m = f["message"].as_str().unwrap();
    assert!(
        m.contains("U2") && m.contains("0x76") && m.contains("TXIS") && m.contains("0x60"),
        "the timeout message names U2, its address, TXIS and the identity register: {m}"
    );
    // The boot itself completed: the word is set, the core ran to the end.
    assert_eq!(
        v["boot"]["boot_word"]["observed"], "0x2100b007",
        "the boot itself completed with the boot word set"
    );
}

#[test]
fn a_value_one_bit_off_fails_as_an_identity_mismatch() {
    // AET-WO-2202: the firmware's record of what it read is one bit off the
    // table's (0x160 for 0x60). The judge's expectation is the table's.
    let elf = mutant("one-bit", |d| {
        edit(
            d,
            "src/main.rs",
            "buf0.iter().fold(0u32,",
            "buf0.iter().fold(1u32,",
        );
    });
    let (code, v) = boot_elf("one bit", &elf, "10ms");
    assert_eq!(code, Some(1), "a one-bit-off value exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["i2c_identity_mismatch"],
        "a one-bit-off value reports exactly i2c_identity_mismatch: {v}"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["expected"], "0x60",
        "the identity mismatch expects 0x60"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["observed"], "0x160",
        "the identity mismatch observes 0x160"
    );
}

/// Renode run directly (not through `simulate`, which refuses a platform other
/// than the emitted one): a boot's monitor script, as `simulate` built it
/// before `AET-WO-2204` (no log is judged here, so the console's level may
/// change), on a platform of the test's own, and the two words of U2's record.
fn record_on(platform: &Path, elf: &Path) -> (String, String) {
    let boot_word = aethelgard_twin::boot::boot_word().unwrap();
    let unset = aethelgard_twin::boot::RECORD_UNSET;
    let (status_at, value_at) = (boot_word + 4, boot_word + 8);
    let commands = [
        "emulation SetSeed 2100".to_owned(),
        "mach create".to_owned(),
        format!("machine LoadPlatformDescription @{}", platform.display()),
        format!("sysbus LoadELF @{}", elf.display()),
        format!("sysbus WriteDoubleWord {boot_word:#010x} 0x0"),
        format!("sysbus WriteDoubleWord {status_at:#010x} {unset:#010x}"),
        format!("sysbus WriteDoubleWord {value_at:#010x} {unset:#010x}"),
        "cpu VectorTableOffset 0x08000000".to_owned(),
        "emulation RunFor \"0.100000000\"".to_owned(),
        "logLevel 3".to_owned(),
        "echo \"BEGIN\"".to_owned(),
        format!("sysbus ReadDoubleWord {status_at:#010x}"),
        format!("sysbus ReadDoubleWord {value_at:#010x}"),
        "echo \"END\"".to_owned(),
        "quit".to_owned(),
    ]
    .join("; ");
    let out = Command::new("renode")
        .args(["--disable-gui", "--console", "--plain", "-e", &commands])
        .env("DOTNET_EnableDiagnostics", "0")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    let lines: Vec<&str> = text.lines().collect();
    let begin = lines
        .iter()
        .position(|l| *l == "BEGIN")
        .unwrap_or_else(|| panic!("{text}"));
    (lines[begin + 1].to_owned(), lines[begin + 2].to_owned())
}

#[test]
fn a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record() {
    // AET-WO-2202: the emitted platform with U2's declaration removed, by hand.
    let emitted = fs::read_to_string(platform()).unwrap();
    assert!(
        emitted.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"),
        "the emitted platform declares the BME280 u2 at i2c1 0x76"
    );
    let dir = std::env::temp_dir().join(format!("aet-2202-t2-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let without = dir.join("without.repl");
    fs::write(
        &without,
        emitted.replace("u2: I2C.BME280 @ i2c1 0x76\n", ""),
    )
    .unwrap();
    let with = dir.join("with.repl");
    fs::write(&with, &emitted).unwrap();

    let ok = aethelgard_twin::boot::STATUS_OK;
    let txis = aethelgard_twin::boot::STATUS_TXIS;
    let (status, value) = record_on(&with, firmware());
    answer(&format!(
        "direct, with the model: status={status} value={value}"
    ));
    assert_eq!(
        (status.as_str(), value.as_str()),
        (
            format!("{ok:#010x}")
                .to_uppercase()
                .replace("0X", "0x")
                .as_str(),
            "0x00000060"
        ),
        "with the model the record holds status ok and value 0x60"
    );
    let (status, _) = record_on(&without, firmware());
    answer(&format!("direct, without the model: status={status}"));
    assert_eq!(
        status,
        format!("{txis:#010x}").to_uppercase().replace("0X", "0x"),
        "without the model the record holds the TXIS timeout status"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_firmware_that_skips_the_read_or_its_record_cannot_pass() {
    // Adversarial 1, on Renode itself. Skipping the transfer leaves the read
    // buffer zero: an OK record carrying 0, a mismatch. Not writing the record
    // leaves the judge's fill: an invalid record.
    let skip = mutant("skip-read", |d| {
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x76, &[0xd0], &mut buf0)",
            "Ok::<usize, I2cError>(0)",
        );
    });
    let (code, v) = boot_elf("skip the read", &skip, "10ms");
    assert_eq!(code, Some(1), "a firmware skipping the read exits 1: {v}");
    // The record carries 0, and the device logged no transaction.
    assert_eq!(
        kinds(&v),
        ["i2c_identity_mismatch", "i2c_transaction_unwitnessed"],
        "a firmware skipping the read reports an identity mismatch and an unwitnessed transaction: {v}"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["observed"], "0x0",
        "the skipped read leaves the record at 0x0"
    );

    let unwritten = mutant("no-record", |d| {
        edit(
            d,
            "src/main.rs",
            "    record(0, status, value);",
            "    let _ = (status, value);",
        );
    });
    let (code, v) = boot_elf("no record", &unwritten, "10ms");
    assert_eq!(code, Some(1), "a firmware with no record exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["i2c_record_invalid"],
        "a firmware with no record reports exactly i2c_record_invalid: {v}"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["status"], "invalid",
        "the missing record's I2C status is invalid"
    );
}

/// What Renode 1.17.0's BME280 logs for U2's identity read (`AET-WO-2204`
/// S1, measured).
const WITNESS: [&str; 2] = ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60"];

/// A finding's message, by kind.
fn message<'a>(v: &'a Value, kind: &str) -> &'a str {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind}: {v}"))["message"]
        .as_str()
        .unwrap()
}

/// U2's read replaced by `forgery` (an expression of type
/// `Result<usize, I2cError>`), with the table's 0x60 in the buffer.
fn forged(name: &str, forgery: &'static str) -> PathBuf {
    mutant(name, move |d| {
        edit(
            d,
            "src/main.rs",
            "let mut buf0 = [0u8; 1];",
            "let mut buf0 = [0x60u8; 1];",
        );
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x76, &[0xd0], &mut buf0)",
            forgery,
        );
    })
}

/// A forgery's verdict: the record is perfect, and the read is unwitnessed:
/// the device logged nothing.
fn assert_unwitnessed(v: &Value) {
    assert_eq!(
        kinds(v),
        ["i2c_transaction_unwitnessed"],
        "the run reports exactly i2c_transaction_unwitnessed: {v}"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["status"], "ok",
        "the forged read's I2C status is ok"
    );
    assert_eq!(
        v["boot"]["i2c"][0]["observed"], "0x60",
        "the forged read's record observes 0x60"
    );
    assert_eq!(
        v["boot"]["i2c_witness"][0]["observed"],
        serde_json::json!([]),
        "the device witnessed no transaction"
    );
    let m = message(v, "i2c_transaction_unwitnessed");
    assert!(
        m.contains("U2") && m.contains("0x76") && m.contains("i2c1.u2"),
        "the unwitnessed message names U2, its address and the device instance: {m}"
    );
}

#[test]
fn a_forged_record_without_a_transaction_fails_unwitnessed() {
    // AET-WO-2202 W1, now AET-WO-2204 D1: the firmware writes STATUS_OK and
    // the table's 0x60 and never calls `transfer`.
    let elf = forged("forged", "Ok::<usize, I2cError>(1)");
    let (code, v) = boot_elf("forged record", &elf, "10ms");
    assert_eq!(code, Some(1), "a forged record exits 1: {v}");
    assert_unwitnessed(&v);
}

#[test]
fn a_firmware_that_writes_the_controller_registers_by_hand_fails_unwitnessed() {
    // AET-WO-1900 §1.10 (review A11.1), measured passing until AET-WO-2204:
    // the forged record, and the reference boot's final `CR2` (0x020104EC)
    // and `TXDR` (0xD0) written straight to I2C1 (base 0x40005400, CR2
    // +0x04, TXDR +0x28), with no call to `transfer`. Renode stores both
    // writes; the BME280 model logs nothing, so the judge fails it. Inverted,
    // not deleted, as 1900 asked.
    let elf = forged(
        "forged-regs",
        "{ unsafe { core::ptr::write_volatile(0x4000_5404 as *mut u32, 0x0201_04EC); core::ptr::write_volatile(0x4000_5428 as *mut u32, 0xD0); } Ok::<usize, I2cError>(1) }",
    );
    let (code, v) = boot_elf("forged record and registers", &elf, "10ms");
    assert_eq!(
        code,
        Some(1),
        "a forged record with hand-written registers exits 1: {v}"
    );
    assert_unwitnessed(&v);
}

#[test]
fn a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed() {
    // AET-WO-2204 S1, measured: every I2C1 register at the reference's final
    // value (STOP alone sets STOPF, ICR clears it as the board does; ISR and
    // RXDR written too), so every register reads as the reference's; and a
    // real transfer to the absent 0x77 followed by the same hand-written CR2
    // and TXDR. Neither reaches the BME280.
    let all = forged(
        "forge-all",
        "{ unsafe { core::ptr::write_volatile(0x4000_5404 as *mut u32, 0x4000); core::ptr::write_volatile(0x4000_5404 as *mut u32, 0x0201_04EC); core::ptr::write_volatile(0x4000_5428 as *mut u32, 0xD0); core::ptr::write_volatile(0x4000_5418 as *mut u32, 0x1); core::ptr::write_volatile(0x4000_5424 as *mut u32, 0x60); core::ptr::write_volatile(0x4000_541C as *mut u32, 0x30); } Ok::<usize, I2cError>(1) }",
    );
    let wrong = forged(
        "wrong-addr-forge",
        "{ let _ = i2c1.transfer(&mut r, 0x77, &[0xd0], &mut buf0); buf0 = [0x60u8]; unsafe { core::ptr::write_volatile(0x4000_5404 as *mut u32, 0x0201_04EC); core::ptr::write_volatile(0x4000_5428 as *mut u32, 0xD0); } Ok::<usize, I2cError>(1) }",
    );
    for (label, elf) in [("forge every register", all), ("wrong address", wrong)] {
        let (code, v) = boot_elf(label, &elf, "10ms");
        assert_eq!(code, Some(1), "each forgery exits 1: {label}: {v}");
        assert_unwitnessed(&v);
    }
}

/// The reference specification with a second BME280, `U5` at 0x77 on
/// `SENSORS` (a test fixture, `AET-WO-2204` §2.2), generated once.
fn two_bme() -> &'static (PathBuf, PathBuf) {
    static TREE: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    TREE.get_or_init(|| {
        let spec = root().join("crates/aethelgard-cli/tests/fixtures/two-bme.json");
        let out = std::env::temp_dir().join(format!("aet-2204-t2-{}-two-bme", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let (code, v) = aethelgard(&[
            "generate",
            "--spec",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        assert_eq!(code, Some(0), "the two-BME spec generates a board: {v}");
        (spec, out.join("board"))
    })
}

#[test]
fn on_a_bus_with_two_devices_each_read_is_witnessed() {
    // AET-WO-2204 S2, measured: today's judge witnessed only the last read on
    // a bus and passed both forgeries.
    let (spec, board) = two_bme();
    let platform = board.join("sentinel-01.repl");
    let real = mutant_of(board, "two-ref", |_| {});
    let first = mutant_of(board, "two-forged-first", |d| {
        edit(
            d,
            "src/main.rs",
            "let mut buf0 = [0u8; 1];",
            "let mut buf0 = [0x60u8; 1];",
        );
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x76, &[0xd0], &mut buf0)",
            "Ok::<usize, I2cError>(1)",
        );
    });
    let last = mutant_of(board, "two-forged-last", |d| {
        edit(
            d,
            "src/main.rs",
            "let mut buf1 = [0u8; 1];",
            "let mut buf1 = [0x60u8; 1];",
        );
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x77, &[0xd0], &mut buf1)",
            "{ unsafe { core::ptr::write_volatile(0x4000_5404 as *mut u32, 0x0201_04EE); core::ptr::write_volatile(0x4000_5428 as *mut u32, 0xD0); } Ok::<usize, I2cError>(1) }",
        );
    });
    let (code, v) = boot_on("two devices", &platform, spec, &real, "10ms");
    assert_eq!(
        code,
        Some(0),
        "the real firmware on the two-BME board passes the spec: {v}"
    );
    let w = v["boot"]["i2c_witness"].as_array().unwrap();
    assert_eq!(
        w.len(),
        2,
        "the witness covers both devices on the bus: {v}"
    );
    assert_eq!(
        w[1]["device"], "i2c1.u5",
        "the second witnessed device is i2c1.u5"
    );
    assert_eq!(
        w[1]["observed"],
        serde_json::json!(["[NOISY] i2c1.u5: Write D0", "[NOISY] i2c1.u5: Read 60"]),
        "Renode's second BME280 logged the write and the read the record claims"
    );
    for (label, elf, who) in [("forged first", first, "U2"), ("forged last", last, "U5")] {
        let (code, v) = boot_on(label, &platform, spec, &elf, "10ms");
        assert_eq!(
            code,
            Some(1),
            "a forged read on the two-BME board fails the spec: {label}: {v}"
        );
        assert_eq!(
            kinds(&v),
            ["i2c_transaction_unwitnessed"],
            "a forged read on the two-BME board is reported unwitnessed: {label}: {v}"
        );
        assert!(
            message(&v, "i2c_transaction_unwitnessed").starts_with(who),
            "the unwitnessed finding names the forged device: {label}: {v}"
        );
    }
}

#[test]
fn a_firmware_parked_in_its_fault_handler_fails_naming_vectactive() {
    // AET-WO-2301 F9, AET-WO-2204 D3: `systick-before`, as a firmware edit:
    // SysTick armed (reload 999, TICKINT) after the boot word is written, so
    // the first tick enters `fault` and parks there, the boot word and record
    // written and MIPS x time executed, on `fault`'s own self-loop.
    let elf = mutant("systick", |d| {
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires",
            "    unsafe { core::ptr::write_volatile(0xE000_E014 as *mut u32, 999); core::ptr::write_volatile(0xE000_E018 as *mut u32, 0); core::ptr::write_volatile(0xE000_E010 as *mut u32, 7); }\n    // No sleep: the judge requires",
        );
    });
    let (code, v) = boot_elf("fault-parked", &elf, "10ms");
    assert_eq!(code, Some(1), "a fault-parked firmware exits 1: {v}");
    // `AET-WO-4210` D6: and SysTick is armed after the boot word.
    assert_eq!(
        kinds(&v),
        ["core_in_exception", "register_mismatch"],
        "a fault-parked firmware reports core_in_exception and the armed SysTick: {v}"
    );
    assert!(
        message(&v, "register_mismatch").starts_with("SysTick.CSR"),
        "{v}"
    );
    assert!(
        message(&v, "core_in_exception").contains("exception 15"),
        "the core_in_exception message names exception 15: {v}"
    );
    assert_eq!(
        v["boot"]["boot_word"]["observed"], "0x2100b007",
        "the fault-parked boot word is still set"
    );
    assert_eq!(
        v["boot"]["instructions"]["observed"], 1_000_000,
        "the fault-parked core ran the full instruction budget"
    );
}

/// `AET-WO-2204` A2, ruled D6 on PR #131: after the boot word the core is
/// parked with no interrupt armed. A SysTick counting without its interrupt,
/// or one NVIC line enabled that never fires, leaves the core on its loop
/// and passed before this build; each now fails naming the register.
#[test]
fn an_interrupt_armed_after_the_boot_word_fails_naming_it() {
    for (label, store, register) in [
        (
            "systick-counting",
            "core::ptr::write_volatile(0xE000_E014 as *mut u32, 999); core::ptr::write_volatile(0xE000_E010 as *mut u32, 1);",
            "SysTick.CSR",
        ),
        (
            "nvic-line",
            "core::ptr::write_volatile(0xE000_E100 as *mut u32, 1);",
            "NVIC.ISER0",
        ),
    ] {
        let elf = mutant(label, |d| {
            edit(
                d,
                "src/main.rs",
                "    // No sleep: the judge requires",
                &format!("    unsafe {{ {store} }}\n    // No sleep: the judge requires"),
            );
        });
        let (code, v) = boot_elf(label, &elf, "10ms");
        assert_eq!(code, Some(1), "{label}: {v}");
        assert_eq!(kinds(&v), ["register_mismatch"], "{label}: {v}");
        assert!(
            message(&v, "register_mismatch").starts_with(register),
            "{label}: {v}"
        );
    }
}

#[test]
fn a_firmware_that_rewrites_its_loop_in_flash_is_not_parked() {
    // AET-WO-2204 adversarial A1 (`smc-flash2`): the ELF carries a `yield;
    // b.n` loop, and after the boot word the firmware stores `str r0, [r1]`
    // over its `yield` (Renode's flash takes the store) and spins there,
    // writing a register. On the ELF's bytes the core looked parked; the
    // judge reads the loop back from the machine.
    let elf = mutant("rewritten-loop", |d| {
        edit(
            d,
            "src/main.rs",
            "unsafe extern \"C\" {\n    fn Reset() -> !;\n}",
            "core::arch::global_asm!(\n    \".section .text.fakeloop,\\\"ax\\\",%progbits\",\n    \".global fakeloop\", \".type fakeloop,%function\", \".thumb_func\", \".balign 4\",\n    \"fakeloop:\", \"    .short 0xbf10\", \"    .short 0xe7fd\",\n    \".size fakeloop, . - fakeloop\",\n);\nunsafe extern \"C\" {\n    fn fakeloop() -> !;\n    fn Reset() -> !;\n}",
        );
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires instructions executed to equal MIPS x time.\n    loop {\n        core::hint::spin_loop();\n    }\n}\n\n#[panic_handler]",
            "    unsafe {\n        let at = (fakeloop as usize & !1) as *mut u16;\n        at.write_volatile(0x6008);\n        core::arch::asm!(\"dsb\", \"isb\");\n        core::arch::asm!(\"bx {t}\", t = in(reg) (at as usize) | 1, in(\"r0\") 0x1u32, in(\"r1\") 0x5802_0014u32, options(noreturn));\n    }\n}\n\n#[panic_handler]",
        );
    });
    let (code, v) = boot_elf("rewritten loop", &elf, "10ms");
    assert_eq!(code, Some(1), "a rewritten loop exits 1: {v}");
    assert_eq!(
        kinds(&v),
        ["pc_not_on_self_loop"],
        "a rewritten loop reports exactly pc_not_on_self_loop: {v}"
    );
    assert!(
        message(&v, "pc_not_on_self_loop").contains("no longer holds"),
        "the pc_not_on_self_loop message says the loop no longer holds: {v}"
    );
}

#[test]
fn the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s() {
    // AET-WO-2204 D8 (a): at 1 us no clock is on yet, so the judge's own
    // reads of I2C1 and the GPIO ports log locked accesses. With synchronous
    // logging they come after BEGIN and are not attributed to the firmware.
    let (code, v) = boot("1us", "1us");
    assert_eq!(code, Some(1), "a 1 us boot fails the spec: {v}");
    // At 1 us nothing is configured yet: the boot word is missing and the
    // registers are at reset. No locked access is the firmware's.
    let k = kinds(&v);
    assert!(
        k.contains(&"boot_word_missing"),
        "a 1 us boot reports the boot word missing: {v}"
    );
    assert!(
        k.iter()
            .all(|k| ["boot_word_missing", "register_mismatch"].contains(k)),
        "no locked access at 1 us is attributed to the firmware: {v}"
    );
}

#[test]
fn a_locked_access_late_in_a_flood_reaches_the_judge_whole() {
    // AET-WO-2204 D8, F2, B10: the spike's `flood-locked`, 37.7 MB of
    // alternating warnings after the boot word, one read of the gated
    // GPIOC.MODER about 600 lines before the run's end (pair 199,461 of the
    // 199,761 measured). Its line reaches the judge, and the markers and
    // values come through whole (a verdict, not exit 5). Since `AET-WO-4300b`
    // the console's echo runs between the boot word and the loop, waiting for
    // the frame injected at 100 us, so the loop starts later: its last pair
    // in the 10 ms boot is 197,874 (measured by bisection on this read's
    // pair, C1 `flood-bisect.sh`), and the read is 300 pairs before it, as
    // before.
    let elf = mutant("flood-locked", |d| {
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires instructions executed to equal MIPS x time.\n    loop {\n        core::hint::spin_loop();\n    }\n",
            "    let mut n: u32 = 0;\n    loop {\n        unsafe { core::ptr::read_volatile(0x40005424 as *const u32); core::ptr::write_volatile(0x4000541c as *mut u32, 0x10); }\n        if n == 197574 { unsafe { core::ptr::read_volatile(0x5802_0800 as *const u32); } }\n        n = n.wrapping_add(1);\n    }\n",
        );
    });
    let (code, v) = boot_elf("flood-locked", &elf, "10ms");
    assert_eq!(code, Some(1), "a flood with a locked access exits 1: {v}");
    let k = kinds(&v);
    assert!(
        k.contains(&"locked_peripheral_access"),
        "the locked access is reported as locked_peripheral_access: {v}"
    );
    assert!(
        message(&v, "locked_peripheral_access").contains("0x58020800"),
        "the locked-access message carries the register address: {v}"
    );
    // Past the filter's budget: the witness is unread, never passed (D2).
    assert!(
        k.contains(&"i2c_transaction_unwitnessed"),
        "past the filter budget the witness is unread, never passed: {v}"
    );
    assert!(
        message(&v, "renode_log_flood").contains("8 MiB"),
        "the flood is reported with its 8 MiB budget: {v}"
    );
}

#[test]
fn a_device_named_for_a_renode_instance_or_keyword_breaks_the_load() {
    // AET-WO-2204 S5, measured: why `validate` refuses such references. A
    // platform declaring a device under a stock instance's name, or under a
    // `.repl` keyword, does not load in Renode 1.17.0.
    let dir = std::env::temp_dir().join(format!("aet-2204-t2-{}-reserved", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let emitted = fs::read_to_string(platform()).unwrap();
    for (name, error) in [
        ("nvic", "Error E02: Variable 'nvic' was already declared"),
        ("using", "Error E00: Syntax error, Unexpected using."),
    ] {
        let repl = dir.join(format!("{name}.repl"));
        fs::write(
            &repl,
            emitted.replace(
                "u2: I2C.BME280 @ i2c1 0x76\n",
                &format!("{name}: I2C.BME280 @ i2c1 0x76\n"),
            ),
        )
        .unwrap();
        let (code, v) = aethelgard(&["simulate", "--platform", repl.to_str().unwrap()]);
        assert_eq!(code, Some(1), "each bad device name exits 1: {name}: {v}");
        assert_eq!(
            kinds(&v),
            ["platform_load_error"],
            "each bad device name reports exactly platform_load_error: {name}: {v}"
        );
        assert!(
            message(&v, "platform_load_error").contains(error),
            "each bad device name's load error carries the expected text: {name}: {v}"
        );
    }
}

/// Renode 1.17.0's own configuration when given none, as it writes it (the
/// home as `$HOME`), measured; the CLI's (`renode::config`, pinned by its
/// unit test) is this with `use-synchronous-logging = true` and, since
/// `AET-WO-4300a` (`4300` D2), `log-timestamps-type = Virtual`.
const RENODE_DEFAULT_CONFIG: &str = "[general]
use-synchronous-logging = False
terminal = Termsharp
compiler-cache-enabled = False
serialization-mode = Generated
always-log-machine-name = False
collapse-repeated-log-entries = True
log-timestamps-type = Host
log-history-limit = 1000
history-path = $HOME/.config/renode/history
store-table-bits = 41
[monitor]
consume-exceptions-from-command = True
break-script-on-exception = True
number-format = Hexadecimal
[plugins]
enabled-plugins = 
[translation]
min-tb-size = 33554432
max-tb-size = 536870912
";

#[test]
fn renode_s_configuration_differs_from_its_default_in_synchronous_logging_and_virtual_time_alone() {
    // AET-WO-2204 D4. Renode writes every key it reads into its default file
    // when given none; given a file stating every one of them, it only reads
    // it: the CLI's file is sealed read-only for the tool uid (AET-WO-1601),
    // and a file missing a key made Renode abort in the sandbox (measured).
    let dir = std::env::temp_dir().join(format!("aet-2204-t2-{}-config", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let renode = |home: &Path, config: Option<&Path>, commands: &str| {
        fs::create_dir_all(home).unwrap();
        let mut c = Command::new("renode");
        if let Some(path) = config {
            c.arg("--config").arg(path);
        }
        let out = c
            .args(["--disable-gui", "--console", "--plain", "-e", commands])
            .env_clear()
            .env("HOME", home)
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("DOTNET_EnableDiagnostics", "0")
            .env("DOTNET_EnableWriteXorExecute", "0")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success(), "Renode runs to completion: {out:?}");
    };
    let sorted = |text: &str| {
        let mut v: Vec<String> = text.lines().map(str::to_owned).collect();
        v.sort();
        v
    };
    // Renode's own default, every key, after a whole boot of the reference.
    let plain = dir.join("plain");
    let boot = format!(
        "mach create; machine LoadPlatformDescription @{}; sysbus LoadELF @{}; cpu VectorTableOffset 0x08000000; emulation RunFor \"0.001000000\"; machine Reset; emulation RunFor \"0.000010000\"; peripherals; quit",
        platform().display(),
        firmware().display()
    );
    renode(&plain, None, &boot);
    let default = fs::read_to_string(plain.join(".config/renode/config")).unwrap();
    assert_eq!(
        sorted(&default.replace(plain.to_str().unwrap(), "$HOME")),
        sorted(RENODE_DEFAULT_CONFIG),
        "Renode's default configuration is exactly the pinned table"
    );
    // The same with the two keys changed is read and never written.
    let given = dir.join("given");
    fs::create_dir_all(&given).unwrap();
    let text = RENODE_DEFAULT_CONFIG
        .replace("$HOME", given.to_str().unwrap())
        .replace(
            "use-synchronous-logging = False",
            "use-synchronous-logging = true",
        )
        .replace(
            "log-timestamps-type = Host",
            "log-timestamps-type = Virtual",
        );
    let config = given.join("renode.config");
    fs::write(&config, &text).unwrap();
    renode(&given, Some(&config), &boot);
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        text,
        "Renode only reads the given configuration and never writes it"
    );
}

/// `AET-WO-4200` S2, as built: the reference firmware reads U4's JEDEC ID
/// through SPI1's multiplexer, and Renode's `GenericSpiFlash` logs its eight
/// `Transmit`-path lines for it; the multiplexer routes every byte.
#[test]
fn the_reference_reads_u4_s_jedec_id_witnessed_behind_spi1_s_multiplexer() {
    let (code, v) = boot("reference spi", "10ms");
    assert_eq!(code, Some(0), "{v}");
    let spi = &v["boot"]["spi"][0];
    assert_eq!(spi["reference"], "U4", "{v}");
    assert_eq!(spi["select"], "PA0");
    assert_eq!(spi["command"], "0x9f");
    assert_eq!(spi["status"], "ok");
    assert_eq!(spi["observed"], "0xef4017");
    let w = &v["boot"]["spi_witness"][0];
    assert_eq!(w["device"], "spi1.spi1mux.u4");
    assert_eq!(w["observed"], w["expected"], "{v}");
    assert_eq!(w["expected"].as_array().unwrap().len(), 8);
    assert_eq!(w["unrouted"], serde_json::json!([]));
}

/// U4's read replaced by `forgery` (an expression of type
/// `Result<usize, SpiError>`), with the table's EF 40 17 in the buffer.
fn spi_forged(name: &str, forgery: &'static str) -> PathBuf {
    mutant(name, move |d| {
        edit(
            d,
            "src/main.rs",
            "let mut buf1 = [0u8; 3];",
            "let mut buf1 = [0xefu8, 0x40, 0x17];",
        );
        edit(
            d,
            "src/main.rs",
            "spi1.transfer(&mut r, Spi1Device::U4, &[0x9f], &mut buf1)",
            forgery,
        );
    })
}

/// `AET-WO-4200` D2: a record that claims the flash's identity is witnessed
/// only by the model's lines for that read. Claimed with no transfer, the
/// model logs nothing; after a transfer of another command, it logs that
/// command (and Renode an `[ERROR]` for it), never the read's lines. That
/// `[ERROR]` falls inside U4's chip-select window, so it fails the identity
/// step by name, beside the run's `renode_error_log` (`AET-WO-4210` D-A1).
#[test]
fn a_forged_spi_record_fails_unwitnessed() {
    let elf = spi_forged("spi-forged", "Ok::<usize, SpiError>(3)");
    let (code, v) = boot_elf("spi forged record", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["spi_transaction_unwitnessed"], "{v}");
    assert_eq!(v["boot"]["spi"][0]["status"], "ok");
    assert_eq!(v["boot"]["spi"][0]["observed"], "0xef4017");
    assert_eq!(
        v["boot"]["spi_witness"][0]["observed"],
        serde_json::json!([])
    );
    let m = message(&v, "spi_transaction_unwitnessed");
    assert!(m.contains("U4") && m.contains("PA0"), "{m}");

    let elf = spi_forged(
        "spi-wrong-command",
        "{ let _ = spi1.transfer(&mut r, Spi1Device::U4, &[0x90], &mut [0u8; 3]); Ok::<usize, SpiError>(3) }",
    );
    let (code, v) = boot_elf("spi wrong command", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(
        k.contains(&"spi_step_error") && k.contains(&"renode_error_log"),
        "{v}"
    );
    let m = message(&v, "spi_step_error");
    assert!(
        m.contains("U4 on SPI1, chip select PA0, its identity step")
            && m.ends_with("[ERROR] spi1.spi1mux.u4: Command decoding failed on byte: 0x90 (144)."),
        "{m}"
    );
    let observed = v["boot"]["spi_witness"][0]["observed"].as_array().unwrap();
    assert_eq!(
        observed[0],
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x90, current state: RecognizeOperation"
    );
}

/// `AET-WO-4210` §2.4: the test fixture `imu-baro-1v8.json`, the board at
/// 1.8 V (D7: the ICM-20948's `VDDIO`; the ICP-10111's only supply), with two
/// ICM-20948s (U5, U7) on SPI1 and a BME280 (U2) and an ICP-10111 (U6) on
/// I2C1. KiCad has no symbol for the stand-ins, so `generate` cannot draw it:
/// the board crate comes from its generator and the platform from the twin's
/// emitter, as the CLI makes them.
fn imu_baro_board(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let spec_path = root().join("crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json");
    let bytes = fs::read(&spec_path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    spec.validate().unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files =
        aethelgard_svd_hal::board::generate_with("sentinel-01-board", &solved, &table, &answers)
            .unwrap();
    let dir = std::env::temp_dir().join(format!("aet-4210-t2-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (rel, text) in &files {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
    let platform = dir.join("imu-baro-1v8.repl");
    fs::write(&platform, aethelgard_twin::platform::emit(&solved).unwrap()).unwrap();
    (dir, platform, spec_path)
}

/// `AET-WO-4210` §2.2-§2.4: every device's script runs and each step is
/// witnessed by its own model's lines, on SPI inside its own chip-select
/// window; each data read answers the datasheet's conversion of the judge's
/// stimulus (D3); no interrupt is armed (D6).
#[test]
fn the_imu_and_barometer_fixture_runs_every_script_step_witnessed() {
    let (dir, platform, spec_path) = imu_baro_board("fixture");
    let elf = mutant_of(&dir, "imu-baro", |_| {});
    let (code, v) = boot_on("imu baro", &platform, &spec_path, &elf, "10ms");
    assert_eq!(code, Some(0), "{v}");
    let steps = |bus: &str| -> Vec<(String, String, String)> {
        v["boot"][bus]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["reference"].as_str().unwrap().to_owned(),
                    c["step"].as_str().unwrap().to_owned(),
                    c["observed"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let t = |r: &str, s: &str, o: &str| (r.to_owned(), s.to_owned(), o.to_owned());
    assert_eq!(
        steps("i2c"),
        [
            t("U2", "identity", "0x60"),
            t("U6", "identity", "0x8"),
            t("U6", "configure", "0x0"),
            t("U6", "data", "0x6666"),
        ]
    );
    assert_eq!(
        steps("spi"),
        [
            t("U5", "identity", "0xea"),
            t("U5", "configure", "0x0"),
            t("U5", "data", "0x4000"),
            t("U7", "identity", "0xea"),
            t("U7", "configure", "0x0"),
            t("U7", "data", "0x4000"),
        ]
    );
    for bus in ["i2c_witness", "spi_witness"] {
        for w in v["boot"][bus].as_array().unwrap() {
            assert_eq!(w["observed"], w["expected"], "{w}");
            assert!(!w["expected"].as_array().unwrap().is_empty(), "{w}");
        }
    }
    let spi = v["boot"]["spi_witness"].as_array().unwrap();
    let lines: Vec<usize> = spi
        .iter()
        .map(|w| w["expected"].as_array().unwrap().len())
        .collect();
    assert_eq!(lines, [4, 4, 14, 4, 4, 14]);
    assert!(spi.iter().all(|w| w["unrouted"] == serde_json::json!([])));
    let csr = v["boot"]["registers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "SysTick.CSR")
        .unwrap();
    answer(&format!(
        "imu baro: SysTick.CSR observed {}",
        csr["observed"]
    ));
    // Scotty's ruling on D1: the acknowledged stand-ins are named in a loud
    // banner, in the verdict and in the header of every generated file.
    let banner: Vec<&str> = v["lifecycle"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert_eq!(banner.len(), 2, "{v}");
    assert!(
        banner[0].starts_with("LIFECYCLE EOL: ICP10111 (U6)"),
        "{banner:?}"
    );
    assert!(
        banner[1].starts_with("LIFECYCLE EOL: ICM20948 (U5, U7)"),
        "{banner:?}"
    );
    for rel in [
        "imu-baro-1v8.repl",
        "src/lib.rs",
        "src/board.rs",
        "src/proofs.rs",
        "Cargo.toml",
        "firmware/src/main.rs",
        "firmware/Cargo.toml",
        "firmware/.cargo/config.toml",
        "firmware/link.x",
    ] {
        let text = fs::read_to_string(dir.join(rel)).unwrap();
        for line in &banner {
            assert!(text.contains(line), "{rel} lacks the banner `{line}`");
        }
    }
}

/// `AET-WO-4210` §2.4, one seeded fault per part, each named by its step:
/// - the IMU (U5) never woken: the wake's transfer deleted and its record
///   claimed (F5: the bench reads zeros, the twin's record says OK);
/// - the barometer (U6) measured with the pressure-first command.
#[test]
fn a_seeded_imu_fault_is_named_by_its_step() {
    let (dir, platform, spec_path) = imu_baro_board("imu-fault");
    let elf = mutant_of(&dir, "imu-unwoken", |d| {
        edit(
            d,
            "src/main.rs",
            "spi1.transfer(&mut r, Spi1Device::U5, &[0x06, 0x01], &mut buf5)",
            "Ok::<usize, SpiError>(0)",
        );
    });
    let (code, v) = boot_on("imu unwoken", &platform, &spec_path, &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(
        !k.is_empty() && k.iter().all(|k| *k == "spi_transaction_unwitnessed"),
        "{v}"
    );
    let m = message(&v, "spi_transaction_unwitnessed");
    assert!(
        m.starts_with("U5 on SPI1") && m.contains("its configure step"),
        "{m}"
    );
}

#[test]
fn a_seeded_barometer_fault_is_named_by_its_step() {
    let (dir, platform, spec_path) = imu_baro_board("baro-fault");
    let elf = mutant_of(&dir, "baro-pressure-first", |d| {
        edit(
            d,
            "src/main.rs",
            "i2c1.transfer(&mut r, 0x63, &[0x78, 0x66], &mut buf3)",
            "i2c1.transfer(&mut r, 0x63, &[0x48, 0xa3], &mut buf3)",
        );
    });
    let (code, v) = boot_on("baro pressure first", &platform, &spec_path, &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(k.contains(&"i2c_transaction_unwitnessed"), "{v}");
    let m = message(&v, "i2c_transaction_unwitnessed");
    assert!(
        m.starts_with("U6 at I2C address 0x63") && m.contains("its data step"),
        "{m}"
    );
}

/// `AET-WO-4300a` §1.7: a motor fixture's board, generated by the board
/// crate's generator and the twin's emitter, with `board_edit` applied to
/// the board crate's source, then its firmware built with `firmware_edit`.
/// Returns the platform, the specification and the firmware.
fn motor_board(
    fixture: &str,
    label: &str,
    board_edit: impl Fn(&Path),
    firmware_edit: impl Fn(&Path),
) -> (PathBuf, PathBuf, PathBuf) {
    let spec_path = root().join(format!(
        "crates/aethelgard-cli/tests/fixtures/{fixture}.json"
    ));
    let bytes = fs::read(&spec_path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files =
        aethelgard_svd_hal::board::generate_with("sentinel-01-board", &solved, &table, &answers)
            .unwrap();
    let dir = std::env::temp_dir().join(format!("aet-4300a-t2-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (rel, text) in &files {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
    board_edit(&dir);
    let platform = dir.join("motors.repl");
    fs::write(&platform, aethelgard_twin::platform::emit(&solved).unwrap()).unwrap();
    let elf = mutant_of(&dir, label, firmware_edit);
    (platform, spec_path, elf)
}

fn motors(v: &Value) -> &Vec<Value> {
    v["boot"]["motor"].as_array().unwrap()
}

/// `AET-WO-4300a` §2: the PWM fixture's four outputs, TIM1 CH1..CH4 on
/// PA8..PA11 at 400 Hz from a 1 µs tick, are witnessed in the twin at the
/// model's timing with edges off the 100 µs grid, every judged TIM1 field
/// reads its solved value, and the static check holds (measured, FEEDBACK).
#[test]
fn the_pwm_fixture_s_outputs_are_witnessed_off_the_grid_and_read_back() {
    let (platform, spec, elf) = motor_board("motors-pwm", "pwm", |_| {}, |_| {});
    let (code, v) = boot_on("motors pwm", &platform, &spec, &elf, "100ms");
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(motors(&v).len(), 4);
    for m in motors(&v) {
        let t = &m["twin"];
        assert_eq!(t["status"], "witnessed", "{m}");
        assert_eq!(
            (&t["period_ns"], &t["high_ns"]),
            (&639_744.into(), &256_000.into())
        );
        assert!(t["rises"].as_u64().unwrap() >= 150, "{m}");
        assert!(t["span_ns"].as_u64().unwrap() < 10, "{m}");
        assert!(t["off_grid"].as_u64().unwrap() > 0, "{m}");
        assert_eq!(t["batched"], 0);
        assert_eq!(m["silicon"]["holds"], true);
        assert_eq!(
            (&m["psc"], &m["arr"], &m["ccr"]),
            (&63.into(), &2499.into(), &1000.into())
        );
        // Q3, ruled: the verdict states the speed is not judged.
        assert_eq!(
            m["speed"],
            "motor pin speed not judged: off-board load undeclared"
        );
    }
    let regs = v["boot"]["registers"].as_array().unwrap();
    let tim1 = regs
        .iter()
        .filter(|r| r["name"].as_str().unwrap().starts_with("TIM1."));
    assert_eq!(tim1.clone().count(), 14, "{v}");
    for r in tim1 {
        assert_eq!(r["observed"], r["expected"], "{r}");
    }
}

/// `AET-WO-4300a` §2 and FEEDBACK D1: DShot600's bit timing (PSC 0, ARR
/// 106, CCR 40; 424 ns and 160 ns in the model) is witnessed in a 2 ms boot,
/// the campaign's 100 ms boot being a decision for the architect.
#[test]
fn the_dshot600_fixture_s_outputs_are_witnessed_in_a_two_millisecond_boot() {
    let (platform, spec, elf) = motor_board("motors-dshot600", "dshot", |_| {}, |_| {});
    let (code, v) = boot_on("motors dshot600", &platform, &spec, &elf, "2ms");
    assert_eq!(code, Some(0), "{v}");
    for m in motors(&v) {
        let t = &m["twin"];
        assert_eq!(t["status"], "witnessed", "{m}");
        assert_eq!((&t["period_ns"], &t["high_ns"]), (&424.into(), &160.into()));
        assert!(t["rises"].as_u64().unwrap() > 1000, "{m}");
        assert_eq!(m["silicon"]["holds"], true);
    }
}

/// The raw-register code the forgeries put where the outputs start: a
/// write and a spin of `n` iterations, in the firmware's own crate.
const RAW: &str = "fn wr(a: u32, v: u32) { unsafe { core::ptr::write_volatile(a as *mut u32, v) } }\nfn spin(n: u32) { for _ in 0..n { core::hint::spin_loop(); } }\n\n#[panic_handler]";

/// `AET-WO-4300a` §5 item 1, through Renode: PA8..PA11 driven by `BSRR`
/// stores with the timer never started, and the `UG` + `BSRR` pair the 4300
/// spike left unmeasured (an update event and a store edge in one quantum).
/// Neither is the model's sequence, and the read-back names the timer left
/// stopped and the update interrupt armed.
#[test]
fn adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail() {
    let start = "let _tim1 = tim1.start_outputs(&mut r, &clock, pa8, pa9, pa10, pa11);";
    let bsrr = "let _ = (tim1, clock, pa8, pa9, pa10, pa11);\n    for _ in 0..20 { wr(0x5802_0018, 0x0F00); spin(3_000); wr(0x5802_0018, 0x0F00_0000); spin(9_000); }";
    let ug = "let _ = (tim1, clock, pa8, pa9, pa10, pa11);\n    wr(0x4001_0028, 63); wr(0x4001_002C, 2499); wr(0x4001_000C, 1);\n    for _ in 0..20 { wr(0x4001_0014, 1); wr(0x5802_0018, 0x0F00); spin(3_000); wr(0x5802_0018, 0x0F00_0000); spin(9_000); }";
    for (label, body, register) in [("bsrr", bsrr, "TIM1.CR1"), ("ug-bsrr", ug, "TIM1.DIER")] {
        let (platform, spec, elf) = motor_board(
            "motors-pwm",
            label,
            |_| {},
            |d| {
                edit(d, "src/main.rs", "#[panic_handler]", RAW);
                edit(d, "src/main.rs", start, body);
            },
        );
        let (code, v) = boot_on(label, &platform, &spec, &elf, "100ms");
        assert_eq!(code, Some(1), "{label}: {v}");
        let k = kinds(&v);
        assert!(k.contains(&"motor_timing_mismatch"), "{label}: {v}");
        assert!(k.contains(&"register_mismatch"), "{label}: {v}");
        assert!(
            v["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["message"].as_str().unwrap().starts_with(register)),
            "{label}: {v}"
        );
        // Stores in one quantum share its stamp (`batched`); spread over
        // quanta they miss the sequence (`mismatch`): either way no pass.
        for m in motors(&v) {
            let status = m["twin"]["status"].as_str().unwrap();
            assert!(["mismatch", "batched"].contains(&status), "{label}: {m}");
            // A store right after a timer access (the `UG` write) carries an
            // exact stamp, so a store edge can be off the grid (FEEDBACK M4):
            // the grid alone never passes or fails a witness.
        }
    }
}

/// `AET-WO-4300a` §5 item 1: the timer run at another rate (PSC 31, a 2 MHz
/// tick) while the topology claims PSC 63. The edges are the timer's own and
/// off the grid, but not the solved timing, and `PSC` reads back wrong.
#[test]
fn adversarial_1_a_timer_at_another_rate_with_the_claim_kept_fails() {
    let (platform, spec, elf) = motor_board(
        "motors-pwm",
        "rerated",
        |d| {
            edit(
                d,
                "src/board.rs",
                "r.write(TOKEN, Block::Tim1, 0x28, 0x0000003f); // PSC",
                "r.write(TOKEN, Block::Tim1, 0x28, 0x0000001f); // PSC",
            )
        },
        |_| {},
    );
    let (code, v) = boot_on("rerated", &platform, &spec, &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    assert!(kinds(&v).contains(&"motor_timing_mismatch"), "{v}");
    let psc = v["boot"]["registers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "TIM1.PSC")
        .unwrap();
    assert_eq!(
        (&psc["expected"], &psc["observed"]),
        (&"0x0000003f".into(), &"0x0000001f".into())
    );
}

/// `AET-WO-4300a` §1.5, `motor_output_dead`: PA8 on AF2, not the solved AF1,
/// so TIM1's channel 1 never reaches the pin. MOTOR1 alone is unwitnessed;
/// the alternate-function register reads the firmware's own wrong write.
#[test]
fn motor_output_dead_leaves_its_pin_silent_and_reads_its_wrong_af() {
    let (platform, spec, elf) = motor_board(
        "motors-pwm",
        "dead",
        |d| {
            edit(
                d,
                "src/board.rs",
                "r.modify(TOKEN, Block::GpioA, 0x24, |v| (v & !0x0000000f) | 0x00000001);",
                "r.modify(TOKEN, Block::GpioA, 0x24, |v| (v & !0x0000000f) | 0x00000002);",
            )
        },
        |_| {},
    );
    let (code, v) = boot_on("dead", &platform, &spec, &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let mut k = kinds(&v);
    k.sort_unstable();
    assert_eq!(k, ["motor_output_unwitnessed", "register_mismatch"], "{v}");
    let status: Vec<&str> = motors(&v)
        .iter()
        .map(|m| m["twin"]["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        status,
        ["unwitnessed", "witnessed", "witnessed", "witnessed"]
    );
}

/// `AET-WO-4300a` §1.3: Kani proves the PWM fixture's TIM1 start writes the
/// solved values, in order, and the reset-clock check is exact. The board's
/// whole-schedule harness (`every_valid_sequence`) did not finish in 33.5 min
/// on this board (FEEDBACK), so CI runs the two targeted harnesses.
#[test]
fn kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check() {
    let (platform, _, _) = motor_board("motors-pwm", "prove", |_| {}, |_| {});
    let board = platform.parent().unwrap();
    for harness in [
        "proofs::tim1_start_writes_the_solved_values",
        "proofs::reset_clock_is_ok_exactly_at_reset",
    ] {
        let start = Instant::now();
        let (code, v) = aethelgard(&[
            "prove",
            "--crate",
            board.to_str().unwrap(),
            "--harness",
            harness,
        ]);
        answer(&format!(
            "prove {harness}: exit={code:?} wall_s={:.2}",
            start.elapsed().as_secs_f64()
        ));
        assert_eq!(code, Some(0), "{harness}: {v}");
        assert_eq!(v["verdict"], "pass", "{harness}: {v}");
    }
    // Q2 (ruled): the check reads `RCC_CR.HSIDIV`. A copy whose check
    // leaves it out fails the harness as a failed proof, not a compile
    // error: the negative that shows the harness holds the field.
    let (platform, _, _) = motor_board(
        "motors-pwm",
        "prove-no-hsidiv",
        |d| {
            let p = d.join("src/board.rs");
            let text = fs::read_to_string(&p).unwrap();
            let cut = text.replace("cr & 0x00000018 == 0 && ", "");
            assert_ne!(cut, text, "the check holds HSIDIV");
            fs::write(&p, cut).unwrap();
        },
        |_| {},
    );
    let (code, v) = aethelgard(&[
        "prove",
        "--crate",
        platform.parent().unwrap().to_str().unwrap(),
        "--harness",
        "proofs::reset_clock_is_ok_exactly_at_reset",
    ]);
    answer(&format!("prove reset_clock without HSIDIV: exit={code:?}"));
    assert_eq!(code, Some(1), "{v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert!(
        !kinds.is_empty() && kinds.iter().all(|k| *k == "proof_failed"),
        "{v}"
    );
}

/// `AET-WO-4300a` D1, ruled (a): the DShot600 fixture's campaign boots inside
/// its board's `DSHOT_WINDOW_NS` (2 ms), named on its line; every output is
/// witnessed and both identity reads complete inside it; replay agrees and
/// `attribute` names none. A droop inside the window (a brownout reset at
/// 100 µs, recovered) passes too (third adversarial review, F1).
#[test]
fn the_dshot600_fixture_s_campaign_passes_inside_its_window() {
    let (_, _, elf) = motor_board("motors-dshot600", "campaign", |_| {}, |_| {});
    let archive =
        std::env::temp_dir().join(format!("aet-4300a-t2-{}-dshot-archive", std::process::id()));
    let _ = fs::remove_dir_all(&archive);
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .current_dir(root())
        .args([
            "campaign",
            "--scenarios",
            "scenarios/motors/dshot600.json",
            "scenarios/motors/dshot600-droop.json",
        ])
        .arg("--firmware")
        .arg(&elf)
        .arg("--archive")
        .arg(&archive)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    answer(&format!(
        "campaign motors-dshot600: exit={:?} wall_s={:.2} {lines:?}",
        out.status.code(),
        start.elapsed().as_secs_f64()
    ));
    assert_eq!(out.status.code(), Some(0), "{lines:?}");
    assert_eq!(lines.len(), 3, "{lines:?}");
    for line in &lines[..2] {
        assert_eq!(line["verdict"], "pass", "{line}");
        assert_eq!(line["matched"], true, "{line}");
        assert_eq!(line["virtual_time_ns"], 2_000_000, "{line}");
        assert_eq!(line["window"], "DSHOT_WINDOW_NS", "{line}");
        let run = line["run"].as_str().unwrap();
        let (code, replayed) = aethelgard(&["replay", "--archive", run]);
        assert_eq!(code, Some(0), "{replayed}");
        let (code, attributed) = aethelgard(&["attribute", "--archive", run]);
        assert_eq!(code, Some(0), "{attributed}");
        assert_eq!(attributed["rule"], "N1", "{attributed}");
    }
    let _ = fs::remove_dir_all(&archive);
}

// ---------------------------------------------------------------- AET-WO-4300b

fn serial(v: &Value) -> &Vec<Value> {
    v["boot"]["serial"].as_array().unwrap()
}

/// `AET-WO-4300b` §2: the console, USART1 on PA9/PA10 at 115200 8N1, sends
/// its pattern once and echoes the frame injected one quantum in; its
/// registers read back, Renode's access log shows its 24 writes (the
/// bring-up's 4, 12 pattern bytes, 8 echoed) with PA9 routed at the first
/// `TDR` write, and silicon's tolerance holds (measured, FEEDBACK P2.1,
/// P3.1, P4.2; fix 1).
#[test]
fn the_console_sends_its_pattern_and_echoes_its_frame() {
    let (code, v) = boot("console", "100ms");
    assert_eq!(code, Some(0), "{v}");
    let ports = serial(&v);
    assert_eq!(ports.len(), 1);
    let c = &ports[0];
    assert_eq!(c["label"], "CONSOLE");
    assert_eq!(
        (&c["peripheral"], &c["brr"]),
        (&"USART1".into(), &556.into())
    );
    let t = &c["twin"];
    assert_eq!(t["patterns"], 1, "{c}");
    assert_eq!(t["echo"], "witnessed", "{c}");
    assert_eq!(t["echoed"], 8);
    assert_eq!(t["stray"], false);
    assert_eq!(t["capture_bytes"], 20);
    assert_eq!(t["rx_at_injection"], "PA10 MODER 0b10, AF7");
    assert_eq!(t["tx_at_end"], "PA9 MODER 0b10, AF7");
    assert_eq!(t["tx_at_send"], "PA9 MODER 0b10, AF7");
    assert_eq!(t["usart_writes"], 24, "{c}");
    assert_eq!(t["tags_unexpected"], serde_json::json!([]), "{c}");
    assert_eq!(c["silicon"]["holds"], true);
    for r in v["boot"]["registers"].as_array().unwrap() {
        if r["name"].as_str().unwrap().starts_with("USART1.") {
            assert_eq!(r["observed"], r["expected"], "{r}");
        }
    }
}

/// `AET-WO-4300b` §1.7 and §2: the four-port fixture (MAVLINK 57600,
/// GPS 115200, CRSF 420000 8N1; SBUS 100000 8E2 inverted) witnesses every
/// port's pattern and echo; SBUS's `M0`, `RXINV` and `TXINV` are seen only
/// in the words its own USART's access log shows written (FEEDBACK P2.2,
/// P4.1, P4.2; fix 1).
#[test]
fn the_four_port_fixture_witnesses_every_port_and_sbus_s_tags() {
    let (platform, spec, elf) = motor_board("serial-four", "serial-four", |_| {}, |_| {});
    let (code, v) = boot_on("serial four", &platform, &spec, &elf, "100ms");
    assert_eq!(code, Some(0), "{v}");
    let ports = serial(&v);
    let labels: Vec<&str> = ports.iter().map(|p| p["label"].as_str().unwrap()).collect();
    assert_eq!(labels, ["MAVLINK", "GPS", "CRSF", "SBUS"]);
    for p in ports {
        let t = &p["twin"];
        assert_eq!(
            (&t["patterns"], &t["echo"]),
            (&1.into(), &"witnessed".into()),
            "{p}"
        );
        assert_eq!(p["silicon"]["holds"], true, "{p}");
        assert_eq!(t["tags_missing"], serde_json::json!([]), "{p}");
        assert_eq!(t["tags_unexpected"], serde_json::json!([]), "{p}");
        if p["label"] == "SBUS" {
            assert_eq!(
                t["tags_needed"],
                serde_json::json!(["CR1.M0", "CR2.RXINV", "CR2.TXINV"])
            );
        }
        let label = p["label"].as_str().unwrap();
        assert_eq!(
            t["usart_writes"].as_u64(),
            // The bring-up's 4, the pattern (label then 5 bytes), 8 echoed.
            Some(4 + label.len() as u64 + 5 + 8),
            "{p}"
        );
        assert_eq!(t["tx_at_send"], t["tx_at_end"], "{p}");
    }
}

/// `AET-WO-4300b` §1.5 and §1.7: the four-port fixture's campaign
/// (`scenarios/serial/serial-four.json`, `expect: pass`) passes with every
/// port witnessed, archives its kinds 27 to 29, replays, and `attribute`
/// names none.
#[test]
fn the_four_port_fixture_s_campaign_passes_and_replays() {
    let (_, _, elf) = motor_board("serial-four", "serial-campaign", |_| {}, |_| {});
    let archive = std::env::temp_dir().join(format!(
        "aet-4300b-t2-{}-serial-archive",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&archive);
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .current_dir(root())
        .args([
            "campaign",
            "--scenarios",
            "scenarios/serial/serial-four.json",
        ])
        .arg("--firmware")
        .arg(&elf)
        .arg("--archive")
        .arg(&archive)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    answer(&format!(
        "campaign serial-four: exit={:?} wall_s={:.2} {lines:?}",
        out.status.code(),
        start.elapsed().as_secs_f64()
    ));
    assert_eq!(out.status.code(), Some(0), "{lines:?}");
    assert_eq!(lines.len(), 2, "{lines:?}");
    let line = &lines[0];
    assert_eq!(
        (&line["verdict"], &line["matched"]),
        (&"pass".into(), &true.into()),
        "{line}"
    );
    let run = line["run"].as_str().unwrap();
    let records = aethelgard_chaos::replay_dir(Path::new(run))
        .unwrap()
        .state
        .records;
    let count = |k: u64| records.iter().filter(|r| r.kind() == k).count();
    assert_eq!((count(27), count(28), count(29)), (4, 4, 4), "{records:?}");
    for r in &records {
        if let aethelgard_chaos::record::Record::SerialWitness { label, echo, .. } = r {
            assert_eq!(echo, "witnessed", "{label}");
        }
    }
    let (code, replayed) = aethelgard(&["replay", "--archive", run]);
    assert_eq!(code, Some(0), "{replayed}");
    assert_eq!(replayed["matched"], true, "{replayed}");
    let (code, attributed) = aethelgard(&["attribute", "--archive", run]);
    assert_eq!(code, Some(0), "{attributed}");
    assert_eq!(attributed["rule"], "N1", "{attributed}");
    let _ = fs::remove_dir_all(&archive);
}

/// `AET-WO-4300b` §2, forgery 1: a firmware that writes `TDR` with `UE`
/// clear sends nothing; the pattern is missing and `CR1` reads without `UE`
/// (FEEDBACK P2.3).
#[test]
fn a_tdr_store_with_ue_clear_sends_no_pattern() {
    let elf = mutant_of_board_rs(
        "ue-clear",
        "        r.write(TOKEN, Block::Usart1, 0x00, 0x2000000d); // CR1 | UE\n",
        "",
    );
    let (code, v) = boot_elf("ue clear", &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(k.contains(&"serial_pattern_missing"), "{v}");
    assert!(k.contains(&"register_mismatch"), "{v}");
    assert_eq!(serial(&v)[0]["twin"]["patterns"], 0, "{v}");
}

/// `AET-WO-4300b` §2, forgery 2: bytes in the capture the firmware never
/// received. An echo that sends a constant without reading `RDR` cannot
/// send the frame, which is derived from the firmware's own sha256: the
/// bytes after the pattern are stray (FEEDBACK P6.2).
#[test]
fn an_echo_the_firmware_never_received_is_forged() {
    let elf = mutant("echo-const", |d| {
        edit(
            d,
            "src/main.rs",
            "let Some(byte) = usart1.receive(&mut r, ECHO_POLLS) else {",
            "let Some(byte) = Some(0x11u8) else {",
        )
    });
    let (code, v) = boot_elf("echo const", &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(k.contains(&"serial_capture_forged"), "{v}");
    assert!(k.contains(&"serial_echo_missing"), "{v}");
    assert!(!k.contains(&"serial_pattern_missing"), "{v}");
}

/// `AET-WO-4300b` §2, forgery 3: the four-port board's MAVLINK USART sends
/// GPS's label. Each capture must hold its own port's pattern.
#[test]
fn a_pattern_on_the_wrong_port_fails() {
    let bytes = |s: &str| {
        s.bytes()
            .map(|b| format!("{b:#04x}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let (mavlink, gps) = (bytes("MAVLINK"), bytes("GPS"));
    let (platform, spec, elf) = motor_board(
        "serial-four",
        "serial-wrong-port",
        |_| {},
        |d| {
            edit(
                d,
                "src/main.rs",
                &format!("[{mavlink}]"),
                &format!("[{gps}]"),
            )
        },
    );
    let (code, v) = boot_on("serial wrong port", &platform, &spec, &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let ports = serial(&v);
    assert_eq!(ports[0]["twin"]["patterns"], 0, "{v}");
    for p in &ports[1..] {
        assert_eq!(p["twin"]["echo"], "witnessed", "{p}");
    }
    let k = kinds(&v);
    assert!(k.contains(&"serial_pattern_missing"), "{v}");
    assert!(k.contains(&"serial_capture_forged"), "{v}");
}

/// `AET-WO-4300b` §1.6 (`serial_rx_dead`): the console's RX pin on AF8.
/// Renode's USART has no pin on its path, so the frame still comes back
/// (FEEDBACK P5.1); the judge reads PA10's mux at the injection, does not
/// count the echo, and the read-back names `GPIOA.AFRH`.
#[test]
fn the_console_s_rx_on_the_wrong_af_keeps_the_pattern_and_loses_the_echo() {
    let elf = mutant_of_board_rs(
        "rx-af8",
        "r.modify(TOKEN, Block::GpioA, 0x24, |v| (v & !0x00000f00) | 0x00000700);",
        "r.modify(TOKEN, Block::GpioA, 0x24, |v| (v & !0x00000f00) | 0x00000800);",
    );
    let (code, v) = boot_elf("rx af8", &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let mut k = kinds(&v);
    k.sort();
    assert_eq!(k, ["register_mismatch", "serial_echo_missing"], "{v}");
    let t = &serial(&v)[0]["twin"];
    assert_eq!(t["patterns"], 1, "{v}");
    assert_eq!(t["echo"], "not_counted", "{v}");
    assert_eq!(t["rx_at_injection"], "PA10 MODER 0b10, AF8", "{v}");
}

/// A copy of the reference board crate with `from` replaced by `to` once in
/// its `src/board.rs`, and its firmware built from it.
fn mutant_of_board_rs(name: &str, from: &str, to: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-4300b-t2-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let src = root().join("boards/sentinel-01");
    for rel in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "src/board.rs",
        "src/proofs.rs",
        "firmware/Cargo.toml",
        "firmware/Cargo.lock",
        "firmware/.cargo/config.toml",
        "firmware/link.x",
        "firmware/src/main.rs",
    ] {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(src.join(rel), &to).unwrap();
    }
    edit(&dir, "src/board.rs", from, to);
    mutant_of(&dir, name, |_| {})
}

// ------------------------------------- adversarial pass 1 (fix 1, ported)

/// A copy of the reference board crate with each `board` edit applied once to
/// `src/board.rs`, and its firmware built with each `fw` edit to `src/main.rs`.
fn adv_reference(name: &str, board: &[(&str, &str)], fw: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-4300b-adv-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let src = root().join("boards/sentinel-01");
    for rel in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "src/board.rs",
        "src/proofs.rs",
        "firmware/Cargo.toml",
        "firmware/Cargo.lock",
        "firmware/.cargo/config.toml",
        "firmware/link.x",
        "firmware/src/main.rs",
    ] {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(src.join(rel), &to).unwrap();
    }
    for (f, t) in board {
        edit(&dir, "src/board.rs", f, t);
    }
    let fw: Vec<(String, String)> = fw
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    mutant_of(&dir, name, move |d| {
        for (f, t) in &fw {
            edit(d, "src/main.rs", f, t);
        }
    })
}

/// The finding of `kind`'s messages in `v`.
fn messages<'a>(v: &'a Value, kind: &str) -> Vec<&'a str> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == kind)
        .map(|f| f["message"].as_str().unwrap())
        .collect()
}

/// Adversarial F4 (ADV-1): PA9 stays an input while the pattern goes out and
/// is put on AF7 afterwards. The pattern never reached the pin: the TX pin's
/// mux at the first `TDR` write, rebuilt from the access log, is not routed.
#[test]
fn adv_1_a_tx_pin_routed_only_after_the_pattern_fails() {
    let elf = adv_reference(
        "tx-late",
        &[(
            "r.modify(TOKEN, Block::GpioA, 0x00, |v| (v & !0x000c0000) | 0x00080000);",
            "r.modify(TOKEN, Block::GpioA, 0x00, |v| v & !0x000c0000);",
        )],
        &[(
            "    let mut buf0 = [0u8; 1];\n",
            "    unsafe { let p = 0x5802_0000 as *mut u32; p.write_volatile((p.read_volatile() & !0x000c0000) | 0x00080000); }\n    let mut buf0 = [0u8; 1];\n",
        )],
    );
    let (code, v) = boot_elf("adv tx late", &elf, "100ms");
    answer(&format!("ADV-1 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_pattern_missing"], "{v}");
    let t = &serial(&v)[0]["twin"];
    assert_eq!(t["tx_at_end"], "PA9 MODER 0b10, AF7", "{v}");
    assert_eq!(t["tx_at_send"], "PA9 MODER 0b00, AF7", "{v}");
}

/// Adversarial F3 (ADV-2): USART1's `PRESC` set to /2 before `UE`; Renode
/// stores no `PRESC`, but its access log shows the write.
#[test]
fn adv_2_a_presc_write_in_the_bring_up_fails() {
    let elf = adv_reference(
        "presc",
        &[(
            "        r.write(TOKEN, Block::Usart1, 0x00, 0x2000000c); // CR1\n",
            "        r.write(TOKEN, Block::Usart1, 0x2c, 0x00000001); // PRESC /2\n        r.write(TOKEN, Block::Usart1, 0x00, 0x2000000c); // CR1\n",
        )],
        &[],
    );
    let (code, v) = boot_elf("adv presc", &elf, "100ms");
    answer(&format!("ADV-2 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    assert_eq!(
        serial(&v)[0]["twin"]["tags_unexpected"],
        serde_json::json!(["0x1 to 0x2C"]),
        "{v}"
    );
}

/// Adversarial F3 (ADV-2b): `UE` cleared, `PRESC` /2, `UE` set again, from
/// `main.rs`, where no harness of the board crate looks.
#[test]
fn adv_2b_a_presc_write_from_main_after_the_bring_up_fails() {
    let elf = adv_reference(
        "presc-main",
        &[],
        &[(
            "    // Serial port 0, `CONSOLE`, on USART1: its label's bytes, then the tail.\n",
            "    unsafe { let b = 0x4001_1000usize; ((b) as *mut u32).write_volatile(0x0c); ((b + 0x2c) as *mut u32).write_volatile(1); ((b) as *mut u32).write_volatile(0x0d); }\n    // Serial port 0, `CONSOLE`, on USART1: its label's bytes, then the tail.\n",
        )],
    );
    let (code, v) = boot_elf("adv presc main", &elf, "100ms");
    answer(&format!("ADV-2b code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    assert_eq!(
        serial(&v)[0]["twin"]["tags_unexpected"],
        serde_json::json!(["CR1 0xC after UE", "0x1 to 0x2C", "CR1 0xD after UE"]),
        "{v}"
    );
}

/// Adversarial F1 (ADV-3): SBUS's `CR1` written with `M0`, then again
/// without it before `UE`: the word on silicon is 8 bits, 7 data and parity.
#[test]
fn adv_3_sbus_s_m0_cleared_before_ue_fails() {
    let (platform, spec, elf) = motor_board(
        "serial-four",
        "adv-m0-cleared",
        |d| {
            edit(
                d,
                "src/board.rs",
                "        r.write(TOKEN, Block::Uart4, 0x00, 0x2000140d); // CR1 | UE\n",
                "        r.write(TOKEN, Block::Uart4, 0x00, 0x2000040c); // CR1, M0 cleared\n        r.write(TOKEN, Block::Uart4, 0x00, 0x2000040d); // CR1 | UE, no M0\n",
            )
        },
        |_| {},
    );
    let (code, v) = boot_on("adv m0 cleared", &platform, &spec, &elf, "100ms");
    answer(&format!("ADV-3 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    assert_eq!(
        serial(&v)[3]["twin"]["tags_missing"],
        serde_json::json!(["CR1.M0"]),
        "{v}"
    );
}

/// Adversarial F2 (ADV-4): SBUS's `CR2` written without inversion before
/// `UE`, and with `TXINV` and `RXINV` only after, when RM0433 makes them
/// read-only.
#[test]
fn adv_4_sbus_s_inversion_written_after_ue_fails() {
    let (platform, spec, elf) = motor_board(
        "serial-four",
        "adv-inv-after-ue",
        |d| {
            edit(
                d,
                "src/board.rs",
                "        r.write(TOKEN, Block::Uart4, 0x04, 0x00032000); // CR2\n",
                "        r.write(TOKEN, Block::Uart4, 0x04, 0x00002000); // CR2, not inverted\n",
            );
            edit(
                d,
                "src/board.rs",
                "        r.write(TOKEN, Block::Uart4, 0x00, 0x2000140d); // CR1 | UE\n",
                "        r.write(TOKEN, Block::Uart4, 0x00, 0x2000140d); // CR1 | UE\n        r.write(TOKEN, Block::Uart4, 0x04, 0x00032000); // CR2 inverted, UE set\n",
            );
        },
        |_| {},
    );
    let (code, v) = boot_on("adv inv after ue", &platform, &spec, &elf, "100ms");
    answer(&format!("ADV-4 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    let t = &serial(&v)[3]["twin"];
    assert_eq!(
        t["tags_missing"],
        serde_json::json!(["CR2.RXINV", "CR2.TXINV"]),
        "{v}"
    );
    assert_eq!(
        t["tags_unexpected"],
        serde_json::json!(["CR2 0x32000 after UE"]),
        "{v}"
    );
}

/// Adversarial F12 (ADV-5): the console's pattern also sent on USART2
/// through PA2, everything put back to reset afterwards: a USART the
/// topology does not name, captured all the same.
#[test]
fn adv_5_a_pattern_on_a_usart_no_port_names_is_forged() {
    let elf = adv_reference(
        "unnamed",
        &[],
        &[(
            "    let mut buf0 = [0u8; 1];\n",
            "    unsafe {\n        let rcc = 0x5802_4400usize; let en = (rcc + 0xe8) as *mut u32; en.write_volatile(en.read_volatile() | (1 << 17));\n        let ga = 0x5802_0000usize; let moder = ga as *mut u32; let afrl = (ga + 0x20) as *mut u32;\n        let (m0, a0) = (moder.read_volatile(), afrl.read_volatile());\n        afrl.write_volatile((a0 & !0xf00) | 0x700); moder.write_volatile((m0 & !0x30) | 0x20);\n        let u = 0x4000_4400usize; ((u + 0x0c) as *mut u32).write_volatile(556); (u as *mut u32).write_volatile(0x09);\n        for &b in [0x43u8, 0x4f, 0x4e, 0x53, 0x4f, 0x4c, 0x45, 0x00, 0xfe, 0x7f, 0x80, 0xff].iter() { ((u + 0x28) as *mut u32).write_volatile(u32::from(b)); }\n        (u as *mut u32).write_volatile(0); ((u + 0x0c) as *mut u32).write_volatile(0);\n        moder.write_volatile(m0); afrl.write_volatile(a0); en.write_volatile(en.read_volatile() & !(1 << 17));\n    }\n    let mut buf0 = [0u8; 1];\n",
        )],
    );
    let (code, v) = boot_elf("adv unnamed", &elf, "100ms");
    answer(&format!("ADV-5 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_capture_forged"], "{v}");
    let m = messages(&v, "serial_capture_forged");
    assert!(
        m[0].starts_with(
            "`usart2.bin` holds 12 byte(s) (43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF), sent on a USART no serial port"
        ),
        "{m:?}"
    );
}

/// The PM's ruling on F4's residual: PA9 routed for the pattern's first
/// byte, made an input for the other eleven, routed again before the end.
/// The TX pin must be routed at every `TDR` write of each boot.
#[test]
fn adv_1b_a_tx_pin_unrouted_after_the_first_byte_fails() {
    let elf = adv_reference(
        "tx-unrouted",
        &[],
        &[
            (
                "    for &byte in [0x43, 0x4f, 0x4e, 0x53, 0x4f, 0x4c, 0x45].iter().chain(PATTERN_TAIL.iter()) {\n        if usart1.send(&mut r, byte).is_err() {",
                "    for (k, &byte) in [0x43, 0x4f, 0x4e, 0x53, 0x4f, 0x4c, 0x45].iter().chain(PATTERN_TAIL.iter()).enumerate() {\n        if k == 1 { unsafe { let p = 0x5802_0000 as *mut u32; p.write_volatile(p.read_volatile() & !0x000c0000); } }\n        if usart1.send(&mut r, byte).is_err() {",
            ),
            (
                "    let mut buf0 = [0u8; 1];\n",
                "    unsafe { let p = 0x5802_0000 as *mut u32; p.write_volatile((p.read_volatile() & !0x000c0000) | 0x00080000); }\n    let mut buf0 = [0u8; 1];\n",
            ),
        ],
    );
    let (code, v) = boot_elf("adv tx unrouted", &elf, "100ms");
    answer(&format!("ADV-1b code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_pattern_missing"], "{v}");
    let t = &serial(&v)[0]["twin"];
    assert_eq!(t["patterns"], 1, "{v}");
    assert_eq!(t["tx_at_end"], "PA9 MODER 0b10, AF7", "{v}");
    assert_eq!(t["tx_at_send"], "PA9 MODER 0b00, AF7", "{v}");
}

// ------------------------------------------------- adversarial pass 2 (ADV2)

/// The unexpected writes kind 28 shows for the port, as the verdict names
/// them.
fn unexpected(v: &Value) -> Vec<String> {
    serial(v)[0]["twin"]["tags_unexpected"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// ADV2-1 (A1, the PM's ruling): GPIOA reset through `RCC_AHB4RSTR` after
/// the bring-up routed PA9, the pattern sent with PA9 in its reset mode, then
/// every GPIOA word written back. The reset leaves no line in GPIOA's access
/// log, so the RCC's is read: the reset fails by name.
#[test]
fn adv2_1_a_gpio_reset_through_the_rcc_fails() {
    let elf = adv_reference(
        "rcc-gpio-reset",
        &[],
        &[
            (
                "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n",
                "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n    let saved: [u32; 10] = core::array::from_fn(|i| unsafe { (0x5802_0000 as *const u32).add(i).read_volatile() });\n    unsafe { let rst = 0x5802_4488 as *mut u32; rst.write_volatile(1); rst.write_volatile(0); }\n",
            ),
            (
                "    let mut buf0 = [0u8; 1];\n",
                "    unsafe { let g = 0x5802_0000 as *mut u32; for i in [1usize, 2, 3, 5, 8, 9, 0] { g.add(i).write_volatile(saved[i]); } }\n    let mut buf0 = [0u8; 1];\n",
            ),
        ],
    );
    let (code, v) = boot_elf("adv2 rcc gpio reset", &elf, "100ms");
    answer(&format!("ADV2-1 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    assert_eq!(unexpected(&v), ["RCC.AHB4RSTR.GPIOARST set"], "{v}");
}

/// ADV2-2 (A2, the PM's ruling): PA9 made open-drain with a pull-down
/// after the bring-up routed it: on silicon the TX line never drives high.
/// The pin's `OTYPER`, `OSPEEDR` and `PUPDR`, rebuilt from the access log,
/// must be the bring-up's at every `TDR` write and at the end.
#[test]
fn adv2_2_an_open_drain_pulled_down_tx_fails() {
    let elf = adv_reference(
        "tx-open-drain",
        &[],
        &[(
            "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n",
            "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n    unsafe { let g = 0x5802_0000 as *mut u32; g.add(1).write_volatile(g.add(1).read_volatile() | 1 << 9); g.add(3).write_volatile((g.add(3).read_volatile() & !(3 << 18)) | 2 << 18); }\n",
        )],
    );
    let (code, v) = boot_elf("adv2 tx open drain", &elf, "100ms");
    answer(&format!("ADV2-2 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
    assert_eq!(
        unexpected(&v),
        [
            "PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at a TDR write",
            "PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at the end"
        ],
        "{v}"
    );
}

/// ADV2-3 (held): one `SYSRESETREQ` at the very start. Renode does not
/// reboot the firmware (no reset macro), so the boot word is never set.
#[test]
fn adv2_3_a_software_reset_at_start_fails() {
    let elf = adv_reference(
        "sysresetreq",
        &[],
        &[(
            "    let mut r = unsafe { Mmio::new() };\n",
            "    unsafe { let f = 0x2000_0200 as *mut u32; if f.read_volatile() != 0xA5A5_0001 { f.write_volatile(0xA5A5_0001); (0xE000_ED0C as *mut u32).write_volatile(0x05FA_0004); loop { core::hint::spin_loop(); } } }\n    let mut r = unsafe { Mmio::new() };\n",
        )],
    );
    let (code, v) = boot_elf("adv2 sysresetreq", &elf, "100ms");
    answer(&format!("ADV2-3 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert!(kinds(&v).contains(&"boot_word_missing"), "{v}");
}

/// ADV2-4 (held): one unaligned 32-bit `STR` at `GPIOA + 0x22`, over AFRH's
/// low half (PA9's AFSEL to 0), the pattern, then AFRH written back whole:
/// the rebuild cannot apply it, so the TX pin is not routed at its sends.
#[test]
fn adv2_4_an_unaligned_store_into_afrh_fails() {
    let elf = adv_reference(
        "afrh-unaligned",
        &[],
        &[
            (
                "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n",
                "    let mut usart1 = usart1.configure(&mut r, &clock, pa9, pa10);\n    unsafe { core::arch::asm!(\"str {v}, [{a}]\", v = in(reg) 0x0700_5550u32, a = in(reg) 0x5802_0022u32, options(nostack)); }\n",
            ),
            (
                "    let mut buf0 = [0u8; 1];\n",
                "    unsafe { (0x5802_0024 as *mut u32).write_volatile(0x770); }\n    let mut buf0 = [0u8; 1];\n",
            ),
        ],
    );
    let (code, v) = boot_elf("adv2 afrh unaligned", &elf, "100ms");
    answer(&format!("ADV2-4 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_pattern_missing"], "{v}");
}

/// ADV2-5 (L2, the PM's ruling): PA9 locked by the `LCKR` key sequence in
/// its reset mode before the HAL routes it. Renode ignores the routing
/// writes the access log shows and says so: the refusal fails by name, and
/// the rebuild does not apply the refused writes.
#[test]
fn adv2_5_a_locked_tx_pin_fails() {
    let elf = adv_reference(
        "lckr",
        &[],
        &[(
            "    let pa9 = gpioa.pa9.into_usart1_tx(&mut r);\n",
            "    unsafe { let l = 0x5802_001C as *mut u32; l.write_volatile(0x10200); l.write_volatile(0x200); l.write_volatile(0x10200); let _ = l.read_volatile(); let _ = l.read_volatile(); }\n    let pa9 = gpioa.pa9.into_usart1_tx(&mut r);\n",
        )],
    );
    let (code, v) = boot_elf("adv2 lckr", &elf, "100ms");
    answer(&format!("ADV2-5 code={code:?} kinds={:?}", kinds(&v)));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        [
            "register_mismatch",
            "register_mismatch",
            "serial_tag_mismatch",
            "serial_pattern_missing"
        ],
        "{v}"
    );
    assert!(
        unexpected(&v)
            .iter()
            .any(|x| x == "gpioPortA refused a write to 0x0 (a locked pin)"),
        "{v}"
    );
    assert_eq!(
        serial(&v)[0]["twin"]["tx_at_send"],
        "PA9 MODER 0b00, AF0",
        "{v}"
    );
}

// ---------------------------------------------------------------- AET-WO-4220

/// `AET-WO-4220` §2.3: the Sentinel-02-shaped fixture with the parts of our
/// own, `sentinel-02-own.json`: the BMP581 (U7) on I2C1 beside the BME280
/// and the TMP117, the ICM-42688-P (U5) on its own SPI bus beside the
/// flash's, every part `active`, so no `accept_lifecycle` entry. Its power
/// tree is left out (`power: null`): `campaign` refuses one until
/// `AET-WO-4300d2` emits it, and the twin judges the buses alone. The board
/// crate comes from its generator and the platform from the twin's emitter,
/// as the CLI makes them (KiCad has no symbol for either part yet: D5).
fn own_board(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let spec_path = root().join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json");
    let (solved, files) = common::verify::generated(&spec_path, "sentinel-01-board");
    let dir = std::env::temp_dir().join(format!("aet-4220-t2-{}-{label}", std::process::id()));
    common::verify::write_files(&dir, &files);
    let platform = dir.join("sentinel-02-own.repl");
    fs::write(&platform, aethelgard_twin::platform::emit(&solved).unwrap()).unwrap();
    (dir, platform, spec_path)
}

/// Each bus's steps as the verdict reports them: (reference, step, observed).
fn steps_of(v: &Value, bus: &str) -> Vec<(String, String, String)> {
    v["boot"][bus]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["reference"].as_str().unwrap().to_owned(),
                c["step"].as_str().unwrap().to_owned(),
                c["observed"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// `AET-WO-4220` §2.3, PB6: both models of our own load from the CLI's
/// pinned copies, and every step of their scripts is witnessed by its own
/// one-line transaction; each data read answers the datasheet's conversion
/// of the judge's stimulus, rounded as the datasheet converts (29 degrees C:
/// 529.92 to 530, `0x0212`); no part needs a lifecycle acknowledgement.
#[test]
fn the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed() {
    let (dir, platform, spec_path) = own_board("fixture");
    let text = fs::read_to_string(&platform).unwrap();
    assert_eq!(
        aethelgard_twin::platform::own_models(&text).unwrap(),
        [
            aethelgard_twin::model::Model::Bmp581,
            aethelgard_twin::model::Model::Icm42688p
        ]
    );
    let elf = mutant_of(&dir, "own", |_| {});
    let (code, v) = boot_on("own", &platform, &spec_path, &elf, "100ms");
    assert_eq!(code, Some(0), "{v}");
    let t = |r: &str, s: &str, o: &str| (r.to_owned(), s.to_owned(), o.to_owned());
    assert_eq!(
        steps_of(&v, "i2c"),
        [
            t("U2", "identity", "0x60"),
            t("U7", "identity", "0x50"),
            t("U7", "configure", "0x0"),
            t("U7", "data", "0x1940f362"),
        ]
    );
    assert_eq!(
        steps_of(&v, "spi"),
        [
            t("U4", "identity", "0xef4017"),
            t("U5", "identity", "0x47"),
            t("U5", "configure 1", "0x0"),
            t("U5", "configure 2", "0x0"),
            t("U5", "configure 3", "0x0"),
            t("U5", "data", "0x120200000040"),
        ]
    );
    for bus in ["i2c_witness", "spi_witness"] {
        for w in v["boot"][bus].as_array().unwrap() {
            assert_eq!(w["observed"], w["expected"], "{w}");
            assert!(!w["expected"].as_array().unwrap().is_empty(), "{w}");
            if ["U5", "U7"].contains(&w["reference"].as_str().unwrap()) {
                assert_eq!(w["expected"].as_array().unwrap().len(), 1, "{w}");
            }
        }
    }
    assert!(v.get("lifecycle").is_none(), "{v}");
    answer(&format!(
        "own fixture: instructions {}",
        v["boot"]["instructions"]
    ));
}

/// `AET-WO-4220` PB7: the models sample on their own timer (D8), so a
/// firmware that reads before the cited wait reads the parts' reset values,
/// as silicon would, and fails each data step by its value.
#[test]
fn without_the_cited_wait_each_data_read_answers_its_reset_value() {
    let (dir, platform, spec_path) = own_board("no-wait");
    let elf = mutant_of(&dir, "own-no-wait", |d| {
        edit(d, "src/main.rs", "    settle(640000);\n", "");
        edit(d, "src/main.rs", "    settle(1280000);\n", "");
    });
    let (code, v) = boot_on("own no wait", &platform, &spec_path, &elf, "100ms");
    assert_eq!(code, Some(1), "{v}");
    let data: Vec<(String, String, String)> = steps_of(&v, "i2c")
        .into_iter()
        .chain(steps_of(&v, "spi"))
        .filter(|s| s.1 == "data")
        .collect();
    assert_eq!(
        data,
        [
            ("U7".into(), "data".into(), "0x7f7f7f7f7f7f".into()),
            // `INTF_CONFIG0` 0xC0 sets `FIFO_HOLD_LAST_DATA_EN`, under which a
            // sense register reads 0 until its first sample (DS-000347 12.8,
            // p. 59), not its reset -32768.
            ("U5".into(), "data".into(), "0x0".into()),
        ]
    );
    let k = kinds(&v);
    assert!(k.contains(&"i2c_identity_mismatch"), "{k:?}");
    assert!(k.contains(&"spi_identity_mismatch"), "{k:?}");
}

/// `AET-WO-4220` PB1, D6: the emitted platform loads with its models of our
/// own included from the CLI's copies, and lists them; a header whose pin is
/// not the twin's is refused before Renode loads anything.
#[test]
fn the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused() {
    let (dir, platform, _) = own_board("listing");
    let (code, v) = aethelgard(&["simulate", "--platform", platform.to_str().unwrap()]);
    answer(&format!("own platform: exit={code:?} verdict={v}"));
    assert_eq!(code, Some(0), "{v}");
    let listed: Vec<(&str, &str)> = v["machine"]["peripherals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap()))
        .collect();
    assert!(listed.contains(&("u5", "ICM42688P")), "{listed:?}");
    assert!(listed.contains(&("u7", "BMP581")), "{listed:?}");
    let text = fs::read_to_string(&platform).unwrap();
    let (_, _, sha) = aethelgard_twin::model::OWN[0];
    for (name, changed) in [
        ("wrong-pin", text.replacen(sha, &"0".repeat(64), 1)),
        (
            "no-header",
            text.lines()
                .filter(|l| !l.contains(aethelgard_twin::platform::OWN_LINE))
                .map(|l| format!("{l}\n"))
                .collect(),
        ),
    ] {
        assert_ne!(changed, text, "{name}");
        let path = dir.join(format!("{name}.repl"));
        fs::write(&path, changed).unwrap();
        let (code, v) = aethelgard(&["simulate", "--platform", path.to_str().unwrap()]);
        assert_eq!(code, Some(3), "{name}: {v}");
        assert_eq!(v["error"]["kind"], "ModelUnpinned", "{name}: {v}");
    }
}

/// `AET-WO-4220`: a campaign archives the fixture's run with each of the
/// ICM-42688-P's three configurations as its own step (`configure 1` to
/// `configure 3`, trace version 10), and replay reproduces it: before the
/// names, two records of one device shared the step `configure`, which the
/// archive refuses as a step read twice.
#[test]
fn the_fixture_s_campaign_archives_each_configuration_and_replays() {
    let (dir, _, _) = own_board("campaign");
    let elf = mutant_of(&dir, "own-campaign", |_| {});
    let archive = dir.join("archive");
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "campaign",
            "--scenarios",
            root()
                .join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own-base.json")
                .to_str()
                .unwrap(),
            "--firmware",
            elf.to_str().unwrap(),
            "--archive",
            archive.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    answer(&format!(
        "own campaign: exit={:?} {stdout}",
        out.status.code()
    ));
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let run: Value = stdout
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|l| l["scenario"] == "sentinel-02-own-base")
        .unwrap();
    assert_eq!(run["verdict"], "pass", "{run}");
    let run_dir = PathBuf::from(run["run"].as_str().unwrap());
    let (code, v) = aethelgard(&["replay", "--archive", run_dir.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{v}");
    let replayed = aethelgard_chaos::replay_dir(&run_dir).unwrap();
    let steps: Vec<&str> = replayed
        .state
        .records
        .iter()
        .filter_map(|r| match r {
            aethelgard_chaos::record::Record::SpiCheck {
                reference, step, ..
            } if reference == "U5" => Some(step.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        steps,
        [
            "identity",
            "configure 1",
            "configure 2",
            "configure 3",
            "data"
        ]
    );
}
