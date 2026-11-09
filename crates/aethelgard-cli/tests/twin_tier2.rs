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

/// The reference specification's I2C devices, as the judge and the generator get them.
fn devices() -> Vec<aethelgard_ir::parts::I2cTarget> {
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(spec()).unwrap()).unwrap();
    aethelgard_ir::parts::i2c_targets(&spec, &topology()).unwrap()
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
    let solved = aethelgard_twin::platform::instances(&topology(), &devices()).unwrap();
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
/// than the emitted one): the monitor script `simulate` builds for a boot, on a
/// platform of the test's own, and the two words of U2's record.
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
    assert_eq!(kinds(&v), ["i2c_identity_mismatch"], "{v}");
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
