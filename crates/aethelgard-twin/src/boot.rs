//! The judge's half of a boot (`AET-WO-2100`, ruled on PR #56, R2 to R5).
//!
//! A firmware boots when three witnesses agree:
//! 1. the boot word, the first word of RAM, holds [`BOOT_MAGIC`];
//! 2. the registers the topology solves read as [`expected`] computes them;
//! 3. the core executed exactly MIPS × virtual time instructions.
//!
//! `AET-WO-2202` adds a fourth: each modelled I2C device's identity read, one
//! record per device after the boot word, judged against `aethelgard-ir`'s
//! part table ([`I2cPlan`]). `AET-WO-2204` witnesses each read by what Renode
//! records, not by what the firmware writes: the device model's own log lines
//! ([`I2cCheck::witness`]); and requires the core parked in thread mode on a
//! self-loop ([`VECTACTIVE_MASK`], [`self_loops`]).
//!
//! Every expected value here comes from the [`SolvedTopology`] and ST's
//! header, never from the firmware or from the generator that wrote it
//! (`INV-07`). The boot word's address and magic are this crate's contract,
//! which the generated linker script and runtime follow.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::parts;
use aethelgard_svd_hal::cmsis;

use crate::model::Model;
use crate::{TwinError, kind, model, platform, split_pin};

// belt-and-suspenders: the generator holds its own copy of this constant
// (`aethelgard-svd-hal` `board/firmware.rs`), because the judge's expectations
// are never taken from the generator (`INV-07`); the boot test proves the two agree.
/// What a firmware writes to the boot word at its fixed point. Neither RAM's
/// reset value (0) nor a value the cold start writes.
pub const BOOT_MAGIC: u32 = 0x2100_B007;

/// The seed every boot gives `emulation SetSeed`, so the platform's random
/// initial values are the same in every run (spike F15).
pub const SEED: u32 = 2100;

/// The ruled bound on a boot's virtual time: 100 ms, 10⁷ instructions at
/// Renode's 100 MIPS (R5).
pub const MAX_VIRTUAL_NS: u64 = 100_000_000;

/// The core's speed in the emitted platform: Renode 1.17.0's default, which
/// the platform never changes (spike S4: `cpu PerformanceInMips` reads 100,
/// and 10 ms executes exactly 1,000,000 instructions).
pub const MIPS: u64 = 100;

/// A half-open address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub start: u64,
    pub end: u64,
}

impl Region {
    /// True when `[address, address + len)` lies inside the region.
    pub fn holds(&self, address: u64, len: u64) -> bool {
        address >= self.start && address.checked_add(len).is_some_and(|e| e <= self.end)
    }
}

/// Where a firmware may load: the part's flash and its DTCM RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    /// `FLASH_BANK1_BASE` to `FLASH_END`, both banks.
    pub flash: Region,
    /// `D1_DTCMRAM_BASE`, of the size its comment states.
    pub ram: Region,
}

// belt-and-suspenders: the generator reads the same macros for its linker
// script (`board/firmware.rs`); the judge reads them itself (`INV-07`).
/// The part's memory, from the header.
pub fn memory() -> Result<Memory, TwinError> {
    let f = cmsis::register_facts()?;
    let flash_start = u64::from(f.value("FLASH_BANK1_BASE")?);
    let flash_end = u64::from(f.value("FLASH_END")?) + 1;
    let ram_start = u64::from(f.value("D1_DTCMRAM_BASE")?);
    let ram_end = ram_start + u64::from(f.region_kib("D1_DTCMRAM_BASE")?) * 1024;
    Ok(Memory {
        flash: Region {
            start: flash_start,
            end: flash_end,
        },
        ram: Region {
            start: ram_start,
            end: ram_end,
        },
    })
}

/// The boot word's address: the first word of RAM.
pub fn boot_word() -> Result<u32, TwinError> {
    let start = memory()?.ram.start;
    Ok(u32::try_from(start).expect("D1_DTCMRAM_BASE is a 32-bit value"))
}

/// One register the judge reads, and what it must hold under `mask`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    /// `GPIOA.MODER`, `RCC.AHB4ENR`.
    pub name: String,
    pub address: u32,
    pub mask: u32,
    pub value: u32,
}

// belt-and-suspenders: the generator holds its own copy of the record layout
// and the status codes (`aethelgard-svd-hal` `board/firmware.rs`), so the
// judge reads the records as it defines them (`INV-07`); `tests/board.rs`
// proves the two agree.
/// Where the first I2C record starts, from the boot word: the word after it.
pub const RECORD_OFFSET: u32 = 4;

/// One record is two words: the status, then the bytes read, first byte on
/// the wire as the most significant.
pub const RECORD_BYTES: u32 = 8;

/// What the judge writes to both words of every record before the core runs,
/// as it does the boot word, so a record is only ever what the firmware wrote.
pub const RECORD_UNSET: u32 = 0xFFFF_FFFF;

/// The statuses a firmware writes, each a word no reset value or stray write
/// is likely to be.
pub const STATUS_OK: u32 = 0x2202_0000;
pub const STATUS_TXIS: u32 = 0x2202_0001;
pub const STATUS_TC: u32 = 0x2202_0002;
pub const STATUS_RXNE: u32 = 0x2202_0003;
pub const STATUS_STOP: u32 = 0x2202_0004;

/// The Cortex-M7's Interrupt Control and State Register, and its `VECTACTIVE`
/// field: the exception the core is handling, 0 in thread mode (ARMv7-M
/// Architecture Reference Manual, B3.2.4). Renode's NVIC model computes it
/// from its own state (`NVIC.cs` L1077), so no firmware store sets it.
pub const ICSR: u32 = 0xE000_ED04;
pub const VECTACTIVE_MASK: u32 = 0x1FF;

/// Thumb `yield`, then `b.n` back to it: the two halfwords
/// `loop { core::hint::spin_loop() }` compiles to (`AET-WO-2204` S3, measured
/// at `0x08000594` in the reference firmware). A pass claims the core parked
/// on such a loop after the boot word, not that the loop is the generator's
/// fixed point: a firmware that writes the boot word and then parks on
/// another thread-mode self-loop also passes (ruled D3, a named residual).
pub const YIELD: u16 = 0xBF10;
pub const B_N_BACK: u16 = 0xE7FD;

/// The address of every `yield; b.n` self-loop in `code`, loaded at `base`:
/// the core is on the loop when its `PC` is one of these or two past it.
pub fn self_loops(base: u64, code: &[u8]) -> Vec<u64> {
    let half = |i: usize| u16::from_le_bytes([code[i], code[i + 1]]);
    (0..code.len().saturating_sub(3))
        .step_by(2)
        .filter(|&i| half(i) == YIELD && half(i + 2) == B_N_BACK)
        .filter_map(|i| base.checked_add(i as u64))
        .collect()
}

/// One identity read the judge expects, from the part table alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I2cCheck {
    pub reference: String,
    /// ST's name of the controller the device is on, such as `I2C1`.
    pub peripheral: String,
    pub address: u8,
    pub register: u8,
    pub width: u8,
    pub expected: u32,
    /// The record's two words in RAM.
    pub status_at: u32,
    pub value_at: u32,
    /// The model's path in Renode, `i2c1.u2`: the bus's stock name, then the
    /// device's emitted name.
    pub device: String,
    /// What Renode itself records when the controller runs this read on the
    /// model (`AET-WO-2204` D1): the model's whole log lines, after the host
    /// timestamp, in order. Only a transfer the controller runs to the
    /// device's address calls the model, so no store writes these lines. All
    /// of it comes from the part table and the emitter, never from the
    /// firmware (`INV-07`). `None` when the model has no known witness format:
    /// a named failure, never a skip (D2).
    pub witness: Option<Vec<String>>,
}

/// The whole log lines `model` writes for a read of `width` bytes from
/// `register` that returns `value`: the exhaustive model-to-format mapping
/// (`AET-WO-2204` D2). Renode 1.17.0's `BME280.cs` logs every write and read at
/// `Noisy` as `Write {bytes}` and `Read {bytes}` (L38, L75), each byte in
/// upper-case hexadecimal without padding (`ToString("X")`), joined by spaces.
pub fn witness_lines(
    model: Model,
    device: &str,
    register: u8,
    width: u8,
    value: u32,
) -> Option<Vec<String>> {
    match model {
        Model::Bme280 => {
            let bytes: Vec<String> = (0..width)
                .rev()
                .map(|i| format!("{:X}", (value >> (8 * u32::from(i))) & 0xFF))
                .collect();
            Some(vec![
                format!("[NOISY] {device}: Write {register:X}"),
                format!("[NOISY] {device}: Read {}", bytes.join(" ")),
            ])
        }
    }
}

/// A device the twin has no model for: reported, never passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmodelled {
    pub reference: String,
    pub part: String,
    pub address: u8,
}

/// What a boot must leave for the I2C devices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct I2cPlan {
    /// One per device whose part has an identity and a model, in the order
    /// the devices are declared; the record index is the position.
    pub checks: Vec<I2cCheck>,
    pub unmodelled: Vec<Unmodelled>,
}

/// How one record reads against its check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    /// The transaction finished and the value is not the part's.
    Mismatch,
    /// The firmware's bounded wait expired; the stage names where.
    Timeout(&'static str),
    /// Neither a status the firmware writes nor the judge's own fill: the
    /// record was never written, or written with something else.
    Invalid,
}

impl I2cCheck {
    /// The status word and value word, judged. Pass needs both: a status of
    /// OK and exactly the table's value.
    pub fn judge(&self, status: u32, value: u32) -> Outcome {
        match status {
            STATUS_OK if value == self.expected => Outcome::Pass,
            STATUS_OK => Outcome::Mismatch,
            STATUS_TXIS => Outcome::Timeout("TXIS"),
            STATUS_TC => Outcome::Timeout("TC"),
            STATUS_RXNE => Outcome::Timeout("RXNE"),
            STATUS_STOP => Outcome::Timeout("STOPF"),
            _ => Outcome::Invalid,
        }
    }
}

/// The I2C reads a boot of the topology's devices must make, in the
/// specification's order, and the devices it cannot check. The identity is the
/// part table's; the device is modelled when [`model::model`] says so.
pub fn i2c(topology: &SolvedTopology) -> Result<I2cPlan, TwinError> {
    let base = boot_word()?;
    let mut plan = I2cPlan::default();
    for (reference, d) in topology.i2c_devices_in_order() {
        let Some(model) = model::kind(&d.part) else {
            plan.unmodelled.push(Unmodelled {
                reference: reference.to_owned(),
                part: d.part.clone(),
                address: d.address,
            });
            continue;
        };
        let Some(id) = parts::identity(&d.part) else {
            continue;
        };
        let at = u32::try_from(plan.checks.len())
            .ok()
            .and_then(|n| n.checked_mul(RECORD_BYTES))
            .and_then(|n| n.checked_add(RECORD_OFFSET))
            .and_then(|n| base.checked_add(n))
            .ok_or_else(|| TwinError::InvalidDeviceName {
                reference: reference.to_owned(),
            })?;
        let device = platform::device_path(&d.peripheral, reference)?;
        plan.checks.push(I2cCheck {
            reference: reference.to_owned(),
            peripheral: d.peripheral.clone(),
            address: d.address,
            register: id.register,
            width: id.width,
            expected: id.value,
            status_at: at,
            value_at: at + 4,
            witness: witness_lines(model, &device, id.register, id.width, id.value),
            device,
        });
    }
    Ok(plan)
}

/// What a boot of `topology` must leave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// The RCC enable registers that gate a solved port or peripheral. The
    /// boot writes 0 to each before it starts, silicon's reset value, because
    /// Renode resets them with every clock on (spike F10).
    pub cold: Vec<Register>,
    /// Every register to compare after the run, in address order: the mode
    /// and alternate function of each solved pin, and each cold register in
    /// full, which must hold exactly the solved enable bits.
    pub registers: Vec<Register>,
    /// Solved fields Renode 1.17.0 does not store, so no read can judge them
    /// (spike F11): `OTYPER` of a port with an I2C pin, an I2C's `TIMINGR`.
    pub unobservable: Vec<String>,
    /// The identity reads of the I2C devices (`AET-WO-2202`).
    pub i2c: I2cPlan,
}

// belt-and-suspenders: the generator's `board.rs` states these too; the
// judge computes the expected image without the generator's code (`INV-07`).
/// MODER values (RM0433 §11.4.1), the two roles the topology solves.
const MODE_OUTPUT: u32 = 0b01;
const MODE_AF: u32 = 0b10;

/// The expected register image of `topology`.
pub fn expected(topology: &SolvedTopology) -> Result<Image, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let i2c = i2c(topology)?;
    let f = cmsis::register_facts()?;
    let gpio = f.layout("GPIO")?;
    let rcc = f.layout("RCC")?;
    let rcc_base = f.base("RCC")?;

    // Pin name to (port, index, Some(af) for a peripheral pin, None for GPIO).
    let mut pins: BTreeMap<&str, (char, u32, Option<u32>)> = BTreeMap::new();
    let mut gates: BTreeSet<String> = BTreeSet::new();
    let mut unobservable = Vec::new();
    for a in topology.assignments.values() {
        let (port, index) = split_pin(&a.pin)?;
        pins.insert(&a.pin, (port, index, Some(u32::from(a.alternate_function))));
        kind(&a.peripheral)?;
        gates.insert(a.peripheral.clone());
        gates.insert(format!("GPIO{port}"));
        if a.peripheral.starts_with("I2C") {
            unobservable.push(format!("GPIO{port}.OTYPER"));
            unobservable.push(format!("{}.TIMINGR", a.peripheral));
        }
    }
    for pin in topology.gpio.values() {
        let (port, index) = split_pin(pin)?;
        pins.insert(pin, (port, index, None));
        gates.insert(format!("GPIO{port}"));
    }
    unobservable.sort();
    unobservable.dedup();

    // (address, name) to (mask, value), so the list comes out in address order.
    let mut regs: BTreeMap<(u32, String), (u32, u32)> = BTreeMap::new();
    let mut field = |address: u32, name: String, mask: u32, value: u32| {
        let e = regs.entry((address, name)).or_insert((0, 0));
        e.0 |= mask;
        e.1 |= value;
    };
    for &(port, index, af) in pins.values() {
        let base = f.base(&format!("GPIO{port}"))?;
        let mode = if af.is_some() { MODE_AF } else { MODE_OUTPUT };
        field(
            base + gpio.offset("MODER", 0)?,
            format!("GPIO{port}.MODER"),
            0b11 << (2 * index),
            mode << (2 * index),
        );
        if let Some(af) = af {
            let word = index / 8;
            let shift = 4 * (index % 8);
            field(
                base + gpio.offset("AFR", word)?,
                format!("GPIO{port}.{}", ["AFRL", "AFRH"][word as usize]),
                0xF << shift,
                af << shift,
            );
        }
    }
    let mut cold: BTreeMap<(u32, String), u32> = BTreeMap::new();
    for peripheral in &gates {
        let (register, bit) = f.enable_bit(peripheral)?;
        let address = rcc_base + rcc.offset(&register, 0)?;
        *cold
            .entry((address, format!("RCC.{register}")))
            .or_default() |= 1 << bit;
    }
    let cold: Vec<Register> = cold
        .into_iter()
        .map(|((address, name), bits)| {
            field(address, name.clone(), u32::MAX, bits);
            Register {
                name,
                address,
                mask: u32::MAX,
                value: 0,
            }
        })
        .collect();
    let registers = regs
        .into_iter()
        .map(|((address, name), (mask, value))| Register {
            name,
            address,
            mask,
            value,
        })
        .collect();
    Ok(Image {
        cold,
        registers,
        unobservable,
        i2c,
    })
}
