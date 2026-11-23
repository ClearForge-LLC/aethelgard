//! Off-board pins (`AET-WO-4300i`, as ruled on `4300h` D0, D1 (a), D5 and
//! F7): validation step 26, and the facts the solver's candidate order reads.
//!
//! - **The motor pull** (D0 `external`): every motor output declares a
//!   resistor from its net to `GND` (`MotorPullNotDown`), a resistor row of
//!   the power parts table (`PowerPartUnknown`), whose declared ohms lie
//!   inside the MCU row's RPD range, `io.weak_pull_down_ohms`
//!   (`MotorPullOutsideRpd`, DS12117 Rev 10 Table 49 = Table 146, the held
//!   source ruled on V). The declared ohms are judged, not a tolerance band:
//!   most motor boards declare no temperature. No value outside the range
//!   is admitted, and nothing names an escape.
//! - **Declared reservations** (F7): `mcu.reserved_pins` names each pin
//!   once (`ReservedPinTwice`) and none the engine already reserves
//!   (`ReservedPinEngineReserved`, [`crate::reserved_pins`]); whether each is
//!   a pin of the table is `solve`'s (`UnknownPin`), which holds the table.
//! - **Off the reset pulls** (adversarial pass F2): a motor output's hand
//!   pin holds no internal pull from reset (`MotorPinResetPull`,
//!   [`crate::pin_devices::MCU_RESET_PULL_UPS`] and `MCU_RESET_PULL_DOWNS`,
//!   RM0433 Rev 9 p. 542), and the solver never offers it one. An idle-high
//!   output validates (F3 reverted by the PM, 2026-11-23): D0 holds every
//!   motor line low from power-up, whatever its idle, and the timer drives
//!   the idle from `start_outputs`.
//! - **FT at connectors** (D1 (a), D5): a net some `connectors[].pins[].net`
//!   names is connector-facing. A hand pin on one must have a held `FT`
//!   structure (`ConnectorPinNotFt`), and one with no held structure is
//!   `McuPinStructureUnheld`. The solver offers such a net FT pins only, the
//!   same bar ([`AtConnector`], adversarial pass F1): every other candidate
//!   is withheld in every branch, and a refusal for want of a pin names it.
//!   Test points and `power.input` are not connectors here.
//!
//! The MCU row is read once, and refused at `mcu.part` (`PowerPartUnknown`)
//! only when something needs it: a motor output, or a claimant (hand or
//! solved) on a connector-facing net (adversarial pass F8).

use std::collections::BTreeSet;

use crate::power_parts::{self, McuIo};
use crate::{IrValidationError, Specification};

/// Each refusal of step 26 is its own kind ([`OffboardRefusal::kind`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffboardKind {
    MotorPullNotDown,
    MotorPullOutsideRpd,
    /// A motor pull's part that is not a resistor row, or an `mcu.part`
    /// with no MCU row where one is needed.
    PowerPartUnknown,
    ReservedPinTwice,
    ReservedPinEngineReserved,
    ConnectorPinNotFt,
    McuPinStructureUnheld,
    MotorPinResetPull,
}

impl OffboardKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::MotorPullNotDown => "MotorPullNotDown",
            Self::MotorPullOutsideRpd => "MotorPullOutsideRpd",
            Self::PowerPartUnknown => "PowerPartUnknown",
            Self::ReservedPinTwice => "ReservedPinTwice",
            Self::ReservedPinEngineReserved => "ReservedPinEngineReserved",
            Self::ConnectorPinNotFt => "ConnectorPinNotFt",
            Self::McuPinStructureUnheld => "McuPinStructureUnheld",
            Self::MotorPinResetPull => "MotorPinResetPull",
        }
    }
}

/// A step-26 refusal: its kind, what it names (`id`: the output, the pin,
/// the net or the part), where (`path`), and why.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{path}`: {message}")]
pub struct OffboardRefusal {
    pub kind: OffboardKind,
    pub id: String,
    pub path: String,
    pub message: String,
}

impl OffboardRefusal {
    pub fn kind(&self) -> &'static str {
        self.kind.name()
    }
}

fn refuse(
    kind: OffboardKind,
    id: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> IrValidationError {
    IrValidationError::Offboard(OffboardRefusal {
        kind,
        id: id.into(),
        path: path.into(),
        message: message.into(),
    })
}

/// Every net a connector pin names, the connector-facing nets.
pub(crate) fn connector_facing(spec: &Specification) -> BTreeSet<&str> {
    spec.declared_connectors()
        .iter()
        .flat_map(|c| &c.pins)
        .filter_map(|p| p.net.as_deref())
        .collect()
}

/// Whether a held structure is of the `FT` family (`FT`, `FT_h`, `FT_fha`,
/// …): the family is the structure before its first `_`, as A1 reads it.
fn is_ft(structure: &str) -> bool {
    structure.split('_').next() == Some("FT")
}

/// The MCU row's I/O facts, or the refusal at `mcu.part` when `mcu.part`
/// has no MCU row (`PowerPartUnknown`): the one home of both, for step 26
/// and the solver.
fn mcu_io(spec: &Specification) -> Result<&'static McuIo, IrValidationError> {
    match power_parts::table()?.get(&spec.mcu.part).map(|r| &r.kind) {
        Some(power_parts::Kind::Mcu(row)) => Ok(&row.io),
        _ => Err(refuse(
            OffboardKind::PowerPartUnknown,
            &spec.mcu.part,
            "mcu.part",
            format!(
                "`{}` has no MCU row in the power parts table, and a motor output's pull and every pin of a net at a connector, by hand or solved, are judged against it",
                spec.mcu.part
            ),
        )),
    }
}

/// The claimant nets a connector pin names, solved or by hand.
fn facing_claims(spec: &Specification) -> BTreeSet<&str> {
    let facing = connector_facing(spec);
    claimants(spec)
        .into_iter()
        .map(|(_, net, _)| net)
        .filter(|net| facing.contains(net))
        .collect()
}

/// What the solver may offer a net at a connector (D1 (a), adversarial pass
/// F1): an FT pin only, the bar a hand pin meets (`ConnectorPinNotFt`). A
/// pin with no held structure counts as non-FT. Every other candidate is
/// withheld in every branch, and a refusal for want of a pin names it
/// ([`AtConnector::note`]).
pub(crate) struct AtConnector<'s> {
    facing: BTreeSet<&'s str>,
    /// The MCU row's I/O facts, read when some claimant faces a connector.
    io: Option<&'static McuIo>,
}

impl<'s> AtConnector<'s> {
    /// Refused at `mcu.part` (`PowerPartUnknown`) when a claimant faces a
    /// connector and the part has no MCU row, which step 26 refused first.
    pub(crate) fn new(spec: &'s Specification) -> Result<Self, IrValidationError> {
        let facing = facing_claims(spec);
        let io = match facing.is_empty() {
            true => None,
            false => Some(mcu_io(spec)?),
        };
        Ok(Self { facing, io })
    }

    /// Whether `net` is a claimant's net a connector pin names.
    pub(crate) fn faces(&self, net: &str) -> bool {
        self.facing.contains(net)
    }

    /// The held structure of `pin`, if any.
    fn structure(&self, pin: &str) -> Option<&'static str> {
        self.io
            .and_then(|io| io.pins.get(pin))
            .map(|(s, _)| s.as_str())
    }

    /// Whether `pin` has a held FT structure.
    pub(crate) fn ft(&self, pin: &str) -> bool {
        self.structure(pin).is_some_and(is_ft)
    }

    /// Whether the solver may offer `pin` to `net`.
    pub(crate) fn offers(&self, net: &str, pin: &str) -> bool {
        !self.faces(net) || self.ft(pin)
    }

    /// What a refusal adds for the `pins` withheld from `net` as not FT, each
    /// with its held structure, once, in key order: nothing when none is.
    pub(crate) fn note<'p>(&self, net: &str, pins: impl IntoIterator<Item = &'p str>) -> String {
        let withheld: BTreeSet<&str> = pins.into_iter().filter(|p| !self.offers(net, p)).collect();
        if withheld.is_empty() {
            return String::new();
        }
        let named: Vec<String> = withheld
            .iter()
            .map(|p| match self.structure(p) {
                Some(s) => format!("`{p}` ({s})"),
                None => format!("`{p}` (no held structure)"),
            })
            .collect();
        format!(
            "; {} withheld as not FT at a connector (`AET-WO-4300i` D1)",
            named.join(", ")
        )
    }
}

/// Every claimant of the specification with its pin's path, its net and its
/// hand pin (`None` for the solver), in document order: the buses' lines,
/// the chip selects, the motor outputs, the serial ports, then each pin
/// device's signals and ADC inputs.
type Claimant<'s> = (String, &'s str, Option<&'s str>);

fn claimants(spec: &Specification) -> Vec<Claimant<'_>> {
    let mut out: Vec<Claimant<'_>> = Vec::new();
    for (i, b) in spec.i2c_buses.iter().enumerate() {
        for (k, s) in [("sda", &b.sda), ("scl", &b.scl)] {
            out.push((format!("i2c_buses[{i}].{k}.pin"), &s.net, s.pin.as_deref()));
        }
    }
    for (i, b) in spec.spi_buses.iter().enumerate() {
        for (k, s) in [("sck", &b.sck), ("mosi", &b.mosi), ("miso", &b.miso)] {
            out.push((format!("spi_buses[{i}].{k}.pin"), &s.net, s.pin.as_deref()));
        }
    }
    for (i, d) in spec.declared_spi_devices().iter().enumerate() {
        out.push((
            format!("spi_devices[{i}].cs.pin"),
            &d.cs.net,
            d.cs.pin.as_deref(),
        ));
    }
    for (i, m) in spec.motor_outputs.iter().enumerate() {
        out.push((
            format!("motor_outputs[{i}].pin"),
            &m.label,
            m.pin.as_deref(),
        ));
    }
    for (i, u) in spec.serial_ports.iter().enumerate() {
        for (k, s) in [("tx", &u.tx), ("rx", &u.rx)] {
            out.push((
                format!("serial_ports[{i}].{k}.pin"),
                &s.net,
                s.pin.as_deref(),
            ));
        }
    }
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        for (k, s) in d.signals.iter().enumerate() {
            out.push((
                format!("pin_devices[{i}].signals[{k}].pin"),
                &s.net,
                s.pin.as_deref(),
            ));
        }
        for (k, a) in d.analog.iter().enumerate() {
            out.push((
                format!("pin_devices[{i}].analog[{k}].pin"),
                &a.net,
                a.pin.as_deref(),
            ));
        }
    }
    out
}

/// Step 26, in this order: the reservations, the MCU row where needed, each
/// motor output's pull and hand pin, then each hand pin on a
/// connector-facing net.
pub(crate) fn validate(spec: &Specification) -> Result<(), IrValidationError> {
    use OffboardKind as K;
    let engine = crate::reserved_pins(spec.mcu.hse.as_ref());
    let mut seen = BTreeSet::new();
    for (i, r) in spec.mcu.reserved_pins.iter().enumerate() {
        let at = format!("mcu.reserved_pins[{i}].pin");
        if !seen.insert(r.pin.as_str()) {
            return Err(refuse(
                K::ReservedPinTwice,
                &r.pin,
                at,
                format!("`{}` is reserved twice: name a pin once", r.pin),
            ));
        }
        if engine.contains(&r.pin.as_str()) {
            return Err(refuse(
                K::ReservedPinEngineReserved,
                &r.pin,
                at,
                format!(
                    "`{}` is reserved by the engine already (the debug port, the LSE, or the HSE `mcu.hse` declares): a reservation has one home",
                    r.pin
                ),
            ));
        }
    }
    let facing = connector_facing(spec);
    let at: Vec<_> = claimants(spec)
        .into_iter()
        .filter(|(_, net, _)| facing.contains(net))
        .collect();
    if spec.motor_outputs.is_empty() && at.is_empty() {
        return Ok(());
    }
    let io = mcu_io(spec)?;
    let pp = power_parts::table()?;
    let rpd = &io.weak_pull_down_ohms;
    for (i, m) in spec.motor_outputs.iter().enumerate() {
        let r = &m.pull.resistor;
        let at = format!("motor_outputs[{i}].pull.resistor");
        if r.to != crate::GND {
            return Err(refuse(
                K::MotorPullNotDown,
                &m.label,
                format!("{at}.to"),
                format!(
                    "`{}`'s pull goes to `{}`: a motor line is pulled down, to `GND` (`AET-WO-4300i`, ruled I)",
                    m.label, r.to
                ),
            ));
        }
        if !matches!(
            pp.get(&r.part).map(|row| &row.kind),
            Some(power_parts::Kind::Resistor(_))
        ) {
            return Err(refuse(
                K::PowerPartUnknown,
                &m.label,
                format!("{at}.part"),
                format!(
                    "`{}` is not a resistor row of the power parts table",
                    r.part
                ),
            ));
        }
        let ohms = i64::from(r.ohms);
        if ohms < rpd.min || ohms > rpd.max {
            return Err(refuse(
                K::MotorPullOutsideRpd,
                &m.label,
                format!("{at}.ohms"),
                format!(
                    "`{}`'s pull is {ohms} ohm, outside RPD's {} to {} ohm ({}): the external pull reproduces the internal one (D0, ruled on V)",
                    m.label,
                    rpd.min,
                    rpd.max,
                    pp.cite(&rpd.at)
                ),
            ));
        }
        if let Some(pin) = m.pin.as_deref().filter(|p| {
            crate::pin_devices::MCU_RESET_PULL_UPS.contains(p)
                || crate::pin_devices::MCU_RESET_PULL_DOWNS.contains(p)
        }) {
            return Err(refuse(
                K::MotorPinResetPull,
                &m.label,
                format!("motor_outputs[{i}].pin"),
                format!(
                    "`{}` is pinned on `{pin}`, which holds an internal pull from reset to the first write (RM0433 Rev 9 p. 542: PA13, PA15 and PB4 pulled up, PA14 down): a motor line carries no internal pull (`AET-WO-4300i` D0), and a pull-up fights the declared pull-down from power-up until the timer takes the pin; pin it elsewhere",
                    m.label
                ),
            ));
        }
    }
    for (path, net, pin) in at {
        let Some(pin) = pin else { continue };
        let ends: Vec<String> = spec
            .declared_connectors()
            .iter()
            .flat_map(|c| {
                c.pins
                    .iter()
                    .filter(|p| p.net.as_deref() == Some(net))
                    .map(|p| format!("{}.{}", c.reference, p.pin))
            })
            .collect();
        let ends = ends.join(", ");
        let Some((structure, at)) = io.pins.get(pin) else {
            return Err(refuse(
                K::McuPinStructureUnheld,
                net,
                path,
                format!(
                    "`{pin}` holds `{net}`, which reaches connector pin {ends}, and has no held I/O structure (DS12117 Rev 10 Table 8 holds the LQFP144's I/O pins): no structure is assumed"
                ),
            ));
        };
        if !is_ft(structure) {
            return Err(refuse(
                K::ConnectorPinNotFt,
                net,
                path,
                format!(
                    "`{net}` reaches connector pin {ends} on `{pin}`, which is {structure} ({}): a pin at a connector is FT (`AET-WO-4300h` D1 (a), D5); pin it on an FT pin or leave it to the solver",
                    pp.cite(at)
                ),
            ));
        }
    }
    Ok(())
}
