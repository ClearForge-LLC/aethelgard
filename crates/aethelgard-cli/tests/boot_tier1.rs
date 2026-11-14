//! `AET-WO-2100`, tier 1: `simulate --firmware` against a stub `renode`, with
//! none of the real tools installed. The stub checks the command line's shape
//! (the configuration file and its synchronous logging, fixed seed, cold RCC,
//! the device's log level, fixed virtual time, no console level changed),
//! then prints a canned transcript, so the judge's every branch runs in
//! `gates`: the witnesses, the hardened reader (R3), and the refusals before
//! Renode runs (R2, R5, R6).

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

mod common;

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

/// 64 bytes at the flash's start with one `yield; b.n` self-loop at
/// `0x08000010`, where the stub's transcripts leave the core.
fn code() -> [u8; 64] {
    let mut c = [0u8; 64];
    c[0x10..0x14].copy_from_slice(&[0x10, 0xBF, 0xFD, 0xE7]);
    c
}

fn elf() -> Vec<u8> {
    elf_at(0x0800_0000, &code(), 40)
}

/// The listing of the emitted platform's CPU and every solved peripheral.
const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} gpioPortA (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} gpioPortB (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} i2c1 (STM32F7_I2C)\n  \u{251c}\u{2500}\u{2500} spi1 (STM32H7_SPI)\n  \u{2502}   \u{2514}\u{2500}\u{2500} spi1mux (SPIMultiplexer)\n  \u{2502}       \u{2514}\u{2500}\u{2500} u4 (GenericSpiFlash)\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n";

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
        // On the synthetic ELF's self-loop.
        "0x8000010".to_owned(),
        // ICSR: thread mode (VECTACTIVE 0), as measured on Renode 1.17.0.
        "0x00000800".to_owned(),
        // The ELF's one self-loop, as the machine still holds it.
        "0xBF10".to_owned(),
        "0xE7FD".to_owned(),
    ];
    v.extend(registers.iter().map(|r| (*r).to_owned()));
    // U2's record: OK, then the BME280's identity; then U4's, the flash's
    // JEDEC ID (`AET-WO-4200`).
    v.extend(GOOD_RECORD.iter().map(|r| (*r).to_owned()));
    v.extend(GOOD_SPI_RECORD.iter().map(|r| (*r).to_owned()));
    v
}

/// The record a firmware that read U4's JEDEC ID (0x9F) as EF 40 17 leaves.
const GOOD_SPI_RECORD: [&str; 2] = ["0x22020000", "0x00EF4017"];

/// What Renode 1.17.0's `GenericSpiFlash` logs for U4's JEDEC ID read behind
/// SPI1's multiplexer (`AET-WO-4200` S2, measured).
const GOOD_SPI_WITNESS: &str = "[03:07:39.4001] [NOISY] spi1.spi1mux.u4: Transmitting data 0x9F, current state: RecognizeOperation\n\
[03:07:39.4002] [NOISY] spi1.spi1mux.u4: Decoded operation: Operation: ReadID, write enabled False\n\
[03:07:39.4003] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[03:07:39.4004] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0xEF\n\
[03:07:39.4005] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[03:07:39.4006] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x40\n\
[03:07:39.4007] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[03:07:39.4008] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x17\n";

/// What Renode 1.17.0's BME280 logs for U2's identity read (`AET-WO-2204`
/// S1, measured): the run's witness, before `BEGIN`.
const GOOD_WITNESS: &str =
    "[03:07:39.3762] [NOISY] i2c1.u2: Write D0\n[03:07:39.3765] [NOISY] i2c1.u2: Read 60\n";

/// The record a firmware that read U2's ID register (0xD0) as 0x60 leaves.
const GOOD_RECORD: [&str; 2] = ["0x22020000", "0x00000060"];

/// A transcript with the device's witness, then `log`, before the reads.
fn with(log: &str, word: &str, instructions: &str, mips: &str, registers: &[&str]) -> String {
    let r = reads(word, instructions, mips, registers);
    transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}{log}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    )
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

/// A `renode` stub that refuses (exit 97) a call without `--config` and a
/// configuration with synchronous logging and host timestamps (`AET-WO-2204` D4),
/// answers `--version`, refuses (exit 99) a command line without the boot's
/// fixed parts, and prints `stdout`.
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
[ "$1" = --config ] || exit 97
/bin/grep -qx 'use-synchronous-logging = true' "$2" || exit 97
/bin/grep -qx 'log-timestamps-type = Host' "$2" || exit 97
if [ "$3" = --version ]; then printf '{version}\n  build: 1.17.0+20260907gitf1dd1b4af\n'; exit 0; fi
# No console level is ever set (`AET-WO-2204` D4); only a device's source level.
printf '%s' "$7" | /bin/grep -Eq 'logLevel [0-3]|logLevel -1 [^s]' && exit 96
case "$7" in
  "emulation SetSeed 2100; mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; sysbus WriteDoubleWord 0x20000000 0x0; sysbus WriteDoubleWord 0x20000004 0xffffffff; sysbus WriteDoubleWord 0x20000008 0xffffffff; sysbus WriteDoubleWord 0x2000000c 0xffffffff; sysbus WriteDoubleWord 0x20000010 0xffffffff; cpu VectorTableOffset 0x08000000; "*"emulation RunFor \"0.010000000\"; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0xe000ed04; sysbus ReadWord 0x08000010; sysbus ReadWord 0x08000012; sysbus ReadDoubleWord 0x58020000;"*"; echo \"AETHELGARD-BOOT-END\"; peripherals; quit") ;;
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

/// A bridge stub that measures both reference lines well inside their
/// 300 ns ceiling, as the real bridge does at 2.2 kΩ.
const PASSING_BRIDGE: &str = r#"#!/bin/sh
printf '%s\n' '{"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":186.4}' '{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","rise_30_70_ns":186.4}'
"#;

/// The CLI, hard-linked into a directory of its own beside a stub
/// `aethelgard-bridge` with body `bridge`, or with no bridge at all: the
/// CLI runs the bridge from its own directory (`AET-WO-2200` §1.4), and the
/// pre-pass runs before every boot. One directory per distinct stub.
fn cli(bridge: Option<&'static str>) -> PathBuf {
    static MADE: Mutex<Vec<(Option<&'static str>, PathBuf)>> = Mutex::new(Vec::new());
    let mut made = MADE.lock().unwrap();
    if let Some((_, path)) = made.iter().find(|(b, _)| *b == bridge) {
        return path.clone();
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "aet-2200-cli-{}-{}",
        std::process::id(),
        made.len()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let me = dir.join("aethelgard");
    if fs::hard_link(env!("CARGO_BIN_EXE_aethelgard"), &me).is_err() {
        fs::copy(env!("CARGO_BIN_EXE_aethelgard"), &me).unwrap();
    }
    if let Some(body) = bridge {
        let stub = dir.join("aethelgard-bridge");
        fs::write(&stub, common::identified_stub(body)).unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    }
    made.push((bridge, me.clone()));
    me
}

fn run(path: &Path, args: &[&OsStr]) -> Output {
    run_cli(&cli(Some(PASSING_BRIDGE)), path, args)
}

fn run_cli(cli: &Path, path: &Path, args: &[&OsStr]) -> Output {
    Command::new(cli)
        .args(args)
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
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
    // sha256 of 0x08000000, 64, 64 (u32 LE each) and [`code`]'s 64 bytes,
    // computed outside this crate.
    assert_eq!(
        b["firmware"]["image_sha256"],
        "dcb9381570c783a1de3fb3d7617b88a7f17761f126db560115e8368693dfae1a"
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
    // The segment's last instruction is inside (and not on the self-loop,
    // `AET-WO-2204` D3); its end is not.
    let (code, v) = boot(&good().replace("0x8000010\n", "0x800003e\n"));
    assert_eq!(kinds(&v), ["pc_not_on_self_loop"], "{v}");
    assert_eq!(code, Some(1), "{v}");
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
    // The bound is 30 s and the stub sleeps 60 s, so any upper limit below
    // 60 s shows the bound killed it; the slack covers teardown on a loaded
    // runner (35.29 s was measured on CI).
    assert!(took.as_secs() >= 30 && took.as_secs() < 50, "{took:?}");
}

// ---- The analog pre-pass (AET-WO-2200 §1.4), with a stub bridge -----------

/// The `renode` stub, which also exits 98 when the platform it is given does
/// (`faulted`) or does not carry the analog fault decision.
fn platform_checking_toolbox(faulted: bool) -> Dir {
    let check =
        r#"p=$(printf '%s' "$7" | /bin/sed -n 's/.*LoadPlatformDescription @\([^;]*\);.*/\1/p')"#;
    let then = if faulted {
        format!(
            "{check}\n/bin/grep -q 'bus `SENSORS` (I2C1) is faulted by the analog pass' \"$p\" || exit 98"
        )
    } else {
        format!("{check}\n/bin/grep -q 'Analog fault' \"$p\" && exit 98\nexit 0")
    };
    toolbox_then(&good(), "Renode v1.17.0", &then)
}

fn pre_pass(bridge: Option<&'static str>, faulted: bool) -> (Option<i32>, Value) {
    pre_pass_on(bridge, faulted, &spec(), &platform())
}

fn pre_pass_on(
    bridge: Option<&'static str>,
    faulted: bool,
    spec: &Path,
    platform: &Path,
) -> (Option<i32>, Value) {
    let tb = platform_checking_toolbox(faulted);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let out = run_cli(
        &cli(bridge),
        &tb.0,
        &[
            OsStr::new("simulate"),
            OsStr::new("--platform"),
            platform.as_os_str(),
            OsStr::new("--firmware"),
            fw.as_os_str(),
            OsStr::new("--spec"),
            spec.as_os_str(),
            OsStr::new("--for"),
            OsStr::new("10ms"),
        ],
    );
    (out.status.code(), report(&out))
}

#[test]
fn each_line_is_recorded_apart_from_the_boot_and_no_fault_leaves_the_platform_as_emitted() {
    let (code, v) = pre_pass(Some(PASSING_BRIDGE), false);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    let analog = v["analog"].as_array().unwrap();
    assert_eq!(analog.len(), 2, "{v}");
    for (record, net) in analog.iter().zip(["I2C_SCL", "I2C_SDA"]) {
        assert_eq!(record["kind"], "analog.rise_time_computed");
        assert_eq!(record["source"], "ngspice");
        assert_eq!(record["net"], net);
        assert_eq!(record["bus"], "SENSORS");
        assert_eq!(record["rise_30_70_ns"], 186.4);
        assert_eq!(record["ceiling_ns"], 300);
        assert_eq!(record["exceeds"], false);
        // The deck's window beside the bus's declared speed (`AET-WO-2205` §1.3).
        assert_eq!(record["deck_hz"], 400_000);
        assert_eq!(record["declared_hz"], 400_000);
    }
    assert_eq!(v["faults"], serde_json::json!([]));
    assert!(v["boot"].is_object() && v["boot"].get("analog").is_none());
}

#[test]
fn a_line_past_its_ceiling_fails_names_its_bus_and_reaches_the_platform() {
    const SLOW_SCL: &str = r#"#!/bin/sh
printf '%s\n' '{"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":3982.3}' '{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","rise_30_70_ns":186.4}'
"#;
    let (code, v) = pre_pass(Some(SLOW_SCL), true);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["verdict"], "fail");
    // The stub's own check passed: the booted platform names the fault.
    assert_eq!(v["tool"]["exit_code"], 0, "{v}");
    assert_eq!(kinds(&v), ["analog_rise_exceeds_ceiling"]);
    let finding = v["findings"][0]["message"].as_str().unwrap();
    for part in ["`SENSORS`", "`I2C_SCL`", "3982.3 ns", "300 ns ceiling"] {
        assert!(finding.contains(part), "{finding}");
    }
    assert_eq!(v["analog"][0]["exceeds"], true);
    assert_eq!(v["analog"][1]["exceeds"], false);
    let faults = v["faults"].as_array().unwrap();
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0]["kind"], "bridge.fault_state");
    assert_eq!(faults[0]["bus"], "SENSORS");
    assert_eq!(faults[0]["nets"], serde_json::json!(["I2C_SCL"]));
    assert_eq!(
        faults[0]["from"],
        serde_json::json!(["analog.rise_time_computed > ceiling_ns"])
    );
    let sees = faults[0]["firmware_sees"].as_str().unwrap();
    assert!(sees.contains("model's rendering") && sees.contains("not a measured waveform"));
    // The firmware's observation is its own record, still judged.
    assert_eq!(v["boot"]["boot_word"]["observed"], "0x2100b007", "{v}");
}

#[test]
fn a_missing_bridge_is_refused_before_renode() {
    let (code, v) = pre_pass(None, false);
    assert_eq!(code, Some(4), "{v}");
    assert_eq!(v["error"]["kind"], "BridgeUnavailable");
    assert_eq!(v["error"]["id"], "aethelgard-bridge");
}

#[test]
fn a_bridge_that_fails_or_answers_wrong_stops_the_call() {
    const STUBS: [&str; 6] = [
        // Fails, with a correct line first.
        "#!/bin/sh\nprintf '%s\\n' '{\"method\":\"interpolated\",\"net\":\"I2C_SCL\",\"ngspice\":\"44.2\",\"rise_30_70_ns\":186.4}' '{\"net\":\"I2C_SDA\",\"error\":\"methods_disagree\",\"message\":\"m\"}'\nexit 1\n",
        // One line short, exit 0.
        "#!/bin/sh\nprintf '%s\\n' '{\"method\":\"interpolated\",\"net\":\"I2C_SCL\",\"ngspice\":\"44.2\",\"rise_30_70_ns\":186.4}'\n",
        // Nothing at all.
        "#!/bin/sh\nexit 0\n",
        // Killed by a signal.
        "#!/bin/sh\nkill -9 $$\n",
        // Another ngspice.
        "#!/bin/sh\nprintf '%s\\n' '{\"method\":\"interpolated\",\"net\":\"I2C_SCL\",\"ngspice\":\"45\",\"rise_30_70_ns\":186.4}' '{\"method\":\"interpolated\",\"net\":\"I2C_SDA\",\"ngspice\":\"45\",\"rise_30_70_ns\":186.4}'\n",
        // The nets swapped.
        "#!/bin/sh\nprintf '%s\\n' '{\"method\":\"interpolated\",\"net\":\"I2C_SDA\",\"ngspice\":\"44.2\",\"rise_30_70_ns\":186.4}' '{\"method\":\"interpolated\",\"net\":\"I2C_SCL\",\"ngspice\":\"44.2\",\"rise_30_70_ns\":186.4}'\n",
    ];
    for stub in STUBS {
        let (code, v) = pre_pass(Some(stub), false);
        assert_eq!(code, Some(5), "{stub}: {v}");
        assert_eq!(v["error"]["kind"], "ToolError", "{stub}: {v}");
        assert_eq!(v["error"]["id"], "aethelgard-bridge", "{stub}: {v}");
    }
}

/// The bridge works in a directory of its own: it can write neither the
/// copies Renode will read nor the scratch root around them.
#[test]
fn the_bridge_cannot_write_what_renode_reads() {
    const WRITER: &str = r#"#!/bin/sh
for f in ../tool/firmware.elf ../tool/platform.repl ../tool/new ../status-bridge.x; do
  if ( : > "$f" ) 2>/dev/null; then exit 7; fi
done
printf '%s\n' '{"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":186.4}' '{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","rise_30_70_ns":186.4}'
"#;
    let (code, v) = pre_pass(Some(WRITER), false);
    assert_eq!(code, Some(0), "{v}");
}

/// The reference specification with each (from, to) replaced everywhere, and
/// the platform `aethelgard-twin` emits for it, in `dir`.
fn variant(dir: &Dir, edits: &[(&str, &str)]) -> (PathBuf, PathBuf) {
    let mut text = fs::read_to_string(spec()).unwrap();
    for (from, to) in edits {
        assert!(text.contains(from), "{from}");
        text = text.replace(from, to);
    }
    let spec = dir.write("spec.json", text.as_bytes());
    let out = run_cli(
        &cli(None),
        &dir.0,
        &[OsStr::new("solve"), OsStr::new("--file"), spec.as_os_str()],
    );
    let topology: aethelgard_ir::SolvedTopology = serde_json::from_value(report(&out)).unwrap();
    let repl = aethelgard_twin::platform::emit(&topology).unwrap();
    (spec, dir.write("platform.repl", repl.as_bytes()))
}

/// Ruled F7 (b) on PR #70, on the adversary's A6 edges: a bus too slow to
/// finish rising in the window, and a pull-up the driver cannot overcome,
/// are faults of the design with a `fail` verdict, never exit 5. The stub
/// answers what the real bridge answers for each in the tools image
/// (`bridge/tests/edges.rs`).
#[test]
fn a_rise_unfinished_in_the_window_or_a_line_stuck_high_is_a_fault() {
    const SLOW: &str = r#"#!/bin/sh
for n in I2C_SCL I2C_SDA; do
  printf '{"complete":false,"method":"interpolated","net":"%s","ngspice":"44.2","rise_30_70_ns_lower_bound":13298.027700387645}\n' $n
done
"#;
    const STUCK: &str = r#"#!/bin/sh
for n in I2C_SCL I2C_SDA; do
  printf '{"method":"interpolated","net":"%s","ngspice":"44.2","volts_at_release":1.6500000000000004}\n' $n
done
"#;
    let dir = Dir::new("a6");
    // 47 kΩ and 400 pF.
    let (spec, platform) = variant(
        &dir,
        &[
            ("\"pull_up_ohms\": 2200", "\"pull_up_ohms\": 47000"),
            (
                "\"max_capacitance_femtofarads\": 100000",
                "\"max_capacitance_femtofarads\": 400000",
            ),
        ],
    );
    let (code, v) = pre_pass_on(Some(SLOW), true, &spec, &platform);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["verdict"], "fail");
    assert_eq!(v["tool"]["exit_code"], 0, "{v}");
    assert_eq!(kinds(&v), ["analog_rise_exceeds_ceiling"; 2]);
    let analog = v["findings"].as_array().unwrap().iter();
    for f in analog.filter(|f| f["kind"] == "analog_rise_exceeds_ceiling") {
        let m = f["message"].as_str().unwrap();
        // serde_json reads a float to within one ulp, so the last digit may move.
        assert!(m.contains("more than 13298.02770038764"), "{m}");
        assert!(
            m.contains("ns (a lower bound), past its 300 ns ceiling"),
            "{m}"
        );
    }
    for line in v["analog"].as_array().unwrap() {
        assert_eq!(line["kind"], "analog.rise_time_computed");
        assert_eq!(line["complete"], false);
        assert_eq!(line["exceeds"], true);
        let bound = line["rise_30_70_ns_lower_bound"].as_f64().unwrap();
        assert!((bound - 13298.0277).abs() < 1e-3, "{line}");
        assert!(line.get("rise_30_70_ns").is_none(), "{line}");
    }
    assert_eq!(
        v["faults"][0]["from"],
        serde_json::json!(["analog.rise_time_computed > ceiling_ns"])
    );

    // `AET-WO-1203`: `solve` now refuses the 10 Ω pull-up this case once used
    // (below the 967 Ω floor), so the specification is the lowest that
    // solves. The stub answers what the real bridge answers for a 10 Ω
    // pull-up (`bridge/tests/edges.rs`); the judgement under test is the same.
    let (spec, platform) = variant(&dir, &[("\"pull_up_ohms\": 2200", "\"pull_up_ohms\": 967")]);
    let (code, v) = pre_pass_on(Some(STUCK), true, &spec, &platform);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["verdict"], "fail");
    assert_eq!(kinds(&v), ["analog_line_stuck_high"; 2]);
    for line in v["analog"].as_array().unwrap() {
        assert_eq!(line["kind"], "analog.line_stuck_high");
        assert_eq!(line["complete"], false);
        let volts = line["volts_at_release"].as_f64().unwrap();
        assert!((volts - 1.65).abs() < 1e-9, "{line}");
    }
    assert_eq!(
        v["faults"][0]["from"],
        serde_json::json!(["analog.line_stuck_high"])
    );
    assert_eq!(
        v["faults"][0]["nets"],
        serde_json::json!(["I2C_SCL", "I2C_SDA"])
    );
}

// `AET-WO-2202`: the identity reads, judged from the part table.

/// A boot that reached the fixed point with `record` in U2's record, and
/// U2's witness before the reads.
fn with_record(record: [&str; 2]) -> String {
    with_log(record, GOOD_WITNESS)
}

/// A boot that reached the fixed point with `record` in U2's record and `log`
/// as the run's log before the reads.
fn with_log(record: [&str; 2], log: &str) -> String {
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    // U2's record, before U4's (`AET-WO-4200`), which stays good, its
    // witness after the test's log.
    let n = r.len();
    r[n - 4] = record[0].to_owned();
    r[n - 3] = record[1].to_owned();
    transcript(
        &format!("{log}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    )
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
    let mut r = reads(
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let n = r.len();
    r[n - 2] = "0xFFFFFFFF".into();
    r[n - 1] = "0xFFFFFFFF".into();
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert!(finding(&v, "boot_word_missing").is_some());
    assert!(finding(&v, "i2c_record_invalid").is_none(), "{v}");
    // Nor its witness, nor where the core parked.
    assert_eq!(kinds(&v), ["boot_word_missing"], "{v}");
    // The unmodelled device is still reported on a failed boot.
    assert!(finding(&v, "device_unmodelled").is_some());
}

// `AET-WO-2204` D1: the record is the firmware's word; the device model's own
// log is the witness. The judge expects exactly BME280.cs's two lines for U2
// (`Write D0`, `Read 60`), whole, after the host timestamp, in order.

#[test]
fn a_witness_that_shows_the_read_passes_and_is_reported() {
    let (code, v) = boot(&good());
    assert_eq!(code, Some(0), "{v}");
    let w = &v["boot"]["i2c_witness"];
    let lines = serde_json::json!(["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60"]);
    assert_eq!(
        *w,
        serde_json::json!([{
            "peripheral": "I2C1", "reference": "U2", "device": "i2c1.u2",
            "expected": lines, "observed": lines, "absent": []
        }])
    );
    assert_eq!(v["boot"]["icsr"], "0x00000800");
    // Other log lines around the witness are not the witness.
    let around = format!(
        "[03:07:39.3700] [INFO] machine-0: Machine started.\n{GOOD_WITNESS}[03:07:39.3790] [WARNING] i2c1: Unhandled write to offset 0x1C.\n"
    );
    let (code, v) = boot(&with_log(GOOD_RECORD, &around));
    assert_eq!(code, Some(0), "{v}");
    // The controller's `Unknown slave` lines are kept for attribution, never
    // judged: here a stray transfer to 0x77 beside a witnessed read passes.
    let stray =
        format!("[03:07:39.3700] [WARNING] i2c1: Unknown slave at address 119.\n{GOOD_WITNESS}");
    let (code, v) = boot(&with_log(GOOD_RECORD, &stray));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        v["boot"]["i2c_witness"][0]["absent"],
        serde_json::json!(["0x77"])
    );
}

#[test]
fn any_witness_but_the_model_s_exact_lines_fails() {
    let w = |a: &str, b: &str| format!("[03:07:39.3762] {a}\n[03:07:39.3765] {b}\n");
    for (log, what) in [
        (
            String::new(),
            "no line at all: a record written with no transfer",
        ),
        (
            w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Write D0"),
            "no read",
        ),
        (
            w("[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 60"),
            "another register",
        ),
        (
            w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 61"),
            "another value",
        ),
        (
            w("[NOISY] i2c1.u2: Read 60", "[NOISY] i2c1.u2: Write D0"),
            "out of order",
        ),
        (
            w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 (2)"),
            "a folded line",
        ),
        (
            w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 600"),
            "a longer value",
        ),
        (
            w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 "),
            "a trailing space",
        ),
        (
            format!(
                "{GOOD_WITNESS}{}",
                w("[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60")
            ),
            "a second read",
        ),
    ] {
        let (code, v) = boot(&with_log(GOOD_RECORD, &log));
        assert_eq!(code, Some(1), "{what}: {v}");
        assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{what}: {v}");
        let m = finding(&v, "i2c_transaction_unwitnessed").unwrap()["message"]
            .as_str()
            .unwrap();
        assert!(
            m.contains("U2") && m.contains("0x76") && m.contains("i2c1.u2"),
            "{m}"
        );
    }
    // Witness-shaped text that is not a whole log line of the device's own is
    // not a witness at all: the read is unwitnessed.
    for log in [
        "[03:07:39.3762] [INFO] i2c1.u2: Write D0\n[03:07:39.3765] [INFO] i2c1.u2: Read 60\n",
        "[NOISY] i2c1.u2: Write D0\n[NOISY] i2c1.u2: Read 60\n",
        "[03:07:39.3762] [NOISY] usart1: [NOISY] i2c1.u2: Write D0\n[03:07:39.3765] [NOISY] usart1: [NOISY] i2c1.u2: Read 60\n",
        "x[03:07:39.3762] [NOISY] i2c1.u2: Write D0\nx[03:07:39.3765] [NOISY] i2c1.u2: Read 60\n",
    ] {
        let (code, v) = boot(&with_log(GOOD_RECORD, log));
        assert_eq!(code, Some(1), "{log}: {v}");
        assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{log}: {v}");
    }
}

#[test]
fn the_witness_is_not_judged_for_a_record_that_claims_nothing() {
    // A wait that expired claims no transfer; `i2c_timeout` alone names it.
    let (code, v) = boot(&with_log(["0x22020001", "0x00000000"], ""));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_timeout"], "{v}");
}

#[test]
fn the_judge_s_own_log_after_begin_is_never_the_run_s() {
    // `AET-WO-2204` D8 (a), F3: lines the judge's reads cause come after
    // `BEGIN`, whole, and are skipped; they neither witness nor fail the run.
    let after = |lines: &[&str]| {
        let mut r = reads(
            "0x2100B007",
            "0x00000000000F4240",
            "0x00000064",
            &GOOD_REGISTERS,
        );
        r.splice(1..1, lines.iter().map(|l| (*l).to_owned()));
        r
    };
    let locked = "[03:07:40.0001] [WARNING] sysbus: Tried to read a locked peripheral: machine-0.i2c1. Address 0x40005404.";
    let r = after(&[locked, locked]);
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(0), "{v}");
    // The witness among the judge's reads is not the run's: unwitnessed.
    let r = after(&[
        "[03:07:40.0001] [NOISY] i2c1.u2: Write D0",
        "[03:07:40.0002] [NOISY] i2c1.u2: Read 60",
    ]);
    let (code, v) = boot(&transcript(
        "",
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["i2c_transaction_unwitnessed", "spi_transaction_unwitnessed"],
        "{v}"
    );
    // Anything after `BEGIN` that is neither a value nor a whole log line is
    // refused, never read (F2: a log line split below line level).
    for split in [
        "0x000000[03:07:40.0001] [WARNING] sysbus: Tried to read a locked peripheral",
        "AETHELGARD-BOOT-B",
        "[WARNING] sysbus: no timestamp",
    ] {
        let r = after(&[split]);
        untrusted(boot(&transcript(
            &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
            &r.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
    }
}

#[test]
fn a_log_past_the_filter_s_budget_leaves_every_read_unwitnessed() {
    // `AET-WO-2204` D2, ruled: past 8 MiB a log line may have been dropped,
    // so the witness, though printed first, cannot be read whole: never a
    // pass.
    let flood = "[03:07:39.4000] [WARNING] i2c1: Receive buffer underflow!\n".repeat(160_000);
    let (code, v) = boot(&with_log(GOOD_RECORD, &format!("{GOOD_WITNESS}{flood}")));
    assert_eq!(code, Some(1), "{v}");
    // Every read, U4's on SPI too (`AET-WO-4200`).
    assert_eq!(
        kinds(&v),
        ["i2c_transaction_unwitnessed", "spi_transaction_unwitnessed"],
        "{v}"
    );
    assert!(finding(&v, "renode_log_flood").is_some(), "{v}");
    assert_eq!(v["boot"]["i2c_witness"][0]["observed"], Value::Null);
}

// `AET-WO-2204` D3: after the boot word, the core is in thread mode on a
// `yield; b.n` self-loop read from the ELF.

#[test]
fn a_core_handling_an_exception_fails_naming_vectactive() {
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    // `systick-before`, measured: VECTACTIVE 15, on `fault`'s own self-loop.
    r[4] = "0x0400080F".into();
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["core_in_exception"], "{v}");
    let m = finding(&v, "core_in_exception").unwrap()["message"]
        .as_str()
        .unwrap();
    assert!(m.contains("VECTACTIVE") && m.contains("15"), "{m}");
    // Bits above VECTACTIVE do not count.
    r[4] = "0x0400F800".into();
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn a_core_not_on_a_self_loop_fails_and_both_of_its_halfwords_pass() {
    let at = |pc: &str| {
        let mut r = reads(
            "0x2100B007",
            "0x00000000000F4240",
            "0x00000064",
            &GOOD_REGISTERS,
        );
        r[3] = pc.into();
        boot(&transcript(
            &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
            &r.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
    };
    for pc in ["0x8000010", "0x8000012"] {
        let (code, v) = at(pc);
        assert_eq!(code, Some(0), "{pc}: {v}");
    }
    for pc in ["0x8000014", "0x800000E", "0x8000020"] {
        let (code, v) = at(pc);
        assert_eq!(code, Some(1), "{pc}: {v}");
        assert_eq!(kinds(&v), ["pc_not_on_self_loop"], "{pc}: {v}");
    }
    // Outside the firmware it is `pc_outside_firmware` alone.
    let (_, v) = at("0x8000040");
    assert_eq!(kinds(&v), ["pc_outside_firmware"], "{v}");
    // Adversarial A1: on the ELF's loop, but the machine no longer holds it
    // (the firmware rewrote its flash: `str r0, [r1]` where the `yield` was).
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    r[5] = "0x6008".into();
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["pc_not_on_self_loop"], "{v}");
    assert!(
        finding(&v, "pc_not_on_self_loop").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("no longer holds"),
        "{v}"
    );
}

/// `AET-WO-2204` D1: on a bus with two modelled devices (the test fixture
/// `two-bme.json`: U2 at 0x76, U5 at 0x77), each read is witnessed by its own
/// device's lines, and the devices' lines must come in the records' order.
#[test]
fn two_devices_witness_their_own_reads_in_the_records_order() {
    let tree = Dir::new("two-bme");
    let out = run_cli(
        &cli(None),
        Path::new(""),
        &[
            OsStr::new("generate"),
            OsStr::new("--spec"),
            root()
                .join("crates/aethelgard-cli/tests/fixtures/two-bme.json")
                .as_os_str(),
            OsStr::new("--out"),
            tree.0.join("g").as_os_str(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let platform = tree.0.join("g/board/sentinel-01.repl");
    let spec = root().join("crates/aethelgard-cli/tests/fixtures/two-bme.json");
    let line = |t: &str, d: &str, l: &str| format!("[03:07:39.{t}] [NOISY] i2c1.{d}: {l}\n");
    let u2 = format!(
        "{}{}",
        line("3762", "u2", "Write D0"),
        line("3765", "u2", "Read 60")
    );
    let u5 = format!(
        "{}{}",
        line("3862", "u5", "Write D0"),
        line("3865", "u5", "Read 60")
    );
    let listing = CPU_LISTING.to_owned() + "  \u{251c}\u{2500}\u{2500} u5 (BME280)\n";
    let boot_two = |log: &str| {
        let mut r = reads(
            "0x2100B007",
            "0x00000000000F4240",
            "0x00000064",
            &GOOD_REGISTERS,
        );
        // U5's record after U2's, before U4's on SPI (`AET-WO-4200`).
        let n = r.len();
        r.splice(n - 2..n - 2, GOOD_RECORD.iter().map(|s| (*s).to_owned()));
        let text = transcript(
            &format!("{log}{GOOD_SPI_WITNESS}"),
            &r.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .replace(CPU_LISTING, &listing);
        // A stub that checks the configuration alone: two devices make
        // another command line.
        let tb = Dir::new("tb-two");
        let outf = tb.write("renode.out", text.as_bytes());
        let stub = tb.write(
            "renode",
            format!(
                "#!/bin/sh\n[ \"$1\" = --config ] || exit 97\nif [ \"$3\" = --version ]; then printf 'Renode v1.17.0\\n  build: 1.17.0+20260907gitf1dd1b4af\\n'; exit 0; fi\n/bin/cat '{}'\n",
                outf.display()
            )
            .as_bytes(),
        );
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        let input = Dir::new("in");
        let fw = input.write("fw.elf", &elf());
        let out = run(
            &tb.0,
            &[
                OsStr::new("simulate"),
                OsStr::new("--platform"),
                platform.as_os_str(),
                OsStr::new("--firmware"),
                fw.as_os_str(),
                OsStr::new("--spec"),
                spec.as_os_str(),
                OsStr::new("--for"),
                OsStr::new("10ms"),
            ],
        );
        (out.status.code(), report(&out))
    };
    let (code, v) = boot_two(&format!("{u2}{u5}"));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["boot"]["i2c_witness"].as_array().unwrap().len(), 2, "{v}");
    // Each device's own lines are right, but U5's come first.
    let (code, v) = boot_two(&format!("{u5}{u2}"));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_witness_out_of_order"], "{v}");
    // One device's lines alone: the other read is unwitnessed, named.
    let (code, v) = boot_two(&u5);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["i2c_transaction_unwitnessed"], "{v}");
    assert!(
        finding(&v, "i2c_transaction_unwitnessed").unwrap()["message"]
            .as_str()
            .unwrap()
            .starts_with("U2")
    );
}
