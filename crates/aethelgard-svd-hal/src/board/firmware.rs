//! The reference firmware crate (`AET-WO-2100`, R1 of the ruling on PR #56):
//! a dependency-free `#![no_std]` binary that uses only the board crate's own
//! API for hardware. It is generated, as a map from path (under `firmware/`)
//! to text, from the same [`Plan`] as the board crate, so the committed bytes
//! are guarded by the same regeneration test.
//!
//! The runtime is the smallest thing that can boot: a vector table, a reset
//! handler in `global_asm!` (zero `.bss`, copy `.data`, enable the FPU, call
//! `main`) and a linker script, with no runtime crate (R1, spike F6 and F7).
//! The memory map is read from ST's header, never typed here.
//!
//! [`BOOT_MAGIC`] and the boot word's place (the first word of RAM) are the
//! judge's contract in `aethelgard-twin`. This crate does not depend on the
//! twin, so the constant is written twice, and a test in `tests/board.rs`
//! compares the two.
//!
//! **The supply and the voltage scale first** (`AET-WO-4240v` §1.4, D-2).
//! `main`'s first hardware access is `Pwr::scale2`: `PWR_CR3`'s supply
//! configuration written once and read back, `PWR_CSR1.ACTVOSRDY` awaited,
//! `PWR_D3CR.VOS` Scale 2, `VOSRDY` awaited, each wait at most [`PWR_POLLS`]
//! reads. It runs in every firmware, a board that reads no clock included,
//! and nothing reads the clock tree or starts a peripheral before it (the
//! `Scale2` token). A fault writes its own boot word ([`BOOT_SUPPLY_NOT_DECLARED`],
//! [`BOOT_ACTVOSRDY_TIMEOUT`], [`BOOT_VOSRDY_TIMEOUT`]) and parks, with
//! nothing else written.
//!
//! **The residue: RAM written before `ACTVOSRDY`.** Until `ACTVOSRDY` reads 1
//! the part is in Run* mode, where "write accesses to the RAMs are not
//! permitted" (RM0433 Rev 9 §6.4.1, p. 262). The reset handler zeroes `.bss`
//! and copies `.data`, and the call to `main` pushes the stack, before the
//! sequence runs; so these RAM writes precede `ACTVOSRDY`. So does a fault's
//! boot word when the fault is [`BOOT_SUPPLY_NOT_DECLARED`] or
//! [`BOOT_ACTVOSRDY_TIMEOUT`]: on silicon that word may not land, and the
//! bench reads `PWR_CR3` and `PWR_CSR1` themselves. The twin models no Run*
//! mode.

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::parts::StepKind;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{
    BoardError, I2C_MODES, Kind, Plan, RESET_HZ, Role, Speeds, hsi_fastest, method, pin_field,
    port_field, type_name,
};
use crate::cmsis::{self, RegisterError, RegisterFacts};

// belt-and-suspenders: the judge holds its own copy
// (`aethelgard_twin::boot::BOOT_MAGIC`), so the expected value never comes from
// the generator (`INV-07`); tests/board.rs checks that the two agree.
/// What the firmware writes to the boot word at its fixed point.
pub const BOOT_MAGIC: u32 = 0x2100_B007;

// belt-and-suspenders: the judge holds its own copies
// (`aethelgard_twin::boot::BOOT_SUPPLY_NOT_DECLARED`, `BOOT_ACTVOSRDY_TIMEOUT`,
// `BOOT_VOSRDY_TIMEOUT`), so the expected values never come from the generator
// (`INV-07`); tests/board.rs checks that the two agree.
/// What the firmware writes to the boot word when `PWR_CR3` reads back a
/// supply configuration other than the declared one (`AET-WO-4240v` §1.4).
pub const BOOT_SUPPLY_NOT_DECLARED: u32 = 0x2100_B0F1;
/// What the firmware writes to the boot word when `PWR_CSR1.ACTVOSRDY` does
/// not read 1 within [`PWR_POLLS`] reads.
pub const BOOT_ACTVOSRDY_TIMEOUT: u32 = 0x2100_B0F2;
/// What the firmware writes to the boot word when `PWR_D3CR.VOSRDY` does not
/// read 1 within [`PWR_POLLS`] reads.
pub const BOOT_VOSRDY_TIMEOUT: u32 = 0x2100_B0F3;

/// The most reads of `PWR_CSR1` the `ACTVOSRDY` wait makes, and of `PWR_D3CR`
/// the `VOSRDY` wait. DS12117 Rev 10 states no settling time or maximum delay
/// for either bit (read at source, `AET-WO-4240v` reads §7); the only figure
/// in either document is RM0433 Rev 9 Figure 31's annotation of 20 µs
/// (p. 281), for VOS1 to VOS0 only, drawn, not specified as a maximum. So the
/// bound is an engine choice, set generously, as [`ECHO_POLLS`] is: at least
/// one core cycle a read, it waits at least 15 ms at HSI's fastest, 750 times
/// that figure.
pub const PWR_POLLS: u32 = 1_000_000;

// belt-and-suspenders: the judge holds its own copy of the record layout and
// the statuses (`aethelgard_twin::boot`), so what a record must hold never
// comes from the generator (`INV-07`); tests/board.rs checks that they agree.
/// The first record's place, in bytes: after the boot word (word 0) and the
/// brown-out word (word 1, `FLASH_OPTSR_CUR` as read; `AET-WO-4230` D10).
pub const RECORD_OFFSET: u32 = 8;
/// The brown-out word's place, in words.
const OPTSR_WORD: u32 = 1;
/// One record: the status word, then the value word.
pub const RECORD_BYTES: u32 = 8;
/// What a failed or impossible transfer leaves in a record, as the judge's fill.
pub const RECORD_UNSET: u32 = 0xFFFF_FFFF;
/// The statuses, by the wait that expired.
pub const STATUS_OK: u32 = 0x2202_0000;
pub const STATUS_TXIS: u32 = 0x2202_0001;
pub const STATUS_TC: u32 = 0x2202_0002;
pub const STATUS_RXNE: u32 = 0x2202_0003;
pub const STATUS_STOP: u32 = 0x2202_0004;
/// An SPI read whose wait for `SR.RXP` expired (`AET-WO-4200`).
pub const STATUS_RXP: u32 = 0x2202_0005;
/// An SPI read whose wait for `SR.EOT` expired after its last frame
/// (`AET-WO-4230`).
pub const STATUS_EOT: u32 = 0x2202_0006;

// belt-and-suspenders: the judge holds its own copies of the pattern and the
// frame's length (`INV-07`); the firmware never knows the frame itself.
/// What each serial port sends after its label's bytes (`AET-WO-4300b`).
pub const PATTERN_TAIL: [u8; 5] = [0x00, 0xFE, 0x7F, 0x80, 0xFF];
/// The most bytes each serial port echoes: the injected frame's length.
pub const FRAME_LEN: u32 = 8;
/// The most reads of `ISR` the echo makes waiting for each byte; a wait that
/// expires ends that port's echo.
pub const ECHO_POLLS: u32 = 20_000;

/// The memory the linker script describes, from the header.
struct Memory {
    flash_origin: u32,
    flash_length: u32,
    ram_origin: u32,
    ram_kib: u32,
}

fn memory(f: &RegisterFacts) -> Result<Memory, BoardError> {
    let flash_origin = f.value("FLASH_BANK1_BASE")?;
    let bank2 = f.value("FLASH_BANK2_BASE")?;
    let ram_origin = f.value("D1_DTCMRAM_BASE")?;
    let ram_kib = f.region_kib("D1_DTCMRAM_BASE")?;
    let bad = |detail: String| BoardError::Register(RegisterError::MalformedSource { detail });
    let flash_length = bank2
        .checked_sub(flash_origin)
        .filter(|&n| n > 0)
        .ok_or_else(|| bad("`FLASH_BANK2_BASE` is not above `FLASH_BANK1_BASE`".into()))?;
    let fits = ram_kib > 0
        && ram_kib
            .checked_mul(1024)
            .and_then(|n| n.checked_add(ram_origin))
            .is_some();
    if !fits {
        return Err(bad(format!(
            "`D1_DTCMRAM_BASE` of {ram_kib} KiB does not fit the address space"
        )));
    }
    Ok(Memory {
        flash_origin,
        flash_length,
        ram_origin,
        ram_kib,
    })
}

/// The provenance lines, without comment markers.
fn provenance(plan: &Plan) -> String {
    format!(
        "@generated by `aethelgard-svd-hal` (AET-WO-2100). Do not edit: the regeneration\n\
         test compares every byte of this tree with the generator's output.\n\
         \n\
         - Solved topology of specification sha256 `{spec}`.\n\
         - Register facts: `{repo}` at `{commit}`, `{file}` sha256 `{hash}`.\n\
         - Pin data: `STM32_open_pin_data` at `{pins}`.\n{banner}",
        spec = plan.specification_sha256,
        repo = cmsis::REPOSITORY,
        commit = cmsis::COMMIT,
        file = cmsis::HEADER_FILE,
        hash = cmsis::HEADER_SHA256,
        pins = plan.pin_commit,
        banner = plan
            .lifecycle
            .iter()
            .map(|l| format!("- {l}.\n"))
            .collect::<String>(),
    )
}

fn hash_header(plan: &Plan) -> String {
    provenance(plan)
        .lines()
        .map(|l| {
            if l.is_empty() {
                "#\n".to_owned()
            } else {
                format!("# {l}\n")
            }
        })
        .collect()
}

/// `sentinel-01-board` → `sentinel_01_board`.
fn ident(crate_name: &str) -> String {
    crate_name.replace('-', "_")
}

/// The firmware crate's files, keyed `firmware/...`.
pub(super) fn emit(
    plan: &Plan,
    f: &RegisterFacts,
    speeds: &Speeds,
    solved: &SolvedTopology,
    answers: &dyn Fn(&str) -> bool,
) -> Result<BTreeMap<String, String>, BoardError> {
    let mem = memory(f)?;
    let reads = reads(plan, solved, answers)?;
    let mut files = BTreeMap::new();
    files.insert("firmware/Cargo.toml".to_owned(), manifest(plan));
    files.insert("firmware/Cargo.lock".to_owned(), lock(plan));
    files.insert("firmware/.cargo/config.toml".to_owned(), config(plan));
    files.insert("firmware/link.x".to_owned(), linker(plan, &mem));
    files.insert(
        "firmware/src/main.rs".to_owned(),
        main_rs(plan, speeds, &reads),
    );
    Ok(files)
}

fn manifest(plan: &Plan) -> String {
    format!(
        "{header}\
         # Not a workspace member (the repository root excludes `boards/`), no build\n\
         # script (no host code runs when a board builds), and one dependency: the\n\
         # board crate beside it.\n\
         [package]\n\
         name = \"{name}-firmware\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         license = \"MIT OR Apache-2.0\"\n\
         \n\
         [dependencies]\n\
         {name} = {{ path = \"..\" }}\n\
         \n\
         [profile.dev]\n\
         panic = \"abort\"\n\
         \n\
         [profile.release]\n\
         panic = \"abort\"\n\
         opt-level = \"s\"\n",
        header = hash_header(plan),
        name = plan.crate_name
    )
}

/// What cargo writes for the board crate and the firmware that depends on it.
fn lock(plan: &Plan) -> String {
    format!(
        "# This file is automatically @generated by Cargo.\n\
         # It is not intended for manual editing.\n\
         version = 4\n\
         \n\
         [[package]]\n\
         name = \"{name}\"\n\
         version = \"0.0.0\"\n\
         \n\
         [[package]]\n\
         name = \"{name}-firmware\"\n\
         version = \"0.0.0\"\n\
         dependencies = [\n \"{name}\",\n]\n",
        name = plan.crate_name
    )
}

fn config(plan: &Plan) -> String {
    format!(
        "{header}\
         [build]\n\
         target = \"thumbv7em-none-eabihf\"\n\
         # Cargo resolves this against the directory that holds `.cargo/`: the\n\
         # repository's git-ignored top-level `target/`. A build must never write\n\
         # under `boards/`, which the regeneration test and `aethelgard prove` read.\n\
         target-dir = \"../../../target/{name}-firmware\"\n\
         \n\
         [target.thumbv7em-none-eabihf]\n\
         rustflags = [\"-C\", \"link-arg=-Tlink.x\"]\n",
        header = hash_header(plan),
        name = plan.crate_name
    )
}

fn linker(plan: &Plan, m: &Memory) -> String {
    let mut s = String::from("/*\n");
    for l in provenance(plan).lines() {
        let _ = writeln!(s, " *{}{l}", if l.is_empty() { "" } else { " " });
    }
    let _ = write!(
        s,
        " */\n\
         MEMORY\n\
         {{\n  \
         /* FLASH_BANK1_BASE, up to FLASH_BANK2_BASE: bank 1. */\n  \
         FLASH : ORIGIN = {fo:#010x}, LENGTH = {fl:#x}\n  \
         /* D1_DTCMRAM_BASE, of the {kib} KB its comment states. */\n  \
         RAM   : ORIGIN = {ro:#010x}, LENGTH = {kib}K\n\
         }}\n\
         \n\
         ENTRY(Reset);\n\
         \n\
         SECTIONS\n\
         {{\n  \
         .vector_table ORIGIN(FLASH) :\n  \
         {{\n    \
         LONG(ORIGIN(RAM) + LENGTH(RAM));   /* initial SP */\n    \
         KEEP(*(.vector_table.exceptions)); /* reset and the 14 core exceptions */\n  \
         }} > FLASH\n\
         \n  \
         .text : {{ *(.text .text.*); }} > FLASH\n  \
         .rodata : {{ *(.rodata .rodata.*); . = ALIGN(4); }} > FLASH\n\
         \n  \
         /* The observable word: first in RAM and never initialised by the reset\n     \
         handler, so a judge reads the program's write, not the reset value. */\n  \
         .aethelgard_boot (NOLOAD) : {{ KEEP(*(.aethelgard_boot)); }} > RAM\n\
         \n  \
         .data : AT(LOADADDR(.rodata) + SIZEOF(.rodata))\n  \
         {{\n    \
         _sdata = .; *(.data .data.*); . = ALIGN(4); _edata = .;\n  \
         }} > RAM\n  \
         _sidata = LOADADDR(.data);\n\
         \n  \
         .bss (NOLOAD) : {{ . = ALIGN(4); _sbss = .; *(.bss .bss.*); . = ALIGN(4); _ebss = .; }} > RAM\n\
         \n  \
         /DISCARD/ : {{ *(.ARM.exidx .ARM.exidx.*); }}\n\
         }}\n",
        fo = m.flash_origin,
        fl = m.flash_length,
        ro = m.ram_origin,
        kib = m.ram_kib,
    );
    s
}

/// The highest I2C mode a peripheral's declared speed admits.
fn highest(speeds: &Speeds, peripheral: &str) -> &'static str {
    // `generate` gives every I2C peripheral an entry with a non-empty list.
    let (_, admitted) = &speeds[peripheral];
    I2C_MODES[*admitted.last().expect("a declared speed admits a mode")].0
}

/// Where an identity read goes.
enum Target {
    /// An I2C device at its address.
    I2c { address: u8 },
    /// An SPI device: its bus's device enum, `Spi1Device`, and its variant.
    Spi { device: String, variant: String },
}

/// One step of a device's script the firmware makes (`AET-WO-4210`): its
/// identity read, or a scripted part's configuration write or data read.
struct Read {
    /// The peripheral's field, `i2c1` or `spi1`.
    bus: String,
    target: Target,
    /// The bytes written first: an I2C register or command, or an SPI command.
    write: Vec<u8>,
    /// The bytes read back; none for a configuration.
    width: u8,
    /// The part's cited start-up before its data read, in microseconds
    /// (`AET-WO-4220`), on its last configuration step; 0 on every other.
    settle_us: u32,
    /// A scripted part's data read: it comes after the one wait.
    data: bool,
}

/// The spins of [`settle`] that make at least `us` microseconds on silicon:
/// one per cycle of the core at its fastest, the reset clock's HSI at
/// [`hsi_fastest`] (`AET-WO-4230` §1.9, amendment 1), rounded up.
fn settle_spins(us: u32) -> u64 {
    (u128::from(us) * u128::from(hsi_fastest(RESET_HZ)))
        .div_ceil(1_000_000)
        .try_into()
        .unwrap_or(u64::MAX)
}

// belt-and-suspenders: the judge holds its own copy of the record layout
// (`aethelgard_twin::boot::value_words`), because its expectations are never
// taken from the generator (`INV-07`); `tests/spi_devices.rs` proves they agree.
impl Read {
    /// The words its record's value takes (`aethelgard_twin::boot::value_words`).
    fn words(&self) -> u32 {
        u32::from(self.width).div_ceil(4).max(1)
    }
}

/// The steps, as the judge orders its records
/// (`aethelgard_twin::boot::identity`): every identity and configuration
/// step, then every data read (`AET-WO-4230` R6, the architect's ruling on
/// Q-E: the devices power up together, so one wait between the two halves
/// serves them all), each half with the I2C devices in the specification's
/// order, then the SPI devices in theirs, each device's script in order
/// (`aethelgard_ir::parts::Part::steps`); for each device that `answers`
/// accepts (the twin has a model for it; this crate cannot
/// depend on the twin) and whose part is in the table. A device on a
/// peripheral the board does not have as a bus of its kind is refused.
fn reads(
    plan: &Plan,
    solved: &SolvedTopology,
    answers: &dyn Fn(&str) -> bool,
) -> Result<Vec<Read>, BoardError> {
    let steps = |part: &str| aethelgard_ir::parts::steps(part).map_err(BoardError::InvalidTopology);
    let mut out = Vec::new();
    for (reference, d) in solved.i2c_devices_in_order() {
        if !answers(&d.part) {
            continue;
        }
        if plan.peripherals.get(&d.peripheral).map(|p| p.kind) != Some(Kind::I2c) {
            return Err(BoardError::DeviceNotOnABus {
                reference: reference.to_owned(),
                peripheral: d.peripheral.clone(),
            });
        }
        let Some(steps) = steps(&d.part)? else {
            continue;
        };
        for step in steps {
            out.push(Read {
                bus: d.peripheral.to_ascii_lowercase(),
                target: Target::I2c { address: d.address },
                data: step.kind == StepKind::Data,
                write: step.write,
                width: step.width,
                settle_us: step.settle_us,
            });
        }
    }
    for (reference, d) in solved.spi_devices_in_order() {
        if !answers(&d.part) {
            continue;
        }
        let Some(steps) = steps(&d.part)? else {
            continue;
        };
        // `Plan::new` placed every SPI device on its bus, or refused it.
        let sel = plan
            .selects
            .get(&d.peripheral)
            .and_then(|s| s.iter().find(|s| s.reference == reference))
            .ok_or_else(|| BoardError::DeviceNotOnABus {
                reference: reference.to_owned(),
                peripheral: d.peripheral.clone(),
            })?;
        for step in steps {
            out.push(Read {
                bus: d.peripheral.to_ascii_lowercase(),
                target: Target::Spi {
                    device: format!("{}Device", type_name(&d.peripheral)),
                    variant: sel.variant.clone(),
                },
                data: step.kind == StepKind::Data,
                write: step.write,
                width: step.width,
                settle_us: step.settle_us,
            });
        }
    }
    // Every identity and configuration step, then every data read, each half
    // in the order above.
    let (mut first, data): (Vec<Read>, Vec<Read>) = out.into_iter().partition(|r| !r.data);
    first.extend(data);
    Ok(first)
}

fn main_rs(plan: &Plan, speeds: &Speeds, reads: &[Read]) -> String {
    let krate = ident(&plan.crate_name);
    let mut s = String::from(
        "//! The reference firmware for one solved topology: the runtime, then `main`,\n//! which puts every solved pin in its role, starts every solved peripheral\n//! through the board crate's API and writes the boot word.\n//!\n",
    );
    s.push_str(
        provenance(plan)
            .lines()
            .map(|l| {
                if l.is_empty() {
                    "//!\n".to_owned()
                } else {
                    format!("//! {l}\n")
                }
            })
            .collect::<String>()
            .as_str(),
    );
    let mut imports = vec!["Board".to_owned(), "PwrFault".to_owned()];
    for (name, p) in &plan.peripherals {
        if p.kind == Kind::I2c {
            imports.push(format!("{}_speed", name.to_ascii_lowercase()));
        }
    }
    let i2c_reads = reads.iter().any(|r| matches!(r.target, Target::I2c { .. }));
    let spi_reads = reads.iter().any(|r| matches!(r.target, Target::Spi { .. }));
    if i2c_reads {
        imports.push("I2cError".to_owned());
    }
    for name in plan.selects.keys() {
        let t = type_name(name);
        let f = name.to_ascii_lowercase();
        if reads.iter().any(|r| r.bus == f) {
            imports.push(format!("{t}Device"));
        }
        imports.push(format!("{t}Selects"));
    }
    if spi_reads {
        imports.push("SpiError".to_owned());
    }
    imports.push("mmio::Mmio".to_owned());
    let (first, stride) = (RECORD_OFFSET / 4, RECORD_BYTES / 4);
    // Every record two words, as every identity read is: the layout and the
    // text a board without a script has always had. Otherwise each record is
    // the status and as many value words as its read needs (`AET-WO-4210`).
    let uniform = reads.iter().all(|r| r.words() == 1);
    let mut at = Vec::with_capacity(reads.len());
    let mut n_words = first as usize;
    for r in reads {
        at.push(n_words);
        n_words += 1 + r.words() as usize;
    }
    let boot_static = if reads.is_empty() {
        format!(
            "// The boot word, then the brown-out word (`aethelgard_twin::boot`). One\n\
             // array, so the first word is the boot word whatever the linker orders.\n\
             static mut AETHELGARD_BOOT: [u32; {n_words}] = [0; {n_words}];"
        )
    } else {
        format!(
            "// The boot word, the brown-out word, then one record per identity read: the\n\
             // status, then the value (`aethelgard_twin::boot`, `RECORD_OFFSET`). One array,\n\
             // so the first word is the boot word whatever the linker orders.\n\
             static mut AETHELGARD_BOOT: [u32; {n_words}] = [0; {n_words}];"
        )
    };
    let mut statuses = format!("const STATUS_OK: u32 = {STATUS_OK:#010x};\n");
    if i2c_reads {
        for (name, v) in [
            ("TXIS", STATUS_TXIS),
            ("TC", STATUS_TC),
            ("RXNE", STATUS_RXNE),
            ("STOP", STATUS_STOP),
        ] {
            let _ = writeln!(statuses, "const STATUS_{name}: u32 = {v:#010x};");
        }
    }
    if spi_reads {
        let _ = writeln!(statuses, "const STATUS_RXP: u32 = {STATUS_RXP:#010x};");
        let _ = writeln!(statuses, "const STATUS_EOT: u32 = {STATUS_EOT:#010x};");
    }
    let records = if reads.is_empty() {
        String::new()
    } else if !uniform {
        format!(
            "/// What a record holds when the read finished: the judge's constants,\n\
             /// `aethelgard_twin::boot::STATUS_*`.\n\
             {statuses}\
             /// What a transfer that could not run leaves, the judge's own fill.\n\
             const RECORD_UNSET: u32 = {unset:#010x};\n\
             \n\
             /// The record at word `at` after the boot word: the status word, then the\n\
             /// value in `words` words, its most significant first.\n\
             fn record(at: usize, words: usize, status: u32, value: u64) {{\n    \
             let base = (&raw mut AETHELGARD_BOOT).cast::<u32>();\n    \
             // SAFETY: words of the array above, which `link.x` placed in RAM; the\n    \
             // only code that writes them.\n    \
             unsafe {{\n        \
             base.add(at).write_volatile(status);\n        \
             for i in 0..words {{\n            \
             let word = (value >> (32 * (words - 1 - i))) & 0xFFFF_FFFF;\n            \
             base.add(at + 1 + i).write_volatile(u32::try_from(word).unwrap_or(RECORD_UNSET));\n        \
             }}\n    \
             }}\n\
             }}\n\
             \n",
            unset = RECORD_UNSET,
        )
    } else {
        format!(
            "/// What a record holds when the read finished: the judge's constants,\n\
             /// `aethelgard_twin::boot::STATUS_*`.\n\
             {statuses}\
             /// What a transfer that could not run leaves, the judge's own fill.\n\
             const RECORD_UNSET: u32 = {unset:#010x};\n\
             \n\
             /// Record `index`: the status word, then the value word, after the boot word.\n\
             fn record(index: usize, status: u32, value: u32) {{\n    \
             let base = (&raw mut AETHELGARD_BOOT).cast::<u32>();\n    \
             // SAFETY: two words of the array above, which `link.x` placed in RAM; the\n    \
             // only code that writes them.\n    \
             unsafe {{\n        \
             base.add({first} + {stride} * index).write_volatile(status);\n        \
             base.add({v} + {stride} * index).write_volatile(value);\n    \
             }}\n\
             }}\n\
             \n",
            unset = RECORD_UNSET,
            v = first + 1,
        )
    };
    // The wait a scripted part's data read needs (`AET-WO-4220`), only where
    // a script states one, so every other board's firmware is unchanged.
    let records = if reads.iter().any(|r| r.settle_us > 0) {
        format!(
            "{records}/// Waits at least `spins` core cycles (AET-WO-4220): each iteration is a volatile\n\
             /// load the compiler keeps, at least one cycle. The counts are sized at HSI's\n\
             /// fastest, {fast} Hz (DS12117 Rev 10 Table 36; AET-WO-4230 amendment 1).\n\
             fn settle(spins: u32) {{\n    \
             let boot = (&raw const AETHELGARD_BOOT).cast::<u32>();\n    \
             for _ in 0..spins {{\n        \
             // SAFETY: the first word of the array above; reading it changes nothing.\n        \
             let _ = unsafe {{ boot.read_volatile() }};\n    \
             }}\n\
             }}\n\
             \n",
            fast = hsi_fastest(RESET_HZ),
        )
    } else {
        records
    };
    let serial_consts = if plan.serials.is_empty() {
        String::new()
    } else {
        let tail: Vec<String> = PATTERN_TAIL.iter().map(|b| format!("{b:#04x}")).collect();
        format!(
            "/// What each serial port sends after its label's bytes (`AET-WO-4300b`).\n\
             const PATTERN_TAIL: [u8; {n}] = [{tail}];\n\
             /// The most bytes each serial port echoes: the length of the judge's frame.\n\
             const FRAME_LEN: u32 = {FRAME_LEN};\n\
             /// The most reads of `ISR` the echo makes waiting for each byte.\n\
             const ECHO_POLLS: u32 = {ECHO_POLLS};\n\
             \n",
            n = PATTERN_TAIL.len(),
            tail = tail.join(", "),
        )
    };
    let boot_ptr = "(&raw mut AETHELGARD_BOOT).cast::<u32>()";
    let _ = write!(
        s,
        "#![no_std]\n\
         #![no_main]\n\
         \n\
         use core::arch::global_asm;\n\
         use core::panic::PanicInfo;\n\
         \n\
         use {krate}::{{{imports}}};\n\
         \n\
         /// What the fixed point writes: `aethelgard_twin::boot::BOOT_MAGIC`, the\n\
         /// judge's constant. Neither RAM's reset value nor any register's.\n\
         const BOOT_MAGIC: u32 = {magic:#010x};\n\
         /// What a fault of the supply and voltage-scale bring-up writes instead: the\n\
         /// judge's `aethelgard_twin::boot::BOOT_SUPPLY_NOT_DECLARED`,\n\
         /// `BOOT_ACTVOSRDY_TIMEOUT` and `BOOT_VOSRDY_TIMEOUT` (`AET-WO-4240v`).\n\
         const BOOT_SUPPLY_NOT_DECLARED: u32 = {supply:#010x};\n\
         const BOOT_ACTVOSRDY_TIMEOUT: u32 = {actvosrdy:#010x};\n\
         const BOOT_VOSRDY_TIMEOUT: u32 = {vosrdy:#010x};\n\
         /// The most reads each of the bring-up's two waits makes.\n\
         const PWR_POLLS: u32 = {polls};\n\
         \n\
         /// The observable word: the first word of RAM (`link.x`, `.aethelgard_boot`).\n\
         #[unsafe(link_section = \".aethelgard_boot\")]\n\
         #[unsafe(no_mangle)]\n\
         {boot_static}\n\
         \n\
         {records}\
         {serial_consts}\
         unsafe extern \"C\" {{\n    \
         fn Reset() -> !;\n\
         }}\n\
         \n\
         /// Reset, then the 14 core exceptions; `link.x` puts the initial stack\n\
         /// pointer before them.\n\
         #[unsafe(link_section = \".vector_table.exceptions\")]\n\
         #[used]\n\
         static EXCEPTIONS: [unsafe extern \"C\" fn() -> !; 15] = [\n    \
         Reset, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault,\n    \
         fault, fault,\n\
         ];\n\
         \n\
         // Reset: zero `.bss`, copy `.data` from flash, enable the FPU (CPACR\n\
         // CP10 and CP11 full access, then `dsb` and `isb`: the first float\n\
         // instruction faults without it), call `main`, trap if it returns.\n\
         global_asm!(\n    \
         \".section .text.Reset,\\\"ax\\\",%progbits\",\n    \
         \".global Reset\",\n    \
         \".type Reset,%function\",\n    \
         \".thumb_func\",\n    \
         \"Reset:\",\n    \
         \"    ldr r0, =_sbss\",\n    \
         \"    ldr r1, =_ebss\",\n    \
         \"    movs r2, #0\",\n    \
         \"0:  cmp r0, r1\",\n    \
         \"    bhs 1f\",\n    \
         \"    stm r0!, {{{{r2}}}}\",\n    \
         \"    b 0b\",\n    \
         \"1:  ldr r0, =_sdata\",\n    \
         \"    ldr r1, =_edata\",\n    \
         \"    ldr r2, =_sidata\",\n    \
         \"2:  cmp r0, r1\",\n    \
         \"    bhs 3f\",\n    \
         \"    ldm r2!, {{{{r3}}}}\",\n    \
         \"    stm r0!, {{{{r3}}}}\",\n    \
         \"    b 2b\",\n    \
         \"3:  ldr r0, =0xE000ED88\",\n    \
         \"    ldr r1, [r0]\",\n    \
         \"    orr r1, r1, #0x00F00000\",\n    \
         \"    str r1, [r0]\",\n    \
         \"    dsb\",\n    \
         \"    isb\",\n    \
         \"    bl main\",\n    \
         \"    udf #0\",\n    \
         \"    .pool\",\n    \
         \".size Reset, . - Reset\",\n\
         );\n\
         \n\
         /// Every other exception parks here, distinguishable from the fixed point\n\
         /// because the boot word is never written.\n\
         unsafe extern \"C\" fn fault() -> ! {{\n    \
         loop {{\n        \
         core::hint::spin_loop();\n    \
         }}\n\
         }}\n\
         \n\
         #[unsafe(no_mangle)]\n\
         extern \"C\" fn main() -> ! {{\n    \
         // SAFETY: this is the part the board crate was generated for, and this is\n    \
         // the only `Mmio`.\n    \
         let mut r = unsafe {{ Mmio::new() }};\n    \
         let Some(Board {{\n        \
         mut rcc,\n        \
         flash,\n        \
         pwr,\n",
        imports = imports.join(", "),
        magic = BOOT_MAGIC,
        supply = BOOT_SUPPLY_NOT_DECLARED,
        actvosrdy = BOOT_ACTVOSRDY_TIMEOUT,
        vosrdy = BOOT_VOSRDY_TIMEOUT,
        polls = PWR_POLLS,
        boot_static = boot_static,
        records = records,
    );
    for port in &plan.ports {
        let _ = writeln!(s, "        {},", port_field(*port));
    }
    for name in plan.peripherals.keys() {
        let _ = writeln!(s, "        {},", name.to_ascii_lowercase());
    }
    for n in plan.timers.keys() {
        let _ = writeln!(s, "        tim{n},");
    }
    for name in plan.serials.keys() {
        let _ = writeln!(s, "        {},", name.to_ascii_lowercase());
    }
    s.push_str(
        "    }) = Board::take()\n    \
         else {\n        \
         loop {\n            \
         core::hint::spin_loop();\n        \
         }\n    \
         };\n",
    );
    // The supply and the voltage scale (`AET-WO-4240v` §1.4, D-2): the first
    // hardware access, in every firmware. A fault writes its own boot word and
    // parks.
    let _ = write!(
        s,
        "    // The supply configuration, then VOS Scale 2 (AET-WO-4240v): the first\n    \
         // hardware access. A fault writes its own boot word and parks, nothing else\n    \
         // written.\n    \
         let {bind} = match pwr.scale2(&mut r, PWR_POLLS) {{\n        \
         Ok(scale2) => scale2,\n        \
         Err(fault) => {{\n            \
         let word = match fault {{\n                \
         PwrFault::SupplyNotDeclared => BOOT_SUPPLY_NOT_DECLARED,\n                \
         PwrFault::ActvosrdyTimeout => BOOT_ACTVOSRDY_TIMEOUT,\n                \
         PwrFault::VosrdyTimeout => BOOT_VOSRDY_TIMEOUT,\n            \
         }};\n            \
         // SAFETY: the boot word, in RAM the linker script placed; the fixed point\n            \
         // below, which writes it otherwise, is never reached.\n            \
         unsafe {{ {boot_ptr}.write_volatile(word) }};\n            \
         loop {{\n                \
         core::hint::spin_loop();\n            \
         }}\n        \
         }}\n    \
         }};\n",
        bind = if plan.needs_clock() {
            "scale2"
        } else {
            "_scale2"
        },
    );
    // The reset clock (`AET-WO-4230` D6), read once, first after the voltage
    // scale, before any write: every value the bring-up writes was solved for
    // it, so without it the firmware stops here and never writes the boot word.
    if plan.needs_clock() {
        s.push_str(CLOCK_CHECK);
    }
    // The brown-out word (`AET-WO-4230` §1.6, D10): `FLASH_OPTSR_CUR`, read
    // once and kept as read, before any write to the hardware.
    let _ = writeln!(
        s,
        "    // Word {OPTSR_WORD}: `FLASH_OPTSR_CUR` as read, once, for the judge's `BOR_LEV`.\n    \
         let optsr = flash.option_status(&mut r);\n    \
         // SAFETY: a word of the array above that only this line writes.\n    \
         unsafe {{ {boot_ptr}.add({OPTSR_WORD}).write_volatile(optsr) }};"
    );
    // Each SPI controller's kernel clock, before any SPI clock enable.
    let kernels = if plan.spi.is_empty() {
        ""
    } else {
        s.push_str("    let kernels = rcc.select_spi_kernels(&mut r, &clock);\n");
        ", &kernels"
    };
    for port in &plan.ports {
        let f = port_field(*port);
        let _ = writeln!(
            s,
            "    let {f} = rcc.enable_{f}(&mut r, {f}).split(&mut r);"
        );
    }
    for name in plan.peripherals.keys() {
        let f = name.to_ascii_lowercase();
        let k = if plan.spi.contains_key(name) {
            kernels
        } else {
            ""
        };
        let _ = writeln!(s, "    let {f} = rcc.enable_{f}(&mut r, {f}{k});");
    }
    for n in plan.timers.keys() {
        let _ = writeln!(s, "    let tim{n} = rcc.enable_tim{n}(&mut r, tim{n});");
    }
    for name in plan.serials.keys() {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(s, "    let {f} = rcc.enable_{f}(&mut r, {f});");
    }
    for (pin, p) in &plan.pins {
        let f = pin_field(pin);
        // An output no bus takes as a chip select is configured and unused.
        let (bind, port) = (
            if p.role == Role::Output && !plan.is_select(pin) {
                "_"
            } else {
                ""
            },
            port_field(p.port),
        );
        let _ = writeln!(s, "    let {bind}{f} = {port}.{f}.{}(&mut r);", method(p));
    }
    for (name, p) in &plan.peripherals {
        let f = name.to_ascii_lowercase();
        let args: Vec<String> = p.pins.values().map(|pin| pin_field(pin)).collect();
        let args = args.join(", ");
        match p.kind {
            Kind::I2c => {
                let bind = if reads.iter().any(|r| r.bus == f) {
                    format!("mut {f}")
                } else {
                    format!("_{f}")
                };
                let _ = writeln!(
                    s,
                    "    let {bind} = {f}.init::<{f}_speed::{}, _>(&mut r, &clock, {args});",
                    highest(speeds, name)
                );
            }
            Kind::Spi => {
                let bind = if reads.iter().any(|r| r.bus == f) {
                    format!("mut {f}")
                } else {
                    format!("_{f}")
                };
                let selects = match plan.selects.get(name) {
                    Some(selects) => {
                        let pins: Vec<String> = selects.iter().map(|s| pin_field(&s.pin)).collect();
                        format!(", {}Selects {{ {} }}", type_name(name), pins.join(", "))
                    }
                    None => String::new(),
                };
                let _ = writeln!(
                    s,
                    "    let {bind} = {f}.init(&mut r, &clock, {args}{selects});"
                );
            }
        }
    }
    let serials = plan.serials_in_order();
    // The serial ports (`AET-WO-4300b`), before the identity reads: each port
    // is configured, and sends its pattern, in the specification's order.
    for (name, sp) in &serials {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            s,
            "    let mut {f} = {f}.configure(&mut r, &clock, {}, {});",
            pin_field(&sp.tx),
            pin_field(&sp.rx)
        );
    }
    for (name, sp) in &serials {
        let label: Vec<String> = sp.label.bytes().map(|b| format!("{b:#04x}")).collect();
        let _ = write!(
            s,
            "    // Serial port {index}, `{text}`, on {name}: its label's bytes, then the tail.\n    \
             for &byte in [{label}].iter().chain(PATTERN_TAIL.iter()) {{\n        \
             if {f}.send(&mut r, byte).is_err() {{\n            \
             break;\n        \
             }}\n    \
             }}\n",
            index = sp.index,
            text = sp.label_text(),
            label = label.join(", "),
            f = name.to_ascii_lowercase(),
        );
    }
    let unset = if uniform { "RECORD_UNSET" } else { "u64::MAX" };
    // One wait, the largest of the scripted parts' cited start-ups, after the
    // last configuration write and before the first data read (`AET-WO-4230`
    // R6, the architect's ruling on Q-E: the parts power up together, so no
    // per-device sum; each start-up counts from its part's last wake-up
    // write, all of which precede it).
    let wait = reads.iter().map(|r| r.settle_us).max().unwrap_or(0);
    let wait_at = reads.iter().position(|r| r.data).unwrap_or(reads.len());
    let settle_line = |s: &mut String| {
        if wait > 0 {
            let _ = writeln!(
                s,
                "    // One wait for every scripted part's start-up, the largest cited\n    \
                 // (AET-WO-4220; AET-WO-4230 R6), {wait} us, before any data read.\n    \
                 settle({spins});",
                spins = settle_spins(wait),
            );
        }
    };
    for (k, read) in reads.iter().enumerate() {
        if k == wait_at {
            settle_line(&mut s);
        }
        let (to, errors) = match &read.target {
            Target::I2c { address } => (
                format!("{address:#04x}"),
                format!(
                    "        Err(I2cError::Txis) => (STATUS_TXIS, 0),\n        \
                     Err(I2cError::Tc) => (STATUS_TC, 0),\n        \
                     Err(I2cError::Rxne) => (STATUS_RXNE, 0),\n        \
                     Err(I2cError::Stop) => (STATUS_STOP, 0),\n        \
                     Err(I2cError::Length) => (RECORD_UNSET, {unset}),\n"
                ),
            ),
            Target::Spi { device, variant } => (
                format!("{device}::{variant}"),
                format!(
                    "        Err(SpiError::Rxp) => (STATUS_RXP, 0),\n        \
                     Err(SpiError::Eot) => (STATUS_EOT, 0),\n        \
                     Err(SpiError::Length) => (RECORD_UNSET, {unset}),\n"
                ),
            ),
        };
        let write: Vec<String> = read.write.iter().map(|b| format!("{b:#04x}")).collect();
        let (fold, store) = if uniform {
            (
                format!("buf{k}.iter().fold(0u32, |v, &b| (v << 8) | u32::from(b))"),
                format!("record({k}, status, value);"),
            )
        } else {
            (
                format!("buf{k}.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b))"),
                format!("record({}, {}, status, value);", at[k], read.words()),
            )
        };
        let _ = write!(
            s,
            "    let mut buf{k} = [0u8; {w}];\n    \
             let (status, value) = match {bus}.transfer(&mut r, {to}, &[{write}], &mut buf{k}) {{\n        \
             Ok(_) => (\n            \
             STATUS_OK,\n            \
             {fold},\n        \
             ),\n\
             {errors}    \
             }};\n    \
             {store}\n",
            w = read.width,
            bus = read.bus,
            write = write.join(", "),
        );
    }
    if wait_at == reads.len() {
        settle_line(&mut s);
    }
    // The motor outputs (`AET-WO-4300a` §1.3), after the identity reads: each
    // timer starts, in ascending number, and is never touched again.
    for (n, t) in &plan.timers {
        let pins: Vec<String> = t.channels.values().map(|(pin, _)| pin_field(pin)).collect();
        let _ = writeln!(
            s,
            "    let _tim{n} = tim{n}.start_outputs(&mut r, &clock, {});",
            pins.join(", ")
        );
    }
    let _ = writeln!(
        s,
        "    // SAFETY: a word only this line writes, in RAM the linker script placed.\n    \
         unsafe {{ {boot_ptr}.write_volatile(BOOT_MAGIC) }};"
    );
    // The echo (`AET-WO-4300b`), after the boot word: each port in the
    // specification's order returns up to `FRAME_LEN` bytes, each awaited for
    // at most `ECHO_POLLS` reads of `ISR`. A wait that expires ends that port's
    // echo. Nothing here touches a timer, and nothing after it a USART.
    for (name, _) in &serials {
        let _ = write!(
            s,
            "    for _ in 0..FRAME_LEN {{\n        \
             let Some(byte) = {f}.receive(&mut r, ECHO_POLLS) else {{\n            \
             break;\n        \
             }};\n        \
             if {f}.send(&mut r, byte).is_err() {{\n            \
             break;\n        \
             }}\n    \
             }}\n",
            f = name.to_ascii_lowercase(),
        );
    }
    let _ = write!(
        s,
        "    // No sleep: the judge requires instructions executed to equal MIPS x time.\n    \
         loop {{\n        \
         core::hint::spin_loop();\n    \
         }}\n\
         }}\n\
         \n\
         #[panic_handler]\n\
         fn panic(_: &PanicInfo) -> ! {{\n    \
         loop {{\n        \
         core::hint::spin_loop();\n    \
         }}\n\
         }}\n"
    );
    s
}

/// The reset-clock check, first in the bring-up (`AET-WO-4230` D6).
const CLOCK_CHECK: &str = "    // Every value below was solved for the reset clock: read it before any write\n    \
     // (a selection the bring-up wrote reads 0 in Renode), and without it stop\n    \
     // here, and the boot word is never written.\n    \
     let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {\n        \
     loop {\n            \
     core::hint::spin_loop();\n        \
     }\n    \
     };\n";
