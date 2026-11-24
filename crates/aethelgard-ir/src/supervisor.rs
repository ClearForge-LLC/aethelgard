//! The reset's falling threshold against every supply-bounded device on the
//! rail it supervises (`AET-WO-4300k` (9), ruled 2026-11-23: build both; a
//! board takes Path A or Path B).
//!
//! - **The rule.** Every I2C or SPI device on the MCU's supply holds its
//!   guarantees down to its VCC minimum at its bus's speed
//!   ([`crate::parts::Part::supply_min_at`]). The MCU may run down to the
//!   reset's falling minimum: the selected BOR level's (DS12117 Table 16 =
//!   Table 115), or a declared supervisor's VIT− minimum. A device whose
//!   minimum is above it may run out of its range before the reset holds.
//!   [`floors_above_reset`] lists each; `verify --fab-out` refuses a package
//!   with one, and every other verdict names it (ruled D12).
//! - **Path A** is a wide-range part, under BOR3's 2.54 V.
//! - **Path B** is a declared supervisor on `NRST`, judged by [`check`]: its
//!   falling minimum at or above every such device's VCC minimum, and its
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
    let table = crate::power_parts::table()?;
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

/// Every device on the MCU's supply whose VCC minimum is above the reset's
/// falling minimum (the rule; ruled D12: refused by `verify --fab-out`,
/// named elsewhere). A device with no minimum at its bus's speed is listed
/// at `u32::MAX`.
pub fn floors_above_reset(spec: &Specification) -> Result<Vec<Floor>, IrValidationError> {
    let (reset_min, reset) = reset_floor(spec)?;
    Ok(devices(spec)?
        .into_iter()
        .filter_map(|(device, part, min)| {
            let vcc = min.unwrap_or(u32::MAX);
            (vcc > reset_min).then(|| Floor {
                device,
                part,
                vcc_min_millivolts: vcc,
                reset_min_millivolts: reset_min,
                reset: reset.clone(),
            })
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
    let Some(s) = &spec.mcu.reset_supervisor else {
        return Ok(Vec::new());
    };
    const RULE: &str = "AET-WO-4300k (9)";
    let table = crate::power_parts::table()?;
    let (reset_min, _) = reset_floor(spec)?;
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
    if ohms < v.pull_up_ohms.range.min || ohms > v.pull_up_ohms.range.max {
        return Err(refuse(
            Refusal::SupervisorOutOfRange,
            &s.pull_up.reference,
            format!(
                "{} is {ohms} Ω, outside the {} to {} Ω pull-up {} names",
                s.pull_up.reference, v.pull_up_ohms.range.min, v.pull_up_ohms.range.max, s.part
            ),
            table.cite(&v.pull_up_ohms.at),
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
