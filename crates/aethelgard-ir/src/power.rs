//! The specification's `power` (`AET-WO-4300d`, ruled on PR #137): the
//! battery, the power-in connector, each regulator with its feedback divider
//! and external parts, the declared loads, the battery sense and the bus
//! crossings. Every check here is static: it reads the specification, the
//! power parts table ([`crate::power_parts`]) and the sensor table, never a
//! generated artifact (`INV-07`).
//!
//! ```json
//! "power": null | {
//!   "conditions": { "board_temperature_celsius": { "min": -20, "max": 85, "source": "..." },
//!                   "dc_bias_derating": [ { "dielectrics": ["X7R"], "ratio_max_permille": 1000,
//!                                           "retained_permille": 500, "source": "..." } ] },
//!   "input": { "reference": "J1", "part": "XT30PW-M",
//!              "hot_plug": [ { "reference": "C1", "part": "UCZ1H470MCL1GS", "side": "board" } ] },
//!   "battery": { "chemistry": "LiPo", "cells": { "min": 2, "max": 6 },
//!                "cell_min_millivolts": { "value": 3000, "source": "..." },
//!                "cell_max_millivolts": { "value": 4350, "source": "..." } },
//!   "regulators": [ { "reference": "U10", "part": "LMR33630ADDA", "input": "battery",
//!                     "output": "VDD_5V",
//!                     "feedback": { "top": { "reference": "R10", "part": "TNPW0603", "ohms": 102000 },
//!                                   "bottom": { ... } } | null,
//!                     "external": [ { "role": "cin", "reference": "C10", "part": "..." } ],
//!                     "junction": { "copper_square_centimetres": { "value": 20, "source": "..." },
//!                                   "loss_milliwatts": { "value": 1170, "source": "..." } } | null,
//!                     "controls": [ { "pin": "EN", "strap": "input", "wire": null },
//!                                   { "pin": "PG", "strap": null, "wire": { "net": "PG_5V", "role": "..." } } ] } ],
//!   "loads": [ { "domain": "VDD_5V", "milliamps": 2000, "source": "..." } ],
//!   "sense": [ { "net": "VBAT_SENSE", "top": {...}, "bottom": {...},
//!                "reservoir": { "reference": "C30", "part": "..." } | null, "adc_resolution_bits": 12 } ],
//!   "crossings": [ { "reference": "U13", "part": "TXU0304PW", "bus": "IMU", "a_domain": "VDD_3V3",
//!                    "b_domain": "VDD_1V8", "cs_pull_up": { "reference": "R40", "part": "...", "ohms": 100000 },
//!                    "external": [ ... ], "controls": [ ... ] } ]
//! }
//! ```
//!
//! **The tree.** Every voltage domain has exactly one source, a regulator
//! whose `output` it is. A regulator's `input` is `battery` or a domain, and
//! following inputs from any domain reaches the battery with no repeat. The
//! MCU's load is its table row's (DS12117 Table 120, `1303` D4), never typed
//! by hand. A regulator's load is its domain's declared loads, the MCU's when
//! the domain is its supply, and each child regulator's input current: an LDO
//! child's load plus its ground current, a buck child's load (in steady state
//! a buck's average input is at most its output, ruled D2), with the buck's
//! own bias named as a condition.
//!
//! **What the checks refuse,** each by name with its citation, in the order
//! [`check`] runs them: see [`Refusal`]. A value the datasheet states only
//! under a condition, or by design, is used and the condition is named in
//! [`Checked::conditions`], which `solve` carries into the topology.
//!
//! **The judgements of `AET-WO-4300d3`.** A battery input feeding a regulator
//! carries a cited bulk capacitor or clamp (`BatteryInputUndamped`). A buck's
//! junction is screened from a declared loss and its row's typical θJA curve
//! (`BuckJunctionOverLimit`), and a role's effective capacitance under DC
//! bias from the maker's curve, interpolated and composed with the part's
//! tolerance and temperature limit, or else a declared derating, a 50 %
//! placeholder (`EffectiveCapacitanceBelowMinimum`; past a curve's end,
//! `DcBiasBeyondCurve`): screens with named margins, never guarantees,
//! because only typical figures exist. The MCU's supply clears
//! the brown-out level's rising maximum (`SupplyFloorBelowBorRising`), and
//! every control pin is strapped as its row allows or wired
//! (`ControlPinUnwired`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::power_parts::{self as pp, Kind, PowerPartsTable, Role};
use crate::{
    IrValidationError, Specification, VoltageDomain, maximum_millivolts, minimum_millivolts,
};

/// A regulator's input when it is fed straight from the battery.
pub const BATTERY: &str = "battery";

/// The battery's net on a drawn board (`AET-WO-4300d2` K10): a rail, reserved
/// for the power tree, which a net class may name when a tree is declared.
pub const BATTERY_NET: &str = "VBAT";

/// Every reference the power tree declares, in the emitter's claim order
/// (moved from `aethelgard-eda` by `AET-WO-1206`, so `validate`'s name checks
/// and the emitter read one list).
pub fn references(p: &Power) -> Vec<&String> {
    let mut out = vec![&p.input.reference];
    for r in &p.regulators {
        out.push(&r.reference);
        if let Some(d) = &r.feedback {
            out.extend([&d.top.reference, &d.bottom.reference]);
        }
        out.extend(r.external.iter().map(|e| &e.reference));
    }
    for s in &p.sense {
        out.extend([&s.top.reference, &s.bottom.reference]);
        out.extend(s.reservoir.iter().map(|c| &c.reference));
    }
    for c in &p.crossings {
        out.extend([&c.reference, &c.cs_pull_up.reference]);
        out.extend(c.external.iter().map(|e| &e.reference));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Power {
    pub conditions: Conditions,
    pub input: PowerIn,
    pub battery: Battery,
    pub regulators: Vec<Regulator>,
    pub loads: Vec<Load>,
    pub sense: Vec<Sense>,
    pub crossings: Vec<Crossing>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Conditions {
    /// The temperature every part and resistor check evaluates at (spike A2).
    pub board_temperature_celsius: Temperature,
    /// The capacitance kept under DC bias where no maker figure is held, per
    /// dielectric and voltage ratio, with its source (`AET-WO-4300d3` §1.3).
    pub dc_bias_derating: Vec<Derating>,
}

/// One declared derating: a ceramic of one of `dielectrics` working at most
/// `ratio_max_permille` of its rated voltage keeps at least
/// `retained_permille` of its rated capacitance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Derating {
    pub dielectrics: Vec<pp::Dielectric>,
    pub ratio_max_permille: u32,
    pub retained_permille: u32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Temperature {
    pub min: i32,
    pub max: i32,
    pub source: String,
}

/// The power-in connector: a row of [`crate::connectors::POWER_IN`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PowerIn {
    pub reference: String,
    pub part: String,
    /// The bulk capacitors and clamps that answer a hot-plug's ring on the
    /// battery input (`AET-WO-4300d3` §1.1).
    pub hot_plug: Vec<HotPlug>,
}

/// A damping part on the battery input: a bulk capacitor or a TVS row, and
/// the side of the connector it sits on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct HotPlug {
    pub reference: String,
    pub part: String,
    pub side: Side,
}

/// Which side of the power-in connector a part sits on: the board's, where
/// the regulators are, or the pack's, beyond the leads' inductance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Board,
    Pack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Battery {
    /// A label; the cell limits are what is judged.
    pub chemistry: String,
    pub cells: Cells,
    pub cell_min_millivolts: SourcedValue,
    pub cell_max_millivolts: SourcedValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Cells {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SourcedValue {
    pub value: u32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Regulator {
    pub reference: String,
    pub part: String,
    /// `battery`, or a voltage domain's id.
    pub input: String,
    /// A voltage domain's id.
    pub output: String,
    /// Required as a key: a buck needs its divider, an LDO takes `null`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub feedback: Option<Divider>,
    pub external: Vec<External>,
    /// Required as a key: a buck's junction screen declarations; an LDO
    /// takes `null`, its row holding its θJA (`AET-WO-4300d3` §1.2).
    #[serde(deserialize_with = "Option::deserialize")]
    pub junction: Option<Junction>,
    /// Each control pin of its row, strapped or wired (§1.5).
    pub controls: Vec<Control>,
}

/// What the buck's junction screen reads from the specification: the copper
/// area on each layer around the part, and the loss bound with its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Junction {
    pub copper_square_centimetres: SourcedValue,
    pub loss_milliwatts: SourcedValue,
}

/// A control pin and what it is tied to: a strap its row allows, or a wire to
/// a named net with its stated role. Both keys are required, and exactly one
/// is `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Control {
    pub pin: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub strap: Option<pp::Strap>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub wire: Option<Wire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Wire {
    pub net: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Divider {
    pub top: Resistor,
    pub bottom: Resistor,
}

/// A resistor: a reference, its series row, and its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Resistor {
    pub reference: String,
    pub part: String,
    pub ohms: u32,
}

/// An external part of a regulator or a crossing, in a role its row names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct External {
    pub role: Role,
    pub reference: String,
    pub part: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Load {
    pub domain: String,
    pub milliamps: u32,
    pub source: String,
}

/// The battery's voltage sense: a divider from the battery to an ADC net,
/// with a reservoir capacitor at the pin when the divider's impedance needs
/// one (ruled D10 (a)), and the ADC's resolution, declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Sense {
    pub net: String,
    pub top: Resistor,
    pub bottom: Resistor,
    #[serde(deserialize_with = "Option::deserialize")]
    pub reservoir: Option<Reservoir>,
    pub adc_resolution_bits: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Reservoir {
    pub reference: String,
    pub part: String,
}

/// A fixed-direction translator splitting one SPI bus between the MCU's side
/// (`a_domain`, the bus's domain) and its one device's side (`b_domain`,
/// the device's supply). The MCU's outputs SCK, MOSI and CS take the row's
/// A→B channels in that order, and MISO its B→A channel (ruled D9). The CS
/// pull-up sits on the A side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Crossing {
    pub reference: String,
    pub part: String,
    pub bus: String,
    pub a_domain: String,
    pub b_domain: String,
    pub cs_pull_up: Resistor,
    pub external: Vec<External>,
    /// The translator's control pins (§1.5).
    pub controls: Vec<Control>,
}

crate::object_only!(
    Power,
    Conditions,
    Temperature,
    PowerIn,
    Battery,
    Cells,
    SourcedValue,
    Regulator,
    Divider,
    Resistor,
    External,
    Load,
    Sense,
    Reservoir,
    Crossing,
    Derating,
    HotPlug,
    Junction,
    Control,
    Wire
);

/// The names a power refusal carries, as the CLI's error envelope shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A part name the tables do not hold, or a row of the wrong kind.
    PowerPartUnknown,
    /// A declared value outside what the schema allows (a zero, a min above
    /// its max, a resolution the table has no row for).
    PowerValueInvalid,
    DomainWithoutSource,
    DomainWithTwoSources,
    PowerLoop,
    BatteryOutsideRegulatorInput,
    DomainOutsideConsumer,
    RegulatorOutsideDomain,
    DropoutViolated,
    BuckHeadroomViolated,
    OnTimeTooShort,
    RegulatorOverloaded,
    PowerInputOverloaded,
    JunctionTooHot,
    BoardOutsidePartTemperature,
    ExternalPartMissing,
    ExternalPartOutOfRange,
    SenseFullScaleExceeded,
    SenseSourceImpedance,
    CrossingInvalid,
    CrossingDomainMismatch,
    CrossingPullUp,
    CrossingTooFast,
    MisoRoundTripTooSlow,
    /// A device behind a crossing whose sensor row holds no output delay.
    OutputDelayMissing,
    /// A battery input feeding a regulator with no cited bulk capacitor or
    /// clamp to answer a hot-plug's ring (`AET-WO-4300d3` §1.1).
    BatteryInputUndamped,
    /// A buck whose junction screen does not clear its limit (§1.2).
    BuckJunctionOverLimit,
    /// A role's effective capacitance under DC bias at or below its stated
    /// minimum (§1.3).
    EffectiveCapacitanceBelowMinimum,
    /// The MCU's supply floor at or below the selected brown-out level's
    /// rising maximum (§1.4).
    SupplyFloorBelowBorRising,
    /// A control pin neither wired nor tied by a strap its row allows (§1.5).
    ControlPinUnwired,
    /// A ceramic working past the last point of its maker's DC-bias curve,
    /// which is never extrapolated (the amendment on PR #158).
    DcBiasBeyondCurve,
}

impl Refusal {
    pub fn name(self) -> &'static str {
        match self {
            Self::PowerPartUnknown => "PowerPartUnknown",
            Self::PowerValueInvalid => "PowerValueInvalid",
            Self::DomainWithoutSource => "DomainWithoutSource",
            Self::DomainWithTwoSources => "DomainWithTwoSources",
            Self::PowerLoop => "PowerLoop",
            Self::BatteryOutsideRegulatorInput => "BatteryOutsideRegulatorInput",
            Self::DomainOutsideConsumer => "DomainOutsideConsumer",
            Self::RegulatorOutsideDomain => "RegulatorOutsideDomain",
            Self::DropoutViolated => "DropoutViolated",
            Self::BuckHeadroomViolated => "BuckHeadroomViolated",
            Self::OnTimeTooShort => "OnTimeTooShort",
            Self::RegulatorOverloaded => "RegulatorOverloaded",
            Self::PowerInputOverloaded => "PowerInputOverloaded",
            Self::JunctionTooHot => "JunctionTooHot",
            Self::BoardOutsidePartTemperature => "BoardOutsidePartTemperature",
            Self::ExternalPartMissing => "ExternalPartMissing",
            Self::ExternalPartOutOfRange => "ExternalPartOutOfRange",
            Self::SenseFullScaleExceeded => "SenseFullScaleExceeded",
            Self::SenseSourceImpedance => "SenseSourceImpedance",
            Self::CrossingInvalid => "CrossingInvalid",
            Self::CrossingDomainMismatch => "CrossingDomainMismatch",
            Self::CrossingPullUp => "CrossingPullUp",
            Self::CrossingTooFast => "CrossingTooFast",
            Self::MisoRoundTripTooSlow => "MisoRoundTripTooSlow",
            Self::OutputDelayMissing => "OutputDelayMissing",
            Self::BatteryInputUndamped => "BatteryInputUndamped",
            Self::BuckJunctionOverLimit => "BuckJunctionOverLimit",
            Self::EffectiveCapacitanceBelowMinimum => "EffectiveCapacitanceBelowMinimum",
            Self::SupplyFloorBelowBorRising => "SupplyFloorBelowBorRising",
            Self::ControlPinUnwired => "ControlPinUnwired",
            Self::DcBiasBeyondCurve => "DcBiasBeyondCurve",
        }
    }
}

fn refuse(
    kind: Refusal,
    id: &str,
    detail: impl Into<String>,
    citation: impl Into<String>,
) -> IrValidationError {
    IrValidationError::Power {
        kind,
        id: id.to_owned(),
        detail: detail.into(),
        citation: citation.into(),
    }
}

/// What the checks derived, for `solve` to carry into the topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Each domain, by id.
    pub domains: BTreeMap<String, CheckedDomain>,
    /// The battery-side current, steady state and fault bound, mA.
    pub input_steady_milliamps: u64,
    pub input_fault_milliamps: u64,
    /// Every named condition the verdict rests on, each once, sorted.
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedDomain {
    pub regulator: String,
    pub part: String,
    /// The regulator's guaranteed output at its source pad, µV.
    pub vout_min_microvolts: u64,
    pub vout_max_microvolts: u64,
    /// The source and return pads (spike S5, ruled K5): `(reference, pad)`.
    pub source: (String, String),
    pub ret: (String, String),
    /// Everything the regulator delivers, mA.
    pub load_milliamps: u64,
    /// Each consumer of the domain and the lowest supply at which its own
    /// guarantees hold, mV (ruled D3).
    pub consumers: Vec<(String, u64)>,
}

/// The resolution an ADC row can declare.
fn rain_row(mcu: &pp::Mcu, bits: u8) -> Option<&pp::Limit> {
    mcu.rain_max_ohms
        .iter()
        .find(|(b, _)| *b == bits)
        .map(|(_, l)| l)
}

/// `ceil(a / b)` for positive `b`.
fn div_ceil(a: i128, b: i128) -> i128 {
    let q = a / b;
    if a % b != 0 && ((a > 0) == (b > 0)) {
        q + 1
    } else {
        q
    }
}

/// A resistor's worst relative error over the board's declared range, ppm:
/// its tolerance, its temperature coefficient over the larger excursion from
/// its reference temperature, and its stated drift (spike A2).
fn resistor_error_ppm(r: &pp::Resistor, t: &Temperature) -> i128 {
    let excursion = (i128::from(t.max) - i128::from(r.tcr_reference_celsius))
        .max(i128::from(r.tcr_reference_celsius) - i128::from(t.min))
        .max(0);
    i128::from(r.tolerance_ppm.value)
        + i128::from(r.tcr_ppm_per_kelvin.value) * excursion
        + i128::from(r.drift_ppm.value)
}

struct Ctx<'a> {
    spec: &'a Specification,
    power: &'a Power,
    table: &'a PowerPartsTable,
    delays: &'a OutputDelays,
    /// Floors found by a check, not by a part row: `(domain, name, mV)`.
    floors: Vec<(String, String, u64)>,
    conditions: BTreeSet<String>,
}

impl<'a> Ctx<'a> {
    fn row(&self, reference: &str, part: &str) -> Result<&'a pp::Row, IrValidationError> {
        self.table.get(part).ok_or_else(|| {
            refuse(
                Refusal::PowerPartUnknown,
                reference,
                format!("part `{part}` is not in the power parts table"),
                "crates/aethelgard-ir/data/power-parts.json",
            )
        })
    }

    fn kind_err(&self, reference: &str, part: &str, want: &str) -> IrValidationError {
        refuse(
            Refusal::PowerPartUnknown,
            reference,
            format!("part `{part}` is not a {want} row"),
            "crates/aethelgard-ir/data/power-parts.json",
        )
    }

    fn resistor(&self, r: &Resistor) -> Result<&'a pp::Resistor, IrValidationError> {
        match &self.row(&r.reference, &r.part)?.kind {
            Kind::Resistor(s) => Ok(s),
            _ => Err(self.kind_err(&r.reference, &r.part, "resistor")),
        }
    }

    fn domain(&self, id: &str) -> Option<&'a VoltageDomain> {
        self.spec.voltage_domains.iter().find(|d| d.id == id)
    }

    fn cite(&self, at: &pp::At) -> String {
        self.table.cite(at)
    }

    fn temperature(&self) -> &'a Temperature {
        &self.power.conditions.board_temperature_celsius
    }

    fn note(&mut self, condition: impl Into<String>) {
        self.conditions.insert(condition.into());
    }

    /// A resistor's worst error, with its series' drift life named.
    fn error_ppm(&mut self, r: &Resistor) -> Result<i128, IrValidationError> {
        let s = self.resistor(r)?;
        let e = resistor_error_ppm(s, self.temperature());
        self.note(format!(
            "{}: its drift is counted as {} (the 8000 h life, ruled D-B4; {})",
            r.part,
            s.drift_condition,
            self.cite(&s.drift_ppm.at)
        ));
        Ok(e)
    }
}

/// Battery minimum and maximum, mV.
fn battery_range(b: &Battery) -> (u64, u64) {
    (
        u64::from(b.cells.min) * u64::from(b.cell_min_millivolts.value),
        u64::from(b.cells.max) * u64::from(b.cell_max_millivolts.value),
    )
}

/// A domain's range, mV.
fn range(d: &VoltageDomain) -> (u64, u64) {
    (
        u64::from(minimum_millivolts(d)),
        u64::from(maximum_millivolts(d)),
    )
}

/// The input range of a regulator, mV: the battery's, or its input domain's.
fn input_range(ctx: &Ctx<'_>, r: &Regulator) -> (u64, u64) {
    if r.input == BATTERY {
        battery_range(&ctx.power.battery)
    } else {
        ctx.domain(&r.input).map_or((0, 0), range)
    }
}

/// Step 1: shape. Every reference distinct (with the specification's own),
/// every name an identifier, every source text a source, every number in
/// range, every domain named exists.
fn shape(spec: &Specification, power: &Power) -> Result<(), IrValidationError> {
    let id = |v: &str, path: String| crate::plain(v, || path);
    let invalid = |path: &str, detail: String| {
        refuse(
            Refusal::PowerValueInvalid,
            path,
            detail,
            "docs/work-orders/AET-WO-4300d.md (power)",
        )
    };
    let t = &power.conditions.board_temperature_celsius;
    crate::board::source(
        &t.source,
        "power.conditions.board_temperature_celsius.source",
    )?;
    if t.min > t.max || !(-273..=500).contains(&t.min) || !(-273..=500).contains(&t.max) {
        return Err(invalid(
            "power.conditions.board_temperature_celsius",
            format!(
                "{} to {} °C is not a range from -273 to 500 °C",
                t.min, t.max
            ),
        ));
    }
    for (i, d) in power.conditions.dc_bias_derating.iter().enumerate() {
        let path = format!("power.conditions.dc_bias_derating[{i}]");
        crate::board::source(&d.source, &format!("{path}.source"))?;
        let mut kinds = d.dielectrics.clone();
        kinds.sort();
        kinds.dedup();
        if kinds.is_empty() || kinds.len() != d.dielectrics.len() {
            return Err(invalid(
                &path,
                "names no dielectric, or one twice".to_owned(),
            ));
        }
        // A wider band keeping more, or two bands at one ratio, would make
        // the verdict depend on the bands' order (adversarial F6).
        let clash = power.conditions.dc_bias_derating[..i].iter().find(|e| {
            e.dielectrics.iter().any(|k| d.dielectrics.contains(k))
                && (e.ratio_max_permille == d.ratio_max_permille
                    || (e.ratio_max_permille < d.ratio_max_permille)
                        != (e.retained_permille > d.retained_permille)
                        && e.retained_permille != d.retained_permille)
        });
        if clash.is_some() {
            return Err(invalid(
                &path,
                "shares a dielectric with an earlier band at the same ratio, or keeps more over a wider ratio".to_owned(),
            ));
        }
        // Ruled on PR #158: a class-2 ceramic's declared derating is the
        // conservative 50 % (SBVS320D p. 15) until a maker's curve is held.
        let class2 = d.dielectrics.iter().any(|k| *k != pp::Dielectric::C0g);
        if class2 && d.retained_permille > 500 {
            return Err(invalid(
                &path,
                format!(
                    "keeps {} per mille of a class-2 ceramic; the declared derating keeps at most 500 (the conservative 50 %, SBVS320D p. 15, ruled on PR #158)",
                    d.retained_permille
                ),
            ));
        }
        // A derating above 1 would raise capacitance under bias.
        if !(1..=1000).contains(&d.ratio_max_permille) || !(1..=1000).contains(&d.retained_permille)
        {
            return Err(invalid(
                &path,
                format!(
                    "a ratio of {} and a retained share of {} per mille are not each 1 to 1000",
                    d.ratio_max_permille, d.retained_permille
                ),
            ));
        }
    }
    id(&power.input.reference, "power.input.reference".into())?;
    id(&power.input.part, "power.input.part".into())?;
    let b = &power.battery;
    crate::board::source(&b.chemistry, "power.battery.chemistry")?;
    crate::board::source(
        &b.cell_min_millivolts.source,
        "power.battery.cell_min_millivolts.source",
    )?;
    crate::board::source(
        &b.cell_max_millivolts.source,
        "power.battery.cell_max_millivolts.source",
    )?;
    if b.cells.min == 0 || b.cells.min > b.cells.max || b.cells.max > 24 {
        return Err(invalid(
            "power.battery.cells",
            format!(
                "{} to {} cells is not 1 <= min <= max <= 24",
                b.cells.min, b.cells.max
            ),
        ));
    }
    let (cmin, cmax) = (b.cell_min_millivolts.value, b.cell_max_millivolts.value);
    if cmin == 0 || cmin > cmax || cmax > 10_000 {
        return Err(invalid(
            "power.battery",
            format!("a cell of {cmin} to {cmax} mV is not 1 <= min <= max <= 10000 mV"),
        ));
    }
    let mut references = vec![power.input.reference.as_str()];
    for (i, h) in power.input.hot_plug.iter().enumerate() {
        id(&h.reference, format!("power.input.hot_plug[{i}].reference"))?;
        id(&h.part, format!("power.input.hot_plug[{i}].part"))?;
        references.push(&h.reference);
    }
    let controls = |list: &[Control], path: &str| -> Result<(), IrValidationError> {
        for (i, c) in list.iter().enumerate() {
            id(&c.pin, format!("{path}[{i}].pin"))?;
            if c.strap.is_some() == c.wire.is_some() {
                return Err(invalid(
                    &format!("{path}[{i}]"),
                    "a control pin takes exactly one of `strap` and `wire`".to_owned(),
                ));
            }
            if let Some(w) = &c.wire {
                id(&w.net, format!("{path}[{i}].wire.net"))?;
                crate::board::source(&w.role, &format!("{path}[{i}].wire.role"))?;
            }
        }
        Ok(())
    };
    let resistor = |r: &Resistor, path: &str| -> Result<(), IrValidationError> {
        id(&r.reference, format!("{path}.reference"))?;
        id(&r.part, format!("{path}.part"))?;
        if r.ohms == 0 {
            return Err(invalid(&r.reference, "a resistor of 0 ohms".to_owned()));
        }
        Ok(())
    };
    let externals = |list: &[External], path: &str| -> Result<(), IrValidationError> {
        for (i, e) in list.iter().enumerate() {
            id(&e.reference, format!("{path}[{i}].reference"))?;
            id(&e.part, format!("{path}[{i}].part"))?;
        }
        Ok(())
    };
    let domains: BTreeSet<&str> = spec.voltage_domains.iter().map(|d| d.id.as_str()).collect();
    let known = |d: &str| {
        if domains.contains(d) {
            Ok(())
        } else {
            Err(IrValidationError::UnknownDomain { id: d.to_owned() })
        }
    };
    for (i, r) in power.regulators.iter().enumerate() {
        let path = format!("power.regulators[{i}]");
        id(&r.reference, format!("{path}.reference"))?;
        id(&r.part, format!("{path}.part"))?;
        if r.input != BATTERY {
            known(&r.input)?;
        }
        known(&r.output)?;
        references.push(&r.reference);
        if let Some(f) = &r.feedback {
            resistor(&f.top, &format!("{path}.feedback.top"))?;
            resistor(&f.bottom, &format!("{path}.feedback.bottom"))?;
            references.extend([f.top.reference.as_str(), f.bottom.reference.as_str()]);
        }
        externals(&r.external, &format!("{path}.external"))?;
        references.extend(r.external.iter().map(|e| e.reference.as_str()));
        controls(&r.controls, &format!("{path}.controls"))?;
        if let Some(j) = &r.junction {
            for (what, v, max) in [
                (
                    "copper_square_centimetres",
                    &j.copper_square_centimetres,
                    10_000,
                ),
                ("loss_milliwatts", &j.loss_milliwatts, 1_000_000),
            ] {
                crate::board::source(&v.source, &format!("{path}.junction.{what}.source"))?;
                if v.value == 0 || v.value > max {
                    return Err(invalid(
                        &r.reference,
                        format!("{path}.junction.{what} is {}, not 1 to {max}", v.value),
                    ));
                }
            }
        }
    }
    for (i, l) in power.loads.iter().enumerate() {
        known(&l.domain)?;
        crate::board::source(&l.source, &format!("power.loads[{i}].source"))?;
        if l.milliamps == 0 {
            return Err(invalid(
                &l.domain,
                format!("power.loads[{i}] declares 0 mA"),
            ));
        }
    }
    // A sense net is a new net: folded, it may not be a signal, a domain or
    // `GND` (KiCad's ERC compares labels with case folded, `AET-WO-1204`).
    let mut taken: BTreeSet<String> = spec
        .i2c_buses
        .iter()
        .flat_map(|b| [&b.sda.net, &b.scl.net])
        .chain(
            spec.spi_buses
                .iter()
                .flat_map(|b| [&b.sck.net, &b.mosi.net, &b.miso.net]),
        )
        .chain(spec.declared_spi_devices().iter().map(|d| &d.cs.net))
        .map(|n| n.to_lowercase())
        .collect();
    let rails: BTreeSet<String> = spec
        .voltage_domains
        .iter()
        .map(|d| d.id.to_lowercase())
        .chain(std::iter::once(crate::GND.to_lowercase()))
        .collect();
    for (i, s) in power.sense.iter().enumerate() {
        let path = format!("power.sense[{i}]");
        id(&s.net, format!("{path}.net"))?;
        let folded = s.net.to_lowercase();
        if rails.contains(&folded) {
            return Err(IrValidationError::NetShadowsDomain { id: s.net.clone() });
        }
        if !taken.insert(folded) {
            return Err(IrValidationError::DuplicateNetName { id: s.net.clone() });
        }
        resistor(&s.top, &format!("{path}.top"))?;
        resistor(&s.bottom, &format!("{path}.bottom"))?;
        references.extend([s.top.reference.as_str(), s.bottom.reference.as_str()]);
        if let Some(c) = &s.reservoir {
            id(&c.reference, format!("{path}.reservoir.reference"))?;
            id(&c.part, format!("{path}.reservoir.part"))?;
            references.push(&c.reference);
        }
    }
    for (i, c) in power.crossings.iter().enumerate() {
        let path = format!("power.crossings[{i}]");
        id(&c.reference, format!("{path}.reference"))?;
        id(&c.part, format!("{path}.part"))?;
        id(&c.bus, format!("{path}.bus"))?;
        known(&c.a_domain)?;
        known(&c.b_domain)?;
        resistor(&c.cs_pull_up, &format!("{path}.cs_pull_up"))?;
        externals(&c.external, &format!("{path}.external"))?;
        controls(&c.controls, &format!("{path}.controls"))?;
        references.push(&c.reference);
        references.push(&c.cs_pull_up.reference);
        references.extend(c.external.iter().map(|e| e.reference.as_str()));
    }
    crate::references_distinct(
        spec.declared_i2c_devices()
            .iter()
            .map(|d| d.reference.as_str())
            .chain(
                spec.declared_spi_devices()
                    .iter()
                    .map(|d| d.reference.as_str()),
            )
            .chain(
                spec.declared_connectors()
                    .iter()
                    .map(|c| c.reference.as_str()),
            )
            .chain(references),
    )
}

/// Step 2: the tree. Every domain has exactly one source; following inputs
/// from any domain reaches the battery with no repeat.
fn tree(spec: &Specification, power: &Power) -> Result<(), IrValidationError> {
    let cite = "AET-WO-4300d spike S2, the tree";
    for d in &spec.voltage_domains {
        let sources: Vec<&Regulator> = power
            .regulators
            .iter()
            .filter(|r| r.output == d.id)
            .collect();
        match sources.as_slice() {
            [] => {
                return Err(refuse(
                    Refusal::DomainWithoutSource,
                    &d.id,
                    "no regulator's output is this domain".to_owned(),
                    cite,
                ));
            }
            [_] => {}
            [a, b, ..] => {
                return Err(refuse(
                    Refusal::DomainWithTwoSources,
                    &d.id,
                    format!(
                        "both `{}` and `{}` output this domain",
                        a.reference, b.reference
                    ),
                    cite,
                ));
            }
        }
    }
    for r in &power.regulators {
        let mut seen = vec![r.output.as_str()];
        let mut at = r;
        while at.input != BATTERY {
            if seen.contains(&at.input.as_str()) {
                return Err(refuse(
                    Refusal::PowerLoop,
                    &r.reference,
                    format!(
                        "following inputs from `{}` returns to `{}`",
                        r.output, at.input
                    ),
                    cite,
                ));
            }
            seen.push(&at.input);
            at = power
                .regulators
                .iter()
                .find(|q| q.output == at.input)
                .expect("every domain has one source (checked above)");
        }
    }
    Ok(())
}

/// The derived facts of one regulator, in order of the specification.
struct Derived<'a> {
    reg: &'a Regulator,
    row: &'a pp::Row,
    vout_uv: (u64, u64),
    source: (String, String),
    ret: (String, String),
}

/// Step 3: each regulator's guaranteed output at its source pad, µV, judged
/// against its domain's range (`RegulatorOutsideDomain`), with the pads.
fn outputs<'a>(ctx: &mut Ctx<'a>) -> Result<Vec<Derived<'a>>, IrValidationError> {
    let mut out = Vec::new();
    for reg in &ctx.power.regulators {
        let row = ctx.row(&reg.reference, &reg.part)?;
        let domain = ctx.domain(&reg.output).expect("checked in shape");
        let (vout, source, ret, citation) = match &row.kind {
            Kind::Buck(b) => {
                let Some(f) = &reg.feedback else {
                    return Err(refuse(
                        Refusal::ExternalPartMissing,
                        &reg.reference,
                        "a buck needs its feedback divider, and `feedback` is null",
                        ctx.cite(&b.vfb_microvolts.at),
                    ));
                };
                if i64::from(f.top.ohms) > b.rfbt_max_ohms.value {
                    return Err(refuse(
                        Refusal::ExternalPartOutOfRange,
                        &f.top.reference,
                        format!(
                            "RFBT is {} ohms, above the {} ohms maximum",
                            f.top.ohms, b.rfbt_max_ohms.value
                        ),
                        ctx.cite(&b.rfbt_max_ohms.at),
                    ));
                }
                let et = ctx.error_ppm(&f.top)?;
                let eb = ctx.error_ppm(&f.bottom)?;
                if et >= 1_000_000 || eb >= 1_000_000 {
                    return Err(refuse(
                        Refusal::RegulatorOutsideDomain,
                        &reg.reference,
                        "a feedback resistor's worst error reaches 100 %",
                        ctx.cite(&b.vfb_microvolts.at),
                    ));
                }
                let (rt, rb) = (i128::from(f.top.ohms), i128::from(f.bottom.ohms));
                let m = 1_000_000i128;
                let vfb = (
                    i128::from(b.vfb_microvolts.range.min),
                    i128::from(b.vfb_microvolts.range.max),
                );
                // VOUT = VFB (1 + RT/RB) ± IFB·RT, all in nV; the band after.
                let rt_hi = rt * (m + et);
                let rt_lo = rt * (m - et);
                let rb_hi = rb * (m + eb);
                let rb_lo = rb * (m - eb);
                let ifb = i128::from(b.ifb_max_nanoamps.value);
                let lo_nv = vfb.0 * 1000 * (rb_hi + rt_lo) / rb_hi - div_ceil(ifb * rt_hi, m);
                let hi_nv =
                    div_ceil(vfb.1 * 1000 * (rb_lo + rt_hi), rb_lo) + div_ceil(ifb * rt_hi, m);
                let band = &b.regulation_permille.range;
                let lo = lo_nv * (1000 + i128::from(band.min)) / 1_000_000;
                let hi = div_ceil(hi_nv * (1000 + i128::from(band.max)), 1_000_000);
                if !b.regulation_production_tested {
                    ctx.note(format!(
                        "{}: the output regulation band {}..{} per mille is a design limit, not production tested ({})",
                        reg.part,
                        band.min,
                        band.max,
                        ctx.cite(&b.regulation_permille.at)
                    ));
                }
                (
                    (
                        u64::try_from(lo.max(0)).unwrap_or(0),
                        u64::try_from(hi).unwrap_or(u64::MAX),
                    ),
                    (f.top.reference.clone(), "1".to_owned()),
                    (f.bottom.reference.clone(), "2".to_owned()),
                    ctx.cite(&b.vfb_microvolts.at),
                )
            }
            Kind::Ldo(l) => {
                if reg.feedback.is_some() {
                    return Err(refuse(
                        Refusal::PowerValueInvalid,
                        &reg.reference,
                        "a fixed-output LDO takes no feedback divider",
                        ctx.cite(&l.vout_millivolts.at),
                    ));
                }
                ctx.note(format!(
                    "{}: the output range is stated {} ({})",
                    reg.part,
                    l.vout_condition,
                    ctx.cite(&l.vout_millivolts.at)
                ));
                let pad = |role: &str| row.pins.get(role).cloned().unwrap_or_default();
                (
                    (
                        u64::try_from(l.vout_millivolts.range.min).unwrap_or(0) * 1000,
                        u64::try_from(l.vout_millivolts.range.max).unwrap_or(0) * 1000,
                    ),
                    (reg.reference.clone(), pad("OUT")),
                    (reg.reference.clone(), pad("GND")),
                    ctx.cite(&l.vout_millivolts.at),
                )
            }
            _ => return Err(ctx.kind_err(&reg.reference, &reg.part, "regulator (buck or LDO)")),
        };
        let (dmin, dmax) = range(domain);
        if vout.0 < dmin * 1000 || vout.1 > dmax * 1000 {
            return Err(refuse(
                Refusal::RegulatorOutsideDomain,
                &reg.reference,
                format!(
                    "the output is {} to {} µV, outside `{}`'s {} to {} mV",
                    vout.0, vout.1, domain.id, dmin, dmax
                ),
                citation,
            ));
        }
        out.push(Derived {
            reg,
            row,
            vout_uv: vout,
            source,
            ret,
        });
    }
    Ok(out)
}

/// One consumer of a domain: its supply range and its floor, mV, with the
/// citation of both.
struct Consumer {
    name: String,
    range: (i64, i64),
    floor: i64,
    /// The floor `SolvedTopology` carries for the copper bound: above
    /// `floor` where another check judges the rest (an LDO's dropout, A3).
    solved_floor: i64,
    cite: String,
}

/// The consumers of a domain whose ranges the power rows hold.
fn consumers(ctx: &Ctx<'_>, domain: &str) -> Result<Vec<Consumer>, IrValidationError> {
    let mut out = Vec::new();
    let spec = ctx.spec;
    if spec.mcu.supply == domain {
        let (row, m) = mcu(ctx)?;
        let level = usize::from(spec.mcu.bor_level);
        let bor = m.bor_falling_max_millivolts.get(level).ok_or_else(|| {
            refuse(
                Refusal::PowerValueInvalid,
                "mcu",
                format!("mcu.bor_level {level} has no row"),
                ctx.cite(&m.vdd_millivolts.at),
            )
        })?;
        let floor = m.vdd_millivolts.range.min.max(bor.value);
        out.push(Consumer {
            name: format!("{} (MCU)", row.part),
            range: (m.vdd_millivolts.range.min, m.vdd_millivolts.range.max),
            floor,
            solved_floor: floor,
            cite: format!(
                "{}; VBOR{level} falling max {}",
                ctx.cite(&m.vdd_millivolts.at),
                ctx.cite(&bor.at)
            ),
        });
    }
    for r in &ctx.power.regulators {
        if r.input != domain {
            continue;
        }
        let row = ctx.row(&r.reference, &r.part)?;
        // An LDO's solved floor is what its dropout needs, not its input
        // minimum: its output holds only with that headroom (A3); the
        // dropout check judges it, so the range check reads the minimum.
        let (vin, at, solved_floor) = match &row.kind {
            Kind::Buck(b) => (
                &b.vin_millivolts,
                &b.vin_millivolts.at,
                b.vin_millivolts.range.min,
            ),
            Kind::Ldo(l) => (
                &l.vin_millivolts,
                &l.vin_millivolts.at,
                l.vin_millivolts
                    .range
                    .min
                    .max(l.vout_millivolts.range.max + l.dropout_max_millivolts.value),
            ),
            _ => return Err(ctx.kind_err(&r.reference, &r.part, "regulator (buck or LDO)")),
        };
        out.push(Consumer {
            name: format!("{} ({} input)", r.reference, r.part),
            range: (vin.range.min, vin.range.max),
            floor: vin.range.min,
            solved_floor,
            cite: ctx.cite(at),
        });
    }
    for c in &ctx.power.crossings {
        let row = ctx.row(&c.reference, &c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(&c.reference, &c.part, "translator"));
        };
        for (side, d, v) in [
            ("VCCA", &c.a_domain, &t.vcca_millivolts),
            ("VCCB", &c.b_domain, &t.vccb_millivolts),
        ] {
            if d == domain {
                out.push(Consumer {
                    name: format!("{} ({} {side})", c.reference, c.part),
                    range: (v.range.min, v.range.max),
                    floor: v.range.min,
                    solved_floor: v.range.min,
                    cite: ctx.cite(&v.at),
                });
            }
        }
    }
    Ok(out)
}

fn mcu<'a>(ctx: &Ctx<'a>) -> Result<(&'a pp::Row, &'a pp::Mcu), IrValidationError> {
    let part = &ctx.spec.mcu.part;
    let row = ctx.row("mcu", part)?;
    match &row.kind {
        Kind::Mcu(m) => Ok((row, m)),
        _ => Err(ctx.kind_err("mcu", part, "mcu")),
    }
}

/// Step 4 (ruled D13 (a)): each domain's declared range lies inside every
/// consumer's supply range, and above every consumer's floor.
fn domains_inside_consumers(ctx: &Ctx<'_>) -> Result<(), IrValidationError> {
    for d in &ctx.spec.voltage_domains {
        let (dmin, dmax) = range(d);
        for Consumer {
            name,
            range: (cmin, cmax),
            floor,
            cite,
            ..
        } in consumers(ctx, &d.id)?
        {
            let inside =
                i128::from(dmin) >= i128::from(cmin) && i128::from(dmax) <= i128::from(cmax);
            if !inside || i128::from(dmin) <= i128::from(floor) {
                return Err(refuse(
                    Refusal::DomainOutsideConsumer,
                    &d.id,
                    format!(
                        "the domain's {dmin} to {dmax} mV is not inside {name}'s {cmin} to {cmax} mV with its minimum above the consumer's floor of {floor} mV"
                    ),
                    cite,
                ));
            }
        }
    }
    Ok(())
}

/// Step 5: the battery against every battery-fed regulator's input range.
fn battery_inputs(ctx: &Ctx<'_>) -> Result<(), IrValidationError> {
    let (bmin, bmax) = battery_range(&ctx.power.battery);
    for r in ctx.power.regulators.iter().filter(|r| r.input == BATTERY) {
        let row = ctx.row(&r.reference, &r.part)?;
        let vin = match &row.kind {
            Kind::Buck(b) => &b.vin_millivolts,
            Kind::Ldo(l) => &l.vin_millivolts,
            _ => return Err(ctx.kind_err(&r.reference, &r.part, "regulator (buck or LDO)")),
        };
        let ok = i128::from(bmin) >= i128::from(vin.range.min)
            && i128::from(bmax) <= i128::from(vin.range.max);
        // D-B9: the regulation band holds only from its stated VIN.
        if ok && let Kind::Buck(b) = &row.kind {
            let nominal = ctx
                .domain(&r.output)
                .map_or(0, |d| i64::from(d.nominal_millivolts));
            if let Some((_, floor)) = b
                .regulation_vin_min
                .iter()
                .find(|(from, _)| nominal >= *from)
                && i128::from(bmin) < i128::from(floor.value)
            {
                return Err(refuse(
                    Refusal::BatteryOutsideRegulatorInput,
                    &r.reference,
                    format!(
                        "the battery's minimum of {bmin} mV is below the {} mV from which the output regulation band is stated for a {nominal} mV output (ruled D-B9)",
                        floor.value
                    ),
                    ctx.cite(&floor.at),
                ));
            }
        }
        if !ok {
            return Err(refuse(
                Refusal::BatteryOutsideRegulatorInput,
                &r.reference,
                format!(
                    "the battery's {bmin} to {bmax} mV is outside the input's {} to {} mV",
                    vin.range.min, vin.range.max
                ),
                ctx.cite(&vin.at),
            ));
        }
    }
    Ok(())
}

/// Step 6: each regulator's load, mA, children first; refused above IOUT.
fn loads(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
) -> Result<BTreeMap<String, u64>, IrValidationError> {
    let mut load: BTreeMap<String, u64> = BTreeMap::new();
    // Resolve in an order where every child precedes its parent: repeat
    // until fixed; the tree has no loop (step 2) and at most one source per
    // domain, so this ends in at most n rounds.
    let n = derived.len();
    for _ in 0..=n {
        for d in derived {
            if load.contains_key(&d.reg.output) {
                continue;
            }
            let children: Vec<&Derived<'_>> = derived
                .iter()
                .filter(|c| c.reg.input == d.reg.output)
                .collect();
            if children.iter().any(|c| !load.contains_key(&c.reg.output)) {
                continue;
            }
            let mut sum: u64 = ctx
                .power
                .loads
                .iter()
                .filter(|l| l.domain == d.reg.output)
                .map(|l| u64::from(l.milliamps))
                .sum();
            if ctx.spec.mcu.supply == d.reg.output {
                let (_, m) = mcu(ctx)?;
                sum += u64::try_from(m.idd_max_milliamps.value).unwrap_or(0);
                ctx.note(format!(
                    "MCU load: {} ({})",
                    m.idd_condition,
                    ctx.cite(&m.idd_max_milliamps.at)
                ));
            }
            for c in children {
                let child = load[&c.reg.output];
                sum += match &c.row.kind {
                    Kind::Ldo(l) => {
                        ctx.note(format!(
                            "{}: its ground current is the no-load maximum; no figure under load is held ({})",
                            c.reg.part,
                            ctx.cite(&l.ignd_max_microamps.at)
                        ));
                        child
                            + u64::try_from(l.ignd_max_microamps.value)
                                .unwrap_or(0)
                                .div_ceil(1000)
                    }
                    _ => {
                        ctx.note(format!(
                            "{}: a buck's own bias current, beyond its output, is not guaranteed and is not counted in its input",
                            c.reg.reference
                        ));
                        child
                    }
                };
            }
            if ctx
                .power
                .crossings
                .iter()
                .any(|c| c.a_domain == d.reg.output || c.b_domain == d.reg.output)
            {
                ctx.note(format!(
                    "{}: a translator's supply current is not counted in its domain's load",
                    d.reg.output
                ));
            }
            load.insert(d.reg.output.clone(), sum);
        }
    }
    for d in derived {
        let (iout, at) = match &d.row.kind {
            Kind::Buck(b) => (&b.iout_max_milliamps, &b.iout_max_milliamps.at),
            Kind::Ldo(l) => (&l.iout_max_milliamps, &l.iout_max_milliamps.at),
            _ => unreachable!("outputs refused every other kind"),
        };
        let have = load[&d.reg.output];
        if i128::from(have) > i128::from(iout.value) {
            return Err(refuse(
                Refusal::RegulatorOverloaded,
                &d.reg.reference,
                format!("its load is {have} mA, above its {} mA", iout.value),
                ctx.cite(at),
            ));
        }
    }
    Ok(load)
}

/// Step 7: the headroom and timing of each regulator.
fn headroom(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    load: &BTreeMap<String, u64>,
) -> Result<(), IrValidationError> {
    for d in derived {
        let (vin_min, vin_max) = if d.reg.input == BATTERY {
            battery_range(&ctx.power.battery)
        } else {
            input_range(ctx, d.reg)
        };
        let i = i128::from(load[&d.reg.output]);
        match &d.row.kind {
            Kind::Buck(b) => {
                // D ≤ 1 − tOFF-MIN(max) × fSW(max), in ppm, rounded down.
                let toff = i128::from(b.toff_min_max_ns.value);
                let fsw = i128::from(b.fsw_hz.range.max);
                let d_ppm = 1_000_000 - div_ceil(toff * fsw, 1000);
                let inductor = d
                    .reg
                    .external
                    .iter()
                    .find(|e| e.role == Role::Inductor)
                    .ok_or_else(|| {
                        refuse(
                            Refusal::ExternalPartMissing,
                            &d.reg.reference,
                            "a buck needs its inductor",
                            ctx.cite(&b.vin_millivolts.at),
                        )
                    })?;
                let lrow = ctx.row(&inductor.reference, &inductor.part)?;
                let Kind::Inductor(l) = &lrow.kind else {
                    return Err(ctx.kind_err(&inductor.reference, &inductor.part, "inductor"));
                };
                let dcr = i128::from(l.dcr_max_milliohms.value);
                ctx.note(format!(
                    "{}: {}'s DCR maximum is stated at 25 °C and is not raised for a {} °C board",
                    d.reg.reference,
                    inductor.part,
                    ctx.temperature().max
                ));
                let rhs = i128::from(b.rds_high_max_milliohms.value);
                let rls = i128::from(b.rds_low_max_milliohms.value);
                let vin_uv = i128::from(vin_min) * 1000;
                // mA × mΩ = µV.
                let avail = d_ppm * (vin_uv - i * rhs) / 1_000_000
                    - div_ceil((1_000_000 - d_ppm) * i * rls, 1_000_000)
                    - i * dcr;
                ctx.note(format!(
                    "{}: tOFF-MIN, RDS(on) and fSW are stated {} ({})",
                    d.reg.part,
                    b.stated_at,
                    ctx.cite(&b.toff_min_max_ns.at)
                ));
                ctx.note(format!(
                    "{}: the headroom counts {}'s DCR maximum ({}) and no input-path drop",
                    d.reg.reference,
                    inductor.part,
                    ctx.cite(&l.dcr_max_milliohms.at)
                ));
                if avail < i128::from(d.vout_uv.1) {
                    return Err(refuse(
                        Refusal::BuckHeadroomViolated,
                        &d.reg.reference,
                        format!(
                            "at its input's minimum of {vin_min} mV and {i} mA, at most {avail} µV is available (duty at most {d_ppm} ppm), below the output's maximum of {} µV",
                            d.vout_uv.1
                        ),
                        ctx.cite(&b.toff_min_max_ns.at),
                    ));
                }
                // tON = VOUT / (VIN × fSW) ≥ tON-MIN(max), at VIN max and fSW max.
                let ton = i128::from(b.ton_min_max_ns.value);
                let lhs = i128::from(d.vout_uv.0) * 1_000_000;
                let rhs_t = ton * i128::from(vin_max) * 1000 * fsw / 1000;
                if lhs < rhs_t {
                    return Err(refuse(
                        Refusal::OnTimeTooShort,
                        &d.reg.reference,
                        format!(
                            "at {vin_max} mV and {fsw} Hz the on-time is {} ns, below the {ton} ns minimum on-time",
                            lhs / (i128::from(vin_max) * fsw)
                        ),
                        ctx.cite(&b.ton_min_max_ns.at),
                    ));
                }
            }
            Kind::Ldo(l) => {
                let need = i128::from(l.vout_millivolts.range.max)
                    + i128::from(l.dropout_max_millivolts.value);
                if i > i128::from(l.dropout_at_milliamps) || i128::from(vin_min) < need {
                    return Err(refuse(
                        Refusal::DropoutViolated,
                        &d.reg.reference,
                        format!(
                            "its input's minimum is {vin_min} mV at {i} mA, and it needs {need} mV (dropout stated at {} mA)",
                            l.dropout_at_milliamps
                        ),
                        ctx.cite(&l.dropout_max_millivolts.at),
                    ));
                }
                // Tj = T_board,max + ((VIN,max − VOUT,min) I + VIN,max IGND) θJA.
                let p_uw = (i128::from(vin_max) - i128::from(l.vout_millivolts.range.min)) * i
                    + div_ceil(
                        i128::from(vin_max) * i128::from(l.ignd_max_microamps.value),
                        1000,
                    );
                let rise_mk = div_ceil(
                    p_uw * i128::from(l.theta_ja_decikelvin_per_watt.value),
                    10_000,
                );
                let tj_mk = i128::from(ctx.temperature().max) * 1000 + rise_mk;
                ctx.note(format!(
                    "{}: θJA is {} ({})",
                    d.reg.part,
                    l.theta_ja_condition,
                    ctx.cite(&l.theta_ja_decikelvin_per_watt.at)
                ));
                if tj_mk > i128::from(l.junction_max_celsius.value) * 1000 {
                    return Err(refuse(
                        Refusal::JunctionTooHot,
                        &d.reg.reference,
                        format!(
                            "{p_uw} µW raises the junction {rise_mk} mK above a {} °C board, to {tj_mk} m°C, above {} °C",
                            ctx.temperature().max,
                            l.junction_max_celsius.value
                        ),
                        ctx.cite(&l.junction_max_celsius.at),
                    ));
                }
            }
            _ => unreachable!("outputs refused every other kind"),
        }
    }
    Ok(())
}

/// Step 8 (ruled D2): the power-in connector, in steady state and in fault.
fn power_in(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    load: &BTreeMap<String, u64>,
) -> Result<(u64, u64), IrValidationError> {
    let input = &ctx.power.input;
    let Some(conn) = crate::connectors::power_in(&input.part) else {
        return Err(refuse(
            Refusal::PowerPartUnknown,
            &input.reference,
            format!("part `{}` is not a power-in connector", input.part),
            "crates/aethelgard-ir/src/connectors.rs (POWER_IN)",
        ));
    };
    let mut steady = 0u64;
    let mut fault = 0u64;
    for d in derived.iter().filter(|d| d.reg.input == BATTERY) {
        let l = load[&d.reg.output];
        match &d.row.kind {
            Kind::Buck(b) => {
                steady += l;
                fault += u64::try_from(b.high_side_limit_milliamps.range.max).unwrap_or(u64::MAX);
                ctx.note(format!(
                    "{}: the fault bound is the high-side current limit's maximum ({})",
                    d.reg.part,
                    ctx.cite(&b.high_side_limit_milliamps.at)
                ));
            }
            _ => {
                return Err(refuse(
                    Refusal::PowerInputOverloaded,
                    &d.reg.reference,
                    "an LDO fed from the battery has no cited fault bound on its input current",
                    conn.cited.section,
                ));
            }
        }
    }
    ctx.note(format!(
        "{}: rated {} mA, {} ({}, sha256 {}); no derating is held for a {} °C board against the part's {} °C; disagreeing: {}, {} (sha256 {}); {}, {} (sha256 {})",
        conn.name,
        conn.rated_milliamps,
        conn.rating_condition,
        conn.cited.document,
        conn.cited.sha256,
        ctx.temperature().max,
        conn.temperature_celsius.1,
        conn.disagreeing[0].document,
        conn.disagreeing[0].section,
        conn.disagreeing[0].sha256,
        conn.disagreeing[1].document,
        conn.disagreeing[1].section,
        conn.disagreeing[1].sha256
    ));
    for (what, amps) in [("steady state", steady), ("fault", fault)] {
        if amps > u64::from(conn.rated_milliamps) {
            return Err(refuse(
                Refusal::PowerInputOverloaded,
                &input.reference,
                format!(
                    "{what}: {amps} mA through it, above its rated {} mA",
                    conn.rated_milliamps
                ),
                format!("{}, {}", conn.cited.document, conn.cited.section),
            ));
        }
    }
    let t = ctx.temperature();
    if t.min < conn.temperature_celsius.0 || t.max > conn.temperature_celsius.1 {
        return Err(refuse(
            Refusal::BoardOutsidePartTemperature,
            &input.reference,
            format!(
                "the board's {} to {} °C is outside {}'s {} to {} °C",
                t.min, t.max, conn.name, conn.temperature_celsius.0, conn.temperature_celsius.1
            ),
            format!(
                "{}, {}",
                conn.temperature_cited.document, conn.temperature_cited.section
            ),
        ));
    }
    Ok((steady, fault))
}

/// Every part a power tree declares, with its reference.
fn every_part(power: &Power) -> Vec<(&str, &str)> {
    let mut v: Vec<(&str, &str)> = power
        .input
        .hot_plug
        .iter()
        .map(|h| (h.reference.as_str(), h.part.as_str()))
        .collect();
    for r in &power.regulators {
        v.push((&r.reference, &r.part));
        if let Some(f) = &r.feedback {
            v.push((&f.top.reference, &f.top.part));
            v.push((&f.bottom.reference, &f.bottom.part));
        }
        v.extend(
            r.external
                .iter()
                .map(|e| (e.reference.as_str(), e.part.as_str())),
        );
    }
    for s in &power.sense {
        v.push((&s.top.reference, &s.top.part));
        v.push((&s.bottom.reference, &s.bottom.part));
        if let Some(c) = &s.reservoir {
            v.push((&c.reference, &c.part));
        }
    }
    for c in &power.crossings {
        v.push((&c.reference, &c.part));
        v.push((&c.cs_pull_up.reference, &c.cs_pull_up.part));
        v.extend(
            c.external
                .iter()
                .map(|e| (e.reference.as_str(), e.part.as_str())),
        );
    }
    v
}

/// Step 9 (ruled D7): the board's declared temperature inside every power
/// part's operating range.
fn temperatures(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    let t = ctx.temperature();
    let mcu = ("mcu", ctx.spec.mcu.part.as_str());
    for (reference, part) in std::iter::once(mcu).chain(every_part(ctx.power)) {
        let row = ctx.row(reference, part)?;
        if row.lifecycle == pp::Lifecycle::Unread {
            ctx.note(format!(
                "{part}: its maker's page states no lifecycle, so its production status is unread"
            ));
        }
        if row.lifecycle == pp::Lifecycle::Eol {
            return Err(refuse(
                Refusal::PowerPartUnknown,
                reference,
                format!(
                    "`{part}` is end of life, and no acknowledgement of an EOL power part exists"
                ),
                "crates/aethelgard-ir/data/power-parts.json (lifecycle)",
            ));
        }
        let r = &row.temperature_celsius.range;
        if i64::from(t.min) < r.min || i64::from(t.max) > r.max {
            return Err(refuse(
                Refusal::BoardOutsidePartTemperature,
                reference,
                format!(
                    "the board's {} to {} °C is outside {part}'s {} to {} °C",
                    t.min, t.max, r.min, r.max
                ),
                ctx.cite(&row.temperature_celsius.at),
            ));
        }
    }
    Ok(())
}

/// Step 10 (ruled D6): every external part a row requires is declared, each
/// role's total rated value inside the row's range, each capacitor's
/// dielectric allowed and its rated voltage at least the row's and, where the
/// row says, the voltage the part works at; a buck's inductor saturates no
/// lower than the high-side limit's maximum.
fn externals(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    struct Owner<'a> {
        reference: &'a str,
        list: &'a [External],
        rules: &'a [pp::ExternalRule],
        /// The working voltage of each role, mV: an input capacitor sees
        /// the input's maximum, an output capacitor the domain's.
        working: Vec<(Role, u64)>,
        buck: Option<&'a pp::Buck>,
        vout_max_millivolts: u64,
        vout_nominal_millivolts: i64,
    }
    let mut owners = Vec::new();
    for r in &ctx.power.regulators {
        let row = ctx.row(&r.reference, &r.part)?;
        let vin_max = input_range(ctx, r).1;
        let domain = ctx.domain(&r.output);
        let vout_max = domain.map_or(0, |d| range(d).1);
        let nominal = domain.map_or(0, |d| i64::from(d.nominal_millivolts));
        let (rules, buck): (&[pp::ExternalRule], _) = match &row.kind {
            Kind::Buck(b) => (&b.external, Some(&**b)),
            Kind::Ldo(l) => (&l.external, None),
            _ => return Err(ctx.kind_err(&r.reference, &r.part, "regulator (buck or LDO)")),
        };
        owners.push(Owner {
            reference: &r.reference,
            list: &r.external,
            rules,
            working: vec![
                (Role::Cin, vin_max),
                (Role::CinHf, vin_max),
                (Role::Cout, vout_max),
            ],
            buck,
            vout_max_millivolts: vout_max,
            vout_nominal_millivolts: nominal,
        });
    }
    for c in &ctx.power.crossings {
        let row = ctx.row(&c.reference, &c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(&c.reference, &c.part, "translator"));
        };
        let a = ctx.domain(&c.a_domain).map_or(0, |d| range(d).1);
        let b = ctx.domain(&c.b_domain).map_or(0, |d| range(d).1);
        owners.push(Owner {
            reference: &c.reference,
            list: &c.external,
            rules: &t.external,
            working: vec![(Role::BypassA, a), (Role::BypassB, b)],
            buck: None,
            vout_max_millivolts: 0,
            vout_nominal_millivolts: 0,
        });
    }
    for o in owners {
        for e in o.list {
            if !o.rules.iter().any(|r| r.role == e.role) {
                return Err(refuse(
                    Refusal::ExternalPartOutOfRange,
                    &e.reference,
                    format!(
                        "`{}` names the role `{}`, which its row does not have",
                        o.reference,
                        e.role.name()
                    ),
                    "crates/aethelgard-ir/data/power-parts.json",
                ));
            }
        }
        for rule in o.rules {
            let parts: Vec<&External> = o.list.iter().filter(|e| e.role == rule.role).collect();
            if parts.is_empty() {
                return Err(refuse(
                    Refusal::ExternalPartMissing,
                    o.reference,
                    format!("no part is declared in the role `{}`", rule.role.name()),
                    ctx.cite(&rule.at),
                ));
            }
            if rule.role == Role::Inductor && parts.len() != 1 {
                return Err(refuse(
                    Refusal::ExternalPartOutOfRange,
                    o.reference,
                    format!(
                        "{} inductors are declared; the role takes exactly one",
                        parts.len()
                    ),
                    ctx.cite(&rule.at),
                ));
            }
            let mut total: i128 = 0;
            let mut caps: Vec<(&str, &pp::Capacitor)> = Vec::new();
            for e in &parts {
                let row = ctx.row(&e.reference, &e.part)?;
                match (&row.kind, rule.role) {
                    (Kind::Inductor(l), Role::Inductor) => {
                        let Some(b) = o.buck else {
                            return Err(ctx.kind_err(o.reference, "", "buck"));
                        };
                        // Eq. 5: L_MIN = k VOUT / fSW, at the domain's
                        // highest output and the lowest frequency, nH.
                        let need = div_ceil(
                            i128::from(b.inductor_min_coefficient.value)
                                * i128::from(o.vout_max_millivolts)
                                * 1000,
                            i128::from(b.fsw_hz.range.min),
                        );
                        if i128::from(l.nanohenries.range.min) < need {
                            return Err(refuse(
                                Refusal::ExternalPartOutOfRange,
                                &e.reference,
                                format!(
                                    "{} is at least {} nH at its tolerance, below Equation 5's {need} nH at {} mV and {} Hz",
                                    e.part,
                                    l.nanohenries.range.min,
                                    o.vout_max_millivolts,
                                    b.fsw_hz.range.min
                                ),
                                ctx.cite(&b.inductor_min_coefficient.at),
                            ));
                        }
                        ctx.note(format!(
                            "{}: the inductance meets Equation 5; Table 9-2 gives 8 uH at 5 V and 6.8 uH at 3.3 V, and Equation 4's 20 to 40 % ripple and the 10 % ripple floor are not met across the whole battery range (ruled D6) ({})",
                            o.reference,
                            ctx.cite(&rule.at)
                        ));
                        let limit = &b.high_side_limit_milliamps;
                        if l.isat_typical_milliamps.value < limit.range.max {
                            return Err(refuse(
                                Refusal::ExternalPartOutOfRange,
                                &e.reference,
                                format!(
                                    "{}'s saturation current, {} mA typical ({}), is below the high-side limit maximum of {} mA",
                                    e.part,
                                    l.isat_typical_milliamps.value,
                                    l.isat_definition,
                                    limit.range.max
                                ),
                                ctx.cite(&limit.at),
                            ));
                        }
                        ctx.note(format!(
                            "{}: {}'s saturation current is stated typical only ({} mA, {}; {}); it is screened against the high-side limit maximum of {} mA, a margin of {}.{:02} times, not guaranteed above it (ruled D-B3)",
                            e.reference,
                            e.part,
                            l.isat_typical_milliamps.value,
                            l.isat_definition,
                            ctx.cite(&l.isat_typical_milliamps.at),
                            limit.range.max,
                            l.isat_typical_milliamps.value * 100 / limit.range.max / 100,
                            l.isat_typical_milliamps.value * 100 / limit.range.max % 100
                        ));
                    }
                    (Kind::Capacitor(c), role) if role != Role::Inductor => {
                        total += i128::from(c.rated_picofarads.value);
                        caps.push((&e.part, c));
                        if c.dielectric == pp::Dielectric::X7s {
                            ctx.note(format!(
                                "{}: {} is X7S, accepted where TI's example says X7R or better (ruled D-B8)",
                                e.reference, e.part
                            ));
                        }
                        if !rule.dielectrics.contains(&c.dielectric) {
                            return Err(refuse(
                                Refusal::ExternalPartOutOfRange,
                                &e.reference,
                                format!(
                                    "{}'s dielectric is not one the role `{}` allows",
                                    e.part,
                                    role.name()
                                ),
                                ctx.cite(&rule.at),
                            ));
                        }
                        let working = if rule.rated_at_least_working {
                            o.working
                                .iter()
                                .find(|(r, _)| *r == role)
                                .map_or(0, |(_, v)| i64::try_from(*v).unwrap_or(i64::MAX))
                        } else {
                            0
                        };
                        let step = match rule.rated_step {
                            Some((from, rated)) if o.vout_nominal_millivolts >= from => rated,
                            _ => 0,
                        };
                        let floor = rule
                            .rated_min_millivolts
                            .unwrap_or(0)
                            .max(working)
                            .max(step);
                        if c.rated_millivolts.value < floor {
                            return Err(refuse(
                                Refusal::ExternalPartOutOfRange,
                                &e.reference,
                                format!(
                                    "{} is rated {} mV, below the {floor} mV the role `{}` needs",
                                    e.part,
                                    c.rated_millivolts.value,
                                    role.name()
                                ),
                                ctx.cite(&rule.at),
                            ));
                        }
                    }
                    _ => {
                        return Err(ctx.kind_err(
                            &e.reference,
                            &e.part,
                            if rule.role == Role::Inductor {
                                "inductor"
                            } else {
                                "capacitor"
                            },
                        ));
                    }
                }
            }
            if let Some(r) = &rule.value
                && (total < i128::from(r.min) || r.max.is_some_and(|m| total > i128::from(m)))
            {
                return Err(refuse(
                    Refusal::ExternalPartOutOfRange,
                    o.reference,
                    format!(
                        "the role `{}` totals {total} pF rated, outside {} to {} pF",
                        rule.role.name(),
                        r.min,
                        r.max
                            .map_or_else(|| "no stated maximum".to_owned(), |m| m.to_string())
                    ),
                    ctx.cite(&rule.at),
                ));
            }
            // §1.3: the effective capacitance under DC bias at the voltage
            // the role works at, screened where the row states a minimum.
            let working = o
                .working
                .iter()
                .find(|(r, _)| *r == rule.role)
                .map(|(_, v)| i64::try_from(*v).unwrap_or(i64::MAX));
            let role = rule.role.name();
            let screened = match working {
                Some(w) if !caps.is_empty() => Some(
                    caps.iter()
                        .map(|(part, c)| effective(ctx, part, c, w))
                        .collect::<Result<Vec<_>, (Refusal, String)>>()
                        .map(|v| {
                            let sum: i64 = v.iter().map(|(pf, _)| pf).sum();
                            let how: Vec<String> = v.into_iter().map(|(_, h)| h).collect();
                            (sum, how.join("; "), w)
                        }),
                ),
                _ => None,
            };
            match (&rule.effective_min_picofarads, screened) {
                _ if rule.role == Role::Inductor => {}
                (Some(min), None) => {
                    return Err(refuse(
                        Refusal::EffectiveCapacitanceBelowMinimum,
                        o.reference,
                        format!(
                            "`{role}`: no working voltage is known for the role, so the screen cannot clear it"
                        ),
                        ctx.cite(&min.limit.at),
                    ));
                }
                (Some(min), Some(Err((kind, why)))) => {
                    return Err(refuse(
                        kind,
                        o.reference,
                        format!("`{role}`: {why}, so the screen cannot clear it"),
                        ctx.cite(&min.limit.at),
                    ));
                }
                (Some(min), Some(Ok((eff, how, w))))
                    if eff <= min.limit.value && min.on_shortfall == pp::OnShortfall::Report =>
                {
                    // Ruled D2 (c), and again with the maker's curve held:
                    // named, not blocking; enforcing it, with the fixture's
                    // input re-picked, is the next power work order's.
                    let short = min.limit.value - eff;
                    ctx.note(format!(
                        "{}: `{role}` holds {}.{:03} µF effective at {w} mV under DC bias ({how}), {}.{:03} µF short of the {}.{:03} µF minimum ({}); a named shortfall, not blocking: its enforcement and the re-pick of the input capacitance are the next power work order's (ruled D2 (c))",
                        o.reference,
                        eff / 1_000_000,
                        eff % 1_000_000 / 1000,
                        short / 1_000_000,
                        short % 1_000_000 / 1000,
                        min.limit.value / 1_000_000,
                        min.limit.value % 1_000_000 / 1000,
                        ctx.cite(&min.limit.at)
                    ));
                }
                (Some(min), Some(Ok((eff, how, w)))) => {
                    let min = &min.limit;
                    if eff <= min.value {
                        return Err(refuse(
                            Refusal::EffectiveCapacitanceBelowMinimum,
                            o.reference,
                            format!(
                                "`{role}` holds {eff} pF effective at {w} mV ({how}), not above the {} pF minimum",
                                min.value
                            ),
                            ctx.cite(&min.at),
                        ));
                    }
                    ctx.note(format!(
                        "{}: `{role}` holds {eff} pF effective at {w} mV under DC bias ({how}), from rated capacitance before its tolerance; it is screened against the {} pF minimum ({}), a margin of {}.{:02} times, typical, not guaranteed",
                        o.reference,
                        min.value,
                        ctx.cite(&min.at),
                        eff * 100 / min.value / 100,
                        eff * 100 / min.value % 100
                    ));
                }
                (None, Some(Ok((eff, how, w)))) if matches!(rule.role, Role::Cin | Role::Cout) => {
                    ctx.note(format!(
                        "{}: `{role}` would hold {eff} pF effective at {w} mV under DC bias ({how}); its row states no effective minimum, so only the rated value is judged",
                        o.reference
                    ));
                }
                _ => {
                    ctx.note(format!(
                        "{}: `{role}` is judged at rated capacitance; its row states no effective minimum under DC bias",
                        o.reference
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Step 11 (ruled D10 (a)): the battery sense's full scale against VREF+ and
/// its source impedance against RAIN at the declared resolution, or the
/// reservoir capacitor AN2834 requires.
fn sense(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    let (_, bmax) = battery_range(&ctx.power.battery);
    let vref_min = ctx.domain(&ctx.spec.mcu.supply).map_or(0, |d| range(d).0);
    for s in &ctx.power.sense {
        let (_, m) = mcu(ctx)?;
        let et = ctx.error_ppm(&s.top)?;
        let eb = ctx.error_ppm(&s.bottom)?;
        let mm = 1_000_000i128;
        let (rt, rb) = (i128::from(s.top.ohms), i128::from(s.bottom.ohms));
        // The largest ratio: the bottom high, the top low.
        let num = rb * (mm + eb);
        let den = num + rt * (mm - et);
        let full_uv = div_ceil(i128::from(bmax) * 1000 * num, den);
        ctx.note(format!(
            "{}: VREF+ is taken as VDDA on the MCU's supply `{}`; the specification declares no separate reference",
            s.net, ctx.spec.mcu.supply
        ));
        // D3: VREF+ is a consumer of the MCU's supply, floored where the
        // divided battery still fits under it.
        ctx.floors.push((
            ctx.spec.mcu.supply.clone(),
            format!("VREF+ ({} full scale)", s.net),
            u64::try_from(div_ceil(full_uv, 1000)).unwrap_or(u64::MAX),
        ));
        if full_uv > i128::from(vref_min) * 1000 {
            return Err(refuse(
                Refusal::SenseFullScaleExceeded,
                &s.net,
                format!(
                    "the battery's {bmax} mV divides to at most {full_uv} µV, above VREF+'s minimum of {vref_min} mV"
                ),
                ctx.cite(&m.vdd_millivolts.at),
            ));
        }
        let Some(rain) = rain_row(m, s.adc_resolution_bits) else {
            return Err(refuse(
                Refusal::PowerValueInvalid,
                &s.net,
                format!(
                    "the MCU's row states no RAIN for {} bits",
                    s.adc_resolution_bits
                ),
                "crates/aethelgard-ir/data/power-parts.json (rain_max_ohms)",
            ));
        };
        // The Thevenin resistance at its largest: both resistors high.
        let rth_hi = rt * (mm + et);
        let rbh = rb * (mm + eb);
        let rth = div_ceil(rth_hi * rbh, (rth_hi + rbh) * mm);
        if rth <= i128::from(rain.value) {
            continue;
        }
        let Some(c) = &s.reservoir else {
            return Err(refuse(
                Refusal::SenseSourceImpedance,
                &s.net,
                format!(
                    "the divider's source resistance is up to {rth} ohms, above RAIN's {} ohms at {} bits, and no reservoir capacitor is declared",
                    rain.value, s.adc_resolution_bits
                ),
                ctx.cite(&rain.at),
            ));
        };
        let row = ctx.row(&c.reference, &c.part)?;
        let Kind::Capacitor(cap) = &row.kind else {
            return Err(ctx.kind_err(&c.reference, &c.part, "capacitor"));
        };
        let need = reservoir_min_picofarads(m, s.adc_resolution_bits);
        if i128::from(cap.rated_picofarads.value) < need {
            return Err(refuse(
                Refusal::SenseSourceImpedance,
                &c.reference,
                format!(
                    "the reservoir is {} pF, below the {need} pF the rule needs at {} bits",
                    cap.rated_picofarads.value, s.adc_resolution_bits
                ),
                ctx.cite(&m.reservoir_rule.at),
            ));
        }
        if cap.rated_millivolts.value < i64::try_from(vref_min).unwrap_or(i64::MAX) {
            return Err(refuse(
                Refusal::ExternalPartOutOfRange,
                &c.reference,
                format!(
                    "the reservoir is rated {} mV, below the pin's {vref_min} mV",
                    cap.rated_millivolts.value
                ),
                ctx.cite(&cap.rated_millivolts.at),
            ));
        }
        ctx.note(format!(
            "{}: the reservoir rule is {}; with CADC typical, {} pF is needed and {} pF is declared, a margin of {}.{:02} times (ruled D-B3); the reservoir is judged at rated capacitance, without its tolerance or DC bias; the ADC's sampling rate must let the reservoir recharge through the divider between conversions ({})",
            s.net,
            m.reservoir_rule.statement,
            need,
            cap.rated_picofarads.value,
            i128::from(cap.rated_picofarads.value) * 100 / need / 100,
            i128::from(cap.rated_picofarads.value) * 100 / need % 100,
            ctx.cite(&m.reservoir_rule.at)
        ));
    }
    Ok(())
}

/// The reservoir capacitance AN2834's rule needs at `bits`, pF.
pub fn reservoir_min_picofarads(m: &pp::Mcu, bits: u8) -> i128 {
    // Cext ≥ 2^(N + extra_bits) × C_ADC.
    let shift = u32::from(bits) + u32::from(m.reservoir_rule.extra_bits);
    div_ceil(
        i128::from(m.reservoir_rule.adc_capacitance_femtofarads) * (1i128 << shift.min(100)),
        1000,
    )
}

/// Step 12 (ruled D9): each crossing.
fn crossings(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    for c in &ctx.power.crossings {
        let row = ctx.row(&c.reference, &c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(&c.reference, &c.part, "translator"));
        };
        let invalid = |detail: String| {
            refuse(
                Refusal::CrossingInvalid,
                &c.reference,
                detail,
                ctx.cite(&t.vcca_millivolts.at),
            )
        };
        let Some(bus) = ctx.spec.spi_buses.iter().find(|b| b.id == c.bus) else {
            return Err(invalid(format!(
                "bus `{}` is not an SPI bus of the specification",
                c.bus
            )));
        };
        if ctx
            .power
            .crossings
            .iter()
            .filter(|o| o.bus == c.bus)
            .count()
            > 1
        {
            return Err(invalid(format!(
                "bus `{}` has more than one crossing",
                c.bus
            )));
        }
        let a_to_b = t
            .channels
            .iter()
            .filter(|ch| ch.direction == pp::Direction::AToB)
            .count();
        let b_to_a = t
            .channels
            .iter()
            .filter(|ch| ch.direction == pp::Direction::BToA)
            .count();
        if a_to_b < 3 || b_to_a < 1 {
            return Err(invalid(format!(
                "{} has {a_to_b} A-to-B and {b_to_a} B-to-A channels; SCK, MOSI and CS need three, MISO one",
                c.part
            )));
        }
        let devices: Vec<&crate::SpiDevice> = ctx
            .spec
            .declared_spi_devices()
            .iter()
            .filter(|d| d.bus == c.bus)
            .collect();
        let [device] = devices.as_slice() else {
            return Err(invalid(format!(
                "bus `{}` has {} devices; a crossing carries one chip select",
                c.bus,
                devices.len()
            )));
        };
        if bus.domain != c.a_domain || device.supply != c.b_domain {
            return Err(refuse(
                Refusal::CrossingDomainMismatch,
                &c.reference,
                format!(
                    "the crossing joins `{}` to `{}`, and bus `{}` is on `{}` with `{}` on `{}`",
                    c.a_domain, c.b_domain, bus.id, bus.domain, device.reference, device.supply
                ),
                ctx.cite(&t.vcca_millivolts.at),
            ));
        }
        let pmax = t.pull_up_max_ohms.value;
        let worst = div_ceil(
            i128::from(c.cs_pull_up.ohms) * (1_000_000 + ctx.error_ppm(&c.cs_pull_up)?),
            1_000_000,
        );
        if worst > i128::from(pmax) {
            return Err(refuse(
                Refusal::CrossingPullUp,
                &c.cs_pull_up.reference,
                format!(
                    "the CS pull-up on side A is up to {worst} ohms, above the {pmax} ohms the inputs' pull-downs allow"
                ),
                ctx.cite(&t.pull_up_max_ohms.at),
            ));
        }
        ctx.note(format!(
            "{}: TMAX is stated with one channel switching at 50 % duty, and SCK and MOSI switch together; tw(SCKL)'s 2 ns shortfall is stated for SCK at PCLK / 2 and is applied at every rate",
            c.reference
        ));
        // A clock at f toggles at 2f transitions a second; a data line at
        // most as often.
        let rate = 2 * u64::from(bus.speed_hz);
        for (dir, tmax) in [
            ("A to B", &t.tmax_a_to_b_bps),
            ("B to A", &t.tmax_b_to_a_bps),
        ] {
            if i128::from(rate) > i128::from(tmax.value) {
                return Err(refuse(
                    Refusal::CrossingTooFast,
                    &c.reference,
                    format!(
                        "bus `{}` at {} Hz needs {rate} bit/s {dir}, above {} bit/s",
                        bus.id, bus.speed_hz, tmax.value
                    ),
                    ctx.cite(&tmax.at),
                ));
            }
        }
        // MISO: SCK out (A→B), the device's output delay, MISO back (B→A)
        // and the MCU's input setup must fit in half a clock period.
        let (_, m) = mcu(ctx)?;
        let Some((tdo, tdo_cite)) = ctx.delays.get(&device.part).cloned() else {
            return Err(refuse(
                Refusal::OutputDelayMissing,
                &device.reference,
                format!(
                    "`{}` behind crossing `{}` has no output delay in the sensor table, so its MISO round trip cannot be judged",
                    device.part, c.reference
                ),
                "crates/aethelgard-ir/data/parts.json (output_delay)",
            ));
        };
        ctx.note(format!(
            "{}: the output delay is {tdo_cite}; the translator's delays are stated at its own test load",
            device.reference
        ));
        let total = i128::from(t.tpd_a_to_b_max_ns.value)
            + i128::from(tdo)
            + i128::from(t.tpd_b_to_a_max_ns.value)
            + i128::from(m.spi_input_setup_min_ns.value);
        // Half a period, ns, rounded down, less the SCK phase's own shortfall.
        let half = 500_000_000i128 / i128::from(bus.speed_hz)
            - i128::from(m.spi_sck_low_shortfall_ns.value);
        ctx.note(format!(
            "{}: the MISO round trip is {total} ns, which holds the bus `{}` at or under {} Hz, under its device's own limit (ruled D-B7)",
            c.reference,
            bus.id,
            500_000_000i128 / (total + i128::from(m.spi_sck_low_shortfall_ns.value))
        ));
        if total > half {
            return Err(refuse(
                Refusal::MisoRoundTripTooSlow,
                &c.reference,
                format!(
                    "SCK out {} + the device's output delay {tdo} + MISO back {} + the MCU's setup {} = {total} ns, above half of {} Hz's period, {half} ns",
                    t.tpd_a_to_b_max_ns.value,
                    t.tpd_b_to_a_max_ns.value,
                    m.spi_input_setup_min_ns.value,
                    bus.speed_hz
                ),
                ctx.cite(&t.tpd_a_to_b_max_ns.at),
            ));
        }
    }
    Ok(())
}

/// Step 9a (`AET-WO-1309` D1): each buck row's thermal-via rule, joined to
/// the declared board's ground planes (or `B.Cu` on a two-layer board, where
/// ground is a `B.Cu` zone), validated by [`crate::thermal::ThermalVias`].
fn thermal(ctx: &Ctx<'_>) -> Result<(), IrValidationError> {
    let board = &ctx.spec.board;
    let layers = thermal_layers(board);
    for r in &ctx.power.regulators {
        if let Kind::Buck(b) = &ctx.row(&r.reference, &r.part)?.kind {
            vias(&b.thermal_vias, layers.clone()).validate(&r.part, board)?;
        }
    }
    Ok(())
}

/// The layers a thermal array joins (`K7`): the declared board's `GND`
/// planes, or `B.Cu` on a two-layer board, where ground is a `B.Cu` zone.
fn thermal_layers(board: &crate::board::Board) -> Vec<String> {
    if board.copper_layers == 2 {
        vec!["B.Cu".to_owned()]
    } else {
        board
            .planes()
            .filter(|(_, net)| *net == crate::GND)
            .map(|(name, _)| name.to_owned())
            .collect()
    }
}

/// Each buck's thermal array on a board (`AET-WO-4300d2` §1.5): its
/// reference, the pad number its row's `pins` gives the rule's pad role, and
/// the rule joined to [`thermal_layers`], as step 9a validated it. For a
/// specification `validate` accepted.
pub fn thermal_arrays(spec: &Specification) -> Vec<(String, String, crate::thermal::ThermalVias)> {
    let (Some(p), Ok(table)) = (&spec.power, pp::table()) else {
        return Vec::new();
    };
    let layers = thermal_layers(&spec.board);
    let mut out = Vec::new();
    for r in &p.regulators {
        // belt-and-suspenders: `validate` refuses an unknown part and a pad
        // role the row does not map (step 9a, the parser's pin roles).
        let Some(row) = table.get(&r.part) else {
            continue;
        };
        let Kind::Buck(b) = &row.kind else { continue };
        let Some(pad) = row.pins.get(&b.thermal_vias.pad) else {
            continue;
        };
        out.push((
            r.reference.clone(),
            pad.clone(),
            vias(&b.thermal_vias, layers.clone()),
        ));
    }
    out
}

/// A row's thermal rule as `1309`'s array, on the given layers.
pub fn vias(rule: &pp::ThermalRule, layers: Vec<String>) -> crate::thermal::ThermalVias {
    crate::thermal::ThermalVias {
        columns: rule.columns,
        rows: rule.rows,
        pitch_um: rule.pitch_um,
        drill_um: rule.drill_um,
        ring_um: rule.ring_um,
        layers,
        sources: rule.sources.clone(),
    }
}

/// Step 5a (`AET-WO-4300d3` §1.1): the battery input's answer to a hot-plug.
/// Each part on it is cited and on the board's side of the connector. A bulk
/// capacitor's rating is above the battery's maximum, and the bulk total sits
/// in every battery-fed buck's damping range. A clamp stands off above the
/// battery's maximum and clamps at or below every battery-fed regulator's VIN
/// absolute maximum. With a regulator fed from the battery, one of the two
/// must hold. The ring's size is named, not computed.
fn hot_plug(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    let input = &ctx.power.input;
    let (_, bmax) = battery_range(&ctx.power.battery);
    let bmax = i64::try_from(bmax).unwrap_or(i64::MAX);
    // Each battery-fed regulator: its VIN ceiling and, for a buck, its range.
    let mut fed: Vec<(&str, &pp::Limit, Option<&pp::Ranged>)> = Vec::new();
    for r in ctx.power.regulators.iter().filter(|r| r.input == BATTERY) {
        match &ctx.row(&r.reference, &r.part)?.kind {
            Kind::Buck(b) => fed.push((
                &r.reference,
                &b.vin_abs_max_millivolts,
                Some(&b.input_damping_picofarads),
            )),
            Kind::Ldo(l) => fed.push((&r.reference, &l.vin_abs_max_millivolts, None)),
            _ => return Err(ctx.kind_err(&r.reference, &r.part, "regulator (buck or LDO)")),
        }
    }
    let rule_cite = fed
        .iter()
        .find_map(|(_, _, range)| range.map(|r| ctx.cite(&r.at)))
        .unwrap_or_else(|| "docs/work-orders/AET-WO-4300d3.md §1.1".to_owned());
    let mut bulk: i64 = 0;
    let mut clamps = 0;
    for h in &input.hot_plug {
        let row = ctx.row(&h.reference, &h.part)?;
        if h.side == Side::Pack {
            return Err(refuse(
                Refusal::BatteryInputUndamped,
                &h.reference,
                format!(
                    "`{}` sits on the pack's side of {}: beyond the leads' inductance, it does not damp the ring at the regulators' input",
                    h.part, input.reference
                ),
                rule_cite,
            ));
        }
        match &row.kind {
            Kind::BulkCapacitor(c) => {
                if c.rated_millivolts.value <= bmax {
                    return Err(refuse(
                        Refusal::BatteryInputUndamped,
                        &h.reference,
                        format!(
                            "{} is rated {} mV, not above the battery's maximum of {bmax} mV",
                            h.part, c.rated_millivolts.value
                        ),
                        ctx.cite(&c.rated_millivolts.at),
                    ));
                }
                bulk += c.rated_picofarads.value;
                ctx.note(format!(
                    "{}: {} damps the input's ring through its ESR, at most {} mOhm {} ({})",
                    h.reference,
                    h.part,
                    c.esr_max_milliohms.value,
                    c.esr_condition,
                    ctx.cite(&c.esr_max_milliohms.at)
                ));
            }
            Kind::Tvs(t) => {
                if t.standoff_millivolts.value <= bmax {
                    return Err(refuse(
                        Refusal::BatteryInputUndamped,
                        &h.reference,
                        format!(
                            "{} stands off {} mV, not above the battery's maximum of {bmax} mV",
                            h.part, t.standoff_millivolts.value
                        ),
                        ctx.cite(&t.standoff_millivolts.at),
                    ));
                }
                for (reference, ceiling, _) in &fed {
                    if t.clamp_max_millivolts.value > ceiling.value {
                        return Err(refuse(
                            Refusal::BatteryInputUndamped,
                            &h.reference,
                            format!(
                                "{} clamps at up to {} mV {}, above {reference}'s VIN absolute maximum of {} mV ({})",
                                h.part,
                                t.clamp_max_millivolts.value,
                                t.clamp_condition,
                                ceiling.value,
                                ctx.cite(&ceiling.at)
                            ),
                            ctx.cite(&t.clamp_max_millivolts.at),
                        ));
                    }
                }
                clamps += 1;
                ctx.note(format!(
                    "{}: {} clamps at up to {} mV {} ({})",
                    h.reference,
                    h.part,
                    t.clamp_max_millivolts.value,
                    t.clamp_condition,
                    ctx.cite(&t.clamp_max_millivolts.at)
                ));
            }
            _ => return Err(ctx.kind_err(&h.reference, &h.part, "bulk capacitor or TVS")),
        }
    }
    if fed.is_empty() {
        return Ok(());
    }
    let outside = fed.iter().find_map(|(reference, _, range)| {
        range
            .filter(|r| bulk < r.range.min || bulk > r.range.max)
            .map(|r| (*reference, r))
    });
    if clamps == 0 && (bulk == 0 || outside.is_some()) {
        let detail = match outside {
            Some((reference, r)) if bulk > 0 => format!(
                "its bulk capacitance totals {bulk} pF rated, outside {reference}'s {} to {} pF",
                r.range.min, r.range.max
            ),
            _ => "no bulk capacitor or clamp is declared on its board side".to_owned(),
        };
        return Err(refuse(
            Refusal::BatteryInputUndamped,
            &input.reference,
            format!("the battery input feeds a regulator, and {detail}"),
            rule_cite,
        ));
    }
    for (reference, ceiling, _) in &fed {
        ctx.note(format!(
            "{reference}: hot-plugging a pack through long leads rings VIN above the battery's {bmax} mV; the ring's size is not computed, and VIN's absolute maximum is {} mV ({})",
            ceiling.value,
            ctx.cite(&ceiling.at)
        ));
    }
    Ok(())
}

/// Step 7a (`AET-WO-4300d3` §1.2): each buck's junction, screened as
/// T_J = T_board,max + P_loss × θJA. P_loss is the declared bound, from a
/// typical efficiency; θJA is the row's typical curve at the declared copper
/// area. A screen with a named margin, never a guarantee.
fn junctions(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    load: &BTreeMap<String, u64>,
) -> Result<(), IrValidationError> {
    for d in derived {
        let r = d.reg;
        let b = match (&d.row.kind, &r.junction) {
            (Kind::Buck(b), _) => b,
            (_, Some(_)) => {
                return Err(refuse(
                    Refusal::PowerValueInvalid,
                    &r.reference,
                    "only a buck declares `junction`; an LDO's junction is judged from its row's θJA",
                    "docs/work-orders/AET-WO-4300d3.md §1.2",
                ));
            }
            _ => continue,
        };
        let curve = &b.theta_ja;
        let Some(j) = &r.junction else {
            return Err(refuse(
                Refusal::BuckJunctionOverLimit,
                &r.reference,
                "no copper area or loss bound is declared (`junction` is null), so the screen cannot clear it",
                ctx.cite(&curve.at),
            ));
        };
        let area = i64::from(j.copper_square_centimetres.value);
        let Some(&(at_area, theta)) = curve.points.iter().rev().find(|(a, _)| *a <= area) else {
            return Err(refuse(
                Refusal::BuckJunctionOverLimit,
                &r.reference,
                format!(
                    "the declared {area} cm² per layer is below the figure's smallest area, {} cm²",
                    curve.points.first().map_or(0, |p| p.0)
                ),
                ctx.cite(&curve.at),
            ));
        };
        let loss = i128::from(j.loss_milliwatts.value);
        // mW × (0.1 K/W) is 0.1 mK.
        let rise_mk = div_ceil(loss * i128::from(theta), 10);
        let tj_mk = i128::from(ctx.temperature().max) * 1000 + rise_mk;
        let limit = &d.row.temperature_celsius;
        let limit_mk = i128::from(limit.range.max) * 1000;
        if tj_mk > limit_mk {
            return Err(refuse(
                Refusal::BuckJunctionOverLimit,
                &r.reference,
                format!(
                    "{loss} mW at θJA {}.{} °C/W raises the junction {rise_mk} mK above a {} °C board, to {tj_mk} m°C, above {} °C",
                    theta / 10,
                    theta % 10,
                    ctx.temperature().max,
                    limit.range.max
                ),
                ctx.cite(&limit.at),
            ));
        }
        ctx.note(format!(
            "{}: its junction is screened, not guaranteed: {loss} mW of loss at {} mA (declared: {}) at θJA {}.{} °C/W, the figure's point at {at_area} cm² for the {area} cm² per layer declared ({}; {}; declared: {}), is {tj_mk} m°C on a {} °C board, {} mK under {} °C; the efficiency figure is typical at its own ambient",
            r.reference,
            load[&r.output],
            j.loss_milliwatts.source,
            theta / 10,
            theta % 10,
            curve.condition,
            ctx.cite(&curve.at),
            j.copper_square_centimetres.source,
            ctx.temperature().max,
            limit_mk - tj_mk,
            limit.range.max
        ));
        let layers = ctx.spec.board.copper_layers;
        if layers != curve.board_copper_layers {
            ctx.note(format!(
                "{}: the board declares {layers} copper layers, outside the θJA figure's {}-layer board, so the screen's θJA is not this board's (FEEDBACK D1)",
                r.reference, curve.board_copper_layers
            ));
        }
    }
    Ok(())
}

/// A dielectric as datasheets write it.
fn dielectric(d: pp::Dielectric) -> &'static str {
    match d {
        pp::Dielectric::C0g => "C0G",
        pp::Dielectric::X5r => "X5R",
        pp::Dielectric::X7r => "X7R",
        pp::Dielectric::X7s => "X7S",
    }
}

/// A ceramic's effective capacitance at `working` mV, pF, and how it was
/// found. Where its maker's curve is held: the curve, interpolated linearly
/// and rounded down, composed by product with the tolerance and the
/// temperature limit, worst case; past the curve's last point it is refused
/// `DcBiasBeyondCurve`, never extrapolated. Otherwise the declared derating
/// with the smallest ratio band that covers it, a placeholder. `Err` names
/// the refusal and why.
fn effective(
    ctx: &Ctx<'_>,
    part: &str,
    c: &pp::Capacitor,
    working: i64,
) -> Result<(i64, String), (Refusal, String)> {
    let rated = c.rated_picofarads.value;
    if let Some(curve) = &c.dc_bias {
        let Some(change) = curve_change(&curve.points, working) else {
            return Err((
                Refusal::DcBiasBeyondCurve,
                format!(
                    "{part} works at {working} mV, past its maker's curve, which ends at {} mV and is never extrapolated ({})",
                    curve.points.last().map_or(0, |p| p.0),
                    ctx.cite(&curve.at)
                ),
            ));
        };
        let (tol, temp) = (
            curve.tolerance_permille.value,
            curve.temperature_permille.value,
        );
        let pf = i128::from(rated)
            * i128::from(1000 + change)
            * i128::from(1000 - tol)
            * i128::from(1000 - temp)
            / 1_000_000_000;
        let pf = i64::try_from(pf).unwrap_or(0);
        return Ok((
            pf,
            format!(
                "{part}: {pf} pF, its rated {rated} pF times its maker's curve at {working} mV, {change} per mille interpolated ({}), its tolerance's -{tol} per mille ({}) and its temperature limit's -{temp} per mille ({}), by product, worst case",
                ctx.cite(&curve.at),
                ctx.cite(&curve.tolerance_permille.at),
                ctx.cite(&curve.temperature_permille.at)
            ),
        ));
    }
    let ratio = div_ceil(
        i128::from(working) * 1000,
        i128::from(c.rated_millivolts.value),
    );
    let d = ctx
        .power
        .conditions
        .dc_bias_derating
        .iter()
        .filter(|d| {
            d.dielectrics.contains(&c.dielectric) && i128::from(d.ratio_max_permille) >= ratio
        })
        .min_by_key(|d| d.ratio_max_permille);
    match d {
        Some(d) => {
            let derated = rated * i64::from(d.retained_permille) / 1000;
            Ok((
                derated,
                format!(
                    "{part}: {derated} pF, {} per mille of its rating kept at {ratio} per mille of its rated voltage, by the declared derating, a placeholder until its maker's curve is held",
                    d.retained_permille
                ),
            ))
        }
        None => Err((
            Refusal::EffectiveCapacitanceBelowMinimum,
            format!(
                "no maker curve is held for {part}, and no declared derating covers {} at {ratio} per mille of its rated voltage",
                dielectric(c.dielectric)
            ),
        )),
    }
}

/// A curve's change at `mv`, per mille: linear between the points around
/// it, rounded down (toward the larger loss); `None` past its last point.
fn curve_change(points: &[(i64, i64)], mv: i64) -> Option<i64> {
    let i = points.iter().rposition(|p| p.0 <= mv)?;
    let (v0, c0) = points[i];
    let Some(&(v1, c1)) = points.get(i + 1) else {
        return (mv == v0).then_some(c0);
    };
    let num = i128::from(c1 - c0) * i128::from(mv - v0);
    let step = -div_ceil(-num, i128::from(v1 - v0));
    i64::try_from(i128::from(c0) + step).ok()
}

/// Step 4a (`AET-WO-4300d3` §1.4): the MCU's supply floor clears the
/// selected brown-out level's rising maximum, as well as its falling one.
fn bor_rising(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    let (_, m) = mcu(ctx)?;
    let level = usize::from(ctx.spec.mcu.bor_level);
    let supply = &ctx.spec.mcu.supply;
    let (Some(rise), Some(d)) = (m.bor_rising_max_millivolts.get(level), ctx.domain(supply)) else {
        // belt-and-suspenders: step 4 refused an unknown level, and
        // `validate` an unknown supply, before this step.
        return Ok(());
    };
    let dmin = range(d).0;
    if i128::from(dmin) <= i128::from(rise.value) {
        return Err(refuse(
            Refusal::SupplyFloorBelowBorRising,
            supply,
            format!(
                "the domain's minimum of {dmin} mV does not clear VBOR{level}'s rising maximum of {} mV, so the MCU may not leave reset",
                rise.value
            ),
            ctx.cite(&rise.at),
        ));
    }
    ctx.note(format!(
        "{supply}: VBOR{level}'s rising maximum of {} mV is judged at the domain's minimum; the floor carried to the copper bound stays the falling maximum ({})",
        rise.value,
        ctx.cite(&rise.at)
    ));
    Ok(())
}

/// Step 9b (`AET-WO-4300d3` §1.5): each regulator's and translator's control
/// pins, each declared once, strapped as its row allows or wired to a named
/// net with a stated role.
fn controls(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    // Rails, ground and the battery, folded as KiCad folds labels.
    let rails: BTreeSet<String> = ctx
        .spec
        .voltage_domains
        .iter()
        .map(|d| d.id.to_lowercase())
        .chain([crate::GND, BATTERY_NET, BATTERY].map(str::to_lowercase))
        .collect();
    let mut wires = Vec::new();
    let owners = ctx
        .power
        .regulators
        .iter()
        .map(|r| (&r.reference, &r.part, &r.controls))
        .chain(
            ctx.power
                .crossings
                .iter()
                .map(|c| (&c.reference, &c.part, &c.controls)),
        );
    for (reference, part, declared) in owners {
        let rules: &[pp::ControlRule] = match &ctx.row(reference, part)?.kind {
            Kind::Buck(b) => &b.controls,
            Kind::Ldo(l) => &l.controls,
            Kind::Translator(t) => &t.controls,
            _ => return Err(ctx.kind_err(reference, part, "regulator or translator")),
        };
        for (i, c) in declared.iter().enumerate() {
            let twice = declared[..i].iter().any(|p| p.pin == c.pin);
            if twice || !rules.iter().any(|r| r.pin == c.pin) {
                return Err(refuse(
                    Refusal::PowerValueInvalid,
                    reference,
                    format!(
                        "`controls` names `{}` twice, or a pin its row lists as no control pin",
                        c.pin
                    ),
                    "crates/aethelgard-ir/data/power-parts.json (controls)",
                ));
            }
        }
        for rule in rules {
            let allowed = || {
                rule.straps
                    .iter()
                    .map(|s| s.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let unwired = |detail: String| {
                refuse(
                    Refusal::ControlPinUnwired,
                    reference,
                    detail,
                    ctx.cite(&rule.at),
                )
            };
            let Some(c) = declared.iter().find(|c| c.pin == rule.pin) else {
                return Err(unwired(format!(
                    "{} is declared neither wired nor strapped (its row allows straps: {})",
                    rule.pin,
                    allowed()
                )));
            };
            match (&c.strap, &c.wire) {
                (Some(s), _) if !rule.straps.contains(s) => {
                    return Err(unwired(format!(
                        "{} is strapped `{}`, which its row does not allow (it allows: {})",
                        rule.pin,
                        s.name(),
                        allowed()
                    )));
                }
                (_, Some(w)) if !rule.wirable => {
                    return Err(unwired(format!(
                        "{} is wired to `{}`, and its row allows no wire",
                        rule.pin, w.net
                    )));
                }
                (_, Some(w)) if rails.contains(&w.net.to_lowercase()) => {
                    return Err(unwired(format!(
                        "{} is wired to `{}`, a rail: a tie to a rail is a strap, which its row judges (adversarial F1, F3)",
                        rule.pin, w.net
                    )));
                }
                (_, Some(w)) => wires.push(format!(
                    "{reference}: {} is declared wired to `{}` ({}); neither the net's driver nor its level is judged, and the board does not draw it (FEEDBACK D5)",
                    rule.pin, w.net, w.role
                )),
                _ => {}
            }
        }
    }
    for w in wires {
        ctx.note(w);
    }
    Ok(())
}

/// Runs every check on a specification's power tree, in the order the module
/// documents, and returns what `solve` carries. `None` when the
/// specification declares no tree.
pub fn check(spec: &Specification) -> Result<Option<Checked>, IrValidationError> {
    if spec.power.is_none() {
        return Ok(None);
    }
    let sensors = crate::parts::table()?;
    check_against(
        spec,
        crate::power_parts::table()?,
        sensors,
        &output_delays(sensors),
    )
}

/// The output delay each SPI part's row holds, with its citation as the
/// verdict names it (`AET-WO-4300d2` K6). A part whose row holds none is
/// absent, and a crossing in front of it is refused `OutputDelayMissing`.
pub fn output_delays(sensors: &crate::parts::PartsTable) -> OutputDelays {
    sensors
        .parts()
        .iter()
        .filter_map(|p| {
            let ns = p.spi_output_delay_max_ns?;
            let c = p.citations.spi_output_delay.as_ref()?;
            Some((
                p.part.clone(),
                (ns, format!("{} {}, {}", c.document, c.revision, c.section)),
            ))
        })
        .collect()
}

/// Each sensor's SPI output delay after the clock edge, at most, ns, with its
/// citation, keyed by part (ruled D9).
pub type OutputDelays = BTreeMap<String, (u32, String)>;

/// [`check`] against the given tables: the embedded ones in every caller but
/// a test, which holds a table with one limit changed.
pub fn check_against(
    spec: &Specification,
    table: &PowerPartsTable,
    sensors: &crate::parts::PartsTable,
    delays: &OutputDelays,
) -> Result<Option<Checked>, IrValidationError> {
    let Some(power) = spec.power.as_ref() else {
        return Ok(None);
    };
    shape(spec, power)?;
    tree(spec, power)?;
    let mut ctx = Ctx {
        spec,
        power,
        table,
        delays,
        floors: Vec::new(),
        conditions: BTreeSet::new(),
    };
    mcu(&ctx)?;
    temperatures(&mut ctx)?;
    thermal(&ctx)?;
    controls(&mut ctx)?;
    let derived = outputs(&mut ctx)?;
    domains_inside_consumers(&ctx)?;
    bor_rising(&mut ctx)?;
    battery_inputs(&ctx)?;
    hot_plug(&mut ctx)?;
    let load = loads(&mut ctx, &derived)?;
    headroom(&mut ctx, &derived, &load)?;
    junctions(&mut ctx, &derived, &load)?;
    externals(&mut ctx)?;
    for d in &power.conditions.dc_bias_derating {
        let kinds: Vec<&str> = d.dielectrics.iter().map(|k| dielectric(*k)).collect();
        ctx.note(format!(
            "power: a {} ceramic at up to {} per mille of its rated voltage keeps {} per mille of its rated capacitance, a declared derating, not a maker's figure (declared: {})",
            kinds.join(" or "),
            d.ratio_max_permille,
            d.retained_permille,
            d.source
        ));
    }
    if !power.input.hot_plug.is_empty()
        || power.regulators.iter().any(|r| !r.controls.is_empty())
        || power.crossings.iter().any(|c| !c.controls.is_empty())
    {
        ctx.note(
            "power: the hot-plug parts and the control declarations are judged here, and the board emitter does not draw them: it ties a regulator's EN to its input, leaves PG open and ties OE to VCCA (FEEDBACK D5)",
        );
    }
    let (steady, fault) = power_in(&mut ctx, &derived, &load)?;
    sense(&mut ctx)?;
    crossings(&mut ctx)?;
    let mut domains = BTreeMap::new();
    for d in &derived {
        let mut consumers: Vec<(String, u64)> = consumers(&ctx, &d.reg.output)?
            .into_iter()
            .map(|c| (c.name, u64::try_from(c.solved_floor).unwrap_or(0)))
            .collect();
        // Each declared device on the domain, at its part's supply minimum:
        // `validate_parts` has already judged its range (`SupplyOutsidePart`).
        let devices = spec
            .declared_i2c_devices()
            .iter()
            .map(|d| (&d.reference, &d.part, &d.supply))
            .chain(
                spec.declared_spi_devices()
                    .iter()
                    .map(|d| (&d.reference, &d.part, &d.supply)),
            );
        let mut uncounted = Vec::new();
        for (reference, part, supply) in devices {
            if *supply == d.reg.output {
                uncounted.push(reference.as_str());
                if let Some(p) = sensors.get(part) {
                    consumers.push((
                        format!("{reference} ({part})"),
                        u64::from(p.supply_millivolts.0),
                    ));
                }
            }
        }
        if !uncounted.is_empty() {
            ctx.note(format!(
                "{}: the supply currents of {} are not in its load, which counts only declared `loads` and the MCU's row (A1)",
                d.reg.output,
                uncounted.join(", ")
            ));
        }
        for (domain, name, floor) in &ctx.floors {
            if *domain == d.reg.output {
                consumers.push((name.clone(), *floor));
            }
        }
        domains.insert(
            d.reg.output.clone(),
            CheckedDomain {
                regulator: d.reg.reference.clone(),
                part: d.reg.part.clone(),
                vout_min_microvolts: d.vout_uv.0,
                vout_max_microvolts: d.vout_uv.1,
                source: d.source.clone(),
                ret: d.ret.clone(),
                load_milliamps: load[&d.reg.output],
                consumers,
            },
        );
    }
    Ok(Some(Checked {
        domains,
        input_steady_milliamps: steady,
        input_fault_milliamps: fault,
        conditions: ctx.conditions.into_iter().collect(),
    }))
}
