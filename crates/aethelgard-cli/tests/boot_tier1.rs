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

const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} spi1 (STM32H7_SPI)\n";

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
        "0x80004ae".to_owned(),
    ];
    v.extend(registers.iter().map(|r| (*r).to_owned()));
    v
}

fn good() -> String {
    let r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    transcript("", &r.iter().map(String::as_str).collect::<Vec<_>>())
}

/// A `renode` stub that answers `--version`, refuses (exit 99) a command
/// line without the boot's fixed parts, and prints `stdout`.
fn toolbox(stdout: &str) -> Dir {
    let tb = Dir::new("tb");
    let out = tb.write("renode.out", stdout.as_bytes());
    let body = format!(
        r#"#!/bin/sh
if [ "$1" = --version ]; then printf 'Renode v1.17.0\n'; exit 0; fi
case "$5" in
  "emulation SetSeed 2100; mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; emulation RunFor \"0.010000000\"; logLevel 3; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0x58020000;"*"; echo \"AETHELGARD-BOOT-END\"; peripherals; quit") ;;
  *) exit 99;;
esac
/bin/cat '{}'
"#,
        out.display()
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

fn kinds(v: &Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
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
    assert_eq!(b["pc"], "0x080004ae");
    assert_eq!(b["registers"].as_array().unwrap().len(), 7);
    assert_eq!(b["registers"][0]["name"], "GPIOA.MODER");
    assert_eq!(
        b["unobservable"],
        serde_json::json!(["GPIOB.OTYPER", "I2C1.TIMINGR"])
    );
    // The image hash is of the loaded bytes alone: 64 zero bytes.
    assert_eq!(
        b["firmware"]["image_sha256"],
        "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"
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
    let r = reads(
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing"]);
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x00000000");
}

#[test]
fn a_halted_core_fails_and_a_set_word_beside_one_is_untrusted() {
    // The spike's half-failed load: word 0, PC 0xeffffffe, 0 instructions.
    let r = reads(
        "0x00000000",
        "0x0000000000000000",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing", "cpu_halted"]);
    let r = reads(
        "0x2100B007",
        "0x00000000000F423F",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    untrusted(boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
}

#[test]
fn more_instructions_than_the_run_holds_or_another_speed_is_untrusted() {
    let r = reads(
        "0x2100B007",
        "0x00000000000F4241",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    untrusted(boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    // At 1 MIPS, 10,000 instructions would fill the run; the platform is 100.
    let r = reads(
        "0x2100B007",
        "0x0000000000002710",
        "0x00000001",
        &GOOD_REGISTERS,
    );
    untrusted(boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
}

#[test]
fn a_register_off_the_solved_image_fails() {
    let mut regs = GOOD_REGISTERS;
    regs[1] = "0x55600000"; // PA7 on AF6, not the solved AF5
    regs[6] = "0x00001001"; // a clock the topology did not solve
    let r = reads("0x2100B007", "0x00000000000F4240", "0x00000064", &regs);
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["register_mismatch", "register_mismatch"]);
    // Bits outside a pin field's mask do not count.
    let mut regs = GOOD_REGISTERS;
    regs[0] = "0x0000A8FD";
    let r = reads("0x2100B007", "0x00000000000F4240", "0x00000064", &regs);
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn a_touch_of_a_gated_peripheral_fails() {
    let r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let log = "[06:00:00.0000] [WARNING] sysbus: [cpu: 0x8000100] Tried to write a locked peripheral: machine-0.gpioPortA. Address 0x58020000, value 0x5\n";
    let (code, v) = boot(&transcript(
        log,
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
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
