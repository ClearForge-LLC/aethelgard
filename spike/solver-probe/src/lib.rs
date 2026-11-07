//! AET-WO-1200 §0 spike: the pin search as S1 describes it, measured. Not the
//! build: the build half writes `solve` in `aethelgard-ir` and deletes this.
//!
//! Order (S1, plus one proposal): buses in document order, I2C before SPI;
//! per bus, instances in ascending name order, then its signals in the order
//! `sda, scl` or `sck, mosi, miso`, each over the table's key order; then every
//! SPI device's `cs` in document order (PROPOSED: cs last, see FEEDBACK).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::{PinAssignment, PinTable, Signal, Specification};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// The SWD port (`AET-WO-1101` FEEDBACK 4, as ruled at review of PR #27).
pub const RESERVED: [&str; 2] = ["PA13", "PA14"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    I2c,
    Spi,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::I2c => "I2C",
            Kind::Spi => "SPI",
        }
    }
    fn signals(self) -> &'static [&'static str] {
        match self {
            Kind::I2c => &["SDA", "SCL"],
            Kind::Spi => &["SCK", "MOSI", "MISO"],
        }
    }
}

/// The refusals S3 asks for, with the extra one the model needs (flagged in
/// FEEDBACK): two buses hand-set to one instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    UnknownPin {
        pin: String,
        path: String,
    },
    ReservedPin {
        pin: String,
        path: String,
    },
    UnknownPeripheral {
        peripheral: String,
        path: String,
    },
    PinClaimedTwice {
        pin: String,
        claimants: [String; 2],
    },
    PeripheralClaimedTwice {
        peripheral: String,
        claimants: [String; 2],
    },
    NoInstance {
        bus: String,
        why: Vec<String>,
    },
    Infeasible {
        buses: Vec<String>,
    },
    BudgetExceeded {
        limit: u64,
        bus: String,
    },
}

impl SolveError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnknownPin { .. } => "UnknownPin",
            Self::ReservedPin { .. } => "ReservedPin",
            Self::UnknownPeripheral { .. } => "UnknownPeripheral",
            Self::PinClaimedTwice { .. } => "PinClaimedTwice",
            Self::PeripheralClaimedTwice { .. } => "PeripheralClaimedTwice",
            Self::NoInstance { .. } => "NoInstance",
            Self::Infeasible { .. } => "Infeasible",
            Self::BudgetExceeded { .. } => "BudgetExceeded",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::UnknownPin { pin, path } => {
                format!("`{path}` names pin `{pin}`, which the pin table does not have")
            }
            Self::ReservedPin { pin, path } => {
                format!("`{path}` names pin `{pin}`, which is reserved for the SWD debug port")
            }
            Self::UnknownPeripheral { peripheral, path } => format!(
                "`{path}` names peripheral `{peripheral}`, which the pin table does not offer for this bus kind"
            ),
            Self::PinClaimedTwice { pin, claimants } => format!(
                "pin `{pin}` is claimed by both `{}` and `{}`",
                claimants[0], claimants[1]
            ),
            Self::PeripheralClaimedTwice {
                peripheral,
                claimants,
            } => format!(
                "peripheral `{peripheral}` is claimed by both `{}` and `{}`",
                claimants[0], claimants[1]
            ),
            Self::NoInstance { bus, why } => {
                format!("no instance can serve bus `{bus}`: {}", why.join("; "))
            }
            Self::Infeasible { buses } => format!(
                "buses {} cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)",
                buses
                    .iter()
                    .map(|b| format!("`{b}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::BudgetExceeded { limit, bus } => format!(
                "the search tried {limit} values without an answer; it was placing bus `{bus}`"
            ),
        }
    }
}

/// S4 prototype: `SolvedTopology` plus the architect's default `gpio` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpikeTopology {
    pub schema_version: u32,
    pub specification_sha256: String,
    pub assignments: BTreeMap<String, PinAssignment>,
    pub gpio: BTreeMap<String, String>,
}

/// Which search to run. All off is the WO's: forward checking only. Each flag
/// only cuts subtrees that hold no solution, so none may change the answer;
/// the probe checks that by comparing output digests across settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pruning {
    /// The buses still to place can take distinct instances (Hall, by matching).
    pub instance_matching: bool,
    /// Their signals can take distinct free pins, each from the union of its
    /// candidates over the bus's still-placeable instances (Hall, by matching).
    pub pin_matching: bool,
    /// Remember a failed suffix by (position, used pins it could touch, used
    /// instances), and never search that state twice.
    pub memo: bool,
    /// Values tried per search before refusing with `BudgetExceeded`; 0 is
    /// none. A count, never wall time, so the refusal is host-independent.
    pub node_limit: u64,
}

impl Pruning {
    pub fn label(self) -> String {
        let mut l = vec!["fc"];
        if self.instance_matching {
            l.push("im");
        }
        if self.pin_matching {
            l.push("pm");
        }
        if self.memo {
            l.push("memo");
        }
        l.join("+")
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    /// Values tried: an instance for a bus, or a pin for a signal.
    pub nodes: u64,
    /// Full searches run (1, plus one per deletion step of the minimal set).
    pub searches: u64,
}

struct BusVar<'s> {
    kind: Kind,
    id: &'s str,
    peripheral: Option<&'s str>,
    /// (`SDA`, the signal, its JSON path)
    signals: Vec<(&'static str, &'s Signal, String)>,
}

struct Model<'s, 't> {
    table: &'t PinTable,
    buses: Vec<BusVar<'s>>,
    /// (cs signal, path)
    cs: Vec<(&'s Signal, String)>,
    /// Instances per kind, ascending.
    instances_i2c: Vec<String>,
    instances_spi: Vec<String>,
    /// Candidate pins per (instance, SIGNAL): `(pin, af)` in table key order,
    /// reserved pins already dropped.
    candidates: BTreeMap<(String, &'static str), Vec<(&'t str, u8)>>,
}

fn instances(table: &PinTable, kind: Kind) -> Vec<String> {
    // An instance is offered when the table carries every one of its kind's
    // signals for it somewhere: `I2C3` needs some `I2C3_SDA` and some `I2C3_SCL`.
    let mut found: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for pin in table.pins.values() {
        for name in pin.alternate_functions.keys() {
            let Some((inst, sig)) = name.split_once('_') else {
                continue;
            };
            let numbered = inst
                .strip_prefix(kind.prefix())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            if numbered && let Some(s) = kind.signals().iter().find(|s| **s == sig) {
                found.entry(inst.to_owned()).or_default().insert(s);
            }
        }
    }
    found
        .into_iter()
        .filter(|(_, sigs)| sigs.len() == kind.signals().len())
        .map(|(inst, _)| inst)
        .collect()
}

impl<'s, 't> Model<'s, 't> {
    fn new(spec: &'s Specification, table: &'t PinTable) -> Self {
        let mut buses = Vec::new();
        for (i, b) in spec.i2c_buses.iter().enumerate() {
            buses.push(BusVar {
                kind: Kind::I2c,
                id: &b.id,
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    ("SDA", &b.sda, format!("i2c_buses[{i}].sda")),
                    ("SCL", &b.scl, format!("i2c_buses[{i}].scl")),
                ],
            });
        }
        for (i, b) in spec.spi_buses.iter().enumerate() {
            buses.push(BusVar {
                kind: Kind::Spi,
                id: &b.id,
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    ("SCK", &b.sck, format!("spi_buses[{i}].sck")),
                    ("MOSI", &b.mosi, format!("spi_buses[{i}].mosi")),
                    ("MISO", &b.miso, format!("spi_buses[{i}].miso")),
                ],
            });
        }
        let cs = spec
            .spi_devices
            .iter()
            .enumerate()
            .map(|(i, d)| (&d.cs, format!("spi_devices[{i}].cs")))
            .collect();
        let instances_i2c = instances(table, Kind::I2c);
        let instances_spi = instances(table, Kind::Spi);
        let mut candidates = BTreeMap::new();
        for (kind, list) in [(Kind::I2c, &instances_i2c), (Kind::Spi, &instances_spi)] {
            for inst in list {
                for sig in kind.signals() {
                    let wanted = format!("{inst}_{sig}");
                    let pins = table
                        .pins
                        .iter()
                        .filter(|(name, _)| !RESERVED.contains(&name.as_str()))
                        .filter_map(|(name, p)| {
                            p.alternate_functions
                                .get(&wanted)
                                .map(|af| (name.as_str(), *af))
                        })
                        .collect();
                    candidates.insert((inst.clone(), *sig), pins);
                }
            }
        }
        Model {
            table,
            buses,
            cs,
            instances_i2c,
            instances_spi,
            candidates,
        }
    }

    fn instances_of(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::I2c => &self.instances_i2c,
            Kind::Spi => &self.instances_spi,
        }
    }

    /// Steps A to D, before any search, each in document order.
    fn precheck(&self) -> Result<(), SolveError> {
        // A: every hand pin resolves to a table key (A5: compare keys, never
        // strings) and is not reserved. Buses, then cs.
        let hand_pins = self
            .buses
            .iter()
            .flat_map(|b| b.signals.iter().map(|(_, s, p)| (*s, p)))
            .chain(self.cs.iter().map(|(s, p)| (*s, p)));
        let mut claimed: BTreeMap<&str, &String> = BTreeMap::new();
        for (signal, path) in hand_pins {
            let Some(pin) = signal.pin.as_deref() else {
                continue;
            };
            let Some((key, _)) = self.table.pins.get_key_value(pin) else {
                return Err(SolveError::UnknownPin {
                    pin: pin.into(),
                    path: format!("{path}.pin"),
                });
            };
            if RESERVED.contains(&key.as_str()) {
                return Err(SolveError::ReservedPin {
                    pin: key.clone(),
                    path: format!("{path}.pin"),
                });
            }
            // C, folded into the same walk so the first repeat in document
            // order is the one named.
            if let Some(first) = claimed.insert(key, path) {
                return Err(SolveError::PinClaimedTwice {
                    pin: key.clone(),
                    claimants: [first.clone(), path.clone()],
                });
            }
        }
        // B: every hand peripheral is an instance of the bus's kind; then no
        // two buses hand-set one instance.
        let mut taken: BTreeMap<&str, String> = BTreeMap::new();
        for (bus, path) in self.buses.iter().zip(self.bus_paths()) {
            let Some(p) = bus.peripheral else { continue };
            if !self.instances_of(bus.kind).iter().any(|i| i == p) {
                return Err(SolveError::UnknownPeripheral {
                    peripheral: p.into(),
                    path: format!("{path}.peripheral"),
                });
            }
            if let Some(first) = taken.insert(p, path.clone()) {
                return Err(SolveError::PeripheralClaimedTwice {
                    peripheral: p.into(),
                    claimants: [first, path],
                });
            }
        }
        // D: each bus alone, honouring reserved pins and every hand pin (its
        // own, and those other signals hold), but no other bus's choices.
        let alone = Search::new(self, (0..self.buses.len()).collect(), Pruning::default());
        for (b, bus) in self.buses.iter().enumerate() {
            let mut why = Vec::new();
            let mut served = false;
            for inst in self.instances_of(bus.kind) {
                if bus.peripheral.is_some_and(|p| p != inst) {
                    continue;
                }
                if alone.placeable(b, inst) {
                    served = true;
                    break;
                }
                let (sig, s, _) = bus
                    .signals
                    .iter()
                    .find(|(sig, s, path)| {
                        !self.candidates[&(inst.clone(), *sig)]
                            .iter()
                            .any(|(pin, _)| {
                                s.pin.as_deref().is_none_or(|h| h == *pin) && alone.free(pin, path)
                            })
                    })
                    .expect("not placeable means some signal has no pin");
                let pins = &self.candidates[&(inst.clone(), *sig)];
                why.push(match s.pin.as_deref() {
                    Some(h) => format!("{inst}: `{inst}_{sig}` is not on pin `{h}`"),
                    None if pins.is_empty() => {
                        format!("{inst}: the table has no unreserved pin for `{inst}_{sig}`")
                    }
                    None => format!(
                        "{inst}: every pin for `{inst}_{sig}` is held by hand ({})",
                        pins.iter()
                            .map(|(pin, _)| format!("`{pin}` by `{}`", alone.used_pins[pin]))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            if !served {
                return Err(SolveError::NoInstance {
                    bus: bus.id.into(),
                    why,
                });
            }
        }
        Ok(())
    }

    fn bus_paths(&self) -> Vec<String> {
        let (mut i, mut s) = (0, 0);
        self.buses
            .iter()
            .map(|b| match b.kind {
                Kind::I2c => {
                    i += 1;
                    format!("i2c_buses[{}]", i - 1)
                }
                Kind::Spi => {
                    s += 1;
                    format!("spi_buses[{}]", s - 1)
                }
            })
            .collect()
    }
}

/// A placed bus: its instance, and per signal its `(pin, af)`.
type Placed<'t> = (String, Vec<(&'t str, u8)>);

/// One search over the buses in `active` (indices into `model.buses`).
struct Search<'m, 's, 't> {
    model: &'m Model<'s, 't>,
    active: Vec<usize>,
    pruning: Pruning,
    /// Pin -> owner: a hand pin is owned by its signal path from the start.
    used_pins: BTreeMap<&'t str, String>,
    used_instances: BTreeSet<String>,
    /// Per active bus: (instance, per signal (pin, af)).
    chosen: Vec<Option<Placed<'t>>>,
    nodes: u64,
    /// Per position: every pin a bus from there on could take.
    relevant: Vec<BTreeSet<&'t str>>,
    failed: BTreeSet<(usize, Vec<&'t str>, Vec<String>)>,
    /// Where the budget ran out, if it did.
    exceeded: Option<usize>,
}

impl<'m, 's, 't> Search<'m, 's, 't> {
    fn new(model: &'m Model<'s, 't>, active: Vec<usize>, pruning: Pruning) -> Self {
        let mut used_pins = BTreeMap::new();
        for &b in &active {
            for (_, s, path) in &model.buses[b].signals {
                if let Some(h) = s.pin.as_deref() {
                    let key = model.table.pins.get_key_value(h).map(|(k, _)| k.as_str());
                    used_pins.insert(key.expect("prechecked"), path.clone());
                }
            }
        }
        for (s, path) in &model.cs {
            if let Some(h) = s.pin.as_deref() {
                let key = model.table.pins.get_key_value(h).map(|(k, _)| k.as_str());
                used_pins.insert(key.expect("prechecked"), path.clone());
            }
        }
        let n = active.len();
        let mut relevant = vec![BTreeSet::new(); n + 1];
        for pos in (0..n).rev() {
            let bus = &model.buses[active[pos]];
            let mut here = relevant[pos + 1].clone();
            for inst in model.instances_of(bus.kind) {
                for (sig, _, _) in &bus.signals {
                    here.extend(
                        model.candidates[&(inst.clone(), *sig)]
                            .iter()
                            .map(|(p, _)| *p),
                    );
                }
            }
            relevant[pos] = here;
        }
        Search {
            model,
            active,
            pruning,
            used_pins,
            used_instances: BTreeSet::new(),
            chosen: vec![None; n],
            nodes: 0,
            relevant,
            failed: BTreeSet::new(),
            exceeded: None,
        }
    }

    /// Counts one value tried; false once the budget is spent.
    fn tick(&mut self, pos: usize) -> bool {
        self.nodes += 1;
        if self.pruning.node_limit != 0 && self.nodes > self.pruning.node_limit {
            self.exceeded.get_or_insert(pos);
        }
        self.exceeded.is_none()
    }

    fn free(&self, pin: &str, owner: &str) -> bool {
        self.used_pins.get(pin).is_none_or(|o| o == owner)
    }

    /// Can bus `b` take instance `inst` with the pins free right now?
    fn placeable(&self, b: usize, inst: &str) -> bool {
        let bus = &self.model.buses[b];
        if bus.peripheral.is_some_and(|p| p != inst) || self.used_instances.contains(inst) {
            return false;
        }
        bus.signals.iter().all(|(sig, s, path)| {
            self.model.candidates[&(inst.to_owned(), *sig)]
                .iter()
                .any(|(pin, _)| s.pin.as_deref().is_none_or(|h| h == *pin) && self.free(pin, path))
        })
    }

    /// Forward check over the buses after position `from`; with matching, the
    /// buses still to place must also take distinct instances (Hall).
    fn consistent(&self, from: usize) -> bool {
        let rest = &self.active[from..];
        let options: Vec<Vec<&str>> = rest
            .iter()
            .map(|&b| {
                self.model
                    .instances_of(self.model.buses[b].kind)
                    .iter()
                    .filter(|i| self.placeable(b, i))
                    .map(String::as_str)
                    .collect()
            })
            .collect();
        if options.iter().any(Vec::is_empty) {
            return false;
        }
        if self.pruning.instance_matching && !perfect_matching(&options) {
            return false;
        }
        if self.pruning.pin_matching {
            let mut signal_pins: Vec<Vec<&str>> = Vec::new();
            for (&b, insts) in rest.iter().zip(&options) {
                for (sig, s, path) in &self.model.buses[b].signals {
                    let mut pins = BTreeSet::new();
                    for inst in insts {
                        for (pin, _) in &self.model.candidates[&((*inst).to_owned(), *sig)] {
                            if s.pin.as_deref().is_none_or(|h| h == *pin) && self.free(pin, path) {
                                pins.insert(*pin);
                            }
                        }
                    }
                    signal_pins.push(pins.into_iter().collect());
                }
            }
            if !perfect_matching(&signal_pins) {
                return false;
            }
        }
        true
    }

    fn state(&self, pos: usize) -> (usize, Vec<&'t str>, Vec<String>) {
        let pins = self
            .used_pins
            .keys()
            .filter(|p| self.relevant[pos].contains(*p))
            .copied()
            .collect();
        (pos, pins, self.used_instances.iter().cloned().collect())
    }

    fn bus(&mut self, pos: usize) -> bool {
        if pos == self.active.len() {
            return true;
        }
        let key = self.pruning.memo.then(|| self.state(pos));
        if key.as_ref().is_some_and(|k| self.failed.contains(k)) {
            return false;
        }
        let b = self.active[pos];
        let bus = &self.model.buses[b];
        for inst in self.model.instances_of(bus.kind) {
            if !self.tick(pos) {
                return false;
            }
            if !self.placeable(b, inst) {
                continue;
            }
            self.used_instances.insert(inst.clone());
            let mut pins = Vec::new();
            if self.signal(pos, inst, 0, &mut pins) {
                return true;
            }
            self.used_instances.remove(inst);
        }
        // A spent budget proves nothing, so it is never remembered as failure.
        if let Some(k) = key.filter(|_| self.exceeded.is_none()) {
            self.failed.insert(k);
        }
        false
    }

    fn signal(&mut self, pos: usize, inst: &str, k: usize, pins: &mut Vec<(&'t str, u8)>) -> bool {
        let b = self.active[pos];
        let model = self.model;
        let bus = &model.buses[b];
        if k == bus.signals.len() {
            if !self.consistent(pos + 1) {
                return false;
            }
            self.chosen[pos] = Some((inst.to_owned(), pins.clone()));
            if self.bus(pos + 1) {
                return true;
            }
            self.chosen[pos] = None;
            return false;
        }
        let (sig, s, path) = &bus.signals[k];
        for &(pin, af) in &model.candidates[&(inst.to_owned(), *sig)] {
            if !self.tick(pos) {
                return false;
            }
            if s.pin.as_deref().is_some_and(|h| h != pin) || !self.free(pin, path) {
                continue;
            }
            let hand = self.used_pins.contains_key(pin);
            self.used_pins.insert(pin, path.clone());
            pins.push((pin, af));
            if self.signal(pos, inst, k + 1, pins) {
                return true;
            }
            pins.pop();
            if !hand {
                self.used_pins.remove(pin);
            }
        }
        false
    }
}

/// Kuhn's augmenting paths, in fixed order. Only answers yes or no, so its
/// order cannot leak into the output.
fn perfect_matching(options: &[Vec<&str>]) -> bool {
    fn augment<'a>(
        v: usize,
        options: &[Vec<&'a str>],
        owner: &mut BTreeMap<&'a str, usize>,
        seen: &mut BTreeSet<&'a str>,
    ) -> bool {
        for &inst in &options[v] {
            if seen.insert(inst) {
                let prev = owner.get(inst).copied();
                if prev.is_none_or(|p| augment(p, options, owner, seen)) {
                    owner.insert(inst, v);
                    return true;
                }
            }
        }
        false
    }
    let mut owner = BTreeMap::new();
    (0..options.len()).all(|v| augment(v, options, &mut owner, &mut BTreeSet::new()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `bytes` are the specification as read; its hash goes in the output as is.
pub fn solve(
    bytes: &[u8],
    table: &PinTable,
    pruning: Pruning,
    stats: &mut Stats,
) -> Result<SpikeTopology, SolveError> {
    let spec: Specification = serde_json::from_slice(bytes).expect("probe input parses");
    spec.validate().expect("probe input validates");
    let model = Model::new(&spec, table);
    model.precheck()?;

    let all: Vec<usize> = (0..model.buses.len()).collect();
    let mut search = Search::new(&model, all.clone(), pruning);
    stats.searches += 1;
    let found = search.consistent(0) && search.bus(0);
    stats.nodes += search.nodes;
    if let Some(pos) = search.exceeded {
        return Err(budget(&model, &search, pos));
    }
    if !found {
        return Err(minimal_set(&model, all, pruning, stats));
    }

    let mut assignments = BTreeMap::new();
    for (pos, &b) in search.active.iter().enumerate() {
        let bus = &model.buses[b];
        let (inst, pins) = search.chosen[pos].clone().expect("placed");
        for ((sig, s, _), (pin, af)) in bus.signals.iter().zip(pins) {
            assignments.insert(
                s.net.clone(),
                PinAssignment {
                    pin: pin.into(),
                    peripheral: inst.clone(),
                    signal: format!("{inst}_{sig}"),
                    alternate_function: af,
                },
            );
        }
    }
    // cs: hand pins are fixed and already owned; the rest take the first free,
    // unreserved pin in key order. Buses always use the same number of pins,
    // so greedy here never loses a solution backtracking would find.
    let mut gpio = BTreeMap::new();
    for (s, path) in &model.cs {
        let pin = match s.pin.as_deref() {
            Some(h) => h,
            None => {
                let Some(pin) = table
                    .pins
                    .keys()
                    .map(String::as_str)
                    .find(|p| !RESERVED.contains(p) && !search.used_pins.contains_key(p))
                else {
                    return Err(SolveError::Infeasible {
                        buses: vec![format!("{path} (no free pin left)")],
                    });
                };
                search.used_pins.insert(pin, path.clone());
                pin
            }
        };
        gpio.insert(s.net.clone(), pin.to_owned());
    }
    Ok(SpikeTopology {
        schema_version: 1,
        specification_sha256: sha256_hex(bytes),
        assignments,
        gpio,
    })
}

/// Deletion-based: drop each bus in document order if the rest stay
/// infeasible. The result is minimal (irreducible), not necessarily minimum.
fn minimal_set(
    model: &Model,
    mut set: Vec<usize>,
    pruning: Pruning,
    stats: &mut Stats,
) -> SolveError {
    let mut i = 0;
    while i < set.len() {
        let mut without = set.clone();
        without.remove(i);
        let mut s = Search::new(model, without.clone(), pruning);
        stats.searches += 1;
        let ok = s.consistent(0) && s.bus(0);
        stats.nodes += s.nodes;
        if let Some(pos) = s.exceeded {
            return budget(model, &s, pos);
        }
        if ok {
            i += 1;
        } else {
            set = without;
        }
    }
    SolveError::Infeasible {
        buses: set.iter().map(|&b| model.buses[b].id.to_owned()).collect(),
    }
}

fn budget(model: &Model, search: &Search, pos: usize) -> SolveError {
    SolveError::BudgetExceeded {
        limit: search.pruning.node_limit,
        bus: model.buses[search.active[pos]].id.to_owned(),
    }
}
