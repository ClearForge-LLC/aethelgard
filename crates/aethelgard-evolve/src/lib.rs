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
//! never evidence. A finding that is the twin's own bound, never the device's
//! (`AET-WO-2302b`: rules `H1` to `H4`), is refused by name and attributed to
//! neither half. The rule table is `README.md`'s.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub mod lines;
pub mod propose;

use lines::{Form, Transaction};

use aethelgard_chaos::Replayed;
use aethelgard_chaos::campaign::{self, Bound};
use aethelgard_chaos::record::{BusLine, Override, Reading, Record, split_pin};
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
    /// The symptom's SPI device did not answer behind its own chip select:
    /// its controller's multiplexer routed the read to the device's own
    /// address and found nothing there, and the model logged nothing
    /// (`AET-WO-4200` D4).
    E2,
    /// The symptom's serial port is one whose RX line the scenario holds
    /// open (kind 5's `serial_rx_open`, `AET-WO-2302` D5), and the port
    /// shows exactly that: its witness (kind 29) has no frame gone in and
    /// no RX mux read at an injection (replay binds both to the override),
    /// and its echo missing; its framing check (kind 28) holds and its
    /// writes, read whole, hold their judgement; its pattern went out on
    /// its TX pin; and no field it owns is wrong (no software cause, so
    /// nothing is masked). With nothing injected the RX pin's mux at an
    /// injection is not read: its mux is the end's (kind 11, owned fields
    /// right) and the writes' (kind 28's tags: once solved, never changed).
    E3,
    /// The symptom's bus is computed healthy (its rises, computed from the
    /// declared values, are within their ceilings), and the firmware's own state on
    /// that bus differs from the solved topology.
    S1,
    /// The symptom is on no bus (the boot, or a register no bus joins), every
    /// bus is computed healthy, and the boot or such a register is wrong.
    S2,
    /// The symptom's SPI read was the firmware's doing: the multiplexer found
    /// no chip select asserted, or several; or the model logged a transfer
    /// other than the read's; or the record claims the read and nothing was
    /// logged or routed (`AET-WO-4200` D4).
    S3,
    /// The symptom's motor output was the firmware's doing (`AET-WO-4300a`):
    /// a register field the output owns read back other than the solved
    /// topology (its pin's mode or alternate function, its port's or timer's
    /// clock gate, or one of its timer's registers). The twin models nothing
    /// electrical on a timer's pin, so no rule attributes a motor symptom to
    /// the electrical half. `CEN` of a timer's `CR1` once the judge's stop
    /// cleared it (kind 30) is the judge's own act, never cited; a reading
    /// wrong in it alone is unexplained by name (ADV-M3).
    S4,
    /// The symptom's serial port was the firmware's doing (`AET-WO-4300b`):
    /// a register field the port owns read back other than the solved
    /// topology (its TX or RX pin's mode or alternate function, its pins'
    /// ports' or its USART's clock gate, or a register of its USART); or the
    /// firmware's writes, rebuilt from Renode's access log, failed the
    /// port's judgement (kind 28's tags: a word wrong at `UE` or changed
    /// after it, a write or a read the bring-up never makes, an RCC write
    /// but to an enable register or one clearing a gate it set, a pin
    /// misconfigured, changed once solved, or locked); or its TX pin was not
    /// routed at a boot's first `TDR` write (kind 29's `tx_at_send`;
    /// adversarial pass 2, M1; pass 3). Each is direct evidence of a write
    /// (a read: of a firmware that reads what it must not). The twin models nothing
    /// electrical on a USART's pins: only a declared open RX line (`E3`)
    /// attributes a serial symptom to the electrical half.
    S5,
    /// `motor_anchor_unobserved` (`AET-WO-2302b`): the output's witness
    /// (kind 26, status `unanchored`) logged edges with no `CEN` rise in the
    /// boot the judge reads (`cen_rises` 0, no `anchor_ns`): the firmware
    /// drove the pin, or started its timer, by a write no CPU store of `CR1`
    /// shows (a GPIO toggle, a DMA write). Every store width is hooked, and
    /// the twin drives no motor pin from the board.
    S6,
    /// `motor_anchor_repeated`: the timer's `CEN` rose more than once in the
    /// boot the judge reads (kind 26's `cen_rises`, its first output's
    /// standing for the timer): the firmware started it again. Not when
    /// `CEN` was set again after the judge's stop cleared it (kind 30, and
    /// kind 11's end read of `CR1` with `CEN` set): no record says when each
    /// rise fell, so a re-enable after the window, harmless on silicon where
    /// `CEN` was still set, cannot be told from a restart, and the kind is
    /// unexplained by name (ADV-M3).
    S7,
    /// `motor_anchor_out_of_order`, in a boot whose boot word is set (kind
    /// 10): the timer's anchor (kind 26's `anchor_ns`) is after the boot
    /// word's instant (placed by kind 29's first injection), which the
    /// bring-up writes last: the firmware's own code order. Decided only when
    /// the verdict names exactly the timers so anchored; its other cases are
    /// in no record, and are unexplained by name.
    S8,
    /// `motor_edge_before_anchor`: the output's witness (kind 26) shows
    /// edges before its timer's anchor in the same boot (`edges_before`):
    /// the firmware drove the pin before it started the timer.
    S9,
    /// `motor_edge_after_window`, once the judge's stop for the output's
    /// anchor ran and left `CEN` cleared (kind 30's `cr1_after`): edges
    /// after it (kind 26's `edges_after`) are the firmware's, which restarted
    /// the timer or drove the pin after the window. Without such a stop the
    /// edges may be the timer still running, and with `CEN` set again at the
    /// end (kind 11's `CR1`) a re-enable harmless on silicon (as for `S7`):
    /// either way the kind is unexplained by name.
    S10,
    /// `judge_stop_mismatch`: the judge's stop (kind 30), which left `CEN`
    /// cleared, read a solved register of the timer back off its solved
    /// value under its mask: the firmware wrote the timer inside its window
    /// (`register_mismatch`'s evidence at the window's end). A register the
    /// stop did not read back shows nothing, and is unexplained by name.
    S11,
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
    /// The symptom's serial port shows no write of the firmware's wrong (no
    /// wrong register field it owns, no failing write judgement, no unrouted
    /// send; an access log that could not be read shows none), or its static
    /// framing check failed, which is the topology's (`AET-WO-4300b`): never
    /// software by elimination.
    I6,
    /// `motor_window_past_run` (`AET-WO-2302b`), the twin's bound: a DShot
    /// timer's anchor (kind 26, its output's protocol kind 24's) plus
    /// `DSHOT_WINDOW_NS` passes the run's end, `MAX_VIRTUAL_NS`. The run is
    /// too short to witness the window; on silicon nothing is wrong. Its
    /// output's `stopped` witness, whose last rise reaches the run's end, is
    /// the same bound's.
    H1,
    /// `judge_stop_missing`, the judge's own act: its stop, due at the DShot
    /// timer's anchor (kind 26) plus `DSHOT_WINDOW_NS` inside the run, did
    /// not run in the boot it reads, or left `CEN` set (kind 30, when
    /// present).
    H2,
    /// `serial_wire_past_run`, the twin's bound: from the boot word's
    /// instant (kind 29's first injection), the ports' frames and echoes
    /// (kind 27's framings) end past `MAX_VIRTUAL_NS`. A port in that sum
    /// past the run whose echo alone is missing is the same bound's, unless
    /// its RX pin was off its AF at the injection: that is the pin's.
    H3,
    /// `serial_injection_out_of_order`, the judge's own act: its injection
    /// chain, one port after another (`AET-WO-2302` D4), put a port's frame
    /// in before the port before it had echoed (kind 29's `injected_ns`).
    H4,
}

impl Rule {
    /// Every rule, in order.
    pub const ALL: [Rule; 25] = [
        Self::E1,
        Self::E2,
        Self::E3,
        Self::S1,
        Self::S2,
        Self::S3,
        Self::S4,
        Self::S5,
        Self::S6,
        Self::S7,
        Self::S8,
        Self::S9,
        Self::S10,
        Self::S11,
        Self::N1,
        Self::I1,
        Self::I2,
        Self::I3,
        Self::I4,
        Self::I5,
        Self::I6,
        Self::H1,
        Self::H2,
        Self::H3,
        Self::H4,
    ];

    /// The rule whose [`id`](Self::id) is `id`, so a reader of an attribution
    /// or a manifest finds the rule, and its evidence kinds, here alone.
    pub fn from_id(id: &str) -> Option<Rule> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::E1 => "E1",
            Self::E2 => "E2",
            Self::E3 => "E3",
            Self::S1 => "S1",
            Self::S2 => "S2",
            Self::S3 => "S3",
            Self::S4 => "S4",
            Self::S5 => "S5",
            Self::S6 => "S6",
            Self::S7 => "S7",
            Self::S8 => "S8",
            Self::S9 => "S9",
            Self::S10 => "S10",
            Self::S11 => "S11",
            Self::N1 => "N1",
            Self::I1 => "I1",
            Self::I2 => "I2",
            Self::I3 => "I3",
            Self::I4 => "I4",
            Self::I5 => "I5",
            Self::I6 => "I6",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::H3 => "H3",
            Self::H4 => "H4",
        }
    }

    /// A harness rule (`H1` to `H4`) is `indeterminate`: it attributes its
    /// finding to neither half.
    pub fn domain(self) -> Domain {
        match self {
            Self::E1 | Self::E2 | Self::E3 => Domain::Electrical,
            Self::S1
            | Self::S2
            | Self::S3
            | Self::S4
            | Self::S5
            | Self::S6
            | Self::S7
            | Self::S8
            | Self::S9
            | Self::S10
            | Self::S11 => Domain::Software,
            Self::N1 => Domain::None,
            _ => Domain::Indeterminate,
        }
    }

    /// The twin's or the judge's own bound a harness rule refuses its
    /// finding by (`AET-WO-2302b`); `None` for every other rule.
    pub fn bound(self) -> Option<&'static str> {
        match self {
            Self::H1 => Some(
                "the DShot window's end, its anchor plus DSHOT_WINDOW_NS, past the run's MAX_VIRTUAL_NS",
            ),
            Self::H2 => Some("the judge's stop at its anchor plus DSHOT_WINDOW_NS"),
            Self::H3 => {
                Some("the run's MAX_VIRTUAL_NS against the ports' wire time from the boot word")
            }
            Self::H4 => Some("the judge's sequential injection, one port after another's echo"),
            _ => None,
        }
    }

    /// The event kinds this rule may name as evidence.
    pub fn evidence_kinds(self) -> &'static [u64] {
        match self {
            Self::E1 => &[7, 8, 9],
            Self::E2 => &[22],
            Self::E3 => &[28, 29],
            Self::S1 => &[7, 11, 13],
            Self::S2 => &[7, 10, 11],
            Self::S3 => &[21, 22],
            Self::S4 => &[11, 26],
            Self::S5 => &[11, 28, 29],
            Self::S6 | Self::S7 | Self::S9 => &[26],
            Self::S8 => &[10, 26, 29],
            Self::S10 => &[26, 30],
            Self::S11 => &[30],
            Self::N1 | Self::I1 => &[],
            Self::I2 => &[15],
            Self::I3 => &[7, 8, 9],
            Self::I4 => &[7, 25, 26],
            Self::I5 => &[7, 8, 9, 10, 11, 13, 21, 22, 26, 28, 29, 30],
            Self::I6 => &[28, 29],
            Self::H1 => &[24, 26],
            Self::H2 => &[24, 26, 30],
            Self::H3 => &[27, 29],
            Self::H4 => &[29],
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
    /// Failing finding kinds no archived event shows (rule `I1`), sorted;
    /// and one of the twin's window findings whose records cannot decide it
    /// (`AET-WO-2302b`), as `{kind}: {what the records lack}`, never a guess
    /// (`INV-06`).
    pub unexplained: Vec<String>,
}

/// One field of a register or of a device witness, read through the join
/// attribution uses: what the solved topology fixes it to, what the run
/// observed under the same mask, and whether attribution counts it as
/// evidence. `register` is named as the ST header names it (`RCC.APB1LENR`,
/// `GPIOB.AFRL`), `boot_word`, or a witness's `i2c1.u2.address`, `.Write`
/// or `.Read`; for a transaction its device's script does not hold
/// (`AET-WO-3306`), `i2c1.u5.transactions` and the step it was,
/// `i2c1.u2.identity.address`. The values are 64-bit, so a read of up to 8
/// bytes shows whole (`AET-WO-4210` F-B5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub register: String,
    pub address: u32,
    pub mask: u64,
    pub solved: u64,
    pub observed: Option<u64>,
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
    /// Kind 26: a motor output's twin witness and what it showed
    /// (`AET-WO-4300a`): a symptom, never a solved expectation of its own.
    MotorWitness { label: String, status: String },
    /// Kind 29: a serial port's twin witness and its echo (`AET-WO-4300b`):
    /// a symptom, never a solved expectation of its own.
    SerialWitness { label: String, echo: String },
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
    /// The run holds more records than attribution is bounded for
    /// ([`MAX_ATTRIBUTE_RECORDS`], `AET-WO-2312`): its cost grows with the
    /// product of symptoms, buses and records, and it is an MCP tool.
    TooManyRecords { records: usize, limit: usize },
}

/// The most records a run may hold to be attributed (`AET-WO-2312`, review
/// round 3 A15). The largest campaign run measured holds 113
/// (`spike/2312/results.md`); at this bound the slowest shape measured
/// attributes in under a second, where 4,604 records took 14 s.
pub const MAX_ATTRIBUTE_RECORDS: usize = 1_024;

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

/// One solved motor output (kind 24, `AET-WO-4300a`) and the register
/// fields it owns: the bus join's pin and gate fields, from the header, and
/// its channel's fields of its timer's block (`timer`, `[start, end)`; see
/// [`timer_bits`]).
struct Motor<'a> {
    /// The kind 24's event index.
    at: usize,
    label: &'a str,
    /// Its protocol is DShot: its window ends `DSHOT_WINDOW_NS` after its
    /// anchor, where the judge's stop (kind 30) runs (`AET-WO-2302`).
    dshot: bool,
    /// `None` when the header lacks a fact the output names: it then owns
    /// nothing, as a bus that cannot join does.
    joined: Option<Joined>,
    timer: (&'a str, u32, u32),
    channel: u8,
}

/// One solved serial port (kind 27, `AET-WO-4300b`) and the register
/// fields it owns: the bus join's fields for its TX and RX pins and its
/// USART's gate, from the header, and every register of its USART's block
/// (`usart`, `[start, end)`).
struct Serial<'a> {
    label: &'a str,
    /// `None` when the header lacks a fact the port names: it then owns
    /// nothing, as a bus that cannot join does.
    joined: Option<Joined>,
    usart: (&'a str, u32, u32),
}

/// The bits of `reading` (of register `name`) that serial port `s` owns in
/// its USART's block: every bit the reading judges, the block's registers
/// being the port's alone; 0 outside it.
fn usart_bits(s: &Serial<'_>, name: &str, reading: &Reading) -> u32 {
    let (usart, start, end) = s.usart;
    let inside = name
        .strip_prefix(usart)
        .is_some_and(|rest| rest.starts_with('.'))
        && (start..end).contains(&reading.address);
    if inside { reading.mask } else { 0 }
}

/// Where a symptom is: on a bus, for a device of it or not, or on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope<'a> {
    Bus(usize, Option<&'a str>),
    /// An SPI device's read, by its reference (`AET-WO-4200`).
    Spi(&'a str),
    /// A motor output, by its position among the kind 24s (`AET-WO-4300a`).
    Motor(usize),
    /// A motor output's failing static check (kind 25): the topology's
    /// timing, never the firmware's (second adversarial review, F1).
    MotorStatic(usize),
    /// A serial port, by its position among the kind 27s (`AET-WO-4300b`).
    Serial(usize),
    /// A serial port's failing static framing check (kind 28): the
    /// topology's, never the firmware's.
    SerialStatic(usize),
    Board,
}

struct Run<'a> {
    records: &'a [Record],
    buses: Vec<Bus<'a>>,
    motors: Vec<Motor<'a>>,
    serial: Vec<Serial<'a>>,
    /// The boot word is set: the judge read the serial witnesses and tags.
    booted: bool,
    /// The resets the run rendered and released.
    releases: u64,
}

/// Bytes as one value, first byte most significant, with their count;
/// `None` for none or more than fit.
fn packed(bytes: &[u8]) -> Option<(u64, usize)> {
    (1..=8).contains(&bytes.len()).then(|| {
        let v = bytes.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b));
        (v, bytes.len())
    })
}

/// `lines`, read whole, as `device`'s transactions in its script's `form`
/// (`AET-WO-3306`; a BMP581's since `AET-WO-3309`), as the model logs them
/// ([`lines::transactions`]). `None` if any line is none (a warning, an
/// error, another device's), or for the ICP-101xx's form, whose witness
/// attribution does not judge.
fn transactions(lines: &[String], device: &str, form: Form) -> Option<Vec<Transaction>> {
    if form == Form::Icp {
        return None;
    }
    let (ts, none) = lines::transactions(lines, device, form);
    none.is_empty().then_some(ts)
}

/// A device's model path, whole script, whole log, and the witness each
/// line of the log went to ([`Run::whole_log`]).
type WholeLog<'a> = (&'a str, Vec<String>, Vec<String>, Vec<usize>);

/// A transaction's write and read widths, as a step's [`step_shape`].
type Shape = (Option<(u64, usize)>, usize);

/// A transaction's shape (`AET-WO-3306`): its write packed, and the bytes
/// its reads return in all.
fn shape(t: &Transaction) -> Shape {
    (packed(&t.written), t.read.as_ref().map_or(0, Vec::len))
}

/// A step's write and read width as a transaction's [`shape`].
fn step_shape(write: &[u8], width: u8) -> Shape {
    let value = write.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b));
    (Some((value, write.len())), usize::from(width))
}

impl<'a> Run<'a> {
    fn bus_named(&self, id: &str) -> Option<usize> {
        self.buses.iter().position(|b| b.id == id)
    }

    fn bus_of_device(&self, reference: &str) -> Option<usize> {
        self.buses
            .iter()
            .position(|b| b.devices.iter().any(|(r, _)| r == reference))
    }

    /// The device's read of `step`: at most one, by the run rules.
    fn check_of(&self, reference: &str, step: &str) -> Option<&'a Record> {
        self.records.iter().find(|r| {
            matches!(r, Record::I2cCheck { reference: x, step: s, .. } if x == reference && s == step)
        })
    }

    /// The device's SPI read of `step`: at most one, by the run rules.
    fn spi_check_of(&self, reference: &str, step: &str) -> Option<&'a Record> {
        self.records.iter().find(|r| {
            matches!(r, Record::SpiCheck { reference: x, step: s, .. } if x == reference && s == step)
        })
    }

    /// An SPI witness's fields, each with its solved value and whether it is
    /// software evidence ([`Self::spi_evidence`]), for the generator report:
    /// the chip select the multiplexer should have found asserted alone (its
    /// address), the command the model decoded, and the read the record
    /// claims. A witness that could not be read whole has none.
    fn spi_fields(&self, record: &Record) -> Vec<Field> {
        let Record::SpiWitness {
            reference,
            device,
            step,
            expected,
            observed: Some(observed),
            unrouted,
            ..
        } = record
        else {
            return Vec::new();
        };
        let Some(Record::SpiCheck {
            address,
            write,
            width,
            expected: value,
            observed: value_read,
            status,
            ..
        }) = self.spi_check_of(reference, step)
        else {
            return Vec::new();
        };
        // The command is the first byte a step writes.
        let command = write.first().copied().unwrap_or(0);
        let unselected = unrouted.iter().any(|l| {
            l.starts_with("[WARNING] ")
                && (l.contains(", but no device is currently selected - ")
                    || l.contains(", but multiple devices are currently selected ("))
        });
        // `GenericSpiFlash.cs` L129: the first byte a read sends is the
        // command, logged in the state that decodes it.
        let decoded = observed.iter().find_map(|l| {
            l.strip_prefix(&format!("[NOISY] {device}: Transmitting data 0x"))?
                .strip_suffix(", current state: RecognizeOperation")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
        });
        let claimed = status.as_deref() == Some("ok");
        let at = |name: &str, mask: u64, solved: u64, observed: Option<u64>, wrong: bool| Field {
            register: format!("{device}.{name}"),
            address: u32::from(*address),
            mask,
            solved,
            observed,
            wrong,
        };
        let mut fields = vec![
            at("select", 0xFF, u64::from(*address), None, unselected),
            at(
                "command",
                0xFF,
                u64::from(command),
                decoded.map(u64::from),
                (!observed.is_empty() && observed != expected)
                    || decoded.is_some_and(|d| d != u32::from(command)),
            ),
        ];
        // A configure step reads nothing back.
        if *width > 0 {
            // The whole read, past 32 bits for a read of 5 to 8 bytes.
            fields.push(at(
                "read",
                record_words(*width),
                *value,
                *value_read,
                claimed && observed.is_empty() && unrouted.is_empty(),
            ));
        }
        fields
    }

    /// An SPI device's evidence (`AET-WO-4200` D4), from its witness (kind
    /// 22) and its read (kind 21), as (electrical, software) events:
    /// - electrical: the multiplexer routed a byte to the device's own
    ///   address and found no device there (`SPIMultiplexer.cs` L69), and the
    ///   model logged nothing: what a device absent behind a chip select the
    ///   firmware asserted leaves;
    /// - software: the multiplexer found no chip select asserted (L56) or
    ///   several (L62), the model logged lines other than the read's, or the
    ///   record says the read completed and nothing was logged or routed.
    ///
    /// A witness that could not be read whole is no evidence.
    fn spi_evidence(&self, reference: &str) -> (Vec<usize>, Vec<usize>) {
        let mut electrical = Vec::new();
        let mut software = Vec::new();
        // Every step's witness of the device (`AET-WO-4210`), in order.
        for w in
            self.indices(|r| matches!(r, Record::SpiWitness { reference: x, .. } if x == reference))
        {
            let Record::SpiWitness {
                device,
                step,
                observed: Some(observed),
                unrouted,
                ..
            } = &self.records[w]
            else {
                continue;
            };
            let Some(Record::SpiCheck { address, .. }) = self.spi_check_of(reference, step) else {
                continue;
            };
            let check = self.records.iter().position(|r| {
                matches!(r, Record::SpiCheck { reference: x, step: s, .. } if x == reference && s == step)
            });
            // The device's own multiplexer's line, whole: its source is the
            // path before the device's name (adversarial A3: a line of
            // another source is no evidence).
            let mux = device.rsplit_once('.').map_or("", |(m, _)| m);
            let absent = format!(
                " to device {address:#X}, but it's not connected - ignoring transfer and returning a dummy byte"
            )
            .replace("0X", "0x");
            let source = format!("[WARNING] {mux}: Tried to transmit byte 0x");
            if observed.is_empty()
                && unrouted
                    .iter()
                    .any(|l| l.starts_with(&source) && l.ends_with(&absent))
            {
                electrical.push(w);
            }
            // The software conditions are the fields' (`spi_fields`): one
            // statement of each. A claim with nothing logged is the read's
            // own event as well as the witness's.
            let fields = self.spi_fields(&self.records[w]);
            if fields.iter().any(|f| f.wrong) {
                software.push(w);
                software.extend(check.filter(|_| fields.get(2).is_some_and(|f| f.wrong)));
            }
        }
        (electrical, software)
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
    /// address other than the device's, during this step (`absent` is the
    /// step's own since trace 16, `AET-WO-3305`), is evidence. A faulted bus
    /// finds none at the device's own, which is not, nor at another step's.
    /// A transfer to another device's address that answers leaves no such
    /// line, but that device logs a transaction its script does not hold:
    /// its own witness is evidence ([`Self::surplus`], `AET-WO-3306`).
    fn witness_wrong(&self, b: usize, record: &Record) -> bool {
        self.witness_fields(b, record)
            .is_some_and(|f| f.iter().any(|f| f.wrong))
    }

    /// A sound witness's register written and value read, each with its
    /// solved value, what the first transaction the model logged wrote and
    /// read (both from that one transaction, `AET-WO-3309` (4)), and whether
    /// it counts as evidence ([`Self::witness_wrong`]); `None` for a witness
    /// that is no evidence. A transaction that wrote or read another number of
    /// bytes than the step's is wrong by its count, named by a field of its
    /// own (`{device}.Write.bytes`, `.Read.bytes`), even where its bytes pack
    /// to the solved value (`AET-WO-3309` (1), D5: `0x00 0x36 0x40 0x81`).
    fn witness_fields(&self, b: usize, record: &Record) -> Option<Vec<Field>> {
        let Record::I2cWitness {
            reference,
            device,
            step,
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
            write,
            width: read_width,
            expected: value,
            mask: value_mask,
            status,
            ..
        }) = self.check_of(reference, step)
        else {
            return None;
        };
        let observed = observed.as_deref()?;
        // A witness is read in its script's form: our BMP581's, or `Write`
        // and `Read` (`AET-WO-3307`). A line in another form is not the
        // device's model's. The ICP-101xx's is placed, never judged.
        let form = lines::form(expected, device).filter(|f| *f != Form::Icp)?;
        let first = |ls: &[String]| lines::transactions(ls, device, form).0.into_iter().next();
        let want = first(expected)?;
        let reg_want = packed(&want.written)?;
        if Some(reg_want) != step_shape(write, *read_width).0 || *read_width > 4 {
            return None;
        }
        // A configure step reads nothing back; the others' read is the part's
        // value under the step's mask.
        let value_want = if *read_width == 0 {
            None
        } else {
            let want = packed(want.read.as_deref()?)?;
            if want.1 != usize::from(*read_width) || (want.0 ^ *value) & *value_mask != 0 {
                return None;
            }
            Some(want)
        };
        let claimed = status.as_deref() == Some("ok");
        let silent = claimed && observed.is_empty();
        let got = first(observed);
        let written = got
            .as_ref()
            .map(|t| t.written.as_slice())
            .filter(|w| !w.is_empty());
        let returned = got.as_ref().and_then(|t| t.read.as_deref());
        let reg = written.and_then(packed).map(|(v, _)| v);
        let read = returned.and_then(packed).map(|(v, _)| v);
        let width_mask = |n: usize| {
            if n >= 4 {
                u64::from(u32::MAX)
            } else {
                (1u64 << (8 * n)) - 1
            }
        };
        let at = |name: &str, mask: u64, solved: u64, got: Option<u64>| Field {
            register: format!("{device}.{name}"),
            address: u32::from(*address),
            mask,
            solved,
            observed: got.map(|g| g & mask),
            wrong: silent || got.is_some_and(|g| g != solved),
        };
        let elsewhere = absent.iter().copied().find(|a| a != address);
        let mut fields = vec![
            Field {
                register: format!("{device}.address"),
                address: u32::from(*address),
                mask: 0x7F,
                solved: u64::from(*address),
                observed: elsewhere.map(u64::from),
                wrong: elsewhere.is_some(),
            },
            at("Write", width_mask(write.len()), reg_want.0, reg),
        ];
        if let Some(want) = value_want {
            fields.push(at("Read", width_mask(want.1), want.0, read));
        }
        let count = |name: &str, solved: usize, got: Option<&[u8]>| {
            got.filter(|g| g.len() != solved).map(|g| Field {
                register: format!("{device}.{name}.bytes"),
                address: u32::from(*address),
                mask: u64::MAX,
                solved: solved as u64,
                observed: Some(g.len() as u64),
                wrong: true,
            })
        };
        fields.extend(count("Write", reg_want.1, written));
        if let Some(want) = value_want {
            fields.extend(count("Read", want.1, returned));
        }
        fields.extend(self.surplus_fields(b, record, *address));
        Some(fields)
    }

    /// The transactions device `reference`'s witnesses observed that its
    /// script does not hold (`AET-WO-3306`), each as the index of the
    /// witness its first line went to and its lines. A device's witnesses,
    /// in record order, hold its whole log: each step takes as many lines as
    /// it expects and the last the rest (`renode.rs` `attribute`). Their
    /// lines are judged whole: every one a transaction of the device in its
    /// script's form ([`transactions`]: `Write`/`Read`, or a BMP581's since
    /// `AET-WO-3309`), and its script's transactions, each whole, in order among the
    /// observed ones. What is left over is a transaction the firmware sent
    /// to this device's address that none of its steps is. Empty unless all
    /// of that holds: a missing or wrong transaction is left to each
    /// witness's own fields, and an unreadable witness says nothing.
    fn surplus(&self, reference: &str) -> (usize, Vec<(usize, Shape)>) {
        let Some((device, script, log, at)) = self.whole_log(reference) else {
            return (0, Vec::new());
        };
        let Some(form) = lines::form(&script, device) else {
            return (0, Vec::new());
        };
        let (Some(want), Some(got)) = (
            transactions(&script, device, form),
            transactions(&log, device, form),
        ) else {
            return (0, Vec::new());
        };
        let text = |all: &[String], t: &Transaction| -> Vec<String> {
            t.lines.iter().map(|&i| all[i].clone()).collect()
        };
        // Each scripted transaction is the earliest observed one equal to it.
        let mut held = 0;
        let mut extra = Vec::new();
        for t in got {
            if want
                .get(held)
                .is_some_and(|w| text(&script, w) == text(&log, &t))
            {
                held += 1;
            } else {
                extra.push((at[t.lines[0]], shape(&t)));
            }
        }
        if held < want.len() {
            return (0, Vec::new());
        }
        (held, extra)
    }

    /// Device `reference`'s model path, its witnesses' expected lines and
    /// observed lines, each in record order (its whole script and log), and
    /// the index of the witness each observed line went to; `None` with no
    /// witness, or one unreadable.
    fn whole_log(&self, reference: &str) -> Option<WholeLog<'a>> {
        let (mut device, mut script, mut log, mut at) = (None, Vec::new(), Vec::new(), Vec::new());
        for (i, r) in self.records.iter().enumerate() {
            if let Record::I2cWitness {
                reference: x,
                device: d,
                expected,
                observed,
                ..
            } = r
                && x == reference
            {
                let observed = observed.as_ref()?;
                device = Some(d.as_str());
                script.extend(expected.iter().cloned());
                log.extend(observed.iter().cloned());
                at.extend(observed.iter().map(|_| i));
            }
        }
        Some((device?, script, log, at))
    }

    /// The fields that witness `record` of a device at `address`, on bus
    /// `b`, shows for the transactions its device's script does not hold
    /// that start in it ([`Self::surplus`]): the device's transaction count
    /// against its script's, at the address the firmware sent them to; and
    /// for each, when exactly one other device on the bus has a step of its
    /// shape (its write, and its read's width) that went elsewhere
    /// ([`Self::sent_elsewhere`]), that step's address field, as the firmware
    /// sent it. Two such steps, none, or one that cannot be told, and only
    /// the address is named.
    fn surplus_fields(&self, b: usize, record: &Record, address: u8) -> Vec<Field> {
        let Record::I2cWitness {
            reference, device, ..
        } = record
        else {
            return Vec::new();
        };
        let (scripted, extra) = self.surplus(reference);
        let here: Vec<&Shape> = extra
            .iter()
            .filter(|(at, _)| std::ptr::eq(&self.records[*at], record))
            .map(|(_, t)| t)
            .collect();
        if here.is_empty() {
            return Vec::new();
        }
        let mut fields = vec![Field {
            register: format!("{device}.transactions"),
            address: u32::from(address),
            mask: u64::MAX,
            solved: scripted as u64,
            observed: Some((scripted + extra.len()) as u64),
            wrong: true,
        }];
        for &extra in here {
            // Each step of this shape on the bus: `Some` if it went elsewhere,
            // `None` if that cannot be told. This device's own never went
            // elsewhere: its log holds every scripted transaction and this one.
            let owners: Vec<Option<(&str, &str, u8)>> = self
                .records
                .iter()
                .filter_map(|r| match r {
                    Record::I2cCheck {
                        reference: x,
                        address: a,
                        step,
                        write: w,
                        width: n,
                        ..
                    } if self.buses[b].devices.iter().any(|(d, _)| d == x)
                        && step_shape(w, *n) == extra =>
                    {
                        match self.sent_elsewhere(r) {
                            Some(true) => Some(Some((x.as_str(), step.as_str(), *a))),
                            Some(false) => None,
                            None => Some(None),
                        }
                    }
                    _ => None,
                })
                .collect();
            if let [Some((owner, step, solved))] = owners[..] {
                let owner_device = self.records.iter().find_map(|r| match r {
                    Record::I2cWitness {
                        reference: x,
                        device: d,
                        ..
                    } if x == owner => Some(d.as_str()),
                    _ => None,
                });
                fields.push(Field {
                    register: format!("{}.{step}.address", owner_device.unwrap_or(owner)),
                    address: u32::from(solved),
                    mask: 0x7F,
                    solved: u64::from(solved),
                    observed: Some(u64::from(address)),
                    wrong: true,
                });
            }
        }
        fields
    }

    /// Whether check `check`'s transfer went to another device's address
    /// that answered (`AET-WO-3306` adversarial F1): its record claims `ok`,
    /// so something answered; its step found no device at an address not its
    /// own; and its device's whole log holds fewer transactions of its shape
    /// (its write, and its read's width) than its script does. `None` when
    /// that cannot be told: no witness of the step, or one unreadable, or a
    /// line that is not a transaction of its form ([`transactions`]).
    fn sent_elsewhere(&self, check: &Record) -> Option<bool> {
        let Record::I2cCheck {
            reference,
            address,
            step,
            write,
            width,
            status,
            ..
        } = check
        else {
            return Some(false);
        };
        let absent = self.records.iter().find_map(|r| match r {
            Record::I2cWitness {
                reference: x,
                step: s,
                absent,
                ..
            } if x == reference && s == step => Some(absent),
            _ => None,
        })?;
        if status.as_deref() != Some("ok") || absent.iter().any(|a| a != address) {
            return Some(false);
        }
        let (device, script, log, _) = self.whole_log(reference)?;
        let form = lines::form(&script, device)?;
        let own = step_shape(write, *width);
        let count = |lines: &[String]| -> Option<usize> {
            Some(
                transactions(lines, device, form)?
                    .iter()
                    .filter(|t| shape(t) == own)
                    .count(),
            )
        };
        Some(count(&log)? < count(&script)?)
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

    /// The bits of a register reading that are wrong and that no bus or
    /// motor output owns.
    fn unowned(&self, name: &str, r: &Reading) -> u32 {
        let owned = self
            .buses
            .iter()
            .flat_map(|b| b.joined.iter().flat_map(|j| &j.owned))
            .chain(
                self.motors
                    .iter()
                    .flat_map(|m| m.joined.iter().flat_map(|j| &j.owned)),
            )
            .chain(
                self.serial
                    .iter()
                    .flat_map(|s| s.joined.iter().flat_map(|j| &j.owned)),
            )
            .filter(|o| o.name == name && o.address == r.address)
            .fold(0, |m, o| m | o.mask)
            | self.timer_owned(name, r)
            | self.usart_owned(name, r);
        mismatch(r) & !owned
    }

    /// The bits of a USART register reading that some serial port owns.
    fn usart_owned(&self, name: &str, r: &Reading) -> u32 {
        self.serial
            .iter()
            .fold(0, |bits, s| bits | usart_bits(s, name, r))
    }

    /// Whether serial port `s` owns a wrong field of `reading`: its pins' or
    /// gates' (as a bus owns its own), or a wrong bit of its USART's block.
    /// Another port's fields of a shared register are never this port's.
    fn serial_owned_wrong(&self, s: usize, name: &str, reading: &Reading) -> bool {
        let port = &self.serial[s];
        port.joined
            .iter()
            .flat_map(|j| &j.owned)
            .any(|o| wrong(o, name, reading))
            || mismatch(reading) & usart_bits(port, name, reading) != 0
    }

    /// Software evidence for serial port `s`: each register it owns that is
    /// wrong; once the boot word is set, its framing check whose write
    /// judgement failed (read and not held: an unread one shows no write),
    /// and its witness whose TX pin was rebuilt unrouted at a boot's first
    /// `TDR` write (adversarial pass 2, M1).
    fn serial_software(&self, s: usize) -> Vec<usize> {
        let label = self.serial[s].label;
        let tx_af = match self.serial_port(label) {
            Some(Record::SerialPortSolved { tx_af, .. }) => Some(*tx_af),
            _ => None,
        };
        self.indices(|r| match r {
            Record::RegisterObserved { name, reading } => self.serial_owned_wrong(s, name, reading),
            Record::SerialFramingCheck {
                label: l,
                tags: Some(t),
                ..
            } => self.booted && l == label && !t.holds(),
            Record::SerialWitness {
                label: l,
                tx_at_send: Some(m),
                ..
            } => {
                self.booted && l == label && tx_af.is_some_and(|af| !campaign::routed(Some(*m), af))
            }
            _ => false,
        })
    }

    /// The solved port (kind 27) labelled `label`.
    fn serial_port(&self, label: &str) -> Option<&'a Record> {
        self.records
            .iter()
            .find(|r| matches!(r, Record::SerialPortSolved { label: l, .. } if l == label))
    }

    /// The finding kinds kind 29 `witness` gives, as replay derives them:
    /// none before the boot word is set.
    fn serial_witness_findings(&self, witness: &Record) -> Vec<&'static str> {
        match witness {
            Record::SerialWitness { label, .. } if self.booted => self
                .serial_port(label)
                .map(|p| campaign::serial_witness_findings(p, witness, self.releases))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Whether kind 28 `check`'s tags failed, or went unread, in a run whose
    /// boot word is set.
    fn serial_tags_failed(&self, check: &Record) -> bool {
        matches!(check, Record::SerialFramingCheck { tags, .. }
            if self.booted && tags.as_ref().is_none_or(|t| !t.holds()))
    }

    /// Serial port `s`'s failing records: its framing check when it does
    /// not hold or its tags failed, and its witness when it gives a finding.
    /// A passing one is never evidence (`4300a` M).
    fn serial_failing(&self, s: usize) -> Vec<usize> {
        let label = self.serial[s].label;
        self.indices(|r| match r {
            Record::SerialFramingCheck {
                label: l, holds, ..
            } => l == label && (!holds || self.serial_tags_failed(r)),
            Record::SerialWitness { label: l, .. } => {
                l == label && !self.serial_witness_findings(r).is_empty()
            }
            _ => false,
        })
    }

    /// `E3`'s evidence for serial port `s` (`AET-WO-2302` D5): its framing
    /// check and its witness, when the scenario (kind 5) holds its RX line
    /// open, the boot word is set, the check holds with its writes read and
    /// held, and the witness shows nothing gone in, no mux read at an
    /// injection, its pattern out and its echo missing and nothing else.
    /// `None` otherwise. The caller requires no software evidence on it.
    fn serial_open(&self, s: usize) -> Option<Vec<usize>> {
        let label = self.serial[s].label;
        let open = matches!(self.records.first(), Some(Record::Scenario { overrides, .. })
            if overrides.iter().any(|o| matches!(o, Override::SerialRxOpen { port } if port == label)));
        // belt-and-suspenders: `self.booted` (mutant E02 survives) - a serial
        // finding is named only once the boot word is set (the judge, and
        // `check_run`'s binding), so without it the witness below never shows
        // `serial_echo_missing` alone.
        if !(open && self.booted) {
            return None;
        }
        // belt-and-suspenders: `holds: true` (E03) - a failing framing check is
        // decided first as `Scope::SerialStatic`, rule I6; `t.holds()` (E04) -
        // a wrong write the port owns is decided first as software evidence
        // (`serial_software`), rule S5; `tags: Some(_)` (E05) - unread tags
        // leave `tx_at_send` unset at replay, so the pattern finding joins the
        // echo's and the witness below fails. Each is stated here so E3 never
        // rests on a rule ordering elsewhere.
        let check = self.records.iter().position(|r| {
            matches!(r, Record::SerialFramingCheck { label: l, holds: true, tags: Some(t), .. }
                if l == label && t.holds())
        })?;
        // belt-and-suspenders: `injected_ns: None` (E06) and
        // `rx_at_injection: None` (E07) - replay already refuses a frame or a
        // mux read into a port the scenario holds open
        // (`aethelgard_chaos::campaign`, `check_serial_joins`), and `attribute`
        // takes only a replayed run.
        let witness = self.records.iter().position(|r| {
            matches!(r, Record::SerialWitness { label: l, injected_ns: None, rx_at_injection: None, .. }
                if l == label
                    && self.serial_witness_findings(r) == [campaign::SERIAL_ECHO_FINDING])
        })?;
        Some(vec![check, witness])
    }

    /// The position among the kind 27s of the port labelled `label`.
    fn serial_named(&self, label: &str) -> Option<usize> {
        self.serial.iter().position(|s| s.label == label)
    }

    /// The bits of a timer register reading that some motor output owns.
    fn timer_owned(&self, name: &str, r: &Reading) -> u32 {
        self.motors
            .iter()
            .fold(0, |bits, m| bits | timer_bits(m, name, r))
    }

    /// Whether motor output `m` owns a wrong field of `reading`: its pin's or
    /// gates' (as a bus owns its own), or a wrong bit of its channel's fields
    /// of its timer (adversarial pass: another channel's `CCRk` made a dead
    /// output software).
    fn motor_owned_wrong(&self, m: usize, name: &str, reading: &Reading) -> bool {
        let motor = &self.motors[m];
        motor
            .joined
            .iter()
            .flat_map(|j| &j.owned)
            .any(|o| wrong(o, name, reading))
            // belt-and-suspenders: `!self.judge_bits(name)` - a reading
            // wrong in the judge's bits alone is unexplained by name
            // (`judge_only`) before any rule reads it, and its kind,
            // `register_mismatch`, is one finding per event, so the run is
            // `I1` whatever a rule would cite.
            || mismatch(reading) & timer_bits(motor, name, reading) & !self.judge_bits(name) != 0
    }

    /// The bits of kind 11 `name` whose expected value is the judge's own
    /// act, never the firmware's (ADV-M3): `CEN` of a timer's `CR1` once
    /// its stop (kind 30) cleared it, which kind 11 then expects cleared.
    /// 0 for any other register.
    fn judge_bits(&self, name: &str) -> u32 {
        let stopped = name.strip_suffix(".CR1").is_some_and(|timer| {
            (0..self.records.len()).any(|s| {
                matches!(&self.records[s], Record::JudgeStop { timer: t, .. } if t == timer)
                    && self.cleared(s)
            })
        });
        u32::from(stopped)
    }

    /// Whether `timer`'s `CEN` was set again after the judge's stop cleared
    /// it (kind 30): kind 11's end read of its `CR1` has `CEN` set (ADV-M3).
    /// No record says when that `CEN` rise fell, so a re-enable after the
    /// window, harmless on silicon where `CEN` was still set, cannot be told
    /// from a restart.
    fn cen_set_again(&self, timer: &str) -> bool {
        let cr1 = format!("{timer}.CR1");
        self.judge_bits(&cr1) != 0
            && self.records.iter().any(|r| {
                matches!(r, Record::RegisterObserved { name, reading }
                    if *name == cr1 && reading.observed.is_some_and(|o| o & 1 != 0))
            })
    }

    /// Kind 11 `i` wrong in the judge's own bits alone (`CR1`'s `CEN` set
    /// again after the stop): what the records lack, named; `None` for any
    /// other event.
    fn judge_only(&self, i: usize) -> Option<String> {
        let Record::RegisterObserved { name, reading } = &self.records[i] else {
            return None;
        };
        let wrong = mismatch(reading);
        (wrong != 0 && wrong & !self.judge_bits(name) == 0)
            .then(|| format!("register_mismatch: {name}: {CEN_AGAIN}"))
    }

    /// Software evidence for motor output `m`: each register it owns that is
    /// wrong.
    fn motor_software(&self, m: usize) -> Vec<usize> {
        self.indices(|r| match r {
            Record::RegisterObserved { name, reading } => self.motor_owned_wrong(m, name, reading),
            _ => false,
        })
    }

    /// Motor output `m`'s failing records: its static check when it does not
    /// hold, and its witness when it failed. A passing one is never evidence
    /// (adversarial pass, and F6 of the second review).
    fn motor_failing(&self, m: usize) -> Vec<usize> {
        let label = self.motors[m].label;
        self.indices(|r| match r {
            Record::MotorTimingCheck {
                label: l, holds, ..
            } => l == label && !holds,
            Record::MotorWitness {
                label: l, status, ..
            } => self.booted && l == label && !PASSING_WITNESS.contains(&status.as_str()),
            _ => false,
        })
    }

    /// The position among the kind 24s of the output labelled `label`.
    fn motor_named(&self, label: &str) -> Option<usize> {
        self.motors.iter().position(|m| m.label == label)
    }

    /// Output `m`'s witness (kind 26): at most one, by the run rules.
    fn motor_witness(&self, m: usize) -> Option<usize> {
        let label = self.motors[m].label;
        self.records
            .iter()
            .position(|r| matches!(r, Record::MotorWitness { label: l, .. } if l == label))
    }

    /// Kind 26 `w`'s anchor, its timer's first `CEN` rise.
    fn anchor(&self, w: usize) -> Option<u64> {
        match &self.records[w] {
            Record::MotorWitness { anchor_ns, .. } => *anchor_ns,
            _ => None,
        }
    }

    /// One output, and its witness, for each timer: the first in kind 24's
    /// order with a witness, and with `dshot`, the first DShot one whose
    /// witness has an anchor, as replay takes a DShot timer's. One timer's
    /// outputs carry one anchor and one count of `CEN` rises (replay's
    /// `check_motor_joins`), so that witness stands for the timer.
    fn timers(&self, dshot: bool) -> Vec<(usize, usize)> {
        let mut seen = BTreeSet::new();
        (0..self.motors.len())
            .filter(|&m| !dshot || self.motors[m].dshot)
            .filter_map(|m| Some((m, self.motor_witness(m)?)))
            .filter(|&(_, w)| !dshot || self.anchor(w).is_some())
            .filter(|&(m, _)| seen.insert(self.motors[m].timer.0))
            .collect()
    }

    /// The judge's stop (kind 30) of `timer` for its window from `anchor`:
    /// at most one, by the run rules.
    fn stop(&self, timer: &str, anchor: u64) -> Option<usize> {
        self.records.iter().position(|r| {
            matches!(r, Record::JudgeStop { timer: t, anchor_ns, .. } if t == timer && *anchor_ns == anchor)
        })
    }

    /// Whether kind 30 `stop` left its timer's `CEN` cleared: the stop ran.
    fn cleared(&self, stop: usize) -> bool {
        matches!(&self.records[stop], Record::JudgeStop { cr1_after: Some(c), .. } if c & 1 == 0)
    }

    /// The first port the judge's chain injects (kind 29, the first not held
    /// open), when its frame went in: its `injected_ns` places the boot
    /// word, one quantum before it (`campaign::boot_word_instant`).
    fn first_injection(&self) -> Option<usize> {
        campaign::first_injection(self.records).map(|(i, _)| i)
    }

    /// The ports' solved records (kind 27) the judge sums the wire time
    /// over: every port not held open.
    fn wire_ports(&self) -> Vec<usize> {
        let open = campaign::open_ports(self.records);
        self.indices(|r| matches!(r, Record::SerialPortSolved { label, .. } if !open.contains(label.as_str())))
    }

    /// `H1`'s evidence for output `m`'s failing witness, when the run's end
    /// cut it (`AET-WO-2302b`): its DShot timer's window, from its anchor,
    /// passes `MAX_VIRTUAL_NS`, and its witness is `stopped`, continuous (a
    /// rise every period from its first to its last), with its last rise
    /// within a period and a grain of the run's end, where a rise was due
    /// every period until it. The output ran until the run ended: the
    /// twin's bound, not the firmware's. `None` otherwise.
    fn cut_by_run(&self, m: usize) -> Option<Vec<usize>> {
        let motor = &self.motors[m];
        let w = self.motor_witness(m)?;
        let Record::MotorWitness {
            status,
            anchor_ns: Some(a),
            last_ns: Some(last),
            period_ns,
            grain_ns,
            ..
        } = &self.records[w]
        else {
            return None;
        };
        let max = campaign::TWIN_MAX_VIRTUAL_NS;
        // belt-and-suspenders: `motor.dshot` (mutant M14 survives) - a PWM
        // window ends at the run's end, so its `stopped` witness's last rise
        // is more than a period before it and the last condition fails.
        // belt-and-suspenders: `witness_continuous` - replay refuses a
        // `stopped` witness that is not continuous (`check_run`'s
        // `witness_consistent`, ADV-M1) before `attribute` reads it.
        (motor.dshot
            && status == "stopped"
            && campaign::witness_continuous(&self.records[w])
            && campaign::dshot_window_past_run(*a)
            && last.saturating_add(*period_ns).saturating_add(*grain_ns) >= max)
            .then(|| vec![motor.at, w])
    }

    /// `H3`'s evidence for serial port `s`, when the run's end cut its echo
    /// (`AET-WO-2302b`): the boot word is set, its frame and echo are in the
    /// wire time that passes `MAX_VIRTUAL_NS` from the boot word's instant
    /// (`campaign::wire_past_run_ports`), its framing check holds with its
    /// writes read and held, and its witness shows its echo missing or cut
    /// short (never `not_counted`) and nothing else, its RX pin on its AF at
    /// the injection when one was read (kind 29's `rx_at_injection`). The evidence is the
    /// first injection (the instant), the port's witness, and the summed
    /// ports' framings. `None` otherwise.
    fn wire_cut(&self, s: usize) -> Option<Vec<usize>> {
        let label = self.serial[s].label;
        if !(self.booted && campaign::wire_past_run_ports(self.records)?.contains(&label)) {
            return None;
        }
        self.records.iter().position(|r| {
            matches!(r, Record::SerialFramingCheck { label: l, holds: true, tags: Some(t), .. }
                if l == label && t.holds())
        })?;
        // The echo `missing` or `partial`, with the RX pin on its AF at the
        // injection when one was read: one `not_counted` came back with the
        // RX pin off its AF there, and with it off, no echo is the pin's,
        // not the bound's, whatever came back (ADV-S1).
        let rx_af = match self.serial_port(label)? {
            Record::SerialPortSolved { rx_af, .. } => *rx_af,
            _ => return None,
        };
        let witness = self.records.iter().position(|r| {
            matches!(r, Record::SerialWitness { label: l, echo, rx_at_injection, .. }
                if l == label
                    && matches!(echo.as_str(), "missing" | "partial")
                    && (rx_at_injection.is_none() || campaign::routed(*rx_at_injection, rx_af))
                    && self.serial_witness_findings(r) == [campaign::SERIAL_ECHO_FINDING])
        })?;
        let mut evidence = vec![self.first_injection()?, witness];
        evidence.extend(self.wire_ports());
        Some(evidence)
    }

    /// What one of the twin's window findings comes to (`AET-WO-2302b`):
    /// the events that show it, each decided by its own rule, the six
    /// software ones (`S6` to `S11`) by the firmware's evidence and the
    /// four harness ones (`H1` to `H4`) refused by name, with how many
    /// findings the records show; and each case the records cannot decide,
    /// named with what they lack (`INV-06`). `None` for any other kind.
    /// Replay binds every count but `motor_anchor_out_of_order`'s (a range)
    /// and `serial_injection_out_of_order`'s (none), whose arms decide it.
    fn windowed(&self, kind: &str, count: u64) -> Option<Windowed> {
        let mut w = Windowed::default();
        let witnesses = |pred: &dyn Fn(&Record) -> bool| {
            self.indices(|r| matches!(r, Record::MotorWitness { .. }) && pred(r))
        };
        let each = |w: &mut Windowed, rule: Rule, symptom: usize, evidence: Vec<usize>| {
            w.symptoms.push(symptom);
            w.decided.push((rule, evidence));
            w.shown += 1;
        };
        // One symptom the records cannot decide, named with what they lack.
        let unexplained = |w: &mut Windowed, symptom: usize, missing: String| {
            w.symptoms.push(symptom);
            w.missing.push(missing);
            w.shown += 1;
        };
        match kind {
            campaign::MOTOR_ANCHOR_UNOBSERVED_FINDING => {
                for i in witnesses(
                    &|r| matches!(r, Record::MotorWitness { status, .. } if status == "unanchored"),
                ) {
                    each(&mut w, Rule::S6, i, vec![i]);
                }
            }
            campaign::MOTOR_ANCHOR_REPEATED_FINDING => {
                for (m, i) in self.timers(false) {
                    if !matches!(self.records[i], Record::MotorWitness { cen_rises, .. } if cen_rises > 1)
                    {
                        continue;
                    }
                    let timer = self.motors[m].timer.0;
                    if self.cen_set_again(timer) {
                        unexplained(&mut w, i, format!("{kind}: {timer}: {CEN_AGAIN}"));
                    } else {
                        each(&mut w, Rule::S7, i, vec![i]);
                    }
                }
            }
            campaign::MOTOR_ANCHOR_ORDER_FINDING => {
                // Only an anchor after the boot word's instant is in the
                // records (kind 29's first injection places it, kind 26 the
                // anchor); the kind's other cases, an anchor before the last
                // device step's witness or no boot word seen by its hook,
                // are the boot's log lines, which no record carries, so only
                // the verdict would say them, and a verdict is never
                // evidence (`AET-WO-2302b` adversarial pass).
                // belt-and-suspenders: kind 10's boot word set - replay
                // refuses the kind in a boot without it, so a boot that
                // lost its word (a droop's reset among its causes) never
                // reaches here.
                let boot = self.indices(|r| {
                    matches!(r, Record::BootObserved { boot_word, .. }
                        if boot_word.observed == Some(boot_word.expected))
                });
                let instant = campaign::boot_word_instant(self.records);
                let after: Vec<usize> = self
                    .timers(false)
                    .into_iter()
                    .filter(|&(_, i)| self.anchor(i).zip(instant).is_some_and(|(a, b)| a > b))
                    .map(|(_, i)| i)
                    .collect();
                if after.len() as u64 == count {
                    for i in after {
                        let evidence = [boot.clone(), vec![i]]
                            .into_iter()
                            .flatten()
                            .chain(self.first_injection())
                            .collect();
                        each(&mut w, Rule::S8, i, evidence);
                    }
                } else {
                    w.missing.push(format!(
                        "{kind}: named {count} time(s), {} timer(s) anchored after the boot word's instant (kind 29's first injection): the boot's order otherwise (an anchor before the last device step's witness, or no boot word seen by its hook) is in no record",
                        after.len()
                    ));
                    w.shown = count;
                }
            }
            campaign::MOTOR_WINDOW_PAST_RUN_FINDING => {
                for (m, i) in self.timers(true) {
                    if self.anchor(i).is_some_and(campaign::dshot_window_past_run) {
                        each(&mut w, Rule::H1, i, vec![self.motors[m].at, i]);
                    }
                }
            }
            campaign::MOTOR_EDGE_BEFORE_FINDING => {
                for i in witnesses(
                    &|r| matches!(r, Record::MotorWitness { edges_before, .. } if *edges_before > 0),
                ) {
                    each(&mut w, Rule::S9, i, vec![i]);
                }
            }
            campaign::MOTOR_EDGE_AFTER_FINDING => {
                for i in witnesses(
                    &|r| matches!(r, Record::MotorWitness { edges_after, .. } if *edges_after > 0),
                ) {
                    let Record::MotorWitness { label, .. } = &self.records[i] else {
                        continue;
                    };
                    let timer = self.motor_named(label).map(|m| self.motors[m].timer.0);
                    let stop = timer
                        .zip(self.anchor(i))
                        .and_then(|(t, a)| self.stop(t, a))
                        .filter(|&s| self.cleared(s));
                    match stop {
                        Some(_) if timer.is_some_and(|t| self.cen_set_again(t)) => {
                            unexplained(&mut w, i, format!("{kind}: {label}: {CEN_AGAIN}"));
                        }
                        Some(s) => each(&mut w, Rule::S10, i, vec![i, s]),
                        None => unexplained(
                            &mut w,
                            i,
                            format!(
                                "{kind}: {label}: no judge's stop of its timer at its anchor left CEN cleared (kind 30), so its edges after the window may be the timer still running"
                            ),
                        ),
                    }
                }
            }
            campaign::JUDGE_STOP_MISSING_FINDING => {
                for (m, i) in self.timers(true) {
                    let timer = self.motors[m].timer.0;
                    let Some(a) = self
                        .anchor(i)
                        .filter(|&a| !campaign::dshot_window_past_run(a))
                    else {
                        continue;
                    };
                    let stop = self.stop(timer, a);
                    if !stop.is_some_and(|s| self.cleared(s)) {
                        let evidence = [vec![self.motors[m].at, i], stop.into_iter().collect()];
                        each(&mut w, Rule::H2, i, evidence.concat());
                    }
                }
            }
            campaign::JUDGE_STOP_MISMATCH_FINDING => {
                for s in self.indices(|r| matches!(r, Record::JudgeStop { .. })) {
                    let Record::JudgeStop {
                        timer, read_back, ..
                    } = &self.records[s]
                    else {
                        continue;
                    };
                    if !self.cleared(s) {
                        continue;
                    }
                    let off = read_back
                        .iter()
                        .filter(|g| g.observed.is_some_and(|o| o & g.mask != g.expected))
                        .count() as u64;
                    let short = read_back.iter().any(|g| g.observed.is_none());
                    if off > 0 {
                        w.decided.push((Rule::S11, vec![s]));
                    }
                    if short {
                        w.missing.push(format!(
                            "{kind}: {timer}: its stop read back fewer registers than the timer's (kind 30's read_back), and an unread register shows no write of the firmware's"
                        ));
                    }
                    if off > 0 || short {
                        w.symptoms.push(s);
                        w.shown += off + u64::from(short);
                    }
                }
            }
            campaign::SERIAL_WIRE_PAST_RUN_FINDING => {
                match campaign::wire_past_run_ports(self.records) {
                    None => {
                        w.missing.push(format!(
                            "{kind}: the first port took no frame, so the boot word's instant the wire time runs from is in no record (kind 29's injected_ns)"
                        ));
                        w.shown = count;
                    }
                    Some(past) if !past.is_empty() => {
                        if let Some(i) = self.first_injection() {
                            each(&mut w, Rule::H3, i, [vec![i], self.wire_ports()].concat());
                        }
                    }
                    Some(_) => {}
                }
            }
            campaign::SERIAL_INJECTION_ORDER_FINDING => {
                // Each port after the chain's first that took a frame: the
                // logged `TDR` bytes that say which went in early are in no
                // record, and either way the act is the judge's.
                let injected = self.indices(|r| {
                    matches!(
                        r,
                        Record::SerialWitness {
                            injected_ns: Some(_),
                            ..
                        }
                    )
                });
                let later = injected.get(1..).unwrap_or_default();
                if count <= later.len() as u64 {
                    w.symptoms.extend(later);
                    w.decided.push((Rule::H4, injected.clone()));
                } else {
                    w.missing.push(format!(
                        "{kind}: named {count} time(s), but {} port(s) after the chain's first took a frame (kind 29's injected_ns)",
                        later.len()
                    ));
                }
                w.shown = count;
            }
            _ => return None,
        }
        Some(w)
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
                    || brown_out_failed(r)
                    || flash_access_failed(r)
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
            Record::SpiCheck { reference, .. } | Record::SpiWitness { reference, .. } => {
                Scope::Spi(reference.as_str())
            }
            Record::MotorTimingCheck { label, .. } => Scope::MotorStatic(self.motor_named(label)?),
            Record::MotorWitness { label, .. } => Scope::Motor(self.motor_named(label)?),
            Record::SerialFramingCheck {
                label,
                holds: false,
                ..
            } => Scope::SerialStatic(self.serial_named(label)?),
            Record::SerialFramingCheck { label, .. } | Record::SerialWitness { label, .. } => {
                Scope::Serial(self.serial_named(label)?)
            }
            Record::RegisterObserved { name, reading } => {
                let on: Vec<usize> = (0..self.buses.len())
                    .filter(|&b| self.owned_wrong(b, name, reading))
                    .collect();
                // A timer register is every output of the timer's: the first
                // of them stands for it (one symptom, one decision).
                let motors: Vec<usize> = (0..self.motors.len())
                    .filter(|&m| self.motor_owned_wrong(m, name, reading))
                    .collect();
                // A register wrong in serial ports' fields alone is theirs
                // (`AET-WO-4300b`): the first of them stands for it (one
                // symptom, one decision), as a timer's register does for its
                // outputs; each port's own symptoms cite it too.
                let ports: Vec<usize> = (0..self.serial.len())
                    .filter(|&s| self.serial_owned_wrong(s, name, reading))
                    .collect();
                let unowned = self.unowned(name, reading);
                match (on.as_slice(), motors.as_slice(), ports.as_slice()) {
                    ([b], [], []) if unowned == 0 => Scope::Bus(*b, None),
                    ([], [m, ..], [])
                        if unowned == 0
                            && motors
                                .iter()
                                .all(|&o| self.motors[o].timer.0 == self.motors[*m].timer.0) =>
                    {
                        Scope::Motor(*m)
                    }
                    ([], [], [s, ..]) if unowned == 0 => Scope::Serial(*s),
                    _ => Scope::Board,
                }
            }
            _ => Scope::Board,
        })
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
            Scope::Spi(reference) => {
                let (electrical, software) = self.spi_evidence(reference);
                if !electrical.is_empty() {
                    (Rule::E2, electrical, software)
                } else if !software.is_empty() {
                    (Rule::S3, software, vec![])
                } else {
                    (Rule::I4, vec![], vec![])
                }
            }
            // A failing static check is the topology's: indeterminate, never
            // software, whatever else of the output's is wrong (F1).
            Scope::MotorStatic(m) => (Rule::I4, self.motor_failing(m), vec![]),
            Scope::Motor(m) => {
                let software = self.motor_software(m);
                if let Some(evidence) = self.cut_by_run(m).filter(|_| software.is_empty()) {
                    (Rule::H1, evidence, vec![])
                } else if software.is_empty() {
                    (Rule::I4, self.motor_failing(m), vec![])
                } else {
                    let witness = self
                        .motor_failing(m)
                        .into_iter()
                        .filter(|&i| matches!(self.records[i], Record::MotorWitness { .. }));
                    (
                        Rule::S4,
                        software.into_iter().chain(witness).collect(),
                        vec![],
                    )
                }
            }
            // A failing static framing check is the topology's (`4300a` F1).
            Scope::SerialStatic(s) => (Rule::I6, self.serial_failing(s), vec![]),
            Scope::Serial(s) => {
                let software = self.serial_software(s);
                if let Some(evidence) = self.serial_open(s).filter(|_| software.is_empty()) {
                    (Rule::E3, evidence, vec![])
                } else if let Some(evidence) = self.wire_cut(s).filter(|_| software.is_empty()) {
                    (Rule::H3, evidence, vec![])
                } else if software.is_empty() {
                    (Rule::I6, self.serial_failing(s), vec![])
                } else {
                    let witness: Vec<usize> = self
                        .serial_failing(s)
                        .into_iter()
                        .filter(|&i| {
                            matches!(self.records[i], Record::SerialWitness { .. })
                                && !software.contains(&i)
                        })
                        .collect();
                    (Rule::S5, [software, witness].concat(), vec![])
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

/// What one of the twin's window findings comes to ([`Run::windowed`]).
#[derive(Default)]
struct Windowed {
    /// The events that show it.
    symptoms: Vec<usize>,
    /// Each decision, by its rule, with its evidence.
    decided: Vec<(Rule, Vec<usize>)>,
    /// Each case the records cannot decide, as `{kind}: {what they lack}`.
    missing: Vec<String>,
    /// How many of the kind's findings the records account for.
    shown: u64,
}

/// What the records lack when a timer's `CEN` was set again after the
/// judge's stop cleared it (ADV-M3, [`Run::cen_set_again`]).
const CEN_AGAIN: &str = "CEN set again after the judge's stop cleared it (kind 30) and read set at the end (kind 11's CR1): when each CEN rise fell is in no record, so a re-enable after the window (harmless on silicon, where CEN was still set) cannot be told from a restart";

/// The witness statuses that pass (`AET-WO-4300a` D4).
const PASSING_WITNESS: [&str; 2] = ["witnessed", "static_only"];

/// The bits of `reading` (of register `name`) that motor output `m` owns in
/// its timer's block (named `TIMn.` and inside it): its own `CCRk` whole,
/// its byte of `CCMR1` or `CCMR2`, its nibble of `CCER`, and every other
/// register of the timer whole, which all the timer's outputs share; 0
/// outside the block and for another channel's fields.
fn timer_bits(m: &Motor<'_>, name: &str, reading: &Reading) -> u32 {
    let (timer, start, end) = m.timer;
    let Some(register) = name
        .strip_prefix(timer)
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|_| (start..end).contains(&reading.address))
    else {
        return 0;
    };
    // Channels 1 to 4 (kind 24's field rule).
    let k = u32::from(m.channel).clamp(1, 4) - 1;
    let ccmr = if k < 2 { "CCMR1" } else { "CCMR2" };
    let bits = match register {
        "CCMR1" | "CCMR2" if register == ccmr => 0xFF << (8 * (k % 2)),
        "CCER" => 0xF << (4 * k),
        r if r == format!("CCR{}", k + 1) => u32::MAX,
        r if r.starts_with("CCMR") || r.starts_with("CCR") => 0,
        _ => u32::MAX,
    };
    bits & reading.mask
}

/// Kind 10 with its `pc` outside every executable segment it archives: the
/// core ran where the firmware put no code (`AET-WO-2205` §1.5, `3100` F13).
fn pc_outside(r: &Record) -> bool {
    matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
        if !aethelgard_chaos::record::inside(executable, *pc))
}

/// Kind 10 of a booted firmware whose brown-out word is not the declared
/// level, or was never recorded (`AET-WO-4230` D10): the judge names it
/// only once the boot word is set, so the event does too.
fn brown_out_failed(r: &Record) -> bool {
    matches!(r, Record::BootObserved { boot_word, brown_out, .. }
        if boot_word.observed == Some(boot_word.expected)
            && campaign::brown_out_fails(brown_out))
}

/// Kind 10 of a booted firmware whose flash controller's accesses were not
/// the one read, or could not be read whole (`AET-WO-4230` D10).
fn flash_access_failed(r: &Record) -> bool {
    matches!(r, Record::BootObserved { boot_word, flash_access, .. }
        if boot_word.observed == Some(boot_word.expected)
            && flash_access.as_deref() != Some(&[]))
}

/// Every bit of a read record's value words: one word for a read of up to
/// 4 bytes, two for 5 to 8.
fn record_words(width: u8) -> u64 {
    if width > 4 {
        u64::MAX
    } else {
        u64::from(u32::MAX)
    }
}

/// The run, judged by exactly the rules `replay` applies after its reader
/// ([`campaign::check_archived`]), with every solved bus joined to the ST
/// header; and whether its verdict passed.
fn joined(run: &Replayed) -> Result<(Run<'_>, bool), AttributeError> {
    let records = run.state.records.len();
    if records > MAX_ATTRIBUTE_RECORDS {
        return Err(AttributeError::TooManyRecords {
            records,
            limit: MAX_ATTRIBUTE_RECORDS,
        });
    }
    let summary = campaign::check_archived(&run.header, &run.state)
        .map_err(AttributeError::NotACampaignRun)?;
    let records = run.state.records.as_slice();
    let f = cmsis::register_facts()?;
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
    let tim_size = f.layout("TIM")?.size;
    let mut motors = Vec::new();
    for (at, r) in records.iter().enumerate() {
        if let Record::MotorOutputSolved {
            label,
            timer,
            channel,
            pin,
            alternate_function,
            protocol,
            ..
        } = r
        {
            let line = BusLine {
                signal: String::new(),
                net: label.clone(),
                pin: pin.clone(),
                alternate_function: *alternate_function,
                ceiling_ps: 0,
            };
            let base = f.base(timer).ok();
            motors.push(Motor {
                at,
                channel: *channel,
                label,
                dshot: protocol.starts_with("dshot"),
                joined: join(&f, timer, std::slice::from_ref(&line)).ok(),
                timer: (
                    timer.as_str(),
                    base.unwrap_or(0),
                    base.map_or(0, |b| b + tim_size),
                ),
            });
        }
    }
    let usart_size = f.layout("USART")?.size;
    let mut serial = Vec::new();
    for r in records {
        if let Record::SerialPortSolved {
            label,
            peripheral,
            tx,
            tx_af,
            rx,
            rx_af,
            ..
        } = r
        {
            let line = |pin: &String, af: &u8| BusLine {
                signal: String::new(),
                net: label.clone(),
                pin: pin.clone(),
                alternate_function: *af,
                ceiling_ps: 0,
            };
            let base = f.base(peripheral).ok();
            serial.push(Serial {
                label,
                joined: join(&f, peripheral, &[line(tx, tx_af), line(rx, rx_af)]).ok(),
                usart: (
                    peripheral.as_str(),
                    base.unwrap_or(0),
                    base.map_or(0, |b| b + usart_size),
                ),
            });
        }
    }
    let booted = campaign::booted(records);
    let state = Run {
        records,
        buses,
        motors,
        serial,
        booted,
        releases: campaign::released(records),
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
                    mask: boot_word.mask.into(),
                    solved: boot_word.expected.into(),
                    observed: boot_word.observed.map(u64::from),
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
                    .chain(
                        run.motors
                            .iter()
                            .flat_map(|m| m.joined.iter().flat_map(|j| &j.owned)),
                    )
                    .chain(
                        run.serial
                            .iter()
                            .flat_map(|s| s.joined.iter().flat_map(|j| &j.owned)),
                    )
                    .filter(|o| {
                        o.name == *name
                            && o.address == reading.address
                            && reading.mask & o.mask == o.mask
                    })
                    .map(|o| Field {
                        register: o.name.clone(),
                        address: o.address,
                        mask: o.mask.into(),
                        solved: o.want.into(),
                        observed: reading.observed.map(|v| u64::from(v & o.mask)),
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
                // The motor outputs' fields of a timer register, as one,
                // without the judge's own bits (ADV-M3).
                let timer = run.timer_owned(name, reading) & !run.judge_bits(name);
                if mismatch(reading) & timer != 0 {
                    fields.push(Field {
                        register: name.clone(),
                        address: reading.address,
                        mask: timer.into(),
                        solved: (reading.expected & timer).into(),
                        observed: reading.observed.map(|v| u64::from(v & timer)),
                        wrong: true,
                    });
                }
                // A serial port's USART register, as one.
                let usart = run.usart_owned(name, reading);
                if mismatch(reading) & usart != 0 {
                    fields.push(Field {
                        register: name.clone(),
                        address: reading.address,
                        mask: usart.into(),
                        solved: (reading.expected & usart).into(),
                        observed: reading.observed.map(|v| u64::from(v & usart)),
                        wrong: true,
                    });
                }
                let unowned = run.unowned(name, reading);
                if unowned != 0 {
                    fields.push(Field {
                        register: name.clone(),
                        address: reading.address,
                        mask: unowned.into(),
                        solved: (reading.expected & unowned).into(),
                        observed: reading.observed.map(|v| u64::from(v & unowned)),
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
                    .unwrap_or_default(),
            },
            // The chip select is the board crate's: its transfer drives it.
            // The command and the record are the firmware's. A wrong select
            // names no device, so the report names the board crate's generator.
            Record::SpiWitness { reference, .. } => {
                let fields = run.spi_fields(record);
                let select = fields.first().is_some_and(|f| f.wrong);
                View::Fields {
                    device: (!select).then(|| reference.clone()),
                    fields,
                }
            }
            Record::MotorWitness { label, status, .. } => View::MotorWitness {
                label: label.clone(),
                status: status.clone(),
            },
            Record::SerialWitness { label, echo, .. } => View::SerialWitness {
                label: label.clone(),
                echo: echo.clone(),
            },
            // The judge's stop (`AET-WO-2302b`): each register it read back,
            // named as kind 11 names it at its address (replay binds the
            // two), against its solved value under its mask, as `S11`'s
            // evidence judges it.
            Record::JudgeStop { read_back, .. } => View::Fields {
                device: None,
                fields: read_back
                    .iter()
                    .map(|g| Field {
                        register: run
                            .records
                            .iter()
                            .find_map(|r| match r {
                                Record::RegisterObserved { name, reading }
                                    if reading.address == g.address =>
                                {
                                    Some(name.clone())
                                }
                                _ => None,
                            })
                            .unwrap_or_else(|| format!("{:#010x}", g.address)),
                        address: g.address,
                        mask: g.mask.into(),
                        solved: g.expected.into(),
                        observed: g.observed.map(|o| u64::from(o & g.mask)),
                        wrong: g.observed.is_some_and(|o| o & g.mask != g.expected),
                    })
                    .collect(),
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
    let shown = campaign::shown(records);
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
    // The symptoms decided where they are ([`Run::scope`]); the twin's
    // window findings are decided by kind ([`Run::windowed`]).
    let mut scoped = BTreeSet::new();
    let mut decided = Vec::new();
    let mut unexplained = BTreeSet::new();
    for (&kind, &count) in &failing {
        if let Some(w) = state.windowed(kind, count) {
            if w.shown != count {
                unexplained.insert(kind.to_owned());
                continue;
            }
            symptoms.extend(w.symptoms);
            decided.extend(w.decided.into_iter().map(|(r, e)| (r, e, Vec::new())));
            unexplained.extend(w.missing);
            continue;
        }
        // The records that show the kind are `replay`'s own
        // ([`campaign::shown`], `AET-WO-2312`): each explains one finding,
        // so their number is the count, except for a kind named once from
        // a log no record holds.
        match shown.iter().find(|s| s.kind == kind) {
            Some(s)
                if !s.records.is_empty()
                    && (s.bound == Bound::AtMostOnce || s.records.len() as u64 == count) =>
            {
                scoped.extend(s.records.iter().copied());
            }
            _ => {
                unexplained.insert(kind.to_owned());
            }
        }
    }
    // And the converse: an event that shows a failure the verdict does not
    // name is unexplained too, so rewriting the verdict alone never moves the
    // domain (`AET-WO-3100` adversarial A1).
    for s in &shown {
        for &i in &s.records {
            if !scoped.contains(&i) {
                unexplained.insert(format!("event {i}"));
            }
        }
    }
    symptoms.extend(scoped.iter().copied());

    for &i in &scoped {
        if let Some(missing) = state.judge_only(i) {
            unexplained.insert(missing);
            continue;
        }
        match state.scope(i) {
            Some(scope) => decided.push(state.decide(scope)),
            // belt-and-suspenders: R2 joins every bus and device, so `check_run`
            // has refused a symptom with no bus; this keeps it explained, not a panic.
            None => {
                unexplained.insert(format!("event {i}"));
            }
        }
    }
    // A harness rule's finding is the twin's own bound (`AET-WO-2302b`):
    // the run takes it only when nothing else decided a symptom, so it never
    // masks a fault of the device's, nor is it ever attributed to one.
    let (harness, rules): (BTreeSet<Rule>, BTreeSet<Rule>) = decided
        .iter()
        .map(|d| d.0)
        .partition(|r| r.bound().is_some());
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
        *rules
            .first()
            .or(harness.first())
            .expect("a failing run has a failing finding")
    };
    // The evidence is the named rule's: I5 names both halves', I1 none, and
    // no rule a harness one's (a record that is also the named rule's own
    // evidence stays: a cut witness's `edges_before`, S9's).
    let mut evidence = BTreeSet::new();
    let mut masked = BTreeSet::new();
    for (r, e, m) in &decided {
        if (rule == Rule::I5 && r.bound().is_none()) || *r == rule {
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
