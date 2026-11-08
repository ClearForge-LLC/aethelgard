//! The pin solver (`AET-WO-1200`, `RUL-02`): a deterministic search from a
//! [`Specification`] and a [`PinTable`] to a [`SolvedTopology`]. The search
//! order, the reserved pins and the refusals, in check order, are in this
//! crate's `README.md`, as is why its output depends on its inputs alone.

use std::collections::{BTreeMap, BTreeSet};

use crate::{IrValidationError, PinAssignment, PinTable, Signal, SolvedTopology, Specification};

/// Never assigned, and refused as a hand pin: the SWD debug port (`PA13`
/// SWDIO, `PA14` SWCLK), so the board can be programmed, and the oscillator
/// pins (`PC14`/`PC15` LSE, `PH0`/`PH1` HSE), so it keeps its crystals.
pub const RESERVED_PINS: [&str; 6] = ["PA13", "PA14", "PC14", "PC15", "PH0", "PH1"];

/// The values [`solve`] may try, across the whole solve including the
/// minimal-set searches, before refusing with [`SolveError::BudgetExceeded`].
pub const NODE_LIMIT: u64 = 10_000_000;

/// One variant per refusal; what `claimants()` names is in the README.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SolveError {
    /// The specification failed [`Specification::validate`], or the hash is
    /// malformed; `solve` re-checks both rather than trusting its caller.
    #[error("{0}")]
    InvalidSpecification(IrValidationError),
    #[error("the specification is for part `{spec}`, the pin table for `{table}`")]
    PartMismatch { spec: String, table: String },
    #[error(
        "I2C bus `{bus}` has no net constraint on `{net}`, so it has no rise time to bound its speed"
    )]
    MissingBusConstraint { bus: String, net: String },
    #[error(
        "I2C bus `{bus}` allows a rise time of {rise_time_ns} ns, slower than any I2C mode (1000 ns at most)"
    )]
    RiseTimeTooSlow { bus: String, rise_time_ns: u32 },
    #[error(
        "I2C bus `{bus}` runs at {speed_hz} Hz, above its rise-time ceiling of {ceiling_hz} Hz"
    )]
    BusTooFast {
        bus: String,
        speed_hz: u32,
        ceiling_hz: u32,
    },
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
            Self::MissingBusConstraint { .. } => "MissingBusConstraint",
            Self::RiseTimeTooSlow { .. } => "RiseTimeTooSlow",
            Self::BusTooFast { .. } => "BusTooFast",
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
            Self::NoInstance { bus, .. }
            | Self::BudgetExceeded { bus, .. }
            | Self::MissingBusConstraint { bus, .. }
            | Self::RiseTimeTooSlow { bus, .. }
            | Self::BusTooFast { bus, .. } => Some(bus),
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
            Self::NoInstance { bus, .. }
            | Self::BudgetExceeded { bus, .. }
            | Self::MissingBusConstraint { bus, .. }
            | Self::RiseTimeTooSlow { bus, .. }
            | Self::BusTooFast { bus, .. } => vec![bus.clone()],
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
    let mut budget = Budget {
        spent: 0,
        limit: node_limit,
    };
    solve_inner(spec, table, specification_sha256, &mut budget, true)
}

/// The solver proper. `forced` switches the forced-instance check
/// (`AET-WO-1201`); it is `false` only in the differential test, and `budget`
/// is a parameter so that test can read the nodes spent.
fn solve_inner(
    spec: &Specification,
    table: &PinTable,
    specification_sha256: &str,
    budget: &mut Budget,
    forced: bool,
) -> Result<SolvedTopology, SolveError> {
    spec.validate().map_err(SolveError::InvalidSpecification)?;
    let mut out = SolvedTopology {
        schema_version: 1,
        specification_sha256: specification_sha256.to_owned(),
        assignments: BTreeMap::new(),
        gpio: BTreeMap::new(),
        i2c_ceilings: BTreeMap::new(),
    };
    out.validate().map_err(SolveError::InvalidSpecification)?;
    out.i2c_ceilings = i2c_ceilings(spec)?;
    if spec.mcu.part != table.part {
        return Err(SolveError::PartMismatch {
            spec: spec.mcu.part.clone(),
            table: table.part.clone(),
        });
    }

    let model = Model::new(spec, table)?;
    let all: Vec<usize> = (0..model.buses.len()).collect();
    let mut search = Search::new(&model, all.clone(), forced);
    let found = search.run(budget);
    if let Some(pos) = search.exceeded {
        return Err(model.budget_error(&search, pos));
    }
    if !found {
        return Err(model.minimal_set(all, budget, forced));
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
    // belt-and-suspenders: the search already keeps every rule `validate`
    // checks; this refuses rather than emits if a change to it ever breaks one.
    out.validate().map_err(SolveError::InvalidSpecification)?;
    Ok(out)
}

/// The fastest I2C mode whose maximum rise time (NXP UM10204) is at least
/// `rise_time_ns`, as its speed in Hz; `None` when it is slower than every mode.
fn ceiling_hz(rise_time_ns: u32) -> Option<u32> {
    match rise_time_ns {
        0..=120 => Some(1_000_000),
        121..=300 => Some(400_000),
        301..=1000 => Some(100_000),
        _ => None,
    }
}

/// Each I2C bus's ceiling, from the stricter of its two lines' declared
/// `max_rise_time_nanoseconds`, in document order. A line with no constraint
/// refuses: nothing is defaulted (`INV-06`). Pull-up and capacitance are not
/// read; that physics is the twin's.
fn i2c_ceilings(spec: &Specification) -> Result<BTreeMap<String, u32>, SolveError> {
    let mut out = BTreeMap::new();
    for bus in &spec.i2c_buses {
        let mut t_max = u32::MAX;
        for net in [&bus.sda.net, &bus.scl.net] {
            let Some(c) = spec.net_constraints.iter().find(|c| &c.net_name == net) else {
                return Err(SolveError::MissingBusConstraint {
                    bus: bus.id.clone(),
                    net: net.clone(),
                });
            };
            t_max = t_max.min(c.max_rise_time_nanoseconds);
        }
        let Some(ceiling) = ceiling_hz(t_max) else {
            return Err(SolveError::RiseTimeTooSlow {
                bus: bus.id.clone(),
                rise_time_ns: t_max,
            });
        };
        if bus.speed_hz > ceiling {
            return Err(SolveError::BusTooFast {
                bus: bus.id.clone(),
                speed_hz: bus.speed_hz,
                ceiling_hz: ceiling,
            });
        }
        out.insert(bus.id.clone(), ceiling);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::ceiling_hz;

    #[test]
    fn the_mapping_at_each_boundary() {
        assert_eq!(ceiling_hz(1), Some(1_000_000));
        assert_eq!(ceiling_hz(120), Some(1_000_000));
        assert_eq!(ceiling_hz(121), Some(400_000));
        assert_eq!(ceiling_hz(300), Some(400_000));
        assert_eq!(ceiling_hz(301), Some(100_000));
        assert_eq!(ceiling_hz(1000), Some(100_000));
        assert_eq!(ceiling_hz(1001), None);
        assert_eq!(ceiling_hz(u32::MAX), None);
    }
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
    path: String,
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
                path: at.clone(),
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
                path: at.clone(),
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
        model.check_peripherals()?;
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

    fn check_peripherals(&self) -> Result<(), SolveError> {
        let mut taken: BTreeMap<&str, String> = BTreeMap::new();
        for bus in &self.buses {
            let Some(p) = bus.peripheral else { continue };
            let claimant = bus.path.clone();
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
        let alone = Search::new(self, (0..self.buses.len()).collect(), false);
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

    fn budget_error(&self, search: &Search, pos: usize) -> SolveError {
        SolveError::BudgetExceeded {
            bus: self.buses[search.active[pos]].id.to_owned(),
            limit: search.budget.limit,
        }
    }

    /// Deletion: drop each bus in document order if the rest stay
    /// infeasible. The set left is minimal (irreducible), not necessarily the
    /// smallest; it costs one search per bus, inside the same budget.
    fn minimal_set(&self, mut set: Vec<usize>, budget: &mut Budget, forced: bool) -> SolveError {
        let mut i = 0;
        while i < set.len() {
            let mut without = set.clone();
            without.remove(i);
            let mut search = Search::new(self, without.clone(), forced);
            let found = search.run(budget);
            if let Some(pos) = search.exceeded {
                return self.budget_error(&search, pos);
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
    /// Whether `consistent` runs the forced-instance check.
    forced: bool,
}

impl<'m, 's, 't> Search<'m, 's, 't> {
    fn new(model: &'m Model<'s, 't>, active: Vec<usize>, forced: bool) -> Self {
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
            forced,
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
        matching(&signal_pins) && (!self.forced || self.forced_instances(rest, &options))
    }

    /// The forced-instance check. For a kind whose unplaced buses are as many
    /// as the instances any of them can still take, those buses use every one
    /// of those instances, so each instance's every signal needs a pin of its
    /// own. The rows are (instance, signal); a row may take any free pin that
    /// some unplaced bus of the kind could give that signal on that instance.
    /// A relaxation of the real choice, so it only prunes.
    fn forced_instances(&self, rest: &[usize], options: &[Vec<&str>]) -> bool {
        let mut rows: Vec<Vec<&str>> = Vec::new();
        for kind in [Kind::I2c, Kind::Spi] {
            let buses: Vec<usize> = (0..rest.len())
                .filter(|&r| self.model.buses[rest[r]].kind == kind)
                .collect();
            let reachable: BTreeSet<&str> = buses
                .iter()
                .flat_map(|&r| options[r].iter().copied())
                .collect();
            if buses.is_empty() || buses.len() != reachable.len() {
                continue;
            }
            for inst in reachable {
                for k in 0..kind.signals().len() {
                    let pins: BTreeSet<&str> = buses
                        .iter()
                        .filter(|&&r| options[r].contains(&inst))
                        .flat_map(|&r| self.pins(&self.model.buses[rest[r]].signals[k], inst))
                        .collect();
                    rows.push(pins.into_iter().collect());
                }
            }
        }
        matching(&rows)
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

/// `AET-WO-1201`: the forced-instance check only prunes. Random small tables
/// and specifications are solved with the check and without it; the answers
/// must be identical, except that a `BudgetExceeded` without it may be
/// `Infeasible` with it.
#[cfg(test)]
mod differential {
    use super::*;
    use crate::{I2cBus, Mcu, NetConstraint, Pin, PinSource, SpiBus, SpiDevice, VoltageDomain};

    const SPECS: u32 = 3000;
    const LIMIT: u64 = 20_000;
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const POOL: [&str; 14] = [
        "PA0", "PA1", "PA2", "PA3", "PA4", "PA5", "PA6", "PA7", "PB0", "PB1", "PB2", "PB3", "PA13",
        "PC14",
    ];

    /// xorshift64*, fixed seed: the same specifications on every host.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() >> 33) as usize % n
        }
        fn one_in(&mut self, n: usize) -> bool {
            self.below(n) == 0
        }
    }

    fn table(rng: &mut Rng) -> PinTable {
        let pool = 6 + rng.below(POOL.len() - 5);
        let mut afs: BTreeMap<&str, BTreeMap<String, u8>> = BTreeMap::new();
        for (kind, count) in [(Kind::I2c, 4), (Kind::Spi, 6)] {
            for n in 1..=count {
                for sig in kind.signals() {
                    // One in thirty signals is missing, so an instance is not offered.
                    if rng.one_in(30) {
                        continue;
                    }
                    for _ in 0..1 + rng.below(3) {
                        let pin = POOL[rng.below(pool)];
                        afs.entry(pin).or_default().insert(
                            format!("{}{n}_{sig}", kind.prefix()),
                            1 + rng.below(8) as u8,
                        );
                    }
                }
            }
        }
        PinTable {
            part: "T".into(),
            package: "T".into(),
            source: PinSource {
                repository: "t".into(),
                commit: "t".into(),
                files: BTreeMap::new(),
            },
            pins: POOL[..pool]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let pin = Pin {
                        position: i as u16 + 1,
                        alternate_functions: afs.remove(name).unwrap_or_default(),
                    };
                    ((*name).to_owned(), pin)
                })
                .collect(),
        }
    }

    fn signal(rng: &mut Rng, net: String, pool: usize) -> Signal {
        // One in fourteen is a hand pin; one in forty of those is not in the table.
        let pin = rng.one_in(14).then(|| {
            if rng.one_in(40) {
                "PZ9".to_owned()
            } else {
                POOL[rng.below(pool)].to_owned()
            }
        });
        Signal { net, pin }
    }

    fn peripheral(rng: &mut Rng, kind: Kind) -> Option<String> {
        rng.one_in(12)
            .then(|| format!("{}{}", kind.prefix(), 1 + rng.below(4)))
    }

    fn spec(rng: &mut Rng, pool: usize) -> Specification {
        let mut i2c_buses = Vec::new();
        for i in 0..rng.below(5) {
            let id = format!("I{i}");
            i2c_buses.push(I2cBus {
                peripheral: peripheral(rng, Kind::I2c),
                speed_hz: 400_000,
                pull_up_ohms: 2200,
                domain: "D".into(),
                sda: signal(rng, format!("{id}_sda"), pool),
                scl: signal(rng, format!("{id}_scl"), pool),
                id,
            });
        }
        let mut spi_buses = Vec::new();
        for i in 0..rng.below(8) {
            let id = format!("S{i}");
            spi_buses.push(SpiBus {
                peripheral: peripheral(rng, Kind::Spi),
                speed_hz: 1_000_000,
                domain: "D".into(),
                sck: signal(rng, format!("{id}_sck"), pool),
                mosi: signal(rng, format!("{id}_mosi"), pool),
                miso: signal(rng, format!("{id}_miso"), pool),
                id,
            });
        }
        let mut spi_devices = Vec::new();
        if !spi_buses.is_empty() {
            for i in 0..rng.below(4) {
                let bus = spi_buses[rng.below(spi_buses.len())].id.clone();
                spi_devices.push(SpiDevice {
                    reference: format!("U{i}"),
                    part: "F".into(),
                    supply: "D".into(),
                    bus,
                    cs: signal(rng, format!("U{i}_cs"), pool),
                });
            }
        }
        let net_constraints = i2c_buses
            .iter()
            .flat_map(|b| [&b.sda.net, &b.scl.net])
            .map(|net| NetConstraint {
                net_name: net.clone(),
                max_capacitance_femtofarads: 100_000,
                max_rise_time_nanoseconds: 300,
                impedance_ohms: None,
            })
            .collect();
        Specification {
            schema_version: 1,
            project_name: "t".into(),
            mcu: Mcu {
                part: "T".into(),
                supply: "D".into(),
            },
            voltage_domains: vec![VoltageDomain {
                id: "D".into(),
                nominal_millivolts: 3300,
                tolerance_permille: 50,
            }],
            i2c_buses,
            spi_buses,
            i2c_devices: vec![],
            spi_devices,
            net_constraints,
        }
    }

    fn run(
        spec: &Specification,
        table: &PinTable,
        forced: bool,
    ) -> (Result<SolvedTopology, SolveError>, u64) {
        let mut budget = Budget {
            spent: 0,
            limit: LIMIT,
        };
        let out = solve_inner(spec, table, HASH, &mut budget, forced);
        (out, budget.spent)
    }

    #[test]
    fn the_forced_instance_check_only_prunes() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        // ok, infeasible, budget, other refusal; then budget -> infeasible.
        let mut seen = [0u32; 4];
        let (mut flipped, mut nodes_off, mut nodes_on) = (0u32, 0u64, 0u64);
        let mut kinds = BTreeMap::new();
        for n in 0..SPECS {
            let table = table(&mut rng);
            let pool = table.pins.len();
            let spec = spec(&mut rng, pool);
            let (off, spent_off) = run(&spec, &table, false);
            let (on, spent_on) = run(&spec, &table, true);
            nodes_off += spent_off;
            nodes_on += spent_on;
            assert!(spent_on <= spent_off, "spec {n}: the check added nodes");
            match (&off, &on) {
                (Err(SolveError::BudgetExceeded { .. }), Err(SolveError::Infeasible { .. })) => {
                    flipped += 1;
                }
                _ => assert_eq!(off, on, "spec {n}: the check changed the answer"),
            }
            if let Err(e) = &off {
                *kinds.entry(e.kind()).or_insert(0u32) += 1;
            }
            seen[match &off {
                Ok(_) => 0,
                Err(SolveError::Infeasible { .. }) => 1,
                Err(SolveError::BudgetExceeded { .. }) => 2,
                Err(_) => 3,
            }] += 1;
        }
        eprintln!(
            "differential: {SPECS} specs; without the check ok {}, infeasible {}, budget {}, other refusal {}; \
             budget->infeasible {flipped}; nodes {nodes_off} without, {nodes_on} with",
            seen[0], seen[1], seen[2], seen[3]
        );
        eprintln!("refusals: {kinds:?}");
        // The generator must reach each outcome, or the test proves nothing.
        assert!(
            [0, 1, 3].iter().all(|&i| seen[i] >= 50),
            "outcomes: {seen:?}"
        );
    }

    /// The 1200 reproducers' shape on a synthetic table: six SPI buses, six
    /// instances, and two instances whose only `MISO` is the same pin. Pin
    /// matching unions over instances and instance matching ignores pins, so
    /// both pass; the forced instances do not.
    #[test]
    fn a_shared_only_pin_is_refused_at_once_not_after_the_budget() {
        let mut pins: BTreeMap<String, Pin> = BTreeMap::new();
        let mut put = |name: String, af: String| {
            let n = pins.len() as u16 + 1;
            pins.entry(name)
                .or_insert(Pin {
                    position: n,
                    alternate_functions: BTreeMap::new(),
                })
                .alternate_functions
                .insert(af, 5);
        };
        for n in 1..=6 {
            put(format!("PA{n}"), format!("SPI{n}_SCK"));
            put(format!("PB{n}"), format!("SPI{n}_MOSI"));
            let miso = if n <= 2 {
                "PC0".to_owned()
            } else {
                format!("PC{n}")
            };
            put(miso, format!("SPI{n}_MISO"));
        }
        // The union of the MISO pins is six, so pin matching passes.
        put("PC7".to_owned(), "SPI3_MISO".to_owned());
        let table = PinTable {
            part: "T".into(),
            package: "T".into(),
            source: PinSource {
                repository: "t".into(),
                commit: "t".into(),
                files: BTreeMap::new(),
            },
            pins,
        };
        let mut rng = Rng(1);
        let mut spec = spec(&mut rng, table.pins.len());
        spec.i2c_buses.clear();
        spec.net_constraints.clear();
        spec.spi_devices.clear();
        spec.spi_buses = (0..6)
            .map(|i| {
                let sig = |s: &str| Signal {
                    net: format!("S{i}_{s}"),
                    pin: None,
                };
                SpiBus {
                    id: format!("S{i}"),
                    peripheral: None,
                    speed_hz: 1_000_000,
                    domain: "D".into(),
                    sck: sig("sck"),
                    mosi: sig("mosi"),
                    miso: sig("miso"),
                }
            })
            .collect();
        let (on, spent_on) = run(&spec, &table, true);
        let (off, spent_off) = run(&spec, &table, false);
        let Err(SolveError::Infeasible { buses }) = &on else {
            panic!("with the check: {on:?}");
        };
        assert_eq!(buses.len(), 6);
        eprintln!("shared-only-pin: {spent_on} nodes with, {spent_off} without ({off:?})");
        assert!(spent_on < spent_off);
        // Between the two, only the check finishes: the flip the work order allows.
        let tight = |forced| {
            let mut budget = Budget {
                spent: 0,
                limit: 300,
            };
            solve_inner(&spec, &table, HASH, &mut budget, forced)
        };
        assert!(matches!(tight(true), Err(SolveError::Infeasible { .. })));
        assert!(matches!(
            tight(false),
            Err(SolveError::BudgetExceeded { .. })
        ));
    }
}
