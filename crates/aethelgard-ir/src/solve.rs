//! The pin solver (`AET-WO-1200`, `RUL-02`): a deterministic search from a
//! [`Specification`] and a [`PinTable`] to a [`SolvedTopology`]. The search
//! order, the reserved pins and the refusals, in check order, are in this
//! crate's `README.md`, as is why its output depends on its inputs alone.

use std::collections::{BTreeMap, BTreeSet};

use crate::clock::Budget as PpmBudget;
use crate::motor::{self, MotorProtocol, PwmTiming, SolvedClock, SolvedMotorOutput, TimingError};
use crate::serial::{self, SolvedSerialPort};
use crate::{
    AnalogLine, IrValidationError, PinAssignment, PinTable, Signal, SolvedI2cDevice,
    SolvedI2cTiming, SolvedSpiController, SolvedSpiDevice, SolvedTopology, Specification,
    SpiKernel, SpiLine, electrical,
};

/// Never assigned, and refused as a hand pin: the SWD debug port (`PA13`
/// SWDIO, `PA14` SWCLK), so the board can be programmed, and the LSE pins
/// (`PC14`/`PC15`), so it keeps its 32 kHz crystal. The HSE pins are
/// reserved beside them only when `mcu.hse` declares one ([`reserved_pins`]).
pub const RESERVED_PINS: [&str; 4] = ["PA13", "PA14", "PC14", "PC15"];

/// The pins `solve` never assigns and refuses as a hand pin
/// (`AET-WO-4240a` §1.6): [`RESERVED_PINS`], and `PH0` (`OSC_IN`) and
/// `PH1` (`OSC_OUT`) beside a crystal, or `PH0` alone beside an external
/// clock, whose `OSC_OUT` is free (RM0433 Rev 9 §8.5.2, Figure 46, p. 339).
pub fn reserved_pins(hse: Option<&crate::clock::Hse>) -> Vec<&'static str> {
    let mut pins = RESERVED_PINS.to_vec();
    match hse {
        None => {}
        Some(crate::clock::Hse::Crystal(_)) => pins.extend(["PH0", "PH1"]),
        Some(crate::clock::Hse::External(_)) => pins.push("PH0"),
    }
    pins
}

/// What the engine's reservation is, as [`SolveError::ReservedPin`] names it.
const ENGINE_RESERVED: &str = "debug port or oscillator";

/// Every pin `solve` never assigns: the engine's ([`reserved_pins`]), then
/// the specification's `mcu.reserved_pins` (`AET-WO-4300i` F7), which every
/// candidate filter reads, so no branch offers one.
pub(crate) fn all_reserved(spec: &Specification) -> Vec<&str> {
    let mut pins = reserved_pins(spec.mcu.hse.as_ref());
    pins.extend(spec.mcu.reserved_pins.iter().map(|r| r.pin.as_str()));
    pins
}

/// The refusal of a hand pin on a reserved `pin`: the engine's reason, or
/// the declared one (`AET-WO-4300i` F7).
pub(crate) fn reserved_refusal(spec: &Specification, pin: &str, claimant: &str) -> SolveError {
    let reason = match spec.mcu.reserved_pins.iter().find(|r| r.pin == pin) {
        // belt-and-suspenders: validate refuses a declared pin the engine also
        // reserves (`ReservedPinEngineReserved`); the engine's reason wins here.
        Some(r) if !reserved_pins(spec.mcu.hse.as_ref()).contains(&pin) => {
            format!("`mcu.reserved_pins`: {}", r.reason)
        }
        _ => ENGINE_RESERVED.to_owned(),
    };
    SolveError::ReservedPin {
        pin: pin.to_owned(),
        claimant: claimant.to_owned(),
        reason,
    }
}

/// The pins `solve` offers no digital claimant and refuses as a digital
/// hand pin (`AET-WO-1217` F4): the MCU row's `io.analog_pads`. An ADC input
/// may take one. A part with no power-parts row has no pin table either
/// (`aethelgard-svd-hal` holds only parts with a row); only a test's own
/// table reaches that case, and its pins are offered as its table gives them.
pub(crate) fn analog_pads(part: &str) -> Result<Vec<&'static str>, IrValidationError> {
    let table = crate::power_parts::table()?;
    Ok(match table.get(part).map(|r| &r.kind) {
        Some(crate::power_parts::Kind::Mcu(m)) => {
            m.io.analog_pads.pins.iter().map(String::as_str).collect()
        }
        _ => Vec::new(),
    })
}

/// The values [`solve`] may try, across the whole solve including the
/// minimal-set searches, before refusing with [`SolveError::BudgetExceeded`].
pub const NODE_LIMIT: u64 = 10_000_000;

/// One variant per refusal; what `claimants()` names is in the README.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SolveError {
    /// The specification failed [`Specification::validate`], or the hash is
    /// malformed; `solve` re-checks both rather than trusting its caller.
    /// The bus refusals `BusTooFast`, `PullUpBelowSinkFloor` and
    /// `BusCapacitanceOverLimit` are validation's own variants, raised here
    /// too (`AET-WO-1205` F1, `AET-WO-1206` §1.5: one error set); each keeps
    /// its kind, `id`, claimant and message. `RiseTimeTooSlow`,
    /// `SupplyOutsideSpeedTable` and `MissingSpiConstraint` (`AET-WO-1207`
    /// §1.2), and `MotorTiming`, `SerialBaudOutOfRange` and
    /// `SerialBaudTolerance` (`AET-WO-1208` §1.3), are `validate`'s alone
    /// (step 21), which `solve` calls first.
    #[error("{0}")]
    InvalidSpecification(IrValidationError),
    #[error("the specification is for part `{spec}`, the pin table for `{table}`")]
    PartMismatch { spec: String, table: String },
    #[error("`{claimant}` names pin `{pin}`, which the pin table does not have")]
    UnknownPin { pin: String, claimant: String },
    /// `reason` is the engine's (debug port or oscillator) or the declared
    /// reservation's (`mcu.reserved_pins`, `AET-WO-4300i` F7).
    #[error("`{claimant}` names pin `{pin}`, which is reserved ({reason})")]
    ReservedPin {
        pin: String,
        claimant: String,
        reason: String,
    },
    #[error(
        "`{claimant}` names pin `{pin}`, whose pad takes only an analog input (the MCU row's `io.analog_pads`: DS12117 Table 8 types it `ANA`, and a Px_C pin sinks at most 1 mA); a digital signal goes on another pin"
    )]
    AnalogPadPin { pin: String, claimant: String },
    #[error(
        "`{claimant}` names peripheral `{peripheral}`, which the pin table does not offer for this bus kind"
    )]
    UnknownPeripheral {
        peripheral: String,
        claimant: String,
    },
    #[error("pin `{pin}` is claimed by both `{first}` and `{second}`")]
    PinClaimedTwice {
        pin: String,
        first: String,
        second: String,
    },
    #[error("peripheral `{peripheral}` is claimed by both `{first}` and `{second}`")]
    PeripheralClaimedTwice {
        peripheral: String,
        first: String,
        second: String,
    },
    #[error("no instance can serve bus `{bus}`: {}", .reasons.join("; "))]
    NoInstance { bus: String, reasons: Vec<String> },
    #[error(
        "buses {} cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)",
        quoted(.buses)
    )]
    Infeasible { buses: Vec<String> },
    #[error("the search tried {limit} values without an answer; it was placing bus `{bus}`")]
    BudgetExceeded { bus: String, limit: u64 },
    /// `withheld` names the pins withheld from a net at a connector as not
    /// FT (`AET-WO-4300i` D1), and is empty when none is.
    #[error("`{claimant}` has no free pin left{withheld}")]
    NoFreePin { claimant: String, withheld: String },
    /// `withheld` as for [`SolveError::NoFreePin`].
    #[error("no free pin carries a single-ended ADC input for ADC net `{net}`{withheld}")]
    NoAdcPin { net: String, withheld: String },
    /// `AET-WO-1211a`: a pin device's ADC input pinned, or its channel
    /// named, where the pin carries no such input.
    #[error("ADC net `{net}`: {reason}")]
    AdcChannelNotOnPin { net: String, reason: String },
    /// `AET-WO-1211a`, `1211` Q8: the only free ADC pins carry an ADC2 input
    /// alone, which the twin does not model; pin one by hand to take it.
    #[error(
        "ADC net `{net}`: the free ADC pins {} carry ADC2 alone, which the twin does not model (`1211` Q8); the solver takes ADC1 or ADC3, and an ADC2 pin only when the specification pins it{withheld}",
        quoted(.pins)
    )]
    AdcUnmodelled {
        net: String,
        pins: Vec<String>,
        /// As for [`SolveError::NoFreePin`].
        withheld: String,
    },
    /// `AET-WO-1211a`, `1211` §2.6: a pin device's driven input on a pin the
    /// MCU pulls up from reset (PA13, PA15, PB4), against the part's own
    /// pull-down until the first write.
    #[error(
        "`{net}` drives a pin device's input from `{pin}`, which the MCU pulls up from reset to the first write (RM0433 Rev 9 p. 542): the part's pull-down cannot hold it; pin it elsewhere"
    )]
    UnheldDuringReset { net: String, pin: String },
    /// `AET-WO-1211a`, `1211` §2.7 (e): an IN/IN bridge's two motor outputs
    /// on two timers, so no one counter times both.
    #[error(
        "bridge `{bridge}`: its inputs `{first}` and `{second}` are on `{first_timer}` and `{second_timer}`; an IN/IN pair shares one timer, so pin both to one"
    )]
    BridgeSplitAcrossTimers {
        bridge: String,
        first: String,
        second: String,
        first_timer: String,
        second_timer: String,
    },
    #[error("no timer channel can serve motor output `{output}`: {}", .reasons.join("; "))]
    NoTimerChannel {
        output: String,
        reasons: Vec<String>,
    },
    #[error(
        "motor outputs `{first}` and `{second}` can only be on timer `{timer}`, and their timing differs: the outputs of one timer share its rate"
    )]
    TimerRateConflict {
        timer: String,
        first: String,
        second: String,
    },
    #[error(
        "motor outputs `{first}` and `{second}` can only be on timer `{timer}`, and their resolution_ps differs: the outputs of one timer share its tick"
    )]
    TimerResolutionConflict {
        timer: String,
        first: String,
        second: String,
    },
    #[error(
        "{} cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)",
        quoted(.claimants)
    )]
    MotorsInfeasible { claimants: Vec<String> },
    #[error(
        "serial port `{port}` fixes TX on `{tx_pin}` and RX on `{rx_pin}`, and no USART or UART it may use carries both: a port's two lines are one peripheral's"
    )]
    SerialPinsSplit {
        port: String,
        tx_pin: String,
        rx_pin: String,
    },
    #[error("no USART or UART can serve serial port `{port}`: {}", .reasons.join("; "))]
    NoSerialPeripheral { port: String, reasons: Vec<String> },
    #[error(
        "{} cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)",
        quoted(.claimants)
    )]
    SerialPortsInfeasible { claimants: Vec<String> },
    #[error(
        "serial port `{port}`: the solved clock gives no kernel clock for `{peripheral}` (RM0433 Rev 9 §8.7.21, `RCC_D2CCIP2R` at reset)"
    )]
    SerialClock { port: String, peripheral: String },
    #[error(
        "SPI bus `{bus}` on `{peripheral}`: the solved clock gives it no kernel clock (`per_ck` or `hsi_ker_ck` from HSI, RM0433 Rev 9 §8.7.19, §8.7.20, §8.7.22)"
    )]
    SpiKernelClockUnavailable { bus: String, peripheral: String },
    #[error(
        "SPI bus `{bus}` declares {speed_hz} Hz, below {floor_hz} Hz, the slowest `{peripheral}` reaches: its {kernel_hz} Hz kernel clock at HSI's fastest, {fast_hz} Hz (DS12117 Rev 10 Table 36 / Table 133), divided by 256 (MBR 111, RM0433 Rev 9 §50.11.3)"
    )]
    SpiSpeedBelowKernelRange {
        bus: String,
        peripheral: String,
        speed_hz: u32,
        kernel_hz: u64,
        fast_hz: u64,
        floor_hz: u64,
    },
    /// An I2C controller's kernel clock gives `mode` no `TIMINGR` at its
    /// source's extremes (`4240` F-PANIC, `AET-WO-4240a` §1.7): once a pin
    /// that was a panic.
    #[error(
        "I2C bus `{bus}` on `{peripheral}`: its {kernel_hz} Hz kernel clock gives {mode} no TIMINGR (below the mode's least kernel at the source's slowest, or no PRESC fits at its fastest; RM0433 Rev 9 §47.4.9)"
    )]
    NoI2cTiming {
        bus: String,
        peripheral: String,
        mode: String,
        kernel_hz: u64,
    },
    /// No selectable output speed serves a solved SPI line (`AET-WO-4400b`
    /// D2: the choice moved here from the board generator).
    #[error("net `{net}` on `{pin}` has no output speed: {refusal}")]
    PinSpeed {
        net: String,
        pin: String,
        refusal: crate::speeds::Refusal,
    },
}

fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl SolveError {
    /// The variant name, as the CLI's refusal envelope carries it. An
    /// `InvalidSpecification` reports the validation refusal's own kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidSpecification(e) => e.kind(),
            Self::PartMismatch { .. } => "PartMismatch",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::ReservedPin { .. } => "ReservedPin",
            Self::AnalogPadPin { .. } => "AnalogPadPin",
            Self::UnknownPeripheral { .. } => "UnknownPeripheral",
            Self::PinClaimedTwice { .. } => "PinClaimedTwice",
            Self::PeripheralClaimedTwice { .. } => "PeripheralClaimedTwice",
            Self::NoInstance { .. } => "NoInstance",
            Self::Infeasible { .. } => "Infeasible",
            Self::BudgetExceeded { .. } => "BudgetExceeded",
            Self::NoFreePin { .. } => "NoFreePin",
            Self::NoAdcPin { .. } => "NoAdcPin",
            Self::AdcChannelNotOnPin { .. } => "AdcChannelNotOnPin",
            Self::AdcUnmodelled { .. } => "AdcUnmodelled",
            Self::BridgeSplitAcrossTimers { .. } => "BridgeSplitAcrossTimers",
            Self::UnheldDuringReset { .. } => "UnheldDuringReset",
            Self::NoTimerChannel { .. } => "NoTimerChannel",
            Self::TimerRateConflict { .. } => "TimerRateConflict",
            Self::TimerResolutionConflict { .. } => "TimerResolutionConflict",
            Self::MotorsInfeasible { .. } => "MotorsInfeasible",
            Self::SerialPinsSplit { .. } => "SerialPinsSplit",
            Self::NoSerialPeripheral { .. } => "NoSerialPeripheral",
            Self::SerialPortsInfeasible { .. } => "SerialPortsInfeasible",
            Self::SerialClock { .. } => "SerialClock",
            Self::SpiKernelClockUnavailable { .. } => "SpiKernelClockUnavailable",
            Self::SpiSpeedBelowKernelRange { .. } => "SpiSpeedBelowKernelRange",
            Self::NoI2cTiming { .. } => "NoI2cTiming",
            Self::PinSpeed { refusal, .. } => match refusal {
                crate::speeds::Refusal::SupplyOutsideTable { .. } => "SupplyOutsideSpeedTable",
                crate::speeds::Refusal::LoadAboveTable { .. } => "LoadAboveSpeedTable",
                crate::speeds::Refusal::TooFastForPin { .. } => "RateAbovePinSpeed",
                crate::speeds::Refusal::RiseFasterThanPin { .. } => "RiseTimeBelowPinSpeed",
            },
        }
    }

    /// The one offending pin, peripheral, bus or part, or `None`.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::InvalidSpecification(e) => e.id(),
            Self::PartMismatch { spec, .. } => Some(spec),
            Self::UnknownPin { pin, .. }
            | Self::ReservedPin { pin, .. }
            | Self::AnalogPadPin { pin, .. }
            | Self::PinClaimedTwice { pin, .. } => Some(pin),
            Self::UnknownPeripheral { peripheral, .. }
            | Self::PeripheralClaimedTwice { peripheral, .. } => Some(peripheral),
            Self::NoInstance { bus, .. }
            | Self::BudgetExceeded { bus, .. }
            | Self::SpiKernelClockUnavailable { bus, .. }
            | Self::SpiSpeedBelowKernelRange { bus, .. }
            | Self::NoI2cTiming { bus, .. } => Some(bus),
            Self::NoAdcPin { net, .. }
            | Self::AdcChannelNotOnPin { net, .. }
            | Self::AdcUnmodelled { net, .. }
            | Self::PinSpeed { net, .. } => Some(net),
            Self::BridgeSplitAcrossTimers { bridge, .. } => Some(bridge),
            Self::UnheldDuringReset { net, .. } => Some(net),
            Self::NoTimerChannel { output, .. } => Some(output),
            Self::TimerRateConflict { timer, .. } | Self::TimerResolutionConflict { timer, .. } => {
                Some(timer)
            }
            Self::SerialPinsSplit { port, .. }
            | Self::NoSerialPeripheral { port, .. }
            | Self::SerialClock { port, .. } => Some(port),
            Self::Infeasible { .. }
            | Self::NoFreePin { .. }
            | Self::MotorsInfeasible { .. }
            | Self::SerialPortsInfeasible { .. } => None,
        }
    }

    /// Who is in conflict, in document order. Empty for the refusals that
    /// are about the whole specification.
    pub fn claimants(&self) -> Vec<String> {
        match self {
            Self::InvalidSpecification(e) => e.claimants(),
            Self::PartMismatch { .. } => Vec::new(),
            Self::UnknownPin { claimant, .. }
            | Self::ReservedPin { claimant, .. }
            | Self::AnalogPadPin { claimant, .. }
            | Self::UnknownPeripheral { claimant, .. }
            | Self::NoFreePin { claimant, .. } => vec![claimant.clone()],
            Self::PinClaimedTwice { first, second, .. }
            | Self::PeripheralClaimedTwice { first, second, .. } => {
                vec![first.clone(), second.clone()]
            }
            Self::NoInstance { bus, .. }
            | Self::BudgetExceeded { bus, .. }
            | Self::SpiKernelClockUnavailable { bus, .. }
            | Self::SpiSpeedBelowKernelRange { bus, .. }
            | Self::NoI2cTiming { bus, .. } => vec![bus.clone()],
            Self::NoAdcPin { net, .. }
            | Self::AdcChannelNotOnPin { net, .. }
            | Self::AdcUnmodelled { net, .. }
            | Self::PinSpeed { net, .. } => vec![net.clone()],
            Self::BridgeSplitAcrossTimers { first, second, .. } => {
                vec![first.clone(), second.clone()]
            }
            Self::UnheldDuringReset { net, .. } => vec![net.clone()],
            Self::Infeasible { buses } => buses.clone(),
            Self::NoTimerChannel { output, .. } => vec![output.clone()],
            Self::TimerRateConflict { first, second, .. }
            | Self::TimerResolutionConflict { first, second, .. } => {
                vec![first.clone(), second.clone()]
            }
            Self::MotorsInfeasible { claimants } | Self::SerialPortsInfeasible { claimants } => {
                claimants.clone()
            }
            Self::SerialPinsSplit { port, .. }
            | Self::NoSerialPeripheral { port, .. }
            | Self::SerialClock { port, .. } => vec![port.clone()],
        }
    }
}

/// [`solve_with_limit`] with [`NODE_LIMIT`].
pub fn solve(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
) -> Result<SolvedTopology, SolveError> {
    solve_with_limit(spec, table, specification_sha256, NODE_LIMIT)
}

/// The solver. `specification_sha256` is the sha256 of the specification's
/// bytes as its caller read them (`INV-03`): the solver never sees the bytes.
/// `node_limit` exists so a test can reach [`SolveError::BudgetExceeded`];
/// every other caller uses [`solve`].
pub fn solve_with_limit(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
    node_limit: u64,
) -> Result<SolvedTopology, SolveError> {
    let mut budget = Budget {
        spent: 0,
        limit: node_limit,
    };
    solve_inner(spec, table, specification_sha256, &mut budget, true)
}

/// The solver proper. `forced` switches the forced-instance check
/// (`AET-WO-1201`); it is `false` only in the differential test, and `budget`
/// is a parameter so that test can read the nodes spent.
fn solve_inner(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
    budget: &mut Budget,
    forced: bool,
) -> Result<SolvedTopology, SolveError> {
    spec.validate().map_err(SolveError::InvalidSpecification)?;
    let reserved = all_reserved(spec);
    let pads = analog_pads(&spec.mcu.part).map_err(SolveError::InvalidSpecification)?;
    let mut out = SolvedTopology {
        schema_version: crate::SOLVED_SCHEMA_VERSION,
        specification_sha256: specification_sha256.to_owned(),
        assignments: BTreeMap::new(),
        gpio: BTreeMap::new(),
        adc_inputs: BTreeMap::new(),
        i2c_ceilings: BTreeMap::new(),
        i2c_speeds: BTreeMap::new(),
        analog_lines: BTreeMap::new(),
        i2c_devices: BTreeMap::new(),
        spi_devices: BTreeMap::new(),
        spi_lines: BTreeMap::new(),
        clock: solved_clock(&spec.mcu).map_err(SolveError::InvalidSpecification)?,
        motor_outputs: BTreeMap::new(),
        power: None,
        serial_ports: BTreeMap::new(),
        spi_controllers: BTreeMap::new(),
        i2c_timings: BTreeMap::new(),
        bor_level: spec.mcu.bor_level,
        core_supply: spec.mcu.core_supply.mode,
        mcu: spec.mcu.part.clone(),
        pin_speeds: BTreeMap::new(),
        foreign_clock: spec.mcu.foreign_clock.as_ref().map(Into::into),
        pin_devices: spec
            .declared_pin_devices()
            .iter()
            .map(|d| {
                (
                    d.reference.clone(),
                    crate::pin_devices::SolvedPinDevice {
                        part: d.part.clone(),
                        bridges: crate::pin_devices::solved_bridges(d),
                        supplies: d.supplies.clone(),
                    },
                )
            })
            .collect(),
    };
    out.validate().map_err(SolveError::InvalidSpecification)?;
    out.i2c_ceilings = i2c_ceilings(spec);
    out.i2c_speeds = spec
        .i2c_buses
        .iter()
        .map(|b| (b.id.clone(), b.speed_hz))
        .collect();
    let spi_lines = spi_lines(spec);
    if spec.mcu.part != table.part {
        return Err(SolveError::PartMismatch {
            spec: spec.mcu.part.clone(),
            table: table.part.clone(),
        });
    }

    let timing = motor_timing(spec, &out.clock).map_err(SolveError::InvalidSpecification)?;
    let model = Model::new(spec, table)?;
    model.check_motors(budget)?;
    model.check_serial(budget)?;
    let all: Vec<usize> = (0..model.buses.len()).collect();
    let mut search = Search::new(&model, all.clone(), forced);
    let found = search.run(budget);
    if let Some(pos) = search.exceeded {
        return Err(model.budget_error(&search, pos));
    }
    if !found {
        return Err(model.minimal_set(all, budget, forced));
    }

    for (pos, &b) in search.active.iter().enumerate() {
        let bus = &model.buses[b];
        let (inst, pins) = search.chosen[pos].as_ref().expect("every bus placed");
        for (sig, (pin, af)) in bus.signals.iter().zip(pins) {
            out.assignments.insert(
                sig.net.to_owned(),
                PinAssignment {
                    pin: (*pin).to_owned(),
                    peripheral: inst.clone(),
                    signal: format!("{inst}_{}", sig.name),
                    alternate_function: *af,
                },
            );
        }
    }
    // The motor outputs, placed inside the search after the buses: each on
    // its timer channel's pin, with the timing solved for the reset clock,
    // and its pull copied with the bound step 26 judged it against
    // (`AET-WO-4300j` D6, D9: one computation, `offboard::motor_pull_window`).
    let window = match spec.motor_outputs.is_empty() {
        true => None,
        false => Some(
            crate::offboard::motor_pull_window(spec).map_err(SolveError::InvalidSpecification)?,
        ),
    };
    for (i, m) in model.motors.iter().enumerate() {
        let (timer, channel, pin, af) = search.motor_chosen[i].expect("every motor placed");
        out.assignments.insert(
            m.label.to_owned(),
            PinAssignment {
                pin: pin.to_owned(),
                peripheral: format!("TIM{timer}"),
                signal: format!("TIM{timer}_CH{channel}"),
                alternate_function: af,
            },
        );
        let (psc, arr, ccr) = timing[i];
        out.motor_outputs.insert(
            m.label.to_owned(),
            SolvedMotorOutput {
                index: u32::try_from(i).expect("fewer outputs than u32::MAX"),
                protocol: m.protocol,
                pwm: m.pwm.cloned(),
                psc,
                arr,
                ccr,
                pull: {
                    let p = &spec.motor_outputs[i].pull;
                    let w = window.expect("computed when a motor output is declared");
                    crate::motor::SolvedMotorPull {
                        fitment: p.fitment,
                        resistor: p.resistor.reference.clone(),
                        ohms: p.resistor.ohms,
                        link: p.link.reference.clone(),
                        floor_ohms: w.floor_ohms,
                        ceiling_ohms: w.ceiling_ohms,
                    }
                },
            },
        );
    }
    // The serial ports, placed inside the search after the motor outputs:
    // each on its peripheral's TX and RX pins, with its BRR solved for the
    // reset clock and held to the receiver's tolerance.
    for (i, u) in model.serial.iter().enumerate() {
        let (p, (tx, tx_af), (rx, rx_af)) = search.serial_chosen[i].expect("every port placed");
        for (sig, pin, af) in [(&u.tx, tx, tx_af), (&u.rx, rx, rx_af)] {
            out.assignments.insert(
                sig.net.to_owned(),
                PinAssignment {
                    pin: pin.to_owned(),
                    peripheral: p.to_owned(),
                    signal: format!("{p}_{}", sig.name),
                    alternate_function: af,
                },
            );
        }
        let declared = &spec.serial_ports[i];
        let port = solved_port(u.label, i, p, declared, &out.clock)?;
        out.serial_ports.insert(u.label.to_owned(), port);
    }
    // Each SPI controller's kernel clock and baud rate, bus by bus in
    // document order, at the lowest speed of the buses on it; each I2C
    // controller's kernel clock, CR1 and TIMINGR words (`AET-WO-4230`).
    for bus in &spec.spi_buses {
        let p = &out.assignments[&bus.sck.net].peripheral;
        let declared = spec
            .spi_buses
            .iter()
            .filter(|b| out.assignments[&b.sck.net].peripheral == *p)
            .map(|b| b.speed_hz)
            .min()
            .expect("this bus is on it");
        let c = spi_controller(&bus.id, p, declared, &out.clock)?;
        out.spi_controllers.insert(p.clone(), c);
    }
    // `per_ck` is selected away from `SPI123SEL`'s reset input `pll1_q_ck`,
    // so PLL1 runs from the HSI across the select (`AET-WO-4240e` D1 (a)).
    if out.clock.hse.is_none()
        && out
            .spi_controllers
            .values()
            .any(|c| c.kernel == SpiKernel::PerCk)
    {
        let [pllckselr, pllcfgr, pll1divr] =
            hsi_switch_words().expect("the HSI's band has a PLL1 setting (unit test)");
        out.clock.kernel_switch = Some(crate::clock::SolvedKernelSwitch {
            pllckselr,
            pllcfgr,
            pll1divr,
        });
    }
    for bus in &spec.i2c_buses {
        let p = out.assignments[&bus.sda.net].peripheral.clone();
        let t = i2c_controller(&bus.id, &out.clock, &p)?;
        out.i2c_timings.insert(p, t);
    }
    // Every cs last, in document order: a hand pin is already owned; the rest
    // take the first free, unreserved pin in key order, an FT one at a
    // connector (`AET-WO-4300i` D1). The buses, the motor outputs and the
    // serial ports use a fixed number of pins, so this greedy pass never
    // loses a solution.
    for cs in &model.cs {
        let pin = match cs.hand {
            Some(pin) => pin,
            None => {
                let free = || {
                    table.pins.keys().map(String::as_str).filter(|p| {
                        !reserved.contains(p) && !pads.contains(p) && !search.used.contains_key(p)
                    })
                };
                let Some(pin) = free().find(|p| model.at.offers(cs.net, p)) else {
                    return Err(SolveError::NoFreePin {
                        claimant: cs.path.clone(),
                        withheld: model.at.note(cs.net, free()),
                    });
                };
                search.used.insert(pin, cs.path.clone());
                pin
            }
        };
        out.gpio.insert(
            cs.net.to_owned(),
            crate::pin_devices::SolvedGpio::output(pin, crate::motor::Level::High),
        );
    }
    out.analog_lines = analog_lines(spec);
    out.i2c_devices = i2c_devices(spec, &out);
    out.spi_devices = spi_devices(spec, &out);
    out.spi_lines = spi_lines;
    // `validate` ran every power check; this derives what they found.
    out.power = crate::power::check(spec)
        .map_err(SolveError::InvalidSpecification)?
        .map(Into::into);
    // Each battery sense net takes the free pin of lowest package position
    // that carries a single-ended ADC input (`AET-WO-4300d2` §1.7), after
    // every pin above is owned, so it never takes one of theirs. It is in
    // `power.sense`, not `assignments`: an analog pin has no alternate function.
    let mut sense_inputs = BTreeMap::new();
    if let (Some(solved), Some(power)) = (out.power.as_mut(), spec.power.as_ref()) {
        let mut candidates: Vec<(&str, &crate::Pin)> = table
            .pins
            .iter()
            .filter(|(name, pin)| {
                !pin.adc_channels.is_empty() && !reserved.contains(&name.as_str())
            })
            .map(|(name, pin)| (name.as_str(), pin))
            .collect();
        candidates.sort_by_key(|(_, pin)| pin.position);
        for s in &power.sense {
            let free: Vec<(&str, &crate::Pin)> = candidates
                .iter()
                .filter(|(name, _)| !search.used.contains_key(name))
                .copied()
                .collect();
            // ADC1 or ADC3 (`AET-WO-1211a`, `1211` Q8): a sense is never
            // pinned, so an ADC2-only pin is never its.
            let modelled = free
                .iter()
                .find(|(_, pin)| {
                    crate::pin_devices::choose_channel(&pin.adc_channels, false).is_some()
                })
                .copied();
            let Some((name, pin)) = modelled else {
                if free.is_empty() {
                    return Err(SolveError::NoAdcPin {
                        net: s.net.clone(),
                        withheld: String::new(),
                    });
                }
                return Err(SolveError::AdcUnmodelled {
                    net: s.net.clone(),
                    pins: free.iter().map(|(n, _)| (*n).to_owned()).collect(),
                    withheld: String::new(),
                });
            };
            let (peripheral, channel) =
                crate::pin_devices::choose_channel(&pin.adc_channels, false).expect("found above");
            sense_inputs.insert(
                s.net.clone(),
                crate::pin_devices::SolvedAdcInput {
                    pin: name.to_owned(),
                    peripheral,
                    channel,
                    bits: None,
                    transfer: None,
                },
            );
            search.used.insert(name, format!("power.sense {}", s.net));
            solved.sense.insert(
                s.net.clone(),
                crate::SolvedSense {
                    pin: name.to_owned(),
                    position: pin.position,
                    channels: pin.adc_channels.iter().cloned().collect(),
                },
            );
        }
    }
    out.adc_inputs = sense_inputs;
    // Each pin device's ADC inputs, then its signals, after every pin above
    // is owned, so none of theirs moves (`AET-WO-1211a`).
    crate::pin_devices::place(
        spec,
        table,
        &reserved,
        &pads,
        &mut search.used,
        &mut out.gpio,
        &mut out.adc_inputs,
    )?;
    crate::pin_devices::reset_held(spec, &out)?;
    // The ADCs a pin device's input enables (a power sense is not converted
    // by the bring-up) convert on `adc_sclk` / 4, `CKMODE` `11` (`1211b`;
    // `AET-WO-4240e`, the ruling on the ADC): `ADCSEL`'s reset input,
    // `pll2_p_ck`, never runs.
    let sensed: BTreeSet<&String> = out.power.iter().flat_map(|p| p.sense.keys()).collect();
    if out.adc_inputs.keys().any(|net| !sensed.contains(net)) {
        out.clock.adc = Some(crate::clock::SolvedAdcClock {
            ckmode: ADC_CKMODE_SCLK_DIV4,
        });
    }
    // An IN/IN bridge's two motor outputs share one timer (`1211` §2.7 (e)).
    for (bridge, first, second) in crate::pin_devices::in_in_motor_pairs(spec) {
        let timer = |l: &str| out.assignments[l].peripheral.clone();
        let (first_timer, second_timer) = (timer(&first), timer(&second));
        if first_timer != second_timer {
            return Err(SolveError::BridgeSplitAcrossTimers {
                bridge,
                first,
                second,
                first_timer,
                second_timer,
            });
        }
    }
    out.pin_speeds = pin_speeds(&out)?;
    // belt-and-suspenders: the search already keeps every rule `validate`
    // checks; this refuses rather than emits if a change to it ever breaks one.
    out.validate().map_err(SolveError::InvalidSpecification)?;
    Ok(out)
}

/// Each solved pin's output speed (`AET-WO-4400b` D2): the slowest the
/// output speed table allows for a line of `spi_lines` (each bus's SCK and
/// MOSI, at its declared rate, supply, load and rise time), and `0` for every
/// other pin, which the speed check does not judge (`AET-WO-4300c` D1, D3,
/// D4). A line no selectable speed serves is refused by name.
fn pin_speeds(t: &SolvedTopology) -> Result<BTreeMap<String, u8>, SolveError> {
    let mut out = BTreeMap::new();
    for (net, pin) in solved_pins(t) {
        out.insert(net.clone(), pin_speed(t, net, pin)?);
    }
    Ok(out)
}

/// Every solved pin, `assignments` then `gpio`, as (net, pin).
pub(crate) fn solved_pins(t: &SolvedTopology) -> impl Iterator<Item = (&String, &String)> {
    t.assignments
        .iter()
        .map(|(net, a)| (net, &a.pin))
        .chain(t.gpio.iter().map(|(net, g)| (net, &g.pin)))
}

/// One pin's speed, as [`pin_speeds`] gives it.
pub(crate) fn pin_speed(t: &SolvedTopology, net: &str, pin: &str) -> Result<u8, SolveError> {
    let Some(line) = t.spi_lines.get(net) else {
        return Ok(0);
    };
    let table = crate::speeds::table().map_err(SolveError::InvalidSpecification)?;
    table
        .choose(
            pin,
            line.rate_hz,
            line.vdd_min_millivolts,
            line.capacitance_femtofarads,
            line.rise_time_ns,
        )
        .map(|c| c.ospeedr)
        .map_err(|refusal| SolveError::PinSpeed {
            net: net.to_owned(),
            pin: pin.to_owned(),
            refusal,
        })
}

/// Serial port `label`, placed on `peripheral`: its kernel clock, then
/// [`port_on`], each refused by name.
fn solved_port(
    label: &str,
    index: usize,
    peripheral: &str,
    declared: &crate::SerialPort,
    clock: &SolvedClock,
) -> Result<SolvedSerialPort, SolveError> {
    let kernel = serial::kernel_hz(clock, peripheral).ok_or_else(|| SolveError::SerialClock {
        port: label.to_owned(),
        peripheral: peripheral.to_owned(),
    })?;
    port_on(label, index, peripheral, kernel, declared, clock)
        .map_err(SolveError::InvalidSpecification)
}

/// `validate`'s step 21 for serial port `label` (`AET-WO-1208` §1.3): its
/// `BRR` and worst-case baud error on `clock`, which gives every USART and
/// UART one kernel clock, so the verdict and the message are the same
/// whichever peripheral the search later picks.
pub(crate) fn serial_timing(
    label: &str,
    index: usize,
    clock: &SolvedClock,
    declared: &crate::SerialPort,
) -> Result<(), IrValidationError> {
    let kernels: BTreeSet<u64> = serial::PERIPHERALS
        .iter()
        .filter_map(|p| serial::kernel_hz(clock, p))
        .collect();
    assert_eq!(
        kernels.len(),
        1,
        "the reset clock gives every USART and UART one kernel clock"
    );
    let (peripheral, kernel) = (
        serial::PERIPHERALS[0],
        kernels.into_iter().next().unwrap_or(0),
    );
    port_on(label, index, peripheral, kernel, declared, clock).map(|_| ())
}

/// Serial port `label` on `peripheral` at `kernel` Hz: its `BRR`
/// ([`serial::solve_brr`]) and the static check ([`serial::check_silicon`]),
/// each refused by name.
fn port_on(
    label: &str,
    index: usize,
    peripheral: &str,
    kernel: u64,
    declared: &crate::SerialPort,
    clock: &SolvedClock,
) -> Result<SolvedSerialPort, IrValidationError> {
    let baud = declared.framing.baud;
    let brr =
        serial::solve_brr(kernel, baud).ok_or_else(|| IrValidationError::SerialBaudOutOfRange {
            port: label.to_owned(),
            baud,
            kernel_hz: kernel,
        })?;
    let port = SolvedSerialPort {
        index: u32::try_from(index).expect("fewer ports than u32::MAX"),
        peripheral: peripheral.to_owned(),
        tx: declared.tx.net.clone(),
        rx: declared.rx.net.clone(),
        role: declared.role,
        framing: declared.framing.clone(),
        negotiated: declared.negotiated.clone(),
        peer_deviation_ppm: declared.peer_deviation_ppm,
        kernel_hz: kernel,
        brr,
        actual_millibaud: serial::actual_millibaud(kernel, brr),
        error_ppm: serial::error_ppm(kernel, brr, baud),
    };
    // The worst case, not the quantization alone: HSI's deviation and the
    // peer's declared one sum with it (`AET-WO-4230` §1.8).
    let check = serial::check_silicon(clock, &port);
    if !check.holds {
        let (row, tolerance_ppm) = check.tolerance.map_or_else(
            || ("no row of the table".to_owned(), 0),
            |r| {
                (
                    format!(
                        "{}, {}, M = {:02b}",
                        serial::TOLERANCE_SOURCE,
                        r.table,
                        r.m_bits
                    ),
                    r.tolerance_ppm,
                )
            },
        );
        return Err(IrValidationError::SerialBaudTolerance {
            port: label.to_owned(),
            row,
            quantization_ppm: check.quantization_ppm,
            hsi_ppm: check.hsi_ppm,
            receiver_ppm: check.receiver_ppm,
            peer_deviation_ppm: check.peer_deviation_ppm,
            worst_case_ppm: check.worst_case_ppm,
            tolerance_ppm,
        });
    }
    Ok(port)
}

/// The clock the firmware runs on (`AET-WO-4240a`): the reset clock when
/// `mcu.hse` is `null`; otherwise H1 (`4240` D1, ruled on #240), PLL1 from
/// the HSE to `sys_ck` 64 MHz with every prescaler 1 at VOS Scale 2, its
/// words chosen by [`h1_words`] and its budget [`crate::clock::declared_budget`]'s.
/// Every kernel the reset tree runs at 64 MHz runs at 64 MHz here, from the
/// HSE. Refused by name: the declaration's own refusals, and a source no
/// setting reaches 64 MHz from (`HseTreeUnreachable`). `validate` calls it,
/// as it calls the timing checks, and [`crate::clock::judge_tree`] judges
/// what it returns.
pub fn solved_clock(mcu: &crate::Mcu) -> Result<SolvedClock, IrValidationError> {
    match &mcu.hse {
        None => Ok(SolvedClock::reset()),
        Some(hse) => hse_clock(hse),
    }
}

/// [`solved_clock`] for a declared HSE. `SolvedTopology::validate` re-solves a
/// topology's own copy of the declaration with it (`AET-WO-4240a` F3), so a
/// path that reads only a topology refuses a derived field edited away from
/// its declaration (`HseCopyNotSolved`).
pub(crate) fn hse_clock(hse: &crate::clock::Hse) -> Result<SolvedClock, IrValidationError> {
    use crate::clock::{HseRefusal, SolvedHse, declared_budget, source_hz};
    let budget = declared_budget(hse)?;
    let (num, den) = source_hz(hse)?;
    let unreachable = || {
        IrValidationError::Hse(HseRefusal::HseTreeUnreachable {
            hz_num: num,
            hz_den: den,
        })
    };
    let [pllckselr, pllcfgr, pll1divr] = h1_words(num, den).ok_or_else(unreachable)?;
    // A reachable source is a whole number of Hz: its reference is.
    let hse_hz = u32::try_from(num / den).map_err(|_| unreachable())?;
    Ok(SolvedClock {
        source: "PLL1".into(),
        sysclk_hz: H1_SYS_HZ as u32,
        hse_hz,
        hse: Some(SolvedHse {
            declared: hse.clone(),
            fast_ppm: budget.fast_ppm,
            slow_ppm: budget.slow_ppm,
            pllckselr,
            pllcfgr,
            pll1divr,
            flash_acr: flash_acr_vos2(budget.fastest_hz(H1_SYS_HZ)).ok_or_else(unreachable)?,
        }),
        ..SolvedClock::reset()
    })
}

/// `FLASH_ACR` for H1 (`AET-WO-4240b` Q6, ruled): `LATENCY` and
/// `WRHIGHFREQ` of the least row of RM0433 Rev 9 Table 17 (p. 160) for VOS
/// Scale 2 whose upper `rcc_aclk` bound is at or above `hclk_fastest_hz`
/// (every prescaler is 1 under H1, so `hclk` = `sys_ck`). The solver's own
/// copy of the Scale 2 rows; the rule's is `aethelgard_ir::vos`'s
/// (`RUL-20`). `None` past the table.
fn flash_acr_vos2(hclk_fastest_hz: u64) -> Option<u32> {
    const ROWS: [(u64, u32, u32); 4] = [
        (55_000_000, 0, 0b00),
        (110_000_000, 1, 0b01),
        (165_000_000, 2, 0b01),
        (225_000_000, 3, 0b10),
    ];
    ROWS.iter()
        .find(|(max, _, _)| hclk_fastest_hz <= *max)
        .map(|(_, latency, wrhighfreq)| wrhighfreq << 4 | latency)
}

/// `ADCx_CCR.CKMODE` `11`: `adc_sclk` / 4 (RM0433 Rev 9 p. 1039), the
/// synchronous clock every generated ADC converts on (`AET-WO-1211b`).
const ADC_CKMODE_SCLK_DIV4: u32 = 0b11;

/// H1's `sys_ck` (`4240` D1): 64 MHz, so every kernel runs where the reset
/// tree's does.
const H1_SYS_HZ: u64 = 64_000_000;

/// The solver's choice of PLL1 words for H1 from a source of `num`/`den` Hz
/// (`AET-WO-4240a` §1.6; predictions P6), `[PLLCKSELR, PLLCFGR, PLL1DIVR]`:
/// - `DIVM1` the least, 1 to 63, whose reference is a whole number of Hz
///   strictly inside one `PLL1RGE` range (1–2, 2–4, 4–8, 8–16 MHz; `4240`
///   F-RGE, and Renode divides in integers, `4240` §6);
/// - `PLL1VCOSEL` medium (150–420 MHz) for a reference under 2 MHz, else
///   wide (192–836 MHz, the stricter silicon revision);
/// - `DIVP1` the least even divisor, 2 to 128, whose VCO, 64 MHz × `DIVP1`,
///   is in that range and a whole `DIVN1` (4 to 512) times the reference;
/// - `DIVQ1` = `DIVP1`, so `pll1_q_ck` is 64 MHz for `4240b`'s SPI select;
///   `DIVR1` at its reset 1 with `DIVR1EN` 0 (RM0433 Rev 9 p. 403: 0 is "not
///   allowed"); `DIVM2` = `DIVM3` = 0 and every PLL2/PLL3 enable cleared.
///
/// RM0433 Rev 9 §8.7.11–§8.7.13, pp. 399–404. `None` when no setting
/// exists. This is the solver's; the judge shares none of it (`RUL-20`).
pub(crate) fn h1_words(num: u64, den: u64) -> Option<[u32; 3]> {
    const RANGES: [(u64, u64); 4] = [
        (1_000_000, 2_000_000),
        (2_000_000, 4_000_000),
        (4_000_000, 8_000_000),
        (8_000_000, 16_000_000),
    ];
    for m in 1..=63u64 {
        let d = den.checked_mul(m)?;
        if !num.is_multiple_of(d) {
            continue;
        }
        let reference = num / d;
        let Some(rge) = RANGES
            .iter()
            .position(|&(lo, hi)| lo < reference && reference < hi)
        else {
            continue;
        };
        let medium = rge == 0;
        let (lo, hi) = if medium {
            (150_000_000, 420_000_000)
        } else {
            (192_000_000, 836_000_000)
        };
        for p in (2..=128u64).step_by(2) {
            let vco = H1_SYS_HZ * p;
            if vco < lo || vco > hi || !vco.is_multiple_of(reference) {
                continue;
            }
            let n = vco / reference;
            if !(4..=512).contains(&n) {
                continue;
            }
            let w = |v: u64| u32::try_from(v).ok();
            let (m, n, p, rge) = (w(m)?, w(n)?, w(p)?, w(rge as u64)?);
            return Some([
                0b10 | m << 4,
                1 << 17 | 1 << 16 | rge << 2 | u32::from(medium) << 1,
                1 << 24 | (p - 1) << 16 | (p - 1) << 9 | (n - 1),
            ]);
        }
    }
    None
}

/// The solver's choice of PLL1 words for the kernel-clock switch on a board
/// with no HSE tree (`AET-WO-4240e` D1 (a)), `[PLLCKSELR, PLLCFGR,
/// PLL1DIVR]`, from the HSI across its whole budget ([`crate::clock::Budget::HSI`],
/// 62,342,784–64,958,976 Hz). PLL1's reset words do not serve: `DIVM1` 32
/// gives a 2 MHz reference, the wide VCO's floor (DS12117 Rev 10 Table 39,
/// p. 129), which the HSI's slow side falls under.
/// - `PLLSRC` `00`, `hsi_ck` (RM0433 Rev 9 p. 399); `DIVM2` = `DIVM3` = 0;
/// - `DIVM1` the least, 1 to 63, whose reference across the band lies
///   strictly inside one `PLL1RGE` range at or above 2 MHz (2–4, 4–8,
///   8–16 MHz, p. 402), the wide VCO's input range;
/// - `PLL1VCOSEL` wide (0), `PLL1FRACEN` 0; `DIVN1` the least, 4 to 512,
///   whose VCO across the band lies in 192–836 MHz (Table 39, the stricter
///   silicon revision);
/// - `DIVQ1` the least, 1 to 128, whose `pll1_q_ck` at the band's top is at
///   most 200 MHz, VOS Scale 3's ceiling (Table 39, p. 130), so it holds at
///   any scale; `DIVQ1EN` the only output enable; `DIVP1` and `DIVR1` at
///   their reset 1 (÷2; 0 is "not allowed", p. 403).
///
/// `None` when no setting exists. The solver's; the rule shares none of it
/// (`RUL-20`).
pub(crate) fn hsi_switch_words() -> Option<[u32; 3]> {
    const RANGES: [(u64, u64, u32); 3] = [
        (2_000_000, 4_000_000, 0b01),
        (4_000_000, 8_000_000, 0b10),
        (8_000_000, 16_000_000, 0b11),
    ];
    let budget = crate::clock::Budget::HSI;
    let (slow, fast) = (budget.slowest_hz(64_000_000), budget.fastest_hz(64_000_000));
    for m in 1..=63u64 {
        let (lo_ref, hi_ref) = (slow / m, fast.div_ceil(m));
        let Some(&(_, _, rge)) = RANGES
            .iter()
            .find(|&&(lo, hi, _)| lo < lo_ref && hi_ref < hi)
        else {
            continue;
        };
        let Some(n) =
            (4..=512u64).find(|n| slow * n / m >= 192_000_000 && fast * n / m < 836_000_000)
        else {
            continue;
        };
        let q = (1..=128u64).find(|q| (fast * n).div_ceil(m * q) <= 200_000_000)?;
        let w = |v: u64| u32::try_from(v).ok();
        let (m, n, q) = (w(m)?, w(n)?, w(q)?);
        return Some([
            m << 4,
            1 << 17 | rge << 2,
            1 << 24 | (q - 1) << 16 | 1 << 9 | (n - 1),
        ]);
    }
    None
}

/// The kernel clock the firmware gives SPI `peripheral`: SPI1–3
/// `per_ck` (`SPI123SEL` = `100`, RM0433 Rev 9 §8.7.20, p. 414), which is
/// `hsi_ker_ck` at `CKPERSEL`'s reset `00` (§8.7.19, p. 412); SPI4/5
/// `hsi_ker_ck` (`SPI45SEL` = `011`, p. 414); SPI6 `hsi_ker_ck` (`SPI6SEL` =
/// `011`, §8.7.22, p. 418). `hsi_ker_ck` is HSI after `HSIDIV` (Figure 45,
/// p. 338): 64 MHz at `HSIDIV`'s reset, whatever `SW` selects, since the
/// bring-up never stops the HSI. An HSE board keeps them until `4240b`
/// moves every select to its reset value (`4240` D6). `None` when
/// `peripheral` is not SPI1–SPI6.
pub(crate) fn spi_kernel(peripheral: &str, clock: &SolvedClock) -> Option<(SpiKernel, u64)> {
    let hsi = 64_000_000;
    match (peripheral, clock.hse.is_some()) {
        ("SPI1" | "SPI2" | "SPI3", false) => Some((SpiKernel::PerCk, hsi)),
        ("SPI4" | "SPI5" | "SPI6", false) => Some((SpiKernel::HsiKerCk, hsi)),
        // `4240` D6 (ruled), `AET-WO-4240b` Q3: every select at reset under
        // H1, whose `DIVQ1` = `DIVP1` and every prescaler 1, so `pll1_q_ck`
        // and the APB clocks are `sys_ck`, 64 MHz ([`h1_words`]).
        ("SPI1" | "SPI2" | "SPI3", true) => Some((SpiKernel::Pll1Q, H1_SYS_HZ)),
        ("SPI4" | "SPI5" | "SPI6", true) => Some((SpiKernel::Pclk, H1_SYS_HZ)),
        _ => None,
    }
}

/// The budget the solver sizes `kernel` by: the HSI's for the HSI-derived
/// kernels (which run whatever `SW` selects, `4240a` D-9), the clock's own
/// for the kernels H1 derives from the HSE.
fn kernel_budget(kernel: SpiKernel, clock: &SolvedClock) -> PpmBudget {
    match kernel {
        SpiKernel::PerCk | SpiKernel::HsiKerCk => PpmBudget::HSI,
        SpiKernel::Pll1Q | SpiKernel::Pclk => clock.budget(),
    }
}

/// `MBR`: the least of 0 to 7 with the kernel at its fastest (`budget`)
/// divided by `2^(MBR+1)` at or below `declared_hz` (RM0433 Rev 9 §50.11.3,
/// p. 2218, `000` ÷2 to `111` ÷256); `None` when even ÷256 is above it.
pub(crate) fn solve_mbr(kernel_hz: u64, declared_hz: u32, budget: PpmBudget) -> Option<u8> {
    let fast = budget.fastest_hz(kernel_hz);
    (0..=7u8).find(|mbr| fast >> (mbr + 1) <= u64::from(declared_hz))
}

/// SPI bus `bus`'s controller `peripheral` at `declared_hz`, each refusal by
/// name.
pub(crate) fn spi_controller(
    bus: &str,
    peripheral: &str,
    declared_hz: u32,
    clock: &SolvedClock,
) -> Result<SolvedSpiController, SolveError> {
    let (kernel, kernel_hz) =
        spi_kernel(peripheral, clock).ok_or_else(|| SolveError::SpiKernelClockUnavailable {
            bus: bus.to_owned(),
            peripheral: peripheral.to_owned(),
        })?;
    let budget = kernel_budget(kernel, clock);
    let fast = budget.fastest_hz(kernel_hz);
    let mbr = solve_mbr(kernel_hz, declared_hz, budget).ok_or_else(|| {
        SolveError::SpiSpeedBelowKernelRange {
            bus: bus.to_owned(),
            peripheral: peripheral.to_owned(),
            speed_hz: declared_hz,
            kernel_hz,
            fast_hz: fast,
            floor_hz: fast >> 8,
        }
    })?;
    Ok(SolvedSpiController {
        kernel,
        kernel_hz,
        declared_hz,
        mbr,
        achieved_hz: kernel_hz >> (mbr + 1),
        achieved_max_hz: fast >> (mbr + 1),
    })
}

/// The kernel clock of I2C `peripheral` on `clock`: `I2C123SEL` at its
/// reset `00` is `rcc_pclk1` (RM0433 Rev 9 §8.7.21, p. 416), `I2C4SEL` at
/// `00` `rcc_pclk4` (§8.7.22, p. 420); `rcc_pclk1` is `rcc_hclk1 / D2PPRE1`,
/// `rcc_pclk4` `rcc_hclk4 / D3PPRE` (Figure 49, p. 351).
pub(crate) fn i2c_kernel_hz(clock: &SolvedClock, peripheral: &str) -> Option<u64> {
    let ppre = match peripheral {
        "I2C1" | "I2C2" | "I2C3" => clock.d2ppre1,
        "I2C4" => clock.d3ppre,
        _ => return None,
    };
    Some(clock.hclk_hz() / u64::from(ppre.max(1)))
}

/// `I2C_CR1` as the bring-up writes it, `PE` clear: every field 0, so the
/// analog filter is on (`ANFOFF` 0), `DNF` is 0, every interrupt and DMA
/// enable is off and `NOSTRETCH` is 0, as controller mode requires (RM0433
/// Rev 9 §47.9.1, p. 2001–2002).
pub(crate) const I2C_CR1: u32 = 0;

/// One I2C mode as the synthesis needs it, ns and Hz: `(name, f_SCL, tLOW,
/// tHIGH, tr(max), tf(max), tSU;DAT(min), tHD;DAT(min), least kernel)`, from
/// RM0433 Rev 9 Table 390 (p. 1973) and Table 388 (p. 1958), and DS12117
/// Rev 10 Table 93 (rev Y) / Table 191 (rev V), analog filter on, `DNF` 0.
/// The solver's own copy.
type I2cMode = (&'static str, u64, u64, u64, u64, u64, u64, u64, u64);
const I2C_MODES: [I2cMode; 3] = [
    (
        "Standard100k",
        100_000,
        4_700,
        4_000,
        1_000,
        300,
        250,
        0,
        2_000_000,
    ),
    ("Fast400k", 400_000, 1_300, 600, 300, 300, 100, 0, 8_000_000),
    (
        "FastPlus1M",
        1_000_000,
        500,
        260,
        120,
        120,
        50,
        0,
        17_000_000,
    ),
];

/// `tAF(min)`, the analog filter's shortest delay: 50 ns (DS12117 Rev 10
/// Table 94 rev Y and Table 192 rev V).
const I2C_AF_MIN_NS: u64 = 50;

/// `TIMINGR` for `mode` on a kernel of `kernel_hz` (RM0433 Rev 9 §47.4.9,
/// p. 1957–1958 and p. 1971; contract D3 and amendment 1), every minimum
/// met at the source's fastest (`budget`). In kernel cycles `c(t) = ⌈t · fast⌉`, with
/// `tSYNC1` and `tSYNC2` two cycles each at least (Tables 391–393 notes):
/// for the least `PRESC` (multiplier `m`) at which every field fits,
/// - `SCLH` the least with `2 + (SCLH+1)·m ≥ c(tHIGH)`;
/// - `SCLL` the least with `2 + (SCLL+1)·m ≥ c(tLOW)` and the shortest period
///   `4 + (SCLL+SCLH+2)·m` at least `⌈fast / f_SCL⌉`: the low phase takes
///   whatever the period needs beyond the two minima;
/// - `SCLDEL` the least with `(SCLDEL+1)·m ≥ c(tr + tSU;DAT)`;
/// - `SDADEL` the least with `SDADEL·m + 3 ≥ c(tf + tHD;DAT − tAF(min))`.
///
/// `tVD;DAT`'s maximum is not a bound here: RM0433 p. 1958, it "can be
/// violated when NOSTRETCH = 0", which [`I2C_CR1`] keeps. `None` for an
/// unknown mode, a kernel below the mode's least at the source's slowest, or
/// no `PRESC` that fits.
pub(crate) fn synthesize_timingr(budget: PpmBudget, kernel_hz: u64, mode: &str) -> Option<u32> {
    let &(_, scl_hz, t_low, t_high, tr, tf, t_su, t_hd, kernel_min) =
        I2C_MODES.iter().find(|m| m.0 == mode)?;
    if budget.slowest_hz(kernel_hz) < kernel_min {
        return None;
    }
    let fast = u128::from(budget.fastest_hz(kernel_hz));
    let cycles = |t_ns: u64| (u128::from(t_ns) * fast).div_ceil(1_000_000_000);
    // The least `n ≥ 0` with `n · m ≥ need`.
    let least = |need: u128, m: u128| need.div_ceil(m);
    let period = fast.div_ceil(u128::from(scl_hz));
    for presc in 0..=15u32 {
        let m = u128::from(presc) + 1;
        let sclh = least(cycles(t_high).saturating_sub(2), m).saturating_sub(1);
        let low = least(cycles(t_low).saturating_sub(2), m);
        let low_for_period = least(period.saturating_sub(4), m).saturating_sub(sclh + 1);
        let scll = low.max(low_for_period).saturating_sub(1);
        let scldel = least(cycles(tr + t_su), m).saturating_sub(1);
        let sdadel = least(
            cycles((tf + t_hd).saturating_sub(I2C_AF_MIN_NS)).saturating_sub(3),
            m,
        );
        if sclh <= 0xFF && scll <= 0xFF && scldel <= 0xF && sdadel <= 0xF {
            let field = |v: u128| u32::try_from(v).expect("checked above");
            return Some(
                presc << 28
                    | field(scldel) << 20
                    | field(sdadel) << 16
                    | field(sclh) << 8
                    | field(scll),
            );
        }
    }
    None
}

/// I2C bus `bus`'s controller `peripheral` on `clock`: its kernel clock, `CR1`
/// and one `TIMINGR` per mode, each refusal by name (`4240` F-PANIC).
pub(crate) fn i2c_controller(
    bus: &str,
    clock: &SolvedClock,
    peripheral: &str,
) -> Result<SolvedI2cTiming, SolveError> {
    let refuse = |mode: &str, kernel_hz: u64| SolveError::NoI2cTiming {
        bus: bus.to_owned(),
        peripheral: peripheral.to_owned(),
        mode: mode.to_owned(),
        kernel_hz,
    };
    let kernel_hz = i2c_kernel_hz(clock, peripheral).ok_or_else(|| refuse("any mode", 0))?;
    let budget = clock.budget();
    let timingr = I2C_MODES
        .iter()
        .map(|&(mode, ..)| {
            synthesize_timingr(budget, kernel_hz, mode)
                .map(|w| (mode.to_owned(), w))
                .ok_or_else(|| refuse(mode, kernel_hz))
        })
        .collect::<Result<_, _>>()?;
    Ok(SolvedI2cTiming {
        kernel_hz,
        cr1: I2C_CR1,
        timingr,
    })
}

/// Each motor output's `PSC`, `ARR` and `CCR` on the clock the firmware runs
/// on, in document order. Every timer's kernel clock is the same on the
/// reset clock (RM0433 Rev 9 Table 57), so the values do not depend on which
/// timer the search picks; they are solved, and refused by name, before it.
pub(crate) fn motor_timing(
    spec: &Specification,
    clock: &SolvedClock,
) -> Result<Vec<(u32, u32, u32)>, IrValidationError> {
    let (k1, k2) = (
        clock.timer_kernel_hz(motor::Apb::Apb1),
        clock.timer_kernel_hz(motor::Apb::Apb2),
    );
    assert_eq!(k1, k2, "the reset clock gives both buses one timer clock");
    spec.motor_outputs
        .iter()
        .map(|m| {
            motor::solve_timing(m.protocol, m.pwm.as_ref(), k1).map_err(|e| {
                let output = m.label.clone();
                let p = m.pwm.as_ref();
                let res = p.map_or(0, |p| p.resolution_ps);
                let tick = |reason| IrValidationError::MotorTiming {
                    output: output.clone(),
                    reason,
                };
                let pwm = |r| IrValidationError::Pwm(r);
                match (e, p) {
                    // At a 1 us tick (every specification before
                    // `AET-WO-1211a`'s) and for DShot, the messages are the
                    // ones they were.
                    (TimingError::RateOutOfRange, _) if res == 1_000_000 || p.is_none() => tick(
                        "its period does not fit a 16-bit ARR (a PWM rate below 16 Hz at a 1 us tick)",
                    ),
                    (TimingError::RateOutOfRange, _) => tick(
                        "its period is not 3 to 65,536 ticks, or its shortest pulse is not at least 1 tick and 2 ticks short of the period",
                    ),
                    (TimingError::PulseExceedsPeriod, _) if res == 1_000_000 => tick(
                        "its longest pulse does not fit inside the period the rate rounds to at a 1 us tick",
                    ),
                    (TimingError::PulseExceedsPeriod, _) => tick(
                        "its longest pulse does not fit inside the period the rate rounds to at its tick",
                    ),
                    (TimingError::ResolutionNotDerivable, _) => {
                        pwm(motor::PwmRefusal::ResolutionNotDerivable {
                            output,
                            resolution_ps: res,
                            kernel_hz: k1,
                        })
                    }
                    (TimingError::PulseNotOnTick, Some(p)) => pwm(motor::PwmRefusal::PulseNotOnTick {
                        output,
                        min_us: p.pulse_min_us,
                        max_us: p.pulse_max_us,
                        resolution_ps: res,
                    }),
                    (TimingError::IdleNotOnTick, Some(p)) => pwm(motor::PwmRefusal::IdleNotOnTick {
                        output,
                        pulse_us: idle_us(p),
                        resolution_ps: res,
                    }),
                    (TimingError::IdleSpelledTwice, Some(p)) => {
                        pwm(motor::PwmRefusal::IdleSpelledTwice {
                            output,
                            pulse_us: idle_us(p),
                            ccr: "ARR + 1",
                            level: "high",
                        })
                    }
                    (TimingError::IdleExceedsPeriod, Some(p)) => {
                        pwm(motor::PwmRefusal::IdleExceedsPeriod {
                            output,
                            pulse_us: idle_us(p),
                            rate_hz: p.rate_hz,
                        })
                    }
                    (_, None) => unreachable!("only a PWM output's timing names a tick"),
                }
            })
        })
        .collect()
}

/// A pulse idle's length; 0 for a level, which no tick refusal names.
fn idle_us(p: &motor::PwmTiming) -> u32 {
    match p.idle {
        motor::Idle::PulseUs(us) => us,
        motor::Idle::Level(_) => 0,
    }
}

/// Each I2C line's analog facts, copied from the specification (`AET-WO-2200`
/// §1.1). `i2c_ceilings` has already refused a line with no constraint, and
/// `validate` a bus on an undeclared domain, so every lookup finds its entry.
fn analog_lines(spec: &Specification) -> BTreeMap<String, AnalogLine> {
    let mut out = BTreeMap::new();
    for bus in &spec.i2c_buses {
        let domain = spec.voltage_domains.iter().find(|d| d.id == bus.domain);
        for net in [&bus.sda.net, &bus.scl.net] {
            let constraint = spec.net_constraints.iter().find(|c| &c.net_name == net);
            let (Some(domain), Some(c)) = (domain, constraint) else {
                continue;
            };
            out.insert(
                net.clone(),
                AnalogLine {
                    bus: bus.id.clone(),
                    pull_up_ohms: bus.pull_up_ohms,
                    capacitance_femtofarads: c.max_capacitance_femtofarads,
                    ceiling_ns: c.max_rise_time_nanoseconds,
                    domain_millivolts: domain.nominal_millivolts,
                },
            );
        }
    }
    out
}

/// Every I2C device of `spec`, on the peripheral the solution gave its bus
/// (the one that carries its `sda` net), with its place in the specification's
/// order (`AET-WO-2203`). The search places every bus, so each device's `sda` net is assigned.
fn i2c_devices(spec: &Specification, solved: &SolvedTopology) -> BTreeMap<String, SolvedI2cDevice> {
    let mut out = BTreeMap::new();
    for (index, d) in spec.i2c_devices.iter().enumerate() {
        let bus = spec
            .i2c_buses
            .iter()
            .find(|b| b.id == d.bus)
            .expect("Specification::validate checked every device's bus");
        let placed = solved
            .assignments
            .get(&bus.sda.net)
            .expect("the search places every bus");
        out.insert(
            d.reference.clone(),
            SolvedI2cDevice {
                index: u32::try_from(index).expect("a specification has under 2^32 devices"),
                part: d.part.clone(),
                bus: d.bus.clone(),
                peripheral: placed.peripheral.clone(),
                address: d.address,
            },
        );
    }
    out
}

/// Every SPI device of `spec`, on the peripheral the solution gave its bus
/// (the one that carries its `sck` net), with its chip select's net and its
/// place in the specification's order (`AET-WO-4200`). The search places every
/// bus and every chip select, so each lookup finds its entry.
fn spi_devices(spec: &Specification, solved: &SolvedTopology) -> BTreeMap<String, SolvedSpiDevice> {
    let mut out = BTreeMap::new();
    for (index, d) in spec.spi_devices.iter().enumerate() {
        let bus = spec
            .spi_buses
            .iter()
            .find(|b| b.id == d.bus)
            .expect("Specification::validate checked every device's bus");
        let placed = solved
            .assignments
            .get(&bus.sck.net)
            .expect("the search places every bus");
        out.insert(
            d.reference.clone(),
            SolvedSpiDevice {
                index: u32::try_from(index).expect("a specification has under 2^32 devices"),
                part: d.part.clone(),
                bus: d.bus.clone(),
                peripheral: placed.peripheral.clone(),
                cs: d.cs.net.clone(),
                orientation: d.orientation,
            },
        );
    }
    out
}

/// Each SPI bus's SCK and MOSI, with what the output speed check reads,
/// copied from the specification in document order (`AET-WO-4300c` D2).
/// Every bus is on the MCU's supply (`validate`'s step 5a), whose minimum
/// selects the VDD band; step 21 held that supply inside the table's bands
/// and refused a line with no constraint: nothing is defaulted (`INV-06`).
fn spi_lines(spec: &Specification) -> BTreeMap<String, SpiLine> {
    let supply = spec
        .voltage_domains
        .iter()
        .find(|d| d.id == spec.mcu.supply)
        .expect("Specification::validate checked the MCU's supply");
    let min = crate::minimum_millivolts(supply);
    let mut out = BTreeMap::new();
    for bus in &spec.spi_buses {
        for (signal, net) in [("SCK", &bus.sck.net), ("MOSI", &bus.mosi.net)] {
            let c = spec
                .net_constraints
                .iter()
                .find(|c| &c.net_name == net)
                .expect("Specification::validate checked every SPI line's constraint");
            out.insert(
                net.clone(),
                SpiLine {
                    bus: bus.id.clone(),
                    signal: signal.to_owned(),
                    rate_hz: bus.speed_hz,
                    capacitance_femtofarads: c.max_capacitance_femtofarads,
                    rise_time_ns: c.max_rise_time_nanoseconds,
                    vdd_min_millivolts: min,
                },
            );
        }
    }
    out
}

/// Each I2C bus's ceiling, from the worse of its two lines' declared
/// `max_rise_time_nanoseconds`, in document order: a mode is legal only when
/// both lines meet its rise time (`AET-WO-1206` §1.7). `validate` has
/// refused a line with no constraint (step 19), and a rise time no mode
/// allows, a speed above its ceiling, a pull-up below its floor and a line
/// over its capacitance limit (step 21).
fn i2c_ceilings(spec: &Specification) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    for bus in &spec.i2c_buses {
        let mut t_max = 0;
        for net in [&bus.sda.net, &bus.scl.net] {
            let c = spec
                .net_constraints
                .iter()
                .find(|c| &c.net_name == net)
                .expect("Specification::validate checked every I2C line's constraint");
            t_max = t_max.max(c.max_rise_time_nanoseconds);
        }
        let ceiling = electrical::ceiling_hz(t_max)
            .expect("Specification::validate checked every I2C bus's rise time (step 21)");
        out.insert(bus.id.clone(), ceiling);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    I2c,
    Spi,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::I2c => "I2C",
            Kind::Spi => "SPI",
        }
    }

    /// The signals of a bus of this kind, in the order they are placed.
    fn signals(self) -> &'static [&'static str] {
        match self {
            Kind::I2c => &["SDA", "SCL"],
            Kind::Spi => &["SCK", "MOSI", "MISO"],
        }
    }
}

/// One signal of a bus, or one `cs`. `hand` is the table's own key for a
/// hand-set pin, so pins are compared as keys, never as strings a person
/// wrote (`AET-WO-1100` FEEDBACK A5).
struct Sig<'s, 't> {
    name: &'static str,
    net: &'s str,
    hand: Option<&'t str>,
    path: String,
}

struct Bus<'s, 't> {
    kind: Kind,
    id: &'s str,
    path: String,
    peripheral: Option<&'s str>,
    signals: Vec<Sig<'s, 't>>,
}

/// One motor output. `key` is what the outputs of one timer must share: the
/// protocol, and a PWM output's rate.
struct Motor<'s, 't> {
    label: &'s str,
    path: String,
    hand: Option<&'t str>,
    protocol: MotorProtocol,
    pwm: Option<&'s PwmTiming>,
    /// The timer-share key: outputs of one timer share its protocol, rate
    /// and tick (`AET-WO-1211a`: `resolution_ps` joins it).
    key: crate::motor::TimerKey,
}

/// A timer channel on a pin: `(timer, channel, pin, af)`.
type Channel<'t> = (u32, u32, &'t str, u8);

/// One serial port: its label, its declared peripheral, and its `TX` and
/// `RX` signals.
struct Uart<'s, 't> {
    label: &'s str,
    path: String,
    peripheral: Option<&'s str>,
    tx: Sig<'s, 't>,
    rx: Sig<'s, 't>,
}

/// A placed serial port: its peripheral, then its TX and RX `(pin, af)`.
type UartPlaced<'t> = (&'static str, (&'t str, u8), (&'t str, u8));

struct Model<'s, 't> {
    buses: Vec<Bus<'s, 't>>,
    cs: Vec<Sig<'s, 't>>,
    /// Each pin device's hand pin with its claimant (`AET-WO-1211a`).
    device_hands: Vec<(&'t str, String)>,
    motors: Vec<Motor<'s, 't>>,
    serial: Vec<Uart<'s, 't>>,
    /// The peripherals of `serial::PERIPHERALS` the table offers (both
    /// `<P>_TX` and `<P>_RX` on some pin), in that order.
    uart_instances: Vec<&'static str>,
    /// Every unreserved timer channel of the table, by timer, channel, then
    /// pin in key order.
    channels: Vec<Channel<'t>>,
    i2c_instances: Vec<String>,
    spi_instances: Vec<String>,
    /// Per (instance, signal): the unreserved pins carrying it, in key order.
    candidates: BTreeMap<(String, &'static str), Vec<(&'t str, u8)>>,
    /// Per (instance, signal): the analog pads carrying it, which
    /// `candidates` leaves out (`AET-WO-1217` F4), in key order; a refusal
    /// names them (`AET-WO-1218`, `1217` R4).
    withheld: BTreeMap<(String, &'static str), Vec<&'t str>>,
    /// What a net at a connector is offered: FT pins only (`AET-WO-4300i`
    /// D1 (a), adversarial pass F1).
    at: crate::offboard::AtConnector<'s>,
}

/// An instance is offered when the table carries each of its kind's signals
/// for it on some pin. Names sort in byte order.
fn instances(table: &PinTable, kind: Kind) -> Vec<String> {
    let mut found: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for pin in table.pins.values() {
        for name in pin.alternate_functions.keys() {
            let Some((inst, sig)) = name.split_once('_') else {
                continue;
            };
            let numbered = inst
                .strip_prefix(kind.prefix())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            if numbered && kind.signals().contains(&sig) {
                found.entry(inst).or_default().insert(sig);
            }
        }
    }
    found
        .into_iter()
        .filter(|(_, sigs)| sigs.len() == kind.signals().len())
        .map(|(inst, _)| inst.to_owned())
        .collect()
}

impl<'s, 't> Model<'s, 't> {
    /// Builds the model and runs the checks before the search, in the
    /// README's order: hand pins (unknown, reserved, claimed twice), hand
    /// peripherals (unknown, claimed twice), then each bus alone.
    fn new(spec: &'s Specification, table: &'t PinTable) -> Result<Self, SolveError> {
        // Each declared reservation is a pin of the table (`AET-WO-4300i`
        // F7); `validate` judged the rest of it.
        for (i, r) in spec.mcu.reserved_pins.iter().enumerate() {
            if !table.pins.contains_key(&r.pin) {
                return Err(SolveError::UnknownPin {
                    pin: r.pin.clone(),
                    claimant: format!("mcu.reserved_pins[{i}]"),
                });
            }
        }
        let reserved = all_reserved(spec);
        let pads = analog_pads(&spec.mcu.part).map_err(SolveError::InvalidSpecification)?;
        // belt-and-suspenders: step 26 already refused a part with no MCU row
        // when a net faces a connector; `solve` re-checks its input rather than
        // trusting its caller, as it re-runs `validate`.
        let at =
            crate::offboard::AtConnector::new(spec).map_err(SolveError::InvalidSpecification)?;
        let mut claimed: BTreeMap<&'t str, String> = BTreeMap::new();
        // `digital`: every claimant but an ADC input, which an analog pad
        // takes (`AET-WO-1217` F4).
        let mut hand =
            |signal: &Signal, path: &str, digital: bool| -> Result<Option<&'t str>, SolveError> {
                let Some(written) = signal.pin.as_deref() else {
                    return Ok(None);
                };
                let claimant = path.to_owned();
                let Some((key, _)) = table.pins.get_key_value(written) else {
                    return Err(SolveError::UnknownPin {
                        pin: written.to_owned(),
                        claimant,
                    });
                };
                let key = key.as_str();
                if reserved.contains(&key) {
                    return Err(reserved_refusal(spec, key, &claimant));
                }
                if digital && pads.contains(&key) {
                    return Err(SolveError::AnalogPadPin {
                        pin: key.to_owned(),
                        claimant,
                    });
                }
                if let Some(first) = claimed.insert(key, claimant.clone()) {
                    return Err(SolveError::PinClaimedTwice {
                        pin: key.to_owned(),
                        first,
                        second: claimant,
                    });
                }
                Ok(Some(key))
            };
        let mut sig = |name: &'static str,
                       signal: &'s Signal,
                       path: String|
         -> Result<Sig<'s, 't>, SolveError> {
            Ok(Sig {
                name,
                net: &signal.net,
                hand: hand(signal, &path, true)?,
                path,
            })
        };

        let mut buses = Vec::new();
        for (i, b) in spec.i2c_buses.iter().enumerate() {
            let at = format!("i2c_buses[{i}]");
            buses.push(Bus {
                kind: Kind::I2c,
                id: &b.id,
                path: at.clone(),
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    sig("SDA", &b.sda, format!("{at}.sda"))?,
                    sig("SCL", &b.scl, format!("{at}.scl"))?,
                ],
            });
        }
        for (i, b) in spec.spi_buses.iter().enumerate() {
            let at = format!("spi_buses[{i}]");
            buses.push(Bus {
                kind: Kind::Spi,
                id: &b.id,
                path: at.clone(),
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    sig("SCK", &b.sck, format!("{at}.sck"))?,
                    sig("MOSI", &b.mosi, format!("{at}.mosi"))?,
                    sig("MISO", &b.miso, format!("{at}.miso"))?,
                ],
            });
        }
        let mut cs = Vec::new();
        for (i, d) in spec.spi_devices.iter().enumerate() {
            cs.push(sig("CS", &d.cs, format!("spi_devices[{i}].cs"))?);
        }
        let mut motors = Vec::new();
        for (i, m) in spec.motor_outputs.iter().enumerate() {
            let path = format!("motor_outputs[{i}]");
            let pin = Signal {
                net: m.label.clone(),
                pin: m.pin.clone(),
            };
            motors.push(Motor {
                label: &m.label,
                hand: hand(&pin, &path, true)?,
                path,
                protocol: m.protocol,
                pwm: m.pwm.as_ref(),
                key: (
                    m.protocol,
                    m.pwm.as_ref().map(|p| (p.rate_hz, p.resolution_ps)),
                ),
            });
        }
        let mut serial = Vec::new();
        for (i, p) in spec.serial_ports.iter().enumerate() {
            let path = format!("serial_ports[{i}]");
            let (tx_path, rx_path) = (format!("{path}.tx"), format!("{path}.rx"));
            serial.push(Uart {
                label: &p.label,
                peripheral: p.peripheral.as_deref(),
                tx: Sig {
                    name: "TX",
                    net: &p.tx.net,
                    hand: hand(&p.tx, &tx_path, true)?,
                    path: tx_path,
                },
                rx: Sig {
                    name: "RX",
                    net: &p.rx.net,
                    hand: hand(&p.rx, &rx_path, true)?,
                    path: rx_path,
                },
                path,
            });
        }
        // No motor output is offered a pin holding an internal pull from
        // reset (`AET-WO-4300i` F2, RM0433 Rev 9 p. 542): a reset pull-up
        // fights the declared pull-down until the timer takes the pin, and a
        // hand pin there is `validate`'s `MotorPinResetPull`.
        let reset_pull = |pin: &str| {
            crate::pin_devices::MCU_RESET_PULL_UPS.contains(&pin)
                || crate::pin_devices::MCU_RESET_PULL_DOWNS.contains(&pin)
        };
        let mut channels: Vec<Channel<'t>> = table
            .pins
            .iter()
            .filter(|(pin, _)| {
                !reserved.contains(&pin.as_str())
                    && !pads.contains(&pin.as_str())
                    && !reset_pull(pin)
            })
            .flat_map(|(pin, p)| {
                p.alternate_functions.iter().filter_map(move |(name, af)| {
                    motor::timer_channel(name).map(|(n, k)| (n, k, pin.as_str(), *af))
                })
            })
            .collect();
        channels.sort_unstable();

        let i2c_instances = instances(table, Kind::I2c);
        let spi_instances = instances(table, Kind::Spi);
        let carried = |wanted: &str| {
            table
                .pins
                .values()
                .any(|p| p.alternate_functions.contains_key(wanted))
        };
        // A pin device's hand pins (`AET-WO-1211a`), claimed here so that
        // the search places nothing on them and two claims are refused by
        // name; the device's signals and ADC inputs are placed after it.
        let mut device_hands = Vec::new();
        for (i, d) in spec.declared_pin_devices().iter().enumerate() {
            let held = d
                .signals
                .iter()
                .enumerate()
                .map(|(k, s)| {
                    (
                        &s.net,
                        &s.pin,
                        format!("pin_devices[{i}].signals[{k}]"),
                        true,
                    )
                })
                .chain(d.analog.iter().enumerate().map(|(k, a)| {
                    (
                        &a.net,
                        &a.pin,
                        format!("pin_devices[{i}].analog[{k}]"),
                        false,
                    )
                }));
            for (net, pin, path, digital) in held {
                let signal = Signal {
                    net: net.clone(),
                    pin: pin.clone(),
                };
                if let Some(key) = hand(&signal, &path, digital)? {
                    device_hands.push((key, path));
                }
            }
        }
        let uart_instances: Vec<&'static str> = serial::PERIPHERALS
            .into_iter()
            .filter(|p| carried(&format!("{p}_TX")) && carried(&format!("{p}_RX")))
            .collect();
        let mut candidates = BTreeMap::new();
        let mut withheld = BTreeMap::new();
        let uart_signals: &[&'static str] = &["TX", "RX"];
        let groups = [
            (Kind::I2c.signals(), i2c_instances.clone()),
            (Kind::Spi.signals(), spi_instances.clone()),
            (
                uart_signals,
                uart_instances.iter().map(|p| (*p).to_owned()).collect(),
            ),
        ];
        for (signals, list) in groups {
            for inst in list {
                for name in signals {
                    let wanted = format!("{inst}_{name}");
                    let pins = table
                        .pins
                        .iter()
                        .filter(|(pin, _)| {
                            !reserved.contains(&pin.as_str()) && !pads.contains(&pin.as_str())
                        })
                        .filter_map(|(pin, p)| {
                            p.alternate_functions
                                .get(&wanted)
                                .map(|af| (pin.as_str(), *af))
                        })
                        .collect();
                    candidates.insert((inst.clone(), *name), pins);
                    let on_pads: Vec<&'t str> = table
                        .pins
                        .iter()
                        .filter(|(pin, p)| {
                            !reserved.contains(&pin.as_str())
                                && pads.contains(&pin.as_str())
                                && p.alternate_functions.contains_key(&wanted)
                        })
                        .map(|(pin, _)| pin.as_str())
                        .collect();
                    withheld.insert((inst.clone(), *name), on_pads);
                }
            }
        }
        let model = Model {
            buses,
            cs,
            device_hands,
            motors,
            serial,
            uart_instances,
            channels,
            i2c_instances,
            spi_instances,
            candidates,
            withheld,
            at,
        };
        model.check_peripherals()?;
        model.check_each_bus()?;
        Ok(model)
    }

    fn instances_of(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::I2c => &self.i2c_instances,
            Kind::Spi => &self.spi_instances,
        }
    }

    fn pins_for(&self, inst: &str, sig: &'static str) -> &[(&'t str, u8)] {
        self.candidates
            .get(&(inst.to_owned(), sig))
            .map_or(&[], Vec::as_slice)
    }

    /// The pins `sig` may take on `inst`, its net's own filter over
    /// [`Model::pins_for`]: at a connector, the FT ones only
    /// (`AET-WO-4300i` D1, adversarial pass F1). Every branch's candidates
    /// for a bus line or a serial line are these.
    fn offered<'a>(
        &'a self,
        inst: &str,
        sig: &'a Sig<'s, 't>,
    ) -> impl Iterator<Item = (&'t str, u8)> + 'a {
        self.pins_for(inst, sig.name)
            .iter()
            .copied()
            .filter(move |(pin, _)| self.at.offers(sig.net, pin))
    }

    /// What a refusal adds for the pins withheld from `inst`'s `sig`: the
    /// analog pads carrying it (`AET-WO-1218`, `1217` R4), then, at a
    /// connector, the non-FT ones (`AET-WO-4300i` D1); nothing when none is.
    fn withheld_note(&self, inst: &str, sig: &Sig<'s, 't>) -> String {
        let pads = self
            .withheld
            .get(&(inst.to_owned(), sig.name))
            .map_or(&[][..], Vec::as_slice);
        let mut note = String::new();
        if !pads.is_empty() {
            let named: Vec<String> = pads.iter().map(|p| format!("`{p}`")).collect();
            note = format!(
                "; {} withheld as an analog pad (the MCU row's `io.analog_pads`)",
                named.join(", ")
            );
        }
        note + &self.at.note(
            sig.net,
            self.pins_for(inst, sig.name).iter().map(|(p, _)| *p),
        )
    }

    fn check_peripherals(&self) -> Result<(), SolveError> {
        let mut taken: BTreeMap<&str, String> = BTreeMap::new();
        for bus in &self.buses {
            let Some(p) = bus.peripheral else { continue };
            let claimant = bus.path.clone();
            if !self.instances_of(bus.kind).iter().any(|i| i == p) {
                return Err(SolveError::UnknownPeripheral {
                    peripheral: p.to_owned(),
                    claimant,
                });
            }
            if let Some(first) = taken.insert(p, claimant.clone()) {
                return Err(SolveError::PeripheralClaimedTwice {
                    peripheral: p.to_owned(),
                    first,
                    second: claimant,
                });
            }
        }
        // `validate` refused a serial peripheral outside `serial::PERIPHERALS`
        // and one named twice; the table may still lack one.
        for u in &self.serial {
            if let Some(p) = u.peripheral.filter(|p| !self.uart_instances.contains(p)) {
                return Err(SolveError::UnknownPeripheral {
                    peripheral: p.to_owned(),
                    claimant: u.path.clone(),
                });
            }
        }
        Ok(())
    }

    /// Each bus alone: reserved pins and every hand pin honoured, no other
    /// bus's choices. A bus that fails here can never be placed.
    fn check_each_bus(&self) -> Result<(), SolveError> {
        let alone = Search::new(self, (0..self.buses.len()).collect(), false);
        for (b, bus) in self.buses.iter().enumerate() {
            let mut reasons = Vec::new();
            let mut served = false;
            for inst in self.instances_of(bus.kind) {
                if bus.peripheral.is_some_and(|p| p != inst) {
                    continue;
                }
                if alone.placeable(b, inst) {
                    served = true;
                    break;
                }
                let sig = bus
                    .signals
                    .iter()
                    .find(|s| !alone.has_pin(s, inst))
                    .expect("an unplaceable bus has a signal with no pin");
                let pins: Vec<(&str, u8)> = self.offered(inst, sig).collect();
                let name = format!("{inst}_{}", sig.name);
                let note = self.withheld_note(inst, sig);
                reasons.push(match sig.hand {
                    Some(h) => format!("{inst}: `{name}` is not on pin `{h}`"),
                    None if pins.is_empty() => {
                        format!("{inst}: the table has no unreserved pin for `{name}`{note}")
                    }
                    None => format!(
                        "{inst}: every pin for `{name}` is held by hand ({}){note}",
                        pins.iter()
                            .map(|(pin, _)| format!("`{pin}` by `{}`", alone.used[pin]))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            if !served {
                return Err(SolveError::NoInstance {
                    bus: bus.id.to_owned(),
                    reasons,
                });
            }
        }
        Ok(())
    }

    /// The checks on the motor outputs before the search, in the README's
    /// order: each alone can take a channel; two whose pins leave each only
    /// one timer, the same one, share their timing; and together, beside
    /// every hand pin and no bus, they can all be placed.
    fn check_motors(&self, budget: &mut Budget) -> Result<(), SolveError> {
        let alone = Search::beside_hand_pins(self);
        let mut timers: Vec<BTreeSet<u32>> = Vec::new();
        for (m, motor) in self.motors.iter().enumerate() {
            let options: Vec<Channel<'t>> = alone.channels_for(m).collect();
            if options.is_empty() {
                let note = self.at.note(motor.label, self.channels.iter().map(|c| c.2));
                let reason = match motor.hand {
                    Some(h) if !self.channels.iter().any(|c| c.2 == h) => {
                        format!("pin `{h}` carries no timer channel (`TIMn_CH1` to `TIMn_CH4`)")
                    }
                    Some(h) => format!("pin `{h}` is held by `{}`", alone.used[h]),
                    None if !self
                        .channels
                        .iter()
                        .any(|c| self.at.offers(motor.label, c.2)) =>
                    {
                        format!("the table has no unreserved pin with a timer channel{note}")
                    }
                    None => format!("every pin with a timer channel is held by hand{note}"),
                };
                return Err(SolveError::NoTimerChannel {
                    output: motor.label.to_owned(),
                    reasons: vec![reason],
                });
            }
            timers.push(options.iter().map(|c| c.0).collect());
        }
        for (a, first) in self.motors.iter().enumerate() {
            for (b, second) in self.motors.iter().enumerate().skip(a + 1) {
                if first.key != second.key && timers[a].len() == 1 && timers[a] == timers[b] {
                    let timer = timers[a].iter().next().expect("one timer");
                    let rate = |k: &crate::motor::TimerKey| (k.0, k.1.map(|p| p.0));
                    if rate(&first.key) == rate(&second.key) {
                        return Err(SolveError::TimerResolutionConflict {
                            timer: format!("TIM{timer}"),
                            first: first.label.to_owned(),
                            second: second.label.to_owned(),
                        });
                    }
                    return Err(SolveError::TimerRateConflict {
                        timer: format!("TIM{timer}"),
                        first: first.label.to_owned(),
                        second: second.label.to_owned(),
                    });
                }
            }
        }
        let all: Vec<usize> = (0..self.motors.len()).collect();
        if self.motors_fit(&all, budget)? {
            return Ok(());
        }
        // Deletion, as for the buses: the set left is minimal.
        let mut set = all;
        let mut i = 0;
        while i < set.len() {
            let mut without = set.clone();
            without.remove(i);
            if self.motors_fit(&without, budget)? {
                i += 1;
            } else {
                set = without;
            }
        }
        Err(SolveError::MotorsInfeasible {
            claimants: set
                .iter()
                .map(|&m| self.motors[m].label.to_owned())
                .collect(),
        })
    }

    /// Whether the motor outputs in `subset` can all be placed beside every
    /// hand pin, with no bus placed.
    fn motors_fit(&self, subset: &[usize], budget: &mut Budget) -> Result<bool, SolveError> {
        let mut search = Search::beside_hand_pins(self);
        // Pigeonhole: each rate needs a timer of its own, so more rates than
        // the timers the subset can reach cannot fit; the search would spend
        // its budget proving it (adversarial pass).
        let (mut keys, mut timers) = (BTreeSet::new(), BTreeSet::new());
        for &m in subset {
            keys.insert(self.motors[m].key);
            timers.extend(search.channels_for(m).map(|c| c.0));
        }
        if keys.len() > timers.len() {
            return Ok(false);
        }
        search.motor_active = subset.to_vec();
        search.serial_active = Vec::new();
        let found = search.run(budget);
        if let Some(pos) = search.exceeded {
            return Err(self.budget_error(&search, pos));
        }
        Ok(found)
    }

    /// The checks on the serial ports before the search, in the README's
    /// order: each alone, beside every hand pin, can take a peripheral and
    /// two pins (a fixed TX and RX on no common peripheral is named
    /// apart); and together, beside every hand pin and no bus or motor
    /// output, they can all be placed.
    fn check_serial(&self, budget: &mut Budget) -> Result<(), SolveError> {
        let alone = Search::beside_hand_pins(self);
        for (u, port) in self.serial.iter().enumerate() {
            if !alone.uart_options(u).is_empty() {
                continue;
            }
            let allowed: Vec<&'static str> = self
                .uart_instances
                .iter()
                .copied()
                .filter(|p| port.peripheral.is_none_or(|d| d == *p))
                .collect();
            let carrying = |sig: &Sig<'s, 't>, pin: &str| -> BTreeSet<&'static str> {
                allowed
                    .iter()
                    .copied()
                    .filter(|p| self.pins_for(p, sig.name).iter().any(|(q, _)| *q == pin))
                    .collect()
            };
            if let (Some(tx), Some(rx)) = (port.tx.hand, port.rx.hand) {
                let (t, r) = (carrying(&port.tx, tx), carrying(&port.rx, rx));
                if !t.is_empty() && !r.is_empty() && t.is_disjoint(&r) {
                    return Err(SolveError::SerialPinsSplit {
                        port: port.label.to_owned(),
                        tx_pin: tx.to_owned(),
                        rx_pin: rx.to_owned(),
                    });
                }
            }
            let reasons = if allowed.is_empty() {
                vec!["the table offers no USART or UART with both TX and RX".to_owned()]
            } else {
                allowed.iter().map(|p| alone.uart_reason(port, p)).collect()
            };
            return Err(SolveError::NoSerialPeripheral {
                port: port.label.to_owned(),
                reasons,
            });
        }
        let all: Vec<usize> = (0..self.serial.len()).collect();
        if self.serial_fit(&all, budget)? {
            return Ok(());
        }
        let mut set = all;
        let mut i = 0;
        while i < set.len() {
            let mut without = set.clone();
            without.remove(i);
            if self.serial_fit(&without, budget)? {
                i += 1;
            } else {
                set = without;
            }
        }
        Err(SolveError::SerialPortsInfeasible {
            claimants: set
                .iter()
                .map(|&u| self.serial[u].label.to_owned())
                .collect(),
        })
    }

    /// Whether the serial ports in `subset` can all be placed beside every
    /// hand pin, with no bus or motor output placed.
    fn serial_fit(&self, subset: &[usize], budget: &mut Budget) -> Result<bool, SolveError> {
        let mut search = Search::beside_hand_pins(self);
        search.motor_active = Vec::new();
        search.serial_active = subset.to_vec();
        let found = search.run(budget);
        if let Some(pos) = search.exceeded {
            return Err(self.budget_error(&search, pos));
        }
        Ok(found)
    }

    fn budget_error(&self, search: &Search, pos: usize) -> SolveError {
        SolveError::BudgetExceeded {
            bus: if pos == MOTORS {
                "motor_outputs".to_owned()
            } else if pos == SERIAL {
                "serial_ports".to_owned()
            } else {
                self.buses[search.active[pos]].id.to_owned()
            },
            limit: search.budget.limit,
        }
    }

    /// Deletion: drop each bus in document order if the rest stay
    /// infeasible. The set left is minimal (irreducible), not necessarily the
    /// smallest; it costs one search per bus, inside the same budget.
    fn minimal_set(&self, mut set: Vec<usize>, budget: &mut Budget, forced: bool) -> SolveError {
        let mut i = 0;
        while i < set.len() {
            let mut without = set.clone();
            without.remove(i);
            let mut search = Search::new(self, without.clone(), forced);
            let found = search.run(budget);
            if let Some(pos) = search.exceeded {
                return self.budget_error(&search, pos);
            }
            if found {
                i += 1;
            } else {
                set = without;
            }
        }
        let buses = set.iter().map(|&b| self.buses[b].id.to_owned());
        if self.motors.is_empty() && self.serial.is_empty() {
            return SolveError::Infeasible {
                buses: buses.collect(),
            };
        }
        // `check_motors` and `check_serial` placed every output and every
        // port beside the hand pins, so the conflict is between these buses
        // and the outputs or ports, which stay in every search.
        let claimants = buses
            .chain(self.motors.iter().map(|m| m.label.to_owned()))
            .chain(self.serial.iter().map(|u| u.label.to_owned()))
            .collect();
        if self.serial.is_empty() {
            SolveError::MotorsInfeasible { claimants }
        } else {
            SolveError::SerialPortsInfeasible { claimants }
        }
    }
}

struct Budget {
    spent: u64,
    limit: u64,
}

/// The position [`Search::tick`] records when the motor outputs spend the
/// budget, where a bus's position is recorded otherwise.
const MOTORS: usize = usize::MAX;
/// The same, for the serial ports.
const SERIAL: usize = usize::MAX - 1;

/// A placed bus: its instance, and per signal its `(pin, af)`.
type Placed<'t> = (String, Vec<(&'t str, u8)>);

/// One search over the buses in `active`, in that order.
struct Search<'m, 's, 't> {
    model: &'m Model<'s, 't>,
    active: Vec<usize>,
    /// Pin to owner path. Every hand pin of an active bus or a `cs` is owned
    /// from the start.
    used: BTreeMap<&'t str, String>,
    used_instances: BTreeSet<String>,
    chosen: Vec<Option<Placed<'t>>>,
    budget: Budget,
    /// The position being placed when the budget ran out.
    exceeded: Option<usize>,
    /// Whether `consistent` runs the forced-instance check.
    forced: bool,
    /// The motor outputs placed once every active bus is, in order.
    motor_active: Vec<usize>,
    /// Per motor output, its channel once placed.
    motor_chosen: Vec<Option<Channel<'t>>>,
    /// The timer channels taken, and each timer's shared key and outputs.
    channels_used: BTreeSet<(u32, u32)>,
    timer_keys: BTreeMap<u32, (crate::motor::TimerKey, usize)>,
    /// The serial ports placed once every motor output is, in order.
    serial_active: Vec<usize>,
    /// Per serial port, its placement once placed.
    serial_chosen: Vec<Option<UartPlaced<'t>>>,
    /// The peripherals the placed serial ports hold.
    serial_taken: BTreeSet<&'static str>,
}

impl<'m, 's, 't> Search<'m, 's, 't> {
    fn new(model: &'m Model<'s, 't>, active: Vec<usize>, forced: bool) -> Self {
        let mut used = BTreeMap::new();
        let hand = active
            .iter()
            .flat_map(|&b| &model.buses[b].signals)
            .chain(&model.cs);
        for sig in hand {
            if let Some(pin) = sig.hand {
                used.insert(pin, sig.path.clone());
            }
        }
        for m in &model.motors {
            if let Some(pin) = m.hand {
                used.insert(pin, m.path.clone());
            }
        }
        for sig in model.serial.iter().flat_map(|u| [&u.tx, &u.rx]) {
            if let Some(pin) = sig.hand {
                used.insert(pin, sig.path.clone());
            }
        }
        for (pin, path) in &model.device_hands {
            used.insert(*pin, path.clone());
        }
        let n = active.len();
        Search {
            model,
            active,
            used,
            used_instances: BTreeSet::new(),
            chosen: vec![None; n],
            budget: Budget { spent: 0, limit: 0 },
            exceeded: None,
            forced,
            motor_active: (0..model.motors.len()).collect(),
            motor_chosen: vec![None; model.motors.len()],
            channels_used: BTreeSet::new(),
            timer_keys: BTreeMap::new(),
            serial_active: (0..model.serial.len()).collect(),
            serial_chosen: vec![None; model.serial.len()],
            serial_taken: BTreeSet::new(),
        }
    }

    /// A search with no bus active, and every hand pin owned: the buses',
    /// the chip selects', the motor outputs' and the serial ports'.
    fn beside_hand_pins(model: &'m Model<'s, 't>) -> Self {
        let mut s = Self::new(model, Vec::new(), false);
        for sig in model.buses.iter().flat_map(|b| &b.signals) {
            if let Some(pin) = sig.hand {
                s.used.insert(pin, sig.path.clone());
            }
        }
        s
    }

    /// Runs the search inside `budget`, and charges what it spent.
    fn run(&mut self, budget: &mut Budget) -> bool {
        self.budget = Budget {
            spent: budget.spent,
            limit: budget.limit,
        };
        let found = self.consistent(0) && self.bus(0);
        budget.spent = self.budget.spent;
        found
    }

    /// Counts one value tried; false once the budget is spent.
    fn tick(&mut self, pos: usize) -> bool {
        self.budget.spent += 1;
        if self.budget.spent > self.budget.limit {
            self.exceeded.get_or_insert(pos);
        }
        self.exceeded.is_none()
    }

    fn free(&self, pin: &str, owner: &str) -> bool {
        self.used.get(pin).is_none_or(|o| o == owner)
    }

    /// The pins `sig` could take on `inst` right now.
    fn pins<'a>(&'a self, sig: &'a Sig<'s, 't>, inst: &str) -> impl Iterator<Item = &'t str> + 'a {
        self.model
            .offered(inst, sig)
            .map(|(pin, _)| pin)
            .filter(move |pin| sig.hand.is_none_or(|h| h == *pin) && self.free(pin, &sig.path))
    }

    fn has_pin(&self, sig: &Sig<'s, 't>, inst: &str) -> bool {
        self.pins(sig, inst).next().is_some()
    }

    fn placeable(&self, b: usize, inst: &str) -> bool {
        let bus = &self.model.buses[b];
        bus.peripheral.is_none_or(|p| p == inst)
            && !self.used_instances.contains(inst)
            && bus.signals.iter().all(|s| self.has_pin(s, inst))
    }

    /// The checks every node passes before the search goes deeper, over the
    /// buses from position `from` on: each still has a placeable instance;
    /// they can take distinct instances; and their signals can take distinct
    /// free pins, each from the union over its bus's placeable instances.
    /// Each only answers yes or no, so none changes which answer is found.
    fn consistent(&self, from: usize) -> bool {
        let rest = &self.active[from..];
        let options: Vec<Vec<&str>> = rest
            .iter()
            .map(|&b| {
                self.model
                    .instances_of(self.model.buses[b].kind)
                    .iter()
                    .filter(|i| self.placeable(b, i))
                    .map(String::as_str)
                    .collect()
            })
            .collect();
        if options.iter().any(Vec::is_empty) || !matching(&options) {
            return false;
        }
        let mut signal_pins: Vec<Vec<&str>> = Vec::new();
        for (&b, insts) in rest.iter().zip(&options) {
            for sig in &self.model.buses[b].signals {
                let pins: BTreeSet<&str> = insts.iter().flat_map(|i| self.pins(sig, i)).collect();
                signal_pins.push(pins.into_iter().collect());
            }
        }
        matching(&signal_pins) && (!self.forced || self.forced_instances(rest, &options))
    }

    /// The forced-instance check. For a kind whose unplaced buses are as many
    /// as the instances any of them can still take, those buses use every one
    /// of those instances, so each instance's every signal needs a pin of its
    /// own. The rows are (instance, signal); a row may take any free pin that
    /// some unplaced bus of the kind could give that signal on that instance.
    /// A relaxation of the real choice, so it only prunes.
    fn forced_instances(&self, rest: &[usize], options: &[Vec<&str>]) -> bool {
        let mut rows: Vec<Vec<&str>> = Vec::new();
        for kind in [Kind::I2c, Kind::Spi] {
            let buses: Vec<usize> = (0..rest.len())
                .filter(|&r| self.model.buses[rest[r]].kind == kind)
                .collect();
            let reachable: BTreeSet<&str> = buses
                .iter()
                .flat_map(|&r| options[r].iter().copied())
                .collect();
            if buses.is_empty() || buses.len() != reachable.len() {
                continue;
            }
            for inst in reachable {
                for k in 0..kind.signals().len() {
                    let pins: BTreeSet<&str> = buses
                        .iter()
                        .filter(|&&r| options[r].contains(&inst))
                        .flat_map(|&r| self.pins(&self.model.buses[rest[r]].signals[k], inst))
                        .collect();
                    rows.push(pins.into_iter().collect());
                }
            }
        }
        matching(&rows)
    }

    fn bus(&mut self, pos: usize) -> bool {
        if pos == self.active.len() {
            return self.motor(0);
        }
        let model = self.model;
        let b = self.active[pos];
        for inst in model.instances_of(model.buses[b].kind) {
            if !self.tick(pos) {
                return false;
            }
            if !self.placeable(b, inst) {
                continue;
            }
            self.used_instances.insert(inst.clone());
            if self.signal(pos, inst, 0, &mut Vec::new()) {
                return true;
            }
            self.used_instances.remove(inst);
        }
        false
    }

    /// The channels motor output `m` could take now: its hand pin's, or any
    /// free pin's, on a channel not taken, of a timer free or already
    /// carrying its key.
    fn channels_for<'a>(&'a self, m: usize) -> impl Iterator<Item = Channel<'t>> + 'a {
        let motor = &self.model.motors[m];
        self.model
            .channels
            .iter()
            .copied()
            .filter(move |&(n, k, pin, _)| {
                motor.hand.is_none_or(|h| h == pin)
                    && self.model.at.offers(motor.label, pin)
                    && self.free(pin, &motor.path)
                    && !self.channels_used.contains(&(n, k))
                    && self
                        .timer_keys
                        .get(&n)
                        .is_none_or(|(key, _)| *key == motor.key)
            })
    }

    /// Places the motor outputs from position `pos` of `motor_active` on,
    /// trying each one's channels in order; the last one placed ends the
    /// search.
    fn motor(&mut self, pos: usize) -> bool {
        if pos == self.motor_active.len() {
            return self.serial(0);
        }
        let m = self.motor_active[pos];
        let options: Vec<Channel<'t>> = self.channels_for(m).collect();
        let (path, key) = {
            let motor = &self.model.motors[m];
            (motor.path.clone(), motor.key)
        };
        for c @ (n, k, pin, _) in options {
            if !self.tick(MOTORS) {
                return false;
            }
            let owned = self.used.contains_key(pin);
            self.used.insert(pin, path.clone());
            self.channels_used.insert((n, k));
            self.timer_keys.entry(n).or_insert((key, 0)).1 += 1;
            self.motor_chosen[m] = Some(c);
            if self.motor(pos + 1) {
                return true;
            }
            self.motor_chosen[m] = None;
            let e = self.timer_keys.get_mut(&n).expect("just counted");
            e.1 -= 1;
            if e.1 == 0 {
                self.timer_keys.remove(&n);
            }
            self.channels_used.remove(&(n, k));
            if !owned {
                self.used.remove(pin);
            }
        }
        false
    }

    /// The placements serial port `u` could take now, in order: each
    /// peripheral it may use (its own, or each the table offers in
    /// `serial::PERIPHERALS` order) not taken, then its TX pins, then its RX
    /// pins, each in key order, each its hand pin or free.
    fn uart_options(&self, u: usize) -> Vec<UartPlaced<'t>> {
        let port = &self.model.serial[u];
        let pins = |p: &str, sig: &'m Sig<'s, 't>| -> Vec<(&'t str, u8)> {
            self.model
                .offered(p, sig)
                .filter(|(pin, _)| sig.hand.is_none_or(|h| h == *pin) && self.free(pin, &sig.path))
                .collect()
        };
        let mut out = Vec::new();
        for &p in &self.model.uart_instances {
            if port.peripheral.is_some_and(|d| d != p) || self.serial_taken.contains(p) {
                continue;
            }
            let (rxs, txs) = (pins(p, &port.rx), pins(p, &port.tx));
            for tx in txs {
                for &rx in rxs.iter().filter(|rx| rx.0 != tx.0) {
                    out.push((p, tx, rx));
                }
            }
        }
        out
    }

    /// Why serial port `port` cannot take peripheral `p` now: a fixed pin
    /// that does not carry its signal, or a signal with no free pin.
    fn uart_reason(&self, port: &Uart<'s, 't>, p: &str) -> String {
        for sig in [&port.tx, &port.rx] {
            let name = format!("{p}_{}", sig.name);
            let pins: Vec<(&str, u8)> = self.model.offered(p, sig).collect();
            let note = self.model.withheld_note(p, sig);
            match sig.hand {
                Some(h) if !pins.iter().any(|(q, _)| *q == h) => {
                    return format!("{p}: `{name}` is not on pin `{h}`");
                }
                Some(_) => {}
                None if pins.is_empty() => {
                    return format!("{p}: the table has no unreserved pin for `{name}`{note}");
                }
                None if pins.iter().all(|(q, _)| !self.free(q, &sig.path)) => {
                    return format!(
                        "{p}: every pin for `{name}` is held by hand ({}){note}",
                        pins.iter()
                            .map(|(q, _)| format!("`{q}` by `{}`", self.used[q]))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                None => {}
            }
        }
        format!("{p}: its TX and RX can only take one pin")
    }

    /// Places the serial ports from position `pos` of `serial_active` on,
    /// trying each one's placements in order; the last one placed ends the
    /// search.
    fn serial(&mut self, pos: usize) -> bool {
        if pos == self.serial_active.len() {
            return true;
        }
        let u = self.serial_active[pos];
        let (tx_path, rx_path) = {
            let port = &self.model.serial[u];
            (port.tx.path.clone(), port.rx.path.clone())
        };
        for c @ (p, (tx, _), (rx, _)) in self.uart_options(u) {
            if !self.tick(SERIAL) {
                return false;
            }
            let owned = (self.used.contains_key(tx), self.used.contains_key(rx));
            self.used.insert(tx, tx_path.clone());
            self.used.insert(rx, rx_path.clone());
            self.serial_taken.insert(p);
            self.serial_chosen[u] = Some(c);
            if self.serial(pos + 1) {
                return true;
            }
            self.serial_chosen[u] = None;
            self.serial_taken.remove(p);
            if !owned.0 {
                self.used.remove(tx);
            }
            if !owned.1 {
                self.used.remove(rx);
            }
        }
        false
    }

    fn signal(&mut self, pos: usize, inst: &str, k: usize, pins: &mut Vec<(&'t str, u8)>) -> bool {
        let model = self.model;
        let bus = &model.buses[self.active[pos]];
        if k == bus.signals.len() {
            if !self.consistent(pos + 1) {
                return false;
            }
            self.chosen[pos] = Some((inst.to_owned(), pins.clone()));
            if self.bus(pos + 1) {
                return true;
            }
            self.chosen[pos] = None;
            return false;
        }
        let sig = &bus.signals[k];
        let order: Vec<(&'t str, u8)> = model.offered(inst, sig).collect();
        for (pin, af) in order {
            if !self.tick(pos) {
                return false;
            }
            if sig.hand.is_some_and(|h| h != pin) || !self.free(pin, &sig.path) {
                continue;
            }
            let owned = self.used.contains_key(pin);
            self.used.insert(pin, sig.path.clone());
            pins.push((pin, af));
            if self.signal(pos, inst, k + 1, pins) {
                return true;
            }
            pins.pop();
            if !owned {
                self.used.remove(pin);
            }
        }
        false
    }
}

/// Whether every row can take a distinct value (Hall's condition), by
/// augmenting paths in fixed order. Only the yes or no leaves this function.
fn matching(options: &[Vec<&str>]) -> bool {
    fn augment<'a>(
        row: usize,
        options: &[Vec<&'a str>],
        owner: &mut BTreeMap<&'a str, usize>,
        seen: &mut BTreeSet<&'a str>,
    ) -> bool {
        for &value in &options[row] {
            if seen.insert(value) {
                let prev = owner.get(value).copied();
                if prev.is_none_or(|p| augment(p, options, owner, seen)) {
                    owner.insert(value, row);
                    return true;
                }
            }
        }
        false
    }
    let mut owner = BTreeMap::new();
    (0..options.len()).all(|row| augment(row, options, &mut owner, &mut BTreeSet::new()))
}

/// `AET-WO-1201`: the forced-instance check only prunes. Random small tables
/// and specifications are solved with the check and without it; the answers
/// must be identical, except that a `BudgetExceeded` without it may be
/// `Infeasible` with it.
#[cfg(test)]
mod differential {
    use super::*;
    use crate::{I2cBus, Mcu, NetConstraint, Pin, PinSource, SpiBus, SpiDevice, VoltageDomain};

    const SPECS: u32 = 3000;
    const LIMIT: u64 = 20_000;
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const POOL: [&str; 14] = [
        "PA0", "PA1", "PA2", "PA3", "PA4", "PA5", "PA6", "PA7", "PB0", "PB1", "PB2", "PB3", "PA13",
        "PC14",
    ];

    /// xorshift64*, fixed seed: the same specifications on every host.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() >> 33) as usize % n
        }
        fn one_in(&mut self, n: usize) -> bool {
            self.below(n) == 0
        }
    }

    fn table(rng: &mut Rng) -> PinTable {
        let pool = 6 + rng.below(POOL.len() - 5);
        let mut afs: BTreeMap<&str, BTreeMap<String, u8>> = BTreeMap::new();
        for (kind, count) in [(Kind::I2c, 4), (Kind::Spi, 6)] {
            for n in 1..=count {
                for sig in kind.signals() {
                    // One in thirty signals is missing, so an instance is not offered.
                    if rng.one_in(30) {
                        continue;
                    }
                    for _ in 0..1 + rng.below(3) {
                        let pin = POOL[rng.below(pool)];
                        afs.entry(pin).or_default().insert(
                            format!("{}{n}_{sig}", kind.prefix()),
                            1 + rng.below(8) as u8,
                        );
                    }
                }
            }
        }
        PinTable {
            part: "T".into(),
            package: "T".into(),
            source: PinSource {
                repository: "t".into(),
                commit: "t".into(),
                files: BTreeMap::new(),
            },
            pins: POOL[..pool]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let pin = Pin {
                        position: i as u16 + 1,
                        alternate_functions: afs.remove(name).unwrap_or_default(),
                        ..Default::default()
                    };
                    ((*name).to_owned(), pin)
                })
                .collect(),
        }
    }

    fn signal(rng: &mut Rng, net: String, pool: usize) -> Signal {
        // One in fourteen is a hand pin; one in forty of those is not in the table.
        let pin = rng.one_in(14).then(|| {
            if rng.one_in(40) {
                "PZ9".to_owned()
            } else {
                POOL[rng.below(pool)].to_owned()
            }
        });
        Signal { net, pin }
    }

    fn peripheral(rng: &mut Rng, kind: Kind) -> Option<String> {
        rng.one_in(12)
            .then(|| format!("{}{}", kind.prefix(), 1 + rng.below(4)))
    }

    /// The reference specification's declared board (`examples/sentinel-01-minimal.json`).
    const BOARD: &str = r#"{"copper_layers": 2, "layers": [{"name": "F.Cu", "role": "signal", "net": null}, {"name": "B.Cu", "role": "signal", "net": null}], "stackup": {"finished_thickness_um": {"nominal": 1600, "min": 340, "max": 2000, "source": "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %"}, "dielectrics": [{"between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "implied: the finished minimum 340 um less twice the copper maximum 70 um"}], "copper_max_um": {"value": 70, "source": "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)"}, "relative_permittivity_max_permille": {"value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz"}}, "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}}"#;

    fn spec(rng: &mut Rng, pool: usize) -> Specification {
        let mut i2c_buses = Vec::new();
        for i in 0..rng.below(5) {
            let id = format!("I{i}");
            i2c_buses.push(I2cBus {
                peripheral: peripheral(rng, Kind::I2c),
                speed_hz: 400_000,
                pull_up_ohms: 2200,
                domain: "D".into(),
                sda: signal(rng, format!("{id}_sda"), pool),
                scl: signal(rng, format!("{id}_scl"), pool),
                id,
            });
        }
        let mut spi_buses = Vec::new();
        for i in 0..rng.below(8) {
            let id = format!("S{i}");
            spi_buses.push(SpiBus {
                peripheral: peripheral(rng, Kind::Spi),
                speed_hz: 1_000_000,
                domain: "D".into(),
                sck: signal(rng, format!("{id}_sck"), pool),
                mosi: signal(rng, format!("{id}_mosi"), pool),
                miso: signal(rng, format!("{id}_miso"), pool),
                id,
            });
        }
        let mut spi_devices = Vec::new();
        if !spi_buses.is_empty() {
            for i in 0..rng.below(4) {
                let bus = spi_buses[rng.below(spi_buses.len())].id.clone();
                spi_devices.push(SpiDevice {
                    reference: format!("U{i}"),
                    part: "F".into(),
                    supply: "D".into(),
                    bus,
                    cs: signal(rng, format!("U{i}_cs"), pool),
                    pin_capacitance: None,
                    orientation: None,
                });
            }
        }
        let mut net_constraints: Vec<NetConstraint> = i2c_buses
            .iter()
            .flat_map(|b| [&b.sda.net, &b.scl.net])
            .map(|net| NetConstraint {
                net_name: net.clone(),
                max_capacitance_femtofarads: 100_000,
                max_rise_time_nanoseconds: 300,
                impedance_ohms: None,
            })
            .collect();
        net_constraints.extend(spi_buses.iter().flat_map(|b| {
            [&b.sck.net, &b.mosi.net].map(|net| NetConstraint {
                net_name: net.clone(),
                max_capacitance_femtofarads: 30_000,
                max_rise_time_nanoseconds: 20,
                impedance_ohms: None,
            })
        }));
        Specification {
            schema_version: crate::SPECIFICATION_SCHEMA_VERSION,
            board: serde_json::from_str(BOARD).expect("the declared board"),
            net_classes: vec![],
            diff_pairs: vec![],
            power: None,
            accept_lifecycle: Vec::new(),
            acknowledge_unwitnessed: Vec::new(),
            project_name: "t".into(),
            mcu: Mcu {
                part: "T".into(),
                supply: "D".into(),
                hse: None,
                bor_level: 0,
                core_supply: crate::CoreSupply {
                    mode: crate::vos::CoreSupplyMode::Ldo,
                    source: "DS12117 Rev 10 sec. 3.5.1 p. 27".into(),
                },
                supply_source: Some(crate::SupplySource {
                    connector: "J2".into(),
                    pin: "1".into(),
                }),
                reset_switch: false,
                foreign_clock: None,
                reserved_pins: Vec::new(),
            },
            voltage_domains: vec![VoltageDomain {
                id: "D".into(),
                nominal_millivolts: 3300,
                tolerance_permille: 50,
            }],
            i2c_buses,
            spi_buses,
            i2c_devices: vec![],
            spi_devices,
            connectors: vec![
                connector(
                    "J1",
                    "SM06B-SRSS-TB",
                    &["D", "", "", "SWDIO", "SWCLK", "GND"],
                ),
                connector("J2", "SM04B-GHS-TB", &["D", "", "", "GND"]),
            ],
            net_constraints,
            motor_outputs: vec![],
            serial_ports: vec![],
            test_points: vec![],
            pin_devices: vec![],
        }
    }

    /// A connector whose pins name these nets, an empty name being no net.
    fn connector(reference: &str, part: &str, nets: &[&str]) -> crate::connectors::Connector {
        crate::connectors::Connector {
            reference: reference.into(),
            part: part.into(),
            standard: None,
            pins: nets
                .iter()
                .enumerate()
                .map(|(i, n)| crate::connectors::ConnectorPin {
                    pin: (i + 1).to_string(),
                    net: (!n.is_empty()).then(|| (*n).to_string()),
                    offboard: None,
                })
                .collect(),
        }
    }

    fn run(
        spec: &Specification,
        table: &PinTable,
        forced: bool,
    ) -> (Result<SolvedTopology, SolveError>, u64) {
        let mut budget = Budget {
            spent: 0,
            limit: LIMIT,
        };
        let out = solve_inner(spec, table, HASH, &mut budget, forced);
        (out, budget.spent)
    }

    /// `AET-WO-4230`: the kernel-clock refusal cannot be reached from a
    /// specification (every SPI instance is SPI1–SPI6), so it is tested
    /// through the function that decides it. `hsi_ker_ck` runs whatever the
    /// system clock (`AET-WO-4240a`), so an HSE tree's SPI kernel is the
    /// reset tree's.
    #[test]
    fn an_spi_controller_with_no_kernel_clock_is_refused_by_name() {
        let err = spi_controller("B", "SPI7", 20_000_000, &SolvedClock::reset()).unwrap_err();
        assert_eq!(err.kind(), "SpiKernelClockUnavailable", "{err}");
        assert_eq!(err.claimants(), ["B"]);
        let c = spi_controller("B", "SPI6", 20_000_000, &SolvedClock::reset()).unwrap();
        assert_eq!(
            (c.kernel, c.kernel_hz, c.mbr),
            (SpiKernel::HsiKerCk, 64_000_000, 1)
        );
        assert_eq!(
            spi_kernel("SPI2", &SolvedClock::reset()),
            Some((SpiKernel::PerCk, 64_000_000))
        );
    }

    #[test]
    fn timingr_is_synthesized_for_every_mode_on_the_reset_kernel() {
        // The hand values of `silicon`'s tests.
        assert_eq!(
            synthesize_timingr(PpmBudget::HSI, 64_000_000, "Standard100k"),
            Some(0x50D3_2A40)
        );
        assert_eq!(
            synthesize_timingr(PpmBudget::HSI, 64_000_000, "Fast400k"),
            Some(0x10C7_123C)
        );
        assert_eq!(
            synthesize_timingr(PpmBudget::HSI, 64_000_000, "FastPlus1M"),
            Some(0x00B2_0E2D)
        );
        assert_eq!(
            synthesize_timingr(PpmBudget::HSI, 64_000_000, "HighSpeed3M4"),
            None
        );
        // Below DS12117 Table 93's 17 MHz at HSI's slowest, no Fm+ word.
        assert_eq!(
            synthesize_timingr(PpmBudget::HSI, 17_000_000, "FastPlus1M"),
            None
        );
        let reset = SolvedClock::reset();
        for p in ["I2C1", "I2C2", "I2C3", "I2C4"] {
            assert_eq!(i2c_kernel_hz(&reset, p), Some(64_000_000));
            assert_eq!(i2c_controller("", &reset, p).unwrap().timingr.len(), 3);
        }
        assert!(i2c_controller("", &reset, "I2C5").is_err());
    }

    /// `4240` F-PANIC: H0 at 8 MHz (`sys_ck` the HSE, every prescaler 1)
    /// gives every I2C controller an 8 MHz kernel. At the source's slowest
    /// that is below Fast-mode's least kernel, 8 MHz (DS12117 Rev 10 Table
    /// 93), the first mode in order to fail; Fast-mode Plus's 17 MHz fails
    /// too. The solver refuses it by name where it once panicked.
    #[test]
    fn an_8_mhz_h0_clock_is_a_named_refusal_not_a_panic() {
        let h0 = SolvedClock {
            source: "HSE".into(),
            sysclk_hz: 8_000_000,
            hse_hz: 8_000_000,
            ..SolvedClock::reset()
        };
        let err = i2c_controller("I2C_BUS_0", &h0, "I2C1").unwrap_err();
        assert_eq!(err.kind(), "NoI2cTiming");
        assert_eq!(
            err,
            SolveError::NoI2cTiming {
                bus: "I2C_BUS_0".into(),
                peripheral: "I2C1".into(),
                mode: "Fast400k".into(),
                kernel_hz: 8_000_000,
            }
        );
        assert_eq!(err.claimants(), ["I2C_BUS_0"]);
        assert!(i2c_controller("I2C_BUS_0", &SolvedClock::reset(), "I2C1").is_ok());
    }

    /// `AET-WO-4240e`: the switch's PLL1 from the HSI. `DIVM1` 5 (4 gives
    /// 16.24 MHz at the band's top, over `PLL1RGE` `11`'s 16), reference
    /// 12,468,556–12,991,795 Hz in `RGE` `11`; `DIVN1` 16 (VCO
    /// 199,496,908–207,868,723 Hz, 15 under 192 MHz at the bottom); `DIVQ1` 2
    /// (103.9 MHz; 1 is 207.9 MHz, over 200); `DIVQ1EN` alone, `PLLSRC` `00`,
    /// `DIVP1` and `DIVR1` at their reset 1.
    #[test]
    fn the_switch_words_hold_across_the_hsi_band() {
        assert_eq!(hsi_switch_words(), Some([0x50, 0x0002_000C, 0x0101_020F]));
    }

    #[test]
    fn the_forced_instance_check_only_prunes() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        // ok, infeasible, budget, other refusal; then budget -> infeasible.
        let mut seen = [0u32; 4];
        let (mut flipped, mut nodes_off, mut nodes_on) = (0u32, 0u64, 0u64);
        let mut kinds = BTreeMap::new();
        for n in 0..SPECS {
            let table = table(&mut rng);
            let pool = table.pins.len();
            let spec = spec(&mut rng, pool);
            let (off, spent_off) = run(&spec, &table, false);
            let (on, spent_on) = run(&spec, &table, true);
            nodes_off += spent_off;
            nodes_on += spent_on;
            assert!(spent_on <= spent_off, "spec {n}: the check added nodes");
            match (&off, &on) {
                (Err(SolveError::BudgetExceeded { .. }), Err(SolveError::Infeasible { .. })) => {
                    flipped += 1;
                }
                _ => assert_eq!(off, on, "spec {n}: the check changed the answer"),
            }
            if let Err(e) = &off {
                *kinds.entry(e.kind()).or_insert(0u32) += 1;
            }
            seen[match &off {
                Ok(_) => 0,
                Err(SolveError::Infeasible { .. }) => 1,
                Err(SolveError::BudgetExceeded { .. }) => 2,
                Err(_) => 3,
            }] += 1;
        }
        eprintln!(
            "differential: {SPECS} specs; without the check ok {}, infeasible {}, budget {}, other refusal {}; \
             budget->infeasible {flipped}; nodes {nodes_off} without, {nodes_on} with",
            seen[0], seen[1], seen[2], seen[3]
        );
        eprintln!("refusals: {kinds:?}");
        // The generator must reach each outcome, or the test proves nothing.
        assert!(
            [0, 1, 3].iter().all(|&i| seen[i] >= 50),
            "outcomes: {seen:?}"
        );
    }

    /// The 1200 reproducers' shape on a synthetic table: six SPI buses, six
    /// instances, and two instances whose only `MISO` is the same pin. Pin
    /// matching unions over instances and instance matching ignores pins, so
    /// both pass; the forced instances do not.
    #[test]
    fn a_shared_only_pin_is_refused_at_once_not_after_the_budget() {
        let mut pins: BTreeMap<String, Pin> = BTreeMap::new();
        let mut put = |name: String, af: String| {
            let n = pins.len() as u16 + 1;
            pins.entry(name)
                .or_insert(Pin {
                    position: n,
                    alternate_functions: BTreeMap::new(),
                    ..Default::default()
                })
                .alternate_functions
                .insert(af, 5);
        };
        for n in 1..=6 {
            put(format!("PA{n}"), format!("SPI{n}_SCK"));
            put(format!("PB{n}"), format!("SPI{n}_MOSI"));
            let miso = if n <= 2 {
                "PC0".to_owned()
            } else {
                format!("PC{n}")
            };
            put(miso, format!("SPI{n}_MISO"));
        }
        // The union of the MISO pins is six, so pin matching passes.
        put("PC7".to_owned(), "SPI3_MISO".to_owned());
        let table = PinTable {
            part: "T".into(),
            package: "T".into(),
            source: PinSource {
                repository: "t".into(),
                commit: "t".into(),
                files: BTreeMap::new(),
            },
            pins,
        };
        let mut rng = Rng(1);
        let mut spec = spec(&mut rng, table.pins.len());
        spec.i2c_buses.clear();
        spec.spi_devices.clear();
        spec.spi_buses = (0..6)
            .map(|i| {
                let sig = |s: &str| Signal {
                    net: format!("S{i}_{s}"),
                    pin: None,
                };
                SpiBus {
                    id: format!("S{i}"),
                    peripheral: None,
                    speed_hz: 1_000_000,
                    domain: "D".into(),
                    sck: sig("sck"),
                    mosi: sig("mosi"),
                    miso: sig("miso"),
                }
            })
            .collect();
        spec.net_constraints = spec
            .spi_buses
            .iter()
            .flat_map(|b| [&b.sck.net, &b.mosi.net])
            .map(|net| NetConstraint {
                net_name: net.clone(),
                max_capacitance_femtofarads: 30_000,
                max_rise_time_nanoseconds: 20,
                impedance_ohms: None,
            })
            .collect();
        let (on, spent_on) = run(&spec, &table, true);
        let (off, spent_off) = run(&spec, &table, false);
        let Err(SolveError::Infeasible { buses }) = &on else {
            panic!("with the check: {on:?}");
        };
        assert_eq!(buses.len(), 6);
        eprintln!("shared-only-pin: {spent_on} nodes with, {spent_off} without ({off:?})");
        assert!(spent_on < spent_off);
        // Between the two, only the check finishes: the flip the work order allows.
        let tight = |forced| {
            let mut budget = Budget {
                spent: 0,
                limit: 300,
            };
            solve_inner(&spec, &table, HASH, &mut budget, forced)
        };
        assert!(matches!(tight(true), Err(SolveError::Infeasible { .. })));
        assert!(matches!(
            tight(false),
            Err(SolveError::BudgetExceeded { .. })
        ));
    }
}
