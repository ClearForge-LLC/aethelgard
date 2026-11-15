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
use crate::record::{
    self, BusLine, Droop, Line, Override, Reading, Record, SERIAL_FRAME_LEN, SerialCapture,
    split_pin, step_index,
};
use crate::replay::State;

/// The scenario format versions read: 1 (static scenarios, unchanged) and 2,
/// since `AET-WO-2301`, which admits the `droop` override and `expect_counts`,
/// and since `AET-WO-4200` the `spi_device_absent` override; a version 1 file
/// may use none of them.
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
/// The boot judge's finding for a core that ended outside every executable
/// segment of the firmware, which kind 10 archives (`AET-WO-2205` §1.5).
pub const PC_OUTSIDE_FINDING: &str = "pc_outside_firmware";
/// The boot judge's finding for a core parked inside the firmware but off
/// every self-loop, made only when `pc` is inside (`AET-WO-2204` D3).
pub const SELF_LOOP_FINDING: &str = "pc_not_on_self_loop";

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
    #[serde(default, deserialize_with = "present")]
    spi_device_absent: Option<serde_json::Value>,
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
        spi_device_absent,
    } = raw;
    let others = bus.is_some()
        || net.is_some()
        || pull_up_ohms.is_some()
        || max_capacitance_femtofarads.is_some()
        || line_held_low.is_some()
        || droop.is_some();
    if let Some(r) = spi_device_absent {
        if others {
            return Err(schema(
                "an absent SPI device override is exactly {spi_device_absent: reference}"
                    .to_owned(),
            ));
        }
        return Ok(Override::SpiDeviceAbsent {
            reference: text(r, "spi_device_absent")?,
        });
    }
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
            "an override is exactly {bus, pull_up_ohms}, {net, max_capacitance_femtofarads}, {bus, line_held_low}, {droop} or {spi_device_absent}"
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
            return Err(schema(format!(
                "droop `{key}` is not from {lo} to {hi}: {v}"
            )));
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
    // A drop that takes the rail to 0 V is no rail at all: refused.
    Some((
        floor.checked_sub(drop).filter(|f| *f > 0)?,
        rest.checked_sub(drop)?,
    ))
}

/// The domain's tolerance floor in µV: its nominal less its tolerance.
pub fn tolerance_floor_uv(nominal_millivolts: u32, tolerance_permille: u32) -> u64 {
    u64::from(nominal_millivolts) * u64::from(1000u32.saturating_sub(tolerance_permille))
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
        // The rail's floor must lie clear of the threshold's whole hysteresis
        // region, from the falling band's minimum to the rising band's
        // maximum: inside it the part's behaviour is not stated, and near a
        // level ngspice computes, the rail's ringing dithers across it
        // (`AET-WO-2301` adversarial F4).
        let region = [t.falling_uv[0], 0, t.rising_uv[2]];
        if power::band(floor, region) == Band::Inside
            && power::band(floor, t.falling_uv) != Band::Inside
        {
            return Err(refuse(format!(
                "the rail floor, {floor} µV, lies inside {}'s hysteresis region {:?} µV, near the levels ngspice computes",
                t.name,
                [region[0], region[2]]
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
        Override::SpiDeviceAbsent { reference } => (5, reference),
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
        let v2_only = overrides
            .iter()
            .any(|o| matches!(o, Override::Droop(_) | Override::SpiDeviceAbsent { .. }))
            || !expect_counts.is_empty();
        if v2_only && raw.schema_version < 2 {
            return Err(schema(
                "a droop override, an absent SPI device and expect_counts need schema_version 2"
                    .to_owned(),
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
/// order: the boot's witnesses (kind 10, then 11, 12, 13, 21, 22, 26, 28 and
/// 29 in that order), and the verdict, last and once. The solved motor
/// outputs (kind 24) and their static checks (kind 25) sit with the solved
/// buses (`AET-WO-4300a`), and so do the solved serial ports (kind 27,
/// `AET-WO-4300b`).
pub fn check_run(records: &[Record]) -> Result<RunSummary, &'static str> {
    fn rank(r: &Record) -> u8 {
        match r {
            Record::Scenario { .. } => 0,
            Record::ExpectedCounts { .. } => 1,
            Record::Solved { .. } => 2,
            Record::I2cBusSolved { .. }
            | Record::SpiBusSolved { .. }
            | Record::MotorOutputSolved { .. }
            | Record::MotorTimingCheck { .. }
            | Record::SerialPortSolved { .. } => 3,
            Record::RiseComputed { .. } | Record::LineStuckHigh { .. } => 4,
            Record::BusFault { .. } => 5,
            Record::RailDeclared { .. } => 6,
            Record::RailCrossing { .. }
            | Record::ResetRendered { .. }
            | Record::FirmwareObserved { .. } => 7,
            Record::BootObserved { .. } => 8,
            Record::RegisterObserved { .. } => 9,
            Record::I2cCheck { .. } => 10,
            Record::I2cWitness { .. } => 11,
            Record::SpiCheck { .. } => 12,
            Record::SpiWitness { .. } => 13,
            Record::MotorWitness { .. } => 14,
            Record::SerialFramingCheck { .. } => 15,
            Record::SerialWitness { .. } => 16,
            Record::Verdict { .. } => 17,
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
    if !(count_rank(0) == 1 && count_rank(2) == 1 && count_rank(8) == 1 && count_rank(17) == 1) {
        return Err("a run holds exactly one scenario, topology, boot and verdict");
    }
    if count_rank(1) > 1 || count_rank(6) > 1 {
        return Err("a run holds at most one declared rail and one set of expected counts");
    }
    check_joins(records)?;
    let booted = records.iter().any(|r| {
        matches!(r, Record::BootObserved { boot_word, .. }
            if boot_word.observed == Some(boot_word.expected))
    });
    check_serial_joins(records, booted)?;
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
            Record::RiseComputed {
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
    // The boot's `pc` against the segments kind 10 archives: the verdict
    // names a core outside the firmware exactly when they place it there.
    let outside = records.iter().any(|r| {
        matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
            if !record::inside(executable, *pc))
    });
    // At any severity: the judge makes it an error or not at all
    // (`AET-WO-2205` adversarial 2).
    if any_severity(PC_OUTSIDE_FINDING) != u64::from(outside)
        || count(PC_OUTSIDE_FINDING) != u64::from(outside)
    {
        return Err("the verdict's pc_outside_firmware does not match the boot's pc and segments");
    }
    // The judge looks for the self-loop only inside the firmware
    // (`AET-WO-2205` adversarial 3).
    let read_inside = records.iter().any(|r| {
        matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
            if record::inside(executable, *pc))
    });
    if any_severity(SELF_LOOP_FINDING) > 0 && !read_inside {
        return Err(
            "the verdict names pc_not_on_self_loop for a pc outside the firmware or unread",
        );
    }
    // The motor outputs (`AET-WO-4300a`): the verdict names a static
    // mismatch once per check that fails, and, once the boot word is set,
    // each witness's finding by its status, exactly; before it, none.
    let mut motor_needed: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::MotorTimingCheck { holds: false, .. } => {
                *motor_needed.entry(MOTOR_STATIC_FINDING).or_default() += 1;
            }
            Record::MotorWitness { status, .. } if booted => {
                let kind = match status.as_str() {
                    "unwitnessed" | "unreadable" => Some(MOTOR_UNWITNESSED_FINDING),
                    "mismatch" | "batched" | "stopped" => Some(MOTOR_MISMATCH_FINDING),
                    "static_only" => Some(MOTOR_STATIC_ONLY_FINDING),
                    _ => None,
                };
                if let Some(kind) = kind {
                    *motor_needed.entry(kind).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    for kind in MOTOR_FINDINGS {
        if any_severity(kind) != motor_needed.get(kind).copied().unwrap_or(0) {
            return Err("the verdict's motor findings do not match the motor records");
        }
    }
    if findings.iter().any(|(k, s, _)| {
        (k == MOTOR_STATIC_ONLY_FINDING) != (s == "info") && MOTOR_FINDINGS.contains(&k.as_str())
    }) {
        return Err("a motor finding has the wrong severity");
    }
    // The serial ports (`AET-WO-4300b`), as the judge decides them: a
    // failing silicon check always; once the boot word is set, a port's tags
    // (kind 28) and its witness (kind 29), exactly. A capture directory
    // holding what no port's backend wrote is forged too and names no port,
    // so the records bound `serial_capture_forged` from below only; with no
    // port there is no capture directory, and none.
    let serial = serial_findings(records, booted);
    for kind in SERIAL_FINDINGS {
        let (have, want) = (any_severity(kind), serial.get(kind).copied().unwrap_or(0));
        let ports = records
            .iter()
            .any(|r| matches!(r, Record::SerialPortSolved { .. }));
        let ok = if kind == SERIAL_FORGED_FINDING && ports {
            have >= want
        } else {
            have == want
        };
        if !ok {
            return Err("the verdict's serial findings do not match the serial records");
        }
    }
    if findings
        .iter()
        .any(|(k, s, _)| s == "info" && SERIAL_FINDINGS.contains(&k.as_str()))
    {
        return Err("a serial finding has the wrong severity");
    }
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
/// Each observation's readings are the boot's own (`AET-WO-2301` adversarial
/// F5): the boot word's address and expectation and the identity read's
/// expected value, each at the mask the boot record judges it by; the
/// record's status and value are full words. An observation's witness lines
/// are judged against the first identity read's witness record (kind 13) and
/// exist exactly when it does (`AET-WO-2204`). Replay trusts no expectation
/// an observation could carry for itself.
fn check_observed_against_the_boot(records: &[Record]) -> Result<(), &'static str> {
    let boot = records.iter().find_map(|r| match r {
        Record::BootObserved { boot_word, .. } => Some(boot_word),
        _ => None,
    });
    // The first identity read: the first I2C one, or the first SPI one on a
    // board with none, as the checks are ordered (`AET-WO-4200` adversarial A1).
    let check = records
        .iter()
        .find_map(|r| match r {
            Record::I2cCheck { expected, .. } => Some(*expected),
            _ => None,
        })
        .or_else(|| {
            records.iter().find_map(|r| match r {
                Record::SpiCheck { expected, .. } => Some(*expected),
                _ => None,
            })
        });
    let has_witness = first_witness(records).is_some();
    let same = |a: &Reading, b: &Reading| {
        (a.address, a.mask, a.expected) == (b.address, b.mask, b.expected)
    };
    for r in records {
        let Record::FirmwareObserved {
            boot_word,
            status,
            value,
            witness,
            ..
        } = r
        else {
            continue;
        };
        let ok = boot.is_some_and(|b| same(boot_word, b))
            && status.mask == u32::MAX
            && value.mask == u32::MAX
            && check.is_none_or(|v| u64::from(value.expected) == v)
            && (has_witness || witness.is_none());
        if !ok {
            return Err("an observation's readings are not the boot's own");
        }
    }
    Ok(())
}

/// The first identity read's expected witness lines (kind 13, or kind 22 on
/// a board with no I2C read), which a droop's observation must show since
/// its release.
fn first_witness(records: &[Record]) -> Option<&[String]> {
    records
        .iter()
        .find_map(|r| match r {
            Record::I2cWitness { expected, .. } => Some(expected.as_slice()),
            _ => None,
        })
        .or_else(|| {
            records.iter().find_map(|r| match r {
                Record::SpiWitness { expected, .. } => Some(expected.as_slice()),
                _ => None,
            })
        })
}

/// A reading as the judge sees it: observed, and equal to the expected
/// value under the mask.
fn reads_as_expected(r: &Reading) -> bool {
    r.observed
        .is_some_and(|o| o & r.mask == r.expected & r.mask)
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
        } => Some((
            droop,
            *nominal_millivolts,
            *tolerance_permille,
            *decoupling_picofarads,
        )),
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
    let want = first_witness(records);
    let observed: Vec<(u64, bool)> = records
        .iter()
        .filter_map(|r| match r {
            Record::FirmwareObserved {
                at_ns,
                boot_word,
                status,
                value,
                witness,
            } => Some((
                *at_ns,
                [boot_word, status, value]
                    .iter()
                    .all(|r| reads_as_expected(r))
                    && witness.as_deref() == want,
            )),
            _ => None,
        })
        .collect();
    check_observed_against_the_boot(records)?;
    let (droop, (rail, nominal, tolerance, decoupling)) = match (droop, declared) {
        (None, None) => {
            if !(crossings.is_empty() && resets.is_empty() && observed.is_empty()) {
                return Err("a run without a droop holds a droop's records");
            }
            return Ok([
                (RESET_FINDING, 0),
                (UNRECOVERED_FINDING, 0),
                (BELOW_TOLERANCE_FINDING, 0),
            ]);
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
    let (floor, _) =
        rail_levels(droop, nominal).ok_or("the declared rail's drop reaches its floor")?;
    let settle_ps = tau_ns(droop.source_milliohms, decoupling)
        .saturating_mul(10)
        .saturating_mul(1000);
    let ps = |ns: u64| ns.saturating_mul(1000);
    let fall = (
        ps(droop.start_ns),
        ps(droop.start_ns + droop.fall_ns).saturating_add(settle_ps),
    );
    let rise_from = droop.start_ns + droop.fall_ns + droop.hold_ns;
    let rise = (
        ps(rise_from),
        ps(rise_from + droop.rise_ns).saturating_add(settle_ps),
    );
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
    if crossings.len()
        != crossings
            .iter()
            .filter(|c| {
                power::levels(droop.bor_level)
                    .iter()
                    .any(|(n, _)| *n == c.level)
            })
            .count()
    {
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
    let tolerance_floor = tolerance_floor_uv(nominal, tolerance);
    Ok([
        (RESET_FINDING, schedule.len() as u64),
        (UNRECOVERED_FINDING, unrecovered),
        (BELOW_TOLERANCE_FINDING, u64::from(floor < tolerance_floor)),
    ])
}

/// The motor output findings the boot judge gives (`AET-WO-4300a`).
pub const MOTOR_STATIC_FINDING: &str = "motor_timing_static_mismatch";
pub const MOTOR_UNWITNESSED_FINDING: &str = "motor_output_unwitnessed";
pub const MOTOR_MISMATCH_FINDING: &str = "motor_timing_mismatch";
/// `info`: the witness fit the model's sequence with every edge on the grid.
pub const MOTOR_STATIC_ONLY_FINDING: &str = "motor_witness_static_only";

/// The serial port findings the boot judge gives (`AET-WO-4300b`): the
/// silicon check of a port's `BRR` and framing, and the twin's witness of its
/// pattern, its echo, its capture and its tags.
pub const SERIAL_STATIC_FINDING: &str = "serial_framing_static_mismatch";
pub const SERIAL_PATTERN_FINDING: &str = "serial_pattern_missing";
pub const SERIAL_ECHO_FINDING: &str = "serial_echo_missing";
pub const SERIAL_FORGED_FINDING: &str = "serial_capture_forged";
pub const SERIAL_TAG_FINDING: &str = "serial_tag_mismatch";

/// Every serial finding kind the records explain.
const SERIAL_FINDINGS: [&str; 5] = [
    SERIAL_STATIC_FINDING,
    SERIAL_PATTERN_FINDING,
    SERIAL_ECHO_FINDING,
    SERIAL_FORGED_FINDING,
    SERIAL_TAG_FINDING,
];

/// Each serial finding kind the records give, and how many: a failing
/// static check (kind 28's `holds`) always; once `booted`, each port's tags
/// that failed or were not read (kind 28), and each witness's findings
/// ([`serial_witness_findings`]).
fn serial_findings(records: &[Record], booted: bool) -> BTreeMap<&'static str, u64> {
    let releases = released(records);
    let mut out: BTreeMap<&'static str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::SerialFramingCheck { holds, tags, .. } => {
                if !holds {
                    *out.entry(SERIAL_STATIC_FINDING).or_default() += 1;
                }
                if booted && tags.as_ref().is_none_or(|t| !t.holds()) {
                    *out.entry(SERIAL_TAG_FINDING).or_default() += 1;
                }
            }
            Record::SerialWitness { label, .. } if booted => {
                let port = records
                    .iter()
                    .find(|p| matches!(p, Record::SerialPortSolved { label: l, .. } if l == label));
                for kind in port
                    .map(|p| serial_witness_findings(p, r, releases))
                    .unwrap_or_default()
                {
                    *out.entry(kind).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    out
}

/// The resets the run rendered and released (kind 18): the boots after the
/// first.
pub fn released(records: &[Record]) -> u64 {
    records
        .iter()
        .filter(|r| matches!(r, Record::ResetRendered { released: true, .. }))
        .count() as u64
}

/// Whether a pin's `MODER` and `AFSEL` fields, as kind 29 carries them, put
/// it in alternate-function mode (`10`, RM0433 Rev 9 §11.4.1) on `af`.
fn routed(mux: Option<(u8, u8)>, af: u8) -> bool {
    mux.is_some_and(|(mode, a)| mode == 0b10 && a == af)
}

/// The finding kinds kind 29 `witness` gives, joined to kind 27 `port`, in a
/// run whose boot word is set, with `releases` boots after the first, as the
/// boot judge gives them: no capture is forged and nothing else; a capture
/// with stray bytes, or more patterns (whole and cut) than boots, is forged;
/// the pattern is out when at least one whole pattern was captured and the
/// TX pin was on its AF at the first `TDR` write of each boot (adversarial
/// F4) and at the end; and the echo fails unless witnessed.
pub fn serial_witness_findings(
    port: &Record,
    witness: &Record,
    releases: u64,
) -> Vec<&'static str> {
    let (
        Record::SerialPortSolved { tx_af, .. },
        Record::SerialWitness {
            capture,
            echo,
            tx_at_end,
            tx_at_send,
            ..
        },
    ) = (port, witness)
    else {
        return Vec::new();
    };
    let Some(c) = capture else {
        return vec![SERIAL_FORGED_FINDING];
    };
    let mut out = Vec::new();
    let boots = releases.saturating_add(1);
    if c.stray || c.patterns.saturating_add(c.cut) > boots {
        out.push(SERIAL_FORGED_FINDING);
    }
    if !(c.patterns > 0 && routed(*tx_at_send, *tx_af) && routed(*tx_at_end, *tx_af)) {
        out.push(SERIAL_PATTERN_FINDING);
    }
    if echo != "witnessed" && echo != "not_judged" {
        out.push(SERIAL_ECHO_FINDING);
    }
    out
}

/// Kinds 28 and 29 each join one solved serial port (kind 27) by label
/// (`AET-WO-4300b`): the ports are in the specification's order, each on
/// its own USART and pins; every port has exactly one framing check, in the
/// ports' order, which is what its solved port gives silicon; and its
/// witnesses are in the ports' order too, every port's or none, and every
/// port's once the boot word is set, each echo the status its own counts and
/// pins give, and its TX pin at its sends rebuilt only where its framing
/// check's writes were (both come from one access log; adversarial pass 2,
/// L1).
fn check_serial_joins(records: &[Record], booted: bool) -> Result<(), &'static str> {
    let ports: Vec<&Record> = records
        .iter()
        .filter(|r| matches!(r, Record::SerialPortSolved { .. }))
        .collect();
    let mut labels = BTreeSet::new();
    let mut peripherals = BTreeSet::new();
    let mut pins = BTreeSet::new();
    let mut last_index = None;
    for p in &ports {
        let Record::SerialPortSolved {
            label,
            index,
            peripheral,
            tx,
            rx,
            ..
        } = p
        else {
            unreachable!("filtered");
        };
        if !labels.insert(label.as_str())
            || !peripherals.insert(peripheral.as_str())
            || !pins.insert(tx.as_str())
            || !pins.insert(rx.as_str())
            || last_index.is_some_and(|l| l >= *index)
        {
            return Err(
                "the solved serial ports are not in the specification's order, each with its own label, USART and pins",
            );
        }
        last_index = Some(*index);
    }
    let label_of = |r: &Record| match r {
        Record::SerialPortSolved { label, .. }
        | Record::SerialFramingCheck { label, .. }
        | Record::SerialWitness { label, .. } => label.clone(),
        _ => String::new(),
    };
    let order: Vec<String> = ports.iter().map(|p| label_of(p)).collect();
    let of = |pick: fn(&Record) -> bool| -> Vec<&Record> {
        records.iter().filter(|r| pick(r)).collect()
    };
    let checks = of(|r| matches!(r, Record::SerialFramingCheck { .. }));
    let witnesses = of(|r| matches!(r, Record::SerialWitness { .. }));
    if checks.iter().map(|r| label_of(r)).collect::<Vec<_>>() != order {
        return Err("the serial framing checks are not one per solved port, in the ports' order");
    }
    let witnessed: Vec<String> = witnesses.iter().map(|r| label_of(r)).collect();
    if witnessed != order && (booted || !witnessed.is_empty()) {
        return Err(
            "the serial witnesses are not one per solved port in the ports' order, or are absent from a run whose boot word is set",
        );
    }
    for (port, check) in ports.iter().zip(&checks) {
        if !serial_silicon(port, check) {
            return Err("a serial framing check is not what its solved port gives");
        }
    }
    for (check, witness) in checks.iter().zip(&witnesses) {
        if matches!(
            (check, witness),
            (
                Record::SerialFramingCheck { tags: None, .. },
                Record::SerialWitness {
                    tx_at_send: Some(_),
                    ..
                }
            )
        ) {
            return Err(
                "a serial witness's TX pin at its sends is rebuilt from an access log its framing check could not read",
            );
        }
    }
    for (port, witness) in ports.iter().zip(&witnesses) {
        if !serial_echo(port, witness, booted) {
            return Err("a serial witness's echo is not what its capture and its RX pin give");
        }
        if !serial_tx_read(port, witness, records) {
            return Err(
                "a serial witness's TX pin at the end is not what kind 11 read of its MODER and AFR",
            );
        }
    }
    Ok(())
}

/// Whether kind 29 `witness`'s TX pin at the end is the `MODER` and `AFSEL`
/// fields of kind 11's reads of the pin's `GPIOx.MODER` and `GPIOx.AFRL` or
/// `AFRH`, joined to kind 27 `port` (adversarial F11): both present and
/// equal, or both absent (the image read neither register).
// belt-and-suspenders: the judge takes both from one read
// (`aethelgard_twin::serial::PinMux`); replay checks an archive without the
// twin, so it splits the pin and places its fields again.
fn serial_tx_read(port: &Record, witness: &Record, records: &[Record]) -> bool {
    let (Record::SerialPortSolved { tx, .. }, Record::SerialWitness { tx_at_end, .. }) =
        (port, witness)
    else {
        return false;
    };
    let mut chars = tx.chars();
    let (Some('P'), Some(letter @ 'A'..='K')) = (chars.next(), chars.next()) else {
        return false;
    };
    let Some(index) = chars.as_str().parse::<u32>().ok().filter(|i| *i < 16) else {
        return false;
    };
    let read = |name: String| {
        records.iter().find_map(|r| match r {
            Record::RegisterObserved { name: n, reading } if *n == name => reading.observed,
            _ => None,
        })
    };
    let moder = read(format!("GPIO{letter}.MODER"));
    let afr = read(format!(
        "GPIO{letter}.{}",
        if index < 8 { "AFRL" } else { "AFRH" }
    ));
    match (tx_at_end, moder.zip(afr)) {
        (None, None) => moder.is_none() && afr.is_none(),
        (Some((mode, af)), Some((m, a))) => {
            u32::from(*mode) == (m >> (2 * index)) & 0b11
                && u32::from(*af) == (a >> (4 * (index % 8))) & 0xF
        }
        _ => false,
    }
}

/// RM0433 Rev 9 §48.5.8, Tables 406 (`BRR[3:0]` = 0) and 407, column
/// `OVER8` = 0, `ONEBIT` = 0: the receiver's tolerance in ppm by `M[1:0]`.
// belt-and-suspenders: `aethelgard_ir::serial::TOLERANCE` is the judge's
// table; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// table again.
const SERIAL_TOLERANCE: [(&str, u8, u32); 6] = [
    ("Table 406", 0b00, 37_500),
    ("Table 406", 0b01, 34_100),
    ("Table 406", 0b10, 41_600),
    ("Table 407", 0b00, 33_300),
    ("Table 407", 0b01, 30_300),
    ("Table 407", 0b10, 37_000),
];

/// Whether kind 28 `check` is exactly the static framing check of kind 27
/// `port`: the figures, the tolerance row, `holds`, and the tags the framing
/// needs, re-derived (as kind 25's are: adversarial pass on `4300a`).
// belt-and-suspenders: `aethelgard_ir::serial::check_silicon` is the judge's
// check; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// same exact-integer rules again: the kernel clock at reset (RM0433 Rev 9
// §8.7.21, every prescaler 1), `BRR` the nearest whole `USARTDIV` from 16 to
// `0xFFFF`, a 7- to 9-bit word and 1 or 2 stop bits, and an error strictly
// below the row's tolerance.
fn serial_silicon(port: &Record, check: &Record) -> bool {
    let (
        Record::SerialPortSolved {
            framing,
            kernel_hz: k_port,
            brr: b_port,
            actual_millibaud: a_port,
            error_ppm: e_port,
            ..
        },
        Record::SerialFramingCheck {
            kernel_hz,
            brr,
            actual_millibaud,
            error_ppm,
            tolerance,
            holds,
            tags_needed,
            ..
        },
    ) = (port, check)
    else {
        return false;
    };
    let (k, b, baud) = (
        u128::from(*kernel_hz),
        u128::from(*brr),
        u128::from(framing.baud),
    );
    let d = b * baud;
    let actual = u64::try_from(k * 1000 / b.max(1)).ok();
    // The error, rounded half away from zero.
    let error = (d > 0).then(|| {
        let num =
            (i128::try_from(k).unwrap_or(i128::MAX) - i128::try_from(d).unwrap_or(0)) * 1_000_000;
        let d = i128::try_from(d).unwrap_or(i128::MAX);
        let q = (2 * num.abs() + d) / (2 * d);
        i64::try_from(if num < 0 { -q } else { q }).unwrap_or(i64::MAX)
    });
    let m_bits = match framing.word_bits() {
        8 => Some(0b00),
        9 => Some(0b01),
        7 => Some(0b10),
        _ => None,
    };
    let table = if brr & 0xF == 0 {
        "Table 406"
    } else {
        "Table 407"
    };
    let row = m_bits.and_then(|m| {
        SERIAL_TOLERANCE
            .iter()
            .find(|(t, rm, _)| *t == table && *rm == m)
    });
    let solved = (baud > 0)
        .then(|| (2 * k + baud) / (2 * baud))
        .is_some_and(|n| n == b && (16..=0xFFFF).contains(&n));
    let within = row.is_some_and(|(_, _, ppm)| k.abs_diff(d) * 1_000_000 < u128::from(*ppm) * d);
    let want_holds =
        *kernel_hz == RESET_KERNEL_HZ && solved && (1..=2).contains(&framing.stop_bits) && within;
    (kernel_hz, brr) == (k_port, b_port)
        && actual == Some(*actual_millibaud)
        && *a_port == *actual_millibaud
        && error.unwrap_or(i64::MAX) == *error_ppm
        && *e_port == *error_ppm
        && tolerance.as_ref().map(|(t, m, p)| (t.as_str(), *m, *p))
            == row.map(|&(t, m, p)| (t, m, p))
        && want_holds == *holds
        && *tags_needed == framing.tags()
}

/// Whether kind 29 `witness`'s echo is the status the judge gives its
/// capture and its RX pin at the injection, joined to kind 27 `port`: not
/// judged without the boot word; else the whole frame after the patterns
/// with the RX pin read is witnessed when that pin was on its AF and not
/// counted otherwise; some of it is partial; anything else is missing.
fn serial_echo(port: &Record, witness: &Record, booted: bool) -> bool {
    let (
        Record::SerialPortSolved { rx_af, .. },
        Record::SerialWitness {
            capture,
            echo,
            rx_at_injection,
            ..
        },
    ) = (port, witness)
    else {
        return false;
    };
    let clean = |c: &&SerialCapture| !c.stray;
    let echoed = capture.as_ref().filter(clean).map_or(0, |c| c.echoed);
    let want = match (booted, rx_at_injection) {
        (false, _) => "not_judged",
        (true, Some(_)) if echoed == SERIAL_FRAME_LEN as u64 => {
            if routed(*rx_at_injection, *rx_af) {
                "witnessed"
            } else {
                "not_counted"
            }
        }
        (true, Some(_)) if echoed > 0 => "partial",
        (true, _) => "missing",
    };
    echo == want
}

/// Each serial port's frame, kind 29's, against the one the judge injects
/// into the firmware the header names (`artifacts` "firmware"): the frame is
/// re-derived from that sha256 and the port's label, never read from the
/// record alone (`AET-WO-4300b`).
pub fn check_serial_frames(firmware: &[u8; 32], records: &[Record]) -> Result<(), &'static str> {
    let forged = records.iter().any(|r| {
        matches!(r, Record::SerialWitness { label, frame, .. }
            if *frame != record::serial_frame(firmware, label))
    });
    if forged {
        Err("a serial witness's frame is not the one its firmware and label give")
    } else {
        Ok(())
    }
}

/// Every motor finding kind the records explain.
const MOTOR_FINDINGS: [&str; 4] = [
    MOTOR_STATIC_FINDING,
    MOTOR_UNWITNESSED_FINDING,
    MOTOR_MISMATCH_FINDING,
    MOTOR_STATIC_ONLY_FINDING,
];

/// Kinds 25 and 26 each join one solved motor output (kind 24) by label,
/// every output has exactly one static check and at most one witness, and
/// a witness's observer is its output's pin's (`AET-WO-4300a`).
fn check_motor_joins(records: &[Record]) -> Result<(), &'static str> {
    let mut solved: BTreeMap<&str, &Record> = BTreeMap::new();
    for r in records {
        if let Record::MotorOutputSolved { label, .. } = r
            && solved.insert(label.as_str(), r).is_some()
        {
            return Err("a motor output is solved twice");
        }
    }
    // The board's window, from its outputs' protocols (`AET-WO-4300a` D1):
    // every witness ends at it. Kind 10's instruction count is not tied to
    // it: a droop's boot counts from the last reset's release (third
    // adversarial review, F1).
    let window = if solved.values().any(
        |m| matches!(m, Record::MotorOutputSolved { protocol, .. } if protocol.starts_with("dshot")),
    ) {
        TWIN_DSHOT_WINDOW_NS
    } else {
        TWIN_MAX_VIRTUAL_NS
    };
    let mut checked: BTreeSet<&str> = BTreeSet::new();
    let mut witnessed: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        match r {
            Record::MotorTimingCheck { label, .. } => {
                let Some(m) = solved.get(label.as_str()) else {
                    return Err("a motor timing check names no solved output");
                };
                if !checked.insert(label) {
                    return Err("a motor timing check names one output twice");
                }
                if silicon_check(m, r) != Some(true) {
                    return Err("a motor timing check is not what its solved output gives");
                }
            }
            Record::MotorWitness {
                label, observer, ..
            } => {
                let Some(m @ Record::MotorOutputSolved { pin, .. }) =
                    solved.get(label.as_str()).copied()
                else {
                    return Err("a motor witness names no solved output");
                };
                if !matches!(r, Record::MotorWitness { end_ns, .. } if *end_ns == window) {
                    return Err("a motor witness does not end at its board's window");
                }
                if !witness_consistent(m, r) {
                    return Err(
                        "a motor witness's status is not what its counts and its output's timing give",
                    );
                }
                // belt-and-suspenders: the twin's `timer::observer_name`
                // states this path; replay checks an archive without the
                // twin, so it states it again.
                let want = split_pin(pin).map(|(port, n)| {
                    format!("gpioPort{port}.p{}{n}obs", port.to_ascii_lowercase())
                });
                if want.as_deref() != Some(observer.as_str()) || !witnessed.insert(label) {
                    return Err(
                        "a motor witness is not its output's pin's observer, or is there twice",
                    );
                }
            }
            _ => {}
        }
    }
    if checked.len() != solved.len() {
        return Err("a solved motor output has no static timing check");
    }
    Ok(())
}

/// The cycle of the twin's timers (4 ns at Renode's fixed 250 MHz), the
/// stamp grain (10 ns) and the quantum (100 µs) the judge witnesses with.
// belt-and-suspenders: `aethelgard_twin::timer::CYCLE_NS`,
// `aethelgard-cli`'s `STAMP_GRAIN_NS` and `aethelgard_twin::boot::QUANTUM_NS`
// state these; replay checks an archive without the twin or the CLI, so it
// states them again.
const TWIN_CYCLE_NS: u64 = 4;
const TWIN_GRAIN_NS: u64 = 10;
const TWIN_QUANTUM_NS: u64 = 100_000;

/// A boot's windows: 100 ms, or 2 ms for a board with a DShot output
/// (`AET-WO-4300a` D1); and every timer's kernel clock, the reset clock's 64 MHz (RM0433 Table 57), the
/// only clock a solved topology admits.
// belt-and-suspenders: `aethelgard_twin::boot::{MAX_VIRTUAL_NS,
// DSHOT_WINDOW_NS}` and `aethelgard_ir::SolvedClock::reset` state
// these, and `aethelgard_twin::boot::window` the rule that picks the window
// (any DShot output); replay checks an archive without the twin or
// `aethelgard-ir` (RUL-16), so it states them again.
const TWIN_MAX_VIRTUAL_NS: u64 = 100_000_000;
const TWIN_DSHOT_WINDOW_NS: u64 = 2_000_000;
const RESET_KERNEL_HZ: u64 = 64_000_000;

/// Whether kind 25 `check` is exactly the static check of kind 24 `m` on its
/// own kernel clock: the figures and `holds`, re-derived (adversarial pass:
/// a forged `holds` beside its own failing figures).
// belt-and-suspenders: `aethelgard_ir::motor::check_silicon` is the judge's
// check; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// same exact-integer rules again.
fn silicon_check(m: &Record, check: &Record) -> Option<bool> {
    let (
        Record::MotorOutputSolved {
            protocol,
            psc,
            arr,
            ccr,
            pwm,
            ..
        },
        Record::MotorTimingCheck {
            kernel_hz,
            period_ps,
            high_ps,
            nominal_period_ps,
            nominal_high_ps,
            holds,
            ..
        },
    ) = (m, check)
    else {
        return None;
    };
    if *kernel_hz != RESET_KERNEL_HZ {
        return Some(false);
    }
    let kernel = u128::from(*kernel_hz);
    let (psc1, arr1, ccr) = (u128::from(*psc) + 1, u128::from(*arr) + 1, u128::from(*ccr));
    let ps = |cycles: u128| u64::try_from(cycles * 1_000_000_000_000 / kernel.max(1)).ok();
    let bit_rate = match protocol.as_str() {
        "dshot300" => Some(300_000u128),
        "dshot600" => Some(600_000),
        _ => None,
    };
    let (want_holds, nominal, nominal_high) = match (bit_rate, pwm) {
        (Some(rate), None) => {
            let period_ok = (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
            let high_ok = (8 * ccr * psc1 * rate).abs_diff(3 * kernel) <= 4 * psc1 * rate;
            let n = 1_000_000_000_000 / rate;
            (period_ok && high_ok, n, n * 3 / 8)
        }
        (None, Some((rate, lo, hi))) => {
            let rate = u128::from(*rate);
            let period_ok =
                rate > 0 && (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
            let high_ok = ccr * psc1 * 1_000_000 == u128::from(*lo) * kernel;
            let max_ok = u128::from(*hi) * kernel < arr1 * psc1 * 1_000_000;
            (
                period_ok && high_ok && max_ok,
                1_000_000_000_000 / rate.max(1),
                u128::from(*lo) * 1_000_000,
            )
        }
        _ => return Some(false),
    };
    Some(
        ps(arr1 * psc1) == Some(*period_ps)
            && ps(ccr * psc1) == Some(*high_ps)
            && u64::try_from(nominal).ok() == Some(*nominal_period_ps)
            && u64::try_from(nominal_high).ok() == Some(*nominal_high_ps)
            && want_holds == *holds,
    )
}

/// Whether kind 26 `w` is consistent with kind 24 `m` and with its own
/// counts: the model's timing is the output's (`ARR` and `CCR` ticks of
/// `PSC + 1` cycles), and a status is one its counts allow. A passing status
/// needs two rises or more, alternating counts, no shared stamp, a spread
/// under the grain, the last rise within a period and a grain of the
/// window's end, and edges off the grid exactly when `witnessed`.
fn witness_consistent(m: &Record, w: &Record) -> bool {
    let (
        Record::MotorOutputSolved { psc, arr, ccr, .. },
        Record::MotorWitness {
            period_ns,
            high_ns,
            grain_ns,
            quantum_ns,
            status,
            rises,
            falls,
            first_ns,
            last_ns,
            end_ns,
            span_ns,
            off_grid,
            batched,
            ..
        },
    ) = (m, w)
    else {
        return false;
    };
    let cycles = (u64::from(*psc) + 1) * TWIN_CYCLE_NS;
    if *period_ns != u64::from(*arr) * cycles
        || *high_ns != u64::from(*ccr) * cycles
        || *grain_ns != TWIN_GRAIN_NS
        || *quantum_ns != TWIN_QUANTUM_NS
    {
        return false;
    }
    // The fields agree with each other (adversarial F3, on `a2c4fa4`): the
    // first rise no later than the last, which is no later than the end;
    // no more edges off the grid than edges; and a passing sequence's last
    // rise exactly `rises - 1` periods after its first, within its span.
    if first_ns.zip(*last_ns).is_some_and(|(f, l)| f > l)
        || last_ns.is_some_and(|l| l > *end_ns)
        || rises
            .checked_add(*falls)
            .is_none_or(|edges| *off_grid > edges)
    {
        return false;
    }
    let none = *rises == 0 && *falls == 0 && first_ns.is_none();
    let shaped = *rises >= 2
        && *batched == 0
        && first_ns.is_some()
        && span_ns.is_some_and(|s| s < *grain_ns);
    let covers =
        last_ns.is_some_and(|l| l.saturating_add(*period_ns).saturating_add(*grain_ns) >= *end_ns);
    // Checked: a forged count must not wrap onto the honest spacing, nor
    // panic (third adversarial review, F2).
    let spaced = first_ns
        .zip(*last_ns)
        .zip(*span_ns)
        .is_some_and(|((f, l), s)| {
            rises
                .checked_sub(1)
                .and_then(|n| n.checked_mul(*period_ns))
                .is_some_and(|due| (l - f).abs_diff(due) <= s)
        });
    let alternating = *falls == *rises || rises.checked_sub(1) == Some(*falls);
    let passing = shaped && spaced && alternating && covers;
    match status.as_str() {
        "witnessed" => passing && *off_grid > 0,
        "static_only" => passing && *off_grid == 0,
        "unwitnessed" => *rises < 2,
        "unreadable" => none,
        "batched" => *rises >= 2 && *batched > 0,
        "stopped" => shaped && !covers,
        // An unreadable log, or edges off the model's sequence.
        "mismatch" => none || *rises >= 2,
        _ => false,
    }
}

fn check_joins(records: &[Record]) -> Result<(), &'static str> {
    check_motor_joins(records)?;
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
    // One measurement per line, one read per step of a device's script and
    // one witness per step, as the campaign writes them (adversarial A2;
    // `AET-WO-2204`: a bus with two devices has two witnesses;
    // `AET-WO-4210`: a device's steps run identity, configure, data, each
    // at most once, the first being identity; `AET-WO-4220`: several
    // configurations each by its own name, in order).
    let mut seen: BTreeSet<(u8, &str, &str)> = BTreeSet::new();
    let mut last_step: BTreeMap<(u8, &str), usize> = BTreeMap::new();
    for r in records {
        let keys: Vec<(u8, &str, &str)> = match r {
            Record::RiseComputed { bus, net, .. } | Record::LineStuckHigh { bus, net, .. } => {
                vec![(0, bus, net)]
            }
            Record::I2cCheck {
                reference, step, ..
            } => vec![(1, reference, step)],
            Record::I2cWitness {
                reference, step, ..
            } => vec![(2, reference, step)],
            Record::SpiCheck {
                reference, step, ..
            } => vec![(3, reference, step)],
            Record::SpiWitness {
                reference, step, ..
            } => vec![(4, reference, step)],
            _ => vec![],
        };
        for key in keys {
            if !seen.insert(key) {
                return Err("a line measured twice, or a device step read or witnessed twice");
            }
            if key.0 != 0 {
                let Some(index) = step_index(key.2) else {
                    return Err("a step is not identity, a configuration or data");
                };
                let previous = last_step.insert((key.0, key.1), index);
                if previous.map_or(index != 0, |p| index <= p) {
                    return Err("a device's steps are not in the order identity, configure, data");
                }
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
            Record::RiseComputed {
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
                device: path,
                ..
            } => match device.get(reference.as_str()) {
                None => return Err("a witness names no device of a solved bus"),
                Some((p, _)) if p != peripheral => {
                    return Err("a witness is not on its device's solved bus");
                }
                Some(_) if !path.ends_with(&format!(".{}", reference.to_ascii_lowercase())) => {
                    return Err("a witness's model path is not its device's emitted name");
                }
                Some(_) => {}
            },
            _ => {}
        }
    }
    // Each step's read is witnessed, and each witness is of a step's read,
    // joined on (reference, step) (`AET-WO-2204` D1, `AET-WO-4210`).
    let named = |witness: bool| -> BTreeSet<(&str, &str)> {
        records
            .iter()
            .filter_map(|r| match r {
                Record::I2cCheck {
                    reference, step, ..
                } if !witness => Some((reference.as_str(), step.as_str())),
                Record::I2cWitness {
                    reference, step, ..
                } if witness => Some((reference.as_str(), step.as_str())),
                _ => None,
            })
            .collect()
    };
    if named(false) != named(true) {
        return Err("a step's read without its witness, or a witness without its read");
    }
    // The same for SPI (`AET-WO-4200`, adversarial A3): each read joins its
    // solved device (kind 23) at its address and chip select, is witnessed
    // once under its own name behind its controller's multiplexer, and every
    // multiplexer line a witness carries is that multiplexer's; an absent
    // device is a solved one, and its model logged nothing; and no device is
    // both an I2C and an SPI one.
    let mut solved_spi: BTreeMap<&str, (&str, u8, &str)> = BTreeMap::new();
    for r in records {
        if let Record::SpiBusSolved {
            peripheral,
            devices,
        } = r
        {
            for (reference, address, select) in devices {
                if solved_spi
                    .insert(
                        reference.as_str(),
                        (peripheral.as_str(), *address, select.as_str()),
                    )
                    .is_some()
                {
                    return Err("an SPI device is on two solved controllers");
                }
            }
        }
    }
    let mut spi: BTreeMap<&str, &str> = BTreeMap::new();
    let mut spi_steps: BTreeSet<(&str, &str)> = BTreeSet::new();
    for r in records {
        if let Record::SpiCheck {
            reference,
            peripheral,
            select,
            address,
            step,
            ..
        } = r
        {
            if solved_spi.get(reference.as_str())
                != Some(&(peripheral.as_str(), *address, select.as_str()))
            {
                return Err("an SPI read names no solved device at its address and chip select");
            }
            spi.insert(reference, peripheral);
            spi_steps.insert((reference, step));
        }
    }
    let absent: BTreeSet<&str> = records
        .iter()
        .filter_map(|r| match r {
            Record::Scenario { overrides, .. } => Some(overrides),
            _ => None,
        })
        .flatten()
        .filter_map(|o| match o {
            Override::SpiDeviceAbsent { reference } => Some(reference.as_str()),
            _ => None,
        })
        .collect();
    if absent.iter().any(|r| !solved_spi.contains_key(r)) {
        return Err("an absent SPI device is no solved one");
    }
    let mut witnessed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for r in records {
        let Record::SpiWitness {
            peripheral,
            reference,
            device: path,
            step,
            observed,
            unrouted,
            ..
        } = r
        else {
            continue;
        };
        // belt-and-suspenders: the twin's `platform::spi_device_path` states
        // this path; replay checks an archive without the twin, so it states
        // it again (this crate never depends on the twin).
        let c = peripheral.to_ascii_lowercase();
        let mux = format!("{c}.{c}mux");
        let want = format!("{mux}.{}", reference.to_ascii_lowercase());
        if spi.get(reference.as_str()) != Some(&peripheral.as_str()) || *path != want {
            return Err(
                "an SPI witness is not its read's device behind its controller's multiplexer",
            );
        }
        let line = format!("[WARNING] {mux}: Tried to transmit byte 0x");
        if !unrouted.iter().all(|l| l.starts_with(&line)) {
            return Err("an SPI witness carries a line that is not its multiplexer's");
        }
        if absent.contains(reference.as_str()) && observed.as_ref().is_some_and(|o| !o.is_empty()) {
            return Err("an absent SPI device's model logged a transfer");
        }
        witnessed.insert((reference, step));
    }
    if witnessed != spi_steps || spi.keys().any(|r| device.contains_key(r)) {
        return Err("an SPI read without its witness, or a device on two buses");
    }
    Ok(())
}

/// The header artifacts of a campaign run, in the header's sorted order.
pub const ARTIFACTS: [&str; 5] = ["base", "firmware", "scenario", "specification", "topology"];

/// The header's tool names (`AET-WO-2205` §1.1): the engine and Renode judge
/// every run; the bridge only a run that computed a line or a rail (kinds 7,
/// 8 or 16), since it runs only then; and ngspice only a run with an answer
/// that named it, a line (kinds 7 and 8) or a crossing (kind 17).
pub const ENGINE: &str = "aethelgard";
pub const RENODE: &str = "renode";
pub const BRIDGE: &str = "aethelgard-bridge";
pub const NGSPICE: &str = "ngspice";

/// The tools a campaign run's header names, sorted.
pub fn tools_of(records: &[Record]) -> Vec<&'static str> {
    fn line(r: &Record) -> bool {
        matches!(
            r,
            Record::RiseComputed { .. } | Record::LineStuckHigh { .. }
        )
    }
    let any = |f: fn(&Record) -> bool| records.iter().any(f);
    let mut tools = vec![ENGINE];
    if any(|r| line(r) || matches!(r, Record::RailDeclared { .. })) {
        tools.push(BRIDGE);
    }
    if any(|r| line(r) || matches!(r, Record::RailCrossing { .. })) {
        tools.push(NGSPICE);
    }
    tools.push(RENODE);
    tools
}

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
    let tools: Vec<&str> = header.tools.iter().map(|(n, _)| n.as_str()).collect();
    if tools != tools_of(&state.records) {
        return Err("the header's tools are not the ones its records say judged the run");
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
    check_serial_frames(hash(1), &state.records)?;
    Ok(summary)
}
