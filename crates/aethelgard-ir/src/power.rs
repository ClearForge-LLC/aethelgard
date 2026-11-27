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
//!                     "junction": { "loss_milliwatts": { "value": 1170, "source": "..." },
//!                                   "loss_at_milliamps": 2000 } | null,
//!                     "controls": [ { "pin": "EN", "strap": "input", "wire": null },
//!                                   { "pin": "PG", "strap": null, "wire": { "net": "PG_5V", "role": "..." } } ] } ],
//!   "loads": [ { "domain": "VDD_5V", "milliamps": 2000, "source": "..." } ],
//!   "sense": [ { "net": "VBAT_SENSE", "top": {...}, "bottom": {...},
//!                "reservoir": { "reference": "C30", "part": "..." } | null, "adc_resolution_bits": 12 } ],
//!   "crossings": [ { "reference": "U13", "part": "TXU0304PW", "bus": "IMU", "a_domain": "VDD_3V3",
//!                    "b_domain": "VDD_1V8", "cs_pull_up": { "reference": "R40", "part": "...", "ohms": 100000 },
//!                    "external": [ ... ], "controls": [ ... ] } ],
//!   "drop_limits": [ { "domain": "VDD_3V3", "max_millivolts": 165, "source": "..." } ]   (optional)
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

/// The power-in connector's positive net when the input is protected
/// (`AET-WO-4300k` (10)): connector, then the fuse.
pub const BATTERY_IN_NET: &str = "VBAT_IN";

/// Between the fuse and the P-FET's drain (`AET-WO-4300k` (10)).
pub const BATTERY_FUSED_NET: &str = "VBAT_FUSED";

/// Every reference the power tree declares, in the emitter's claim order
/// (moved from `aethelgard-eda` by `AET-WO-1206`, so `validate`'s name checks
/// and the emitter read one list); each hot-plug part's after the power-in
/// connector's (`AET-WO-1315` §1.1).
pub fn references(p: &Power) -> Vec<&String> {
    let mut out = vec![&p.input.reference];
    out.extend(p.input.hot_plug.iter().map(|h| &h.reference));
    if let Some(x) = &p.input.protection {
        out.extend([
            &x.fuse.reference,
            &x.fet.reference,
            &x.zener.reference,
            &x.gate_resistor.reference,
        ]);
    }
    for r in &p.regulators {
        out.push(&r.reference);
        if let Some(d) = &r.feedback {
            out.extend([&d.top.reference, &d.bottom.reference]);
        }
        out.extend(r.external.iter().map(|e| &e.reference));
        // `AET-WO-4411` (iii): an enable's pull-up to the PG it follows
        out.extend(
            r.controls
                .iter()
                .filter_map(|c| c.after.as_ref())
                .map(|a| &a.pull_up.reference),
        );
    }
    for s in &p.sense {
        out.extend([&s.top.reference, &s.bottom.reference]);
        out.extend(s.reservoir.iter().map(|c| &c.reference));
    }
    for c in &p.crossings {
        out.push(&c.cs_pull_up.reference);
    }
    for t in p.translators() {
        out.push(t.reference);
        out.extend(t.external.iter().map(|e| &e.reference));
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
    /// Each translator on a serial port (`AET-WO-4411` (c1), ruled
    /// 2026-11-26T04:03Z). Absent or empty declares none and is not written
    /// back, by `pin_devices`'s precedent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uart_crossings: Vec<UartCrossing>,
    /// The supply and ground drop each domain allows its load, VDD and VSS
    /// together (`AET-WO-1303b`, D7), judged by `extract`'s supply gate, which
    /// needs the stackup's minima. **Required** (`AET-WO-4300k` (2), `RUL-14`
    /// as amended 2026-11-17: its absence turned the gate off), and the MCU's
    /// supply domain, the gate's one load, must be named in it
    /// (`DropLimitUndeclared`). Any other domain is named as a condition
    /// (`drop_not_judged`, ruled D13).
    pub drop_limits: Vec<DropLimit>,
}

/// One domain's drop limit: VDD plus VSS, mV, at most, with its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct DropLimit {
    pub domain: String,
    pub max_millivolts: u32,
    pub source: String,
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
    /// The battery input's protection (`AET-WO-4300k` (10)): a P-FET against
    /// reverse polarity, its gate held by a resistor and a zener, and a
    /// fast-acting fuse, between the connector and `VBAT`. Required as a key;
    /// `null` is refused by name (`BatteryInputUnprotected`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub protection: Option<Protection>,
}

/// The battery input's protection parts (`AET-WO-4300k` (10)), in the order
/// the current meets them: the fuse, then the P-FET (drain to the fuse,
/// source to `VBAT`), its gate pulled to `GND` by `gate_resistor` and held
/// within its rating by `zener` (cathode on the source).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Protection {
    pub fuse: Placed,
    pub fet: Placed,
    pub zener: Placed,
    pub gate_resistor: Resistor,
    /// Why these parts, as the ruling chose them (ruled Q1: the fuse's
    /// rating set by the hot-plug's I²t, not by twice the input current).
    pub source: String,
}

/// A part on the board by its reference and its row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Placed {
    pub reference: String,
    pub part: String,
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
    /// The healthy path's series resistance, mΩ: the pack, its leads, the
    /// input connector and the `VBAT` copper (`AET-WO-2304` D3), with its
    /// source; `null` is undeclared, and a battery sag then halts by name
    /// (`SeriesResistanceUndeclared`). `validate` judges nothing else with it
    /// (`2304b` K6).
    #[serde(deserialize_with = "Option::deserialize")]
    pub series_milliohms: Option<SourcedValue>,
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
    pub loss_milliwatts: SourcedValue,
    /// The output current the loss's source states it at, mA: a gap to the
    /// solved load is printed (`AET-WO-4300k` A13). The copper the screen
    /// reads is measured on the drawn board (`extract.thermal`, ruled (a)),
    /// never declared.
    pub loss_at_milliamps: u32,
}

/// A control pin and what it is tied to: a strap its row allows, a wire to
/// a named net with its stated role, or (an enable only) another
/// regulator's power-good. `strap` and `wire` are required keys; `after`
/// is optional; exactly one of the three is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Control {
    pub pin: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub strap: Option<pp::Strap>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub wire: Option<Wire>,
    /// `AET-WO-4411` (iii), ruled 2026-11-25T21:17Z: this `EN` follows the
    /// named regulator's open-drain `PG`, pulled up to that regulator's
    /// output, so this regulator starts only once that output is good. The
    /// named regulator's `PG` is declared here, never in its own `controls`.
    #[serde(
        default,
        deserialize_with = "crate::board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub after: Option<After>,
}

/// An enable held by another regulator's power-good (`AET-WO-4411` (iii)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct After {
    pub regulator: String,
    pub pull_up: Resistor,
    pub source: String,
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
    /// The part this load is (`AET-WO-1303b`, the load ruling): optional; a
    /// domain `extract`'s supply gate judges needs one entry naming each
    /// part on it that can draw current, other than the MCU and the child
    /// regulators, which their part rows count.
    #[serde(
        default,
        deserialize_with = "crate::board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub reference: Option<String>,
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

/// A translator on a declared serial port (`AET-WO-4411` (c1)): side A on
/// the MCU's supply carries the port's TX (A to B) and RX (B to A) to the
/// port's radio module on side B, so each line's driver and receiver share a
/// rail. Every input it does not carry is tied to `GND`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct UartCrossing {
    pub reference: String,
    pub part: String,
    pub serial_port: String,
    pub a_domain: String,
    pub b_domain: String,
    pub external: Vec<External>,
    pub controls: Vec<Control>,
}

/// A translator a power tree declares, on an SPI bus or a serial port: what
/// the steps that judge any translator read.
pub struct TranslatorUse<'a> {
    pub reference: &'a String,
    pub part: &'a String,
    pub a_domain: &'a String,
    pub b_domain: &'a String,
    pub external: &'a [External],
    pub controls: &'a [Control],
    /// On a serial port, not an SPI bus.
    pub uart: bool,
}

impl Power {
    /// Every declared translator, the SPI crossings first.
    pub fn translators(&self) -> impl Iterator<Item = TranslatorUse<'_>> {
        self.crossings
            .iter()
            .map(|c| TranslatorUse {
                reference: &c.reference,
                part: &c.part,
                a_domain: &c.a_domain,
                b_domain: &c.b_domain,
                external: &c.external,
                controls: &c.controls,
                uart: false,
            })
            .chain(self.uart_crossings.iter().map(|c| TranslatorUse {
                reference: &c.reference,
                part: &c.part,
                a_domain: &c.a_domain,
                b_domain: &c.b_domain,
                external: &c.external,
                controls: &c.controls,
                uart: true,
            }))
    }
}

crate::object_only!(
    Power,
    Conditions,
    Temperature,
    PowerIn,
    Protection,
    Placed,
    Battery,
    Cells,
    SourcedValue,
    Regulator,
    Divider,
    Resistor,
    External,
    Load,
    DropLimit,
    Sense,
    Reservoir,
    Crossing,
    UartCrossing,
    Derating,
    HotPlug,
    Junction,
    Control,
    After,
    Wire
);

/// The names a power refusal carries, as the CLI's error envelope shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A part name the tables do not hold, or a row of the wrong kind.
    PowerPartUnknown,
    /// A battery-fed regulator the chain cannot model: not a buck, no
    /// declared loss, or no stated regulation input (`AET-WO-2304b` §1.5).
    BatterySideUnmodelled,
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
    /// `power` declared with no drop limit on the MCU's supply domain
    /// (`AET-WO-4300k` (2)).
    DropLimitUndeclared,
    /// A regulated domain nothing loads (`AET-WO-4300k` A13).
    RegulatedDomainUnloaded,
    /// An `after` control that names no regulator, itself, a regulator
    /// whose row has no open-drain `PG`, or sits on a pin that is no enable,
    /// or a loop of them (`AET-WO-4411` (iii)).
    ControlAfterInvalid,
    /// A radio module on its own rail while the MCU's regulator does not
    /// start after it (`AET-WO-4411` (iii), ruled 2026-11-25T21:17Z).
    RadioRailUnsequenced,
    /// A UART line's driver level outside its receiver's thresholds at the
    /// receiver's pin, directly or through a translator (`AET-WO-4411` (c1),
    /// ruled 2026-11-26T04:03Z).
    LineLevelUnmet,
    /// A UART crossing's OE not strapped to VCCA (`AET-WO-4411` (c1), ruled
    /// 2026-11-26T04:51Z and 08:59Z R1).
    CrossingOeUnheld,
    /// A module rail's guaranteed minimum, less its drop limit, under the
    /// module's minimum supply (`AET-WO-4411`, ruled 2026-11-25T20:10Z).
    RadioSupplyBelowMinimum,
    /// With no clamp on the battery input, twice the pack's maximum above a
    /// battery-fed regulator's VIN absolute maximum (`AET-WO-4300k` (6)).
    DoubledPackOverAbsoluteMaximum,
    /// A battery input with no protection declared (`AET-WO-4300k` (10)).
    BatteryInputUnprotected,
    /// A protection part outside what its row allows on this battery
    /// (`AET-WO-4300k` (10)).
    ProtectionPartOutOfRange,
    /// A fuse whose melting I²t is under the hot-plug's bound (`AET-WO-4300k`
    /// (10)).
    FuseMeltsOnHotPlug,
    /// A fuse on a branch that carries motor current (`AET-WO-4300k` (10)).
    FuseInMotorPath,
    /// A motor output with no ESC declared on a protected input: whether its
    /// current crosses the fuse is unknown (`AET-WO-4300k`, the power
    /// review's F1; `INV-06`). The battery sag's name for the same gap.
    EscUndeclared,
    /// A buck's inductor peak at or above its high-side limit's minimum
    /// (`AET-WO-4300k` A13).
    PeakCurrentOverLimit,
    /// A reset supervisor that falls under a supply-bounded device's VCC
    /// minimum before it resets (`AET-WO-4300k` (9)).
    SupervisorBelowDeviceFloor,
    /// A reset supervisor that releases at or above its rail's minimum
    /// (`AET-WO-4300k` (9)).
    SupervisorAboveRail,
    /// A reset supervisor, its supply or its pull-up outside its row
    /// (`AET-WO-4300k` (9)).
    SupervisorOutOfRange,
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
            Self::BatterySideUnmodelled => "BatterySideUnmodelled",
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
            Self::DropLimitUndeclared => "DropLimitUndeclared",
            Self::RegulatedDomainUnloaded => "RegulatedDomainUnloaded",
            Self::ControlAfterInvalid => "ControlAfterInvalid",
            Self::RadioRailUnsequenced => "RadioRailUnsequenced",
            Self::LineLevelUnmet => "LineLevelUnmet",
            Self::CrossingOeUnheld => "CrossingOeUnheld",
            Self::RadioSupplyBelowMinimum => "RadioSupplyBelowMinimum",
            Self::DoubledPackOverAbsoluteMaximum => "DoubledPackOverAbsoluteMaximum",
            Self::BatteryInputUnprotected => "BatteryInputUnprotected",
            Self::ProtectionPartOutOfRange => "ProtectionPartOutOfRange",
            Self::FuseMeltsOnHotPlug => "FuseMeltsOnHotPlug",
            Self::FuseInMotorPath => "FuseInMotorPath",
            Self::EscUndeclared => "EscUndeclared",
            Self::PeakCurrentOverLimit => "PeakCurrentOverLimit",
            Self::SupervisorBelowDeviceFloor => "SupervisorBelowDeviceFloor",
            Self::SupervisorAboveRail => "SupervisorAboveRail",
            Self::SupervisorOutOfRange => "SupervisorOutOfRange",
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
    /// The conditions a fabrication package needs acknowledged
    /// (`AET-WO-4300k` (1), (d)): screened, typical, not computed, declared
    /// or not production tested, each by its stable id `<kind>:<reference>`,
    /// with its text. A method statement is a condition with no id.
    pub screened: BTreeMap<String, String>,
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
pub(crate) fn resistor_error_ppm(r: &pp::Resistor, t: &Temperature) -> i128 {
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
    screened: BTreeMap<String, String>,
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

    /// A condition a package needs acknowledged by its id
    /// `<kind>:<reference>` (`AET-WO-4300k` (d)); its text is a condition as
    /// [`Ctx::note`] records one.
    fn screen(&mut self, kind: &str, reference: &str, condition: impl Into<String>) {
        let text = condition.into();
        self.screened
            .insert(format!("{kind}:{reference}"), text.clone());
        self.conditions.insert(text);
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
pub(crate) fn range(d: &VoltageDomain) -> (u64, u64) {
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
    if let Some(r) = &b.series_milliohms {
        crate::board::source(&r.source, "power.battery.series_milliohms.source")?;
        if r.value == 0 {
            return Err(invalid(
                "power.battery.series_milliohms",
                "a series resistance of 0 mΩ is not a path".to_owned(),
            ));
        }
    }
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
    for (i, h) in power.input.hot_plug.iter().enumerate() {
        id(&h.reference, format!("power.input.hot_plug[{i}].reference"))?;
        id(&h.part, format!("power.input.hot_plug[{i}].part"))?;
    }
    if let Some(x) = &power.input.protection {
        for (what, part) in [("fuse", &x.fuse), ("fet", &x.fet), ("zener", &x.zener)] {
            id(
                &part.reference,
                format!("power.input.protection.{what}.reference"),
            )?;
            id(&part.part, format!("power.input.protection.{what}.part"))?;
        }
        id(
            &x.gate_resistor.reference,
            "power.input.protection.gate_resistor.reference".to_owned(),
        )?;
        id(
            &x.gate_resistor.part,
            "power.input.protection.gate_resistor.part".to_owned(),
        )?;
        crate::board::source(&x.source, "power.input.protection.source")?;
    }
    let resistor = |r: &Resistor, path: &str| -> Result<(), IrValidationError> {
        id(&r.reference, format!("{path}.reference"))?;
        id(&r.part, format!("{path}.part"))?;
        if r.ohms == 0 {
            return Err(invalid(&r.reference, "a resistor of 0 ohms".to_owned()));
        }
        Ok(())
    };
    let controls = |list: &[Control], path: &str| -> Result<(), IrValidationError> {
        for (i, c) in list.iter().enumerate() {
            id(&c.pin, format!("{path}[{i}].pin"))?;
            let set = [c.strap.is_some(), c.wire.is_some(), c.after.is_some()];
            if set.iter().filter(|x| **x).count() != 1 {
                return Err(invalid(
                    &format!("{path}[{i}]"),
                    "a control pin takes exactly one of `strap`, `wire` and `after`".to_owned(),
                ));
            }
            if let Some(w) = &c.wire {
                id(&w.net, format!("{path}[{i}].wire.net"))?;
                crate::board::source(&w.role, &format!("{path}[{i}].wire.role"))?;
            }
            if let Some(a) = &c.after {
                id(&a.regulator, format!("{path}[{i}].after.regulator"))?;
                resistor(&a.pull_up, &format!("{path}[{i}].after.pull_up"))?;
                crate::board::source(&a.source, &format!("{path}[{i}].after.source"))?;
            }
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
        if let Some(f) = &r.feedback {
            resistor(&f.top, &format!("{path}.feedback.top"))?;
            resistor(&f.bottom, &format!("{path}.feedback.bottom"))?;
        }
        externals(&r.external, &format!("{path}.external"))?;
        controls(&r.controls, &format!("{path}.controls"))?;
        if let Some(j) = &r.junction {
            if j.loss_at_milliamps == 0 || j.loss_at_milliamps > 100_000 {
                return Err(invalid(
                    &r.reference,
                    format!(
                        "{path}.junction.loss_at_milliamps is {}, not 1 to 100000",
                        j.loss_at_milliamps
                    ),
                ));
            }
            let v = &j.loss_milliwatts;
            crate::board::source(
                &v.source,
                &format!("{path}.junction.loss_milliwatts.source"),
            )?;
            if v.value == 0 || v.value > 1_000_000 {
                return Err(invalid(
                    &r.reference,
                    format!(
                        "{path}.junction.loss_milliwatts is {}, not 1 to 1000000",
                        v.value
                    ),
                ));
            }
        }
    }
    let mut named = BTreeSet::new();
    for (i, l) in power.loads.iter().enumerate() {
        known(&l.domain)?;
        crate::board::source(&l.source, &format!("power.loads[{i}].source"))?;
        if let Some(r) = &l.reference {
            id(r, format!("power.loads[{i}].reference"))?;
            if !named.insert((l.domain.as_str(), r.as_str())) {
                return Err(invalid(
                    &l.domain,
                    format!("power.loads[{i}] names {r} on its domain a second time"),
                ));
            }
        }
        if l.milliamps == 0 {
            return Err(invalid(
                &l.domain,
                format!("power.loads[{i}] declares 0 mA"),
            ));
        }
    }
    let mut limited = BTreeSet::new();
    for (i, l) in power.drop_limits.iter().enumerate() {
        known(&l.domain)?;
        crate::board::source(&l.source, &format!("power.drop_limits[{i}].source"))?;
        if l.max_millivolts == 0 || l.max_millivolts > 100_000 {
            return Err(invalid(
                &l.domain,
                format!(
                    "power.drop_limits[{i}] is {} mV, not 1 to 100000",
                    l.max_millivolts
                ),
            ));
        }
        if !limited.insert(l.domain.as_str()) {
            return Err(invalid(
                &l.domain,
                format!("power.drop_limits[{i}] names a domain a second time"),
            ));
        }
    }
    if !limited.contains(spec.mcu.supply.as_str()) {
        return Err(refuse(
            Refusal::DropLimitUndeclared,
            &spec.mcu.supply,
            format!(
                "power.drop_limits names no limit for `{}`, the MCU's supply: extract's supply gate judges the MCU's pins, and with no limit it judges nothing",
                spec.mcu.supply
            ),
            "AET-WO-4300k (2): drop_limits is required wherever power is declared (RUL-14, amended 2026-11-17)",
        ));
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
        if let Some(c) = &s.reservoir {
            id(&c.reference, format!("{path}.reservoir.reference"))?;
            id(&c.part, format!("{path}.reservoir.part"))?;
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
        // `after` is a regulator's enable following another's PG
        if c.controls.iter().any(|x| x.after.is_some()) {
            return Err(invalid(
                &format!("{path}.controls"),
                "a crossing's control takes a strap or a wire; `after` is a regulator's enable"
                    .to_owned(),
            ));
        }
    }
    for (i, c) in power.uart_crossings.iter().enumerate() {
        let path = format!("power.uart_crossings[{i}]");
        id(&c.reference, format!("{path}.reference"))?;
        id(&c.part, format!("{path}.part"))?;
        id(&c.serial_port, format!("{path}.serial_port"))?;
        known(&c.a_domain)?;
        known(&c.b_domain)?;
        externals(&c.external, &format!("{path}.external"))?;
        controls(&c.controls, &format!("{path}.controls"))?;
        if c.controls.iter().any(|x| x.after.is_some()) {
            return Err(invalid(
                &format!("{path}.controls"),
                "a crossing's control takes a strap or a wire; `after` is a regulator's enable"
                    .to_owned(),
            ));
        }
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
            .chain(references(power).into_iter().map(String::as_str)),
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
                    ctx.screen(
"regulation_not_production_tested",
&reg.reference,
format!(
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
        out.push(mcu_consumer(ctx.table, spec, row, m)?);
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
    for c in ctx.power.translators() {
        let row = ctx.row(c.reference, c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(c.reference, c.part, "translator"));
        };
        let s = supplies(ctx, t, c.a_domain, c.b_domain, c.uart);
        for (side, d, v) in [
            ("VCCA", c.a_domain, &s.vcca_millivolts),
            ("VCCB", c.b_domain, &s.vccb_millivolts),
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

/// The MCU as a consumer of its supply: its row's operating range (DS12117
/// Table 111) and its floor, the larger of that range's minimum and the
/// declared `bor_level`'s VBOR falling maximum (Table 115). One rule for
/// step 16 and for step 22 without a power tree (`AET-WO-1208` F1/F5, the
/// architect's ruling on PR #182).
fn mcu_consumer(
    table: &PowerPartsTable,
    spec: &Specification,
    row: &pp::Row,
    m: &pp::Mcu,
) -> Result<Consumer, IrValidationError> {
    let level = usize::from(spec.mcu.bor_level);
    let bor = m.bor_falling_max_millivolts.get(level).ok_or_else(|| {
        refuse(
            Refusal::PowerValueInvalid,
            "mcu",
            format!("mcu.bor_level {level} has no row"),
            table.cite(&m.vdd_millivolts.at),
        )
    })?;
    let floor = m.vdd_millivolts.range.min.max(bor.value);
    Ok(Consumer {
        name: format!("{} (MCU)", row.part),
        range: (m.vdd_millivolts.range.min, m.vdd_millivolts.range.max),
        floor,
        solved_floor: floor,
        cite: format!(
            "{}; VBOR{level} falling max {}",
            table.cite(&m.vdd_millivolts.at),
            table.cite(&bor.at)
        ),
    })
}

/// Step 22 of `validate` (`AET-WO-1208` F1/F5): with no power tree, the
/// MCU's supply domain judged by step 16's own consumer rule
/// ([`domain_inside_consumer`], [`mcu_consumer`]), so one rule and one floor
/// hold the MCU's rail whether or not a tree declares its source. With a
/// tree, step 16 has judged it. A part with no MCU row is the emitter's to
/// refuse, as before.
pub(crate) fn mcu_without_tree(spec: &Specification) -> Result<(), IrValidationError> {
    if spec.power.is_some() {
        return Ok(());
    }
    let table = crate::power_parts::table()?;
    let (Some(row), Some(domain)) = (
        table.get(&spec.mcu.part),
        spec.voltage_domains
            .iter()
            .find(|d| d.id == spec.mcu.supply),
    ) else {
        return Ok(());
    };
    let Kind::Mcu(m) = &row.kind else {
        return Ok(());
    };
    domain_inside_consumer(domain, &mcu_consumer(table, spec, row, m)?)
}

/// Step 3a (`AET-WO-4411`, ruled 2026-11-25T20:10Z and 21:17Z): a radio
/// module on a rail of its own, not the MCU's. The MCU's regulator starts
/// after the module's (`after`), and the module rail's lowest, less its
/// declared drop limit, stays at the module's minimum supply. Then every
/// module's UART lines, on its own rail or the MCU's, directly or through a
/// UART crossing ([`radio_lines`]).
fn radio_rails(ctx: &mut Ctx<'_>, derived: &[Derived<'_>]) -> Result<(), IrValidationError> {
    let spec = ctx.spec;
    let mcu_supply = &spec.mcu.supply;
    uart_crossings_shape(ctx)?;
    for m in spec.declared_radio_modules() {
        // `radio::validate` refuses an unknown part and an unsourced rail
        let t = crate::radio::table()?;
        let Some(row) = t.get(&m.part) else {
            continue;
        };
        // a rail outside the module's range is `radio::validate`'s to refuse
        let (lo, hi) = ctx.domain(&m.supply).map_or((0, 0), range);
        if lo < u64::from(row.supply.min_mv) || hi > u64::from(row.supply.max_mv) {
            continue;
        }
        let crossing = ctx
            .power
            .uart_crossings
            .iter()
            .find(|c| c.serial_port == m.serial_port);
        if &m.supply != mcu_supply {
            let (Some(lead), Some(mcu)) = (
                derived.iter().find(|d| d.reg.output == m.supply),
                derived.iter().find(|d| &d.reg.output == mcu_supply),
            ) else {
                continue;
            };
            // the module is its rail's load, and the gate judges its pins there
            let Some(limit) = ctx.power.drop_limits.iter().find(|l| l.domain == m.supply) else {
                return Err(refuse(
                    Refusal::DropLimitUndeclared,
                    &m.supply,
                    format!(
                        "power.drop_limits names no limit for `{}`, {}'s own rail: its minimum supply is judged on that rail less its drop",
                        m.supply, m.reference
                    ),
                    "AET-WO-4411, ruled 2026-11-25T20:10Z: the module's minimum judged on its own rail, with its own drop gate",
                ));
            };
            let vih = &row.uart.vih_over_supply_mv;
            let sequenced = mcu.reg.controls.iter().any(|c| {
                c.after
                    .as_ref()
                    .is_some_and(|a| a.regulator == lead.reg.reference)
            });
            if !sequenced {
                let why = match crossing {
                    Some(c) => format!(
                        "the module's rail leads the MCU's (ruled 2026-11-25T21:17Z (iii)); {} holds the UART lines apart while either is down",
                        c.reference
                    ),
                    None => format!(
                        "at power-up the MCU's UART can drive the module's RXD while its rail is down, above its VIH maximum of VDD + {} mV",
                        vih.value
                    ),
                };
                return Err(refuse(
                    Refusal::RadioRailUnsequenced,
                    &m.reference,
                    format!(
                        "{} is on `{}` and the MCU on `{mcu_supply}`, and {}'s EN does not follow {}'s PG (`after`): {why}",
                        m.reference, m.supply, mcu.reg.reference, lead.reg.reference
                    ),
                    t.cite(&vih.at),
                ));
            }
            let limit = u64::from(limit.max_millivolts) * 1000;
            let floor = lead.vout_uv.0.saturating_sub(limit);
            let min = u64::from(row.supply.min_mv) * 1000;
            if floor < min {
                return Err(refuse(
                    Refusal::RadioSupplyBelowMinimum,
                    &m.reference,
                    format!(
                        "`{}`'s guaranteed {} uV at {}, less its {} uV drop limit, is {floor} uV at {}'s pins, under its minimum of {min} uV",
                        m.supply, lead.vout_uv.0, lead.reg.reference, limit, m.reference
                    ),
                    t.cite(&row.supply.at),
                ));
            }
            // a UART crossing's Ioff holds the lines apart while either rail
            // is down (ruled 2026-11-26T04:03Z: it closes these two rows)
            if crossing.is_none() {
                ctx.screen(
                "radio_power_down_order",
                &m.reference,
                format!(
                    "{}: `{}` and `{mcu_supply}` are not ordered at power-down{}: while the module's rail is down, the MCU's TX may drive its RXD above VDD + {} mV ({}); the module's TX onto the MCU while `{mcu_supply}` is down is step 25's (`radio_module_tx`, at VDD = 0)",
                    m.reference,
                    m.supply,
                    {
                        // off-board ends that can power the MCU's rail ahead of
                        // the module's (adversarial F3)
                        let ends: Vec<&str> = spec
                            .declared_connectors()
                            .iter()
                            .filter(|c| c.pins.iter().any(|p| p.net.as_deref() == Some(mcu_supply.as_str())))
                            .map(|c| c.reference.as_str())
                            .collect();
                        if ends.is_empty() {
                            String::new()
                        } else {
                            format!(", and {} carry `{mcu_supply}` from off the board, which can power it while the module's rail is down", ends.join(" and "))
                        }
                    },
                    vih.value,
                    t.cite(&vih.at)
                ),
            );
                ctx.screen(
                "radio_rail_ioff",
                &m.reference,
                format!(
                    "{}: no buffer with powered-off isolation (Ioff) sits on the MCU's TX to its RXD; one would hold the line whenever `{}` is down (ruled 2026-11-25T21:17Z: a row)",
                    m.reference, m.supply
                ),
            );
            }
            ctx.screen(
                "radio_rail_monitor",
                &m.reference,
                format!(
                    "{}: no monitor holds its EN on `{}`: neither the module's datasheet nor the ESP32-C3 hardware design guidelines state a brown-out detector, and the guidelines recommend a power monitor 'around 3.0 V' for slow or unstable supplies ({})",
                    m.reference,
                    m.supply,
                    t.cite(&crate::radio::At {
                        document: "ESP32-C3-HDG".into(),
                        section: "PDF p. 10 (printed 7), Table 1's attention box: reserve a power monitor chip, its threshold around 3.0 V".into(),
                    })
                ),
            );
        }
        radio_lines(ctx, derived, m, row, t, crossing)?;
    }
    Ok(())
}

/// Each UART crossing carries a declared serial port with a radio module
/// on it, one crossing a port, joins the MCU's supply (side A) to the
/// module's (side B), and its part is a translator with an A-to-B and a
/// B-to-A channel.
fn uart_crossings_shape(ctx: &Ctx<'_>) -> Result<(), IrValidationError> {
    let spec = ctx.spec;
    for (i, c) in ctx.power.uart_crossings.iter().enumerate() {
        let row = ctx.row(&c.reference, &c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(&c.reference, &c.part, "translator"));
        };
        let s = supplies(ctx, t, &c.a_domain, &c.b_domain, true);
        let invalid = |detail: String| {
            refuse(
                Refusal::CrossingInvalid,
                &c.reference,
                detail,
                ctx.cite(&s.vcca_millivolts.at),
            )
        };
        let Some(m) = spec
            .declared_radio_modules()
            .iter()
            .find(|m| m.serial_port == c.serial_port)
        else {
            return Err(invalid(format!(
                "serial port `{}` carries no declared radio module, the one peer a UART crossing judges",
                c.serial_port
            )));
        };
        if ctx.power.uart_crossings[..i]
            .iter()
            .any(|o| o.serial_port == c.serial_port)
        {
            return Err(invalid(format!(
                "serial port `{}` has more than one crossing",
                c.serial_port
            )));
        }
        if t.uart_channels().is_none() {
            return Err(invalid(format!(
                "{} has no A-to-B channel for the MCU's TX and B-to-A channel for its RX",
                c.part
            )));
        }
        // SCES935A 9.1 (ruled 04:51Z, 08:59Z R1): OE on VCCA, the rail (iii)
        // raises last, holds the outputs high-impedance through power-up; a
        // wire's driver and level through partial power-down are not judged
        if !c
            .controls
            .iter()
            .any(|x| x.pin == "OE" && x.strap == Some(pp::Strap::Vcca))
        {
            return Err(refuse(
                Refusal::CrossingOeUnheld,
                &c.reference,
                format!(
                    "{}'s OE is not strapped to VCCA (`{}`), the rail that rises last: SCES935A 9.1 holds the outputs high-impedance through power-up with OE low",
                    c.reference, c.a_domain
                ),
                t.controls
                    .iter()
                    .find(|r| r.pin == "OE")
                    .map_or_else(String::new, |r| ctx.cite(&r.at)),
            ));
        }
        if c.a_domain != spec.mcu.supply || c.b_domain != m.supply {
            return Err(refuse(
                Refusal::CrossingDomainMismatch,
                &c.reference,
                format!(
                    "the crossing joins `{}` to `{}`, and port `{}` runs from the MCU on `{}` to {} on `{}`",
                    c.a_domain, c.b_domain, c.serial_port, spec.mcu.supply, m.reference, m.supply
                ),
                ctx.cite(&s.vcca_millivolts.at),
            ));
        }
        if let Some(port) = spec.serial_ports.iter().find(|p| p.label == c.serial_port) {
            let rate = i64::from(port.framing.baud);
            for (dir, tmax) in [
                ("A to B", &s.tmax_a_to_b_bps),
                ("B to A", &s.tmax_b_to_a_bps),
            ] {
                if rate > tmax.value {
                    return Err(refuse(
                        Refusal::CrossingTooFast,
                        &c.reference,
                        format!(
                            "port `{}` at {rate} baud needs {rate} bit/s {dir}, above {} bit/s",
                            port.label, tmax.value
                        ),
                        ctx.cite(&tmax.at),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// One pin's supply for [`crate::levels`]: its domain's regulator's band
/// (the domain's declared range where no regulator outputs it) and its drop
/// limit; `None` without a limit, which steps 3a and 22 refuse by name.
fn line_supply<'s>(
    ctx: &Ctx<'_>,
    derived: &[Derived<'_>],
    domain: &'s str,
) -> Option<crate::levels::Supply<'s>> {
    let wide = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
    let band_uv = derived
        .iter()
        .find(|d| d.reg.output == domain)
        .map(|d| (wide(d.vout_uv.0), wide(d.vout_uv.1)))
        .or_else(|| {
            ctx.domain(domain).map(|d| {
                let (lo, hi) = range(d);
                (wide(lo) * 1000, wide(hi) * 1000)
            })
        })?;
    let drop = ctx.power.drop_limits.iter().find(|l| l.domain == domain)?;
    Some(crate::levels::Supply {
        domain,
        band_uv,
        drop_uv: i64::from(drop.max_millivolts) * 1000,
    })
}

/// Step 3a's lines (`AET-WO-4411` (c1), ruled 2026-11-26T04:03Z): the
/// port's TX and RX, each driver's VOH and VOL at its pin against the
/// receiver's VIH and VIL at its own ([`crate::levels::judge`]). Directly,
/// the MCU and the module; through a UART crossing, each to the
/// translator's side on its own rail.
fn radio_lines(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    m: &crate::radio::RadioModule,
    row: &crate::radio::RadioPart,
    t: &crate::radio::RadioTable,
    crossing: Option<&UartCrossing>,
) -> Result<(), IrValidationError> {
    use crate::levels::{Driver, Linear, Receiver, judge};
    let spec = ctx.spec;
    let lin = |permille: i64, offset_mv: i64| Linear {
        permille,
        offset_uv: offset_mv * 1000,
    };
    let Some(port) = spec.serial_ports.iter().find(|p| p.label == m.serial_port) else {
        return Ok(());
    };
    let (Some(ms), Some(rs)) = (
        line_supply(ctx, derived, &spec.mcu.supply),
        line_supply(ctx, derived, &m.supply),
    ) else {
        return Ok(());
    };
    let (_, mc) = mcu(ctx)?;
    let (o, b) = (&mc.io.output, &mc.io.motor_pull_bound);
    let at = |pin: &Option<String>| pin.as_deref().unwrap_or("its pin").to_owned();
    let mcu_drv = Driver {
        who: format!("the MCU's TX ({})", at(&port.tx.pin)),
        supply: ms,
        high_min: lin(1000, -o.high_below_supply_millivolts.value),
        low_max: lin(0, o.low_max_millivolts.value),
        load_ua: Some(o.load_microamps.value),
    };
    // DS12117 Table 10 holds its inputs' maximum, step 25 at VDD = 0
    let mcu_rcv = Receiver {
        who: format!("the MCU's RX ({})", at(&port.rx.pin)),
        supply: ms,
        high_min: lin(b.input_high_min_permille.value, 0),
        low_max: lin(b.input_low_max_permille.value, 0),
        high_max: None,
        current_na: b.leakage_max_nanoamps.value,
    };
    let u = &row.uart;
    let module_drv = Driver {
        who: format!("{}'s TXD", m.reference),
        supply: rs,
        high_min: lin(i64::from(u.voh_min_permille.value), 0),
        low_max: lin(i64::from(u.vol_max_permille.value), 0),
        load_ua: None,
    };
    let module_rcv = Receiver {
        who: format!("{}'s RXD", m.reference),
        supply: rs,
        high_min: lin(i64::from(u.vih_min_permille.value), 0),
        low_max: lin(i64::from(u.vil_max_permille.value), 0),
        high_max: Some(lin(1000, i64::from(u.vih_over_supply_mv.value))),
        current_na: i64::from(row.input_leakage_na.value),
    };
    let mcu_cite = ctx.cite(&o.high_below_supply_millivolts.at);
    let module_cite = t.cite(&u.vih_min_permille.at);
    let mut lines: Vec<(String, Driver<'_>, Receiver<'_>, String)> = Vec::new();
    // The MCU side's lines at the reset's release floor (`AET-WO-4310` (2)).
    let mut release: Vec<(String, Driver<'_>, Receiver<'_>, String, bool)> = Vec::new();
    let mut release_floor: Option<(u32, String, Option<String>)> = None;
    let mut subject = m.reference.as_str();
    match crossing {
        None => {
            lines.push((
                port.tx.net.clone(),
                mcu_drv,
                module_rcv,
                module_cite.clone(),
            ));
            lines.push((port.rx.net.clone(), module_drv, mcu_rcv, mcu_cite));
        }
        Some(c) => {
            subject = &c.reference;
            let tr = ctx.row(&c.reference, &c.part)?;
            // `uart_crossings_shape` refused any other part
            let Kind::Translator(tt) = &tr.kind else {
                return Err(ctx.kind_err(&c.reference, &c.part, "translator"));
            };
            let Some([tx, rx]) = tt.uart_channels() else {
                return Err(ctx.kind_err(&c.reference, &c.part, "translator with two channels"));
            };
            let l = &tt.levels;
            let sr = supplies(ctx, tt, &c.a_domain, &c.b_domain, true);
            let txu_cite = ctx.cite(&l.input_high_min_permille.at);
            for s in [&ms, &rs] {
                let (lo, hi) = (s.band_uv.0 - s.drop_uv, s.band_uv.1);
                if lo < l.supply_millivolts.range.min * 1000
                    || hi > l.supply_millivolts.range.max * 1000
                {
                    return Err(refuse(
                        Refusal::LineLevelUnmet,
                        &c.reference,
                        format!(
                            "`{}` reaches {lo} to {hi} uV at {}'s pins, outside the {} to {} mV its levels are stated over",
                            s.domain,
                            c.reference,
                            l.supply_millivolts.range.min,
                            l.supply_millivolts.range.max
                        ),
                        ctx.cite(&l.supply_millivolts.at),
                    ));
                }
            }
            // `AET-WO-4310` (2), ruled 2 (a): the MCU's side is judged again at
            // the reset's release floor, where the MCU still runs: its rail's
            // source as low as the reset's falling minimum, less the rail's
            // drop to each pin. Its thresholds are the stated band's where
            // they hold, else the lower band's (the worse of the two rows
            // either side, an assumption named), else refused by name.
            let (reset_mv, reset_why) = crate::supervisor::reset_floor_against(spec, ctx.table)?;
            let release_uv = i64::from(reset_mv) * 1000;
            let release_lo = release_uv - ms.drop_uv;
            let (release_high, release_low, assumed) = if release_lo
                >= l.supply_millivolts.range.min * 1000
            {
                (
                    l.input_high_min_permille.value,
                    l.input_low_max_permille.value,
                    None,
                )
            } else if let Some(b) = l
                .lower_band
                .as_ref()
                .filter(|b| release_lo >= b.supply_millivolts.range.min * 1000)
            {
                (
                    b.input_high_min_permille.value,
                    b.input_low_max_permille.value,
                    Some(b),
                )
            } else {
                let lowest = l
                    .lower_band
                    .as_ref()
                    .map_or(l.supply_millivolts.range.min, |b| {
                        b.supply_millivolts.range.min
                    });
                return Err(refuse(
                    Refusal::LineLevelUnmet,
                    &c.reference,
                    format!(
                        "at the reset's release floor, {release_uv} uV ({reset_why}), `{}` reaches {release_lo} uV at {}'s pins, under the lowest supply its input thresholds are stated at, {lowest} mV: nothing is stated below it (AET-WO-4310 (2))",
                        ms.domain, c.reference
                    ),
                    ctx.cite(&l.supply_millivolts.at),
                ));
            };
            let ms_release = crate::levels::Supply {
                domain: ms.domain,
                band_uv: (release_uv, ms.band_uv.1),
                drop_uv: ms.drop_uv,
            };
            let drv = |pin: &str, supply| Driver {
                who: format!("{}'s {pin}", c.reference),
                supply,
                high_min: lin(1000, -l.output_high_below_supply_millivolts.value),
                low_max: lin(0, l.output_low_max_millivolts.value),
                load_ua: Some(l.output_load_max_microamps.value),
            };
            let rcv = |pin: &str, supply| Receiver {
                who: format!("{}'s {pin}", c.reference),
                supply,
                high_min: lin(l.input_high_min_permille.value, 0),
                low_max: lin(l.input_low_max_permille.value, 0),
                high_max: Some(lin(0, l.input_max_millivolts.value)),
                current_na: l.input_current_max_nanoamps.value,
            };
            let release_rcv = Receiver {
                high_min: lin(release_high, 0),
                low_max: lin(release_low, 0),
                ..rcv(&tx.input, ms_release)
            };
            release.push((
                port.tx.net.clone(),
                Driver {
                    supply: ms_release,
                    ..mcu_drv.clone()
                },
                release_rcv,
                txu_cite.clone(),
                true,
            ));
            release.push((
                port.rx.net.clone(),
                drv(&rx.output, ms_release),
                Receiver {
                    supply: ms_release,
                    ..mcu_rcv.clone()
                },
                mcu_cite.clone(),
                false,
            ));
            release_floor = Some((reset_mv, reset_why, assumed.map(|b| b.assumption.clone())));
            let (b_tx, b_rx) = b_nets(&c.reference, tx, rx);
            ctx.note(format!(
                "{}: every input it does not carry is tied to `GND` ({})",
                c.reference,
                ctx.cite(&tt.unused_inputs_at)
            ));
            lines.push((
                port.tx.net.clone(),
                mcu_drv,
                rcv(&tx.input, ms),
                txu_cite.clone(),
            ));
            lines.push((b_tx.clone(), drv(&tx.output, rs), module_rcv, module_cite));
            lines.push((b_rx, module_drv, rcv(&rx.input, rs), txu_cite));
            lines.push((port.rx.net.clone(), drv(&rx.output, ms), mcu_rcv, mcu_cite));
            ctx.note(format!(
                "{}: `radio_power_down_order` and `radio_rail_ioff` are closed by {}: each of its outputs is powered from its receiver's own rail, and its Ioff rows hold either side at 0 V (SCES935A p. 8, 7.5; 9.3.4)",
                m.reference, c.reference
            ));
            ctx.screen(
                "translator_partial_power",
                &c.reference,
                format!(
                    "{}: SCES935A 9.3.5 holds its outputs high-impedance while a supply is under 100 mV with the other in its operating range (1.08 V at least, 7.3); between 100 mV and 1.08 V it states nothing; it recommends inputs low before a supply floats, and a UART idles high; while it isolates, `{}` and `{}` are driven by nothing",
                    c.reference, port.rx.net, b_tx
                ),
            );
            ctx.screen(
                "translator_threshold_inferred",
                &c.reference,
                format!(
                    "{}: VT+ and VT- are inferred, not stated: {}; ruled 2026-11-26T04:51Z G1 (a), an inference; a bench row (RUL-20 (3)): VT+ and VT- measured at the board's rails at bring-up{}",
                    c.reference,
                    ctx.cite(&l.input_high_min_permille.at),
                    assumed.map_or_else(String::new, |b| format!(
                        "; at the reset's release floor of {reset_mv} mV its A side's inputs are judged with VT+ {} and VT- {} per mille ({}), an assumption, not a datasheet figure: {}",
                        b.input_high_min_permille.value,
                        b.input_low_max_permille.value,
                        ctx.cite(&b.input_high_min_permille.at),
                        b.assumption
                    ))
                ),
            );
            ctx.screen(
                "translator_rate_inferred",
                &c.reference,
                format!(
                    "{}: port `{}`'s {} bit/s is judged against a TMAX inferred from another row, {} bit/s ({}); tpd is as stated, {} ns ({}); ruled 2026-11-26T04:51Z G2",
                    c.reference,
                    port.label,
                    port.framing.baud,
                    sr.tmax_a_to_b_bps.value,
                    ctx.cite(&sr.tmax_a_to_b_bps.at),
                    sr.tpd_a_to_b_max_ns.value,
                    ctx.cite(&sr.tpd_a_to_b_max_ns.at)
                ),
            );
            ctx.screen(
                "translator_supply_drop",
                &c.reference,
                format!(
                    "{}: its VCCA and VCCB pins are taken at `{}`'s and `{}`'s drop limits; the supply gate measures each rail at its load, and {} draws microamps (SCES935A p. 8, 7.5 ICCA and ICCB)",
                    c.reference, c.a_domain, c.b_domain, c.reference
                ),
            );
            ctx.screen(
                "uart_internal_pulls",
                &c.reference,
                format!(
                    "{}: its output levels are stated at 0.1 mA ({}); the MCU's {} is written FLOATING by the generated hwdef (its pull off), and {}'s RXD pull-up is its firmware's, not judged: on, it draws over 0.1 mA (RPU 45 kOhm typical, no minimum, WROOM-02 Table 6-3)",
                    c.reference,
                    ctx.cite(&l.output_load_max_microamps.at),
                    at(&port.rx.pin),
                    m.reference
                ),
            );
        }
    }
    for (net, d, r, cite) in &lines {
        let margins = judge(net, d, r)
            .map_err(|e| refuse(Refusal::LineLevelUnmet, subject, e.0, cite.clone()))?;
        if crossing.is_some() {
            ctx.note(format!(
                "{subject}: `{net}`, {} to {}, holds high by {} uV and low by {} uV{}",
                d.who,
                r.who,
                margins.high_uv,
                margins.low_uv,
                margins.over_uv.map_or_else(String::new, |o| format!(
                    ", and under the receiver's maximum by {o} uV"
                ))
            ));
        }
        if d.load_ua.is_none() {
            ctx.screen(
                "radio_output_load",
                &m.reference,
                format!(
                    "{}: its VOH and VOL are stated at a high-impedance load ({}); {} draws up to {} nA",
                    d.who,
                    t.cite(&u.voh_min_permille.at),
                    r.who,
                    r.current_na
                ),
            );
        }
    }
    if let Some((reset_mv, why, assumed)) = &release_floor {
        for (net, d, r, cite, banded) in &release {
            let margins = judge(net, d, r).map_err(|e| {
                refuse(
                    Refusal::LineLevelUnmet,
                    subject,
                    format!(
                        "at the reset's release floor of {reset_mv} mV ({why}): {}",
                        e.0
                    ),
                    cite.clone(),
                )
            })?;
            ctx.note(format!(
                "{subject}: `{net}`, {} to {}, at the reset's release floor of {reset_mv} mV, holds high by {} uV and low by {} uV{} (AET-WO-4310 (2))",
                d.who,
                r.who,
                margins.high_uv,
                margins.low_uv,
                assumed
                    .as_ref()
                    .filter(|_| *banded)
                    .map_or_else(String::new, |a| format!("; the receiver's thresholds are {a}"))
            ));
        }
    }
    Ok(())
}

/// The B-side nets of the UART crossing on serial port `port`
/// (`AET-WO-4411` (c1)): the one the port's TX leaves on, to the module's
/// RXD, and the one its RX arrives on, from the module's TXD. `None` with no
/// crossing on the port, or one whose part is no translator with both
/// channels, which `validate` refuses (`CrossingInvalid`).
pub fn uart_b_nets(spec: &Specification, port: &str) -> Option<(String, String)> {
    let c = spec
        .power
        .iter()
        .flat_map(|p| &p.uart_crossings)
        .find(|c| c.serial_port == port)?;
    let Kind::Translator(t) = &pp::table().ok()?.get(&c.part)?.kind else {
        return None;
    };
    let [tx, rx] = t.uart_channels()?;
    Some(b_nets(&c.reference, tx, rx))
}

/// A UART crossing's two B-side net names, from its TX and RX channels.
fn b_nets(reference: &str, tx: &pp::Channel, rx: &pp::Channel) -> (String, String) {
    (
        format!("{reference}_{}", tx.output),
        format!("{reference}_{}", rx.input),
    )
}

/// The nets a radio module's UART pads sit on, `(TXD, RXD)`: through a UART
/// crossing on `port`, its B side's ([`uart_b_nets`]); else the port's RX
/// and TX (`AET-WO-4411` (c1)).
pub fn module_uart_nets(
    spec: &Specification,
    port: &crate::serial::SerialPort,
) -> (String, String) {
    match uart_b_nets(spec, &port.label) {
        Some((tx_b, rx_b)) => (rx_b, tx_b),
        None => (port.rx.net.clone(), port.tx.net.clone()),
    }
}

/// The supply row a translator joining `a` to `b` is judged by
/// ([`pp::Translator::supplies_for`], on the domains' declared ranges).
fn supplies<'t>(
    ctx: &Ctx<'_>,
    t: &'t pp::Translator,
    a: &str,
    b: &str,
    uart: bool,
) -> &'t pp::Supplies {
    let mv = |d: &str| {
        ctx.domain(d).map_or((0, 0), |d| {
            let (lo, hi) = range(d);
            (
                i64::try_from(lo).unwrap_or(i64::MAX),
                i64::try_from(hi).unwrap_or(i64::MAX),
            )
        })
    };
    t.supplies_for(mv(a), mv(b), uart)
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
        for c in consumers(ctx, &d.id)? {
            domain_inside_consumer(d, &c)?;
        }
    }
    Ok(())
}

/// Domain `d`'s declared range inside consumer `c`'s supply range, its
/// minimum above `c`'s floor.
fn domain_inside_consumer(d: &VoltageDomain, c: &Consumer) -> Result<(), IrValidationError> {
    let (dmin, dmax) = range(d);
    let Consumer {
        name,
        range: (cmin, cmax),
        floor,
        cite,
        ..
    } = c;
    let inside = i128::from(dmin) >= i128::from(*cmin) && i128::from(dmax) <= i128::from(*cmax);
    if !inside || i128::from(dmin) <= i128::from(*floor) {
        return Err(refuse(
            Refusal::DomainOutsideConsumer,
            &d.id,
            format!(
                "the domain's {dmin} to {dmax} mV is not inside {name}'s {cmin} to {cmax} mV with its minimum above the consumer's floor of {floor} mV"
            ),
            cite.clone(),
        ));
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
            if let Some(floor) = regulation_floor(b, nominal)
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

/// The input from which a buck's output regulation band is stated, for an
/// output of `nominal` mV (D-B9): the first step whose start the output
/// reaches. One lookup for `battery_inputs` and [`battery_side`].
fn regulation_floor(b: &pp::Buck, nominal: i64) -> Option<&pp::Limit> {
    b.regulation_vin_min
        .iter()
        .find(|(from, _)| nominal >= *from)
        .map(|(_, floor)| floor)
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
            // Each radio module's stated supply requirement (`AET-WO-4411`
            // D5), counted from its row, never declared.
            let radio = crate::radio::load_ma(ctx.spec, &d.reg.output)?;
            if radio > 0 {
                sum += radio;
                ctx.note(format!(
                    "{}: {radio} mA for its radio modules, each row's stated supply requirement, not its typical transmit peak",
                    d.reg.output
                ));
            }
            if ctx.spec.mcu.supply == d.reg.output {
                let (_, m) = mcu(ctx)?;
                sum += u64::try_from(m.idd_max_milliamps.value).unwrap_or(0);
                ctx.screen(
                    "mcu_load_excludes",
                    &ctx.spec.mcu.part,
                    format!(
                        "MCU load: {} ({})",
                        m.idd_condition,
                        ctx.cite(&m.idd_max_milliamps.at)
                    ),
                );
            }
            for c in children {
                let child = load[&c.reg.output];
                sum += match &c.row.kind {
                    Kind::Ldo(l) => {
                        ctx.screen(
"ground_current_no_load",
&c.reg.reference,
format!(
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
                        ctx.screen(
"buck_bias_not_counted",
&c.reg.reference,
format!(
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
                || ctx.power.uart_crossings.iter().any(|c| {
                    (c.a_domain == d.reg.output || c.b_domain == d.reg.output)
                        && !ctx.power.loads.iter().any(|l| {
                            l.domain == d.reg.output && l.reference.as_deref() == Some(&c.reference)
                        })
                })
            {
                ctx.screen(
                    "translator_supply_not_counted",
                    &d.reg.reference,
                    format!(
                        "{}: a translator's supply current is not counted in its domain's load",
                        d.reg.output
                    ),
                );
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
/// Step 6a (`AET-WO-4300k` (10)): the battery input's protection, judged
/// like any power part, and the drop it puts in front of every battery-fed
/// regulator, mV, which [`headroom`] counts.
///
/// - The P-FET stands off twice the pack's maximum (the doubled pack, ruled
///   D7), its gate is held by the zener within its rating, the gate drive at
///   the pack's minimum reaches a stated RDS(on), and it carries the
///   branch's input current.
/// - The fuse is rated at or above the pack's maximum, at least twice the
///   branch's input current (ruled (e)), holds the input current at 85 °C
///   (its clearing table's 100 %), and its melting I²t is above the
///   hot-plug's bound, C·V²/(2·R_min): a step charging C through any series
///   RLC dissipates C·V²/2 in the loop's resistance, and the fuse's own
///   minimum is the only resistance counted (a derivation).
/// - No ESC is fed through the board's `VBAT` copper (a fuse there could cut
///   a motor in flight).
///
/// The input current is computed from the engine's own numbers at the
/// pack's minimum: each battery-fed buck's output power at its highest
/// output plus its declared loss, each battery-fed LDO's load. The tree's
/// `input_steady_milliamps` is the sum of the outputs, a bound, not this.
fn protection(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    load: &BTreeMap<String, u64>,
) -> Result<u64, IrValidationError> {
    const RULE: &str = "AET-WO-4300k (10)";
    let input = &ctx.power.input;
    let Some(x) = &input.protection else {
        return Err(refuse(
            Refusal::BatteryInputUnprotected,
            &input.reference,
            "power.input.protection is null: a reversed pack would reach every battery-fed regulator, and a short on the branch would draw the pack's current, with nothing to stop either",
            RULE,
        ));
    };
    if let Some(m) = ctx
        .spec
        .motor_outputs
        .iter()
        .find(|m| matches!(m.esc, Some(crate::motor::Esc::BoardVbat(_)) | None))
    {
        return Err(if m.esc.is_none() {
            // Undeclared is not "not through `VBAT`" (`INV-06`; the power
            // review's F1): the fuse's path is unknown, so it is refused.
            refuse(
                Refusal::EscUndeclared,
                &m.label,
                format!(
                    "motor output `{}` declares no ESC (`esc: null`), so whether its current crosses {}'s fuse is unknown: declare `none` with its source, or `board_vbat`",
                    m.label, x.fuse.reference
                ),
                RULE,
            )
        } else {
            refuse(
                Refusal::FuseInMotorPath,
                &x.fuse.reference,
                format!(
                    "motor output `{}` declares an ESC fed through the board's `VBAT` copper, so {}'s fuse carries motor current, and a fuse that opens there cuts a motor in flight",
                    m.label, x.fuse.reference
                ),
                RULE,
            )
        });
    }
    let (bmin, bmax) = battery_range(&ctx.power.battery);
    let (bmin, bmax) = (
        i64::try_from(bmin).unwrap_or(i64::MAX),
        i64::try_from(bmax).unwrap_or(i64::MAX),
    );
    let out_of_range = |reference: &str, detail: String, cite: String| {
        refuse(Refusal::ProtectionPartOutOfRange, reference, detail, cite)
    };
    // The branch's power: each battery-fed buck's output at its highest plus
    // its declared loss, µW, and each battery-fed linear regulator's load, mA.
    let mut microwatts: i128 = 0;
    let mut ldo_milliamps: i128 = 0;
    for d in derived.iter().filter(|d| d.reg.input == BATTERY) {
        let l = i128::from(load[&d.reg.output]);
        match &d.row.kind {
            Kind::Buck(_) => {
                let loss = d
                    .reg
                    .junction
                    .as_ref()
                    .map_or(0, |j| i128::from(j.loss_milliwatts.value));
                // µV × mA / 1000 = µW.
                microwatts += i128::from(d.vout_uv.1) * l / 1000 + loss * 1000;
            }
            _ => ldo_milliamps += l,
        }
    }
    // The input current at `mv` at the regulators, mA, rounded up.
    let current = |mv: i64| div_ceil(microwatts, i128::from(mv.max(1))) + ldo_milliamps;
    let fet_row = ctx.row(&x.fet.reference, &x.fet.part)?;
    let Kind::Pfet(fet) = &fet_row.kind else {
        return Err(ctx.kind_err(&x.fet.reference, &x.fet.part, "pfet"));
    };
    let zener_row = ctx.row(&x.zener.reference, &x.zener.part)?;
    let Kind::Zener(zener) = &zener_row.kind else {
        return Err(ctx.kind_err(&x.zener.reference, &x.zener.part, "zener"));
    };
    let fuse_row = ctx.row(&x.fuse.reference, &x.fuse.part)?;
    let Kind::Fuse(fuse) = &fuse_row.kind else {
        return Err(ctx.kind_err(&x.fuse.reference, &x.fuse.part, "fuse"));
    };
    let doubled = bmax.saturating_mul(2);
    if fet.vds_max_millivolts.value < doubled {
        return Err(out_of_range(
            &x.fet.reference,
            format!(
                "{} stands off {} mV, under twice the pack's maximum, {doubled} mV (the doubled pack, a derivation, ruled D7)",
                x.fet.part, fet.vds_max_millivolts.value
            ),
            ctx.cite(&fet.vds_max_millivolts.at),
        ));
    }
    // VGS at the pack's maximum: the zener clamps it at VZ, at most its max.
    let vgs_max_seen = bmax.min(zener.vz_millivolts.range.max);
    if vgs_max_seen > fet.vgs_max_millivolts.value {
        return Err(out_of_range(
            &x.fet.reference,
            format!(
                "its gate sees up to {vgs_max_seen} mV ({}'s VZ maximum {} mV, the pack's maximum {bmax} mV), above {}'s |VGS| maximum of {} mV",
                x.zener.part,
                zener.vz_millivolts.range.max,
                x.fet.part,
                fet.vgs_max_millivolts.value
            ),
            ctx.cite(&fet.vgs_max_millivolts.at),
        ));
    }
    // The fuse's resistance, µΩ: its typical less and plus its tolerance.
    let typ = i128::from(fuse.resistance_typ_microohms.value);
    let tol = i128::from(fuse.resistance_tolerance_permille.value);
    let r_min = typ * (1000 - tol) / 1000;
    let r_max = div_ceil(typ * (1000 + tol), 1000);
    // The pack's own series resistance, ahead of the board (`2304b` K6,
    // `AET-WO-4310` (3)): the pack, its leads and the connector, as the
    // specification declares it with its source. Undeclared, it is named,
    // never taken as 0 silently.
    let series = ctx.power.battery.series_milliohms.clone();
    let r_pack = series.as_ref().map_or(0, |s| i128::from(s.value) * 1000);
    if series.is_none() {
        ctx.screen(
            "headroom_pack_series_undeclared",
            &input.reference,
            format!(
                "{}: `power.battery.series_milliohms` is undeclared, so each battery-fed regulator's headroom counts no drop in the pack, its leads or the connector (AET-WO-4310 (3))",
                input.reference
            ),
        );
    }
    // mA × µΩ / 10⁶ = mV, rounded up.
    let volts = |i: i128, microohms: i128| div_ceil(i * microohms, 1_000_000);
    // The gate drive at `i`: the pack's minimum less the pack's and the fuse's
    // drop (the gate
    // returns to ground, its source sits after the fuse), or the zener's VZ
    // minimum if lower; and the stated RDS(on) at the highest |VGS| at or under
    // it, µΩ (the power review's F6 (a)).
    let rds_at = |i: i128| {
        let fuse_drop = i64::try_from(volts(i, r_pack + r_max)).unwrap_or(i64::MAX);
        let drive = bmin
            .saturating_sub(fuse_drop)
            .min(zener.vz_millivolts.range.min);
        let row = fet
            .rds_on_max
            .iter()
            .filter(|r| r.vgs_millivolts <= drive)
            .max_by_key(|r| r.vgs_millivolts);
        (drive, row)
    };
    // The input current, iterated once (the power review's F4): at the pack's
    // minimum, then at the pack's minimum less the path's drop at that
    // current, which the regulators see.
    let i0 = current(bmin);
    let (_, row0) = rds_at(i0);
    let d0 = row0.map_or(0, |r| {
        volts(
            i0,
            i128::from(r.max_milliohms.value) * 1000 + r_max + r_pack,
        )
    });
    let i_wide = current(bmin.saturating_sub(i64::try_from(d0).unwrap_or(i64::MAX)));
    let (drive, row) = rds_at(i_wide);
    let Some(rds) = row else {
        return Err(out_of_range(
            &x.fet.reference,
            format!(
                "its gate drive at the pack's minimum is {drive} mV, under every |VGS| {} states an RDS(on) at",
                x.fet.part
            ),
            ctx.cite(&fet.rds_on_max[0].max_milliohms.at),
        ));
    };
    let path = i128::from(rds.max_milliohms.value) * 1000 + r_max + r_pack;
    let drop = volts(i_wide, path);
    let i_in = i64::try_from(i_wide).unwrap_or(i64::MAX);
    ctx.note(format!(
        "{}: the branch's input current is {i_in} mA at the pack's minimum of {bmin} mV less the input path's drop, from each battery-fed buck's output at its highest plus its declared loss (one iteration); the tree's input_steady_milliamps is the sum of the outputs, a bound, not the input current",
        input.reference
    ));
    if fet.id_max_milliamps.value < i_in {
        return Err(out_of_range(
            &x.fet.reference,
            format!(
                "{} carries {} mA continuous, under the branch's input current of {i_in} mA",
                x.fet.part, fet.id_max_milliamps.value
            ),
            ctx.cite(&fet.id_max_milliamps.at),
        ));
    }
    if let Some(at) = &fet.advance_information {
        ctx.screen(
            "datasheet_advance_information",
            &x.fet.reference,
            format!(
                "{}: {}'s datasheet is advance information, so its figures may change before production ({})",
                x.fet.reference,
                x.fet.part,
                ctx.cite(at)
            ),
        );
    }
    ctx.screen(
        "zener_below_test_current",
        &x.zener.reference,
        format!(
            "{}: {}'s VZ is stated at {} µA ({}); through {} its current is far under that, where its voltage is lower, so the gate is held at or under VZ's maximum",
            x.zener.reference,
            x.zener.part,
            zener.izt_microamps.value,
            ctx.cite(&zener.vz_millivolts.at),
            x.gate_resistor.reference
        ),
    );
    if drive == zener.vz_millivolts.range.min {
        // The zener, not the pack, sets the drive, and under its test current
        // its voltage may sit under VZ's minimum (the power review's F6 (b)).
        ctx.screen(
            "gate_drive_at_zener",
            &x.fet.reference,
            format!(
                "{}: its gate drive is {}'s VZ minimum, {drive} mV, stated at {} µA; through {} the zener's voltage may be lower, so RDS(on) is read at a drive the board may not reach",
                x.fet.reference, x.zener.part, zener.izt_microamps.value, x.gate_resistor.reference
            ),
        );
    }
    ctx.resistor(&x.gate_resistor)?;
    // The fuse.
    let rated = fuse.rated_milliamps.value;
    if fuse.voltage_max_millivolts.value < bmax {
        return Err(out_of_range(
            &x.fuse.reference,
            format!(
                "{} is rated {} mV, under the pack's maximum of {bmax} mV",
                x.fuse.part, fuse.voltage_max_millivolts.value
            ),
            ctx.cite(&fuse.voltage_max_millivolts.at),
        ));
    }
    if rated < i_in.saturating_mul(2) {
        return Err(out_of_range(
            &x.fuse.reference,
            format!(
                "{} is rated {rated} mA, under twice the branch's input current of {i_in} mA (ruled (e))",
                x.fuse.part
            ),
            ctx.cite(&fuse.rated_milliamps.at),
        ));
    }
    // The row holds the fuse's derating at 85 °C only: a hotter board has no
    // held figure (the power review's F5; `INV-06`).
    let board_max = ctx.power.conditions.board_temperature_celsius.max;
    if board_max > 85 {
        return Err(out_of_range(
            &x.fuse.reference,
            format!(
                "the board reaches {board_max} °C, and {}'s row holds its current derating at 85 °C only",
                x.fuse.part
            ),
            ctx.cite(&fuse.kept_at_85c_permille.at),
        ));
    }
    let hot = rated * fuse.kept_at_85c_permille.value / 1000;
    if hot < i_in {
        return Err(out_of_range(
            &x.fuse.reference,
            format!(
                "{} keeps {hot} mA at 85 °C, under the branch's input current of {i_in} mA, so it may open in steady state",
                x.fuse.part
            ),
            ctx.cite(&fuse.kept_at_85c_permille.at),
        ));
    }
    // The hot-plug's I²t bound: C at its tolerance's top, V the pack's
    // maximum, R the fuse's minimum (typical less its tolerance).
    let mut picofarads: i128 = 0;
    for h in &input.hot_plug {
        if let Kind::BulkCapacitor(c) = &ctx.row(&h.reference, &h.part)?.kind {
            picofarads += i128::from(c.rated_picofarads.value);
            ctx.screen(
                "inrush_bulk_at_rated",
                &h.reference,
                format!(
                    "{}: its capacitance enters the hot-plug's I²t bound at its rated value; its row holds no tolerance",
                    h.reference
                ),
            );
        }
    }
    for r in ctx.power.regulators.iter().filter(|r| r.input == BATTERY) {
        for e in r
            .external
            .iter()
            .filter(|e| matches!(e.role, Role::Cin | Role::CinHf))
        {
            if let Kind::Capacitor(c) = &ctx.row(&e.reference, &e.part)?.kind {
                let rated = i128::from(c.rated_picofarads.value);
                // The row's tolerance, or its DC-bias block's, which holds the
                // same maker's figure (the power review's F7); neither held is
                // named, never assumed.
                picofarads += match (&c.tolerance, &c.dc_bias) {
                    (Some(pp::Tolerance::Permille(p)), _) => {
                        rated * (1000 + i128::from(p.value)) / 1000
                    }
                    (Some(pp::Tolerance::Femtofarads(f)), _) => {
                        rated + div_ceil(i128::from(f.value), 1000)
                    }
                    (None, Some(b)) => div_ceil(
                        rated * (1000 + i128::from(b.tolerance_permille.value.abs())),
                        1000,
                    ),
                    (None, None) => {
                        ctx.screen(
                            "inrush_ceramic_at_rated",
                            &e.reference,
                            format!(
                                "{}: its capacitance enters the hot-plug's I²t bound at its rated value; its row holds no tolerance",
                                e.reference
                            ),
                        );
                        rated
                    }
                };
            }
        }
    }
    // pF × mV² / (2 × µΩ) × 10⁻⁶ = mA²s.
    let v = i128::from(bmax);
    let i2t = div_ceil(picofarads * v * v, 2 * r_min.max(1) * 1_000_000);
    let melting = i128::from(fuse.melting_i2t_milliamp2_seconds.value);
    if i2t >= melting {
        return Err(refuse(
            Refusal::FuseMeltsOnHotPlug,
            &x.fuse.reference,
            format!(
                "hot-plugging the pack into {picofarads} pF at {bmax} mV puts up to {i2t} mA²s through {}, at or above its melting I²t of {melting} mA²s ({}): C·V²/(2·R_min), the fuse's own minimum {r_min} µΩ the only resistance counted (a derivation)",
                x.fuse.part, fuse.melting_condition
            ),
            ctx.cite(&fuse.melting_i2t_milliamp2_seconds.at),
        ));
    }
    ctx.screen(
        "fuse_hot_plug_bound_fuse_only",
        &x.fuse.reference,
        format!(
            "{}: the hot-plug's I²t is at most {i2t} mA²s against {}'s melting I²t of {melting} mA²s, which is {} ({}); the bound, a derivation, counts only the fuse's own minimum resistance, because no FET, copper, connector or pack minimum is held (ruled Q1 (a); holding one is a backlog row)",
            x.fuse.reference,
            x.fuse.part,
            fuse.melting_condition,
            ctx.cite(&fuse.melting_i2t_milliamp2_seconds.at)
        ),
    );
    // The drop the input path puts in front of the regulators: the input
    // current through the FET's RDS(on) and the fuse's maximum, each a 25 °C
    // figure (the power review's F3).
    ctx.screen(
        "input_path_resistance_at_25c",
        &x.fet.reference,
        format!(
            "{}: the input path's drop is {drop} mV at {i_in} mA through {}'s RDS(on) {} mΩ (|VGS| {} mV) and {}'s {r_max} µΩ{}, each stated at 25 °C; on a {board_max} °C board both rise, the FET's by its datasheet's RDS(on)-against-temperature figure",
            x.fet.reference,
            x.fet.part,
            rds.max_milliohms.value,
            rds.vgs_millivolts,
            x.fuse.part,
            series.as_ref().map_or_else(String::new, |s| format!(" and the pack's series {r_pack} µΩ ({})", s.source))
        ),
    );
    ctx.note(format!(
        "{}: {} mΩ ({} at |VGS| {} mV) and {} {r_max} µΩ{} at most drop {drop} mV at {i_in} mA, counted in every battery-fed regulator's headroom",
        input.reference,
        rds.max_milliohms.value,
        x.fet.part,
        rds.vgs_millivolts,
        x.fuse.part,
        series.as_ref().map_or_else(String::new, |s| format!(" and the pack's series {r_pack} µΩ ({})", s.source))
    ));
    Ok(u64::try_from(drop).unwrap_or(u64::MAX))
}

fn headroom(
    ctx: &mut Ctx<'_>,
    derived: &[Derived<'_>],
    load: &BTreeMap<String, u64>,
    input_drop_millivolts: u64,
) -> Result<(), IrValidationError> {
    for d in derived {
        let (vin_min, vin_max) = if d.reg.input == BATTERY {
            // The fuse and the P-FET sit between the pack and every
            // battery-fed regulator (`AET-WO-4300k` (10)).
            let (lo, hi) = battery_range(&ctx.power.battery);
            (lo.saturating_sub(input_drop_millivolts), hi)
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
                ctx.screen(
"inductor_dcr_at_25c",
&d.reg.reference,
format!(
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
                ctx.screen(
                    "buck_timing_conditions",
                    &d.reg.reference,
                    format!(
                        "{}: tOFF-MIN, RDS(on) and fSW are stated {} ({})",
                        d.reg.part,
                        b.stated_at,
                        ctx.cite(&b.toff_min_max_ns.at)
                    ),
                );
                ctx.note(format!(
                    "{}: the headroom counts {}'s DCR maximum ({}) and the input path's drop, {input_drop_millivolts} mV",
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
                // `AET-WO-4300k` A13: the inductor's peak, IOUT plus half the
                // ripple at VIN's maximum, under the high-side limit's minimum,
                // which ends each on-time early (SNVSAN3F p. 14, 8.3.3). The
                // ripple is Equation 4 solved for K·IOUT (p. 21, 9.2.2.4):
                // (VIN − VOUT)·VOUT / (VIN·L·fSW), at the inductor's minimum,
                // the lowest fSW and the highest output, so it is the most.
                let (vin_hi, vout_hi) = (i128::from(vin_max), i128::from(d.vout_uv.1) / 1000);
                if vin_hi > vout_hi {
                    // mV·mV/mV / (nH·Hz) × 10⁹ = mA.
                    let ripple = div_ceil(
                        (vin_hi - vout_hi) * vout_hi * 1_000_000_000,
                        vin_hi
                            * i128::from(l.nanohenries.range.min)
                            * i128::from(b.fsw_hz.range.min),
                    );
                    let peak = i + div_ceil(ripple, 2);
                    let limit = &b.high_side_limit_milliamps;
                    if peak >= i128::from(limit.range.min) {
                        return Err(refuse(
                            Refusal::PeakCurrentOverLimit,
                            &d.reg.reference,
                            format!(
                                "at {vin_max} mV its inductor peaks at {peak} mA ({i} mA out plus half of {ripple} mA ripple, at {} nH and {} Hz), at or above the high-side limit's minimum of {} mA, which would end each on-time early",
                                l.nanohenries.range.min, b.fsw_hz.range.min, limit.range.min
                            ),
                            ctx.cite(&limit.at),
                        ));
                    }
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
    ctx.screen(
"power_in_rating_underated",
&input.reference,
format!(
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
        // `AET-WO-4411` (iii): an enable's pull-up to the PG it follows
        v.extend(
            r.controls
                .iter()
                .filter_map(|c| c.after.as_ref())
                .map(|a| (a.pull_up.reference.as_str(), a.pull_up.part.as_str())),
        );
    }
    if let Some(x) = &power.input.protection {
        v.extend([
            (x.fuse.reference.as_str(), x.fuse.part.as_str()),
            (x.fet.reference.as_str(), x.fet.part.as_str()),
            (x.zener.reference.as_str(), x.zener.part.as_str()),
            (
                x.gate_resistor.reference.as_str(),
                x.gate_resistor.part.as_str(),
            ),
        ]);
    }
    for s in &power.sense {
        v.push((&s.top.reference, &s.top.part));
        v.push((&s.bottom.reference, &s.bottom.part));
        if let Some(c) = &s.reservoir {
            v.push((&c.reference, &c.part));
        }
    }
    for c in &power.crossings {
        v.push((&c.cs_pull_up.reference, &c.cs_pull_up.part));
    }
    for t in power.translators() {
        v.push((t.reference, t.part));
        v.extend(
            t.external
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
            ctx.screen(
"lifecycle_unread",
part,
format!(
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
    for c in ctx.power.translators() {
        let row = ctx.row(c.reference, c.part)?;
        let Kind::Translator(t) = &row.kind else {
            return Err(ctx.kind_err(c.reference, c.part, "translator"));
        };
        let a = ctx.domain(c.a_domain).map_or(0, |d| range(d).1);
        let b = ctx.domain(c.b_domain).map_or(0, |d| range(d).1);
        owners.push(Owner {
            reference: c.reference,
            list: c.external,
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
                        ctx.screen(
"inductor_ripple_outside_guidance",
o.reference,
format!(
                            "{}: the inductance meets Equation 5; Table 9-2's typical value is in the citation, and Equation 4's 20 to 40 % ripple and the 10 % ripple floor are not met across the whole battery range (ruled D6) ({})",
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
                        ctx.screen(
"isat_typical",
&e.reference,
format!(
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
                            ctx.screen(
"dielectric_accepted",
&e.reference,
format!(
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
                        ctx.cite(&min.at),
                    ));
                }
                (Some(min), Some(Err((kind, why)))) => {
                    return Err(refuse(
                        kind,
                        o.reference,
                        format!("`{role}`: {why}, so the screen cannot clear it"),
                        ctx.cite(&min.at),
                    ));
                }
                (Some(min), Some(Ok((eff, how, w)))) => {
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
                    ctx.screen(
"effective_typical",
&format!("{}.{role}", o.reference),
format!(
                        "{}: `{role}` holds {eff} pF effective at {w} mV under DC bias ({how}), from rated capacitance before its tolerance; it is screened against the {} pF minimum ({}), a margin of {}.{:02} times, typical, not guaranteed",
                        o.reference,
                        min.value,
                        ctx.cite(&min.at),
                        eff * 100 / min.value / 100,
                        eff * 100 / min.value % 100
                    ));
                }
                (None, Some(Ok((eff, how, w)))) if matches!(rule.role, Role::Cin | Role::Cout) => {
                    ctx.screen(
"effective_no_minimum",
&format!("{}.{role}", o.reference),
format!(
                        "{}: `{role}` would hold {eff} pF effective at {w} mV under DC bias ({how}); its row states no effective minimum, so only the rated value is judged",
                        o.reference
                    ));
                }
                _ => {
                    ctx.screen(
"rated_only",
&format!("{}.{role}", o.reference),
format!(
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
/// reservoir capacitor AN2834 requires, rated for VDDA's maximum
/// (`AET-WO-1218`). A declared reservoir is a capacitor row either way.
fn sense(ctx: &mut Ctx<'_>) -> Result<(), IrValidationError> {
    let (_, bmax) = battery_range(&ctx.power.battery);
    let vref_min = ctx.domain(&ctx.spec.mcu.supply).map_or(0, |d| range(d).0);
    for s in &ctx.power.sense {
        let (_, m) = mcu(ctx)?;
        let et = ctx.error_ppm(&s.top)?;
        let eb = ctx.error_ppm(&s.bottom)?;
        let mm = 1_000_000i128;
        let (rt, rb) = (i128::from(s.top.ohms), i128::from(s.bottom.ohms));
        let full_uv = sense_max_microvolts(bmax, s, et, eb);
        ctx.screen(
"vref_is_vdda",
&s.net,
format!(
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
        // A declared reservoir is a capacitor row whether or not RAIN needs
        // it (`AET-WO-1218`, `1217` R5): `emit` draws it either way.
        let reservoir = match &s.reservoir {
            None => None,
            Some(c) => match &ctx.row(&c.reference, &c.part)?.kind {
                Kind::Capacitor(cap) => Some((c, cap)),
                _ => return Err(ctx.kind_err(&c.reference, &c.part, "capacitor")),
            },
        };
        if rth <= i128::from(rain.value) {
            continue;
        }
        let Some((c, cap)) = reservoir else {
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
        // Rated for VDDA's maximum, by the pin devices' rule (`AET-WO-1218`,
        // `1217` R3): the pin can sit anywhere up to it.
        if let Some(vdda) = ctx.domain(&ctx.spec.mcu.supply)
            && !crate::pin_devices::reservoir_rated(cap.rated_millivolts.value, vdda)
        {
            return Err(refuse(
                Refusal::ExternalPartOutOfRange,
                &c.reference,
                format!(
                    "the reservoir is rated {} mV, below VDDA's maximum of {} mV on `{}`",
                    cap.rated_millivolts.value,
                    crate::maximum_millivolts(vdda),
                    vdda.id
                ),
                ctx.cite(&cap.rated_millivolts.at),
            ));
        }
        ctx.screen(
"reservoir_cadc_typical",
&s.net,
format!(
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

/// The battery's `battery_max_mv` through `s` at its largest ratio, µV: the
/// bottom resistor high and the top low, each by its worst error in ppm
/// (step 11's full scale, and `validate`'s step 25 at VDD = 0).
pub(crate) fn sense_max_microvolts(battery_max_mv: u64, s: &Sense, et: i128, eb: i128) -> i128 {
    let mm = 1_000_000i128;
    let num = i128::from(s.bottom.ohms) * (mm + eb);
    let den = num + i128::from(s.top.ohms) * (mm - et);
    div_ceil(i128::from(battery_max_mv) * 1000 * num, den)
}

/// The battery's maximum, mV: its most cells at their maximum.
pub(crate) fn battery_max_millivolts(b: &Battery) -> u64 {
    battery_range(b).1
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
        let s = supplies(ctx, t, &c.a_domain, &c.b_domain, false);
        let invalid = |detail: String| {
            refuse(
                Refusal::CrossingInvalid,
                &c.reference,
                detail,
                ctx.cite(&s.vcca_millivolts.at),
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
                ctx.cite(&s.vcca_millivolts.at),
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
        ctx.screen(
"translator_tmax_condition",
&c.reference,
format!(
            "{}: TMAX is stated with one channel switching at 50 % duty, and SCK and MOSI switch together; tw(SCKL)'s 2 ns shortfall is stated for SCK at PCLK / 2 and is applied at every rate",
            c.reference
        ));
        // A clock at f toggles at 2f transitions a second; a data line at
        // most as often.
        let rate = 2 * u64::from(bus.speed_hz);
        for (dir, tmax) in [
            ("A to B", &s.tmax_a_to_b_bps),
            ("B to A", &s.tmax_b_to_a_bps),
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
        ctx.screen(
"translator_delay_test_load",
&device.reference,
format!(
            "{}: the output delay is {tdo_cite}; the translator's delays are stated at its own test load",
            device.reference
        ));
        let total = i128::from(s.tpd_a_to_b_max_ns.value)
            + i128::from(tdo)
            + i128::from(s.tpd_b_to_a_max_ns.value)
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
                    s.tpd_a_to_b_max_ns.value,
                    s.tpd_b_to_a_max_ns.value,
                    m.spi_input_setup_min_ns.value,
                    bus.speed_hz
                ),
                ctx.cite(&s.tpd_a_to_b_max_ns.at),
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
    // Each radio module's ground pad (`AET-WO-4411` D4), after the
    // regulators' so none of theirs moves.
    if let Ok(t) = crate::radio::table() {
        for m in spec.declared_radio_modules() {
            if let Some(row) = t.get(&m.part) {
                out.push((
                    m.reference.clone(),
                    row.thermal_vias.pad.clone(),
                    vias(&row.thermal_vias, layers.clone()),
                ));
            }
        }
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
/// must hold. With no clamp, twice the battery's maximum must sit at or
/// under every battery-fed regulator's VIN absolute maximum (`AET-WO-4300k`
/// (6), a derivation, ruled D7).
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
                ctx.screen(
                    "bulk_esr_condition",
                    &h.reference,
                    format!(
                        "{}: {} damps the input's ring through its ESR, at most {} mOhm {} ({})",
                        h.reference,
                        h.part,
                        c.esr_max_milliohms.value,
                        c.esr_condition,
                        ctx.cite(&c.esr_max_milliohms.at)
                    ),
                );
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
                ctx.screen(
                    "tvs_clamp_condition",
                    &h.reference,
                    format!(
                        "{}: {} clamps at up to {} mV {} ({})",
                        h.reference,
                        h.part,
                        t.clamp_max_millivolts.value,
                        t.clamp_condition,
                        ctx.cite(&t.clamp_max_millivolts.at)
                    ),
                );
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
    // `AET-WO-4300k` (6), ruled D7: with no clamp, the ring is bounded by an
    // undamped LC step, twice the pack's maximum. A derivation, not a cited
    // figure: an ideal series LC driven by a step from rest peaks at twice the
    // step, and any loss (the bulk capacitor's ESR, the leads) only lowers it.
    let doubled = bmax.saturating_mul(2);
    for (reference, ceiling, _) in &fed {
        if clamps == 0 && doubled > ceiling.value {
            return Err(refuse(
                Refusal::DoubledPackOverAbsoluteMaximum,
                reference,
                format!(
                    "hot-plugging the pack rings VIN to at most twice its {bmax} mV, {doubled} mV (a derivation: an undamped LC step peaks at twice the step), above VIN's absolute maximum of {} mV, and no clamp is declared on the input",
                    ceiling.value
                ),
                ctx.cite(&ceiling.at),
            ));
        }
        if clamps == 0 {
            ctx.screen(
                "hot_plug_doubled_pack",
                reference,
                format!(
                    "{reference}: hot-plugging a pack through long leads rings VIN to at most twice the battery's {bmax} mV, {doubled} mV, under VIN's absolute maximum of {} mV ({}); the factor 2 is a derivation, not a cited figure: an ideal series LC stepped from rest peaks at twice the step, and loss only lowers it (ruled D7)",
                    ceiling.value,
                    ctx.cite(&ceiling.at)
                ),
            );
        }
    }
    Ok(())
}

/// Step 7a (`AET-WO-4300d3` §1.2, `AET-WO-4300k` (a)): each buck's
/// junction, T_J = T_board,max + P_loss × θJA. P_loss is the declared bound,
/// from a typical efficiency; θJA is the row's typical curve at the copper
/// area per layer **measured on the drawn board** ([`junction_screen`], run
/// by `extract`). Here the smallest of the figure's areas that clears the
/// limit is found, and a buck no area clears is refused; the gap between the
/// loss's sourced current and the solved load is printed (A13).
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
                "no loss bound is declared (`junction` is null), so the screen cannot clear it",
                ctx.cite(&curve.at),
            ));
        };
        let need = clearing(&curve.points, ctx.temperature().max, j, d.row);
        let Some(&(need_area, _)) = need else {
            let (at, theta) = curve.points.last().copied().unwrap_or((0, 0));
            return Err(refuse(
                Refusal::BuckJunctionOverLimit,
                &r.reference,
                format!(
                    "{} mW at θJA {}.{} °C/W, the figure's point at its largest area, {at} cm², raises the junction to {} m°C on a {} °C board, above {} °C: no copper area clears it",
                    j.loss_milliwatts.value,
                    theta / 10,
                    theta % 10,
                    junction_mk(ctx.temperature().max, j, theta),
                    ctx.temperature().max,
                    d.row.temperature_celsius.range.max
                ),
                ctx.cite(&curve.at),
            ));
        };
        ctx.screen(
            "buck_junction_screened",
            &r.reference,
            format!(
                "{}: its junction is screened, not guaranteed: it needs at least {need_area} cm² of copper per joined layer by the θJA figure, typical and for illustration only ({}); extract measures the drawn board's and screens it (AET-WO-4300k (a))",
                r.reference,
                ctx.cite(&curve.at)
            ),
        );
        // A plane of another net is copper the pad does not join; the figure
        // is copper per layer on a four-layer board (ruled (a)).
        for l in ctx.spec.board.layers.iter() {
            if let Some(net) = l.net.as_deref().filter(|n| *n != crate::GND) {
                ctx.screen(
                    "plane_not_joined",
                    &format!("{}.{}", r.reference, l.name),
                    format!(
                        "{}: {} is a plane of `{net}`, which its thermal pad does not join, so it counts no copper toward the screen",
                        r.reference, l.name
                    ),
                );
            }
        }
        let solved = load[&r.output];
        if solved > u64::from(j.loss_at_milliamps) {
            ctx.screen(
                "loss_current_gap",
                &r.reference,
                format!(
                    "{}: its loss is sourced at {} mA ({}) and its solved load is {solved} mA, a gap of {} mA the declared loss does not cover",
                    r.reference,
                    j.loss_at_milliamps,
                    j.loss_milliwatts.source,
                    solved - u64::from(j.loss_at_milliamps)
                ),
            );
        }
        let layers = ctx.spec.board.copper_layers;
        if layers != curve.board_copper_layers {
            ctx.screen(
                "theta_ja_board_mismatch",
                &r.reference,
                format!(
                    "{}: the board declares {layers} copper layers, outside the θJA figure's {}-layer board, so the screen's θJA is not this board's (FEEDBACK D1)",
                    r.reference, curve.board_copper_layers
                ),
            );
        }
        // The figure's board carries 2 oz outer and 1 oz inner copper; less
        // spreads less heat, so its θJA is not this board's (the power
        // review's F2). An undeclared minimum is less.
        let st = &ctx.spec.board.stackup;
        let outer = st.outer_copper_min_um.as_ref().map_or(0, |v| v.value);
        let inner = st.plane_copper_min_um.as_ref().map_or(0, |v| v.value);
        if outer < u32::from(curve.outer_copper_um) || inner < u32::from(curve.inner_copper_um) {
            ctx.screen(
                "theta_ja_copper_mismatch",
                &r.reference,
                format!(
                    "{}: the board's copper is at least {outer} um outer and {inner} um inner, under the θJA figure's {} um and {} um ({}), so the screen's θJA is lower than this board's",
                    r.reference,
                    curve.outer_copper_um,
                    curve.inner_copper_um,
                    ctx.cite(&curve.at)
                ),
            );
        }
    }
    Ok(())
}

/// The junction, m°C, on a `board_max` °C board at `theta` (0.1 °C/W).
fn junction_mk(board_max: i32, j: &Junction, theta: i64) -> i128 {
    // mW × (0.1 K/W) is 0.1 mK.
    i128::from(board_max) * 1000
        + div_ceil(i128::from(j.loss_milliwatts.value) * i128::from(theta), 10)
}

/// The θJA figure's first point, smallest area first, at which `j`'s loss
/// keeps the junction at `row`'s limit on a `board_max` °C board: step 9's
/// screen and [`junction_need_cm2`] alike.
fn clearing<'p>(
    points: &'p [(i64, i64)],
    board_max: i32,
    j: &Junction,
    row: &pp::Row,
) -> Option<&'p (i64, i64)> {
    points
        .iter()
        .find(|(_, theta)| junction_mk(board_max, j, *theta) <= limit_mk(row))
}

fn limit_mk(row: &pp::Row) -> i128 {
    i128::from(row.temperature_celsius.range.max) * 1000
}

/// Each buck's copper need: the θJA figure's smallest area, cm² per joined
/// layer, at which its declared loss keeps the junction at its limit on the
/// board's hottest declared temperature, as step 9's screen finds it; a buck
/// with no loss declared, or one no area clears, is absent (`validate`
/// refuses both). The router sizes the F.Cu it keeps about each buck's
/// thermal pad from it (`AET-WO-4411` (b)).
pub fn junction_need_cm2(spec: &Specification) -> BTreeMap<String, i64> {
    let (Ok(table), Some(power)) = (crate::power_parts::table(), spec.power.as_ref()) else {
        return BTreeMap::new();
    };
    let board_max = power.conditions.board_temperature_celsius.max;
    power
        .regulators
        .iter()
        .filter_map(|r| {
            let row = table.get(&r.part)?;
            let (Kind::Buck(b), Some(j)) = (&row.kind, &r.junction) else {
                return None;
            };
            clearing(&b.theta_ja.points, board_max, j, row)
                .map(|&(area, _)| (r.reference.clone(), area))
        })
        .collect()
}

/// A buck's junction screened at the copper `area_mm2` (mm²) measured per
/// joined layer on the drawn board (`AET-WO-4300k` (a), run by `extract`):
/// θJA is the figure's point at the largest of its areas at or under the
/// measured one. `Ok` carries the screened condition's text; a junction over
/// its limit, or an area under the figure's smallest, is refused
/// `BuckJunctionOverLimit`.
pub fn junction_screen(
    spec: &Specification,
    reference: &str,
    area_mm2: u64,
) -> Result<String, IrValidationError> {
    let table = crate::power_parts::table()?;
    let power = spec.power.as_ref().ok_or_else(|| {
        refuse(
            Refusal::PowerValueInvalid,
            reference,
            "no power tree",
            "AET-WO-4300k (a)",
        )
    })?;
    let r = power
        .regulators
        .iter()
        .find(|r| r.reference == reference)
        .ok_or_else(|| {
            refuse(
                Refusal::PowerValueInvalid,
                reference,
                "no such regulator",
                "AET-WO-4300k (a)",
            )
        })?;
    let row = table.get(&r.part).ok_or_else(|| {
        refuse(
            Refusal::PowerPartUnknown,
            reference,
            format!("part `{}`", r.part),
            "power-parts.json",
        )
    })?;
    let (Kind::Buck(b), Some(j)) = (&row.kind, &r.junction) else {
        return Err(refuse(
            Refusal::PowerValueInvalid,
            reference,
            "not a buck with a declared loss",
            "AET-WO-4300k (a)",
        ));
    };
    let curve = &b.theta_ja;
    let board_max = power.conditions.board_temperature_celsius.max;
    let cm2 = i64::try_from(area_mm2 / 100).unwrap_or(i64::MAX);
    let Some(&(at_area, theta)) = curve.points.iter().rev().find(|(a, _)| *a <= cm2) else {
        return Err(refuse(
            Refusal::BuckJunctionOverLimit,
            reference,
            format!(
                "the drawn board joins {}.{:02} cm² of copper per layer to its thermal pad at least, under the θJA figure's smallest area, {} cm²",
                area_mm2 / 100,
                area_mm2 % 100,
                curve.points.first().map_or(0, |p| p.0)
            ),
            table.cite(&curve.at),
        ));
    };
    let tj = junction_mk(board_max, j, theta);
    let limit = limit_mk(row);
    if tj > limit {
        return Err(refuse(
            Refusal::BuckJunctionOverLimit,
            reference,
            format!(
                "{} mW at θJA {}.{} °C/W (the figure's point at {at_area} cm², for the {}.{:02} cm² per layer measured) raises the junction to {tj} m°C on a {board_max} °C board, above {} °C",
                j.loss_milliwatts.value,
                theta / 10,
                theta % 10,
                area_mm2 / 100,
                area_mm2 % 100,
                row.temperature_celsius.range.max
            ),
            table.cite(&row.temperature_celsius.at),
        ));
    }
    Ok(format!(
        "{reference}: its junction is screened, not guaranteed: {} mW of loss (declared: {}) at θJA {}.{} °C/W, the figure's point at {at_area} cm² for the {}.{:02} cm² per joined layer measured on the drawn board ({}; {}), is {tj} m°C on a {board_max} °C board, {} mK under {} °C; the efficiency figure is typical at its own ambient",
        j.loss_milliwatts.value,
        j.loss_milliwatts.source,
        theta / 10,
        theta % 10,
        area_mm2 / 100,
        area_mm2 % 100,
        curve.condition,
        table.cite(&curve.at),
        limit - tj,
        row.temperature_celsius.range.max
    ))
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
    // `AET-WO-4411` (iii): each regulator whose `PG` another's `EN` follows,
    // by the follower; that `PG` is declared there, once.
    let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();
    for r in &ctx.power.regulators {
        for c in &r.controls {
            if let Some(a) = &c.after {
                let bad = |why: String| {
                    refuse(
                        Refusal::ControlAfterInvalid,
                        &r.reference,
                        format!(
                            "{}.{}: `after` names `{}`: {why}",
                            r.reference, c.pin, a.regulator
                        ),
                        "AET-WO-4411 (iii), ruled 2026-11-25T21:17Z: an enable follows another regulator's open-drain power-good",
                    )
                };
                let Some(lead) = ctx
                    .power
                    .regulators
                    .iter()
                    .find(|g| g.reference == a.regulator)
                else {
                    return Err(bad("no declared regulator".into()));
                };
                if lead.reference == r.reference {
                    return Err(bad("the regulator itself".into()));
                }
                let open_pg = match &ctx.row(&lead.reference, &lead.part)?.kind {
                    Kind::Buck(b) => b
                        .controls
                        .iter()
                        .any(|x| x.pin == "PG" && x.straps.contains(&pp::Strap::Open)),
                    _ => false,
                };
                if !open_pg {
                    return Err(bad(format!("{}'s row has no open-drain `PG`", lead.part)));
                }
                if claimed
                    .insert(lead.reference.as_str(), r.reference.as_str())
                    .is_some()
                {
                    return Err(bad("its `PG` already holds another enable".into()));
                }
                ctx.resistor(&a.pull_up)?;
            }
        }
    }
    // A loop starts nothing: a regulator waits on each regulator its
    // `after` names and on the one that outputs its input, and none on a
    // cycle of those ever starts (adversarial F1: a lead fed from its own
    // follower's rail).
    if !claimed.is_empty() {
        let waits = |r: &str| -> Vec<&str> {
            let Some(g) = ctx.power.regulators.iter().find(|g| g.reference == r) else {
                return Vec::new();
            };
            g.controls
                .iter()
                .filter_map(|c| c.after.as_ref().map(|a| a.regulator.as_str()))
                .chain(
                    ctx.power
                        .regulators
                        .iter()
                        .filter(|s| s.output == g.input)
                        .map(|s| s.reference.as_str()),
                )
                .collect()
        };
        for start in ctx.power.regulators.iter().map(|g| g.reference.as_str()) {
            let mut seen = BTreeSet::new();
            let mut todo = waits(start);
            while let Some(at) = todo.pop() {
                if at == start {
                    return Err(refuse(
                        Refusal::ControlAfterInvalid,
                        start,
                        format!(
                            "{start}'s enable waits on a loop of `after`s and supplies that comes back to it: none of them starts"
                        ),
                        "AET-WO-4411 (iii)",
                    ));
                }
                if seen.insert(at) {
                    todo.extend(waits(at));
                }
            }
        }
    }
    let owners = ctx
        .power
        .regulators
        .iter()
        .map(|r| (&r.reference, &r.part, r.controls.as_slice()))
        .chain(
            ctx.power
                .translators()
                .map(|c| (c.reference, c.part, c.controls)),
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
            let follower = (rule.pin == "PG")
                .then(|| claimed.get(reference.as_str()))
                .flatten();
            let Some(c) = declared.iter().find(|c| c.pin == rule.pin) else {
                if let Some(f) = follower {
                    ctx.screen(
                        "control_after",
                        f,
                        format!(
                            "{f}: its EN follows {reference}'s open-drain PG, pulled up to `{}`, so {f} starts only once {reference}'s PG releases, and stops whenever it pulls low ({}); the order at power-down is not set by it, and the pull-up against PG's low level and EN's threshold is not judged",
                            ctx.power.regulators.iter().find(|g| &g.reference == reference).map_or("", |g| g.output.as_str()),
                            ctx.cite(&rule.at)
                        ),
                    );
                    continue;
                }
                return Err(unwired(format!(
                    "{} is declared neither wired nor strapped (its row allows straps: {})",
                    rule.pin,
                    allowed()
                )));
            };
            if let Some(f) = follower {
                return Err(refuse(
                    Refusal::PowerValueInvalid,
                    reference,
                    format!("`controls` declares PG, which {f}'s `after` declares"),
                    "AET-WO-4411 (iii): the PG an enable follows is declared there, once",
                ));
            }
            if c.after.is_some() && !rule.straps.contains(&pp::Strap::Input) {
                return Err(refuse(
                    Refusal::ControlAfterInvalid,
                    reference,
                    format!(
                        "{} takes `after`, and its row names it no enable (an enable is strapped to the input)",
                        rule.pin
                    ),
                    ctx.cite(&rule.at),
                ));
            }
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
                (_, Some(w)) => wires.push((
                    format!("{reference}.{}", rule.pin),
                    format!(
                        "{reference}: {} is declared wired to `{}` ({}); neither the net's driver nor its level is judged",
                        rule.pin, w.net, w.role
                    ),
                )),
                _ => {}
            }
        }
    }
    for (id, w) in wires {
        ctx.screen("control_wire_unjudged", &id, w);
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
        screened: BTreeMap::new(),
    };
    mcu(&ctx)?;
    temperatures(&mut ctx)?;
    thermal(&ctx)?;
    controls(&mut ctx)?;
    let derived = outputs(&mut ctx)?;
    radio_rails(&mut ctx, &derived)?;
    domains_inside_consumers(&ctx)?;
    bor_rising(&mut ctx)?;
    battery_inputs(&ctx)?;
    hot_plug(&mut ctx)?;
    let load = loads(&mut ctx, &derived)?;
    let input_drop = protection(&mut ctx, &derived, &load)?;
    headroom(&mut ctx, &derived, &load, input_drop)?;
    junctions(&mut ctx, &derived, &load)?;
    externals(&mut ctx)?;
    for d in &power.conditions.dc_bias_derating {
        let kinds: Vec<&str> = d.dielectrics.iter().map(|k| dielectric(*k)).collect();
        ctx.screen(
"derating_declared",
&kinds.join("+"),
format!(
            "power: a {} ceramic at up to {} per mille of its rated voltage keeps {} per mille of its rated capacitance, a declared derating, not a maker's figure (declared: {})",
            kinds.join(" or "),
            d.ratio_max_permille,
            d.retained_permille,
            d.source
        ));
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
                // A device a `loads` entry names by `reference` is counted
                // (`AET-WO-4300k` A13: the line named every device before).
                let loaded = ctx.power.loads.iter().any(|l| {
                    l.domain == d.reg.output && l.reference.as_deref() == Some(reference.as_str())
                });
                if !loaded {
                    uncounted.push(reference.as_str());
                }
                if let Some(p) = sensors.get(part) {
                    consumers.push((
                        format!("{reference} ({part})"),
                        u64::from(p.supply_millivolts.0),
                    ));
                }
            }
        }
        if !uncounted.is_empty() {
            ctx.screen(
                "supply_current_not_counted",
                &d.reg.output,
                format!(
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
        // `AET-WO-4300k` A13: a regulated rail nothing loads is refused, so
        // an emptied `loads` cannot run every load-bound check at 0 mA.
        if load[&d.reg.output] == 0 {
            return Err(refuse(
                Refusal::RegulatedDomainUnloaded,
                &d.reg.output,
                format!(
                    "{} regulates `{}`, and nothing loads it: no `power.loads` entry, no MCU and no regulator draws from it, so every check that reads its load would run at 0 mA",
                    d.reg.reference, d.reg.output
                ),
                "AET-WO-4300k A13",
            ));
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
    for domain in domains.keys() {
        if !ctx.power.drop_limits.iter().any(|l| &l.domain == domain) {
            ctx.screen(
                "drop_not_judged",
                domain,
                format!(
                    "{domain}: no drop limit is declared, and extract's supply gate judges the MCU's pins (on `{}`) and a radio module's on its own rail only, so this domain's copper drop is not judged (ruled D13)",
                    ctx.spec.mcu.supply
                ),
            );
        }
    }
    Ok(Some(Checked {
        domains,
        input_steady_milliamps: steady,
        input_fault_milliamps: fault,
        conditions: ctx.conditions.into_iter().collect(),
        screened: ctx.screened,
    }))
}

/// One battery-fed regulator, as the battery sees it (`AET-WO-2304b` §1.5):
/// a constant-power sink with its floors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatterySink {
    pub reference: String,
    /// Its output maximum × its load, rounded up, plus its declared
    /// `loss_milliwatts`, mW: what it draws from the battery.
    pub input_milliwatts: u64,
    /// Its stated minimum operating input (`vin_millivolts.min`), mV: below
    /// it the part's behaviour is not stated (F5).
    pub shutdown_millivolts: u64,
    /// The input from which its regulation band is stated for its output
    /// (`regulation_vin_min`, D-B9), mV.
    pub regulation_millivolts: u64,
}

impl BatterySink {
    /// The input the sink needs: the higher of its two floors, mV.
    pub fn floor_millivolts(&self) -> u64 {
        self.shutdown_millivolts.max(self.regulation_millivolts)
    }
}

/// A capacitor on `VBAT`, as the chain counts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryCapacitor {
    pub reference: String,
    pub picofarads: u64,
    /// The bulk part's ESR maximum, mΩ; `None` for a ceramic, whose ESR no
    /// row holds (L723), so the deck takes it as ideal.
    pub esr_max_milliohms: Option<u64>,
}

/// The battery side of a power tree (`AET-WO-2304b` §1.5): one derivation,
/// from the specification and the parts table alone, which the scenario's
/// checks, the deck and the coverage rule all read. It shares no code with
/// the generator (`RUL-20` (1)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatterySide {
    /// `cells.min × cell_min_millivolts`, µV: the pack's declared floor,
    /// taken as an ideal source behind the path R.
    pub v_open_microvolts: u64,
    /// The healthy path R, as declared: `None` is undeclared.
    pub healthy: Option<SourcedValue>,
    /// Each battery-fed regulator, in the specification's order.
    pub sinks: Vec<BatterySink>,
    /// The hot-plug bulk parts, at their rated capacitance with their ESR
    /// maximum, then the battery-fed regulators' input ceramics at the
    /// effective capacitance IR's static checks credit them.
    pub capacitors: Vec<BatteryCapacitor>,
}

/// [`battery_side_against`] with the embedded tables. `None` when the
/// specification declares no power tree.
pub fn battery_side(spec: &Specification) -> Result<Option<BatterySide>, IrValidationError> {
    if spec.power.is_none() {
        return Ok(None);
    }
    let sensors = crate::parts::table()?;
    battery_side_against(
        spec,
        crate::power_parts::table()?,
        sensors,
        &output_delays(sensors),
    )
}

/// The battery side, after every check [`check_against`] makes: a tree it
/// refuses has no battery side.
pub fn battery_side_against(
    spec: &Specification,
    table: &PowerPartsTable,
    sensors: &crate::parts::PartsTable,
    delays: &OutputDelays,
) -> Result<Option<BatterySide>, IrValidationError> {
    let Some(checked) = check_against(spec, table, sensors, delays)? else {
        return Ok(None);
    };
    let power = spec.power.as_ref().expect("check_against returned a tree");
    let ctx = Ctx {
        spec,
        power,
        table,
        delays,
        floors: Vec::new(),
        conditions: BTreeSet::new(),
        screened: BTreeMap::new(),
    };
    let unmodelled = |reference: &str, detail: String| {
        refuse(
            Refusal::BatterySideUnmodelled,
            reference,
            detail,
            "docs/work-orders/AET-WO-2304b.md §1.5",
        )
    };
    // A row's figure the chain needs is positive, or the chain is refused by
    // name; never read as 0 (`INV-06`, adversarial 8).
    let positive = |reference: &str, what: &str, v: i64| {
        u64::try_from(v).ok().filter(|v| *v > 0).ok_or_else(|| {
            unmodelled(
                reference,
                format!("its {what} is {v}, not a positive figure"),
            )
        })
    };
    let mut sinks = Vec::new();
    let mut capacitors = Vec::new();
    for h in &power.input.hot_plug {
        let row = ctx.row(&h.reference, &h.part)?;
        if let Kind::BulkCapacitor(c) = &row.kind {
            capacitors.push(BatteryCapacitor {
                reference: h.reference.clone(),
                picofarads: positive(&h.reference, "rated capacitance", c.rated_picofarads.value)?,
                esr_max_milliohms: Some(positive(
                    &h.reference,
                    "ESR maximum",
                    c.esr_max_milliohms.value,
                )?),
            });
        }
    }
    for r in power.regulators.iter().filter(|r| r.input == BATTERY) {
        let row = ctx.row(&r.reference, &r.part)?;
        // `power_in` refuses a battery-fed LDO (no cited fault bound), so a
        // checked tree holds only bucks here; another kind is refused.
        let Kind::Buck(b) = &row.kind else {
            return Err(unmodelled(
                &r.reference,
                format!(
                    "{} is not a buck: the chain models a battery-fed buck only",
                    r.part
                ),
            ));
        };
        // `junctions` refuses a buck with no junction declaration.
        let Some(j) = &r.junction else {
            return Err(unmodelled(
                &r.reference,
                "no `loss_milliwatts` is declared (`junction` is null), so its input power is not stated".to_owned(),
            ));
        };
        let d = &checked.domains[&r.output];
        let nominal = ctx
            .domain(&r.output)
            .map_or(0, |v| i64::from(v.nominal_millivolts));
        let Some(regulation) = regulation_floor(b, nominal) else {
            return Err(unmodelled(
                &r.reference,
                format!("its row states no regulation input for a {nominal} mV output"),
            ));
        };
        // µV × mA = nW; mW rounded up, so the sink is never under-counted.
        let out_mw = u128::from(d.vout_max_microvolts)
            .saturating_mul(u128::from(d.load_milliamps))
            .div_ceil(1_000_000);
        let input_milliwatts = u64::try_from(out_mw)
            .unwrap_or(u64::MAX)
            .saturating_add(u64::from(j.loss_milliwatts.value));
        sinks.push(BatterySink {
            reference: r.reference.clone(),
            input_milliwatts,
            shutdown_millivolts: positive(
                &r.reference,
                "minimum operating input",
                b.vin_millivolts.range.min,
            )?,
            regulation_millivolts: positive(&r.reference, "regulation input", regulation.value)?,
        });
        let working = i64::try_from(battery_range(&power.battery).1).unwrap_or(i64::MAX);
        for e in r
            .external
            .iter()
            .filter(|e| matches!(e.role, Role::Cin | Role::CinHf))
        {
            let pf = credited(&ctx, &e.reference, &e.part, working)?;
            capacitors.push(BatteryCapacitor {
                reference: e.reference.clone(),
                picofarads: positive(&e.reference, "credited capacitance", pf)?,
                esr_max_milliohms: None,
            });
        }
    }
    let (bmin, _) = battery_range(&power.battery);
    Ok(Some(BatterySide {
        v_open_microvolts: bmin.saturating_mul(1000),
        healthy: power.battery.series_milliohms.clone(),
        sinks,
        capacitors,
    }))
}

/// A ceramic's capacitance as IR's static checks credit it at `working` mV
/// ([`effective`]): its maker's curve where held, the declared derating
/// otherwise, refused where neither covers it, never the bare nominal.
fn credited(
    ctx: &Ctx<'_>,
    reference: &str,
    part: &str,
    working: i64,
) -> Result<i64, IrValidationError> {
    let row = ctx.row(reference, part)?;
    let Kind::Capacitor(c) = &row.kind else {
        return Err(ctx.kind_err(reference, part, "capacitor"));
    };
    effective(ctx, part, c, working)
        .map(|(pf, _)| pf)
        .map_err(|(kind, why)| {
            refuse(
                kind,
                reference,
                why,
                "crates/aethelgard-ir/data/power-parts.json",
            )
        })
}

/// One regulator output capacitor (`Role::Cout`) as IR's static checks
/// credit it (`AET-WO-2304b` F3): the domain it holds, and pF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditedCapacitor {
    pub domain: String,
    pub picofarads: u64,
}

/// Every regulator's output capacitors, by reference, at the capacitance
/// IR's static checks credit them at the domain's maximum (`AET-WO-2304b`
/// F3). The emitter adds each one it draws to its rail's decoupling
/// (`aethelgard-eda`'s `decoupling_picofarads`), so the droop's τ and the
/// static screen read one derivation (`RUL-20`). Empty when no tree is
/// declared.
pub fn output_capacitance(
    spec: &Specification,
) -> Result<BTreeMap<String, CreditedCapacitor>, IrValidationError> {
    let Some(power) = spec.power.as_ref() else {
        return Ok(BTreeMap::new());
    };
    let sensors = crate::parts::table()?;
    let delays = output_delays(sensors);
    let ctx = Ctx {
        spec,
        power,
        table: crate::power_parts::table()?,
        delays: &delays,
        floors: Vec::new(),
        conditions: BTreeSet::new(),
        screened: BTreeMap::new(),
    };
    let mut out = BTreeMap::new();
    for r in &power.regulators {
        let working = ctx
            .domain(&r.output)
            .map_or(0, |d| i64::try_from(range(d).1).unwrap_or(i64::MAX));
        for e in r.external.iter().filter(|e| e.role == Role::Cout) {
            let pf = credited(&ctx, &e.reference, &e.part, working)?;
            let picofarads = u64::try_from(pf).ok().filter(|v| *v > 0).ok_or_else(|| {
                refuse(
                    Refusal::PowerValueInvalid,
                    &e.reference,
                    format!("its credited capacitance is {pf} pF, not a positive figure"),
                    "crates/aethelgard-ir/data/power-parts.json",
                )
            })?;
            out.insert(
                e.reference.clone(),
                CreditedCapacitor {
                    domain: r.output.clone(),
                    picofarads,
                },
            );
        }
    }
    Ok(out)
}

/// What a voltage domain supplies that a fault on it could reach
/// (`AET-WO-2304b` §1.7), from the specification alone: each serial port
/// whose RX net (the line its off-board peer drives) is on a connector that
/// also carries the domain, with that connector's declared standard; each
/// SPI and I2C device whose `supply` is the domain; each I2C bus with a
/// line on such a connector, and whether it has a device; and every other
/// net on such a connector, with the connector's reference.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RailConsumers {
    pub serial_rx: BTreeSet<String>,
    /// The `standard` of each connector a rendered port's RX net is on
    /// (`None` where it declares none).
    pub standards: BTreeSet<Option<String>>,
    pub spi_devices: BTreeSet<String>,
    pub i2c_devices: BTreeSet<String>,
    /// Each I2C bus with a line on such a connector: whether any device is
    /// declared on it.
    pub i2c_buses: BTreeMap<String, bool>,
    /// Serial ports, by label, with only their TX net on such a connector:
    /// their peer is fed from the domain, and nothing observes its absence.
    pub tx_only: BTreeSet<String>,
    /// Each regulator whose input is the domain.
    pub regulators: BTreeSet<String>,
    /// Each radio module the domain supplies (`AET-WO-4411`): its port's
    /// RX, which its TX drives, is in `serial_rx`.
    pub radio_modules: BTreeSet<String>,
    /// `(connector, net)` for each other net on a connector carrying it.
    pub other_nets: Vec<(String, String)>,
}

/// [`RailConsumers`] of `domain`.
pub fn rail_consumers(spec: &Specification, domain: &str) -> RailConsumers {
    let on = |d: &str| d == domain;
    let mut r = RailConsumers {
        spi_devices: spec
            .declared_spi_devices()
            .iter()
            .filter(|d| on(&d.supply))
            .map(|d| d.reference.clone())
            .collect(),
        i2c_devices: spec
            .declared_i2c_devices()
            .iter()
            .filter(|d| on(&d.supply))
            .map(|d| d.reference.clone())
            .collect(),
        regulators: spec
            .power
            .iter()
            .flat_map(|p| &p.regulators)
            .filter(|r| on(&r.input))
            .map(|r| r.reference.clone())
            .collect(),
        ..RailConsumers::default()
    };
    for c in spec.declared_connectors() {
        let nets: Vec<&str> = c.pins.iter().filter_map(|p| p.net.as_deref()).collect();
        if !nets.iter().any(|n| on(n)) {
            continue;
        }
        for net in nets {
            if on(net) || net == crate::GND {
                continue;
            }
            if let Some(p) = spec
                .serial_ports
                .iter()
                .find(|p| p.rx.net == net || p.tx.net == net)
            {
                if p.rx.net == net {
                    r.serial_rx.insert(p.label.clone());
                    r.standards.insert(c.standard.clone());
                } else {
                    r.tx_only.insert(p.label.clone());
                }
            } else if let Some(b) = spec
                .i2c_buses
                .iter()
                .find(|b| b.sda.net == net || b.scl.net == net)
            {
                let used = spec.declared_i2c_devices().iter().any(|d| d.bus == b.id);
                r.i2c_buses.insert(b.id.clone(), used);
            } else {
                r.other_nets.push((c.reference.clone(), net.to_owned()));
            }
        }
    }
    for m in spec
        .declared_radio_modules()
        .iter()
        .filter(|m| on(&m.supply))
    {
        r.serial_rx.insert(m.serial_port.clone());
        r.radio_modules.insert(m.reference.clone());
    }
    let rx = r.serial_rx.clone();
    r.tx_only.retain(|p| !rx.contains(p));
    r
}
