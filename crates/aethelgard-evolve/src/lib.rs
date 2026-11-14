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

use std::collections::{BTreeMap, BTreeSet};

pub mod propose;

use aethelgard_chaos::Replayed;
use aethelgard_chaos::campaign::{self, HELD_LOW};
use aethelgard_chaos::record::{BusLine, Reading, Record, split_pin};
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
    /// The symptom's bus is computed healthy (its rises, computed from the
    /// declared values, are within their ceilings), and the firmware's own state on
    /// that bus differs from the solved topology.
    S1,
    /// The symptom is on no bus (the boot, or a register no bus joins), every
    /// bus is computed healthy, and the boot or such a register is wrong.
    S2,
    /// The run passed.
    N1,
    /// A failing finding has no archived event that shows it, or an event
    /// shows a failure no finding names.
    I1,
    /// A line of the symptom's bus (or, for a symptom on no bus, of any bus)
    /// has no computed rise, the header cannot join the bus, or there is no bus.
    I2,
    /// The symptom's bus is computed healthy with no software evidence, and
    /// the only electrical fault is on another bus.
    I3,
    /// Every bus is computed healthy and no software evidence exists.
    I4,
    /// Some symptoms are electrical and others software.
    I5,
}

impl Rule {
    /// Every rule, in order.
    pub const ALL: [Rule; 9] = [
        Self::E1,
        Self::S1,
        Self::S2,
        Self::N1,
        Self::I1,
        Self::I2,
        Self::I3,
        Self::I4,
        Self::I5,
    ];

    /// The rule whose [`id`](Self::id) is `id`, so a reader of an attribution
    /// or a manifest finds the rule, and its evidence kinds, here alone.
    pub fn from_id(id: &str) -> Option<Rule> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

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

/// One field of a register or of a device witness, read through the join
/// attribution uses: what the solved topology fixes it to, what the run
/// observed under the same mask, and whether attribution counts it as
/// evidence. `register` is named as the ST header names it (`RCC.APB1LENR`,
/// `GPIOB.AFRL`), `boot_word`, or a witness's `i2c1.u2.address`, `.Write`
/// or `.Read`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub register: String,
    pub address: u32,
    pub mask: u32,
    pub solved: u32,
    pub observed: Option<u32>,
    pub wrong: bool,
}

/// What one archive event says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    /// Kind 7.
    Rise {
        net: String,
        rise_ps: u64,
        complete: bool,
        ceiling_ps: u64,
        exceeds: bool,
    },
    /// Kind 8.
    StuckHigh { net: String },
    /// Kind 9: its causes, as the analog pass names them, and its nets.
    BusFault {
        from: Vec<String>,
        nets: Vec<String>,
    },
    /// Kind 10: with whether the core ended outside every executable
    /// segment the record archives (`AET-WO-2205` §1.5).
    Boot {
        boot_word: Field,
        instructions_expected: u64,
        instructions_observed: Option<u64>,
        pc: Option<u32>,
        pc_outside: bool,
    },
    /// Kind 11, or kind 13 with its device: each field the join gives a
    /// solved value. A witness that is no evidence either way has none.
    Fields {
        device: Option<String>,
        fields: Vec<Field>,
    },
    /// Any other kind.
    Other,
}

/// One event of a run with its bus and what it says. The bus is where
/// `attribute` puts the event's symptom: a register is on a bus only when it
/// is wrong in that bus's fields alone, and on none (`None`) otherwise, even
/// when it is also evidence on a bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub event: EventRef,
    pub bus: Option<String>,
    pub view: View,
}

/// A computed rise (kind 7) of one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rise {
    pub rise_ps: u64,
    pub complete: bool,
}

/// One line of a solved bus (kind 15), with every rise the run computed for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineRises {
    pub net: String,
    pub ceiling_ps: u64,
    pub rises: Vec<Rise>,
}

/// One solved bus (kind 15) and its lines' computed rises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolvedBus {
    pub bus: String,
    pub lines: Vec<LineRises>,
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
// does not evaluate, so the widths are stated here from RM0433 (§11.4.1: MODER
// 2 bits a pin; §11.4.9: AFSEL 4 bits a pin). Every position comes from the
// header, as the judge's does.
const MODE_BITS: u32 = 2;
const AFSEL_BITS: u32 = 4;
// belt-and-suspenders: the twin's judge states this too (RM0433 §11.4.1); a
// pin with an alternate function is in alternate-function mode, so the mode a
// line's pin must hold follows from kind 15, not from the reading.
const MODE_AF: u32 = 0b10;

fn field(pos: u32, bits: u32) -> u32 {
    ((1u32 << bits) - 1) << pos
}

/// A register field one bus owns: a register by name and address, the field's
/// mask, and the value it must hold under that mask, from kind 15 alone.
#[derive(Clone, Debug)]
struct Owned {
    name: String,
    address: u32,
    mask: u32,
    want: u32,
}

/// What the ST header says of one solved bus: the register fields it owns
/// (`AET-WO-3100` R5).
struct Joined {
    owned: Vec<Owned>,
}

/// The fields a bus owns: its controller's clock gate, the gates of its pins'
/// ports, and each pin's mode and alternate function. Names, addresses and
/// positions come from the header.
fn join(f: &RegisterFacts, peripheral: &str, lines: &[BusLine]) -> Result<Joined, RegisterError> {
    let rcc = f.base("RCC")?;
    let rcc_layout = f.layout("RCC")?;
    let gpio = f.layout("GPIO")?;
    let gate = |block: &str| -> Result<Owned, RegisterError> {
        let (register, bit) = f.enable_bit(block)?;
        Ok(Owned {
            address: rcc + rcc_layout.offset(&register, 0)?,
            name: format!("RCC.{register}"),
            mask: 1 << bit,
            want: 1 << bit,
        })
    };
    let mut owned = vec![gate(peripheral)?];
    for line in lines {
        let (port, index) = split_pin(&line.pin).ok_or_else(|| RegisterError::Undefined {
            name: format!("pin {}", line.pin),
        })?;
        let block = format!("GPIO{port}");
        let base = f.base(&block)?;
        owned.push(gate(&block)?);
        let mode = f.position(&format!("GPIO_MODER_MODE{index}_Pos"))?;
        owned.push(Owned {
            name: format!("{block}.MODER"),
            address: base + gpio.offset("MODER", 0)?,
            mask: field(mode, MODE_BITS),
            want: MODE_AF << mode,
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
        owned.push(Owned {
            name: format!("{block}.{half}"),
            address: base + gpio.offset("AFR", word)?,
            mask: field(pos, AFSEL_BITS),
            want: u32::from(line.alternate_function) << pos,
        });
    }
    Ok(Joined { owned })
}

/// Whether `owned`'s field in `reading` (of register `name`) is wrong, as a
/// fact about the firmware: the register is the one named, at its address, the
/// field is covered and observed, and differs from what it must hold. A
/// reading whose own expectation disagrees with the solved value is no
/// evidence either way.
fn wrong(o: &Owned, name: &str, r: &Reading) -> bool {
    let Some(observed) = r.observed else {
        return false;
    };
    name == o.name
        && r.address == o.address
        && r.mask & o.mask == o.mask
        && r.expected & o.mask == o.want
        && observed & o.mask != o.want
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
    /// `None` when the header lacks a fact the bus names (a port or a pin it
    /// does not have): the bus is then never computed healthy and owns nothing,
    /// so its symptoms are `I2`, and no other bus's attribution is refused.
    joined: Option<Joined>,
}

/// Where a symptom is: on a bus, for a device of it or not, or on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope<'a> {
    Bus(usize, Option<&'a str>),
    Board,
}

struct Run<'a> {
    records: &'a [Record],
    buses: Vec<Bus<'a>>,
    rcc: (u32, u32),
}

/// The bytes of a witness line `[NOISY] {device}: {what} {bytes}`, as the
/// model logs them (each byte in upper-case hexadecimal, joined by spaces),
/// first byte most significant, with their count; `None` for any other line.
fn witness_bytes(line: &str, device: &str, what: &str) -> Option<(u64, usize)> {
    let rest = line
        .strip_prefix("[NOISY] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")?
        .strip_prefix(what)?
        .strip_prefix(' ')?;
    let bytes: Vec<u8> = rest
        .split(' ')
        .map(|b| {
            (!b.is_empty() && b.len() <= 2 && b.bytes().all(|c| c.is_ascii_hexdigit()))
                .then(|| u8::from_str_radix(b, 16).ok())
                .flatten()
        })
        .collect::<Option<_>>()?;
    (1..=8).contains(&bytes.len()).then(|| {
        let v = bytes.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b));
        (v, bytes.len())
    })
}
/// The finding kinds that each select their own events, one finding each.
const PER_EVENT: [&str; 11] = [
    campaign::PAST_CEILING_FINDING,
    campaign::STUCK_HIGH_FINDING,
    campaign::HELD_LOW_FINDING,
    "i2c_timeout",
    "i2c_record_invalid",
    "i2c_identity_mismatch",
    "i2c_transaction_unwitnessed",
    "register_mismatch",
    "boot_word_missing",
    "cpu_halted",
    "locked_peripheral_access",
];

impl<'a> Run<'a> {
    fn bus_named(&self, id: &str) -> Option<usize> {
        self.buses.iter().position(|b| b.id == id)
    }

    fn bus_of_device(&self, reference: &str) -> Option<usize> {
        self.buses
            .iter()
            .position(|b| b.devices.iter().any(|(r, _)| r == reference))
    }

    /// The device's identity read: at most one, by R2.
    fn check_of(&self, reference: &str) -> Option<&'a Record> {
        self.records
            .iter()
            .find(|r| matches!(r, Record::I2cCheck { reference: x, .. } if x == reference))
    }

    /// Kinds 7 past ceiling, 8 and 9 on bus `b`.
    fn faults(&self, b: usize) -> Vec<usize> {
        let id = self.buses[b].id;
        self.indices(|r| match r {
            Record::RiseComputed { bus, exceeds, .. } => bus == id && *exceeds,
            Record::LineStuckHigh { bus, .. } | Record::BusFault { bus, .. } => bus == id,
            _ => false,
        })
    }

    fn rises(&self, b: usize) -> Vec<usize> {
        let id = self.buses[b].id;
        self.indices(|r| matches!(r, Record::RiseComputed { bus, .. } if bus == id))
    }

    /// For a bus with no fault (`decide` asks only of those): the header knows
    /// it, and each of its lines has its computed rise. R2 allows one per line,
    /// and a rise past its ceiling, incomplete ones included, is a fault.
    fn healthy(&self, b: usize) -> bool {
        let bus = &self.buses[b];
        bus.joined.is_some()
            && bus.lines.iter().all(|line| {
                self.records.iter().any(|r| {
                    matches!(r, Record::RiseComputed { bus: x, net, .. }
                        if x == bus.id && *net == line.net)
                })
            })
    }

    fn indices(&self, pred: impl Fn(&Record) -> bool) -> Vec<usize> {
        (0..self.records.len())
            .filter(|&i| pred(&self.records[i]))
            .collect()
    }

    /// A device's witness on bus `b` disagrees with its solved read
    /// (`AET-WO-2204`): the model logged a write of another register, or
    /// returned another value, so the bus reached the device and the firmware
    /// asked for the wrong thing; or the record says the read completed and
    /// the model logged nothing, which only the firmware's own claim can do. A
    /// witness whose own expectation is not the solved register and value, or
    /// that could not be read whole, is no evidence. A transfer to a wrong
    /// address leaves the device's log empty, as a fault on the bus does, but
    /// its controller logs that it found no device at that address: an
    /// address other than the device's is evidence. A
    /// faulted bus finds none at the device's own, which is not.
    fn witness_wrong(&self, b: usize, record: &Record) -> bool {
        self.witness_fields(b, record)
            .is_some_and(|f| f.iter().any(|f| f.wrong))
    }

    /// A sound witness's register written and value read, each with its
    /// solved value, the first the model logged, and whether it counts as
    /// evidence ([`Self::witness_wrong`]); `None` for a witness that is no
    /// evidence.
    fn witness_fields(&self, b: usize, record: &Record) -> Option<[Field; 3]> {
        let Record::I2cWitness {
            reference,
            device,
            expected,
            observed,
            absent,
            ..
        } = record
        else {
            return None;
        };
        let bus = &self.buses[b];
        bus.joined.as_ref()?;
        let (_, address) = bus.devices.iter().find(|(r, _)| r == reference)?;
        let Some(Record::I2cCheck {
            register,
            expected: value,
            status,
            ..
        }) = self.check_of(reference)
        else {
            return None;
        };
        let observed = observed.as_deref()?;
        let logged = |lines: &[String], what: &str| {
            lines.iter().find_map(|l| witness_bytes(l, device, what))
        };
        let (Some(reg_want), Some(value_want)) =
            (logged(expected, "Write"), logged(expected, "Read"))
        else {
            return None;
        };
        let width = u32::try_from(value_want.1).ok()?;
        if reg_want.0 != u64::from(*register) || value_want.0 != *value || width > 4 {
            return None;
        }
        let claimed = status.as_deref() == Some("ok");
        let silent = claimed && observed.is_empty();
        let reg = logged(observed, "Write").map(|(v, _)| v);
        let read = logged(observed, "Read").map(|(v, _)| v);
        let mask = if width == 4 {
            u32::MAX
        } else {
            (1u32 << (8 * width)) - 1
        };
        let at = |name: &str, mask: u32, solved: u64, got: Option<u64>| Field {
            register: format!("{device}.{name}"),
            address: u32::from(*address),
            mask,
            solved: solved as u32,
            observed: got.map(|g| g as u32 & mask),
            wrong: silent || got.is_some_and(|g| g != solved),
        };
        let elsewhere = absent.iter().copied().find(|a| a != address);
        Some([
            Field {
                register: format!("{device}.address"),
                address: u32::from(*address),
                mask: 0x7F,
                solved: u32::from(*address),
                observed: elsewhere.map(u32::from),
                wrong: elsewhere.is_some(),
            },
            at("Write", 0xFF, reg_want.0, reg),
            at("Read", mask, value_want.0, read),
        ])
    }

    /// The fields of `reading` the bus owns that are wrong.
    fn owned_wrong(&self, b: usize, name: &str, reading: &Reading) -> bool {
        self.buses[b]
            .joined
            .iter()
            .flat_map(|j| &j.owned)
            .any(|o| wrong(o, name, reading))
    }

    /// Positive software evidence on bus `b`: an owned register that is wrong,
    /// and a witness that disagrees: only `device`'s, for a device's symptom,
    /// since another device's transaction says nothing of this one's.
    fn software(&self, b: usize, device: Option<&str>) -> Vec<usize> {
        self.indices(|r| match r {
            Record::I2cWitness { reference, .. } => {
                device.is_none_or(|d| d == reference) && self.witness_wrong(b, r)
            }
            Record::RegisterObserved { name, reading } => self.owned_wrong(b, name, reading),
            _ => false,
        })
    }

    /// The bits of a register reading that are wrong and that no bus owns.
    fn unowned(&self, name: &str, r: &Reading) -> u32 {
        let owned = self
            .buses
            .iter()
            .flat_map(|b| b.joined.iter().flat_map(|j| &j.owned))
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
                    || pc_outside(r)
            }
            Record::RegisterObserved { name, reading } => self.unowned(name, reading) != 0,
            _ => false,
        })
    }

    fn scope(&self, i: usize) -> Option<Scope<'a>> {
        Some(match &self.records[i] {
            Record::RiseComputed { bus, .. }
            | Record::LineStuckHigh { bus, .. }
            | Record::BusFault { bus, .. } => Scope::Bus(self.bus_named(bus)?, None),
            Record::I2cCheck { reference, .. } | Record::I2cWitness { reference, .. } => {
                Scope::Bus(self.bus_of_device(reference)?, Some(reference.as_str()))
            }
            Record::RegisterObserved { name, reading } => {
                let on: Vec<usize> = (0..self.buses.len())
                    .filter(|&b| self.owned_wrong(b, name, reading))
                    .collect();
                match on.as_slice() {
                    [b] if self.unowned(name, reading) == 0 => Scope::Bus(*b, None),
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
                self.indices(|r| matches!(r, Record::RiseComputed { exceeds: true, .. }))
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
                    expected,
                    observed,
                    ..
                } => {
                    matches!(self.check_of(reference),
                        Some(Record::I2cCheck { status: Some(s), .. }) if s == "ok")
                        && observed.as_deref() != Some(expected.as_slice())
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
            campaign::PC_OUTSIDE_FINDING => self.indices(pc_outside),
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
            Scope::Bus(b, _) if faulted.contains(&b) => {
                (Rule::E1, self.faults(b), self.software(b, None))
            }
            Scope::Bus(b, _) if !self.healthy(b) => (Rule::I2, vec![self.buses[b].at], vec![]),
            Scope::Bus(b, device) => {
                let software = self.software(b, device);
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

/// Kind 10 with its `pc` outside every executable segment it archives: the
/// core ran where the firmware put no code (`AET-WO-2205` §1.5, `3100` F13).
fn pc_outside(r: &Record) -> bool {
    matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
        if !aethelgard_chaos::record::inside(executable, *pc))
}

/// The run, judged by exactly the rules `replay` applies after its reader
/// ([`campaign::check_archived`]), with every solved bus joined to the ST
/// header; and whether its verdict passed.
fn joined(run: &Replayed) -> Result<(Run<'_>, bool), AttributeError> {
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
                joined: join(&f, peripheral, lines).ok(),
            });
        }
    }
    let state = Run {
        records,
        buses,
        rcc: (rcc, rcc + f.layout("RCC")?.size),
    };
    Ok((state, summary.pass))
}

/// A read-only view of one run through the join attribution uses
/// (`AET-WO-3200` §8, review A8.12): for an event, its bus, and for a
/// register or witness event, the solved value against the observed one.
/// `attribute` itself does not use it, so its output is unchanged.
pub struct RunView<'a>(Run<'a>);

/// The view of `run`, judged first as [`attribute`] judges it.
pub fn view(run: &Replayed) -> Result<RunView<'_>, AttributeError> {
    joined(run).map(|(state, _)| RunView(state))
}

impl RunView<'_> {
    /// What event `e` says; `None` when the run has no event `e` of that kind.
    pub fn event(&self, e: EventRef) -> Option<Evidence> {
        let run = &self.0;
        let i = usize::try_from(e.index).ok()?;
        let record = run.records.get(i).filter(|r| r.kind() == e.kind)?;
        let bus_of = |b: usize| run.buses[b].id.to_owned();
        let bus = match run.scope(i) {
            Some(Scope::Bus(b, _)) => Some(bus_of(b)),
            _ => None,
        };
        let view = match record {
            Record::RiseComputed {
                net,
                rise_30_70_ps,
                complete,
                ceiling_ps,
                exceeds,
                ..
            } => View::Rise {
                net: net.clone(),
                rise_ps: *rise_30_70_ps,
                complete: *complete,
                ceiling_ps: *ceiling_ps,
                exceeds: *exceeds,
            },
            Record::LineStuckHigh { net, .. } => View::StuckHigh { net: net.clone() },
            Record::BusFault { from, nets, .. } => View::BusFault {
                from: from.clone(),
                nets: nets.clone(),
            },
            Record::BootObserved {
                boot_word,
                instructions_expected,
                instructions_observed,
                pc,
                ..
            } => View::Boot {
                pc: *pc,
                pc_outside: pc_outside(record),
                boot_word: Field {
                    register: "boot_word".to_owned(),
                    address: boot_word.address,
                    mask: boot_word.mask,
                    solved: boot_word.expected,
                    observed: boot_word.observed,
                    wrong: boot_word.observed != Some(boot_word.expected),
                },
                instructions_expected: *instructions_expected,
                instructions_observed: *instructions_observed,
            },
            Record::RegisterObserved { name, reading } => {
                let mut fields: Vec<Field> = run
                    .buses
                    .iter()
                    .flat_map(|b| b.joined.iter().flat_map(|j| &j.owned))
                    .filter(|o| {
                        o.name == *name
                            && o.address == reading.address
                            && reading.mask & o.mask == o.mask
                    })
                    .map(|o| Field {
                        register: o.name.clone(),
                        address: o.address,
                        mask: o.mask,
                        solved: o.want,
                        observed: reading.observed.map(|v| v & o.mask),
                        wrong: wrong(o, name, reading),
                    })
                    .collect();
                // Two pins on one port own its one gate: say it once.
                let mut seen = Vec::new();
                fields.retain(|f| {
                    let new = !seen.contains(f);
                    seen.push(f.clone());
                    new
                });
                let unowned = run.unowned(name, reading);
                if unowned != 0 {
                    fields.push(Field {
                        register: name.clone(),
                        address: reading.address,
                        mask: unowned,
                        solved: reading.expected & unowned,
                        observed: reading.observed.map(|v| v & unowned),
                        wrong: true,
                    });
                }
                View::Fields {
                    device: None,
                    fields,
                }
            }
            Record::I2cWitness { reference, .. } => View::Fields {
                device: Some(reference.clone()),
                fields: run
                    .bus_of_device(reference)
                    .and_then(|b| run.witness_fields(b, record))
                    .map(Vec::from)
                    .unwrap_or_default(),
            },
            _ => View::Other,
        };
        Some(Evidence {
            event: e,
            bus,
            view,
        })
    }

    /// Every solved bus, in the run's order, with each line's computed rises.
    pub fn buses(&self) -> Vec<SolvedBus> {
        let run = &self.0;
        run.buses
            .iter()
            .map(|b| SolvedBus {
                bus: b.id.to_owned(),
                lines: b
                    .lines
                    .iter()
                    .map(|line| LineRises {
                        net: line.net.clone(),
                        ceiling_ps: line.ceiling_ps,
                        rises: run
                            .records
                            .iter()
                            .filter_map(|r| match r {
                                Record::RiseComputed {
                                    bus,
                                    net,
                                    rise_30_70_ps,
                                    complete,
                                    ..
                                } if bus == b.id && *net == line.net => Some(Rise {
                                    rise_ps: *rise_30_70_ps,
                                    complete: *complete,
                                }),
                                _ => None,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }
}

/// Attribute one archived campaign run. The run is first judged by exactly
/// the rules `replay` applies after its reader ([`campaign::check_archived`]),
/// so a forged join or a broken record order is refused, never attributed.
pub fn attribute(run: &Replayed) -> Result<Attribution, AttributeError> {
    let (state, pass) = joined(run)?;
    let records = state.records;
    let Some(Record::Verdict { findings, .. }) = records.last() else {
        unreachable!("check_run puts the verdict last");
    };
    // Each failing kind's count, summed over its severities above `info`.
    let mut failing: BTreeMap<&str, u64> = BTreeMap::new();
    for (kind, severity, count) in findings {
        if severity != "info" {
            *failing.entry(kind).or_default() += count;
        }
    }

    let mut symptoms = BTreeSet::new();
    let mut unexplained = BTreeSet::new();
    for (&kind, &count) in &failing {
        match state.select(kind) {
            Some((events, exact))
                if !events.is_empty() && (!exact || events.len() as u64 == count) =>
            {
                symptoms.extend(events);
            }
            _ => {
                unexplained.insert(kind.to_owned());
            }
        }
    }
    // And the converse: an event that shows a failure the verdict does not
    // name is unexplained too, so rewriting the verdict alone never moves the
    // domain (`AET-WO-3100` adversarial A1).
    for kind in PER_EVENT {
        for i in state.select(kind).map(|s| s.0).unwrap_or_default() {
            if !symptoms.contains(&i) {
                unexplained.insert(format!("event {i}"));
            }
        }
    }

    let mut decided = Vec::new();
    for &i in &symptoms {
        match state.scope(i) {
            Some(scope) => decided.push(state.decide(scope)),
            // belt-and-suspenders: R2 joins every bus and device, so `check_run`
            // has refused a symptom with no bus; this keeps it explained, not a panic.
            None => {
                unexplained.insert(format!("event {i}"));
            }
        }
    }
    let rules: BTreeSet<Rule> = decided.iter().map(|d| d.0).collect();
    let domains: BTreeSet<Domain> = rules.iter().map(|r| r.domain()).collect();
    let rule = if pass && unexplained.is_empty() {
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
    // The evidence is the named rule's: I5 names both halves', I1 none.
    let mut evidence = BTreeSet::new();
    let mut masked = BTreeSet::new();
    for (r, e, m) in &decided {
        if rule == Rule::I5 || *r == rule {
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
