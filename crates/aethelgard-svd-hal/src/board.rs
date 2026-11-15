//! The board-crate generator (`AET-WO-1400`, `RUL-04` as amended on PR #42):
//! a [`SolvedTopology`] and the part's [`PinTable`] in, the source of a
//! `#![no_std]`, dependency-free board crate out, as a map from relative path
//! to file text. Everything is a function of its inputs: `BTreeMap` order, no
//! clock, no environment, no host name (`INV-01`).
//!
//! Two emitters share the plan and nothing else. [`emit_lib`] writes the
//! typestate API and its register writes. [`emit_proofs`] writes the Kani
//! harness and its oracle, whose expected register values are computed here
//! from the plan, never by reading back what [`emit_lib`] wrote (`INV-07`).
//!
//! Register facts (base addresses, offsets, bit positions, field widths) come
//! from ST's CMSIS header ([`crate::cmsis`]). Pin roles' electrical settings
//! (output type, pull) are this generator's policy, named in [`Role::policy`];
//! each pin's speed is the output speed check's ([`pin_speeds`],
//! `AET-WO-4300c`). `I2C_TIMINGR` values are placeholders: clock-tree solving
//! is out of scope (WO §4).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use aethelgard_ir::motor;
use aethelgard_ir::speeds::{self, OutputSpeeds};
use aethelgard_ir::{IrValidationError, PinTable, RESERVED_PINS, SolvedTopology};

use crate::cmsis::{self, RegisterError, RegisterFacts};

mod firmware;

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BoardError {
    #[error("`{name}` is not a crate name: lowercase ASCII letters, digits and single `-`")]
    InvalidCrateName { name: String },
    #[error("the solved topology is malformed: {0}")]
    InvalidTopology(IrValidationError),
    #[error("pin `{pin}` is reserved (debug or crystal) and is never emitted")]
    ReservedPin { pin: String },
    #[error("pin `{pin}` is not a GPIO pin name (`P[A-K]` and 0 to 15)")]
    InvalidPin { pin: String },
    #[error("pin `{pin}` is not in the part's pin table")]
    UnknownPin { pin: String },
    #[error(
        "net `{net}`: `{pin}` carries `{signal}` on AF{table:?} in the pin table, not AF{solved}"
    )]
    AssignmentMismatch {
        net: String,
        pin: String,
        signal: String,
        solved: u8,
        table: Option<u8>,
    },
    #[error(
        "peripheral `{peripheral}` is not one this generator emits (I2Cn, SPIn, and TIMn on a motor output)"
    )]
    UnsupportedPeripheral { peripheral: String },
    #[error(
        "device `{reference}` is on `{peripheral}`, which is not a peripheral of this board of its bus's kind"
    )]
    DeviceNotOnABus {
        reference: String,
        peripheral: String,
    },
    #[error("SPI devices `{first}` and `{reference}` share the chip select `{pin}`")]
    SharedChipSelect {
        pin: String,
        first: String,
        reference: String,
    },
    #[error("signal `{signal}` is not a role of `{peripheral}`")]
    UnsupportedSignal { peripheral: String, signal: String },
    #[error("`{peripheral}` has its `{role}` signal {count} times; it needs exactly one")]
    IncompletePeripheral {
        peripheral: String,
        role: &'static str,
        count: usize,
    },
    #[error(
        "the topology has I2C ceilings for buses {buses:?}, not one per I2C peripheral {peripherals:?}"
    )]
    CeilingMismatch {
        peripherals: Vec<String>,
        buses: Vec<String>,
    },
    #[error("a ceiling of {hz} Hz is below every I2C mode")]
    CeilingTooLow { hz: u32 },
    #[error("net `{net}` on `{pin}` has no output speed: {refusal}")]
    PinSpeed {
        net: String,
        pin: String,
        refusal: speeds::Refusal,
    },
    #[error("{0}")]
    SpeedTable(IrValidationError),
    #[error("the output speed table is for `{table}`, but the pin table is for `{part}`")]
    SpeedTableMismatch { table: String, part: String },
    #[error(
        "{first} and {second} may both change {register}: the split schedule proof needs their footprints disjoint (AET-WO-3303)"
    )]
    FootprintOverlap {
        first: String,
        second: String,
        register: String,
    },
    #[error("no part of the split schedule proof schedules {action} (AET-WO-3303)")]
    UnscheduledAction { action: String },
    #[error(transparent)]
    Register(#[from] RegisterError),
}

impl BoardError {
    /// The variant name, or the register error's.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidCrateName { .. } => "InvalidCrateName",
            Self::InvalidTopology(_) => "InvalidTopology",
            Self::ReservedPin { .. } => "ReservedPin",
            Self::InvalidPin { .. } => "InvalidPin",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::AssignmentMismatch { .. } => "AssignmentMismatch",
            Self::UnsupportedPeripheral { .. } => "UnsupportedPeripheral",
            Self::DeviceNotOnABus { .. } => "DeviceNotOnABus",
            Self::SharedChipSelect { .. } => "SharedChipSelect",
            Self::UnsupportedSignal { .. } => "UnsupportedSignal",
            Self::IncompletePeripheral { .. } => "IncompletePeripheral",
            Self::PinSpeed { refusal, .. } => match refusal {
                speeds::Refusal::SupplyOutsideTable { .. } => "SupplyOutsideSpeedTable",
                speeds::Refusal::LoadAboveTable { .. } => "LoadAboveSpeedTable",
                speeds::Refusal::TooFastForPin { .. } => "RateAbovePinSpeed",
                speeds::Refusal::RiseFasterThanPin { .. } => "RiseTimeBelowPinSpeed",
            },
            Self::SpeedTable(e) => e.kind(),
            Self::SpeedTableMismatch { .. } => "SpeedTableMismatch",
            Self::CeilingMismatch { .. } => "CeilingMismatch",
            Self::CeilingTooLow { .. } => "CeilingTooLow",
            Self::FootprintOverlap { .. } => "FootprintOverlap",
            Self::UnscheduledAction { .. } => "UnscheduledAction",
            Self::Register(e) => e.kind(),
        }
    }
}

/// The I2C modes of UM10204, with placeholder `TIMINGR` values.
const I2C_MODES: [(&str, u32, u32); 3] = [
    ("Standard100k", 100_000, 0x1070_7DBC),
    ("Fast400k", 400_000, 0x0060_2173),
    ("FastPlus1M", 1_000_000, 0x0030_0B29),
];

/// The board crate for `crate_name`, from `solved`.
///
/// The I2C speeds each peripheral admits come from `solved.i2c_speeds`, the
/// speed each bus's specification declared (`AET-WO-1205`), keyed by bus id;
/// `i2c_ceilings` (`AET-WO-1202`) is only counted against the peripherals. The
/// topology does not say which peripheral serves which bus, so the generator
/// gives every I2C peripheral the lowest declared speed: exact for one bus, and
/// never above any bus's declared speed for more.
pub fn generate(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, BoardError> {
    generate_with(crate_name, solved, table, &|_| false)
}

/// [`generate`], and the reference firmware also reads the identity of each of
/// the topology's I2C devices (`SolvedTopology::i2c_devices`, `AET-WO-2203`),
/// then of each of its SPI devices (`SolvedTopology::spi_devices`,
/// `AET-WO-4200`), whose part has an identity and for which `answers` says the
/// twin has a model (`AET-WO-2202`). This crate cannot depend on the twin, so
/// the caller passes whether `aethelgard_twin::model::model(part)` is
/// `Ok(Some(_))`.
pub fn generate_with(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
    answers: &dyn Fn(&str) -> bool,
) -> Result<BTreeMap<String, String>, BoardError> {
    let plan = Plan::new(crate_name, solved, table)?;
    let i2c: Vec<String> = plan
        .peripherals
        .iter()
        .filter(|(_, p)| p.kind == Kind::I2c)
        .map(|(n, _)| n.clone())
        .collect();
    if solved.i2c_ceilings.len() != i2c.len() {
        return Err(BoardError::CeilingMismatch {
            peripherals: i2c,
            buses: solved.i2c_ceilings.keys().cloned().collect(),
        });
    }
    let mut speeds = BTreeMap::new();
    if let Some(&hz) = solved.i2c_speeds.values().min() {
        let admitted: Vec<usize> = (0..I2C_MODES.len())
            .filter(|&i| I2C_MODES[i].1 <= hz)
            .collect();
        // belt-and-suspenders: `SolvedTopology::validate` admits only 100, 400
        // and 1000 kHz, so a ceiling below every mode cannot arrive here.
        if admitted.is_empty() {
            return Err(BoardError::CeilingTooLow { hz });
        }
        for name in i2c {
            speeds.insert(name, (hz, admitted.clone()));
        }
    }
    let register_facts = cmsis::register_facts()?;
    let facts = Facts::new(&plan, &register_facts)?;
    let mut files = BTreeMap::new();
    files.insert("Cargo.toml".to_owned(), emit_manifest(&plan));
    files.insert("Cargo.lock".to_owned(), emit_lock(&plan));
    files.insert("src/lib.rs".to_owned(), emit_root(&plan));
    files.insert("src/board.rs".to_owned(), emit_lib(&plan, &facts, &speeds));
    files.insert(
        "src/proofs.rs".to_owned(),
        emit_proofs(&plan, &facts, &speeds)?,
    );
    files.extend(firmware::emit(
        &plan,
        &register_facts,
        &speeds,
        solved,
        answers,
    )?);
    Ok(files)
}

/// The items every I2C peripheral's transfer shares (`AET-WO-2202`, ruled on
/// the I2C read path, F1): the bound on a wait, the typed refusals, and the
/// bounded poll. Additive: nothing above them changes.
const I2C_SHARED: &str = "/// The most reads of `ISR` any one wait of a transfer makes before it gives up.\n\
pub const I2C_WAIT_SPINS: u32 = 20_000;\n\
\n\
/// Why a transfer gave up. A wait that expires names where; a transfer never\n\
/// panics and never waits without a bound.\n\
#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
pub enum I2cError {\n    \
    /// More than 255 bytes in a direction: `NBYTES` is eight bits.\n    \
    Length,\n    \
    /// `TXIS` never rose: the address went unanswered.\n    \
    Txis,\n    \
    /// `TC` never rose after the last byte written, before a repeated start.\n    \
    Tc,\n    \
    /// `RXNE` never rose: a byte of the read did not arrive.\n    \
    Rxne,\n    \
    /// `STOPF` never rose after the last byte.\n    \
    Stop,\n\
}\n\
\n\
/// Reads `ISR` at most `spins` times for any bit of `mask`.\n\
fn i2c_wait<R: Registers>(r: &mut R, block: Block, isr: usize, mask: u32, spins: u32) -> bool {\n    \
    let mut n = 0;\n    \
    while n < spins {\n        \
        if r.read(TOKEN, block, isr) & mask != 0 {\n            \
            return true;\n        \
        }\n        \
        n += 1;\n    \
    }\n    \
    false\n\
}\n\
\n";

/// One I2C peripheral's bounded transfer: write `N` bytes, then read `M`
/// after a repeated start, or only write, or only read.
fn emit_transfer(w: &mut String, name: &str, i: &I2cFacts, txdr: u32) {
    let t = type_name(name);
    let m = name.to_ascii_lowercase();
    let bit = |pos: u32| format!("{:#010x}", 1u32 << pos);
    let _ = writeln!(
        w,
        "impl<S: {m}_speed::AtMostCeiling> {t}<Enabled<S>> {{\n    \
             /// Writes `write`, then, if `read` is not empty, reads it back after a repeated\n    \
             /// start; or only writes; or only reads. Every wait is bounded by\n    \
             /// [`I2C_WAIT_SPINS`]. Returns the bytes read, or the wait that expired.\n    \
             pub fn transfer<R: Registers>(\n        \
                 &mut self,\n        \
                 r: &mut R,\n        \
                 address: u8,\n        \
                 write: &[u8],\n        \
                 read: &mut [u8],\n    \
             ) -> Result<usize, I2cError> {{\n        \
                 self.transfer_within(r, address, write, read, I2C_WAIT_SPINS)\n    \
             }}\n\
             \n    \
             /// [`Self::transfer`] with its own bound on each wait, so the proof can use a\n    \
             /// small one.\n    \
             #[doc(hidden)]\n    \
             pub fn transfer_within<R: Registers>(\n        \
                 &mut self,\n        \
                 r: &mut R,\n        \
                 address: u8,\n        \
                 write: &[u8],\n        \
                 read: &mut [u8],\n        \
                 spins: u32,\n    \
             ) -> Result<usize, I2cError> {{\n        \
                 let (Ok(nw), Ok(nr)) = (u8::try_from(write.len()), u8::try_from(read.len())) else {{\n            \
                     return Err(I2cError::Length);\n        \
                 }};\n        \
                 if nw == 0 && nr == 0 {{\n            \
                     return Ok(0);\n        \
                 }}\n        \
                 let sadd = u32::from(address) << {sadd};\n        \
                 let mut fail = None;\n        \
                 let mut got = 0;\n        \
                 if nw > 0 {{\n            \
                     let end = if nr == 0 {{ {autoend} }} else {{ 0 }};\n            \
                     r.write(TOKEN, Block::{t}, {cr2:#04x}, sadd | (u32::from(nw) << {nbytes}) | {start} | end);\n            \
                     for &byte in write {{\n                \
                         if !i2c_wait(r, Block::{t}, {isr:#04x}, {txis}, spins) {{\n                    \
                             fail = Some(I2cError::Txis);\n                    \
                             break;\n                \
                         }}\n                \
                         r.write(TOKEN, Block::{t}, {txdr:#04x}, u32::from(byte));\n            \
                     }}\n            \
                     if fail.is_none() && nr > 0 && !i2c_wait(r, Block::{t}, {isr:#04x}, {tc}, spins) {{\n                \
                         fail = Some(I2cError::Tc);\n            \
                     }}\n        \
                 }}\n        \
                 if fail.is_none() && nr > 0 {{\n            \
                     r.write(TOKEN, Block::{t}, {cr2:#04x}, sadd | {rd_wrn} | (u32::from(nr) << {nbytes}) | {start} | {autoend});\n            \
                     for slot in read.iter_mut() {{\n                \
                         if !i2c_wait(r, Block::{t}, {isr:#04x}, {rxne}, spins) {{\n                    \
                             fail = Some(I2cError::Rxne);\n                    \
                             break;\n                \
                         }}\n                \
                         *slot = u8::try_from(r.read(TOKEN, Block::{t}, {rxdr:#04x}) & 0xFF).unwrap_or(0);\n                \
                         got += 1;\n            \
                     }}\n        \
                 }}\n        \
                 if fail.is_none() && !i2c_wait(r, Block::{t}, {isr:#04x}, {stopf}, spins) {{\n            \
                     fail = Some(I2cError::Stop);\n        \
                 }}\n        \
                 r.write(TOKEN, Block::{t}, {icr:#04x}, {stopcf} | {nackcf});\n        \
                 match fail {{\n            \
                     None => Ok(got),\n            \
                     Some(e) => Err(e),\n        \
                 }}\n    \
             }}\n\
             }}\n",
        sadd = i.sadd + 1,
        autoend = bit(i.autoend),
        cr2 = i.cr2,
        nbytes = i.nbytes,
        start = bit(i.start),
        isr = i.isr,
        txis = bit(i.txis),
        txdr = txdr,
        tc = bit(i.tc),
        rd_wrn = bit(i.rd_wrn),
        rxne = bit(i.rxne),
        rxdr = i.rxdr,
        stopf = bit(i.stopf),
        icr = i.icr,
        stopcf = bit(i.stopcf),
        nackcf = bit(i.nackcf),
    );
}

/// The items every SPI peripheral's transfer shares (`AET-WO-4200`): the
/// bound on a wait, the bound on a transfer, the typed refusals, and one
/// frame. Emitted when some SPI peripheral has a device.
fn emit_spi_shared(w: &mut String, s: &SpiFacts, txdr: u32) {
    let _ = writeln!(
        w,
        "/// The most reads of `SR` any one wait of an SPI transfer makes before it gives up.\n\
         pub const SPI_WAIT_SPINS: u32 = 20_000;\n\
         \n\
         /// The most frames one SPI transfer moves: an identity read is the command and\n\
         /// at most four bytes, and one transaction per device is all this crate makes.\n\
         pub const SPI_MAX_FRAMES: u32 = 8;\n\
         \n\
         /// Why an SPI transfer gave up. It never panics and never waits without a bound.\n\
         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
         pub enum SpiError {{\n    \
             /// No frame, or more than [`SPI_MAX_FRAMES`]. No register was touched.\n    \
             Length,\n    \
             /// `SR.RXP` never rose: a frame's byte did not arrive.\n    \
             Rxp,\n\
         }}\n\
         \n\
         /// One 8-bit frame: `byte` out through `TXDR`, a wait of at most `spins` reads of\n\
         /// `SR` for `RXP`, then the byte in from `RXDR`. Both data accesses are one byte\n\
         /// wide ([`Registers`]). `None` when `RXP` never rose, and `RXDR` is then not\n\
         /// read: an empty FIFO.\n\
         fn spi_frame<R: Registers>(r: &mut R, block: Block, byte: u8, spins: u32) -> Option<u8> {{\n    \
             r.write8(TOKEN, block, {txdr:#04x}, byte);\n    \
             let mut n = 0;\n    \
             while n < spins {{\n        \
                 if r.read(TOKEN, block, {sr:#04x}) & {rxp:#010x} != 0 {{\n            \
                     return Some(r.read8(TOKEN, block, {rxdr:#04x}));\n        \
                 }}\n        \
                 n += 1;\n    \
             }}\n    \
             None\n\
         }}\n",
        sr = s.sr,
        rxp = 1u32 << s.rxp,
        rxdr = s.rxdr,
    );
}

/// `expr` shifted by `pos` with `op`, or `expr` alone when `pos` is 0.
fn shifted(expr: &str, op: &str, pos: u32) -> String {
    if pos == 0 {
        expr.to_owned()
    } else {
        format!("({expr}) {op} {pos}")
    }
}

/// One SPI peripheral's bounded transfer (`AET-WO-4200`): the command and
/// argument bytes out, then the answer in, with `0x00` clocked out while it
/// arrives. The register order is the one measured against Renode 1.17.0
/// (`TSIZE` while disabled, select, `SPE`, `CSTART`, the frames, `EOTC`, `SPE`
/// off, deselect).
fn emit_spi_transfer(w: &mut String, name: &str, selects: &[Select], plan: &Plan, facts: &Facts) {
    let t = type_name(name);
    let pf = &facts.peripherals[name];
    let Some(s) = &pf.spi else { return };
    let bit = |pos: u32| format!("{:#010x}", 1u32 << pos);
    let arms: Vec<String> = selects
        .iter()
        .map(|sel| {
            let pin = &plan.pins[&sel.pin];
            let pinf = &facts.pins[&sel.pin];
            format!(
                "            {t}Device::{v} => (Block::{port}, {br:#010x}, {bs:#010x}),",
                v = sel.variant,
                port = port_type(pin.port),
                br = 1u32 << pinf.br,
                bs = 1u32 << pinf.bs,
            )
        })
        .collect();
    let _ = writeln!(
        w,
        "impl {t}<Enabled<()>> {{\n    \
             /// Selects `device` (its chip select low) and only it, writes `write`, then\n    \
             /// reads `read` while it clocks out `0x00`; then disables the peripheral and\n    \
             /// deselects the device, also after a wait that expired. Every frame is one\n    \
             /// byte, and every wait is bounded by [`SPI_WAIT_SPINS`]. Returns the bytes\n    \
             /// read, or why it gave up.\n    \
             pub fn transfer<R: Registers>(\n        \
                 &mut self,\n        \
                 r: &mut R,\n        \
                 device: {t}Device,\n        \
                 write: &[u8],\n        \
                 read: &mut [u8],\n    \
             ) -> Result<usize, SpiError> {{\n        \
                 self.transfer_within(r, device, write, read, SPI_WAIT_SPINS)\n    \
             }}\n\
             \n    \
             /// [`Self::transfer`] with its own bound on each wait, so the proof can use a\n    \
             /// small one.\n    \
             #[doc(hidden)]\n    \
             pub fn transfer_within<R: Registers>(\n        \
                 &mut self,\n        \
                 r: &mut R,\n        \
                 device: {t}Device,\n        \
                 write: &[u8],\n        \
                 read: &mut [u8],\n        \
                 spins: u32,\n    \
             ) -> Result<usize, SpiError> {{\n        \
                 let frames = match u32::try_from(write.len() + read.len()) {{\n            \
                     Ok(n) if n > 0 && n <= SPI_MAX_FRAMES => n,\n            \
                     _ => return Err(SpiError::Length),\n        \
                 }};\n        \
                 let (port, select, deselect) = match device {{\n\
         {arms}\n        \
                 }};\n        \
                 r.write(TOKEN, Block::{t}, {cr2:#04x}, {tsize_word});\n        \
                 r.write(TOKEN, port, {bsrr:#04x}, select);\n        \
                 r.modify(TOKEN, Block::{t}, {cr1:#04x}, |v| v | {spe});\n        \
                 r.modify(TOKEN, Block::{t}, {cr1:#04x}, |v| v | {cstart});\n        \
                 let mut fail = false;\n        \
                 for &byte in write {{\n            \
                     if spi_frame(r, Block::{t}, byte, spins).is_none() {{\n                \
                         fail = true;\n                \
                         break;\n            \
                     }}\n        \
                 }}\n        \
                 let mut got = 0;\n        \
                 if !fail {{\n            \
                     for slot in read.iter_mut() {{\n                \
                         match spi_frame(r, Block::{t}, 0x00, spins) {{\n                    \
                             Some(byte) => *slot = byte,\n                    \
                             None => {{\n                        \
                                 fail = true;\n                        \
                                 break;\n                    \
                             }}\n                \
                         }}\n                \
                         got += 1;\n            \
                     }}\n        \
                 }}\n        \
                 r.write(TOKEN, Block::{t}, {ifcr:#04x}, {eotc});\n        \
                 r.modify(TOKEN, Block::{t}, {cr1:#04x}, |v| v & !{spe});\n        \
                 r.write(TOKEN, port, {bsrr:#04x}, deselect);\n        \
                 if fail {{ Err(SpiError::Rxp) }} else {{ Ok(got) }}\n    \
             }}\n\
         }}\n",
        arms = arms.join("\n"),
        cr2 = s.cr2,
        tsize_word = shifted("frames", "<<", s.tsize),
        bsrr = facts.gpio.bsrr,
        cr1 = pf.cr1,
        spe = bit(pf.enable),
        cstart = bit(s.cstart),
        ifcr = s.ifcr,
        eotc = bit(s.eotc),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    I2cScl,
    I2cSda,
    SpiSck,
    SpiMosi,
    SpiMiso,
    Output,
    TimCh1,
    TimCh2,
    TimCh3,
    TimCh4,
}

/// MODER values (RM0433 §11.4.1).
const MODE_OUTPUT: u32 = 0b01;
const MODE_AF: u32 = 0b10;

/// `SPI_CFG1.DSIZE` for 8-bit frames: the frame's bits less one (RM0433
/// §50.11.3). The generator's policy: every SPI part this crate reads takes
/// bytes.
const SPI_DSIZE: u32 = 7;

impl Role {
    fn signal(self) -> &'static str {
        match self {
            Self::I2cScl => "SCL",
            Self::I2cSda => "SDA",
            Self::SpiSck => "SCK",
            Self::SpiMosi => "MOSI",
            Self::SpiMiso => "MISO",
            Self::Output => "OUT",
            Self::TimCh1 => "CH1",
            Self::TimCh2 => "CH2",
            Self::TimCh3 => "CH3",
            Self::TimCh4 => "CH4",
        }
    }

    /// The role of timer channel `k`, 1 to 4.
    fn timer_channel(k: u32) -> Option<Self> {
        [Self::TimCh1, Self::TimCh2, Self::TimCh3, Self::TimCh4]
            .get(usize::try_from(k.checked_sub(1)?).ok()?)
            .copied()
    }

    /// The generator's policy for a role: (mode, output type, pull). I2C is
    /// open-drain with no internal pull (the specification requires an
    /// external pull-up); SPI is push-pull; a GPIO output is push-pull and
    /// starts high (an inactive chip select); a timer channel is push-pull. The
    /// speed is [`Role::scope`]'s.
    fn policy(self) -> (u32, u32, u32) {
        match self {
            Self::I2cScl | Self::I2cSda => (MODE_AF, 1, 0b00),
            Self::SpiSck | Self::SpiMosi | Self::SpiMiso => (MODE_AF, 0, 0b00),
            Self::TimCh1 | Self::TimCh2 | Self::TimCh3 | Self::TimCh4 => (MODE_AF, 0, 0b00),
            Self::Output => (MODE_OUTPUT, 0, 0b00),
        }
    }

    /// Why the output speed check does not judge a role's pin, which then
    /// takes `00`, the slowest speed (`AET-WO-4300c` D1, D3, D4); `None` for
    /// SCK and MOSI, whose edges the MCU drives at the rate its peripheral
    /// sets, and which the check judges.
    fn scope(self) -> Option<&'static str> {
        match self {
            Self::SpiSck | Self::SpiMosi => None,
            Self::I2cScl | Self::I2cSda => Some(OPEN_DRAIN),
            Self::SpiMiso => Some(MASTER_INPUT),
            Self::Output => Some(SOFTWARE_DRIVEN),
            Self::TimCh1 | Self::TimCh2 | Self::TimCh3 | Self::TimCh4 => Some(MOTOR_OUTPUT),
        }
    }
}

/// Out of the output speed check's scope, each with its reason; the pin
/// takes `00`.
pub const OPEN_DRAIN: &str = "open-drain I2C: its rise is the pull-up's, judged by extract and the campaign (RUL-19); DS12117 Rev 10 6.3.32 guarantees UM10204 timing by design";
pub const MASTER_INPUT: &str = "input in master mode: no electrical effect";
pub const SOFTWARE_DRIVEN: &str =
    "software-driven GPIO output: firmware may toggle it at any rate, so the topology states none";
pub const MOTOR_OUTPUT: &str = "motor output (a timer channel): it leaves the board, so its load is off-board, and no connector part declares it yet";

/// What the output speed check found for one generated pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinSpeed {
    pub pin: String,
    pub net: String,
    /// ST's signal name (`SPI1_SCK`), or `OUT` for a GPIO output.
    pub signal: String,
    /// The `OSPEEDR` value emitted, 0 to 1.
    pub ospeedr: u8,
    pub check: SpeedCheck,
}

/// A judged line's figures, or why the line is not judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeedCheck {
    Judged {
        rate_hz: u32,
        vdd_min_millivolts: u32,
        capacitance_femtofarads: u64,
        rise_time_ns: u32,
        /// The row it was judged at, and that row's figures.
        row: speeds::Row,
        fmax_hz: u32,
    },
    OutOfScope {
        reason: &'static str,
    },
}

/// Every pin [`generate`] emits, in pin order, with its speed and the
/// check's reasons (`AET-WO-4300c` §1.2, §1.4). A pure function of the
/// topology and the pin table (`RUL-16`); a line no selectable speed serves
/// is refused by name.
pub fn pin_speeds(solved: &SolvedTopology, table: &PinTable) -> Result<Vec<PinSpeed>, BoardError> {
    let plan = Plan::new("pin-speeds", solved, table)?;
    Ok(plan
        .pins
        .iter()
        .map(|(pin, p)| PinSpeed {
            pin: pin.clone(),
            net: p.net.clone(),
            signal: match &p.peripheral {
                Some(periph) => format!("{periph}_{}", p.role.signal()),
                None => p.role.signal().to_owned(),
            },
            ospeedr: u8::try_from(p.speed).expect("00 or 01"),
            check: p.check.clone(),
        })
        .collect())
}

/// One pin's speed: `00` for a role out of scope, else the slowest the table
/// allows for its line, read from `spi_lines` (`RUL-16`).
fn choose_speed(
    speeds_table: &OutputSpeeds,
    solved: &SolvedTopology,
    pin: &str,
    net: &str,
    role: Role,
) -> Result<(u32, SpeedCheck), BoardError> {
    if let Some(reason) = role.scope() {
        return Ok((0b00, SpeedCheck::OutOfScope { reason }));
    }
    // `SolvedTopology::validate` gives every solved SCK and MOSI a line.
    let line = solved.spi_lines.get(net).ok_or_else(|| {
        BoardError::InvalidTopology(IrValidationError::SpiLineMismatch { id: net.to_owned() })
    })?;
    let choice = speeds_table
        .choose(
            pin,
            line.rate_hz,
            line.vdd_min_millivolts,
            line.capacitance_femtofarads,
            line.rise_time_ns,
        )
        .map_err(|refusal| BoardError::PinSpeed {
            net: net.to_owned(),
            pin: pin.to_owned(),
            refusal,
        })?;
    Ok((
        u32::from(choice.ospeedr),
        SpeedCheck::Judged {
            rate_hz: line.rate_hz,
            vdd_min_millivolts: line.vdd_min_millivolts,
            capacitance_femtofarads: line.capacitance_femtofarads,
            rise_time_ns: line.rise_time_ns,
            row: choice.row,
            fmax_hz: choice.fmax_hz,
        },
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    I2c,
    Spi,
}

impl Kind {
    fn roles(self) -> &'static [Role] {
        match self {
            Self::I2c => &[Role::I2cScl, Role::I2cSda],
            Self::Spi => &[Role::SpiSck, Role::SpiMosi, Role::SpiMiso],
        }
    }
    fn header(self) -> &'static str {
        match self {
            Self::I2c => "I2C",
            Self::Spi => "SPI",
        }
    }
}

#[derive(Debug, Clone)]
struct PinPlan {
    port: char,
    index: u32,
    role: Role,
    af: u32,
    peripheral: Option<String>,
    net: String,
    /// `OSPEEDR`, from the output speed check.
    speed: u32,
    check: SpeedCheck,
}

impl PinPlan {
    /// (mode, output type, speed, pull).
    fn policy(&self) -> (u32, u32, u32, u32) {
        let (mode, otype, pupd) = self.role.policy();
        (mode, otype, self.speed, pupd)
    }
}

#[derive(Debug, Clone)]
struct Periph {
    kind: Kind,
    /// Role to pin name, in role order.
    pins: BTreeMap<Role, String>,
}

/// One SPI device and its chip select (`AET-WO-4200`).
#[derive(Debug, Clone)]
struct Select {
    reference: String,
    part: String,
    /// The device's variant in its bus's device enum: the reference
    /// upper-cased. References are ASCII and distinct in any case
    /// (`SolvedTopology::validate`), so the variants are distinct; and no
    /// upper-case word is a Rust keyword (`Self` is mixed case).
    variant: String,
    /// The chip select's pin, `PA0`: a solved GPIO output.
    pin: String,
}

/// One timer's motor outputs (`AET-WO-4300a`): its solved channels, and the
/// values its start writes. The outputs of one timer share `PSC` and `ARR`
/// (`SolvedTopology::validate`).
#[derive(Debug, Clone)]
struct TimerPlan {
    psc: u32,
    arr: u32,
    /// Channel, 1 to 4, to its pin and its `CCRk`.
    channels: BTreeMap<u32, (String, u32)>,
}

/// What the topology asks for, checked against the pin table.
#[derive(Debug, Clone)]
struct Plan {
    crate_name: String,
    specification_sha256: String,
    pin_commit: String,
    /// Pin name (`PB6`) to its plan.
    pins: BTreeMap<String, PinPlan>,
    /// Peripheral name (`I2C1`) to its plan.
    peripherals: BTreeMap<String, Periph>,
    ports: BTreeSet<char>,
    /// SPI peripheral name (`SPI1`) to its devices, in the specification's
    /// order. A peripheral with no device has no entry.
    selects: BTreeMap<String, Vec<Select>>,
    /// I2C peripheral name (`I2C1`) to the most bytes any step of a script on
    /// it writes and reads, at least two each: its transfer proof's bounds
    /// (`AET-WO-4210` adversarial A9).
    i2c_frames: BTreeMap<String, (usize, usize)>,
    /// The lifecycle banner every generated file's header carries
    /// (`aethelgard_ir::lifecycle_banner`; `AET-WO-4210` D1): empty for a
    /// board of `active` parts, whose files are then unchanged.
    lifecycle: Vec<String>,
    /// Timer number to its plan, so in ascending number. Kept apart from
    /// `peripherals`: a topology without motor outputs emits what it did
    /// before `AET-WO-4300a`, byte for byte.
    timers: BTreeMap<u32, TimerPlan>,
}

impl Plan {
    /// Whether `pin` is some SPI device's chip select.
    fn is_select(&self, pin: &str) -> bool {
        self.selects.values().flatten().any(|s| s.pin == pin)
    }
}

fn crate_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn split_pin(pin: &str) -> Result<(char, u32), BoardError> {
    pin_name(pin).ok_or_else(|| BoardError::InvalidPin { pin: pin.into() })
}

/// `PB6` is port `B`, pin 6: `P`, a port `A` to `K`, then 0 to 15 in decimal
/// digits with no leading zero and no sign. The one parser of pin names, for
/// the generator and for `aethelgard-twin`'s judge.
pub fn pin_name(pin: &str) -> Option<(char, u32)> {
    let rest = pin.strip_prefix('P')?;
    let mut chars = rest.chars();
    let port = chars.next().filter(|c| ('A'..='K').contains(c))?;
    let digits = chars.as_str();
    if digits.is_empty()
        || digits.len() > 2
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || digits.starts_with('0') && digits.len() > 1
    {
        return None;
    }
    let index: u32 = digits.parse().ok()?;
    (index <= 15).then_some((port, index))
}

impl Plan {
    fn new(
        crate_name: &str,
        solved: &SolvedTopology,
        table: &PinTable,
    ) -> Result<Self, BoardError> {
        if !crate_name_ok(crate_name) {
            return Err(BoardError::InvalidCrateName {
                name: crate_name.into(),
            });
        }
        solved.validate().map_err(BoardError::InvalidTopology)?;
        let speeds_table = speeds::table().map_err(BoardError::SpeedTable)?;
        if speeds_table.mcu != table.part {
            return Err(BoardError::SpeedTableMismatch {
                table: speeds_table.mcu.clone(),
                part: table.part.clone(),
            });
        }
        let reserved = solved
            .assignments
            .values()
            .map(|a| &a.pin)
            .chain(solved.gpio.values())
            .find(|p| RESERVED_PINS.contains(&p.as_str()));
        if let Some(pin) = reserved {
            return Err(BoardError::ReservedPin { pin: pin.clone() });
        }
        let mut pins: BTreeMap<String, PinPlan> = BTreeMap::new();
        let mut roles: BTreeMap<String, (Kind, BTreeMap<Role, Vec<String>>)> = BTreeMap::new();
        let mut timers: BTreeMap<u32, TimerPlan> = BTreeMap::new();
        for (net, a) in &solved.assignments {
            let (port, index) = split_pin(&a.pin)?;
            let entry = table
                .pins
                .get(&a.pin)
                .ok_or_else(|| BoardError::UnknownPin { pin: a.pin.clone() })?;
            let in_table = entry.alternate_functions.get(&a.signal).copied();
            if in_table != Some(a.alternate_function) {
                return Err(BoardError::AssignmentMismatch {
                    net: net.clone(),
                    pin: a.pin.clone(),
                    signal: a.signal.clone(),
                    solved: a.alternate_function,
                    table: in_table,
                });
            }
            // A motor output: `validate` holds every `TIM` assignment to one
            // `TIMn_CHk` of its own timer, with a motor output on its net.
            let channel = motor::timer_channel(&a.signal)
                .filter(|(n, _)| a.peripheral == format!("TIM{n}"))
                .and_then(|(n, k)| Some((n, k, Role::timer_channel(k)?)));
            // The pin's role, and its peripheral's kind unless it is a timer's.
            let (role, kind) = if let Some((n, k, role)) = channel {
                let mismatch = || {
                    BoardError::InvalidTopology(IrValidationError::MotorOutputMismatch {
                        id: net.clone(),
                    })
                };
                let m = solved.motor_outputs.get(net).ok_or_else(mismatch)?;
                let t = timers.entry(n).or_insert_with(|| TimerPlan {
                    psc: m.psc,
                    arr: m.arr,
                    channels: BTreeMap::new(),
                });
                if (t.psc, t.arr) != (m.psc, m.arr) {
                    return Err(BoardError::InvalidTopology(
                        IrValidationError::TimerShared {
                            id: a.peripheral.clone(),
                        },
                    ));
                }
                if t.channels.insert(k, (a.pin.clone(), m.ccr)).is_some() {
                    return Err(mismatch());
                }
                (role, None)
            } else {
                let kind = peripheral_kind(&a.peripheral)?;
                let role_name = a
                    .signal
                    .strip_prefix(a.peripheral.as_str())
                    .and_then(|s| s.strip_prefix('_'));
                let role = kind
                    .roles()
                    .iter()
                    .copied()
                    .find(|r| Some(r.signal()) == role_name)
                    .ok_or_else(|| BoardError::UnsupportedSignal {
                        peripheral: a.peripheral.clone(),
                        signal: a.signal.clone(),
                    })?;
                (role, Some(kind))
            };
            let (speed, check) = choose_speed(speeds_table, solved, &a.pin, net, role)?;
            let plan = PinPlan {
                port,
                index,
                role,
                af: u32::from(a.alternate_function),
                peripheral: Some(a.peripheral.clone()),
                net: net.clone(),
                speed,
                check,
            };
            pins.insert(a.pin.clone(), plan);
            if let Some(kind) = kind {
                roles
                    .entry(a.peripheral.clone())
                    .or_insert_with(|| (kind, BTreeMap::new()))
                    .1
                    .entry(role)
                    .or_default()
                    .push(a.pin.clone());
            }
        }
        for (net, pin) in &solved.gpio {
            let (port, index) = split_pin(pin)?;
            if !table.pins.contains_key(pin) {
                return Err(BoardError::UnknownPin { pin: pin.clone() });
            }
            let (speed, check) = choose_speed(speeds_table, solved, pin, net, Role::Output)?;
            let plan = PinPlan {
                port,
                index,
                role: Role::Output,
                af: 0,
                peripheral: None,
                net: net.clone(),
                speed,
                check,
            };
            pins.insert(pin.clone(), plan);
        }
        let mut peripherals = BTreeMap::new();
        for (name, (kind, by_role)) in roles {
            let mut assigned = BTreeMap::new();
            for role in kind.roles() {
                let found = by_role.get(role).map(Vec::as_slice).unwrap_or_default();
                let [pin] = found else {
                    return Err(BoardError::IncompletePeripheral {
                        peripheral: name,
                        role: role.signal(),
                        count: found.len(),
                    });
                };
                assigned.insert(*role, pin.clone());
            }
            peripherals.insert(
                name,
                Periph {
                    kind,
                    pins: assigned,
                },
            );
        }
        let mut selects: BTreeMap<String, Vec<Select>> = BTreeMap::new();
        let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
        for (reference, d) in solved.spi_devices_in_order() {
            if peripherals.get(&d.peripheral).map(|p: &Periph| p.kind) != Some(Kind::Spi) {
                return Err(BoardError::DeviceNotOnABus {
                    reference: reference.to_owned(),
                    peripheral: d.peripheral.clone(),
                });
            }
            // `validate` refuses a chip select that is not a solved GPIO.
            let pin = solved.gpio.get(&d.cs).ok_or_else(|| {
                BoardError::InvalidTopology(IrValidationError::SpiDeviceSelect {
                    id: reference.to_owned(),
                })
            })?;
            // A pin handed to two buses, or twice to one, would not compile:
            // refuse it by name.
            if let Some(first) = owner.insert(pin, reference) {
                return Err(BoardError::SharedChipSelect {
                    pin: pin.clone(),
                    first: first.to_owned(),
                    reference: reference.to_owned(),
                });
            }
            selects
                .entry(d.peripheral.clone())
                .or_default()
                .push(Select {
                    reference: reference.to_owned(),
                    part: d.part.clone(),
                    variant: reference.to_ascii_uppercase(),
                    pin: pin.clone(),
                });
        }
        let ports = pins.values().map(|p| p.port).collect();
        Ok(Plan {
            crate_name: crate_name.into(),
            specification_sha256: solved.specification_sha256.clone(),
            pin_commit: table.source.commit.clone(),
            pins,
            peripherals,
            ports,
            selects,
            i2c_frames: {
                let mut frames: BTreeMap<String, (usize, usize)> = BTreeMap::new();
                for (_, d) in solved.i2c_devices_in_order() {
                    let e = frames.entry(d.peripheral.clone()).or_insert((2, 2));
                    for step in aethelgard_ir::parts::steps(&d.part)
                        .map_err(BoardError::InvalidTopology)?
                        .unwrap_or_default()
                    {
                        e.0 = e.0.max(step.write.len());
                        e.1 = e.1.max(usize::from(step.width));
                    }
                }
                frames
            },
            lifecycle: solved
                .lifecycle_banner()
                .map_err(BoardError::InvalidTopology)?,
            timers,
        })
    }
}

fn peripheral_kind(name: &str) -> Result<Kind, BoardError> {
    [Kind::I2c, Kind::Spi]
        .into_iter()
        .find(|kind| {
            name.strip_prefix(kind.header())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .ok_or_else(|| BoardError::UnsupportedPeripheral {
            peripheral: name.into(),
        })
}

/// A clock gate: the RCC register's offset and name, and the bit.
#[derive(Debug, Clone)]
struct Gate {
    register: String,
    offset: u32,
    bit: u32,
}

#[derive(Debug, Clone)]
struct PortFacts {
    base: u32,
    gate: Gate,
}

/// GPIO register offsets and field widths, and each pin's positions.
#[derive(Debug, Clone)]
struct GpioFacts {
    words: u32,
    moder: u32,
    otyper: u32,
    ospeedr: u32,
    pupdr: u32,
    /// The output levels, which a `BSRR` write sets and resets.
    odr: u32,
    bsrr: u32,
    afr: [u32; 2],
    /// (MODER, OTYPER, OSPEEDR, PUPDR, AFR) widths, from consecutive positions.
    widths: [u32; 5],
}

#[derive(Debug, Clone)]
struct PinFacts {
    moder: u32,
    otyper: u32,
    ospeedr: u32,
    pupdr: u32,
    afr_index: u32,
    afr: u32,
    bs: u32,
    br: u32,
}

#[derive(Debug, Clone)]
struct PeriphFacts {
    base: u32,
    gate: Gate,
    words: u32,
    /// `CR1`, and its enable bit (`PE` or `SPE`).
    cr1: u32,
    enable: u32,
    /// The register written while disabled: `TIMINGR` or `CFG2`.
    config: u32,
    /// SPI only: `CFG2.MASTER`.
    master: Option<u32>,
    txdr: u32,
    /// I2C only: the registers and bits of a transfer.
    i2c: Option<I2cFacts>,
    /// SPI only: the registers and bits of a transfer.
    spi: Option<SpiFacts>,
}

/// The SPI registers (RM0433 §50.11) and bit positions a transfer uses, all
/// read from the header.
#[derive(Debug, Clone)]
struct SpiFacts {
    cr2: u32,
    cfg1: u32,
    sr: u32,
    ifcr: u32,
    rxdr: u32,
    cstart: u32,
    tsize: u32,
    dsize: u32,
    /// `DSIZE`'s width: the next field's position, `FTHLV`, less its own.
    dsize_width: u32,
    rxp: u32,
    eotc: u32,
}

/// The I2Cv2 registers (RM0433 §48.7) and bit positions a transfer uses, all
/// read from the header.
#[derive(Debug, Clone)]
struct I2cFacts {
    cr2: u32,
    isr: u32,
    icr: u32,
    rxdr: u32,
    sadd: u32,
    rd_wrn: u32,
    nbytes: u32,
    start: u32,
    autoend: u32,
    txis: u32,
    tc: u32,
    rxne: u32,
    stopf: u32,
    stopcf: u32,
    nackcf: u32,
}

/// One timer instance: its base, clock gate and block size.
#[derive(Debug, Clone)]
struct TimerFacts {
    base: u32,
    gate: Gate,
    words: u32,
}

/// The `TIM_TypeDef` registers (RM0433 Rev 9 §38.4) a timer's start writes,
/// and the bit positions of its fields, all read from the header. Index
/// `k - 1` of a per-channel array is channel `k`'s.
#[derive(Debug, Clone)]
struct TimFacts {
    cr1: u32,
    egr: u32,
    /// `CCMR1` (channels 1 and 2) and `CCMR2` (3 and 4).
    ccmr: [u32; 2],
    ccer: u32,
    psc: u32,
    arr: u32,
    ccr: [u32; 4],
    bdtr: u32,
    cen: u32,
    arpe: u32,
    ug: u32,
    moe: u32,
    /// `OCkM` and `OCkPE`, each in its channel's `CCMR`.
    ocm: [u32; 4],
    ocpe: [u32; 4],
    /// `CCkE`.
    cce: [u32; 4],
}

/// `PWM mode 1`, `OCkM = 110` (RM0433 Rev 9 §38.4.7): the channel is active
/// while the counter is below `CCRk`. The fourth bit, `OCkM[3]`, stays 0.
const PWM_MODE_1: u32 = 0b110;

/// The registers [`Rcc::reset_clock`]-to-be reads and the fields each must
/// hold at 0, their reset value (RM0433 Rev 9 §8.7.2, §8.7.7–8.7.9): register name,
/// `RCC_TypeDef` offset, and each field's name, position and width.
type ClockFacts = Vec<(&'static str, u32, Vec<(&'static str, u32, u32)>)>;

/// The reset clock's fields, by register: `CR` `HSIDIV` (HSI undivided, so
/// 64 MHz; `AET-WO-4300a` Q2, ruled), `CFGR` `SWS` (HSI) and `TIMPRE`,
/// `D1CFGR` `D1CPRE`, `D1PPRE` and `HPRE`, `D2CFGR` `D2PPRE1` and `D2PPRE2`.
const RESET_CLOCK_FIELDS: [(&str, &[&str]); 4] = [
    ("CR", &["HSIDIV"]),
    ("CFGR", &["SWS", "TIMPRE"]),
    ("D1CFGR", &["D1CPRE", "D1PPRE", "HPRE"]),
    ("D2CFGR", &["D2PPRE1", "D2PPRE2"]),
];

#[derive(Debug, Clone)]
struct Facts {
    rcc_base: u32,
    rcc_words: u32,
    gpio: GpioFacts,
    ports: BTreeMap<char, PortFacts>,
    pins: BTreeMap<String, PinFacts>,
    peripherals: BTreeMap<String, PeriphFacts>,
    /// Timer number to its instance facts; empty without motor outputs, and
    /// then `tim` and `clock` are `None`: nothing timer-shaped is read.
    timers: BTreeMap<u32, TimerFacts>,
    tim: Option<TimFacts>,
    clock: Option<ClockFacts>,
}

fn tim_facts(f: &RegisterFacts) -> Result<TimFacts, BoardError> {
    let t = f.layout("TIM")?;
    let each = |name: &dyn Fn(usize) -> String| -> Result<[u32; 4], BoardError> {
        let mut out = [0; 4];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = f.position(&name(i))?;
        }
        Ok(out)
    };
    let ccmr = |i: usize| if i < 2 { 1 } else { 2 };
    Ok(TimFacts {
        cr1: t.offset("CR1", 0)?,
        egr: t.offset("EGR", 0)?,
        ccmr: [t.offset("CCMR1", 0)?, t.offset("CCMR2", 0)?],
        ccer: t.offset("CCER", 0)?,
        psc: t.offset("PSC", 0)?,
        arr: t.offset("ARR", 0)?,
        ccr: [
            t.offset("CCR1", 0)?,
            t.offset("CCR2", 0)?,
            t.offset("CCR3", 0)?,
            t.offset("CCR4", 0)?,
        ],
        bdtr: t.offset("BDTR", 0)?,
        cen: f.position("TIM_CR1_CEN_Pos")?,
        arpe: f.position("TIM_CR1_ARPE_Pos")?,
        ug: f.position("TIM_EGR_UG_Pos")?,
        moe: f.position("TIM_BDTR_MOE_Pos")?,
        ocm: each(&|i| format!("TIM_CCMR{}_OC{}M_Pos", ccmr(i), i + 1))?,
        ocpe: each(&|i| format!("TIM_CCMR{}_OC{}PE_Pos", ccmr(i), i + 1))?,
        cce: each(&|i| format!("TIM_CCER_CC{}E_Pos", i + 1))?,
    })
}

fn clock_facts(f: &RegisterFacts, rcc: &cmsis::Layout) -> Result<ClockFacts, BoardError> {
    let mut out = Vec::new();
    for (register, fields) in RESET_CLOCK_FIELDS {
        let mut read = Vec::new();
        for field in fields {
            let (pos, width) = f.field(&format!("RCC_{register}_{field}"))?;
            read.push((*field, pos, width));
        }
        out.push((register, rcc.offset(register, 0)?, read));
    }
    Ok(out)
}

fn spi_facts(f: &RegisterFacts, layout: &cmsis::Layout) -> Result<SpiFacts, BoardError> {
    let dsize = f.position("SPI_CFG1_DSIZE_Pos")?;
    let dsize_width = f
        .position("SPI_CFG1_FTHLV_Pos")?
        .checked_sub(dsize)
        .filter(|w| (3..=5).contains(w))
        .ok_or_else(|| {
            BoardError::Register(RegisterError::MalformedSource {
                detail: "`SPI_CFG1_DSIZE` and `SPI_CFG1_FTHLV` positions do not give a field width"
                    .into(),
            })
        })?;
    Ok(SpiFacts {
        cr2: layout.offset("CR2", 0)?,
        cfg1: layout.offset("CFG1", 0)?,
        sr: layout.offset("SR", 0)?,
        ifcr: layout.offset("IFCR", 0)?,
        rxdr: layout.offset("RXDR", 0)?,
        cstart: f.position("SPI_CR1_CSTART_Pos")?,
        tsize: f.position("SPI_CR2_TSIZE_Pos")?,
        dsize,
        dsize_width,
        rxp: f.position("SPI_SR_RXP_Pos")?,
        eotc: f.position("SPI_IFCR_EOTC_Pos")?,
    })
}

fn gate(facts: &RegisterFacts, rcc: &cmsis::Layout, name: &str) -> Result<Gate, BoardError> {
    let (register, bit) = facts.enable_bit(name)?;
    let offset = rcc.offset(&register, 0)?;
    Ok(Gate {
        register,
        offset,
        bit,
    })
}

impl Facts {
    fn new(plan: &Plan, f: &RegisterFacts) -> Result<Self, BoardError> {
        let rcc = f.layout("RCC")?;
        let g = f.layout("GPIO")?;
        let width = |field: &str| -> Result<u32, BoardError> {
            let p0 = f.position(&format!("GPIO_{field}0_Pos"))?;
            let p1 = f.position(&format!("GPIO_{field}1_Pos"))?;
            p1.checked_sub(p0)
                .filter(|w| (1..=4).contains(w))
                .ok_or_else(|| {
                    BoardError::Register(RegisterError::MalformedSource {
                        detail: format!("`GPIO_{field}` positions do not give a field width"),
                    })
                })
        };
        let gpio = GpioFacts {
            words: g.size / 4,
            moder: g.offset("MODER", 0)?,
            otyper: g.offset("OTYPER", 0)?,
            ospeedr: g.offset("OSPEEDR", 0)?,
            pupdr: g.offset("PUPDR", 0)?,
            odr: g.offset("ODR", 0)?,
            bsrr: g.offset("BSRR", 0)?,
            afr: [g.offset("AFR", 0)?, g.offset("AFR", 1)?],
            widths: [
                width("MODER_MODE")?,
                width("OTYPER_OT")?,
                width("OSPEEDR_OSPEED")?,
                width("PUPDR_PUPD")?,
                width("AFRL_AFSEL")?,
            ],
        };
        let mut ports = BTreeMap::new();
        for port in &plan.ports {
            let name = format!("GPIO{port}");
            ports.insert(
                *port,
                PortFacts {
                    base: f.base(&name)?,
                    gate: gate(f, &rcc, &name)?,
                },
            );
        }
        let mut pins = BTreeMap::new();
        for (name, p) in &plan.pins {
            let n = p.index;
            let (afr_index, afr) = if n < 8 {
                (0, f.position(&format!("GPIO_AFRL_AFSEL{n}_Pos"))?)
            } else {
                (1, f.position(&format!("GPIO_AFRH_AFSEL{n}_Pos"))?)
            };
            pins.insert(
                name.clone(),
                PinFacts {
                    moder: f.position(&format!("GPIO_MODER_MODE{n}_Pos"))?,
                    otyper: f.position(&format!("GPIO_OTYPER_OT{n}_Pos"))?,
                    ospeedr: f.position(&format!("GPIO_OSPEEDR_OSPEED{n}_Pos"))?,
                    pupdr: f.position(&format!("GPIO_PUPDR_PUPD{n}_Pos"))?,
                    afr_index,
                    afr,
                    bs: f.position(&format!("GPIO_BSRR_BS{n}_Pos"))?,
                    br: f.position(&format!("GPIO_BSRR_BR{n}_Pos"))?,
                },
            );
        }
        let mut peripherals = BTreeMap::new();
        for (name, p) in &plan.peripherals {
            let kind = p.kind.header();
            let layout = f.layout(kind)?;
            let (enable, config, master) = match p.kind {
                Kind::I2c => (
                    f.position("I2C_CR1_PE_Pos")?,
                    layout.offset("TIMINGR", 0)?,
                    None,
                ),
                Kind::Spi => (
                    f.position("SPI_CR1_SPE_Pos")?,
                    layout.offset("CFG2", 0)?,
                    Some(f.position("SPI_CFG2_MASTER_Pos")?),
                ),
            };
            peripherals.insert(
                name.clone(),
                PeriphFacts {
                    base: f.base(name)?,
                    gate: gate(f, &rcc, name)?,
                    words: layout.size / 4,
                    cr1: layout.offset("CR1", 0)?,
                    enable,
                    config,
                    master,
                    txdr: layout.offset("TXDR", 0)?,
                    i2c: match p.kind {
                        Kind::I2c => Some(I2cFacts {
                            cr2: layout.offset("CR2", 0)?,
                            isr: layout.offset("ISR", 0)?,
                            icr: layout.offset("ICR", 0)?,
                            rxdr: layout.offset("RXDR", 0)?,
                            sadd: f.position("I2C_CR2_SADD_Pos")?,
                            rd_wrn: f.position("I2C_CR2_RD_WRN_Pos")?,
                            nbytes: f.position("I2C_CR2_NBYTES_Pos")?,
                            start: f.position("I2C_CR2_START_Pos")?,
                            autoend: f.position("I2C_CR2_AUTOEND_Pos")?,
                            txis: f.position("I2C_ISR_TXIS_Pos")?,
                            tc: f.position("I2C_ISR_TC_Pos")?,
                            rxne: f.position("I2C_ISR_RXNE_Pos")?,
                            stopf: f.position("I2C_ISR_STOPF_Pos")?,
                            stopcf: f.position("I2C_ICR_STOPCF_Pos")?,
                            nackcf: f.position("I2C_ICR_NACKCF_Pos")?,
                        }),
                        Kind::Spi => None,
                    },
                    spi: match p.kind {
                        Kind::I2c => None,
                        Kind::Spi => Some(spi_facts(f, &layout)?),
                    },
                },
            );
        }
        let mut timers = BTreeMap::new();
        let (mut tim, mut clock) = (None, None);
        if !plan.timers.is_empty() {
            let words = f.layout("TIM")?.size / 4;
            for n in plan.timers.keys() {
                let name = format!("TIM{n}");
                timers.insert(
                    *n,
                    TimerFacts {
                        base: f.base(&name)?,
                        gate: gate(f, &rcc, &name)?,
                        words,
                    },
                );
            }
            tim = Some(tim_facts(f)?);
            clock = Some(clock_facts(f, &rcc)?);
        }
        Ok(Facts {
            rcc_base: f.base("RCC")?,
            rcc_words: rcc.size / 4,
            gpio,
            ports,
            pins,
            peripherals,
            timers,
            tim,
            clock,
        })
    }
}

/// `I2C1` → `I2c1`, `A` → `GpioA`.
fn type_name(peripheral: &str) -> String {
    let mut out = String::new();
    for (i, c) in peripheral.chars().enumerate() {
        out.push(if i == 0 { c } else { c.to_ascii_lowercase() });
    }
    out
}
fn port_type(port: char) -> String {
    format!("Gpio{port}")
}
fn port_field(port: char) -> String {
    format!("gpio{}", port.to_ascii_lowercase())
}
fn pin_field(pin: &str) -> String {
    pin.to_ascii_lowercase()
}
fn method(p: &PinPlan) -> String {
    match &p.peripheral {
        Some(periph) => format!(
            "into_{}_{}",
            periph.to_ascii_lowercase(),
            p.role.signal().to_ascii_lowercase()
        ),
        None => "into_output".to_owned(),
    }
}
fn pin_state(p: &PinPlan) -> String {
    match p.role {
        Role::Output => "Output".to_owned(),
        _ => format!("AlternateFunction<{}>", p.af),
    }
}

fn header(plan: &Plan, what: &str) -> String {
    format!(
        "//! {what}\n//!\n\
         //! @generated by `aethelgard-svd-hal` (AET-WO-1400). Do not edit: the regeneration\n\
         //! test compares every byte of this tree with the generator's output.\n\
         //!\n\
         //! - Solved topology of specification sha256 `{spec}`.\n\
         //! - Register facts: `{repo}` at `{commit}`, `{file}` sha256 `{hash}`.\n\
         //! - Pin data: `STM32_open_pin_data` at `{pins}`.\n{banner}",
        spec = plan.specification_sha256,
        repo = cmsis::REPOSITORY,
        commit = cmsis::COMMIT,
        file = cmsis::HEADER_FILE,
        hash = cmsis::HEADER_SHA256,
        pins = plan.pin_commit,
        banner = plan
            .lifecycle
            .iter()
            .map(|l| format!("//! - {l}.\n"))
            .collect::<String>(),
    )
}

fn emit_manifest(plan: &Plan) -> String {
    format!(
        "# @generated by aethelgard-svd-hal (AET-WO-1400). Not a workspace member: the\n\
         # repository root excludes `boards/`. No dependencies, so `aethelgard prove`\n\
         # can judge it with no network.\n\
         {banner}\
         [package]\n\
         name = \"{name}\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         license = \"MIT OR Apache-2.0\"\n\
         \n\
         [lints.rust]\n\
         unexpected_cfgs = {{ level = \"warn\", check-cfg = [\"cfg(kani)\"] }}\n",
        name = plan.crate_name,
        banner = plan
            .lifecycle
            .iter()
            .map(|l| format!("# {l}.\n"))
            .collect::<String>(),
    )
}

/// What cargo writes for a package with no dependencies.
fn emit_lock(plan: &Plan) -> String {
    format!(
        "# This file is automatically @generated by Cargo.\n\
         # It is not intended for manual editing.\n\
         version = 4\n\
         \n\
         [[package]]\n\
         name = \"{}\"\n\
         version = \"0.0.0\"\n",
        plan.crate_name
    )
}

/// The crate root: a shell, already in `rustfmt`'s form. `cargo fmt --all`
/// reaches this crate through svd-hal's path dev-dependency, so the generated
/// modules carry `#[rustfmt::skip]` (an inner `#![rustfmt::skip]` is unstable):
/// the generator, not the formatter, owns their bytes, and regeneration
/// compares exactly what the generator wrote.
fn emit_root(plan: &Plan) -> String {
    let mut s = header(plan, "The board crate for one solved topology.");
    s.push_str(
        "#![no_std]\n\
         \n\
         #[rustfmt::skip]\n\
         mod board;\n\
         #[cfg(kani)]\n\
         #[rustfmt::skip]\n\
         mod proofs;\n\
         \n\
         pub use board::*;\n",
    );
    s
}

/// A per-I2C-peripheral speed trait, with the modes it admits.
type Speeds = BTreeMap<String, (u32, Vec<usize>)>;

fn emit_lib(plan: &Plan, facts: &Facts, speeds: &Speeds) -> String {
    let mut s = header(
        plan,
        "Typestates for every solved pin, port and peripheral.",
    );
    let w = &mut s;
    let _ = writeln!(
        w,
        "//!\n\
         //! Every register access goes through [`Registers`]. [`mmio`] is its only\n\
         //! implementation at real addresses and the crate's only `unsafe`. The Kani\n\
         //! harness in `proofs` implements it over a register image it owns.\n\
         //!\n\
         //! [`Board::take`] is the crate's one runtime check: a second call returns\n\
         //! `None`. Everything else that is illegal does not compile (`INV-02`).\n\
         \n\
         use core::marker::PhantomData;\n\
         use core::sync::atomic::{{AtomicBool, Ordering}};\n\
         \n\
         /// The register blocks this board touches.\n\
         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
         pub enum Block {{\n    Rcc,"
    );
    for port in &plan.ports {
        let _ = writeln!(w, "    {},", port_type(*port));
    }
    for name in plan.peripherals.keys() {
        let _ = writeln!(w, "    {},", type_name(name));
    }
    for n in plan.timers.keys() {
        let _ = writeln!(w, "    Tim{n},");
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         /// The lock on [`Registers`].\n\
         pub(crate) mod sealed {{\n    \
             pub trait Sealed {{}}\n\
         \n    \
             /// Only this crate can make one.\n    \
             pub struct Token(pub(crate) ());\n\
         }}\n\
         \n\
         const TOKEN: sealed::Token = sealed::Token(());\n\
         \n\
         /// The one door to the hardware. Offsets are byte offsets into the block.\n\
         ///\n\
         /// It is sealed, and every method takes a [`sealed::Token`] that only this\n\
         /// crate can make. So firmware can neither implement it, which would advance\n\
         /// the typestates on a stand-in, nor call it, which would reach a register\n\
         /// around them. Only the typestate API below can (`INV-02`; adversarial E2, E3).\n\
         ///\n\
         /// `read8` and `write8` are single-byte accesses, for the SPI data registers\n\
         /// only: with frames of 8 bits or less, a 16- or 32-bit access to `SPI_TXDR` or\n\
         /// `SPI_RXDR` packs two or four frames on silicon (RM0433 Rev 9 §50.4.13) and\n\
         /// one in Renode 1.17.0, so only a byte access means the same on both.\n\
         pub trait Registers: sealed::Sealed {{\n    \
             fn read(&mut self, token: sealed::Token, block: Block, offset: usize) -> u32;\n    \
             fn write(&mut self, token: sealed::Token, block: Block, offset: usize, value: u32);\n    \
             fn read8(&mut self, token: sealed::Token, block: Block, offset: usize) -> u8;\n    \
             fn write8(&mut self, token: sealed::Token, block: Block, offset: usize, value: u8);\n    \
             fn modify(&mut self, token: sealed::Token, block: Block, offset: usize, f: impl FnOnce(u32) -> u32) {{\n        \
                 let value = self.read(sealed::Token(()), block, offset);\n        \
                 self.write(token, block, offset, f(value));\n    \
             }}\n\
         }}\n\
         \n\
         /// Volatile access at the part's real base addresses. Never exercised by\n\
         /// the proof: Kani cannot own this memory (`architecture.md` §2.2).\n\
         pub mod mmio {{\n    \
             use super::{{Block, Registers, sealed}};\n\
         \n    \
             pub struct Mmio {{\n        \
                 _private: (),\n    \
             }}\n\
         \n    \
             impl Mmio {{\n        \
                 /// # Safety\n        \
                 /// The caller runs on the part this crate was generated for, and no\n        \
                 /// other `Mmio` exists.\n        \
                 pub unsafe fn new() -> Self {{\n            \
                     Mmio {{ _private: () }}\n        \
                 }}\n    \
             }}\n\
         \n    \
             const fn base(block: Block) -> usize {{\n        \
                 match block {{\n            \
                     Block::Rcc => {rcc:#010x},",
        rcc = facts.rcc_base
    );
    for port in &plan.ports {
        let _ = writeln!(
            w,
            "            Block::{} => {:#010x},",
            port_type(*port),
            facts.ports[port].base
        );
    }
    for name in plan.peripherals.keys() {
        let _ = writeln!(
            w,
            "            Block::{} => {:#010x},",
            type_name(name),
            facts.peripherals[name].base
        );
    }
    for (n, tf) in &facts.timers {
        let _ = writeln!(w, "            Block::Tim{n} => {:#010x},", tf.base);
    }
    let _ = writeln!(
        w,
        "        }}\n    \
         }}\n\
         \n    \
         impl sealed::Sealed for Mmio {{}}\n\
         \n    \
         impl Registers for Mmio {{\n        \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n            \
                 // SAFETY: a register of a peripheral that exists on this part.\n            \
                 unsafe {{ core::ptr::read_volatile((base(block) + offset) as *const u32) }}\n        \
             }}\n        \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n            \
                 // SAFETY: as for `read`.\n            \
                 unsafe {{ core::ptr::write_volatile((base(block) + offset) as *mut u32, value) }}\n        \
             }}\n        \
             fn read8(&mut self, _: sealed::Token, block: Block, offset: usize) -> u8 {{\n            \
                 // SAFETY: as for `read`, one byte wide.\n            \
                 unsafe {{ core::ptr::read_volatile((base(block) + offset) as *const u8) }}\n        \
             }}\n        \
             fn write8(&mut self, _: sealed::Token, block: Block, offset: usize, value: u8) {{\n            \
                 // SAFETY: as for `read`, one byte wide.\n            \
                 unsafe {{ core::ptr::write_volatile((base(block) + offset) as *mut u8, value) }}\n        \
             }}\n    \
         }}\n\
         }}\n\
         \n\
         /// Pin modes.\n\
         pub struct Unconfigured;\n\
         pub struct AlternateFunction<const AF: u8>;\n\
         pub struct Output;\n\
         \n\
         /// Clock and peripheral states.\n\
         pub struct Gated;\n\
         pub struct Clocked;\n\
         pub struct Enabled<S>(PhantomData<S>);\n"
    );
    if !plan.timers.is_empty() {
        let _ = writeln!(
            w,
            "/// A timer whose outputs run. It has no method: after `CEN`, nothing reads or\n\
             /// writes the timer again (`AET-WO-4300a`: in Renode, a core polling a running\n\
             /// timer collapses its edges onto one timestamp).\n\
             pub struct Running;\n\
             \n\
             /// The clock tree is the reset clock the timers' values were solved for: HSI\n\
             /// at 64 MHz, every prescaler 1, `TIMPRE` 0. Only [`Rcc::reset_clock`] makes one.\n\
             pub struct ResetClock {{\n    \
                 _private: (),\n\
             }}\n\
             \n\
             /// [`Rcc::reset_clock`] read a field away from its reset value.\n\
             #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
             pub struct NotResetClock;\n"
        );
    }
    // Speeds.
    for (name, (hz, admitted)) in speeds {
        let note = format!(
            "/// The speeds at or under `{name}`'s declared speed of {hz} Hz.\n\
             /// Sealed: firmware cannot admit another."
        );
        let _ = writeln!(
            w,
            "/// `{name}` bus speeds. `TIMINGR` values are placeholders for a 64 MHz kernel\n\
             /// clock: clock-tree solving is out of scope.\n\
             pub mod {m}_speed {{\n    \
                 mod sealed {{\n        \
                     pub trait Sealed {{}}\n    \
                 }}\n\
             \n    \
                 {note}\n    \
                 pub trait AtMostCeiling: sealed::Sealed {{\n        \
                     const TIMINGR: u32;\n    \
                 }}\n",
            m = name.to_ascii_lowercase(),
            note = note.replace('\n', "\n    "),
        );
        for (mode, _, _) in I2C_MODES {
            let _ = writeln!(w, "    pub struct {mode};");
        }
        for &i in admitted {
            let (mode, _, timingr) = I2C_MODES[i];
            let _ = writeln!(
                w,
                "\n    impl sealed::Sealed for {mode} {{}}\n    \
                 impl AtMostCeiling for {mode} {{\n        \
                     const TIMINGR: u32 = {timingr:#010x};\n    \
                 }}"
            );
        }
        let _ = writeln!(w, "}}\n");
    }
    if !speeds.is_empty() {
        w.push_str(I2C_SHARED);
    }
    // Every SPI peripheral shares `SPI_TypeDef` and the header's positions.
    let spi = plan.selects.keys().find_map(|name| {
        Some((
            facts.peripherals[name].spi.as_ref()?,
            facts.peripherals[name].txdr,
        ))
    });
    if let Some((spi, txdr)) = spi {
        emit_spi_shared(w, spi, txdr);
    }
    // Pins.
    for (pin, p) in &plan.pins {
        let pf = &facts.pins[pin];
        let g = &facts.gpio;
        let (mode, otype, ospeed, pupd) = p.policy();
        let [mw, tw, sw, pw, aw] = g.widths;
        let block = port_type(p.port);
        let what = match &p.peripheral {
            Some(periph) => format!("`{periph}_{}` on AF{}", p.role.signal(), p.af),
            None => "a GPIO output".to_owned(),
        };
        let _ = writeln!(
            w,
            "/// `{pin}`, net `{net}`: {what}.\n\
             pub struct {pin}<M> {{\n    \
                 _mode: PhantomData<M>,\n\
             }}\n\
             \n\
             impl {pin}<Unconfigured> {{\n    \
                 pub fn {method}<R: Registers>(self, r: &mut R) -> {pin}<{state}> {{",
            net = p.net,
            method = method(p),
            state = pin_state(p),
        );
        let field = |w: &mut String, reg: u32, pos: u32, width: u32, value: u32| {
            let mask = ((1u32 << width) - 1) << pos;
            let set = value << pos;
            let body = if set == 0 {
                format!("v & !{mask:#010x}")
            } else {
                format!("(v & !{mask:#010x}) | {set:#010x}")
            };
            let _ = writeln!(
                w,
                "        r.modify(TOKEN, Block::{block}, {reg:#04x}, |v| {body});"
            );
        };
        field(w, g.otyper, pf.otyper, tw, otype);
        field(w, g.ospeedr, pf.ospeedr, sw, ospeed);
        field(w, g.pupdr, pf.pupdr, pw, pupd);
        if p.role == Role::Output {
            let _ = writeln!(
                w,
                "        r.write(TOKEN, Block::{block}, {:#04x}, {:#010x});",
                g.bsrr,
                1u32 << pf.bs
            );
        } else {
            field(w, g.afr[pf.afr_index as usize], pf.afr, aw, p.af);
        }
        field(w, g.moder, pf.moder, mw, mode);
        let _ = writeln!(w, "        {pin} {{ _mode: PhantomData }}\n    }}\n}}\n");
        if p.role == Role::Output {
            let _ = writeln!(
                w,
                "impl {pin}<Output> {{\n    \
                     pub fn set_high<R: Registers>(&mut self, r: &mut R) {{\n        \
                         r.write(TOKEN, Block::{block}, {bsrr:#04x}, {bs:#010x});\n    \
                     }}\n    \
                     pub fn set_low<R: Registers>(&mut self, r: &mut R) {{\n        \
                         r.write(TOKEN, Block::{block}, {bsrr:#04x}, {br:#010x});\n    \
                     }}\n\
                 }}\n",
                bsrr = g.bsrr,
                bs = 1u32 << pf.bs,
                br = 1u32 << pf.br,
            );
        }
    }
    // Ports.
    for port in &plan.ports {
        let t = port_type(*port);
        let pins: Vec<&String> = plan
            .pins
            .iter()
            .filter(|(_, p)| p.port == *port)
            .map(|(n, _)| n)
            .collect();
        let _ = writeln!(
            w,
            "/// GPIO port {port}. Its pins exist only once its clock is enabled.\n\
             pub struct {t}<S> {{\n    \
                 _state: PhantomData<S>,\n\
             }}\n\
             \n\
             /// The solved pins of port {port}.\n\
             pub struct {t}Pins {{"
        );
        for pin in &pins {
            let _ = writeln!(w, "    pub {}: {pin}<Unconfigured>,", pin_field(pin));
        }
        let _ = writeln!(
            w,
            "}}\n\
             \n\
             impl {t}<Clocked> {{\n    \
                 pub fn split(self) -> {t}Pins {{\n        \
                     {t}Pins {{"
        );
        for pin in &pins {
            let _ = writeln!(
                w,
                "            {}: {pin} {{ _mode: PhantomData }},",
                pin_field(pin)
            );
        }
        let _ = writeln!(w, "        }}\n    }}\n}}\n");
    }
    // Peripherals.
    for (name, p) in &plan.peripherals {
        let t = type_name(name);
        let pf = &facts.peripherals[name];
        let block = &t;
        let params: Vec<String> = p
            .pins
            .iter()
            .map(|(role, pin)| {
                format!(
                    "_{}: {pin}<{}>",
                    role.signal().to_ascii_lowercase(),
                    pin_state(&plan.pins[pin])
                )
            })
            .collect();
        let _ = writeln!(
            w,
            "/// `{name}`. It starts only from `Clocked`, and only on its solved pins.\n\
             pub struct {t}<S> {{\n    \
                 _state: PhantomData<S>,\n\
             }}\n"
        );
        let enable = 1u32 << pf.enable;
        match p.kind {
            Kind::I2c => {
                let m = name.to_ascii_lowercase();
                let _ = writeln!(
                    w,
                    "impl {t}<Clocked> {{\n    \
                         pub fn init<S: {m}_speed::AtMostCeiling, R: Registers>(\n        \
                             self,\n        \
                             r: &mut R,\n        \
                             {params},\n    \
                         ) -> {t}<Enabled<S>> {{\n        \
                             r.modify(TOKEN, Block::{block}, {cr1:#04x}, |v| v & !{enable:#010x});\n        \
                             r.write(TOKEN, Block::{block}, {cfg:#04x}, S::TIMINGR);\n        \
                             r.modify(TOKEN, Block::{block}, {cr1:#04x}, |v| v | {enable:#010x});\n        \
                             {t} {{ _state: PhantomData }}\n    \
                         }}\n\
                     }}\n\
                     \n\
                     impl<S: {m}_speed::AtMostCeiling> {t}<Enabled<S>> {{",
                    params = params.join(",\n        "),
                    cr1 = pf.cr1,
                    cfg = pf.config,
                );
            }
            Kind::Spi => {
                let master = 1u32 << pf.master.unwrap_or_default();
                let selects = plan
                    .selects
                    .get(name)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let mut params = params;
                // Each select's BSRR set write, to drive it high in `init`.
                let mut high = String::new();
                if !selects.is_empty() {
                    let _ = writeln!(
                        w,
                        "/// The chip selects of `{name}`'s devices, each already an output. [`{t}::init`]\n\
                         /// takes them all, so nothing transfers on `{name}` before every chip select of\n\
                         /// the bus is driven: an undriven chip select is a selected part (`AET-WO-4200`\n\
                         /// F3).\n\
                         pub struct {t}Selects {{"
                    );
                    for sel in selects {
                        let pin = &plan.pins[&sel.pin];
                        let _ = writeln!(
                            w,
                            "    /// `{r}`'s, net `{net}`.\n    pub {f}: {p}<Output>,",
                            r = sel.reference,
                            net = pin.net,
                            f = pin_field(&sel.pin),
                            p = sel.pin,
                        );
                        let _ = writeln!(
                            high,
                            "        r.write(TOKEN, Block::{port}, {bsrr:#04x}, {bs:#010x});",
                            port = port_type(pin.port),
                            bsrr = facts.gpio.bsrr,
                            bs = 1u32 << facts.pins[&sel.pin].bs,
                        );
                    }
                    let _ = writeln!(
                        w,
                        "}}\n\
                         \n\
                         /// The devices on `{name}`, in the specification's order: each reference,\n\
                         /// upper-cased.\n\
                         #[allow(non_camel_case_types)]\n\
                         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
                         pub enum {t}Device {{"
                    );
                    for sel in selects {
                        let _ = writeln!(
                            w,
                            "    /// `{r}`, a `{part}`, chip select `{pin}`.\n    {v},",
                            r = sel.reference,
                            part = sel.part,
                            pin = sel.pin,
                            v = sel.variant,
                        );
                    }
                    let _ = writeln!(w, "}}\n");
                    params.push(format!("_selects: {t}Selects"));
                }
                let doc_selects = if selects.is_empty() {
                    ""
                } else {
                    " Every chip select of the bus is\n    \
                     /// driven high first (deselected), whatever firmware did with it before."
                };
                let _ = writeln!(
                    w,
                    "impl {t}<Clocked> {{\n    \
                         /// Master mode, 8-bit frames, and the peripheral left disabled: each\n    \
                         /// transfer enables it and disables it again.{doc_selects} The\n    \
                         /// baud rate, clock polarity and phase, and slave-select management are not\n    \
                         /// solved yet: they stay at reset (Renode 1.17.0 does not model them).\n    \
                         pub fn init<R: Registers>(\n        \
                             self,\n        \
                             r: &mut R,\n        \
                             {params},\n    \
                         ) -> {t}<Enabled<()>> {{\n\
                         {high}        \
                             r.modify(TOKEN, Block::{block}, {cr1:#04x}, |v| v & !{enable:#010x});\n        \
                             r.modify(TOKEN, Block::{block}, {cfg1:#04x}, |v| (v & !{dmask:#010x}) | {dsize:#010x});\n        \
                             r.modify(TOKEN, Block::{block}, {cfg:#04x}, |v| v | {master:#010x});\n        \
                             {t} {{ _state: PhantomData }}\n    \
                         }}\n\
                     }}\n",
                    params = params.join(",\n        "),
                    cr1 = pf.cr1,
                    cfg = pf.config,
                    cfg1 = pf.spi.as_ref().map_or(0, |s| s.cfg1),
                    dmask = pf
                        .spi
                        .as_ref()
                        .map_or(0, |s| oracle_mask(s.dsize_width, s.dsize)),
                    dsize = pf.spi.as_ref().map_or(0, |s| SPI_DSIZE << s.dsize),
                );
                if !selects.is_empty() {
                    emit_spi_transfer(w, name, selects, plan, facts);
                }
            }
        }
        if let Some(i) = &pf.i2c {
            let _ = writeln!(
                w,
                "    pub fn write_byte<R: Registers>(&mut self, r: &mut R, byte: u8) {{\n        \
                     r.write(TOKEN, Block::{block}, {txdr:#04x}, u32::from(byte));\n    \
                 }}\n\
                 }}\n",
                txdr = pf.txdr
            );
            emit_transfer(w, name, i, pf.txdr);
        }
    }
    if let Some(tf) = &facts.tim {
        for (n, t) in &plan.timers {
            emit_timer_start(w, *n, t, plan, tf);
        }
    }
    // RCC.
    let _ = writeln!(
        w,
        "/// The clock gates, one method per solved port and peripheral.\n\
         pub struct Rcc {{\n    \
             _private: (),\n\
         }}\n\
         \n\
         impl Rcc {{"
    );
    let gates = facts
        .ports
        .iter()
        .map(|(port, pf)| (port_field(*port), port_type(*port), &pf.gate))
        .chain(
            facts
                .peripherals
                .iter()
                .map(|(name, pf)| (name.to_ascii_lowercase(), type_name(name), &pf.gate)),
        )
        .chain(
            facts
                .timers
                .iter()
                .map(|(n, tf)| (format!("tim{n}"), format!("Tim{n}"), &tf.gate)),
        );
    let mut first = true;
    for (field, t, g) in gates {
        if !first {
            let _ = writeln!(w);
        }
        first = false;
        let _ = writeln!(
            w,
            "    /// `RCC_{reg}` bit {bit}.\n    \
             pub fn enable_{field}<R: Registers>(&mut self, r: &mut R, _gated: {t}<Gated>) -> {t}<Clocked> {{\n        \
                 r.modify(TOKEN, Block::Rcc, {off:#05x}, |v| v | {mask:#010x});\n        \
                 {t} {{ _state: PhantomData }}\n    \
             }}",
            reg = g.register,
            bit = g.bit,
            off = g.offset,
            mask = 1u32 << g.bit,
        );
    }
    if let Some(clock) = &facts.clock {
        emit_reset_clock(w, clock);
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         /// Everything on the board, once per program.\n\
         pub struct Board {{\n    \
             pub rcc: Rcc,"
    );
    for port in &plan.ports {
        let _ = writeln!(
            w,
            "    pub {}: {}<Gated>,",
            port_field(*port),
            port_type(*port)
        );
    }
    for name in plan.peripherals.keys() {
        let _ = writeln!(
            w,
            "    pub {}: {}<Gated>,",
            name.to_ascii_lowercase(),
            type_name(name)
        );
    }
    for n in plan.timers.keys() {
        let _ = writeln!(w, "    pub tim{n}: Tim{n}<Gated>,");
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         static TAKEN: AtomicBool = AtomicBool::new(false);\n\
         \n\
         impl Board {{\n    \
             /// The board, the first time; `None` after that. The crate's one\n    \
             /// runtime check.\n    \
             pub fn take() -> Option<Board> {{\n        \
                 if TAKEN.swap(true, Ordering::AcqRel) {{\n            \
                     return None;\n        \
                 }}\n        \
                 Some(Board {{\n            \
                     rcc: Rcc {{ _private: () }},"
    );
    for port in &plan.ports {
        let _ = writeln!(
            w,
            "            {}: {} {{ _state: PhantomData }},",
            port_field(*port),
            port_type(*port)
        );
    }
    for name in plan.peripherals.keys() {
        let _ = writeln!(
            w,
            "            {}: {} {{ _state: PhantomData }},",
            name.to_ascii_lowercase(),
            type_name(name)
        );
    }
    for n in plan.timers.keys() {
        let _ = writeln!(w, "            tim{n}: Tim{n} {{ _state: PhantomData }},");
    }
    let _ = writeln!(w, "        }})\n    }}\n}}");
    s
}

/// One timer's typestate and its start (`AET-WO-4300a` §1.3): the solved
/// values, every register written in full and none read, in the order `PSC`,
/// `ARR`, each used channel's `CCRk`, `CCMR1`, `CCMR2`, `CCER`, `BDTR` (only
/// on a timer with a main output enable), `EGR`, then `CR1` last.
fn emit_timer_start(w: &mut String, n: u32, t: &TimerPlan, plan: &Plan, tf: &TimFacts) {
    let mut writes: Vec<(&str, u32, u32)> = vec![("PSC", tf.psc, t.psc), ("ARR", tf.arr, t.arr)];
    let mut ccmr = [0u32; 2];
    let mut ccer = 0u32;
    for (&k, (_, ccr)) in &t.channels {
        let i = (k - 1) as usize;
        writes.push((["CCR1", "CCR2", "CCR3", "CCR4"][i], tf.ccr[i], *ccr));
        ccmr[i / 2] |= (PWM_MODE_1 << tf.ocm[i]) | (1 << tf.ocpe[i]);
        ccer |= 1 << tf.cce[i];
    }
    writes.push(("CCMR1", tf.ccmr[0], ccmr[0]));
    writes.push(("CCMR2", tf.ccmr[1], ccmr[1]));
    writes.push(("CCER", tf.ccer, ccer));
    if motor::has_main_output_enable(n) {
        writes.push(("BDTR", tf.bdtr, 1 << tf.moe));
    }
    writes.push(("EGR", tf.egr, 1 << tf.ug));
    writes.push(("CR1", tf.cr1, (1 << tf.arpe) | (1 << tf.cen)));
    let params: Vec<String> = t
        .channels
        .iter()
        .map(|(k, (pin, _))| format!("_ch{k}: {pin}<{}>", pin_state(&plan.pins[pin])))
        .collect();
    let _ = writeln!(
        w,
        "/// `TIM{n}`. Its outputs start only from `Clocked`, only on its solved channel\n\
         /// pins, each in its solved function, and only with a [`ResetClock`].\n\
         pub struct Tim{n}<S> {{\n    \
             _state: PhantomData<S>,\n\
         }}\n\
         \n\
         impl Tim{n}<Clocked> {{\n    \
             /// PWM mode 1 on each solved channel at its solved values, every register\n    \
             /// written in full and none read, `CR1` last. The result has no method.\n    \
             pub fn start_outputs<R: Registers>(\n        \
                 self,\n        \
                 r: &mut R,\n        \
                 _clock: &ResetClock,\n        \
                 {params},\n    \
             ) -> Tim{n}<Running> {{",
        params = params.join(",\n        "),
    );
    for (name, offset, value) in writes {
        let _ = writeln!(
            w,
            "        r.write(TOKEN, Block::Tim{n}, {offset:#04x}, {value:#010x}); // {name}"
        );
    }
    let _ = writeln!(w, "        Tim{n} {{ _state: PhantomData }}\n    }}\n}}\n");
}

/// [`Rcc`]'s `reset_clock`: the four registers read, and each one's fields
/// held at 0 as one mask.
fn emit_reset_clock(w: &mut String, clock: &ClockFacts) {
    let reads: Vec<String> = clock
        .iter()
        .map(|(register, offset, _)| {
            format!(
                "        let {} = r.read(TOKEN, Block::Rcc, {offset:#05x});",
                register.to_ascii_lowercase()
            )
        })
        .collect();
    let tests: Vec<String> = clock
        .iter()
        .map(|(register, _, fields)| {
            let mask = fields.iter().fold(0u32, |m, (_, pos, width)| {
                m | (((1u32 << width) - 1) << pos)
            });
            format!("{} & {mask:#010x} == 0", register.to_ascii_lowercase())
        })
        .collect();
    let named: Vec<String> = clock
        .iter()
        .map(|(register, _, fields)| {
            let names: Vec<String> = fields.iter().map(|(f, _, _)| format!("`{f}`")).collect();
            format!("    /// - `RCC_{register}`: {}.", names.join(", "))
        })
        .collect();
    let _ = writeln!(
        w,
        "\n    /// A [`ResetClock`], when each of these fields holds 0, its reset value\n    \
         /// (RM0433 Rev 9 §8.7.2, §8.7.7–8.7.9). Reads the four registers and writes nothing.\n\
         {named}\n    \
         pub fn reset_clock<R: Registers>(&self, r: &mut R) -> Result<ResetClock, NotResetClock> {{\n\
         {reads}\n        \
             if {tests} {{\n            \
                 Ok(ResetClock {{ _private: () }})\n        \
             }} else {{\n            \
                 Err(NotResetClock)\n        \
             }}\n    \
         }}",
        named = named.join("\n"),
        reads = reads.join("\n"),
        tests = tests.join(" && "),
    );
}

/// The oracle's writes of one timer's start, in order: (register, offset,
/// value), from the plan and the header's positions.
///
/// belt-and-suspenders: this restates `emit_timer_start`'s values and order on
/// purpose. The oracle must not read back what the crate under proof does
/// (`INV-07`), so a slip in one copy is caught by the other.
fn timer_oracle(n: u32, t: &TimerPlan, tf: &TimFacts) -> Vec<(String, u32, u32)> {
    let mut out = vec![
        ("PSC".to_owned(), tf.psc, t.psc),
        ("ARR".to_owned(), tf.arr, t.arr),
    ];
    for (k, (_, ccr)) in &t.channels {
        out.push((format!("CCR{k}"), tf.ccr[(k - 1) as usize], *ccr));
    }
    for (m, channels) in [(1u32, [1u32, 2]), (2, [3, 4])] {
        let mut value = 0;
        for k in channels.into_iter().filter(|k| t.channels.contains_key(k)) {
            let i = (k - 1) as usize;
            value |= PWM_MODE_1 << tf.ocm[i];
            value |= 1 << tf.ocpe[i];
        }
        out.push((format!("CCMR{m}"), tf.ccmr[(m - 1) as usize], value));
    }
    let ccer = t
        .channels
        .keys()
        .fold(0, |v, k| v | (1 << tf.cce[(k - 1) as usize]));
    out.push(("CCER".to_owned(), tf.ccer, ccer));
    if motor::has_main_output_enable(n) {
        out.push(("BDTR".to_owned(), tf.bdtr, 1 << tf.moe));
    }
    out.push(("EGR".to_owned(), tf.egr, 1 << tf.ug));
    out.push(("CR1".to_owned(), tf.cr1, (1 << tf.cen) | (1 << tf.arpe)));
    out
}

/// A field's mask, as the oracle computes it.
fn oracle_mask(width: u32, pos: u32) -> u32 {
    ((1u32 << width) - 1) << pos
}

/// The oracle's expectation for one GPIO register: (mask, value).
///
/// belt-and-suspenders: this restates the mask arithmetic of `emit_lib`'s
/// field writes on purpose. The oracle must not read back what the crate
/// under proof does (`INV-07`), so a slip in one copy is caught by the other.
fn expect_field(acc: &mut BTreeMap<u32, (u32, u32)>, reg: u32, pos: u32, width: u32, value: u32) {
    let mask = oracle_mask(width, pos);
    let e = acc.entry(reg).or_insert((0, 0));
    e.0 |= mask;
    e.1 |= value << pos;
}

/// A slice of the whole-schedule harness (`AET-WO-3303` §1.2): the pins,
/// ports, peripherals and timers whose actions it schedules. The whole
/// harness's scope is everything, and its text is the one emitted before the
/// split; a part's scope is one unit's ([`parts`]).
struct Scope {
    /// `None` for the whole harness; the unit's name for a part: `i2c1`,
    /// `spi1`, `tim1`, or `gpio`.
    part: Option<String>,
    pins: BTreeSet<String>,
    ports: BTreeSet<char>,
    peripherals: BTreeSet<String>,
    timers: BTreeSet<u32>,
}

/// One action of the schedule: one `match` arm of the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Port(char),
    Clock(String),
    Pin(String),
    Start(String),
    ResetClock,
    TimerClock(u32),
    TimerStart(u32),
}

impl Action {
    fn describe(&self) -> String {
        match self {
            Self::Port(p) => format!("the GPIO{p} enable"),
            Self::Clock(name) => format!("the {name} clock enable"),
            Self::Pin(pin) => format!("the {pin} configure"),
            Self::Start(name) => format!("the {name} start"),
            Self::ResetClock => "the reset-clock read".to_owned(),
            Self::TimerClock(n) => format!("the TIM{n} clock enable"),
            Self::TimerStart(n) => format!("the TIM{n} start"),
        }
    }
}

/// The bits an action may change, by (`Block` variant, register offset).
type Footprint = BTreeMap<(String, u32), u32>;

impl Scope {
    fn whole(plan: &Plan) -> Self {
        Self {
            part: None,
            pins: plan.pins.keys().cloned().collect(),
            ports: plan.ports.clone(),
            peripherals: plan.peripherals.keys().cloned().collect(),
            timers: plan.timers.keys().copied().collect(),
        }
    }

    fn unit(plan: &Plan, name: String, pins: BTreeSet<String>) -> Self {
        Self {
            part: Some(name),
            ports: pins.iter().map(|p| plan.pins[p].port).collect(),
            pins,
            peripherals: BTreeSet::new(),
            timers: BTreeSet::new(),
        }
    }

    /// `_i2c1` for a part, empty for the whole harness.
    fn suffix(&self) -> String {
        self.part
            .as_ref()
            .map(|p| format!("_{p}"))
            .unwrap_or_default()
    }

    /// `I2c1` for a part, empty for the whole harness.
    fn type_suffix(&self) -> String {
        self.part
            .as_deref()
            .map(|p| type_name(&p.to_ascii_uppercase()))
            .unwrap_or_default()
    }

    /// Its actions, in the harness's arm order.
    fn actions(&self, plan: &Plan) -> Vec<Action> {
        let mut out: Vec<Action> = self.ports.iter().map(|p| Action::Port(*p)).collect();
        let peripherals = || {
            plan.peripherals
                .keys()
                .filter(|n| self.peripherals.contains(*n))
        };
        out.extend(peripherals().map(|n| Action::Clock(n.clone())));
        out.extend(
            plan.pins
                .keys()
                .filter(|p| self.pins.contains(*p))
                .map(|p| Action::Pin(p.clone())),
        );
        out.extend(peripherals().map(|n| Action::Start(n.clone())));
        if !self.timers.is_empty() {
            out.push(Action::ResetClock);
        }
        out.extend(self.timers.iter().map(|n| Action::TimerClock(*n)));
        out.extend(self.timers.iter().map(|n| Action::TimerStart(*n)));
        out
    }
}

/// The parts a board with motor outputs proves its schedule in
/// (`AET-WO-3303` §1.2): one per I2C or SPI peripheral, holding its pins and
/// its chip selects; one per timer, holding its channel pins; and one for the
/// GPIO outputs no unit owns, if there are any. Each part also schedules the
/// enable of every port its pins are on, and a timer's the reset-clock read.
fn parts(plan: &Plan) -> Vec<Scope> {
    let mut out = Vec::new();
    let mut owned: BTreeSet<String> = BTreeSet::new();
    for (name, p) in &plan.peripherals {
        let mut pins: BTreeSet<String> = p.pins.values().cloned().collect();
        pins.extend(
            plan.selects
                .get(name)
                .into_iter()
                .flatten()
                .map(|s| s.pin.clone()),
        );
        owned.extend(pins.iter().cloned());
        let mut part = Scope::unit(plan, name.to_ascii_lowercase(), pins);
        part.peripherals.insert(name.clone());
        out.push(part);
    }
    for (n, t) in &plan.timers {
        let pins: BTreeSet<String> = t.channels.values().map(|(p, _)| p.clone()).collect();
        owned.extend(pins.iter().cloned());
        let mut part = Scope::unit(plan, format!("tim{n}"), pins);
        part.timers.insert(*n);
        out.push(part);
    }
    let rest: BTreeSet<String> = plan
        .pins
        .keys()
        .filter(|p| !owned.contains(*p))
        .cloned()
        .collect();
    if !rest.is_empty() {
        out.push(Scope::unit(plan, "gpio".to_owned(), rest));
    }
    out
}

/// The bits `action` may change: exactly the bits the whole harness's
/// final-state oracle lets differ from the starting image, each given to the
/// action that sets it. Computed from the plan and the register facts, as the
/// oracle is, never from `emit_lib`'s writes (`INV-07`).
fn footprint(plan: &Plan, facts: &Facts, action: &Action) -> Footprint {
    let g = &facts.gpio;
    let [mw, tw, sw, pw, aw] = g.widths;
    let mut fp = Footprint::new();
    let mut add = |variant: String, offset: u32, mask: u32| {
        *fp.entry((variant, offset)).or_default() |= mask;
    };
    let gate = |gate: &Gate| ("Rcc".to_owned(), gate.offset, 1u32 << gate.bit);
    // An output's level and its `BSRR` set and reset bits.
    let level = |add: &mut dyn FnMut(String, u32, u32), pin: &str| {
        let t = port_type(plan.pins[pin].port);
        let pf = &facts.pins[pin];
        add(t.clone(), g.odr, 1u32 << pf.bs);
        add(t, g.bsrr, (1u32 << pf.bs) | (1u32 << pf.br));
    };
    match action {
        Action::Port(p) => {
            let (v, o, m) = gate(&facts.ports[p].gate);
            add(v, o, m);
        }
        Action::Clock(name) => {
            let (v, o, m) = gate(&facts.peripherals[name].gate);
            add(v, o, m);
        }
        Action::TimerClock(n) => {
            let (v, o, m) = gate(&facts.timers[n].gate);
            add(v, o, m);
        }
        Action::Pin(pin) => {
            let p = &plan.pins[pin];
            let pf = &facts.pins[pin];
            let t = port_type(p.port);
            add(t.clone(), g.moder, oracle_mask(mw, pf.moder));
            add(t.clone(), g.otyper, oracle_mask(tw, pf.otyper));
            add(t.clone(), g.ospeedr, oracle_mask(sw, pf.ospeedr));
            add(t.clone(), g.pupdr, oracle_mask(pw, pf.pupdr));
            if p.role == Role::Output {
                level(&mut add, pin);
            } else {
                add(t, g.afr[pf.afr_index as usize], oracle_mask(aw, pf.afr));
            }
        }
        Action::Start(name) => {
            let pf = &facts.peripherals[name];
            let t = type_name(name);
            add(t.clone(), pf.cr1, 1u32 << pf.enable);
            match &pf.spi {
                None => {
                    add(t.clone(), pf.config, !0);
                    add(t, pf.txdr, !0);
                }
                Some(spi) => {
                    add(t.clone(), spi.cfg1, oracle_mask(spi.dsize_width, spi.dsize));
                    add(t, pf.config, 1u32 << pf.master.unwrap_or_default());
                    // It drives its own chip selects.
                    for s in plan.selects.get(name).into_iter().flatten() {
                        level(&mut add, &s.pin);
                    }
                }
            }
        }
        Action::ResetClock => {}
        Action::TimerStart(n) => {
            if let Some(tf) = &facts.tim {
                for (_, offset, _) in timer_oracle(*n, &plan.timers[n], tf) {
                    add(format!("Tim{n}"), offset, !0);
                }
            }
        }
    }
    fp
}

/// Refuses a split whose conjunction would not be the whole claim
/// (`AET-WO-3303` §1.2): an action of the whole schedule no part schedules,
/// or two owners whose footprints share a bit. The owners are each port's
/// enable, which every part on that port schedules, and each part's own
/// actions.
fn check_parts(plan: &Plan, facts: &Facts, parts: &[Scope]) -> Result<(), BoardError> {
    let scheduled: Vec<Vec<Action>> = parts.iter().map(|p| p.actions(plan)).collect();
    for action in Scope::whole(plan).actions(plan) {
        if !scheduled.iter().any(|acts| acts.contains(&action)) {
            return Err(BoardError::UnscheduledAction {
                action: action.describe(),
            });
        }
    }
    let mut owners: Vec<(String, Footprint)> = plan
        .ports
        .iter()
        .map(|p| {
            let a = Action::Port(*p);
            (a.describe(), footprint(plan, facts, &a))
        })
        .collect();
    for (part, acts) in parts.iter().zip(&scheduled) {
        let mut own = Footprint::new();
        for a in acts.iter().filter(|a| !matches!(a, Action::Port(_))) {
            for (key, mask) in footprint(plan, facts, a) {
                *own.entry(key).or_default() |= mask;
            }
        }
        owners.push((
            format!("the {} part", part.part.as_deref().unwrap_or_default()),
            own,
        ));
    }
    disjoint(&owners)
}

/// Refuses two owners whose footprints share a bit.
fn disjoint(owners: &[(String, Footprint)]) -> Result<(), BoardError> {
    for (i, (first, a)) in owners.iter().enumerate() {
        for (second, b) in &owners[i + 1..] {
            for ((variant, offset), mask) in a {
                let shared = b.get(&(variant.clone(), *offset)).copied().unwrap_or(0) & mask;
                if shared != 0 {
                    return Err(BoardError::FootprintOverlap {
                        first: first.clone(),
                        second: second.clone(),
                        register: format!("{variant} at {offset:#x}, bits {shared:#010x}"),
                    });
                }
            }
        }
    }
    Ok(())
}

fn emit_proofs(plan: &Plan, facts: &Facts, speeds: &Speeds) -> Result<String, BoardError> {
    let mut s = header(
        plan,
        "The Kani harness: the specification's register rules over an image this\n\
         //! harness owns. Every number below is computed by the generator from the\n\
         //! solved topology and the register source, not read from `lib.rs`.",
    );
    let w = &mut s;
    let _ = writeln!(
        w,
        "\n\
         use crate::board::sealed;\n\
         use crate::*;\n"
    );
    // A board with motor outputs proves its schedule in parts (`AET-WO-3303`
    // §1.2): the whole harness does not finish on one (`AET-WO-4300a` Q5).
    // Without a timer, the whole harness is emitted, byte for byte as before.
    // The whole image stays: the targeted harnesses set up through it.
    if plan.timers.is_empty() {
        emit_schedule(w, plan, facts, speeds, &Scope::whole(plan), false);
    } else {
        let parts = parts(plan);
        check_parts(plan, facts, &parts)?;
        emit_schedule(w, plan, facts, speeds, &Scope::whole(plan), true);
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                w.push('\n');
            }
            emit_schedule(w, plan, facts, speeds, part, false);
        }
    }
    emit_transfer_proofs(w, plan, facts, speeds);
    emit_spi_transfer_proofs(w, plan, facts);
    emit_timer_proofs(w, plan, facts);
    Ok(s)
}

/// The whole-schedule harness for `scope`: its register image and the
/// image's write-time rules, its final-state oracle, and its scheduler. For a
/// part, also the rely–guarantee that makes the parts' conjunction the whole
/// claim (`AET-WO-3303` §1.2):
/// - each write asserts it changes only bits of the running action's
///   [`footprint`] (a stored `BSRR`: writes only such bits);
/// - an access to a block outside the part fails;
/// - before each step every bit another part's action may change is
///   arbitrary (the rely), so the part holds whatever those actions did.
///
/// With `image_only`, only the image and its rules.
fn emit_schedule(
    w: &mut String,
    plan: &Plan,
    facts: &Facts,
    speeds: &Speeds,
    scope: &Scope,
    image_only: bool,
) {
    let g = &facts.gpio;
    let part = scope.part.as_deref();
    let ty = scope.type_suffix();
    let sfx = scope.suffix();
    // Blocks: (Block variant, image field, words, gate).
    let mut blocks: Vec<(String, String, u32, Option<&Gate>)> =
        vec![("Rcc".into(), "rcc".into(), facts.rcc_words, None)];
    for port in &scope.ports {
        blocks.push((
            port_type(*port),
            port_field(*port),
            g.words,
            Some(&facts.ports[port].gate),
        ));
    }
    for (name, pf) in facts
        .peripherals
        .iter()
        .filter(|(n, _)| scope.peripherals.contains(*n))
    {
        blocks.push((
            type_name(name),
            name.to_ascii_lowercase(),
            pf.words,
            Some(&pf.gate),
        ));
    }
    for (n, tf) in facts
        .timers
        .iter()
        .filter(|(n, _)| scope.timers.contains(n))
    {
        blocks.push((
            format!("Tim{n}"),
            format!("tim{n}"),
            tf.words,
            Some(&tf.gate),
        ));
    }
    // A part's image holds only its own blocks; any other is outside it.
    let partial = blocks.len() < 1 + plan.ports.len() + plan.peripherals.len() + plan.timers.len();
    let acts = scope.actions(plan);
    let actions = acts.len();
    let unwind = blocks
        .iter()
        .map(|b| b.2 as usize)
        .max()
        .unwrap_or(0)
        .max(actions)
        + 1;
    // The rely, by (variant, offset): every bit of the image that an action
    // outside the part may change. A stored `BSRR` is arbitrary whole.
    let mut rely: Footprint = Footprint::new();
    if part.is_some() {
        for a in Scope::whole(plan)
            .actions(plan)
            .iter()
            .filter(|a| !acts.contains(a))
        {
            for ((variant, offset), mask) in footprint(plan, facts, a) {
                if blocks.iter().any(|b| b.0 == variant) {
                    *rely.entry((variant, offset)).or_default() |= mask;
                }
            }
        }
        for port in &scope.ports {
            if let Some(m) = rely.get_mut(&(port_type(*port), g.bsrr)) {
                *m = !0;
            }
        }
    }

    let _ = writeln!(
        w,
        "/// The harness-owned register image, and the oracle for the ordering rules.\n\
         #[derive(Clone, Copy)]\n\
         struct Image{ty} {{"
    );
    for (_, field, words, _) in &blocks {
        let _ = writeln!(w, "    {field}: [u32; {words}],");
    }
    if part.is_some() {
        let _ = writeln!(
            w,
            "    /// The running action, its `match` arm's number: whose footprint a\n    \
             /// write may change.\n    \
             running: u8,"
        );
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         impl Image{ty} {{\n    \
             fn slot(&mut self, block: Block, offset: usize) -> &mut u32 {{\n        \
                 let words: &mut [u32] = match block {{"
    );
    for (variant, field, _, _) in &blocks {
        let _ = writeln!(w, "            Block::{variant} => &mut self.{field},");
    }
    if partial {
        let _ = writeln!(
            w,
            "            _ => unreachable!(\"`clocked` refuses a block outside the part\"),"
        );
    }
    let _ = writeln!(
        w,
        "        }};\n        \
             kani::assert(\n            \
                 offset % 4 == 0 && offset / 4 < words.len(),\n            \
                 \"register access past the end of its block\",\n        \
             );\n        \
             &mut words[offset / 4]\n    \
         }}\n\
         \n    \
         /// A gated block may not be read or written.\n    \
         fn clocked(&self, block: Block) {{\n        \
             match block {{\n            \
                 Block::Rcc => {{}}"
    );
    for (variant, _, _, gate) in &blocks {
        let Some(gate) = gate else { continue };
        let _ = writeln!(
            w,
            "            Block::{variant} => kani::assert(\n                \
                 self.rcc[{word}] & {mask:#010x} != 0,\n                \
                 \"{variant} accessed before RCC_{reg} bit {bit} is set\",\n            \
             ),",
            word = gate.offset / 4,
            mask = 1u32 << gate.bit,
            reg = gate.register,
            bit = gate.bit,
        );
    }
    if let (true, Some(part)) = (partial, part) {
        let _ = writeln!(
            w,
            "            _ => {{\n                \
                 kani::assert(false, \"a block outside the {part} part was accessed\");\n                \
                 kani::assume(false);\n            \
             }}"
        );
    }
    let _ = writeln!(w, "        }}\n    }}");
    if let Some(part) = part {
        let _ = writeln!(
            w,
            "\n    \
             /// The bits the running action may change in one register: the\n    \
             /// generator's footprint of that action, from the oracle's own facts.\n    \
             fn footprint(&self, block: Block, offset: usize) -> u32 {{\n        \
                 match (self.running, block, offset) {{"
        );
        for (a, act) in acts.iter().enumerate() {
            for ((variant, offset), mask) in footprint(plan, facts, act) {
                let _ = writeln!(
                    w,
                    "            ({a}, Block::{variant}, {offset:#x}) => {mask:#010x},"
                );
            }
        }
        let _ = writeln!(
            w,
            "            _ => 0,\n        \
                 }}\n    \
             }}\n\
             \n    \
             /// The rely: every bit an action outside the {part} part may change\n    \
             /// is arbitrary. A stored `BSRR` is the last value written, which any\n    \
             /// writer of its port replaces whole, so it is arbitrary whole.\n    \
             fn rely(&mut self) {{"
        );
        for ((variant, offset), mask) in &rely {
            let field = &blocks
                .iter()
                .find(|b| &b.0 == variant)
                .expect("in the image")
                .1;
            let i = offset / 4;
            if *mask == !0 {
                let _ = writeln!(w, "        self.{field}[{i}] = kani::any();");
            } else {
                let _ = writeln!(
                    w,
                    "        self.{field}[{i}] = (self.{field}[{i}] & !{mask:#010x}) | (kani::any::<u32>() & {mask:#010x});"
                );
            }
        }
        let _ = writeln!(w, "    }}");
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         impl sealed::Sealed for Image{ty} {{}}\n\
         \n\
         impl Registers for Image{ty} {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 self.clocked(block);"
    );
    // Nothing reads a timer: its start writes only, and after `CEN` nothing
    // touches it (`AET-WO-4300a` §1.3).
    for n in &scope.timers {
        let _ = writeln!(
            w,
            "        kani::assert(block != Block::Tim{n}, \"TIM{n} was read\");"
        );
    }
    let _ = writeln!(
        w,
        "        *self.slot(block, offset)\n    \
         }}\n    \
         fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
             self.clocked(block);"
    );
    for (name, pf) in facts
        .peripherals
        .iter()
        .filter(|(n, _)| scope.peripherals.contains(*n))
    {
        let t = type_name(name);
        let field = name.to_ascii_lowercase();
        let what = match plan.peripherals[name].kind {
            Kind::I2c => "TIMINGR",
            Kind::Spi => "CFG2",
        };
        let _ = writeln!(
            w,
            "        if block == Block::{t} && offset == {cfg:#04x} {{\n            \
                 kani::assert(\n                \
                     self.{field}[{cr1}] & {en:#010x} == 0,\n                \
                     \"{t} {what} written while the peripheral is enabled\",\n            \
                 );\n        \
             }}",
            cfg = pf.config,
            cr1 = pf.cr1 / 4,
            en = 1u32 << pf.enable,
        );
        if let Some(spi) = &pf.spi {
            let _ = writeln!(
                w,
                "        if block == Block::{t} && offset == {cfg1:#04x} {{\n            \
                     kani::assert(\n                \
                         self.{field}[{cr1}] & {en:#010x} == 0,\n                \
                         \"{t} CFG1 written while the peripheral is enabled\",\n            \
                     );\n        \
                 }}",
                cfg1 = spi.cfg1,
                cr1 = pf.cr1 / 4,
                en = 1u32 << pf.enable,
            );
        }
    }
    // GPIO write-time rules (adversarial E6, E7): a BSRR write may touch only a
    // solved output's set or reset bit; a MODER write may move only solved pins,
    // only to their final mode, and only once every other field of that pin
    // already holds its final value (an output's level already high). In a
    // part, only its own pins' rules: the footprint assertion refuses a write
    // to any other pin's bits.
    let [mw, tw, sw, pw, aw] = g.widths;
    for port in &scope.ports {
        let t = port_type(*port);
        let field = port_field(*port);
        let on_port: Vec<(&String, &PinPlan)> =
            plan.pins.iter().filter(|(_, p)| p.port == *port).collect();
        let own: Vec<(&String, &PinPlan)> = on_port
            .iter()
            .filter(|(n, _)| scope.pins.contains(*n))
            .copied()
            .collect();
        let allowed: Vec<String> = own
            .iter()
            .filter(|(_, p)| p.role == Role::Output)
            .flat_map(|(n, _)| {
                let pf = &facts.pins[*n];
                [1u32 << pf.bs, 1u32 << pf.br]
            })
            .map(|v| format!("value == {v:#010x}"))
            .collect();
        let rule = if allowed.is_empty() {
            "false".to_owned()
        } else {
            allowed.join(" || ")
        };
        let owner = match part {
            None => "no solved output owns".to_owned(),
            Some(part) => format!("no solved output of the {part} part owns"),
        };
        let level = match part {
            None => format!(
                "self.{field}[{oi}] = (self.{field}[{oi}] & !(value >> 16)) | (value & 0xffff);",
                oi = g.odr / 4
            ),
            Some(_) => format!(
                "let level = (self.{field}[{oi}] & !(value >> 16)) | (value & 0xffff);\n            \
                 kani::assert(\n                \
                     (level ^ self.{field}[{oi}]) & !self.footprint(block, {odr:#04x}) == 0,\n                \
                     \"{t} ODR: a level changed outside the running action's footprint\",\n            \
                 );\n            \
                 self.{field}[{oi}] = level;",
                oi = g.odr / 4,
                odr = g.odr,
            ),
        };
        let _ = writeln!(
            w,
            "        if block == Block::{t} && offset == {bsrr:#04x} {{\n            \
                 kani::assert({rule}, \"{t} BSRR write touches a pin {owner}\");\n            \
                 // The level it leaves: a set bit wins over a reset bit (RM0433 §11.4.7).\n            \
                 {level}\n        \
             }}\n        \
             if block == Block::{t} && offset == {moder:#04x} {{\n            \
                 let old = self.{field}[{mi}];",
            bsrr = g.bsrr,
            moder = g.moder,
            mi = g.moder / 4,
        );
        let solved_mask = on_port.iter().fold(0u32, |m, (pin, _)| {
            m | oracle_mask(mw, facts.pins[*pin].moder)
        });
        for (pin, p) in &own {
            let pf = &facts.pins[*pin];
            let (mode, otype, ospeed, pupd) = p.policy();
            let mmask = oracle_mask(mw, pf.moder);
            let mut ready = vec![
                format!(
                    "self.{field}[{}] & {:#010x} == {:#010x}",
                    g.otyper / 4,
                    oracle_mask(tw, pf.otyper),
                    otype << pf.otyper
                ),
                format!(
                    "self.{field}[{}] & {:#010x} == {:#010x}",
                    g.ospeedr / 4,
                    oracle_mask(sw, pf.ospeedr),
                    ospeed << pf.ospeedr
                ),
                format!(
                    "self.{field}[{}] & {:#010x} == {:#010x}",
                    g.pupdr / 4,
                    oracle_mask(pw, pf.pupdr),
                    pupd << pf.pupdr
                ),
            ];
            if p.role == Role::Output {
                ready.push(format!(
                    "self.{field}[{}] == {:#010x}",
                    g.bsrr / 4,
                    1u32 << pf.bs
                ));
            } else {
                let afr = g.afr[pf.afr_index as usize] / 4;
                ready.push(format!(
                    "self.{field}[{afr}] & {:#010x} == {:#010x}",
                    oracle_mask(aw, pf.afr),
                    p.af << pf.afr
                ));
            }
            let _ = writeln!(
                w,
                "            if value & {mmask:#010x} != old & {mmask:#010x} {{\n                \
                     kani::assert(value & {mmask:#010x} == {fin:#010x}, \"{pin}'s mode moved to one its role never takes\");\n                \
                     kani::assert(\n                    \
                         {ready},\n                    \
                         \"{pin} entered its mode before its other fields were final\",\n                \
                     );\n            \
                 }}",
                fin = mode << pf.moder,
                ready = ready.join("\n                        && "),
            );
        }
        let _ = writeln!(
            w,
            "            kani::assert(\n                \
                 value & !{solved_mask:#010x} == old & !{solved_mask:#010x},\n                \
                 \"a {t} MODER write moved a pin the topology does not use\",\n            \
             );\n        \
             }}"
        );
    }
    // The guarantee: a write changes only the running action's bits. A stored
    // `BSRR` is replaced whole, so there the bits written are judged.
    if part.is_some() {
        let bsrr: Vec<String> = scope
            .ports
            .iter()
            .map(|p| format!("(Block::{}, {:#04x})", port_type(*p), g.bsrr))
            .collect();
        let _ = writeln!(
            w,
            "        let changed = if matches!((block, offset), {}) {{\n            \
                 value\n        \
             }} else {{\n            \
                 *self.slot(block, offset) ^ value\n        \
             }};\n        \
             kani::assert(\n            \
                 changed & !self.footprint(block, offset) == 0,\n            \
                 \"a write changed a bit outside the running action's footprint\",\n        \
             );",
            bsrr.join(" | ")
        );
    }
    let _ = writeln!(
        w,
        "        *self.slot(block, offset) = value;\n    \
         }}\n    \
         // A byte access models the register as its word: the low byte read, the\n    \
         // byte written zero-extended, under every rule above.\n    \
         fn read8(&mut self, token: sealed::Token, block: Block, offset: usize) -> u8 {{\n        \
             self.read(token, block, offset).to_le_bytes()[0]\n    \
         }}\n    \
         fn write8(&mut self, token: sealed::Token, block: Block, offset: usize, value: u8) {{\n        \
             self.write(token, block, offset, u32::from(value));\n    \
         }}\n\
         }}\n"
    );
    if image_only {
        return;
    }

    // The final-state oracle.
    let i2c_in_scope = || {
        plan.peripherals
            .iter()
            .filter(|(n, p)| p.kind == Kind::I2c && scope.peripherals.contains(*n))
    };
    let timingr_params: Vec<String> = i2c_in_scope()
        .map(|(n, _)| format!(", {}_timingr: u32", n.to_ascii_lowercase()))
        .collect();
    match part {
        None => {
            let _ = writeln!(
                w,
                "/// The state the specification allows, from any starting image."
            );
        }
        Some(part) => {
            let _ = writeln!(
                w,
                "/// The {part} part's share of the state the specification allows, from\n\
                 /// any starting image: its own fields as solved, and every bit no action\n\
                 /// may change unchanged. Another part's bits are that part's to prove."
            );
        }
    }
    let _ = writeln!(
        w,
        "fn check_final{sfx}(before: &Image{ty}, after: &Image{ty}{}) {{",
        timingr_params.concat()
    );
    for port in &scope.ports {
        let field = port_field(*port);
        let mut expect: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let mut own: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let [mw, tw, sw, pw, aw] = g.widths;
        let mut touched: BTreeSet<u32> = BTreeSet::new();
        for (pin, p) in plan.pins.iter().filter(|(_, p)| p.port == *port) {
            let pf = &facts.pins[pin];
            let (mode, otype, ospeed, pupd) = p.policy();
            let fields = |acc: &mut BTreeMap<u32, (u32, u32)>| {
                expect_field(acc, g.moder, pf.moder, mw, mode);
                expect_field(acc, g.otyper, pf.otyper, tw, otype);
                expect_field(acc, g.ospeedr, pf.ospeedr, sw, ospeed);
                expect_field(acc, g.pupdr, pf.pupdr, pw, pupd);
                if p.role != Role::Output {
                    expect_field(acc, g.afr[pf.afr_index as usize], pf.afr, aw, p.af);
                }
            };
            fields(&mut expect);
            if scope.pins.contains(pin) {
                fields(&mut own);
            }
            if p.role == Role::Output {
                touched.insert(g.bsrr);
            }
        }
        for (reg, (mask, _)) in &expect {
            let i = reg / 4;
            let t = port_type(*port);
            if let Some((own_mask, own_value)) = own.get(reg) {
                let _ = writeln!(
                    w,
                    "    kani::assert(\n        \
                         after.{field}[{i}] & {own_mask:#010x} == {own_value:#010x},\n        \
                         \"{t} word {i}: a solved pin's field is not what the specification fixes\",\n    \
                     );"
                );
            }
            let _ = writeln!(
                w,
                "    kani::assert(\n        \
                     after.{field}[{i}] & !{mask:#010x} == before.{field}[{i}] & !{mask:#010x},\n        \
                     \"{t} word {i}: an unsolved pin's field changed\",\n    \
                 );"
            );
            touched.insert(*reg);
        }
        // The levels: every chip select high once its bus started (F3, and
        // nothing drives one after `init` takes it); a level no solved output
        // owns, unchanged.
        let outputs: Vec<(&String, &PinPlan)> = plan
            .pins
            .iter()
            .filter(|(_, p)| p.port == *port && p.role == Role::Output)
            .collect();
        if !outputs.is_empty() {
            let i = g.odr / 4;
            let t = port_type(*port);
            let mut owned = 0u32;
            for (pin, _) in &outputs {
                let bit = 1u32 << facts.pins[*pin].bs;
                owned |= bit;
                if plan.is_select(pin) && scope.pins.contains(*pin) {
                    let _ = writeln!(
                        w,
                        "    kani::assert(after.{field}[{i}] & {bit:#010x} != 0, \"{pin}, a chip select, is not high once its bus started\");"
                    );
                }
            }
            let _ = writeln!(
                w,
                "    kani::assert(\n        \
                     after.{field}[{i}] & !{owned:#010x} == before.{field}[{i}] & !{owned:#010x},\n        \
                     \"{t} word {i}: a level no solved output owns changed\",\n    \
                 );"
            );
            touched.insert(g.odr);
        }
        for i in 0..g.words {
            if !touched.contains(&(i * 4)) {
                let _ = writeln!(
                    w,
                    "    kani::assert(after.{field}[{i}] == before.{field}[{i}], \"{t} word {i} changed\");",
                    t = port_type(*port),
                );
            }
        }
    }
    // RCC: before plus exactly the gates; in a part, its own gates, and
    // another part's gate bits are the rely's.
    let mut gates: BTreeMap<u32, u32> = BTreeMap::new();
    for gate in facts
        .ports
        .iter()
        .filter(|(p, _)| scope.ports.contains(p))
        .map(|(_, p)| &p.gate)
        .chain(
            facts
                .peripherals
                .iter()
                .filter(|(n, _)| scope.peripherals.contains(*n))
                .map(|(_, p)| &p.gate),
        )
        .chain(
            facts
                .timers
                .iter()
                .filter(|(n, _)| scope.timers.contains(n))
                .map(|(_, t)| &t.gate),
        )
    {
        *gates.entry(gate.offset / 4).or_default() |= 1u32 << gate.bit;
    }
    let _ = writeln!(w, "    let mut rcc = before.rcc;");
    for (word, mask) in &gates {
        let _ = writeln!(w, "    rcc[{word}] |= {mask:#010x};");
    }
    if part.is_none() {
        let _ = writeln!(
            w,
            "    // Word by word: `==` on the arrays is a `memcmp` loop under CBMC.\n    \
             for i in 0..{n} {{\n        \
                 kani::assert(after.rcc[i] == rcc[i], \"RCC is not before plus exactly the solved clock gates\");\n    \
             }}",
            n = facts.rcc_words
        );
    } else {
        let others: Vec<(u32, u32)> = rely
            .iter()
            .filter(|((v, _), _)| v == "Rcc")
            .map(|((_, o), m)| (o / 4, *m))
            .collect();
        let _ = writeln!(
            w,
            "    let {}rely = [0u32; {n}];",
            if others.is_empty() { "" } else { "mut " },
            n = facts.rcc_words
        );
        for (word, mask) in &others {
            let _ = writeln!(w, "    rely[{word}] = {mask:#010x};");
        }
        let _ = writeln!(
            w,
            "    // Word by word: `==` on the arrays is a `memcmp` loop under CBMC.\n    \
             for i in 0..{n} {{\n        \
                 kani::assert(\n            \
                     after.rcc[i] & !rely[i] == rcc[i] & !rely[i],\n            \
                     \"RCC is not before plus exactly the solved clock gates\",\n        \
                 );\n    \
             }}",
            n = facts.rcc_words
        );
    }
    for (name, pf) in facts
        .peripherals
        .iter()
        .filter(|(n, _)| scope.peripherals.contains(*n))
    {
        let t = type_name(name);
        let field = name.to_ascii_lowercase();
        let cr1 = pf.cr1 / 4;
        // An I2C peripheral is left enabled; an SPI one disabled, as every
        // transfer leaves it (`AET-WO-4200`).
        let (enabled, state) = match plan.peripherals[name].kind {
            Kind::I2c => ("!=", "is not enabled"),
            Kind::Spi => ("==", "is left enabled"),
        };
        let _ = writeln!(
            w,
            "    kani::assert(after.{field}[{cr1}] & {en:#010x} {enabled} 0, \"{t} {state}\");\n    \
             kani::assert(\n        \
                 after.{field}[{cr1}] & !{en:#010x} == before.{field}[{cr1}] & !{en:#010x},\n        \
                 \"{t} CR1 changed beyond its enable bit\",\n    \
             );",
            en = 1u32 << pf.enable
        );
        let written = match &pf.spi {
            None => vec![pf.cr1, pf.config, pf.txdr],
            Some(spi) => {
                let mask = oracle_mask(spi.dsize_width, spi.dsize);
                let _ = writeln!(
                    w,
                    "    kani::assert(after.{field}[{c}] & {mask:#010x} == {v:#010x}, \"{t} frames are not 8 bits\");\n    \
                     kani::assert(\n        \
                         after.{field}[{c}] & !{mask:#010x} == before.{field}[{c}] & !{mask:#010x},\n        \
                         \"{t} CFG1 changed beyond DSIZE\",\n    \
                     );",
                    c = spi.cfg1 / 4,
                    v = SPI_DSIZE << spi.dsize,
                );
                vec![pf.cr1, spi.cfg1, pf.config]
            }
        };
        match plan.peripherals[name].kind {
            Kind::I2c => {
                let _ = writeln!(
                    w,
                    "    kani::assert(\n        \
                         after.{field}[{c}] == {field}_timingr,\n        \
                         \"{t} TIMINGR is not the chosen speed's\",\n    \
                     );",
                    c = pf.config / 4
                );
            }
            Kind::Spi => {
                let master = 1u32 << pf.master.unwrap_or_default();
                let _ = writeln!(
                    w,
                    "    kani::assert(after.{field}[{c}] & {master:#010x} != 0, \"{t} is not a master\");\n    \
                     kani::assert(\n        \
                         after.{field}[{c}] & !{master:#010x} == before.{field}[{c}] & !{master:#010x},\n        \
                         \"{t} CFG2 changed beyond MASTER\",\n    \
                     );",
                    c = pf.config / 4
                );
            }
        }
        for i in 0..pf.words {
            if !written.contains(&(i * 4)) {
                let _ = writeln!(
                    w,
                    "    kani::assert(after.{field}[{i}] == before.{field}[{i}], \"{t} word {i} changed\");"
                );
            }
        }
    }
    // Each timer: every register its start writes holds the solved value, and
    // every other word is unchanged.
    if let Some(tf) = &facts.tim {
        for (n, t) in plan.timers.iter().filter(|(n, _)| scope.timers.contains(n)) {
            let writes = timer_oracle(*n, t, tf);
            for (name, offset, value) in &writes {
                let _ = writeln!(
                    w,
                    "    kani::assert(after.tim{n}[{i}] == {value:#010x}, \"TIM{n} {name} is not the solved value\");",
                    i = offset / 4
                );
            }
            for i in 0..facts.timers[n].words {
                if !writes.iter().any(|(_, offset, _)| *offset == i * 4) {
                    let _ = writeln!(
                        w,
                        "    kani::assert(after.tim{n}[{i}] == before.tim{n}[{i}], \"TIM{n} word {i} changed\");"
                    );
                }
            }
        }
    }
    let _ = writeln!(w, "}}\n");

    // The scheduler.
    match part {
        None => {
            let _ = writeln!(
                w,
                "/// Every order the typestate API admits: at each of {actions} steps, any\n\
                 /// action whose inputs exist. The `assume` admits only those orders; the\n\
                 /// types forbid the rest at compile time. Each I2C peripheral writes zero or\n\
                 /// one byte when it starts; each SPI peripheral takes its bus's chip selects;\n\
                 /// and the output pins are driven zero or one time first."
            );
        }
        Some(part) => {
            let _ = writeln!(
                w,
                "/// The {part} part of the whole schedule (`AET-WO-3303` §1.2): every order\n\
                 /// the typestate API admits of its {actions} actions, under the rely that\n\
                 /// any other part's action may have run between any two of them. Each\n\
                 /// write may change only its running action's footprint; the parts'\n\
                 /// footprints are disjoint, so together the parts prove every schedule\n\
                 /// of the whole board."
            );
        }
    }
    let _ = writeln!(
        w,
        "#[kani::proof]\n\
         #[kani::unwind({unwind})]\n\
         fn every_valid_sequence{sfx}() {{\n    \
             let mut img = Image{ty} {{"
    );
    for (_, field, _, _) in &blocks {
        let _ = writeln!(w, "        {field}: kani::any(),");
    }
    if part.is_some() {
        let _ = writeln!(w, "        running: u8::MAX,");
    }
    let _ = writeln!(
        w,
        "    }};\n    \
         let before = img;\n    \
         let board = Board::take().unwrap();\n    \
         let mut rcc = board.rcc;"
    );
    for port in &scope.ports {
        let _ = writeln!(
            w,
            "    let mut {f} = Some(board.{f});",
            f = port_field(*port)
        );
    }
    for name in plan
        .peripherals
        .keys()
        .filter(|n| scope.peripherals.contains(*n))
    {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "    let mut {f} = Some(board.{f});\n    let mut {f}_clocked = None;\n    let mut {f}_done = false;"
        );
    }
    for (name, _) in i2c_in_scope() {
        let _ = writeln!(
            w,
            "    let mut {}_timingr = 0u32;",
            name.to_ascii_lowercase()
        );
    }
    for pin in plan.pins.keys().filter(|p| scope.pins.contains(*p)) {
        let f = pin_field(pin);
        let _ = writeln!(w, "    let mut {f} = None;\n    let mut {f}_done = None;");
    }
    if !scope.timers.is_empty() {
        let _ = writeln!(w, "    let mut clock = None;");
    }
    for n in &scope.timers {
        let _ = writeln!(
            w,
            "    let mut tim{n} = Some(board.tim{n});\n    let mut tim{n}_clocked = None;\n    let mut tim{n}_done = false;"
        );
    }
    let rely_step = if part.is_some() {
        "\n        img.rely();"
    } else {
        ""
    };
    let _ = writeln!(
        w,
        "    for _ in 0..{actions} {{{rely_step}\n        \
             let action: u8 = kani::any();\n        \
             match action {{"
    );
    // In a part, each arm names itself the running action once its inputs
    // exist.
    let running = |w: &mut String, a: usize| {
        if part.is_some() {
            let _ = writeln!(w, "                img.running = {a};");
        }
    };
    let mut a = 0usize;
    for port in &scope.ports {
        debug_assert_eq!(acts[a], Action::Port(*port));
        let f = port_field(*port);
        let pins: Vec<String> = plan
            .pins
            .keys()
            .filter(|n| plan.pins[*n].port == *port && scope.pins.contains(*n))
            .map(|n| pin_field(n))
            .collect();
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                let pins = rcc.enable_{f}(&mut img, {f}.take().unwrap()).split();"
        );
        for p in &pins {
            let _ = writeln!(w, "                {p} = Some(pins.{p});");
        }
        let _ = writeln!(w, "            }}");
        a += 1;
    }
    for name in plan
        .peripherals
        .keys()
        .filter(|n| scope.peripherals.contains(*n))
    {
        debug_assert_eq!(acts[a], Action::Clock(name.clone()));
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                {f}_clocked = Some(rcc.enable_{f}(&mut img, {f}.take().unwrap()));\n            \
             }}"
        );
        a += 1;
    }
    for (pin, p) in plan.pins.iter().filter(|(n, _)| scope.pins.contains(*n)) {
        debug_assert_eq!(acts[a], Action::Pin(pin.clone()));
        let f = pin_field(pin);
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                let {mutable}pin = {f}.take().unwrap().{m}(&mut img);",
            m = method(p),
            mutable = if p.role == Role::Output { "mut " } else { "" },
        );
        if p.role == Role::Output {
            let pf = &facts.pins[pin];
            let port = port_field(p.port);
            let _ = writeln!(
                w,
                "                if kani::any() {{\n                    \
                     if kani::any() {{\n                        \
                         pin.set_low(&mut img);\n                        \
                         kani::assert(img.{port}[{b}] == {br:#010x}, \"{pin} set_low did not reset it\");\n                        \
                         kani::cover!(true, \"{pin} driven low\");\n                    \
                     }} else {{\n                        \
                         pin.set_high(&mut img);\n                        \
                         kani::assert(img.{port}[{b}] == {bs:#010x}, \"{pin} set_high did not set it\");\n                        \
                         kani::cover!(true, \"{pin} driven high\");\n                    \
                     }}\n                \
                 }}",
                b = facts.gpio.bsrr / 4,
                bs = 1u32 << pf.bs,
                br = 1u32 << pf.br,
            );
        }
        let _ = writeln!(w, "                {f}_done = Some(pin);\n            }}");
        a += 1;
    }
    for (name, p) in plan
        .peripherals
        .iter()
        .filter(|(n, _)| scope.peripherals.contains(*n))
    {
        debug_assert_eq!(acts[a], Action::Start(name.clone()));
        let f = name.to_ascii_lowercase();
        let pins: Vec<String> = p.pins.values().map(|n| pin_field(n)).collect();
        let selects: Vec<String> = plan
            .selects
            .get(name)
            .into_iter()
            .flatten()
            .map(|s| pin_field(&s.pin))
            .collect();
        let ready: Vec<String> = pins
            .iter()
            .chain(&selects)
            .map(|n| format!("{n}_done.is_some()"))
            .collect();
        let mut args: Vec<String> = pins
            .iter()
            .map(|n| format!("{n}_done.take().unwrap()"))
            .collect();
        if !selects.is_empty() {
            let fields: Vec<String> = selects
                .iter()
                .map(|n| format!("{n}: {n}_done.take().unwrap()"))
                .collect();
            args.push(format!(
                "{}Selects {{ {} }}",
                type_name(name),
                fields.join(", ")
            ));
        }
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}_clocked.is_some() && {});",
            ready.join(" && ")
        );
        running(w, a);
        let _ = writeln!(w, "                let i = {f}_clocked.take().unwrap();");
        // A byte written to a peripheral must not touch any output's level.
        let bsrr_ports: Vec<String> = scope
            .ports
            .iter()
            .map(|p| format!("img.{}[{}]", port_field(*p), facts.gpio.bsrr / 4))
            .collect();
        let start = |w: &mut String, ty: &str, label: &str, indent: &str| {
            let _ = writeln!(
                w,
                "{indent}let mut bus = i.init::<{ty}>(&mut img, {args});\n\
                 {indent}kani::cover!(true, \"{name} started{label}\");\n\
                 {indent}if kani::any() {{\n\
                 {indent}    let levels = [{levels}];\n\
                 {indent}    bus.write_byte(&mut img, kani::any());\n\
                 {indent}    kani::assert(levels == [{levels}], \"{name} write_byte touched a GPIO BSRR\");\n\
                 {indent}    kani::cover!(true, \"{name} wrote a byte\");\n\
                 {indent}}}",
                args = args.join(", "),
                levels = bsrr_ports.join(", "),
            );
        };
        match p.kind {
            Kind::I2c => {
                let (_, admitted) = &speeds[name];
                let m = name.to_ascii_lowercase();
                let modes: Vec<&str> = admitted.iter().map(|&i| I2C_MODES[i].0).collect();
                let _ = writeln!(w, "                let speed: u8 = kani::any();");
                for (k, mode) in modes.iter().enumerate() {
                    let open = if k == 0 {
                        format!("                if speed == {k} {{")
                    } else if k + 1 == modes.len() {
                        "                } else {".to_owned()
                    } else {
                        format!("                }} else if speed == {k} {{")
                    };
                    let _ = writeln!(w, "{open}");
                    let _ = writeln!(
                        w,
                        "                    {m}_timingr = {:#010x};",
                        I2C_MODES[admitted[k]].2
                    );
                    start(
                        w,
                        &format!("{m}_speed::{mode}, _"),
                        &format!(" at {mode}"),
                        "                    ",
                    );
                }
                if modes.len() > 1 {
                    let _ = writeln!(w, "                }}");
                } else {
                    // One admitted speed: the `if` above has no `else`.
                    let _ = writeln!(
                        w,
                        "                }} else {{\n                    kani::assume(false);\n                }}"
                    );
                }
            }
            Kind::Spi => {
                let _ = writeln!(
                    w,
                    "                let _bus = i.init::<_>(&mut img, {args});\n                \
                     kani::cover!(true, \"{name} started\");",
                    args = args.join(", "),
                );
            }
        }
        let _ = writeln!(w, "                {f}_done = true;\n            }}");
        a += 1;
    }
    // The timers (`AET-WO-4300a`): the reset clock read once, each timer
    // clocked, then started from its own configured pins and the token. The
    // schedule runs on the reset clock; `reset_clock_is_ok_exactly_at_reset`
    // proves the check itself.
    if !scope.timers.is_empty() {
        debug_assert_eq!(acts[a], Action::ResetClock);
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume(clock.is_none());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                let read = rcc.reset_clock(&mut img);\n                \
                 kani::assume(read.is_ok());\n                \
                 clock = read.ok();\n                \
                 kani::cover!(true, \"the reset clock was read\");\n            \
             }}"
        );
        a += 1;
    }
    for n in &scope.timers {
        debug_assert_eq!(acts[a], Action::TimerClock(*n));
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume(tim{n}.is_some());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                tim{n}_clocked = Some(rcc.enable_tim{n}(&mut img, tim{n}.take().unwrap()));\n            \
             }}"
        );
        a += 1;
    }
    for (n, t) in plan.timers.iter().filter(|(n, _)| scope.timers.contains(n)) {
        debug_assert_eq!(acts[a], Action::TimerStart(*n));
        let pins: Vec<String> = t.channels.values().map(|(p, _)| pin_field(p)).collect();
        let ready: Vec<String> = pins
            .iter()
            .map(|p| format!(" && {p}_done.is_some()"))
            .collect();
        let args: Vec<String> = pins
            .iter()
            .map(|p| format!(", {p}_done.take().unwrap()"))
            .collect();
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume(tim{n}_clocked.is_some() && clock.is_some(){ready});",
            ready = ready.concat(),
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                let _running = tim{n}_clocked\n                    \
                     .take()\n                    \
                     .unwrap()\n                    \
                     .start_outputs(&mut img, clock.as_ref().unwrap(){args});\n                \
                 kani::cover!(true, \"TIM{n} started\");\n                \
                 tim{n}_done = true;\n            \
             }}",
            args = args.concat(),
        );
        a += 1;
    }
    debug_assert_eq!(a, actions);
    let _ = writeln!(
        w,
        "            _ => kani::assume(false),\n        }}\n    }}"
    );
    for name in plan
        .peripherals
        .keys()
        .filter(|n| scope.peripherals.contains(*n))
    {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "    kani::assert({f}_done, \"the schedule did not start {name}\");"
        );
    }
    for n in &scope.timers {
        let _ = writeln!(
            w,
            "    kani::assert(tim{n}_done, \"the schedule did not start TIM{n}\");"
        );
    }
    // A chip select is configured before its bus starts, which takes it.
    for (pin, p) in plan.pins.iter().filter(|(n, _)| scope.pins.contains(*n)) {
        if p.role == Role::Output && !plan.is_select(pin) {
            let f = pin_field(pin);
            let _ = writeln!(
                w,
                "    kani::assert({f}_done.is_some(), \"the schedule did not configure {pin}\");"
            );
        }
    }
    let timingr_args: Vec<String> = i2c_in_scope()
        .map(|(n, _)| format!(", {}_timingr", n.to_ascii_lowercase()))
        .collect();
    let _ = writeln!(
        w,
        "    check_final{sfx}(&before, &img{});\n    \
         kani::cover!(true, \"the end of a complete schedule is reachable\");\n\
         }}",
        timingr_args.concat()
    );
}

/// The proofs of the motor outputs (`AET-WO-4300a` §1.3), when the topology
/// has a timer: that `reset_clock` is `Ok` exactly when every field it checks
/// holds its reset value, reading only its four registers and writing none;
/// and, for each timer, that its start writes exactly the solved registers and
/// values in order, reading none. Every expected value is the oracle's
/// ([`timer_oracle`], and the fields' positions and widths), never read from
/// `board.rs`.
fn emit_timer_proofs(w: &mut String, plan: &Plan, facts: &Facts) {
    let (Some(tf), Some(clock)) = (&facts.tim, &facts.clock) else {
        return;
    };
    let words: Vec<String> = clock
        .iter()
        .map(|(r, _, _)| r.to_ascii_lowercase())
        .collect();
    let arms: Vec<String> = clock
        .iter()
        .enumerate()
        .map(|(i, (r, offset, _))| {
            format!(
                "if offset == {offset:#05x} {{\n            \
                     kani::assert(!self.reads[{i}], \"reset_clock read {r} twice\");\n            \
                     self.reads[{i}] = true;\n            \
                     self.{f}\n        \
                 }} else ",
                f = r.to_ascii_lowercase()
            )
        })
        .collect();
    let reset: Vec<String> = clock
        .iter()
        .flat_map(|(r, _, fields)| {
            let f = r.to_ascii_lowercase();
            fields.iter().map(move |(_, pos, width)| {
                let field = if *pos == 0 {
                    f.clone()
                } else {
                    format!("({f} >> {pos})")
                };
                format!("{field} & {:#x} == 0", oracle_mask(*width, 0))
            })
        })
        .collect();
    let n_reads = clock.len();
    let _ = writeln!(
        w,
        "\n\
         /// The registers `reset_clock` reads, as any words.\n\
         struct ClockSpy {{\n    \
             {fields},\n    \
             reads: [bool; {n_reads}],\n\
         }}\n\
         \n\
         impl sealed::Sealed for ClockSpy {{}}\n\
         \n\
         impl Registers for ClockSpy {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 kani::assert(block == Block::Rcc, \"reset_clock read outside RCC\");\n        \
                 {arms}{{\n            \
                     kani::assert(false, \"reset_clock read a register it does not check\");\n            \
                     0\n        \
                 }}\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, _: Block, _: usize, _: u32) {{\n        \
                 kani::assert(false, \"reset_clock wrote a register\");\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"reset_clock made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"reset_clock made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// `reset_clock` is `Ok` exactly when every field it checks is 0, its reset\n\
         /// value, for any words in its registers; it reads each register once and\n\
         /// writes none.\n\
         #[kani::proof]\n\
         fn reset_clock_is_ok_exactly_at_reset() {{\n    \
             let ({words}) = ({anys});\n    \
             let mut spy = ClockSpy {{ {inits}, reads: [false; {n_reads}] }};\n    \
             let board = Board::take().unwrap();\n    \
             let result = board.rcc.reset_clock(&mut spy);\n    \
             let reset = {reset};\n    \
             kani::assert(result.is_ok() == reset, \"reset_clock is not Ok exactly at the reset clock\");\n    \
             kani::assert({all_read}, \"reset_clock did not read every register it checks\");\n    \
             kani::cover!(result.is_ok(), \"the reset clock is admitted\");\n    \
             kani::cover!(result.is_err(), \"another clock is refused\");\n\
         }}",
        fields = words
            .iter()
            .map(|f| format!("{f}: u32"))
            .collect::<Vec<_>>()
            .join(",\n    "),
        arms = arms.concat(),
        words = words.join(", "),
        anys = vec!["kani::any()"; n_reads].join(", "),
        inits = words.join(", "),
        reset = reset.join("\n        && "),
        all_read = (0..n_reads)
            .map(|i| format!("spy.reads[{i}]"))
            .collect::<Vec<_>>()
            .join(" && "),
    );
    let mut blocks: Vec<String> = vec!["rcc".into()];
    blocks.extend(plan.ports.iter().map(|p| port_field(*p)));
    blocks.extend(plan.peripherals.keys().map(|n| n.to_ascii_lowercase()));
    blocks.extend(plan.timers.keys().map(|n| format!("tim{n}")));
    for (n, t) in &plan.timers {
        let writes = timer_oracle(*n, t, tf);
        let count = writes.len();
        let pins: Vec<&String> = t.channels.values().map(|(p, _)| p).collect();
        let ports: BTreeSet<char> = pins.iter().map(|p| plan.pins[*p].port).collect();
        let mut setup = String::new();
        for b in &blocks {
            let _ = writeln!(setup, "        {b}: kani::any(),");
        }
        let _ = writeln!(
            setup,
            "    }};\n    \
             let board = Board::take().unwrap();\n    \
             let mut rcc = board.rcc;"
        );
        for port in &ports {
            let f = port_field(*port);
            let _ = writeln!(
                setup,
                "    let {f} = rcc.enable_{f}(&mut img, board.{f}).split();"
            );
        }
        for pin in &pins {
            let _ = writeln!(
                setup,
                "    let {f} = {port}.{f}.{method}(&mut img);",
                f = pin_field(pin),
                port = port_field(plan.pins[*pin].port),
                method = method(&plan.pins[*pin])
            );
        }
        let expect: Vec<String> = writes
            .iter()
            .enumerate()
            .map(|(i, (name, offset, value))| {
                format!(
                    "    kani::assert(spy.writes[{i}] == (Block::Tim{n}, {offset:#04x}, {value:#010x}), \"TIM{n} write {k} is not {name} = {value}\");",
                    k = i + 1
                )
            })
            .collect();
        let args: Vec<String> = pins.iter().map(|p| pin_field(p)).collect();
        let _ = writeln!(
            w,
            "\n\
             /// TIM{n}'s start as the list of its writes, in order.\n\
             struct Tim{n}Spy {{\n    \
                 writes: [(Block, usize, u32); {count}],\n    \
                 n: usize,\n\
             }}\n\
             \n\
             impl sealed::Sealed for Tim{n}Spy {{}}\n\
             \n\
             impl Registers for Tim{n}Spy {{\n    \
                 fn read(&mut self, _: sealed::Token, _: Block, _: usize) -> u32 {{\n        \
                     kani::assert(false, \"TIM{n}'s start read a register\");\n        \
                     0\n    \
                 }}\n    \
                 fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                     kani::assert(self.n < {count}, \"TIM{n}'s start wrote more than {count} registers\");\n        \
                     if self.n < {count} {{\n            \
                         self.writes[self.n] = (block, offset, value);\n        \
                     }}\n        \
                     self.n += 1;\n    \
                 }}\n    \
                 fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                     kani::assert(false, \"TIM{n}'s start made a byte access\");\n        \
                     0\n    \
                 }}\n    \
                 fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                     kani::assert(false, \"TIM{n}'s start made a byte access\");\n    \
                 }}\n\
             }}\n\
             \n\
             /// From its configured pins, its clock and the reset clock's token, TIM{n}'s\n\
             /// start writes exactly the solved registers and values, in order, and\n\
             /// reads none.\n\
             #[kani::proof]\n\
             fn tim{n}_start_writes_the_solved_values() {{\n    \
                 let mut img = Image {{\n\
             {setup}    \
                 let clock = rcc.reset_clock(&mut img);\n    \
                 kani::assume(clock.is_ok());\n    \
                 let clock = clock.unwrap();\n    \
                 let tim = rcc.enable_tim{n}(&mut img, board.tim{n});\n    \
                 let mut spy = Tim{n}Spy {{ writes: [(Block::Rcc, 0, 0); {count}], n: 0 }};\n    \
                 let _running = tim.start_outputs(&mut spy, &clock, {args});\n    \
                 kani::assert(spy.n == {count}, \"TIM{n}'s start did not make exactly {count} writes\");\n\
             {expect}\n    \
                 kani::cover!(true, \"TIM{n} started\");\n\
             }}",
            args = args.join(", "),
            expect = expect.join("\n"),
        );
    }
}

/// The proof over each SPI peripheral's transfer (`AET-WO-4200`), on the
/// harness-owned register image: from the state `init` leaves (the
/// peripheral disabled, every chip select high), a transfer of any device
/// touches only that peripheral and that device's chip select, in the order
/// of a transfer, within its bound, and leaves the same state. The
/// environment is nondeterministic: every `SR` and `RXDR` read returns any
/// value, and `CSTART` may or may not have cleared itself.
fn emit_spi_transfer_proofs(w: &mut String, plan: &Plan, facts: &Facts) {
    for (name, selects) in &plan.selects {
        // The proof's frames: up to two bytes out and three in (an identity
        // read is one and three), or as many as any step of a script on this
        // bus writes and reads (`AET-WO-4210`: the IMU's data read is one and
        // six), so the proof covers every transfer the firmware makes.
        let steps: Vec<aethelgard_ir::parts::Step> = selects
            .iter()
            .filter_map(|s| aethelgard_ir::parts::steps(&s.part).ok().flatten())
            .flatten()
            .collect();
        let wmax = steps.iter().map(|s| s.write.len()).fold(2, usize::max);
        let rmax = steps
            .iter()
            .map(|s| usize::from(s.width))
            .fold(3, usize::max);
        let scripted: Vec<(usize, usize, &'static str)> = {
            let mut v: Vec<(usize, usize, &'static str)> = steps
                .iter()
                .filter(|s| s.kind != aethelgard_ir::parts::StepKind::Identity)
                .map(|s| (s.write.len(), usize::from(s.width), s.kind.name()))
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let pf = &facts.peripherals[name];
        let Some(s) = &pf.spi else { continue };
        let p = &plan.peripherals[name];
        let t = type_name(name);
        let m = name.to_ascii_lowercase();
        let bit = |pos: u32| format!("{:#010x}", 1u32 << pos);
        let (spe, cstart) = (bit(pf.enable), bit(s.cstart));
        let arms: Vec<String> = selects
            .iter()
            .map(|sel| {
                let pinf = &facts.pins[&sel.pin];
                format!(
                    "            {t}Device::{v} => (Block::{port}, {br:#010x}, {bs:#010x}),",
                    v = sel.variant,
                    port = port_type(plan.pins[&sel.pin].port),
                    br = 1u32 << pinf.br,
                    bs = 1u32 << pinf.bs,
                )
            })
            .collect();
        let _ = writeln!(
            w,
            "\n\
             /// {t}'s registers and its devices' chip selects as a transfer may touch them,\n\
             /// in order. `stage` is 0 before, 1 once TSIZE is set, 2 once the device is\n\
             /// selected, 3 once enabled, 4 once started, 5 once EOT is cleared, 6 once\n\
             /// disabled, 7 once deselected.\n\
             struct {t}Spy {{\n    \
                 device: {t}Device,\n    \
                 out: [u8; {wmax}],\n    \
                 wl: usize,\n    \
                 stage: u8,\n    \
                 cr1: u32,\n    \
                 tsize: u32,\n    \
                 sr_reads: u32,\n    \
                 rxp: bool,\n    \
                 sent: usize,\n    \
                 received: usize,\n    \
                 touched: bool,\n\
             }}\n\
             \n\
             impl {t}Spy {{\n    \
                 /// The device's chip select: its port, and its BSRR reset and set words.\n    \
                 fn select(&self) -> (Block, u32, u32) {{\n        \
                     match self.device {{\n\
             {arms}\n        \
                     }}\n    \
                 }}\n\
             }}\n\
             \n\
             impl sealed::Sealed for {t}Spy {{}}\n\
             \n\
             impl Registers for {t}Spy {{\n    \
                 fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                     self.touched = true;\n        \
                     kani::assert(block == Block::{t}, \"a {t} transfer read another block\");\n        \
                     if offset == {cr1:#04x} {{\n            \
                         if kani::any() {{ self.cr1 & !{cstart} }} else {{ self.cr1 }}\n        \
                     }} else if offset == {sr:#04x} {{\n            \
                         kani::assert(self.stage == 4, \"SR polled outside the transfer\");\n            \
                         self.sr_reads += 1;\n            \
                         let v: u32 = kani::any();\n            \
                         self.rxp = v & {rxp} != 0;\n            \
                         v\n        \
                     }} else {{\n            \
                         kani::assert(false, \"a {t} transfer read a register it does not own\");\n            \
                         0\n        \
                     }}\n    \
                 }}\n    \
                 fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                     self.touched = true;\n        \
                     let (port, select, deselect) = self.select();\n        \
                     if block == port && offset == {bsrr:#04x} && value == select {{\n            \
                         kani::assert(self.stage == 1, \"the device was selected out of order\");\n            \
                         self.stage = 2;\n        \
                     }} else if block == port && offset == {bsrr:#04x} && value == deselect {{\n            \
                         kani::assert(self.stage == 6, \"the device was deselected before the peripheral was disabled\");\n            \
                         self.stage = 7;\n        \
                     }} else if block == Block::{t} && offset == {cr2:#04x} {{\n            \
                         kani::assert(self.stage == 0, \"CR2 written out of order\");\n            \
                         kani::assert(self.cr1 & {spe} == 0, \"TSIZE written while the peripheral is enabled\");\n            \
                         self.tsize = value;\n            \
                         self.stage = 1;\n        \
                     }} else if block == Block::{t} && offset == {cr1:#04x} {{\n            \
                         kani::assert(\n                \
                             (value ^ self.cr1) & !({spe} | {cstart}) == 0,\n                \
                             \"CR1 changed beyond SPE and CSTART\",\n            \
                         );\n            \
                         // `CSTART` is set-only: the enable may carry a stale one back.\n            \
                         if self.stage == 2 {{\n                \
                             kani::assert(value & {spe} != 0, \"the peripheral was not enabled\");\n                \
                             self.stage = 3;\n            \
                         }} else if self.stage == 3 {{\n                \
                             kani::assert(\n                    \
                                 value & ({spe} | {cstart}) == {spe} | {cstart},\n                    \
                                 \"the transfer was not started\",\n                \
                             );\n                \
                             self.stage = 4;\n            \
                         }} else if self.stage == 5 {{\n                \
                             kani::assert(value & {spe} == 0, \"the peripheral was not disabled\");\n                \
                             self.stage = 6;\n            \
                         }} else {{\n                \
                             kani::assert(false, \"CR1 written out of order\");\n            \
                         }}\n            \
                         self.cr1 = value;\n        \
                     }} else if block == Block::{t} && offset == {ifcr:#04x} {{\n            \
                         kani::assert(self.stage == 4, \"EOT cleared out of order\");\n            \
                         kani::assert(value == {eotc}, \"IFCR cleared more than EOTC\");\n            \
                         self.stage = 5;\n        \
                     }} else {{\n            \
                         kani::assert(false, \"a {t} transfer wrote a register it does not own, or another device's chip select\");\n        \
                     }}\n    \
                 }}\n    \
                 fn read8(&mut self, _: sealed::Token, block: Block, offset: usize) -> u8 {{\n        \
                     self.touched = true;\n        \
                     kani::assert(block == Block::{t} && offset == {rxdr:#04x}, \"a byte read of anything but RXDR\");\n        \
                     kani::assert(self.stage == 4, \"RXDR read outside the transfer\");\n        \
                     kani::assert(self.rxp, \"RXDR read without RXP\");\n        \
                     self.rxp = false;\n        \
                     self.received += 1;\n        \
                     kani::any()\n    \
                 }}\n    \
                 fn write8(&mut self, _: sealed::Token, block: Block, offset: usize, value: u8) {{\n        \
                     self.touched = true;\n        \
                     kani::assert(block == Block::{t} && offset == {txdr:#04x}, \"a byte write to anything but TXDR\");\n        \
                     kani::assert(self.stage == 4, \"TXDR written outside the transfer\");\n        \
                     let expected = if self.sent < self.wl {{ self.out[self.sent] }} else {{ 0 }};\n        \
                     kani::assert(value == expected, \"a frame is not the caller's byte, or a dummy is not 0x00\");\n        \
                     self.sent += 1;\n        \
                     kani::assert(self.sent as u32 <= {frames_set}, \"more frames sent than TSIZE\");\n    \
                 }}\n\
             }}",
            arms = arms.join("\n"),
            cr1 = pf.cr1,
            sr = s.sr,
            rxp = bit(s.rxp),
            bsrr = facts.gpio.bsrr,
            cr2 = s.cr2,
            ifcr = s.ifcr,
            eotc = bit(s.eotc),
            rxdr = s.rxdr,
            txdr = pf.txdr,
            frames_set = shifted("self.tsize", ">>", s.tsize),
        );
        // The setup: the bus's ports, its pins and chip selects, `init`.
        let mut blocks: Vec<String> = vec!["rcc".into()];
        blocks.extend(plan.ports.iter().map(|p| port_field(*p)));
        blocks.extend(plan.peripherals.keys().map(|n| n.to_ascii_lowercase()));
        blocks.extend(plan.timers.keys().map(|n| format!("tim{n}")));
        let pins: Vec<&String> = p.pins.values().collect();
        let select_pins: Vec<&String> = selects.iter().map(|s| &s.pin).collect();
        let ports: BTreeSet<char> = pins
            .iter()
            .chain(&select_pins)
            .map(|n| plan.pins[*n].port)
            .collect();
        let mut setup = String::new();
        for b in &blocks {
            let _ = writeln!(setup, "        {b}: kani::any(),");
        }
        let _ = writeln!(
            setup,
            "    }};\n    \
             let board = Board::take().unwrap();\n    \
             let mut rcc = board.rcc;"
        );
        for port in &ports {
            let f = port_field(*port);
            let _ = writeln!(
                setup,
                "    let {f} = rcc.enable_{f}(&mut img, board.{f}).split();"
            );
        }
        for pin in pins.iter().chain(&select_pins) {
            let _ = writeln!(
                setup,
                "    let {f} = {port}.{f}.{method}(&mut img);",
                f = pin_field(pin),
                port = port_field(plan.pins[*pin].port),
                method = method(&plan.pins[*pin])
            );
        }
        let args: Vec<String> = pins.iter().map(|n| pin_field(n)).collect();
        let fields: Vec<String> = select_pins.iter().map(|n| pin_field(n)).collect();
        let variants: Vec<String> = selects
            .iter()
            .map(|s| format!("{t}Device::{}", s.variant))
            .collect();
        let _ = writeln!(
            setup,
            "    let mut bus = rcc\n        \
                 .enable_{m}(&mut img, board.{m})\n        \
                 .init(&mut img, {args}, {t}Selects {{ {fields} }});\n    \
             let devices = [{variants}];\n    \
             let pick: usize = kani::any();\n    \
             kani::assume(pick < devices.len());\n    \
             let device = devices[pick];\n    \
             // The state `init` and every transfer leave: disabled (`every_valid_sequence`,\n    \
             // and the end of this proof).\n    \
             let cr1: u32 = kani::any();\n    \
             kani::assume(cr1 & {spe} == 0);",
            args = args.join(", "),
            fields = fields.join(", "),
            variants = variants.join(", "),
        );
        let spy = format!(
            "{t}Spy {{ device, out, wl, stage: 0, cr1, tsize: 0, sr_reads: 0, rxp: false, sent: 0, received: 0, touched: false }}"
        );
        let rest = "and any answers, with each wait bounded at two reads\n\
             /// of `SR`: selects that device before its first frame and deselects it after\n\
             /// the peripheral is disabled, also after a wait that expired; reads `RXDR`\n\
             /// only after `RXP`; and returns `Ok` only when every frame moved.\n";
        let doc = if (wmax, rmax) == (2, 3) {
            format!(
                "/// A transfer of any device, up to two bytes out and three in (an identity\n\
                 /// read is one and three) {rest}"
            )
        } else {
            format!(
                "/// A transfer of any device, up to {wmax} bytes out and {rmax} in (an identity\n\
                 /// read, or the largest step of a script on this bus, `AET-WO-4210`)\n\
                 /// {rest}"
            )
        };
        let unwind = wmax + rmax;
        let most = match unwind {
            5 => "five".to_owned(),
            n => n.to_string(),
        };
        let covers: String = scripted
            .iter()
            .map(|(a, b, step)| {
                format!(
                    "            kani::cover!(wl == {a} && rl == {b}, \"{t} made a {step} step's frames\");\n"
                )
            })
            .collect();
        let _ = writeln!(
            w,
            "\n\
             {doc}\
             #[kani::proof]\n\
             #[kani::unwind({unwind})]\n\
             fn {m}_transfer_is_bounded_and_ordered() {{\n    \
                 let mut img = Image {{\n\
             {setup}    \
                 let (wl, rl): (usize, usize) = (kani::any(), kani::any());\n    \
                 kani::assume(wl <= {wmax} && rl <= {rmax});\n    \
                 let out: [u8; {wmax}] = kani::any();\n    \
                 let mut back = [0u8; {rmax}];\n    \
                 let mut spy = {spy};\n    \
                 let spins = 2;\n    \
                 let result = bus.transfer_within(&mut spy, device, &out[..wl], &mut back[..rl], spins);\n    \
                 let frames = wl + rl;\n    \
                 if frames == 0 {{\n        \
                     kani::assert(result == Err(SpiError::Length), \"a transfer of nothing was not refused\");\n        \
                     kani::assert(!spy.touched, \"a refused transfer touched a register\");\n        \
                     kani::cover!(true, \"{t} refused a transfer of nothing\");\n        \
                     return;\n    \
                 }}\n    \
                 kani::assert(spy.stage == 7, \"a transfer did not end disabled and deselected\");\n    \
                 kani::assert(spy.tsize == {tsize_word}, \"TSIZE is not the frames of the transfer\");\n    \
                 kani::assert(\n        \
                     spy.sr_reads as usize <= frames * spins as usize,\n        \
                     \"a wait read SR more often than its bound\",\n    \
                 );\n    \
                 match result {{\n        \
                     Ok(n) => {{\n            \
                         kani::assert(n == rl && spy.sent == frames && spy.received == frames, \"Ok without every frame moved\");\n            \
                         kani::cover!(wl > 0 && rl > 0, \"{t} wrote, then read\");\n            \
                         kani::cover!(wl == 1 && rl == 3, \"{t} made an identity read's frames\");\n{covers}        \
                     }}\n        \
                     Err(SpiError::Rxp) => {{\n            \
                         kani::assert(spy.received < frames, \"Rxp after every frame moved\");\n            \
                         kani::cover!(true, \"{t} timed out waiting for RXP\");\n        \
                     }}\n        \
                     Err(SpiError::Length) => kani::assert(false, \"a transfer of at most {most} frames was refused\"),\n    \
                 }}\n\
             }}\n\
             \n\
             /// More frames than [`SPI_MAX_FRAMES`] are refused, and touch no register. The\n\
             /// bound fails the proof if a loop of the transfer is reached.\n\
             #[kani::proof]\n\
             #[kani::unwind(2)]\n\
             fn {m}_transfer_refuses_too_many_frames() {{\n    \
                 let mut img = Image {{\n\
             {setup}    \
                 let (wl, out) = (1, [0u8; {wmax}]);\n    \
                 let mut back = [0u8; SPI_MAX_FRAMES as usize];\n    \
                 let mut spy = {spy};\n    \
                 let result = bus.transfer(&mut spy, device, &out[..wl], &mut back);\n    \
                 kani::assert(result == Err(SpiError::Length), \"a transfer of too many frames was not refused\");\n    \
                 kani::assert(!spy.touched, \"a refused transfer touched a register\");\n    \
                 kani::cover!(true, \"{t} refused too many frames\");\n\
             }}",
            tsize_word = shifted("frames as u32", "<<", s.tsize),
        );
    }
}

/// The proof over each I2C peripheral's transfer (`AET-WO-2202`, ruled on the
/// I2C read path, F1), on the harness-owned register image: a transfer
/// touches only that peripheral's registers, in the order of a transfer, and
/// polls `ISR` no more than its bound allows. The environment is
/// nondeterministic: every `ISR` read and every `RXDR` read returns any word.
fn emit_transfer_proofs(w: &mut String, plan: &Plan, facts: &Facts, speeds: &Speeds) {
    for (name, p) in plan.peripherals.iter().filter(|(_, p)| p.kind == Kind::I2c) {
        let (pf, i) = (&facts.peripherals[name], &facts.peripherals[name].i2c);
        let Some(i) = i else { continue };
        let t = type_name(name);
        let m = name.to_ascii_lowercase();
        // The proof's bytes: two each way, or as many as any step of a script
        // on this bus writes and reads (`AET-WO-4210` adversarial A9).
        let (wmax, rmax) = plan.i2c_frames.get(name).copied().unwrap_or((2, 2));
        let unwind = wmax + rmax + 2;
        let (doc, most) = if (wmax, rmax) == (2, 2) {
            (
                "/// A transfer of any address, up to two bytes each way and any answers, with\n"
                    .to_owned(),
                "two".to_owned(),
            )
        } else {
            (
                format!(
                    "/// A transfer of any address, up to {wmax} bytes out and {rmax} in (the largest step\n\
                     /// of a script on this bus, `AET-WO-4210`) and any answers, with\n"
                ),
                format!("{wmax} out or {rmax} in"),
            )
        };
        let bit = |pos: u32| format!("{:#010x}", 1u32 << pos);
        let (_, admitted) = &speeds[name];
        let mode = I2C_MODES[admitted[0]].0;
        let pins: Vec<String> = p.pins.values().map(|n| pin_field(n)).collect();
        let ports: BTreeSet<char> = p.pins.values().map(|n| plan.pins[n].port).collect();
        let mut blocks: Vec<String> = vec!["rcc".into()];
        blocks.extend(plan.ports.iter().map(|p| port_field(*p)));
        blocks.extend(plan.peripherals.keys().map(|n| n.to_ascii_lowercase()));
        blocks.extend(plan.timers.keys().map(|n| format!("tim{n}")));
        let _ = writeln!(
            w,
            "\n\
             /// {t}'s registers as a transfer may touch them, in order. `stage` is 0 before\n\
             /// the first START, 1 in the write phase, 2 in the read phase, 3 after the\n\
             /// flags are cleared.\n\
             struct {t}Spy {{\n    \
                 stage: u8,\n    \
                 isr_reads: u32,\n    \
                 bytes_written: usize,\n\
             }}\n\
             \n\
             impl sealed::Sealed for {t}Spy {{}}\n\
             \n\
             impl Registers for {t}Spy {{\n    \
                 fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                     kani::assert(block == Block::{t}, \"a {t} transfer read another block\");\n        \
                     if offset == {isr:#04x} {{\n            \
                         kani::assert(self.stage >= 1, \"ISR polled before the START\");\n            \
                         self.isr_reads += 1;\n            \
                         kani::any()\n        \
                     }} else if offset == {rxdr:#04x} {{\n            \
                         kani::assert(self.stage == 2, \"RXDR read outside the read phase\");\n            \
                         kani::any()\n        \
                     }} else {{\n            \
                         kani::assert(false, \"a {t} transfer read a register it does not own\");\n            \
                         0\n        \
                     }}\n    \
                 }}\n    \
                 fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                     kani::assert(block == Block::{t}, \"a {t} transfer wrote another block\");\n        \
                     if offset == {cr2:#04x} {{\n            \
                         kani::assert(value & {start} != 0, \"CR2 written without START\");\n            \
                         if value & {rd_wrn} != 0 {{\n                \
                             kani::assert(self.stage <= 1, \"the read phase did not follow the write phase\");\n                \
                             kani::assert(value & {autoend} != 0, \"a read ends without AUTOEND\");\n                \
                             kani::assert(\n                    \
                                 self.stage == 0 || self.isr_reads as usize > self.bytes_written,\n                    \
                                 \"the read phase began without waiting for TC\",\n                \
                             );\n                \
                             self.stage = 2;\n            \
                         }} else {{\n                \
                             kani::assert(self.stage == 0, \"the write phase is not first\");\n                \
                             self.stage = 1;\n            \
                         }}\n        \
                     }} else if offset == {txdr:#04x} {{\n            \
                         kani::assert(self.stage == 1, \"TXDR written outside the write phase\");\n            \
                         self.bytes_written += 1;\n        \
                     }} else if offset == {icr:#04x} {{\n            \
                         kani::assert(self.stage >= 1, \"flags cleared before the START\");\n            \
                         kani::assert(value == {stopcf} | {nackcf}, \"ICR cleared more than STOPCF and NACKCF\");\n            \
                         self.stage = 3;\n        \
                     }} else {{\n            \
                         kani::assert(false, \"a {t} transfer wrote a register it does not own\");\n        \
                     }}\n    \
                 }}\n    \
                 fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                     kani::assert(false, \"a {t} transfer made a byte access\");\n        \
                     0\n    \
                 }}\n    \
                 fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                     kani::assert(false, \"a {t} transfer made a byte access\");\n    \
                 }}\n\
             }}\n\
             \n\
             {doc}\
             /// each wait bounded at two reads of `ISR`: touches only {t}, in order, within\n\
             /// the bound, and ends in the result its phases imply.\n\
             #[kani::proof]\n\
             #[kani::unwind({unwind})]\n\
             fn {m}_transfer_is_bounded_and_ordered() {{\n    \
                 let mut img = Image {{",
            isr = i.isr,
            rxdr = i.rxdr,
            cr2 = i.cr2,
            txdr = pf.txdr,
            icr = i.icr,
            start = bit(i.start),
            rd_wrn = bit(i.rd_wrn),
            autoend = bit(i.autoend),
            stopcf = bit(i.stopcf),
            nackcf = bit(i.nackcf),
        );
        for b in &blocks {
            let _ = writeln!(w, "        {b}: kani::any(),");
        }
        let _ = writeln!(
            w,
            "    }};\n    \
             let board = Board::take().unwrap();\n    \
             let mut rcc = board.rcc;"
        );
        for port in &ports {
            let f = port_field(*port);
            let _ = writeln!(
                w,
                "    let {f} = rcc.enable_{f}(&mut img, board.{f}).split();"
            );
        }
        for pin in p.pins.values() {
            let f = pin_field(pin);
            let _ = writeln!(
                w,
                "    let {f} = {port}.{f}.{method}(&mut img);",
                port = port_field(plan.pins[pin].port),
                method = method(&plan.pins[pin])
            );
        }
        let _ = writeln!(
            w,
            "    let mut bus = rcc\n        \
                 .enable_{m}(&mut img, board.{m})\n        \
                 .init::<{m}_speed::{mode}, _>(&mut img, {args});\n    \
             let mut spy = {t}Spy {{ stage: 0, isr_reads: 0, bytes_written: 0 }};\n    \
             let (wl, rl): (usize, usize) = (kani::any(), kani::any());\n    \
             kani::assume(wl <= {wmax} && rl <= {rmax});\n    \
             let out: [u8; {wmax}] = kani::any();\n    \
             let mut back = [0u8; {rmax}];\n    \
             let spins = 2;\n    \
             let result = bus.transfer_within(&mut spy, kani::any(), &out[..wl], &mut back[..rl], spins);\n    \
             let waits = wl + usize::from(wl > 0 && rl > 0) + rl + usize::from(wl + rl > 0);\n    \
             kani::assert(\n        \
                 spy.isr_reads as usize <= waits * spins as usize,\n        \
                 \"a wait read ISR more often than its bound\",\n    \
             );\n    \
             kani::assert((wl + rl == 0) == (spy.stage == 0), \"a transfer of nothing touched a register, or one of something did not\");\n    \
             if wl + rl > 0 {{\n        \
                 kani::assert(spy.stage == 3, \"a transfer did not end by clearing its flags\");\n    \
             }}\n    \
             match result {{\n        \
                 Ok(n) => {{\n            \
                     kani::assert(n == rl && spy.bytes_written == wl, \"Ok without every byte moved\");\n            \
                     kani::cover!(wl > 0 && rl > 0, \"{t} wrote, then read\");\n            \
                     kani::cover!(wl > 0 && rl == 0, \"{t} wrote only\");\n            \
                     kani::cover!(wl == 0 && rl > 0, \"{t} read only\");\n        \
                 }}\n        \
                 Err(I2cError::Txis) => kani::cover!(true, \"{t} timed out waiting for TXIS\"),\n        \
                 Err(I2cError::Tc) => kani::cover!(true, \"{t} timed out waiting for TC\"),\n        \
                 Err(I2cError::Rxne) => kani::cover!(true, \"{t} timed out waiting for RXNE\"),\n        \
                 Err(I2cError::Stop) => kani::cover!(true, \"{t} timed out waiting for STOPF\"),\n        \
                 Err(I2cError::Length) => kani::assert(false, \"a transfer of at most {most} bytes was refused as too long\"),\n    \
             }}\n\
             }}",
            args = pins.join(", "),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethelgard_ir::{AnalogLine, PinAssignment};

    fn table() -> PinTable {
        crate::pin_table("STM32H753ZITx").unwrap()
    }

    fn assign(pin: &str, peripheral: &str, signal: &str, af: u8) -> PinAssignment {
        PinAssignment {
            pin: pin.into(),
            peripheral: peripheral.into(),
            signal: signal.into(),
            alternate_function: af,
        }
    }

    /// Sentinel-01's solved topology, written out.
    fn solved() -> SolvedTopology {
        SolvedTopology {
            schema_version: aethelgard_ir::SOLVED_SCHEMA_VERSION,
            specification_sha256: "0".repeat(64),
            assignments: BTreeMap::from([
                ("SCL".into(), assign("PB6", "I2C1", "I2C1_SCL", 4)),
                ("SDA".into(), assign("PB7", "I2C1", "I2C1_SDA", 4)),
            ]),
            gpio: BTreeMap::from([("CS".into(), "PA0".into())]),
            i2c_ceilings: BTreeMap::from([("BUS".into(), 400_000)]),
            i2c_speeds: BTreeMap::from([("BUS".into(), 400_000)]),
            analog_lines: analog_lines(&[("SCL", "BUS"), ("SDA", "BUS")]),
            i2c_devices: BTreeMap::new(),
            spi_devices: BTreeMap::new(),
            spi_lines: BTreeMap::new(),
            clock: aethelgard_ir::SolvedClock::reset(),
            motor_outputs: BTreeMap::new(),
            power: None,
        }
    }

    /// Each (net, bus) as the reference line's analog facts (`AET-WO-2200`).
    fn analog_lines(lines: &[(&str, &str)]) -> BTreeMap<String, AnalogLine> {
        let line = |bus: &str| AnalogLine {
            bus: bus.into(),
            pull_up_ohms: 2200,
            capacitance_femtofarads: 100_000,
            ceiling_ns: 300,
            domain_millivolts: 3300,
        };
        lines
            .iter()
            .map(|(net, bus)| ((*net).to_owned(), line(bus)))
            .collect()
    }

    /// A bus's ceiling with the rise-time bound that derives it, on every one
    /// of its lines: `SolvedTopology::validate` holds a hand-written topology
    /// to the ceiling `solve` derives (`AET-WO-1204`). The bus is declared at
    /// its ceiling; `set_speed` lowers it.
    fn set_ceiling(s: &mut SolvedTopology, bus: &str, hz: u32) {
        let ns = match hz {
            100_000 => 1000,
            400_000 => 300,
            _ => 120,
        };
        s.i2c_ceilings.insert(bus.into(), hz);
        s.i2c_speeds.insert(bus.into(), hz);
        for line in s.analog_lines.values_mut().filter(|l| l.bus == bus) {
            line.ceiling_ns = ns;
        }
    }

    fn set_speed(s: &mut SolvedTopology, bus: &str, hz: u32) {
        s.i2c_speeds.insert(bus.into(), hz);
    }

    fn kind(result: Result<BTreeMap<String, String>, BoardError>) -> &'static str {
        result.unwrap_err().kind()
    }

    fn admitted(files: &BTreeMap<String, String>) -> Vec<&'static str> {
        I2C_MODES
            .iter()
            .map(|m| m.0)
            .filter(|m| files["src/board.rs"].contains(&format!("impl AtMostCeiling for {m} ")))
            .collect()
    }

    #[test]
    fn the_solved_ceiling_admits_only_the_modes_at_or_under_it() {
        for (hz, modes) in [
            (100_000, vec!["Standard100k"]),
            (400_000, vec!["Standard100k", "Fast400k"]),
            (1_000_000, vec!["Standard100k", "Fast400k", "FastPlus1M"]),
        ] {
            let mut s = solved();
            set_ceiling(&mut s, "BUS", hz);
            let files = generate("b", &s, &table()).unwrap();
            assert_eq!(admitted(&files), modes, "{hz}");
            // The harness drives exactly the admitted speeds.
            let proofs = &files["src/proofs.rs"];
            for (mode, _, _) in I2C_MODES {
                assert_eq!(
                    proofs.contains(&format!("i2c1_speed::{mode},")),
                    modes.contains(&mode)
                );
            }
        }
    }

    #[test]
    fn the_declared_speed_not_the_ceiling_decides_the_admitted_modes() {
        // AET-WO-1205: a 100 kHz bus under a 400 kHz ceiling admits only
        // Standard100k, and the harness drives only that speed.
        let mut s = solved();
        set_speed(&mut s, "BUS", 100_000);
        let files = generate("b", &s, &table()).unwrap();
        assert_eq!(admitted(&files), vec!["Standard100k"]);
        let proofs = &files["src/proofs.rs"];
        for (mode, _, _) in I2C_MODES {
            assert_eq!(
                proofs.contains(&format!("i2c1_speed::{mode},")),
                mode == "Standard100k",
                "{mode}"
            );
        }
        assert!(files["src/board.rs"].contains("declared speed of 100000 Hz"));
    }

    #[test]
    fn ceilings_must_number_the_i2c_peripherals_and_the_lowest_applies() {
        let mut s = solved();
        s.i2c_ceilings.clear();
        // Since AET-WO-2200 an I2C line with no bus ceiling is refused by
        // `validate`, before the generator looks: each solved I2C line has an
        // analog line, whose bus must have a ceiling.
        assert_eq!(kind(generate("b", &s, &table())), "InvalidTopology");
        let mut s = solved();
        s.i2c_ceilings.insert("OTHER".into(), 100_000);
        // A ceiling for a bus with no analog line is refused by `validate`
        // since `AET-WO-1204`, before the generator counts them.
        assert_eq!(kind(generate("b", &s, &table())), "InvalidTopology");
        // Two buses: each peripheral gets the lower ceiling.
        let mut s = solved();
        s.assignments
            .insert("SCL2".into(), assign("PB10", "I2C2", "I2C2_SCL", 4));
        s.assignments
            .insert("SDA2".into(), assign("PB11", "I2C2", "I2C2_SDA", 4));
        s.i2c_ceilings = BTreeMap::new();
        s.i2c_speeds = BTreeMap::new();
        s.analog_lines = analog_lines(&[("SCL", "A"), ("SDA", "A"), ("SCL2", "B"), ("SDA2", "B")]);
        set_ceiling(&mut s, "A", 1_000_000);
        set_ceiling(&mut s, "B", 100_000);
        let files = generate("b", &s, &table()).unwrap();
        let board = &files["src/board.rs"];
        for m in ["i2c1_speed", "i2c2_speed"] {
            let module = board.split(&format!("pub mod {m} {{")).nth(1).unwrap();
            let module = module.split("\n}\n").next().unwrap();
            assert!(module.contains("impl AtMostCeiling for Standard100k "));
            assert!(!module.contains("impl AtMostCeiling for Fast400k "));
        }
    }

    #[test]
    fn what_the_generator_cannot_emit_is_refused() {
        let t = table();
        let mut s = solved();
        s.assignments
            .insert("SCL".into(), assign("PB6", "I2C1", "I2C1_SCL", 5));
        assert_eq!(kind(generate("b", &s, &t)), "AssignmentMismatch");

        let mut s = solved();
        s.assignments
            .insert("TX".into(), assign("PB14", "USART1", "USART1_TX", 4));
        assert_eq!(kind(generate("b", &s, &t)), "UnsupportedPeripheral");

        let mut s = solved();
        s.assignments
            .insert("SMBA".into(), assign("PB5", "I2C1", "I2C1_SMBA", 4));
        assert_eq!(kind(generate("b", &s, &t)), "UnsupportedSignal");

        let mut s = solved();
        s.assignments.remove("SDA");
        s.analog_lines.remove("SDA");
        // A bus without its SDA or SCL line is refused by `validate` since
        // `AET-WO-1204`, before the generator looks.
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.assignments
            .insert("SDA2".into(), assign("PB9", "I2C1", "I2C1_SDA", 4));
        s.analog_lines = analog_lines(&[("SCL", "BUS"), ("SDA", "BUS"), ("SDA2", "BUS")]);
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.gpio.insert("CS2".into(), "PB6".into());
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.gpio.insert("SWDIO".into(), "PA13".into());
        assert_eq!(kind(generate("b", &s, &t)), "ReservedPin");

        let mut s = solved();
        s.specification_sha256 = "x".into();
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.gpio.insert("CS2".into(), "PB16".into());
        assert_eq!(kind(generate("b", &s, &t)), "InvalidPin");

        let mut s = solved();
        s.gpio.insert("CS2".into(), "PK15".into());
        assert_eq!(kind(generate("b", &s, &t)), "UnknownPin");

        for name in ["", "Board", "b_1", "-b", "b-", "b--c", "1b"] {
            assert_eq!(
                kind(generate(name, &solved(), &t)),
                "InvalidCrateName",
                "{name}"
            );
        }
    }

    #[test]
    fn the_generated_text_carries_no_unsafe_outside_mmio() {
        let files = generate("b", &solved(), &table()).unwrap();
        let lib = &files["src/board.rs"];
        let (before, rest) = lib.split_once("pub mod mmio {").unwrap();
        let (_, after) = rest.split_once("\n}\n").unwrap();
        let code = |text: &str| {
            text.contains("unsafe {") || text.contains("unsafe fn") || text.contains("unsafe impl")
        };
        assert!(!code(before));
        assert!(!code(after));
        assert!(!code(&files["src/proofs.rs"]));
        assert!(rest.contains("unsafe fn new"));
    }

    #[test]
    fn pin_names_are_read_strictly() {
        assert_eq!(pin_name("PB6"), Some(('B', 6)));
        assert_eq!(pin_name("PK15"), Some(('K', 15)));
        // A sign `u32::from_str` would take, a leading zero, a lowercase port,
        // a port past K, a pin past 15, a suffix.
        for bad in [
            "PA+5", "PA05", "Pa5", "PL1", "PA16", "PA5_", "PA", "A5", "PA-1",
        ] {
            assert_eq!(pin_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_firmware_follows_the_plan_and_the_header() {
        let files = generate("b", &solved(), &table()).unwrap();
        let main = &files["firmware/src/main.rs"];
        // The highest admitted speed, every pin in its role, init in the board
        // crate's argument order.
        assert!(main.contains("init::<i2c1_speed::Fast400k, _>(&mut r, pb6, pb7)"));
        assert!(main.contains("let _pa0 = gpioa.pa0.into_output(&mut r);"));
        assert!(main.contains("let pb6 = gpiob.pb6.into_i2c1_scl(&mut r);"));
        assert!(main.contains("use b::{Board, i2c1_speed, mmio::Mmio};"));
        let mut s = solved();
        set_ceiling(&mut s, "BUS", 1_000_000);
        let files = generate("b", &s, &table()).unwrap();
        assert!(files["firmware/src/main.rs"].contains("i2c1_speed::FastPlus1M, _"));
        // The memory map is the header's, and no file of the firmware runs host code.
        let link = &files["firmware/link.x"];
        assert!(link.contains("FLASH : ORIGIN = 0x08000000, LENGTH = 0x100000"));
        assert!(link.contains("RAM   : ORIGIN = 0x20000000, LENGTH = 128K"));
        assert!(files["firmware/Cargo.toml"].contains("b = { path = \"..\" }"));
        assert!(files["firmware/Cargo.lock"].contains("name = \"b-firmware\""));
        assert!(!files.keys().any(|k| k.ends_with("build.rs")));
    }

    fn spi_device(index: u32, peripheral: &str, cs: &str) -> aethelgard_ir::SolvedSpiDevice {
        aethelgard_ir::SolvedSpiDevice {
            index,
            part: "W25Q64JVxIQ".into(),
            bus: format!("{peripheral}_BUS"),
            peripheral: peripheral.into(),
            cs: cs.into(),
        }
    }

    /// The reference's SPI line facts: 20 MHz, 30 pF, 20 ns, 3135 mV.
    fn spi_line(signal: &str) -> aethelgard_ir::SpiLine {
        aethelgard_ir::SpiLine {
            bus: "SPI1_BUS".into(),
            signal: signal.into(),
            rate_hz: 20_000_000,
            capacitance_femtofarads: 30_000,
            rise_time_ns: 20,
            vdd_min_millivolts: 3135,
        }
    }

    /// `AET-WO-4300c` §1.4: SCK and MOSI take the slowest speed whose Fmax
    /// meets the bus's rate at its declared load (`01` at 20 MHz and 30 pF);
    /// MISO, a chip select and I2C take `00`, each with its reason.
    #[test]
    fn each_pin_takes_the_slowest_speed_its_line_allows() {
        let s = with_spi(&[("U4", "CS", "PA0")]);
        let speeds = pin_speeds(&s, &table()).unwrap();
        let by_pin = |pin: &str| speeds.iter().find(|p| p.pin == pin).unwrap();
        for pin in ["PA5", "PA7"] {
            let p = by_pin(pin);
            assert_eq!(p.ospeedr, 1, "{pin}");
            let SpeedCheck::Judged { row, fmax_hz, .. } = &p.check else {
                panic!("{pin} is judged");
            };
            assert_eq!((row.load_picofarads, *fmax_hz), (30, 80_000_000));
        }
        for (pin, reason) in [
            ("PA6", MASTER_INPUT),
            ("PA0", SOFTWARE_DRIVEN),
            ("PB6", OPEN_DRAIN),
            ("PB7", OPEN_DRAIN),
        ] {
            let p = by_pin(pin);
            assert_eq!(p.ospeedr, 0, "{pin}");
            assert_eq!(p.check, SpeedCheck::OutOfScope { reason }, "{pin}");
        }
        assert_eq!(speeds.len(), 6);
        // At 7 MHz `00` suffices (12 MHz at 30 pF).
        let mut slow = s.clone();
        for line in slow.spi_lines.values_mut() {
            line.rate_hz = 7_000_000;
        }
        let speeds = pin_speeds(&slow, &table()).unwrap();
        assert!(speeds.iter().all(|p| p.ospeedr == 0));
    }

    /// The generated register write and the oracle both carry the chosen
    /// speed: PA5 and PA7 at `01` (OSPEEDR bits 10-11 and 14-15), PA6 at `00`.
    #[test]
    fn the_generated_crate_and_its_oracle_carry_the_chosen_speed() {
        let files = generate("b", &with_spi(&[("U4", "CS", "PA0")]), &table()).unwrap();
        let lib = &files["src/board.rs"];
        assert!(lib.contains("Block::GpioA, 0x08, |v| (v & !0x00000c00) | 0x00000400);"));
        assert!(lib.contains("Block::GpioA, 0x08, |v| (v & !0x0000c000) | 0x00004000);"));
        assert!(lib.contains("Block::GpioA, 0x08, |v| v & !0x00003000);"));
        assert!(!lib.contains("0x08, |v| (v & !0x00000c00) | 0x00000800);"));
        let proofs = &files["src/proofs.rs"];
        assert!(
            proofs.contains("& 0x00000c00 == 0x00000400"),
            "PA5 in the oracle"
        );
        assert!(
            proofs.contains("& 0x00003000 == 0x00000000"),
            "PA6 in the oracle"
        );
    }

    /// §1.4: a line whose rate needs `10` or `11`, or whose load is above the
    /// table, is refused by name; nothing is sped up.
    #[test]
    fn a_line_no_selectable_speed_serves_is_refused() {
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        for line in s.spi_lines.values_mut() {
            line.rate_hz = 80_000_001;
        }
        let err = generate("b", &s, &table()).unwrap_err();
        assert_eq!(err.kind(), "RateAbovePinSpeed", "{err}");
        assert!(err.to_string().contains("`MOSI` on `PA7`"), "{err}");
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        s.spi_lines.get_mut("MOSI").unwrap().capacitance_femtofarads = 50_001;
        let err = generate("b", &s, &table()).unwrap_err();
        assert_eq!(err.kind(), "LoadAboveSpeedTable", "{err}");
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        s.spi_lines.get_mut("MOSI").unwrap().rise_time_ns = 4;
        assert_eq!(
            generate("b", &s, &table()).unwrap_err().kind(),
            "RiseTimeBelowPinSpeed"
        );
        for line in s.spi_lines.values_mut() {
            line.rise_time_ns = 20;
            line.vdd_min_millivolts = 1600;
        }
        assert_eq!(
            generate("b", &s, &table()).unwrap_err().kind(),
            "SupplyOutsideSpeedTable"
        );
    }

    /// [`solved`], with SPI1 on PA5, PA6 and PA7 and the devices `devices`
    /// (reference, chip-select net, pin) on it.
    fn with_spi(devices: &[(&str, &str, &str)]) -> SolvedTopology {
        let mut s = solved();
        for (net, pin, signal) in [
            ("SCK", "PA5", "SCK"),
            ("MISO", "PA6", "MISO"),
            ("MOSI", "PA7", "MOSI"),
        ] {
            s.assignments.insert(
                net.into(),
                assign(pin, "SPI1", &format!("SPI1_{signal}"), 5),
            );
            if signal != "MISO" {
                s.spi_lines.insert(net.into(), spi_line(signal));
            }
        }
        s.gpio.clear();
        for (i, (reference, net, pin)) in devices.iter().enumerate() {
            s.gpio.insert((*net).into(), (*pin).into());
            s.spi_devices.insert(
                (*reference).into(),
                spi_device(u32::try_from(i).unwrap(), "SPI1", net),
            );
        }
        s
    }

    /// The generated `impl` block that starts with `head`.
    fn block<'a>(text: &'a str, head: &str) -> &'a str {
        let at = text.find(head).unwrap_or_else(|| panic!("no `{head}`"));
        let rest = &text[at..];
        &rest[..rest.find("\n}\n").unwrap()]
    }

    #[test]
    fn spi_data_registers_are_reached_one_byte_at_a_time() {
        let files = generate("b", &with_spi(&[("U4", "CS", "PA0")]), &table()).unwrap();
        let lib = &files["src/board.rs"];
        let mmio = block(lib, "pub mod mmio {");
        assert!(mmio.contains("read_volatile((base(block) + offset) as *const u8)"));
        assert!(mmio.contains("write_volatile((base(block) + offset) as *mut u8, value)"));
        // TXDR (0x20) and RXDR (0x30) are reached only in `spi_frame`, and
        // there only through the byte accessors.
        let frame = block(lib, "fn spi_frame<");
        assert!(frame.contains("r.write8(TOKEN, block, 0x20, byte);"));
        assert!(frame.contains("return Some(r.read8(TOKEN, block, 0x30));"));
        for line in frame
            .lines()
            .filter(|l| l.contains("0x20") || l.contains("0x30"))
        {
            assert!(
                line.contains("write8(") || line.contains("read8("),
                "{line}"
            );
        }
        assert!(!lib.contains("Block::Spi1, 0x20") && !lib.contains("Block::Spi1, 0x30"));
        let transfer = block(lib, "impl Spi1<Enabled<()>> {");
        assert!(!transfer.contains("0x20") && !transfer.contains("0x30"));
        // `write_byte`, a 32-bit TXDR write, is I2C's only.
        assert!(!block(lib, "impl Spi1<Enabled<()>> {").contains("write_byte"));
        assert!(
            block(lib, "impl<S: i2c1_speed::AtMostCeiling> I2c1<Enabled<S>> {")
                .contains("pub fn write_byte")
        );
    }

    #[test]
    fn spi_init_takes_every_select_and_drives_it_high() {
        let s = with_spi(&[("U4", "CS", "PA0"), ("u_4", "CS2", "PA1")]);
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        let selects = block(lib, "pub struct Spi1Selects {");
        assert!(selects.contains("pub pa0: PA0<Output>,"));
        assert!(selects.contains("pub pa1: PA1<Output>,"));
        let init = block(lib, "impl Spi1<Clocked> {");
        assert!(init.contains("_selects: Spi1Selects,"));
        // Both high (BSRR set) before anything else of the bus.
        let pa0 = init.find("Block::GpioA, 0x18, 0x00000001);").unwrap();
        let pa1 = init.find("Block::GpioA, 0x18, 0x00000002);").unwrap();
        let spi = init.find("Block::Spi1").unwrap();
        assert!(pa0 < spi && pa1 < spi);
        // Each device's own chip select, and no other, by its variant.
        let transfer = block(lib, "impl Spi1<Enabled<()>> {");
        assert!(transfer.contains("Spi1Device::U4 => (Block::GpioA, 0x00010000, 0x00000001),"));
        assert!(transfer.contains("Spi1Device::U_4 => (Block::GpioA, 0x00020000, 0x00000002),"));
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains("Spi1Device::U4 => (Block::GpioA, 0x00010000, 0x00000001),"));
        assert!(proofs.contains("Spi1Device::U_4 => (Block::GpioA, 0x00020000, 0x00000002),"));
        assert!(proofs.contains("let devices = [Spi1Device::U4, Spi1Device::U_4];"));
        // The firmware hands both over in `init`, each bound after its `into_output`.
        let main = &files["firmware/src/main.rs"];
        assert!(main.contains("let pa1 = gpioa.pa1.into_output(&mut r);"));
        assert!(
            main.contains(
                "let _spi1 = spi1.init(&mut r, pa5, pa7, pa6, Spi1Selects { pa0, pa1 });"
            )
        );
    }

    #[test]
    fn device_variants_are_the_references_upper_cased() {
        // Distinct in any case (`validate`), so distinct upper-cased; and no
        // upper-case word is a keyword, which `r#` cannot escape for these.
        let refs = ["Self", "crate", "super", "fn"];
        let pins = ["PA0", "PA1", "PA2", "PA3"];
        let nets = ["CS0", "CS1", "CS2", "CS3"];
        let devices: Vec<(&str, &str, &str)> =
            (0..4).map(|i| (refs[i], nets[i], pins[i])).collect();
        let files = generate("b", &with_spi(&devices), &table()).unwrap();
        let device = block(&files["src/board.rs"], "pub enum Spi1Device {");
        for v in ["SELF", "CRATE", "SUPER", "FN"] {
            assert!(device.contains(&format!("    {v},")), "{v}");
        }
    }

    #[test]
    fn a_bus_without_devices_keeps_its_init_and_has_no_transfer() {
        let mut s = with_spi(&[]);
        s.gpio.insert("CS".into(), "PA0".into());
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        let init = block(lib, "impl Spi1<Clocked> {");
        assert!(init.contains("_miso: PA6<AlternateFunction<5>>,\n    ) -> Spi1<Enabled<()>> {"));
        assert!(!lib.contains("Spi1Selects") && !lib.contains("SpiError"));
        assert!(!files["src/proofs.rs"].contains("Spi1Spy"));
        let main = &files["firmware/src/main.rs"];
        assert!(main.contains("let _pa0 = gpioa.pa0.into_output(&mut r);"));
        assert!(main.contains("let _spi1 = spi1.init(&mut r, pa5, pa7, pa6);"));
    }

    #[test]
    fn each_bus_transfers_only_to_its_own_devices() {
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        for (net, pin, signal) in [
            ("SCK2", "PB13", "SCK"),
            ("MISO2", "PB14", "MISO"),
            ("MOSI2", "PB15", "MOSI"),
        ] {
            s.assignments.insert(
                net.into(),
                assign(pin, "SPI2", &format!("SPI2_{signal}"), 5),
            );
            if signal != "MISO" {
                let line = aethelgard_ir::SpiLine {
                    bus: "SPI2_BUS".into(),
                    ..spi_line(signal)
                };
                s.spi_lines.insert(net.into(), line);
            }
        }
        s.gpio.insert("CS2".into(), "PA1".into());
        s.spi_devices
            .insert("U5".into(), spi_device(1, "SPI2", "CS2"));
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        for (t, other) in [("Spi1", "Spi2"), ("Spi2", "Spi1")] {
            let transfer = block(lib, &format!("impl {t}<Enabled<()>> {{"));
            assert!(transfer.contains(&format!("device: {t}Device,")));
            assert!(!transfer.contains(&format!("{other}Device")), "{t}");
        }
        assert!(block(lib, "pub struct Spi2Selects {").contains("pub pa1: PA1<Output>,"));
    }

    #[test]
    fn a_chip_select_shared_by_two_devices_is_refused() {
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        s.spi_devices
            .insert("U5".into(), spi_device(1, "SPI1", "CS"));
        let err = generate("b", &s, &table()).unwrap_err();
        assert_eq!(
            err,
            BoardError::SharedChipSelect {
                pin: "PA0".into(),
                first: "U4".into(),
                reference: "U5".into(),
            }
        );
    }

    /// The PWM motor fixture's plan and facts, from its solved topology as
    /// `solve` writes it (`tests/ui/motors-pwm/topology.json`, held equal to
    /// `solve`'s output by `tests/motor_outputs.rs`): a generator reads a
    /// topology, never a specification (`RUL-16`).
    fn motor_plan() -> (Plan, Facts) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/motors-pwm/topology.json");
        let solved: SolvedTopology = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let plan = Plan::new("b", &solved, &table()).unwrap();
        let facts = Facts::new(&plan, &cmsis::register_facts().unwrap()).unwrap();
        (plan, facts)
    }

    #[test]
    fn footprints_that_share_a_bit_are_refused() {
        let fp = |mask: u32| Footprint::from([(("Rcc".to_owned(), 0xe0), mask)]);
        let owners = |mask: u32| {
            vec![
                ("the GPIOA enable".to_owned(), fp(0x1)),
                ("the i2c1 part".to_owned(), fp(mask)),
            ]
        };
        assert_eq!(disjoint(&owners(0x2)), Ok(()));
        assert_eq!(
            disjoint(&owners(0x3)),
            Err(BoardError::FootprintOverlap {
                first: "the GPIOA enable".into(),
                second: "the i2c1 part".into(),
                register: "Rcc at 0xe0, bits 0x00000001".into(),
            })
        );
    }

    #[test]
    fn a_motor_plan_splits_into_disjoint_parts_that_schedule_every_action() {
        let (plan, facts) = motor_plan();
        let mut parts = parts(&plan);
        let names: Vec<&str> = parts.iter().filter_map(|p| p.part.as_deref()).collect();
        assert_eq!(names, ["i2c1", "spi1", "tim1"]);
        assert_eq!(check_parts(&plan, &facts, &parts), Ok(()));
        // The chip select is SPI1's; port A's enable is in both parts on it.
        assert!(parts[1].pins.contains("PA0"));
        assert_eq!(
            parts[2].actions(&plan)[..2],
            [Action::Port('A'), Action::Pin("PA10".into())]
        );
        // A part left out leaves its actions unscheduled.
        assert_eq!(
            check_parts(&plan, &facts, &parts[..2]),
            Err(BoardError::UnscheduledAction {
                action: "the PA10 configure".into(),
            })
        );
        // A pin in two parts puts its fields in two footprints.
        parts[2].pins.insert("PA0".into());
        assert_eq!(
            check_parts(&plan, &facts, &parts),
            Err(BoardError::FootprintOverlap {
                first: "the spi1 part".into(),
                second: "the tim1 part".into(),
                register: "GpioA at 0x0, bits 0x00000003".into(),
            })
        );
    }

    #[test]
    fn a_board_without_timers_keeps_the_whole_schedule_harness() {
        let proofs = &generate("b", &solved(), &table()).unwrap()["src/proofs.rs"];
        assert!(proofs.contains("fn every_valid_sequence() {"));
        assert!(!proofs.contains("fn every_valid_sequence_"));
        assert!(!proofs.contains("fn rely("));
    }
}
