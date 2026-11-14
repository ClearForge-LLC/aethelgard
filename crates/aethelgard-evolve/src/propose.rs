//! Proposals after an attributed failure (`AET-WO-3200`, `RUL-07`, `RUL-18`).
//!
//! A proposer takes an [`Attribution`], the run's evidence read through the
//! join ([`crate::RunView`]) and the solved facts the CLI hands it as plain
//! integers and identifiers, and answers with exactly one [`Outcome`]: a
//! [`Proposal`] of specification changes, a [`GeneratorReport`], or
//! [`Outcome::NoProposal`]. Proposers run in a fixed order and the first that
//! answers wins ([`propose`]). Today there is one, [`Rules`]. Nothing here
//! applies a change, reads a file or starts a process: the CLI applies a
//! proposal through `campaign`'s own override path, and the gates judge it.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Attribution, Domain, EventRef, Evidence, Field, Rule, SolvedBus, View};

/// The one thing a proposal may change: a bus's pull-up. It is the scenario
/// override `{"bus", "pull_up_ohms"}`. The other two override forms,
/// `max_capacitance_femtofarads` and `line_held_low`, describe a fault, never
/// a fix, and a constraint the gates judge is never a lever (`INV-07`), so no
/// other change can be expressed (`AET-WO-3200` §8, review A8.11).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Change {
    pub bus: String,
    pub pull_up_ohms: u32,
}

/// What the CLI hands a proposer for one solved bus, from the solved topology
/// it verified against the run: the pull-up and rail both lines carry, and
/// the pull-up floor from `aethelgard_ir::electrical`, the gate's own
/// definition. This crate keeps no copy of it (`AET-WO-3200` §8, review A2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusFacts {
    pub pull_up_ohms: u32,
    pub domain_millivolts: u32,
    pub floor_ohms: u32,
}

/// Everything a proposer may read.
pub struct Input<'a> {
    pub attribution: &'a Attribution,
    /// The attribution's evidence events, in its order, through the join.
    pub evidence: &'a [Evidence],
    /// Every solved bus of the run, with its lines' measured rises.
    pub buses: &'a [SolvedBus],
    /// The solved facts, by bus id.
    pub facts: &'a BTreeMap<String, BusFacts>,
}

/// One bus's pull-up, sized from its measured rise. Every rise here is
/// computed from declared values (`RUL-19`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sizing {
    pub bus: String,
    /// The pull-up the run had.
    pub r_now_ohms: u32,
    /// The line whose rise binds: the least `ceiling / rise` on the bus.
    pub binding_net: String,
    pub rise_worst_ps: u64,
    pub ceiling_ps: u64,
    /// `R_now × ceiling × 0.8 / rise_worst`, rounded down.
    pub r_target_ohms: u64,
    /// The pull-up floor (`aethelgard_ir::electrical`).
    pub rp_min_ohms: u32,
    /// The largest E12 value at or below `R_target`; `None` when none is.
    pub e12_ohms: Option<u32>,
    /// The binding line's rise at the E12 value, rounded up: linear in R × C
    /// (`architecture.md` §2.18). `None` with no E12 value.
    pub predicted_rise_ps: Option<u64>,
}

/// Specification changes, with the numbers that chose them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// Sorted by bus, one per bus.
    pub changes: Vec<Change>,
    pub sizing: Vec<Sizing>,
}

/// The generator a solved expectation belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Generator {
    /// The firmware (`firmware/src/main.rs`): the device reads and the boot.
    Firmware,
    /// The board crate (`src/board.rs`): clock gates, pin modes and
    /// alternate functions, and every other register it configures.
    BoardCrate,
}

impl Generator {
    pub fn name(self) -> &'static str {
        match self {
            Self::Firmware => "firmware",
            Self::BoardCrate => "board_crate",
        }
    }
}

/// One solved expectation the generated firmware did not meet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expectation {
    pub event: EventRef,
    pub generator: Generator,
    /// The register field, as [`Field`] names it, `instructions` for the
    /// boot's instruction count, or `pc_inside_firmware` for where the core
    /// ended: solved 1, inside an executable segment; observed 0, outside
    /// (`AET-WO-2205` §1.5).
    pub what: String,
    pub address: Option<u32>,
    pub mask: Option<u32>,
    pub solved: u64,
    pub observed: Option<u64>,
}

/// A software attribution in generated firmware is a generator defect: this
/// is its bug report, and it carries no specification change (`RUL-18`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorReport {
    /// Sorted by event, then field.
    pub expectations: Vec<Expectation>,
}

/// What a proposer answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Proposal(Proposal),
    GeneratorReport(GeneratorReport),
    /// `sizing` holds the numbers when a rule matched but no value may be
    /// proposed, and is empty otherwise.
    NoProposal {
        reason: String,
        sizing: Vec<Sizing>,
    },
}

/// A source of proposals. It answers `None` when it has no rule for the run.
pub trait Proposer {
    fn id(&self) -> &'static str;
    fn propose(&self, input: &Input) -> Option<Outcome>;
}

/// The reason given when no proposer answers.
pub const MODEL_SLOT: &str = "no rule; RUL-18 model proposer slot";

/// The cause the analog pass names in a bus fault for a rise past its
/// ceiling (`aethelgard-cli`'s `analog.rs`, `PAST_CEILING`; pinned there by a
/// test that reads it back from a campaign run).
pub const PAST_CEILING_CAUSE: &str = "analog.rise_time_computed > ceiling_ns";

/// Headroom on the ceiling, 0.8: for a ±5 % resistor tolerance and a
/// capacitance spread the run did not measure (`AET-WO-3200` §1.2).
pub const HEADROOM: (u64, u64) = (8, 10);

/// The E12 series' mantissas.
const E12: [u64; 12] = [10, 12, 15, 18, 22, 27, 33, 39, 47, 56, 68, 82];

/// Who answered, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The answering proposer's id; `None` when none answered, or when the
    /// answer was refused for its domain.
    pub proposer: Option<&'static str>,
    pub outcome: Outcome,
}

fn no(reason: String) -> Outcome {
    Outcome::NoProposal {
        reason,
        sizing: Vec::new(),
    }
}

/// Run `proposers` in order; the first that answers wins. Whatever proposer
/// answers, a [`Proposal`] stands only for an `electrical` run, with one
/// change per bus the CLI gave facts for, at or above that bus's floor; a
/// [`GeneratorReport`] stands only for a `software` run. Anything else is
/// refused here, so no proposer, a later model proposer included, can propose
/// for an `indeterminate` or `none` run (`INV-05`) or below the floor.
pub fn propose(proposers: &[&dyn Proposer], input: &Input) -> Answer {
    let Some((id, outcome)) = proposers
        .iter()
        .find_map(|p| p.propose(input).map(|o| (p.id(), o)))
    else {
        return Answer {
            proposer: None,
            outcome: no(MODEL_SLOT.to_owned()),
        };
    };
    let domain = input.attribution.domain;
    let refused = |what: &str| Answer {
        proposer: None,
        outcome: no(format!(
            "proposer `{id}` answered {what} for a run attributed `{}` by {}: refused (INV-05, RUL-18)",
            domain.name(),
            input.attribution.rule.id()
        )),
    };
    match &outcome {
        Outcome::Proposal(_) if domain != Domain::Electrical => refused("a proposal"),
        Outcome::Proposal(p) if !admissible(p, input.facts) => {
            refused("changes outside the solved buses or below a floor")
        }
        Outcome::GeneratorReport(_) if domain != Domain::Software => refused("a generator report"),
        _ => Answer {
            proposer: Some(id),
            outcome,
        },
    }
}

/// At least one change, at most one per bus, each on a bus with facts and at
/// or above its floor.
fn admissible(p: &Proposal, facts: &BTreeMap<String, BusFacts>) -> bool {
    let buses: BTreeSet<&str> = p.changes.iter().map(|c| c.bus.as_str()).collect();
    !p.changes.is_empty()
        && buses.len() == p.changes.len()
        && p.changes.iter().all(|c| {
            facts
                .get(&c.bus)
                .is_some_and(|f| c.pull_up_ohms >= f.floor_ohms)
        })
}

/// The rules proposer: a closed table, each row citing the attribution rule
/// it answers (`RUL-18`, `AET-WO-3200` §1.2).
pub struct Rules;

impl Proposer for Rules {
    fn id(&self) -> &'static str {
        "rules"
    }

    fn propose(&self, input: &Input) -> Option<Outcome> {
        let a = input.attribution;
        match a.rule {
            Rule::E1 => past_ceiling(input),
            Rule::S1 | Rule::S2 => Some(report(input)),
            rule => Some(no(format!(
                "attribution rule {} (`{}`): attribution precedes mutation, so nothing is proposed (INV-05)",
                rule.id(),
                rule.domain().name()
            ))),
        }
    }
}

/// `E1` with every fault a complete rise past its ceiling: size each faulted
/// bus's pull-up. A held-low line, a stuck-high line, an incomplete rise, or
/// a bus fault with any other cause has no rule (`None`): a held line is a
/// board fault no value change fixes.
fn past_ceiling(input: &Input) -> Option<Outcome> {
    let mut faulted = BTreeSet::new();
    for e in input.evidence {
        let ok = match &e.view {
            View::Rise {
                complete, exceeds, ..
            } => *complete && *exceeds,
            View::BusFault { from, .. } => {
                !from.is_empty() && from.iter().all(|c| c == PAST_CEILING_CAUSE)
            }
            _ => false,
        };
        if !ok {
            return None;
        }
        faulted.insert(e.bus.as_deref()?);
    }
    let mut sizing = Vec::new();
    for bus in faulted {
        let solved = input.buses.iter().find(|b| b.bus == bus)?;
        let Some(facts) = input.facts.get(bus) else {
            return Some(no(format!("no solved facts for bus `{bus}`")));
        };
        sizing.push(size(solved, facts)?);
    }
    if let Some(s) = sizing
        .iter()
        .find(|s| s.e12_ohms.is_none_or(|v| v < s.rp_min_ohms))
    {
        let reason = format!(
            "bus `{}`: no E12 value lies in [Rp_min, R_target] = [{}, {}] ohms, and a value below the pull-up floor is never proposed",
            s.bus, s.rp_min_ohms, s.r_target_ohms
        );
        return Some(Outcome::NoProposal { reason, sizing });
    }
    let changes = sizing
        .iter()
        .map(|s| Change {
            bus: s.bus.clone(),
            pull_up_ohms: s.e12_ohms.expect("checked above"),
        })
        .collect();
    Some(Outcome::Proposal(Proposal { changes, sizing }))
}

/// One faulted bus's sizing. Every line must have exactly one complete
/// measured rise, and the binding line must be past its ceiling; otherwise
/// there is no rule (`None`).
fn size(bus: &SolvedBus, facts: &BusFacts) -> Option<Sizing> {
    let mut binding: Option<(&str, u64, u64)> = None;
    for line in &bus.lines {
        let [rise] = line.rises.as_slice() else {
            return None;
        };
        if !rise.complete || rise.rise_ps == 0 {
            return None;
        }
        // The least ceiling / rise binds: compare a/b < c/d as a·d < c·b.
        let tighter = binding.is_none_or(|(_, r, c)| {
            u128::from(line.ceiling_ps) * u128::from(r) < u128::from(c) * u128::from(rise.rise_ps)
        });
        if tighter {
            binding = Some((&line.net, rise.rise_ps, line.ceiling_ps));
        }
    }
    let (net, rise, ceiling) = binding?;
    if rise <= ceiling {
        return None;
    }
    let r_now = facts.pull_up_ohms;
    let (num, den) = HEADROOM;
    let r_target = u128::from(r_now) * u128::from(ceiling) * u128::from(num)
        / (u128::from(rise) * u128::from(den));
    let r_target = u64::try_from(r_target).expect("below R_now, a u32");
    let e12 = largest_e12_at_most(r_target);
    let predicted = e12.map(|v| {
        let p = (u128::from(rise) * u128::from(v)).div_ceil(u128::from(r_now));
        u64::try_from(p).expect("below the measured rise")
    });
    Some(Sizing {
        bus: bus.bus.clone(),
        r_now_ohms: r_now,
        binding_net: net.to_owned(),
        rise_worst_ps: rise,
        ceiling_ps: ceiling,
        r_target_ohms: r_target,
        rp_min_ohms: facts.floor_ohms,
        e12_ohms: e12,
        predicted_rise_ps: predicted,
    })
}

/// The largest E12 value, in ohms from 10 Ω up and within a `u32`, at or
/// below `limit`.
pub fn largest_e12_at_most(limit: u64) -> Option<u32> {
    let mut best = None;
    let mut decade = 1u64;
    while decade <= u64::from(u32::MAX) {
        for m in E12 {
            let v = m * decade;
            if v <= limit {
                best = u32::try_from(v).ok().or(best);
            }
        }
        decade *= 10;
    }
    best
}

/// `S1` or `S2`: every solved expectation the evidence shows unmet, with the
/// generator it belongs to. A witness (kind 13) and the boot (kind 10) are
/// the firmware's; a register (kind 11) is the board crate's. A software
/// evidence event that shows no unmet field is not reported as a defect: the
/// view and the attribution disagree, so nothing is said.
fn report(input: &Input) -> Outcome {
    let mut expectations = Vec::new();
    for e in input.evidence {
        let field = |f: &Field, generator| Expectation {
            event: e.event,
            generator,
            what: f.register.clone(),
            address: Some(f.address),
            mask: Some(f.mask),
            solved: f.solved.into(),
            observed: f.observed.map(u64::from),
        };
        let found: Vec<Expectation> = match &e.view {
            View::Rise { .. } => continue,
            View::Fields { device, fields } => {
                let generator = if device.is_some() {
                    Generator::Firmware
                } else {
                    Generator::BoardCrate
                };
                fields
                    .iter()
                    .filter(|f| f.wrong)
                    .map(|f| field(f, generator))
                    .collect()
            }
            View::Boot {
                boot_word,
                instructions_expected,
                instructions_observed,
                pc_outside,
                ..
            } => {
                let mut v: Vec<Expectation> = [boot_word]
                    .into_iter()
                    .filter(|f| f.wrong)
                    .map(|f| field(f, Generator::Firmware))
                    .collect();
                if instructions_observed.is_some_and(|n| n < *instructions_expected) {
                    v.push(Expectation {
                        event: e.event,
                        generator: Generator::Firmware,
                        what: "instructions".to_owned(),
                        address: None,
                        mask: None,
                        solved: *instructions_expected,
                        observed: *instructions_observed,
                    });
                }
                if *pc_outside {
                    v.push(Expectation {
                        event: e.event,
                        generator: Generator::Firmware,
                        what: "pc_inside_firmware".to_owned(),
                        address: None,
                        mask: None,
                        solved: 1,
                        observed: Some(0),
                    });
                }
                v
            }
            _ => Vec::new(),
        };
        if found.is_empty() {
            return no(format!(
                "evidence event {} (kind {}) shows no unmet solved expectation, so no generator report is made",
                e.event.index, e.event.kind
            ));
        }
        expectations.extend(found);
    }
    Outcome::GeneratorReport(GeneratorReport { expectations })
}
