//! `AET-WO-2100`, tier 1: `simulate --firmware` against a stub `renode`, with
//! none of the real tools installed. The stub checks the command line's shape
//! (fixed seed, cold RCC, fixed virtual time, reads after `logLevel 3`), then
//! prints a canned transcript, so the judge's every branch runs in `gates`:
//! the three witnesses, the hardened reader (R3), and the refusals before
//! Renode runs (R2, R5, R6).

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

struct Dir(PathBuf);

impl Dir {
    fn new(label: &str) -> Dir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aet-2100-t1-{}-{}-{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Dir(fs::canonicalize(path).unwrap())
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec() -> PathBuf {
    root().join("examples/sentinel-01-minimal.json")
}

fn platform() -> PathBuf {
    root().join("boards/sentinel-01/sentinel-01.repl")
}

/// An ELF32 LE `EM_ARM` `ET_EXEC` with one `PT_LOAD` of `code` at `paddr`.
fn elf_at(paddr: u32, code: &[u8], machine: u16) -> Vec<u8> {
    let mut b = vec![0u8; 52];
    b[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    b[16..18].copy_from_slice(&2u16.to_le_bytes());
    b[18..20].copy_from_slice(&machine.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[28..32].copy_from_slice(&52u32.to_le_bytes());
    b[40..42].copy_from_slice(&52u16.to_le_bytes());
    b[42..44].copy_from_slice(&32u16.to_le_bytes());
    b[44..46].copy_from_slice(&1u16.to_le_bytes());
    let len = code.len() as u32;
    for v in [1u32, 84, paddr, paddr, len, len, 5, 4] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(code);
    b
}

fn elf() -> Vec<u8> {
    elf_at(0x0800_0000, &[0u8; 64], 40)
}

/// The listing of the emitted platform's CPU and every solved peripheral.
const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} gpioPortA (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} gpioPortB (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} i2c1 (STM32F7_I2C)\n  \u{251c}\u{2500}\u{2500} spi1 (STM32H7_SPI)\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n";

/// The reference topology's seven registers as a correct boot leaves them.
const GOOD_REGISTERS: [&str; 7] = [
    "0x0000A801",
    "0x55500000",
    "0x0000A000",
    "0x44000000",
    "0x00000003",
    "0x00200000",
    "0x00001000",
];

/// A transcript: the run's log lines, then the reads between the markers,
/// then the listing. The first line is how Renode echoes the command line,
/// markers and all, so a marker matched by substring would be found there.
fn transcript(run_log: &str, reads: &[&str]) -> String {
    let mut s = String::from(
        "(monitor) emulation SetSeed 2100;  echo \"AETHELGARD-BOOT-BEGIN\";  echo \"AETHELGARD-BOOT-END\"\n",
    );
    s.push_str(run_log);
    s.push_str("AETHELGARD-BOOT-BEGIN\n");
    for r in reads {
        s.push_str(r);
        s.push('\n');
    }
    s.push_str("AETHELGARD-BOOT-END\n");
    s.push_str(CPU_LISTING);
    s
}

fn reads(word: &str, instructions: &str, mips: &str, registers: &[&str]) -> Vec<String> {
    let mut v = vec![
        word.to_owned(),
        instructions.to_owned(),
        mips.to_owned(),
        // Inside the synthetic ELF's one executable segment.
        "0x8000010".to_owned(),
    ];
    v.extend(registers.iter().map(|r| (*r).to_owned()));
    // U2's record: OK, then the BME280's identity.
    v.extend(GOOD_RECORD.iter().map(|r| (*r).to_owned()));
    // I2C1's CR2 and TXDR after that read, as measured on Renode 1.17.0.
    v.extend(GOOD_CONTROLLER.iter().map(|r| (*r).to_owned()));
    v
}

/// `CR2` (`SADD` 0x76 << 1, `RD_WRN`, `NBYTES` 1, `AUTOEND`) and `TXDR` (0xD0).
const GOOD_CONTROLLER: [&str; 2] = ["0x020104EC", "0x000000D0"];

/// The record a firmware that read U2's ID register (0xD0) as 0x60 leaves.
const GOOD_RECORD: [&str; 2] = ["0x22020000", "0x00000060"];

/// A transcript with `log` before the reads.
fn with(log: &str, word: &str, instructions: &str, mips: &str, registers: &[&str]) -> String {
    let r = reads(word, instructions, mips, registers);
    transcript(log, &r.iter().map(String::as_str).collect::<Vec<_>>())
}

fn good() -> String {
    with(
        "",
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    )
}

/// A `renode` stub that answers `--version`, refuses (exit 99) a command
/// line without the boot's fixed parts, and prints `stdout`.
fn toolbox(stdout: &str) -> Dir {
    toolbox_version(stdout, "Renode v1.17.0")
}

fn toolbox_version(stdout: &str, version: &str) -> Dir {
    toolbox_then(stdout, version, "")
}

/// As `toolbox`, with `then` run after the transcript is printed: the stub's
/// way to overrun a bound.
fn toolbox_then(stdout: &str, version: &str, then: &str) -> Dir {
    let tb = Dir::new("tb");
    let out = tb.write("renode.out", stdout.as_bytes());
    let body = format!(
        r#"#!/bin/sh
if [ "$1" = --version ]; then printf '{version}\n'; exit 0; fi
case "$5" in
  "emulation SetSeed 2100; mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; sysbus WriteDoubleWord 0x20000000 0x0; sysbus WriteDoubleWord 0x20000004 0xffffffff; sysbus WriteDoubleWord 0x20000008 0xffffffff; cpu VectorTableOffset 0x08000000; emulation RunFor \"0.010000000\"; logLevel 3; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0x58020000;"*"; echo \"AETHELGARD-BOOT-END\"; peripherals; quit") ;;
  *) exit 99;;
esac
/bin/cat '{out}'
{then}
"#,
        out = out.display()
    );
    let stub = tb.write("renode", body.as_bytes());
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    tb
}

fn run(path: &Path, args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .env_clear()
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn boot_with(path: &Path, platform: &Path, firmware: &Path, time: &str) -> Output {
    run(
        path,
        &[
            OsStr::new("simulate"),
            OsStr::new("--platform"),
            platform.as_os_str(),
            OsStr::new("--firmware"),
            firmware.as_os_str(),
            OsStr::new("--spec"),
            spec().as_os_str(),
            OsStr::new("--for"),
            OsStr::new(time),
        ],
    )
}

/// One boot of a synthetic ELF on the committed platform, with the stub
/// printing `stdout`.
fn boot(stdout: &str) -> (Option<i32>, Value) {
    let tb = toolbox(stdout);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let out = boot_with(&tb.0, &platform(), &fw, "10ms");
    (out.status.code(), report(&out))
}

fn report(out: &Output) -> Value {
    let stdout = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    serde_json::from_str(stdout).unwrap()
}

/// The kinds of the findings that fail a boot: everything but the `info`
/// ones, which report what the twin could not check (`device_unmodelled`).
fn kinds(v: &Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] != "info")
        .map(|f| f["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn untrusted((code, v): (Option<i32>, Value)) {
    assert_eq!(code, Some(5), "{v}");
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
}

#[test]
fn three_agreeing_witnesses_pass() {
    let (code, v) = boot(&good());
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass");
    let b = &v["boot"];
    assert_eq!(b["seed"], 2100);
    assert_eq!(b["virtual_time_ns"], 10_000_000);
    assert_eq!(b["mips"], 100);
    assert_eq!(b["instructions"]["expected"], 1_000_000);
    assert_eq!(b["instructions"]["observed"], 1_000_000);
    assert_eq!(b["boot_word"]["address"], "0x20000000");
    assert_eq!(b["boot_word"]["observed"], "0x2100b007");
    assert_eq!(b["pc"], "0x08000010");
    assert_eq!(b["registers"].as_array().unwrap().len(), 7);
    assert_eq!(b["registers"][0]["name"], "GPIOA.MODER");
    assert_eq!(
        b["unobservable"],
        serde_json::json!(["GPIOB.OTYPER", "I2C1.TIMINGR"])
    );
    // The image hash binds each segment's address and sizes to its bytes:
    // sha256 of 0x08000000, 64, 64 (u32 LE each) and 64 zero bytes, computed
    // outside this crate.
    assert_eq!(
        b["firmware"]["image_sha256"],
        "548141f7eafd58bfe2d127063336109587d6af4f8e099d1649efa842a81dad22"
    );
    assert_ne!(b["firmware"]["sha256"], b["firmware"]["image_sha256"]);
}

#[test]
fn a_stray_carriage_return_around_a_line_is_tolerated() {
    let text = good()
        .replace(
            "\nAETHELGARD-BOOT-BEGIN\n",
            "\n\rAETHELGARD-BOOT-BEGIN\r\r\n",
        )
        .replace("0x2100B007\n", "0x2100B007\r\n");
    let (code, v) = boot(&text);
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn log_text_inside_a_value_is_refused() {
    let text = good().replace(
        "0x0000A801\n",
        "0x00[05:58:25.9357] [WARNING] sysbus: x\n00A801\n",
    );
    untrusted(boot(&text));
    let text = good().replace("0x2100B007\n", "0x2100\rB007\n");
    untrusted(boot(&text));
    let text = good().replace("0x2100B007\n", "value 0x2100B007\n");
    untrusted(boot(&text));
}

#[test]
fn markers_must_appear_once_each_as_whole_lines() {
    untrusted(boot(&good().replace("AETHELGARD-BOOT-END\n", "")));
    untrusted(boot(&good().replace(
        "AETHELGARD-BOOT-BEGIN\n",
        "AETHELGARD-BOOT-BEGIN\nAETHELGARD-BOOT-BEGIN\n",
    )));
    // A missing read leaves one line fewer between the markers.
    untrusted(boot(&good().replace("0x00001000\n", "")));
    // With no listing after the reads, no CPU witnessed the run.
    untrusted(boot(&good().replace(CPU_LISTING, "")));
}

#[test]
fn a_boot_word_never_written_fails() {
    let (code, v) = boot(&with(
        "",
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing"]);
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x00000000");
}

#[test]
fn a_halted_core_fails_and_a_set_word_beside_one_is_untrusted() {
    // The spike's half-failed load: word 0, PC 0xeffffffe, 0 instructions.
    let (code, v) = boot(&with(
        "",
        "0x00000000",
        "0x0000000000000000",
        "0x00000064",
        &GOOD_REGISTERS,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing", "cpu_halted"]);
    untrusted(boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F423F",
        "0x00000064",
        &GOOD_REGISTERS,
    )));
}

#[test]
fn more_instructions_than_the_run_holds_or_another_speed_is_untrusted() {
    untrusted(boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F4241",
        "0x00000064",
        &GOOD_REGISTERS,
    )));
    // At 1 MIPS, 10,000 instructions would fill the run; the platform is 100.
    untrusted(boot(&with(
        "",
        "0x2100B007",
        "0x0000000000002710",
        "0x00000001",
        &GOOD_REGISTERS,
    )));
}

#[test]
fn a_register_off_the_solved_image_fails() {
    let mut regs = GOOD_REGISTERS;
    regs[1] = "0x55600000"; // PA7 on AF6, not the solved AF5
    regs[6] = "0x00001001"; // a clock the topology did not solve
    let (code, v) = boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &regs,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["register_mismatch", "register_mismatch"]);
    // Bits outside a pin field's mask do not count.
    let mut regs = GOOD_REGISTERS;
    regs[0] = "0x0000A8FD";
    let (code, v) = boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &regs,
    ));
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn a_core_ending_outside_the_firmware_fails() {
    // Adversarial A7: a jump into zero-filled external memory after the word.
    let (code, v) = boot(&good().replace("0x8000010\n", "0x701e7bca\n"));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["pc_outside_firmware"]);
    // The segment's last instruction is inside; its end is not.
    let (code, v) = boot(&good().replace("0x8000010\n", "0x800003e\n"));
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = boot(&good().replace("0x8000010\n", "0x8000040\n"));
    assert_eq!(code, Some(1), "{v}");
}

#[test]
fn a_listing_without_a_solved_peripheral_is_untrusted() {
    let listing = CPU_LISTING.replace("  \u{251c}\u{2500}\u{2500} i2c1 (STM32F7_I2C)\n", "");
    untrusted(boot(&good().replace(CPU_LISTING, &listing)));
    let wrong = CPU_LISTING.replace("spi1 (STM32H7_SPI)", "spi1 (STM32F4_SPI)");
    untrusted(boot(&good().replace(CPU_LISTING, &wrong)));
}

#[test]
fn another_renode_is_unusable_for_a_boot() {
    let tb = toolbox_version(&good(), "Renode v1.16.1");
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let out = boot_with(&tb.0, &platform(), &fw, "10ms");
    let v = report(&out);
    assert_eq!(out.status.code(), Some(4), "{v}");
    assert_eq!(v["error"]["kind"], "ToolUnusable", "{v}");
}

#[test]
fn a_touch_of_a_gated_peripheral_fails() {
    let log = "[06:00:00.0000] [WARNING] sysbus: [cpu: 0x8000100] Tried to write a locked peripheral: machine-0.gpioPortA. Address 0x58020000, value 0x5\n";
    let (code, v) = boot(&with(
        log,
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["locked_peripheral_access"]);
}

#[test]
fn a_failed_firmware_load_fails_and_names_itself() {
    let text = "There was an error executing command 'sysbus LoadELF @/x/firmware.elf': \nError while loading ELF: Could not load ELF from path\n";
    let (code, v) = boot(text);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["firmware_load_error"]);
    assert!(v["boot"]["boot_word"]["observed"].is_null(), "{v}");
    // A load error beside reads is two witnesses disagreeing.
    untrusted(boot(&format!("{text}{}", good())));
    // An error in any other command is not a verdict.
    untrusted(boot(
        "There was an error executing command 'echo \"AETHELGARD-BOOT-BEGIN\"': \nx\n",
    ));
}

#[test]
fn inputs_are_refused_before_renode_is_looked_for() {
    // No `renode` on PATH at all: each refusal still comes first (exit 3).
    let empty = Dir::new("empty");
    let input = Dir::new("in");
    let refused = |platform: &Path, firmware: &Path, kind: &str| {
        let out = boot_with(&empty.0, platform, firmware, "10ms");
        let v = report(&out);
        assert_eq!(out.status.code(), Some(3), "{v}");
        assert_eq!(v["error"]["kind"], kind, "{v}");
    };
    let good = input.write("good.elf", &elf());
    refused(
        &platform(),
        &input.write("x86.elf", &elf_at(0x0800_0000, &[0; 4], 62)),
        "MalformedFirmware",
    );
    refused(
        &platform(),
        &input.write("junk.elf", b"not an elf"),
        "MalformedFirmware",
    );
    refused(
        &platform(),
        &input.write("far.elf", &elf_at(0x0A00_0000, &[0; 4], 40)),
        "FirmwareOutsideMemory",
    );
    // Adversarial A2: the magic loaded straight into the boot word.
    refused(
        &platform(),
        &input.write(
            "word.elf",
            &elf_at(0x2000_0000, &0x2100_B007u32.to_le_bytes(), 40),
        ),
        "FirmwareLoadsBootWord",
    );
    let mut edited = fs::read(platform()).unwrap();
    edited.extend_from_slice(b"\n");
    refused(
        &input.write("edited.repl", &edited),
        &good,
        "PlatformNotEmitted",
    );
    refused(
        Path::new("@platforms/cpus/stm32h753.repl"),
        &good,
        "PlatformNotEmitted",
    );
    // With the inputs good, the missing tool is what stops it.
    let out = boot_with(&empty.0, &platform(), &good, "10ms");
    assert_eq!(out.status.code(), Some(4), "{}", report(&out));
}

#[test]
fn the_virtual_time_is_an_integer_unit_up_to_100_ms() {
    let tb = toolbox(&good());
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    for bad in [
        "101ms",
        "0ms",
        "010ms",
        "10s",
        "ms",
        "10 ms",
        "15ns",
        "-1ms",
        "18446744073709551615ms",
    ] {
        let out = boot_with(&tb.0, &platform(), &fw, bad);
        assert_eq!(out.status.code(), Some(64), "{bad}: {}", report(&out));
    }
    for ok in ["100ms", "100000us", "10ns"] {
        let out = boot_with(&tb.0, &platform(), &fw, ok);
        // The stub refuses any RunFor but 10 ms, so a parsed time reaches it.
        assert_ne!(out.status.code(), Some(64), "{ok}");
    }
}

#[test]
fn the_boot_flags_come_together_or_not_at_all() {
    let tb = toolbox(&good());
    for args in [
        vec!["simulate", "--platform", "p", "--firmware", "f"],
        vec!["simulate", "--platform", "p", "--spec", "s"],
        vec!["simulate", "--platform", "p", "--for", "10ms"],
        vec![
            "simulate",
            "--platform",
            "p",
            "--firmware",
            "f",
            "--spec",
            "s",
        ],
    ] {
        let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
        let out = run(&tb.0, &args);
        assert_eq!(out.status.code(), Some(64), "{}", report(&out));
    }
}

/// A boot the reaper kills at a bound: the stub's output, then `then`.
fn bounded(output: &str, then: &str) -> (Option<i32>, Value, std::time::Duration) {
    let tb = toolbox_then(output, "Renode v1.17.0", then);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let start = std::time::Instant::now();
    let out = boot_with(&tb.0, &platform(), &fw, "10ms");
    (out.status.code(), report(&out), start.elapsed())
}

/// About 1.3 GB held in a shell variable, past the 1 GiB cap.
const HOG: &str = "x=$(/usr/bin/yes | /usr/bin/head -c 1300000000); /bin/sleep 60";

#[test]
fn a_boot_past_the_memory_cap_fails_and_names_it() {
    let (code, v, took) = bounded("", HOG);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["memory_cap_exceeded"]);
    assert_eq!(v["tool"]["exit_code"], -1);
    assert!(v["boot"]["boot_word"]["observed"].is_null(), "{v}");
    assert!(took.as_secs() < 30, "{took:?}");
}

#[test]
fn an_aborted_machine_killed_at_a_bound_is_cpu_aborted() {
    let log = "[08:06:18.9274] [ERROR] machine-0: This machine has aborted. Emulation cannot continue until this machine is removed.\n";
    let (code, v, _) = bounded(log, HOG);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["cpu_aborted"]);
}

#[test]
fn a_boot_past_the_wall_bound_fails_and_names_it() {
    // The one slow test: the bound is 30 s (`renode::BOOT_BOUNDS`).
    let (code, v, took) = bounded("", "/bin/sleep 60");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["wall_bound_exceeded"]);
    assert!(took.as_secs() >= 30 && took.as_secs() < 35, "{took:?}");
}

// `AET-WO-2202`: the identity reads, judged from the part table.

/// A boot that reached the fixed point with `record` in U2's record.
fn with_record(record: [&str; 2]) -> String {
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let n = r.len();
    r[n - 4] = record[0].to_owned();
    r[n - 3] = record[1].to_owned();
    transcript("", &r.iter().map(String::as_str).collect::<Vec<_>>())
}

/// A boot that reached the fixed point, with a good record and `controller`
/// in I2C1's `CR2` and `TXDR`.
fn with_controller(controller: [&str; 2]) -> String {
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let n = r.len();
    r[n - 2] = controller[0].to_owned();
    r[n - 1] = controller[1].to_owned();
    transcript("", &r.iter().map(String::as_str).collect::<Vec<_>>())
}

fn finding<'a>(v: &'a Value, kind: &str) -> Option<&'a Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == kind)
}

#[test]
fn a_passing_identity_read_passes_and_the_unmodelled_part_is_reported_as_info() {
    let (code, v) = boot(&good());
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass");
    assert_eq!(
        v["boot"]["i2c"],
        serde_json::json!([{
            "reference": "U2", "address": "0x76", "register": "0xd0",
            "expected": "0x60", "observed": "0x60", "status": "ok"
        }])
    );
    // U3 (TMP117) is listed, not passed silently, and does not fail the run.
    let f = finding(&v, "device_unmodelled").expect("U3 is reported");
    assert_eq!(f["severity"], "info");
    assert!(f["message"].as_str().unwrap().contains("U3 (TMP117)"));
    assert_eq!(v["findings"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn a_value_that_is_not_the_part_tables_fails_as_an_identity_mismatch() {
    for value in ["0x00000061", "0x00000000", "0x00000160", "0xFFFFFFFF"] {
        let (code, v) = boot(&with_record(["0x22020000", value]));
        assert_eq!(code, Some(1), "{value}: {v}");
        let f = finding(&v, "i2c_identity_mismatch").unwrap_or_else(|| panic!("{value}: {v}"));
        assert_eq!(f["severity"], "error");
        assert!(f["message"].as_str().unwrap().contains("U2"), "{f}");
        assert_eq!(v["boot"]["i2c"][0]["status"], "ok");
    }
}

#[test]
fn a_wait_that_expired_fails_as_a_timeout_naming_the_wait() {
    for (status, stage, name) in [
        ("0x22020001", "TXIS", "timeout_txis"),
        ("0x22020002", "TC", "timeout_tc"),
        ("0x22020003", "RXNE", "timeout_rxne"),
        ("0x22020004", "STOPF", "timeout_stopf"),
    ] {
        // Even with the right value beside it: the status decides.
        let (code, v) = boot(&with_record([status, "0x00000060"]));
        assert_eq!(code, Some(1), "{stage}: {v}");
        let f = finding(&v, "i2c_timeout").unwrap_or_else(|| panic!("{stage}: {v}"));
        assert!(f["message"].as_str().unwrap().contains(stage), "{f}");
        assert!(finding(&v, "i2c_identity_mismatch").is_none());
        assert_eq!(v["boot"]["i2c"][0]["status"], name);
    }
}

#[test]
fn a_record_the_firmware_never_wrote_or_wrote_oddly_fails() {
    // Adversarial 1: the judge fills every record with 0xFFFFFFFF before the
    // run, so a skipped read, or one preloaded in the image, reads as the fill.
    for record in [
        ["0xFFFFFFFF", "0xFFFFFFFF"],
        ["0xFFFFFFFF", "0x00000060"],
        ["0x00000000", "0x00000060"],
        ["0x22020005", "0x00000060"],
        ["0x00000060", "0x22020000"],
    ] {
        let (code, v) = boot(&with_record(record));
        assert_eq!(code, Some(1), "{record:?}: {v}");
        assert!(
            finding(&v, "i2c_record_invalid").is_some(),
            "{record:?}: {v}"
        );
        assert_eq!(v["boot"]["i2c"][0]["status"], "invalid");
    }
}

#[test]
fn the_expected_value_is_the_tables_whatever_the_record_says_it_expects() {
    // Adversarial 2: nothing the firmware writes names the expected value.
    // The judge's side of the verdict is the table's 0x60 for every record.
    let (_, v) = boot(&with_record(["0x22020000", "0x00000061"]));
    assert_eq!(v["boot"]["i2c"][0]["expected"], "0x60");
    assert_eq!(v["boot"]["i2c"][0]["observed"], "0x61");
}

#[test]
fn a_record_is_not_judged_when_the_boot_word_is_missing() {
    let r = reads(
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let mut r = r;
    let n = r.len();
    r[n - 4] = "0xFFFFFFFF".into();
    r[n - 3] = "0xFFFFFFFF".into();
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert!(finding(&v, "boot_word_missing").is_some());
    assert!(finding(&v, "i2c_record_invalid").is_none(), "{v}");
    // The unmodelled device is still reported on a failed boot.
    assert!(finding(&v, "device_unmodelled").is_some());
}

// `AET-WO-2202` W1: the record is the firmware's word; the controller is the
// witness. The judge computes `CR2` and `TXDR` from the solved address and the
// part table (0x76, read, 1 byte, AUTOEND; register 0xD0).

#[test]
fn a_controller_that_shows_the_read_passes_and_is_reported() {
    let (code, v) = boot(&good());
    assert_eq!(code, Some(0), "{v}");
    let w = &v["boot"]["i2c_witness"][0];
    assert_eq!(
        (w["peripheral"].as_str(), w["reference"].as_str()),
        (Some("I2C1"), Some("U2"))
    );
    assert_eq!(w["cr2"]["address"], "0x40005404");
    assert_eq!(w["cr2"]["expected"], "0x020104ec");
    assert_eq!(w["cr2"]["observed"], "0x020104ec");
    assert_eq!(w["txdr"]["address"], "0x40005428");
    assert_eq!(w["txdr"]["expected"], "0x000000d0");
    // START is not stored once set, and bits outside the fields do not count.
    let (code, v) = boot(&with_controller(["0x020124EC", "0x000001D0"]));
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn each_field_of_the_controller_state_fails_alone() {
    for (cr2, txdr, field) in [
        ("0x020104EE", "0x000000D0", "SADD reads 0xee, expected 0xec"),
        ("0x020100EC", "0x000000D0", "RD_WRN reads 0x0, expected 0x1"),
        ("0x020204EC", "0x000000D0", "NBYTES reads 0x2, expected 0x1"),
        (
            "0x000104EC",
            "0x000000D0",
            "AUTOEND reads 0x0, expected 0x1",
        ),
        ("0x020104EC", "0x000000D1", "TXDR reads 0xd1, expected 0xd0"),
    ] {
        let (code, v) = boot(&with_controller([cr2, txdr]));
        assert_eq!(code, Some(1), "{field}: {v}");
        assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{field}: {v}");
        let f = finding(&v, "i2c_transaction_unwitnessed").unwrap();
        let m = f["message"].as_str().unwrap();
        assert!(
            m.contains("U2") && m.contains("0x76") && m.contains("I2C1"),
            "{m}"
        );
        assert!(m.contains(field), "{field}: {m}");
        assert_eq!(f["severity"], "error");
    }
}

#[test]
fn a_forged_record_over_an_untouched_controller_fails_on_every_field() {
    // OK and the table's value written with no transaction: CR2 and TXDR are 0.
    let (code, v) = boot(&with_controller(["0x00000000", "0x00000000"]));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{v}");
    let m = finding(&v, "i2c_transaction_unwitnessed").unwrap()["message"]
        .as_str()
        .unwrap();
    for field in ["SADD", "RD_WRN", "NBYTES", "AUTOEND", "TXDR"] {
        assert!(m.contains(field), "{field}: {m}");
    }
}

#[test]
fn the_controller_is_not_judged_for_a_record_that_claims_nothing() {
    // A wait that expired leaves the controller wherever the failed transfer
    // left it; `i2c_timeout` alone names the failure.
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let n = r.len();
    r[n - 4] = "0x22020001".into();
    r[n - 2] = "0x00000000".into();
    r[n - 1] = "0x00000000".into();
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_timeout"], "{v}");
    // Nor when the boot word is missing.
    let mut r = reads(
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let n = r.len();
    r[n - 2] = "0x00000000".into();
    let (_, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(finding(&v, "i2c_transaction_unwitnessed").is_none(), "{v}");
}
