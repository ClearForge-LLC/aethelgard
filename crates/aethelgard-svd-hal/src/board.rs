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
//! (output type, speed, pull) are this generator's policy, named in
//! [`Role::policy`]. `I2C_TIMINGR` values are placeholders: clock-tree solving
//! is out of scope (WO §4).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

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
    #[error("peripheral `{peripheral}` is not one this generator emits (I2Cn, SPIn)")]
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
            Self::CeilingMismatch { .. } => "CeilingMismatch",
            Self::CeilingTooLow { .. } => "CeilingTooLow",
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
        emit_proofs(&plan, &facts, &speeds),
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
        }
    }

    /// The generator's policy for a role: (mode, output type, speed, pull).
    /// I2C is open-drain, low speed, no internal pull (the specification
    /// requires an external pull-up); SPI is push-pull at high speed; a GPIO
    /// output is push-pull, low speed, and starts high (an inactive chip select).
    fn policy(self) -> (u32, u32, u32, u32) {
        match self {
            Self::I2cScl | Self::I2cSda => (MODE_AF, 1, 0b00, 0b00),
            Self::SpiSck | Self::SpiMosi | Self::SpiMiso => (MODE_AF, 0, 0b10, 0b00),
            Self::Output => (MODE_OUTPUT, 0, 0b00, 0b00),
        }
    }
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
            let plan = PinPlan {
                port,
                index,
                role,
                af: u32::from(a.alternate_function),
                peripheral: Some(a.peripheral.clone()),
                net: net.clone(),
            };
            pins.insert(a.pin.clone(), plan);
            roles
                .entry(a.peripheral.clone())
                .or_insert_with(|| (kind, BTreeMap::new()))
                .1
                .entry(role)
                .or_default()
                .push(a.pin.clone());
        }
        for (net, pin) in &solved.gpio {
            let (port, index) = split_pin(pin)?;
            if !table.pins.contains_key(pin) {
                return Err(BoardError::UnknownPin { pin: pin.clone() });
            }
            let plan = PinPlan {
                port,
                index,
                role: Role::Output,
                af: 0,
                peripheral: None,
                net: net.clone(),
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

#[derive(Debug, Clone)]
struct Facts {
    rcc_base: u32,
    rcc_words: u32,
    gpio: GpioFacts,
    ports: BTreeMap<char, PortFacts>,
    pins: BTreeMap<String, PinFacts>,
    peripherals: BTreeMap<String, PeriphFacts>,
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
        Ok(Facts {
            rcc_base: f.base("RCC")?,
            rcc_words: rcc.size / 4,
            gpio,
            ports,
            pins,
            peripherals,
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
         //! - Pin data: `STM32_open_pin_data` at `{pins}`.\n",
        spec = plan.specification_sha256,
        repo = cmsis::REPOSITORY,
        commit = cmsis::COMMIT,
        file = cmsis::HEADER_FILE,
        hash = cmsis::HEADER_SHA256,
        pins = plan.pin_commit,
    )
}

fn emit_manifest(plan: &Plan) -> String {
    format!(
        "# @generated by aethelgard-svd-hal (AET-WO-1400). Not a workspace member: the\n\
         # repository root excludes `boards/`. No dependencies, so `aethelgard prove`\n\
         # can judge it with no network.\n\
         [package]\n\
         name = \"{name}\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         license = \"MIT OR Apache-2.0\"\n\
         \n\
         [lints.rust]\n\
         unexpected_cfgs = {{ level = \"warn\", check-cfg = [\"cfg(kani)\"] }}\n",
        name = plan.crate_name
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
        let (mode, otype, ospeed, pupd) = p.role.policy();
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
    let _ = writeln!(w, "        }})\n    }}\n}}");
    s
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

fn emit_proofs(plan: &Plan, facts: &Facts, speeds: &Speeds) -> String {
    let g = &facts.gpio;
    let mut s = header(
        plan,
        "The Kani harness: the specification's register rules over an image this\n\
         //! harness owns. Every number below is computed by the generator from the\n\
         //! solved topology and the register source, not read from `lib.rs`.",
    );
    let w = &mut s;
    // Blocks: (Block variant, image field, words, gate).
    let mut blocks: Vec<(String, String, u32, Option<&Gate>)> =
        vec![("Rcc".into(), "rcc".into(), facts.rcc_words, None)];
    for port in &plan.ports {
        blocks.push((
            port_type(*port),
            port_field(*port),
            g.words,
            Some(&facts.ports[port].gate),
        ));
    }
    for (name, pf) in &facts.peripherals {
        blocks.push((
            type_name(name),
            name.to_ascii_lowercase(),
            pf.words,
            Some(&pf.gate),
        ));
    }
    let actions = plan.ports.len() + 2 * plan.peripherals.len() + plan.pins.len();
    let unwind = blocks
        .iter()
        .map(|b| b.2 as usize)
        .max()
        .unwrap_or(0)
        .max(actions)
        + 1;

    let _ = writeln!(
        w,
        "\n\
         use crate::board::sealed;\n\
         use crate::*;\n\
         \n\
         /// The harness-owned register image, and the oracle for the ordering rules.\n\
         #[derive(Clone, Copy)]\n\
         struct Image {{"
    );
    for (_, field, words, _) in &blocks {
        let _ = writeln!(w, "    {field}: [u32; {words}],");
    }
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         impl Image {{\n    \
             fn slot(&mut self, block: Block, offset: usize) -> &mut u32 {{\n        \
                 let words: &mut [u32] = match block {{"
    );
    for (variant, field, _, _) in &blocks {
        let _ = writeln!(w, "            Block::{variant} => &mut self.{field},");
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
    let _ = writeln!(
        w,
        "        }}\n    \
         }}\n\
         }}\n\
         \n\
         impl sealed::Sealed for Image {{}}\n\
         \n\
         impl Registers for Image {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 self.clocked(block);\n        \
                 *self.slot(block, offset)\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 self.clocked(block);"
    );
    for (name, pf) in &facts.peripherals {
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
    // already holds its final value (an output's level already high).
    let [mw, tw, sw, pw, aw] = g.widths;
    for port in &plan.ports {
        let t = port_type(*port);
        let field = port_field(*port);
        let on_port: Vec<(&String, &PinPlan)> =
            plan.pins.iter().filter(|(_, p)| p.port == *port).collect();
        let allowed: Vec<String> = on_port
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
        let _ = writeln!(
            w,
            "        if block == Block::{t} && offset == {bsrr:#04x} {{\n            \
                 kani::assert({rule}, \"{t} BSRR write touches a pin no solved output owns\");\n            \
                 // The level it leaves: a set bit wins over a reset bit (RM0433 §11.4.7).\n            \
                 self.{field}[{oi}] = (self.{field}[{oi}] & !(value >> 16)) | (value & 0xffff);\n        \
             }}\n        \
             if block == Block::{t} && offset == {moder:#04x} {{\n            \
                 let old = self.{field}[{mi}];",
            bsrr = g.bsrr,
            oi = g.odr / 4,
            moder = g.moder,
            mi = g.moder / 4,
        );
        let mut solved_mask = 0u32;
        for (pin, p) in &on_port {
            let pf = &facts.pins[*pin];
            let (mode, otype, ospeed, pupd) = p.role.policy();
            let mmask = oracle_mask(mw, pf.moder);
            solved_mask |= mmask;
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

    // The final-state oracle.
    let timingr_params: Vec<String> = plan
        .peripherals
        .iter()
        .filter(|(_, p)| p.kind == Kind::I2c)
        .map(|(n, _)| format!(", {}_timingr: u32", n.to_ascii_lowercase()))
        .collect();
    let _ = writeln!(
        w,
        "/// The state the specification allows, from any starting image.\n\
         fn check_final(before: &Image, after: &Image{}) {{",
        timingr_params.concat()
    );
    for port in &plan.ports {
        let field = port_field(*port);
        let mut expect: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let [mw, tw, sw, pw, aw] = g.widths;
        let mut touched: BTreeSet<u32> = BTreeSet::new();
        for (pin, p) in plan.pins.iter().filter(|(_, p)| p.port == *port) {
            let pf = &facts.pins[pin];
            let (mode, otype, ospeed, pupd) = p.role.policy();
            expect_field(&mut expect, g.moder, pf.moder, mw, mode);
            expect_field(&mut expect, g.otyper, pf.otyper, tw, otype);
            expect_field(&mut expect, g.ospeedr, pf.ospeedr, sw, ospeed);
            expect_field(&mut expect, g.pupdr, pf.pupdr, pw, pupd);
            if p.role == Role::Output {
                touched.insert(g.bsrr);
            } else {
                expect_field(&mut expect, g.afr[pf.afr_index as usize], pf.afr, aw, p.af);
            }
        }
        for (reg, (mask, value)) in &expect {
            let i = reg / 4;
            let _ = writeln!(
                w,
                "    kani::assert(\n        \
                     after.{field}[{i}] & {mask:#010x} == {value:#010x},\n        \
                     \"{t} word {i}: a solved pin's field is not what the specification fixes\",\n    \
                 );\n    \
                 kani::assert(\n        \
                     after.{field}[{i}] & !{mask:#010x} == before.{field}[{i}] & !{mask:#010x},\n        \
                     \"{t} word {i}: an unsolved pin's field changed\",\n    \
                 );",
                t = port_type(*port),
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
                if plan.is_select(pin) {
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
    // RCC: before plus exactly the gates.
    let mut gates: BTreeMap<u32, u32> = BTreeMap::new();
    for gate in facts
        .ports
        .values()
        .map(|p| &p.gate)
        .chain(facts.peripherals.values().map(|p| &p.gate))
    {
        *gates.entry(gate.offset / 4).or_default() |= 1u32 << gate.bit;
    }
    let _ = writeln!(w, "    let mut rcc = before.rcc;");
    for (word, mask) in &gates {
        let _ = writeln!(w, "    rcc[{word}] |= {mask:#010x};");
    }
    let _ = writeln!(
        w,
        "    // Word by word: `==` on the arrays is a `memcmp` loop under CBMC.\n    \
         for i in 0..{n} {{\n        \
             kani::assert(after.rcc[i] == rcc[i], \"RCC is not before plus exactly the solved clock gates\");\n    \
         }}",
        n = facts.rcc_words
    );
    for (name, pf) in &facts.peripherals {
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
    let _ = writeln!(w, "}}\n");

    // The scheduler.
    let _ = writeln!(
        w,
        "/// Every order the typestate API admits: at each of {actions} steps, any\n\
         /// action whose inputs exist. The `assume` admits only those orders; the\n\
         /// types forbid the rest at compile time. Each I2C peripheral writes zero or\n\
         /// one byte when it starts; each SPI peripheral takes its bus's chip selects;\n\
         /// and the output pins are driven zero or one time first.\n\
         #[kani::proof]\n\
         #[kani::unwind({unwind})]\n\
         fn every_valid_sequence() {{\n    \
             let mut img = Image {{"
    );
    for (_, field, _, _) in &blocks {
        let _ = writeln!(w, "        {field}: kani::any(),");
    }
    let _ = writeln!(
        w,
        "    }};\n    \
         let before = img;\n    \
         let board = Board::take().unwrap();\n    \
         let mut rcc = board.rcc;"
    );
    for port in &plan.ports {
        let _ = writeln!(
            w,
            "    let mut {f} = Some(board.{f});",
            f = port_field(*port)
        );
    }
    for name in plan.peripherals.keys() {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "    let mut {f} = Some(board.{f});\n    let mut {f}_clocked = None;\n    let mut {f}_done = false;"
        );
    }
    for (name, p) in &plan.peripherals {
        if p.kind == Kind::I2c {
            let _ = writeln!(
                w,
                "    let mut {}_timingr = 0u32;",
                name.to_ascii_lowercase()
            );
        }
    }
    for pin in plan.pins.keys() {
        let f = pin_field(pin);
        let _ = writeln!(w, "    let mut {f} = None;\n    let mut {f}_done = None;");
    }
    let _ = writeln!(
        w,
        "    for _ in 0..{actions} {{\n        \
             let action: u8 = kani::any();\n        \
             match action {{"
    );
    let mut a = 0usize;
    for port in &plan.ports {
        let f = port_field(*port);
        let pins: Vec<String> = plan
            .pins
            .keys()
            .filter(|n| plan.pins[*n].port == *port)
            .map(|n| pin_field(n))
            .collect();
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());\n                \
                 let pins = rcc.enable_{f}(&mut img, {f}.take().unwrap()).split();"
        );
        for p in &pins {
            let _ = writeln!(w, "                {p} = Some(pins.{p});");
        }
        let _ = writeln!(w, "            }}");
        a += 1;
    }
    for name in plan.peripherals.keys() {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());\n                \
                 {f}_clocked = Some(rcc.enable_{f}(&mut img, {f}.take().unwrap()));\n            \
             }}"
        );
        a += 1;
    }
    for (pin, p) in &plan.pins {
        let f = pin_field(pin);
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some());\n                \
                 let {mutable}pin = {f}.take().unwrap().{m}(&mut img);",
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
    for (name, p) in &plan.peripherals {
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
                 kani::assume({f}_clocked.is_some() && {});\n                \
                 let i = {f}_clocked.take().unwrap();",
            ready.join(" && ")
        );
        // A byte written to a peripheral must not touch any output's level.
        let bsrr_ports: Vec<String> = plan
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
    let _ = writeln!(
        w,
        "            _ => kani::assume(false),\n        }}\n    }}"
    );
    for name in plan.peripherals.keys() {
        let f = name.to_ascii_lowercase();
        let _ = writeln!(
            w,
            "    kani::assert({f}_done, \"the schedule did not start {name}\");"
        );
    }
    // A chip select is configured before its bus starts, which takes it.
    for (pin, p) in &plan.pins {
        if p.role == Role::Output && !plan.is_select(pin) {
            let f = pin_field(pin);
            let _ = writeln!(
                w,
                "    kani::assert({f}_done.is_some(), \"the schedule did not configure {pin}\");"
            );
        }
    }
    let timingr_args: Vec<String> = plan
        .peripherals
        .iter()
        .filter(|(_, p)| p.kind == Kind::I2c)
        .map(|(n, _)| format!(", {}_timingr", n.to_ascii_lowercase()))
        .collect();
    let _ = writeln!(
        w,
        "    check_final(&before, &img{});\n    \
         kani::cover!(true, \"the end of a complete schedule is reachable\");\n\
         }}",
        timingr_args.concat()
    );
    emit_transfer_proofs(w, plan, facts, speeds);
    emit_spi_transfer_proofs(w, plan, facts);
    s
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
                 out: [u8; 2],\n    \
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
        let _ = writeln!(
            w,
            "\n\
             /// A transfer of any device, up to two bytes out and three in (an identity\n\
             /// read is one and three) and any answers, with each wait bounded at two reads\n\
             /// of `SR`: selects that device before its first frame and deselects it after\n\
             /// the peripheral is disabled, also after a wait that expired; reads `RXDR`\n\
             /// only after `RXP`; and returns `Ok` only when every frame moved.\n\
             #[kani::proof]\n\
             #[kani::unwind(5)]\n\
             fn {m}_transfer_is_bounded_and_ordered() {{\n    \
                 let mut img = Image {{\n\
             {setup}    \
                 let (wl, rl): (usize, usize) = (kani::any(), kani::any());\n    \
                 kani::assume(wl <= 2 && rl <= 3);\n    \
                 let out: [u8; 2] = kani::any();\n    \
                 let mut back = [0u8; 3];\n    \
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
                         kani::cover!(wl == 1 && rl == 3, \"{t} made an identity read's frames\");\n        \
                     }}\n        \
                     Err(SpiError::Rxp) => {{\n            \
                         kani::assert(spy.received < frames, \"Rxp after every frame moved\");\n            \
                         kani::cover!(true, \"{t} timed out waiting for RXP\");\n        \
                     }}\n        \
                     Err(SpiError::Length) => kani::assert(false, \"a transfer of at most five frames was refused\"),\n    \
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
                 let (wl, out) = (1, [0u8; 2]);\n    \
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
        let bit = |pos: u32| format!("{:#010x}", 1u32 << pos);
        let (_, admitted) = &speeds[name];
        let mode = I2C_MODES[admitted[0]].0;
        let pins: Vec<String> = p.pins.values().map(|n| pin_field(n)).collect();
        let ports: BTreeSet<char> = p.pins.values().map(|n| plan.pins[n].port).collect();
        let mut blocks: Vec<String> = vec!["rcc".into()];
        blocks.extend(plan.ports.iter().map(|p| port_field(*p)));
        blocks.extend(plan.peripherals.keys().map(|n| n.to_ascii_lowercase()));
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
             /// A transfer of any address, up to two bytes each way and any answers, with\n\
             /// each wait bounded at two reads of `ISR`: touches only {t}, in order, within\n\
             /// the bound, and ends in the result its phases imply.\n\
             #[kani::proof]\n\
             #[kani::unwind(6)]\n\
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
             kani::assume(wl <= 2 && rl <= 2);\n    \
             let out: [u8; 2] = kani::any();\n    \
             let mut back = [0u8; 2];\n    \
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
                 Err(I2cError::Length) => kani::assert(false, \"a transfer of at most two bytes was refused as too long\"),\n    \
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
            schema_version: 3,
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
            part: "W25Q64JV".into(),
            bus: format!("{peripheral}_BUS"),
            peripheral: peripheral.into(),
            cs: cs.into(),
        }
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
}
