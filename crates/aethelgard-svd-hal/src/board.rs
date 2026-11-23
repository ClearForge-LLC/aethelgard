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
//! `AET-WO-4300c`). The register values a controller's bring-up writes are
//! the solved topology's (`AET-WO-4230` §1.1–§1.4: the SPI kernel clocks and
//! `MBR`, the I2C `CR1` and `TIMINGR`), copied, never computed here; the
//! oracle's expected words come from `aethelgard_ir::silicon`, the independent
//! rule, which [`check_silicon`] runs before anything is emitted.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use aethelgard_ir::motor;
use aethelgard_ir::serial::{self, Parity};
use aethelgard_ir::silicon;
use aethelgard_ir::speeds::{self, OutputSpeeds};
use aethelgard_ir::vos::{self, CoreSupplyMode};
use aethelgard_ir::{
    IrValidationError, PinTable, RESERVED_PINS, SolvedI2cTiming, SolvedSpiController,
    SolvedTopology, SpiKernel,
    motor::Level,
    pin_devices::{Direction, McuPull},
};

use crate::cmsis::{self, RegisterError, RegisterFacts};

mod adc;
mod firmware;
mod ledger;

pub use firmware::{
    BOOT_ACTVOSRDY_TIMEOUT, BOOT_FLASH_ACR_NOT_TAKEN, BOOT_HSE_CSS_FAULT, BOOT_HSE_NOT_READY,
    BOOT_PLL1_NOT_LOCKED, BOOT_SUPPLY_NOT_DECLARED, BOOT_SW_NOT_TAKEN, BOOT_VOSRDY_TIMEOUT,
    ECHO_POLLS, FRAME_LEN, PATTERN_TAIL, PWR_POLLS,
};
pub use ledger::{FieldAccess, FieldUse, register_fields};

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
        "peripheral `{peripheral}` is not one this generator emits (I2Cn, SPIn, TIMn on a motor output, and USARTn or UARTn on a serial port)"
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
    #[error("I2C peripheral `{peripheral}` serves no one bus: {why}")]
    I2cBusUnresolved { peripheral: String, why: String },
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
    #[error(
        "`{peripheral}`'s solved values fail the independent silicon rule (aethelgard_ir::silicon, AET-WO-4230): {finding}"
    )]
    SiliconRuleFails { peripheral: String, finding: String },
    #[error(
        "`{clock}` fails the voltage-scaling rule (aethelgard_ir::vos::judge_generated, AET-WO-4240v §1.3): {finding}"
    )]
    VosRuleFails { clock: String, finding: String },
    #[error(
        "the core supply is declared `bypass`: no board this engine emits draws an external VCORE, and under bypass the voltage scale is the external supply's, VOSRDY forced to 1 by hardware (RM0433 Rev 9 §6.8.6, p. 310; AET-WO-4240v D-1)"
    )]
    CoreSupplyNotGenerated,
    #[error(
        "the {wait} wait of {us} us is more reads than a u32 counts at HSI's fastest ({hz} Hz): the bound would wrap, so it is refused (AET-WO-4240b)"
    )]
    HsePollsOverflow {
        wait: &'static str,
        us: u64,
        hz: u64,
    },
    /// `AET-WO-1211b`: an ADC input on a peripheral or channel this
    /// generator does not bring up (ADC2 is never solved unpinned, `1211` Q8).
    #[error(
        "ADC input `{net}` is on {peripheral}: the generator brings up ADC1 and ADC3 inputs INP0-INP19 only (AET-WO-1211b)"
    )]
    AdcNotGenerated { net: String, peripheral: String },
    /// `AET-WO-1211b`: a pin device's ADC input on an HSE board, whose
    /// bring-up proof is written for the reset clock (a merge of `AET-WO-4240b`
    /// and this work order; no fixture has both).
    #[error(
        "ADC input `{net}` on an HSE board: the ADC bring-up and its proof are written for the reset clock (AET-WO-1211b)"
    )]
    AdcOnHseNotGenerated { net: String },
    /// `AET-WO-1211b` S1: the solved topology carries no resolution for a
    /// device ADC input, and none is substituted (`INV-06`).
    #[error(
        "ADC input `{net}`: the solved topology carries no declared resolution, and the generator substitutes none (AET-WO-1211b S1, INV-06)"
    )]
    AdcResolutionNotSolved { net: String },
    /// `AET-WO-1211b`: no `ADC_CFGR.RES` code means `bits` on both silicon
    /// revisions (RM0433 Rev 9 p. 1013: 8 bits is `100` on Y, `111` on V).
    #[error(
        "ADC input `{net}` declares {bits} bits, which no ADC_CFGR.RES code means on both silicon revisions Y and V (RM0433 Rev 9 p. 1013); the revision is not known"
    )]
    AdcResolutionNotCommon { net: String, bits: u8 },
    /// `AET-WO-1211b`: one ADC converts at one resolution (`ADC_CFGR.RES`).
    #[error(
        "the inputs of {peripheral} declare different resolutions; one ADC converts at one (ADC_CFGR.RES)"
    )]
    AdcResolutionsDiffer { peripheral: String },
    /// `AET-WO-1211b`: the bring-up's sequence is `SQ1`-`SQ4` (`ADC_SQR1`).
    #[error(
        "{peripheral} has {inputs} inputs; the bring-up converts at most four, the sequence ADC_SQR1 holds"
    )]
    AdcSequenceTooLong { peripheral: String, inputs: usize },
    /// `AET-WO-1211b` (`1211` §2.5, Q3): a bridge's static idle, as the
    /// generator would leave it, is no non-driving row of its part's mode
    /// (`aethelgard_ir::pin_parts`), or cannot be read as one.
    #[error("bridge `{bridge}` of `{reference}`: {detail}")]
    BridgeIdleNotSafe {
        reference: String,
        bridge: String,
        detail: String,
    },
    /// `AET-WO-1211b`: a bridge whose enable must follow inputs on two
    /// timers (its own two, or two bridges' sharing the enable), each its own
    /// part of the schedule proof, whose order no one part could prove.
    #[error(
        "bridge `{bridge}` of `{reference}`: its enable follows inputs on more than one timer, so no one proof part holds its order"
    )]
    BridgeAcrossParts { reference: String, bridge: String },
    /// `AET-WO-1211b` (`INV-06`): the bring-up wakes a part whose row states
    /// no `t_wake`.
    #[error("`{reference}` ({part}) is woken at bring-up, and its row states no t_wake to wait")]
    WakeTimeUnstated { reference: String, part: String },
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
            Self::I2cBusUnresolved { .. } => "I2cBusUnresolved",
            Self::FootprintOverlap { .. } => "FootprintOverlap",
            Self::UnscheduledAction { .. } => "UnscheduledAction",
            Self::SiliconRuleFails { .. } => "SiliconRuleFails",
            Self::VosRuleFails { .. } => "VosRuleFails",
            Self::CoreSupplyNotGenerated => "CoreSupplyNotGenerated",
            Self::HsePollsOverflow { .. } => "HsePollsOverflow",
            Self::AdcNotGenerated { .. } => "AdcNotGenerated",
            Self::AdcOnHseNotGenerated { .. } => "AdcOnHseNotGenerated",
            Self::AdcResolutionNotSolved { .. } => "AdcResolutionNotSolved",
            Self::AdcResolutionNotCommon { .. } => "AdcResolutionNotCommon",
            Self::AdcResolutionsDiffer { .. } => "AdcResolutionsDiffer",
            Self::AdcSequenceTooLong { .. } => "AdcSequenceTooLong",
            Self::BridgeIdleNotSafe { .. } => "BridgeIdleNotSafe",
            Self::BridgeAcrossParts { .. } => "BridgeAcrossParts",
            Self::WakeTimeUnstated { .. } => "WakeTimeUnstated",
            Self::Register(e) => e.kind(),
        }
    }
}

/// The I2C modes of UM10204 and their SCL frequencies. Each mode's
/// `TIMINGR` is the solved topology's (`SolvedI2cTiming::timingr`).
const I2C_MODES: [(&str, u32); 3] = [
    ("Standard100k", 100_000),
    ("Fast400k", 400_000),
    ("FastPlus1M", 1_000_000),
];

/// HSI's worst-case deviations from nominal, ppm: DS12117 Rev 10 Table 36
/// (rev Y) = Table 133 (rev V), rendered: ±0.3 MHz of 64 MHz at 3.3 V and
/// 30 °C (4,688 ppm, rounded away from zero), −0.12/+0.03 % over VDD 1.62 to
/// 3.6 V, −2/+1 % over TJ −40 °C to TJmax, summed. The generator's own copy
/// (`AET-WO-4230` amendment 1); `aethelgard_ir::silicon` holds another.
const HSI_FAST_PPM: u64 = 14_984;
const HSI_SLOW_PPM: u64 = 25_894;

/// The reset clock this generator emits for: HSI at 64 MHz, undivided, as the
/// core clock and as every HSI-derived kernel (`ResetClock`, `AET-WO-4300a`
/// Q2, ruled).
const RESET_HZ: u64 = 64_000_000;

/// The fastest `hz` of HSI can run, rounded up.
fn hsi_fastest(hz: u64) -> u64 {
    (u128::from(hz) * u128::from(1_000_000 + HSI_FAST_PPM))
        .div_ceil(1_000_000)
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The slowest `hz` of HSI can run, rounded down.
fn hsi_slowest(hz: u64) -> u64 {
    (u128::from(hz) * u128::from(1_000_000 - HSI_SLOW_PPM) / 1_000_000)
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Refuses a solved SPI or I2C controller, or a serial port's `BRR`, whose
/// values the independent rule does not hold (`AET-WO-4230` §1.3, review A5,
/// R3): the generator copies them
/// into code, and the oracle's expected words come from the same rule
/// (`aethelgard_ir::silicon::spi_words`, `i2c_cr1`), so a value it refuses
/// is never emitted. [`generate`] runs it first; `SolvedTopology::validate`
/// also holds the topology to what `solve` derives, so through [`generate`]
/// this refuses only a solver the rule disagrees with.
///
/// First, the voltage-scaling rule (`AET-WO-4240v` §1.3,
/// `aethelgard_ir::vos::judge_generated`): every bus and enabled kernel clock
/// at its fastest within its maximum at the topology's `vos`, every kernel
/// read, and `FLASH_ACR` at reset within RM0433 Table 17. A verdict that does
/// not hold is refused by the clock's name, so a refused tree is never
/// generated. `SolvedTopology::validate` refuses most such trees first.
pub fn check_silicon(solved: &SolvedTopology) -> Result<(), BoardError> {
    let rule = vos::judge_generated(solved);
    if !rule.holds() {
        if let Some(c) = rule.clocks.iter().find(|c| c.holds != Some(true)) {
            return Err(BoardError::VosRuleFails {
                clock: c.clock.clone(),
                finding: format!(
                    "{} Hz at its fastest against {} at VOS {} ({})",
                    c.fastest_hz
                        .map_or_else(|| "unknown".to_owned(), |hz| hz.to_string()),
                    c.limit_hz.map_or_else(
                        || "no cited maximum".to_owned(),
                        |hz| format!("a maximum of {hz} Hz")
                    ),
                    rule.vos,
                    c.cite.unwrap_or("uncited"),
                ),
            });
        }
        if let Some(kernel) = rule.unread.first() {
            return Err(BoardError::VosRuleFails {
                clock: kernel.clone(),
                finding: "its kernel select is one the rule cannot read".to_owned(),
            });
        }
        return Err(BoardError::VosRuleFails {
            clock: "FLASH_ACR".to_owned(),
            finding: format!(
                "LATENCY and WRHIGHFREQ at reset are below RM0433 Rev 9 Table 17's row for VOS {} at an AXI clock of {} Hz at its fastest",
                rule.vos, rule.flash.hclk_fastest_hz
            ),
        });
    }
    for (peripheral, c) in &solved.spi_controllers {
        let f = silicon::check_spi(peripheral, c, &solved.clock);
        if !f.holds {
            let failed: Vec<&str> = [
                ("kernel_ok", f.kernel_ok),
                ("mbr_ok", f.mbr_ok),
                ("achieved_ok", f.achieved_ok),
            ]
            .into_iter()
            .filter(|(_, ok)| !ok)
            .map(|(name, _)| name)
            .collect();
            return Err(BoardError::SiliconRuleFails {
                peripheral: peripheral.clone(),
                finding: format!(
                    "check_spi: {} false (kernel {:?} at {} Hz, MBR {}, declared {} Hz)",
                    failed.join(", "),
                    c.kernel,
                    c.kernel_hz,
                    c.mbr,
                    c.declared_hz
                ),
            });
        }
    }
    for (peripheral, t) in &solved.i2c_timings {
        let f = silicon::check_i2c_on(solved.clock.budget(), peripheral, t);
        if !f.holds {
            let mut failed: Vec<String> = [
                ("kernel_ok", f.kernel_ok),
                ("cr1_ok", f.cr1_ok),
                ("modes_ok", f.modes_ok),
            ]
            .into_iter()
            .filter(|(_, ok)| !ok)
            .map(|(name, _)| name.to_owned())
            .collect();
            for w in f.timingr.iter().filter(|w| !w.holds) {
                let c = &w.constraints;
                let flags: Vec<&str> = [
                    ("t_low", c.t_low),
                    ("t_high", c.t_high),
                    ("scl_frequency", c.scl_frequency),
                    ("data_setup", c.data_setup),
                    ("data_hold", c.data_hold),
                    ("kernel_minimum", c.kernel_minimum),
                    ("tight", w.tight),
                    ("presc_least", w.presc_least),
                    ("reserved_zero", w.reserved_zero),
                ]
                .into_iter()
                .filter(|(_, ok)| !ok)
                .map(|(name, _)| name)
                .collect();
                failed.push(format!(
                    "{} TIMINGR {:#010x}: {}",
                    w.mode,
                    w.timingr,
                    flags.join(", ")
                ));
            }
            if f.timingr.len() != t.timingr.len() {
                failed.push("a mode the rule does not know".to_owned());
            }
            return Err(BoardError::SiliconRuleFails {
                peripheral: peripheral.clone(),
                finding: format!("check_i2c: {} false", failed.join("; ")),
            });
        }
    }
    // `USART_BRR` (`AET-WO-4230` R3, the architect's ruling on Q-H): the
    // nearest whole divider, by the rule's own inequality, never by
    // re-running the solver's `serial::solve_brr`.
    // The kernel clock is the rule's own (`silicon::usart_kernel_hz`, the
    // adversarial pass's F1), and the topology's must be it.
    for port in solved.serial_ports.values() {
        let baud = port.framing.baud;
        let f = silicon::check_brr(
            &port.peripheral,
            &solved.clock,
            port.kernel_hz,
            baud,
            port.brr,
        );
        if !f.kernel_ok {
            return Err(BoardError::SiliconRuleFails {
                peripheral: port.peripheral.clone(),
                finding: format!(
                    "check_brr: the topology's kernel clock {} Hz is not the rule's {} Hz",
                    port.kernel_hz, f.kernel_hz
                ),
            });
        }
        if !f.holds {
            return Err(BoardError::SiliconRuleFails {
                peripheral: port.peripheral.clone(),
                finding: format!(
                    "check_brr: BRR {} at {} baud on a {} Hz kernel is not the nearest whole divider of at least 16",
                    port.brr, baud, f.kernel_hz
                ),
            });
        }
    }
    Ok(())
}

/// The declared speed of the one bus `peripheral` serves (`AET-WO-3402`
/// §1.3), from the solved topology alone (`RUL-16`): each I2C line names its
/// bus (`analog_lines`), and the assignment of the line's net names its
/// peripheral. No bus, two buses, or a bus with a line on another peripheral
/// is refused by name (`INV-06`); nothing falls back to another bus's speed.
fn bus_speed(solved: &SolvedTopology, peripheral: &str) -> Result<u32, BoardError> {
    let unresolved = |why: String| BoardError::I2cBusUnresolved {
        peripheral: peripheral.to_owned(),
        why,
    };
    let on = |net: &str| solved.assignments.get(net).map(|a| a.peripheral.as_str());
    let buses: std::collections::BTreeSet<&str> = solved
        .analog_lines
        .iter()
        .filter(|(net, _)| on(net) == Some(peripheral))
        .map(|(_, line)| line.bus.as_str())
        .collect();
    let bus = match buses.iter().collect::<Vec<_>>().as_slice() {
        [] => return Err(unresolved("no I2C line of the topology is on it".into())),
        [bus] => **bus,
        _ => {
            return Err(unresolved(format!(
                "it carries the lines of buses {buses:?}"
            )));
        }
    };
    for (net, _) in solved.analog_lines.iter().filter(|(_, l)| l.bus == bus) {
        if on(net) != Some(peripheral) {
            return Err(unresolved(format!(
                "bus `{bus}`'s line `{net}` is on {}",
                on(net).map_or("no peripheral".to_owned(), |p| format!("`{p}`"))
            )));
        }
    }
    solved
        .i2c_speeds
        .get(bus)
        .copied()
        .ok_or_else(|| unresolved(format!("bus `{bus}` declares no speed")))
}

/// The board crate for `crate_name`, from `solved`.
///
/// The I2C speeds each peripheral admits come from `solved.i2c_speeds`, the
/// speed each bus's specification declared (`AET-WO-1205`), keyed by bus id;
/// `i2c_ceilings` (`AET-WO-1202`) is only counted against the peripherals.
/// Each I2C peripheral takes its own bus's declared speed, joined through the
/// solved topology ([`bus_speed`], `AET-WO-3402` §1.3).
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
/// `AET-WO-4200`), whose part has an identity, modelled or not
/// (`AET-WO-4241`); a scripted part's configuration and data steps only where
/// `answers` says the twin has a model (`AET-WO-2202`). This crate cannot
/// depend on the twin, so the caller passes whether
/// `aethelgard_twin::model::model(part)` is `Ok(Some(_))`.
pub fn generate_with(
    crate_name: &str,
    solved: &SolvedTopology,
    table: &PinTable,
    answers: &dyn Fn(&str) -> bool,
) -> Result<BTreeMap<String, String>, BoardError> {
    let plan = Plan::new(crate_name, solved, table)?;
    check_silicon(solved)?;
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
    for name in i2c {
        let hz = bus_speed(solved, &name)?;
        let admitted: Vec<usize> = (0..I2C_MODES.len())
            .filter(|&i| I2C_MODES[i].1 <= hz)
            .collect();
        // belt-and-suspenders: `SolvedTopology::validate` admits only 100, 400
        // and 1000 kHz, so a ceiling below every mode cannot arrive here.
        if admitted.is_empty() {
            return Err(BoardError::CeilingTooLow { hz });
        }
        speeds.insert(name, (hz, admitted));
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
             Rxp,\n    \
             /// `SR.EOT` never rose after the last frame: the transfer did not end\n    \
             /// (RM0433 Rev 9 §50.4.12, p. 2177, step 1 of disabling a master).\n    \
             Eot,\n\
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
         }}\n\
         \n\
         /// {round} reads of `SR`, one round of the delay between `EOT` and `SPE` = 0 that\n\
         /// ES0392 Rev 16 §2.22.6 asks for, written out so that no proof unwinds them.\n\
         fn spi_sr_round<R: Registers>(r: &mut R, block: Block) {{\n\
         {reads}\n\
         }}\n",
        round = SR_ROUND,
        reads =
            vec![format!("    let _ = r.read(TOKEN, block, {:#04x});", s.sr); SR_ROUND as usize]
                .join("\n"),
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

/// The reads of `SR` in one round of the erratum's delay, straight-line, so a
/// proof unwinds the delay's loop once per round: at `MBR` 7 the delay is
/// 267 reads, and a loop of 267 iterations made the transfer proof's unwind
/// bound 268, which ran out of 8 GiB (measured). A delay rounded up to whole
/// rounds is only longer.
const SR_ROUND: u32 = 16;

/// The reads of `SR` an SPI transfer makes after `EOT` before it clears
/// `SPE`: ES0392 Rev 16 §2.22.6 (p. 47) says only to "add a delay between the
/// EOT event and SPI disable action", since a disable that comes too fast may
/// truncate the last SCK pulse. `EOT` is set on sampling the last bit, so at
/// most one SCK period of that pulse remains; the delay is one whole period,
/// `2^(MBR+1)` kernel cycles, at the slowest kernel, counted in core cycles at
/// the fastest core (both are HSI, but no common drift is credited), each
/// read at least one core cycle.
///
/// On an HSE board (`AET-WO-4240b`) both are the source's: the core after the
/// switch, and `pll1_q_ck` or the APB clock, each at its extreme.
fn eot_delay_reads(plan: &Plan, c: &SolvedSpiController) -> u64 {
    let period = u128::from(2u64 << c.mbr) * u128::from(plan.core_fastest());
    let slowest = match (&plan.hse, c.kernel) {
        (Some(h), SpiKernel::Pll1Q | SpiKernel::Pclk) => h.slowest(c.kernel_hz),
        _ => hsi_slowest(c.kernel_hz),
    };
    period
        .div_ceil(u128::from(slowest.max(1)))
        .try_into()
        .unwrap_or(u64::MAX)
}

/// One SPI peripheral's bounded transfer (`AET-WO-4200`): the command and
/// argument bytes out, then the answer in, with `0x00` clocked out while it
/// arrives. The register order is the one measured against Renode 1.17.0
/// (`TSIZE` while disabled, select, `SPE`, `CSTART`, the frames, `EOTC`, `SPE`
/// off, deselect), with a bounded wait for `SR.EOT` before `EOTC`
/// (`AET-WO-4230`; Renode sets `EOT` once `TSIZE` frames moved,
/// `STM32H7_SPI.cs` L388–395), then the erratum's delay
/// ([`eot_delay_reads`]). `CR1` is written whole each time, never read
/// (`AET-WO-4230` D5).
fn emit_spi_transfer(w: &mut String, name: &str, selects: &[Select], plan: &Plan, facts: &Facts) {
    let t = type_name(name);
    let pf = &facts.peripherals[name];
    let Some(s) = &pf.spi else { return };
    let c = &plan.spi[name];
    let code = spi_code(c, s, pf.enable, pf.master.unwrap_or_default());
    let delay = eot_delay_reads(plan, c);
    let rounds = delay.div_ceil(u64::from(SR_ROUND));
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
                 r.write(TOKEN, Block::{t}, {cr1:#04x}, {enable_word:#010x});\n        \
                 r.write(TOKEN, Block::{t}, {cr1:#04x}, {start_word:#010x});\n        \
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
                 // A master is disabled only once `EOT` is set (RM0433 Rev 9 §50.4.12,\n        \
                 // p. 2177, step 1), and `EOTC` clears it.\n        \
                 let mut ended = fail;\n        \
                 let mut n = 0;\n        \
                 while !ended && n < spins {{\n            \
                     ended = r.read(TOKEN, Block::{t}, {sr:#04x}) & {eot} != 0;\n            \
                     n += 1;\n        \
                 }}\n        \
                 // ES0392 Rev 16 §2.22.6: at least one SCK period between `EOT` and\n        \
                 // `SPE` = 0, {delay} reads of `SR` at the fastest core clock, as {rounds} of\n        \
                 // [`spi_sr_round`].\n        \
                 if ended && !fail {{\n            \
                     let mut k = 0;\n            \
                     while k < {rounds} {{\n                \
                         spi_sr_round(r, Block::{t});\n                \
                         k += 1;\n            \
                     }}\n        \
                 }}\n        \
                 r.write(TOKEN, Block::{t}, {ifcr:#04x}, {eotc});\n        \
                 r.write(TOKEN, Block::{t}, {cr1:#04x}, {idle_word:#010x});\n        \
                 r.write(TOKEN, port, {bsrr:#04x}, deselect);\n        \
                 if fail {{\n            \
                     Err(SpiError::Rxp)\n        \
                 }} else if !ended {{\n            \
                     Err(SpiError::Eot)\n        \
                 }} else {{\n            \
                     Ok(got)\n        \
                 }}\n    \
             }}\n\
         }}\n",
        arms = arms.join("\n"),
        cr2 = s.cr2,
        tsize_word = shifted("frames", "<<", s.tsize),
        bsrr = facts.gpio.bsrr,
        cr1 = pf.cr1,
        enable_word = code.cr1_enable,
        start_word = code.cr1_start,
        idle_word = code.cr1_idle,
        ifcr = s.ifcr,
        eotc = bit(s.eotc),
        sr = s.sr,
        eot = bit(s.eot),
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
    UsartTx,
    UsartRx,
    /// A pin device's GPIO input (`AET-WO-1211b`): `MODER` `00`, its declared
    /// pull.
    Input,
    /// A solved ADC input's pin (`AET-WO-1211b`): `MODER` `11`, no pull.
    Analog,
}

/// MODER values (RM0433 §11.4.1).
const MODE_INPUT: u32 = 0b00;
const MODE_OUTPUT: u32 = 0b01;
const MODE_AF: u32 = 0b10;
const MODE_ANALOG: u32 = 0b11;

/// The generator's `PUPDR` field for a GPIO input's declared MCU pull
/// (RM0433 Rev 9 §11.4.4, p. 542: `01` up, `10` down, `00` none).
///
/// belt-and-suspenders: `aethelgard_ir::silicon::implied_pupd` is the
/// oracle's copy (`INV-07`).
fn input_pull(pull: Option<McuPull>) -> u32 {
    match pull {
        Some(McuPull::Up) => 0b01,
        Some(McuPull::Down) => 0b10,
        Some(McuPull::None) | None => 0b00,
    }
}

/// The generator's `PUPDR` field for a motor output's pin, by its declared
/// fitment (`AET-WO-4300j` D6; RM0433 Rev 9 §11.4.4, p. 542): `fitted`, `00`,
/// no internal pull, since the declared external resistor holds the line
/// (`AET-WO-4300i` D0); `open`, `10`, the internal pull-down, since the
/// link is not fitted.
///
/// belt-and-suspenders: `aethelgard_ir::silicon::implied_pupd` is the
/// oracle's copy (`INV-07`).
fn motor_pull(fitment: motor::PullFitment) -> u32 {
    match fitment {
        motor::PullFitment::Fitted => 0b00,
        motor::PullFitment::Open => 0b10,
    }
}

/// A port's `PUPDR` at reset (RM0433 Rev 9 §11.4.4, p. 542): `0x6400_0000`
/// in port A (PA13 pull-up, PA14 pull-down, PA15 pull-up) and `0x0000_0100`
/// in port B (PB4 pull-up), the debug port's pulls; 0 in every other port.
/// `split` keeps every unsolved pin's field at it.
fn pupdr_reset(port: char) -> u32 {
    match port {
        'A' => 0x6400_0000,
        'B' => 0x0000_0100,
        _ => 0,
    }
}

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
            Self::UsartTx => "TX",
            Self::UsartRx => "RX",
            Self::Input => "IN",
            Self::Analog => "ANALOG",
        }
    }

    /// A USART's TX or RX.
    fn is_serial(self) -> bool {
        matches!(self, Self::UsartTx | Self::UsartRx)
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
    /// starts high (an inactive chip select); a timer channel is push-pull,
    /// and the pull here is not a motor output's: its pin's pull follows the
    /// declared fitment ([`motor_pull`], `AET-WO-4300j` D6); a USART's TX and
    /// RX are push-pull with no internal pull. The speed is [`Role::scope`]'s.
    fn policy(self) -> (u32, u32, u32) {
        match self {
            Self::I2cScl | Self::I2cSda => (MODE_AF, 1, 0b00),
            Self::SpiSck | Self::SpiMosi | Self::SpiMiso => (MODE_AF, 0, 0b00),
            Self::TimCh1 | Self::TimCh2 | Self::TimCh3 | Self::TimCh4 => (MODE_AF, 0, 0b00),
            Self::UsartTx | Self::UsartRx => (MODE_AF, 0, 0b00),
            Self::Output => (MODE_OUTPUT, 0, 0b00),
            Self::Input => (MODE_INPUT, 0, 0b00),
            Self::Analog => (MODE_ANALOG, 0, 0b00),
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
            Self::UsartTx | Self::UsartRx => Some(SERIAL_LINE),
            Self::Input => Some(GPIO_INPUT),
            Self::Analog => Some(ANALOG_INPUT),
        }
    }

    /// A pin the firmware configures by `MODER` alone, with no alternate
    /// function and no output level: a GPIO input or an analog pin.
    fn mode_only(self) -> bool {
        matches!(self, Self::Input | Self::Analog)
    }
}

/// Out of the output speed check's scope, each with its reason; the pin
/// takes `00`.
pub const OPEN_DRAIN: &str = "open-drain I2C: its rise is the pull-up's, judged by extract and the campaign (RUL-19); DS12117 Rev 10 6.3.32 guarantees UM10204 timing by design";
pub const MASTER_INPUT: &str = "input in master mode: no electrical effect";
pub const SOFTWARE_DRIVEN: &str =
    "software-driven GPIO output: firmware may toggle it at any rate, so the topology states none";
pub const MOTOR_OUTPUT: &str = "motor output (a timer channel): it leaves the board, so its load is off-board, and no connector part declares it yet";
/// A USART's TX or RX (`AET-WO-4300b` §1.4): the line is peripheral-driven at
/// its declared baud and leaves the board at a connector, so the speed check
/// (`AET-WO-4300c` D4) has no load for it, and it takes the slowest speed.
pub const SERIAL_LINE: &str = "serial line (a USART TX or RX): peripheral-driven at its declared baud; it leaves the board, so the check has no off-board load for it";
/// A pin device's GPIO input and a solved ADC input's pin (`AET-WO-1211b`;
/// `1211` §2.7 (b), (c): the speed check judges nothing on either, and says
/// so). Neither `OSPEEDR` is written: the MCU drives neither pin.
pub const GPIO_INPUT: &str =
    "GPIO input (a pin device's signal): the MCU drives nothing on it, so no edge rate applies";
pub const ANALOG_INPUT: &str = "analog input (an ADC channel, MODER 11): the output buffer is disconnected (RM0433 Rev 9 p. 539), so no edge rate applies";

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
    /// `PUPDR`: the role's pull ([`Role::policy`]), except a motor output's,
    /// by its declared fitment ([`motor_pull`], `AET-WO-4300j` D6), and a
    /// GPIO input's, its declared pull ([`input_pull`]) (`AET-WO-1211b`).
    pull: u32,
    /// A GPIO output's declared initial level, which its `BSRR` write sets
    /// before `MODER` (`AET-WO-1211b`); `None` for every other role.
    initial: Option<Level>,
}

impl PinPlan {
    /// (mode, output type, speed, pull).
    fn policy(&self) -> (u32, u32, u32, u32) {
        let (mode, otype, _) = self.role.policy();
        (mode, otype, self.speed, self.pull)
    }

    /// The `BSRR` bit a GPIO output's configure writes: its set bit for an
    /// initial `high`, its reset bit for `low` (RM0433 Rev 9 §11.4.7).
    fn level_bit(&self, pf: &PinFacts) -> u32 {
        match self.initial {
            Some(Level::Low) => 1u32 << pf.br,
            _ => 1u32 << pf.bs,
        }
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

/// One serial port (`AET-WO-4300b`): its place in the specification and its
/// label, its two pins, and what its configuration writes, all copied from
/// the solved port (`RUL-16`).
#[derive(Debug, Clone)]
struct SerialPlan {
    index: u32,
    label: String,
    /// The TX and RX pins, `PA2`.
    tx: String,
    rx: String,
    brr: u32,
    /// `CR1.M[1:0]` and `CR2.STOP[1:0]`, from the framing.
    m_bits: u8,
    stop: u8,
    parity: Parity,
    inverted: bool,
    /// `115200 baud 8N1`, for the documentation.
    framing: String,
}

impl SerialPlan {
    /// The port's label as a comment can hold it.
    fn label_text(&self) -> String {
        self.label.escape_debug().to_string()
    }
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
    /// Peripheral name (`USART2`) to its serial port (`AET-WO-4300b`), kept
    /// apart in the same way: a topology without serial ports emits what it did
    /// before, byte for byte.
    serials: BTreeMap<String, SerialPlan>,
    /// Each SPI peripheral's solved controller (`AET-WO-4230` §1.1, §1.2),
    /// copied from the topology (`RUL-16`).
    spi: BTreeMap<String, SolvedSpiController>,
    /// Each I2C peripheral's solved `CR1` and `TIMINGR` words (§1.4), copied.
    i2c: BTreeMap<String, SolvedI2cTiming>,
    /// The core clock the topology was solved for, Hz: `sysclk_hz` over
    /// `D1CPRE` (the oracle's, for the erratum delay's sizing).
    core_hz: u64,
    /// The declared supply configuration (`mcu.core_supply.mode`, copied by
    /// `solve`; `AET-WO-4240v` D-1): `Ldo`, since [`Plan::new`] refuses
    /// `Bypass`.
    core_supply: CoreSupplyMode,
    /// The proof oracle's own reading of the topology (`INV-07`,
    /// `AET-WO-1211b`), apart from the pin plans: each solved pin's `PUPDR`
    /// field by the rule (`aethelgard_ir::silicon::implied_pupd`), and each
    /// GPIO output's declared level, as `solved.gpio` states it.
    oracle_pupd: BTreeMap<String, u32>,
    oracle_levels: BTreeMap<String, Level>,
    /// Each timer channel's `CCRk` as the oracle derives it (`AET-WO-1211b`),
    /// by (timer, channel): a PWM output's declared idle ([`oracle_idle_word`]),
    /// a DShot output's solved word (its zero-bit duty, judged by
    /// `motor::check_silicon`).
    oracle_ccr: BTreeMap<(u32, u32), u32>,
    /// Each ADC a pin device's input is solved onto (`AET-WO-1211b`), its
    /// inputs and resolution. Empty without one, so every other board's text
    /// is unchanged.
    adcs: BTreeMap<String, adc::AdcPlan>,
    /// Each pin device's bridge with an enable (`AET-WO-1211b`): the enable
    /// is configured after every input is at its idle, and woken only after.
    bridges: Vec<BridgePlan>,
    /// The proof oracle's own reading of each bridge with an enable
    /// (`INV-07`; the adversarial pass's F3, F4): from the topology and the
    /// part's row, never from [`bridge_plans`].
    oracle_bridges: Vec<OracleBridge>,
    /// The proof oracle's own reading of each ADC (`adc::oracle_of`).
    oracle_adcs: BTreeMap<String, adc::OracleAdc>,
    /// The HSE tree (`AET-WO-4240b`), `None` on a board that runs on the HSI.
    hse: Option<HsePlan>,
    /// The topology's clock, for the oracle only (its budgets, `aethelgard_ir`).
    ir_clock: aethelgard_ir::motor::SolvedClock,
}

/// One bridge as the oracle reads it.
#[derive(Debug, Clone)]
struct OracleBridge {
    /// `U2`'s `M1`.
    name: String,
    enable: String,
    /// Each input: a timer channel (timer, channel) or a GPIO output's pin.
    inputs: Vec<Result<(u32, u32), String>>,
    /// The row's non-driving, non-forbidden states as (enable, input 0,
    /// input 1), `None` for an input the row leaves free.
    safe: Vec<(Level, Option<Level>, Option<Level>)>,
}

/// The oracle's bridges (`AET-WO-1211b`): each bridge, its inputs read from
/// `assignments` and `gpio`, and its mode's non-driving rows from the part's
/// row (`aethelgard_ir::pin_parts`, the datasheet's truth table), looked up
/// by part and mode.
///
/// belt-and-suspenders: this restates what [`bridge_plans`] reads on
/// purpose; the proof's expectations must not come from the plan the
/// generator orders the firmware by (`INV-07`).
fn oracle_bridges(solved: &SolvedTopology) -> Result<Vec<OracleBridge>, BoardError> {
    let table = aethelgard_ir::parts::table().map_err(BoardError::InvalidTopology)?;
    let mut out = Vec::new();
    for (reference, d) in &solved.pin_devices {
        for (id, b) in &d.bridges {
            let refuse = |detail: &str| BoardError::BridgeIdleNotSafe {
                reference: reference.clone(),
                bridge: id.clone(),
                detail: format!("the oracle cannot read it: {detail}"),
            };
            let Some(e) = &b.enable else {
                return Err(refuse("no enable"));
            };
            let mode = table
                .pin_part(&d.part)
                .and_then(|row| row.mode(&b.mode))
                .ok_or_else(|| refuse("no row or mode"))?;
            let enable = solved
                .gpio
                .get(&e.net)
                .map(|g| g.pin.clone())
                .ok_or_else(|| refuse("the enable is no GPIO"))?;
            let inputs = b
                .inputs
                .iter()
                .map(|net| {
                    match solved
                        .assignments
                        .get(net)
                        .and_then(|a| motor::timer_channel(&a.signal))
                    {
                        Some(tk) => Ok(Ok(tk)),
                        None => solved
                            .gpio
                            .get(net)
                            .map(|g| Err(g.pin.clone()))
                            .ok_or_else(|| refuse("an input is neither")),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let safe = mode
                .states
                .iter()
                .filter(|st| !st.drives && !mode.forbidden.contains(&st.name))
                .map(|st| (st.enable, st.inputs[0], st.inputs[1]))
                .collect();
            out.push(OracleBridge {
                name: format!("`{reference}`'s `{id}`"),
                enable,
                inputs,
                safe,
            });
        }
    }
    Ok(out)
}

/// One bridge's enable and what it must follow (`AET-WO-1211b`; `1211`
/// §2.7 (a)).
#[derive(Debug, Clone)]
struct BridgePlan {
    reference: String,
    id: String,
    /// The timer its timer-channel inputs are on (at most one, else
    /// `BridgeAcrossParts`), whose `CEN` the enable follows.
    timer: Option<u32>,
    /// The pins of its GPIO-output inputs.
    gpio_inputs: Vec<String>,
    /// The enable's pin.
    enable: String,
    /// The part's `t_wake`, µs, when the enable's declared level wakes it.
    wake_us: Option<u32>,
}

/// What an enable pin's configure must follow, over every bridge it enables:
/// the timer (at most one) and the GPIO inputs, and the longest wake.
#[derive(Debug, Clone, Default)]
struct EnableAfter {
    timer: Option<u32>,
    gpio_inputs: Vec<String>,
    wake_us: Option<u32>,
}

impl Plan {
    /// What `pin` must follow, if it is some bridge's enable.
    fn enable_after(&self, pin: &str) -> Option<EnableAfter> {
        let mut out: Option<EnableAfter> = None;
        for b in self.bridges.iter().filter(|b| b.enable == pin) {
            let e = out.get_or_insert_with(EnableAfter::default);
            e.timer = e.timer.or(b.timer);
            e.gpio_inputs.extend(b.gpio_inputs.iter().cloned());
            e.wake_us = e.wake_us.max(b.wake_us);
        }
        out
    }
}

/// The bridges of `solved` with an enable, each judged at its idle against
/// its part's row (`aethelgard_ir::pin_parts`, the datasheet's truth table,
/// looked up by part and mode; `AET-WO-1211b` S1). The tuple is read from the
/// topology's declared levels, never from the plan's words: an input on a
/// timer at its PWM idle level (CCR 0 holds OCxREF low, ARR + 1 high, RM0433
/// Rev 9 p. 1558), an input on a GPIO at its initial level, and the enable at
/// its initial level. A tuple that is no row, or a driving row, is refused.
fn bridge_plans(solved: &SolvedTopology) -> Result<Vec<BridgePlan>, BoardError> {
    let mut out = Vec::new();
    for (reference, d) in &solved.pin_devices {
        for (id, b) in &d.bridges {
            let refuse = |detail: String| BoardError::BridgeIdleNotSafe {
                reference: reference.clone(),
                bridge: id.clone(),
                detail,
            };
            // A bridge with no enable has no idle to judge it by (validate
            // refuses it, `DevicePinUnbound`; the adversarial pass's F7).
            let Some(e) = &b.enable else {
                return Err(refuse("it has no enable, so its idle is unjudged".into()));
            };
            let row = aethelgard_ir::parts::table()
                .map_err(BoardError::InvalidTopology)?
                .pin_part(&d.part)
                .ok_or_else(|| refuse(format!("`{}` has no busless row", d.part)))?;
            let mode = row
                .mode(&b.mode)
                .ok_or_else(|| refuse(format!("`{}` has no mode `{}`", d.part, b.mode)))?;
            let mut timers = BTreeSet::new();
            let mut gpio_inputs = Vec::new();
            let mut levels = [Level::Low; 2];
            if b.inputs.len() != 2 {
                return Err(refuse("a bridge has two inputs".into()));
            }
            for (k, net) in b.inputs.iter().enumerate() {
                if let Some(m) = solved.motor_outputs.get(net) {
                    levels[k] = match m.pwm.as_ref().map(|p| p.idle) {
                        Some(motor::Idle::Level(l)) => l,
                        _ => return Err(refuse(format!("input `{net}` does not idle at a level"))),
                    };
                    let (n, _) = solved
                        .assignments
                        .get(net)
                        .and_then(|a| motor::timer_channel(&a.signal))
                        .ok_or_else(|| refuse(format!("input `{net}` has no timer channel")))?;
                    timers.insert(n);
                } else if let Some(g) = solved.gpio.get(net) {
                    levels[k] = g
                        .initial
                        .ok_or_else(|| refuse(format!("input `{net}` is not an output")))?;
                    gpio_inputs.push(g.pin.clone());
                } else {
                    return Err(refuse(format!(
                        "input `{net}` is neither a motor output nor a GPIO"
                    )));
                }
            }
            let g = solved
                .gpio
                .get(&e.net)
                .filter(|g| g.direction == Direction::Output)
                .ok_or_else(|| refuse(format!("enable `{}` is not a GPIO output", e.net)))?;
            let initial = g
                .initial
                .ok_or_else(|| refuse(format!("enable `{}` states no initial level", e.net)))?;
            let state = mode.state(initial, levels).ok_or_else(|| {
                refuse(format!(
                    "its idle (enable {}, inputs {} {}) is no row of `{}` mode `{}` ({}, {})",
                    initial.name(),
                    levels[0].name(),
                    levels[1].name(),
                    d.part,
                    b.mode,
                    row.document,
                    mode.page
                ))
            })?;
            if state.drives || mode.forbidden.contains(&state.name) {
                return Err(refuse(format!(
                    "its idle is `{}` of `{}` mode `{}`, a driving or forbidden row ({}, {})",
                    state.name, d.part, b.mode, row.document, mode.page
                )));
            }
            if timers.len() > 1 {
                return Err(BoardError::BridgeAcrossParts {
                    reference: reference.clone(),
                    bridge: id.clone(),
                });
            }
            let wake_us = if initial == e.active {
                Some(row.t_wake_max_us.as_ref().map(|t| t.value).ok_or_else(|| {
                    BoardError::WakeTimeUnstated {
                        reference: reference.clone(),
                        part: d.part.clone(),
                    }
                })?)
            } else {
                None
            };
            out.push(BridgePlan {
                reference: reference.clone(),
                id: id.clone(),
                timer: timers.first().copied(),
                gpio_inputs,
                enable: g.pin.clone(),
                wake_us,
            });
        }
    }
    // An enable shared by bridges follows all of them: still one timer's part.
    for b in &out {
        let timers: BTreeSet<u32> = out
            .iter()
            .filter(|o| o.enable == b.enable)
            .filter_map(|o| o.timer)
            .collect();
        if timers.len() > 1 {
            return Err(BoardError::BridgeAcrossParts {
                reference: b.reference.clone(),
                bridge: b.id.clone(),
            });
        }
    }
    Ok(out)
}

/// An HSE board's bring-up (`AET-WO-4240b`): the solved words, copied
/// (`RUL-16`), the source's fastest after the switch, and each wait's bound in
/// reads, all by this generator's own arithmetic.
#[derive(Debug, Clone)]
struct HsePlan {
    /// `RCC_CR.HSEBYP`: an external clock into `OSC_IN`.
    bypass: bool,
    pllckselr: u32,
    pllcfgr: u32,
    pll1divr: u32,
    flash_acr: u32,
    slow_ppm: u32,
    /// `sys_ck` after the switch at the source's fastest, Hz, rounded up.
    core_fastest: u64,
    /// The reads of the `HSERDY`, `PLL1RDY` and `SWS` waits.
    polls: [u32; 3],
}

/// `PLL1RDY`'s bound, µs: DS12117 Rev 10 Table 39 (rev Y, p. 130) = Table 136
/// (rev V), PLL lock time 150 µs maximum.
pub const PLL_LOCK_US: u32 = 150;
/// `SWS`'s bound, µs (`AET-WO-4240b` Q5, ruled as a generous bound): once
/// `PLL1RDY` reads 1 only the switch's handover of two cycles of each clock
/// remains (RM0433 Rev 9 p. 371), far below this.
pub const SWS_US: u32 = 150;

impl HsePlan {
    /// `hse`'s plan: the declared start-up bound, the lock time and the
    /// switch's bound each in reads of a register at HSI's fastest (the core
    /// runs on the HSI until `SW`; a read takes at least one core cycle),
    /// rounded up, and refused where a `u32` cannot count them.
    fn new(hse: &aethelgard_ir::clock::SolvedHse, sysclk_hz: u64) -> Result<Self, BoardError> {
        use aethelgard_ir::clock::Hse;
        let hz = hsi_fastest(RESET_HZ);
        let polls = |wait: &'static str, us: u32| -> Result<u32, BoardError> {
            (u128::from(us) * u128::from(hz))
                .div_ceil(1_000_000)
                .try_into()
                .map_err(|_| BoardError::HsePollsOverflow {
                    wait,
                    us: u64::from(us),
                    hz,
                })
        };
        Ok(Self {
            bypass: matches!(hse.declared, Hse::External(_)),
            pllckselr: hse.pllckselr,
            pllcfgr: hse.pllcfgr,
            pll1divr: hse.pll1divr,
            flash_acr: hse.flash_acr,
            slow_ppm: hse.slow_ppm,
            core_fastest: (u128::from(sysclk_hz) * (1_000_000 + u128::from(hse.fast_ppm)))
                .div_ceil(1_000_000)
                .try_into()
                .unwrap_or(u64::MAX),
            polls: [
                polls("HSERDY", hse.declared.startup_max_us())?,
                polls("PLL1RDY", PLL_LOCK_US)?,
                polls("SWS", SWS_US)?,
            ],
        })
    }

    /// The slowest `hz` of a clock from the HSE can run, rounded down.
    fn slowest(&self, hz: u64) -> u64 {
        (u128::from(hz) * (1_000_000 - u128::from(self.slow_ppm.min(999_999))) / 1_000_000)
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

impl Plan {
    /// Whether `pin` is some SPI device's chip select.
    fn is_select(&self, pin: &str) -> bool {
        self.selects.values().flatten().any(|s| s.pin == pin)
    }

    /// The serial ports in the specification's order, each with its
    /// peripheral.
    fn serials_in_order(&self) -> Vec<(&String, &SerialPlan)> {
        let mut out: Vec<(&String, &SerialPlan)> = self.serials.iter().collect();
        out.sort_by_key(|(_, s)| s.index);
        out
    }

    /// Whether the board reads the reset clock: its timers' values, its
    /// USARTs' `BRR`, and its I2C and SPI peripherals' kernel clocks were
    /// solved for it (`AET-WO-4230` D6).
    /// An HSE board always does: its bring-up takes the reset clock's token.
    fn needs_clock(&self) -> bool {
        !self.timers.is_empty()
            || !self.serials.is_empty()
            || !self.peripherals.is_empty()
            || !self.adcs.is_empty()
            || self.hse.is_some()
    }

    /// Whether the bring-up selects SPI kernels: never on an HSE board, whose
    /// selects stay at reset (`4240` D6, `AET-WO-4240b` Q3).
    fn selects_kernels(&self) -> bool {
        !self.spi.is_empty() && self.hse.is_none()
    }

    /// The core clock at its fastest: HSI's on a board without HSE, the
    /// source's after the switch on an HSE board.
    fn core_fastest(&self) -> u64 {
        self.hse
            .as_ref()
            .map_or_else(|| hsi_fastest(RESET_HZ), |h| h.core_fastest)
    }
}

/// The serial ports' nets (`AET-WO-4300b`), each to its port's peripheral
/// and line, and each port's plan with its pins still empty: every port on a
/// peripheral this generator emits. `SolvedTopology::validate` holds the
/// rest (`aethelgard_ir::serial`'s `validate_solved`): each net one port's,
/// each port's TX and RX solved assignments on its own USART's signals, one
/// port per peripheral, and a framing and `BRR` its USART can hold.
type SerialNets<'a> = BTreeMap<&'a str, (&'a str, Role)>;

fn serial_plans(
    solved: &SolvedTopology,
) -> Result<(SerialNets<'_>, BTreeMap<String, SerialPlan>), BoardError> {
    let mut nets = BTreeMap::new();
    let mut plans: BTreeMap<String, SerialPlan> = BTreeMap::new();
    for (label, p) in &solved.serial_ports {
        if !serial::PERIPHERALS.contains(&p.peripheral.as_str()) {
            return Err(BoardError::UnsupportedPeripheral {
                peripheral: p.peripheral.clone(),
            });
        }
        for (net, role) in [(&p.tx, Role::UsartTx), (&p.rx, Role::UsartRx)] {
            nets.insert(net.as_str(), (p.peripheral.as_str(), role));
        }
        let plan = SerialPlan {
            index: p.index,
            label: label.clone(),
            tx: String::new(),
            rx: String::new(),
            brr: p.brr,
            m_bits: p
                .framing
                .m_bits()
                .expect("validate holds every port's word to 7, 8 or 9 bits"),
            stop: p
                .framing
                .stop_field()
                .expect("validate holds every port to 1 or 2 stop bits"),
            parity: p.framing.parity,
            inverted: p.framing.inverted,
            framing: format!("{} baud {}", p.framing.baud, p.framing.short()),
        };
        plans.insert(p.peripheral.clone(), plan);
    }
    Ok((nets, plans))
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
        // `bypass` is a true declaration the engine does not build
        // (`AET-WO-4240v` D-1): no emitted board draws an external VCORE.
        if solved.core_supply == CoreSupplyMode::Bypass {
            return Err(BoardError::CoreSupplyNotGenerated);
        }
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
            .chain(solved.gpio.values().map(|g| &g.pin))
            .chain(solved.adc_inputs.values().map(|a| &a.pin))
            .find(|p| RESERVED_PINS.contains(&p.as_str()));
        if let Some(pin) = reserved {
            return Err(BoardError::ReservedPin { pin: pin.clone() });
        }
        let mut pins: BTreeMap<String, PinPlan> = BTreeMap::new();
        let mut roles: BTreeMap<String, (Kind, BTreeMap<Role, Vec<String>>)> = BTreeMap::new();
        let mut timers: BTreeMap<u32, TimerPlan> = BTreeMap::new();
        let (serial_nets, mut serials) = serial_plans(solved)?;
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
            // A serial line: `validate` holds every assignment on a USART to
            // the TX or RX net of the one port on it, on its own signal.
            let serial = serial_nets.get(net.as_str()).copied();
            // The pin's role, its peripheral's kind unless it is a timer's or
            // a USART's, and its `PUPDR` field: the role's pull, except a
            // motor output's, which follows its declared fitment
            // ([`motor_pull`], `AET-WO-4300j` D6).
            let (role, kind, pull) = if let Some((peripheral, role)) = serial {
                let s = serials
                    .get_mut(peripheral)
                    .expect("serial_plans planned every port's peripheral");
                *if role == Role::UsartTx {
                    &mut s.tx
                } else {
                    &mut s.rx
                } = a.pin.clone();
                (role, None, role.policy().2)
            } else if let Some((n, k, role)) = channel {
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
                (role, None, motor_pull(m.pull.fitment))
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
                (role, Some(kind), role.policy().2)
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
                pull,
                initial: None,
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
        // The GPIO signals (`AET-WO-1211b`): an output at its declared level
        // (a chip select's is `high`), an input with its declared pull; and
        // each pin device's ADC input's pin, analog. `SolvedTopology::validate`
        // gives every output an `initial` and every input a `pull`. A
        // `power.sense` input is solved into `adc_inputs` too (`AET-WO-1211a`)
        // but no firmware has read it, and none does here: generating it would
        // move every sensed board's bytes, which this work order forbids.
        let gpio = solved.gpio.iter().map(|(net, g)| {
            let (role, pull) = match g.direction {
                Direction::Output => (Role::Output, 0b00),
                Direction::Input => (Role::Input, input_pull(g.pull)),
            };
            (net, &g.pin, role, pull, g.initial)
        });
        let analog =
            device_adc_inputs(solved).map(|(net, a)| (net, &a.pin, Role::Analog, 0b00, None));
        for (net, pin, role, pull, initial) in gpio.chain(analog) {
            let (port, index) = split_pin(pin)?;
            if !table.pins.contains_key(pin) {
                return Err(BoardError::UnknownPin { pin: pin.clone() });
            }
            let (speed, check) = choose_speed(speeds_table, solved, pin, net, role)?;
            let plan = PinPlan {
                port,
                index,
                role,
                af: 0,
                peripheral: None,
                net: net.clone(),
                speed,
                check,
                pull,
                initial,
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
            let pin = solved.gpio.get(&d.cs).map(|g| &g.pin).ok_or_else(|| {
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
        // `SolvedTopology::validate` gives every SPI and I2C peripheral of
        // `assignments` exactly one solved controller (`AET-WO-4230`).
        let mut spi = BTreeMap::new();
        let mut i2c = BTreeMap::new();
        for (name, p) in &peripherals {
            match p.kind {
                Kind::Spi => {
                    let c = solved.spi_controllers.get(name).ok_or_else(|| {
                        BoardError::InvalidTopology(IrValidationError::SpiControllerMismatch {
                            id: name.clone(),
                        })
                    })?;
                    spi.insert(name.clone(), c.clone());
                }
                Kind::I2c => {
                    let t = solved.i2c_timings.get(name).ok_or_else(|| {
                        BoardError::InvalidTopology(IrValidationError::I2cTimingMismatch {
                            id: name.clone(),
                        })
                    })?;
                    i2c.insert(name.clone(), t.clone());
                }
            }
        }
        let ports = pins.values().map(|p| p.port).collect();
        let oracle_pupd = pins
            .iter()
            .map(|(pin, p)| (pin.clone(), silicon::implied_pupd(solved, &p.net)))
            .collect();
        let mut oracle_ccr = BTreeMap::new();
        for (net, m) in &solved.motor_outputs {
            let channel = solved
                .assignments
                .get(net)
                .and_then(|a| motor::timer_channel(&a.signal));
            if let Some((n, k)) = channel {
                let word = match &m.pwm {
                    Some(p) => oracle_idle_word(p, m.arr).ok_or_else(|| {
                        BoardError::InvalidTopology(IrValidationError::MotorOutputMismatch {
                            id: net.clone(),
                        })
                    })?,
                    None => m.ccr,
                };
                oracle_ccr.insert((n, k), word);
            }
        }
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
            serials,
            spi,
            i2c,
            core_hz: u64::from(solved.clock.sysclk_hz) / u64::from(solved.clock.d1cpre.max(1)),
            core_supply: solved.core_supply,
            oracle_pupd,
            oracle_ccr,
            adcs: {
                let adcs = adc::of(solved)?;
                if let (Some(p), Some(_)) = (adcs.values().next(), &solved.clock.hse) {
                    return Err(BoardError::AdcOnHseNotGenerated {
                        net: p.inputs[0].net.clone(),
                    });
                }
                adcs
            },
            bridges: bridge_plans(solved)?,
            oracle_bridges: oracle_bridges(solved)?,
            oracle_adcs: adc::oracle_of(solved),
            oracle_levels: solved
                .gpio
                .values()
                .filter_map(|g| Some((g.pin.clone(), g.initial?)))
                .collect(),
            hse: solved
                .clock
                .hse
                .as_ref()
                .map(|h| HsePlan::new(h, u64::from(solved.clock.sysclk_hz)))
                .transpose()?,
            ir_clock: solved.clock.clone(),
        })
    }
}

/// The ADC inputs the generator brings up (`AET-WO-1211b`): every solved ADC
/// input but a `power.sense` net's.
fn device_adc_inputs(
    solved: &SolvedTopology,
) -> impl Iterator<Item = (&String, &aethelgard_ir::pin_devices::SolvedAdcInput)> {
    let sense: BTreeSet<&String> = solved.power.iter().flat_map(|p| p.sense.keys()).collect();
    solved
        .adc_inputs
        .iter()
        .filter(move |(net, _)| !sense.contains(net))
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
    rxp: u32,
    /// `SR.EOT` and `IFCR.EOTC`.
    eot: u32,
    eotc: u32,
    /// `CFG1.MBR` and `CFG1.CRCSIZE`, `CFG2.SSM` and `CFG2.AFCNTR`, `CR1.SSI`.
    mbr: u32,
    crcsize: u32,
    ssm: u32,
    afcntr: u32,
    ssi: u32,
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
/// hold at 0, their reset value: register name, `RCC_TypeDef` offset, and
/// each field's name, position and width.
type ClockFacts = Vec<(&'static str, u32, Vec<(&'static str, u32, u32)>)>;

/// The reset clock's fields, by register, in offset order, each read before
/// any write and at 0 its reset value (`AET-WO-4230` D6):
/// - `CR` `HSIDIV` (HSI undivided, so 64 MHz; `AET-WO-4300a` Q2, ruled), `CFGR`
///   `SWS` (HSI) and `TIMPRE`, `D1CFGR` `D1CPRE`, `D1PPRE` and `HPRE`, `D2CFGR`
///   `D2PPRE1` and `D2PPRE2` (RM0433 Rev 9 §8.7.2, §8.7.7–8.7.9);
/// - `D1CCIPR` `CKPERSEL` (§8.7.19, p. 412: reset `0x0000 0000`, `00` is
///   `hsi_ker_ck` as `per_ck`), the clock SPI1–3 take through `per_ck`;
/// - `D2CCIP1R` `SPI123SEL` and `SPI45SEL` (§8.7.20, p. 413: reset `0x0000
///   0000`), read before the bring-up selects SPI1–5's kernel clock;
/// - `D2CCIP2R` the two USART selections (`AET-WO-4300b`; at 0, `rcc_pclk2`
///   for USART1 and USART6 and `rcc_pclk1` for the rest,
///   `aethelgard_ir::serial::kernel_hz`) and `I2C123SEL` (§8.7.21, pp.
///   416–417: reset `0x0000 0000`, `00` is `rcc_pclk1`, I2C1–3's kernel clock).
///   RM0433 names the second USART field `USART234578SEL`; ST's header names
///   the same bits 2:0 `USART28SEL`, and the positions come from the header,
///   so its name is the one written here.
///
/// These are read on every board that reads the reset clock, whichever of
/// its peripherals it has: each is at reset after a reset, so the check
/// refuses only a clock some earlier code changed. `D3CFGR` and `D3CCIPR` are
/// read only for a peripheral that depends on them ([`clock_fields`]).
const RESET_CLOCK_FIELDS: [(&str, &[&str]); 7] = [
    ("CR", &["HSIDIV"]),
    ("CFGR", &["SWS", "TIMPRE"]),
    ("D1CFGR", &["D1CPRE", "D1PPRE", "HPRE"]),
    ("D2CFGR", &["D2PPRE1", "D2PPRE2"]),
    ("D1CCIPR", &["CKPERSEL"]),
    ("D2CCIP1R", &["SPI123SEL", "SPI45SEL"]),
    ("D2CCIP2R", &["USART16SEL", "USART28SEL", "I2C123SEL"]),
];

/// [`RESET_CLOCK_FIELDS`], and the domain 3 fields the plan's peripherals
/// depend on, in offset order: `D3CFGR` `D3PPRE` (§8.7.10, p. 397: reset
/// `0x0000 0000`, `0xx` is `rcc_pclk4 = rcc_hclk4`) and `D3CCIPR` `I2C4SEL`
/// (§8.7.22, p. 418: reset `0x0000 0000`; `00` is `rcc_pclk4`, p. 420) with I2C4;
/// `D3CCIPR` `SPI6SEL` (§8.7.22, p. 418: `000` is `rcc_pclk4`) with SPI6.
fn clock_fields(plan: &Plan) -> Vec<(&'static str, Vec<&'static str>)> {
    let has = |name: &str| plan.peripherals.contains_key(name);
    let mut out: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for (register, fields) in RESET_CLOCK_FIELDS {
        if register == "D1CCIPR" && has("I2C4") {
            out.push(("D3CFGR", vec!["D3PPRE"]));
        }
        out.push((register, fields.to_vec()));
    }
    let d3: Vec<&'static str> = [("I2C4", "I2C4SEL"), ("SPI6", "SPI6SEL")]
        .into_iter()
        .filter(|(p, _)| has(p))
        .map(|(_, f)| f)
        .collect();
    if !d3.is_empty() {
        out.push(("D3CCIPR", d3));
    }
    out
}

/// One USART or UART instance (`AET-WO-4300b`): its base and clock gate.
#[derive(Debug, Clone)]
struct SerialFacts {
    base: u32,
    gate: Gate,
}

/// The `USART_TypeDef` registers (RM0433 Rev 9 §48.8) a port's configuration,
/// send and receive touch, and the bit positions of their fields, all read
/// from the header.
#[derive(Debug, Clone)]
struct UsartFacts {
    cr1: u32,
    cr2: u32,
    brr: u32,
    isr: u32,
    rdr: u32,
    tdr: u32,
    ue: u32,
    re: u32,
    te: u32,
    ps: u32,
    pce: u32,
    m0: u32,
    m1: u32,
    /// `CR1.FIFOEN` (RM0433 Rev 9 §48.8.1, bit 29): FIFO mode, so a frame
    /// that arrives while the firmware is elsewhere waits in the 16-deep
    /// RXFIFO (§48.4 Table 402) instead of overrunning `RDR`.
    fifoen: u32,
    /// `CR2.STOP`: position and width.
    stop: (u32, u32),
    txinv: u32,
    rxinv: u32,
    /// `ISR.TXFNF` and `ISR.RXFNE` (the header's `TXE_TXFNF` and
    /// `RXNE_RXFNE`: with `FIFOEN` set, the TXFIFO not full and the RXFIFO
    /// not empty, §48.8.10; the same bits as `TXE` and `RXNE` with it clear).
    txe: u32,
    rxne: u32,
}

fn usart_facts(f: &RegisterFacts) -> Result<UsartFacts, BoardError> {
    let u = f.layout("USART")?;
    Ok(UsartFacts {
        cr1: u.offset("CR1", 0)?,
        cr2: u.offset("CR2", 0)?,
        brr: u.offset("BRR", 0)?,
        isr: u.offset("ISR", 0)?,
        rdr: u.offset("RDR", 0)?,
        tdr: u.offset("TDR", 0)?,
        ue: f.position("USART_CR1_UE_Pos")?,
        re: f.position("USART_CR1_RE_Pos")?,
        te: f.position("USART_CR1_TE_Pos")?,
        ps: f.position("USART_CR1_PS_Pos")?,
        pce: f.position("USART_CR1_PCE_Pos")?,
        m0: f.position("USART_CR1_M0_Pos")?,
        m1: f.position("USART_CR1_M1_Pos")?,
        fifoen: f.position("USART_CR1_FIFOEN_Pos")?,
        stop: f.field("USART_CR2_STOP")?,
        txinv: f.position("USART_CR2_TXINV_Pos")?,
        rxinv: f.position("USART_CR2_RXINV_Pos")?,
        txe: f.position("USART_ISR_TXE_TXFNF_Pos")?,
        rxne: f.position("USART_ISR_RXNE_RXFNE_Pos")?,
    })
}

/// A port's configuration (`AET-WO-4300b`): (`BRR`, `CR2`, `CR1` without
/// `UE`). `CR2` is `STOP`, with `TXINV` and `RXINV` when the port is
/// inverted; `CR1` is `M0` and `M1` from `M[1:0]`, `PCE` with any parity, `PS`
/// for odd, `TE`, `RE` and `FIFOEN` (writable only with `UE` clear, §48.8.1;
/// the PM's ruling on adversarial pass 2, H1).
fn usart_values(s: &SerialPlan, u: &UsartFacts) -> (u32, u32, u32) {
    let mut cr2 = u32::from(s.stop) << u.stop.0;
    if s.inverted {
        cr2 |= (1 << u.txinv) | (1 << u.rxinv);
    }
    let mut cr1 = (u32::from(s.m_bits & 1) << u.m0) | (u32::from(s.m_bits >> 1) << u.m1);
    if s.parity != Parity::None {
        cr1 |= 1 << u.pce;
    }
    if s.parity == Parity::Odd {
        cr1 |= 1 << u.ps;
    }
    cr1 |= (1 << u.te) | (1 << u.re) | (1 << u.fifoen);
    (s.brr, cr2, cr1)
}

#[derive(Debug, Clone)]
struct Facts {
    rcc_base: u32,
    rcc_words: u32,
    gpio: GpioFacts,
    ports: BTreeMap<char, PortFacts>,
    pins: BTreeMap<String, PinFacts>,
    peripherals: BTreeMap<String, PeriphFacts>,
    /// Timer number to its instance facts; empty without motor outputs, and
    /// then `tim` is `None`: nothing timer-shaped is read.
    timers: BTreeMap<u32, TimerFacts>,
    tim: Option<TimFacts>,
    /// Peripheral name to its instance facts; empty without serial ports, and
    /// then `usart` is `None`.
    serials: BTreeMap<String, SerialFacts>,
    usart: Option<UsartFacts>,
    /// Each planned ADC's facts (`AET-WO-1211b`).
    adcs: BTreeMap<String, adc::AdcFacts>,
    /// `None` unless the board has a timer or a serial port.
    clock: Option<ClockFacts>,
    /// The kernel-clock select writes, empty without an SPI peripheral.
    kernel_selects: Vec<SelectWrite>,
    flash: FlashFacts,
    pwr: PwrFacts,
    /// An HSE board's clock bring-up registers (`AET-WO-4240b`).
    hse: Option<HseFacts>,
}

/// The registers and fields an HSE board's bring-up writes and polls
/// (`AET-WO-4240b`), every place and position from ST's header: `RCC_CR`'s
/// `HSION`, `HSEON`, `HSERDY`, `HSEBYP`, `CSSHSEON` (RM0433's `HSECSSON`),
/// `PLL1ON` and `PLL1RDY`; `RCC_CFGR`'s `SW` and `SWS`; the three PLL
/// registers; `FLASH_ACR`'s `LATENCY` and `WRHIGHFREQ`.
#[derive(Debug, Clone)]
struct HseFacts {
    cr: u32,
    cfgr: u32,
    pllckselr: u32,
    pllcfgr: u32,
    pll1divr: u32,
    hsion: u32,
    hseon: u32,
    hserdy: u32,
    hsebyp: u32,
    hsecsson: u32,
    pll1on: u32,
    pll1rdy: u32,
    sw: (u32, u32),
    sws: (u32, u32),
    acr: u32,
    /// `LATENCY` and `WRHIGHFREQ`: the bits the read-back compares.
    acr_mask: u32,
    flash_words: u32,
}

/// `RCC_CFGR.SW`'s code for PLL1, `011` (RM0433 Rev 9 §8.7.7, p. 393). The
/// generator's own copy.
const SW_PLL1: u32 = 0b011;

impl HseFacts {
    fn new(f: &RegisterFacts, rcc: &cmsis::Layout) -> Result<Self, BoardError> {
        let field_mask = |name: &str| -> Result<u32, BoardError> {
            let (pos, width) = f.field(name)?;
            Ok(((1u32 << width) - 1) << pos)
        };
        Ok(Self {
            cr: rcc.offset("CR", 0)?,
            cfgr: rcc.offset("CFGR", 0)?,
            pllckselr: rcc.offset("PLLCKSELR", 0)?,
            pllcfgr: rcc.offset("PLLCFGR", 0)?,
            pll1divr: rcc.offset("PLL1DIVR", 0)?,
            hsion: f.position("RCC_CR_HSION_Pos")?,
            hseon: f.position("RCC_CR_HSEON_Pos")?,
            hserdy: f.position("RCC_CR_HSERDY_Pos")?,
            hsebyp: f.position("RCC_CR_HSEBYP_Pos")?,
            hsecsson: f.position("RCC_CR_CSSHSEON_Pos")?,
            pll1on: f.position("RCC_CR_PLL1ON_Pos")?,
            pll1rdy: f.position("RCC_CR_PLL1RDY_Pos")?,
            sw: f.field("RCC_CFGR_SW")?,
            sws: f.field("RCC_CFGR_SWS")?,
            acr: f.layout("FLASH")?.offset("ACR", 0)?,
            acr_mask: field_mask("FLASH_ACR_LATENCY")? | field_mask("FLASH_ACR_WRHIGHFREQ")?,
            flash_words: f.layout("FLASH")?.size / 4,
        })
    }

    /// The whole `RCC_CR` words the bring-up writes, in order, each its
    /// reset value (`0x0000 0001`, `HSION` alone: RM0433 Rev 9 §8.7.2,
    /// p. 383) with the bits set so far: `HSEBYP` (external only), `HSEON`,
    /// `HSECSSON`, then `PLL1ON`. (name, word).
    fn cr_words(&self, bypass: bool) -> Vec<(&'static str, u32)> {
        let reset = 1u32 << self.hsion;
        let byp = if bypass { 1u32 << self.hsebyp } else { 0 };
        let on = reset | byp | (1 << self.hseon);
        let css = on | (1 << self.hsecsson);
        let mut out = Vec::new();
        if bypass {
            out.push(("HSEBYP", reset | byp));
        }
        out.push(("HSEON", on));
        out.push(("HSECSSON", css));
        out.push(("PLL1ON", css | (1 << self.pll1on)));
        out
    }

    /// `RCC_CFGR` as the switch writes it, whole: `SW` = PLL1, every other
    /// field at its reset value 0 (`RCC_CFGR` reset `0x0000 0000`, §8.7.7).
    fn cfgr_word(&self) -> u32 {
        SW_PLL1 << self.sw.0
    }
}

/// The flash controller's register block (`FLASH_R_BASE`, not the memory at
/// `FLASH_BASE`) and `FLASH_OPTSR_CUR`, which the bring-up reads once for
/// `BOR_LEV` (`AET-WO-4230` §1.6, D7, D10; RM0433 Rev 9 §4.9.8, p. 212).
/// The flash interface has no clock gate in RCC: the header has no
/// `RCC_*ENR_FLASHEN`.
#[derive(Debug, Clone)]
struct FlashFacts {
    base: u32,
    optsr_cur: u32,
    bor_lev: (u32, u32),
}

// belt-and-suspenders: the generator's own PWR words, from the header and the
// declaration; `aethelgard_ir::vos` holds the rule's, which the oracle, the
// twin and the ledger read, so a wrong word is a disagreement (`INV-07`);
// `the_generators_pwr_words_are_the_rules` checks that they agree.
/// The power controller's registers the bring-up touches (`AET-WO-4240v`
/// §1.4), every place and position from ST's header: `PWR_BASE`,
/// `PWR_TypeDef`'s `CSR1`, `CR3` and `D3CR`, and the `PWR_CR3_SCUEN`,
/// `LDOEN`, `BYPASS`, `PWR_CSR1_ACTVOSRDY`, `PWR_D3CR_VOS` and `VOSRDY`
/// fields. The power controller has no clock gate in RCC: the header has no
/// `RCC_*ENR_PWREN`.
#[derive(Debug, Clone)]
struct PwrFacts {
    base: u32,
    words: u32,
    csr1: u32,
    cr3: u32,
    d3cr: u32,
    scuen: u32,
    ldoen: u32,
    bypass: u32,
    actvosrdy: u32,
    vos: (u32, u32),
    vosrdy: u32,
}

// belt-and-suspenders: `aethelgard_ir::vos::D3CR_VOS_SCALE2` is the rule's copy.
/// `PWR_D3CR.VOS`'s code for Scale 2, `10` (RM0433 Rev 9 §6.8.6, p. 310:
/// "00: Reserved (Scale 3 selected) 01: Scale 3 (default) 10: Scale 2 11:
/// Scale 1"). The generator's own copy; the header gives only the field's
/// place.
const VOS_SCALE2_CODE: u32 = 0b10;

impl PwrFacts {
    fn new(f: &RegisterFacts) -> Result<Self, BoardError> {
        let l = f.layout("PWR")?;
        Ok(Self {
            base: f.base("PWR")?,
            words: l.size / 4,
            csr1: l.offset("CSR1", 0)?,
            cr3: l.offset("CR3", 0)?,
            d3cr: l.offset("D3CR", 0)?,
            scuen: f.position("PWR_CR3_SCUEN_Pos")?,
            ldoen: f.position("PWR_CR3_LDOEN_Pos")?,
            bypass: f.position("PWR_CR3_BYPASS_Pos")?,
            actvosrdy: f.position("PWR_CSR1_ACTVOSRDY_Pos")?,
            vos: f.field("PWR_D3CR_VOS")?,
            vosrdy: f.position("PWR_D3CR_VOSRDY_Pos")?,
        })
    }

    /// `SCUEN`, `LDOEN` and `BYPASS`: the bits the `PWR_CR3` write sets.
    fn supply_mask(&self) -> u32 {
        (1 << self.scuen) | (1 << self.ldoen) | (1 << self.bypass)
    }

    /// `LDOEN` and `BYPASS`, the supply configuration RM0433 Rev 9 Table 34
    /// (p. 261) names: the read-back compares these two. What `SCUEN` reads
    /// after the single write is not stated (§6.8.4, p. 308).
    fn config_mask(&self) -> u32 {
        (1 << self.ldoen) | (1 << self.bypass)
    }

    /// The supply bits `mode` writes under [`Self::supply_mask`]: its row of
    /// RM0433 Rev 9 Table 34 (p. 261), "LDO supply" `LDOEN` 1 and `BYPASS` 0,
    /// "LDO Bypass" `LDOEN` 0 and `BYPASS` 1; and `SCUEN` 0, "Supply
    /// configuration update locked" (§6.8.4, p. 308), the state the single
    /// write leaves. No other pair is written: an illegal one locks `PWR_CR3`
    /// until a power cycle (p. 308).
    fn supply_bits(&self, mode: CoreSupplyMode) -> u32 {
        match mode {
            CoreSupplyMode::Ldo => 1 << self.ldoen,
            CoreSupplyMode::Bypass => 1 << self.bypass,
        }
    }

    /// `PWR_D3CR` as the bring-up writes it, whole: `VOS` = `10`, every
    /// reserved bit 0, its reset value (reset `0x0000 4000`, RM0433 Rev 9
    /// §6.8.6, p. 310), and `VOSRDY`, which is read-only, 0.
    fn d3cr_scale2(&self) -> u32 {
        VOS_SCALE2_CODE << self.vos.0
    }
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

fn clock_facts(
    plan: &Plan,
    f: &RegisterFacts,
    rcc: &cmsis::Layout,
) -> Result<ClockFacts, BoardError> {
    let mut out = Vec::new();
    for (register, fields) in clock_fields(plan) {
        let mut read = Vec::new();
        for field in fields {
            let (pos, width) = f.field(&format!("RCC_{register}_{field}"))?;
            read.push((field, pos, width));
        }
        out.push((register, rcc.offset(register, 0)?, read));
    }
    Ok(out)
}

fn spi_facts(f: &RegisterFacts, layout: &cmsis::Layout) -> Result<SpiFacts, BoardError> {
    let dsize = f.position("SPI_CFG1_DSIZE_Pos")?;
    Ok(SpiFacts {
        cr2: layout.offset("CR2", 0)?,
        cfg1: layout.offset("CFG1", 0)?,
        sr: layout.offset("SR", 0)?,
        ifcr: layout.offset("IFCR", 0)?,
        rxdr: layout.offset("RXDR", 0)?,
        cstart: f.position("SPI_CR1_CSTART_Pos")?,
        tsize: f.position("SPI_CR2_TSIZE_Pos")?,
        dsize,
        rxp: f.position("SPI_SR_RXP_Pos")?,
        eot: f.position("SPI_SR_EOT_Pos")?,
        eotc: f.position("SPI_IFCR_EOTC_Pos")?,
        mbr: f.position("SPI_CFG1_MBR_Pos")?,
        crcsize: f.position("SPI_CFG1_CRCSIZE_Pos")?,
        ssm: f.position("SPI_CFG2_SSM_Pos")?,
        afcntr: f.position("SPI_CFG2_AFCNTR_Pos")?,
        ssi: f.position("SPI_CR1_SSI_Pos")?,
    })
}

/// `SPI_CFG1.DSIZE` and `SPI_CFG1.CRCSIZE` reset to `00111` (RM0433 Rev 9
/// §50.11.3, p. 2218: reset `0x0007 0007`); "if CRC calculation is disabled
/// by CRCEN=0, the CRCSIZE field must be kept at its default setting"
/// (p. 2219; ES0392 Rev 16 §2.22.5, the same workaround). The generator's own
/// copy of the reset value.
const SPI_CRCSIZE_RESET: u32 = 0b0_0111;

/// The whole words one SPI controller's bring-up writes (`AET-WO-4230` D4,
/// D5), composed here from the solved `MBR` and the header's positions:
/// `CR1` idle, `SSI` (with `SSM` set, NSS reads high, so the master raises no
/// mode fault: RM0433 Rev 9 §50.11.1 p. 2215, §50.5.2 p. 2184), and with
/// `SPE`, and with `SPE` and `CSTART`; `CFG1` `MBR`, `CRCSIZE` at reset and
/// `DSIZE` 8 bits, every other field 0; `CFG2` `MASTER`, `SSM` and `AFCNTR`,
/// every other field 0 (mode 0, full duplex, Motorola, `SSOE` 0). `AFCNTR`
/// keeps the master driving its pins while `SPE` is 0 (RM0433 Rev 9 §50.11.4,
/// p. 2221), so SCK and MOSI do not float while the transfer's chip select is
/// low before `SPE` rises and after it falls (the architect's ruling on Q-D;
/// ST's HAL calls it `MasterKeepIOState`).
#[derive(Debug, Clone, Copy)]
struct SpiCode {
    cr1_idle: u32,
    cr1_enable: u32,
    cr1_start: u32,
    cfg1: u32,
    cfg2: u32,
}

fn spi_code(c: &SolvedSpiController, s: &SpiFacts, enable: u32, master: u32) -> SpiCode {
    let ssi = 1u32 << s.ssi;
    let spe = 1u32 << enable;
    SpiCode {
        cr1_idle: ssi,
        cr1_enable: ssi | spe,
        cr1_start: ssi | spe | (1u32 << s.cstart),
        cfg1: (u32::from(c.mbr) << s.mbr)
            | (SPI_CRCSIZE_RESET << s.crcsize)
            | (SPI_DSIZE << s.dsize),
        cfg2: (1u32 << master) | (1u32 << s.ssm) | (1u32 << s.afcntr),
    }
}

/// The select field and code that give SPI peripheral `name` its solved
/// kernel clock (`AET-WO-4230` §1.1), by register and field name (RM0433 Rev 9
/// §8.7.20, p. 414: `SPI123SEL` `100` is `per_ck`, `SPI45SEL` `011` is
/// `hsi_ker_ck`; §8.7.22, p. 418: `SPI6SEL` `011` is `hsi_ker_ck`). The
/// generator's own table; positions come from the header.
fn select_code(name: &str, kernel: SpiKernel) -> Option<(&'static str, &'static str, u32)> {
    match (name, kernel) {
        ("SPI1" | "SPI2" | "SPI3", SpiKernel::PerCk) => Some(("D2CCIP1R", "SPI123SEL", 0b100)),
        ("SPI4" | "SPI5", SpiKernel::HsiKerCk) => Some(("D2CCIP1R", "SPI45SEL", 0b011)),
        ("SPI6", SpiKernel::HsiKerCk) => Some(("D3CCIPR", "SPI6SEL", 0b011)),
        _ => None,
    }
}

/// One whole-word write of a kernel-clock select register: its name, its
/// `RCC_TypeDef` offset and the word, whose every field but the solved
/// controllers' is at its reset value, 0
/// (`RCC_D2CCIP1R` and `RCC_D3CCIPR` reset `0x0000 0000`, RM0433 Rev 9
/// §8.7.20 p. 413, §8.7.22 p. 418).
#[derive(Debug, Clone)]
struct SelectWrite {
    register: &'static str,
    offset: u32,
    word: u32,
}

/// The select writes the plan's SPI controllers need, in offset order.
fn kernel_selects(
    plan: &Plan,
    f: &RegisterFacts,
    rcc: &cmsis::Layout,
) -> Result<Vec<SelectWrite>, BoardError> {
    let mut words: BTreeMap<u32, SelectWrite> = BTreeMap::new();
    for (name, c) in &plan.spi {
        let (register, field, code) =
            select_code(name, c.kernel).ok_or_else(|| BoardError::SiliconRuleFails {
                peripheral: name.clone(),
                finding: format!("no select code gives it the kernel {:?}", c.kernel),
            })?;
        let (pos, width) = f.field(&format!("RCC_{register}_{field}"))?;
        if code >> width != 0 {
            return Err(BoardError::Register(RegisterError::MalformedSource {
                detail: format!("`RCC_{register}_{field}` is {width} bits, too few for {code:#b}"),
            }));
        }
        let offset = rcc.offset(register, 0)?;
        words
            .entry(offset)
            .or_insert(SelectWrite {
                register,
                offset,
                word: 0,
            })
            .word |= code << pos;
    }
    Ok(words.into_values().collect())
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
        let mut tim = None;
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
        }
        let mut serials = BTreeMap::new();
        let mut usart = None;
        if !plan.serials.is_empty() {
            for name in plan.serials.keys() {
                serials.insert(
                    name.clone(),
                    SerialFacts {
                        base: f.base(name)?,
                        gate: gate(f, &rcc, name)?,
                    },
                );
            }
            usart = Some(usart_facts(f)?);
        }
        let adcs = plan
            .adcs
            .keys()
            .map(|name| Ok((name.clone(), adc::facts(f, &rcc, name)?)))
            .collect::<Result<BTreeMap<_, _>, BoardError>>()?;
        let clock = if plan.needs_clock() {
            Some(clock_facts(plan, f, &rcc)?)
        } else {
            None
        };
        let fl = f.layout("FLASH")?;
        let flash = FlashFacts {
            base: f.base("FLASH_R")?,
            optsr_cur: fl.offset("OPTSR_CUR", 0)?,
            bor_lev: f.field("FLASH_OPTSR_BOR_LEV")?,
        };
        Ok(Facts {
            kernel_selects: if plan.selects_kernels() {
                kernel_selects(plan, f, &rcc)?
            } else {
                Vec::new()
            },
            hse: plan
                .hse
                .as_ref()
                .map(|_| HseFacts::new(f, &rcc))
                .transpose()?,
            flash,
            pwr: PwrFacts::new(f)?,
            rcc_base: f.base("RCC")?,
            rcc_words: rcc.size / 4,
            gpio,
            ports,
            pins,
            peripherals,
            timers,
            tim,
            serials,
            usart,
            adcs,
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
        None => match p.role {
            Role::Input => "into_input".to_owned(),
            Role::Analog => "into_analog".to_owned(),
            _ => "into_output".to_owned(),
        },
    }
}
fn pin_state(p: &PinPlan) -> String {
    match p.role {
        Role::Output => "Output".to_owned(),
        Role::Input => "Input".to_owned(),
        Role::Analog => "Analog".to_owned(),
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
         pub enum Block {{\n    Rcc,\n    Flash,\n    Pwr,"
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
    for name in plan.serials.keys() {
        let _ = writeln!(w, "    {},", type_name(name));
    }
    for name in plan.adcs.keys() {
        let _ = writeln!(
            w,
            "    {},\n    {},",
            adc::block(name),
            adc::common_block(name)
        );
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
                     Block::Rcc => {rcc:#010x},\n            \
                     Block::Flash => {flash:#010x},\n            \
                     Block::Pwr => {pwr:#010x},",
        rcc = facts.rcc_base,
        flash = facts.flash.base,
        pwr = facts.pwr.base,
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
    for (name, sf) in &facts.serials {
        let _ = writeln!(
            w,
            "            Block::{} => {:#010x},",
            type_name(name),
            sf.base
        );
    }
    for (name, af) in &facts.adcs {
        let _ = writeln!(
            w,
            "            Block::{} => {:#010x},\n            Block::{} => {:#010x},",
            adc::block(name),
            af.base,
            adc::common_block(name),
            af.common_base
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
    if !plan.timers.is_empty() {
        w.push_str(
            "/// A timer whose outputs run. It has no method: after `CEN`, nothing reads or\n\
             /// writes the timer again (`AET-WO-4300a`: in Renode, a core polling a running\n\
             /// timer collapses its edges onto one timestamp).\n\
             pub struct Running;\n\
             \n",
        );
    }
    // A pin device's inputs and the ADC inputs' pins (`AET-WO-1211b`), only
    // where the topology has one, so every other board's text is unchanged.
    for (role, text) in [
        (
            Role::Input,
            "/// A GPIO input (`AET-WO-1211b`): `MODER` `00`, its declared pull.\n\
             pub struct Input;\n\
             \n",
        ),
        (
            Role::Analog,
            "/// An ADC input's pin (`AET-WO-1211b`): `MODER` `11`, no pull.\n\
             pub struct Analog;\n\
             \n",
        ),
    ] {
        if plan.pins.values().any(|p| p.role == role) {
            w.push_str(text);
        }
    }
    w.push_str(
        "/// The core at VOS Scale 2 on the declared supply configuration (`AET-WO-4240v`\n\
         /// §1.4). Only [`Pwr::scale2`] makes one, once: [`Rcc::reset_clock`] and every clock\n\
         /// enable take it, so no clock-tree read, kernel selection, clock enable or\n\
         /// peripheral start precedes it (`AET-WO-4240v` F2).\n\
         #[derive(Clone, Copy)]\n\
         pub struct Scale2 {\n    \
             _private: (),\n\
         }\n\
         \n\
         /// Why [`Pwr::scale2`] stopped, each by its own name.\n\
         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
         pub enum PwrFault {\n    \
             /// `PWR_CR3` read back `LDOEN` and `BYPASS` other than declared: another\n    \
             /// writer set them since power-on, and only a power cycle clears them.\n    \
             SupplyNotDeclared,\n    \
             /// `PWR_CSR1.ACTVOSRDY` did not read 1 within the bound.\n    \
             ActvosrdyTimeout,\n    \
             /// `PWR_D3CR.VOSRDY` did not read 1 within the bound.\n    \
             VosrdyTimeout,\n\
         }\n\
         \n",
    );
    if plan.hse.is_some() {
        w.push_str(
            "/// The clock tree at reset, read before any write: HSI at 64 MHz, every prescaler\n\
             /// 1, `TIMPRE` 0, every kernel clock selection at its reset value, and the core at\n\
             /// VOS Scale 2 ([`Scale2`]). Only [`Rcc::reset_clock`] makes one, and only\n\
             /// [`Hse::start`] takes it: this board's values were solved for PLL1 from the HSE\n\
             /// (`AET-WO-4240b`), so no peripheral starts on it.\n\
             pub struct ResetClock {\n    \
                 _private: (),\n\
             }\n\
             \n\
             /// [`Rcc::reset_clock`] read a field away from its reset value.\n\
             #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
             pub struct NotResetClock;\n\
             \n\
             /// The clock tree this board's values were solved for (`AET-WO-4240b`): PLL1 from\n\
             /// the HSE, `SWS` read PLL1, every prescaler 1, every kernel clock selection at its\n\
             /// reset value (SPI1–3 on `pll1_q_ck`, SPI4–6 on the APB clock), the core at VOS\n\
             /// Scale 2. Only [`Hse::start`] makes one, after `HSERDY`, `PLL1RDY` and `SWS` each\n\
             /// read ready; every peripheral start and every SPI clock enable takes it.\n\
             pub struct SolvedClock {\n    \
                 _private: (),\n\
             }\n\
             \n\
             /// Why [`Hse::start`] stopped, each by its own name. Nothing is written after one.\n\
             #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
             pub enum ClockFault {\n    \
                 /// `RCC_CR.HSERDY` did not read 1 within its bound.\n    \
                 HseNotReady,\n    \
                 /// `RCC_CR.PLL1RDY` did not read 1 within its bound.\n    \
                 Pll1NotLocked,\n    \
                 /// `FLASH_ACR` read back `LATENCY` or `WRHIGHFREQ` other than written.\n    \
                 FlashAcrNotTaken,\n    \
                 /// `RCC_CFGR.SWS` did not read PLL1 within its bound.\n    \
                 SwNotTaken,\n\
             }\n\
             \n",
        );
    } else if plan.needs_clock() {
        let solved_for = "/// The clock tree is the reset clock this board's values were solved for: HSI\n\
             /// at 64 MHz, every prescaler 1, `TIMPRE` 0, and every kernel clock selection at\n\
             /// its reset value; the core is at VOS Scale 2 ([`Scale2`]). Only\n\
             /// [`Rcc::reset_clock`] makes one: read it first, before any write, and never\n\
             /// again, since Renode 1.17.0 stores no selection and a later read would see 0\n\
             /// where silicon sees what the bring-up selected.\n";
        let _ = writeln!(
            w,
            "{solved_for}\
             pub struct ResetClock {{\n    \
                 _private: (),\n\
             }}\n\
             \n\
             /// [`Rcc::reset_clock`] read a field away from its reset value.\n\
             #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
             pub struct NotResetClock;\n"
        );
    }
    if plan.selects_kernels() {
        w.push_str(
            "/// The SPI controllers' kernel clocks selected, each the solved topology's\n\
             /// (`AET-WO-4230` §1.1). Only [`Rcc::select_spi_kernels`] makes one, from a\n\
             /// [`ResetClock`]; every SPI clock enable takes it, so no SPI controller is\n\
             /// clocked before its kernel clock is selected.\n\
             pub struct SpiKernels {\n    \
                 _private: (),\n\
             }\n\
             \n",
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
            "/// `{name}` bus speeds. Each mode's `TIMINGR` is the solved topology's\n\
             /// (`AET-WO-4230` §1.4: RM0433 Rev 9 §47.4.9 at the kernel clock, judged by\n\
             /// `aethelgard_ir::silicon::check_i2c` before this crate was generated).\n\
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
        for (mode, _) in I2C_MODES {
            let _ = writeln!(w, "    pub struct {mode};");
        }
        for &i in admitted {
            let (mode, _) = I2C_MODES[i];
            // `Plan::new` gave every I2C peripheral its solved timing, and
            // `check_silicon` held it to all three modes.
            let timingr = plan.i2c[name].timingr[mode];
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
    if let Some(u) = &facts.usart {
        emit_usart_shared(w, u);
    }
    // Pins.
    for (pin, p) in &plan.pins {
        let pf = &facts.pins[pin];
        let g = &facts.gpio;
        // `OTYPER` and `PUPDR` are the port's, written whole by `split`
        // (`AET-WO-4230` D5, R5).
        let (mode, _, ospeed, _) = p.policy();
        let [mw, _, sw, _, aw] = g.widths;
        let block = port_type(p.port);
        let what = match (&p.peripheral, p.role) {
            (Some(periph), _) => format!("`{periph}_{}` on AF{}", p.role.signal(), p.af),
            (None, Role::Input) => "a GPIO input".to_owned(),
            (None, Role::Analog) => "an ADC input, analog".to_owned(),
            (None, _) => "a GPIO output".to_owned(),
        };
        // A bridge's enable (`AET-WO-1211b`): configured only once its bridge's
        // inputs sit at their idle, each a parameter it cannot exist without.
        let (after, after_doc) = match plan.enable_after(pin) {
            Some(e) => {
                let mut params = String::new();
                if let Some(n) = e.timer {
                    let _ = write!(params, ", _cen: &Tim{n}<Running>");
                }
                for g in &e.gpio_inputs {
                    let _ = write!(params, ", _{}: &{g}<Output>", pin_field(g));
                }
                (
                    params,
                    "\n    /// A bridge's enable: only after its inputs are at their idle, a timer's\n    \
                     /// after its `CEN` (`AET-WO-1211b`; `1211` §2.7 (a)).",
                )
            }
            None => (String::new(), ""),
        };
        let _ = writeln!(
            w,
            "/// `{pin}`, net `{net}`: {what}.\n\
             pub struct {pin}<M> {{\n    \
                 _mode: PhantomData<M>,\n\
             }}\n\
             \n\
             impl {pin}<Unconfigured> {{{after_doc}\n    \
                 pub fn {method}<R: Registers>(self, r: &mut R{after}) -> {pin}<{state}> {{",
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
        // A GPIO input and an analog pin take their mode alone: the MCU drives
        // neither, and the port's `PUPDR` holds an input's pull (`AET-WO-1211b`).
        if !p.role.mode_only() {
            field(w, g.ospeedr, pf.ospeedr, sw, ospeed);
        }
        if p.role == Role::Output {
            // Its declared level before `MODER` (`AET-WO-1211b`; RM0433 Rev 9
            // p. 534: the value written to ODR is output once the pin is an
            // output).
            let _ = writeln!(
                w,
                "        r.write(TOKEN, Block::{block}, {:#04x}, {:#010x});",
                g.bsrr,
                p.level_bit(pf)
            );
        } else if !p.role.mode_only() {
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
        // The port's whole `OTYPER`: each solved pin's output type, every other
        // pin's its reset value 0 (RM0433 Rev 9 §11.4.2). Renode 1.17.0 does not
        // store `OT`, so a read-modify-write of it diverges (`AET-WO-4230` D5).
        let otyper = plan
            .pins
            .iter()
            .filter(|(_, p)| p.port == *port)
            .fold(0u32, |word, (pin, p)| {
                word | (p.policy().1 << facts.pins[pin].otyper)
            });
        let _ = writeln!(
            w,
            "}}\n\
             \n\
             impl {t}<Clocked> {{\n    \
                 /// The port's pins, once its whole `OTYPER` and `PUPDR` are written: each\n    \
                 /// solved pin's output type and pull, and every other pin's its reset\n    \
                 /// value (the debug port's pulls on ports A and B).\n    \
                 pub fn split<R: Registers>(self, r: &mut R) -> {t}Pins {{\n        \
                     r.write(TOKEN, Block::{t}, {off:#04x}, {otyper:#010x});\n        \
                     r.write(TOKEN, Block::{t}, {pupd_off:#04x}, {pupdr:#010x});\n        \
                     {t}Pins {{",
            off = facts.gpio.otyper,
            pupd_off = facts.gpio.pupdr,
            pupdr = port_pupdr(plan, facts, *port),
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
    let mark = w.len();
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
                         /// Only with a [`ResetClock`]: its kernel clock is the reset clock's.\n    \
                         /// `CR1` is written whole, the solved word with `PE` clear, then `TIMINGR`,\n    \
                         /// then `CR1` with `PE`; none is read (`AET-WO-4230` D5: Renode 1.17.0\n    \
                         /// stores few `CR1` fields and no `TIMINGR`).\n    \
                         pub fn init<S: {m}_speed::AtMostCeiling, R: Registers>(\n        \
                             self,\n        \
                             r: &mut R,\n        \
                             _clock: &ResetClock,\n        \
                             {params},\n    \
                         ) -> {t}<Enabled<S>> {{\n        \
                             r.write(TOKEN, Block::{block}, {cr1:#04x}, {word:#010x});\n        \
                             r.write(TOKEN, Block::{block}, {cfg:#04x}, S::TIMINGR);\n        \
                             r.write(TOKEN, Block::{block}, {cr1:#04x}, {word_pe:#010x});\n        \
                             {t} {{ _state: PhantomData }}\n    \
                         }}\n\
                     }}\n\
                     \n\
                     impl<S: {m}_speed::AtMostCeiling> {t}<Enabled<S>> {{",
                    params = params.join(",\n        "),
                    cr1 = pf.cr1,
                    cfg = pf.config,
                    word = plan.i2c[name].cr1,
                    word_pe = plan.i2c[name].cr1 | enable,
                );
            }
            Kind::Spi => {
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
                    "\n    /// Every chip select of the bus is driven high first (deselected), whatever\n    \
                     /// firmware did with it before."
                };
                // `Plan::new` gave every SPI peripheral its solved controller.
                let sf = pf
                    .spi
                    .as_ref()
                    .expect("Facts::new reads an SPI peripheral's facts");
                let c = &plan.spi[name];
                let code = spi_code(c, sf, pf.enable, pf.master.unwrap_or_default());
                let _ = writeln!(
                    w,
                    "impl {t}<Clocked> {{\n    \
                         /// Master mode with software slave select, 8-bit frames, mode 0, the solved\n    \
                         /// baud rate (`MBR` {mbr}: the {kernel_hz} Hz kernel over {div}), and the peripheral\n    \
                         /// left disabled: each transfer enables it and disables it again.{doc_selects}\n    \
                         /// `CR1` (`SSI`, `SPE` clear), `CFG1` and `CFG2` are written whole, in that\n    \
                         /// order, and none is read (`AET-WO-4230` D5): the configuration registers\n    \
                         /// change only while `SPE` is 0 (RM0433 Rev 9 §50.11.1, p. 2216), and `SSI`\n    \
                         /// is high before `MASTER` and `SSM` are, so no mode fault can arise\n    \
                         /// (§50.5.2, p. 2184). Only with a [`ResetClock`]: its kernel clock is the\n    \
                         /// reset clock's.\n    \
                         pub fn init<R: Registers>(\n        \
                             self,\n        \
                             r: &mut R,\n        \
                             _clock: &ResetClock,\n        \
                             {params},\n    \
                         ) -> {t}<Enabled<()>> {{\n\
                         {high}        \
                             r.write(TOKEN, Block::{block}, {cr1:#04x}, {idle:#010x});\n        \
                             r.write(TOKEN, Block::{block}, {cfg1:#04x}, {cfg1_word:#010x});\n        \
                             r.write(TOKEN, Block::{block}, {cfg:#04x}, {cfg2_word:#010x});\n        \
                             {t} {{ _state: PhantomData }}\n    \
                         }}\n\
                     }}\n",
                    params = params.join(",\n        "),
                    cr1 = pf.cr1,
                    cfg = pf.config,
                    cfg1 = sf.cfg1,
                    idle = code.cr1_idle,
                    cfg1_word = code.cfg1,
                    cfg2_word = code.cfg2,
                    mbr = c.mbr,
                    kernel_hz = c.kernel_hz,
                    div = 2u32 << c.mbr,
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
    if let Some(u) = &facts.usart {
        for (name, sp) in &plan.serials {
            emit_usart(w, name, sp, plan, u);
        }
    }
    if !plan.adcs.is_empty() {
        adc::emit_shared(w);
        for (name, p) in &plan.adcs {
            adc::emit(w, name, p, &facts.adcs[name], &|pin| pin.to_owned());
        }
    }
    // On an HSE board every start takes the solved clock (`AET-WO-4240b`).
    if plan.hse.is_some() {
        let starts = w.split_off(mark);
        w.push_str(
            &starts
                .replace("&ResetClock", "&SolvedClock")
                .replace("[`ResetClock`]", "[`SolvedClock`]")
                .replace("reset clock's", "solved tree's"),
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
    // Each gate, and the register its enable reads as a dummy read of the
    // block, its first: a port's and an I2C or SPI peripheral's. A timer's and a USART's do
    // not: their proofs hold that nothing reads them (`AET-WO-4300a`,
    // `AET-WO-4300b`).
    // An SPI peripheral's enable also takes the selected kernel clocks.
    let gates = facts
        .ports
        .iter()
        .map(|(port, pf)| {
            (
                port_field(*port),
                port_type(*port),
                &pf.gate,
                Some(facts.gpio.moder),
                false,
            )
        })
        .chain(facts.peripherals.iter().map(|(name, pf)| {
            (
                name.to_ascii_lowercase(),
                type_name(name),
                &pf.gate,
                Some(pf.cr1),
                plan.spi.contains_key(name),
            )
        }))
        .chain(
            facts
                .timers
                .iter()
                .map(|(n, tf)| (format!("tim{n}"), format!("Tim{n}"), &tf.gate, None, false)),
        )
        .chain(facts.serials.iter().map(|(name, sf)| {
            (
                name.to_ascii_lowercase(),
                type_name(name),
                &sf.gate,
                None,
                false,
            )
        }))
        .chain(facts.adcs.iter().map(|(name, af)| {
            (
                name.to_ascii_lowercase(),
                adc::block(name),
                &af.gate,
                None,
                false,
            )
        }));
    let mut first = true;
    for (field, t, g, dummy, spi) in gates {
        if !first {
            let _ = writeln!(w);
        }
        first = false;
        let (doc, read) = if let Some(at) = dummy {
            (
                "    /// pending in a write buffer, and a dummy read of the block, which takes the\n    \
                 /// two clock periods an enable may need (RM0433 Rev 9 §8.5.10, p. 372, steps b\n    \
                 /// and d).",
                format!("\n        let _ = r.read(TOKEN, Block::{t}, {at:#04x});"),
            )
        } else {
            (
                "    /// pending in a write buffer (RM0433 Rev 9 §8.5.10, p. 372, step b).",
                String::new(),
            )
        };
        let _ = writeln!(
            w,
            "    /// `RCC_{reg}` bit {bit}. Then the register read back, so the write is not\n\
             {doc}\n    \
             {kernels_doc}\
             pub fn enable_{field}<R: Registers>(&mut self, r: &mut R, _gated: {t}<Gated>, _scale2: Scale2{kernels}) -> {t}<Clocked> {{\n        \
                 r.modify(TOKEN, Block::Rcc, {off:#05x}, |v| v | {mask:#010x});\n        \
                 let _ = r.read(TOKEN, Block::Rcc, {off:#05x});{read}\n        \
                 {t} {{ _state: PhantomData }}\n    \
             }}",
            reg = g.register,
            bit = g.bit,
            off = g.offset,
            mask = 1u32 << g.bit,
            kernels_doc = match (spi, plan.hse.is_some()) {
                (false, _) => "",
                (true, false) =>
                    "/// Only once its kernel clock is selected ([`SpiKernels`]).\n    ",
                (true, true) => {
                    "/// Only once its kernel clock runs ([`SolvedClock`]: the select at reset).\n    "
                }
            },
            kernels = match (spi, plan.hse.is_some()) {
                (false, _) => "",
                (true, false) => ", _kernels: &SpiKernels",
                (true, true) => ", _clock: &SolvedClock",
            },
        );
    }
    if let Some(clock) = &facts.clock {
        emit_reset_clock(w, clock);
    }
    if !facts.kernel_selects.is_empty() {
        emit_select_kernels(w, facts, plan);
    }
    let (bor_pos, bor_width) = facts.flash.bor_lev;
    let _ = writeln!(
        w,
        "}}\n\
         \n\
         /// The flash controller's registers: only `FLASH_OPTSR_CUR`, read once.\n\
         pub struct Flash {{\n    \
             _private: (),\n\
         }}\n\
         \n\
         impl Flash {{\n    \
             /// `FLASH_OPTSR_CUR`, read once, whole: its `BOR_LEV` (bits {hi}:{bor_pos}) is the\n    \
             /// brown-out level the option bytes hold (RM0433 Rev 9 §4.9.8, p. 212;\n    \
             /// `AET-WO-4230` §1.6). The firmware writes nothing to the flash controller:\n    \
             /// programming the option bytes is a factory step.\n    \
             pub fn option_status<R: Registers>(self, r: &mut R) -> u32 {{\n        \
                 r.read(TOKEN, Block::Flash, {optsr:#04x})\n    \
             }}\n\
         }}\n\
         \n\
         {pwr}\
         \n\
         /// Everything on the board, once per program.\n\
         pub struct Board {{\n    \
             pub rcc: Rcc,\n    \
             pub flash: Flash,\n    \
             pub pwr: Pwr,{hse_field}",
        hi = bor_pos + bor_width - 1,
        optsr = facts.flash.optsr_cur,
        pwr = pwr_api(plan, &facts.pwr) + &hse_api(plan, facts),
        hse_field = if plan.hse.is_some() {
            "\n    pub hse: Hse,"
        } else {
            ""
        },
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
    for name in plan.serials.keys() {
        let _ = writeln!(
            w,
            "    pub {}: {}<Gated>,",
            name.to_ascii_lowercase(),
            type_name(name)
        );
    }
    for name in plan.adcs.keys() {
        let _ = writeln!(
            w,
            "    pub {}: {}<Gated>,",
            name.to_ascii_lowercase(),
            adc::block(name)
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
                     rcc: Rcc {{ _private: () }},\n            \
                     flash: Flash {{ _private: () }},\n            \
                     pwr: Pwr {{ _private: () }},{hse}",
        hse = if plan.hse.is_some() {
            "\n            hse: Hse { _private: () },"
        } else {
            ""
        },
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
    for name in plan.serials.keys() {
        let _ = writeln!(
            w,
            "            {}: {} {{ _state: PhantomData }},",
            name.to_ascii_lowercase(),
            type_name(name)
        );
    }
    for name in plan.adcs.keys() {
        let _ = writeln!(
            w,
            "            {}: {} {{ _state: PhantomData }},",
            name.to_ascii_lowercase(),
            adc::block(name)
        );
    }
    let _ = writeln!(w, "        }})\n    }}\n}}");
    emit_css_hold(w, plan, facts);
    s
}

/// An HSE board's [`Hse`]-to-be and its one method, `start` (`AET-WO-4240b`):
/// the contract's steps 1–11, every word whole and written once, each wait
/// bounded by its argument. The PLL and flash words are the solved
/// topology's, copied (`RUL-16`); `RCC_CR`'s and `RCC_CFGR`'s are this
/// generator's own ([`HseFacts::cr_words`], [`HseFacts::cfgr_word`]), every
/// place the header's. Empty on a board without HSE.
fn hse_api(plan: &Plan, facts: &Facts) -> String {
    let (Some(h), Some(f)) = (&plan.hse, &facts.hse) else {
        return String::new();
    };
    let cr = f.cr_words(h.bypass);
    let ready = |w: &str, offset: u32, mask: u32, want: u32, polls: &str, fault: &str| {
        format!(
            "        if !clock_ready(r, {offset:#05x}, {mask:#010x}, {want:#010x}, {polls}) {{\n            \
                 return Err(ClockFault::{fault}); // {w}\n        \
             }}\n"
        )
    };
    let write = |offset: u32, word: u32, what: &str| {
        format!("        r.write(TOKEN, Block::Rcc, {offset:#05x}, {word:#010x}); // {what}\n")
    };
    let mut body = String::new();
    for (name, word) in &cr[..cr.len() - 2] {
        body += &write(f.cr, *word, &format!("CR: {name}"));
    }
    body += &ready(
        "HSERDY",
        f.cr,
        1 << f.hserdy,
        1 << f.hserdy,
        "hse_polls",
        "HseNotReady",
    );
    let (_, css) = cr[cr.len() - 2];
    body += &write(f.cr, css, "CR: HSECSSON");
    body += &write(f.pllckselr, h.pllckselr, "PLLCKSELR");
    body += &write(f.pllcfgr, h.pllcfgr, "PLLCFGR");
    body += &write(f.pll1divr, h.pll1divr, "PLL1DIVR");
    let (_, pll) = cr[cr.len() - 1];
    body += &write(f.cr, pll, "CR: PLL1ON");
    body += &ready(
        "PLL1RDY",
        f.cr,
        1 << f.pll1rdy,
        1 << f.pll1rdy,
        "pll_polls",
        "Pll1NotLocked",
    );
    let _ = write!(
        body,
        "        r.write(TOKEN, Block::Flash, {acr:#04x}, {word:#010x}); // ACR: LATENCY, WRHIGHFREQ\n        \
             if (r.read(TOKEN, Block::Flash, {acr:#04x}) ^ {word:#010x}) & {mask:#010x} != 0 {{\n            \
                 return Err(ClockFault::FlashAcrNotTaken);\n        \
             }}\n",
        acr = f.acr,
        word = h.flash_acr,
        mask = f.acr_mask,
    );
    body += &write(f.cfgr, f.cfgr_word(), "CFGR: SW = PLL1");
    let sws_mask = ((1u32 << f.sws.1) - 1) << f.sws.0;
    body += &ready(
        "SWS",
        f.cfgr,
        sws_mask,
        SW_PLL1 << f.sws.0,
        "sws_polls",
        "SwNotTaken",
    );
    format!(
        "\n\
         /// The HSE and PLL1, once (`AET-WO-4240b`): [`Hse::start`] consumes it.\n\
         pub struct Hse {{\n    \
             _private: (),\n\
         }}\n\
         \n\
         /// Reads `RCC` at `offset` at most `polls` times, until a read masked by `mask` is\n\
         /// `want`.\n\
         fn clock_ready<R: Registers>(r: &mut R, offset: usize, mask: u32, want: u32, polls: u32) -> bool {{\n    \
             for _ in 0..polls {{\n        \
                 if r.read(TOKEN, Block::Rcc, offset) & mask == want {{\n            \
                     return true;\n        \
                 }}\n    \
             }}\n    \
             false\n\
         }}\n\
         \n\
         impl Hse {{\n    \
             /// H1 from the reset clock, never falling back to the HSI (`4240` D5): {source}\n    \
             /// on, at most `hse_polls` reads for `HSERDY`, then `HSECSSON`; `PLLCKSELR`,\n    \
             /// `PLLCFGR` and `PLL1DIVR` as solved, then `PLL1ON` and at most `pll_polls` reads\n    \
             /// for `PLL1RDY`; `FLASH_ACR` as solved (Table 17's VOS Scale 2 row) and read back;\n    \
             /// then `SW` = PLL1 and at most `sws_polls` reads for `SWS`. Every word is written\n    \
             /// whole, once; a wait that expires is its own [`ClockFault`], and nothing is written\n    \
             /// after it. The kernel clock selections stay at reset.\n    \
             pub fn start<R: Registers>(\n        \
                 self,\n        \
                 r: &mut R,\n        \
                 _clock: ResetClock,\n        \
                 hse_polls: u32,\n        \
                 pll_polls: u32,\n        \
                 sws_polls: u32,\n    \
             ) -> Result<SolvedClock, ClockFault> {{\n\
         {body}        \
                 Ok(SolvedClock {{ _private: () }})\n    \
             }}\n\
         }}\n",
        source = if h.bypass {
            "`HSEBYP`, then `HSEON`"
        } else {
            "`HSEON` (a crystal)"
        },
    )
}

/// An HSE board's `css_hold` (`AET-WO-4240b`), which the firmware's CSS NMI
/// calls: every motor output driven to its safe level, low (the level its
/// declared pull-down holds: the external resistor when `fitted`, the
/// internal pull-down `split` wrote when `open`, `AET-WO-4300j` D6), as a GPIO output
/// (its `BSRR` reset bit, then its `MODER` field `01`), then a park that
/// never returns. It writes no `PUPDR`. Every motor is driven, whatever its
/// timer: the CSS break event reaches TIM1/8/15/16/17
/// only through `BDTR.BKE` (RM0433 Rev 9 §38.4.20 p. 1614, §41.5.16
/// p. 1818), and [`timer_writes`] writes `BDTR` as `MOE` alone, so no timer's
/// break is armed; TIM2–5 have no break at all.
fn emit_css_hold(w: &mut String, plan: &Plan, facts: &Facts) {
    if plan.hse.is_none() {
        return;
    }
    let mw = facts.gpio.widths[0];
    let mut motors = Vec::new();
    let mut body = String::new();
    for (n, t) in &plan.timers {
        for (k, (pin, _)) in &t.channels {
            let p = &plan.pins[pin];
            let pf = &facts.pins[pin];
            motors.push(format!(
                "/// - `{pin}`, net `{}`: `TIM{n}` channel {k}.",
                p.net
            ));
            let block = port_type(p.port);
            let mask = oracle_mask(mw, pf.moder);
            let _ = write!(
                body,
                "    r.write(TOKEN, Block::{block}, {bsrr:#04x}, {br:#010x}); // {pin} low\n    \
                 r.modify(TOKEN, Block::{block}, {moder:#04x}, |v| (v & !{mask:#010x}) | {out:#010x}); // {pin} output\n",
                bsrr = facts.gpio.bsrr,
                br = 1u32 << pf.br,
                moder = facts.gpio.moder,
                out = MODE_OUTPUT << pf.moder,
            );
        }
    }
    let motors = if motors.is_empty() {
        "/// - none: the board has no motor output.".to_owned()
    } else {
        motors.join("\n")
    };
    let _ = writeln!(
        w,
        "\n\
         /// The clock security system's hold (`AET-WO-4240b`), for the firmware's NMI only:\n\
         /// the HSE failed, the part switched to the HSI and PLL1 stopped (RM0433 Rev 9 §8.5.3,\n\
         /// p. 344). Each motor output is driven low, its safe level, as a GPIO output (its\n\
         /// `BSRR` reset bit, then `MODER` output), then the core parks for good: it never\n\
         /// returns, so nothing runs on after it on the HSI. No timer's break holds a motor:\n\
         /// TIM2–5 have none, and TIM1/8/15/16/17's takes the CSS event only with\n\
         /// `BDTR.BKE`, which this crate never sets (RM0433 Rev 9 p. 1614, p. 1818). The\n\
         /// motors it drives:\n\
         {motors}\n\
         pub fn css_hold<R: Registers>(r: &mut R) -> ! {{\n    \
             css_drive(r);\n    \
             loop {{\n        \
                 core::hint::spin_loop();\n    \
             }}\n\
         }}\n\
         \n\
         /// [`css_hold`]'s writes, so a proof can follow them.\n\
         fn css_drive<R: Registers>(r: &mut R) {{\n\
         {body}}}"
    );
}

/// One timer's start (`AET-WO-4300a` §1.3), as [`emit_timer_start`] writes
/// it: (register, offset, value), in order.
fn timer_writes(n: u32, t: &TimerPlan, tf: &TimFacts) -> Vec<(&'static str, u32, u32)> {
    let mut writes: Vec<(&'static str, u32, u32)> =
        vec![("PSC", tf.psc, t.psc), ("ARR", tf.arr, t.arr)];
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
    writes
}

/// One timer's typestate and its start (`AET-WO-4300a` §1.3): the solved
/// values, every register written in full and none read, in the order `PSC`,
/// `ARR`, each used channel's `CCRk`, `CCMR1`, `CCMR2`, `CCER`, `BDTR` (only
/// on a timer with a main output enable), `EGR`, then `CR1` last.
fn emit_timer_start(w: &mut String, n: u32, t: &TimerPlan, plan: &Plan, tf: &TimFacts) {
    let writes = timer_writes(n, t, tf);
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

/// [`Rcc`]'s `reset_clock`: its registers read, and each one's fields held
/// at 0 as one mask.
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
         /// (RM0433 Rev 9 §8.7.2, §8.7.7–8.7.10, §8.7.19–8.7.22). Reads the {count} registers\n    \
         /// and writes nothing.\n\
         {named}\n    \
         /// Only with a [`Scale2`]: the core is at VOS Scale 2 before the clock tree is read.\n    \
         pub fn reset_clock<R: Registers>(&self, r: &mut R, _scale2: &Scale2) -> Result<ResetClock, NotResetClock> {{\n\
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
        count = count_word(clock.len()),
    );
}

/// [`Board`]'s `Pwr` and its one method, `scale2` (`AET-WO-4240v` §1.4): the
/// supply configuration, then VOS Scale 2, each wait bounded. Every place and
/// position is the header's ([`PwrFacts`]), and every word the generator's
/// own ([`PwrFacts::supply_bits`], [`PwrFacts::d3cr_scale2`]), never
/// `aethelgard_ir::vos`'s, which the oracle holds it to (`INV-07`).
fn pwr_api(plan: &Plan, p: &PwrFacts) -> String {
    let bits = p.supply_bits(plan.core_supply);
    let config = p.config_mask();
    let mode = match plan.core_supply {
        CoreSupplyMode::Ldo => "LDO supply: `LDOEN` 1, `BYPASS` 0",
        CoreSupplyMode::Bypass => "LDO Bypass: `LDOEN` 0, `BYPASS` 1",
    };
    format!(
        "/// The power controller: the supply configuration and the core's voltage scale,\n\
         /// once (`AET-WO-4240v` §1.4).\n\
         pub struct Pwr {{\n    \
             _private: (),\n\
         }}\n\
         \n\
         /// Reads `PWR` at `offset` at most `polls` times, until a read has a bit of `mask`.\n\
         fn pwr_ready<R: Registers>(r: &mut R, offset: usize, mask: u32, polls: u32) -> bool {{\n    \
             for _ in 0..polls {{\n        \
                 if r.read(TOKEN, Block::Pwr, offset) & mask != 0 {{\n            \
                     return true;\n        \
                 }}\n    \
             }}\n    \
             false\n\
         }}\n\
         \n\
         impl Pwr {{\n    \
             /// The core at VOS Scale 2, in RM0433 Rev 9's order (§6.4.1, p. 262; §6.8.4,\n    \
             /// p. 308; §6.8.6, p. 310), every wait at most `polls` reads:\n    \
             /// 1. `PWR_CR3` read, then written as read but for the supply bits, the\n    \
             ///    declared configuration's ({mode}) and `SCUEN` 0:\n    \
             ///    every other bit is reserved, \"must be kept at reset value\". Then it is read\n    \
             ///    back: `LDOEN` and `BYPASS` other than declared are\n    \
             ///    [`PwrFault::SupplyNotDeclared`], since the register is written once after\n    \
             ///    power-on, so another writer locked it, and nothing more is written. Equal,\n    \
             ///    it goes on, whichever write set them.\n    \
             /// 2. `PWR_CSR1.ACTVOSRDY` awaited ([`PwrFault::ActvosrdyTimeout`]).\n    \
             /// 3. `PWR_D3CR` written whole, `VOS` `10` (Scale 2), every other bit 0.\n    \
             /// 4. `PWR_D3CR.VOSRDY` awaited ([`PwrFault::VosrdyTimeout`]).\n    \
             pub fn scale2<R: Registers>(self, r: &mut R, polls: u32) -> Result<Scale2, PwrFault> {{\n        \
                 r.modify(TOKEN, Block::Pwr, {cr3:#04x}, |v| (v & !{mask:#010x}) | {bits:#010x});\n        \
                 if r.read(TOKEN, Block::Pwr, {cr3:#04x}) & {config:#010x} != {declared:#010x} {{\n            \
                     return Err(PwrFault::SupplyNotDeclared);\n        \
                 }}\n        \
                 if !pwr_ready(r, {csr1:#04x}, {actvosrdy:#010x}, polls) {{\n            \
                     return Err(PwrFault::ActvosrdyTimeout);\n        \
                 }}\n        \
                 r.write(TOKEN, Block::Pwr, {d3cr:#04x}, {scale2:#010x});\n        \
                 if !pwr_ready(r, {d3cr:#04x}, {vosrdy:#010x}, polls) {{\n            \
                     return Err(PwrFault::VosrdyTimeout);\n        \
                 }}\n        \
                 Ok(Scale2 {{ _private: () }})\n    \
             }}\n\
         }}\n",
        cr3 = p.cr3,
        csr1 = p.csr1,
        d3cr = p.d3cr,
        mask = p.supply_mask(),
        declared = bits & config,
        actvosrdy = 1u32 << p.actvosrdy,
        scale2 = p.d3cr_scale2(),
        vosrdy = 1u32 << p.vosrdy,
    )
}

/// [`Rcc`]'s `select_spi_kernels`: each select register the plan's SPI
/// controllers need, written whole (`AET-WO-4230` §1.1, D4, D5), from a
/// [`ResetClock`]: the reset clock was read before, and is never read again.
fn emit_select_kernels(w: &mut String, facts: &Facts, plan: &Plan) {
    let kernels: Vec<String> = plan
        .spi
        .iter()
        .map(|(name, c)| {
            let kernel = match c.kernel {
                SpiKernel::PerCk => "`per_ck` (HSI at `CKPERSEL`'s reset)",
                SpiKernel::HsiKerCk => "`hsi_ker_ck`",
                SpiKernel::Pll1Q => "`pll1_q_ck`",
                SpiKernel::Pclk => "the APB clock",
            };
            format!("    /// - `{name}`: {kernel}, {} Hz.", c.kernel_hz)
        })
        .collect();
    let writes: Vec<String> = facts
        .kernel_selects
        .iter()
        .map(|s| {
            format!(
                "        r.write(TOKEN, Block::Rcc, {:#05x}, {:#010x}); // {}",
                s.offset, s.word, s.register
            )
        })
        .collect();
    let _ = writeln!(
        w,
        "\n    /// Selects each solved SPI controller's kernel clock, every select register\n    \
         /// written whole, every other field at its reset value, 0 (RM0433 Rev 9\n    \
         /// §8.7.20, §8.7.22), and none read: Renode 1.17.0 stores no selection.\n\
         {kernels}\n    \
         pub fn select_spi_kernels<R: Registers>(&mut self, r: &mut R, _clock: &ResetClock) -> SpiKernels {{\n\
         {writes}\n        \
             SpiKernels {{ _private: () }}\n    \
         }}",
        kernels = kernels.join("\n"),
        writes = writes.join("\n"),
    );
}

/// `5` → `five`, for the generated prose.
fn count_word(n: usize) -> String {
    [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ]
    .get(n)
    .map_or_else(|| n.to_string(), |w| (*w).to_owned())
}

/// The oracle's writes of one timer's start, in order: (register, offset,
/// value), from the plan and the header's positions.
///
/// belt-and-suspenders: this restates `emit_timer_start`'s values and order on
/// purpose. The oracle must not read back what the crate under proof does
/// (`INV-07`), so a slip in one copy is caught by the other.
///
/// Each `CCRk` is the oracle's own idle word (`AET-WO-1211b`), not the
/// plan's: [`Plan::oracle_ccr`].
fn timer_oracle(plan: &Plan, n: u32, tf: &TimFacts) -> Vec<(String, u32, u32)> {
    let t = &plan.timers[&n];
    let mut out = vec![
        ("PSC".to_owned(), tf.psc, t.psc),
        ("ARR".to_owned(), tf.arr, t.arr),
    ];
    for k in t.channels.keys() {
        out.push((
            format!("CCR{k}"),
            tf.ccr[(k - 1) as usize],
            plan.oracle_ccr[&(n, *k)],
        ));
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

/// The items every USART shares (`AET-WO-4300b`): the bound on a send's wait,
/// its refusal, and the bounded poll of `ISR`.
fn emit_usart_shared(w: &mut String, u: &UsartFacts) {
    let _ = writeln!(
        w,
        "/// The most reads of `ISR` one USART send makes waiting for `TXFNF` before it gives up.\n\
         pub const USART_WAIT_SPINS: u32 = 20_000;\n\
         \n\
         /// `ISR.TXFNF` did not rise within its bound, and the byte was not written. A send\n\
         /// never panics and never waits without a bound.\n\
         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
         pub struct TxeTimeout;\n\
         \n\
         /// Reads a USART's `ISR` at most `spins` times for any bit of `mask`.\n\
         fn usart_wait<R: Registers>(r: &mut R, block: Block, mask: u32, spins: u32) -> bool {{\n    \
             let mut n = 0;\n    \
             while n < spins {{\n        \
                 if r.read(TOKEN, block, {isr:#04x}) & mask != 0 {{\n            \
                     return true;\n        \
                 }}\n        \
                 n += 1;\n    \
             }}\n    \
             false\n\
         }}\n",
        isr = u.isr,
    );
}

/// One serial port's USART typestate (`AET-WO-4300b` §1.4): configured from
/// `Clocked`, its two solved pins and the reset clock's token, every register
/// written in full and none read, in the order `BRR`, `CR2`, `CR1` without
/// `UE`, then `CR1` with `UE`; then a bounded send and receive, one byte each.
fn emit_usart(w: &mut String, name: &str, sp: &SerialPlan, plan: &Plan, u: &UsartFacts) {
    let t = type_name(name);
    let (brr, cr2, cr1) = usart_values(sp, u);
    let writes = [
        ("BRR", u.brr, brr),
        ("CR2", u.cr2, cr2),
        ("CR1", u.cr1, cr1),
        ("CR1 | UE", u.cr1, cr1 | (1 << u.ue)),
    ];
    let _ = writeln!(
        w,
        "/// `{name}`: serial port `{label}` (specification index {index}), TX `{tx}`, RX\n\
         /// `{rx}`, {framing}. It is configured only from `Clocked`, only on its two\n\
         /// solved pins, each in its solved function, only with a [`ResetClock`], and\n\
         /// once; it sends and receives only once configured.\n\
         pub struct {t}<S> {{\n    \
             _state: PhantomData<S>,\n\
         }}\n\
         \n\
         impl {t}<Clocked> {{\n    \
             /// The solved framing, every register written in full and none read, the\n    \
             /// USART disabled until the last write: `BRR`, `CR2`, `CR1` with `UE` clear\n    \
             /// and FIFO mode on (`FIFOEN`), then `CR1` with `UE`. `CR3` and `PRESC` keep\n    \
             /// their reset values (`OVER8` and `ONEBIT` 0, the kernel clock undivided),\n    \
             /// which the solved `BRR` assumes.\n    \
             pub fn configure<R: Registers>(\n        \
                 self,\n        \
                 r: &mut R,\n        \
                 _clock: &ResetClock,\n        \
                 _tx: {tx}<{tx_state}>,\n        \
                 _rx: {rx}<{rx_state}>,\n    \
             ) -> {t}<Enabled<()>> {{",
        label = sp.label_text(),
        index = sp.index,
        tx = sp.tx,
        rx = sp.rx,
        framing = sp.framing,
        tx_state = pin_state(&plan.pins[&sp.tx]),
        rx_state = pin_state(&plan.pins[&sp.rx]),
    );
    for (register, offset, value) in writes {
        let _ = writeln!(
            w,
            "        r.write(TOKEN, Block::{t}, {offset:#04x}, {value:#010x}); // {register}"
        );
    }
    let _ = writeln!(
        w,
        "        {t} {{ _state: PhantomData }}\n    \
             }}\n\
         }}\n\
         \n\
         impl {t}<Enabled<()>> {{\n    \
             /// Waits at most [`USART_WAIT_SPINS`] reads of `ISR` for `TXFNF`, then writes\n    \
             /// `byte` to `TDR`; or gives up, writing nothing.\n    \
             pub fn send<R: Registers>(&mut self, r: &mut R, byte: u8) -> Result<(), TxeTimeout> {{\n        \
                 if usart_wait(r, Block::{t}, {txe:#010x}, USART_WAIT_SPINS) {{\n            \
                     r.write(TOKEN, Block::{t}, {tdr:#04x}, u32::from(byte));\n            \
                     Ok(())\n        \
                 }} else {{\n            \
                     Err(TxeTimeout)\n        \
                 }}\n    \
             }}\n\
             \n    \
             /// Waits at most `polls` reads of `ISR` for `RXFNE`, then reads the low byte of\n    \
             /// `RDR`; `None`, with `RDR` unread, when nothing arrived.\n    \
             pub fn receive<R: Registers>(&mut self, r: &mut R, polls: u32) -> Option<u8> {{\n        \
                 if usart_wait(r, Block::{t}, {rxne:#010x}, polls) {{\n            \
                     Some(u8::try_from(r.read(TOKEN, Block::{t}, {rdr:#04x}) & 0xFF).unwrap_or(0))\n        \
                 }} else {{\n            \
                     None\n        \
                 }}\n    \
             }}\n\
         }}\n",
        txe = 1u32 << u.txe,
        tdr = u.tdr,
        rxne = 1u32 << u.rxne,
        rdr = u.rdr,
    );
}

/// The oracle's writes of one port's configuration, in order: (register,
/// offset, value), from the plan and the header's positions.
///
/// belt-and-suspenders: this restates `usart_values`' arithmetic and
/// `emit_usart`'s order on purpose. The oracle must not read back what the
/// crate under proof does (`INV-07`), so a slip in one copy is caught by the
/// other.
fn usart_oracle(sp: &SerialPlan, u: &UsartFacts) -> Vec<(String, u32, u32)> {
    let word = match sp.m_bits {
        0b01 => 1 << u.m0,
        0b10 => 1 << u.m1,
        _ => 0,
    };
    let parity = match sp.parity {
        Parity::None => 0,
        Parity::Even => 1 << u.pce,
        Parity::Odd => (1 << u.pce) | (1 << u.ps),
    };
    let cr1 = word | parity | (1 << u.re) | (1 << u.te) | (1 << u.fifoen);
    let inversion = if sp.inverted {
        (1 << u.rxinv) | (1 << u.txinv)
    } else {
        0
    };
    let cr2 = inversion | (u32::from(sp.stop) << u.stop.0);
    vec![
        ("BRR".to_owned(), u.brr, sp.brr),
        ("CR2".to_owned(), u.cr2, cr2),
        ("CR1".to_owned(), u.cr1, cr1),
        ("CR1 | UE".to_owned(), u.cr1, cr1 | (1 << u.ue)),
    ]
}

/// The oracle's whole `OTYPER` of `port` (`AET-WO-4230` D5), from the pins'
/// peripherals, not from [`Role::policy`] (review A5): `OTn` is bit `n`
/// (RM0433 Rev 9 §11.4.2), set exactly for a pin of an I2C peripheral, an
/// open-drain line; every other pin of the port, solved or not, push-pull, 0,
/// its reset value.
fn oracle_otyper(plan: &Plan, port: char) -> u32 {
    plan.pins
        .values()
        .filter(|p| p.port == port)
        .filter(|p| {
            p.peripheral
                .as_deref()
                .is_some_and(|n| n.starts_with("I2C"))
        })
        .fold(0, |word, p| word | (1 << p.index))
}

/// The port's whole `PUPDR` as `split` writes it (`AET-WO-4230` R5, D5): the
/// port's reset word ([`pupdr_reset`]), each solved pin's field replaced by
/// its role's pull ([`Role::policy`]).
fn port_pupdr(plan: &Plan, facts: &Facts, port: char) -> u32 {
    let pw = facts.gpio.widths[3];
    plan.pins
        .iter()
        .filter(|(_, p)| p.port == port)
        .fold(pupdr_reset(port), |word, (pin, p)| {
            let pos = facts.pins[pin].pupdr;
            (word & !(((1u32 << pw) - 1) << pos)) | (p.policy().3 << pos)
        })
}

/// The oracle's whole `PUPDR` of `port` (`AET-WO-4230` R5), from neither
/// [`Role::policy`] nor [`port_pupdr`]: `PUPDRn` is bits `2n+1:2n` (RM0433
/// Rev 9 §11.4.4, p. 542). Each solved pin takes the rule's pull
/// (`aethelgard_ir::silicon::implied_pupd`): a GPIO input's is its declared
/// pull (`AET-WO-1211b`), a motor output's follows its declared fitment
/// (`AET-WO-4300j` D6: `fitted` `00`, the declared external resistor holds
/// the line; `open` `10`, the internal pull-down), every other solved pin
/// none, `00`; every other pin of the port its reset value.
///
/// belt-and-suspenders: the reset words restate [`pupdr_reset`]'s on purpose,
/// read again from the page; the oracle must not take what the crate under
/// proof writes (`INV-07`).
fn oracle_pupdr(plan: &Plan, port: char) -> u32 {
    let reset: u32 = match port {
        'A' => 0x6400_0000,
        'B' => 0x0000_0100,
        _ => 0,
    };
    plan.pins
        .iter()
        .filter(|(_, p)| p.port == port)
        .fold(reset, |word, (pin, p)| {
            let pull = plan.oracle_pupd[pin];
            (word & !(0b11 << (2 * p.index))) | (pull << (2 * p.index))
        })
}

/// The image's condition before any write to a bridge's enable
/// (`AET-WO-1211b`; `1211` §2.7 (a)): its timer's `CR1.CEN` set (the inputs
/// hold their idle words from there, RM0433 Rev 9 p. 1558), and each GPIO
/// input already an output. From the header's positions, never from the
/// typestate the enable's method takes; `None` for any other pin.
fn oracle_enable_ready(plan: &Plan, facts: &Facts, pin: &str) -> Option<String> {
    let bridges: Vec<&OracleBridge> = plan
        .oracle_bridges
        .iter()
        .filter(|b| b.enable == pin)
        .collect();
    if bridges.is_empty() {
        return None;
    }
    let timers: BTreeSet<u32> = bridges
        .iter()
        .flat_map(|b| {
            b.inputs
                .iter()
                .filter_map(|i| i.as_ref().ok().map(|(n, _)| *n))
        })
        .collect();
    let gpio_inputs: BTreeSet<&String> = bridges
        .iter()
        .flat_map(|b| b.inputs.iter().filter_map(|i| i.as_ref().err()))
        .collect();
    let g = &facts.gpio;
    let mut terms = Vec::new();
    if let Some(tf) = &facts.tim {
        for n in &timers {
            terms.push(format!(
                "self.tim{n}[{}] & {:#010x} != 0",
                tf.cr1 / 4,
                1u32 << tf.cen
            ));
        }
    }
    for input in gpio_inputs {
        let pf = &facts.pins[input];
        let p = &plan.pins[input];
        terms.push(format!(
            "self.{}[{}] & {:#010x} == {:#010x}",
            port_field(p.port),
            g.moder / 4,
            oracle_mask(g.widths[0], pf.moder),
            MODE_OUTPUT << pf.moder
        ));
    }
    Some(terms.join(" && "))
}

/// The `BSRR` bit the oracle expects a GPIO output's configure to leave
/// (`AET-WO-1211b`): its declared level's, the set bit for `high` and the
/// reset bit for `low` (RM0433 Rev 9 §11.4.7), read from the oracle's own
/// copy of `solved.gpio`.
///
/// belt-and-suspenders: this restates [`PinPlan::level_bit`] on purpose; the
/// oracle must not take what the crate under proof writes (`INV-07`).
fn oracle_level_bit(plan: &Plan, pf: &PinFacts, pin: &str) -> u32 {
    match plan.oracle_levels.get(pin) {
        Some(Level::Low) => 1u32 << pf.br,
        Some(Level::High) => 1u32 << pf.bs,
        None => 0,
    }
}

/// A PWM output's `CCR` from its declared idle and its period's `ARR`
/// (`AET-WO-1211b`; `1211` §2.4, RM0433 Rev 9 p. 1558): `0` for `low` (OCxREF
/// held at 0), `ARR + 1` for `high` (a compare value above the auto-reload
/// holds OCxREF at 1), and the idle pulse in ticks, `pulse_us` × 10^6 /
/// `resolution_ps`, otherwise; `None` for a pulse off the tick, which the
/// solver refuses.
///
/// belt-and-suspenders: this restates `aethelgard_ir::motor::solve_timing`'s
/// idle on purpose; the proof's expected word must not be the topology's
/// copy the generator writes (`INV-07`).
fn oracle_idle_word(p: &motor::PwmTiming, arr: u32) -> Option<u32> {
    match p.idle {
        motor::Idle::Level(Level::Low) => Some(0),
        motor::Idle::Level(Level::High) => arr.checked_add(1),
        motor::Idle::PulseUs(us) => {
            let ps = u64::from(us) * 1_000_000;
            let res = p.resolution_ps;
            (res != 0 && ps % res == 0)
                .then(|| u32::try_from(ps / res).ok())
                .flatten()
        }
    }
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

/// The `TIMINGR` the oracle expects where the schedule starts an I2C
/// peripheral at `modes[k]`, the modes this generator admits at a declared
/// speed of `declared_hz`, in order (the adversarial pass's F3). At the last,
/// the mode the firmware starts the bus at (`firmware::highest`), it is the
/// topology's word for the mode the independent rule gives the declared
/// speed, `aethelgard_ir::silicon::i2c_start_mode`, never the admitted
/// mode's: a generator that starts a bus above or below the rule's mode
/// writes another word there, and the proof fails. At a lower admitted mode
/// it is that mode's own word. `None` when the rule gives no mode or the
/// topology no word for it.
fn oracle_timingr(
    timing: &SolvedI2cTiming,
    declared_hz: u32,
    modes: &[&str],
    k: usize,
) -> Option<u32> {
    let mode = if k + 1 == modes.len() {
        silicon::i2c_start_mode(u64::from(declared_hz))?
    } else {
        modes.get(k)?
    };
    timing.timingr.get(mode).copied()
}

/// The oracle's words of SPI peripheral `name` (`AET-WO-4230` §1.3, D4): the
/// independent rule's, `aethelgard_ir::silicon::spi_words`, never this
/// generator's composition ([`spi_code`], [`kernel_selects`]).
fn oracle_spi(plan: &Plan, name: &str) -> silicon::SpiWords {
    silicon::spi_words(&plan.spi, name)
        .expect("check_silicon held every SPI controller's kernel to a select code")
}

/// The oracle's select writes, by RCC offset: each solved SPI controller's
/// `kernel_select`, from the independent rule. Controllers that share a
/// register give one word for it (`spi_words` sets every solved field).
fn oracle_selects(plan: &Plan) -> BTreeMap<u32, u32> {
    plan.spi
        .keys()
        .map(|name| oracle_spi(plan, name).kernel_select)
        .collect()
}

/// The harness image's fields, in its order: RCC, the ports, the I2C and SPI
/// peripherals, then the timers. A USART's block is never in it.
fn image_fields(plan: &Plan) -> Vec<String> {
    let mut blocks: Vec<String> = vec!["rcc".into(), "pwr".into()];
    if plan.hse.is_some() {
        blocks.push("flash".into());
    }
    blocks.extend(plan.ports.iter().map(|p| port_field(*p)));
    blocks.extend(plan.peripherals.keys().map(|n| n.to_ascii_lowercase()));
    blocks.extend(plan.timers.keys().map(|n| format!("tim{n}")));
    blocks
}

/// A slice of the whole-schedule harness (`AET-WO-3303` §1.2): the pins,
/// ports, peripherals and timers whose actions it schedules. The whole
/// harness's scope is everything but a USART's pins, which no step configures
/// (`AET-WO-4300b`), and its text is the one emitted before the split; a
/// part's scope is one unit's ([`parts`]).
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
    /// The SPI kernel-clock selection (`AET-WO-4230` §1.1).
    SelectKernels,
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
            Self::SelectKernels => "the SPI kernel-clock selection".to_owned(),
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
            pins: plan
                .pins
                .iter()
                .filter(|(_, p)| !p.role.is_serial())
                .map(|(n, _)| n.clone())
                .collect(),
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
        // Every peripheral's start and every timer's takes the reset clock,
        // and an SPI peripheral's clock enable the kernel selection.
        if !self.timers.is_empty() || !self.peripherals.is_empty() {
            out.push(Action::ResetClock);
        }
        if plan.selects_kernels() && self.peripherals.iter().any(|n| plan.spi.contains_key(n)) {
            out.push(Action::SelectKernels);
        }
        out.extend(self.timers.iter().map(|n| Action::TimerClock(*n)));
        out.extend(self.timers.iter().map(|n| Action::TimerStart(*n)));
        out
    }
}

/// The parts a board proves its schedule in when there are two or more
/// (`AET-WO-3303` §1.2, `AET-WO-3303b` §1.2): one per I2C or SPI peripheral,
/// holding its pins and its chip selects; one per timer, holding its channel
/// pins; and one for the GPIO outputs no unit owns, if there are any. Each
/// part also schedules the enable of every port its pins are on, and a
/// peripheral's or a timer's the reset-clock read ([`Scope::actions`]).
/// A USART's pins are in no part (no step configures one, `AET-WO-4300b`); a
/// port whose every pin is a USART's still has its enable scheduled, in the
/// `gpio` part.
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
        let mut pins: BTreeSet<String> = t.channels.values().map(|(p, _)| p.clone()).collect();
        // A bridge's enable on this timer's channels, whose order this part
        // proves (`AET-WO-1211b`).
        // Its GPIO inputs too, ruled allowed (`1211` Q7): the enable follows
        // both in one part (the adversarial pass's F2).
        for b in plan.bridges.iter().filter(|b| b.timer == Some(*n)) {
            pins.insert(b.enable.clone());
            pins.extend(b.gpio_inputs.iter().cloned());
        }
        owned.extend(pins.iter().cloned());
        let mut part = Scope::unit(plan, format!("tim{n}"), pins);
        part.timers.insert(*n);
        out.push(part);
    }
    let rest: BTreeSet<String> = plan
        .pins
        .iter()
        .filter(|(n, p)| !owned.contains(*n) && !p.role.is_serial())
        .map(|(n, _)| n.clone())
        .collect();
    let mut gpio = Scope::unit(plan, "gpio".to_owned(), rest);
    let covered: BTreeSet<char> = out.iter().flat_map(|p| p.ports.iter().copied()).collect();
    gpio.ports
        .extend(plan.ports.iter().filter(|p| !covered.contains(*p)));
    if !gpio.ports.is_empty() {
        out.push(gpio);
    }
    out
}

/// The bits `action` may change: exactly the bits the whole harness's
/// final-state oracle lets differ from the starting image, each given to the
/// action that sets it. Computed from the plan and the register facts, as the
/// oracle is, never from `emit_lib`'s writes (`INV-07`).
fn footprint(plan: &Plan, facts: &Facts, action: &Action) -> Footprint {
    let g = &facts.gpio;
    let [mw, _, sw, _, aw] = g.widths;
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
            // `split` writes the port's whole `OTYPER` and `PUPDR`
            // (`AET-WO-4230` D5, R5).
            add(port_type(*p), g.otyper, !0);
            add(port_type(*p), g.pupdr, !0);
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
            if p.role.mode_only() {
                // A GPIO input or an analog pin: its mode alone.
            } else if p.role == Role::Output {
                add(t.clone(), g.ospeedr, oracle_mask(sw, pf.ospeedr));
                level(&mut add, pin);
            } else {
                add(t.clone(), g.ospeedr, oracle_mask(sw, pf.ospeedr));
                add(t, g.afr[pf.afr_index as usize], oracle_mask(aw, pf.afr));
            }
        }
        Action::Start(name) => {
            let pf = &facts.peripherals[name];
            let t = type_name(name);
            // `CR1` and the configuration are written whole (`AET-WO-4230` D5).
            add(t.clone(), pf.cr1, !0);
            match &pf.spi {
                None => {
                    add(t.clone(), pf.config, !0);
                    add(t, pf.txdr, !0);
                }
                Some(spi) => {
                    add(t.clone(), spi.cfg1, !0);
                    add(t, pf.config, !0);
                    // It drives its own chip selects.
                    for s in plan.selects.get(name).into_iter().flatten() {
                        level(&mut add, &s.pin);
                    }
                }
            }
        }
        // On an HSE board the reset clock's read and the bring-up, one action
        // (`AET-WO-4240b`): `RCC_CR`, `RCC_CFGR` and the PLL words whole, and
        // `FLASH_ACR`, at the oracle's offsets.
        Action::ResetClock => {
            if plan.hse.is_some() {
                for offset in ORACLE_RCC_HSE_WORDS {
                    add("Rcc".to_owned(), offset, !0);
                }
                add("Flash".to_owned(), ORACLE_FLASH_ACR, !0);
            }
        }
        // Each select register is written whole; the offsets are the oracle's
        // (`aethelgard_ir::silicon::spi_words`).
        Action::SelectKernels => {
            for (offset, _) in oracle_selects(plan) {
                add("Rcc".to_owned(), offset, !0);
            }
        }
        Action::TimerStart(n) => {
            if let Some(tf) = &facts.tim {
                for (_, offset, _) in timer_oracle(plan, *n, tf) {
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
    // The kernel selection, like a port's enable, is one owner however many
    // SPI parts schedule it.
    let mut owners: Vec<(String, Footprint)> = plan
        .ports
        .iter()
        .map(|p| Action::Port(*p))
        .chain(plan.selects_kernels().then_some(Action::SelectKernels))
        // An HSE board's bring-up is one owner too (`AET-WO-4240b`).
        .chain(
            (plan.hse.is_some()
                && Scope::whole(plan)
                    .actions(plan)
                    .contains(&Action::ResetClock))
            .then_some(Action::ResetClock),
        )
        .map(|a| (a.describe(), footprint(plan, facts, &a)))
        .collect();
    for (part, acts) in parts.iter().zip(&scheduled) {
        let mut own = Footprint::new();
        for a in acts.iter().filter(|a| {
            !matches!(
                a,
                Action::Port(_) | Action::SelectKernels | Action::ResetClock
            )
        }) {
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
    // A board proves its schedule in parts whenever `parts` yields two or
    // more, timers or not (`AET-WO-3303` §1.2, `AET-WO-3303b` §1.2): the whole
    // harness does not finish on a motor board (`AET-WO-4300a` Q5) and needs
    // more than 8 GiB on a board with an SPI bus (`AET-WO-4600` F11). A split
    // into one part is the whole, so such a board emits the whole harness (a
    // board with one timer and no other unit proved a one-part split before
    // `AET-WO-3303b`). The whole image stays: the targeted harnesses set up
    // through it.
    let parts = parts(plan);
    if parts.len() < 2 {
        emit_schedule(w, plan, facts, speeds, &Scope::whole(plan), false);
    } else {
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
    emit_flash_proof(w);
    emit_pwr_proof(w, plan);
    if let Some(h) = &plan.hse {
        emit_hse_proof(w, plan, h, facts.rcc_words);
        // Every targeted harness takes its token from the stub
        // (`solved_clock_token`), so its image's RCC holds only its gates.
        for regs in ["img", "spy"] {
            s = s.replace(
                &format!(
                    "    let clock = rcc.reset_clock(&mut {regs}, &scale2);\n    \
                     kani::assume(clock.is_ok());\n    \
                     let clock = clock.unwrap();\n"
                ),
                "    let clock = solved_clock_token(&rcc, board.hse, &scale2);\n",
            );
        }
        s = s.replace(
            "    let kernels = rcc.select_spi_kernels(&mut img, &clock);\n",
            "",
        );
        s = s.replace(", scale2, &kernels)\n", ", scale2, &clock)\n");
    }
    Ok(s)
}

// belt-and-suspenders: the oracle's RCC and flash places and bits for an HSE
// board's bring-up (`AET-WO-4240b`), written from RM0433 Rev 9; the code's
// come from the header ([`HseFacts`]), so an error in either is a failed proof
// (`INV-07`).
/// `RCC_CR` (§8.7.2, p. 383, offset 0x000, reset `0x0000 0001`), `RCC_CFGR`
/// (§8.7.7, 0x010), `RCC_PLLCKSELR` (§8.7.11, 0x028), `RCC_PLLCFGR` (§8.7.12,
/// 0x02C), `RCC_PLL1DIVR` (§8.7.13, 0x030), and `FLASH_ACR` (§4.9.1, p. 201,
/// 0x000).
const ORACLE_RCC_CR: u32 = 0x000;
const ORACLE_RCC_CFGR: u32 = 0x010;
const ORACLE_RCC_HSE_WORDS: [u32; 5] = [0x000, 0x010, 0x028, 0x02C, 0x030];
const ORACLE_FLASH_ACR: u32 = 0x000;
/// `RCC_CR`: `HSION` 0, `HSEON` 16, `HSERDY` 17, `HSEBYP` 18, `HSECSSON` 19,
/// `PLL1ON` 24, `PLL1RDY` 25.
const ORACLE_CR_RESET: u32 = 1;
const ORACLE_CR_HSEON: u32 = 1 << 16;
const ORACLE_CR_HSERDY: u32 = 1 << 17;
const ORACLE_CR_HSEBYP: u32 = 1 << 18;
const ORACLE_CR_HSECSSON: u32 = 1 << 19;
const ORACLE_CR_PLL1ON: u32 = 1 << 24;
const ORACLE_CR_PLL1RDY: u32 = 1 << 25;
/// `RCC_CFGR.SW` (bits 2:0) and `SWS` (bits 5:3); `011` is PLL1 (p. 393).
const ORACLE_CFGR_SW: u32 = 0b111;
const ORACLE_CFGR_SWS: u32 = 0b111 << 3;
const ORACLE_CFGR_PLL1: u32 = 0b011;
/// `FLASH_ACR`'s `LATENCY` (bits 3:0) and `WRHIGHFREQ` (bits 5:4), p. 201.
const ORACLE_ACR_MASK: u32 = 0x3F;

/// The oracle's `RCC_CR` words in the order the contract writes them
/// (`AET-WO-4240b` steps 1, 2, 4 and 6): (name, word).
fn oracle_cr_words(h: &HsePlan) -> Vec<(&'static str, u32)> {
    let byp = if h.bypass { ORACLE_CR_HSEBYP } else { 0 };
    let mut out = Vec::new();
    if h.bypass {
        out.push(("HSEBYP", ORACLE_CR_RESET | byp));
    }
    let on = ORACLE_CR_RESET | byp | ORACLE_CR_HSEON;
    out.push(("HSEON", on));
    out.push(("HSECSSON", on | ORACLE_CR_HSECSSON));
    out.push(("PLL1ON", on | ORACLE_CR_HSECSSON | ORACLE_CR_PLL1ON));
    out
}

/// What the schedule's image holds in each RCC word the bring-up writes
/// (offset, word, what): the last `RCC_CR` word with `HSERDY` and `PLL1RDY`,
/// `RCC_CFGR` `SW` and `SWS` PLL1, and the topology's PLL words.
fn oracle_hse_words(h: &HsePlan) -> Vec<(u32, u32, &'static str)> {
    let (_, cr) = *oracle_cr_words(h).last().expect("four words");
    vec![
        (
            ORACLE_RCC_CR,
            cr | ORACLE_CR_HSERDY | ORACLE_CR_PLL1RDY,
            "CR, HSERDY and PLL1RDY read their ON bits",
        ),
        (
            ORACLE_RCC_CFGR,
            ORACLE_CFGR_PLL1 | (ORACLE_CFGR_PLL1 << 3),
            "CFGR: SW = SWS = PLL1",
        ),
        (0x028, h.pllckselr, "PLLCKSELR, solved"),
        (0x02C, h.pllcfgr, "PLLCFGR, solved"),
        (0x030, h.pll1divr, "PLL1DIVR, solved"),
    ]
}

/// The final-state oracle's flash clause on an HSE board (`AET-WO-4240b`):
/// `FLASH_ACR` is the topology's word once the bring-up ran (an action
/// outside a part may have changed it), and every other word is unchanged.
fn emit_flash_clause(w: &mut String, plan: &Plan, words: u32, brought_up: bool) {
    let acr = ORACLE_FLASH_ACR / 4;
    if let (Some(h), true) = (&plan.hse, brought_up) {
        let _ = writeln!(
            w,
            "    kani::assert(after.flash[{acr}] == {word:#010x}, \"FLASH_ACR is not the solved word (clock.hse.flash_acr)\");",
            word = h.flash_acr,
        );
    }
    for i in (0..words).filter(|i| *i != acr) {
        let _ = writeln!(
            w,
            "    kani::assert(after.flash[{i}] == before.flash[{i}], \"the flash controller's word {i} changed\");"
        );
    }
}

/// An HSE board's bring-up proof (`AET-WO-4240b`),
/// `hse_start_is_in_order_bounded_and_named`: on a spy whose every ready read
/// (`HSERDY`, `PLL1RDY`, `SWS`) and whose `FLASH_ACR` read-back are any
/// words, at symbolic bounds of one or two reads, `Hse::start` writes exactly
/// the contract's words, in order, each once; a timeout is its own fault with
/// nothing written after it; `SolvedClock` comes back only after all three
/// ready bits read ready and the read-back matched; and on success the
/// RCC/FLASH writes are exactly the solved words. Also the stub every
/// targeted harness takes its `SolvedClock` from.
fn emit_hse_proof(w: &mut String, plan: &Plan, h: &HsePlan, rcc_words: u32) {
    let mut expect: Vec<(String, u32, u32)> = oracle_cr_words(h)
        .into_iter()
        .map(|(_, word)| ("Block::Rcc".to_owned(), ORACLE_RCC_CR, word))
        .collect();
    // Step order: HSEBYP/HSEON, (HSERDY), HSECSSON, the PLL words, PLL1ON,
    // (PLL1RDY), FLASH_ACR, (read-back), SW, (SWS).
    let n = expect.len();
    let pll1on = expect.remove(n - 1);
    expect.push(("Block::Rcc".into(), 0x028, h.pllckselr));
    expect.push(("Block::Rcc".into(), 0x02C, h.pllcfgr));
    expect.push(("Block::Rcc".into(), 0x030, h.pll1divr));
    expect.push(pll1on);
    expect.push(("Block::Flash".into(), ORACLE_FLASH_ACR, h.flash_acr));
    expect.push(("Block::Rcc".into(), ORACLE_RCC_CFGR, ORACLE_CFGR_PLL1));
    let total = expect.len();
    // How many writes each outcome has made: before HSERDY, before PLL1RDY,
    // before the read-back, before SWS (the last).
    let at_hse = if h.bypass { 2 } else { 1 };
    let at_pll = at_hse + 5;
    let at_acr = at_pll + 1;
    let checks: Vec<String> = expect
        .iter()
        .enumerate()
        .map(|(i, (b, off, word))| {
            format!(
                "            {i} => kani::assert(block == {b} && offset == {off:#05x} && value == {word:#010x}, \"write {k} is not the contract's\"),",
                k = i + 1
            )
        })
        .collect();
    let _ = writeln!(
        w,
        "\n\
         /// RCC and the flash controller as `Hse::start` may touch them: each write held\n\
         /// to the contract's, in order; each ready read and the `FLASH_ACR` read-back any\n\
         /// word. `hse`, `pll`, `sws`: the last ready read; `acr`: the read-back matched.\n\
         struct HseSpy {{\n    \
             writes: usize,\n    \
             hse_reads: u32,\n    \
             pll_reads: u32,\n    \
             sws_reads: u32,\n    \
             hse: bool,\n    \
             pll: bool,\n    \
             sws: bool,\n    \
             acr: Option<bool>,\n\
         }}\n\
         \n\
         impl sealed::Sealed for HseSpy {{}}\n\
         \n\
         impl Registers for HseSpy {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 let v: u32 = kani::any();\n        \
                 if block == Block::Rcc && offset == {cr:#05x} && self.writes == {at_hse} {{\n            \
                     kani::assert(!self.hse, \"CR polled after HSERDY read 1\");\n            \
                     self.hse_reads += 1;\n            \
                     self.hse = v & {hserdy:#010x} != 0;\n        \
                 }} else if block == Block::Rcc && offset == {cr:#05x} && self.writes == {at_pll} {{\n            \
                     kani::assert(!self.pll, \"CR polled after PLL1RDY read 1\");\n            \
                     self.pll_reads += 1;\n            \
                     self.pll = v & {pll1rdy:#010x} != 0;\n        \
                 }} else if block == Block::Flash && offset == {acr:#05x} && self.writes == {at_acr} {{\n            \
                     kani::assert(self.acr.is_none(), \"FLASH_ACR read back twice\");\n            \
                     self.acr = Some((v ^ {flash:#010x}) & {acr_mask:#010x} == 0);\n        \
                 }} else if block == Block::Rcc && offset == {cfgr:#05x} && self.writes == {total} {{\n            \
                     kani::assert(!self.sws, \"CFGR polled after SWS read PLL1\");\n            \
                     self.sws_reads += 1;\n            \
                     self.sws = v & {sws:#010x} == {sws_pll1:#010x};\n        \
                 }} else {{\n            \
                     kani::assert(false, \"start read a register out of the contract's order\");\n        \
                 }}\n        \
                 v\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 match self.writes {{\n\
         {checks}\n            \
                     _ => kani::assert(false, \"start wrote more than the contract's {total} words\"),\n        \
                 }}\n        \
                 // Each write only once its step's waits read ready.\n        \
                 kani::assert(self.writes < {at_hse} || self.hse, \"a write after HSEON before HSERDY read 1\");\n        \
                 kani::assert(self.writes < {at_pll} || self.pll, \"a write after PLL1ON before PLL1RDY read 1\");\n        \
                 kani::assert(self.writes < {at_acr} || self.acr == Some(true), \"SW written before FLASH_ACR read back as written\");\n        \
                 self.writes += 1;\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"start made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"start made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// `Hse::start` on any ready reads and any `FLASH_ACR` read-back, its three bounds\n\
         /// symbolic, one or two reads each: the contract's {total} words, in order, once each;\n\
         /// each timeout its own [`ClockFault`] after exactly its bound's reads, with nothing\n\
         /// written after it; `SolvedClock` only once `HSERDY`, `PLL1RDY` and `SWS` read ready\n\
         /// and `FLASH_ACR` read back as written, every word written.\n\
         #[kani::proof]\n\
         #[kani::unwind(3)]\n\
         fn hse_start_is_in_order_bounded_and_named() {{\n    \
             let (hb, pb, sb): (u32, u32, u32) = (kani::any(), kani::any(), kani::any());\n    \
             kani::assume((1..=2).contains(&hb) && (1..=2).contains(&pb) && (1..=2).contains(&sb));\n    \
             let mut regs = ClockReady {{ rcc: [0; {rcc_words}], acr: 0 }};\n    \
             let board = Board::take().unwrap();\n    \
             let scale2 = scale2_token(board.pwr);\n    \
             let reset = board.rcc.reset_clock(&mut regs, &scale2);\n    \
             kani::assert(reset.is_ok(), \"the reset clock refused a reset RCC\");\n    \
             let mut spy = HseSpy {{ writes: 0, hse_reads: 0, pll_reads: 0, sws_reads: 0, hse: false, pll: false, sws: false, acr: None }};\n    \
             let result = board.hse.start(&mut spy, reset.unwrap(), hb, pb, sb);\n    \
             match result {{\n        \
                 Ok(_) => {{\n            \
                     kani::assert(\n                \
                         spy.writes == {total} && spy.hse && spy.pll && spy.acr == Some(true) && spy.sws,\n                \
                         \"SolvedClock without every word written and HSERDY, PLL1RDY, the read-back and SWS ready\",\n            \
                     );\n            \
                     kani::cover!(true, \"the clock is brought up\");\n        \
                 }}\n        \
                 Err(ClockFault::HseNotReady) => {{\n            \
                     kani::assert(spy.writes == {at_hse} && !spy.hse && spy.hse_reads == hb, \"HseNotReady other than after the bound's reads of HSERDY, each 0, with nothing written after\");\n            \
                     kani::cover!(true, \"HSERDY times out\");\n        \
                 }}\n        \
                 Err(ClockFault::Pll1NotLocked) => {{\n            \
                     kani::assert(spy.writes == {at_pll} && spy.hse && !spy.pll && spy.pll_reads == pb, \"Pll1NotLocked other than after the bound's reads of PLL1RDY, each 0, with nothing written after\");\n            \
                     kani::cover!(true, \"PLL1RDY times out\");\n        \
                 }}\n        \
                 Err(ClockFault::FlashAcrNotTaken) => {{\n            \
                     kani::assert(spy.writes == {at_acr} && spy.pll && spy.acr == Some(false), \"FlashAcrNotTaken other than on a read-back that differs, with nothing written after\");\n            \
                     kani::cover!(true, \"FLASH_ACR is not taken\");\n        \
                 }}\n        \
                 Err(ClockFault::SwNotTaken) => {{\n            \
                     kani::assert(spy.writes == {total} && spy.acr == Some(true) && !spy.sws && spy.sws_reads == sb, \"SwNotTaken other than after the bound's reads of SWS, none PLL1\");\n            \
                     kani::cover!(true, \"SWS times out\");\n        \
                 }}\n    \
             }}\n\
         }}\n\
         \n\
         /// RCC and the flash controller always ready: each word reads as last written\n\
         /// (0 at first, the reset clock), `HSERDY` and `PLL1RDY` their ON bits, `SWS` `SW`.\n\
         struct ClockReady {{\n    \
             rcc: [u32; {rcc_words}],\n    \
             acr: u32,\n\
         }}\n\
         \n\
         impl sealed::Sealed for ClockReady {{}}\n\
         \n\
         impl Registers for ClockReady {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 if block == Block::Flash {{ self.acr }} else {{ self.rcc[offset / 4] }}\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 if block == Block::Flash {{\n            \
                     self.acr = value;\n        \
                 }} else if offset == {cr:#05x} {{\n            \
                     self.rcc[offset / 4] = value | ((value & {on:#010x}) << 1);\n        \
                 }} else if offset == {cfgr:#05x} {{\n            \
                     self.rcc[offset / 4] = (value & !{sws:#010x}) | ((value & {sw:#010x}) << 3);\n        \
                 }} else {{\n            \
                     self.rcc[offset / 4] = value;\n        \
                 }}\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{}}\n\
         }}\n\
         \n\
         /// The token the targeted harnesses start their peripherals with: the reset clock\n\
         /// and the bring-up on [`ClockReady`], which returns `Ok`\n\
         /// (`hse_start_is_in_order_bounded_and_named` proves the bring-up itself).\n\
         #[allow(dead_code)]\n\
         fn solved_clock_token(rcc: &Rcc, hse: Hse, scale2: &Scale2) -> SolvedClock {{\n    \
             let mut regs = ClockReady {{ rcc: [0; {rcc_words}], acr: 0 }};\n    \
             let reset = rcc.reset_clock(&mut regs, scale2);\n    \
             kani::assert(reset.is_ok(), \"the reset clock refused a reset RCC\");\n    \
             let solved = hse.start(&mut regs, reset.unwrap(), 1, 1, 1);\n    \
             kani::assert(solved.is_ok(), \"the bring-up refused a ready RCC\");\n    \
             solved.unwrap()\n\
         }}",
        cr = ORACLE_RCC_CR,
        cfgr = ORACLE_RCC_CFGR,
        acr = ORACLE_FLASH_ACR,
        hserdy = ORACLE_CR_HSERDY,
        pll1rdy = ORACLE_CR_PLL1RDY,
        on = ORACLE_CR_HSEON | ORACLE_CR_PLL1ON,
        sws = ORACLE_CFGR_SWS,
        sw = ORACLE_CFGR_SW,
        sws_pll1 = ORACLE_CFGR_PLL1 << 3,
        flash = h.flash_acr,
        acr_mask = ORACLE_ACR_MASK,
        checks = checks.join("\n"),
    );
    let _ = plan;
}

/// `FLASH_OPTSR_CUR`'s offset in the flash controller's block: "Address
/// offset: 0x01C or 0x11C" (RM0433 Rev 9 §4.9.8, p. 212). The oracle's own
/// copy; the code's comes from the header's `FLASH_TypeDef`.
const ORACLE_OPTSR_CUR: u32 = 0x01C;

/// The proof that the brown-out word is one read of `FLASH_OPTSR_CUR`, the
/// word exactly as read, and no write to the flash controller (`AET-WO-4230`
/// §1.6, D7, D10).
fn emit_flash_proof(w: &mut String) {
    let _ = writeln!(
        w,
        "\n\
         /// The flash controller as `Flash::option_status` may touch it.\n\
         struct FlashSpy {{\n    \
             reads: u32,\n    \
             word: u32,\n\
         }}\n\
         \n\
         impl sealed::Sealed for FlashSpy {{}}\n\
         \n\
         impl Registers for FlashSpy {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 kani::assert(block == Block::Flash && offset == {optsr:#05x}, \"option_status read anything but FLASH_OPTSR_CUR\");\n        \
                 self.reads += 1;\n        \
                 self.word\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, _: Block, _: usize, _: u32) {{\n        \
                 kani::assert(false, \"option_status wrote a register: the firmware never programs the option bytes\");\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"option_status made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"option_status made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// The brown-out word is exactly one read of `FLASH_OPTSR_CUR`, returned as\n\
         /// read, for any word there; nothing is written.\n\
         #[kani::proof]\n\
         fn flash_option_status_is_one_read() {{\n    \
             let word: u32 = kani::any();\n    \
             let mut spy = FlashSpy {{ reads: 0, word }};\n    \
             let board = Board::take().unwrap();\n    \
             let read = board.flash.option_status(&mut spy);\n    \
             kani::assert(spy.reads == 1, \"option_status did not read FLASH_OPTSR_CUR exactly once\");\n    \
             kani::assert(read == word, \"option_status did not return the word as read\");\n    \
             kani::cover!(read & 0xc == 0xc, \"BOR_LEV 11 is read\");\n\
         }}",
        optsr = ORACLE_OPTSR_CUR,
    );
}

// belt-and-suspenders: the oracle's offsets, written from RM0433; the code's
// come from the header, so an error in either is a failed proof (`INV-07`).
/// `PWR_CSR1`'s, `PWR_CR3`'s and `PWR_D3CR`'s offsets in the power
/// controller's block: "Address offset: 0x004", "0x00C" and "0x018" (RM0433
/// Rev 9 §6.8.2 p. 305, §6.8.4 p. 307, §6.8.6 p. 310). The oracle's own
/// copies; the code's come from the header's `PWR_TypeDef`.
const ORACLE_PWR_CSR1: u32 = 0x004;
const ORACLE_PWR_CR3: u32 = 0x00C;
const ORACLE_PWR_D3CR: u32 = 0x018;
/// `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR.VOSRDY`, each bit 13, read-only (RM0433
/// Rev 9 §6.8.2 p. 306, §6.8.6 p. 310). The oracle's own copy.
const ORACLE_PWR_READY: u32 = 1 << 13;

/// The final-state oracle's PWR clause (`AET-WO-4240v` §1.5): PWR is before
/// plus exactly the declared supply and VOS Scale 2. `CR3` is the word as it
/// was with its three supply bits the rule's; `D3CR` is the rule's whole word
/// (`VOSRDY` aside, which is read-only); every other word is unchanged. The
/// words are `aethelgard_ir::vos`'s, never the generator's
/// ([`PwrFacts::supply_bits`]), and the offsets the oracle's own.
fn emit_pwr_clause(w: &mut String, plan: &Plan, words: u32) {
    let cr3 = ORACLE_PWR_CR3 / 4;
    let d3cr = ORACLE_PWR_D3CR / 4;
    let _ = writeln!(
        w,
        "    // PWR is before plus exactly the declared supply and VOS Scale 2\n    \
         // (aethelgard_ir::vos::cr3_supply_bits, D3CR_VOS_SCALE2).\n    \
         kani::assert(\n        \
             after.pwr[{cr3}] == (before.pwr[{cr3}] & !{mask:#010x}) | {bits:#010x},\n        \
             \"PWR is not before plus exactly the declared supply and VOS Scale 2: CR3's supply bits are not the rule's, or another bit moved\",\n    \
         );\n    \
         kani::assert(\n        \
             after.pwr[{d3cr}] & {vos:#010x} == {scale2:#010x},\n        \
             \"PWR is not before plus exactly the declared supply and VOS Scale 2: D3CR.VOS is not 10\",\n    \
         );\n    \
         kani::assert(\n        \
             after.pwr[{d3cr}] & !{ready:#010x} == {scale2:#010x},\n        \
             \"PWR is not before plus exactly the declared supply and VOS Scale 2: D3CR is not the rule's whole word\",\n    \
         );",
        mask = vos::CR3_SUPPLY_MASK,
        bits = vos::cr3_supply_bits(plan.core_supply),
        vos = vos::D3CR_VOS_MASK,
        scale2 = vos::D3CR_VOS_SCALE2,
        ready = ORACLE_PWR_READY,
    );
    for i in (0..words).filter(|i| *i != cr3 && *i != d3cr) {
        let _ = writeln!(
            w,
            "    kani::assert(after.pwr[{i}] == before.pwr[{i}], \"PWR is not before plus exactly the declared supply and VOS Scale 2: word {i} changed\");"
        );
    }
}

/// The proof of the bring-up's first step (`AET-WO-4240v` §1.5),
/// `pwr_scale2_is_once_in_order_and_bounded`: on a spy whose every read is
/// any word, at a bound of two polls, `Pwr::scale2` writes exactly `CR3`, then
/// `D3CR`, with the rule's words; reads `CR3` once before its write and once
/// after; polls `CSR1` only after `CR3` read back as declared and `D3CR` only
/// after its write, each at most the bound and never after the bit read 1;
/// writes nothing after a fault; touches no block but PWR; and returns
/// `Scale2` exactly when `CR3` read back as declared and both bits read 1, and
/// each fault by its own name. Every expected word is `aethelgard_ir::vos`'s.
///
/// With a reset-clock read on the board, also the stub the targeted harnesses
/// take their token from: it answers `CR3` as written and every other read
/// with all ones, so `scale2` returns `Ok` on its one path.
fn emit_pwr_proof(w: &mut String, plan: &Plan) {
    let bits = vos::cr3_supply_bits(plan.core_supply);
    let declared = bits & vos::CR3_SUPPLY_CONFIG_MASK;
    let _ = writeln!(
        w,
        "\n\
         /// PWR as `Pwr::scale2` may touch it: every read any word, the order held at\n\
         /// each access. `step`: 0 before the `CR3` read, 1 after it, 2 after the `CR3`\n\
         /// write, 3 after the read-back, 4 after the `D3CR` write.\n\
         struct PwrSpy {{\n    \
             step: u8,\n    \
             cr3: u32,\n    \
             readback: u32,\n    \
             csr1_reads: u32,\n    \
             actvosrdy: bool,\n    \
             d3cr_reads: u32,\n    \
             vosrdy: bool,\n    \
             writes: u32,\n\
         }}\n\
         \n\
         impl sealed::Sealed for PwrSpy {{}}\n\
         \n\
         impl Registers for PwrSpy {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 kani::assert(block == Block::Pwr, \"scale2 touched a block other than PWR\");\n        \
                 let v: u32 = kani::any();\n        \
                 if offset == {cr3:#05x} {{\n            \
                     kani::assert(self.step == 0 || self.step == 2, \"CR3 read other than once before its write and once after\");\n            \
                     if self.step == 0 {{\n                \
                         self.cr3 = v;\n                \
                         self.step = 1;\n            \
                     }} else {{\n                \
                         self.readback = v;\n                \
                         self.step = 3;\n            \
                     }}\n        \
                 }} else if offset == {csr1:#05x} {{\n            \
                     kani::assert(\n                \
                         self.step == 3 && self.readback & {config:#010x} == {declared:#010x},\n                \
                         \"CSR1 polled before CR3 read back as declared (aethelgard_ir::vos::cr3_supply_bits)\",\n            \
                     );\n            \
                     kani::assert(!self.actvosrdy, \"CSR1 polled after ACTVOSRDY read 1\");\n            \
                     self.csr1_reads += 1;\n            \
                     self.actvosrdy = v & {ready:#010x} != 0;\n        \
                 }} else if offset == {d3cr:#05x} {{\n            \
                     kani::assert(self.step == 4, \"D3CR read before its write\");\n            \
                     kani::assert(!self.vosrdy, \"D3CR polled after VOSRDY read 1\");\n            \
                     self.d3cr_reads += 1;\n            \
                     self.vosrdy = v & {ready:#010x} != 0;\n        \
                 }} else {{\n            \
                     kani::assert(false, \"scale2 read a PWR register other than CSR1, CR3 and D3CR\");\n        \
                 }}\n        \
                 v\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 kani::assert(block == Block::Pwr, \"scale2 touched a block other than PWR\");\n        \
                 self.writes += 1;\n        \
                 if offset == {cr3:#05x} {{\n            \
                     kani::assert(self.step == 1, \"CR3 written other than once, after its read\");\n            \
                     kani::assert(\n                \
                         value == (self.cr3 & !{mask:#010x}) | {bits:#010x},\n                \
                         \"CR3 is not the word as read with the rule's supply bits (aethelgard_ir::vos::cr3_supply_bits)\",\n            \
                     );\n            \
                     self.step = 2;\n        \
                 }} else if offset == {d3cr:#05x} {{\n            \
                     kani::assert(\n                \
                         self.step == 3 && self.readback & {config:#010x} == {declared:#010x} && self.actvosrdy,\n                \
                         \"D3CR written other than once, after CR3 read back as declared and ACTVOSRDY read 1\",\n            \
                     );\n            \
                     kani::assert(value == {scale2:#010x}, \"D3CR is not the rule's word (aethelgard_ir::vos::D3CR_VOS_SCALE2)\");\n            \
                     self.step = 4;\n        \
                 }} else {{\n            \
                     kani::assert(false, \"scale2 wrote a PWR register other than CR3 and D3CR\");\n        \
                 }}\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"scale2 made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"scale2 made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// `Pwr::scale2` at a bound of two polls, on any words: `CR3` then `D3CR`\n\
         /// written, once each, in order, with the rule's words; nothing after a fault;\n\
         /// each outcome by its own name, `Scale2` only when `CR3` read back as declared\n\
         /// and `ACTVOSRDY` and `VOSRDY` each read 1 within the bound.\n\
         #[kani::proof]\n\
         #[kani::unwind(3)]\n\
         fn pwr_scale2_is_once_in_order_and_bounded() {{\n    \
             let mut spy = PwrSpy {{\n        \
                 step: 0,\n        \
                 cr3: 0,\n        \
                 readback: 0,\n        \
                 csr1_reads: 0,\n        \
                 actvosrdy: false,\n        \
                 d3cr_reads: 0,\n        \
                 vosrdy: false,\n        \
                 writes: 0,\n    \
             }};\n    \
             let board = Board::take().unwrap();\n    \
             let result = board.pwr.scale2(&mut spy, 2);\n    \
             let declared = spy.step >= 3 && spy.readback & {config:#010x} == {declared:#010x};\n    \
             kani::assert(spy.csr1_reads <= 2 && spy.d3cr_reads <= 2, \"a wait read past its bound\");\n    \
             match result {{\n        \
                 Ok(_) => {{\n            \
                     kani::assert(\n                \
                         spy.step == 4 && spy.writes == 2 && declared && spy.actvosrdy && spy.vosrdy,\n                \
                         \"Scale2 without CR3 read back as declared and both bits read 1 after both writes\",\n            \
                     );\n            \
                     kani::cover!(true, \"Scale 2 reached\");\n        \
                 }}\n        \
                 Err(PwrFault::SupplyNotDeclared) => {{\n            \
                     kani::assert(\n                \
                         spy.step == 3 && spy.writes == 1 && !declared,\n                \
                         \"SupplyNotDeclared other than on a CR3 read back undeclared, right after its one write\",\n            \
                     );\n            \
                     kani::cover!(true, \"the supply is not the declared one\");\n        \
                 }}\n        \
                 Err(PwrFault::ActvosrdyTimeout) => {{\n            \
                     kani::assert(\n                \
                         spy.step == 3 && spy.writes == 1 && declared && !spy.actvosrdy && spy.csr1_reads == 2,\n                \
                         \"ActvosrdyTimeout other than after the bound's reads of CSR1, each 0\",\n            \
                     );\n            \
                     kani::cover!(true, \"ACTVOSRDY times out\");\n        \
                 }}\n        \
                 Err(PwrFault::VosrdyTimeout) => {{\n            \
                     kani::assert(\n                \
                         spy.step == 4 && spy.writes == 2 && spy.actvosrdy && !spy.vosrdy && spy.d3cr_reads == 2,\n                \
                         \"VosrdyTimeout other than after the bound's reads of D3CR, each 0\",\n            \
                     );\n            \
                     kani::cover!(true, \"VOSRDY times out\");\n        \
                 }}\n    \
             }}\n\
         }}",
        cr3 = ORACLE_PWR_CR3,
        csr1 = ORACLE_PWR_CSR1,
        d3cr = ORACLE_PWR_D3CR,
        ready = ORACLE_PWR_READY,
        mask = vos::CR3_SUPPLY_MASK,
        config = vos::CR3_SUPPLY_CONFIG_MASK,
        scale2 = vos::D3CR_VOS_SCALE2,
    );
    if plan.needs_clock() {
        let _ = writeln!(
            w,
            "\n\
             /// PWR always ready: `CR3` reads as last written, every other read all ones.\n\
             struct PwrReady {{\n    \
                 cr3: u32,\n\
             }}\n\
             \n\
             impl sealed::Sealed for PwrReady {{}}\n\
             \n\
             impl Registers for PwrReady {{\n    \
                 fn read(&mut self, _: sealed::Token, _: Block, offset: usize) -> u32 {{\n        \
                     if offset == {cr3:#05x} {{ self.cr3 }} else {{ !0 }}\n    \
                 }}\n    \
                 fn write(&mut self, _: sealed::Token, _: Block, offset: usize, value: u32) {{\n        \
                     if offset == {cr3:#05x} {{\n            \
                         self.cr3 = value;\n        \
                     }}\n    \
                 }}\n    \
                 fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                     0\n    \
                 }}\n    \
                 fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{}}\n\
             }}\n\
             \n\
             /// The token the targeted harnesses read the reset clock with: `scale2` on\n\
             /// [`PwrReady`], which returns `Ok` (`pwr_scale2_is_once_in_order_and_bounded`\n\
             /// proves the sequence itself).\n\
             fn scale2_token(pwr: Pwr) -> Scale2 {{\n    \
                 let token = pwr.scale2(&mut PwrReady {{ cr3: 0 }}, 1);\n    \
                 kani::assert(token.is_ok(), \"scale2 refused a ready PWR\");\n    \
                 token.unwrap()\n\
             }}",
            cr3 = ORACLE_PWR_CR3,
        );
    }
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
    let mut blocks: Vec<(String, String, u32, Option<&Gate>)> = vec![
        ("Rcc".into(), "rcc".into(), facts.rcc_words, None),
        ("Pwr".into(), "pwr".into(), facts.pwr.words, None),
    ];
    // An HSE board's image holds the flash controller: the bring-up writes
    // `FLASH_ACR` and reads it back (`AET-WO-4240b`).
    if let Some(h) = &facts.hse {
        blocks.push(("Flash".into(), "flash".into(), h.flash_words, None));
    }
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
    let partial = blocks.len()
        < 2 + usize::from(facts.hse.is_some())
            + plan.ports.len()
            + plan.peripherals.len()
            + plan.timers.len();
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
    // A USART's block is not in the image: no step of any harness on it
    // reaches one (`usartN_start_writes_the_solved_values` proves the
    // configuration on a spy of its own). In a part, `clocked` refuses it.
    if partial {
        let _ = writeln!(
            w,
            "            _ => unreachable!(\"`clocked` refuses a block outside the part\"),"
        );
    } else {
        // Nor is the flash controller's: only `Flash::option_status` reads it
        // (`flash_option_status_is_one_read`). On an HSE board it is.
        if facts.hse.is_none() {
            let _ = writeln!(
                w,
                "            Block::Flash => {{\n                \
                     kani::assert(false, \"the flash controller was accessed through the harness image\");\n                \
                     &mut self.rcc\n            \
                 }}"
            );
        }
        for name in plan.serials.keys() {
            let _ = writeln!(
                w,
                "            Block::{} => {{\n                \
                     kani::assert(false, \"{name} was accessed through the harness image\");\n                \
                     &mut self.rcc\n            \
                 }}",
                type_name(name)
            );
        }
        // An ADC's blocks (`AET-WO-1211b`): its bring-up is proved on its own
        // spy (`adcN_bring_up_is_in_order_and_bounded`).
        for name in plan.adcs.keys() {
            for b in [adc::block(name), adc::common_block(name)] {
                let _ = writeln!(
                    w,
                    "            Block::{b} => {{\n                \
                         kani::assert(false, \"{name} was accessed through the harness image\");\n                \
                         &mut self.rcc\n            \
                     }}"
                );
            }
        }
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
                 Block::Rcc => {{}}\n            \
                 Block::Pwr => {{}}"
    );
    if !partial || facts.hse.is_some() {
        let _ = writeln!(w, "            Block::Flash => {{}}");
    }
    if !partial {
        for name in plan.serials.keys() {
            let _ = writeln!(w, "            Block::{} => {{}}", type_name(name));
        }
        for name in plan.adcs.keys() {
            let _ = writeln!(
                w,
                "            Block::{} => {{}}\n            Block::{} => {{}}",
                adc::block(name),
                adc::common_block(name)
            );
        }
    }
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
    // `OTYPER` is written whole, never read (`AET-WO-4230` D5): Renode 1.17.0
    // does not store `OT`, so a read-modify-write of it diverges. `PUPDR` is
    // written whole too (R5): no pin read-modify-writes it.
    for port in &scope.ports {
        let _ = writeln!(
            w,
            "        kani::assert(\n            \
                 !(block == Block::{t} && offset == {otyper:#04x}),\n            \
                 \"{t} OTYPER was read\",\n        \
             );\n        \
             kani::assert(\n            \
                 !(block == Block::{t} && offset == {pupdr:#04x}),\n            \
                 \"{t} PUPDR was read\",\n        \
             );",
            t = port_type(*port),
            otyper = g.otyper,
            pupdr = g.pupdr,
        );
    }
    // The configuration registers are written whole, never read (`AET-WO-4230`
    // D5): Renode 1.17.0 stores no `TIMINGR` and few `CFG1`/`CFG2` fields. A
    // `CR1` is read once by its clock enable's dummy read (RM0433 Rev 9 §8.5.10),
    // so its whole word is the final check's.
    for (name, pf) in facts
        .peripherals
        .iter()
        .filter(|(n, _)| scope.peripherals.contains(*n))
    {
        let t = type_name(name);
        let regs: Vec<(&str, u32)> = match &pf.spi {
            None => vec![("TIMINGR", pf.config)],
            Some(spi) => vec![("CFG1", spi.cfg1), ("CFG2", pf.config)],
        };
        for (reg, offset) in regs {
            let _ = writeln!(
                w,
                "        kani::assert(\n            \
                     !(block == Block::{t} && offset == {offset:#04x}),\n            \
                     \"{t} {reg} was read\",\n        \
                 );"
            );
        }
    }
    if facts.hse.is_some() {
        let _ = writeln!(
            w,
            "        kani::assert(\n            \
                 block != Block::Flash || offset == {acr:#05x},\n            \
                 \"a flash controller register other than ACR was read\",\n        \
             );",
            acr = ORACLE_FLASH_ACR,
        );
    }
    let _ = writeln!(
        w,
        "        *self.slot(block, offset)\n    \
         }}\n    \
         fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
             self.clocked(block);\n        \
             // PWR (RM0433 Rev 9 §6.8): only `CR3` and `D3CR` are written, and\n        \
             // `D3CR.VOSRDY` is read-only, so a write leaves it as it was.\n        \
             let value = if block == Block::Pwr {{\n            \
                 kani::assert(\n                \
                     offset == {cr3:#05x} || offset == {d3cr:#05x},\n                \
                     \"a PWR register other than CR3 and D3CR was written\",\n            \
                 );\n            \
                 if offset == {d3cr:#05x} {{\n                \
                     (value & !{ready:#010x}) | (self.pwr[{d}] & {ready:#010x})\n            \
                 }} else {{\n                \
                     value\n            \
                 }}\n        \
             }} else {{\n            \
                 value\n        \
             }};",
        cr3 = ORACLE_PWR_CR3,
        d3cr = ORACLE_PWR_D3CR,
        d = ORACLE_PWR_D3CR / 4,
        ready = ORACLE_PWR_READY,
    );
    // An HSE board (`AET-WO-4240b`): `HSERDY` and `PLL1RDY` read their ON bits
    // and `SWS` reads `SW`, at once, as Renode 1.17.0's do; the waits are
    // `hse_start_is_in_order_bounded_and_named`'s, on any reads. The flash
    // controller takes `ACR` alone.
    if facts.hse.is_some() {
        let _ = writeln!(
            w,
            "        // RCC (RM0433 Rev 9 §8.7.2, §8.7.7): HSERDY and PLL1RDY follow HSEON and\n        \
             // PLL1ON, SWS follows SW, at once.\n        \
             let value = if block == Block::Rcc && offset == {cr:#05x} {{\n            \
                 value | ((value & {on:#010x}) << 1)\n        \
             }} else if block == Block::Rcc && offset == {cfgr:#05x} {{\n            \
                 (value & !{sws:#010x}) | ((value & {sw:#010x}) << 3)\n        \
             }} else {{\n            \
                 value\n        \
             }};\n        \
             kani::assert(\n            \
                 block != Block::Flash || offset == {acr:#05x},\n            \
                 \"a flash controller register other than ACR was written\",\n        \
             );",
            cr = ORACLE_RCC_CR,
            cfgr = ORACLE_RCC_CFGR,
            on = ORACLE_CR_HSEON | ORACLE_CR_PLL1ON,
            sws = ORACLE_CFGR_SWS,
            sw = ORACLE_CFGR_SW,
            acr = ORACLE_FLASH_ACR,
        );
    }
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
        // `TIMINGR` is written while `CR1` is the rule's whole word with `PE`
        // clear (the filters are set only with `PE` 0, RM0433 Rev 9 §47.9.1),
        // so a `CR1` read-modify-write before it is caught here, though the
        // whole `CR1 | PE` after it would hide it from the final check.
        if pf.spi.is_none() {
            let _ = writeln!(
                w,
                "        if block == Block::{t} && offset == {cfg:#04x} {{\n            \
                     kani::assert(\n                \
                         self.{field}[{cr1}] == {word:#010x},\n                \
                         \"{t} TIMINGR written while CR1 is not the rule's word (aethelgard_ir::silicon::i2c_cr1)\",\n            \
                     );\n        \
                 }}",
                cfg = pf.config,
                cr1 = pf.cr1 / 4,
                word = silicon::i2c_cr1(),
            );
        }
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
            // No `SPE` before master mode with software slave select (the
            // 3302 bench, NUCLEO-H753ZI rev V: with `SSM` 0 the first `SPE`
            // raised `MODF`, `SR` 0x00001202, and cleared `MASTER`). The bits
            // are the independent rule's (`aethelgard_ir::silicon::spi_words`).
            let words = oracle_spi(plan, name);
            let _ = writeln!(
                w,
                "        if block == Block::{t} && offset == {cr1_off:#04x} && value & {en:#010x} != 0 {{\n            \
                     kani::assert(\n                \
                         self.{field}[{cfg2}] & {master_ssm:#010x} == {master_ssm:#010x},\n                \
                         \"{t} SPE set before CFG2 holds MASTER and SSM\",\n            \
                     );\n            \
                     kani::assert(value & {ssi:#010x} != 0, \"{t} SPE set without SSI\");\n        \
                 }}",
                cr1_off = pf.cr1,
                cfg2 = pf.config / 4,
                en = 1u32 << pf.enable,
                master_ssm = words.cfg2,
                ssi = words.cr1_idle,
            );
        }
    }
    // GPIO write-time rules (adversarial E6, E7): a BSRR write may touch only a
    // solved output's set or reset bit; a MODER write may move only solved pins,
    // only to their final mode, and only once every other field of that pin
    // already holds its final value (an output's level already high). In a
    // part, only its own pins' rules: the footprint assertion refuses a write
    // to any other pin's bits.
    let [mw, _, sw, pw, aw] = g.widths;
    for port in &scope.ports {
        let t = port_type(*port);
        let field = port_field(*port);
        let on_port: Vec<(&String, &PinPlan)> =
            plan.pins.iter().filter(|(_, p)| p.port == *port).collect();
        // The whole harness also holds a USART's pins to their mode rule,
        // though no step configures one; in a part, the footprint does.
        let own: Vec<(&String, &PinPlan)> = on_port
            .iter()
            .filter(|(n, p)| scope.pins.contains(*n) || (part.is_none() && p.role.is_serial()))
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
            "        if block == Block::{t} && offset == {otyper:#04x} {{\n            \
                 kani::assert(value == {word:#010x}, \"{t} OTYPER is not the solved word, whole\");\n        \
             }}\n        \
             if block == Block::{t} && offset == {pupdr:#04x} {{\n            \
                 kani::assert(value == {pull:#010x}, \"{t} PUPDR is not the solved word, whole\");\n        \
             }}",
            otyper = g.otyper,
            word = oracle_otyper(plan, *port),
            pupdr = g.pupdr,
            pull = oracle_pupdr(plan, *port),
        );
        // A bridge's enable is written only after its inputs' idle
        // (`AET-WO-1211b`).
        let mut enable_rules = String::new();
        for (pin, _) in &own {
            if let Some(cond) = oracle_enable_ready(plan, facts, pin) {
                let pf = &facts.pins[*pin];
                let _ = write!(
                    enable_rules,
                    "\n            if value & {bits:#010x} != 0 {{\n                \
                         kani::assert({cond}, \"{pin}, a bridge's enable, written before its inputs sat at their idle\");\n            \
                     }}",
                    bits = (1u32 << pf.bs) | (1u32 << pf.br),
                );
            }
        }
        let _ = writeln!(
            w,
            "        if block == Block::{t} && offset == {bsrr:#04x} {{\n            \
                 kani::assert({rule}, \"{t} BSRR write touches a pin {owner}\");{enable_rules}\n            \
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
            let (mode, _, ospeed, _) = p.policy();
            let mmask = oracle_mask(mw, pf.moder);
            let ot = 1u32 << p.index;
            let mut ready = vec![format!(
                "self.{field}[{}] & {ot:#010x} == {:#010x}",
                g.otyper / 4,
                oracle_otyper(plan, *port) & ot
            )];
            if !p.role.mode_only() {
                ready.push(format!(
                    "self.{field}[{}] & {:#010x} == {:#010x}",
                    g.ospeedr / 4,
                    oracle_mask(sw, pf.ospeedr),
                    ospeed << pf.ospeedr
                ));
            }
            ready.push(format!(
                "self.{field}[{}] & {:#010x} == {:#010x}",
                g.pupdr / 4,
                oracle_mask(pw, pf.pupdr),
                oracle_pupdr(plan, *port) & oracle_mask(pw, pf.pupdr)
            ));
            if p.role == Role::Output {
                // The declared level's word, the oracle's own (`AET-WO-1211b`).
                ready.push(format!(
                    "self.{field}[{}] == {:#010x}",
                    g.bsrr / 4,
                    oracle_level_bit(plan, pf, pin)
                ));
                // A bridge's enable: after its inputs' idle (`AET-WO-1211b`).
                ready.extend(oracle_enable_ready(plan, facts, pin));
            } else if p.role.mode_only() {
                // A GPIO input or an analog pin: no alternate function.
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
        // PWR is written only by the bring-up's first step, before the
        // schedule (no action owns it, so no rely covers it), and the final
        // check fixes every PWR word exactly.
        let _ = writeln!(
            w,
            "        if block != Block::Pwr {{\n            \
                 let changed = if matches!((block, offset), {}) {{\n                \
                     value\n            \
                 }} else {{\n                \
                     *self.slot(block, offset) ^ value\n            \
                 }};\n            \
                 kani::assert(\n                \
                     changed & !self.footprint(block, offset) == 0,\n                \
                     \"a write changed a bit outside the running action's footprint\",\n            \
                 );\n        \
             }}",
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
    emit_pwr_clause(w, plan, facts.pwr.words);
    if let Some(h) = &facts.hse {
        emit_flash_clause(w, plan, h.flash_words, acts.contains(&Action::ResetClock));
    }
    for port in &scope.ports {
        let field = port_field(*port);
        let mut expect: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let mut own: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let [mw, _, sw, _, aw] = g.widths;
        let mut touched: BTreeSet<u32> = BTreeSet::new();
        // A USART's pins are not configured by the schedule: their fields
        // are held unchanged with the unsolved pins' (`AET-WO-4300b`).
        for (pin, p) in plan
            .pins
            .iter()
            .filter(|(_, p)| p.port == *port && !p.role.is_serial())
        {
            let pf = &facts.pins[pin];
            let (mode, _, ospeed, _) = p.policy();
            let fields = |acc: &mut BTreeMap<u32, (u32, u32)>| {
                expect_field(acc, g.moder, pf.moder, mw, mode);
                if !p.role.mode_only() {
                    expect_field(acc, g.ospeedr, pf.ospeedr, sw, ospeed);
                }
                if p.role != Role::Output && !p.role.mode_only() {
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
        // `OTYPER` and `PUPDR`: the whole solved words, every pin's, whatever
        // they held.
        let _ = writeln!(
            w,
            "    kani::assert(after.{field}[{i}] == {word:#010x}, \"{t} OTYPER is not the solved word\");\n    \
             kani::assert(after.{field}[{j}] == {pull:#010x}, \"{t} PUPDR is not the solved word\");",
            i = g.otyper / 4,
            t = port_type(*port),
            word = oracle_otyper(plan, *port),
            j = g.pupdr / 4,
            pull = oracle_pupdr(plan, *port),
        );
        touched.insert(g.otyper);
        touched.insert(g.pupdr);
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
                } else if scope.pins.contains(*pin) {
                    // A pin device's output (`AET-WO-1211b`): at its declared
                    // level, the oracle's own copy of `solved.gpio`.
                    let want = match plan.oracle_levels.get(*pin) {
                        Some(Level::High) => bit,
                        _ => 0,
                    };
                    let _ = writeln!(
                        w,
                        "    kani::assert(after.{field}[{i}] & {bit:#010x} == {want:#010x}, \"{pin} is not at its declared initial level\");"
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
    // An HSE board's bring-up (`AET-WO-4240b`): RCC is before plus exactly the
    // solved words, with the ready bits the image sets.
    if let (Some(h), true) = (&plan.hse, acts.contains(&Action::ResetClock)) {
        for (offset, word, what) in oracle_hse_words(h) {
            let _ = writeln!(w, "    rcc[{i}] = {word:#010x}; // {what}", i = offset / 4);
        }
    }
    // Each kernel-clock select register, the rule's whole word.
    if acts.contains(&Action::SelectKernels) {
        for (offset, word) in oracle_selects(plan) {
            let _ = writeln!(
                w,
                "    rcc[{i}] = {word:#010x}; // the kernel selection, whole (aethelgard_ir::silicon::spi_words)",
                i = offset / 4
            );
        }
    }
    if part.is_none() {
        let _ = writeln!(
            w,
            "    // Word by word: `==` on the arrays is a `memcmp` loop under CBMC.\n    \
             for i in 0..{n} {{\n        \
                 kani::assert(after.rcc[i] == rcc[i], \"RCC is not before plus exactly the solved clock gates and selections\");\n    \
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
                     \"RCC is not before plus exactly the solved clock gates and selections\",\n        \
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
        // Every word the start writes is whole, and the expected word is the
        // independent rule's (`AET-WO-4230` §1.3, D4, A5): an I2C peripheral is
        // left enabled; an SPI one idle and disabled, as every transfer leaves
        // it (`AET-WO-4200`).
        let written = match &pf.spi {
            None => {
                let _ = writeln!(
                    w,
                    "    kani::assert(\n        \
                         after.{field}[{cr1}] == {word:#010x},\n        \
                         \"{t} CR1 is not the rule's word with PE (aethelgard_ir::silicon::i2c_cr1)\",\n    \
                     );\n    \
                     kani::assert(\n        \
                         after.{field}[{c}] == {field}_timingr,\n        \
                         \"{t} TIMINGR is not the chosen speed's\",\n    \
                     );",
                    word = silicon::i2c_cr1() | (1u32 << pf.enable),
                    c = pf.config / 4,
                );
                vec![pf.cr1, pf.config, pf.txdr]
            }
            Some(spi) => {
                let words = oracle_spi(plan, name);
                for (reg, offset, word) in [
                    ("CR1", pf.cr1, words.cr1_idle),
                    ("CFG1", spi.cfg1, words.cfg1),
                    ("CFG2", pf.config, words.cfg2),
                ] {
                    let _ = writeln!(
                        w,
                        "    kani::assert(\n        \
                             after.{field}[{i}] == {word:#010x},\n        \
                             \"{t} {reg} is not the rule's word (aethelgard_ir::silicon::spi_words)\",\n    \
                         );",
                        i = offset / 4,
                    );
                }
                vec![pf.cr1, spi.cfg1, pf.config]
            }
        };
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
        for n in plan.timers.keys().filter(|n| scope.timers.contains(n)) {
            let writes = timer_oracle(plan, *n, tf);
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
    emit_bridge_idle(w, plan, facts, scope);
    let _ = writeln!(w, "}}\n");

    // The scheduler.
    let usarts = if plan.serials.is_empty() {
        ""
    } else {
        "\n/// No step touches a USART, its pins or its clock gate: the final check holds\n\
         /// them at their starting values, the image holds no USART block, and any\n\
         /// access to one fails. Each port's bring-up is\n\
         /// `usartN_start_writes_the_solved_values`' (`AET-WO-4300b`: USART steps in\n\
         /// this schedule ran out of CBMC's memory)."
    };
    match part {
        None => {
            let _ = writeln!(
                w,
                "/// Every order the typestate API admits: at each of {actions} steps, any\n\
                 /// action whose inputs exist. The `assume` admits only those orders; the\n\
                 /// types forbid the rest at compile time. Each I2C peripheral writes zero or\n\
                 /// one byte when it starts; each SPI peripheral takes its bus's chip selects;\n\
                 /// and the output pins are driven zero or one time first.{usarts}"
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
                 /// of the whole board.{usarts}"
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
         let mut rcc = board.rcc;\n    \
         // The bring-up's first step, as `main` takes it (`AET-WO-4240v` §1.4): it\n    \
         // touches only PWR (`pwr_scale2_is_once_in_order_and_bounded`), and no\n    \
         // action touches PWR, so it commutes with every action.\n    \
         let scale2 = board.pwr.scale2(&mut img, 1);\n    \
         kani::assume(scale2.is_ok());\n    \
         let {bind} = scale2.unwrap();",
        bind = if !acts.is_empty() {
            "scale2"
        } else {
            "_scale2"
        },
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
    if acts.contains(&Action::ResetClock) {
        let _ = writeln!(w, "    let mut clock = None;");
        if plan.hse.is_some() {
            let _ = writeln!(w, "    let mut hse = Some(board.hse);");
        }
    }
    if acts.contains(&Action::SelectKernels) {
        let _ = writeln!(w, "    let mut kernels = None;");
    }
    for n in &scope.timers {
        let _ = writeln!(
            w,
            "    let mut tim{n} = Some(board.tim{n});\n    let mut tim{n}_clocked = None;\n    let mut tim{n}_done = false;"
        );
        // A bridge's timer keeps its `Running` token for the enable
        // (`AET-WO-1211b`).
        if plan.bridges.iter().any(|b| b.timer == Some(*n)) {
            let _ = writeln!(w, "    let mut tim{n}_running = None;");
        }
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
            "                let {bind} = rcc.enable_{f}(&mut img, {f}.take().unwrap(), scale2).split(&mut img);",
            bind = if pins.is_empty() { "_pins" } else { "pins" },
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
        let (ready, kernels) = match (plan.spi.contains_key(name), plan.hse.is_some()) {
            (true, false) => (" && kernels.is_some()", ", kernels.as_ref().unwrap()"),
            (true, true) => (" && clock.is_some()", ", clock.as_ref().unwrap()"),
            (false, _) => ("", ""),
        };
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some(){ready});"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                {f}_clocked = Some(rcc.enable_{f}(&mut img, {f}.take().unwrap(), scale2{kernels}));\n            \
             }}"
        );
        a += 1;
    }
    for (pin, p) in plan.pins.iter().filter(|(n, _)| scope.pins.contains(*n)) {
        debug_assert_eq!(acts[a], Action::Pin(pin.clone()));
        let f = pin_field(pin);
        // A bridge's enable waits for its inputs' idle, and takes them
        // (`AET-WO-1211b`).
        let (ready, args) = match plan.enable_after(pin) {
            Some(e) => {
                let mut ready = String::new();
                let mut args = String::new();
                if let Some(n) = e.timer {
                    let _ = write!(ready, " && tim{n}_running.is_some()");
                    let _ = write!(args, ", tim{n}_running.as_ref().unwrap()");
                }
                for g in &e.gpio_inputs {
                    let gf = pin_field(g);
                    let _ = write!(ready, " && {gf}_done.is_some()");
                    let _ = write!(args, ", {gf}_done.as_ref().unwrap()");
                }
                (ready, args)
            }
            None => (String::new(), String::new()),
        };
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume({f}.is_some(){ready});"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                let {mutable}pin = {f}.take().unwrap().{m}(&mut img{args});",
            m = method(p),
            mutable = if plan.is_select(pin) { "mut " } else { "" },
        );
        // A chip select is driven both ways before its bus takes it; a pin
        // device's output is left at its declared level, which the final
        // state holds (`AET-WO-1211b`).
        if plan.is_select(pin) {
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
                 kani::assume({f}_clocked.is_some() && clock.is_some() && {});",
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
                "{indent}let mut bus = i.init::<{ty}>(&mut img, clock.as_ref().unwrap(), {args});\n\
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
                let (hz, admitted) = &speeds[name];
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
                    // The topology's word, which `check_silicon` held to
                    // `aethelgard_ir::silicon::check_i2c`, for the mode the
                    // oracle takes ([`oracle_timingr`]); read here on its own.
                    match oracle_timingr(&plan.i2c[name], *hz, &modes, k) {
                        Some(word) => {
                            let _ = writeln!(w, "                    {m}_timingr = {word:#010x};");
                        }
                        None => {
                            let _ = writeln!(
                                w,
                                "                    kani::assert(false, \"aethelgard_ir::silicon::i2c_start_mode gives {name} no mode at {hz} Hz\");"
                            );
                        }
                    }
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
                    "                let _bus = i.init::<_>(&mut img, clock.as_ref().unwrap(), {args});\n                \
                     kani::cover!(true, \"{name} started\");",
                    args = args.join(", "),
                );
            }
        }
        let _ = writeln!(w, "                {f}_done = true;\n            }}");
        a += 1;
    }
    // The timers (`AET-WO-4300a`): the reset clock read once, each timer
    // clocked, then started from its own configured pins and the token; an
    // I2C or SPI peripheral's start takes the token too (`AET-WO-4230` D6).
    // The schedule runs on the reset clock; `reset_clock_is_ok_exactly_at_reset`
    // proves the check itself.
    if acts.contains(&Action::ResetClock) {
        debug_assert_eq!(acts[a], Action::ResetClock);
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume(clock.is_none());"
        );
        running(w, a);
        if plan.hse.is_some() {
            // The bring-up on the image, whose ready bits follow their ON
            // bits: it must reach `SolvedClock` at one read a wait.
            let _ = writeln!(
                w,
                "                let read = rcc.reset_clock(&mut img, &scale2);\n                \
                     kani::assume(read.is_ok());\n                \
                     let solved = hse.take().unwrap().start(&mut img, read.unwrap(), 1, 1, 1);\n                \
                     kani::assert(solved.is_ok(), \"the bring-up stopped though every ready bit followed its ON bit\");\n                \
                     clock = solved.ok();\n                \
                     kani::cover!(true, \"the clock was brought up\");\n            \
                 }}"
            );
        } else {
            let _ = writeln!(
                w,
                "                let read = rcc.reset_clock(&mut img, &scale2);\n                \
                     kani::assume(read.is_ok());\n                \
                     clock = read.ok();\n                \
                     kani::cover!(true, \"the reset clock was read\");\n            \
                 }}"
            );
        }
        a += 1;
    }
    if acts.contains(&Action::SelectKernels) {
        debug_assert_eq!(acts[a], Action::SelectKernels);
        let _ = writeln!(
            w,
            "            {a} => {{\n                \
                 kani::assume(clock.is_some() && kernels.is_none());"
        );
        running(w, a);
        let _ = writeln!(
            w,
            "                kernels = Some(rcc.select_spi_kernels(&mut img, clock.as_ref().unwrap()));\n                \
                 kani::cover!(true, \"the SPI kernel clocks were selected\");\n            \
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
            "                tim{n}_clocked = Some(rcc.enable_tim{n}(&mut img, tim{n}.take().unwrap(), scale2));\n            \
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
        let keep = if plan.bridges.iter().any(|b| b.timer == Some(*n)) {
            format!("tim{n}_running = Some(running);\n                ")
        } else {
            String::new()
        };
        let _ = writeln!(
            w,
            "                let {bind} = tim{n}_clocked\n                    \
                     .take()\n                    \
                     .unwrap()\n                    \
                     .start_outputs(&mut img, clock.as_ref().unwrap(){args});\n                \
                 {keep}kani::cover!(true, \"TIM{n} started\");\n                \
                 tim{n}_done = true;\n            \
             }}",
            args = args.concat(),
            bind = if keep.is_empty() {
                "_running"
            } else {
                "running"
            },
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
        if (p.role == Role::Output && !plan.is_select(pin)) || p.role == Role::Input {
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

/// Each bridge whose enable this scope holds (`AET-WO-1211b`; `1211` §2.5,
/// Q3; §2.7 (e) Kani): its idle as the final image leaves it is a
/// non-driving row of its part's truth table. Each input's level is read
/// from the image as silicon would hold it, a timer channel's in PWM mode 1
/// by its `CCR` against `ARR` (0 holds OCxREF at 0, above `ARR` at 1, RM0433
/// Rev 9 p. 1558; anything between is a pulse, 2, which no row holds), a
/// GPIO's and the enable's by `ODR`; the rows are the oracle's
/// ([`oracle_bridges`]), never the generator's judgement.
fn emit_bridge_idle(w: &mut String, plan: &Plan, facts: &Facts, scope: &Scope) {
    let g = &facts.gpio;
    let level = |pin: &str| {
        let p = &plan.pins[pin];
        format!(
            "((after.{}[{}] >> {}) & 1) as u8",
            port_field(p.port),
            g.odr / 4,
            facts.pins[pin].bs
        )
    };
    let bit = |l: Option<Level>| match l {
        Some(Level::Low) => "0",
        Some(Level::High) => "1",
        None => "_",
    };
    for b in plan
        .oracle_bridges
        .iter()
        .filter(|b| scope.pins.contains(&b.enable))
    {
        let mut inputs = Vec::new();
        for i in &b.inputs {
            match (i, &facts.tim) {
                (Ok((n, k)), Some(tf)) => inputs.push(format!(
                    "{{\n        let c = after.tim{n}[{c}];\n        if c == 0 {{\n            0u8\n        }} else if c > after.tim{n}[{a}] {{\n            1\n        }} else {{\n            2\n        }}\n    }}",
                    c = tf.ccr[(*k - 1) as usize] / 4,
                    a = tf.arr / 4,
                )),
                (Err(pin), _) => inputs.push(level(pin)),
                (Ok(_), None) => inputs.push("2u8".to_owned()),
            }
        }
        let rows: Vec<String> = b
            .safe
            .iter()
            .map(|(e, i0, i1)| format!("({}, {}, {})", bit(Some(*e)), bit(*i0), bit(*i1)))
            .collect();
        let rule = if rows.is_empty() {
            "false".to_owned()
        } else {
            format!("matches!((en, i0, i1), {})", rows.join(" | "))
        };
        let _ = writeln!(
            w,
            "    // {name}'s idle: a non-driving row of its part's truth table.\n    \
             let en = {en};\n    \
             let i0 = {i0};\n    \
             let i1 = {i1};\n    \
             kani::assert({rule}, \"{name} idles in a driving or unlisted state of its truth table\");",
            name = b.name,
            en = level(&b.enable),
            i0 = inputs.first().cloned().unwrap_or_else(|| "2u8".into()),
            i1 = inputs.get(1).cloned().unwrap_or_else(|| "2u8".into()),
        );
    }
}

/// The proofs of the motor outputs (`AET-WO-4300a` §1.3), when the topology
/// has a timer: that `reset_clock` is `Ok` exactly when every field it checks
/// holds its reset value, reading only its four registers and writing none;
/// and, for each timer, that its start writes exactly the solved registers and
/// values in order, reading none. Every expected value is the oracle's
/// ([`timer_oracle`], and the fields' positions and widths), never read from
/// `board.rs`.
fn emit_timer_proofs(w: &mut String, plan: &Plan, facts: &Facts) {
    let Some(clock) = &facts.clock else {
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
             let scale2 = scale2_token(board.pwr);\n    \
             let result = board.rcc.reset_clock(&mut spy, &scale2);\n    \
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
    for (n, t) in &plan.timers {
        let Some(tf) = &facts.tim else { break };
        let writes = timer_oracle(plan, *n, tf);
        let count = writes.len();
        let pins: Vec<&String> = t.channels.values().map(|(p, _)| p).collect();
        let setup = harness_setup(plan, &pins);
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
                 let clock = rcc.reset_clock(&mut img, &scale2);\n    \
                 kani::assume(clock.is_ok());\n    \
                 let clock = clock.unwrap();\n    \
                 let tim = rcc.enable_tim{n}(&mut img, board.tim{n}, scale2);\n    \
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
    if let Some(u) = &facts.usart {
        for (name, sp) in &plan.serials {
            emit_usart_proof(w, name, sp, plan, facts, u);
        }
    }
    // Each ADC's bring-up on its own spy (`AET-WO-1211b`), from its analog
    // pins, its clock and the reset clock's token.
    for (name, p) in &plan.adcs {
        let pins: Vec<&String> = p.inputs.iter().map(|i| &i.pin).collect();
        let f = name.to_ascii_lowercase();
        let setup = format!(
            "    let mut img = Image {{\n{}    \
             let clock = rcc.reset_clock(&mut img, &scale2);\n    \
             kani::assume(clock.is_ok());\n    \
             let clock = clock.unwrap();\n    \
             let {f} = rcc.enable_{f}(&mut img, board.{f}, scale2);\n",
            harness_setup(plan, &pins)
        );
        // The oracle's own reading. An ADC it does not see is proved against
        // no input, so its proof fails: it fails closed.
        let none = adc::OracleAdc {
            channels: Vec::new(),
            bits: 0,
        };
        let oracle = plan.oracle_adcs.get(name).unwrap_or(&none);
        adc::emit_proof(w, name, p, oracle, &facts.adcs[name], &setup);
    }
}

/// A targeted harness's image and setup, after `let mut img = Image {`: every
/// block any, the board taken, and each of `pins`' ports clocked and the pin
/// put in its role.
fn harness_setup(plan: &Plan, pins: &[&String]) -> String {
    let ports: BTreeSet<char> = pins.iter().map(|p| plan.pins[*p].port).collect();
    let mut setup = String::new();
    for b in image_fields(plan) {
        let _ = writeln!(setup, "        {b}: kani::any(),");
    }
    let _ = writeln!(
        setup,
        "    }};\n    \
         let board = Board::take().unwrap();\n    \
         let mut rcc = board.rcc;\n    \
         let scale2 = scale2_token(board.pwr);"
    );
    for port in &ports {
        let f = port_field(*port);
        let _ = writeln!(
            setup,
            "    let {f} = rcc.enable_{f}(&mut img, board.{f}, scale2).split(&mut img);"
        );
    }
    for pin in pins {
        let _ = writeln!(
            setup,
            "    let {f} = {port}.{f}.{method}(&mut img);",
            f = pin_field(pin),
            port = port_field(plan.pins[*pin].port),
            method = method(&plan.pins[*pin])
        );
    }
    setup
}

/// One port's proof (`AET-WO-4300b` §1.4): its whole bring-up from the board
/// as `Board::take` hands it over, on any register contents. RCC and its pins'
/// ports go to the harness image, under its clock-gate and write rules; every
/// access to the USART is recorded, and any other block fails. Its pins' ports
/// clocked, both pins put in their roles, the reset clock read, the USART
/// clocked and configured: RCC is then before plus exactly those gates; each
/// pin's fields hold their solved values and every other field of its port
/// its old one; and the USART was written exactly the oracle's four writes, in
/// order, each after its gate was set, and never read. Every expected value is
/// [`usart_oracle`]'s or the generator's pin policy, never read from
/// `board.rs`.
fn emit_usart_proof(
    w: &mut String,
    name: &str,
    sp: &SerialPlan,
    plan: &Plan,
    facts: &Facts,
    u: &UsartFacts,
) {
    let t = type_name(name);
    let f = name.to_ascii_lowercase();
    let g = &facts.gpio;
    let writes = usart_oracle(sp, u);
    let count = writes.len();
    let gate = &facts.serials[name].gate;
    let pins = [&sp.tx, &sp.rx];
    let ports: BTreeSet<char> = pins.iter().map(|p| plan.pins[*p].port).collect();
    let owned: Vec<String> = std::iter::once("Block::Rcc".to_owned())
        .chain(ports.iter().map(|p| format!("Block::{}", port_type(*p))))
        .collect();
    let owned = owned
        .iter()
        .map(|b| format!("block == {b}"))
        .collect::<Vec<_>>()
        .join(" || ");
    let mut body = String::new();
    for b in image_fields(plan) {
        let _ = writeln!(body, "        {b}: kani::any(),");
    }
    let _ = writeln!(
        body,
        "    }};\n    \
         let before = img;\n    \
         let mut spy = {t}Bringup {{ img, writes: [(0, 0); {count}], n: 0 }};\n    \
         let board = Board::take().unwrap();\n    \
         let mut rcc = board.rcc;\n    \
         let scale2 = scale2_token(board.pwr);"
    );
    for port in &ports {
        let pf = port_field(*port);
        let _ = writeln!(
            body,
            "    let {pf} = rcc.enable_{pf}(&mut spy, board.{pf}, scale2).split(&mut spy);"
        );
    }
    for pin in pins {
        let _ = writeln!(
            body,
            "    let {f} = {port}.{f}.{method}(&mut spy);",
            f = pin_field(pin),
            port = port_field(plan.pins[pin].port),
            method = method(&plan.pins[pin])
        );
    }
    let _ = writeln!(
        body,
        "    let clock = rcc.reset_clock(&mut spy, &scale2);\n    \
         kani::assume(clock.is_ok());\n    \
         let clock = clock.unwrap();\n    \
         let usart = rcc.enable_{f}(&mut spy, board.{f}, scale2);\n    \
         let _port = usart.configure(&mut spy, &clock, {tx}, {rx});\n    \
         kani::assert(spy.n == {count}, \"{name}'s bring-up did not write it exactly {count} times\");",
        tx = pin_field(&sp.tx),
        rx = pin_field(&sp.rx),
    );
    for (i, (register, offset, value)) in writes.iter().enumerate() {
        let _ = writeln!(
            body,
            "    kani::assert(spy.writes[{i}] == ({offset:#04x}, {value:#010x}), \"{name} write {k} is not {register} = {value:#010x}\");",
            k = i + 1,
        );
    }
    // RCC: before plus exactly the ports' gates and the USART's.
    let mut gates: BTreeMap<u32, u32> = BTreeMap::new();
    for g in ports.iter().map(|p| &facts.ports[p].gate).chain([gate]) {
        *gates.entry(g.offset / 4).or_default() |= 1u32 << g.bit;
    }
    let _ = writeln!(body, "    let mut rcc_after = before.rcc;");
    for (word, mask) in &gates {
        let _ = writeln!(body, "    rcc_after[{word}] |= {mask:#010x};");
    }
    let _ = writeln!(
        body,
        "    for i in 0..{n} {{\n        \
             kani::assert(spy.img.rcc[i] == rcc_after[i], \"RCC is not before plus exactly {name}'s gate and its pins' ports'\");\n    \
         }}",
        n = facts.rcc_words
    );
    // Each pin's port: its whole `OTYPER` and `PUPDR` the solved words, the pin's other
    // fields solved, every other field unchanged.
    let [mw, _, sw, _, aw] = g.widths;
    for port in &ports {
        let field = port_field(*port);
        let mut expect: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        for pin in pins.iter().filter(|p| plan.pins[**p].port == *port) {
            let p = &plan.pins[*pin];
            let pf = &facts.pins[*pin];
            let (mode, _, ospeed, _) = p.policy();
            expect_field(&mut expect, g.moder, pf.moder, mw, mode);
            expect_field(&mut expect, g.ospeedr, pf.ospeedr, sw, ospeed);
            expect_field(&mut expect, g.afr[pf.afr_index as usize], pf.afr, aw, p.af);
        }
        for i in 0..g.words {
            if i * 4 == g.otyper {
                let _ = writeln!(
                    body,
                    "    kani::assert(spy.img.{field}[{i}] == {word:#010x}, \"{t}: {field} OTYPER is not the solved word\");",
                    word = oracle_otyper(plan, *port),
                );
                continue;
            }
            if i * 4 == g.pupdr {
                let _ = writeln!(
                    body,
                    "    kani::assert(spy.img.{field}[{i}] == {word:#010x}, \"{t}: {field} PUPDR is not the solved word\");",
                    word = oracle_pupdr(plan, *port),
                );
                continue;
            }
            match expect.get(&(i * 4)) {
                Some((mask, value)) => {
                    let _ = writeln!(
                        body,
                        "    kani::assert(spy.img.{field}[{i}] & {mask:#010x} == {value:#010x}, \"{t}: a pin's field in {field} word {i} is not its solved value\");\n    \
                         kani::assert(spy.img.{field}[{i}] & !{mask:#010x} == before.{field}[{i}] & !{mask:#010x}, \"{t}'s bring-up changed another pin's field in {field} word {i}\");"
                    );
                }
                None => {
                    let _ = writeln!(
                        body,
                        "    kani::assert(spy.img.{field}[{i}] == before.{field}[{i}], \"{t}'s bring-up changed {field} word {i}\");"
                    );
                }
            }
        }
    }
    let _ = writeln!(
        w,
        "\n\
         /// {name}'s bring-up as the hardware sees it: RCC and its pins' ports go to\n\
         /// the harness image, under its gate and write rules; each {name} write is\n\
         /// recorded, after its gate is set; any other access fails.\n\
         struct {t}Bringup {{\n    \
             img: Image,\n    \
             writes: [(usize, u32); {count}],\n    \
             n: usize,\n\
         }}\n\
         \n\
         impl sealed::Sealed for {t}Bringup {{}}\n\
         \n\
         impl Registers for {t}Bringup {{\n    \
             fn read(&mut self, token: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 kani::assert({owned}, \"{name}'s bring-up read a block other than RCC and its pins' ports\");\n        \
                 self.img.read(token, block, offset)\n    \
             }}\n    \
             fn write(&mut self, token: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 if block == Block::{t} {{\n            \
                     kani::assert(self.img.rcc[{word}] & {mask:#010x} != 0, \"{name} written before RCC_{register} bit {bit} is set\");\n            \
                     kani::assert(self.n < {count}, \"{name}'s bring-up wrote it more than {count} times\");\n            \
                     if self.n < {count} {{\n                \
                         self.writes[self.n] = (offset, value);\n            \
                     }}\n            \
                     self.n += 1;\n        \
                 }} else {{\n            \
                     kani::assert({owned}, \"{name}'s bring-up wrote a block other than RCC, its pins' ports and {name}\");\n            \
                     self.img.write(token, block, offset, value);\n        \
                 }}\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"{name}'s bring-up made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"{name}'s bring-up made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// Serial port `{label}`, brought up from the board as `Board::take` hands it\n\
         /// over, on any register contents: its pins' ports clocked, both pins in\n\
         /// their roles, the reset clock read, {name} clocked and configured. RCC is\n\
         /// then before plus exactly those gates; the pins' fields hold their solved\n\
         /// values and no other field of their ports changed; and {name} was written\n\
         /// exactly the solved `BRR`, `CR2`, `CR1`, then `CR1` with `UE`, in order, each\n\
         /// after its gate was set, and never read.\n\
         #[kani::proof]\n\
         fn {f}_start_writes_the_solved_values() {{\n    \
             let img = Image {{\n\
         {body}    \
             kani::cover!(true, \"{name} brought up\");\n\
         }}",
        label = sp.label_text(),
        word = gate.offset / 4,
        mask = 1u32 << gate.bit,
        register = gate.register,
        bit = gate.bit,
    );
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
        let spe = bit(pf.enable);
        // The words each `CR1` write must be: the independent rule's.
        let words = oracle_spi(plan, name);
        // The erratum's delay, by the oracle's own arithmetic: one SCK period,
        // `2^(MBR+1)` kernel cycles at the slowest kernel, in core cycles at
        // the fastest core, each `SR` read at least one (ES0392 Rev 16 §2.22.6).
        let c = &plan.spi[name];
        // On an HSE board the core and the kernel are the tree's budget's
        // (`aethelgard_ir::clock`, `silicon::spi_kernel_budget`; `AET-WO-4240b`).
        let delay = (u128::from(1u64 << (c.mbr + 1))
            * u128::from(plan.ir_clock.budget().fastest_hz(plan.core_hz)))
        .div_ceil(u128::from(
            silicon::spi_kernel_budget(c.kernel, &plan.ir_clock)
                .slowest_hz(c.kernel_hz)
                .max(1),
        ));
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
                 /// `SR.EOT` read set once every frame was received.\n    \
                 eot: bool,\n    \
                 /// Reads of `SR` after `EOT` was read set: the erratum's delay.\n    \
                 after_eot: u32,\n    \
                 /// `CFG2` as `init` left it in the harness image.\n    \
                 cfg2: u32,\n    \
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
                         kani::assert(false, \"a {t} transfer read CR1: it writes whole words (AET-WO-4230 D5)\");\n            \
                         self.cr1\n        \
                     }} else if offset == {sr:#04x} {{\n            \
                         kani::assert(self.stage == 4, \"SR polled outside the transfer\");\n            \
                         self.sr_reads += 1;\n            \
                         if self.eot {{\n                \
                             self.after_eot += 1;\n            \
                         }}\n            \
                         let v: u32 = kani::any();\n            \
                         self.rxp = v & {rxp} != 0;\n            \
                         if v & {eot} != 0 && self.received as u32 == {frames_set} {{\n                \
                             self.eot = true;\n            \
                         }}\n            \
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
                         // Each `CR1` write is the rule's whole word for its stage\n            \
                         // (`aethelgard_ir::silicon::spi_words`), and none sets `SPE`\n            \
                         // before master mode with software slave select (the 3302\n            \
                         // bench: with `SSM` 0 the first `SPE` raised `MODF`).\n            \
                         if value & {spe} != 0 {{\n                \
                             kani::assert(self.cfg2 & {master_ssm:#010x} == {master_ssm:#010x}, \"SPE set before CFG2 holds MASTER and SSM\");\n                \
                             kani::assert(value & {idle_word:#010x} != 0, \"SPE set without SSI\");\n            \
                         }}\n            \
                         if self.stage == 2 {{\n                \
                             kani::assert(value == {enable_word:#010x}, \"the peripheral was not enabled with SSI, alone\");\n                \
                             self.stage = 3;\n            \
                         }} else if self.stage == 3 {{\n                \
                             kani::assert(value == {start_word:#010x}, \"the transfer was not started with SSI and SPE, alone\");\n                \
                             self.stage = 4;\n            \
                         }} else if self.stage == 5 {{\n                \
                             kani::assert(value == {idle_word:#010x}, \"the peripheral was not left idle: SSI, alone\");\n                \
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
            enable_word = words.cr1_enable,
            start_word = words.cr1_start,
            idle_word = words.cr1_idle,
            master_ssm = words.cfg2,
            sr = s.sr,
            rxp = bit(s.rxp),
            eot = bit(s.eot),
            bsrr = facts.gpio.bsrr,
            cr2 = s.cr2,
            ifcr = s.ifcr,
            eotc = bit(s.eotc),
            rxdr = s.rxdr,
            txdr = pf.txdr,
            frames_set = shifted("self.tsize", ">>", s.tsize),
        );
        // The setup: the bus's ports, its pins and chip selects, `init`.
        let blocks = image_fields(plan);
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
             let mut rcc = board.rcc;\n    \
             let scale2 = scale2_token(board.pwr);\n    \
             let clock = rcc.reset_clock(&mut img, &scale2);\n    \
             kani::assume(clock.is_ok());\n    \
             let clock = clock.unwrap();"
        );
        for port in &ports {
            let f = port_field(*port);
            let _ = writeln!(
                setup,
                "    let {f} = rcc.enable_{f}(&mut img, board.{f}, scale2).split(&mut img);"
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
            "    let kernels = rcc.select_spi_kernels(&mut img, &clock);\n    \
             let mut bus = rcc\n        \
                 .enable_{m}(&mut img, board.{m}, scale2, &kernels)\n        \
                 .init(&mut img, &clock, {args}, {t}Selects {{ {fields} }});\n    \
             let devices = [{variants}];\n    \
             let pick: usize = kani::any();\n    \
             kani::assume(pick < devices.len());\n    \
             let device = devices[pick];\n    \
             // The state `init` and every transfer leave: `CR1` idle, `SSI` alone\n    \
             // (the schedule harness or each of its parts, and the end of this proof);\n    \
             // `CFG2` as `init` wrote it.\n    \
             let cr1: u32 = {idle:#010x};\n    \
             let cfg2 = img.{m}[{cfg2_word}];",
            args = args.join(", "),
            fields = fields.join(", "),
            variants = variants.join(", "),
            idle = words.cr1_idle,
            cfg2_word = pf.config / 4,
        );
        let spy = format!(
            "{t}Spy {{ device, out, wl, stage: 0, cr1, tsize: 0, sr_reads: 0, rxp: false, eot: false, after_eot: 0, cfg2, sent: 0, received: 0, touched: false }}"
        );
        let rest = "and any answers, with each wait bounded at two reads\n\
             /// of `SR`: selects that device before its first frame and deselects it after\n\
             /// the peripheral is disabled, also after a wait that expired; reads `RXDR`\n\
             /// only after `RXP`; writes `CR1` only whole and never reads it; and returns\n\
             /// `Ok` only when every frame moved, `EOT` was read set after the last, before\n\
             /// `EOTC`, and the erratum's delay followed it (ES0392 Rev 16 §2.22.6).\n";
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
        let frames_most = wmax + rmax;
        let most = match frames_most {
            5 => "five".to_owned(),
            n => n.to_string(),
        };
        // The delay's loop of rounds is the longest when it outruns the
        // frames. Its count is the code's: an unwind too small fails the
        // proof, never passes it.
        let unwind = u128::try_from(frames_most)
            .unwrap_or(u128::MAX)
            .max(u128::from(eot_delay_reads(plan, c).div_ceil(u64::from(SR_ROUND))) + 1);
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
                     spy.sr_reads as usize <= (frames + 1) * spins as usize + {delay_most},\n        \
                     \"a wait read SR more often than its bound\",\n    \
                 );\n    \
                 kani::assert(spy.cr1 == {idle:#010x}, \"a transfer did not leave CR1 idle\");\n    \
                 match result {{\n        \
                     Ok(n) => {{\n            \
                         kani::assert(n == rl && spy.sent == frames && spy.received == frames, \"Ok without every frame moved\");\n            \
                         kani::assert(spy.eot, \"Ok without EOT read after the last frame, before EOTC\");\n            \
                         kani::assert(spy.after_eot >= {delay}, \"SPE cleared sooner than one SCK period after EOT (ES0392 Rev 16 2.22.6)\");\n            \
                         kani::cover!(wl > 0 && rl > 0, \"{t} wrote, then read\");\n            \
                         kani::cover!(wl == 1 && rl == 3, \"{t} made an identity read's frames\");\n{covers}        \
                     }}\n        \
                     Err(SpiError::Rxp) => {{\n            \
                         kani::assert(spy.received < frames, \"Rxp after every frame moved\");\n            \
                         kani::cover!(true, \"{t} timed out waiting for RXP\");\n        \
                     }}\n        \
                     Err(SpiError::Eot) => {{\n            \
                         kani::assert(spy.received == frames && !spy.eot, \"Eot with a frame missing, or after EOT was read\");\n            \
                         kani::cover!(true, \"{t} timed out waiting for EOT\");\n        \
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
            idle = words.cr1_idle,
            // The delay in whole rounds: the most reads it may make.
            delay_most = delay.div_ceil(u128::from(SR_ROUND)) * u128::from(SR_ROUND),
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
        let blocks = image_fields(plan);
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
             let mut rcc = board.rcc;\n    \
             let scale2 = scale2_token(board.pwr);\n    \
             let clock = rcc.reset_clock(&mut img, &scale2);\n    \
             kani::assume(clock.is_ok());\n    \
             let clock = clock.unwrap();"
        );
        for port in &ports {
            let f = port_field(*port);
            let _ = writeln!(
                w,
                "    let {f} = rcc.enable_{f}(&mut img, board.{f}, scale2).split(&mut img);"
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
                 .enable_{m}(&mut img, board.{m}, scale2)\n        \
                 .init::<{m}_speed::{mode}, _>(&mut img, &clock, {args});\n    \
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
            gpio: BTreeMap::from([(
                "CS".into(),
                aethelgard_ir::pin_devices::SolvedGpio::output(
                    "PA0",
                    aethelgard_ir::motor::Level::High,
                ),
            )]),
            i2c_ceilings: BTreeMap::from([("BUS".into(), 400_000)]),
            i2c_speeds: BTreeMap::from([("BUS".into(), 400_000)]),
            analog_lines: analog_lines(&[("SCL", "BUS"), ("SDA", "BUS")]),
            i2c_devices: BTreeMap::new(),
            spi_devices: BTreeMap::new(),
            spi_lines: BTreeMap::new(),
            clock: aethelgard_ir::SolvedClock::reset(),
            motor_outputs: BTreeMap::new(),
            power: None,
            serial_ports: BTreeMap::new(),
            spi_controllers: BTreeMap::new(),
            // I2C1 as `solve` gives it on the reset clock (`AET-WO-4230`).
            i2c_timings: BTreeMap::from([(
                "I2C1".into(),
                aethelgard_ir::SolvedI2cTiming {
                    kernel_hz: 64_000_000,
                    cr1: 0,
                    timingr: BTreeMap::from([
                        ("Fast400k".into(), 0x10C7_123C),
                        ("FastPlus1M".into(), 0x00B2_0E2D),
                        ("Standard100k".into(), 0x50D3_2A40),
                    ]),
                },
            )]),
            bor_level: 0,
            core_supply: aethelgard_ir::vos::CoreSupplyMode::Ldo,
            mcu: "STM32H753ZITx".into(),
            pin_speeds: BTreeMap::from([("CS".into(), 0), ("SCL".into(), 0), ("SDA".into(), 0)]),
            adc_inputs: BTreeMap::new(),
            pin_devices: BTreeMap::new(),
            foreign_clock: None,
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
            for (mode, _) in I2C_MODES {
                assert_eq!(
                    proofs.contains(&format!("i2c1_speed::{mode},")),
                    modes.contains(&mode)
                );
            }
        }
    }

    /// Each `init` the schedule harness makes on I2C1 at a mode, with the
    /// `TIMINGR` the oracle expects set just before it, in order.
    fn started(proofs: &str) -> Vec<(u32, String)> {
        let lines: Vec<&str> = proofs.lines().map(str::trim).collect();
        lines
            .windows(2)
            .filter_map(|w| {
                let word = w[0].strip_prefix("i2c1_timingr = 0x")?.strip_suffix(';')?;
                let mode = w[1].split("i2c1_speed::").nth(1)?.split(',').next()?;
                Some((u32::from_str_radix(word, 16).ok()?, mode.to_owned()))
            })
            .collect()
    }

    /// `AET-WO-4230` R3-fix (the adversarial pass's F3, mutant M3): where the
    /// schedule starts a bus at the mode the firmware starts it at (the
    /// generator's highest admitted), the oracle expects the topology's
    /// `TIMINGR` for the mode the independent rule gives the declared speed
    /// (`silicon::i2c_start_mode`), not the admitted mode's. A generator that
    /// admitted Fast-mode Plus on a 400 kHz bus starts it there, and the
    /// oracle still expects Fast400k's word, which that `init` does not
    /// write: the proof fails.
    #[test]
    fn the_oracle_expects_the_rule_s_start_mode_s_word_where_a_bus_starts() {
        let mut s = solved();
        set_speed(&mut s, "BUS", 400_000);
        let table = table();
        let plan = Plan::new("b", &s, &table).unwrap();
        let rf = cmsis::register_facts().unwrap();
        let facts = Facts::new(&plan, &rf).unwrap();
        let words = |admitted: Vec<usize>| {
            let speeds: Speeds = BTreeMap::from([("I2C1".to_owned(), (400_000, admitted))]);
            started(&emit_proofs(&plan, &facts, &speeds).unwrap())
        };
        let (sm, fm) = (0x50D3_2A40, 0x10C7_123C);
        let pairs = |p: &[(u32, &str)]| -> Vec<(u32, String)> {
            p.iter().map(|(w, m)| (*w, (*m).to_owned())).collect()
        };
        assert_eq!(
            words(vec![0, 1]),
            pairs(&[(sm, "Standard100k"), (fm, "Fast400k")])
        );
        // M3's admission: Fm+ at the top, Fast400k's word expected there.
        assert_eq!(
            words(vec![0, 1, 2]),
            pairs(&[(sm, "Standard100k"), (fm, "Fast400k"), (fm, "FastPlus1M")])
        );
        // A generator that started the bus below the rule's mode.
        assert_eq!(words(vec![0]), pairs(&[(fm, "Standard100k")]));
        // A declared speed the rule gives no mode: the start fails the proof.
        let speeds: Speeds = BTreeMap::from([("I2C1".to_owned(), (50_000, vec![0]))]);
        let proofs = emit_proofs(&plan, &facts, &speeds).unwrap();
        assert!(proofs.contains(
            "kani::assert(false, \"aethelgard_ir::silicon::i2c_start_mode gives I2C1 no mode at 50000 Hz\");"
        ));
        assert!(started(&proofs).is_empty());
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
        for (mode, _) in I2C_MODES {
            assert_eq!(
                proofs.contains(&format!("i2c1_speed::{mode},")),
                mode == "Standard100k",
                "{mode}"
            );
        }
        assert!(files["src/board.rs"].contains("declared speed of 100000 Hz"));
    }

    #[test]
    fn ceilings_must_number_the_i2c_peripherals_and_each_takes_its_own_bus() {
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
        // Two buses (`AET-WO-3402` §1.3, ruled on PR #203): each peripheral
        // takes its own bus's declared speed, never the lower one.
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
        s.derive_controllers();
        let files = generate("b", &s, &table()).unwrap();
        let board = &files["src/board.rs"];
        let module = |m: &str| {
            let module = board.split(&format!("pub mod {m} {{")).nth(1).unwrap();
            module.split("\n}\n").next().unwrap().to_owned()
        };
        // I2C1 serves bus A at 1 MHz: every mode; I2C2 serves B at 100 kHz.
        let (one, two) = (module("i2c1_speed"), module("i2c2_speed"));
        for mode in ["Standard100k", "Fast400k", "FastPlus1M"] {
            assert!(
                one.contains(&format!("impl AtMostCeiling for {mode} ")),
                "{mode}"
            );
        }
        assert!(two.contains("impl AtMostCeiling for Standard100k "));
        assert!(!two.contains("impl AtMostCeiling for Fast400k "));
        assert!(files["firmware/src/main.rs"].contains("init::<i2c1_speed::FastPlus1M, _>"));
        assert!(files["firmware/src/main.rs"].contains("init::<i2c2_speed::Standard100k, _>"));
    }

    /// `AET-WO-3402` §5: a peripheral carrying two buses' lines, and a bus
    /// whose lines sit on two peripherals, are refused by name; nothing falls
    /// back to the lowest speed (`INV-06`).
    #[test]
    fn an_i2c_peripheral_with_no_one_bus_is_refused_by_name() {
        let two_buses = || {
            let mut s = solved();
            s.assignments
                .insert("SCL2".into(), assign("PB10", "I2C2", "I2C2_SCL", 4));
            s.assignments
                .insert("SDA2".into(), assign("PB11", "I2C2", "I2C2_SDA", 4));
            s.i2c_ceilings = BTreeMap::new();
            s.i2c_speeds = BTreeMap::new();
            s
        };
        // Each peripheral carries one line of each bus.
        let mut s = two_buses();
        s.analog_lines = analog_lines(&[("SCL", "A"), ("SDA", "B"), ("SCL2", "B"), ("SDA2", "A")]);
        set_ceiling(&mut s, "A", 400_000);
        set_ceiling(&mut s, "B", 100_000);
        s.derive_controllers();
        // `SolvedTopology::validate` refuses it first (measured): every
        // analog line shares one bus with exactly its own peripheral's lines.
        let err = generate("b", &s, &table()).unwrap_err();
        assert_eq!(err.kind(), "InvalidTopology", "{err}");
        assert!(err.to_string().contains("does not share one bus"), "{err}");
        // `bus_speed` refuses it on its own, by name, as the generator's
        // second line: two buses on I2C1, and a bus with a line elsewhere.
        let e = bus_speed(&s, "I2C1").unwrap_err();
        assert_eq!(e.kind(), "I2cBusUnresolved", "{e}");
        assert!(e.to_string().contains("carries the lines of buses"), "{e}");
        let mut s = two_buses();
        s.analog_lines = analog_lines(&[("SCL", "A"), ("SDA", "A"), ("SCL2", "A"), ("SDA2", "B")]);
        let e = bus_speed(&s, "I2C1").unwrap_err();
        assert!(
            e.to_string().contains("bus `A`'s line `SCL2` is on `I2C2`"),
            "{e}"
        );
        // No line on a peripheral: refused, never the lowest speed.
        let e = bus_speed(&s, "I2C3").unwrap_err();
        assert!(e.to_string().contains("no I2C line"), "{e}");
        // A bus with no declared speed.
        let mut s = two_buses();
        s.analog_lines = analog_lines(&[("SCL", "A"), ("SDA", "A"), ("SCL2", "B"), ("SDA2", "B")]);
        let e = bus_speed(&s, "I2C1").unwrap_err();
        assert!(e.to_string().contains("bus `A` declares no speed"), "{e}");
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
        // A USART line no solved serial port names (`AET-WO-4300b`): the
        // topology's own serial check refuses it, naming the net, before the
        // generator looks.
        assert!(matches!(
            generate("b", &s, &t),
            Err(BoardError::InvalidTopology(
                IrValidationError::SerialPortMismatch { id }
            )) if id == "TX"
        ));

        let mut s = solved();
        s.assignments
            .insert("SMBA".into(), assign("PB5", "I2C1", "I2C1_SMBA", 4));
        s.pin_speeds.insert("SMBA".into(), 0);
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
        s.gpio.insert(
            "CS2".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PB6",
                aethelgard_ir::motor::Level::High,
            ),
        );
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.gpio.insert(
            "SWDIO".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PA13",
                aethelgard_ir::motor::Level::High,
            ),
        );
        s.pin_speeds.insert("SWDIO".into(), 0);
        assert_eq!(kind(generate("b", &s, &t)), "ReservedPin");

        let mut s = solved();
        s.specification_sha256 = "x".into();
        assert_eq!(kind(generate("b", &s, &t)), "InvalidTopology");

        let mut s = solved();
        s.gpio.insert(
            "CS2".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PB16",
                aethelgard_ir::motor::Level::High,
            ),
        );
        s.pin_speeds.insert("CS2".into(), 0);
        assert_eq!(kind(generate("b", &s, &t)), "InvalidPin");

        let mut s = solved();
        s.gpio.insert(
            "CS2".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PK15",
                aethelgard_ir::motor::Level::High,
            ),
        );
        s.pin_speeds.insert("CS2".into(), 0);
        assert_eq!(kind(generate("b", &s, &t)), "UnknownPin");

        for name in ["", "Board", "b_1", "-b", "b-", "b--c", "1b"] {
            assert_eq!(
                kind(generate(name, &solved(), &t)),
                "InvalidCrateName",
                "{name}"
            );
        }
    }

    /// `AET-WO-4230` D5: each port's `OTYPER` is written once, whole, by
    /// `split`, and never read-modify-written by a pin; the oracle's word is
    /// the hand-computed one (PB6 and PB7 open-drain: `0xC0`; PA0 push-pull:
    /// 0), and the harness holds the final word to it.
    #[test]
    fn otyper_is_written_whole_by_split_and_the_oracle_is_the_hand_word() {
        let s = solved();
        let plan = Plan::new("b", &s, &table()).unwrap();
        assert_eq!(oracle_otyper(&plan, 'B'), 0xC0);
        assert_eq!(oracle_otyper(&plan, 'A'), 0);
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        assert!(
            lib.contains(
                "    pub fn split<R: Registers>(self, r: &mut R) -> GpioBPins {\n        \
                 r.write(TOKEN, Block::GpioB, 0x04, 0x000000c0);\n"
            ),
            "{lib}"
        );
        assert!(lib.contains("r.write(TOKEN, Block::GpioA, 0x04, 0x00000000);\n"));
        for port in ["GpioA", "GpioB"] {
            assert_eq!(
                lib.matches(&format!("Block::{port}, 0x04")).count(),
                1,
                "{port}"
            );
        }
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains(
            "kani::assert(after.gpiob[1] == 0x000000c0, \"GpioB OTYPER is not the solved word\");"
        ));
        assert!(proofs.contains(
            "kani::assert(value == 0x000000c0, \"GpioB OTYPER is not the solved word, whole\");"
        ));
        assert!(proofs.contains("\"GpioB OTYPER was read\""));
    }

    /// `AET-WO-4230` R5: each port's `PUPDR` is written once, whole, by
    /// `split`, beside `OTYPER`, and never read-modify-written by a pin; the
    /// word keeps the debug port's reset pulls (RM0433 Rev 9 §11.4.4, p. 542:
    /// port A `0x6400_0000`, port B `0x0000_0100`) and gives no solved pin of
    /// this board a pull. The harness holds the final word to the oracle's.
    #[test]
    fn pupdr_is_written_whole_by_split_and_keeps_the_debug_pulls() {
        let s = solved();
        let plan = Plan::new("b", &s, &table()).unwrap();
        assert_eq!(oracle_pupdr(&plan, 'A'), 0x6400_0000);
        assert_eq!(oracle_pupdr(&plan, 'B'), 0x0000_0100);
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        assert!(
            lib.contains(
                "    pub fn split<R: Registers>(self, r: &mut R) -> GpioBPins {\n        \
                 r.write(TOKEN, Block::GpioB, 0x04, 0x000000c0);\n        \
                 r.write(TOKEN, Block::GpioB, 0x0c, 0x00000100);\n"
            ),
            "{lib}"
        );
        assert!(lib.contains("r.write(TOKEN, Block::GpioA, 0x0c, 0x64000000);\n"));
        for port in ["GpioA", "GpioB"] {
            assert_eq!(
                lib.matches(&format!("Block::{port}, 0x0c")).count(),
                1,
                "{port}"
            );
        }
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains(
            "kani::assert(after.gpiob[3] == 0x00000100, \"GpioB PUPDR is not the solved word\");"
        ));
        assert!(proofs.contains(
            "kani::assert(value == 0x64000000, \"GpioA PUPDR is not the solved word, whole\");"
        ));
        assert!(proofs.contains("\"GpioA PUPDR was read\""));
    }

    /// `AET-WO-4230` D6: every board that reads the reset clock checks
    /// `CKPERSEL`, both SPI selections and `I2C123SEL`; the domain 3 fields
    /// only with the peripheral that depends on them.
    #[test]
    fn the_reset_clock_reads_the_domain_3_fields_only_for_their_peripheral() {
        let fields = |s: &SolvedTopology| {
            let plan = Plan::new("b", s, &table()).unwrap();
            let f = cmsis::register_facts().unwrap();
            clock_facts(&plan, &f, &f.layout("RCC").unwrap())
                .unwrap()
                .into_iter()
                .map(|(r, offset, fields)| {
                    let names: Vec<&str> = fields.iter().map(|(n, _, _)| *n).collect();
                    (r, offset, names.join(" "))
                })
                .collect::<Vec<_>>()
        };
        let base = [
            ("CR", 0x00, "HSIDIV"),
            ("CFGR", 0x10, "SWS TIMPRE"),
            ("D1CFGR", 0x18, "D1CPRE D1PPRE HPRE"),
            ("D2CFGR", 0x1C, "D2PPRE1 D2PPRE2"),
            ("D1CCIPR", 0x4C, "CKPERSEL"),
            ("D2CCIP1R", 0x50, "SPI123SEL SPI45SEL"),
            ("D2CCIP2R", 0x54, "USART16SEL USART28SEL I2C123SEL"),
        ];
        let want = |extra: &[(&'static str, u32, &str)]| {
            let mut v: Vec<(&'static str, u32, String)> = base
                .iter()
                .map(|(r, o, f)| (*r, *o, (*f).to_owned()))
                .collect();
            v.extend(extra.iter().map(|(r, o, f)| (*r, *o, (*f).to_owned())));
            v.sort_by_key(|(_, o, _)| *o);
            v
        };
        assert_eq!(fields(&solved()), want(&[]));
        // I2C4 on PB6 and PB7 (AF6): its kernel clock is `rcc_pclk4`, behind
        // `D3PPRE`, selected by `I2C4SEL`.
        let mut s = solved();
        s.assignments
            .insert("SCL".into(), assign("PB6", "I2C4", "I2C4_SCL", 6));
        s.assignments
            .insert("SDA".into(), assign("PB7", "I2C4", "I2C4_SDA", 6));
        s.derive_controllers();
        assert_eq!(
            fields(&s),
            want(&[("D3CFGR", 0x20, "D3PPRE"), ("D3CCIPR", 0x58, "I2C4SEL")])
        );
        // SPI6 on PA5 to PA7 (AF8): `SPI6SEL`, and no prescaler of domain 3.
        let mut s = with_spi(&[]);
        for (net, pin, signal) in [
            ("SCK", "PA5", "SCK"),
            ("MISO", "PA6", "MISO"),
            ("MOSI", "PA7", "MOSI"),
        ] {
            s.assignments.insert(
                net.into(),
                assign(pin, "SPI6", &format!("SPI6_{signal}"), 8),
            );
        }
        s.derive_controllers();
        assert_eq!(fields(&s), want(&[("D3CCIPR", 0x58, "SPI6SEL")]));
    }

    /// A16 row 1, real at source (RM0433 Rev 9 §50.4.12, p. 2177): the
    /// transfer waits, bounded, for `SR.EOT` (bit 3 at `0x14`) after its last
    /// frame and before `EOTC` and `SPE` off, and names the timeout.
    #[test]
    fn spi_transfer_waits_for_eot_before_clearing_it() {
        let files = generate("b", &with_spi(&[("U4", "CS", "PA0")]), &table()).unwrap();
        let transfer = block(&files["src/board.rs"], "impl Spi1<Enabled<()>> {");
        let wait = transfer
            .find(
                "        while !ended && n < spins {\n            \
                 ended = r.read(TOKEN, Block::Spi1, 0x14) & 0x00000008 != 0;\n",
            )
            .expect("the EOT wait");
        let frames = transfer.rfind("spi_frame(").unwrap();
        let eotc = transfer
            .find("r.write(TOKEN, Block::Spi1, 0x18, 0x00000008);")
            .unwrap();
        assert!(frames < wait && wait < eotc, "{transfer}");
        assert!(transfer.contains("        let mut ended = fail;\n"));
        assert!(transfer.contains("} else if !ended {\n            Err(SpiError::Eot)\n"));
        let files =
            generate_with("b", &with_spi(&[("U4", "CS", "PA0")]), &table(), &|_| true).unwrap();
        let main = &files["firmware/src/main.rs"];
        assert!(
            main.contains("Err(SpiError::Eot) => (STATUS_EOT, 0),"),
            "{main}"
        );
        assert!(main.contains("const STATUS_EOT: u32 = 0x22020006;"));
    }

    /// A16 row 1, real at source (RM0433 Rev 9 §8.5.10, p. 372): each enable
    /// reads its `RCC_xxxxENR` back (step b), and a port's or a bus's then
    /// reads its block (step d).
    #[test]
    fn each_enable_reads_its_register_back() {
        let files = generate("b", &with_spi(&[("U4", "CS", "PA0")]), &table()).unwrap();
        let rcc = block(&files["src/board.rs"], "impl Rcc {");
        for (field, ty, off, first) in [
            ("gpioa", "GpioA", "0x0e0", "0x00"),
            ("gpiob", "GpioB", "0x0e0", "0x00"),
            ("i2c1", "I2c1", "0x0e8", "0x00"),
            ("spi1", "Spi1", "0x0f0", "0x00"),
        ] {
            let head = format!("pub fn enable_{field}<");
            let at = rcc.find(&head).unwrap_or_else(|| panic!("{head}"));
            let body = &rcc[at..at + rcc[at..].find("\n    }").unwrap()];
            assert!(
                body.contains(&format!(
                    "        r.modify(TOKEN, Block::Rcc, {off}, |v| v | "
                )) && body.contains(&format!(
                    ");\n        let _ = r.read(TOKEN, Block::Rcc, {off});\n        \
                     let _ = r.read(TOKEN, Block::{ty}, {first});\n"
                )),
                "{body}"
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
        assert!(main.contains("init::<i2c1_speed::Fast400k, _>(&mut r, &clock, pb6, pb7)"));
        // A board with only an I2C bus reads the reset clock too, first, and
        // its crate makes the token (`AET-WO-4230` D6).
        let clock = main
            .find("    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {\n")
            .unwrap();
        assert!(clock < main.find("rcc.enable_").unwrap());
        assert!(files["src/board.rs"].contains("pub struct ResetClock {"));
        assert!(main.contains("let _pa0 = gpioa.pa0.into_output(&mut r);"));
        assert!(main.contains("let pb6 = gpiob.pb6.into_i2c1_scl(&mut r);"));
        assert!(main.contains("use b::{Board, PwrFault, i2c1_speed, mmio::Mmio};"));
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
            orientation: None,
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
        slow.derive_controllers();
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

    // `a_line_no_selectable_speed_serves_is_refused` moved to `aethelgard-ir`
    // (`tests/validate.rs`, `AET-WO-4400b2` F4): `validate` re-derives each
    // stored speed, so such a line is refused by the topology before any
    // generator reads it.

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
            s.gpio.insert(
                (*net).into(),
                aethelgard_ir::pin_devices::SolvedGpio::output(
                    *pin,
                    aethelgard_ir::motor::Level::High,
                ),
            );
            s.spi_devices.insert(
                (*reference).into(),
                spi_device(u32::try_from(i).unwrap(), "SPI1", net),
            );
        }
        s.derive_controllers();
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
        // The firmware hands both over in `init`, each bound after its
        // `into_output`; it then reads both devices' identities, the twin's
        // model set aside (`generate`: `answers` is false; `AET-WO-4241`).
        let main = &files["firmware/src/main.rs"];
        assert!(main.contains("let pa1 = gpioa.pa1.into_output(&mut r);"));
        assert!(main.contains(
            "let mut spi1 = spi1.init(&mut r, &clock, pa5, pa7, pa6, Spi1Selects { pa0, pa1 });"
        ));
        assert_eq!(main.matches("spi1.transfer(").count(), 2);
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
        s.gpio.insert(
            "CS".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PA0",
                aethelgard_ir::motor::Level::High,
            ),
        );
        s.pin_speeds.insert("CS".into(), 0);
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        let init = block(lib, "impl Spi1<Clocked> {");
        assert!(init.contains("_miso: PA6<AlternateFunction<5>>,\n    ) -> Spi1<Enabled<()>> {"));
        assert!(!lib.contains("Spi1Selects") && !lib.contains("SpiError"));
        assert!(!files["src/proofs.rs"].contains("Spi1Spy"));
        let main = &files["firmware/src/main.rs"];
        assert!(main.contains("let _pa0 = gpioa.pa0.into_output(&mut r);"));
        assert!(main.contains("let _spi1 = spi1.init(&mut r, &clock, pa5, pa7, pa6);"));
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
        s.gpio.insert(
            "CS2".into(),
            aethelgard_ir::pin_devices::SolvedGpio::output(
                "PA1",
                aethelgard_ir::motor::Level::High,
            ),
        );
        s.spi_devices
            .insert("U5".into(), spi_device(1, "SPI2", "CS2"));
        s.derive_controllers();
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

    /// The PWM motor fixture with two serial ports: USART2 on PA2/PA3 (port A,
    /// which the SPI1 and TIM1 parts hold) and UART4 on PC10/PC11 (port C,
    /// which only it uses).
    fn motor_serial_topology() -> SolvedTopology {
        use aethelgard_ir::{Framing, SerialRole, SolvedClock, SolvedSerialPort};
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/motors-pwm/topology.json");
        let mut solved: SolvedTopology =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let clock = SolvedClock::reset();
        let framing = Framing {
            baud: 115_200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
            inverted: false,
        };
        for (i, (label, peripheral, tx, rx, af)) in [
            ("console", "USART2", "PA2", "PA3", 7),
            ("gps", "UART4", "PC10", "PC11", 8),
        ]
        .into_iter()
        .enumerate()
        {
            let kernel = serial::kernel_hz(&clock, peripheral).unwrap();
            let brr = serial::solve_brr(kernel, framing.baud).unwrap();
            let up = label.to_ascii_uppercase();
            let (tx_net, rx_net) = (format!("{up}_TX"), format!("{up}_RX"));
            for (net, pin, line) in [(&tx_net, tx, "TX"), (&rx_net, rx, "RX")] {
                solved.assignments.insert(
                    net.clone(),
                    PinAssignment {
                        pin: pin.into(),
                        peripheral: peripheral.into(),
                        signal: format!("{peripheral}_{line}"),
                        alternate_function: af,
                    },
                );
            }
            solved.serial_ports.insert(
                label.into(),
                SolvedSerialPort {
                    index: u32::try_from(i).unwrap(),
                    peripheral: peripheral.into(),
                    tx: tx_net,
                    rx: rx_net,
                    role: if i == 0 {
                        SerialRole::Console
                    } else {
                        SerialRole::Gps
                    },
                    framing: framing.clone(),
                    negotiated: None,
                    peer_deviation_ppm: 0,
                    kernel_hz: kernel,
                    brr,
                    actual_millibaud: serial::actual_millibaud(kernel, brr),
                    error_ppm: serial::error_ppm(kernel, brr, framing.baud),
                },
            );
        }
        // Every pin a stored speed (`AET-WO-4400b` D2): no line here is SPI.
        let nets: Vec<String> = solved
            .assignments
            .keys()
            .chain(solved.gpio.keys())
            .cloned()
            .collect();
        for net in nets {
            solved.pin_speeds.entry(net).or_insert(0);
        }
        solved
    }

    /// A motor board with serial ports splits its schedule with no USART
    /// action (`AET-WO-4300b` merged with `AET-WO-3303` §1.2): a USART's pins
    /// are in no part, port C, which only UART4 uses, has its enable in the
    /// `gpio` part, and the split leaves no action of the whole unscheduled.
    #[test]
    fn a_motor_board_with_serial_ports_splits_with_no_usart_action() {
        let solved = motor_serial_topology();
        let plan = Plan::new("b", &solved, &table()).unwrap();
        let facts = Facts::new(&plan, &cmsis::register_facts().unwrap()).unwrap();
        let parts = parts(&plan);
        let names: Vec<&str> = parts.iter().filter_map(|p| p.part.as_deref()).collect();
        assert_eq!(names, ["i2c1", "spi1", "tim1", "gpio"]);
        assert_eq!(check_parts(&plan, &facts, &parts), Ok(()));
        assert_eq!(parts[3].actions(&plan), [Action::Port('C')]);
        let serial = ["PA2", "PA3", "PC10", "PC11"];
        for scope in parts.iter().chain([&Scope::whole(&plan)]) {
            for pin in serial {
                assert!(!scope.pins.contains(pin), "{pin}");
                assert!(!scope.actions(&plan).contains(&Action::Pin(pin.into())));
            }
        }
        let proofs = &generate("b", &solved, &table()).unwrap()["src/proofs.rs"];
        for part in ["i2c1", "spi1", "tim1", "gpio"] {
            assert!(proofs.contains(&format!("fn every_valid_sequence_{part}() {{")));
        }
        assert!(proofs.contains("fn usart2_start_writes_the_solved_values() {"));
        assert!(proofs.contains("fn uart4_start_writes_the_solved_values() {"));
        // The whole image, which the targeted harnesses set up through, holds
        // no USART block and fails any access to one.
        let image = block(proofs, "impl Image {");
        assert!(image.contains("USART2 was accessed through the harness image"));
        assert!(image.contains("UART4 was accessed through the harness image"));
        let gpio = block(proofs, "fn every_valid_sequence_gpio() {");
        assert!(gpio.contains("let _pins = rcc.enable_gpioc("));
        assert!(!gpio.contains("Usart2") && !gpio.contains("Uart4"));
    }

    /// [`with_spi`] with a second I2C bus and no timer (`AET-WO-3303b` §1.3):
    /// I2C1 on PB6/PB7 (bus A), I2C2 on PB10/PB11 (bus B), SPI1 on PA5-PA7
    /// with U4's chip select on PA0.
    fn two_i2c_one_spi() -> (SolvedTopology, Plan, Facts) {
        let mut s = with_spi(&[("U4", "CS", "PA0")]);
        s.assignments
            .insert("SCL2".into(), assign("PB10", "I2C2", "I2C2_SCL", 4));
        s.assignments
            .insert("SDA2".into(), assign("PB11", "I2C2", "I2C2_SDA", 4));
        s.i2c_ceilings = BTreeMap::new();
        s.i2c_speeds = BTreeMap::new();
        s.analog_lines = analog_lines(&[("SCL", "A"), ("SDA", "A"), ("SCL2", "B"), ("SDA2", "B")]);
        set_ceiling(&mut s, "A", 400_000);
        set_ceiling(&mut s, "B", 400_000);
        s.derive_controllers();
        let plan = Plan::new("b", &s, &table()).unwrap();
        let facts = Facts::new(&plan, &cmsis::register_facts().unwrap()).unwrap();
        (s, plan, facts)
    }

    /// `AET-WO-3303b` §1.2: a board without a timer splits its schedule
    /// whenever [`parts`] yields two or more, and each part's scope is what
    /// [`parts`] documents: its peripheral, its pins and chip selects, their
    /// ports' enables, the reset-clock read, and for SPI the kernel selection.
    #[test]
    fn a_board_without_timers_splits_its_schedule_into_its_parts() {
        let (solved, plan, facts) = two_i2c_one_spi();
        assert!(plan.timers.is_empty());
        let parts = parts(&plan);
        let names: Vec<&str> = parts.iter().filter_map(|p| p.part.as_deref()).collect();
        assert_eq!(names, ["i2c1", "i2c2", "spi1"]);
        assert_eq!(check_parts(&plan, &facts, &parts), Ok(()));
        let set =
            |items: &[&str]| -> BTreeSet<String> { items.iter().map(|&i| i.into()).collect() };
        for (part, peripheral, pins, port) in [
            (&parts[0], "I2C1", set(&["PB6", "PB7"]), 'B'),
            (&parts[1], "I2C2", set(&["PB10", "PB11"]), 'B'),
            (&parts[2], "SPI1", set(&["PA0", "PA5", "PA6", "PA7"]), 'A'),
        ] {
            assert_eq!(part.peripherals, set(&[peripheral]));
            assert_eq!(part.pins, pins);
            assert_eq!(part.ports, BTreeSet::from([port]));
            assert!(part.timers.is_empty());
        }
        use Action::{Clock, Pin, Port, ResetClock, SelectKernels, Start};
        assert_eq!(
            parts[1].actions(&plan),
            [
                Port('B'),
                Clock("I2C2".into()),
                Pin("PB10".into()),
                Pin("PB11".into()),
                Start("I2C2".into()),
                ResetClock,
            ]
        );
        assert_eq!(
            parts[2].actions(&plan),
            [
                Port('A'),
                Clock("SPI1".into()),
                Pin("PA0".into()),
                Pin("PA5".into()),
                Pin("PA6".into()),
                Pin("PA7".into()),
                Start("SPI1".into()),
                ResetClock,
                SelectKernels,
            ]
        );
        // Emitted as parts, each a proof, and the whole schedule is no proof.
        let proofs = &generate("b", &solved, &table()).unwrap()["src/proofs.rs"];
        for part in names {
            let at = proofs
                .find(&format!("fn every_valid_sequence_{part}() {{"))
                .unwrap_or_else(|| panic!("no {part} part"));
            assert!(
                proofs[..at]
                    .rfind("#[kani::proof]\n#[kani::unwind(")
                    .is_some_and(|h| at - h < 40),
                "{part}"
            );
        }
        assert!(!proofs.contains("fn every_valid_sequence() {"));
        assert_eq!(proofs.matches("fn every_valid_sequence_").count(), 3);
    }

    /// A mutant of [`a_board_without_timers_splits_its_schedule_into_its_parts`]:
    /// one of PB10's bits in a second part is two owners of one bit.
    #[test]
    fn a_timerless_split_whose_parts_share_a_bit_is_refused() {
        let (_, plan, facts) = two_i2c_one_spi();
        let mut parts = parts(&plan);
        parts[0].pins.insert("PB10".into());
        assert_eq!(
            check_parts(&plan, &facts, &parts),
            Err(BoardError::FootprintOverlap {
                first: "the i2c1 part".into(),
                second: "the i2c2 part".into(),
                register: "GpioB at 0x0, bits 0x00300000".into(),
            })
        );
    }

    /// A mutant of [`a_board_without_timers_splits_its_schedule_into_its_parts`]:
    /// an action of the whole schedule that no part schedules.
    #[test]
    fn a_timerless_split_that_leaves_an_action_unscheduled_is_refused() {
        let (_, plan, facts) = two_i2c_one_spi();
        let mut parts = parts(&plan);
        parts[1].pins.remove("PB11");
        assert_eq!(
            check_parts(&plan, &facts, &parts),
            Err(BoardError::UnscheduledAction {
                action: "the PB11 configure".into(),
            })
        );
    }

    /// `AET-WO-3303b` §1.2: a split into one part is the whole, so a board
    /// whose [`parts`] yields one keeps the whole harness, byte for byte; the
    /// reference's I2C1 with an output no unit owns is two parts.
    #[test]
    fn a_board_with_one_part_keeps_the_whole_schedule_harness() {
        let mut one = solved();
        one.gpio.clear();
        one.pin_speeds.remove("CS");
        let proofs = &generate("b", &one, &table()).unwrap()["src/proofs.rs"];
        assert!(proofs.contains("fn every_valid_sequence() {"));
        assert!(!proofs.contains("fn every_valid_sequence_"));
        assert!(!proofs.contains("fn rely("));
        let proofs = &generate("b", &solved(), &table()).unwrap()["src/proofs.rs"];
        assert!(!proofs.contains("fn every_valid_sequence() {"));
        for part in ["i2c1", "gpio"] {
            assert!(proofs.contains(&format!("fn every_valid_sequence_{part}() {{")));
        }
        let gpio = block(proofs, "fn every_valid_sequence_gpio() {");
        assert!(gpio.contains("into_output("), "the gpio part drives PA0");
    }

    /// `AET-WO-1207` §1.5: the one SPI mode the generated firmware drives is
    /// `aethelgard_ir::parts::DRIVEN_SPI_MODE`, the mode `validate` holds every
    /// SPI device's row to (`SpiModeNotDriven`). `CFG2`'s `CPOL` and `CPHA`,
    /// at the header's positions, are that mode's two bits in the word
    /// [`spi_code`] composes, for every kernel and divider: the mode is one
    /// fixed value, whatever the solve or the part.
    #[test]
    fn the_cfg2_word_drives_the_spi_mode_validate_judges() {
        let f = cmsis::register_facts().unwrap();
        let s = spi_facts(&f, &f.layout("SPI").unwrap()).unwrap();
        let cpol = f.position("SPI_CFG2_CPOL_Pos").unwrap();
        let cpha = f.position("SPI_CFG2_CPHA_Pos").unwrap();
        let master = f.position("SPI_CFG2_MASTER_Pos").unwrap();
        let enable = f.position("SPI_CR1_SPE_Pos").unwrap();
        assert_eq!((cpol, cpha), (25, 24));
        for kernel in [SpiKernel::PerCk, SpiKernel::HsiKerCk] {
            for mbr in 0..=7 {
                let c = SolvedSpiController {
                    kernel,
                    kernel_hz: 64_000_000,
                    declared_hz: 1_000_000,
                    mbr,
                    achieved_hz: 1_000_000,
                    achieved_max_hz: 1_000_000,
                };
                let cfg2 = spi_code(&c, &s, enable, master).cfg2;
                let mode = ((cfg2 >> cpol) & 1) << 1 | ((cfg2 >> cpha) & 1);
                assert_eq!(
                    mode,
                    u32::from(aethelgard_ir::parts::DRIVEN_SPI_MODE),
                    "{kernel:?} MBR {mbr}: CFG2 {cfg2:#010x}"
                );
            }
        }
    }

    // ---- AET-WO-4240v §1.4: the supply configuration and VOS Scale 2.

    /// A board with one GPIO output and nothing else: its plan reads no clock
    /// (`needs_clock()` false), so it has no `ResetClock`.
    fn gpio_only() -> SolvedTopology {
        let mut s = solved();
        s.assignments.clear();
        s.i2c_ceilings.clear();
        s.i2c_speeds.clear();
        s.analog_lines.clear();
        s.i2c_timings.clear();
        s.pin_speeds = BTreeMap::from([("CS".into(), 0)]);
        s
    }

    /// The generator's own words (from the header and the declaration) are the
    /// rule's (`aethelgard_ir::vos`), which the oracle and the twin hold the
    /// firmware to; and the header's places are RM0433's (§6.8.2, §6.8.4,
    /// §6.8.6: `CSR1` 0x004, `CR3` 0x00C, `D3CR` 0x018, both ready bits 13).
    #[test]
    fn the_generators_pwr_words_are_the_rules() {
        let p = PwrFacts::new(&cmsis::register_facts().unwrap()).unwrap();
        assert_eq!(p.base, 0x5802_4800);
        assert_eq!(
            (p.csr1, p.cr3, p.d3cr),
            (ORACLE_PWR_CSR1, ORACLE_PWR_CR3, ORACLE_PWR_D3CR)
        );
        assert_eq!(1 << p.actvosrdy, ORACLE_PWR_READY);
        assert_eq!(1 << p.vosrdy, ORACLE_PWR_READY);
        assert_eq!(p.supply_mask(), vos::CR3_SUPPLY_MASK);
        assert_eq!(p.config_mask(), vos::CR3_SUPPLY_CONFIG_MASK);
        for mode in [CoreSupplyMode::Ldo, CoreSupplyMode::Bypass] {
            assert_eq!(p.supply_bits(mode), vos::cr3_supply_bits(mode), "{mode:?}");
        }
        assert_eq!(p.d3cr_scale2(), vos::D3CR_VOS_SCALE2);
        assert_eq!(((1u32 << p.vos.1) - 1) << p.vos.0, vos::D3CR_VOS_MASK);
    }

    /// The sequence's text: `CR3` read-modify-written once with the LDO bits,
    /// read back under `LDOEN` and `BYPASS`, `ACTVOSRDY` awaited, `D3CR`
    /// written whole once, `VOSRDY` awaited, in that order; `scale2` consumes
    /// the one `Pwr`, and `reset_clock` takes its token.
    #[test]
    fn scale2_writes_cr3_then_d3cr_once_each_in_order() {
        let files = generate("b", &solved(), &table()).unwrap();
        let board = &files["src/board.rs"];
        let at = |s: &str| {
            assert_eq!(board.matches(s).count(), 1, "{s}");
            board.find(s).unwrap()
        };
        let order = [
            at(
                "pub fn scale2<R: Registers>(self, r: &mut R, polls: u32) -> Result<Scale2, PwrFault> {",
            ),
            at("r.modify(TOKEN, Block::Pwr, 0x0c, |v| (v & !0x00000007) | 0x00000002);"),
            at(
                "if r.read(TOKEN, Block::Pwr, 0x0c) & 0x00000003 != 0x00000002 {\n            return Err(PwrFault::SupplyNotDeclared);",
            ),
            at(
                "if !pwr_ready(r, 0x04, 0x00002000, polls) {\n            return Err(PwrFault::ActvosrdyTimeout);",
            ),
            at("r.write(TOKEN, Block::Pwr, 0x18, 0x00008000);"),
            at(
                "if !pwr_ready(r, 0x18, 0x00002000, polls) {\n            return Err(PwrFault::VosrdyTimeout);",
            ),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert_eq!(board.matches("Block::Pwr").count(), 5);
        assert!(board.contains("            Block::Pwr => 0x58024800,"));
        assert!(
            board.contains("pub fn reset_clock<R: Registers>(&self, r: &mut R, _scale2: &Scale2)")
        );
        assert_eq!(board.matches("Scale2 { _private: () }").count(), 1);
        assert!(board.contains("    pub pwr: Pwr,\n"));
    }

    /// `bypass` validates and solves, and `generate` and the ledger refuse it
    /// by name (`AET-WO-4240v` D-1).
    #[test]
    fn a_bypass_supply_is_refused_as_not_generated() {
        let mut s = solved();
        s.core_supply = CoreSupplyMode::Bypass;
        s.validate().unwrap();
        let e = generate("b", &s, &table()).unwrap_err();
        assert_eq!(e.kind(), "CoreSupplyNotGenerated");
        assert_eq!(
            register_fields(&s, &table()).unwrap_err().kind(),
            "CoreSupplyNotGenerated"
        );
        assert!(generate("b", &solved(), &table()).is_ok());
    }

    /// `AET-WO-1211b`: a pin device's input is generated: `MODER` `00` alone
    /// (no speed, no alternate function), and its declared pull in the port's
    /// whole `PUPDR`, which the oracle takes from `silicon::implied_pupd`; an
    /// output declared `low` (PB8) is written by its `BSRR` reset bit before
    /// `MODER`.
    #[test]
    fn a_pin_device_s_signals_are_generated_at_their_declared_pull_and_level() {
        use aethelgard_ir::motor::Level;
        use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio, SolvedPinDevice};
        let mut s = solved();
        s.pin_devices.insert(
            "U9".into(),
            SolvedPinDevice {
                part: "WAKE_SWITCH".into(),
                bridges: Default::default(),
                supplies: Default::default(),
            },
        );
        s.gpio.insert(
            "WAKE".into(),
            SolvedGpio {
                pin: "PB5".into(),
                direction: Direction::Input,
                initial: None,
                pull: Some(McuPull::Up),
                active: Some(aethelgard_ir::motor::Level::Low),
                pulled: Some(aethelgard_ir::motor::Level::High),
            },
        );
        s.gpio
            .insert("SLEEP".into(), SolvedGpio::output("PB8", Level::Low));
        s.pin_speeds.insert("WAKE".into(), 0);
        s.pin_speeds.insert("SLEEP".into(), 0);
        s.validate().unwrap();
        let files = generate("b", &s, &table()).unwrap();
        let lib = &files["src/board.rs"];
        let input = block(
            lib,
            "    pub fn into_input<R: Registers>(self, r: &mut R) -> PB5<Input> {",
        );
        assert_eq!(
            input.lines().filter(|l| l.contains("r.modify")).count(),
            1,
            "{input}"
        );
        assert!(
            input.contains("Block::GpioB, 0x00, |v| v & !0x00000c00"),
            "{input}"
        );
        let output = block(
            lib,
            "    pub fn into_output<R: Registers>(self, r: &mut R) -> PB8<Output> {",
        );
        let reset = output.find("0x18, 0x01000000").expect("BR8");
        let moder = output.find("Block::GpioB, 0x00,").expect("MODER");
        assert!(reset < moder, "{output}");
        // PB4's reset pull-up, PB5's declared pull-up; every other field 0.
        let pupdr = 0x0000_0100 | (0b01 << 10);
        assert!(
            lib.contains(&format!(
                "r.write(TOKEN, Block::GpioB, 0x0c, {pupdr:#010x});"
            )),
            "the generator's PUPDR"
        );
        let proofs = &files["src/proofs.rs"];
        assert!(
            proofs.contains(&format!(
                "value == {pupdr:#010x}, \"GpioB PUPDR is not the solved word, whole\""
            )),
            "the oracle's PUPDR"
        );
        assert!(proofs.contains("PB8 is not at its declared initial level"));
        assert!(generate("b", &solved(), &table()).is_ok());
    }

    /// `check_silicon` runs the voltage-scaling rule first and refuses a tree
    /// it does not hold by the clock's name: the reset tree at VOS3 runs
    /// `rcc_pclk1` and every APB at 64,958,976 Hz at its fastest, over 50 MHz
    /// (DS12117 Rev 10 Table 111). `validate` refuses it first, so the check
    /// is called on its own.
    #[test]
    fn check_silicon_refuses_a_tree_the_vos_rule_does_not_hold() {
        check_silicon(&solved()).unwrap();
        let mut s = solved();
        s.clock.vos = aethelgard_ir::vos::Vos::Scale3;
        let e = check_silicon(&s).unwrap_err();
        assert_eq!(e.kind(), "VosRuleFails");
        let BoardError::VosRuleFails { clock, finding } = &e else {
            unreachable!()
        };
        let failing: Vec<String> = vos::judge_generated(&s)
            .clocks
            .into_iter()
            .filter(|c| c.holds == Some(false))
            .map(|c| c.clock)
            .collect();
        assert!(failing.contains(clock), "{clock} not in {failing:?}");
        assert!(finding.contains("64958976 Hz"), "{finding}");
        assert!(finding.contains("50000000 Hz"), "{finding}");
    }

    /// `main`: the sequence is the first hardware access, before the reset
    /// clock, the brown-out word and any enable; a fault writes its own boot
    /// word, once, and parks; the fixed point writes `BOOT_MAGIC`.
    #[test]
    fn main_runs_the_supply_and_scale2_first_and_a_fault_writes_only_its_word() {
        let files = generate("b", &solved(), &table()).unwrap();
        let main = &files["firmware/src/main.rs"];
        let at = |s: &str| main.find(s).unwrap_or_else(|| panic!("{s}\n{main}"));
        let order = [
            at("    }) = Board::take()\n"),
            at("    let scale2 = match pwr.scale2(&mut r, PWR_POLLS) {\n"),
            at("    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {\n"),
            at("    let optsr = flash.option_status(&mut r);\n"),
            at("rcc.enable_"),
            at(".write_volatile(BOOT_MAGIC)"),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        let fault = &main[at("        Err(fault) => {\n")..at("    let Ok(clock)")];
        for (name, word) in [
            ("SupplyNotDeclared", "BOOT_SUPPLY_NOT_DECLARED"),
            ("ActvosrdyTimeout", "BOOT_ACTVOSRDY_TIMEOUT"),
            ("VosrdyTimeout", "BOOT_VOSRDY_TIMEOUT"),
        ] {
            assert!(
                fault.contains(&format!("PwrFault::{name} => {word},")),
                "{name}"
            );
        }
        assert_eq!(fault.matches("write_volatile").count(), 1);
        assert!(fault.contains(".write_volatile(word)"));
        assert!(fault.contains("loop {"));
        assert!(!fault.contains("r."), "a register access after a fault");
        // The three words' `const` lines: `tests/board.rs`
        // (`the_firmware_follows_the_judges_pwr_boot_words`).
        assert!(main.contains(&format!("const PWR_POLLS: u32 = {PWR_POLLS};")));
        // Three distinct words, none the magic.
        let words = [
            BOOT_SUPPLY_NOT_DECLARED,
            BOOT_ACTVOSRDY_TIMEOUT,
            BOOT_VOSRDY_TIMEOUT,
            firmware::BOOT_MAGIC,
        ];
        assert_eq!(BTreeSet::from(words).len(), 4);
    }

    /// A plan that reads no clock still runs the sequence first (`AET-WO-4240v`
    /// §1.4: every firmware), and its port enables take the token (`4240v` F2).
    #[test]
    fn a_board_without_a_clock_still_runs_the_sequence_first() {
        let s = gpio_only();
        let plan = Plan::new("b", &s, &table()).unwrap();
        assert!(!plan.needs_clock());
        let files = generate("b", &s, &table()).unwrap();
        let main = &files["firmware/src/main.rs"];
        let seq = main
            .find("    let scale2 = match pwr.scale2(&mut r, PWR_POLLS) {\n")
            .unwrap_or_else(|| panic!("{main}"));
        assert!(seq < main.find("flash.option_status(").unwrap());
        assert!(seq < main.find("rcc.enable_").unwrap());
        assert!(!main.contains("reset_clock"));
        let board = &files["src/board.rs"];
        assert!(board.contains("pub fn scale2<R: Registers>"));
        assert!(!board.contains("pub struct ResetClock"));
        // Its proofs: the PWR harness, and the schedule's first step on the image.
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains("fn pwr_scale2_is_once_in_order_and_bounded()"));
        assert!(proofs.contains("    let scale2 = scale2.unwrap();\n"));
        assert!(!proofs.contains("fn scale2_token("));
    }

    /// The oracle's PWR clause and the new harness carry `aethelgard_ir::vos`'s
    /// words; every schedule (whole and part) takes the first step and checks
    /// the clause; the targeted harnesses take their token from the stub.
    #[test]
    fn the_oracle_holds_pwr_to_the_rules_words() {
        let files = generate("b", &solved(), &table()).unwrap();
        let proofs = &files["src/proofs.rs"];
        assert!(proofs.contains("after.pwr[3] == (before.pwr[3] & !0x00000007) | 0x00000002,"));
        assert!(proofs.contains("after.pwr[6] & 0x0000c000 == 0x00008000,"));
        assert!(proofs.contains("after.pwr[6] & !0x00002000 == 0x00008000,"));
        for i in [0, 1, 2, 4, 5, 7, 8, 9, 10] {
            assert!(
                proofs.contains(&format!("kani::assert(after.pwr[{i}] == before.pwr[{i}], ")),
                "{i}"
            );
        }
        assert!(proofs.contains(
            "    let scale2 = board.pwr.scale2(&mut img, 1);\n    kani::assume(scale2.is_ok());\n"
        ));
        assert!(proofs.contains("let read = rcc.reset_clock(&mut img, &scale2);"));
        assert!(proofs.contains("let result = board.rcc.reset_clock(&mut spy, &scale2);"));
        assert!(
            proofs.contains("#[kani::unwind(3)]\nfn pwr_scale2_is_once_in_order_and_bounded() {")
        );
        assert!(proofs.contains("let result = board.pwr.scale2(&mut spy, 2);"));
        for cover in [
            "Scale 2 reached",
            "the supply is not the declared one",
            "ACTVOSRDY times out",
            "VOSRDY times out",
        ] {
            assert!(
                proofs.contains(&format!("kani::cover!(true, \"{cover}\");")),
                "{cover}"
            );
        }
        // A motor board: every part takes the first step and checks the clause.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/motors-pwm/topology.json");
        let motors: SolvedTopology = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let proofs = &generate("b", &motors, &table()).unwrap()["src/proofs.rs"];
        let parts = proofs.matches("fn every_valid_sequence_").count();
        assert!(parts > 1);
        assert_eq!(
            proofs
                .matches("let scale2 = board.pwr.scale2(&mut img, 1);")
                .count(),
            parts
        );
        assert_eq!(
            proofs
                .matches("PWR is not before plus exactly the declared supply and VOS Scale 2: CR3")
                .count(),
            parts
        );
        assert_eq!(
            proofs.matches("        if block != Block::Pwr {\n").count(),
            parts
        );
    }
}
