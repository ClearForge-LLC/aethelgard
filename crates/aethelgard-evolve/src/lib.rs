//! Attribution from traces and mutation proposals; never merges anything or
//! bypasses a gate (`docs/architecture.md` §3).
//!
//! [`attribute`] is a pure function from one archived campaign run to one
//! [`Attribution`] (`AET-WO-3100`, `INV-05`): the domain a failure belongs
//! to, the one rule that decided it, and the archive events that prove it.
//! It reads the run's records and the vendored ST header through
//! `aethelgard-svd-hal`, and nothing else: no specification (`RUL-16`), no
//! file, no process. The rules are closed and ordered, each needs positive
//! evidence, and neither half is ever concluded by eliminating the other.
//! The verdict's finding kinds choose which events need explaining; they are
//! never evidence. The rule table is `README.md`'s.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use aethelgard_chaos::Replayed;
use aethelgard_chaos::campaign::{self, HELD_LOW};
use aethelgard_chaos::record::{BusLine, Reading, Record};
use aethelgard_svd_hal::cmsis::{self, RegisterError, RegisterFacts};

/// The half a failing run is attributed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    Electrical,
    Software,
    /// The run passed.
    None,
    Indeterminate,
}

impl Domain {
    pub fn name(self) -> &'static str {
        match self {
            Self::Electrical => "electrical",
            Self::Software => "software",
            Self::None => "none",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// The closed, ordered rules (`README.md`). Within a domain, the lowest
/// rule that decided any symptom names the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// The symptom's bus is electrically faulted: a rise past its ceiling, a
    /// line stuck high, or a bus fault on it.
    E1,
    /// The symptom's bus is measured healthy, and the firmware's own state on
    /// that bus differs from the solved topology.
    S1,
    /// The symptom is on no bus (the boot, or a register no bus joins), every
    /// bus is measured healthy, and the boot or such a register is wrong.
    S2,
    /// The run passed.
    N1,
    /// A failing finding has no archived event that shows it.
    I1,
    /// A line of the symptom's bus (or, for a symptom on no bus, of any bus)
    /// is not measured exactly once, or there is no bus at all.
    I2,
    /// The symptom's bus is measured healthy with no software evidence, and
    /// the only electrical fault is on another bus.
    I3,
    /// Everything is measured healthy and no software evidence exists.
    I4,
    /// Some symptoms are electrical and others software.
    I5,
}

impl Rule {
    pub fn id(self) -> &'static str {
        match self {
            Self::E1 => "E1",
            Self::S1 => "S1",
            Self::S2 => "S2",
            Self::N1 => "N1",
            Self::I1 => "I1",
            Self::I2 => "I2",
            Self::I3 => "I3",
            Self::I4 => "I4",
            Self::I5 => "I5",
        }
    }

    pub fn domain(self) -> Domain {
        match self {
            Self::E1 => Domain::Electrical,
            Self::S1 | Self::S2 => Domain::Software,
            Self::N1 => Domain::None,
            _ => Domain::Indeterminate,
        }
    }

    /// The event kinds this rule may name as evidence.
    pub fn evidence_kinds(self) -> &'static [u64] {
        match self {
            Self::E1 => &[7, 8, 9],
            Self::S1 => &[7, 11, 13],
            Self::S2 => &[7, 10, 11],
            Self::N1 | Self::I1 => &[],
            Self::I2 => &[15],
            Self::I3 => &[7, 8, 9],
            Self::I4 => &[7],
            Self::I5 => &[7, 8, 9, 10, 11, 13],
        }
    }
}

/// One archive event: its index in the trace and its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventRef {
    pub index: u64,
    pub kind: u64,
}

/// What attribution says of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribution {
    pub run_id: [u8; 16],
    /// The state-sequence hash, as `replay` recomputes it.
    pub state_hash: [u8; 32],
    pub domain: Domain,
    pub rule: Rule,
    /// The events that prove the cause, sorted.
    pub evidence: Vec<EventRef>,
    /// The failing events the verdict's findings select, sorted.
    pub symptoms: Vec<EventRef>,
    /// Software evidence on an electrically faulted bus: listed, never used.
    pub masked: Vec<EventRef>,
    /// Failing finding kinds no archived event shows (rule `I1`), sorted.
    pub unexplained: Vec<String>,
}

/// Why a run cannot be attributed at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeError {
    /// The run's records break a rule `replay` judges by.
    NotACampaignRun(&'static str),
    /// The vendored ST header cannot give a register fact the run names.
    RegisterFacts(RegisterError),
}

impl From<RegisterError> for AttributeError {
    fn from(e: RegisterError) -> Self {
        Self::RegisterFacts(e)
    }
}

// belt-and-suspenders: the header's `_Msk` macros are shifts the header reader
// does not evaluate, so the widths are stated here from RM0433 (§48.7.2: SADD
// 10 bits, NBYTES 8; §11.4.1: MODER 2 bits a pin; §11.4.9: AFSEL 4 bits a pin).
// Every position comes from the header, as the judge's does.
const SADD_BITS: u32 = 10;
const NBYTES_BITS: u32 = 8;
const MODE_BITS: u32 = 2;
const AFSEL_BITS: u32 = 4;
/// `TXDR`'s data byte.
const TXDR_DATA: u32 = 0xFF;

fn field(pos: u32, bits: u32) -> u32 {
    ((1u32 << bits) - 1) << pos
}

/// A register field one bus owns: a register by name and address, the field's
/// mask, and the value it must hold under that mask.
#[derive(Clone, Debug)]
struct Owned {
    name: String,
    address: u32,
    mask: u32,
    want: Want,
}

#[derive(Clone, Copy, Debug)]
enum Want {
    /// A clock gate: the bit must be set.
    Set,
    /// A pin's mode: the reading's own expectation.
    Expected,
    /// A pin's alternate function, from kind 15.
    Value(u32),
}

/// The register fields a bus owns: its controller's clock gate, the gates of
/// its pins' ports, and each pin's mode and alternate function (`AET-WO-3100`
/// R5). Names and positions come from the header.
fn owned(
    f: &RegisterFacts,
    peripheral: &str,
    lines: &[BusLine],
) -> Result<Vec<Owned>, RegisterError> {
    let rcc = f.base("RCC")?;
    let rcc_layout = f.layout("RCC")?;
    let gpio = f.layout("GPIO")?;
    let gate = |block: &str| -> Result<Owned, RegisterError> {
        let (register, bit) = f.enable_bit(block)?;
        Ok(Owned {
            address: rcc + rcc_layout.offset(&register, 0)?,
            name: format!("RCC.{register}"),
            mask: 1 << bit,
            want: Want::Set,
        })
    };
    let mut out = vec![gate(peripheral)?];
    for line in lines {
        let (port, index) = split_pin(&line.pin).ok_or_else(|| RegisterError::Undefined {
            name: format!("pin {}", line.pin),
        })?;
        let block = format!("GPIO{port}");
        let base = f.base(&block)?;
        out.push(gate(&block)?);
        let mode = f.position(&format!("GPIO_MODER_MODE{index}_Pos"))?;
        out.push(Owned {
            name: format!("{block}.MODER"),
            address: base + gpio.offset("MODER", 0)?,
            mask: field(mode, MODE_BITS),
            want: Want::Expected,
        });
        // AFRL holds pins 0 to 7 and AFRH 8 to 15: the header says which.
        let (half, word, pos) = match f.position(&format!("GPIO_AFRL_AFSEL{index}_Pos")) {
            Ok(pos) => ("AFRL", 0, pos),
            Err(_) => (
                "AFRH",
                1,
                f.position(&format!("GPIO_AFRH_AFSEL{index}_Pos"))?,
            ),
        };
        out.push(Owned {
            name: format!("{block}.{half}"),
            address: base + gpio.offset("AFR", word)?,
            mask: field(pos, AFSEL_BITS),
            want: Want::Value(u32::from(line.alternate_function) << pos),
        });
    }
    Ok(out)
}

/// `PB6` as (`B`, 6).
fn split_pin(pin: &str) -> Option<(char, u32)> {
    let mut chars = pin.chars();
    let (Some('P'), Some(port)) = (chars.next(), chars.next()) else {
        return None;
    };
    let digits = chars.as_str();
    let ok = port.is_ascii_uppercase()
        && !digits.is_empty()
        && digits.len() <= 2
        && digits.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| digits.parse().ok().map(|i| (port, i))).flatten()
}

/// Whether `owned`'s field in `reading` (of register `name`) is wrong, as a
/// fact about the firmware: the register is the one named, the field is
/// covered, observed, and differs from what it must hold. A reading whose own
/// expectation disagrees with the solved value is no evidence either way.
fn wrong(o: &Owned, name: &str, r: &Reading) -> bool {
    let Some(observed) = r.observed else {
        return false;
    };
    if name != o.name || r.address != o.address || r.mask & o.mask != o.mask {
        return false;
    }
    match o.want {
        Want::Set => r.expected & o.mask == o.mask && observed & o.mask != o.mask,
        Want::Expected => (observed ^ r.expected) & o.mask != 0,
        Want::Value(v) => r.expected & o.mask == v && observed & o.mask != v,
    }
}

/// A register reading that differs from its expectation, under its mask.
fn mismatch(r: &Reading) -> u32 {
    r.observed.map_or(0, |o| (o ^ r.expected) & r.mask)
}

struct Bus<'a> {
    /// The kind 15's event index.
    at: usize,
    id: &'a str,
    lines: &'a [BusLine],
    devices: &'a [(String, u8)],
    owned: Vec<Owned>,
}

/// Where a symptom is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Bus(usize),
    Board,
}

struct Run<'a> {
    records: &'a [Record],
    buses: Vec<Bus<'a>>,
    sadd: u32,
    nbytes: u32,
    rcc: (u32, u32),
}

/// Statuses whose record shows the write phase ran, so `TXDR` was written.
const WROTE_TXDR: [&str; 4] = ["ok", "timeout_tc", "timeout_rxne", "timeout_stopf"];
/// Statuses whose record shows the read phase started, so `CR2` holds the
/// read's `NBYTES`. Before it, `CR2` holds the write's.
const READ_PHASE: [&str; 3] = ["ok", "timeout_rxne", "timeout_stopf"];

impl<'a> Run<'a> {
    fn bus_named(&self, id: &str) -> Option<usize> {
        self.buses.iter().position(|b| b.id == id)
    }

    fn bus_of_device(&self, reference: &str) -> Option<usize> {
        self.buses
            .iter()
            .position(|b| b.devices.iter().any(|(r, _)| r == reference))
    }

    fn check_of(&self, reference: &str) -> Option<&'a Record> {
        self.records
            .iter()
            .find(|r| matches!(r, Record::I2cCheck { reference: x, .. } if x == reference))
    }

    /// Kinds 7 past ceiling, 8 and 9 on bus `b`.
    fn faults(&self, b: usize) -> Vec<usize> {
        let id = self.buses[b].id;
        self.indices(|r| match r {
            Record::RiseMeasured { bus, exceeds, .. } => bus == id && *exceeds,
            Record::LineStuckHigh { bus, .. } | Record::BusFault { bus, .. } => bus == id,
            _ => false,
        })
    }

    fn rises(&self, b: usize) -> Vec<usize> {
        let id = self.buses[b].id;
        self.indices(|r| matches!(r, Record::RiseMeasured { bus, .. } if bus == id))
    }

    /// No fault, and each of the bus's lines has exactly one measured rise,
    /// complete and within its ceiling.
    fn healthy(&self, b: usize) -> bool {
        let bus = &self.buses[b];
        self.faults(b).is_empty()
            && bus.lines.iter().all(|line| {
                let on_line: Vec<&Record> = self
                    .records
                    .iter()
                    .filter(|r| {
                        matches!(r, Record::RiseMeasured { bus: x, net, .. }
                            if x == bus.id && *net == line.net)
                    })
                    .collect();
                matches!(
                    on_line.as_slice(),
                    [Record::RiseMeasured {
                        complete: true,
                        exceeds: false,
                        ..
                    }]
                )
            })
    }

    fn indices(&self, pred: impl Fn(&Record) -> bool) -> Vec<usize> {
        (0..self.records.len())
            .filter(|&i| pred(&self.records[i]))
            .collect()
    }

    /// The controller witness on bus `b` disagrees with its solved device:
    /// `SADD` always (both phases write it), `NBYTES` once the read phase
    /// began, `TXDR` once the write phase ran. A witness whose own expectation
    /// is not the solved address and register is no evidence.
    fn witness_wrong(&self, b: usize, record: &Record) -> bool {
        let Record::I2cWitness {
            reference,
            cr2,
            txdr,
            ..
        } = record
        else {
            return false;
        };
        let Some((_, address)) = self.buses[b].devices.iter().find(|(r, _)| r == reference) else {
            return false;
        };
        let Some(Record::I2cCheck {
            register, status, ..
        }) = self.check_of(reference)
        else {
            return false;
        };
        let sadd_want = (u32::from(*address) << 1) << self.sadd.trailing_zeros();
        let covered = cr2.mask & (self.sadd | self.nbytes) == self.sadd | self.nbytes
            && txdr.mask & TXDR_DATA == TXDR_DATA;
        if !covered
            || cr2.expected & self.sadd != sadd_want
            || txdr.expected & TXDR_DATA != u32::from(*register)
        {
            return false;
        }
        let status = status.as_deref().unwrap_or("");
        let sadd = cr2.observed.is_some_and(|o| o & self.sadd != sadd_want);
        let nbytes = READ_PHASE.contains(&status)
            && cr2
                .observed
                .is_some_and(|o| o & self.nbytes != cr2.expected & self.nbytes);
        let data = WROTE_TXDR.contains(&status)
            && txdr
                .observed
                .is_some_and(|o| o & TXDR_DATA != u32::from(*register));
        sadd || nbytes || data
    }

    /// The fields of `reading` the bus owns that are wrong.
    fn owned_wrong(&self, b: usize, name: &str, reading: &Reading) -> bool {
        self.buses[b].owned.iter().any(|o| wrong(o, name, reading))
    }

    /// Positive software evidence on bus `b`: a witness or an owned register.
    fn software(&self, b: usize) -> Vec<usize> {
        self.indices(|r| match r {
            Record::I2cWitness { .. } => self.witness_wrong(b, r),
            Record::RegisterObserved { name, reading } => self.owned_wrong(b, name, reading),
            _ => false,
        })
    }

    /// The bits of a register reading that are wrong and that no bus owns.
    fn unowned(&self, name: &str, r: &Reading) -> u32 {
        let owned = self
            .buses
            .iter()
            .flat_map(|b| &b.owned)
            .filter(|o| o.name == name && o.address == r.address)
            .fold(0, |m, o| m | o.mask);
        mismatch(r) & !owned
    }

    /// Software evidence on no bus: the boot, and registers wrong outside
    /// every bus's fields.
    fn board_software(&self) -> Vec<usize> {
        self.indices(|r| match r {
            Record::BootObserved {
                boot_word,
                instructions_expected,
                instructions_observed,
                ..
            } => {
                boot_word.observed != Some(boot_word.expected)
                    || instructions_observed.is_some_and(|n| n < *instructions_expected)
            }
            Record::RegisterObserved { name, reading } => self.unowned(name, reading) != 0,
            _ => false,
        })
    }

    fn scope(&self, i: usize) -> Option<Scope> {
        Some(match &self.records[i] {
            Record::RiseMeasured { bus, .. }
            | Record::LineStuckHigh { bus, .. }
            | Record::BusFault { bus, .. } => Scope::Bus(self.bus_named(bus)?),
            Record::I2cCheck { reference, .. } | Record::I2cWitness { reference, .. } => {
                Scope::Bus(self.bus_of_device(reference)?)
            }
            Record::RegisterObserved { name, reading } => {
                let on: Vec<usize> = (0..self.buses.len())
                    .filter(|&b| self.owned_wrong(b, name, reading))
                    .collect();
                match on.as_slice() {
                    [b] if self.unowned(name, reading) == 0 => Scope::Bus(*b),
                    _ => Scope::Board,
                }
            }
            _ => Scope::Board,
        })
    }

    /// The events a failing finding kind selects, and whether their number
    /// must equal the finding's count. `None` for a kind no event shows.
    fn select(&self, kind: &str) -> Option<(Vec<usize>, bool)> {
        let status_is = |r: &Record, f: &dyn Fn(&str) -> bool| matches!(r, Record::I2cCheck { status: Some(s), .. } if f(s));
        let events = match kind {
            campaign::PAST_CEILING_FINDING => {
                self.indices(|r| matches!(r, Record::RiseMeasured { exceeds: true, .. }))
            }
            campaign::STUCK_HIGH_FINDING => {
                self.indices(|r| matches!(r, Record::LineStuckHigh { .. }))
            }
            campaign::HELD_LOW_FINDING => self.indices(
                |r| matches!(r, Record::BusFault { from, .. } if from.iter().any(|f| f == HELD_LOW)),
            ),
            "i2c_timeout" => self.indices(|r| status_is(r, &|s| s.starts_with("timeout_"))),
            "i2c_record_invalid" => self.indices(|r| status_is(r, &|s| s == "invalid")),
            "i2c_identity_mismatch" => self.indices(|r| {
                matches!(r, Record::I2cCheck { status: Some(s), expected, observed, .. }
                    if s == "ok" && *observed != Some(*expected))
            }),
            "i2c_transaction_unwitnessed" => self.indices(|r| match r {
                Record::I2cWitness {
                    reference,
                    cr2,
                    txdr,
                    ..
                } => {
                    matches!(self.check_of(reference),
                        Some(Record::I2cCheck { status: Some(s), .. }) if s == "ok")
                        && (mismatch(cr2) != 0 || mismatch(txdr) != 0)
                }
                _ => false,
            }),
            "register_mismatch" => self.indices(
                |r| matches!(r, Record::RegisterObserved { reading, .. } if mismatch(reading) != 0),
            ),
            "boot_word_missing" => self.indices(|r| {
                matches!(r, Record::BootObserved { boot_word, .. }
                    if boot_word.observed != Some(boot_word.expected))
            }),
            "cpu_halted" => self.indices(|r| {
                matches!(r, Record::BootObserved { instructions_expected, instructions_observed, .. }
                    if instructions_observed.is_some_and(|n| n < *instructions_expected))
            }),
            // One finding for every access to a gated peripheral: the RCC
            // enable bits the boot left off are the same fact, in the archive.
            "locked_peripheral_access" => {
                return Some((
                    self.indices(|r| match r {
                        Record::RegisterObserved { reading, .. } => {
                            (self.rcc.0..self.rcc.1).contains(&reading.address)
                                && reading
                                    .observed
                                    .is_some_and(|o| reading.expected & reading.mask & !o != 0)
                        }
                        _ => false,
                    }),
                    false,
                ));
            }
            _ => return None,
        };
        Some((events, true))
    }

    /// One symptom's rule, its evidence and its masked events.
    fn decide(&self, scope: Scope) -> (Rule, Vec<usize>, Vec<usize>) {
        let all = 0..self.buses.len();
        let faulted: Vec<usize> = all
            .clone()
            .filter(|&b| !self.faults(b).is_empty())
            .collect();
        let faults_of = |buses: &[usize]| -> Vec<usize> {
            buses.iter().flat_map(|&b| self.faults(b)).collect()
        };
        match scope {
            Scope::Bus(b) if faulted.contains(&b) => (Rule::E1, self.faults(b), self.software(b)),
            Scope::Bus(b) if !self.healthy(b) => (Rule::I2, vec![self.buses[b].at], vec![]),
            Scope::Bus(b) => {
                let software = self.software(b);
                if !software.is_empty() {
                    (Rule::S1, [software, self.rises(b)].concat(), vec![])
                } else if !faulted.is_empty() {
                    (
                        Rule::I3,
                        [self.rises(b), faults_of(&faulted)].concat(),
                        vec![],
                    )
                } else {
                    (Rule::I4, self.rises(b), vec![])
                }
            }
            Scope::Board if !faulted.is_empty() => (Rule::I3, faults_of(&faulted), vec![]),
            Scope::Board => {
                let unmeasured: Vec<usize> = all.clone().filter(|&b| !self.healthy(b)).collect();
                let rises: Vec<usize> = all.flat_map(|b| self.rises(b)).collect();
                if self.buses.is_empty() || !unmeasured.is_empty() {
                    let at = unmeasured.iter().map(|&b| self.buses[b].at).collect();
                    return (Rule::I2, at, vec![]);
                }
                let software = self.board_software();
                if software.is_empty() {
                    (Rule::I4, rises, vec![])
                } else {
                    (Rule::S2, [software, rises].concat(), vec![])
                }
            }
        }
    }
}

/// Attribute one archived campaign run. The run is first judged by exactly
/// the rules `replay` applies after its reader ([`campaign::check_archived`]),
/// so a forged join or a broken record order is refused, never attributed.
pub fn attribute(run: &Replayed) -> Result<Attribution, AttributeError> {
    let summary = campaign::check_archived(&run.header, &run.state)
        .map_err(AttributeError::NotACampaignRun)?;
    let records = run.state.records.as_slice();
    let f = cmsis::register_facts()?;
    let rcc = f.base("RCC")?;
    let mut buses = Vec::new();
    for (at, r) in records.iter().enumerate() {
        if let Record::I2cBusSolved {
            bus,
            peripheral,
            lines,
            devices,
        } = r
        {
            buses.push(Bus {
                at,
                id: bus,
                lines,
                devices,
                owned: owned(&f, peripheral, lines)?,
            });
        }
    }
    let state = Run {
        records,
        buses,
        sadd: field(f.position("I2C_CR2_SADD_Pos")?, SADD_BITS),
        nbytes: field(f.position("I2C_CR2_NBYTES_Pos")?, NBYTES_BITS),
        rcc: (rcc, rcc + f.layout("RCC")?.size),
    };
    let Some(Record::Verdict { findings, .. }) = records.last() else {
        unreachable!("check_run puts the verdict last");
    };

    let mut symptoms = BTreeSet::new();
    let mut unexplained = BTreeSet::new();
    for (kind, severity, count) in findings {
        if severity == "info" {
            continue;
        }
        match state.select(kind) {
            Some((events, exact))
                if !events.is_empty() && (!exact || events.len() as u64 == *count) =>
            {
                symptoms.extend(events);
            }
            _ => {
                unexplained.insert(kind.clone());
            }
        }
    }

    let mut decided = Vec::new();
    for &i in &symptoms {
        match state.scope(i) {
            Some(scope) => decided.push(state.decide(scope)),
            // R2 joins every device and bus; a symptom with none is unexplained.
            None => {
                unexplained.insert(format!("event {i}"));
            }
        }
    }
    let rules: BTreeSet<Rule> = decided.iter().map(|d| d.0).collect();
    let domains: BTreeSet<Domain> = rules.iter().map(|r| r.domain()).collect();
    let rule = if summary.pass {
        Rule::N1
    } else if !unexplained.is_empty() {
        Rule::I1
    } else if let Some(r) = rules.iter().find(|r| r.domain() == Domain::Indeterminate) {
        *r
    } else if domains.len() > 1 {
        Rule::I5
    } else {
        *rules.first().expect("a failing run has a failing finding")
    };
    let mut evidence = BTreeSet::new();
    let mut masked = BTreeSet::new();
    // An electrical or software run gathers every symptom's evidence (they
    // share the domain); I5 gathers both halves; I2 to I4 only what decided
    // them; I1 has none.
    let gathers = |r: Rule| match rule {
        Rule::I1 => false,
        Rule::I2 | Rule::I3 | Rule::I4 => r == rule,
        _ => true,
    };
    for (r, e, m) in &decided {
        if gathers(*r) {
            evidence.extend(e.iter().copied());
        }
        masked.extend(m.iter().copied());
    }
    let refs = |set: BTreeSet<usize>| -> Vec<EventRef> {
        set.into_iter()
            .map(|i| EventRef {
                index: i as u64,
                kind: records[i].kind(),
            })
            .collect()
    };
    Ok(Attribution {
        run_id: run.header.run_id,
        state_hash: run.manifest.state_hash,
        domain: rule.domain(),
        rule,
        evidence: refs(evidence),
        symptoms: refs(symptoms),
        masked: refs(masked),
        unexplained: unexplained.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::split_pin;

    #[test]
    fn pins_split_into_port_and_index() {
        assert_eq!(split_pin("PB6"), Some(('B', 6)));
        assert_eq!(split_pin("PA15"), Some(('A', 15)));
        for bad in ["B6", "Pb6", "PB", "PB123", "PB6x", "", "P"] {
            assert_eq!(split_pin(bad), None, "{bad}");
        }
    }
}
