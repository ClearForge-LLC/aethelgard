//! The power parts table (`AET-WO-4300d`, ruled D11 on PR #137): regulators,
//! the bus translator, the passives they need and the MCU's supply facts, as
//! cited data beside `parts.json`'s sensor rows.
//!
//! The table is `data/power-parts.json`, embedded in this crate and pinned by
//! its sha256 in a tier-1 test. One parser, [`parse_power_parts`], reads it and
//! every test fixture. It never panics, and it refuses a malformed row by
//! naming the part and the field (`INV-06`).
//!
//! **Citations.** The file's `documents` map names each datasheet once: its
//! title, its revision, and its sha256 when held, or the https URL it must be
//! fetched from (`pending`). Every value carries `at`: the document's key and
//! the page and table it was read from. A document no value cites is refused,
//! as is a value citing a document the map lacks.
//!
//! **Bounds.** A value is a guaranteed minimum or maximum the document prints,
//! never a typical. A limit the document gives "by design, not production
//! tested" says so (`production_tested: false`), and every check that uses it
//! names it as a condition. A field the document states only as typical is
//! not a field here. Where a check needs it, the check names it as a
//! condition (spike S3, "typical-only fields are never a bound").

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use serde::Deserialize;

use crate::IrValidationError;

/// The table as embedded: `data/power-parts.json`.
pub const POWER_PARTS_JSON: &str = include_str!("../data/power-parts.json");

/// The `schema_version` the parser reads. Version 4 gave the MCU row
/// `flash_kilobytes` and `ardupilot` (`AET-WO-4400b` D4, D6). Version 5
/// added the `crystal` kind (`AET-WO-4240a` §1.3). Version 6 gave the MCU
/// row `io`, each I/O pin's structure and the input each structure takes in
/// operation (`AET-WO-1211e` A1). Version 7 gave `io` its absolute maximum
/// ratings, DS12117 Table 10 (`AET-WO-1216b`). Version 8 gave it
/// `analog_pads`, the pins whose pad takes only an analog input
/// (`AET-WO-1217` F4). Version 9 gave it `weak_pull_down_ohms`, the RPD
/// range a declared motor pull-down is held inside (`AET-WO-4300i`, D0).
/// Version 10 replaced that with `motor_pull_bound`, the inputs the motor
/// pull-down's bound is computed from per board (`AET-WO-4300j` D3 (a)), and
/// added the `jumper` kind, the pull's series link (D4).
pub const POWER_PARTS_SCHEMA_VERSION: u32 = 10;

/// A document the table cites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub title: String,
    pub revision: String,
    pub held: crate::parts::Held,
}

/// Where one value was read: the document's key in `documents`, and the page
/// and table or section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct At {
    pub document: String,
    pub section: String,
}

/// A guaranteed range. `min` and `max` are each the document's printed limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub min: i64,
    pub max: i64,
}

/// One cited value: a single limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub value: i64,
    pub at: At,
}

/// One cited range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranged {
    pub range: Range,
    pub at: At,
}

/// One value that is an engine choice or a stand-in, not a datasheet's
/// figure: the value and its basis, the work order or ruling that set it
/// (as `bound_um` beside `bound_basis`). No document is cited for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub value: i64,
    pub basis: String,
}

/// A part's production state, as its maker's page states it, or `unread`
/// when no page with a lifecycle field was readable (`4300d` spike S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lifecycle {
    Active,
    Eol,
    Unread,
}

/// A synchronous buck converter with an external feedback divider
/// (the LMR33630ADDA).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buck {
    /// VIN: the maximum of the minimum operating input, and the recommended
    /// maximum, mV.
    pub vin_millivolts: Ranged,
    /// VIN's absolute maximum, mV: the ceiling a hot-plug ring is judged
    /// against (`AET-WO-4300d3` §1.1).
    pub vin_abs_max_millivolts: Limit,
    /// The bulk capacitance TI asks for in parallel with the ceramics to damp
    /// the input's ring on long leads, pF (§1.1).
    pub input_damping_picofarads: Ranged,
    /// The feedback reference, µV.
    pub vfb_microvolts: Ranged,
    /// The current into FB, at most, nA.
    pub ifb_max_nanoamps: Limit,
    /// The output's regulation band from no load to full load, per mille of
    /// the set point; and whether the maker tests it in production.
    pub regulation_permille: Ranged,
    pub regulation_production_tested: bool,
    /// The lowest VIN the band is stated from, by output: `(nominal mV from
    /// which the row applies, VIN min)`, highest first. Below it the band is
    /// unstated, and the regulator is refused (ruled D-B9).
    pub regulation_vin_min: Vec<(i64, Limit)>,
    /// The switching frequency, Hz.
    pub fsw_hz: Ranged,
    /// Minimum on-time and off-time, each at most, ns.
    pub ton_min_max_ns: Limit,
    pub toff_min_max_ns: Limit,
    /// High-side and low-side on-resistance, at most, mΩ.
    pub rds_high_max_milliohms: Limit,
    pub rds_low_max_milliohms: Limit,
    /// The high-side current limit, mA: its maximum bounds a fault.
    pub high_side_limit_milliamps: Ranged,
    /// The recommended output current, at most, mA.
    pub iout_max_milliamps: Limit,
    /// The feedback divider's top resistor, at most, Ω.
    pub rfbt_max_ohms: Limit,
    /// Equation 5's minimum inductance, `L_MIN >= k x VOUT / fSW`, as `k` in
    /// nH·MHz/V (0.28 µH·MHz/V is 280).
    pub inductor_min_coefficient: Limit,
    /// The test condition the timing, on-resistance and frequency limits
    /// are stated at, named in every check that reads them (ruled D5).
    pub stated_at: String,
    /// The required external parts (ruled D6).
    pub external: Vec<ExternalRule>,
    /// The thermal-via rule for the exposed pad (`AET-WO-1309` D1).
    pub thermal_vias: ThermalRule,
    /// The θJA the junction screen reads (`AET-WO-4300d3` §1.2).
    pub theta_ja: ThetaCurve,
    /// The control pins and how each may be tied (§1.5).
    pub controls: Vec<ControlRule>,
}

/// A typical θJA curve against the copper area on each layer, as a maker
/// plots it for one package on one board. A screen, never a bound: each point
/// is read from the plot and rounded up, and the curve falls with area, so a
/// declared area takes the point at or below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThetaCurve {
    /// The package code the curve is for; the row's part ends with it.
    pub package: String,
    /// The copper layers of the board the curve was taken on.
    pub board_copper_layers: u8,
    pub condition: String,
    /// `(cm² per layer, tenths of a kelvin per watt)`, area rising.
    pub points: Vec<(i64, i64)>,
    pub at: At,
}

/// How a control pin may be tied when nothing drives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strap {
    /// To the regulator's own input.
    Input,
    /// Left unconnected, where the datasheet allows it.
    Open,
    Vcca,
    Vccb,
}

impl Strap {
    pub fn name(self) -> &'static str {
        match self {
            Strap::Input => "input",
            Strap::Open => "open",
            Strap::Vcca => "vcca",
            Strap::Vccb => "vccb",
        }
    }
}

/// One control pin: the straps its datasheet allows, and whether it may be
/// wired to a net instead (`AET-WO-4300d3` §1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRule {
    /// A pin role of the row.
    pub pin: String,
    pub straps: Vec<Strap>,
    pub wirable: bool,
    pub at: At,
}

/// A linear regulator with a fixed output (the TLV75518PDBV).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ldo {
    /// The input range, mV.
    pub vin_millivolts: Ranged,
    /// VIN's absolute maximum, mV.
    pub vin_abs_max_millivolts: Limit,
    /// The output over the junction range, mV, and the condition it is
    /// tested at, named in every check that reads it.
    pub vout_millivolts: Ranged,
    pub vout_condition: String,
    /// The dropout, at most, mV, at `dropout_at_milliamps`.
    pub dropout_max_millivolts: Limit,
    pub dropout_at_milliamps: i64,
    pub iout_max_milliamps: Limit,
    /// The ground current, at most, µA.
    pub ignd_max_microamps: Limit,
    /// θJA on the JEDEC board, in tenths of a kelvin per watt, and the board
    /// it is stated for, named in the junction check (ruled D1).
    pub theta_ja_decikelvin_per_watt: Limit,
    pub theta_ja_condition: String,
    /// The junction's operating maximum, °C.
    pub junction_max_celsius: Limit,
    pub external: Vec<ExternalRule>,
    pub controls: Vec<ControlRule>,
}

/// One channel of a fixed-direction translator: an input pin on one side
/// and its output on the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub input: String,
    pub output: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    AToB,
    BToA,
}

/// A fixed-direction translator (the TXU0304PW).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translator {
    /// The band each side's supply must lie in for the timing row the checks
    /// read, mV. It is narrower than the part's operating range, and is the
    /// range the domains are judged against (ruled D13).
    pub vcca_millivolts: Ranged,
    pub vccb_millivolts: Ranged,
    /// Propagation delay, at most, ns, in that row.
    pub tpd_a_to_b_max_ns: Limit,
    pub tpd_b_to_a_max_ns: Limit,
    /// The highest data rate, at least, bit/s, each way, in that row.
    pub tmax_a_to_b_bps: Limit,
    pub tmax_b_to_a_bps: Limit,
    /// The largest pull-up a floating input may take against its internal
    /// pull-down, Ω (ruled D9).
    pub pull_up_max_ohms: Limit,
    pub channels: Vec<Channel>,
    pub external: Vec<ExternalRule>,
    pub controls: Vec<ControlRule>,
}

impl Translator {
    /// The four pins a crossing carries, as the emitter draws them: the
    /// outputs of the first three A-to-B channels (SCK, MOSI, CS) and the
    /// input of the first B-to-A channel (MISO). `None` when the part has
    /// fewer (`AET-WO-1206`: one list for the emitter and `validate`).
    pub fn crossing_lines(&self) -> Option<[&str; 4]> {
        let mut a = self
            .channels
            .iter()
            .filter(|c| c.direction == Direction::AToB);
        let miso = self
            .channels
            .iter()
            .find(|c| c.direction == Direction::BToA)?;
        Some([
            a.next()?.output.as_str(),
            a.next()?.output.as_str(),
            a.next()?.output.as_str(),
            miso.input.as_str(),
        ])
    }
}

/// A thin-film or thick-film resistor series: every value of the series
/// shares these limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resistor {
    /// Tolerance, ppm of the value.
    pub tolerance_ppm: Limit,
    /// Temperature coefficient, at most, ppm/K, from `tcr_reference_celsius`.
    pub tcr_ppm_per_kelvin: Limit,
    pub tcr_reference_celsius: i64,
    /// The drift limit over the life the series states, ppm, and that life.
    pub drift_ppm: Limit,
    pub drift_condition: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Dielectric {
    C0g,
    X5r,
    X7r,
    X7s,
}

/// One orderable capacitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capacitor {
    pub rated_picofarads: Limit,
    pub rated_millivolts: Limit,
    pub dielectric: Dielectric,
    /// The maker's typical DC-bias curve, where one is held: a screen,
    /// never a bound (`AET-WO-4300d3` §1.3, the amendment on PR #158).
    pub dc_bias: Option<DcBiasCurve>,
    /// The capacitance tolerance, either side, where the row states one
    /// (`AET-WO-4240c` A2); `None` on a row that does not.
    pub tolerance: Option<Tolerance>,
}

/// A capacitance tolerance, either side of the rated value, cited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tolerance {
    /// Per mille of the rated value: ±1 % is 10.
    Permille(Limit),
    /// Femtofarads: ±0.25 pF is 250.
    Femtofarads(Limit),
}

/// A maker's DC-bias curve: the capacitance change at each voltage, read
/// from the maker's chart, with the part's tolerance and its dielectric's
/// temperature limit, which compose with it by product, worst case. Between
/// points it is interpolated linearly; past its last point it is never
/// extrapolated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DcBiasCurve {
    /// `(mV, change in per mille)`, voltage rising from 0 mV.
    pub points: Vec<(i64, i64)>,
    pub at: At,
    /// The tolerance's and the temperature limit's worst loss, per mille.
    pub tolerance_permille: Limit,
    pub temperature_permille: Limit,
}

/// An aluminium electrolytic or polymer capacitor that damps a battery
/// input's ring (`AET-WO-4300d3` §1.1). Its ESR is cited because damping is
/// the point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkCapacitor {
    pub rated_picofarads: Limit,
    pub rated_millivolts: Limit,
    /// ESR, at most, mΩ, at `esr_condition`.
    pub esr_max_milliohms: Limit,
    pub esr_condition: String,
}

/// A transient voltage suppressor on a battery input (§1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tvs {
    /// The reverse stand-off voltage, mV.
    pub standoff_millivolts: Limit,
    /// The clamping voltage, at most, mV, at the rated pulse `clamp_condition`.
    pub clamp_max_millivolts: Limit,
    pub clamp_condition: String,
}

/// A zero-ohm jumper (`AET-WO-4300j` D4): its resistance and current, each
/// at most. A jumper has no tolerance, TCR or drift a resistor row needs
/// (VISHAY_20035 p. 2: TCR "is not specified for 0 Ohm jumpers").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumper {
    pub max_milliohms: Limit,
    pub max_milliamps: Limit,
}

/// One orderable inductor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inductor {
    /// Inductance, nH, at its tolerance.
    pub nanohenries: Ranged,
    /// Saturation current, mA, and its stated definition. Inductor makers
    /// state it typical only (the IHLP-5050FD-01 does), so it is a screen and
    /// a named condition, never a bound (spike S3).
    pub isat_typical_milliamps: Limit,
    pub isat_definition: String,
    /// DC resistance, at most, mΩ.
    pub dcr_max_milliohms: Limit,
}

/// A quartz crystal for the HSE (`AET-WO-4240a` §1.3): the figures the ppm
/// budget ([`crate::clock`]) and DS12117's `Gm_crit_max` rule read. The
/// row's `temperature_celsius` is the range its stability holds over.
/// Each figure is the datasheet's bound for the ordered part, its option
/// codes included; a typical-only figure is not a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crystal {
    /// The nominal frequency, Hz.
    pub frequency_hz: Limit,
    /// The load capacitance the frequency is specified at, fF.
    pub load_femtofarads: Limit,
    /// The frequency tolerance at +25 °C, ppm.
    pub tolerance_ppm: Ranged,
    /// The frequency stability over the row's operating temperature, ppm,
    /// referred to +25 °C.
    pub stability_ppm: Ranged,
    /// The aging over `aging_hours`, ppm.
    pub aging_ppm: Ranged,
    /// The span the aging figure is stated for, hours: a declared service
    /// life beyond it has no cited aging term (`AET-WO-4240a` §5).
    pub aging_hours: Limit,
    /// The equivalent series resistance at this frequency, at most, mΩ.
    pub esr_max_milliohms: Limit,
    /// The shunt capacitance, at most, fF.
    pub c0_max_femtofarads: Limit,
    /// The drive level, at most, µW: a bench measurement on the first board
    /// judges it (AN2867 §3.5; not computable from datasheets, `4240` §3).
    pub drive_level_max_microwatts: Limit,
}

/// The MCU's supply facts: its range, its declared load, and its brown-out
/// thresholds' falling maxima.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcu {
    pub vdd_millivolts: Ranged,
    /// The largest supply current the datasheet tables (`1303` D4).
    pub idd_max_milliamps: Limit,
    pub idd_condition: String,
    /// VBOR0 to VBOR3 falling, each at most, mV.
    pub bor_falling_max_millivolts: Vec<Limit>,
    /// VBOR0 to VBOR3 rising, each at most, mV: the supply must clear it to
    /// leave reset (`AET-WO-4300d3` §1.4).
    pub bor_rising_max_millivolts: Vec<Limit>,
    /// The ADC's largest external input resistance per resolution, Ω, as
    /// `(bits, limit)`, highest resolution first.
    pub rain_max_ohms: Vec<(u8, Limit)>,
    /// The reservoir rule for a source above RAIN (ruled D10 (a)).
    pub reservoir_rule: ReservoirRule,
    /// The SPI master's data-input setup time, at least, ns (ruled D9).
    pub spi_input_setup_min_ns: Limit,
    /// How much shorter than half a period SCK's low phase may be, ns.
    pub spi_sck_low_shortfall_ns: Limit,
    /// The support network its datasheet draws (`AET-WO-4300g`).
    pub support: Support,
    /// The embedded flash, KB (`AET-WO-4400b` D4).
    pub flash_kilobytes: Limit,
    /// What ArduPilot `Copter-4.7.2` calls the MCU and the SPI kernel clocks
    /// it runs it at (`AET-WO-4400b` D4, D6).
    pub ardupilot: McuArduPilot,
    /// Each I/O pin's structure and what it tolerates (`AET-WO-1211e` A1).
    pub io: McuIo,
}

/// The MCU's I/O pins against the input each takes (`AET-WO-1211e` A1):
/// every pin the pin table holds (the vendored pin data's `I/O` pins, by
/// canonical name), its structure as the datasheet's pin table prints it
/// (`FT_h`, `TT_a`, …), and each structure family's input in operation. A
/// pull is permanent, so the operating conditions bound it, not the stress
/// ratings (ruled D-A1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McuIo {
    /// Each pin's structure, and where it was read.
    pub pins: BTreeMap<String, (String, At)>,
    /// Each family (the structure before its first `_`), once.
    pub families: Vec<IoFamily>,
    /// Above the supply plus this, the internal pulls must be off.
    pub pulls_off_above_supply: Limit,
    /// The absolute maximum ratings (DS12117 Table 10), the stress limits a
    /// pin takes at any supply, read at VDD = 0 by `validate`'s step 25
    /// (`AET-WO-1216b`).
    pub absolute_maximum: IoAbsoluteMaximum,
    /// The pins whose pad takes only an analog input (DS12117 Table 8's type
    /// `ANA`): the solver offers them to no digital claimant and refuses one
    /// by hand (`AET-WO-1217` F4).
    pub analog_pads: AnalogPads,
    /// The inputs a declared external motor pull-down's bound is computed
    /// from, per board, on `mcu.supply` (`AET-WO-4300j` D3 (a),
    /// `crate::offboard::motor_pull_window`).
    pub motor_pull_bound: MotorPullBound,
}

/// The motor pull-down's bound, held as its inputs (`AET-WO-4300j`
/// DERIVATION.md §3, §4 and §5a, D1 (a) as ruled): the floor keeps the idle
/// line at or above the input-high threshold plus the margin against the
/// weakest internal pull-up in BLHeli passthrough, and the ceiling keeps it
/// at or below the input-low threshold less the margin against the leakage
/// at reset. The four DS12117 figures are cited; the peer's input current
/// (a stand-in, S2) and the margin are rulings, each with its basis. The
/// reader holds each above zero, the thresholds below 1000 permille and in
/// order, and the margin inside both: `margin < input_low_max` and
/// `input_high_min + margin < 1000`, so neither threshold is non-positive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotorPullBound {
    /// RPU, the weak pull-up's equivalent resistor, max, Ω.
    pub pull_up_max_ohms: Limit,
    /// VIH min, per mille of VDD.
    pub input_high_min_permille: Limit,
    /// VIL max, per mille of VDD.
    pub input_low_max_permille: Limit,
    /// The pad's input leakage, at most, nA.
    pub leakage_max_nanoamps: Limit,
    /// The peer's (the ESC input's) current, nA: a stand-in.
    pub peer_input_current_nanoamps: Chosen,
    /// The margin at each threshold, per mille of VDD.
    pub margin_permille: Chosen,
}

/// Pins of [`McuIo::pins`] whose pad is analog only, and where that is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalogPads {
    pub pins: BTreeSet<String>,
    pub at: At,
}

/// Table 10's input limits: each structure family's, the BOOT0 pin's and
/// every other pin's (`NRST` among them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoAbsoluteMaximum {
    /// Each family of [`McuIo::families`], once, in the same order.
    pub families: Vec<IoFamily>,
    pub boot0: Limit,
    pub other: Limit,
}

/// What one structure family takes: the lesser of the pin's supply plus
/// `above_supply_mv` and `max_mv`, each where stated (at least one is).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoFamily {
    pub family: String,
    pub above_supply_mv: Option<i64>,
    pub max_mv: Option<i64>,
    pub at: At,
}

impl McuIo {
    /// The family of `pin`'s structure and the input it tolerates with its
    /// supply at `supply_mv`; `None` for a pin whose structure is not held.
    pub fn tolerated(&self, pin: &str, supply_mv: i64) -> Option<(&str, &At, i64, &IoFamily)> {
        let (structure, at) = self.pins.get(pin)?;
        let family = self.family(structure)?;
        Some((structure.as_str(), at, family.limit(supply_mv), family))
    }

    /// The least input any held pin tolerates with its supply at `supply_mv`:
    /// a pull at or below it is tolerated wherever it lands.
    pub fn floor(&self, supply_mv: i64) -> i64 {
        self.families
            .iter()
            .map(|f| f.limit(supply_mv))
            .min()
            .unwrap_or(0)
    }

    fn family(&self, structure: &str) -> Option<&IoFamily> {
        let name = structure.split('_').next().unwrap_or(structure);
        self.families.iter().find(|f| f.family == name)
    }

    /// The most `pin` takes by Table 10 with its supply at `supply_mv`, and
    /// where that is stated: `BOOT0`'s row, `NRST` by Table 10's "any other
    /// pin", and a held pin by its structure's family. With no pin (a signal
    /// the solver places) or a pin with no held structure, the least of every
    /// row, since it may be any of them; nothing is assumed.
    pub fn absolute_maximum(&self, pin: Option<&str>, supply_mv: i64) -> (i64, &At) {
        let a = &self.absolute_maximum;
        let family = |structure: &str| {
            let name = structure.split('_').next().unwrap_or(structure);
            a.families.iter().find(|f| f.family == name)
        };
        match pin {
            Some(crate::BOOT0) => (a.boot0.value, &a.boot0.at),
            Some(crate::NRST) => (a.other.value, &a.other.at),
            Some(p) => match self.pins.get(p).and_then(|(s, _)| family(s)) {
                Some(f) => (f.limit(supply_mv), &f.at),
                None => self.least_absolute_maximum(supply_mv),
            },
            None => self.least_absolute_maximum(supply_mv),
        }
    }

    fn least_absolute_maximum(&self, supply_mv: i64) -> (i64, &At) {
        let a = &self.absolute_maximum;
        // The first of equal limits, so a structure's row is cited before
        // "any other pin".
        a.families
            .iter()
            .map(|f| (f.limit(supply_mv), &f.at))
            .chain([(a.other.value, &a.other.at)])
            .min_by_key(|(v, _)| *v)
            .unwrap_or((a.other.value, &a.other.at))
    }
}

impl IoFamily {
    /// The input this family tolerates with its supply at `supply_mv`.
    pub fn limit(&self, supply_mv: i64) -> i64 {
        let above = self.above_supply_mv.map_or(i64::MAX, |a| supply_mv + a);
        above.min(self.max_mv.unwrap_or(i64::MAX))
    }
}

/// ArduPilot's names for the MCU (`MCU <series> <mcu>`), and, for the one
/// `OSCILLATOR_HZ` these rows hold for, the kernel clock ArduPilot gives each
/// SPI peripheral, which its divider halves from `kernel / 2` at most seven
/// times (`spi_divider_at`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McuArduPilot {
    pub series: String,
    pub mcu: String,
    pub mcu_at: At,
    pub table_at: At,
    pub oscillator_hz: u32,
    pub oscillator_at: At,
    /// Keyed `SPI1` to `SPI6`, each once.
    pub spi_kernel_hz: BTreeMap<String, Limit>,
    pub spi_divider_at: At,
}

/// The MCU's support network, as its datasheet draws it (`AET-WO-4300g`):
/// its package pins by group, each capacitor role, and each strap. The
/// emitter draws from it, and the support gate judges an emitted netlist
/// against it, never against the emitter's choices (`RUL-20`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Support {
    /// Each group's package pin numbers, ascending; no pin is in two groups.
    pub pins: BTreeMap<String, Vec<String>>,
    pub pins_at: At,
    /// The group whose pins are all on the MCU's supply.
    pub supply: String,
    /// The groups whose pins are all on `GND`.
    pub grounds: Vec<String>,
    /// Groups each on a declared voltage domain whose whole tolerance band
    /// lies in the group's operating range.
    pub powered: Vec<Powered>,
    /// Groups whose every pin is on a net of its own: no declared domain,
    /// not `GND`, and no other support pin's (`AET-WO-4300g` adversarial A1).
    pub own_nets: Vec<OwnNet>,
    pub capacitors: Vec<SupportCapacitor>,
    pub straps: Vec<SupportStrap>,
}

impl Support {
    /// The package pins of `group`; empty for a group the row does not name.
    pub fn group(&self, group: &str) -> &[String] {
        self.pins.get(group).map_or(&[], Vec::as_slice)
    }
}

/// A powered group and the range its supply must hold, mV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Powered {
    pub group: String,
    pub millivolts: Range,
    pub at: At,
}

/// A group whose pins each need a net of their own, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnNet {
    pub group: String,
    pub at: At,
}

/// What a support capacitor does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportRole {
    VddBypass,
    VddBulk,
    AnalogBypass,
    AnalogBulk,
    UsbBypass,
    UsbBulk,
    Vcap,
    Reset,
}

impl SupportRole {
    pub fn name(self) -> &'static str {
        match self {
            Self::VddBypass => "vdd_bypass",
            Self::VddBulk => "vdd_bulk",
            Self::AnalogBypass => "analog_bypass",
            Self::AnalogBulk => "analog_bulk",
            Self::UsbBypass => "usb_bypass",
            Self::UsbBulk => "usb_bulk",
            Self::Vcap => "vcap",
            Self::Reset => "reset",
        }
    }
}

/// One capacitor role: between its owner group's net and its ground group's
/// net, of `picofarads` within `tolerance_permille` (its value class), one at
/// each owner pin (`per_pin`) or one for the group, and, where the gate can
/// check it, within `bound_um` of its owner pin, copper edge to copper edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportCapacitor {
    pub role: SupportRole,
    pub owner: String,
    pub per_pin: bool,
    pub ground: String,
    pub picofarads: i64,
    pub tolerance_permille: i64,
    /// `None`: no distance the gate can check, a named condition.
    pub bound_um: Option<i64>,
    pub bound_basis: String,
    pub at: At,
}

impl SupportCapacitor {
    /// The value class, pF: `picofarads` less and plus its tolerance.
    pub fn class(&self) -> (i64, i64) {
        let d = self.picofarads * self.tolerance_permille / 1000;
        (self.picofarads - d, self.picofarads + d)
    }
}

/// A single pin tied to a group's net, directly (an MCU support strap; a
/// control pin's is [`Strap`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportStrap {
    pub pin: String,
    pub to: String,
    pub at: At,
}

/// AN2834's rule for an external reservoir capacitor at an ADC input,
/// `Cext >= Csh x Umax / Ulsb` (Umax the full scale in LSB, Ulsb 0.5 LSB),
/// held as `Cext >= 2^(N + extra_bits) x C_ADC` with `extra_bits` 1, which is
/// at least the rule at every resolution. DS12117 states C_ADC typical only,
/// so the rule is a screen with C_ADC named as a condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservoirRule {
    pub statement: String,
    pub extra_bits: u8,
    pub adc_capacitance_femtofarads: i64,
    pub at: At,
}

/// What a row is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Buck(Box<Buck>),
    Ldo(Box<Ldo>),
    Translator(Box<Translator>),
    Resistor(Resistor),
    Capacitor(Capacitor),
    Inductor(Inductor),
    Mcu(Box<Mcu>),
    BulkCapacitor(BulkCapacitor),
    Tvs(Tvs),
    Crystal(Box<Crystal>),
    Jumper(Jumper),
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Buck(_) => "buck",
            Kind::Ldo(_) => "ldo",
            Kind::Translator(_) => "translator",
            Kind::Resistor(_) => "resistor",
            Kind::Capacitor(_) => "capacitor",
            Kind::Inductor(_) => "inductor",
            Kind::Mcu(_) => "mcu",
            Kind::BulkCapacitor(_) => "bulk_capacitor",
            Kind::Tvs(_) => "tvs",
            Kind::Crystal(_) => "crystal",
            Kind::Jumper(_) => "jumper",
        }
    }
}

/// A role an external part plays for its regulator or translator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The bulk input capacitance.
    Cin,
    /// The small high-frequency input capacitor at the input pins.
    CinHf,
    Cboot,
    Cvcc,
    Cout,
    Inductor,
    /// A supply bypass on side A or B of a translator.
    BypassA,
    BypassB,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Cin => "cin",
            Role::CinHf => "cin_hf",
            Role::Cboot => "cboot",
            Role::Cvcc => "cvcc",
            Role::Cout => "cout",
            Role::Inductor => "inductor",
            Role::BypassA => "bypass_a",
            Role::BypassB => "bypass_b",
        }
    }
}

/// A role's total rated capacitance, pF: at least `min`, and at most `max`
/// where the datasheet states a maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacitance {
    pub min: i64,
    pub max: Option<i64>,
}

/// What a regulator's datasheet requires of one external role: the total
/// rated capacitance of the parts declared in it, pF (`None` for the
/// inductor, which [`Buck::inductor_min_coefficient`] judges), the least
/// rated voltage, a higher least rating from a domain's nominal up, the
/// dielectrics allowed, and whether the rated voltage must also cover the
/// voltage the part works at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalRule {
    pub role: Role,
    pub value: Option<Capacitance>,
    pub rated_min_millivolts: Option<i64>,
    /// `(nominal mV, rated mV)`: on a domain of at least that nominal, the
    /// rating is at least that (LMR33630: 16 V or more for 5 V and above).
    pub rated_step: Option<(i64, i64)>,
    /// When true, the rated voltage must also be at least the highest
    /// voltage the part sees: the input's maximum for an input capacitor,
    /// the domain's maximum for an output capacitor.
    pub rated_at_least_working: bool,
    pub dielectrics: Vec<Dielectric>,
    pub at: At,
    /// The least effective capacitance (under DC bias) the datasheet states
    /// for the role, where it states one, pF (`AET-WO-4300d3` §1.3).
    pub effective_min_picofarads: Option<EffectiveMin>,
}

/// A role's effective minimum, and what a shortfall does: refuse, or be
/// reported by name, not blocking (ruled D2 (c) on PR #158: the buck's input,
/// whose enforcement and re-pick are the next power work order's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveMin {
    pub limit: Limit,
    pub on_shortfall: OnShortfall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnShortfall {
    Enforce,
    Report,
}

/// The thermal-via rule of an exposed pad, as the datasheet states it. The
/// layers are not a row's: they are the declared board's ground planes, or
/// `B.Cu` on a two-layer board, joined at `validate` (`AET-WO-4300d` K7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalRule {
    pub pad: String,
    pub columns: u8,
    pub rows: u8,
    pub pitch_um: u32,
    pub drill_um: u32,
    pub ring_um: u32,
    pub sources: crate::thermal::ThermalSources,
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub part: String,
    pub lifecycle: Lifecycle,
    /// KiCad 10.0.7's symbol and footprint, `library:name`, for slice B's
    /// emitter (`RUL-11`).
    pub symbol: String,
    pub footprint: String,
    /// Pin roles to pin numbers, for an IC: what `SolvedTopology` names as a
    /// source or return pad (spike S5).
    pub pins: BTreeMap<String, String>,
    /// The operating temperature, °C: the board's declared range must lie
    /// inside it (ruled D7).
    pub temperature_celsius: Ranged,
    pub kind: Kind,
}

/// The parsed table: its documents and its rows, sorted by part name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerPartsTable {
    documents: BTreeMap<String, Document>,
    rows: Vec<Row>,
}

impl PowerPartsTable {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn get(&self, part: &str) -> Option<&Row> {
        self.rows
            .binary_search_by(|r| r.part.as_str().cmp(part))
            .ok()
            .map(|i| &self.rows[i])
    }

    pub fn document(&self, key: &str) -> Option<&Document> {
        self.documents.get(key)
    }

    /// A value's citation as one line: the document, its revision, the
    /// section, and the sha256 or the URL it is pending at.
    pub fn cite(&self, at: &At) -> String {
        match self.documents.get(&at.document) {
            Some(d) => {
                let held = match &d.held {
                    crate::parts::Held::Sha256(h) => format!("sha256 {h}"),
                    crate::parts::Held::Pending(url) => format!("pending {url}"),
                };
                format!("{} {}, {}, {held}", d.title, d.revision, at.section)
            }
            None => format!("{} (not in the table), {}", at.document, at.section),
        }
    }

    /// Every document not held: (key, URL).
    pub fn pending(&self) -> Vec<(&str, &str)> {
        self.documents
            .iter()
            .filter_map(|(k, d)| match &d.held {
                crate::parts::Held::Pending(url) => Some((k.as_str(), url.as_str())),
                crate::parts::Held::Sha256(_) => None,
            })
            .collect()
    }
}

// ---- the wire form ---------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    schema_version: u32,
    /// A key given twice is refused, never collapsed (adversarial A5).
    #[serde(deserialize_with = "crate::unique_keys")]
    documents: BTreeMap<String, RawDocument>,
    /// Each row's own text, parsed strictly on its own, so a key given twice
    /// in a row is refused, never collapsed (`AET-WO-4200` A2).
    parts: Vec<Box<serde_json::value::RawValue>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDocument {
    title: String,
    revision: String,
    sha256: Option<String>,
    pending: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAt {
    document: String,
    section: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimit {
    value: i64,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRanged {
    min: i64,
    max: i64,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRow {
    part: String,
    lifecycle: Lifecycle,
    symbol: String,
    footprint: String,
    #[serde(deserialize_with = "crate::unique_keys")]
    pins: BTreeMap<String, String>,
    temperature_celsius: RawRanged,
    kind: RawKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum RawKind {
    Buck(Box<RawBuck>),
    Ldo(Box<RawLdo>),
    Translator(Box<RawTranslator>),
    Resistor(RawResistor),
    Capacitor(RawCapacitor),
    Inductor(RawInductor),
    Mcu(Box<RawMcu>),
    BulkCapacitor(RawBulk),
    Tvs(RawTvs),
    Crystal(Box<RawCrystal>),
    Jumper(RawJumper),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawJumper {
    max_milliohms: RawLimit,
    max_milliamps: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuck {
    vin_millivolts: RawRanged,
    vin_abs_max_millivolts: RawLimit,
    input_damping_picofarads: RawRanged,
    vfb_microvolts: RawRanged,
    ifb_max_nanoamps: RawLimit,
    regulation_permille: RawRanged,
    regulation_production_tested: bool,
    regulation_vin_min: Vec<RawVinStep>,
    fsw_hz: RawRanged,
    ton_min_max_ns: RawLimit,
    toff_min_max_ns: RawLimit,
    rds_high_max_milliohms: RawLimit,
    rds_low_max_milliohms: RawLimit,
    high_side_limit_milliamps: RawRanged,
    iout_max_milliamps: RawLimit,
    rfbt_max_ohms: RawLimit,
    inductor_min_coefficient: RawLimit,
    stated_at: String,
    external: Vec<RawExternal>,
    theta_ja: RawTheta,
    controls: Vec<RawControl>,
    thermal_vias: RawThermal,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTheta {
    package: String,
    board_copper_layers: u8,
    condition: String,
    points: Vec<RawThetaPoint>,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawThetaPoint {
    square_centimetres: i64,
    decikelvin_per_watt: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawControl {
    pin: String,
    straps: Vec<Strap>,
    wirable: bool,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVinStep {
    from_vout_millivolts: i64,
    vin_min: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLdo {
    vin_millivolts: RawRanged,
    vin_abs_max_millivolts: RawLimit,
    vout_millivolts: RawRanged,
    vout_condition: String,
    dropout_max_millivolts: RawLimit,
    dropout_at_milliamps: i64,
    iout_max_milliamps: RawLimit,
    ignd_max_microamps: RawLimit,
    theta_ja_decikelvin_per_watt: RawLimit,
    theta_ja_condition: String,
    junction_max_celsius: RawLimit,
    external: Vec<RawExternal>,
    controls: Vec<RawControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChannel {
    input: String,
    output: String,
    direction: Direction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTranslator {
    vcca_millivolts: RawRanged,
    vccb_millivolts: RawRanged,
    tpd_a_to_b_max_ns: RawLimit,
    tpd_b_to_a_max_ns: RawLimit,
    tmax_a_to_b_bps: RawLimit,
    tmax_b_to_a_bps: RawLimit,
    pull_up_max_ohms: RawLimit,
    channels: Vec<RawChannel>,
    external: Vec<RawExternal>,
    controls: Vec<RawControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResistor {
    tolerance_ppm: RawLimit,
    tcr_ppm_per_kelvin: RawLimit,
    tcr_reference_celsius: i64,
    drift_ppm: RawLimit,
    drift_condition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapacitor {
    rated_picofarads: RawLimit,
    rated_millivolts: RawLimit,
    dielectric: Dielectric,
    /// Required as a key: `null` where no maker curve is held.
    #[serde(deserialize_with = "Option::deserialize")]
    dc_bias: Option<RawCurve>,
    /// The capacitance tolerance, where the row states one (`AET-WO-4240c`
    /// A2): an optional key, absent on the rows read before it.
    #[serde(default)]
    tolerance: Option<RawTolerance>,
}

/// A capacitance tolerance as the maker states it: a share of the value, or
/// an absolute figure (a small C0G part's "±0.25 pF").
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum RawTolerance {
    Permille(RawLimit),
    Femtofarads(RawLimit),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCurve {
    points: Vec<RawCurvePoint>,
    at: RawAt,
    tolerance_permille: RawLimit,
    temperature_permille: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCurvePoint {
    millivolts: i64,
    change_permille: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBulk {
    rated_picofarads: RawLimit,
    rated_millivolts: RawLimit,
    esr_max_milliohms: RawLimit,
    esr_condition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTvs {
    standoff_millivolts: RawLimit,
    clamp_max_millivolts: RawLimit,
    clamp_condition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCrystal {
    frequency_hz: RawLimit,
    load_femtofarads: RawLimit,
    tolerance_ppm: RawRanged,
    stability_ppm: RawRanged,
    aging_ppm: RawRanged,
    aging_hours: RawLimit,
    esr_max_milliohms: RawLimit,
    c0_max_femtofarads: RawLimit,
    drive_level_max_microwatts: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInductor {
    nanohenries: RawRanged,
    isat_typical_milliamps: RawLimit,
    isat_definition: String,
    dcr_max_milliohms: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcu {
    vdd_millivolts: RawRanged,
    idd_max_milliamps: RawLimit,
    idd_condition: String,
    bor_falling_max_millivolts: Vec<RawLimit>,
    bor_rising_max_millivolts: Vec<RawLimit>,
    rain_max_ohms: Vec<RawRain>,
    reservoir_rule: RawReservoirRule,
    spi_input_setup_min_ns: RawLimit,
    spi_sck_low_shortfall_ns: RawLimit,
    support: RawSupport,
    flash_kilobytes: RawLimit,
    ardupilot: RawMcuArduPilot,
    io: RawMcuIo,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcuIo {
    #[serde(deserialize_with = "crate::unique_keys")]
    pins: BTreeMap<String, RawIoPin>,
    pins_at: RawAt,
    families: Vec<RawIoFamily>,
    pulls_off_above_supply_millivolts: RawLimit,
    absolute_maximum: RawIoAbsoluteMaximum,
    analog_pads: RawAnalogPads,
    motor_pull_bound: RawMotorPullBound,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMotorPullBound {
    pull_up_max_ohms: RawLimit,
    input_high_min_permille: RawLimit,
    input_low_max_permille: RawLimit,
    leakage_max_nanoamps: RawLimit,
    peer_input_current_nanoamps: RawChosen,
    margin_permille: RawChosen,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChosen {
    value: i64,
    basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnalogPads {
    pins: Vec<String>,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIoAbsoluteMaximum {
    families: Vec<RawIoFamily>,
    boot0_max_millivolts: RawLimit,
    other_max_millivolts: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIoPin {
    structure: String,
    page: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIoFamily {
    family: String,
    #[serde(deserialize_with = "Option::deserialize")]
    above_supply_millivolts: Option<i64>,
    #[serde(deserialize_with = "Option::deserialize")]
    max_millivolts: Option<i64>,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcuArduPilot {
    series: String,
    mcu: String,
    mcu_at: RawAt,
    table_at: RawAt,
    oscillator_hz: u32,
    oscillator_at: RawAt,
    #[serde(deserialize_with = "crate::unique_keys")]
    spi_kernel_hz: BTreeMap<String, RawLimit>,
    spi_divider_at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupport {
    #[serde(deserialize_with = "crate::unique_keys")]
    pins: BTreeMap<String, Vec<String>>,
    pins_at: RawAt,
    supply: String,
    grounds: Vec<String>,
    powered: Vec<RawPowered>,
    own_nets: Vec<RawOwnNet>,
    capacitors: Vec<RawSupportCapacitor>,
    straps: Vec<RawStrap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPowered {
    group: String,
    min_millivolts: i64,
    max_millivolts: i64,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOwnNet {
    group: String,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupportCapacitor {
    role: SupportRole,
    owner: String,
    per_pin: bool,
    ground: String,
    picofarads: i64,
    tolerance_permille: i64,
    #[serde(deserialize_with = "Option::deserialize")]
    bound_um: Option<i64>,
    bound_basis: String,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrap {
    pin: String,
    to: String,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRain {
    bits: u8,
    max_ohms: RawLimit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReservoirRule {
    statement: String,
    extra_bits: u8,
    adc_capacitance_femtofarads: i64,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExternal {
    role: Role,
    /// Each required as a key; `null` for the inductor.
    #[serde(deserialize_with = "Option::deserialize")]
    min_picofarads: Option<i64>,
    #[serde(deserialize_with = "Option::deserialize")]
    max_picofarads: Option<i64>,
    #[serde(deserialize_with = "Option::deserialize")]
    effective_min_picofarads: Option<RawEffective>,
    #[serde(deserialize_with = "Option::deserialize")]
    rated_min_millivolts: Option<i64>,
    #[serde(deserialize_with = "Option::deserialize")]
    rated_step: Option<RawStep>,
    rated_at_least_working: bool,
    dielectrics: Vec<Dielectric>,
    at: RawAt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEffective {
    value: i64,
    at: RawAt,
    on_shortfall: OnShortfall,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStep {
    from_nominal_millivolts: i64,
    rated_min_millivolts: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawThermal {
    pad: String,
    columns: u8,
    rows: u8,
    pitch_um: u32,
    drill_um: u32,
    ring_um: u32,
    sources: RawThermalSources,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawThermalSources {
    count: String,
    pitch: String,
    drill: String,
    ring: String,
    layers: String,
}

// ---- the parser ------------------------------------------------------------

/// The largest magnitude any value may take: no power part's figure comes
/// near it in its unit, and anything above is a typo.
pub const MAX_VALUE: i64 = 1_000_000_000_000;

/// The MCU row's `ardupilot`: names ArduPilot reads as identifiers, and a
/// kernel clock above zero for exactly `SPI1` to `SPI6`.
fn mcu_ardupilot(m: RawMcuArduPilot, r: &mut Reader<'_>) -> Result<McuArduPilot, String> {
    for (v, f) in [(&m.series, "series"), (&m.mcu, "mcu")] {
        if v.is_empty() || v.len() > 32 || !v.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(format!(
                "`kind.mcu.ardupilot.{f}` is `{v}`, not 1 to 32 letters and digits"
            ));
        }
    }
    let want: BTreeSet<String> = (1..=6).map(|n| format!("SPI{n}")).collect();
    let got: BTreeSet<String> = m.spi_kernel_hz.keys().cloned().collect();
    if got != want {
        return Err(
            "`kind.mcu.ardupilot.spi_kernel_hz` is keyed by exactly SPI1 to SPI6".to_owned(),
        );
    }
    let mut spi_kernel_hz = BTreeMap::new();
    for (p, l) in m.spi_kernel_hz {
        let field = format!("kind.mcu.ardupilot.spi_kernel_hz.{p}");
        spi_kernel_hz.insert(p, r.positive(l, &field)?);
    }
    Ok(McuArduPilot {
        series: m.series,
        mcu: m.mcu,
        mcu_at: r.at(m.mcu_at, "kind.mcu.ardupilot.mcu")?,
        table_at: r.at(m.table_at, "kind.mcu.ardupilot.table")?,
        oscillator_hz: m.oscillator_hz,
        oscillator_at: r.at(m.oscillator_at, "kind.mcu.ardupilot.oscillator")?,
        spi_kernel_hz,
        spi_divider_at: r.at(m.spi_divider_at, "kind.mcu.ardupilot.spi_divider")?,
    })
}

/// Reads values, recording each document cited so the parser can refuse a
/// document nothing cites.
struct Reader<'d> {
    documents: &'d BTreeMap<String, Document>,
    cited: BTreeSet<String>,
}

impl Reader<'_> {
    fn at(&mut self, raw: RawAt, field: &str) -> Result<At, String> {
        if !self.documents.contains_key(&raw.document) {
            return Err(format!(
                "`{field}.at.document` names `{}`, which `documents` does not hold",
                raw.document
            ));
        }
        text(&raw.section, &format!("{field}.at.section"), 256)?;
        self.cited.insert(raw.document.clone());
        Ok(At {
            document: raw.document,
            section: raw.section,
        })
    }

    fn limit(&mut self, raw: RawLimit, field: &str) -> Result<Limit, String> {
        bounded(raw.value, field)?;
        Ok(Limit {
            value: raw.value,
            at: self.at(raw.at, field)?,
        })
    }

    /// A limit that must be above zero.
    fn positive(&mut self, raw: RawLimit, field: &str) -> Result<Limit, String> {
        let l = self.limit(raw, field)?;
        if l.value <= 0 {
            return Err(format!("`{field}.value` is {}, not above zero", l.value));
        }
        Ok(l)
    }

    fn ranged(&mut self, raw: RawRanged, field: &str) -> Result<Ranged, String> {
        bounded(raw.min, field)?;
        bounded(raw.max, field)?;
        if raw.min > raw.max {
            return Err(format!(
                "`{field}` is {} to {}: its min is above its max",
                raw.min, raw.max
            ));
        }
        Ok(Ranged {
            range: Range {
                min: raw.min,
                max: raw.max,
            },
            at: self.at(raw.at, field)?,
        })
    }

    /// A range whose min is above zero.
    fn ranged_positive(&mut self, raw: RawRanged, field: &str) -> Result<Ranged, String> {
        let r = self.ranged(raw, field)?;
        if r.range.min <= 0 {
            return Err(format!("`{field}.min` is {}, not above zero", r.range.min));
        }
        Ok(r)
    }

    fn external(
        &mut self,
        raw: Vec<RawExternal>,
        field: &str,
    ) -> Result<Vec<ExternalRule>, String> {
        let mut out: Vec<ExternalRule> = Vec::with_capacity(raw.len());
        for e in raw {
            let f = format!("{field}.{}", e.role.name());
            if out.iter().any(|o| o.role == e.role) {
                return Err(format!(
                    "`{field}` names the role `{}` twice",
                    e.role.name()
                ));
            }
            let at = self.at(e.at, &f)?;
            let inductor = e.role == Role::Inductor;
            let value = match (e.min_picofarads, e.max_picofarads) {
                (None, None) if inductor => None,
                (Some(min), max) if !inductor => {
                    bounded(min, &f)?;
                    if min <= 0 || max.is_some_and(|m| m < min) {
                        return Err(format!("`{f}` is {min} to {max:?} pF, not 0 < min <= max"));
                    }
                    if let Some(m) = max {
                        bounded(m, &f)?;
                    }
                    Some(Capacitance { min, max })
                }
                _ => {
                    return Err(format!(
                        "`{f}` needs `min_picofarads` (and `max_picofarads` or null) for a capacitor role, and both null for the inductor"
                    ));
                }
            };
            let positive = |v: Option<i64>, what: &str| -> Result<(), String> {
                match v {
                    Some(v) if v <= 0 => Err(format!("`{f}.{what}` is not above zero")),
                    Some(v) => bounded(v, &f),
                    None => Ok(()),
                }
            };
            positive(e.rated_min_millivolts, "rated_min_millivolts")?;
            let step = match e.rated_step {
                Some(s) => {
                    positive(
                        Some(s.from_nominal_millivolts),
                        "rated_step.from_nominal_millivolts",
                    )?;
                    positive(
                        Some(s.rated_min_millivolts),
                        "rated_step.rated_min_millivolts",
                    )?;
                    Some((s.from_nominal_millivolts, s.rated_min_millivolts))
                }
                None => None,
            };
            let rated = e.rated_min_millivolts.is_some() || e.rated_at_least_working;
            if inductor && (rated || step.is_some() || !e.dielectrics.is_empty()) {
                return Err(format!(
                    "`{f}`: an inductor has no voltage rating or dielectric"
                ));
            }
            if !inductor && (!rated || e.dielectrics.is_empty()) {
                return Err(format!(
                    "`{f}`: a capacitor role needs a rating rule and its allowed dielectrics"
                ));
            }
            let effective = match e.effective_min_picofarads {
                Some(_) if inductor => {
                    return Err(format!("`{f}`: an inductor has no effective capacitance"));
                }
                Some(m) => Some(EffectiveMin {
                    limit: self.positive(
                        RawLimit {
                            value: m.value,
                            at: m.at,
                        },
                        &format!("{f}.effective_min_picofarads"),
                    )?,
                    on_shortfall: m.on_shortfall,
                }),
                None => None,
            };
            out.push(ExternalRule {
                role: e.role,
                value,
                rated_min_millivolts: e.rated_min_millivolts,
                rated_step: step,
                rated_at_least_working: e.rated_at_least_working,
                dielectrics: e.dielectrics,
                at,
                effective_min_picofarads: effective,
            });
        }
        out.sort_by_key(|e| e.role);
        Ok(out)
    }
}

impl Reader<'_> {
    /// A maker's DC-bias curve: from 0 mV, voltage rising, to at most the
    /// rated voltage; each change above -1000 per mille; and, composed with
    /// the tolerance and the temperature limit, never above the rated
    /// capacitance (a bias lie, `AET-WO-4300d3` §5.3).
    fn curve(&mut self, c: RawCurve, rated_mv: i64) -> Result<DcBiasCurve, String> {
        let f = "kind.capacitor.dc_bias";
        let tol = self.limit(c.tolerance_permille, &format!("{f}.tolerance_permille"))?;
        let temp = self.limit(c.temperature_permille, &format!("{f}.temperature_permille"))?;
        for (what, l) in [("tolerance", &tol), ("temperature", &temp)] {
            if !(0..1000).contains(&l.value) {
                return Err(format!(
                    "`{f}.{what}_permille` is {}, not 0 to 999",
                    l.value
                ));
            }
        }
        if c.points.len() < 2 || c.points[0].millivolts != 0 {
            return Err(format!("`{f}.points` needs two or more, the first at 0 mV"));
        }
        let mut points: Vec<(i64, i64)> = Vec::with_capacity(c.points.len());
        for q in c.points {
            bounded(q.millivolts, f)?;
            bounded(q.change_permille, f)?;
            if q.change_permille <= -1000
                || q.millivolts > rated_mv
                || points.last().is_some_and(|(v, _)| *v >= q.millivolts)
            {
                return Err(format!(
                    "`{f}.points` holds {} mV at {} per mille: not voltage rising to at most the rated {rated_mv} mV, with a change above -1000",
                    q.millivolts, q.change_permille
                ));
            }
            points.push((q.millivolts, q.change_permille));
        }
        let most = points.iter().map(|p| p.1).max().unwrap_or(0);
        if (1000 + i128::from(most))
            * (1000 - i128::from(tol.value))
            * (1000 - i128::from(temp.value))
            > 1_000_000_000
        {
            return Err(format!(
                "`{f}` composes to above the rated capacitance at its highest point ({most} per mille)"
            ));
        }
        Ok(DcBiasCurve {
            points,
            at: self.at(c.at, f)?,
            tolerance_permille: tol,
            temperature_permille: temp,
        })
    }

    /// The control rules of a row, which name exactly the kind's control
    /// pins (`roles`, sorted), each once (adversarial F7).
    fn controls(
        &mut self,
        raw: Vec<RawControl>,
        roles: &[&str],
        field: &str,
    ) -> Result<Vec<ControlRule>, String> {
        let mut named: Vec<&str> = raw.iter().map(|c| c.pin.as_str()).collect();
        named.sort_unstable();
        if named != roles {
            return Err(format!(
                "`{field}` names {named:?}, and the kind's control pins are exactly {roles:?}"
            ));
        }
        let mut out: Vec<ControlRule> = Vec::with_capacity(raw.len());
        for c in raw {
            let f = format!("{field}.{}", c.pin);
            let mut straps = c.straps;
            straps.sort();
            if straps.windows(2).any(|w| w[0] == w[1]) {
                return Err(format!("`{f}.straps` names a strap twice"));
            }
            if straps.is_empty() && !c.wirable {
                return Err(format!("`{f}` allows neither a strap nor a wire"));
            }
            out.push(ControlRule {
                at: self.at(c.at, &f)?,
                pin: c.pin,
                straps,
                wirable: c.wirable,
            });
        }
        Ok(out)
    }

    fn theta(&mut self, t: RawTheta) -> Result<ThetaCurve, String> {
        let f = "kind.buck.theta_ja";
        text(&t.package, &format!("{f}.package"), 8)?;
        text(&t.condition, &format!("{f}.condition"), 200)?;
        if t.board_copper_layers == 0 || !t.board_copper_layers.is_multiple_of(2) {
            return Err(format!(
                "`{f}.board_copper_layers` is {}, not an even count above zero",
                t.board_copper_layers
            ));
        }
        if t.points.is_empty() {
            return Err(format!("`{f}.points` is empty"));
        }
        let mut points: Vec<(i64, i64)> = Vec::with_capacity(t.points.len());
        for q in t.points {
            bounded(q.square_centimetres, f)?;
            bounded(q.decikelvin_per_watt, f)?;
            if q.square_centimetres <= 0 || q.decikelvin_per_watt <= 0 {
                return Err(format!("`{f}.points` holds a value not above zero"));
            }
            // The curve falls with area: the lookup's rounding is safe only then.
            if points
                .last()
                .is_some_and(|(a, th)| *a >= q.square_centimetres || *th < q.decikelvin_per_watt)
            {
                return Err(format!(
                    "`{f}.points` is not area rising with θ falling or level"
                ));
            }
            points.push((q.square_centimetres, q.decikelvin_per_watt));
        }
        Ok(ThetaCurve {
            package: t.package,
            board_copper_layers: t.board_copper_layers,
            condition: t.condition,
            points,
            at: self.at(t.at, f)?,
        })
    }
}

fn bounded(v: i64, field: &str) -> Result<(), String> {
    if !(-MAX_VALUE..=MAX_VALUE).contains(&v) {
        return Err(format!("`{field}` holds {v}, beyond ±{MAX_VALUE}"));
    }
    Ok(())
}

/// Printable ASCII, 1 to `max` bytes.
fn text(value: &str, field: &str, max: usize) -> Result<(), String> {
    if value.is_empty() || value.len() > max || !value.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return Err(format!(
            "`{field}` is not 1 to {max} bytes of printable ASCII"
        ));
    }
    Ok(())
}

/// The MCU's support network, refused where a group is empty, unknown or
/// shares a pin, a pin is not a package position, a role repeats, a value or
/// tolerance is out of range, a strap is not one pin, or a text is not.
fn support(raw: RawSupport, r: &mut Reader<'_>) -> Result<Support, String> {
    const F: &str = "kind.mcu.support";
    let mut seen = BTreeSet::new();
    for (group, pins) in &raw.pins {
        text(group, &format!("{F}.pins key"), 16)?;
        if pins.is_empty() {
            return Err(format!("`{F}.pins.{group}` is empty"));
        }
        let mut last = 0;
        for p in pins {
            let n = p
                .parse::<u16>()
                .ok()
                .filter(|n| p == &n.to_string() && *n > 0);
            let Some(n) = n else {
                return Err(format!(
                    "`{F}.pins.{group}` holds `{p}`, not a package position"
                ));
            };
            if n <= last {
                return Err(format!("`{F}.pins.{group}` is not ascending"));
            }
            last = n;
            if !seen.insert(n) {
                return Err(format!("`{F}.pins` names pin {p} in two groups"));
            }
        }
    }
    let group = |name: &str, field: &str| {
        if raw.pins.contains_key(name) {
            Ok(name.to_owned())
        } else {
            Err(format!(
                "`{F}.{field}` names `{name}`, which `pins` does not"
            ))
        }
    };
    let supply = group(&raw.supply, "supply")?;
    let grounds = raw
        .grounds
        .iter()
        .map(|g| group(g, "grounds"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut powered = Vec::with_capacity(raw.powered.len());
    for p in raw.powered {
        let g = group(&p.group, "powered")?;
        if p.min_millivolts < 0 || p.min_millivolts > p.max_millivolts {
            return Err(format!("`{F}.powered.{g}` is not a range of 0 mV or more"));
        }
        bounded(p.max_millivolts, &format!("{F}.powered.{g}"))?;
        powered.push(Powered {
            millivolts: Range {
                min: p.min_millivolts,
                max: p.max_millivolts,
            },
            at: r.at(p.at, &format!("{F}.powered.{g}"))?,
            group: g,
        });
    }
    let mut own_nets = Vec::with_capacity(raw.own_nets.len());
    for o in raw.own_nets {
        let g = group(&o.group, "own_nets")?;
        own_nets.push(OwnNet {
            at: r.at(o.at, &format!("{F}.own_nets.{g}"))?,
            group: g,
        });
    }
    if grounds.is_empty() {
        return Err(format!("`{F}.grounds` is empty"));
    }
    let mut capacitors: Vec<SupportCapacitor> = Vec::with_capacity(raw.capacitors.len());
    for c in raw.capacitors {
        let f = format!("{F}.capacitors.{}", c.role.name());
        if capacitors.iter().any(|o| o.role == c.role) {
            return Err(format!(
                "`{F}.capacitors` names the role `{}` twice",
                c.role.name()
            ));
        }
        let owner = group(&c.owner, &format!("capacitors.{}.owner", c.role.name()))?;
        let ground = group(&c.ground, &format!("capacitors.{}.ground", c.role.name()))?;
        if !grounds.contains(&ground) {
            return Err(format!("`{f}.ground` is `{ground}`, not one of `grounds`"));
        }
        bounded(c.picofarads, &f)?;
        if c.picofarads <= 0 || !(0..=500).contains(&c.tolerance_permille) {
            return Err(format!(
                "`{f}` needs picofarads above zero and a tolerance of 0 to 500 permille"
            ));
        }
        if c.bound_um.is_some_and(|b| !(1..=100_000).contains(&b)) {
            return Err(format!("`{f}.bound_um` is not 1 to 100000"));
        }
        text(&c.bound_basis, &format!("{f}.bound_basis"), 256)?;
        capacitors.push(SupportCapacitor {
            role: c.role,
            owner,
            per_pin: c.per_pin,
            ground,
            picofarads: c.picofarads,
            tolerance_permille: c.tolerance_permille,
            bound_um: c.bound_um,
            bound_basis: c.bound_basis,
            at: r.at(c.at, &f)?,
        });
    }
    let mut straps: Vec<SupportStrap> = Vec::with_capacity(raw.straps.len());
    for s in raw.straps {
        let f = format!("{F}.straps.{}", s.pin);
        let pin = group(&s.pin, "straps.pin")?;
        if raw.pins[&pin].len() != 1 {
            return Err(format!("`{f}` names a group of more than one pin"));
        }
        if straps.iter().any(|o| o.pin == pin) {
            return Err(format!("`{F}.straps` names `{pin}` twice"));
        }
        let to = group(&s.to, &format!("straps.{pin}.to"))?;
        straps.push(SupportStrap {
            pin,
            to,
            at: r.at(s.at, &f)?,
        });
    }
    Ok(Support {
        pins: raw.pins,
        pins_at: r.at(raw.pins_at, &format!("{F}.pins"))?,
        supply,
        grounds,
        powered,
        own_nets,
        capacitors,
        straps,
    })
}

/// The pin roles each IC kind must name, exactly.
fn pin_roles(kind: &Kind) -> &'static [&'static str] {
    match kind {
        Kind::Buck(_) => &["BOOT", "EN", "EP", "FB", "PG", "PGND", "SW", "VCC", "VIN"],
        Kind::Ldo(_) => &["EN", "GND", "IN", "NC", "OUT"],
        Kind::Translator(_) => &["GND", "OE", "VCCA", "VCCB"],
        Kind::Resistor(_)
        | Kind::Capacitor(_)
        | Kind::Inductor(_)
        | Kind::Mcu(_)
        | Kind::BulkCapacitor(_)
        | Kind::Tvs(_)
        | Kind::Crystal(_)
        | Kind::Jumper(_) => &[],
    }
}

fn kind(raw: RawKind, r: &mut Reader<'_>) -> Result<Kind, String> {
    Ok(match raw {
        RawKind::Buck(b) => {
            let regulation = r.ranged(b.regulation_permille, "kind.buck.regulation_permille")?;
            if regulation.range.min > 0 || regulation.range.max < 0 {
                return Err("`kind.buck.regulation_permille` does not hold zero".to_owned());
            }
            text(&b.stated_at, "kind.buck.stated_at", 200)?;
            let t = b.thermal_vias;
            text(&t.pad, "kind.buck.thermal_vias.pad", 8)?;
            let vin_max = b.vin_millivolts.max;
            Kind::Buck(Box::new(Buck {
                vin_millivolts: r.ranged_positive(b.vin_millivolts, "kind.buck.vin_millivolts")?,
                vin_abs_max_millivolts: {
                    let abs =
                        r.positive(b.vin_abs_max_millivolts, "kind.buck.vin_abs_max_millivolts")?;
                    if abs.value < vin_max {
                        return Err(
                            "`kind.buck.vin_abs_max_millivolts` is below the recommended VIN maximum"
                                .to_owned(),
                        );
                    }
                    abs
                },
                input_damping_picofarads: r.ranged_positive(
                    b.input_damping_picofarads,
                    "kind.buck.input_damping_picofarads",
                )?,
                vfb_microvolts: r.ranged_positive(b.vfb_microvolts, "kind.buck.vfb_microvolts")?,
                ifb_max_nanoamps: r.limit(b.ifb_max_nanoamps, "kind.buck.ifb_max_nanoamps")?,
                regulation_permille: regulation,
                regulation_production_tested: b.regulation_production_tested,
                regulation_vin_min: {
                    if b.regulation_vin_min.is_empty() {
                        return Err("`kind.buck.regulation_vin_min` is empty".to_owned());
                    }
                    let mut steps: Vec<(i64, Limit)> = Vec::new();
                    for st in b.regulation_vin_min {
                        bounded(st.from_vout_millivolts, "kind.buck.regulation_vin_min")?;
                        if st.from_vout_millivolts < 0
                            || steps
                                .last()
                                .is_some_and(|(f, _)| *f <= st.from_vout_millivolts)
                        {
                            return Err(
                                "`kind.buck.regulation_vin_min` is not highest first, each from 0 mV or more"
                                    .to_owned(),
                            );
                        }
                        let l = r.positive(st.vin_min, "kind.buck.regulation_vin_min")?;
                        steps.push((st.from_vout_millivolts, l));
                    }
                    if steps.last().is_some_and(|(f, _)| *f != 0) {
                        return Err(
                            "`kind.buck.regulation_vin_min` has no row from 0 mV".to_owned()
                        );
                    }
                    steps
                },
                fsw_hz: r.ranged_positive(b.fsw_hz, "kind.buck.fsw_hz")?,
                ton_min_max_ns: r.positive(b.ton_min_max_ns, "kind.buck.ton_min_max_ns")?,
                toff_min_max_ns: r.positive(b.toff_min_max_ns, "kind.buck.toff_min_max_ns")?,
                rds_high_max_milliohms: r
                    .positive(b.rds_high_max_milliohms, "kind.buck.rds_high_max_milliohms")?,
                rds_low_max_milliohms: r
                    .positive(b.rds_low_max_milliohms, "kind.buck.rds_low_max_milliohms")?,
                high_side_limit_milliamps: r.ranged_positive(
                    b.high_side_limit_milliamps,
                    "kind.buck.high_side_limit_milliamps",
                )?,
                iout_max_milliamps: r
                    .positive(b.iout_max_milliamps, "kind.buck.iout_max_milliamps")?,
                rfbt_max_ohms: r.positive(b.rfbt_max_ohms, "kind.buck.rfbt_max_ohms")?,
                inductor_min_coefficient: r.positive(
                    b.inductor_min_coefficient,
                    "kind.buck.inductor_min_coefficient",
                )?,
                stated_at: b.stated_at,
                external: r.external(b.external, "kind.buck.external")?,
                theta_ja: r.theta(b.theta_ja)?,
                controls: r.controls(b.controls, &["EN", "PG"], "kind.buck.controls")?,
                thermal_vias: ThermalRule {
                    pad: t.pad,
                    columns: t.columns,
                    rows: t.rows,
                    pitch_um: t.pitch_um,
                    drill_um: t.drill_um,
                    ring_um: t.ring_um,
                    sources: crate::thermal::ThermalSources {
                        count: t.sources.count,
                        pitch: t.sources.pitch,
                        drill: t.sources.drill,
                        ring: t.sources.ring,
                        layers: t.sources.layers,
                    },
                },
            }))
        }
        RawKind::Ldo(l) => {
            text(&l.vout_condition, "kind.ldo.vout_condition", 200)?;
            text(&l.theta_ja_condition, "kind.ldo.theta_ja_condition", 200)?;
            if l.dropout_at_milliamps <= 0 {
                return Err("`kind.ldo.dropout_at_milliamps` is not above zero".to_owned());
            }
            bounded(l.dropout_at_milliamps, "kind.ldo.dropout_at_milliamps")?;
            let vin_max = l.vin_millivolts.max;
            Kind::Ldo(Box::new(Ldo {
                vin_millivolts: r.ranged_positive(l.vin_millivolts, "kind.ldo.vin_millivolts")?,
                vin_abs_max_millivolts: {
                    let abs =
                        r.positive(l.vin_abs_max_millivolts, "kind.ldo.vin_abs_max_millivolts")?;
                    if abs.value < vin_max {
                        return Err(
                            "`kind.ldo.vin_abs_max_millivolts` is below the recommended VIN maximum"
                                .to_owned(),
                        );
                    }
                    abs
                },
                vout_millivolts: r
                    .ranged_positive(l.vout_millivolts, "kind.ldo.vout_millivolts")?,
                vout_condition: l.vout_condition,
                dropout_max_millivolts: r
                    .positive(l.dropout_max_millivolts, "kind.ldo.dropout_max_millivolts")?,
                dropout_at_milliamps: l.dropout_at_milliamps,
                iout_max_milliamps: r
                    .positive(l.iout_max_milliamps, "kind.ldo.iout_max_milliamps")?,
                ignd_max_microamps: r
                    .positive(l.ignd_max_microamps, "kind.ldo.ignd_max_microamps")?,
                theta_ja_decikelvin_per_watt: r.positive(
                    l.theta_ja_decikelvin_per_watt,
                    "kind.ldo.theta_ja_decikelvin_per_watt",
                )?,
                theta_ja_condition: l.theta_ja_condition,
                junction_max_celsius: r
                    .limit(l.junction_max_celsius, "kind.ldo.junction_max_celsius")?,
                external: r.external(l.external, "kind.ldo.external")?,
                controls: r.controls(l.controls, &["EN"], "kind.ldo.controls")?,
            }))
        }
        RawKind::Translator(t) => {
            if t.channels.is_empty() {
                return Err("`kind.translator.channels` is empty".to_owned());
            }
            let mut pins = BTreeSet::new();
            let mut channels = Vec::with_capacity(t.channels.len());
            for c in t.channels {
                text(&c.input, "kind.translator.channels.input", 8)?;
                text(&c.output, "kind.translator.channels.output", 8)?;
                for p in [&c.input, &c.output] {
                    if !pins.insert(p.clone()) {
                        return Err(format!("`kind.translator.channels` names pin `{p}` twice"));
                    }
                }
                channels.push(Channel {
                    input: c.input,
                    output: c.output,
                    direction: c.direction,
                });
            }
            Kind::Translator(Box::new(Translator {
                vcca_millivolts: r
                    .ranged_positive(t.vcca_millivolts, "kind.translator.vcca_millivolts")?,
                vccb_millivolts: r
                    .ranged_positive(t.vccb_millivolts, "kind.translator.vccb_millivolts")?,
                tpd_a_to_b_max_ns: r
                    .positive(t.tpd_a_to_b_max_ns, "kind.translator.tpd_a_to_b_max_ns")?,
                tpd_b_to_a_max_ns: r
                    .positive(t.tpd_b_to_a_max_ns, "kind.translator.tpd_b_to_a_max_ns")?,
                tmax_a_to_b_bps: r
                    .positive(t.tmax_a_to_b_bps, "kind.translator.tmax_a_to_b_bps")?,
                tmax_b_to_a_bps: r
                    .positive(t.tmax_b_to_a_bps, "kind.translator.tmax_b_to_a_bps")?,
                pull_up_max_ohms: r
                    .positive(t.pull_up_max_ohms, "kind.translator.pull_up_max_ohms")?,
                channels,
                external: r.external(t.external, "kind.translator.external")?,
                controls: r.controls(t.controls, &["OE"], "kind.translator.controls")?,
            }))
        }
        RawKind::Resistor(s) => {
            text(&s.drift_condition, "kind.resistor.drift_condition", 200)?;
            bounded(
                s.tcr_reference_celsius,
                "kind.resistor.tcr_reference_celsius",
            )?;
            Kind::Resistor(Resistor {
                tolerance_ppm: r.positive(s.tolerance_ppm, "kind.resistor.tolerance_ppm")?,
                tcr_ppm_per_kelvin: r
                    .positive(s.tcr_ppm_per_kelvin, "kind.resistor.tcr_ppm_per_kelvin")?,
                tcr_reference_celsius: s.tcr_reference_celsius,
                drift_ppm: r.positive(s.drift_ppm, "kind.resistor.drift_ppm")?,
                drift_condition: s.drift_condition,
            })
        }
        RawKind::Capacitor(c) => {
            let rated = r.positive(c.rated_picofarads, "kind.capacitor.rated_picofarads")?;
            let volts = r.positive(c.rated_millivolts, "kind.capacitor.rated_millivolts")?;
            let dc_bias = match c.dc_bias {
                Some(curve) => Some(r.curve(curve, volts.value)?),
                None => None,
            };
            let tolerance = match c.tolerance {
                None => None,
                Some(RawTolerance::Permille(l)) => Some(Tolerance::Permille(
                    r.positive(l, "kind.capacitor.tolerance.permille")?,
                )),
                Some(RawTolerance::Femtofarads(l)) => Some(Tolerance::Femtofarads(
                    r.positive(l, "kind.capacitor.tolerance.femtofarads")?,
                )),
            };
            Kind::Capacitor(Capacitor {
                rated_picofarads: rated,
                rated_millivolts: volts,
                dielectric: c.dielectric,
                dc_bias,
                tolerance,
            })
        }
        RawKind::BulkCapacitor(c) => {
            text(&c.esr_condition, "kind.bulk_capacitor.esr_condition", 200)?;
            Kind::BulkCapacitor(BulkCapacitor {
                rated_picofarads: r
                    .positive(c.rated_picofarads, "kind.bulk_capacitor.rated_picofarads")?,
                rated_millivolts: r
                    .positive(c.rated_millivolts, "kind.bulk_capacitor.rated_millivolts")?,
                esr_max_milliohms: r
                    .positive(c.esr_max_milliohms, "kind.bulk_capacitor.esr_max_milliohms")?,
                esr_condition: c.esr_condition,
            })
        }
        RawKind::Crystal(c) => {
            // A deviation is symmetric about zero in every held sheet, and
            // the budget adds each side's magnitude: a range that does not
            // straddle zero is a typing error.
            let deviation = |r: &mut Reader<'_>, raw: RawRanged, field: &str| {
                let d = r.ranged(raw, field)?;
                if d.range.min > 0 || d.range.max < 0 {
                    return Err(format!("`{field}` does not straddle zero"));
                }
                Ok(d)
            };
            Kind::Crystal(Box::new(Crystal {
                frequency_hz: r.positive(c.frequency_hz, "kind.crystal.frequency_hz")?,
                load_femtofarads: r
                    .positive(c.load_femtofarads, "kind.crystal.load_femtofarads")?,
                tolerance_ppm: deviation(r, c.tolerance_ppm, "kind.crystal.tolerance_ppm")?,
                stability_ppm: deviation(r, c.stability_ppm, "kind.crystal.stability_ppm")?,
                aging_ppm: deviation(r, c.aging_ppm, "kind.crystal.aging_ppm")?,
                aging_hours: r.positive(c.aging_hours, "kind.crystal.aging_hours")?,
                esr_max_milliohms: r
                    .positive(c.esr_max_milliohms, "kind.crystal.esr_max_milliohms")?,
                c0_max_femtofarads: r
                    .positive(c.c0_max_femtofarads, "kind.crystal.c0_max_femtofarads")?,
                drive_level_max_microwatts: r.positive(
                    c.drive_level_max_microwatts,
                    "kind.crystal.drive_level_max_microwatts",
                )?,
            }))
        }
        RawKind::Tvs(t) => {
            text(&t.clamp_condition, "kind.tvs.clamp_condition", 200)?;
            let standoff = r.positive(t.standoff_millivolts, "kind.tvs.standoff_millivolts")?;
            let clamp = r.positive(t.clamp_max_millivolts, "kind.tvs.clamp_max_millivolts")?;
            if clamp.value <= standoff.value {
                return Err("`kind.tvs.clamp_max_millivolts` is not above the stand-off".to_owned());
            }
            Kind::Tvs(Tvs {
                standoff_millivolts: standoff,
                clamp_max_millivolts: clamp,
                clamp_condition: t.clamp_condition,
            })
        }
        RawKind::Inductor(i) => {
            text(&i.isat_definition, "kind.inductor.isat_definition", 200)?;
            Kind::Inductor(Inductor {
                nanohenries: r.ranged_positive(i.nanohenries, "kind.inductor.nanohenries")?,
                isat_typical_milliamps: r.positive(
                    i.isat_typical_milliamps,
                    "kind.inductor.isat_typical_milliamps",
                )?,
                isat_definition: i.isat_definition,
                dcr_max_milliohms: r
                    .positive(i.dcr_max_milliohms, "kind.inductor.dcr_max_milliohms")?,
            })
        }
        RawKind::Jumper(j) => Kind::Jumper(Jumper {
            max_milliohms: r.positive(j.max_milliohms, "kind.jumper.max_milliohms")?,
            max_milliamps: r.positive(j.max_milliamps, "kind.jumper.max_milliamps")?,
        }),
        RawKind::Mcu(m) => {
            text(&m.idd_condition, "kind.mcu.idd_condition", 200)?;
            if m.bor_falling_max_millivolts.len() != 4 {
                return Err(format!(
                    "`kind.mcu.bor_falling_max_millivolts` holds {} levels, not VBOR0 to VBOR3",
                    m.bor_falling_max_millivolts.len()
                ));
            }
            let mut bor = Vec::with_capacity(4);
            for (i, l) in m.bor_falling_max_millivolts.into_iter().enumerate() {
                let l = r.positive(l, &format!("kind.mcu.bor_falling_max_millivolts[{i}]"))?;
                if bor.last().is_some_and(|p: &Limit| p.value >= l.value) {
                    return Err(
                        "`kind.mcu.bor_falling_max_millivolts` does not rise from VBOR0 to VBOR3"
                            .to_owned(),
                    );
                }
                bor.push(l);
            }
            if m.bor_rising_max_millivolts.len() != 4 {
                return Err(format!(
                    "`kind.mcu.bor_rising_max_millivolts` holds {} levels, not VBOR0 to VBOR3",
                    m.bor_rising_max_millivolts.len()
                ));
            }
            let mut rising: Vec<Limit> = Vec::with_capacity(4);
            for (i, l) in m.bor_rising_max_millivolts.into_iter().enumerate() {
                let l = r.positive(l, &format!("kind.mcu.bor_rising_max_millivolts[{i}]"))?;
                if rising.last().is_some_and(|p| p.value >= l.value) || l.value < bor[i].value {
                    return Err(
                        "`kind.mcu.bor_rising_max_millivolts` does not rise from VBOR0 to VBOR3, each at or above its falling maximum"
                            .to_owned(),
                    );
                }
                rising.push(l);
            }
            if m.rain_max_ohms.is_empty() {
                return Err("`kind.mcu.rain_max_ohms` is empty".to_owned());
            }
            let mut rain: Vec<(u8, Limit)> = Vec::with_capacity(m.rain_max_ohms.len());
            for x in m.rain_max_ohms {
                if !(6..=16).contains(&x.bits) || rain.iter().any(|(b, _)| *b <= x.bits) {
                    return Err(format!(
                        "`kind.mcu.rain_max_ohms` holds {} bits: each from 6 to 16, highest first, once",
                        x.bits
                    ));
                }
                let l = r.positive(x.max_ohms, &format!("kind.mcu.rain_max_ohms[{}]", x.bits))?;
                rain.push((x.bits, l));
            }
            let rule = m.reservoir_rule;
            text(&rule.statement, "kind.mcu.reservoir_rule.statement", 200)?;
            if rule.extra_bits > 8 || rule.adc_capacitance_femtofarads <= 0 {
                return Err(
                    "`kind.mcu.reservoir_rule` needs extra_bits 0 to 8 and a capacitance above zero".to_owned(),
                );
            }
            bounded(
                rule.adc_capacitance_femtofarads,
                "kind.mcu.reservoir_rule.adc_capacitance_femtofarads",
            )?;
            Kind::Mcu(Box::new(Mcu {
                vdd_millivolts: r.ranged_positive(m.vdd_millivolts, "kind.mcu.vdd_millivolts")?,
                idd_max_milliamps: r.positive(m.idd_max_milliamps, "kind.mcu.idd_max_milliamps")?,
                idd_condition: m.idd_condition,
                bor_falling_max_millivolts: bor,
                bor_rising_max_millivolts: rising,
                rain_max_ohms: rain,
                reservoir_rule: ReservoirRule {
                    statement: rule.statement,
                    extra_bits: rule.extra_bits,
                    adc_capacitance_femtofarads: rule.adc_capacitance_femtofarads,
                    at: r.at(rule.at, "kind.mcu.reservoir_rule")?,
                },
                spi_sck_low_shortfall_ns: {
                    let l = r.limit(
                        m.spi_sck_low_shortfall_ns,
                        "kind.mcu.spi_sck_low_shortfall_ns",
                    )?;
                    if l.value < 0 {
                        return Err("`kind.mcu.spi_sck_low_shortfall_ns` is below zero".to_owned());
                    }
                    l
                },
                spi_input_setup_min_ns: r
                    .positive(m.spi_input_setup_min_ns, "kind.mcu.spi_input_setup_min_ns")?,
                support: support(m.support, r)?,
                flash_kilobytes: r.positive(m.flash_kilobytes, "kind.mcu.flash_kilobytes")?,
                ardupilot: mcu_ardupilot(m.ardupilot, r)?,
                io: mcu_io(m.io, r)?,
            }))
        }
    })
}

/// The MCU's I/O rows: each pin a canonical name (`P[A-K]` and 0 to 15) with
/// a structure of a held family, cited on its page; each family once, its
/// limit above zero.
fn mcu_io(m: RawMcuIo, r: &mut Reader<'_>) -> Result<McuIo, String> {
    let families = io_families(m.families, "kind.mcu.io.families", r)?;
    let a = m.absolute_maximum;
    let stress = io_families(a.families, "kind.mcu.io.absolute_maximum.families", r)?;
    // Table 10 rates every structure Table 13 does: a pin judged in
    // operation is judged at stress by the same family.
    if stress
        .iter()
        .map(|f| &f.family)
        .ne(families.iter().map(|f| &f.family))
    {
        return Err(
            "`kind.mcu.io.absolute_maximum.families` does not name `kind.mcu.io.families`' families, in order"
                .to_owned(),
        );
    }
    let absolute_maximum = IoAbsoluteMaximum {
        families: stress,
        boot0: r.positive(
            a.boot0_max_millivolts,
            "kind.mcu.io.absolute_maximum.boot0_max_millivolts",
        )?,
        other: r.positive(
            a.other_max_millivolts,
            "kind.mcu.io.absolute_maximum.other_max_millivolts",
        )?,
    };
    let base = r.at(m.pins_at, "kind.mcu.io.pins_at")?;
    let mut pins = BTreeMap::new();
    for (name, p) in m.pins {
        let canonical = name.len() <= 4
            && name.starts_with('P')
            && name
                .as_bytes()
                .get(1)
                .is_some_and(|c| (b'A'..=b'K').contains(c))
            && name[2..]
                .parse::<u8>()
                .is_ok_and(|n| n <= 15 && name[2..] == n.to_string());
        let family = p.structure.split('_').next().unwrap_or_default();
        if !canonical || !families.iter().any(|f| f.family == family) || p.page == 0 {
            return Err(format!(
                "`kind.mcu.io.pins.{name}` is not a canonical pin with a structure of a held family on a page"
            ));
        }
        let at = At {
            document: base.document.clone(),
            section: format!("p. {}, {}: `{name}` {}", p.page, base.section, p.structure),
        };
        pins.insert(name, (p.structure, at));
    }
    if pins.is_empty() {
        return Err("`kind.mcu.io.pins` is empty".to_owned());
    }
    let mut pads = BTreeSet::new();
    for pin in m.analog_pads.pins {
        if !pins.contains_key(&pin) || !pads.insert(pin.clone()) {
            return Err(format!(
                "`kind.mcu.io.analog_pads.pins` names `{pin}`, which is not a held pin of `kind.mcu.io.pins`, or names it twice"
            ));
        }
    }
    let analog_pads = AnalogPads {
        pins: pads,
        at: r.at(m.analog_pads.at, "kind.mcu.io.analog_pads.at")?,
    };
    Ok(McuIo {
        pins,
        families,
        pulls_off_above_supply: r.positive(
            m.pulls_off_above_supply_millivolts,
            "kind.mcu.io.pulls_off_above_supply_millivolts",
        )?,
        absolute_maximum,
        analog_pads,
        motor_pull_bound: motor_pull_bound(m.motor_pull_bound, r)?,
    })
}

/// `kind.mcu.io.motor_pull_bound`: every figure above zero; the thresholds
/// below 1000 permille, low below high; and the margin inside both, so that
/// `input_low_max - margin` and `1000 - (input_high_min + margin)` are each
/// above zero (`AET-WO-4300j` D3 (a)).
fn motor_pull_bound(raw: RawMotorPullBound, r: &mut Reader<'_>) -> Result<MotorPullBound, String> {
    let f = "kind.mcu.io.motor_pull_bound";
    let chosen = |c: RawChosen, field: &str| -> Result<Chosen, String> {
        let field = format!("{f}.{field}");
        bounded(c.value, &field)?;
        if c.value <= 0 {
            return Err(format!("`{field}.value` is {}, not above zero", c.value));
        }
        text(&c.basis, &format!("{field}.basis"), 256)?;
        Ok(Chosen {
            value: c.value,
            basis: c.basis,
        })
    };
    let b = MotorPullBound {
        pull_up_max_ohms: r.positive(raw.pull_up_max_ohms, &format!("{f}.pull_up_max_ohms"))?,
        input_high_min_permille: r.positive(
            raw.input_high_min_permille,
            &format!("{f}.input_high_min_permille"),
        )?,
        input_low_max_permille: r.positive(
            raw.input_low_max_permille,
            &format!("{f}.input_low_max_permille"),
        )?,
        leakage_max_nanoamps: r.positive(
            raw.leakage_max_nanoamps,
            &format!("{f}.leakage_max_nanoamps"),
        )?,
        peer_input_current_nanoamps: chosen(
            raw.peer_input_current_nanoamps,
            "peer_input_current_nanoamps",
        )?,
        margin_permille: chosen(raw.margin_permille, "margin_permille")?,
    };
    let (high, low, margin) = (
        b.input_high_min_permille.value,
        b.input_low_max_permille.value,
        b.margin_permille.value,
    );
    if high >= 1000 || low >= high || margin >= low || high + margin >= 1000 {
        return Err(format!(
            "`{f}` holds input high {high}, input low {low} and margin {margin} permille: each threshold below 1000 and low below high, and the margin leaves both above zero (low - margin > 0, 1000 - (high + margin) > 0)"
        ));
    }
    Ok(b)
}

/// Each family once, upper case, with at least one term, each above zero.
fn io_families(
    raw: Vec<RawIoFamily>,
    path: &str,
    r: &mut Reader<'_>,
) -> Result<Vec<IoFamily>, String> {
    let mut families: Vec<IoFamily> = Vec::with_capacity(raw.len());
    for f in raw {
        let at = format!("{path}.{}", f.family);
        if f.family.is_empty()
            || !f.family.bytes().all(|b| b.is_ascii_uppercase())
            || families.iter().any(|g| g.family == f.family)
        {
            return Err(format!("`{at}` is not an upper-case family named once"));
        }
        let terms = [f.above_supply_millivolts, f.max_millivolts];
        if terms.iter().all(Option::is_none) || terms.iter().flatten().any(|v| *v <= 0) {
            return Err(format!("`{at}` states neither term, or one not above zero"));
        }
        families.push(IoFamily {
            above_supply_mv: f.above_supply_millivolts,
            max_mv: f.max_millivolts,
            at: r.at(f.at, &format!("{at}.at"))?,
            family: f.family,
        });
    }
    Ok(families)
}

fn row(raw: RawRow, r: &mut Reader<'_>) -> Result<Row, String> {
    text(&raw.symbol, "symbol", 128)?;
    text(&raw.footprint, "footprint", 128)?;
    for (field, v) in [("symbol", &raw.symbol), ("footprint", &raw.footprint)] {
        let ok = v
            .split_once(':')
            .is_some_and(|(lib, name)| !lib.is_empty() && !name.is_empty() && !name.contains(':'));
        if !ok {
            return Err(format!("`{field}` is `{v}`, not `library:name`"));
        }
    }
    let temperature = r.ranged(raw.temperature_celsius, "temperature_celsius")?;
    let kind = kind(raw.kind, r)?;
    let roles = pin_roles(&kind);
    let named: Vec<&str> = raw.pins.keys().map(String::as_str).collect();
    if named != roles {
        return Err(format!(
            "`pins` names {named:?}, and a {} row names exactly {roles:?}",
            kind.name()
        ));
    }
    for (role, pin) in &raw.pins {
        text(pin, &format!("pins.{role}"), 8)?;
    }
    if let Kind::Buck(b) = &kind
        && !raw.part.ends_with(&b.theta_ja.package)
    {
        return Err(format!(
            "`kind.buck.theta_ja.package` is `{}`, and the part is not in that package",
            b.theta_ja.package
        ));
    }
    if let Kind::Buck(b) = &kind
        && !raw.pins.contains_key(&b.thermal_vias.pad)
    {
        return Err(format!(
            "`kind.buck.thermal_vias.pad` names `{}`, which is not a pin role",
            b.thermal_vias.pad
        ));
    }
    Ok(Row {
        part: raw.part,
        lifecycle: raw.lifecycle,
        symbol: raw.symbol,
        footprint: raw.footprint,
        pins: raw.pins,
        temperature_celsius: temperature,
        kind,
    })
}

fn refused(part: Option<&str>, reason: impl Into<String>) -> IrValidationError {
    IrValidationError::PowerPartsTable {
        part: part.map(str::to_owned),
        reason: reason.into(),
    }
}

/// Parses a power parts table, refusing the first malformed row by naming its
/// part and the field: an unknown key, a field of the wrong kind or out of
/// range, a min above its max, a value citing a document the table does not
/// hold, a document no value cites, a document neither or both held and
/// pending, the wrong pin roles for the row's kind, a key given twice in a
/// row, a part named twice in any case, or an empty table. It never panics.
pub fn parse_power_parts(json: &str) -> Result<PowerPartsTable, IrValidationError> {
    let raw: RawTable = serde_json::from_str(json).map_err(|e| refused(None, e.to_string()))?;
    if raw.schema_version != POWER_PARTS_SCHEMA_VERSION {
        return Err(refused(
            None,
            format!(
                "schema_version is {}, not {POWER_PARTS_SCHEMA_VERSION}",
                raw.schema_version
            ),
        ));
    }
    if raw.parts.is_empty() {
        return Err(refused(None, "`parts` is empty"));
    }
    let mut documents = BTreeMap::new();
    for (key, d) in raw.documents {
        crate::check_identifier(crate::IdentifierKind::Plain, &key).map_err(|rule| {
            refused(
                None,
                format!("document key `{key}` is not an identifier: {rule}"),
            )
        })?;
        let c = crate::parts::citation(
            crate::parts::RawCitation::new(
                d.title,
                d.revision,
                "-".to_owned(),
                d.sha256,
                d.pending,
            ),
            &format!("documents.{key}"),
        )
        .map_err(|e| refused(None, e))?;
        documents.insert(
            key,
            Document {
                title: c.document,
                revision: c.revision,
                held: c.held,
            },
        );
    }
    let mut reader = Reader {
        documents: &documents,
        cited: BTreeSet::new(),
    };
    let mut rows: Vec<Row> = Vec::with_capacity(raw.parts.len());
    for (i, text) in raw.parts.iter().enumerate() {
        let entry: serde_json::Value =
            serde_json::from_str(text.get()).map_err(|e| refused(None, e.to_string()))?;
        let name = entry
            .get("part")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| refused(None, format!("entry {i} has no string `part`")))?;
        let named = |reason: String| refused(Some(&name), reason);
        crate::check_identifier(crate::IdentifierKind::Plain, &name)
            .map_err(|rule| named(format!("`part` is not an identifier: {rule}")))?;
        let one_kind = entry
            .get("kind")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|k| k.len() == 1);
        if !one_kind {
            return Err(named(
                "`kind` is not an object holding exactly one kind".to_owned(),
            ));
        }
        let raw: RawRow = serde_json::from_str(text.get()).map_err(|e| named(e.to_string()))?;
        let r = row(raw, &mut reader).map_err(named)?;
        if rows.iter().any(|q| q.part.eq_ignore_ascii_case(&r.part)) {
            return Err(named(
                "the part is in the table twice, in some case".to_owned(),
            ));
        }
        rows.push(r);
    }
    if let Some(unused) = documents.keys().find(|k| !reader.cited.contains(*k)) {
        return Err(refused(
            None,
            format!("document `{unused}` is cited by no value"),
        ));
    }
    if rows
        .iter()
        .filter(|r| matches!(r.kind, Kind::Mcu(_)))
        .count()
        > 1
    {
        return Err(refused(None, "more than one `mcu` row"));
    }
    rows.sort_by(|a, b| a.part.cmp(&b.part));
    Ok(PowerPartsTable { documents, rows })
}

static TABLE: LazyLock<Result<PowerPartsTable, IrValidationError>> =
    LazyLock::new(|| parse_power_parts(POWER_PARTS_JSON));

/// The embedded table, parsed once. A malformed table is an error here, which
/// every caller returns, never a panic.
pub fn table() -> Result<&'static PowerPartsTable, IrValidationError> {
    TABLE.as_ref().map_err(Clone::clone)
}
