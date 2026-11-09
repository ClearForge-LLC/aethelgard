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
use crate::record::{BusLine, Line, Override, Record};
use crate::replay::State;

/// The only scenario format version.
pub const SCENARIO_VERSION: u32 = 1;
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawScenario {
    schema_version: u32,
    name: String,
    base: String,
    expect: Expect,
    expect_findings: Vec<String>,
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

object_only!(RawOverride, RawScenario);

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
    } = raw;
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
            "an override is exactly {bus, pull_up_ohms}, {net, max_capacitance_femtofarads} or {bus, line_held_low}"
                .to_owned(),
        )),
    }
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
        if raw.schema_version != SCENARIO_VERSION {
            return Err(ScenarioError::new(
                "UnsupportedSchemaVersion",
                None,
                format!(
                    "scenario schema_version {} is not {SCENARIO_VERSION}",
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
            sha256: Sha256::digest(bytes).into(),
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
/// The order is: the scenario, the solved topology, the solved I2C buses
/// (kind 15, sorted by bus), the analog records (kinds 7 and 8), the bus
/// faults, the boot's witnesses (kind 10, then 11, 12 and 13 in that order),
/// and the verdict, last and once.
pub fn check_run(records: &[Record]) -> Result<RunSummary, &'static str> {
    fn rank(r: &Record) -> u8 {
        match r {
            Record::Scenario { .. } => 0,
            Record::Solved { .. } => 1,
            Record::I2cBusSolved { .. } => 2,
            Record::RiseMeasured { .. } | Record::LineStuckHigh { .. } => 3,
            Record::BusFault { .. } => 4,
            Record::BootObserved { .. } => 5,
            Record::RegisterObserved { .. } => 6,
            Record::I2cCheck { .. } => 7,
            Record::I2cWitness { .. } => 8,
            Record::Verdict { .. } => 9,
        }
    }
    let once = |n: u8| records.iter().filter(|r| rank(r) == n).count() == 1;
    if records.is_empty() || !records.windows(2).all(|w| rank(&w[0]) <= rank(&w[1])) {
        return Err("the records are not in the campaign's order");
    }
    if !(once(0) && once(1) && once(5) && once(9)) {
        return Err("a run holds exactly one scenario, topology, boot and verdict");
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
    let matched = failing_kinds == *expect_findings;
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
    let unique = |items: Vec<&str>| {
        let n = items.len();
        items.into_iter().collect::<BTreeSet<_>>().len() == n
    };
    let lines = || solved.iter().flat_map(|b| b.2.iter());
    if !unique(lines().map(|l| l.net.as_str()).collect())
        || !unique(lines().map(|l| l.pin.as_str()).collect())
        || !unique(solved.iter().map(|b| b.1).collect())
    {
        return Err("two solved buses share a net, a pin or a controller");
    }
    if !solved.iter().all(|b| {
        let addresses: BTreeSet<u8> = b.3.iter().map(|d| d.1).collect();
        addresses.len() == b.3.len()
    }) {
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
    let (
        Record::Scenario { sha256, .. },
        Record::Solved {
            specification_sha256,
            topology_sha256,
        },
    ) = (&state.records[0], &state.records[1])
    else {
        unreachable!("check_run puts the scenario and the topology first");
    };
    if hash(2) != sha256 || hash(3) != specification_sha256 || hash(4) != topology_sha256 {
        return Err("the header's hashes are not the ones its records name");
    }
    if run_id(hash(2), hash(3), hash(1)) != header.run_id {
        return Err("the run id is not the one its scenario, specification and firmware give");
    }
    Ok(summary)
}
