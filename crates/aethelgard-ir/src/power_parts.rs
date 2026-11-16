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

/// The `schema_version` the parser reads.
pub const POWER_PARTS_SCHEMA_VERSION: u32 = 2;

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
        | Kind::Tvs(_) => &[],
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
            Kind::Capacitor(Capacitor {
                rated_picofarads: rated,
                rated_millivolts: volts,
                dielectric: c.dielectric,
                dc_bias,
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
            }))
        }
    })
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
