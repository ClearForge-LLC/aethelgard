//! `AET-WO-2100`, tier 1: `simulate --firmware` against a stub `renode`, with
//! none of the real tools installed. The stub checks the command line's shape
//! (the configuration file and its synchronous logging, fixed seed, cold RCC,
//! the device's log level, fixed virtual time, no console level changed),
//! then prints a canned transcript, so the judge's every branch runs in
//! `gates`: the witnesses, the hardened reader (R3), and the refusals before
//! Renode runs (R2, R5, R6).

use aethelgard_twin::platform::RENODE_BUILD;
use std::ffi::OsStr;
use std::fs;
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
    elf_segments(machine, &[(paddr, code)])
}

/// An ELF32 LE `ET_EXEC` for `machine` with one executable `PT_LOAD` per
/// `(paddr, code)`, in order.
fn elf_segments(machine: u16, segments: &[(u32, &[u8])]) -> Vec<u8> {
    let mut b = vec![0u8; 52];
    b[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    b[16..18].copy_from_slice(&2u16.to_le_bytes());
    b[18..20].copy_from_slice(&machine.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[28..32].copy_from_slice(&52u32.to_le_bytes());
    b[40..42].copy_from_slice(&52u16.to_le_bytes());
    b[42..44].copy_from_slice(&32u16.to_le_bytes());
    b[44..46].copy_from_slice(&(segments.len() as u16).to_le_bytes());
    let mut offset = 52 + 32 * segments.len() as u32;
    for (paddr, code) in segments {
        let len = code.len() as u32;
        for v in [1u32, offset, *paddr, *paddr, len, len, 5, 4] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        offset += len;
    }
    for (_, code) in segments {
        b.extend_from_slice(code);
    }
    b
}

/// The foreign tests' firmware: [`foreign_code`] at the flash's start with
/// its panic at `0x08000020` (`common::foreign::with_panic`).
fn foreign_elf() -> Vec<u8> {
    common::foreign::with_panic(elf_at(0x0800_0000, &foreign_code(), 40), 0x0800_0020)
}

/// [`code`] with a vector table's four fault entries (`0x0800000C` to
/// `0x0800001B`) each naming the handler at `0x08000030`, Thumb bit set: the
/// foreign judge hooks them, and refuses a table it cannot (`AET-WO-2314`).
/// The foreign judge holds the core to no self-loop, so the one at
/// `0x08000010` that the table overwrites is not needed.
fn foreign_code() -> [u8; 64] {
    let mut c = code();
    for at in (0x0C..0x1C).step_by(4) {
        c[at..at + 4].copy_from_slice(&0x0800_0031u32.to_le_bytes());
    }
    c
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
const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} gpioPortA (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} gpioPortB (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} i2c1 (STM32F7_I2C)\n  \u{251c}\u{2500}\u{2500} spi1 (STM32H7_SPI)\n  \u{2502}   \u{2514}\u{2500}\u{2500} spi1mux (SPIMultiplexer)\n  \u{2502}       \u{2514}\u{2500}\u{2500} u4 (GenericSpiFlash)\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n  \u{251c}\u{2500}\u{2500} usart1 (STM32F7_USART)\n";

/// The reference topology's registers as a correct boot leaves them, in
/// address order: the console's USART1 (`AET-WO-4300b`: CR1 `UE|RE|TE`, CR2,
/// CR3, BRR 556), the ports' mode and AF registers (PA9/PA10 at AF7 since
/// the console), then every RCC enable register (APB2ENR with `USART1EN`);
/// then SysTick's control word, unarmed, and the five NVIC set-enable words,
/// 0: no interrupt armed (`AET-WO-2204` A2, ruled D6 on PR #131;
/// `AET-WO-4210`).
const GOOD_REGISTERS: [&str; 24] = [
    "0x0000000D",
    "0x00000000",
    "0x00000000",
    "0x0000022C",
    "0x0028A801",
    "0x55500000",
    "0x00000770",
    "0x0000A000",
    "0x44000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000003",
    "0x00000000",
    "0x00200000",
    "0x00000000",
    "0x00001010",
    "0x00010000",
    // `CLKSOURCE` reads 1 in Renode whatever is written (`NVIC.cs` L1040).
    "0x00000004",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
];

/// The registers of a board without the console: the reference's before
/// `AET-WO-4300b`, as `two-bme.json` still solves them.
const NO_SERIAL_REGISTERS: [&str; 19] = [
    "0x0000A801",
    "0x55500000",
    "0x0000A000",
    "0x44000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000003",
    "0x00000000",
    "0x00200000",
    "0x00000000",
    "0x00001000",
    "0x00010000",
    // `CLKSOURCE` reads 1 in Renode whatever is written (`NVIC.cs` L1040).
    "0x00000004",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x00000000",
];

/// The lines the CLI's own hooks log in the reference's boot (`AET-WO-2302`
/// D4, the shapes measured in build probe P5): the boot word at instruction
/// 2575, then each frame byte a quantum and a character (100 us at 115 200
/// 8N1) apart from it, the first with the console's RX pin, PA10, in AF mode
/// on AF7 (GPIOA's `MODER`, then `AFRH`).
const INJECTION: &str = "[00:00:00.000025750] [WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 2575\n\
[00:00:00.000125750] [WARNING] sysbus: AETHELGARD-INJECT 0 0 2575 12575 28a801 770\n\
[00:00:00.000225750] [WARNING] sysbus: AETHELGARD-INJECT 0 1 2575 22575\n\
[00:00:00.000325750] [WARNING] sysbus: AETHELGARD-INJECT 0 2 2575 32575\n\
[00:00:00.000425750] [WARNING] sysbus: AETHELGARD-INJECT 0 3 2575 42575\n\
[00:00:00.000525750] [WARNING] sysbus: AETHELGARD-INJECT 0 4 2575 52575\n\
[00:00:00.000625750] [WARNING] sysbus: AETHELGARD-INJECT 0 5 2575 62575\n\
[00:00:00.000725750] [WARNING] sysbus: AETHELGARD-INJECT 0 6 2575 72575\n\
[00:00:00.000825750] [WARNING] sysbus: AETHELGARD-INJECT 0 7 2575 82575\n";

/// The boot word's hook in the stub's accepted command line: the CLI's own
/// Python, between its double quotes (`AET-WO-2302`).
const HOOK: &str = r#"sysbus AddWatchpointHook 0x20000000 DoubleWord Write \""*"\""#;

/// The console's bring-up as Renode 1.17.0's access log shows it
/// (`AET-WO-4300b` fix 1, measured): PA10's and PA9's AF7, their modes, then
/// USART1's `BRR`, `CR2`, `CR1`, `CR1 | UE` (in FIFO mode), then the
/// pattern's first byte, with a poll of `ISR` the boot's filter discards.
const CONSOLE_ACCESS: &str = "[00:00:00.000000000] [INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x24 (AlternateFunctionHigh), value 0x770\n\
[00:00:00.000000000] [INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x0 (Mode), value 0x28A801\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0xC (BaudRate), value 0x22C\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x4 (ControlRegister2), value 0x0\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000000C\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000000D\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x8000990] ReadUInt32 from 0x1C (InterruptAndStatus), returned 0x206000C0\n\
[00:00:00.000000000] [INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x28 (TransmitData), value 0x43\n";

/// The reference's brown-out word as the firmware records it: Renode's
/// `OPTSR_CUR` with `BOR_LEV` 3 presented (`AET-WO-4230` D10, C1 m2).
const BROWN_OUT: &str = "0x0406AAFC";

/// The flash controller's access log of a boot (`AET-WO-4230` D10, measured
/// C1 m2): Renode's answer to the judge's command at the load, then the
/// bring-up's one word read of `OPTSR_CUR`.
const FLASH_ACCESS: &str = "[00:00:00.000000000] [INFO] flashController: Logging is already enabled
[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000102] ReadUInt32 from 0x1C (OptionStatusCurrent), returned 0x406AAFC
";

/// The PWR's access log of a generated boot (`AET-WO-4240v` §1.6; the
/// shape measured in Renode 1.17.0, FEEDBACK): `PWR_CR3` read at revision
/// V's `0x46` and written once with `LDOEN` (`0x42`), then `PWR_D3CR`
/// written `0x8000` (VOS Scale 2), before any RCC enable write. The waits'
/// polls of `CSR1` and `D3CR` are discarded by the boot's log filter.
const PWR_ACCESS: &str =
    "[00:00:00.000000000] [INFO] pwr: [cpu: 0x800000A] ReadUInt32 from 0xC (unknown), returned 0x46
[00:00:00.000000000] [INFO] pwr: [cpu: 0x8000014] WriteUInt32 to 0xC (unknown), value 0x42
[00:00:00.000000000] [INFO] pwr: [cpu: 0x8000016] ReadUInt32 from 0xC (unknown), returned 0x42
[00:00:00.000000000] [INFO] pwr: [cpu: 0x800002A] WriteUInt32 to 0x18 (unknown), value 0x8000
";

/// What USART1's file backend holds after a boot of `firmware` that sent
/// the console's pattern and echoed its frame (`AET-WO-4300b`; measured).
fn capture_of(firmware: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    let sha: [u8; 32] = sha2::Sha256::digest(firmware).into();
    let mut c = aethelgard_twin::serial::pattern("CONSOLE");
    c.extend(aethelgard_twin::serial::frame(&sha, "CONSOLE"));
    c
}

/// A transcript: the run's log lines, then the reads between the markers,
/// then the listing. The first line is how Renode echoes the command line,
/// markers and all, so a marker matched by substring would be found there.
fn transcript(run_log: &str, reads: &[&str]) -> String {
    let mut s = String::from(
        "(monitor) emulation SetSeed 2100;  echo \"AETHELGARD-BOOT-BEGIN\";  echo \"AETHELGARD-BOOT-END\"\n",
    );
    s.push_str(FLASH_ACCESS);
    s.push_str(PWR_ACCESS);
    s.push_str(CONSOLE_ACCESS);
    s.push_str(INJECTION);
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
        // The brown-out word: `FLASH_OPTSR_CUR` as the platform presents
        // the reference's level 3 (`AET-WO-4230` D10, measured C1 m2).
        BROWN_OUT.to_owned(),
    ];
    v.extend(registers.iter().map(|r| (*r).to_owned()));
    // U2's record: OK, then the BME280's identity; then U4's, the flash's
    // JEDEC ID (`AET-WO-4200`).
    v.extend(GOOD_RECORD.iter().map(|r| (*r).to_owned()));
    v.extend(GOOD_SPI_RECORD.iter().map(|r| (*r).to_owned()));
    // U3's, the unmodelled TMP117's identity, read after every modelled
    // step: no model answers, so the TXIS wait expired (`AET-WO-4241`,
    // measured).
    v.extend(UNMODELLED_RECORD.iter().map(|r| (*r).to_owned()));
    v
}

/// The record the TMP117's identity read leaves in the twin, which has no
/// model for it (`AET-WO-4241` §1.1, measured): `TXIS` timed out, value 0.
const UNMODELLED_RECORD: [&str; 2] = ["0x22020001", "0x00000000"];

/// The record a firmware that read U4's JEDEC ID (0x9F) as EF 40 17 leaves.
const GOOD_SPI_RECORD: [&str; 2] = ["0x22020000", "0x00EF4017"];

/// What Renode 1.17.0's `GenericSpiFlash` logs for U4's JEDEC ID read behind
/// SPI1's multiplexer (`AET-WO-4200` S2, measured).
const GOOD_SPI_WITNESS: &str = "[00:00:00.040010000] [NOISY] spi1.spi1mux.u4: Transmitting data 0x9F, current state: RecognizeOperation\n\
[00:00:00.040020000] [NOISY] spi1.spi1mux.u4: Decoded operation: Operation: ReadID, write enabled False\n\
[00:00:00.040030000] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[00:00:00.040040000] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0xEF\n\
[00:00:00.040050000] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[00:00:00.040060000] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x40\n\
[00:00:00.040070000] [NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand\n\
[00:00:00.040080000] [NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x17\n";

/// What Renode 1.17.0's BME280 logs for U2's identity read (`AET-WO-2204`
/// S1, measured): the run's witness, before `BEGIN`.
const GOOD_WITNESS: &str = "[00:00:00.037620000] [NOISY] i2c1.u2: Write D0\n[00:00:00.037650000] [NOISY] i2c1.u2: Read 60\n";

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
    toolbox_capture(stdout, version, then, &capture_of(&elf()))
}

/// As [`toolbox_then`], with the stub writing `capture` where the command
/// line's `CreateFileBackend` names USART1's file, as Renode's file backend
/// would (`AET-WO-4300b`).
fn toolbox_capture(stdout: &str, version: &str, then: &str, capture: &[u8]) -> Dir {
    let tb = Dir::new("tb");
    let out = tb.write("renode.out", stdout.as_bytes());
    let bytes = tb.write("capture.bin", capture);
    let body = format!(
        r#"#!/bin/sh
[ "$1" = --config ] || exit 97
/bin/grep -qx 'use-synchronous-logging = true' "$2" || exit 97
/bin/grep -qx 'log-timestamps-type = Virtual' "$2" || exit 97
if [ "$3" = --version ]; then printf '{version}\n  build: {RENODE_BUILD}\n'; exit 0; fi
# No console level is ever set (`AET-WO-2204` D4); only a device's source level.
printf '%s' "$7" | /bin/grep -Eq 'logLevel [0-3]|logLevel -1 [^s]' && exit 96
case "$7" in
  "emulation SetSeed 2100; emulation SetGlobalQuantum \"0.000100000\"; mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; sysbus LogPeripheralAccess sysbus.flashController true; sysbus WriteDoubleWord 0x580244d4 0x0; sysbus WriteDoubleWord 0x580244d8 0x0; sysbus WriteDoubleWord 0x580244dc 0x0; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e4 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244ec 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; sysbus WriteDoubleWord 0x580244f4 0x10000; sysbus WriteDoubleWord 0x20000000 0x0; sysbus WriteDoubleWord 0x20000004 0xffffffff; sysbus WriteDoubleWord 0x20000008 0xffffffff; sysbus WriteDoubleWord 0x2000000c 0xffffffff; sysbus WriteDoubleWord 0x20000010 0xffffffff; sysbus WriteDoubleWord 0x20000014 0xffffffff; sysbus WriteDoubleWord 0x20000018 0xffffffff; sysbus WriteDoubleWord 0x2000001c 0xffffffff; cpu VectorTableOffset 0x08000000; "*"sysbus.usart1 CreateFileBackend @"*"/capture/usart1.bin true; {HOOK}; emulation RunFor \"0.0100000005\"; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0xe000ed04; sysbus ReadWord 0x08000010; sysbus ReadWord 0x08000012; sysbus ReadDoubleWord 0x20000004; sysbus ReadDoubleWord 0x40011000;"*"; echo \"AETHELGARD-BOOT-END\"; peripherals; quit") ;;
  *) exit 99;;
esac
cap=$(printf '%s' "$7" | /bin/sed -n 's/.*CreateFileBackend @\([^ ]*\) true.*/\1/p')
/bin/cat '{bytes}' > "$cap" || exit 95
/bin/cat '{out}'
{then}
"#,
        out = out.display(),
        bytes = bytes.display()
    );
    let stub = tb.0.join("renode");
    common::stub::write_executable(&stub, body.as_bytes());
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
        common::stub::write_executable(&me, fs::read(env!("CARGO_BIN_EXE_aethelgard")).unwrap());
    }
    if let Some(body) = bridge {
        let stub = dir.join("aethelgard-bridge");
        common::stub::write_executable(&stub, common::identified_stub(body));
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
/// ones, which report what the twin could not check (`identity_not_witnessable`).
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
    // Seven solved registers and, since `AET-WO-4300a` (2100 F24), the six
    // other enable registers, each judged in full; since `AET-WO-4300b`, the
    // console's USART1 (four registers) and its pins' `AFRH`; then, since
    // `AET-WO-4210`, SysTick's control word and the five NVIC set-enable words.
    assert_eq!(b["registers"].as_array().unwrap().len(), 24);
    assert_eq!(b["registers"][0]["name"], "USART1.CR1");
    assert_eq!(b["registers"][4]["name"], "GPIOA.MODER");
    // `AET-WO-2204` A2 (ruled D6): no interrupt armed.
    assert_eq!(b["registers"][18]["name"], "SysTick.CSR");
    assert_eq!(b["registers"][23]["name"], "NVIC.ISER4");
    // Every configured port's `OTYPER`, written whole (`AET-WO-4230` B0);
    // the two PWR bits the emitted PWR reads 1 (`AET-WO-4240v` §1.6).
    assert_eq!(
        b["unobservable"],
        serde_json::json!([
            "GPIOA.OTYPER",
            "GPIOB.OTYPER",
            "I2C1.TIMINGR",
            "PWR.CSR1.ACTVOSRDY",
            "PWR.D3CR.VOSRDY",
            "USART1.CR1.FIFOEN",
            "USART1.CR1.M0",
            "USART1.CR1.M1",
            "USART1.CR2.RXINV",
            "USART1.CR2.TXINV",
            "USART1.CR3.ONEBIT"
        ])
    );
    // The console: its pattern out, its frame echoed, the RX pin routed at
    // the injection, and its writes the bring-up's (`AET-WO-4300b`).
    let serial = &b["serial"][0];
    assert_eq!(serial["label"], "CONSOLE");
    assert_eq!(serial["peripheral"], "USART1");
    assert_eq!(serial["brr"], 556);
    assert_eq!(serial["silicon"]["holds"], true);
    assert_eq!(serial["silicon"]["tolerance"]["table"], "Table 407");
    assert_eq!(serial["twin"]["patterns"], 1);
    assert_eq!(serial["twin"]["echo"], "witnessed");
    assert_eq!(serial["twin"]["rx_at_injection"], "PA10 MODER 0b10, AF7");
    assert_eq!(serial["twin"]["tx_at_end"], "PA9 MODER 0b10, AF7");
    // Renode's access log: the bring-up's four writes and one byte, PA9
    // routed at it (`AET-WO-4300b` fix 1).
    assert_eq!(serial["twin"]["usart_writes"], 5);
    assert_eq!(serial["twin"]["tx_at_send"], "PA9 MODER 0b10, AF7");
    assert_eq!(
        serial["speed"],
        "serial pin speed not judged: off-board load undeclared"
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
        "0x0028A801\n",
        "0x00[00:00:00.093570000] [WARNING] sysbus: x\n28A801\n",
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
    untrusted(boot(&good().replace("0x00001010\n", "")));
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
    // A set word beside a halt was untrusted (exit 5) by `AET-WO-2100`'s
    // ruled A10; `AET-WO-2310` overturns it: the firmware's `cpu_halted`.
    let (code, v) = boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F423F",
        "0x00000064",
        &GOOD_REGISTERS,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["cpu_halted"]);
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
    // Untrusted by its speed, never the firmware's `cpu_halted` (`AET-WO-2310`).
    let (code, v) = boot(&with(
        "",
        "0x2100B007",
        "0x0000000000002710",
        "0x00000001",
        &GOOD_REGISTERS,
    ));
    assert!(
        v["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("at 1 MIPS, not the platform's 100")),
        "{v}"
    );
    untrusted((code, v));
}

#[test]
fn a_register_off_the_solved_image_fails() {
    let mut regs = GOOD_REGISTERS;
    regs[5] = "0x55600000"; // PA7 on AF6, not the solved AF5
    regs[11] = "0x00001001"; // a clock the topology did not solve
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
    regs[4] = "0x0028A8FD";
    let (code, v) = boot(&with(
        "",
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &regs,
    ));
    assert_eq!(code, Some(0), "{v}");
}

/// `AET-WO-2204` A2, ruled D6 on PR #131: "parked" claims no interrupt is
/// armed. SysTick armed, or counted to 0, or any NVIC line enabled, after the
/// boot word, fails by name; `CLKSOURCE`, which Renode reads as 1 whatever is
/// written, does not count (`AET-WO-4210`).
#[test]
fn an_armed_interrupt_after_the_boot_word_fails() {
    for (at, value, name) in [
        (18, "0x00000007", "SysTick.CSR"),
        (18, "0x00010004", "SysTick.CSR"),
        (19, "0x00000001", "NVIC.ISER0"),
        (23, "0x00200000", "NVIC.ISER4"),
    ] {
        let mut regs = GOOD_REGISTERS;
        regs[at] = value;
        let (code, v) = boot(&with(
            "",
            "0x2100B007",
            "0x00000000000F4240",
            "0x00000064",
            &regs,
        ));
        assert_eq!(code, Some(1), "{v}");
        assert_eq!(kinds(&v), ["register_mismatch"], "{v}");
        let m = v["findings"][0]["message"].as_str().unwrap();
        assert!(m.starts_with(name), "{m}");
    }
    let mut regs = GOOD_REGISTERS;
    regs[18] = "0x00000000";
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
    let log = "[00:00:00.000000000] [WARNING] sysbus: [cpu: 0x8000100] Tried to write a locked peripheral: machine-0.gpioPortA. Address 0x58020000, value 0x5\n";
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

/// `AET-WO-4400g` §1.1–§1.2 (ruled on PR #229): each clock serves one run
/// kind. `--foreign` on a specification with no `mcu.foreign_clock` is
/// `ForeignClockUndeclared`, never a boot at the solved clock; a generated
/// run on one that declares it is `ForeignClockOnGeneratedRun`. Both are
/// refused before any input is read or a tool looked for (exit 3, no
/// `renode` on `PATH`), so even a firmware that is not there gets the name.
#[test]
fn a_clock_never_crosses_its_run_kind() {
    let empty = Dir::new("empty");
    let input = Dir::new("in");
    let missing = input.0.join("absent.elf");
    let (declaring, declared_platform) = common::foreign::write(&input.0);
    let args = |spec: &Path, platform: &Path, kind: &str, time: &str| -> Value {
        let out = run(
            &empty.0,
            &[
                OsStr::new("simulate"),
                OsStr::new("--platform"),
                platform.as_os_str(),
                OsStr::new("--firmware"),
                missing.as_os_str(),
                OsStr::new("--spec"),
                spec.as_os_str(),
                OsStr::new(kind),
                OsStr::new(time),
            ],
        );
        let v = report(&out);
        assert_eq!(out.status.code(), Some(3), "{v}");
        v
    };
    let v = args(&spec(), &platform(), "--foreign", "10ms");
    assert_eq!(v["error"]["kind"], "ForeignClockUndeclared", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mcu.foreign_clock"),
        "{v}"
    );
    let v = args(&declaring, &declared_platform, "--for", "10ms");
    assert_eq!(v["error"]["kind"], "ForeignClockOnGeneratedRun", "{v}");
}

/// `AET-WO-4400g` §1.2, the check wired into the judge: the reference
/// declared at 64 MHz, and the RCC registers reading back as ArduPilot left
/// them (200 MHz timers, measured), fails by name on both buses, with both
/// figures; at reset they hold (adversarial pass: dropping the check from
/// the judge survived every tier-1 test).
#[test]
fn a_foreign_clock_the_rcc_registers_contradict_fails_by_name() {
    let ardupilot = [
        "0x3F003025",
        "0x0000801B",
        "0x00000048",
        "0x00000440",
        "0x00408040",
        "0x01090231",
    ];
    let (code, v) = foreign(&foreign_transcript_rcc(
        &good_foreign_log(),
        "0x00000800",
        &flash_unchanged(),
        &ardupilot,
    ));
    assert_eq!(code, Some(1), "{v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] != "info")
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["foreign_clock_contradicted"; 2], "{v}");
    let checked: Vec<(u64, u64)> = v["boot"]["clock"]["checked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["declared_hz"].as_u64().unwrap(),
                c["decoded_hz"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(checked, [(64_000_000, 200_000_000); 2], "{v}");
    assert_eq!(
        v["boot"]["clock"]["registers"][5]["value"], "0x01090231",
        "{v}"
    );
    let (code, v) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000800"));
    assert_eq!(code, Some(0), "{v}");
    assert!(
        v["boot"]["clock"]["checked"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["holds"] == true),
        "{v}"
    );
}

/// `AET-WO-4400g` §1.5: a boot on Renode 1.17.0 of any build but the pinned
/// one, the stock release's among them, is refused by name before it runs.
#[test]
fn a_boot_on_another_renode_build_is_refused_by_name() {
    let tb = Dir::new("tb-stock");
    let out = tb.write("renode.out", good().as_bytes());
    common::stub::write_executable(
        &tb.0.join("renode"),
        format!(
            "#!/bin/sh\n[ \"$1\" = --config ] || exit 97\nif [ \"$3\" = --version ]; then printf 'Renode v1.17.0\\n  build: 1.17.0+20260907gitf1dd1b4af\\n'; exit 0; fi\n/bin/cat '{}'\n",
            out.display()
        )
        .as_bytes(),
    );
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let refused = boot_with(&tb.0, &platform(), &fw, "10ms");
    let v = report(&refused);
    assert_eq!(refused.status.code(), Some(4), "{v}");
    assert_eq!(v["error"]["kind"], "RenodeBuildNotPinned", "{v}");
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

/// The boot's memory cap, 1.5 GiB (`RUL-13` as amended by `AET-WO-2313`),
/// written out so that a change to `renode::BOOT_BOUNDS` fails here.
const CAP_KIB: u64 = 1_572_864;

#[test]
fn a_boot_past_the_memory_cap_fails_and_names_it() {
    let (code, v, took) = bounded("", common::HOLD_PAST_CAP);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["memory_cap_exceeded"]);
    assert_eq!(v["tool"]["exit_code"], -1);
    assert!(v["boot"]["boot_word"]["observed"].is_null(), "{v}");
    // The reaper's own record (`AET-WO-2302` D6) shows the cap was passed:
    // it records the peak it killed at, never a time.
    let memory = &v["memory"];
    assert_eq!(memory["bound_kib"], CAP_KIB, "{v}");
    assert!(memory["peak_rss_kib"].as_u64().unwrap() > CAP_KIB, "{v}");
    // The kind already proves the reaper fired. The wall time need only show
    // the kill came before the stub's own 60 s sleep could end, so it is
    // bounded at 50 s: 10 s of margin, as for the wall bound below, for a
    // loaded runner's start-up and teardown (31.8 s was measured on CI, PR
    // #185, run 43970150780, against the former 30 s bound).
    assert!(took.as_secs() < 50, "{took:?}");
}

/// A tree between the former 1 GiB cap and today's 1.5 GiB is not reaped
/// (`AET-WO-2313`): the boot runs to its end and passes, and the reaper's
/// record shows the tree really was past 1 GiB.
#[test]
fn a_boot_between_one_and_one_and_a_half_gib_is_not_reaped() {
    let (code, v, _) = bounded(&good(), common::HOLD_BETWEEN_CAPS);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    let memory = &v["memory"];
    assert_eq!(memory["bound_kib"], CAP_KIB, "{v}");
    let peak = memory["peak_rss_kib"].as_u64().unwrap();
    assert!(peak > 1024 * 1024 && peak < CAP_KIB, "{v}");
}

/// The bridge keeps its own 1 GiB (`analog::BRIDGE_BOUNDS`, `AET-WO-2313`):
/// the same tree that a boot may hold kills a bridge at its memory bound,
/// well before its 30 s wall bound.
#[test]
fn a_bridge_between_one_and_one_and_a_half_gib_is_reaped_at_its_memory_bound() {
    let hogging: &'static str = format!("#!/bin/sh\n{}\n", common::HOLD_BETWEEN_CAPS).leak();
    let start = std::time::Instant::now();
    let (code, v) = pre_pass(Some(hogging), false);
    assert_eq!(code, Some(5), "{v}");
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
    assert_eq!(v["error"]["id"], "aethelgard-bridge", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Memory bound"),
        "{v}"
    );
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
}

#[test]
fn an_aborted_machine_killed_at_a_bound_is_cpu_aborted() {
    let log = "[00:00:00.092740000] [ERROR] machine-0: This machine has aborted. Emulation cannot continue until this machine is removed.\n";
    let (code, v, _) = bounded(log, common::HOLD_PAST_CAP);
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
    // (below the 1022 Ω floor), so the specification is the lowest that
    // solves. The stub answers what the real bridge answers for a 10 Ω
    // pull-up (`bridge/tests/edges.rs`); the judgement under test is the same.
    let (spec, platform) = variant(
        &dir,
        &[("\"pull_up_ohms\": 2200", "\"pull_up_ohms\": 1022")],
    );
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
    // witness after the test's log; then U3's (`AET-WO-4241`).
    let n = r.len();
    r[n - 6] = record[0].to_owned();
    r[n - 5] = record[1].to_owned();
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
            "reference": "U2", "address": "0x76", "step": "identity", "register": "0xd0",
            "width": 1, "expected": "0x60", "mask": "0xff", "observed": "0x60", "status": "ok"
        }])
    );
    // U3 (TMP117) is listed, not passed silently, and does not fail the run.
    let f = finding(&v, "identity_not_witnessable").expect("U3 is reported");
    assert_eq!(f["severity"], "info");
    assert!(f["message"].as_str().unwrap().contains("U3 (TMP117)"));
    // `AET-WO-4241` §1.3: U3's read is in `not_witnessable` alone, never in
    // the witness arrays the campaign archives as I2C and SPI witnesses,
    // which are what `AET-WO-2306`'s droop `judged` counts.
    let b = &v["boot"];
    assert_eq!(b["not_witnessable"]["count"], 1, "{v}");
    assert_eq!(b["not_witnessable"]["reads"][0]["reference"], "U3", "{v}");
    for list in ["i2c", "i2c_witness", "spi", "spi_witness"] {
        let refs: Vec<&str> = b[list]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|r| r["reference"].as_str())
            .collect();
        assert!(!refs.contains(&"U3"), "{list}: {refs:?}");
    }
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
    // U4's record, before U3's unmodelled one (`AET-WO-4241`).
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
    // Nor its witness, nor where the core parked.
    assert_eq!(kinds(&v), ["boot_word_missing"], "{v}");
    // The unmodelled device is still reported on a failed boot.
    assert!(finding(&v, "identity_not_witnessable").is_some());
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
            "peripheral": "I2C1", "reference": "U2", "step": "identity", "device": "i2c1.u2",
            "expected": lines, "observed": lines, "absent": []
        }])
    );
    assert_eq!(v["boot"]["icsr"], "0x00000800");
    // Other log lines around the witness are not the witness.
    let around = format!(
        "[00:00:00.037000000] [INFO] machine-0: Machine started.\n{GOOD_WITNESS}[00:00:00.037900000] [WARNING] i2c1: Unhandled write to offset 0x1C.\n"
    );
    let (code, v) = boot(&with_log(GOOD_RECORD, &around));
    assert_eq!(code, Some(0), "{v}");
    // The controller's `Unknown slave` lines are kept for attribution, never
    // judged: here a stray transfer to 0x77 beside a witnessed read passes.
    let stray = format!(
        "[00:00:00.037000000] [WARNING] i2c1: Unknown slave at address 119.\n{GOOD_WITNESS}"
    );
    let (code, v) = boot(&with_log(GOOD_RECORD, &stray));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        v["boot"]["i2c_witness"][0]["absent"],
        serde_json::json!(["0x77"])
    );
}

#[test]
fn any_witness_but_the_model_s_exact_lines_fails() {
    let w = |a: &str, b: &str| format!("[00:00:00.037620000] {a}\n[00:00:00.037650000] {b}\n");
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
        "[00:00:00.037620000] [INFO] i2c1.u2: Write D0\n[00:00:00.037650000] [INFO] i2c1.u2: Read 60\n",
        "[NOISY] i2c1.u2: Write D0\n[NOISY] i2c1.u2: Read 60\n",
        "[00:00:00.037620000] [NOISY] usart1: [NOISY] i2c1.u2: Write D0\n[00:00:00.037650000] [NOISY] usart1: [NOISY] i2c1.u2: Read 60\n",
        "x[00:00:00.037620000] [NOISY] i2c1.u2: Write D0\nx[00:00:00.037650000] [NOISY] i2c1.u2: Read 60\n",
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
    let locked = "[00:00:00.000010000] [WARNING] sysbus: Tried to read a locked peripheral: machine-0.i2c1. Address 0x40005404.";
    let r = after(&[locked, locked]);
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(0), "{v}");
    // The witness among the judge's reads is not the run's: unwitnessed.
    let r = after(&[
        "[00:00:00.000010000] [NOISY] i2c1.u2: Write D0",
        "[00:00:00.000020000] [NOISY] i2c1.u2: Read 60",
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
        "0x000000[00:00:00.000010000] [WARNING] sysbus: Tried to read a locked peripheral",
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
    let flood = "[00:00:00.040000000] [WARNING] i2c1: Receive buffer underflow!\n".repeat(160_000);
    let (code, v) = boot(&with_log(GOOD_RECORD, &format!("{GOOD_WITNESS}{flood}")));
    assert_eq!(code, Some(1), "{v}");
    // Every read, U4's on SPI too (`AET-WO-4200`), the console's writes and
    // its TX pin at the first TDR write (`AET-WO-4300b`), the flash
    // controller's one read (`AET-WO-4230` D10), the PWR's writes
    // (`AET-WO-4240v` §1.6), and the injection, read from the hooks' own log
    // lines since `AET-WO-2302`: a dropped line could be one, so the echo is
    // never counted.
    assert_eq!(
        kinds(&v),
        [
            "flash_controller_access",
            "pwr_access",
            "i2c_transaction_unwitnessed",
            "spi_transaction_unwitnessed",
            "serial_tag_mismatch",
            "serial_pattern_missing",
            "serial_echo_missing"
        ],
        "{v}"
    );
    assert!(finding(&v, "renode_log_flood").is_some(), "{v}");
    assert_eq!(v["boot"]["i2c_witness"][0]["observed"], Value::Null);
    assert!(
        finding(&v, "serial_echo_missing").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("no frame went in: Renode's log could not be read whole"),
        "{v}"
    );
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
/// device's lines, in either device's order (`AET-WO-4210`: `2204` A4).
#[test]
fn two_devices_witness_their_own_reads_in_either_order() {
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
    let line = |t: &str, d: &str, l: &str| format!("[00:00:00.0{t}0000] [NOISY] i2c1.{d}: {l}\n");
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
            &NO_SERIAL_REGISTERS,
        );
        // `two-bme.json` declares `bor_level` 0: the model's own word, as
        // the one read of it returned (below).
        let at = r.iter().position(|w| w == BROWN_OUT).unwrap();
        r[at] = "0x0406AAF0".into();
        // U5's record after U2's, before U4's on SPI (`AET-WO-4200`) and
        // U3's unmodelled one (`AET-WO-4241`).
        let n = r.len();
        r.splice(n - 4..n - 4, GOOD_RECORD.iter().map(|s| (*s).to_owned()));
        // `two-bme.json` declares no serial port: no injection, no USART1.
        let text = transcript(
            &format!("{log}{GOOD_SPI_WITNESS}"),
            &r.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .replace(INJECTION, "")
        .replace(CPU_LISTING, &listing)
        .replacen("returned 0x406AAFC\n", "returned 0x406AAF0\n", 1);
        // A stub that checks the configuration alone: two devices make
        // another command line.
        let tb = Dir::new("tb-two");
        let outf = tb.write("renode.out", text.as_bytes());
        let stub = tb.0.join("renode");
        common::stub::write_executable(&stub, format!(
                "#!/bin/sh\n[ \"$1\" = --config ] || exit 97\nif [ \"$3\" = --version ]; then printf 'Renode v1.17.0\\n  build: {RENODE_BUILD}\\n'; exit 0; fi\n/bin/cat '{}'\n",
                outf.display()
            )
            .as_bytes(),);
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
    // Each device's own lines are right, U5's first: each device is judged
    // by its own steps in order, never across devices (`2204` A4, closed by
    // `AET-WO-4210`), so the devices may interleave.
    let (code, v) = boot_two(&format!("{u5}{u2}"));
    assert_eq!(code, Some(0), "{v}");
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

/// `AET-WO-2302`, ruled (a-i): every board boots `MAX_VIRTUAL_NS`, a board
/// with a DShot output too (it booted `DSHOT_WINDOW_NS`, 2 ms, refused past
/// it as `BootPastWindow`, under `AET-WO-4300a` D1). At 100 ms the inputs
/// pass to the missing tool; past it `--for` itself is refused, as on every
/// board.
#[test]
fn a_dshot_board_boots_the_whole_100_ms() {
    let empty = Dir::new("empty-dshot");
    let input = Dir::new("in-dshot");
    let spec =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/motors-dshot600.json");
    let bytes = fs::read(&spec).unwrap();
    let parsed: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = <sha2::Sha256 as sha2::Digest>::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&parsed.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&parsed, &table, &sha).unwrap();
    let platform = input.write(
        "dshot.repl",
        aethelgard_twin::platform::emit(&solved).unwrap().as_bytes(),
    );
    let good = input.write("good.elf", &elf());
    let boot = |time: &str| {
        run(
            &empty.0,
            &[
                OsStr::new("simulate"),
                OsStr::new("--platform"),
                platform.as_os_str(),
                OsStr::new("--firmware"),
                good.as_os_str(),
                OsStr::new("--spec"),
                spec.as_os_str(),
                OsStr::new("--for"),
                OsStr::new(time),
            ],
        )
    };
    // The whole boot: the inputs are good, and the missing tool stops it
    // (exit 4); 3 ms, once past the old window, the same.
    for time in ["100ms", "3ms", "2ms"] {
        let out = boot(time);
        assert_eq!(out.status.code(), Some(4), "{time}: {}", report(&out));
    }
    // Past `MAX_VIRTUAL_NS`, `--for` is refused on this board as on any.
    let out = boot("101ms");
    assert_ne!(out.status.code(), Some(4));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("BootPastWindow"));
}

// ---------------------------------------------------------------- AET-WO-4300b

/// One boot of the synthetic ELF whose stub writes `capture` as USART1's
/// file and then runs `then` (`$cap` is the file's path).
fn boot_capture(stdout: &str, capture: &[u8], then: &str) -> (Option<i32>, Value) {
    let tb = toolbox_capture(stdout, "Renode v1.17.0", then, capture);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let out = boot_with(&tb.0, &platform(), &fw, "10ms");
    (out.status.code(), report(&out))
}

fn console(v: &Value) -> &Value {
    &v["boot"]["serial"][0]["twin"]
}

/// `AET-WO-4300b` §2: bytes the firmware did not send fail by name. The
/// frame is the firmware's own (its sha256), so only bytes read from `RDR`
/// can be it; each capture must be its own pattern then the frame, exactly.
#[test]
fn a_capture_that_is_not_the_pattern_then_the_frame_fails_by_name() {
    let whole = capture_of(&elf());
    let p = aethelgard_twin::serial::pattern("CONSOLE");
    let frame = &whole[p.len()..];
    let cases: [(&str, Vec<u8>, &[&str]); 8] = [
        ("pattern only", p.clone(), &["serial_echo_missing"]),
        (
            "a pattern cut short in a boot no reset ended",
            [&p[..3], &whole[..]].concat(),
            &["serial_capture_forged"],
        ),
        ("frame only", frame.to_vec(), &["serial_pattern_missing"]),
        (
            "a byte after the frame",
            [&whole[..], &[0]].concat(),
            &["serial_capture_forged", "serial_echo_missing"],
        ),
        (
            "two patterns in one boot",
            [&p[..], &whole[..]].concat(),
            &["serial_capture_forged"],
        ),
        (
            "another port's pattern",
            [&aethelgard_twin::serial::pattern("GPS")[..], frame].concat(),
            &[
                "serial_capture_forged",
                "serial_pattern_missing",
                "serial_echo_missing",
            ],
        ),
        (
            "three of the frame's bytes",
            [&p[..], &frame[..3]].concat(),
            &["serial_echo_missing"],
        ),
        (
            "nothing",
            Vec::new(),
            &["serial_pattern_missing", "serial_echo_missing"],
        ),
    ];
    for (what, bytes, want) in cases {
        let (code, v) = boot_capture(&good(), &bytes, "");
        assert_eq!(code, Some(1), "{what}: {v}");
        assert_eq!(kinds(&v), want, "{what}: {v}");
    }
    let (code, v) = boot_capture(&good(), &[&p[..], &frame[..3]].concat(), "");
    assert_eq!(code, Some(1));
    assert_eq!(console(&v)["echo"], "partial", "{v}");
    assert_eq!(console(&v)["echoed"], 3, "{v}");
    assert!(
        finding(&v, "serial_echo_missing").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("3 of the frame's 8 bytes"),
        "{v}"
    );
}

/// `AET-WO-4300b` §2, a capture filled from the host: anything in the
/// capture directory but the ports' own files is forged, whatever the boot
/// did; a link or a file past the cap is never read.
#[test]
fn anything_beside_the_ports_files_in_the_capture_directory_is_forged() {
    let good_bytes = capture_of(&elf());
    for (what, then) in [
        ("a moved file", r#"printf planted > "$cap.1""#),
        ("a link", r#"/bin/ln -s /etc/hostname "$cap.x""#),
        ("a directory", r#"/bin/mkdir "$cap.d""#),
    ] {
        let (code, v) = boot_capture(&good(), &good_bytes, then);
        assert_eq!(code, Some(1), "{what}: {v}");
        assert_eq!(kinds(&v), ["serial_capture_forged"], "{what}: {v}");
    }
    // The port's own file replaced by a link: not read, and no capture.
    let (code, v) = boot_capture(
        &good(),
        &good_bytes,
        r#"/bin/rm "$cap"; /bin/ln -s /etc/hostname "$cap""#,
    );
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["serial_capture_forged", "serial_capture_forged"],
        "{v}"
    );
    // Past the cap.
    let big = vec![0x55u8; 64 * 1024 + 1];
    let (code, v) = boot_capture(&good(), &big, "");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["serial_capture_forged", "serial_capture_forged"],
        "{v}"
    );
    // Without the boot word, the witness is not judged, but a planted file
    // still is.
    let unbooted = with(
        "",
        "0x00000000",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let (code, v) = boot_capture(&unbooted, &[], "");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["boot_word_missing"], "{v}");
    assert_eq!(console(&v)["echo"], "not_judged", "{v}");
    let (_, v) = boot_capture(&unbooted, &[], r#"printf planted > "$cap.1""#);
    assert_eq!(
        kinds(&v),
        ["boot_word_missing", "serial_capture_forged"],
        "{v}"
    );
}

/// `AET-WO-4300b` FEEDBACK P5.1: Renode routes no pin to a USART, so the
/// judge reads the RX pin's mux at the injection: off its AF, or not in AF
/// mode, the echo does not count; the TX pin off its AF at the end, the
/// pattern does not.
#[test]
fn a_pin_off_its_af_does_not_count_its_bytes() {
    let capture = capture_of(&elf());
    for (moder, afr, at) in [
        ("28a801", "870", "PA10 MODER 0b10, AF8"),
        ("38a801", "770", "PA10 MODER 0b11, AF7"),
    ] {
        let text = good().replacen("28a801 770", &format!("{moder} {afr}"), 1);
        let (code, v) = boot_capture(&text, &capture, "");
        assert_eq!(code, Some(1), "{v}");
        assert_eq!(kinds(&v), ["serial_echo_missing"], "{v}");
        assert_eq!(console(&v)["echo"], "not_counted", "{v}");
        assert_eq!(console(&v)["rx_at_injection"], at, "{v}");
        assert_eq!(console(&v)["patterns"], 1, "{v}");
    }
    // PA9 on AF8 at the end: its pattern never reached the pin.
    let mut regs = GOOD_REGISTERS;
    regs[6] = "0x00000780";
    let text = with("", "0x2100B007", "0x00000000000F4240", "0x00000064", &regs);
    let (code, v) = boot_capture(&text, &capture, "");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["register_mismatch", "serial_pattern_missing"],
        "{v}"
    );
    assert!(
        finding(&v, "serial_pattern_missing").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("TX pin ends at PA9 MODER 0b10, AF8"),
        "{v}"
    );
}

/// `AET-WO-2302` D4: the injection is read from the CLI's own hook lines,
/// whole and in place. A frame that did not go in once, whole and in order
/// from the boot word, is no injection and fails the echo by name; a line of
/// ours in no shape it writes, or stamped where its count does not place
/// it, is refused as untrusted (exit 5). It took the place of the
/// `INJECT-BEGIN`/`END` block (adversarial pass 2), each refusal of which
/// has its counterpart here.
#[test]
fn the_injection_s_lines_are_read_whole_and_in_place() {
    let capture = capture_of(&elf());
    let first =
        "[00:00:00.000125750] [WARNING] sysbus: AETHELGARD-INJECT 0 0 2575 12575 28a801 770\n";
    let second = "[00:00:00.000225750] [WARNING] sysbus: AETHELGARD-INJECT 0 1 2575 22575\n";
    // No lines (the boot word never observed), a byte twice (the boot word
    // written over another word twice), a byte missing in the middle: the
    // echo is not counted, named.
    for (what, text, why) in [
        (
            "no lines",
            good().replacen(INJECTION, "", 1),
            "no frame went in: no write of the boot word was observed by its hook",
        ),
        (
            "a byte twice",
            good().replacen(second, &second.repeat(2), 1),
            "not one frame in order from one anchor",
        ),
        (
            "a byte missing",
            good().replacen(second, "", 1),
            "not one frame in order from one anchor",
        ),
    ] {
        let (code, v) = boot_capture(&text, &capture, "");
        assert_eq!(code, Some(1), "{what}: {v}");
        assert_eq!(kinds(&v), ["serial_echo_missing"], "{what}: {v}");
        assert_eq!(console(&v)["rx_at_injection"], Value::Null, "{what}: {v}");
        let m = finding(&v, "serial_echo_missing").unwrap()["message"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(m.contains(why), "{what}: {m}");
    }
    // A missing read of the mux, text in a line, a line before its count's
    // instant or a quantum after it: untrusted.
    for (what, text) in [
        (
            "a missing read",
            good().replacen(first, &first.replace(" 28a801 770", ""), 1),
        ),
        (
            "text in a line",
            good().replacen(first, &first.replace(" 770", " hello"), 1),
        ),
        (
            "stamped after its instant",
            good().replacen(first, &first.replace("000125750", "000125760"), 1),
        ),
        (
            "stamped a quantum early",
            good().replacen(first, &first.replace("000125750", "000025750"), 1),
        ),
    ] {
        let (code, v) = boot_capture(&text, &capture, "");
        assert_eq!(code, Some(5), "{what}: {v}");
        assert_eq!(v["error"]["kind"], "ToolError", "{what}: {v}");
    }
    // A whole log line among them is another source's, and skipped.
    let text = good().replacen(
        second,
        &format!("[00:00:00.000200000] [WARNING] sysbus: x\n{second}"),
        1,
    );
    let (code, v) = boot_capture(&text, &capture, "");
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(console(&v)["injected_ns"], 125_750, "{v}");
    assert_eq!(v["boot"]["boot_word_ns"], 25_750, "{v}");
}

/// `AET-WO-4300b` §1.5, adversarial F1–F3: a write the bring-up never
/// makes, from the port's own USART, fails, whatever its offset or width;
/// one from another source is not the port's.
#[test]
fn a_write_the_bring_up_never_makes_fails_and_another_source_s_does_not_count() {
    let capture = capture_of(&elf());
    let write = |source: &str, op: &str, at: &str, value: &str| {
        format!(
            "[00:00:00.000100000] [INFO] {source}: [cpu: 0x800070A] {op} to {at} (TransmitData+0x4), value {value}\n"
        )
    };
    for (line, unexpected) in [
        (write("usart1", "WriteUInt32", "0x2C", "0x1"), "0x1 to 0x2C"),
        (
            write("usart1", "WriteByte", "0x1", "0x10"),
            "WriteByte to 0x1",
        ),
        (
            write("usart1", "WriteUInt32", "0x4", "0x2000"),
            "CR2 0x2000 after UE",
        ),
    ] {
        let text = with(
            &line,
            "0x2100B007",
            "0x00000000000F4240",
            "0x00000064",
            &GOOD_REGISTERS,
        );
        let (code, v) = boot_capture(&text, &capture, "");
        assert_eq!(code, Some(1), "{v}");
        assert_eq!(kinds(&v), ["serial_tag_mismatch"], "{v}");
        assert_eq!(
            console(&v)["tags_unexpected"],
            serde_json::json!([unexpected])
        );
    }
    let text = with(
        &write("usart2", "WriteUInt32", "0x2C", "0x1"),
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let (code, v) = boot_capture(&text, &capture, "");
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(console(&v)["usart_writes"], 5);
    // A line of the port's own `INFO` log that is no whole access: its
    // writes cannot be rebuilt, which fails closed.
    let text = with(
        "[00:00:00.000100000] [INFO] usart1: something else\n",
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let (code, v) = boot_capture(&text, &capture, "");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["serial_tag_mismatch", "serial_pattern_missing"],
        "{v}"
    );
}

/// The ruling's amendment 3, under `AET-WO-2302` D4: a `--for` too short to
/// hold the injection, one quantum after the boot word, is not cut to fit.
/// The run is one `RunFor` with the hooks; the frame never goes in, and
/// once the boot word is set the echo fails closed, by name, and so does the
/// ports' wire time past the run (`serial_wire_past_run`).
#[test]
fn a_boot_too_short_for_the_injection_fails_closed_on_the_echo() {
    let tb = Dir::new("tb-short");
    let reads = reads(
        "0x2100B007",
        "0x0000000000002EE0",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    let text = transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &reads.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .replacen(
        INJECTION,
        "[00:00:00.000025750] [WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 2575\n",
        1,
    );
    let out = tb.write("renode.out", text.as_bytes());
    let bytes = tb.write("capture.bin", &aethelgard_twin::serial::pattern("CONSOLE"));
    // A stub that checks the configuration, that the run is one `RunFor` of
    // 120 us after the boot word's hook, no block of reads, and writes the
    // capture.
    let stub = tb.0.join("renode");
    common::stub::write_executable(&stub, format!(
            r#"#!/bin/sh
[ "$1" = --config ] || exit 97
if [ "$3" = --version ]; then printf 'Renode v1.17.0\n  build: {RENODE_BUILD}\n'; exit 0; fi
case "$7" in *INJECT-BEGIN*) exit 98;; esac
case "$7" in *"CreateFileBackend @"*"/capture/usart1.bin true; {HOOK}; emulation RunFor \"0.0001200005\"; echo \"AETHELGARD-BOOT-BEGIN\""*) ;; *) exit 99;; esac
cap=$(printf '%s' "$7" | /bin/sed -n 's/.*CreateFileBackend @\([^ ]*\) true.*/\1/p')
/bin/cat '{}' > "$cap" || exit 95
/bin/cat '{}'
"#,
            bytes.display(),
            out.display()
        )
        .as_bytes(),);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let out = boot_with(&tb.0, &platform(), &fw, "120us");
    let v = report(&out);
    assert_eq!(out.status.code(), Some(1), "{v}");
    assert_eq!(
        kinds(&v),
        ["serial_echo_missing", "serial_wire_past_run"],
        "{v}"
    );
    assert!(
        finding(&v, "serial_echo_missing").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("no frame went in: the boot ended before the boot word's quantum"),
        "{v}"
    );
    assert!(
        finding(&v, "serial_wire_past_run").unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("from one quantum after the boot word at 25750 ns, passes the boot's end at 120000 ns"),
        "{v}"
    );
    assert_eq!(console(&v)["patterns"], 1, "{v}");
    assert_eq!(console(&v)["rx_at_injection"], Value::Null, "{v}");
}

/// The PM's ruling on adversarial F5, under `AET-WO-2302`: a port's pattern,
/// the frame and one echoed character are judged before the boot from the
/// earliest instant the frame can go in, one quantum after the boot's start,
/// against `MAX_VIRTUAL_NS`, which every board now boots. The DShot600
/// fixture with a 9600 8N1 console (`serial-dshot-9600.json`: 22 342 347 ns
/// on the wire) was refused against `DSHOT_WINDOW_NS`; it now fits, and its
/// inputs pass to the stub. The same board at 1200 8N1 does not fit even
/// from 100 us: refused by name, and the stub is never asked for a boot.
#[test]
fn a_console_too_slow_for_the_boot_is_refused_before_it() {
    let tb = toolbox("");
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let fixture = root().join("crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json");
    let slow = input.write(
        "serial-dshot-1200.json",
        fs::read_to_string(&fixture)
            .unwrap()
            .replace("\"baud\": 9600", "\"baud\": 1200")
            .as_bytes(),
    );
    let simulate = |spec: &Path, label: &str| {
        // The platform the twin emits for it (`generate` starts no process).
        let generated = input.0.join(label);
        let made = run(
            &tb.0,
            &[
                OsStr::new("generate"),
                OsStr::new("--spec"),
                spec.as_os_str(),
                OsStr::new("--out"),
                generated.as_os_str(),
            ],
        );
        assert_eq!(made.status.code(), Some(0), "{made:?}");
        let platform = generated.join("board/motors-dshot600-console.repl");
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
                OsStr::new("100ms"),
            ],
        );
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code(), v)
    };
    let (code, v) = simulate(&fixture, "gen-9600");
    assert_ne!(code, Some(3), "{v}");
    assert_ne!(v["error"]["kind"], "SerialWireTimePastWindow", "{v}");
    let (code, v) = simulate(&slow, "gen-1200");
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "SerialWireTimePastWindow", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("serial port `CONSOLE` (USART1, 1200 8N1)")
            && m.contains(
                "from the earliest injection at 100000 ns, past MAX_VIRTUAL_NS of 100000000 ns"
            ),
        "{m}"
    );
}

/// `AET-WO-4230` §1.6, D10, WO §5.3 (lie about the brown-out level): word 1
/// of the boot record must hold the declared `BOR_LEV` (the reference
/// declares 3, which the platform presents); another level, or the judge's
/// fill (whose bits 3:2 read 3), fails by name, naming both.
#[test]
fn a_brown_out_word_that_is_not_the_declared_level_fails_naming_both() {
    let (code, v) = boot(&good());
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        v["boot"]["brown_out"],
        serde_json::json!({
            "level": 3, "address": "0x20000004", "mask": "0x0000000c",
            "expected": "0x0000000c", "observed": "0x0406aafc", "flash_access": []
        })
    );
    for (word, says) in [
        (
            "0x0406AAF0",
            "reads 0x0406aaf0: BOR_LEV 0, and the specification declares level 3",
        ),
        (
            "0x0406AAF8",
            "reads 0x0406aaf8: BOR_LEV 2, and the specification declares level 3",
        ),
        (
            "0xFFFFFFFF",
            "reads 0xffffffff, the judge's fill: the firmware never recorded BOR_LEV; the specification declares level 3",
        ),
    ] {
        // The platform presented that word, and the firmware recorded what
        // it read (the judge's fill aside, which no read returns).
        let read = match word {
            "0xFFFFFFFF" => "0x406AAFC".to_owned(),
            w => format!("0x{}", w[2..].trim_start_matches('0')),
        };
        let text = good()
            .replace(&format!("\n{BROWN_OUT}\n"), &format!("\n{word}\n"))
            .replacen("returned 0x406AAFC\n", &format!("returned {read}\n"), 1);
        let (code, v) = boot(&text);
        assert_eq!(code, Some(1), "{word}: {v}");
        assert_eq!(kinds(&v), ["brown_out_level_not_programmed"], "{word}");
        let m = finding(&v, "brown_out_level_not_programmed").unwrap()["message"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            m,
            format!("the boot record's FLASH_OPTSR_CUR word at 0x20000004 {says}")
        );
    }
    // Before the boot word, nothing of it is judged, as a record is not.
    let text = good()
        .replacen("\n0x2100B007\n", "\n0x00000000\n", 1)
        .replace(&format!("\n{BROWN_OUT}\n"), "\n0xFFFFFFFF\n");
    let (_, v) = boot(&text);
    assert!(
        finding(&v, "brown_out_level_not_programmed").is_none(),
        "{v}"
    );
}

/// `AET-WO-4230` D10, WO §5.3: the flash controller's access log must show
/// exactly the bring-up's one word read of `FLASH_OPTSR_CUR`: a firmware
/// that programs the option bytes itself, reads nothing, or whose log
/// cannot be read whole fails by name.
#[test]
fn the_flash_controller_is_read_once_and_never_written_or_the_boot_fails() {
    let read = "[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000102] ReadUInt32 from 0x1C (OptionStatusCurrent), returned 0x406AAFC\n";
    let program = "[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000104] WriteUInt32 to 0x8 (OptionKey), value 0x8192A3B\n\
[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000104] WriteUInt32 to 0x8 (OptionKey), value 0x4C5D6E7F\n\
[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000104] WriteUInt32 to 0x20 (OptionStatusProgram), value 0x406AAFC\n\
[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000104] WriteUInt32 to 0x18 (OptionControl), value 0x2\n";
    for (edit, says) in [
        (
            format!("{program}{read}"),
            "the firmware wrote the flash controller (32 bits at 0x8, value 0x8192A3B); the firmware wrote the flash controller (32 bits at 0x8, value 0x4C5D6E7F); the firmware wrote the flash controller (32 bits at 0x20, value 0x406AAFC); the firmware wrote the flash controller (32 bits at 0x18, value 0x2)",
        ),
        (
            String::new(),
            "the firmware read FLASH_OPTSR_CUR 0 times in the boot the reads judge, not once",
        ),
        (
            format!("{read}{read}"),
            "the firmware read FLASH_OPTSR_CUR 2 times in the boot the reads judge, not once",
        ),
        (
            // A line of the logged source that is not a whole access.
            "[00:00:00.000000000] [INFO] flashController: ReadUInt32 from 0x1C (OptionStatusCurrent), returned 0x406AAFC\n".to_owned(),
            "the flash controller's access log could not be read whole, so its one read of FLASH_OPTSR_CUR is unwitnessed",
        ),
    ] {
        let text = good().replacen(read, &edit, 1);
        let (code, v) = boot(&text);
        assert_eq!(code, Some(1), "{says}: {v}");
        assert_eq!(kinds(&v), ["flash_controller_access"], "{says}");
        assert_eq!(
            finding(&v, "flash_controller_access").unwrap()["message"],
            says
        );
    }
}

/// `AET-WO-4230` D10 (the adversarial pass's F2, `adv_bor.rs`): word 1 of the
/// boot record must be, exactly, the value the one read of `OPTSR_CUR`
/// returned. A read of `BOR_LEV` 00 (0x0406AAF0, a part fresh from the
/// factory) recorded as the declared level 3 is a firmware that wrote a
/// word it did not read, and the boot fails by name, naming both.
#[test]
fn word_one_that_is_not_the_value_the_flash_read_returned_fails_naming_both() {
    let read3 = "returned 0x406AAFC\n";
    let text = good();
    assert!(text.contains(read3));
    let forged = text.replacen(read3, "returned 0x406AAF0\n", 1);
    let (code, v) = boot(&forged);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["flash_controller_access"], "{v}");
    assert_eq!(
        finding(&v, "flash_controller_access").unwrap()["message"],
        "the boot record's word 1 is 0x0406AAFC, not 0x0406AAF0, the value the firmware's one read of FLASH_OPTSR_CUR returned"
    );
}

/// `AET-WO-4230` B0: an SPI read whose bounded wait for `EOT` expired is a
/// timeout naming it, on SPI alone; on I2C the same word is invalid.
#[test]
fn an_spi_read_whose_eot_wait_expired_fails_as_a_timeout_naming_eot() {
    let mut r = reads(
        "0x2100B007",
        "0x00000000000F4240",
        "0x00000064",
        &GOOD_REGISTERS,
    );
    // U4's status, before U3's unmodelled record (`AET-WO-4241`).
    let n = r.len();
    r[n - 4] = "0x22020006".into();
    let (code, v) = boot(&transcript(
        &format!("{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        &r.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(code, Some(1), "{v}");
    let f = finding(&v, "spi_timeout").unwrap_or_else(|| panic!("{v}"));
    assert!(f["message"].as_str().unwrap().contains("EOT"), "{f}");
    assert_eq!(v["boot"]["spi"][0]["status"], "timeout_eot");
    let (code, v) = boot(&with_record(["0x22020006", "0x00000060"]));
    assert_eq!(code, Some(1), "{v}");
    assert!(finding(&v, "i2c_record_invalid").is_some(), "{v}");
}

// ---------------------------------------------------------------- AET-WO-4400e

/// A `renode` stub for a foreign boot (`AET-WO-4400e`): it answers
/// `--version`, accepts only the foreign command line (the ELF right after
/// the platform, SPI1's access log for D4,
/// a `RunFor` of `time`, and none of the generated runtime's contract: no
/// boot word written, no hook of ours, no capture), and prints `stdout`.
/// A stub Renode for a foreign boot. With `flash`, the run must hash
/// [`elf`]'s one flash segment (64 bytes at `0x08000000`) after the reads and
/// before the listing; without it, nothing is hashed.
fn toolbox_foreign(stdout: &str, time: &str, flash: bool) -> Dir {
    toolbox_foreign_then(stdout, time, flash, "")
}

/// As [`toolbox_foreign`], running `then` after its output.
fn toolbox_foreign_then(stdout: &str, time: &str, flash: bool, then: &str) -> Dir {
    let tb = Dir::new("tb");
    let hash = match flash {
        true => {
            r#"case "$7" in *"echo \"AETHELGARD-BOOT-END\"; python \"print('AETHELGARD-FLASH 8000000 ' + "*"ReadBytes(0x8000000, 0x40)"*"; peripherals; quit") ;; *) exit 93;; esac"#
        }
        false => r#"case "$7" in *AETHELGARD-FLASH*) exit 93;; esac"#,
    };
    let out = tb.write("renode.out", stdout.as_bytes());
    let body = format!(
        r#"#!/bin/sh
[ "$1" = --config ] || exit 97
if [ "$3" = --version ]; then printf 'Renode v1.17.0\n  build: {RENODE_BUILD}\n'; exit 0; fi
case "$7" in
  *"mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; "*) ;;
  *) exit 99;;
esac
case "$7" in *AddWatchpointHook*|*"WriteDoubleWord 0x20000000"*|*CreateFileBackend*|*LogPeripheralAccess\ sysbus.flashController*) exit 98;; esac
case "$7" in *"LogPeripheralAccess sysbus.spi"*) exit 94;; esac
case "$7" in *"sysbus SetHookBeforePeripheralWrite sysbus.spi1 \"(offset == 0xC) and "*) ;; *) exit 96;; esac
case "$7" in *"sysbus SetHookBeforePeripheralWrite sysbus.flashController \"((offset in (0xc, 0x10c)) and (value >> 7) & 1 or offset in (0x18, 0x118) and ((value >> 4) & 1 or (value >> 1) & 1)) and "*) ;; *) exit 92;; esac
case "$7" in *"emulation RunFor \"{time}\""*) ;; *) exit 95;; esac
case "$7" in *"cpu AddHook 0x8000030 \"r = lambda x: "*"ok = v in (3, 4, 5, 6, ); "*"setattr(self, 'IsHalted', True)\"; "*"emulation RunFor"*) ;; *) exit 91;; esac
{hash}
/bin/cat '{out}'
{then}
"#,
        out = out.display()
    );
    common::stub::write_executable(&tb.0.join("renode"), body.as_bytes());
    tb
}

/// The BME280's listed read (`parts.json` `reads`, a burst from 0xF7), as
/// Renode 1.17.0's model logs ArduPilot's (`AET-WO-4400e` §1.1, measured).
const BME280_READ: &str = "[00:00:00.050000000] [NOISY] i2c1.u2: Write F7\n[00:00:00.050001000] [NOISY] i2c1.u2: Read 0 0 90 80 0 D0\n";

/// [`foreign_elf`]'s one loaded flash segment as Renode hashes it after the
/// run: [`foreign_code`] unchanged.
fn flash_unchanged() -> String {
    let sha: String = <sha2::Sha256 as sha2::Digest>::digest(foreign_code())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("AETHELGARD-FLASH 8000000 {sha}\n")
}

/// A foreign transcript of [`elf`]: the run's log, then the foreign reads
/// (executed instructions, MIPS, PC, ICSR, then every image register in
/// address order), its flash unchanged, then the listing.
fn foreign_transcript(run_log: &str, icsr: &str) -> String {
    foreign_transcript_flash(run_log, icsr, &flash_unchanged())
}

/// A foreign transcript with `flash`, the flash hash lines, as given.
fn foreign_transcript_flash(run_log: &str, icsr: &str, flash: &str) -> String {
    foreign_transcript_rcc(run_log, icsr, flash, &common::foreign::RCC_AT_RESET)
}

/// The reference board's CONSOLE (USART1) as unmodified ArduPilot brings it
/// up in the access log (`AET-WO-2311`, `spike/2311/out/m4`): `CR2` with
/// `UE` clear, then `CR1` with `UE`. Every foreign transcript starts with it,
/// so its serial options hold unless a test plants a word after it.
const CONSOLE_BRINGUP: &str = "[00:00:00.003200000] [INFO] usart1: [cpu: 0x80FDE5C] WriteUInt32 to 0x4 (ControlRegister2), value 0x40\n\
[00:00:00.003200000] [INFO] usart1: [cpu: 0x80FDE6E] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000013D\n";

/// GPIOA's `PUPDR` as unmodified ArduPilot leaves it: PA9 and PA10, the
/// CONSOLE's TX and RX, pulled up; read after the RCC registers.
const CONSOLE_PULLS: &str = "0x00140000";

/// A foreign transcript whose RCC registers read back as `rcc`.
fn foreign_transcript_rcc(run_log: &str, icsr: &str, flash: &str, rcc: &[&str; 6]) -> String {
    let mut s = String::from(
        "(monitor) emulation SetSeed 2100;  echo \"AETHELGARD-BOOT-BEGIN\";  echo \"AETHELGARD-BOOT-END\"\n",
    );
    s.push_str(CONSOLE_BRINGUP);
    s.push_str(run_log);
    s.push_str("AETHELGARD-BOOT-BEGIN\n");
    // Then the RCC registers the clock check reads (`AET-WO-4400g`).
    // Then each serial line's `PUPDR` (`AET-WO-2311`).
    for r in ["0x0000000000123456", "0x00000064", "0x8000010", icsr]
        .iter()
        .chain(GOOD_REGISTERS.iter())
        .chain(rcc.iter())
        .chain([CONSOLE_PULLS].iter())
    {
        s.push_str(r);
        s.push('\n');
    }
    s.push_str("AETHELGARD-BOOT-END\n");
    s.push_str(flash);
    s.push_str(CPU_LISTING);
    s
}

/// A `--foreign` boot of `firmware` on the reference board with a foreign
/// clock declared, and its platform (`AET-WO-4400g`), each written for it.
fn foreign_with(path: &Path, firmware: &Path, time: &str) -> Output {
    let dir = Dir::new("foreign-in");
    let (spec, platform) = common::foreign::write(&dir.0);
    run(
        path,
        &[
            OsStr::new("simulate"),
            OsStr::new("--platform"),
            platform.as_os_str(),
            OsStr::new("--firmware"),
            firmware.as_os_str(),
            OsStr::new("--spec"),
            spec.as_os_str(),
            OsStr::new("--foreign"),
            OsStr::new(time),
        ],
    )
}

fn foreign(stdout: &str) -> (Option<i32>, Value) {
    foreign_of(stdout, &foreign_elf())
}

fn foreign_of(stdout: &str, firmware: &[u8]) -> (Option<i32>, Value) {
    let tb = toolbox_foreign(stdout, "0.0100000005", true);
    let input = Dir::new("in");
    let fw = input.write("fw.elf", firmware);
    let out = foreign_with(&tb.0, &fw, "10ms");
    (out.status.code(), report(&out))
}

fn good_foreign_log() -> String {
    format!("{GOOD_WITNESS}{BME280_READ}{GOOD_SPI_WITNESS}")
}

/// `AET-WO-1609`: the panic hook's line, the CPU's own, fails a foreign
/// run that would otherwise pass, by name, with each entry in `boot.panic`;
/// the same boot without it passes with none. A line at another address,
/// or of another source, is not an entry.
#[test]
fn a_foreign_boot_that_enters_the_panic_fails_by_name() {
    let (code, v) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000800"));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        v["boot"]["panic"],
        serde_json::json!({"symbol": "_ZN6AP_HAL5panicEPKcz", "address": "0x08000020", "entered_ns": []}),
        "{v}"
    );
    let entered = format!(
        "{}[00:00:00.009000000] [WARNING] cpu: AETHELGARD-PANIC 8000020\n",
        good_foreign_log()
    );
    let (code, v) = foreign(&foreign_transcript(&entered, "0x00000800"));
    assert_eq!(code, Some(1), "{v}");
    let failing: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] == "error")
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(failing, ["foreign_panic"], "{v}");
    assert_eq!(
        v["boot"]["panic"]["entered_ns"],
        serde_json::json!([9_000_000]),
        "{v}"
    );
    // A line Renode printed after the begin marker still counts (adversarial
    // review, finding 3): only the CPU writes it.
    let late = foreign_transcript(&good_foreign_log(), "0x00000800").replacen(
        "AETHELGARD-BOOT-BEGIN\n",
        "AETHELGARD-BOOT-BEGIN\n[00:00:00.010000000] [WARNING] cpu: AETHELGARD-PANIC 8000020\n",
        1,
    );
    let (code, v) = foreign(&late);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        v["boot"]["panic"]["entered_ns"],
        serde_json::json!([10_000_000]),
        "{v}"
    );
    for other in [
        "[00:00:00.009000000] [WARNING] cpu: AETHELGARD-PANIC 8000030\n",
        "[00:00:00.009000000] [WARNING] usart1: AETHELGARD-PANIC 8000020\n",
    ] {
        let log = format!("{}{other}", good_foreign_log());
        let (code, v) = foreign(&foreign_transcript(&log, "0x00000800"));
        assert_eq!(code, Some(0), "{other}: {v}");
    }
}

/// `AET-WO-1609`: a foreign firmware whose symbol table does not name the
/// panic is refused by name before any boot (`INV-06`).
#[test]
fn a_foreign_firmware_with_no_panic_symbol_is_refused() {
    let (code, v) = foreign_of(
        &foreign_transcript(&good_foreign_log(), "0x00000800"),
        &elf(),
    );
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "ForeignPanicSymbolMissing", "{v}");
}

/// `AET-WO-4400e` D3, D5, D6 (i): a foreign boot is judged by its models'
/// own lines, booted at the first identity a declared device's model logs,
/// and its verdict says `judge: foreign` with the rule table; none of the
/// generated runtime's fields is there.
#[test]
fn a_foreign_boot_is_judged_by_its_models_lines_and_says_so() {
    let (code, v) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000800"));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    // The build that judged it, as Renode named it (`AET-WO-4400g` §1.5).
    assert_eq!(v["tool"]["build"], RENODE_BUILD, "{v}");
    let b = &v["boot"];
    assert_eq!(b["judge"], "foreign", "{v}");
    assert_eq!(b["window"], "FOREIGN_MAX_VIRTUAL_NS");
    assert_eq!(b["booted"]["reference"], "U2", "{v}");
    assert_eq!(b["booted"]["at_ns"], 37_620_000, "{v}");
    assert!(
        b.get("boot_word").is_none() && b.get("i2c").is_none(),
        "{v}"
    );
    assert_eq!(b["memory_regions"].as_array().unwrap().len(), 7);
    let steps: Vec<(String, String, Value)> = b["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["reference"].as_str().unwrap().to_owned(),
                d["step"].as_str().unwrap().to_owned(),
                d["observed_ns"].clone(),
            )
        })
        .collect();
    assert!(
        steps.contains(&("U2".into(), "identity".into(), 37_620_000.into())),
        "{steps:?}"
    );
    assert!(
        steps.contains(&("U2".into(), "data (reads[0])".into(), 50_000_000.into())),
        "{steps:?}"
    );
    assert!(
        steps.contains(&("U4".into(), "identity".into(), 40_010_000.into())),
        "{steps:?}"
    );
    let rules = b["rules"].as_array().unwrap();
    assert!(
        rules
            .iter()
            .any(|r| r["rule"] == "device configure" && r["disposition"] == "dropped")
    );
    assert!(
        rules
            .iter()
            .all(|r| r["judges"].as_str().is_some_and(|j| !j.is_empty()))
    );
}

/// D5: with no identity a declared device's model logged, nothing booted;
/// a device's data read before its identity does not count (D6 (i)).
#[test]
fn a_foreign_boot_no_model_witnesses_fails_by_name() {
    let (code, v) = foreign(&foreign_transcript("", "0x00000800"));
    assert_eq!(code, Some(1), "{v}");
    let k = kinds(&v);
    assert!(k.contains(&"foreign_not_booted".to_owned()), "{k:?}");
    assert!(
        k.contains(&"i2c_transaction_unwitnessed".to_owned()),
        "{k:?}"
    );
    let (_, v) = foreign(&foreign_transcript(
        &format!("{BME280_READ}{GOOD_WITNESS}{GOOD_SPI_WITNESS}"),
        "0x00000800",
    ));
    assert_eq!(v["boot"]["booted"]["reference"], "U2", "{v}");
    assert!(
        kinds(&v).contains(&"i2c_transaction_unwitnessed".to_owned()),
        "{v}"
    );
}

/// `AET-WO-2311`: the CONSOLE's serial options are judged from its access
/// log, end to end: its pulls are info and named in `boot.serial_pulls`,
/// and a planted `CR2` word with SWAP, written with `UE` clear, fails by name.
#[test]
fn a_foreign_boot_judges_the_serial_options_and_reports_the_pulls() {
    let (code, v) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000800"));
    assert_eq!(code, Some(0), "{v}");
    let b = &v["boot"];
    assert_eq!(b["serial_options"][0]["label"], "CONSOLE", "{v}");
    assert_eq!(b["serial_options"][0]["judged"], "0x00000040", "{v}");
    assert_eq!(b["serial_options"][0]["judged_ns"], 3_200_000, "{v}");
    assert_eq!(
        b["serial_options"][0]["expected"][0]["field"], "SWAP",
        "{v}"
    );
    assert_eq!(
        b["serial_options"][0]["expected"][0]["expected"], "clear",
        "{v}"
    );
    let pulls: Vec<(String, String)> = b["serial_pulls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["pin"].as_str().unwrap().into(),
                p["pupdr"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        pulls,
        [
            ("PA9".into(), "pull-up".into()),
            ("PA10".into(), "pull-up".into())
        ]
    );
    let info: Vec<_> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "serial_pull")
        .collect();
    assert_eq!(info.len(), 2, "{v}");
    assert!(info.iter().all(|f| f["severity"] == "info"), "{v}");
    let swap = "[00:00:00.060000000] [INFO] usart1: [cpu: 0x80F6AF2] WriteUInt32 to 0x0 (ControlRegister1), value 0x10C\n\
[00:00:00.060000000] [INFO] usart1: [cpu: 0x80FDE5C] WriteUInt32 to 0x4 (ControlRegister2), value 0x8040\n";
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{swap}", good_foreign_log()),
        "0x00000800",
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["serial_swap_set"], "{v}");
}

/// D4 (ruled on PR #215; its evidence narrowed by D7 (b), PR #212): the SPI
/// model's slave-mode `[ERROR]` is exempt only directly after a `CFG2` write
/// with `MASTER` 0 while `CR1` held `SPE` 0, as the `CFG2` hook logs it; any
/// other `[ERROR]` fails.
#[test]
fn the_slave_mode_error_is_exempt_only_after_spi_lld_stop_s_writes() {
    let stop = "[00:00:00.041000000] [WARNING] spi1: AETHELGARD-CFG2 0 0\n\
[00:00:00.041000000] [ERROR] spi1: Attempted to set peripheral into SPI slave mode. Only master mode is supported\n";
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{stop}", good_foreign_log()),
        "0x00000800",
    ));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["boot"]["spi_errors_exempted"], 1, "{v}");
    let enabled = stop.replace("AETHELGARD-CFG2 0 0", "AETHELGARD-CFG2 0 1");
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{enabled}", good_foreign_log()),
        "0x00000800",
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["renode_error_log"], "{v}");
    assert_eq!(v["boot"]["spi_errors_exempted"], 0, "{v}");
}

/// Ruled on PR #212 (comment 6965511448): the firmware's own loaded flash
/// bytes, hashed in Renode after the run, must be the ELF's; an access to an
/// unclocked peripheral is an `info` finding listing each address; Renode's
/// `Unhandled write` warnings are one line per kind with their count and
/// first and last time (D7 (a)).
#[test]
fn a_foreign_boot_s_flash_locked_accesses_and_warnings_are_reported() {
    let other = format!("AETHELGARD-FLASH 8000000 {}\n", "0".repeat(64));
    let (code, v) = foreign(&foreign_transcript_flash(
        &good_foreign_log(),
        "0x00000800",
        &other,
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["flash_overwritten"], "{v}");
    // An erase of its own sector fails even when its bytes hash unchanged.
    let erase = "[00:00:00.020000000] [WARNING] flashController: AETHELGARD-ERASE c 84\n";
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{erase}", good_foreign_log()),
        "0x00000800",
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["flash_overwritten"], "{v}");
    assert_eq!(v["boot"]["erases"][0]["start"], "0x08000000", "{v}");
    // Another sector's erase is reported and passes.
    let (code, v) = foreign(&foreign_transcript(
        &format!(
            "{}{}",
            good_foreign_log(),
            erase.replace(" c 84", " 10c 784")
        ),
        "0x00000800",
    ));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["boot"]["erases"][0]["start"], "0x081e0000", "{v}");
    // A mass erase (MER) erases its own flash; an option-byte change
    // (OPTSTART) is info, with its time.
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{}", good_foreign_log(), erase.replace(" c 84", " 18 10")),
        "0x00000800",
    ));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["flash_overwritten"], "{v}");
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{}", good_foreign_log(), erase.replace(" c 84", " 18 2")),
        "0x00000800",
    ));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["boot"]["option_changes"][0], 20_000_000u64, "{v}");
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["kind"] == "flash_option_change" && f["severity"] == "info"),
        "{v}"
    );
    let (_, v) = foreign(&foreign_transcript_flash(
        &good_foreign_log(),
        "0x00000800",
        "",
    ));
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
    let noise = "[00:00:00.030000000] [WARNING] sysbus: Tried to write a locked peripheral: machine-0.dma1. Address 0x40020010.\n\
[00:00:00.031000000] [WARNING] sysbus: Tried to write a locked peripheral: machine-0.dma1. Address 0x40020010.\n\
[00:00:00.032000000] [WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x30012340.\n\
[00:00:00.033000000] [WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x30012360.\n\
[00:00:00.034000000] [WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x30012380.\n";
    let (code, v) = foreign(&foreign_transcript(
        &format!("{}{noise}", good_foreign_log()),
        "0x00000800",
    ));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    let b = &v["boot"];
    assert_eq!(b["flash"][0]["address"], "0x08000000", "{v}");
    assert_eq!(
        b["flash"][0]["expected_sha256"], b["flash"][0]["observed_sha256"],
        "{v}"
    );
    assert_eq!(b["locked"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(b["locked"][0]["count"], 2, "{v}");
    assert_eq!(b["locked"][0]["first_ns"], 30_000_000u64, "{v}");
    assert_eq!(b["warnings"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(b["warnings"][0]["count"], 3, "{v}");
    assert_eq!(b["warnings"][0]["first_ns"], 32_000_000u64, "{v}");
    assert_eq!(b["warnings"][0]["last_ns"], 34_000_000u64, "{v}");
    assert_eq!(b["log_budget_bytes"], 8 * 1024 * 1024, "{v}");
    assert_eq!(b["log_lines_dropped"], 0, "{v}");
    assert!(b["log_kept_bytes"].as_u64().unwrap() > 0, "{v}");
    let infos: Vec<_> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "locked_peripheral_access")
        .collect();
    assert_eq!(infos.len(), 1, "{v}");
    assert_eq!(infos[0]["severity"], "info", "{v}");
}

/// D5: the run must end in thread mode, as for a generated firmware.
#[test]
fn a_foreign_boot_ending_in_an_exception_fails() {
    let (code, v) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000803"));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(kinds(&v), ["core_in_exception"], "{v}");
}

/// D3: the judge is declared, never inferred. `--for` and `--foreign`
/// together are refused, as is a foreign window past its bound; a generated
/// boot's verdict names no judge; an ELF with RAM in D2 SRAM is refused
/// without the declaration and loads with it, beside a flash segment that
/// holds its vector table (one with no table is refused for that,
/// `AET-WO-2314`); an ELF outside every region is refused either way
/// (`FirmwareOutsideMemory`).
#[test]
fn the_foreign_judge_is_declared_and_its_memory_is_the_header_s_regions() {
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &elf());
    let tb = toolbox(&good());
    let both = run(
        &tb.0,
        &[
            OsStr::new("simulate"),
            OsStr::new("--platform"),
            platform().as_os_str(),
            OsStr::new("--firmware"),
            fw.as_os_str(),
            OsStr::new("--spec"),
            spec().as_os_str(),
            OsStr::new("--for"),
            OsStr::new("10ms"),
            OsStr::new("--foreign"),
            OsStr::new("10ms"),
        ],
    );
    assert_eq!(both.status.code(), Some(64), "{both:?}");
    let past = foreign_with(&tb.0, &fw, "5001ms");
    assert_eq!(past.status.code(), Some(64), "{past:?}");
    let (status, v) = boot(&good());
    assert_eq!(status, Some(0), "{v}");
    assert!(v["boot"].get("judge").is_none(), "{v}");
    // A firmware whose one segment is in D2 SRAM, as ArduPilot's .data is.
    let d2 = input.write(
        "d2.elf",
        &common::foreign::with_panic(elf_at(0x3000_0000, &code(), 40), 0x3000_0020),
    );
    let generated = boot_with(&tb.0, &platform(), &d2, "10ms");
    assert_eq!(generated.status.code(), Some(3), "{generated:?}");
    assert_eq!(report(&generated)["error"]["kind"], "FirmwareOutsideMemory");
    // The D2 firmware loads nothing into flash, so nothing is hashed.
    let tf = toolbox_foreign(
        &foreign_transcript_flash(&good_foreign_log(), "0x00000800", ""),
        "0.0100000005",
        false,
    );
    // It passes the memory rule, and is refused next because nothing it
    // loads holds a vector table to hook its faults from (`AET-WO-2314`).
    let loaded = foreign_with(&tf.0, &d2, "10ms");
    let e = report(&loaded);
    assert_eq!(e["error"]["kind"], "ForeignFaultVectorUnusable", "{e}");
    assert!(
        e["error"]["message"].as_str().unwrap().contains(
            "no single PT_LOAD's file bytes hold the vector table's fault entries at 0x0800000c"
        ),
        "{e}"
    );
    let with_table = input.write(
        "d2-flash.elf",
        &common::foreign::with_panic(
            elf_segments(
                40,
                &[(0x0800_0000, &foreign_code()), (0x3000_0000, &code())],
            ),
            0x0800_0020,
        ),
    );
    let tt = toolbox_foreign(
        &foreign_transcript(&good_foreign_log(), "0x00000800"),
        "0.0100000005",
        true,
    );
    let loaded = foreign_with(&tt.0, &with_table, "10ms");
    assert_eq!(loaded.status.code(), Some(0), "{loaded:?}");
    assert_eq!(report(&loaded)["boot"]["judge"], "foreign", "{loaded:?}");
    let outside = input.write("fmc.elf", &elf_at(0x6000_0000, &code(), 40));
    let refused = foreign_with(&tf.0, &outside, "10ms");
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    let e = report(&refused);
    assert_eq!(e["error"]["kind"], "FirmwareOutsideMemory", "{e}");
    assert!(
        e["error"]["message"]
            .as_str()
            .unwrap()
            .contains("D2_AHBSRAM_BASE"),
        "{e}"
    );
}

// ---------------------------------------------------------------- AET-WO-2314

/// `AET-WO-2314` D2: the CLI hooks the fault handler its vector table names
/// (the stub refuses a command line without it, exit 91), and a run whose
/// CPU logged the hook's mark fails `foreign_fault`, naming the vector from
/// `ICSR.VECTACTIVE` and the PC the exception stacked, beside
/// `core_in_exception`. A mark at another address is not an entry.
#[test]
fn a_foreign_boot_that_takes_a_fault_fails_by_name_with_its_vector_and_pc() {
    let mark = |body: &str| {
        format!(
            "{}[00:00:00.004392440] [WARNING] cpu: AETHELGARD-FAULT {body}\n",
            good_foreign_log()
        )
    };
    let (code, v) = foreign(&foreign_transcript(
        &mark("3 8000030 30027328 80360aa"),
        "0x00000803",
    ));
    assert_eq!(code, Some(1), "{v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] != "info")
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["core_in_exception", "foreign_fault"], "{v}");
    assert_eq!(
        finding(&v, "foreign_fault").unwrap()["message"],
        "the core took HardFault (exception 3) at 4392440 ns, from PC 0x080360aa (read from its stack frame at 0x30027328); its handler's hook halted the core there",
        "{v}"
    );
    let (_, v) = foreign(&foreign_transcript(
        &mark("4 8000030 30027328 8000010"),
        "0x00000804",
    ));
    assert!(
        finding(&v, "foreign_fault").unwrap()["message"]
            .as_str()
            .unwrap()
            .starts_with("the core took MemManage (exception 4) at "),
        "{v}"
    );
    let (code, v) = foreign(&foreign_transcript(
        &mark("3 8000032 30027328 80360aa"),
        "0x00000800",
    ));
    assert_eq!(code, Some(0), "{v}");
    assert!(finding(&v, "foreign_fault").is_none(), "{v}");
    assert!(
        v["boot"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["rule"] == "foreign_fault"),
        "{v}"
    );
}

/// `AET-WO-2314` adversarial pass: a run whose fault hook logged its mark
/// and that was then killed at a bound ended two ways, and names both.
#[test]
fn a_foreign_boot_that_faults_then_reaches_a_bound_names_both() {
    let log = format!(
        "{}[00:00:00.004392440] [WARNING] cpu: AETHELGARD-FAULT 3 8000030 30027328 80360aa\n",
        good_foreign_log()
    );
    let tb = toolbox_foreign_then(
        &foreign_transcript(&log, "0x00000803"),
        "0.0100000005",
        true,
        common::HOLD_PAST_CAP,
    );
    let input = Dir::new("in");
    let fw = input.write("fw.elf", &foreign_elf());
    let out = foreign_with(&tb.0, &fw, "10ms");
    let v = report(&out);
    assert_eq!(out.status.code(), Some(1), "{v}");
    assert_eq!(kinds(&v), ["memory_cap_exceeded", "foreign_fault"], "{v}");
}

/// `AET-WO-2314` D2 (i): a vector table whose fault entry is 0, or outside
/// every executable `PT_LOAD`, cannot be hooked, so the firmware is refused
/// by name before a boot (the stub would exit 99 on any boot); the D2-only
/// firmware with no table at all is `the_foreign_judge_is_declared…`'s. A 0
/// entry is refused even when code is loaded at address 0 (ITCM, a region
/// the foreign judge admits).
#[test]
fn a_foreign_firmware_whose_fault_vector_cannot_be_hooked_is_refused_by_name() {
    let tb = toolbox_foreign("", "0.0100000005", true);
    let input = Dir::new("in");
    let mut far = foreign_code();
    far[0x14..0x18].copy_from_slice(&0x0900_0001u32.to_le_bytes());
    let zero = "the vector table's HardFault entry (vector 3, at 0x0800000c) reads 0x00000000, no handler, so the foreign judge cannot hook it";
    for (name, elf, says) in [
        ("zero.elf", elf_at(0x0800_0000, &code(), 40), zero),
        (
            "itcm.elf",
            elf_segments(40, &[(0x0800_0000, &code()), (0x0000_0000, &[0u8; 16])]),
            zero,
        ),
        (
            "far.elf",
            elf_at(0x0800_0000, &far, 40),
            "the vector table's BusFault entry (vector 5, at 0x08000014) reads 0x09000001, outside every executable PT_LOAD, so the foreign judge cannot hook it",
        ),
    ] {
        let fw = input.write(name, &common::foreign::with_panic(elf, 0x0800_0020));
        let out = foreign_with(&tb.0, &fw, "10ms");
        let e = report(&out);
        assert_eq!(out.status.code(), Some(3), "{e}");
        assert_eq!(e["error"]["kind"], "ForeignFaultVectorUnusable", "{e}");
        assert_eq!(e["error"]["message"], says, "{e}");
    }
}

// ---------------------------------------------------------------------------
// `AET-WO-4400i`: a declared input (`simulate --foreign --input`).
// ---------------------------------------------------------------------------

/// Frames pymavlink 2.4.50 wrote in the pinned ArduPilot image, an encoder
/// that is not the judge's (the twin's `mavlink` tests hold the same).
const HB_GCS: &str = "fd09000000ffbe0000000000000006080000035c2b";
const ARM: &str = "fd20000000ffbe4c00000000803f000000000000000000000000000000000000000000000000900101019e4e";
const ARM_FORCE_21196: &str = "fd20000000ffbe4c00000000803f0098a546000000000000000000000000000000000000000090010101abb2";
const ARM_FORCE_2989: &str = "fd20000000ffbe4c00000000803f00d03a45000000000000000000000000000000000000000090010101b8e6";
const SKIPCHK_1: &str = "fd17000000ffbe1700000000803f010141524d494e475f534b495043484b000006f5d4";
const REQUIRE_0: &str = "fd17000000ffbe17000000000000010141524d494e475f52455155495245000002470c";
const SYSTEM_TIME: &str = "fd01000000ffbe020000008edf";
const SIGNED_HB: &str = "fd09010000ffbe000000000000000608000003bbd3000100000000003bc231d90cb8";
const UNKNOWN_PARAM: &str = "fd17000000ffbe1700000000803f01014145545f4e4f545f415f504152414d000948ef";

/// Copter 4.7.2's own USART1 transmissions from `spike/4400i` (the twin's
/// fixtures): `early-fill` arms (its first armed heartbeat at byte 11339),
/// `reject` answers `AET_NOT_A_PARAM` with `PARAM_ERROR`.
fn ardupilot_tx(name: &str) -> Vec<u8> {
    fs::read(root().join(format!("crates/aethelgard-twin/tests/data/4400i/{name}-tx.bin"))).unwrap()
}

/// An input file: `(at_ns, hex, fill)` frames for `port`.
fn input_file(dir: &Dir, name: &str, port: &str, frames: &[(u64, &str, u32)]) -> PathBuf {
    let frames: Vec<Value> = frames
        .iter()
        .map(|(t, h, f)| serde_json::json!({"at_ns": t, "mavlink": h, "fill": f}))
        .collect();
    let v = serde_json::json!({"input_version": 1, "port": port, "frames": frames});
    dir.write(name, &serde_json::to_vec(&v).unwrap())
}

/// A `--foreign 10ms` run of the foreign ELF with `input` (and `extra`
/// arguments), on `board` (a specification and its emitted platform; the
/// reference's by default), with `toolbox`'s `renode`.
fn input_run(toolbox: &Path, input: &Path, board: Option<(PathBuf, PathBuf)>, extra: &[&OsStr]) -> Output {
    let dir = Dir::new("input-in");
    let (spec, platform) = board.unwrap_or_else(|| common::foreign::write(&dir.0));
    let fw = dir.write("fw.elf", &foreign_elf());
    let mut args = vec![
        OsStr::new("simulate"),
        OsStr::new("--platform"),
        platform.as_os_str(),
        OsStr::new("--firmware"),
        fw.as_os_str(),
        OsStr::new("--spec"),
        spec.as_os_str(),
        OsStr::new("--foreign"),
        OsStr::new("10ms"),
        OsStr::new("--input"),
        input.as_os_str(),
    ];
    args.extend_from_slice(extra);
    run(toolbox, &args)
}

/// A stub `renode` for an input run: the command line includes the input's
/// timeline; the stub checks that file's first line is USART1's
/// `CreateFileBackend`, and, with `expected`, that every line after it is
/// exactly `expected` (the sandbox lets it write nothing outside its
/// scratch, so the test states the timeline rather than reading it back);
/// then writes `capture` where the backend names, as Renode's would, and
/// prints the good foreign transcript.
fn toolbox_input(capture: &[u8], expected: Option<&str>) -> Dir {
    let tb = Dir::new("tb-input");
    let out = tb.write(
        "renode.out",
        foreign_transcript(&good_foreign_log(), "0x00000800").as_bytes(),
    );
    let bytes = tb.write("capture.bin", capture);
    let check = match expected {
        Some(e) => format!(
            "/bin/sed 1d \"$script\" | /bin/cmp -s - '{}' || exit 90",
            tb.write("expected.resc", e.as_bytes()).display()
        ),
        None => String::new(),
    };
    let body = format!(
        r#"#!/bin/sh
[ "$1" = --config ] || exit 97
if [ "$3" = --version ]; then printf 'Renode v1.17.0\n  build: {RENODE_BUILD}\n'; exit 0; fi
case "$7" in *"; include @"*"/input.resc; echo \"AETHELGARD-BOOT-BEGIN\""*) ;; *) exit 95;; esac
case "$7" in *"emulation RunFor"*) exit 94;; esac
script=$(printf '%s' "$7" | /bin/sed -n 's/.*; include @\([^;]*\); echo .*/\1/p')
[ -r "$script" ] || exit 93
cap=$(/bin/sed -n '1s/^sysbus.usart1 CreateFileBackend @\(.*\) true$/\1/p' "$script")
[ -n "$cap" ] || exit 92
{check}
/bin/cat '{bytes}' > "$cap" || exit 91
/bin/cat '{out}'
"#,
        bytes = bytes.display(),
        out = out.display()
    );
    common::stub::write_executable(&tb.0.join("renode"), body.as_bytes());
    tb
}

/// `AET-WO-4400i` D8: each refusal of a declared input is named, with exit
/// 2, before any tool is looked for (no `renode` on `PATH`).
#[test]
fn each_declared_input_refusal_is_named_before_any_tool() {
    let empty = Dir::new("empty");
    let dir = Dir::new("inputs");
    let refused = |input: &Path, kind: &str| {
        let out = input_run(&empty.0, input, None, &[]);
        let v = report(&out);
        assert_eq!(v["error"]["kind"], kind, "{v}");
        assert_eq!(out.status.code(), Some(2), "{v}");
        v
    };
    let one = |name: &str, hex: &str| input_file(&dir, name, "CONSOLE", &[(1_000_000, hex, 0)]);
    let v = refused(&one("force.json", ARM_FORCE_21196), "InputForcesArm");
    assert!(v["error"]["message"].as_str().unwrap().contains("21196"), "{v}");
    refused(&one("force2989.json", ARM_FORCE_2989), "InputForcesArm");
    refused(&one("skip.json", SKIPCHK_1), "InputSkipsChecks");
    refused(&one("require.json", REQUIRE_0), "InputSkipsChecks");
    refused(&one("unlisted.json", SYSTEM_TIME), "InputMessageUnlisted");
    refused(&one("signed.json", SIGNED_HB), "InputSigned");
    refused(&one("text.json", &hex(b"ARM NOW")), "InputNotMavlink");
    refused(
        &input_file(&dir, "late.json", "CONSOLE", &[(9_000_000, HB_GCS, 0)]),
        "InputPastWindow",
    );
    refused(
        &input_file(&dir, "off.json", "CONSOLE", &[(1_000_050, HB_GCS, 0)]),
        "InputTimeOffQuantum",
    );
    refused(
        &input_file(&dir, "overlap.json", "CONSOLE", &[(1_000_000, HB_GCS, 0), (3_000_000, ARM, 0)]),
        "InputFramesOverlap",
    );
    refused(
        &input_file(&dir, "port.json", "RADIO", &[(1_000_000, HB_GCS, 0)]),
        "InputPortUndeclared",
    );
    refused(&dir.write("json.json", b"{\"input_version\": 1,"), "InputMalformed");
    refused(
        &dir.write("v2.json", br#"{"input_version": 2, "port": "CONSOLE", "frames": []}"#),
        "InputMalformed",
    );
    refused(
        &dir.write("odd.json", br#"{"input_version": 1, "port": "CONSOLE", "frames": [{"at_ns": 0, "mavlink": "fd0"}]}"#),
        "InputMalformed",
    );
    // A missing file is unreadable (exit 3).
    let out = input_run(&empty.0, &dir.0.join("absent.json"), None, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", report(&out));
    // Good frames, with the tool missing: the tool is what stops it.
    let out = input_run(&empty.0, &one("good.json", ARM), None, &[]);
    assert_eq!(out.status.code(), Some(4), "{}", report(&out));
}

/// `AET-WO-4400i` D4: a board with an IMU whose orientation is undeclared
/// has no rest to start an input run from: refused by name, before any tool.
#[test]
fn an_input_run_on_an_imu_with_no_orientation_is_refused() {
    let empty = Dir::new("empty");
    let dir = Dir::new("noorient");
    let mut v: Value = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json")).unwrap(),
    )
    .unwrap();
    for d in v["spi_devices"].as_array_mut().unwrap() {
        d.as_object_mut().unwrap().remove("orientation");
    }
    let bytes = serde_json::to_vec_pretty(&v).unwrap();
    let spec = dir.write("spec.json", &bytes);
    let s: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&s.mcu.part).unwrap();
    let hash: String = <sha2::Sha256 as sha2::Digest>::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let topology = aethelgard_ir::solve(&s, &table, &hash).unwrap();
    let platform = dir.write(
        "platform.repl",
        aethelgard_twin::platform::emit(&topology).unwrap().as_bytes(),
    );
    let input = input_file(&dir, "in.json", "CONSOLE", &[(1_000_000, HB_GCS, 0)]);
    let out = input_run(&empty.0, &input, Some((spec, platform)), &[]);
    let r = report(&out);
    assert_eq!(r["error"]["kind"], "RestOrientationUndeclared", "{r}");
    assert_eq!(out.status.code(), Some(2), "{r}");
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// `AET-WO-4400i` §2: each byte is sent at its stated instant from the
/// timeline file, the gap is named, and the arm is read from the firmware's
/// own transmission, never from the input.
#[test]
fn an_input_run_reads_the_arm_from_the_firmwares_own_heartbeat() {
    let tx = ardupilot_tx("early-fill");
    // The timeline, stated by hand: 1 ms, then the heartbeat's 21 bytes 100 us
    // apart; the ARM's 44 bytes and 11 bytes of fill from 4 ms; then the rest
    // of the 10 ms. The reference board has no IMU or barometer: no rest.
    let mut want = String::from("emulation RunFor \"0.0010000005\"\n");
    let send = |want: &mut String, hex: &str, fill: usize| {
        let mut bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        bytes.resize(bytes.len() + fill, 0);
        for (k, b) in bytes.iter().enumerate() {
            if k > 0 {
                want.push_str("emulation RunFor \"0.0001000005\"\n");
            }
            want.push_str(&format!("sysbus.usart1 WriteChar 0x{b:02X}\n"));
        }
    };
    send(&mut want, HB_GCS, 0);
    // From 3.0 ms (the heartbeat's last byte at 3.0 ms) to 4 ms.
    want.push_str("emulation RunFor \"0.0010000005\"\n");
    send(&mut want, ARM, 11);
    // The ARM is 44 bytes (a 32-byte payload): with its fill, 55 bytes, the
    // last at 4 + 54 × 0.1 = 9.4 ms; 0.6 ms to the end.
    want.push_str("emulation RunFor \"0.0006000005\"\n");
    let tb = toolbox_input(&tx, Some(&want));
    let dir = Dir::new("arm");
    // A heartbeat at 1 ms, then an ARM at 4 ms with 11 bytes of fill.
    let input = input_file(&dir, "in.json", "CONSOLE", &[(1_000_000, HB_GCS, 0), (4_000_000, ARM, 11)]);
    let out = input_run(&tb.0, &input, None, &[]);
    let v = report(&out);
    assert_eq!(out.status.code(), Some(0), "{v}");
    let i = &v["boot"]["input"];
    assert_eq!(i["input"]["frames"], 2, "{v}");
    assert_eq!(i["input"]["fill_bytes"], 11, "{v}");
    assert_eq!(i["input"]["spacing_ns"], 100_000, "{v}");
    let sha: String = <sha2::Sha256 as sha2::Digest>::digest(fs::read(&input).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(i["input"]["sha256"], sha, "{v}");
    assert_eq!(i["witness"]["armed"]["offset"], 11339, "{v}");
    assert_eq!(i["witness"]["armed"]["base_mode"], 209, "{v}");
    assert_eq!(i["witness"]["captured_bytes"], tx.len(), "{v}");
    assert!(
        v["boot"]["unobservable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u.as_str().unwrap().starts_with("usart_idle_unmodelled")),
        "{v}"
    );
    // No input: no `input`, so a run without one reports as before.
    let (_, plain) = foreign(&foreign_transcript(&good_foreign_log(), "0x00000800"));
    assert!(plain["boot"].get("input").is_none(), "{plain}");
}

/// `AET-WO-4400i` §5: an armed heartbeat the input itself carried, echoed
/// back on the port, is not the firmware's: it never witnesses the arm.
#[test]
fn an_armed_heartbeat_the_input_carried_never_witnesses_the_arm() {
    let tx = ardupilot_tx("early-fill");
    let forged = &tx[11339..11339 + 21];
    let tb = toolbox_input(forged, None);
    let dir = Dir::new("forged");
    let input = input_file(&dir, "in.json", "CONSOLE", &[(1_000_000, &hex(forged), 0)]);
    let out = input_run(&tb.0, &input, None, &[]);
    let v = report(&out);
    assert_eq!(out.status.code(), Some(0), "{v}");
    assert!(v["boot"]["input"]["witness"]["armed"].is_null(), "{v}");
    assert_eq!(v["boot"]["input"]["witness"]["heartbeats"], 0, "{v}");
}

/// `AET-WO-4400i` D9: a parameter the firmware does not have is reported
/// rejected, from its own `PARAM_ERROR`, and fails the run.
#[test]
fn a_parameter_the_firmware_rejects_is_reported_from_its_own_reply() {
    let tb = toolbox_input(&ardupilot_tx("reject"), None);
    let dir = Dir::new("reject");
    let input = input_file(&dir, "in.json", "CONSOLE", &[(1_000_000, UNKNOWN_PARAM, 0)]);
    let out = input_run(&tb.0, &input, None, &[]);
    let v = report(&out);
    assert_eq!(out.status.code(), Some(1), "{v}");
    let rejected: Vec<&Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "foreign_param_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "{v}");
    assert!(rejected[0]["message"].as_str().unwrap().contains("AET_NOT_A_PARAM"), "{v}");
    assert_eq!(v["boot"]["input"]["witness"]["params"][0]["error"], 1, "{v}");
}
