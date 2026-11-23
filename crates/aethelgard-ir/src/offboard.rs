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
//! - **FT at connectors** (D1 (a), D5): a net some `connectors[].pins[].net`
//!   names is connector-facing. A hand pin on one must have a held `FT`
//!   structure (`ConnectorPinNotFt`), and one with no held structure is
//!   `McuPinStructureUnheld`; the solver tries FT pins first for such a net
//!   ([`ft_first`]). Test points and `power.input` are not connectors here.
//!
//! The MCU row is read once, and refused at `mcu.part` (`PowerPartUnknown`)
//! only when something needs it: a motor output, or a hand pin on a
//! connector-facing net.

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

/// The pins of `part`'s MCU row whose held structure is `FT`; none for a part
/// with no MCU row, so every pin then counts as non-FT.
pub(crate) fn ft_pins(part: &str) -> Result<BTreeSet<&'static str>, IrValidationError> {
    let table = power_parts::table()?;
    Ok(match table.get(part).map(|r| &r.kind) {
        Some(power_parts::Kind::Mcu(m)) => {
            m.io.pins
                .iter()
                .filter(|(_, (s, _))| is_ft(s))
                .map(|(p, _)| p.as_str())
                .collect()
        }
        _ => BTreeSet::new(),
    })
}

/// `items` stably partitioned when `net` is connector-facing: those on an FT
/// pin first in their order, then the rest in theirs (D1 (a)); unchanged
/// otherwise.
pub(crate) fn ft_first<T>(
    items: Vec<T>,
    net: &str,
    facing: &BTreeSet<&str>,
    ft: &BTreeSet<&str>,
    pin: impl Fn(&T) -> &str,
) -> Vec<T> {
    if !facing.contains(net) {
        return items;
    }
    let (mut first, rest): (Vec<T>, Vec<T>) = items.into_iter().partition(|t| ft.contains(pin(t)));
    first.extend(rest);
    first
}

/// The facts [`ft_first`] reads for one specification: its connector-facing
/// nets and its MCU row's FT pins.
pub(crate) struct FtOrder<'s> {
    facing: BTreeSet<&'s str>,
    ft: BTreeSet<&'static str>,
}

impl<'s> FtOrder<'s> {
    pub(crate) fn new(spec: &'s Specification) -> Result<Self, IrValidationError> {
        Ok(Self {
            facing: connector_facing(spec),
            ft: ft_pins(&spec.mcu.part)?,
        })
    }

    /// `items` in the order `net` tries them ([`ft_first`]).
    pub(crate) fn order<T>(&self, net: &str, items: Vec<T>, pin: impl Fn(&T) -> &str) -> Vec<T> {
        ft_first(items, net, &self.facing, &self.ft, pin)
    }
}

/// Every hand pin of the specification with its net and its path, in
/// document order: the buses' lines, the chip selects, the motor outputs,
/// the serial ports, then each pin device's signals and ADC inputs.
fn hand_pins(spec: &Specification) -> Vec<(String, &str, &str)> {
    let mut out: Vec<(String, &str, &str)> = Vec::new();
    for (i, b) in spec.i2c_buses.iter().enumerate() {
        for (k, s) in [("sda", &b.sda), ("scl", &b.scl)] {
            if let Some(p) = &s.pin {
                out.push((format!("i2c_buses[{i}].{k}.pin"), &s.net, p));
            }
        }
    }
    for (i, b) in spec.spi_buses.iter().enumerate() {
        for (k, s) in [("sck", &b.sck), ("mosi", &b.mosi), ("miso", &b.miso)] {
            if let Some(p) = &s.pin {
                out.push((format!("spi_buses[{i}].{k}.pin"), &s.net, p));
            }
        }
    }
    for (i, d) in spec.declared_spi_devices().iter().enumerate() {
        if let Some(p) = &d.cs.pin {
            out.push((format!("spi_devices[{i}].cs.pin"), &d.cs.net, p));
        }
    }
    for (i, m) in spec.motor_outputs.iter().enumerate() {
        if let Some(p) = &m.pin {
            out.push((format!("motor_outputs[{i}].pin"), &m.label, p));
        }
    }
    for (i, u) in spec.serial_ports.iter().enumerate() {
        for (k, s) in [("tx", &u.tx), ("rx", &u.rx)] {
            if let Some(p) = &s.pin {
                out.push((format!("serial_ports[{i}].{k}.pin"), &s.net, p));
            }
        }
    }
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        for (k, s) in d.signals.iter().enumerate() {
            if let Some(p) = &s.pin {
                out.push((format!("pin_devices[{i}].signals[{k}].pin"), &s.net, p));
            }
        }
        for (k, a) in d.analog.iter().enumerate() {
            if let Some(p) = &a.pin {
                out.push((format!("pin_devices[{i}].analog[{k}].pin"), &a.net, p));
            }
        }
    }
    out
}

/// Step 26, in this order: the reservations, the MCU row where needed, each
/// motor output's pull, then each hand pin on a connector-facing net.
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
    let hands: Vec<_> = hand_pins(spec)
        .into_iter()
        .filter(|(_, net, _)| facing.contains(net))
        .collect();
    if spec.motor_outputs.is_empty() && hands.is_empty() {
        return Ok(());
    }
    let pp = power_parts::table()?;
    let Some(power_parts::Kind::Mcu(row)) = pp.get(&spec.mcu.part).map(|r| &r.kind) else {
        return Err(refuse(
            K::PowerPartUnknown,
            &spec.mcu.part,
            "mcu.part",
            format!(
                "`{}` has no MCU row in the power parts table, and a motor output's pull or a hand pin at a connector is judged against it",
                spec.mcu.part
            ),
        ));
    };
    let io: &McuIo = &row.io;
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
    }
    for (path, net, pin) in hands {
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
