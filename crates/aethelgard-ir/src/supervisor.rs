//! The reset's falling threshold against every supply-bounded device on the
//! rail it supervises (`AET-WO-4300k` (9), ruled 2026-11-23: build both; a
//! board takes Path A or Path B).
//!
//! - **The rule** (`AET-WO-4310` (2), ruled D1 (B)): every powered part is
//!   judged against its own rail's monitor ([`powered`], [`judged`]). The MCU
//!   may run down to the reset's falling minimum: the selected BOR level's
//!   (DS12117 Table 16 = Table 115), or a declared supervisor's VIT− minimum.
//!   A part on the MCU's rail whose minimum is above it may run out of its
//!   range before the reset holds ([`floors_above_reset`]); a part on any
//!   other rail is named, since no monitor holds that rail's consumers, and
//!   so is a part with no held minimum ([`named`]). `verify --fab-out`
//!   refuses a package with either (ruled D12; ruled 3, 2026-11-27T07:42Z);
//!   the radio line names a module's case.
//! - **Path A** is a wide-range part, under BOR3's 2.54 V.
//! - **Path B** is a declared supervisor on `NRST`, judged by [`check`]: its
//!   falling minimum at or above every I2C or SPI device's VCC minimum on
//!   its rail (bus devices only, as before `4310`; decision-needed), and its
//!   rising maximum (VIT− maximum plus its hysteresis, typical only, ruled D6)
//!   under the rail's minimum at its pin (the solved minimum less the rail's
//!   drop limit), so it never resets a healthy board.

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::power::{self, Placed, Refusal, Resistor};
use crate::power_parts::Kind;
use crate::{IrValidationError, Specification};

/// A voltage supervisor on `NRST` (`mcu.reset_supervisor`): its part, the
/// open-drain output's pull-up to the MCU's supply, and its supply's bypass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct ResetSupervisor {
    pub reference: String,
    pub part: String,
    pub pull_up: Resistor,
    pub bypass: Placed,
}

crate::object_only!(ResetSupervisor);

/// A rail monitor (`power.rail_monitors`, `AET-WO-4413` (2), ruled D2): a
/// push-pull supervisor powered from `rail`, a rail other than the MCU's,
/// whose reset holds each part that names it in reset or off while the rail
/// is under the part's range: a radio module whose `enable.monitor` names
/// it, a translator whose `OE` control's `monitor` does. It credits only the
/// parts it drives ([`judged`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct RailMonitor {
    pub reference: String,
    pub part: String,
    pub rail: String,
    pub bypass: Placed,
}

crate::object_only!(RailMonitor);

/// A rail monitor's reset net, every pin it drives on it.
pub fn reset_net(reference: &str) -> String {
    format!("{reference}_RESET")
}

/// Each part a rail monitor holds, as [`powered`] names it, with the
/// monitor: every radio module whose enable names it; every translator side
/// on the monitor's rail whose `OE` names it; the monitor itself, which
/// holds its own output. A name that matches no declared monitor holds
/// nothing ([`check`] refuses it).
fn held(spec: &Specification) -> Vec<(String, &RailMonitor)> {
    let Some(p) = &spec.power else {
        return Vec::new();
    };
    let find = |r: &str| p.rail_monitors.iter().find(|m| m.reference == r);
    let mut out: Vec<(String, &RailMonitor)> = p
        .rail_monitors
        .iter()
        .map(|m| (m.reference.clone(), m))
        .collect();
    for m in spec.declared_radio_modules() {
        if let Some(mon) = m.enable.monitor.as_deref().and_then(find) {
            out.push((m.reference.clone(), mon));
        }
    }
    for t in p.translators() {
        let Some(mon) = t
            .controls
            .iter()
            .filter(|c| c.pin == "OE")
            .find_map(|c| c.monitor.as_deref())
            .and_then(find)
        else {
            continue;
        };
        for (side, rail) in [("A", t.a_domain), ("B", t.b_domain)] {
            if *rail == mon.rail {
                out.push((format!("{}.{side}", t.reference), mon));
            }
        }
    }
    out
}

/// One device whose VCC minimum is above the reset's falling minimum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Floor {
    pub device: String,
    pub part: String,
    pub vcc_min_millivolts: u32,
    pub reset_min_millivolts: u32,
    /// What sets the reset, with its citation.
    pub reset: String,
}

impl Floor {
    pub fn text(&self) -> String {
        format!(
            "{} ({}): its VCC minimum of {} mV is above the reset's falling minimum of {} mV ({}), so it may run out of its range before the reset holds (AET-WO-4300k (9))",
            self.device, self.part, self.vcc_min_millivolts, self.reset_min_millivolts, self.reset
        )
    }
}

fn refuse(kind: Refusal, id: &str, detail: String, citation: String) -> IrValidationError {
    IrValidationError::Power {
        kind,
        id: id.to_owned(),
        detail,
        citation,
    }
}

/// The reset's falling minimum, mV, and what sets it with its citation.
pub fn reset_floor(spec: &Specification) -> Result<(u32, String), IrValidationError> {
    reset_floor_against(spec, crate::power_parts::table()?)
}

/// [`reset_floor`] against `table`: the embedded one in every caller but a
/// test, which holds a row with one limit changed.
pub fn reset_floor_against(
    spec: &Specification,
    table: &crate::power_parts::PowerPartsTable,
) -> Result<(u32, String), IrValidationError> {
    if let Some(s) = &spec.mcu.reset_supervisor {
        let row = table.get(&s.part).ok_or_else(|| {
            refuse(
                Refusal::PowerPartUnknown,
                &s.reference,
                format!("part `{}` is not a row of the power parts table", s.part),
                "crates/aethelgard-ir/data/power-parts.json".to_owned(),
            )
        })?;
        let Kind::Supervisor(v) = &row.kind else {
            return Err(refuse(
                Refusal::PowerPartUnknown,
                &s.reference,
                format!(
                    "part `{}` is a {} row, not a supervisor",
                    s.part,
                    row.kind.name()
                ),
                "crates/aethelgard-ir/data/power-parts.json".to_owned(),
            ));
        };
        let min = u32::try_from(v.falling_millivolts.range.min).unwrap_or(0);
        return Ok((
            min,
            format!(
                "{}'s VIT− minimum, {}",
                s.reference,
                table.cite(&v.falling_millivolts.at)
            ),
        ));
    }
    let row = table.get(&spec.mcu.part).ok_or_else(|| {
        refuse(
            Refusal::PowerPartUnknown,
            &spec.mcu.part,
            "the MCU has no row".to_owned(),
            "crates/aethelgard-ir/data/power-parts.json".to_owned(),
        )
    })?;
    let Kind::Mcu(m) = &row.kind else {
        return Err(refuse(
            Refusal::PowerPartUnknown,
            &spec.mcu.part,
            "the MCU's row is not an mcu row".to_owned(),
            "crates/aethelgard-ir/data/power-parts.json".to_owned(),
        ));
    };
    let level = usize::from(spec.mcu.bor_level);
    let l = m.bor_falling_min_millivolts.get(level).ok_or_else(|| {
        refuse(
            Refusal::PowerValueInvalid,
            &spec.mcu.part,
            format!("mcu.bor_level {level} has no row"),
            "crates/aethelgard-ir/data/power-parts.json".to_owned(),
        )
    })?;
    Ok((
        u32::try_from(l.value).unwrap_or(0),
        format!("BOR level {level}'s falling minimum, {}", table.cite(&l.at)),
    ))
}

/// Each I2C or SPI device on the MCU's supply, its part, and its VCC
/// minimum at its bus's speed; `None` where its rows hold none at it.
fn devices(spec: &Specification) -> Result<Vec<(String, String, Option<u32>)>, IrValidationError> {
    let parts = crate::parts::table()?;
    let supply = &spec.mcu.supply;
    let mut out = Vec::new();
    let i2c = spec
        .declared_i2c_devices()
        .iter()
        .map(|d| (&d.reference, &d.part, &d.supply, &d.bus, true));
    let spi = spec
        .declared_spi_devices()
        .iter()
        .map(|d| (&d.reference, &d.part, &d.supply, &d.bus, false));
    for (reference, part, on, bus, is_i2c) in i2c.chain(spi) {
        if on != supply {
            continue;
        }
        let Some(p) = parts.get(part) else { continue };
        let hz = if is_i2c {
            spec.i2c_buses
                .iter()
                .find(|b| &b.id == bus)
                .map(|b| b.speed_hz)
        } else {
            spec.spi_buses
                .iter()
                .find(|b| &b.id == bus)
                .map(|b| b.speed_hz)
        };
        let min = p.supply_min_at(u64::from(hz.unwrap_or(u32::MAX)));
        out.push((reference.clone(), part.clone(), min));
    }
    Ok(out)
}

/// What the floor rule says of one powered part (`AET-WO-4310` (2), ruled
/// D1 (B)): each part is judged against its own rail's monitor, the reset
/// that holds that rail's consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// Its minimum is at or below the monitor's falling minimum.
    Pass,
    /// Its minimum is above it: [`Floor`].
    Floor(Floor),
    /// Not passed: no monitor holds its rail, or no minimum is held for it.
    Named(Named),
    /// Judged by another rule, named: a battery-fed regulator's input, by
    /// the headroom and the battery chain.
    Elsewhere(String),
}

/// One part named, never passed (`AET-WO-4310` (2)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub device: String,
    pub part: String,
    pub rail: String,
    /// Why nothing passes it.
    pub why: String,
}

impl Named {
    pub fn text(&self) -> String {
        format!(
            "{} ({}) on `{}`: {} (AET-WO-4310 (2))",
            self.device, self.part, self.rail, self.why
        )
    }
}

/// One powered part, its rail, and its held supply minimum with its
/// citation (`None`: no row holds one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Powered {
    pub device: String,
    pub part: String,
    pub rail: String,
    pub min: Option<(u32, String)>,
    /// `Some(why)`: judged by another rule ([`Judgement::Elsewhere`]).
    pub elsewhere: Option<String>,
}

/// Every powered part a specification declares, in a fixed order: the MCU
/// (its minimum once its reset is released, VBOR0's falling minimum, DS12117
/// Table 111 note 1, ruled D2); each I2C and SPI device at its bus's speed;
/// each radio module; the declared supervisor; each translator side; each
/// regulator (one fed from a rail, by its input minimum; one fed from the
/// battery, judged elsewhere); each pin device's supply pin, by its row's
/// range.
pub fn powered(spec: &Specification) -> Result<Vec<Powered>, IrValidationError> {
    let pp = crate::power_parts::table()?;
    let parts = crate::parts::table()?;
    let radio = crate::radio::table()?;
    let mut out = Vec::new();
    let mcu = pp.get(&spec.mcu.part).and_then(|r| match &r.kind {
        Kind::Mcu(m) => m.bor_falling_min_millivolts.first().map(|l| {
            (
                u32::try_from(l.value).unwrap_or(0),
                format!("{}; {}", pp.cite(&m.released_vdd_at), pp.cite(&l.at)),
            )
        }),
        _ => None,
    });
    out.push(Powered {
        device: "MCU".to_owned(),
        part: spec.mcu.part.clone(),
        rail: spec.mcu.supply.clone(),
        min: mcu,
        elsewhere: None,
    });
    let bus = |reference: &str, part: &str, rail: &str, hz: Option<u32>| {
        let min = parts.get(part).and_then(|p| {
            p.supply_min_at(u64::from(hz.unwrap_or(u32::MAX)))
                .map(|v| (v, format!("{part}'s supply at its bus's speed, parts.json")))
        });
        Powered {
            device: reference.to_owned(),
            part: part.to_owned(),
            rail: rail.to_owned(),
            min,
            elsewhere: None,
        }
    };
    for d in spec.declared_i2c_devices() {
        let hz = spec
            .i2c_buses
            .iter()
            .find(|b| b.id == d.bus)
            .map(|b| b.speed_hz);
        out.push(bus(&d.reference, &d.part, &d.supply, hz));
    }
    for d in spec.declared_spi_devices() {
        let hz = spec
            .spi_buses
            .iter()
            .find(|b| b.id == d.bus)
            .map(|b| b.speed_hz);
        out.push(bus(&d.reference, &d.part, &d.supply, hz));
    }
    for m in spec.declared_radio_modules() {
        out.push(Powered {
            device: m.reference.clone(),
            part: m.part.clone(),
            rail: m.supply.clone(),
            min: radio.get(&m.part).map(|r| {
                (
                    r.supply.min_mv,
                    format!("{}, {}", r.supply.at.document, r.supply.at.section),
                )
            }),
            elsewhere: None,
        });
    }
    if let Some(s) = &spec.mcu.reset_supervisor {
        out.push(Powered {
            device: s.reference.clone(),
            part: s.part.clone(),
            rail: spec.mcu.supply.clone(),
            min: pp.get(&s.part).and_then(|r| match &r.kind {
                Kind::Supervisor(v) => Some((
                    u32::try_from(v.vdd_millivolts.range.min).unwrap_or(0),
                    pp.cite(&v.vdd_millivolts.at),
                )),
                _ => None,
            }),
            elsewhere: None,
        });
    }
    if let Some(p) = &spec.power {
        for m in &p.rail_monitors {
            out.push(Powered {
                device: m.reference.clone(),
                part: m.part.clone(),
                rail: m.rail.clone(),
                min: pp.get(&m.part).and_then(|r| match &r.kind {
                    Kind::Supervisor(v) => Some((
                        u32::try_from(v.vdd_millivolts.range.min).unwrap_or(0),
                        pp.cite(&v.vdd_millivolts.at),
                    )),
                    _ => None,
                }),
                elsewhere: None,
            });
        }
        for t in p.translators() {
            let row = pp.get(t.part).and_then(|r| match &r.kind {
                Kind::Translator(x) => Some(x),
                _ => None,
            });
            for (side, rail, range) in [
                ("A", t.a_domain, row.map(|x| &x.vcca_operating_millivolts)),
                ("B", t.b_domain, row.map(|x| &x.vccb_operating_millivolts)),
            ] {
                out.push(Powered {
                    device: format!("{}.{side}", t.reference),
                    part: t.part.clone(),
                    rail: rail.clone(),
                    min: range.map(|r| (u32::try_from(r.range.min).unwrap_or(0), pp.cite(&r.at))),
                    elsewhere: None,
                });
            }
        }
        for r in &p.regulators {
            let vin = pp.get(&r.part).and_then(|row| match &row.kind {
                Kind::Buck(b) => Some(&b.vin_millivolts),
                Kind::Ldo(l) => Some(&l.vin_millivolts),
                _ => None,
            });
            let battery = r.input == power::BATTERY;
            out.push(Powered {
                device: r.reference.clone(),
                part: r.part.clone(),
                rail: r.input.clone(),
                min: vin.map(|v| (u32::try_from(v.range.min).unwrap_or(0), pp.cite(&v.at))),
                elsewhere: battery.then(|| {
                    "its input is the battery, judged by the headroom and the battery chain (power::headroom), not by a reset".to_owned()
                }),
            });
        }
    }
    // Each pin device's supply pin, by its row's range (`AET-WO-4310`
    // adversarial F1): a pin whose row holds none is named.
    for d in spec.declared_pin_devices() {
        let row = parts.pin_part(&d.part);
        for (pin, rail) in &d.supplies {
            let min = row
                .and_then(|r| r.pins.get(pin).map(|p| (r, p)))
                .and_then(|(r, p)| match &p.kind {
                    crate::pin_parts::PinKind::Supply { range_mv } => Some((
                        range_mv.0,
                        format!("{} {}, {}", r.document, r.revision, p.page),
                    )),
                    _ => None,
                });
            out.push(Powered {
                device: format!("{}.{pin}", d.reference),
                part: d.part.clone(),
                rail: rail.clone(),
                min,
                elsewhere: None,
            });
        }
    }
    Ok(out)
}

/// Each powered part, judged (ruled D1 (B)): on the MCU's supply, against
/// the reset's falling minimum ([`reset_floor`]); on another rail, against
/// the falling minimum of the rail monitor that drives it (`AET-WO-4413`
/// (2)), and otherwise named, since nothing holds it while the MCU runs;
/// with no held minimum, named.
pub fn judged(spec: &Specification) -> Result<Vec<(Powered, Judgement)>, IrValidationError> {
    judged_against(spec, crate::power_parts::table()?)
}

/// [`judged`] against `pp` for the resets' thresholds (see
/// [`reset_floor_against`]): a test plants a monitor's row there.
pub fn judged_against(
    spec: &Specification,
    pp: &crate::power_parts::PowerPartsTable,
) -> Result<Vec<(Powered, Judgement)>, IrValidationError> {
    let (reset_min, reset) = reset_floor_against(spec, pp)?;
    let held = held(spec);
    // The monitor holding `device` on `rail`, with its VIT− minimum.
    let monitor = |device: &str, rail: &str| -> Option<(u32, String)> {
        let (_, m) = held.iter().find(|(d, m)| d == device && m.rail == rail)?;
        match &pp.get(&m.part)?.kind {
            Kind::Supervisor(v) => Some((
                u32::try_from(v.falling_millivolts.range.min).unwrap_or(0),
                format!(
                    "{}'s VIT− minimum, {}",
                    m.reference,
                    pp.cite(&v.falling_millivolts.at)
                ),
            )),
            _ => None,
        }
    };
    let mut out: Vec<(Powered, Judgement)> = powered(spec)?
        .into_iter()
        .map(|p| {
            let named = |why: String| Named {
                device: p.device.clone(),
                part: p.part.clone(),
                rail: p.rail.clone(),
                why,
            };
            let j = if let Some(why) = &p.elsewhere {
                Judgement::Elsewhere(why.clone())
            } else {
                let floor = |min: u32, at: u32, by: &str| {
                    if min > at {
                        Judgement::Floor(Floor {
                            device: p.device.clone(),
                            part: p.part.clone(),
                            vcc_min_millivolts: min,
                            reset_min_millivolts: at,
                            reset: by.to_owned(),
                        })
                    } else {
                        Judgement::Pass
                    }
                };
                let held_by = (p.rail != spec.mcu.supply)
                    .then(|| monitor(&p.device, &p.rail))
                    .flatten();
                match &p.min {
                    None => Judgement::Named(named(
                        "no supply minimum is held for it, so it is never passed".to_owned(),
                    )),
                    Some((min, _)) if held_by.is_some() => {
                        let (at, by) = held_by.unwrap_or_default();
                        floor(*min, at, &by)
                    }
                    Some((min, cite)) if p.rail != spec.mcu.supply => Judgement::Named(named(format!(
                        "its supply minimum of {min} mV ({cite}) is held by no monitor: nothing holds `{}`'s consumers in reset or off, so it may run out of its range while the MCU runs",
                        p.rail
                    ))),
                    Some((min, _)) => floor(*min, reset_min, &reset),
                }
            };
            (p, j)
        })
        .collect();
    out.extend(spi_timing(spec, reset_min, pp)?);
    Ok(out)
}

/// Each SPI crossing whose side on the MCU's supply has its timing stated
/// only above the reset's falling minimum (`AET-WO-4310` (2), ruled
/// 2026-11-27T08:22Z (1)): between the two, its lines are not judged, so it
/// is named, never passed. A UART crossing's lines are judged at the release
/// floor instead (`power::radio_lines`).
fn spi_timing(
    spec: &Specification,
    reset_min: u32,
    pp: &crate::power_parts::PowerPartsTable,
) -> Result<Vec<(Powered, Judgement)>, IrValidationError> {
    let mut out = Vec::new();
    let Some(p) = &spec.power else {
        return Ok(out);
    };
    for t in p.translators().filter(|t| !t.uart) {
        let Some(Kind::Translator(x)) = pp.get(t.part).map(|r| &r.kind) else {
            continue;
        };
        let side: Option<fn(&crate::power_parts::Supplies) -> &crate::power_parts::Ranged> =
            if *t.a_domain == spec.mcu.supply {
                Some(|s| &s.vcca_millivolts)
            } else if *t.b_domain == spec.mcu.supply {
                Some(|s| &s.vccb_millivolts)
            } else {
                None
            };
        let Some(side) = side else { continue };
        let Some(lowest) = x
            .supplies
            .iter()
            .filter(|s| !s.uart_only)
            .map(side)
            .min_by_key(|r| r.range.min)
        else {
            continue;
        };
        let mv = u32::try_from(lowest.range.min).unwrap_or(u32::MAX);
        if mv <= reset_min {
            continue;
        }
        let named = Named {
            device: format!("{}.timing", t.reference),
            part: t.part.clone(),
            rail: spec.mcu.supply.clone(),
            why: format!(
                "SPI crossing timing stated only from {}.{} V ({}); at the release floor not judged",
                mv / 1000,
                mv % 1000 / 100,
                pp.cite(&lowest.at)
            ),
        };
        out.push((
            Powered {
                device: named.device.clone(),
                part: named.part.clone(),
                rail: named.rail.clone(),
                min: Some((mv, pp.cite(&lowest.at))),
                elsewhere: None,
            },
            Judgement::Named(named),
        ));
    }
    Ok(out)
}

/// Every powered part on the MCU's supply whose minimum is above the reset's
/// falling minimum (the rule; ruled D12: refused by `verify --fab-out`).
pub fn floors_above_reset(spec: &Specification) -> Result<Vec<Floor>, IrValidationError> {
    Ok(judged(spec)?
        .into_iter()
        .filter_map(|(_, j)| match j {
            Judgement::Floor(f) => Some(f),
            _ => None,
        })
        .collect())
}

/// Every powered part named, never passed (`AET-WO-4310` (2)): readiness
/// refuses a package with one, as with a floor (ruled 3, 2026-11-27T07:42Z).
pub fn named(spec: &Specification) -> Result<Vec<Named>, IrValidationError> {
    Ok(judged(spec)?
        .into_iter()
        .filter_map(|(_, j)| match j {
            Judgement::Named(n) => Some(n),
            _ => None,
        })
        .collect())
}

/// A declared supervisor, judged (Path B): its row a supervisor; its VDD range
/// covering the rail; its pull-up a resistor row inside the range its maker
/// names; its falling minimum at or above every supply-bounded device's VCC
/// minimum on the rail; its rising maximum under the rail's minimum at its
/// pin. `checked` is the power tree's, when one is declared: the rail's
/// minimum is then its solved minimum less its drop limit; without one, the
/// domain's declared minimum. Returns the conditions it rests on, each with
/// its id.
pub fn check(
    spec: &Specification,
    checked: Option<&power::Checked>,
) -> Result<Vec<(String, String)>, IrValidationError> {
    check_against(spec, checked, crate::power_parts::table()?)
}

/// [`check`] against `table` (see [`reset_floor_against`]).
pub fn check_against(
    spec: &Specification,
    checked: Option<&power::Checked>,
    table: &crate::power_parts::PowerPartsTable,
) -> Result<Vec<(String, String)>, IrValidationError> {
    let mut out = check_nrst(spec, checked, table)?;
    out.extend(check_rail_monitors(spec, checked, table)?);
    Ok(out)
}

/// The rail's minimum at a supervisor's pin, mV, and how it is read: the
/// solved minimum less the rail's drop limit with a power tree; without one,
/// or with no limit for the rail, the domain's declared minimum.
fn rail_minimum(
    spec: &Specification,
    checked: Option<&power::Checked>,
    rail: &str,
    domain: &crate::VoltageDomain,
) -> (u64, String) {
    let solved = checked.and_then(|c| c.domains.get(rail));
    let limit = spec
        .power
        .as_ref()
        .and_then(|p| p.drop_limits.iter().find(|l| l.domain == rail));
    match (solved, limit) {
        (Some(d), Some(l)) => {
            let vout = d.vout_min_microvolts / 1000;
            (
                vout.saturating_sub(u64::from(l.max_millivolts)),
                format!(
                    "{rail}'s solved minimum of {vout} mV less its drop limit of {} mV",
                    l.max_millivolts
                ),
            )
        }
        _ => (
            u64::from(crate::minimum_millivolts(domain)),
            format!("{rail}'s declared minimum"),
        ),
    }
}

/// Each rail monitor, judged (`AET-WO-4413` (2), ruled D1, D2): a
/// push-pull supervisor row; its bypass a capacitor row; its rail a declared
/// domain other than the MCU's; its VDD range covering the rail; its release
/// (VIT− maximum plus its typical hysteresis, `4300k` D6's precedent) under
/// the rail's minimum at its pin; and every part naming it on its rail and
/// declared, at least one. A radio module's enable it drives stays low for
/// the module's settling time: its delay's minimum at or above it.
fn check_rail_monitors(
    spec: &Specification,
    checked: Option<&power::Checked>,
    table: &crate::power_parts::PowerPartsTable,
) -> Result<Vec<(String, String)>, IrValidationError> {
    const RULE: &str = "AET-WO-4413 (2)";
    let mut out = Vec::new();
    let Some(p) = &spec.power else {
        return Ok(out);
    };
    let invalid =
        |id: &str, detail: String| refuse(Refusal::RailMonitorInvalid, id, detail, RULE.to_owned());
    // A monitor drives a module's enable or a translator's OE, nothing else.
    for r in &p.regulators {
        if let Some(c) = r.controls.iter().find(|c| c.monitor.is_some()) {
            return Err(invalid(
                &r.reference,
                format!(
                    "{}.{} names a monitor: a monitor drives a radio module's enable or a translator's `OE` only",
                    r.reference, c.pin
                ),
            ));
        }
    }
    // Every name a part gives a monitor, with the part's rails.
    let mut named: Vec<(String, &str, Vec<&String>)> = Vec::new();
    for m in spec.declared_radio_modules() {
        if let Some(mon) = &m.enable.monitor {
            named.push((m.reference.clone(), mon, vec![&m.supply]));
        }
    }
    for t in p.translators() {
        for c in t
            .controls
            .iter()
            .filter_map(|c| c.monitor.as_deref().map(|m| (c, m)))
        {
            // belt-and-suspenders: every held translator row's one control
            // pin is `OE`, and `power::check` refuses a pin its row lists
            // as none first; a later row with another would credit nothing
            // here, so it is refused rather than silently ignored.
            if c.0.pin != "OE" {
                return Err(invalid(
                    t.reference,
                    format!(
                        "{}.{} names `{}`: a monitor drives a translator's `OE` only",
                        t.reference, c.0.pin, c.1
                    ),
                ));
            }
            named.push((t.reference.clone(), c.1, vec![t.a_domain, t.b_domain]));
        }
    }
    for (part, mon, rails) in &named {
        let Some(m) = p.rail_monitors.iter().find(|m| &m.reference == mon) else {
            return Err(invalid(
                part,
                format!("{part} names `{mon}`, which is no declared rail monitor"),
            ));
        };
        if !rails.contains(&&m.rail) {
            return Err(invalid(
                part,
                format!(
                    "{part} names `{mon}`, which monitors `{}`, a rail {part} is not on",
                    m.rail
                ),
            ));
        }
    }
    let radio = crate::radio::table()?;
    for m in &p.rail_monitors {
        let row = table.get(&m.part).ok_or_else(|| {
            refuse(
                Refusal::PowerPartUnknown,
                &m.reference,
                format!("part `{}` is not a row of the power parts table", m.part),
                RULE.to_owned(),
            )
        })?;
        let Kind::Supervisor(v) = &row.kind else {
            return Err(refuse(
                Refusal::PowerPartUnknown,
                &m.reference,
                format!(
                    "part `{}` is a {} row, not a supervisor",
                    m.part,
                    row.kind.name()
                ),
                RULE.to_owned(),
            ));
        };
        if v.output != crate::power_parts::SupervisorOutput::PushPull {
            return Err(refuse(
                Refusal::SupervisorOutOfRange,
                &m.reference,
                format!(
                    "{} has an open-drain output: a rail monitor drives its pins itself, and declares no pull-up",
                    m.part
                ),
                table.cite(&v.falling_millivolts.at),
            ));
        }
        if table.get(&m.bypass.part).map(|r| r.kind.name()) != Some("capacitor") {
            return Err(refuse(
                Refusal::PowerPartUnknown,
                &m.bypass.reference,
                format!(
                    "part `{}` is not a capacitor row of the power parts table",
                    m.bypass.part
                ),
                RULE.to_owned(),
            ));
        }
        if m.rail == spec.mcu.supply {
            return Err(invalid(
                &m.reference,
                format!(
                    "`{}` is the MCU's supply, held by its reset (`mcu.reset_supervisor` or the BOR), not by a rail monitor",
                    m.rail
                ),
            ));
        }
        if !named.iter().any(|(_, mon, _)| *mon == m.reference) {
            return Err(invalid(
                &m.reference,
                format!(
                    "{} drives nothing: no part names it, so it holds nothing",
                    m.reference
                ),
            ));
        }
        let domain = spec
            .voltage_domains
            .iter()
            .find(|d| d.id == m.rail)
            .ok_or_else(|| invalid(&m.reference, format!("`{}` is no voltage domain", m.rail)))?;
        let (rail_min, how) = rail_minimum(spec, checked, &m.rail, domain);
        let rising = v.falling_millivolts.range.max + v.hysteresis_typ_millivolts.value;
        if u64::try_from(rising).unwrap_or(u64::MAX) >= rail_min {
            return Err(refuse(
                Refusal::SupervisorAboveRail,
                &m.reference,
                format!(
                    "{} releases at up to {rising} mV (VIT− maximum {} mV plus its {} mV hysteresis, typical), at or above the rail's minimum at its pin, {rail_min} mV ({how}), so it could hold a healthy part in reset",
                    m.part, v.falling_millivolts.range.max, v.hysteresis_typ_millivolts.value
                ),
                table.cite(&v.falling_millivolts.at),
            ));
        }
        let (vdd_lo, vdd_hi) = (
            u64::try_from(v.vdd_millivolts.range.min).unwrap_or(0),
            u64::try_from(v.vdd_millivolts.range.max).unwrap_or(0),
        );
        if u64::from(crate::maximum_millivolts(domain)) > vdd_hi || rail_min < vdd_lo {
            return Err(refuse(
                Refusal::SupervisorOutOfRange,
                &m.reference,
                format!(
                    "{} runs from {vdd_lo} to {vdd_hi} mV, and `{}` spans {rail_min} to {} mV",
                    m.part,
                    m.rail,
                    crate::maximum_millivolts(domain)
                ),
                table.cite(&v.vdd_millivolts.at),
            ));
        }
        // A module's enable stays low for its settling time from the
        // supply's rise (the RC's rule, `radio.rs`): the delay's minimum.
        for module in spec
            .declared_radio_modules()
            .iter()
            .filter(|x| x.enable.monitor.as_deref() == Some(m.reference.as_str()))
        {
            let Some(r) = radio.get(&module.part) else {
                continue;
            };
            let need = u64::from(r.enable.stable_us);
            if u64::try_from(v.delay_min_microseconds.value).unwrap_or(0) < need {
                return Err(invalid(
                    &m.reference,
                    format!(
                        "{}'s reset delay is at least {} us ({}), under the {need} us {} must be held off after its supply settles ({}, {})",
                        m.part,
                        v.delay_min_microseconds.value,
                        table.cite(&v.delay_min_microseconds.at),
                        module.reference,
                        r.enable.at.document,
                        r.enable.at.section
                    ),
                ));
            }
        }
        out.push((
            format!("supervisor_hysteresis_typical:{}", m.reference),
            format!(
                "{}: {}'s rising threshold is VIT− maximum {} mV plus its hysteresis, {} mV, which is stated typical only ({}), so its release at up to {rising} mV, under the rail's minimum of {rail_min} mV ({how}), is screened, not guaranteed (`4300k` D6's precedent, ruled `AET-WO-4413` D1)",
                m.reference,
                m.part,
                v.falling_millivolts.range.max,
                v.hysteresis_typ_millivolts.value,
                table.cite(&v.hysteresis_typ_millivolts.at)
            ),
        ));
    }
    Ok(out)
}

/// `mcu.reset_supervisor`, judged (Path B, [`check`]).
fn check_nrst(
    spec: &Specification,
    checked: Option<&power::Checked>,
    table: &crate::power_parts::PowerPartsTable,
) -> Result<Vec<(String, String)>, IrValidationError> {
    let Some(s) = &spec.mcu.reset_supervisor else {
        return Ok(Vec::new());
    };
    const RULE: &str = "AET-WO-4300k (9)";
    let (reset_min, _) = reset_floor_against(spec, table)?;
    let row = table.get(&s.part).ok_or_else(|| {
        refuse(
            Refusal::PowerPartUnknown,
            &s.reference,
            format!("part `{}`", s.part),
            RULE.to_owned(),
        )
    })?;
    let Kind::Supervisor(v) = &row.kind else {
        unreachable!("reset_floor refused a non-supervisor row");
    };
    // `NRST` is the MCU's own open-drain reset too: a push-pull output
    // there would fight it (`AET-WO-4413` (2)).
    let crate::power_parts::SupervisorOutput::OpenDrain { pull_up_ohms } = &v.output else {
        return Err(refuse(
            Refusal::SupervisorOutOfRange,
            &s.reference,
            format!(
                "{} has a push-pull output, and `NRST` takes an open-drain one with its pull-up",
                s.part
            ),
            table.cite(&v.falling_millivolts.at),
        ));
    };

    for (reference, part, want) in [
        (&s.pull_up.reference, &s.pull_up.part, "resistor"),
        (&s.bypass.reference, &s.bypass.part, "capacitor"),
    ] {
        let kind = table.get(part).map(|r| r.kind.name());
        if kind != Some(want) {
            return Err(refuse(
                Refusal::PowerPartUnknown,
                reference,
                format!("part `{part}` is not a {want} row of the power parts table"),
                RULE.to_owned(),
            ));
        }
    }
    for (device, part, min) in devices(spec)? {
        let vcc = min.unwrap_or(u32::MAX);
        if vcc > reset_min {
            return Err(refuse(
                Refusal::SupervisorBelowDeviceFloor,
                &s.reference,
                format!(
                    "{} ({}) falls to {reset_min} mV at least before it resets, under {device} ({part})'s VCC minimum of {vcc} mV",
                    s.reference, s.part
                ),
                table.cite(&v.falling_millivolts.at),
            ));
        }
    }
    let domain = spec
        .voltage_domains
        .iter()
        .find(|d| d.id == spec.mcu.supply)
        .ok_or_else(|| {
            refuse(
                Refusal::PowerValueInvalid,
                &s.reference,
                "the MCU's supply is no domain".to_owned(),
                RULE.to_owned(),
            )
        })?;
    let solved = checked.and_then(|c| c.domains.get(&spec.mcu.supply));
    let (rail_min, how) = match (solved, spec.power.as_ref()) {
        (Some(d), Some(p)) => {
            let limit = p
                .drop_limits
                .iter()
                .find(|l| l.domain == spec.mcu.supply)
                .map_or(0, |l| u64::from(l.max_millivolts));
            let vout = d.vout_min_microvolts / 1000;
            (
                vout.saturating_sub(limit),
                format!(
                    "{}'s solved minimum of {vout} mV less its drop limit of {limit} mV",
                    spec.mcu.supply
                ),
            )
        }
        _ => (
            u64::from(crate::minimum_millivolts(domain)),
            format!("{}'s declared minimum", spec.mcu.supply),
        ),
    };
    let rising = v.falling_millivolts.range.max + v.hysteresis_typ_millivolts.value;
    let rising_u = u64::try_from(rising).unwrap_or(u64::MAX);
    if rising_u >= rail_min {
        return Err(refuse(
            Refusal::SupervisorAboveRail,
            &s.reference,
            format!(
                "{} releases at up to {rising} mV (VIT− maximum {} mV plus its {} mV hysteresis, typical), at or above the rail's minimum at its pin, {rail_min} mV ({how}), so it could hold a healthy board in reset",
                s.part, v.falling_millivolts.range.max, v.hysteresis_typ_millivolts.value
            ),
            table.cite(&v.falling_millivolts.at),
        ));
    }
    let (vdd_lo, vdd_hi) = (
        u64::try_from(v.vdd_millivolts.range.min).unwrap_or(0),
        u64::try_from(v.vdd_millivolts.range.max).unwrap_or(0),
    );
    if u64::from(crate::maximum_millivolts(domain)) > vdd_hi || rail_min < vdd_lo {
        return Err(refuse(
            Refusal::SupervisorOutOfRange,
            &s.reference,
            format!(
                "{} runs from {vdd_lo} to {vdd_hi} mV, and `{}` spans {rail_min} to {} mV",
                s.part,
                spec.mcu.supply,
                crate::maximum_millivolts(domain)
            ),
            table.cite(&v.vdd_millivolts.at),
        ));
    }
    let ohms = i64::from(s.pull_up.ohms);
    if ohms < pull_up_ohms.range.min || ohms > pull_up_ohms.range.max {
        return Err(refuse(
            Refusal::SupervisorOutOfRange,
            &s.pull_up.reference,
            format!(
                "{} is {ohms} Ω, outside the {} to {} Ω pull-up {} names",
                s.pull_up.reference, pull_up_ohms.range.min, pull_up_ohms.range.max, s.part
            ),
            table.cite(&pull_up_ohms.at),
        ));
    }
    let mut out = Vec::new();
    if row.lifecycle == crate::power_parts::Lifecycle::Unread {
        out.push((
            format!("lifecycle_unread:{}", s.part),
            format!(
                "{}: its maker's page states no lifecycle, so its production status is unread",
                s.part
            ),
        ));
    }
    // The reset delay is stated only above VIT− by an overdrive; a rail
    // whose minimum sits under that is named (the power review's F9).
    let stated_from = v.falling_millivolts.range.max + v.delay_overdrive_millivolts.value;
    if u64::try_from(stated_from).unwrap_or(u64::MAX) > rail_min {
        out.push((
            format!("supervisor_delay_unstated:{}", s.reference),
            format!(
                "{}: {}'s reset delay is stated at VDD at least {stated_from} mV (VIT− maximum plus {} mV, {}), above the rail's minimum of {rail_min} mV ({how}), so the delay at the rail's minimum is not stated",
                s.reference,
                s.part,
                v.delay_overdrive_millivolts.value,
                table.cite(&v.delay_overdrive_millivolts.at)
            ),
        ));
    }
    out.push((
        format!("supervisor_hysteresis_typical:{}", s.reference),
        format!(
            "{}: {}'s rising threshold is VIT− maximum {} mV plus its hysteresis, {} mV, which is stated typical only ({}), so its release at up to {rising} mV, under the rail's minimum of {rail_min} mV ({how}), is screened, not guaranteed (ruled D6)",
            s.reference,
            s.part,
            v.falling_millivolts.range.max,
            v.hysteresis_typ_millivolts.value,
            table.cite(&v.hysteresis_typ_millivolts.at)
        ),
    ));
    Ok(out)
}

#[cfg(test)]
mod tests_4310 {
    /// `AET-WO-4310` (2), ruled 08:22Z (1): an SPI crossing's timing is named
    /// only while its lowest stated band is above the reset; at it, it holds.
    #[test]
    fn spi_crossing_timing_is_named_only_above_the_reset() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sentinel-02-power.json"
        );
        let spec: crate::Specification =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let t = crate::power_parts::table().unwrap();
        assert_eq!(super::spi_timing(&spec, 2999, t).unwrap().len(), 1);
        assert!(super::spi_timing(&spec, 3000, t).unwrap().is_empty());
        // A UART-only row is never an SPI crossing's: stated lower there, the
        // SPI rows' 3.0 V still decides.
        let mut v: serde_json::Value =
            serde_json::from_str(crate::power_parts::POWER_PARTS_JSON).unwrap();
        for row in v["parts"].as_array_mut().unwrap() {
            if row["part"] == "TXU0304PW" {
                for s in row["kind"]["translator"]["supplies"]
                    .as_array_mut()
                    .unwrap()
                {
                    if s["uart_only"] == true {
                        s["vcca_millivolts"]["min"] = serde_json::json!(1650);
                    }
                }
            }
        }
        let low = crate::power_parts::parse_power_parts(&v.to_string()).unwrap();
        assert_eq!(super::spi_timing(&spec, 2999, &low).unwrap().len(), 1);
    }
}
