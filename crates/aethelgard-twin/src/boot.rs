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
use aethelgard_svd_hal::cmsis::{self, RegisterError};

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

/// The bound on the boot of a board with a DShot output (`AET-WO-4300a` D1,
/// ruled (a)): four DShot600 channels for 100 ms flood the log (139.6 MB,
/// 17 times its budget) and cost 10.8 s of timer events; 2 ms fits (37,748
/// lines, 2.80 MB, 2.64 s on Furnace; FEEDBACK P5.2). Every other board
/// keeps [`MAX_VIRTUAL_NS`].
pub const DSHOT_WINDOW_NS: u64 = 2_000_000;

/// The bound on a boot of `topology`'s board, and the name of the constant
/// that states it: [`DSHOT_WINDOW_NS`] when any motor output is DShot, else
/// [`MAX_VIRTUAL_NS`].
pub fn window(topology: &SolvedTopology) -> (u64, &'static str) {
    if topology
        .motor_outputs
        .values()
        .any(|m| m.protocol.bit_rate().is_some())
    {
        (DSHOT_WINDOW_NS, "DSHOT_WINDOW_NS")
    } else {
        (MAX_VIRTUAL_NS, "MAX_VIRTUAL_NS")
    }
}

/// The core's speed in the emitted platform: Renode 1.17.0's default, which
/// the platform never changes (spike S4: `cpu PerformanceInMips` reads 100,
/// and 10 ms executes exactly 1,000,000 instructions).
pub const MIPS: u64 = 100;

/// The quantum every boot runs at, in nanoseconds (`AET-WO-4300` D2): Renode
/// 1.17.0's own default (`TimeSourceBase.DefaultQuantum`, 100 µs), stated by
/// the CLI's composition. A line the CPU causes carries the start of its
/// quantum, unless the core has just accessed a timer, which brings the
/// clock to that instant (`AET-WO-4300a` FEEDBACK M4): an edge off this grid
/// is D4's third part of the witness, never proof alone.
pub const QUANTUM_NS: u64 = 100_000;

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
/// An SPI read whose bounded wait for `SR.RXP` expired (`AET-WO-4200`).
pub const STATUS_RXP: u32 = 0x2202_0005;

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
/// Nor does it claim that no interrupt runs after the boot word (adversarial
/// A2: a SysTick handler that returns leaves the core on the loop).
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

/// Where a device sits on its bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wire {
    /// An I2C device at its 7-bit address.
    I2c { address: u8 },
    /// An SPI device at its multiplexer address, selected by the pin `select`
    /// (`PA0`), driven low (`AET-WO-4200` D1).
    Spi { address: u8, select: String },
}

impl Wire {
    /// How a finding names where the device is: `at I2C address 0x76`, or
    /// `on SPI1, chip select PA0`.
    pub fn describe(&self, peripheral: &str) -> String {
        match self {
            Wire::I2c { address } => format!("at I2C address {address:#04x}"),
            Wire::Spi { select, .. } => format!("on {peripheral}, chip select {select}"),
        }
    }
}

/// One identity read the judge expects, from the parts table alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCheck {
    pub reference: String,
    /// ST's name of the controller the device is on, such as `I2C1`.
    pub peripheral: String,
    pub wire: Wire,
    /// The byte written first: an I2C register, or an SPI command.
    pub write: u8,
    pub width: u8,
    pub expected: u32,
    /// The record's two words in RAM.
    pub status_at: u32,
    pub value_at: u32,
    /// The model's path in Renode: `i2c1.u2` (the bus's stock name, then the
    /// device's emitted name), or `spi1.spi1mux.u4` (with the multiplexer).
    pub device: String,
    /// What Renode itself records when the controller runs this read on the
    /// model (`AET-WO-2204` D1): the model's whole log lines, after the host
    /// timestamp, in order. Only a transfer the controller runs to the
    /// device's address calls the model, so no store writes these lines. All
    /// of it comes from the part table and the emitter, never from the
    /// firmware (`INV-07`). `None` when the model has no known witness format:
    /// a named failure, never a skip (D2).
    pub witness: Option<Vec<String>>,
    /// The model's `Noisy` lines that are not a transfer, which the judge
    /// skips; any other line from the device fails (`AET-WO-4200` D2).
    pub skipped: Vec<String>,
}

/// A device the twin has no model for: reported, never passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmodelled {
    pub reference: String,
    pub part: String,
    pub peripheral: String,
    pub wire: Wire,
}

/// What a boot must leave for the devices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentityPlan {
    /// One per device whose part has an identity and a model: the I2C
    /// devices in the order they are declared, then the SPI devices in theirs;
    /// the record index is the position.
    pub checks: Vec<IdentityCheck>,
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

impl IdentityCheck {
    /// The status word and value word, judged. Pass needs both: a status of
    /// OK and exactly the table's value.
    pub fn judge(&self, status: u32, value: u32) -> Outcome {
        // A status counts only on its own bus: an I2C read never writes
        // `RXP`, and an SPI read none of the I2C waits.
        let spi = matches!(self.wire, Wire::Spi { .. });
        match (status, spi) {
            (STATUS_OK, _) if value == self.expected => Outcome::Pass,
            (STATUS_OK, _) => Outcome::Mismatch,
            (STATUS_TXIS, false) => Outcome::Timeout("TXIS"),
            (STATUS_TC, false) => Outcome::Timeout("TC"),
            (STATUS_RXNE, false) => Outcome::Timeout("RXNE"),
            (STATUS_STOP, false) => Outcome::Timeout("STOPF"),
            (STATUS_RXP, true) => Outcome::Timeout("RXP"),
            _ => Outcome::Invalid,
        }
    }
}

/// The identity reads a boot of the topology's devices must make: the I2C
/// devices in the specification's order, then the SPI devices in theirs; and
/// the devices it cannot check. The identity is the parts table's; the device
/// is modelled when [`model::kind`] says so.
pub fn identity(topology: &SolvedTopology) -> Result<IdentityPlan, TwinError> {
    let base = boot_word()?;
    let mut plan = IdentityPlan::default();
    let push = |plan: &mut IdentityPlan,
                reference: &str,
                peripheral: &str,
                part: &str,
                wire: Wire,
                device: &dyn Fn() -> Result<String, TwinError>|
     -> Result<(), TwinError> {
        let Some(model) = model::kind(part)? else {
            plan.unmodelled.push(Unmodelled {
                reference: reference.to_owned(),
                part: part.to_owned(),
                peripheral: peripheral.to_owned(),
                wire,
            });
            return Ok(());
        };
        let Some(id) = parts::identity(part).map_err(TwinError::InvalidTopology)? else {
            return Ok(());
        };
        let at = u32::try_from(plan.checks.len())
            .ok()
            .and_then(|n| n.checked_mul(RECORD_BYTES))
            .and_then(|n| n.checked_add(RECORD_OFFSET))
            .and_then(|n| base.checked_add(n))
            .ok_or_else(|| TwinError::InvalidDeviceName {
                reference: reference.to_owned(),
            })?;
        let device = device()?;
        plan.checks.push(IdentityCheck {
            reference: reference.to_owned(),
            peripheral: peripheral.to_owned(),
            wire,
            write: id.write,
            width: id.width,
            expected: id.value,
            status_at: at,
            value_at: at + 4,
            witness: model.witness(&device, id.write, id.width, id.value),
            skipped: model.skipped(&device),
            device,
        });
        Ok(())
    };
    for (reference, d) in topology.i2c_devices_in_order() {
        push(
            &mut plan,
            reference,
            &d.peripheral,
            &d.part,
            Wire::I2c { address: d.address },
            &|| platform::device_path(&d.peripheral, reference),
        )?;
    }
    for s in platform::spi_wiring(topology)? {
        push(
            &mut plan,
            &s.reference,
            &s.peripheral,
            &s.part,
            Wire::Spi {
                address: s.address,
                select: s.select.clone(),
            },
            &|| platform::spi_device_path(&s.peripheral, &s.reference),
        )?;
    }
    Ok(plan)
}

/// What a boot of `topology` must leave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Every RCC enable register the header defines (`AET-WO-4300a` §1.6,
    /// 2100 F24), each with silicon's reset value as `value`: the boot writes
    /// it before it starts, because Renode resets them with every clock on
    /// (spike F10).
    pub cold: Vec<Register>,
    /// Every register to compare after the run, in address order: the mode
    /// and alternate function of each solved pin, and each cold register in
    /// full, which must hold exactly the solved enable bits.
    pub registers: Vec<Register>,
    /// Solved fields Renode 1.17.0 does not store, so no read can judge them
    /// (spike F11): `OTYPER` of a port with an I2C pin, an I2C's `TIMINGR`;
    /// a timer's `BDTR.MOE` and each channel's `OCxPE` (`AET-WO-4300a`).
    pub unobservable: Vec<String>,
    /// The identity reads of the devices (`AET-WO-2202`, `AET-WO-4200`).
    pub identity: IdentityPlan,
}

/// The enable registers whose reset value is not 0 (RM0433 Rev 9 Tables
/// 66–74): `RCC_APB4ENR` resets with `RTCAPBEN` (bit 16) set. Every other
/// `*ENR` of the header resets to 0.
pub const ENR_RESET: [(&str, u32); 1] = [("APB4ENR", 0x0001_0000)];

/// Each solved timer's registers after the boot, from the topology alone
/// (`AET-WO-4300a` §1.4, `INV-07`): the fields Renode 1.17.0's
/// `STM32_Timer` stores, each with the value the bring-up writes. `CR1`:
/// `CEN | ARPE` under the stored bits 0–7 (L143–165; `CKD` is a tag). `DIER`:
/// 0 under its stored interrupt enables (bits 0–4 and 6; the DMA enables
/// are tags), so a firmware that arms an update interrupt fails here.
/// `CCMR1` and `CCMR2`: each used channel `OCxM` = 110 (PWM mode 1) and
/// `CCxS` = 00 (output), under the stored `CCxS` and `OCxM` bits (L425–445;
/// `OCxFE`, `OCxPE` and `OCxCE` are tags). `CCER`: each used channel's
/// `CCxE`, polarity 0, under the stored `CCxE`, `CCxP` and `CCxNP` bits
/// (`CCxNE` is a tag). `PSC`, `ARR` and each used channel's `CCRx` in full.
/// `CR2.TI1S`, `SMCR`'s `SMS` and `TS`, and `RCR` (where the timer has one)
/// at their reset value 0, which the bring-up never changes. `BDTR` is all
/// tags, so `MOE` and `BKE` are listed as unobservable, with `SMCR.ECE`. An unused
/// channel's `CCRx` is not written and not read: Renode resets it to
/// `0xFFFF` where silicon resets it to 0 (measured, FEEDBACK P8.1).
fn timer_registers(
    topology: &SolvedTopology,
    f: &cmsis::RegisterFacts,
    field: &mut impl FnMut(u32, String, u32, u32),
) -> Result<(), TwinError> {
    let tim = f.layout("TIM")?;
    // Each timer's shared values, and each used channel with its CCR.
    let mut timers = BTreeMap::new();
    for (net, m, a) in topology.motor_outputs_in_order() {
        let (timer, channel) = aethelgard_ir::motor::timer_channel(&a.signal).ok_or_else(|| {
            TwinError::UnsupportedPeripheral {
                peripheral: format!("{} ({net})", a.signal),
            }
        })?;
        timers
            .entry(timer)
            .or_insert((m, Vec::new()))
            .1
            .push((channel, m.ccr));
    }
    for (timer, (m, channels)) in timers {
        let name = format!("TIM{timer}");
        let base = f.base(&name)?;
        let mut reg = |r: &str, mask: u32, value: u32| -> Result<(), TwinError> {
            field(base + tim.offset(r, 0)?, format!("{name}.{r}"), mask, value);
            Ok(())
        };
        reg("CR1", 0xFF, CR1_ARPE | CR1_CEN)?;
        // `CR2.TI1S`, and `SMCR`'s slave mode and trigger (L175, L186–207),
        // at reset: a gated or trigger slave mode keeps the model counting
        // while it stops silicon's counter, or stops the model's with `CEN`
        // still set (adversarial H1, H2).
        reg("CR2", 0x80, 0)?;
        reg("SMCR", 0x77, 0)?;
        reg("DIER", 0x5F, 0)?;
        let (mut ccmr, mut ccer) = ([0u32; 2], 0u32);
        for &(c, _) in &channels {
            let half = (c - 1) % 2;
            ccmr[((c - 1) / 2) as usize] |= OCM_PWM1 << (4 + 8 * half);
            ccer |= 1 << (4 * (c - 1));
        }
        reg("CCMR1", 0x7373, ccmr[0])?;
        reg("CCMR2", 0x7373, ccmr[1])?;
        reg("CCER", 0xBBBB, ccer)?;
        reg("PSC", u32::MAX, m.psc)?;
        reg("ARR", u32::MAX, m.arr)?;
        // The repetition counter, at reset, on the timers that have one (the
        // same set as `MOE`'s, RM0433 Rev 9 §38.4.15, §41.5.13, §41.6.12).
        if aethelgard_ir::motor::has_main_output_enable(timer) {
            reg("RCR", 0xFF, 0)?;
        }
        for &(c, ccr) in &channels {
            reg(&format!("CCR{c}"), u32::MAX, ccr)?;
        }
    }
    Ok(())
}

/// `TIMx_CR1.CEN` and `ARPE` (RM0433 Rev 9 §38.4.1).
const CR1_CEN: u32 = 1;
const CR1_ARPE: u32 = 1 << 7;
/// `OCxM` = 110, PWM mode 1 (RM0433 Rev 9 §38.4.8, the output-compare view).
const OCM_PWM1: u32 = 0b110;

// belt-and-suspenders: the generator's `board.rs` states these too; the
// judge computes the expected image without the generator's code (`INV-07`).
/// MODER values (RM0433 §11.4.1), the two roles the topology solves.
const MODE_OUTPUT: u32 = 0b01;
const MODE_AF: u32 = 0b10;

/// The expected register image of `topology`.
pub fn expected(topology: &SolvedTopology) -> Result<Image, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let identity = identity(topology)?;
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
        if let Some((timer, channel)) = aethelgard_ir::motor::timer_channel(&a.signal) {
            let ccmr = if channel <= 2 { 1 } else { 2 };
            unobservable.push(format!("{}.CCMR{ccmr}.OC{channel}PE", a.peripheral));
            // External clock mode 2 (`ECE`, L211) would leave silicon's
            // counter waiting on `ETR`; a motor pin's output type is a tag
            // too (adversarial H2).
            unobservable.push(format!("{}.SMCR.ECE", a.peripheral));
            unobservable.push(format!("GPIO{port}.OTYPER"));
            if aethelgard_ir::motor::has_main_output_enable(timer) {
                unobservable.push(format!("{}.BDTR.MOE", a.peripheral));
                unobservable.push(format!("{}.BDTR.BKE", a.peripheral));
            }
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
    timer_registers(topology, &f, &mut field)?;
    // Every enable register (`*ENR`, not `*LPENR`) of the header's
    // `RCC_TypeDef`, cold at its reset value and judged in full: the reset
    // value and exactly the solved enable bits (2100 F24).
    let mut cold: BTreeMap<(u32, String), (u32, u32)> = BTreeMap::new();
    for reg in rcc
        .fields
        .iter()
        .filter(|r| r.name.ends_with("ENR") && !r.name.ends_with("LPENR"))
    {
        let reset = ENR_RESET
            .iter()
            .find(|(name, _)| *name == reg.name)
            .map_or(0, |(_, v)| *v);
        cold.insert(
            (rcc_base + reg.offset, format!("RCC.{}", reg.name)),
            (reset, 0),
        );
    }
    for peripheral in &gates {
        let (register, bit) = f.enable_bit(peripheral)?;
        let address = rcc_base + rcc.offset(&register, 0)?;
        cold.get_mut(&(address, format!("RCC.{register}")))
            .ok_or_else(|| RegisterError::Undefined {
                name: format!("RCC_TypeDef.{register}"),
            })?
            .1 |= 1 << bit;
    }
    let cold: Vec<Register> = cold
        .into_iter()
        .map(|((address, name), (reset, bits))| {
            field(address, name.clone(), u32::MAX, reset | bits);
            Register {
                name,
                address,
                mask: u32::MAX,
                value: reset,
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
        identity,
    })
}
