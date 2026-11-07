//! The pin solver (`AET-WO-1200`, `RUL-02`): a deterministic search from a
//! [`Specification`] and a [`PinTable`] to a [`SolvedTopology`]. The search
//! order, the reserved pins and the refusals, in check order, are in this
//! crate's `README.md`.
//!
//! Every collection is a `BTreeMap`, `BTreeSet` or `Vec`, and the search
//! counts its work in nodes, never in time, so the output is a function of
//! the specification, the table and the hash alone (`INV-01`).

use std::collections::{BTreeMap, BTreeSet};

use crate::{IrValidationError, PinAssignment, PinTable, Signal, SolvedTopology, Specification};

/// Never assigned, and refused as a hand pin: the SWD debug port (`PA13`
/// SWDIO, `PA14` SWCLK), so the board can be programmed, and the oscillator
/// pins (`PC14`/`PC15` LSE, `PH0`/`PH1` HSE), so it keeps its crystals.
pub const RESERVED_PINS: [&str; 6] = ["PA13", "PA14", "PC14", "PC15", "PH0", "PH1"];

/// The values [`solve`] may try, across the whole solve including the
/// minimal-set searches, before refusing with [`SolveError::BudgetExceeded`].
pub const NODE_LIMIT: u64 = 10_000_000;

/// One variant per refusal. `claimants` are JSON paths for the pin refusals
/// (the signal, such as `i2c_buses[0].sda`) and the peripheral refusals (the
/// bus, such as `spi_buses[1]`), and bus ids for the refusals about whole
/// buses that the search reaches.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SolveError {
    /// The specification failed [`Specification::validate`], or the hash is
    /// malformed; `solve` re-checks both rather than trusting its caller.
    #[error("{0}")]
    InvalidSpecification(IrValidationError),
    #[error("the specification is for part `{spec}`, the pin table for `{table}`")]
    PartMismatch { spec: String, table: String },
    #[error("`{claimant}` names pin `{pin}`, which the pin table does not have")]
    UnknownPin { pin: String, claimant: String },
    #[error("`{claimant}` names pin `{pin}`, which is reserved (debug port or oscillator)")]
    ReservedPin { pin: String, claimant: String },
    #[error(
        "`{claimant}` names peripheral `{peripheral}`, which the pin table does not offer for this bus kind"
    )]
    UnknownPeripheral {
        peripheral: String,
        claimant: String,
    },
    #[error("pin `{pin}` is claimed by both `{first}` and `{second}`")]
    PinClaimedTwice {
        pin: String,
        first: String,
        second: String,
    },
    #[error("peripheral `{peripheral}` is claimed by both `{first}` and `{second}`")]
    PeripheralClaimedTwice {
        peripheral: String,
        first: String,
        second: String,
    },
    #[error("no instance can serve bus `{bus}`: {}", .reasons.join("; "))]
    NoInstance { bus: String, reasons: Vec<String> },
    #[error(
        "buses {} cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)",
        quoted(.buses)
    )]
    Infeasible { buses: Vec<String> },
    #[error("the search tried {limit} values without an answer; it was placing bus `{bus}`")]
    BudgetExceeded { bus: String, limit: u64 },
    #[error("`{claimant}` has no free pin left")]
    NoFreePin { claimant: String },
}

fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl SolveError {
    /// The variant name, as the CLI's refusal envelope carries it. An
    /// `InvalidSpecification` reports the validation refusal's own kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidSpecification(e) => e.kind(),
            Self::PartMismatch { .. } => "PartMismatch",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::ReservedPin { .. } => "ReservedPin",
            Self::UnknownPeripheral { .. } => "UnknownPeripheral",
            Self::PinClaimedTwice { .. } => "PinClaimedTwice",
            Self::PeripheralClaimedTwice { .. } => "PeripheralClaimedTwice",
            Self::NoInstance { .. } => "NoInstance",
            Self::Infeasible { .. } => "Infeasible",
            Self::BudgetExceeded { .. } => "BudgetExceeded",
            Self::NoFreePin { .. } => "NoFreePin",
        }
    }

    /// The one offending pin, peripheral, bus or part, or `None`.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::InvalidSpecification(e) => e.id(),
            Self::PartMismatch { spec, .. } => Some(spec),
            Self::UnknownPin { pin, .. }
            | Self::ReservedPin { pin, .. }
            | Self::PinClaimedTwice { pin, .. } => Some(pin),
            Self::UnknownPeripheral { peripheral, .. }
            | Self::PeripheralClaimedTwice { peripheral, .. } => Some(peripheral),
            Self::NoInstance { bus, .. } | Self::BudgetExceeded { bus, .. } => Some(bus),
            Self::Infeasible { .. } | Self::NoFreePin { .. } => None,
        }
    }

    /// Who is in conflict, in document order. Empty for the refusals that
    /// are about the whole specification.
    pub fn claimants(&self) -> Vec<String> {
        match self {
            Self::InvalidSpecification(_) | Self::PartMismatch { .. } => Vec::new(),
            Self::UnknownPin { claimant, .. }
            | Self::ReservedPin { claimant, .. }
            | Self::UnknownPeripheral { claimant, .. }
            | Self::NoFreePin { claimant } => vec![claimant.clone()],
            Self::PinClaimedTwice { first, second, .. }
            | Self::PeripheralClaimedTwice { first, second, .. } => {
                vec![first.clone(), second.clone()]
            }
            Self::NoInstance { bus, .. } | Self::BudgetExceeded { bus, .. } => vec![bus.clone()],
            Self::Infeasible { buses } => buses.clone(),
        }
    }
}

/// [`solve_with_limit`] with [`NODE_LIMIT`].
pub fn solve(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
) -> Result<SolvedTopology, SolveError> {
    solve_with_limit(spec, table, specification_sha256, NODE_LIMIT)
}

/// The solver. `specification_sha256` is the sha256 of the specification's
/// bytes as its caller read them (`INV-03`): the solver never sees the bytes.
/// `node_limit` exists so a test can reach [`SolveError::BudgetExceeded`];
/// every other caller uses [`solve`].
pub fn solve_with_limit(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
    node_limit: u64,
) -> Result<SolvedTopology, SolveError> {
    spec.validate().map_err(SolveError::InvalidSpecification)?;
    let mut out = SolvedTopology {
        schema_version: 1,
        specification_sha256: specification_sha256.to_owned(),
        assignments: BTreeMap::new(),
        gpio: BTreeMap::new(),
    };
    out.validate().map_err(SolveError::InvalidSpecification)?;
    if spec.mcu.part != table.part {
        return Err(SolveError::PartMismatch {
            spec: spec.mcu.part.clone(),
            table: table.part.clone(),
        });
    }

    let model = Model::new(spec, table)?;
    let mut budget = Budget {
        spent: 0,
        limit: node_limit,
    };
    let all: Vec<usize> = (0..model.buses.len()).collect();
    let mut search = Search::new(&model, all.clone());
    let found = search.run(&mut budget);
    if let Some(pos) = search.exceeded {
        return Err(model.budget_error(&search, pos, node_limit));
    }
    if !found {
        return Err(model.minimal_set(all, &mut budget, node_limit));
    }

    for (pos, &b) in search.active.iter().enumerate() {
        let bus = &model.buses[b];
        let (inst, pins) = search.chosen[pos].as_ref().expect("every bus placed");
        for (sig, (pin, af)) in bus.signals.iter().zip(pins) {
            out.assignments.insert(
                sig.net.to_owned(),
                PinAssignment {
                    pin: (*pin).to_owned(),
                    peripheral: inst.clone(),
                    signal: format!("{inst}_{}", sig.name),
                    alternate_function: *af,
                },
            );
        }
    }
    // Every cs last, in document order: a hand pin is already owned; the rest
    // take the first free, unreserved pin in key order. The buses use a fixed
    // number of pins, so this greedy pass never loses a solution.
    for cs in &model.cs {
        let pin = match cs.hand {
            Some(pin) => pin,
            None => {
                let free = table
                    .pins
                    .keys()
                    .map(String::as_str)
                    .find(|p| !RESERVED_PINS.contains(p) && !search.used.contains_key(p));
                let Some(pin) = free else {
                    return Err(SolveError::NoFreePin {
                        claimant: cs.path.clone(),
                    });
                };
                search.used.insert(pin, cs.path.clone());
                pin
            }
        };
        out.gpio.insert(cs.net.to_owned(), pin.to_owned());
    }
    out.validate().map_err(SolveError::InvalidSpecification)?;
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
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

    /// The signals of a bus of this kind, in the order they are placed.
    fn signals(self) -> &'static [&'static str] {
        match self {
            Kind::I2c => &["SDA", "SCL"],
            Kind::Spi => &["SCK", "MOSI", "MISO"],
        }
    }
}

/// One signal of a bus, or one `cs`. `hand` is the table's own key for a
/// hand-set pin, so pins are compared as keys, never as strings a person
/// wrote (`AET-WO-1100` FEEDBACK A5).
struct Sig<'s, 't> {
    name: &'static str,
    net: &'s str,
    hand: Option<&'t str>,
    path: String,
}

struct Bus<'s, 't> {
    kind: Kind,
    id: &'s str,
    peripheral: Option<&'s str>,
    signals: Vec<Sig<'s, 't>>,
}

struct Model<'s, 't> {
    buses: Vec<Bus<'s, 't>>,
    cs: Vec<Sig<'s, 't>>,
    i2c_instances: Vec<String>,
    spi_instances: Vec<String>,
    /// Per (instance, signal): the unreserved pins carrying it, in key order.
    candidates: BTreeMap<(String, &'static str), Vec<(&'t str, u8)>>,
}

/// An instance is offered when the table carries each of its kind's signals
/// for it on some pin. Names sort in byte order.
fn instances(table: &PinTable, kind: Kind) -> Vec<String> {
    let mut found: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for pin in table.pins.values() {
        for name in pin.alternate_functions.keys() {
            let Some((inst, sig)) = name.split_once('_') else {
                continue;
            };
            let numbered = inst
                .strip_prefix(kind.prefix())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            if numbered && kind.signals().contains(&sig) {
                found.entry(inst).or_default().insert(sig);
            }
        }
    }
    found
        .into_iter()
        .filter(|(_, sigs)| sigs.len() == kind.signals().len())
        .map(|(inst, _)| inst.to_owned())
        .collect()
}

impl<'s, 't> Model<'s, 't> {
    /// Builds the model and runs the checks before the search, in the
    /// README's order: hand pins (unknown, reserved, claimed twice), hand
    /// peripherals (unknown, claimed twice), then each bus alone.
    fn new(spec: &'s Specification, table: &'t PinTable) -> Result<Self, SolveError> {
        let mut claimed: BTreeMap<&'t str, String> = BTreeMap::new();
        let mut hand = |signal: &Signal, path: &str| -> Result<Option<&'t str>, SolveError> {
            let Some(written) = signal.pin.as_deref() else {
                return Ok(None);
            };
            let claimant = path.to_owned();
            let Some((key, _)) = table.pins.get_key_value(written) else {
                return Err(SolveError::UnknownPin {
                    pin: written.to_owned(),
                    claimant,
                });
            };
            let key = key.as_str();
            if RESERVED_PINS.contains(&key) {
                return Err(SolveError::ReservedPin {
                    pin: key.to_owned(),
                    claimant,
                });
            }
            if let Some(first) = claimed.insert(key, claimant.clone()) {
                return Err(SolveError::PinClaimedTwice {
                    pin: key.to_owned(),
                    first,
                    second: claimant,
                });
            }
            Ok(Some(key))
        };
        let mut sig = |name: &'static str,
                       signal: &'s Signal,
                       path: String|
         -> Result<Sig<'s, 't>, SolveError> {
            Ok(Sig {
                name,
                net: &signal.net,
                hand: hand(signal, &path)?,
                path,
            })
        };

        let mut buses = Vec::new();
        for (i, b) in spec.i2c_buses.iter().enumerate() {
            let at = format!("i2c_buses[{i}]");
            buses.push(Bus {
                kind: Kind::I2c,
                id: &b.id,
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    sig("SDA", &b.sda, format!("{at}.sda"))?,
                    sig("SCL", &b.scl, format!("{at}.scl"))?,
                ],
            });
        }
        for (i, b) in spec.spi_buses.iter().enumerate() {
            let at = format!("spi_buses[{i}]");
            buses.push(Bus {
                kind: Kind::Spi,
                id: &b.id,
                peripheral: b.peripheral.as_deref(),
                signals: vec![
                    sig("SCK", &b.sck, format!("{at}.sck"))?,
                    sig("MOSI", &b.mosi, format!("{at}.mosi"))?,
                    sig("MISO", &b.miso, format!("{at}.miso"))?,
                ],
            });
        }
        let mut cs = Vec::new();
        for (i, d) in spec.spi_devices.iter().enumerate() {
            cs.push(sig("CS", &d.cs, format!("spi_devices[{i}].cs"))?);
        }

        let i2c_instances = instances(table, Kind::I2c);
        let spi_instances = instances(table, Kind::Spi);
        let mut candidates = BTreeMap::new();
        for (kind, list) in [(Kind::I2c, &i2c_instances), (Kind::Spi, &spi_instances)] {
            for inst in list {
                for name in kind.signals() {
                    let wanted = format!("{inst}_{name}");
                    let pins = table
                        .pins
                        .iter()
                        .filter(|(pin, _)| !RESERVED_PINS.contains(&pin.as_str()))
                        .filter_map(|(pin, p)| {
                            p.alternate_functions
                                .get(&wanted)
                                .map(|af| (pin.as_str(), *af))
                        })
                        .collect();
                    candidates.insert((inst.clone(), *name), pins);
                }
            }
        }
        let model = Model {
            buses,
            cs,
            i2c_instances,
            spi_instances,
            candidates,
        };
        model.check_peripherals(spec)?;
        model.check_each_bus()?;
        Ok(model)
    }

    fn instances_of(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::I2c => &self.i2c_instances,
            Kind::Spi => &self.spi_instances,
        }
    }

    fn pins_for(&self, inst: &str, sig: &'static str) -> &[(&'t str, u8)] {
        self.candidates
            .get(&(inst.to_owned(), sig))
            .map_or(&[], Vec::as_slice)
    }

    fn check_peripherals(&self, spec: &Specification) -> Result<(), SolveError> {
        let paths = (0..spec.i2c_buses.len())
            .map(|i| format!("i2c_buses[{i}]"))
            .chain((0..spec.spi_buses.len()).map(|i| format!("spi_buses[{i}]")));
        let mut taken: BTreeMap<&str, String> = BTreeMap::new();
        for (bus, path) in self.buses.iter().zip(paths) {
            let Some(p) = bus.peripheral else { continue };
            let claimant = path;
            if !self.instances_of(bus.kind).iter().any(|i| i == p) {
                return Err(SolveError::UnknownPeripheral {
                    peripheral: p.to_owned(),
                    claimant,
                });
            }
            if let Some(first) = taken.insert(p, claimant.clone()) {
                return Err(SolveError::PeripheralClaimedTwice {
                    peripheral: p.to_owned(),
                    first,
                    second: claimant,
                });
            }
        }
        Ok(())
    }

    /// Each bus alone: reserved pins and every hand pin honoured, no other
    /// bus's choices. A bus that fails here can never be placed.
    fn check_each_bus(&self) -> Result<(), SolveError> {
        let alone = Search::new(self, (0..self.buses.len()).collect());
        for (b, bus) in self.buses.iter().enumerate() {
            let mut reasons = Vec::new();
            let mut served = false;
            for inst in self.instances_of(bus.kind) {
                if bus.peripheral.is_some_and(|p| p != inst) {
                    continue;
                }
                if alone.placeable(b, inst) {
                    served = true;
                    break;
                }
                let sig = bus
                    .signals
                    .iter()
                    .find(|s| !alone.has_pin(s, inst))
                    .expect("an unplaceable bus has a signal with no pin");
                let pins = self.pins_for(inst, sig.name);
                let name = format!("{inst}_{}", sig.name);
                reasons.push(match sig.hand {
                    Some(h) => format!("{inst}: `{name}` is not on pin `{h}`"),
                    None if pins.is_empty() => {
                        format!("{inst}: the table has no unreserved pin for `{name}`")
                    }
                    None => format!(
                        "{inst}: every pin for `{name}` is held by hand ({})",
                        pins.iter()
                            .map(|(pin, _)| format!("`{pin}` by `{}`", alone.used[pin]))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            if !served {
                return Err(SolveError::NoInstance {
                    bus: bus.id.to_owned(),
                    reasons,
                });
            }
        }
        Ok(())
    }

    fn budget_error(&self, search: &Search, pos: usize, limit: u64) -> SolveError {
        SolveError::BudgetExceeded {
            bus: self.buses[search.active[pos]].id.to_owned(),
            limit,
        }
    }

    /// Deletion: drop each bus in document order if the rest stay
    /// infeasible. The set left is minimal (irreducible), not necessarily the
    /// smallest; it costs one search per bus, inside the same budget.
    fn minimal_set(&self, mut set: Vec<usize>, budget: &mut Budget, limit: u64) -> SolveError {
        let mut i = 0;
        while i < set.len() {
            let mut without = set.clone();
            without.remove(i);
            let mut search = Search::new(self, without.clone());
            let found = search.run(budget);
            if let Some(pos) = search.exceeded {
                return self.budget_error(&search, pos, limit);
            }
            if found {
                i += 1;
            } else {
                set = without;
            }
        }
        SolveError::Infeasible {
            buses: set.iter().map(|&b| self.buses[b].id.to_owned()).collect(),
        }
    }
}

struct Budget {
    spent: u64,
    limit: u64,
}

/// A placed bus: its instance, and per signal its `(pin, af)`.
type Placed<'t> = (String, Vec<(&'t str, u8)>);

/// One search over the buses in `active`, in that order.
struct Search<'m, 's, 't> {
    model: &'m Model<'s, 't>,
    active: Vec<usize>,
    /// Pin to owner path. Every hand pin of an active bus or a `cs` is owned
    /// from the start.
    used: BTreeMap<&'t str, String>,
    used_instances: BTreeSet<String>,
    chosen: Vec<Option<Placed<'t>>>,
    budget: Budget,
    /// The position being placed when the budget ran out.
    exceeded: Option<usize>,
}

impl<'m, 's, 't> Search<'m, 's, 't> {
    fn new(model: &'m Model<'s, 't>, active: Vec<usize>) -> Self {
        let mut used = BTreeMap::new();
        let hand = active
            .iter()
            .flat_map(|&b| &model.buses[b].signals)
            .chain(&model.cs);
        for sig in hand {
            if let Some(pin) = sig.hand {
                used.insert(pin, sig.path.clone());
            }
        }
        let n = active.len();
        Search {
            model,
            active,
            used,
            used_instances: BTreeSet::new(),
            chosen: vec![None; n],
            budget: Budget { spent: 0, limit: 0 },
            exceeded: None,
        }
    }

    /// Runs the search inside `budget`, and charges what it spent.
    fn run(&mut self, budget: &mut Budget) -> bool {
        self.budget = Budget {
            spent: budget.spent,
            limit: budget.limit,
        };
        let found = self.consistent(0) && self.bus(0);
        budget.spent = self.budget.spent;
        found
    }

    /// Counts one value tried; false once the budget is spent.
    fn tick(&mut self, pos: usize) -> bool {
        self.budget.spent += 1;
        if self.budget.spent > self.budget.limit {
            self.exceeded.get_or_insert(pos);
        }
        self.exceeded.is_none()
    }

    fn free(&self, pin: &str, owner: &str) -> bool {
        self.used.get(pin).is_none_or(|o| o == owner)
    }

    /// The pins `sig` could take on `inst` right now.
    fn pins<'a>(&'a self, sig: &'a Sig<'s, 't>, inst: &str) -> impl Iterator<Item = &'t str> + 'a {
        self.model
            .pins_for(inst, sig.name)
            .iter()
            .map(|(pin, _)| *pin)
            .filter(move |pin| sig.hand.is_none_or(|h| h == *pin) && self.free(pin, &sig.path))
    }

    fn has_pin(&self, sig: &Sig<'s, 't>, inst: &str) -> bool {
        self.pins(sig, inst).next().is_some()
    }

    fn placeable(&self, b: usize, inst: &str) -> bool {
        let bus = &self.model.buses[b];
        bus.peripheral.is_none_or(|p| p == inst)
            && !self.used_instances.contains(inst)
            && bus.signals.iter().all(|s| self.has_pin(s, inst))
    }

    /// The checks every node passes before the search goes deeper, over the
    /// buses from position `from` on: each still has a placeable instance;
    /// they can take distinct instances; and their signals can take distinct
    /// free pins, each from the union over its bus's placeable instances.
    /// Each only answers yes or no, so none changes which answer is found.
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
        if options.iter().any(Vec::is_empty) || !matching(&options) {
            return false;
        }
        let mut signal_pins: Vec<Vec<&str>> = Vec::new();
        for (&b, insts) in rest.iter().zip(&options) {
            for sig in &self.model.buses[b].signals {
                let pins: BTreeSet<&str> = insts.iter().flat_map(|i| self.pins(sig, i)).collect();
                signal_pins.push(pins.into_iter().collect());
            }
        }
        matching(&signal_pins)
    }

    fn bus(&mut self, pos: usize) -> bool {
        if pos == self.active.len() {
            return true;
        }
        let model = self.model;
        let b = self.active[pos];
        for inst in model.instances_of(model.buses[b].kind) {
            if !self.tick(pos) {
                return false;
            }
            if !self.placeable(b, inst) {
                continue;
            }
            self.used_instances.insert(inst.clone());
            if self.signal(pos, inst, 0, &mut Vec::new()) {
                return true;
            }
            self.used_instances.remove(inst);
        }
        false
    }

    fn signal(&mut self, pos: usize, inst: &str, k: usize, pins: &mut Vec<(&'t str, u8)>) -> bool {
        let model = self.model;
        let bus = &model.buses[self.active[pos]];
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
        let sig = &bus.signals[k];
        for &(pin, af) in model.pins_for(inst, sig.name) {
            if !self.tick(pos) {
                return false;
            }
            if sig.hand.is_some_and(|h| h != pin) || !self.free(pin, &sig.path) {
                continue;
            }
            let owned = self.used.contains_key(pin);
            self.used.insert(pin, sig.path.clone());
            pins.push((pin, af));
            if self.signal(pos, inst, k + 1, pins) {
                return true;
            }
            pins.pop();
            if !owned {
                self.used.remove(pin);
            }
        }
        false
    }
}

/// Whether every row can take a distinct value (Hall's condition), by
/// augmenting paths in fixed order. Only the yes or no leaves this function.
fn matching(options: &[Vec<&str>]) -> bool {
    fn augment<'a>(
        row: usize,
        options: &[Vec<&'a str>],
        owner: &mut BTreeMap<&'a str, usize>,
        seen: &mut BTreeSet<&'a str>,
    ) -> bool {
        for &value in &options[row] {
            if seen.insert(value) {
                let prev = owner.get(value).copied();
                if prev.is_none_or(|p| augment(p, options, owner, seen)) {
                    owner.insert(value, row);
                    return true;
                }
            }
        }
        false
    }
    let mut owner = BTreeMap::new();
    (0..options.len()).all(|row| augment(row, options, &mut owner, &mut BTreeSet::new()))
}
