//! Fault scenarios and the campaign's run rules (`AET-WO-2300`).
//!
//! A scenario is a strict JSON file, authored apart from anything generated
//! (`INV-07`): a name, the specification it starts from, what it expects, and
//! closed-world overrides. [`Scenario::parse`] reads it; the runner, in the
//! CLI, applies the overrides to the specification, because nothing in this
//! crate reads a `Specification` (`RUL-16`, `AET-WO-2203`). [`check_run`] and
//! [`check_archived`] are what `replay` judges a campaign run by, from its
//! header and records alone.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

use crate::event::{Header, name_ok};
use crate::power::{self, Band};
use crate::record::{BusLine, Droop, Line, Override, Reading, Record};
use crate::replay::State;

/// The scenario format version. 2 since `AET-WO-2301`, which admits the
/// `droop` override and `expect_counts`; a version 1 file parses as before
/// and may use neither.
pub const SCENARIO_VERSION: u32 = 2;
/// The versions read: 1 (static scenarios, unchanged) and 2.
pub const SCENARIO_VERSIONS: [u32; 2] = [1, 2];
/// At most this many overrides, so the scenario record stays inside the
/// reader's 4,096-byte event bound.
pub const MAX_OVERRIDES: usize = 32;
/// At most this many expected finding kinds, for the same reason.
pub const MAX_EXPECTED: usize = 16;
/// Why a held-low line faults its bus, as the fault record's `from` names it.
pub const HELD_LOW: &str = "scenario.line_held_low";
/// The finding a held-low line fails its run with.
pub const HELD_LOW_FINDING: &str = "i2c_line_held_low";
/// The findings the analog pre-pass fails a run with, one per line.
pub const PAST_CEILING_FINDING: &str = "analog_rise_exceeds_ceiling";
/// One per reset a droop rendered (`info`): the reset itself is the
/// supervisor doing its job, so it never fails a run.
pub const RESET_FINDING: &str = "brownout_reset";
/// One per reset the firmware did not recover from within
/// [`power::RECOVERY_BOUND_NS`] of its release, or that was never released.
pub const UNRECOVERED_FINDING: &str = "brownout_unrecovered";
/// A droop whose rail left the domain's declared tolerance (`info`).
pub const BELOW_TOLERANCE_FINDING: &str = "rail_below_tolerance";
pub const STUCK_HIGH_FINDING: &str = "analog_line_stuck_high";

/// A scenario refused, with the kind `aethelgard` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioError {
    pub kind: &'static str,
    pub id: Option<String>,
    pub message: String,
}

impl ScenarioError {
    pub fn new(kind: &'static str, id: Option<&str>, message: String) -> Self {
        Self {
            kind,
            id: id.map(str::to_owned),
            message,
        }
    }
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Expect {
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LineName {
    Sda,
    Scl,
}

/// Present, even as `null`, is `Some`; so a key a variant does not use is
/// refused whatever its value.
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<serde_json::Value>, D::Error> {
    serde_json::Value::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawOverride {
    #[serde(default, deserialize_with = "present")]
    bus: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    net: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    pull_up_ohms: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    max_capacitance_femtofarads: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    line_held_low: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    droop: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawDroop {
    domain: String,
    floor_millivolts: u64,
    start_ns: u64,
    fall_ns: u64,
    hold_ns: u64,
    rise_ns: u64,
    source_milliohms: u64,
    load_milliamps: u64,
    bor_level: u64,
}

/// `expect_counts`: an object of finding kinds to counts, in the file's order;
/// a duplicate key is refused rather than one silently winning.
fn counts<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<(String, u64)>>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<Vec<(String, u64)>>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an object of finding kinds to counts")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out: Vec<(String, u64)> = Vec::new();
            while let Some((k, n)) = map.next_entry::<String, u64>()? {
                if out.iter().any(|(seen, _)| *seen == k) {
                    return Err(serde::de::Error::custom(format!(
                        "expect_counts names `{k}` twice"
                    )));
                }
                out.push((k, n));
            }
            Ok(Some(out))
        }
    }
    d.deserialize_map(V)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawScenario {
    schema_version: u32,
    name: String,
    base: String,
    expect: Expect,
    expect_findings: Vec<String>,
    #[serde(default, deserialize_with = "counts")]
    expect_counts: Option<Vec<(String, u64)>>,
    overrides: Vec<RawOverride>,
}

/// The derives above are `remote = "Self"`; these wrap them so each type
/// deserializes only from an object, never from a positional array, as
/// `aethelgard-ir`'s types do.
macro_rules! object_only {
    ($($ty:ident),*) => {$(
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;
                impl<'de> Visitor<'de> for ObjectVisitor {
                    type Value = $ty;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " object"))
                    }
                    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<$ty, A::Error> {
                        $ty::deserialize(MapAccessDeserializer::new(map))
                    }
                }
                deserializer.deserialize_map(ObjectVisitor)
            }
        }
    )*};
}

object_only!(RawOverride, RawScenario, RawDroop);

/// One scenario file, parsed and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub name: String,
    /// The specification it starts from, relative to the scenario file's
    /// directory; the caller resolves and bounds it.
    pub base: String,
    pub expect_pass: bool,
    /// Sorted and unique; empty exactly when `expect_pass`.
    pub expect_findings: Vec<String>,
    /// In the file's order.
    pub overrides: Vec<Override>,
    /// Exact counts of finding kinds, any severity, sorted by kind; empty
    /// when the file declares none (`AET-WO-2300` F11, `AET-WO-2301` §2.3).
    pub expect_counts: Vec<(String, u64)>,
    /// The file's sha256.
    pub sha256: [u8; 32],
}

fn schema(message: String) -> ScenarioError {
    ScenarioError::new("SchemaViolation", None, message)
}

fn override_from(raw: RawOverride) -> Result<Override, ScenarioError> {
    use serde_json::Value as J;
    let text = |v: J, key: &str| match v {
        J::String(s) if name_ok(&s) => Ok(s),
        other => Err(schema(format!(
            "override `{key}` is not 1 to 64 printable ASCII bytes: {other}"
        ))),
    };
    let int = |v: J, key: &str, max: u64| match v.as_u64() {
        Some(n) if n <= max && v.is_u64() => Ok(n),
        _ => Err(schema(format!(
            "override `{key}` is not an integer from 0 to {max}: {v}"
        ))),
    };
    let RawOverride {
        bus,
        net,
        pull_up_ohms,
        max_capacitance_femtofarads,
        line_held_low,
        droop,
    } = raw;
    if let Some(d) = droop {
        if bus.is_some()
            || net.is_some()
            || pull_up_ohms.is_some()
            || max_capacitance_femtofarads.is_some()
            || line_held_low.is_some()
        {
            return Err(schema(
                "a droop override is exactly {droop: {...}}".to_owned(),
            ));
        }
        return droop_from(d).map(Override::Droop);
    }
    match (bus, net, pull_up_ohms, max_capacitance_femtofarads, line_held_low) {
        (Some(bus), None, Some(ohms), None, None) => Ok(Override::PullUpOhms {
            bus: text(bus, "bus")?,
            ohms: u32::try_from(int(ohms, "pull_up_ohms", u32::MAX.into())?)
                .expect("bounded by u32::MAX"),
        }),
        (None, Some(net), None, Some(ff), None) => Ok(Override::MaxCapacitance {
            net: text(net, "net")?,
            femtofarads: int(ff, "max_capacitance_femtofarads", u64::MAX)?,
        }),
        (Some(bus), None, None, None, Some(line)) => Ok(Override::LineHeldLow {
            bus: text(bus, "bus")?,
            line: match serde_json::from_value::<LineName>(line.clone()) {
                Ok(LineName::Sda) => Line::Sda,
                Ok(LineName::Scl) => Line::Scl,
                Err(_) => {
                    return Err(schema(format!(
                        "override `line_held_low` is not \"sda\" or \"scl\": {line}"
                    )));
                }
            },
        }),
        _ => Err(schema(
            "an override is exactly {bus, pull_up_ohms}, {net, max_capacitance_femtofarads}, {bus, line_held_low} or {droop}"
                .to_owned(),
        )),
    }
}

/// The bounds of a droop's integers (`AET-WO-2301` §5.4), each checked on
/// its own before any arithmetic: the run is 100 ms, a source is at most
/// 1 kΩ, a load at most 10 A.
const DROOP_BOUNDS: [(&str, u64, u64); 8] = [
    ("floor_millivolts", 1, 5_000),
    ("start_ns", 1, 100_000_000),
    ("fall_ns", 1, 10_000_000),
    ("hold_ns", 1, 100_000_000),
    ("rise_ns", 1, 10_000_000),
    ("source_milliohms", 1, 1_000_000),
    ("load_milliamps", 0, 10_000),
    ("bor_level", 0, power::MAX_BOR_LEVEL as u64),
];

fn droop_from(v: serde_json::Value) -> Result<Droop, ScenarioError> {
    let raw: RawDroop = serde_json::from_value(v).map_err(|e| schema(format!("droop: {e}")))?;
    if !name_ok(&raw.domain) {
        return Err(schema(format!(
            "droop domain `{}` is not 1 to 64 printable ASCII bytes",
            raw.domain.escape_debug()
        )));
    }
    let values = [
        raw.floor_millivolts,
        raw.start_ns,
        raw.fall_ns,
        raw.hold_ns,
        raw.rise_ns,
        raw.source_milliohms,
        raw.load_milliamps,
        raw.bor_level,
    ];
    for ((key, lo, hi), v) in DROOP_BOUNDS.iter().zip(values) {
        if !(*lo..=*hi).contains(&v) {
            return Err(schema(format!("droop `{key}` is not from {lo} to {hi}: {v}")));
        }
    }
    let narrow = |v: u64| u32::try_from(v).expect("bounded above");
    Ok(Droop {
        domain: raw.domain,
        floor_millivolts: narrow(raw.floor_millivolts),
        start_ns: raw.start_ns,
        fall_ns: raw.fall_ns,
        hold_ns: raw.hold_ns,
        rise_ns: raw.rise_ns,
        source_milliohms: narrow(raw.source_milliohms),
        load_milliamps: narrow(raw.load_milliamps),
        bor_level: u8::try_from(raw.bor_level).expect("bounded above"),
    })
}

/// The rail's time constant R·C in ns, rounded up.
pub fn tau_ns(source_milliohms: u32, decoupling_picofarads: u64) -> u64 {
    // mΩ · pF = 1e-15 s = 1e-6 ns.
    (u128::from(source_milliohms) * u128::from(decoupling_picofarads))
        .div_ceil(1_000_000)
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The rail's lowest and resting levels, in µV: the floor and the nominal,
/// each less the load's I·R drop. `None` when the drop reaches the floor.
pub fn rail_levels(droop: &Droop, nominal_millivolts: u32) -> Option<(u64, u64)> {
    let drop = droop.drop_microvolts();
    let floor = u64::from(droop.floor_millivolts) * 1000;
    let rest = u64::from(nominal_millivolts) * 1000;
    Some((floor.checked_sub(drop)?, rest.checked_sub(drop)?))
}

/// The time after which nothing a droop causes can still happen: its sag
/// ends, the rail settles (10 τ), BOR0's temporization runs, the release's
/// nudge, the recovery bound, and the grid's rounding on both edges.
pub fn droop_end_ns(droop: &Droop, decoupling_picofarads: u64) -> u64 {
    let settle = tau_ns(droop.source_milliohms, decoupling_picofarads).saturating_mul(10);
    [
        droop.start_ns,
        droop.fall_ns,
        droop.hold_ns,
        droop.rise_ns,
        settle,
        power::T_RSTTEMPO_NS,
        power::NUDGE_NS,
        power::RECOVERY_BOUND_NS,
        2 * power::RESOLUTION_NS,
    ]
    .into_iter()
    .fold(0u64, u64::saturating_add)
}

/// A droop the runner can render, once it knows the domain's nominal and the
/// decoupling on it (ruled D3, D7): refused when its rail floor, or its
/// resting level for a threshold it crosses, lies inside that threshold's
/// min–max band, because the part's behaviour there is not stated; when the
/// resting rail is not above every active threshold, so the part would not
/// run at all; when the floor is held less than 10 τ, so the rail never
/// reaches it; and when its effects run past the boot's `span_ns`.
pub fn check_droop(
    droop: &Droop,
    nominal_millivolts: u32,
    decoupling_picofarads: u64,
    span_ns: u64,
) -> Result<(), ScenarioError> {
    let refuse = |m: String| ScenarioError::new("DroopRefused", Some(&droop.domain), m);
    if droop.floor_millivolts >= nominal_millivolts {
        return Err(refuse(format!(
            "the floor {} mV is not below the domain's nominal {nominal_millivolts} mV",
            droop.floor_millivolts
        )));
    }
    if decoupling_picofarads == 0 {
        return Err(refuse("the domain has no decoupling".to_owned()));
    }
    let (floor, rest) = rail_levels(droop, nominal_millivolts)
        .ok_or_else(|| refuse("the load's I·R drop reaches the floor".to_owned()))?;
    let tau = tau_ns(droop.source_milliohms, decoupling_picofarads);
    if droop.hold_ns < tau.saturating_mul(10) {
        return Err(refuse(format!(
            "the floor is held {} ns, less than 10 τ ({} ns), so the rail does not reach it",
            droop.hold_ns,
            tau.saturating_mul(10)
        )));
    }
    for t in power::active(droop.bor_level) {
        if power::band(rest, t.falling_uv) != Band::Above {
            return Err(refuse(format!(
                "the resting rail, {rest} µV, is not above {}'s falling band",
                t.name
            )));
        }
        match power::band(floor, t.falling_uv) {
            Band::Inside => {
                return Err(refuse(format!(
                    "the rail floor, {floor} µV, lies inside {}'s falling band {:?} µV, where the part's behaviour is not stated",
                    t.name,
                    [t.falling_uv[0], t.falling_uv[2]]
                )));
            }
            Band::Below if power::band(rest, t.rising_uv) != Band::Above => {
                return Err(refuse(format!(
                    "the resting rail, {rest} µV, is not above {}'s rising band, so the release is not stated",
                    t.name
                )));
            }
            _ => {}
        }
    }
    let end = droop_end_ns(droop, decoupling_picofarads);
    if end > span_ns {
        return Err(refuse(format!(
            "the droop's effects run to {end} ns, past the boot's {span_ns} ns"
        )));
    }
    Ok(())
}

/// What an override sets, so two overrides of one thing are refused.
fn target(o: &Override) -> (u8, &str) {
    match o {
        Override::PullUpOhms { bus, .. } => (0, bus),
        Override::MaxCapacitance { net, .. } => (1, net),
        Override::LineHeldLow { bus, line } => (
            match line {
                Line::Sda => 2,
                Line::Scl => 3,
            },
            bus,
        ),
        // One droop per scenario, whatever its domain.
        Override::Droop(_) => (4, ""),
    }
}

impl Scenario {
    /// Parse one scenario file's bytes. Unknown keys, a positional array, a
    /// duplicate key, a wrong type, an unknown version, a name outside
    /// `RUL-14`'s set, an expectation that contradicts itself and two
    /// overrides of one thing are each refused.
    pub fn parse(bytes: &[u8]) -> Result<Self, ScenarioError> {
        let raw: RawScenario = serde_json::from_slice(bytes).map_err(|e| {
            if e.classify() == serde_json::error::Category::Data {
                schema(e.to_string())
            } else {
                ScenarioError::new("InvalidJson", None, e.to_string())
            }
        })?;
        if !SCENARIO_VERSIONS.contains(&raw.schema_version) {
            return Err(ScenarioError::new(
                "UnsupportedSchemaVersion",
                None,
                format!(
                    "scenario schema_version {} is not one of {SCENARIO_VERSIONS:?}",
                    raw.schema_version
                ),
            ));
        }
        if !name_ok(&raw.name) {
            return Err(schema(format!(
                "scenario name `{}` is not 1 to 64 printable ASCII bytes",
                raw.name.escape_debug()
            )));
        }
        if raw.base.is_empty() {
            return Err(schema("scenario base is empty".to_owned()));
        }
        if raw.overrides.len() > MAX_OVERRIDES {
            return Err(schema(format!(
                "a scenario has at most {MAX_OVERRIDES} overrides"
            )));
        }
        if raw.expect_findings.len() > MAX_EXPECTED {
            return Err(schema(format!(
                "a scenario expects at most {MAX_EXPECTED} finding kinds"
            )));
        }
        let mut expect_findings = raw.expect_findings;
        if let Some(bad) = expect_findings.iter().find(|k| !name_ok(k)) {
            return Err(schema(format!(
                "expected finding kind `{}` is not 1 to 64 printable ASCII bytes",
                bad.escape_debug()
            )));
        }
        let given = expect_findings.len();
        expect_findings.sort();
        expect_findings.dedup();
        if expect_findings.len() != given {
            return Err(schema(
                "an expected finding kind is listed twice".to_owned(),
            ));
        }
        let expect_pass = raw.expect == Expect::Pass;
        if expect_pass != expect_findings.is_empty() {
            return Err(ScenarioError::new(
                "ExpectContradiction",
                Some(&raw.name),
                "`expect: pass` takes no expect_findings, and `expect: fail` names at least one"
                    .to_owned(),
            ));
        }
        let overrides = raw
            .overrides
            .into_iter()
            .map(override_from)
            .collect::<Result<Vec<_>, _>>()?;
        let mut expect_counts = raw.expect_counts.unwrap_or_default();
        let v2_only = overrides.iter().any(|o| matches!(o, Override::Droop(_)))
            || !expect_counts.is_empty();
        if v2_only && raw.schema_version < 2 {
            return Err(schema(
                "a droop override and expect_counts need schema_version 2".to_owned(),
            ));
        }
        if expect_counts.len() > MAX_EXPECTED {
            return Err(schema(format!(
                "a scenario expects at most {MAX_EXPECTED} counts"
            )));
        }
        if let Some((bad, _)) = expect_counts.iter().find(|(k, _)| !name_ok(k)) {
            return Err(schema(format!(
                "expect_counts kind `{}` is not 1 to 64 printable ASCII bytes",
                bad.escape_debug()
            )));
        }
        expect_counts.sort();
        let mut seen = BTreeSet::new();
        for o in &overrides {
            if !seen.insert(target(o)) {
                return Err(ScenarioError::new(
                    "DuplicateOverride",
                    Some(target(o).1),
                    format!("two overrides set the same thing: {o:?}"),
                ));
            }
        }
        Ok(Self {
            name: raw.name,
            base: raw.base,
            expect_pass,
            expect_findings,
            overrides,
            expect_counts,
            sha256: Sha256::digest(bytes).into(),
        })
    }

    /// The scenario's droop, if it declares one.
    pub fn droop(&self) -> Option<&Droop> {
        self.overrides.iter().find_map(|o| match o {
            Override::Droop(d) => Some(d),
            _ => None,
        })
    }

    /// Kind 20, when the scenario declares `expect_counts`.
    pub fn counts_record(&self) -> Option<Record> {
        (!self.expect_counts.is_empty()).then(|| Record::ExpectedCounts {
            counts: self.expect_counts.clone(),
        })
    }

    /// The scenario's record, the first of its run.
    pub fn record(&self) -> Record {
        Record::Scenario {
            name: self.name.clone(),
            sha256: self.sha256,
            expect_pass: self.expect_pass,
            expect_findings: self.expect_findings.clone(),
            overrides: self.overrides.clone(),
        }
    }
}

/// A run's id: a function of what it judged, so a second campaign over the
/// same inputs names the same run (and an archive refuses it as existing).
pub fn run_id(scenario: &[u8; 32], specification: &[u8; 32], firmware: &[u8; 32]) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"aethelgard-run\0");
    h.update(scenario);
    h.update(specification);
    h.update(firmware);
    let digest: [u8; 32] = h.finalize().into();
    digest[..16].try_into().expect("16 of 32 bytes")
}

/// What `replay` reports of a campaign run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSummary {
    pub name: String,
    pub expect_pass: bool,
    pub expect_findings: Vec<String>,
    pub pass: bool,
    /// The verdict's finding kinds above `info`, sorted and unique.
    pub failing_kinds: Vec<String>,
    /// The verdict is the one expected, for the reasons expected.
    pub matched: bool,
}

/// Judge a campaign run from its records alone: their order, and that every
/// fault, measurement and verdict agrees with the evidence beside it. A run
/// that fails any of these is refused with the reason.
///
/// The order is: the scenario, its expected counts (kind 20, if any), the
/// solved topology, the solved I2C buses (kind 15, sorted by bus), the analog
/// records (kinds 7 and 8), the bus faults, the declared rail (kind 16), the
/// droop's crossings, resets and observations (kinds 17 to 19) in time
/// order, the boot's witnesses (kind 10, then 11, 12 and 13 in that order),
/// and the verdict, last and once.
pub fn check_run(records: &[Record]) -> Result<RunSummary, &'static str> {
    fn rank(r: &Record) -> u8 {
        match r {
            Record::Scenario { .. } => 0,
            Record::ExpectedCounts { .. } => 1,
            Record::Solved { .. } => 2,
            Record::I2cBusSolved { .. } => 3,
            Record::RiseMeasured { .. } | Record::LineStuckHigh { .. } => 4,
            Record::BusFault { .. } => 5,
            Record::RailDeclared { .. } => 6,
            Record::RailCrossing { .. }
            | Record::ResetRendered { .. }
            | Record::FirmwareObserved { .. } => 7,
            Record::BootObserved { .. } => 8,
            Record::RegisterObserved { .. } => 9,
            Record::I2cCheck { .. } => 10,
            Record::I2cWitness { .. } => 11,
            Record::Verdict { .. } => 12,
        }
    }
    /// Within rank 7: time in ps, then crossing, reset, observation.
    fn when(r: &Record) -> (u64, u8) {
        match r {
            Record::RailCrossing { at_ps, .. } => (*at_ps, 0),
            Record::ResetRendered { at_ns, .. } => (at_ns.saturating_mul(1000), 1),
            Record::FirmwareObserved { at_ns, .. } => (at_ns.saturating_mul(1000), 2),
            _ => (0, 0),
        }
    }
    let count_rank = |n: u8| records.iter().filter(|r| rank(r) == n).count();
    let ordered = records.windows(2).all(|w| {
        let (a, b) = (rank(&w[0]), rank(&w[1]));
        a < b || (a == b && (a != 7 || when(&w[0]) <= when(&w[1])))
    });
    if records.is_empty() || !ordered {
        return Err("the records are not in the campaign's order");
    }
    if !(count_rank(0) == 1 && count_rank(2) == 1 && count_rank(8) == 1 && count_rank(12) == 1) {
        return Err("a run holds exactly one scenario, topology, boot and verdict");
    }
    if count_rank(1) > 1 || count_rank(6) > 1 {
        return Err("a run holds at most one declared rail and one set of expected counts");
    }
    check_joins(records)?;
    let Record::Scenario {
        name,
        expect_pass,
        expect_findings,
        overrides,
        ..
    } = &records[0]
    else {
        unreachable!("ranked first");
    };
    let Some(Record::Verdict { pass, findings }) = records.last() else {
        unreachable!("ranked last");
    };

    // Each line that faults its bus, by (bus, net), and how many of each finding they need.
    let mut causes: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut needed: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::RiseMeasured {
                bus,
                net,
                exceeds: true,
                ..
            } => {
                causes.insert((bus, net));
                *needed.entry(PAST_CEILING_FINDING).or_default() += 1;
            }
            Record::LineStuckHigh { bus, net, .. } => {
                causes.insert((bus, net));
                *needed.entry(STUCK_HIGH_FINDING).or_default() += 1;
            }
            _ => {}
        }
    }
    let held_buses: BTreeSet<&str> = overrides
        .iter()
        .filter_map(|o| match o {
            Override::LineHeldLow { bus, .. } => Some(bus.as_str()),
            _ => None,
        })
        .collect();
    let mut faulted: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut held_faults = 0u64;
    for r in records {
        let Record::BusFault {
            bus, from, nets, ..
        } = r
        else {
            continue;
        };
        if from.iter().any(|f| f == HELD_LOW) {
            if from.len() != 1 || nets.len() != 1 || !held_buses.contains(bus.as_str()) {
                return Err("a held-low fault names no held-low override of its bus");
            }
            held_faults += 1;
        } else {
            for net in nets {
                if !causes.contains(&(bus.as_str(), net.as_str())) {
                    return Err("a bus fault names a net with no measurement past its ceiling");
                }
                faulted.insert((bus, net));
            }
        }
    }
    if faulted != causes {
        return Err("a measurement past its ceiling faulted no bus");
    }
    let held_lines = overrides
        .iter()
        .filter(|o| matches!(o, Override::LineHeldLow { .. }))
        .count() as u64;
    if held_faults != held_lines {
        return Err("a held-low override faulted no bus, or one bus twice");
    }
    if held_faults > 0 {
        needed.insert(HELD_LOW_FINDING, held_faults);
    }
    let count = |kind: &str| -> u64 {
        findings
            .iter()
            .filter(|(k, s, _)| k == kind && s != "info")
            .map(|(_, _, n)| *n)
            .sum()
    };
    for kind in [PAST_CEILING_FINDING, STUCK_HIGH_FINDING, HELD_LOW_FINDING] {
        if count(kind) != needed.get(kind).copied().unwrap_or(0) {
            return Err("the verdict's analog and held-low findings do not match the records");
        }
    }
    let any_severity = |kind: &str| -> u64 {
        findings
            .iter()
            .filter(|(k, _, _)| k == kind)
            .map(|(_, _, n)| *n)
            .sum()
    };
    let droop_counts = check_droop_records(records, overrides)?;
    for (kind, want) in droop_counts {
        if any_severity(kind) != want {
            return Err("the verdict's droop findings do not match the droop's records");
        }
    }
    if findings
        .iter()
        .any(|(k, s, _)| (k == RESET_FINDING || k == BELOW_TOLERANCE_FINDING) && s != "info")
        || findings
            .iter()
            .any(|(k, s, _)| k == UNRECOVERED_FINDING && s == "info")
    {
        return Err("a droop finding has the wrong severity");
    }
    let expected_counts: &[(String, u64)] = records
        .iter()
        .find_map(|r| match r {
            Record::ExpectedCounts { counts } => Some(counts.as_slice()),
            _ => None,
        })
        .unwrap_or(&[]);
    let counts_match = expected_counts
        .iter()
        .all(|(kind, n)| any_severity(kind) == *n);
    let failing_kinds: Vec<String> = findings
        .iter()
        .filter(|(_, s, _)| s != "info")
        .map(|(k, _, _)| k.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // A run matches when it fails for exactly the kinds named, no more and no
    // fewer, `info` never counting (`AET-WO-2300` §5.4). The records' own rules
    // make that the whole test: a passing verdict has no finding above `info`,
    // and a passing scenario names none.
    let matched = failing_kinds == *expect_findings && counts_match;
    Ok(RunSummary {
        name: name.clone(),
        expect_pass: *expect_pass,
        expect_findings: expect_findings.clone(),
        pass: *pass,
        failing_kinds,
        matched,
    })
}

/// `AET-WO-3100` R2: every bus, net, device and controller the other records
/// name is joined by exactly one solved bus (kind 15), so a dropped or forged
/// join is refused here and never reaches attribution as a guess.
/// A reading as the judge sees it: observed, and equal to the expected
/// value under the mask.
fn reads_as_expected(r: &Reading) -> bool {
    r.observed.is_some_and(|o| o & r.mask == r.expected & r.mask)
}

/// The droop's records against the scenario and against each other, and the
/// counts of the droop's findings they require (`AET-WO-2301` §2.2, §2.3, §5):
/// - kinds 16 to 19 appear only with a droop override, and kind 16 then
///   exactly once, carrying the override's own values;
/// - each active threshold's level is crossed exactly when the declared rail
///   floor is below it: once falling while the source falls, once rising
///   while it rises (within 10 τ of each ramp), at the table's typical value;
/// - the resets are exactly [`power::schedule`] of those crossings, each
///   with its fixed rendering;
/// - each release has exactly one observation, at the recovery bound after it,
///   unless the boot was killed before its reads (its boot record observed
///   nothing), when there is none and every release counts as unrecovered.
fn check_droop_records(
    records: &[Record],
    overrides: &[Override],
) -> Result<[(&'static str, u64); 3], &'static str> {
    let droop = overrides.iter().find_map(|o| match o {
        Override::Droop(d) => Some(d),
        _ => None,
    });
    let declared = records.iter().find_map(|r| match r {
        Record::RailDeclared {
            droop,
            nominal_millivolts,
            tolerance_permille,
            decoupling_picofarads,
        } => Some((droop, *nominal_millivolts, *tolerance_permille, *decoupling_picofarads)),
        _ => None,
    });
    let crossings: Vec<power::Crossing> = records
        .iter()
        .filter_map(|r| match r {
            Record::RailCrossing {
                level,
                microvolts,
                at_ps,
                falling,
                ..
            } => Some(power::Crossing {
                level: level.clone(),
                microvolts: *microvolts,
                at_ps: *at_ps,
                falling: *falling,
            }),
            _ => None,
        })
        .collect();
    let resets: Vec<(u64, bool, &[String], &str)> = records
        .iter()
        .filter_map(|r| match r {
            Record::ResetRendered {
                at_ns,
                released,
                cause,
                rendering,
            } => Some((*at_ns, *released, cause.as_slice(), rendering.as_str())),
            _ => None,
        })
        .collect();
    let observed: Vec<(u64, bool)> = records
        .iter()
        .filter_map(|r| match r {
            Record::FirmwareObserved {
                at_ns,
                boot_word,
                status,
                value,
            } => Some((
                *at_ns,
                reads_as_expected(boot_word) && reads_as_expected(status) && reads_as_expected(value),
            )),
            _ => None,
        })
        .collect();
    let (droop, (rail, nominal, tolerance, decoupling)) = match (droop, declared) {
        (None, None) => {
            if !(crossings.is_empty() && resets.is_empty() && observed.is_empty()) {
                return Err("a run without a droop holds a droop's records");
            }
            return Ok([(RESET_FINDING, 0), (UNRECOVERED_FINDING, 0), (BELOW_TOLERANCE_FINDING, 0)]);
        }
        (Some(d), Some(r)) => (d, r),
        _ => return Err("a droop override and its declared rail do not come together"),
    };
    if rail != droop {
        return Err("the declared rail is not the scenario's droop");
    }
    if records
        .iter()
        .any(|r| matches!(r, Record::RailCrossing { domain, .. } if *domain != droop.domain))
    {
        return Err("a crossing names another domain");
    }
    let (floor, _) = rail_levels(droop, nominal).ok_or("the declared rail's drop reaches its floor")?;
    let settle_ps = tau_ns(droop.source_milliohms, decoupling)
        .saturating_mul(10)
        .saturating_mul(1000);
    let ps = |ns: u64| ns.saturating_mul(1000);
    let fall = (ps(droop.start_ns), ps(droop.start_ns + droop.fall_ns).saturating_add(settle_ps));
    let rise_from = droop.start_ns + droop.fall_ns + droop.hold_ns;
    let rise = (ps(rise_from), ps(rise_from + droop.rise_ns).saturating_add(settle_ps));
    for (name, uv) in power::levels(droop.bor_level) {
        let here: Vec<&power::Crossing> = crossings.iter().filter(|c| c.level == name).collect();
        let within = |c: &power::Crossing, (a, b): (u64, u64)| a < c.at_ps && c.at_ps <= b;
        let ok = if floor < uv {
            matches!(here.as_slice(), [f, r]
                if f.falling && !r.falling && f.microvolts == uv && r.microvolts == uv
                    && within(f, fall) && within(r, rise))
        } else {
            here.is_empty()
        };
        if !ok {
            return Err("a threshold's crossings do not follow from the declared rail");
        }
    }
    if crossings.len() != crossings.iter().filter(|c| power::levels(droop.bor_level).iter().any(|(n, _)| *n == c.level)).count() {
        return Err("a crossing names a level no active threshold has");
    }
    let schedule = power::schedule(droop.bor_level, &crossings)?;
    let mut want: Vec<(u64, bool, &[String], &str)> = Vec::new();
    for r in &schedule {
        want.push((r.asserted_ns, false, &r.cause, power::ASSERT_RENDERING));
        if let Some(at) = r.released_ns {
            want.push((at, true, &r.cause, power::RELEASE_RENDERING));
        }
    }
    if resets != want {
        return Err("the rendered resets are not the supervisor's rule applied to the crossings");
    }
    let probes: Vec<u64> = schedule
        .iter()
        .filter_map(|r| r.released_ns)
        .map(|at| at + power::RECOVERY_BOUND_NS)
        .collect();
    let boot_read = records.iter().any(|r| {
        matches!(
            r,
            Record::BootObserved {
                instructions_observed: Some(_),
                ..
            }
        )
    });
    let at: Vec<u64> = observed.iter().map(|o| o.0).collect();
    let unrecovered_probes = if boot_read {
        if at != probes {
            return Err("each release has not exactly one observation at the recovery bound");
        }
        observed.iter().filter(|o| !o.1).count() as u64
    } else {
        if !at.is_empty() {
            return Err("a boot that read nothing holds an observation");
        }
        probes.len() as u64
    };
    let unrecovered =
        unrecovered_probes + schedule.iter().filter(|r| r.released_ns.is_none()).count() as u64;
    let tolerance_floor = u64::from(nominal) * u64::from(1000u32.saturating_sub(tolerance));
    Ok([
        (RESET_FINDING, schedule.len() as u64),
        (UNRECOVERED_FINDING, unrecovered),
        (BELOW_TOLERANCE_FINDING, u64::from(floor < tolerance_floor)),
    ])
}

fn check_joins(records: &[Record]) -> Result<(), &'static str> {
    /// A kind 15's bus, peripheral, lines and devices.
    type Solved<'a> = (&'a str, &'a str, &'a [BusLine], &'a [(String, u8)]);
    let solved: Vec<Solved> = records
        .iter()
        .filter_map(|r| match r {
            Record::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => Some((bus.as_str(), peripheral.as_str(), &lines[..], &devices[..])),
            _ => None,
        })
        .collect();
    if !solved.windows(2).all(|w| w[0].0 < w[1].0) {
        return Err("the solved buses are not sorted by unique bus");
    }
    // No net, pin or controller belongs to two buses, and no two devices on
    // one bus share an address (`AET-WO-3100` adversarial A6).
    fn unique<T: Ord>(items: Vec<T>) -> bool {
        let n = items.len();
        items.into_iter().collect::<BTreeSet<_>>().len() == n
    }
    let lines = || solved.iter().flat_map(|b| b.2.iter());
    if !unique(lines().map(|l| l.net.as_str()).collect())
        || !unique(lines().map(|l| l.pin.as_str()).collect())
        || !unique(solved.iter().map(|b| b.1).collect())
    {
        return Err("two solved buses share a net, a pin or a controller");
    }
    if !solved
        .iter()
        .all(|b| unique(b.3.iter().map(|d| d.1).collect()))
    {
        return Err("two devices on one solved bus share an address");
    }
    // One measurement per line, one identity read per device, one witness per
    // device and per controller, as the campaign writes them (adversarial A2).
    let mut seen: BTreeSet<(u8, &str, &str)> = BTreeSet::new();
    for r in records {
        let keys: Vec<(u8, &str, &str)> = match r {
            Record::RiseMeasured { bus, net, .. } | Record::LineStuckHigh { bus, net, .. } => {
                vec![(0, bus, net)]
            }
            Record::I2cCheck { reference, .. } => vec![(1, reference, "")],
            Record::I2cWitness {
                peripheral,
                reference,
                ..
            } => vec![(2, reference, ""), (3, peripheral, "")],
            _ => vec![],
        };
        for key in keys {
            if !seen.insert(key) {
                return Err(
                    "a line measured twice, or a device read or witnessed twice, or a controller witnessed twice",
                );
            }
        }
    }
    let bus = |id: &str| {
        solved
            .iter()
            .find(|b| b.0 == id)
            .ok_or("an analog record or a bus fault names a bus no kind 15 solves")
    };
    let line = |id: &str, net: &str| -> Result<&BusLine, &'static str> {
        bus(id)?
            .2
            .iter()
            .find(|l| l.net == net)
            .ok_or("an analog record or a bus fault names a net that is not a line of its bus")
    };
    // Each device on exactly one bus: (bus's peripheral, address).
    let mut device: BTreeMap<&str, (&str, u8)> = BTreeMap::new();
    for (_, peripheral, _, devices) in &solved {
        for (reference, address) in *devices {
            if device
                .insert(reference.as_str(), (*peripheral, *address))
                .is_some()
            {
                return Err("a device is on two solved buses");
            }
        }
    }
    for r in records {
        match r {
            Record::RiseMeasured {
                bus,
                net,
                ceiling_ps,
                ..
            }
            | Record::LineStuckHigh {
                bus,
                net,
                ceiling_ps,
                ..
            } => {
                if line(bus, net)?.ceiling_ps != *ceiling_ps {
                    return Err("an analog record's ceiling is not its solved line's");
                }
            }
            Record::BusFault { bus, nets, .. } => {
                for net in nets {
                    line(bus, net)?;
                }
            }
            Record::I2cCheck {
                reference, address, ..
            } => {
                if device.get(reference.as_str()).map(|d| d.1) != Some(*address) {
                    return Err("an identity read names no device of a solved bus at its address");
                }
            }
            Record::I2cWitness {
                peripheral,
                reference,
                ..
            } => match device.get(reference.as_str()) {
                None => return Err("a controller witness names no device of a solved bus"),
                Some((p, _)) if p != peripheral => {
                    return Err("a controller witness is not on its device's solved bus");
                }
                Some(_) => {}
            },
            _ => {}
        }
    }
    Ok(())
}

/// The header artifacts of a campaign run, in the header's sorted order.
pub const ARTIFACTS: [&str; 5] = ["base", "firmware", "scenario", "specification", "topology"];

/// Judge an archived campaign run, after the strict reader: it holds no
/// event but campaign records, its header names exactly what its records
/// name, and its `run_id` is the one those hashes give (`AET-WO-2300`
/// adversarial F4, F5). Then [`check_run`].
pub fn check_archived(header: &Header, state: &State) -> Result<RunSummary, &'static str> {
    let raw = !state.registers.is_empty()
        || !state.pins.is_empty()
        || !state.interrupts.is_empty()
        || !state.nets.is_empty()
        || state.faults != 0;
    if raw {
        return Err("a campaign run holds an event that is not a campaign record");
    }
    let summary = check_run(&state.records)?;
    let names: Vec<&str> = header.artifacts.iter().map(|(n, _)| n.as_str()).collect();
    if names != ARTIFACTS {
        return Err("the header's artifacts are not a campaign run's");
    }
    let hash = |i: usize| &header.artifacts[i].1;
    // The topology follows the scenario, and its expected counts if any.
    let solved = state
        .records
        .iter()
        .find(|r| matches!(r, Record::Solved { .. }));
    let (
        Record::Scenario { sha256, .. },
        Some(Record::Solved {
            specification_sha256,
            topology_sha256,
        }),
    ) = (&state.records[0], solved)
    else {
        unreachable!("check_run puts the scenario first and holds one topology");
    };
    if hash(2) != sha256 || hash(3) != specification_sha256 || hash(4) != topology_sha256 {
        return Err("the header's hashes are not the ones its records name");
    }
    if run_id(hash(2), hash(3), hash(1)) != header.run_id {
        return Err("the run id is not the one its scenario, specification and firmware give");
    }
    Ok(summary)
}
