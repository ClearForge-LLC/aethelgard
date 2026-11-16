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
    /// the electrical half.
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
    /// The symptom is a read whose record is still as the judge filled it
    /// when a short window ended, in a boot without its boot word
    /// (`identity_read_outside_window`, `AET-WO-4300a` D1 and Q7): the read
    /// did not complete inside the window, and the unset record cannot say
    /// whether the bus or the firmware was too slow. The record decides it.
    I7,
}

impl Rule {
    /// Every rule, in order.
    pub const ALL: [Rule; 16] = [
        Self::E1,
        Self::E2,
        Self::E3,
        Self::S1,
        Self::S2,
        Self::S3,
        Self::S4,
        Self::S5,
        Self::N1,
        Self::I1,
        Self::I2,
        Self::I3,
        Self::I4,
        Self::I5,
        Self::I6,
        Self::I7,
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
            Self::N1 => "N1",
            Self::I1 => "I1",
            Self::I2 => "I2",
            Self::I3 => "I3",
            Self::I4 => "I4",
            Self::I5 => "I5",
            Self::I6 => "I6",
            Self::I7 => "I7",
        }
    }

    pub fn domain(self) -> Domain {
        match self {
            Self::E1 | Self::E2 | Self::E3 => Domain::Electrical,
            Self::S1 | Self::S2 | Self::S3 | Self::S4 | Self::S5 => Domain::Software,
            Self::N1 => Domain::None,
            _ => Domain::Indeterminate,
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
            Self::N1 | Self::I1 => &[],
            Self::I2 => &[15],
            Self::I3 => &[7, 8, 9],
            Self::I4 => &[7, 25, 26],
            Self::I5 => &[7, 8, 9, 10, 11, 13, 21, 22, 26, 28, 29],
            Self::I6 => &[28, 29],
            Self::I7 => &[12, 21],
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
/// or `.Read`. The values are 64-bit, so a read of up to 8 bytes shows
/// whole (`AET-WO-4210` F-B5).
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

/// One solved motor output (kind 24, `AET-WO-4300a`) and the register
/// fields it owns: the bus join's pin and gate fields, from the header, and
/// its channel's fields of its timer's block (`timer`, `[start, end)`; see
/// [`timer_bits`]).
struct Motor<'a> {
    label: &'a str,
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
    /// A read's record left unset when a short window ended, by its index
    /// (`AET-WO-4300a` Q7): neither the bus's nor the firmware's.
    Window(usize),
    Board,
}

struct Run<'a> {
    records: &'a [Record],
    buses: Vec<Bus<'a>>,
    motors: Vec<Motor<'a>>,
    serial: Vec<Serial<'a>>,
    rcc: (u32, u32),
    /// The boot word is set: the judge read the serial witnesses and tags.
    booted: bool,
    /// The boot was read without its boot word on a short window: the
    /// judge named each record still unset (`AET-WO-4300a` D1).
    late: bool,
    /// The resets the run rendered and released.
    releases: u64,
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

/// A step's read as its check records it (`AET-WO-4210`), as the judge holds
/// it (`aethelgard_twin::boot::StepCheck::judge`): a value was observed, no
/// bit of it lies above the read's `width` bytes (so a configuration's is 0),
/// and it equals the expected one under the mask.
fn read_matches(width: u8, expected: u64, mask: u64, observed: Option<u64>) -> bool {
    // The bits a read of `width` bytes can set, as the judge holds them
    // (`aethelgard_ir::parts::width_mask`, which evolve does not depend on): none for a configuration.
    let read = match width {
        0 => 0,
        8.. => u64::MAX,
        w => (1u64 << (8 * u32::from(w))) - 1,
    };
    observed.is_some_and(|o| o & !read == 0 && o & mask == expected & mask)
}

/// The finding kinds that each select their own events, one finding each.
const PER_EVENT: [&str; 30] = [
    campaign::PAST_CEILING_FINDING,
    campaign::STUCK_HIGH_FINDING,
    campaign::HELD_LOW_FINDING,
    "i2c_timeout",
    "i2c_record_invalid",
    "i2c_identity_mismatch",
    "i2c_configure_mismatch",
    "i2c_data_mismatch",
    "i2c_transaction_unwitnessed",
    "spi_timeout",
    "spi_record_invalid",
    "spi_identity_mismatch",
    "spi_configure_mismatch",
    "spi_data_mismatch",
    "spi_transaction_unwitnessed",
    campaign::OUTSIDE_WINDOW_FINDING,
    "register_mismatch",
    campaign::BOOT_WORD_FINDING,
    "cpu_halted",
    "locked_peripheral_access",
    campaign::BROWN_OUT_FINDING,
    campaign::FLASH_ACCESS_FINDING,
    campaign::MOTOR_STATIC_FINDING,
    campaign::MOTOR_UNWITNESSED_FINDING,
    campaign::MOTOR_MISMATCH_FINDING,
    campaign::SERIAL_STATIC_FINDING,
    campaign::SERIAL_PATTERN_FINDING,
    campaign::SERIAL_ECHO_FINDING,
    campaign::SERIAL_FORGED_FINDING,
    campaign::SERIAL_TAG_FINDING,
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

    /// A read (kind 12 or 21) whose record is still unset at the end of a
    /// boot read without its boot word on a short window: what the judge
    /// names `identity_read_outside_window`.
    fn outside_window(&self, r: &Record) -> bool {
        self.late
            && match r {
                Record::I2cCheck {
                    width,
                    status,
                    observed,
                    ..
                }
                | Record::SpiCheck {
                    width,
                    status,
                    observed,
                    ..
                } => unset_record(*width, status.as_deref(), *observed),
                _ => false,
            }
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
        let logged = |lines: &[String], what: &str| {
            lines.iter().find_map(|l| witness_bytes(l, device, what))
        };
        let reg_want = logged(expected, "Write")?;
        let write_value = write.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b));
        if reg_want != (write_value, write.len()) || *read_width > 4 {
            return None;
        }
        // A configure step reads nothing back; the others' read is the part's
        // value under the step's mask.
        let value_want = if *read_width == 0 {
            None
        } else {
            let want = logged(expected, "Read")?;
            if want.1 != usize::from(*read_width) || (want.0 ^ *value) & *value_mask != 0 {
                return None;
            }
            Some(want)
        };
        let claimed = status.as_deref() == Some("ok");
        let silent = claimed && observed.is_empty();
        let reg = logged(observed, "Write").map(|(v, _)| v);
        let read = logged(observed, "Read").map(|(v, _)| v);
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
        Some(fields)
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
        if !(open && self.booted) {
            return None;
        }
        let check = self.records.iter().position(|r| {
            matches!(r, Record::SerialFramingCheck { label: l, holds: true, tags: Some(t), .. }
                if l == label && t.holds())
        })?;
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
            || mismatch(reading) & timer_bits(motor, name, reading) != 0
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
            r if self.outside_window(r) => Scope::Window(i),
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
            // A read's mismatch is named by its bus and step (`AET-WO-4210`
            // F-B10): kind 12 is I2C's, kind 21 SPI's.
            "i2c_identity_mismatch"
            | "i2c_configure_mismatch"
            | "i2c_data_mismatch"
            | "spi_identity_mismatch"
            | "spi_configure_mismatch"
            | "spi_data_mismatch" => self.indices(|r| {
                matches!(r, Record::I2cCheck { status: Some(s), expected, mask, width, observed, step, .. }
                    | Record::SpiCheck { status: Some(s), expected, mask, width, observed, step, .. }
                    if s == "ok" && !read_matches(*width, *expected, *mask, *observed)
                        && mismatch_kind(if r.kind() == 12 { "i2c" } else { "spi" }, step).as_deref()
                            == Some(kind))
            }),
            "i2c_transaction_unwitnessed" => self.indices(|r| match r {
                Record::I2cWitness {
                    reference,
                    step,
                    expected,
                    observed,
                    ..
                } => {
                    matches!(self.check_of(reference, step),
                        Some(Record::I2cCheck { status: Some(s), .. }) if s == "ok")
                        && observed.as_deref() != Some(expected.as_slice())
                }
                _ => false,
            }),
            "spi_timeout" => self.indices(
                |r| matches!(r, Record::SpiCheck { status: Some(s), .. } if s.starts_with("timeout_")),
            ),
            "spi_record_invalid" => self.indices(
                |r| matches!(r, Record::SpiCheck { status: Some(s), .. } if s == "invalid"),
            ),
            campaign::OUTSIDE_WINDOW_FINDING => self.indices(|r| self.outside_window(r)),
            "spi_transaction_unwitnessed" => self.indices(|r| match r {
                Record::SpiWitness {
                    reference,
                    step,
                    expected,
                    observed,
                    ..
                } => {
                    matches!(self.spi_check_of(reference, step),
                        Some(Record::SpiCheck { status: Some(s), .. }) if s == "ok")
                        && observed.as_deref() != Some(expected.as_slice())
                }
                _ => false,
            }),
            campaign::MOTOR_STATIC_FINDING => {
                self.indices(|r| matches!(r, Record::MotorTimingCheck { holds: false, .. }))
            }
            // A witness is judged once the boot word is set, as `check_run`
            // binds it; before it, it shows nothing (`AET-WO-3201`).
            campaign::MOTOR_UNWITNESSED_FINDING => self.indices(|r| {
                self.booted
                    && matches!(r, Record::MotorWitness { status, .. }
                    if status == "unwitnessed" || status == "unreadable")
            }),
            campaign::MOTOR_MISMATCH_FINDING => self.indices(|r| {
                self.booted
                    && matches!(r, Record::MotorWitness { status, .. }
                    if status == "mismatch" || status == "batched" || status == "stopped")
            }),
            campaign::SERIAL_STATIC_FINDING => {
                self.indices(|r| matches!(r, Record::SerialFramingCheck { holds: false, .. }))
            }
            campaign::SERIAL_TAG_FINDING => self.indices(|r| self.serial_tags_failed(r)),
            campaign::SERIAL_PATTERN_FINDING | campaign::SERIAL_ECHO_FINDING => {
                self.indices(|r| self.serial_witness_findings(r).contains(&kind))
            }
            // A capture directory holding what no port wrote is forged too and
            // names no port, so the count may pass the witnesses.
            campaign::SERIAL_FORGED_FINDING => {
                return Some((
                    self.indices(|r| self.serial_witness_findings(r).contains(&kind)),
                    false,
                ));
            }
            "register_mismatch" => self.indices(
                |r| matches!(r, Record::RegisterObserved { reading, .. } if mismatch(reading) != 0),
            ),
            // A boot that was read, as `check_run` binds the finding; an
            // unread one names why it read nothing instead (adversarial F3).
            campaign::BOOT_WORD_FINDING => self.indices(|r| {
                matches!(r, Record::BootObserved { boot_word, .. }
                    if boot_word.observed.is_some_and(|o| o != boot_word.expected))
            }),
            campaign::PC_OUTSIDE_FINDING => self.indices(pc_outside),
            campaign::BROWN_OUT_FINDING => self.indices(brown_out_failed),
            campaign::FLASH_ACCESS_FINDING => self.indices(flash_access_failed),
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
                if software.is_empty() {
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
            Scope::Window(i) => (Rule::I7, vec![i], vec![]),
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

/// The finding a read's mismatch is named by, from its step: `{bus}_identity_mismatch`,
/// `{bus}_configure_mismatch` or `{bus}_data_mismatch` (`AET-WO-4210`
/// F-B10), as the judge names it from the part's step.
fn mismatch_kind(bus: &str, step: &str) -> Option<String> {
    let step = match aethelgard_chaos::record::step_index(step)? {
        0 => "identity",
        1..=4 => "configure",
        _ => "data",
    };
    Some(format!("{bus}_{step}_mismatch"))
}

/// Whether a read's record is as the judge filled it before the core ran:
/// status `invalid` and every value word the fill.
// belt-and-suspenders: `aethelgard_twin::boot::{RECORD_UNSET, value_words}`
// state the fill and the word count; evolve reads an archive without the
// twin, so it states them again.
fn unset_record(width: u8, status: Option<&str>, observed: Option<u64>) -> bool {
    status == Some("invalid") && observed == Some(record_words(width))
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
    let tim_size = f.layout("TIM")?.size;
    let mut motors = Vec::new();
    for r in records {
        if let Record::MotorOutputSolved {
            label,
            timer,
            channel,
            pin,
            alternate_function,
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
                channel: *channel,
                label,
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
    let booted = records.iter().any(|r| {
        matches!(r, Record::BootObserved { boot_word, .. }
            if boot_word.observed == Some(boot_word.expected))
    });
    let state = Run {
        records,
        buses,
        motors,
        serial,
        rcc: (rcc, rcc + f.layout("RCC")?.size),
        booted,
        // belt-and-suspenders: `check_run` already refuses the finding off
        // a short window; this states the judge's rule where it is used.
        late: !booted
            && campaign::short_window(records)
            && records.iter().any(|r| {
                matches!(r, Record::BootObserved { boot_word, .. } if boot_word.observed.is_some())
            }),
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
                // The motor outputs' fields of a timer register, as one.
                let timer = run.timer_owned(name, reading);
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
