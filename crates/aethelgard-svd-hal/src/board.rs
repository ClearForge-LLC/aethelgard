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
        "device `{reference}` is on `{peripheral}`, which is not an I2C peripheral of this board"
    )]
    DeviceNotOnABus {
        reference: String,
        peripheral: String,
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
/// The I2C speeds each peripheral admits come from `solved.i2c_ceilings`
/// (`AET-WO-1202`), keyed by bus id. The topology does not say which
/// peripheral serves which bus, so the generator needs one ceiling per I2C
/// peripheral and gives every I2C peripheral the lowest of them: exact for one
/// bus, and never above any bus's ceiling for more.
pub fn generate(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, BoardError> {
    generate_with(crate_name, solved, table, &|_| false)
}

/// [`generate`], and the reference firmware also reads the identity register
/// of each of the topology's I2C devices (`SolvedTopology::i2c_devices`,
/// `AET-WO-2203`) whose part has an identity and for which `answers` says the
/// twin has a model (`AET-WO-2202`). This crate cannot depend on the twin, so
/// the caller passes `aethelgard_twin::model::model(part).is_some()`.
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
    if let Some(&hz) = solved.i2c_ceilings.values().min() {
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
        let ports = pins.values().map(|p| p.port).collect();
        Ok(Plan {
            crate_name: crate_name.into(),
            specification_sha256: solved.specification_sha256.clone(),
            pin_commit: table.source.commit.clone(),
            pins,
            peripherals,
            ports,
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
         pub trait Registers: sealed::Sealed {{\n    \
             fn read(&mut self, token: sealed::Token, block: Block, offset: usize) -> u32;\n    \
             fn write(&mut self, token: sealed::Token, block: Block, offset: usize, value: u32);\n    \
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
            "/// The speeds at or under `{name}`'s solved rise-time ceiling of {hz} Hz.\n\
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
                let _ = writeln!(
                    w,
                    "impl {t}<Clocked> {{\n    \
                         /// Master mode; baud rate and frame format are not solved yet.\n    \
                         pub fn init<R: Registers>(\n        \
                             self,\n        \
                             r: &mut R,\n        \
                             {params},\n    \
                         ) -> {t}<Enabled<()>> {{\n        \
                             r.modify(TOKEN, Block::{block}, {cr1:#04x}, |v| v & !{enable:#010x});\n        \
                             r.modify(TOKEN, Block::{block}, {cfg:#04x}, |v| v | {master:#010x});\n        \
                             r.modify(TOKEN, Block::{block}, {cr1:#04x}, |v| v | {enable:#010x});\n        \
                             {t} {{ _state: PhantomData }}\n    \
                         }}\n\
                     }}\n\
                     \n\
                     impl {t}<Enabled<()>> {{",
                    params = params.join(",\n        "),
                    cr1 = pf.cr1,
                    cfg = pf.config,
                );
            }
        }
        let _ = writeln!(
            w,
            "    pub fn write_byte<R: Registers>(&mut self, r: &mut R, byte: u8) {{\n        \
                 r.write(TOKEN, Block::{block}, {txdr:#04x}, u32::from(byte));\n    \
             }}\n\
             }}\n",
            txdr = pf.txdr
        );
        if let Some(i) = &pf.i2c {
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
                 kani::assert({rule}, \"{t} BSRR write touches a pin no solved output owns\");\n        \
             }}\n        \
             if block == Block::{t} && offset == {moder:#04x} {{\n            \
                 let old = self.{field}[{mi}];",
            bsrr = g.bsrr,
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
        "        *self.slot(block, offset) = value;\n    }}\n}}\n"
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
        let _ = writeln!(
            w,
            "    kani::assert(after.{field}[{cr1}] & {en:#010x} != 0, \"{t} is not enabled\");\n    \
             kani::assert(\n        \
                 after.{field}[{cr1}] & !{en:#010x} == before.{field}[{cr1}] & !{en:#010x},\n        \
                 \"{t} CR1 changed beyond its enable bit\",\n    \
             );",
            en = 1u32 << pf.enable
        );
        let written = [pf.cr1, pf.config, pf.txdr];
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
         /// types forbid the rest at compile time. Each peripheral writes zero or one\n\
         /// byte when it starts, and the output pins are driven zero or one time.\n\
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
        let ready: Vec<String> = pins.iter().map(|n| format!("{n}_done.is_some()")).collect();
        let args: Vec<String> = pins
            .iter()
            .map(|n| format!("{n}_done.take().unwrap()"))
            .collect();
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
            Kind::Spi => start(w, "_", "", "                "),
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
    for (pin, p) in &plan.pins {
        if p.role == Role::Output {
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
    s
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
            schema_version: 1,
            specification_sha256: "0".repeat(64),
            assignments: BTreeMap::from([
                ("SCL".into(), assign("PB6", "I2C1", "I2C1_SCL", 4)),
                ("SDA".into(), assign("PB7", "I2C1", "I2C1_SDA", 4)),
            ]),
            gpio: BTreeMap::from([("CS".into(), "PA0".into())]),
            i2c_ceilings: BTreeMap::from([("BUS".into(), 400_000)]),
            analog_lines: analog_lines(&[("SCL", "BUS"), ("SDA", "BUS")]),
            i2c_devices: BTreeMap::new(),
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
    /// to the ceiling `solve` derives (`AET-WO-1204`).
    fn set_ceiling(s: &mut SolvedTopology, bus: &str, hz: u32) {
        let ns = match hz {
            100_000 => 1000,
            400_000 => 300,
            _ => 120,
        };
        s.i2c_ceilings.insert(bus.into(), hz);
        for line in s.analog_lines.values_mut().filter(|l| l.bus == bus) {
            line.ceiling_ns = ns;
        }
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
        assert_eq!(kind(generate("b", &s, &t)), "IncompletePeripheral");

        let mut s = solved();
        s.assignments
            .insert("SDA2".into(), assign("PB9", "I2C1", "I2C1_SDA", 4));
        s.analog_lines = analog_lines(&[("SCL", "BUS"), ("SDA", "BUS"), ("SDA2", "BUS")]);
        assert_eq!(kind(generate("b", &s, &t)), "IncompletePeripheral");

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
}
