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

use aethelgard_ir::{PinTable, SolvedTopology};

use crate::cmsis::{self, RegisterError, RegisterFacts};

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BoardError {
    #[error("`{name}` is not a crate name: lowercase ASCII letters, digits and single `-`")]
    InvalidCrateName { name: String },
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
    #[error("pin `{pin}` is assigned twice")]
    PinUsedTwice { pin: String },
    #[error("peripheral `{peripheral}` is not one this generator emits (I2Cn, SPIn)")]
    UnsupportedPeripheral { peripheral: String },
    #[error("signal `{signal}` is not a role of `{peripheral}`")]
    UnsupportedSignal { peripheral: String, signal: String },
    #[error("`{peripheral}` has its `{role}` signal {count} times; it needs exactly one")]
    IncompletePeripheral {
        peripheral: String,
        role: &'static str,
        count: usize,
    },
    #[error("the I2C ceilings name {got:?}, not exactly the I2C peripherals {expected:?}")]
    CeilingMismatch {
        expected: Vec<String>,
        got: Vec<String>,
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
            Self::InvalidPin { .. } => "InvalidPin",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::AssignmentMismatch { .. } => "AssignmentMismatch",
            Self::PinUsedTwice { .. } => "PinUsedTwice",
            Self::UnsupportedPeripheral { .. } => "UnsupportedPeripheral",
            Self::UnsupportedSignal { .. } => "UnsupportedSignal",
            Self::IncompletePeripheral { .. } => "IncompletePeripheral",
            Self::CeilingMismatch { .. } => "CeilingMismatch",
            Self::CeilingTooLow { .. } => "CeilingTooLow",
            Self::Register(e) => e.kind(),
        }
    }
}

/// The highest I2C bus speed one peripheral may run at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {
    /// The topology carries no ceiling yet (`AET-WO-1202` adds it). Every mode
    /// is emitted, and the generated trait says so: a recorded gap, not a
    /// default.
    NotSolved,
    /// The solver's maximum, in Hz.
    Hz(u32),
}

/// The I2C modes of UM10204, with placeholder `TIMINGR` values.
const I2C_MODES: [(&str, u32, u32); 3] = [
    ("Standard100k", 100_000, 0x1070_7DBC),
    ("Fast400k", 400_000, 0x0060_2173),
    ("FastPlus1M", 1_000_000, 0x0030_0B29),
];

/// The board crate for `crate_name`, from `solved`, with every I2C ceiling
/// [`Ceiling::NotSolved`] until the topology carries one.
pub fn generate(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, BoardError> {
    let plan = Plan::new(crate_name, solved, table)?;
    let ceilings = plan
        .peripherals
        .iter()
        .filter(|(_, p)| p.kind == Kind::I2c)
        .map(|(name, _)| (name.clone(), Ceiling::NotSolved))
        .collect();
    generate_with(plan, &ceilings)
}

/// As [`generate`], with each I2C peripheral's ceiling given.
pub fn generate_with_ceilings(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
    ceilings: &BTreeMap<String, Ceiling>,
) -> Result<BTreeMap<String, String>, BoardError> {
    generate_with(Plan::new(crate_name, solved, table)?, ceilings)
}

fn generate_with(
    plan: Plan,
    ceilings: &BTreeMap<String, Ceiling>,
) -> Result<BTreeMap<String, String>, BoardError> {
    let i2c: Vec<String> = plan
        .peripherals
        .iter()
        .filter(|(_, p)| p.kind == Kind::I2c)
        .map(|(n, _)| n.clone())
        .collect();
    let named: Vec<String> = ceilings.keys().cloned().collect();
    if named != i2c {
        return Err(BoardError::CeilingMismatch {
            expected: i2c,
            got: named,
        });
    }
    let mut speeds = BTreeMap::new();
    for (name, ceiling) in ceilings {
        let admitted: Vec<usize> = (0..I2C_MODES.len())
            .filter(|&i| match ceiling {
                Ceiling::NotSolved => true,
                Ceiling::Hz(hz) => I2C_MODES[i].1 <= *hz,
            })
            .collect();
        if admitted.is_empty() {
            let Ceiling::Hz(hz) = ceiling else {
                unreachable!("NotSolved admits every mode")
            };
            return Err(BoardError::CeilingTooLow { hz: *hz });
        }
        speeds.insert(name.clone(), (*ceiling, admitted));
    }
    let facts = Facts::new(&plan, &cmsis::register_facts()?)?;
    let mut files = BTreeMap::new();
    files.insert("Cargo.toml".to_owned(), emit_manifest(&plan));
    files.insert("Cargo.lock".to_owned(), emit_lock(&plan));
    files.insert("src/lib.rs".to_owned(), emit_root(&plan));
    files.insert("src/board.rs".to_owned(), emit_lib(&plan, &facts, &speeds));
    files.insert(
        "src/proofs.rs".to_owned(),
        emit_proofs(&plan, &facts, &speeds),
    );
    Ok(files)
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
    let bad = || BoardError::InvalidPin { pin: pin.into() };
    let rest = pin.strip_prefix('P').ok_or_else(bad)?;
    let mut chars = rest.chars();
    let port = chars
        .next()
        .filter(|c| ('A'..='K').contains(c))
        .ok_or_else(bad)?;
    let digits = chars.as_str();
    if digits.is_empty() || digits.len() > 2 || digits.starts_with('0') && digits.len() > 1 {
        return Err(bad());
    }
    let index: u32 = digits.parse().map_err(|_| bad())?;
    if index > 15 {
        return Err(bad());
    }
    Ok((port, index))
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
            if pins.insert(a.pin.clone(), plan).is_some() {
                return Err(BoardError::PinUsedTwice { pin: a.pin.clone() });
            }
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
            if pins.insert(pin.clone(), plan).is_some() {
                return Err(BoardError::PinUsedTwice { pin: pin.clone() });
            }
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
    let numbered = |prefix: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    if numbered("I2C") {
        Ok(Kind::I2c)
    } else if numbered("SPI") {
        Ok(Kind::Spi)
    } else {
        Err(BoardError::UnsupportedPeripheral {
            peripheral: name.into(),
        })
    }
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
type Speeds = BTreeMap<String, (Ceiling, Vec<usize>)>;

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
         /// The one door to the hardware. Offsets are byte offsets into the block.\n\
         pub trait Registers {{\n    \
             fn read(&mut self, block: Block, offset: usize) -> u32;\n    \
             fn write(&mut self, block: Block, offset: usize, value: u32);\n    \
             fn modify(&mut self, block: Block, offset: usize, f: impl FnOnce(u32) -> u32) {{\n        \
                 let value = self.read(block, offset);\n        \
                 self.write(block, offset, f(value));\n    \
             }}\n\
         }}\n\
         \n\
         /// Volatile access at the part's real base addresses. Never exercised by\n\
         /// the proof: Kani cannot own this memory (`architecture.md` §2.2).\n\
         pub mod mmio {{\n    \
             use super::{{Block, Registers}};\n\
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
         impl Registers for Mmio {{\n        \
             fn read(&mut self, block: Block, offset: usize) -> u32 {{\n            \
                 // SAFETY: a register of a peripheral that exists on this part.\n            \
                 unsafe {{ core::ptr::read_volatile((base(block) + offset) as *const u32) }}\n        \
             }}\n        \
             fn write(&mut self, block: Block, offset: usize, value: u32) {{\n            \
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
    for (name, (ceiling, admitted)) in speeds {
        let note = match ceiling {
            Ceiling::NotSolved => format!(
                "/// The topology carries no ceiling for `{name}` yet (`AET-WO-1202`), so\n\
                 /// every mode is admitted. This is a recorded gap, not a default."
            ),
            Ceiling::Hz(hz) => format!(
                "/// The speeds at or under `{name}`'s solved ceiling of {hz} Hz. Sealed:\n\
                 /// firmware cannot admit another."
            ),
        };
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
                "        r.modify(Block::{block}, {reg:#04x}, |v| {body});"
            );
        };
        field(w, g.otyper, pf.otyper, tw, otype);
        field(w, g.ospeedr, pf.ospeedr, sw, ospeed);
        field(w, g.pupdr, pf.pupdr, pw, pupd);
        if p.role == Role::Output {
            let _ = writeln!(
                w,
                "        r.write(Block::{block}, {:#04x}, {:#010x});",
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
                         r.write(Block::{block}, {bsrr:#04x}, {bs:#010x});\n    \
                     }}\n    \
                     pub fn set_low<R: Registers>(&mut self, r: &mut R) {{\n        \
                         r.write(Block::{block}, {bsrr:#04x}, {br:#010x});\n    \
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
                             r.modify(Block::{block}, {cr1:#04x}, |v| v & !{enable:#010x});\n        \
                             r.write(Block::{block}, {cfg:#04x}, S::TIMINGR);\n        \
                             r.modify(Block::{block}, {cr1:#04x}, |v| v | {enable:#010x});\n        \
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
                             r.modify(Block::{block}, {cr1:#04x}, |v| v & !{enable:#010x});\n        \
                             r.modify(Block::{block}, {cfg:#04x}, |v| v | {master:#010x});\n        \
                             r.modify(Block::{block}, {cr1:#04x}, |v| v | {enable:#010x});\n        \
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
                 r.write(Block::{block}, {txdr:#04x}, u32::from(byte));\n    \
             }}\n\
             }}\n",
            txdr = pf.txdr
        );
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
                 r.modify(Block::Rcc, {off:#05x}, |v| v | {mask:#010x});\n        \
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

/// The oracle's expectation for one GPIO register: (mask, value).
fn expect_field(acc: &mut BTreeMap<u32, (u32, u32)>, reg: u32, pos: u32, width: u32, value: u32) {
    let mask = ((1u32 << width) - 1) << pos;
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
         impl Registers for Image {{\n    \
             fn read(&mut self, block: Block, offset: usize) -> u32 {{\n        \
                 self.clocked(block);\n        \
                 *self.slot(block, offset)\n    \
             }}\n    \
             fn write(&mut self, block: Block, offset: usize, value: u32) {{\n        \
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
    let _ = writeln!(
        w,
        "        *self.slot(block, offset) = value;\n    }}\n}}\n"
    );

    // The final-state oracle.
    let _ = writeln!(
        w,
        "/// The state the specification allows, from any starting image.\n\
         fn check_final(before: &Image, after: &Image) {{"
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
            "    kani::assert(after.{field}[{cr1}] & {en:#010x} != 0, \"{t} is not enabled\");",
            en = 1u32 << pf.enable
        );
        let written = [pf.cr1, pf.config, pf.txdr];
        match plan.peripherals[name].kind {
            Kind::I2c => {
                let (_, admitted) = &speeds[name];
                let ok: Vec<String> = admitted
                    .iter()
                    .map(|&i| {
                        format!(
                            "after.{field}[{}] == {:#010x}",
                            pf.config / 4,
                            I2C_MODES[i].2
                        )
                    })
                    .collect();
                let _ = writeln!(
                    w,
                    "    kani::assert(\n        {},\n        \"{t} TIMINGR is not an admitted speed's\",\n    );",
                    ok.join("\n            || ")
                );
            }
            Kind::Spi => {
                let master = 1u32 << pf.master.unwrap_or_default();
                let _ = writeln!(
                    w,
                    "    kani::assert(after.{field}[{c}] & {master:#010x} != 0, \"{t} is not a master\");",
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
            let _ = writeln!(
                w,
                "                if kani::any() {{\n                    \
                     if kani::any() {{\n                        \
                         pin.set_low(&mut img);\n                    \
                     }} else {{\n                        \
                         pin.set_high(&mut img);\n                    \
                     }}\n                \
                 }}"
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
        let start = |w: &mut String, ty: &str, indent: &str| {
            let _ = writeln!(
                w,
                "{indent}let mut bus = i.init::<{ty}>(&mut img, {});\n\
                 {indent}if kani::any() {{\n\
                 {indent}    bus.write_byte(&mut img, kani::any());\n\
                 {indent}}}",
                args.join(", ")
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
                    start(w, &format!("{m}_speed::{mode}, _"), "                    ");
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
            Kind::Spi => start(w, "_", "                "),
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
    let _ = writeln!(
        w,
        "    check_final(&before, &img);\n    \
         kani::cover!(true, \"the end of a complete schedule is reachable\");\n\
         }}"
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethelgard_ir::PinAssignment;

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
        }
    }

    fn kind(result: Result<BTreeMap<String, String>, BoardError>) -> &'static str {
        result.unwrap_err().kind()
    }

    #[test]
    fn a_solved_ceiling_admits_only_the_modes_under_it() {
        let ceilings = BTreeMap::from([("I2C1".into(), Ceiling::Hz(400_000))]);
        let files = generate_with_ceilings("b", &solved(), &table(), &ceilings).unwrap();
        let lib = &files["src/board.rs"];
        assert!(lib.contains("impl AtMostCeiling for Standard100k"));
        assert!(lib.contains("impl AtMostCeiling for Fast400k"));
        assert!(!lib.contains("impl AtMostCeiling for FastPlus1M"));
        assert!(lib.contains("solved ceiling of 400000 Hz"));
        // The harness drives exactly the admitted speeds.
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains("i2c1_speed::Fast400k"));
        assert!(!proofs.contains("FastPlus1M"));
        let low = BTreeMap::from([("I2C1".into(), Ceiling::Hz(99_999))]);
        assert_eq!(
            kind(generate_with_ceilings("b", &solved(), &table(), &low)),
            "CeilingTooLow"
        );
        let none = BTreeMap::new();
        assert_eq!(
            kind(generate_with_ceilings("b", &solved(), &table(), &none)),
            "CeilingMismatch"
        );
        let extra = BTreeMap::from([
            ("I2C1".into(), Ceiling::NotSolved),
            ("I2C2".into(), Ceiling::NotSolved),
        ]);
        assert_eq!(
            kind(generate_with_ceilings("b", &solved(), &table(), &extra)),
            "CeilingMismatch"
        );
    }

    #[test]
    fn an_unsolved_ceiling_admits_every_mode_and_says_so() {
        let files = generate("b", &solved(), &table()).unwrap();
        let lib = &files["src/board.rs"];
        assert!(lib.contains("impl AtMostCeiling for FastPlus1M"));
        assert!(lib.contains("This is a recorded gap, not a default."));
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
        assert_eq!(kind(generate("b", &s, &t)), "IncompletePeripheral");

        let mut s = solved();
        s.assignments
            .insert("SDA2".into(), assign("PB9", "I2C1", "I2C1_SDA", 4));
        assert_eq!(kind(generate("b", &s, &t)), "IncompletePeripheral");

        let mut s = solved();
        s.gpio.insert("CS2".into(), "PB6".into());
        assert_eq!(kind(generate("b", &s, &t)), "PinUsedTwice");

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
}
