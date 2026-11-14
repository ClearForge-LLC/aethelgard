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
    let solved =
        aethelgard_twin::platform::instances(&topology(), &std::collections::BTreeSet::new())
            .unwrap();
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
    // AET-WO-2202: U2's ID register reads 0x60, from the part table; U3 is
    // reported and does not fail the run.
    assert_eq!(
        b["i2c"],
        serde_json::json!([{
            "reference": "U2", "address": "0x76", "register": "0xd0",
            "expected": "0x60", "observed": "0x60", "status": "ok"
        }])
    );
    let unmodelled: Vec<&Value> = first["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "device_unmodelled")
        .collect();
    assert_eq!(unmodelled.len(), 1, "{first}");
    assert_eq!(unmodelled[0]["severity"], "info");
    // AET-WO-2204 D1: Renode's BME280 logged the read the record claims,
    // exactly; D3: the core ended in thread mode.
    assert_eq!(
        b["i2c_witness"],
        serde_json::json!([{
            "peripheral": "I2C1", "reference": "U2", "device": "i2c1.u2",
            "expected": WITNESS, "observed": WITNESS, "absent": []
        }])
    );
    assert_eq!(b["icsr"], "0x00000800");
    assert!(
        unmodelled[0]["message"]
            .as_str()
            .unwrap()
            .contains("U3 (TMP117)")
    );
    assert_eq!(first["findings"].as_array().unwrap().len(), 1, "{first}");
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

/// `AET-WO-2200` §3, the P2 gate's clause 2, as one test: the reference
/// specification at 2.2 kΩ passes, with U2 reading 0x60, and at 47 kΩ fails,
/// with three separate records: the analog evidence (`analog`), the bridge's
/// fault decision (`faults`), and the firmware's own observation, U2 timing
/// out at TXIS (`boot.i2c`, `AET-WO-2202`'s judge). Each case runs twice with
/// byte-identical records. The ns values are `AET-WO-2101` S2's.
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
        let text = spec_text.replace(
            "\"pull_up_ohms\": 2200",
            &format!("\"pull_up_ohms\": {ohms}"),
        );
        let spec = dir.join(format!("spec-{ohms}.json"));
        fs::write(&spec, &text).unwrap();
        let (code, v) = aethelgard(&["solve", "--file", spec.to_str().unwrap()]);
        assert_eq!(code, Some(0), "{v}");
        let topology: SolvedTopology = serde_json::from_value(v).unwrap();
        // The caller's platform is the unfaulted emission, U2 declared (F6).
        let platform = dir.join(format!("platform-{ohms}.repl"));
        let emitted = aethelgard_twin::platform::emit(&topology).unwrap();
        assert!(emitted.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"));
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
        let analog = v["analog"].as_array().unwrap_or_else(|| panic!("{v}"));
        assert_eq!(analog.len(), 2, "{v}");
        for line in analog {
            let measured = line["rise_30_70_ns"].as_f64().unwrap();
            assert!((measured - rise).abs() <= tolerance, "{ohms} ohm: {line}");
            assert_eq!(line["ceiling_ns"], 300);
            assert_eq!(line["complete"], true);
            assert_eq!(line["exceeds"], faulted);
            assert_eq!(line["bus"], "SENSORS");
            assert_eq!(line["ngspice"], "44.2");
        }
        // The boot completed either way: the word is set and the core ran.
        assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007", "{v}");
        assert_eq!(v["boot"]["instructions"]["observed"], 10_000_000);
        let u2 = &v["boot"]["i2c"][0];
        assert_eq!(u2["reference"], "U2", "{v}");
        assert_eq!(u2["address"], "0x76");
        assert_eq!(u2["expected"], "0x60");
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
            assert_eq!(code, Some(1), "{v}");
            assert_eq!(v["verdict"], "fail");
            // The fault reached the platform: Renode built no BME280.
            assert!(!bme, "{v}");
            let faults = v["faults"].as_array().unwrap();
            assert_eq!(faults.len(), 1, "{v}");
            assert_eq!(faults[0]["kind"], "bridge.fault_state");
            assert_eq!(faults[0]["bus"], "SENSORS");
            // The firmware's observation, its own record: U2 timed out at TXIS.
            assert_eq!(u2["status"], "timeout_txis", "{v}");
            assert_eq!(
                kinds,
                [
                    "analog_rise_exceeds_ceiling",
                    "analog_rise_exceeds_ceiling",
                    "i2c_timeout"
                ],
                "{v}"
            );
            for f in &v["findings"].as_array().unwrap()[..2] {
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
            assert!(bme, "{v}");
            assert_eq!(
                (&u2["status"], &u2["observed"]),
                (&"ok".into(), &"0x60".into())
            );
            assert!(kinds.is_empty(), "{v}");
        }
        let (_, again) = run(2);
        for record in ["analog", "faults"] {
            assert_eq!(v[record].to_string(), again[record].to_string(), "{record}");
        }
        assert_eq!(
            v["boot"]["i2c"].to_string(),
            again["boot"]["i2c"].to_string()
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
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_timeout"], "{v}");
    assert_eq!(v["boot"]["i2c"][0]["status"], "timeout_txis");
    let f = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == "i2c_timeout")
        .unwrap();
    let m = f["message"].as_str().unwrap();
    assert!(
        m.contains("U2") && m.contains("0x76") && m.contains("TXIS") && m.contains("0x60"),
        "{m}"
    );
    // The boot itself completed: the word is set, the core ran to the end.
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007");
}

#[test]
fn a_value_one_bit_off_fails_as_an_identity_mismatch() {
    // AET-WO-2202: the firmware's record of what it read is one bit off the
    // table's (0x160 for 0x60). The judge's expectation is the table's.
    let elf = mutant("one-bit", |d| {
        edit(d, "src/main.rs", "fold(0u32,", "fold(1u32,");
    });
    let (code, v) = boot_elf("one bit", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_identity_mismatch"], "{v}");
    assert_eq!(v["boot"]["i2c"][0]["expected"], "0x60");
    assert_eq!(v["boot"]["i2c"][0]["observed"], "0x160");
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
    assert!(emitted.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"));
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
        )
    );
    let (status, _) = record_on(&without, firmware());
    answer(&format!("direct, without the model: status={status}"));
    assert_eq!(
        status,
        format!("{txis:#010x}").to_uppercase().replace("0X", "0x")
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
    assert_eq!(code, Some(1), "{v}");
    // The record carries 0, and the device logged no transaction.
    assert_eq!(
        kinds(&v),
        ["i2c_identity_mismatch", "i2c_transaction_unwitnessed"],
        "{v}"
    );
    assert_eq!(v["boot"]["i2c"][0]["observed"], "0x0");

    let unwritten = mutant("no-record", |d| {
        edit(
            d,
            "src/main.rs",
            "    record(0, status, value);",
            "    let _ = (status, value);",
        );
    });
    let (code, v) = boot_elf("no record", &unwritten, "10ms");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_record_invalid"], "{v}");
    assert_eq!(v["boot"]["i2c"][0]["status"], "invalid");
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
    assert_eq!(kinds(v), ["i2c_transaction_unwitnessed"], "{v}");
    assert_eq!(v["boot"]["i2c"][0]["status"], "ok");
    assert_eq!(v["boot"]["i2c"][0]["observed"], "0x60");
    assert_eq!(
        v["boot"]["i2c_witness"][0]["observed"],
        serde_json::json!([])
    );
    let m = message(v, "i2c_transaction_unwitnessed");
    assert!(
        m.contains("U2") && m.contains("0x76") && m.contains("i2c1.u2"),
        "{m}"
    );
}

#[test]
fn a_forged_record_without_a_transaction_fails_unwitnessed() {
    // AET-WO-2202 W1, now AET-WO-2204 D1: the firmware writes STATUS_OK and
    // the table's 0x60 and never calls `transfer`.
    let elf = forged("forged", "Ok::<usize, I2cError>(1)");
    let (code, v) = boot_elf("forged record", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
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
    assert_eq!(code, Some(1), "{v}");
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
        assert_eq!(code, Some(1), "{label}: {v}");
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
        assert_eq!(code, Some(0), "{v}");
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
    assert_eq!(code, Some(0), "{v}");
    let w = v["boot"]["i2c_witness"].as_array().unwrap();
    assert_eq!(w.len(), 2, "{v}");
    assert_eq!(w[1]["device"], "i2c1.u5");
    assert_eq!(
        w[1]["observed"],
        serde_json::json!(["[NOISY] i2c1.u5: Write D0", "[NOISY] i2c1.u5: Read 60"])
    );
    for (label, elf, who) in [("forged first", first, "U2"), ("forged last", last, "U5")] {
        let (code, v) = boot_on(label, &platform, spec, &elf, "10ms");
        assert_eq!(code, Some(1), "{label}: {v}");
        assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{label}: {v}");
        assert!(
            message(&v, "i2c_transaction_unwitnessed").starts_with(who),
            "{label}: {v}"
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
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["core_in_exception"], "{v}");
    assert!(
        message(&v, "core_in_exception").contains("exception 15"),
        "{v}"
    );
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007");
    assert_eq!(v["boot"]["instructions"]["observed"], 1_000_000);
}

#[test]
fn the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s() {
    // AET-WO-2204 D8 (a): at 1 us no clock is on yet, so the judge's own
    // reads of I2C1 and the GPIO ports log locked accesses. With synchronous
    // logging they come after BEGIN and are not attributed to the firmware.
    let (code, v) = boot("1us", "1us");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing"], "{v}");
}

#[test]
fn a_locked_access_late_in_a_flood_reaches_the_judge_whole() {
    // AET-WO-2204 D8, F2, B10: the spike's `flood-locked`, 37.7 MB of
    // alternating warnings after the boot word, one read of the gated
    // GPIOC.MODER about 600 lines before the run's end (pair 199,461 of the
    // 199,761 measured). Its line reaches the judge, and the markers and
    // values come through whole (a verdict, not exit 5).
    let elf = mutant("flood-locked", |d| {
        edit(
            d,
            "src/main.rs",
            "    // No sleep: the judge requires instructions executed to equal MIPS x time.\n    loop {\n        core::hint::spin_loop();\n    }\n",
            "    let mut n: u32 = 0;\n    loop {\n        unsafe { core::ptr::read_volatile(0x40005424 as *const u32); core::ptr::write_volatile(0x4000541c as *mut u32, 0x10); }\n        if n == 199461 { unsafe { core::ptr::read_volatile(0x5802_0800 as *const u32); } }\n        n = n.wrapping_add(1);\n    }\n",
        );
    });
    let (code, v) = boot_elf("flood-locked", &elf, "10ms");
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(k.contains(&"locked_peripheral_access"), "{v}");
    assert!(
        message(&v, "locked_peripheral_access").contains("0x58020800"),
        "{v}"
    );
    // Past the filter's budget: the witness is unread, never passed (D2).
    assert!(k.contains(&"i2c_transaction_unwitnessed"), "{v}");
    assert!(message(&v, "renode_log_flood").contains("8 MiB"), "{v}");
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
        assert_eq!(code, Some(1), "{name}: {v}");
        assert_eq!(kinds(&v), ["platform_load_error"], "{name}: {v}");
        assert!(
            message(&v, "platform_load_error").contains(error),
            "{name}: {v}"
        );
    }
}

#[test]
fn renode_s_configuration_differs_from_its_default_in_synchronous_logging_alone() {
    // AET-WO-2204 D4: Renode writes every key it uses into the file it is
    // given, and into its default file when given none; the two differ in
    // the one key the CLI's configuration sets.
    let dir = std::env::temp_dir().join(format!("aet-2204-t2-{}-config", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let renode = |home: &Path, config: Option<&Path>| {
        fs::create_dir_all(home).unwrap();
        let mut c = Command::new("renode");
        if let Some(path) = config {
            c.arg("--config").arg(path);
        }
        let out = c
            .args(["--disable-gui", "--console", "--plain", "-e", "quit"])
            .env_clear()
            .env("HOME", home)
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("DOTNET_EnableDiagnostics", "0")
            .env("DOTNET_EnableWriteXorExecute", "0")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    };
    let (plain, given) = (dir.join("plain"), dir.join("given"));
    renode(&plain, None);
    let config = given.join("renode.config");
    fs::create_dir_all(&given).unwrap();
    fs::write(&config, "[general]\nuse-synchronous-logging = true\n").unwrap();
    renode(&given, Some(&config));
    let keys = |text: String, home: &Path| -> Vec<String> {
        let mut v: Vec<String> = text
            .lines()
            .map(|l| l.replace(home.to_str().unwrap(), "$HOME"))
            .collect();
        v.sort();
        v
    };
    let default = keys(
        fs::read_to_string(plain.join(".config/renode/config")).unwrap(),
        &plain,
    );
    let supplied = keys(fs::read_to_string(&config).unwrap(), &given);
    let only = |a: &[String], b: &[String]| -> Vec<String> {
        a.iter().filter(|l| !b.contains(l)).cloned().collect()
    };
    assert_eq!(
        only(&default, &supplied),
        ["use-synchronous-logging = False"]
    );
    assert_eq!(
        only(&supplied, &default),
        ["use-synchronous-logging = true"]
    );
    answer(&format!("renode config keys: {supplied:?}"));
}
