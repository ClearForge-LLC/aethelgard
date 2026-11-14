//! A campaign run's records (`AET-WO-2300`): event kinds 5 to 20, one per
//! record `aethelgard campaign` archives (kind 15 since `AET-WO-3100`; 16 to
//! 20, a declared supply droop, since `AET-WO-2301`). Every field is an integer, a byte
//! string, a text string or an array of them; nothing is a float or a map
//! (`RUL-06`). An absent value is an empty array, a present one an array of
//! one element.

use crate::cbor::Value;
use crate::event::{name_ok, text_ok};

/// The first campaign kind; [`Record::kind`] is this plus the variant's index.
pub const FIRST_KIND: u64 = 5;
/// The last campaign kind.
pub const LAST_KIND: u64 = 20;

/// One override a scenario applies to its specification (`AET-WO-2300` §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Override {
    /// `[0, bus, ohms]`.
    PullUpOhms { bus: String, ohms: u32 },
    /// `[1, net, femtofarads]`.
    MaxCapacitance { net: String, femtofarads: u64 },
    /// `[2, bus, line]`, `line` 0 for SDA and 1 for SCL.
    LineHeldLow { bus: String, line: Line },
    /// `[3, domain, floor_mv, start_ns, fall_ns, hold_ns, rise_ns,
    /// source_milliohms, load_milliamps, bor_level]` (`AET-WO-2301`).
    Droop(Droop),
}

/// A declared sag of a voltage domain's source (`AET-WO-2301` P2, ruled D2
/// and D7): from the domain's nominal down to `floor_millivolts` and back,
/// through `source_milliohms` into the emitter's decoupling, with a constant
/// `load_milliamps`; `bor_level` is the part's option-byte BOR level, declared
/// because nothing else states it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Droop {
    pub domain: String,
    pub floor_millivolts: u32,
    pub start_ns: u64,
    pub fall_ns: u64,
    pub hold_ns: u64,
    pub rise_ns: u64,
    pub source_milliohms: u32,
    pub load_milliamps: u32,
    pub bor_level: u8,
}

impl Droop {
    /// The I·R drop the load makes across the source, in µV.
    pub fn drop_microvolts(&self) -> u64 {
        u64::from(self.load_milliamps) * u64::from(self.source_milliohms)
    }
}

/// One line of an I2C bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Sda,
    Scl,
}

/// One line of a solved I2C bus, as kind 15 names it: the solver's signal
/// (`I2C1_SCL`), its net, its pin and alternate function, and the net's
/// rise-time ceiling, which `analog_lines` holds per net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusLine {
    pub signal: String,
    pub net: String,
    pub pin: String,
    pub alternate_function: u8,
    pub ceiling_ps: u64,
}

/// A solved pin's port letter and index, `PB6` as (`B`, 6): `P`, one capital
/// letter, then 0 to 15 in decimal with no leading zero, the form the solver
/// names pins in. Anything else is not a pin.
pub fn split_pin(pin: &str) -> Option<(char, u8)> {
    let rest = pin.strip_prefix('P')?;
    let mut chars = rest.chars();
    let port = chars.next().filter(char::is_ascii_uppercase)?;
    let digits = chars.as_str();
    let canonical = matches!(digits.len(), 1 | 2)
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !(digits.len() == 2 && digits.starts_with('0'));
    let index: u8 = digits.parse().ok().filter(|_| canonical)?;
    (index <= 15).then_some((port, index))
}

/// The highest alternate function a GPIO pin has (`AFSEL` is four bits).
pub const MAX_ALTERNATE_FUNCTION: u8 = 15;

/// A register as the boot read it: what the topology expects under `mask`,
/// and what Renode read, absent when the boot was killed before the reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub address: u32,
    pub mask: u32,
    pub expected: u32,
    pub observed: Option<u32>,
}

/// One campaign record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// Kind 5: the scenario file's sha256, its name, what it expects, and its overrides.
    Scenario {
        name: String,
        sha256: [u8; 32],
        expect_pass: bool,
        /// The finding kinds a failing scenario expects, sorted and unique.
        expect_findings: Vec<String>,
        overrides: Vec<Override>,
    },
    /// Kind 6: the overridden specification's and the solved topology's sha256.
    Solved {
        specification_sha256: [u8; 32],
        topology_sha256: [u8; 32],
    },
    /// Kind 7: `analog.rise_time_measured`. With `complete` false, the rise
    /// is a lower bound.
    RiseMeasured {
        bus: String,
        net: String,
        rise_30_70_ps: u64,
        complete: bool,
        ceiling_ps: u64,
        exceeds: bool,
    },
    /// Kind 8: `analog.line_stuck_high`.
    LineStuckHigh {
        bus: String,
        net: String,
        microvolts_at_release: i64,
        ceiling_ps: u64,
    },
    /// Kind 9: `bridge.fault_state`.
    BusFault {
        bus: String,
        fault: String,
        source: String,
        from: Vec<String>,
        nets: Vec<String>,
        firmware_sees: String,
    },
    /// Kind 10: the boot's witnesses (`firmware.observed`).
    BootObserved {
        image_sha256: [u8; 32],
        boot_word: Reading,
        instructions_expected: u64,
        instructions_observed: Option<u64>,
        pc: Option<u32>,
    },
    /// Kind 11: one solved register after the boot.
    RegisterObserved { name: String, reading: Reading },
    /// Kind 12: one identity read, as the firmware recorded it.
    I2cCheck {
        reference: String,
        address: u8,
        register: u8,
        expected: u64,
        observed: Option<u64>,
        status: Option<String>,
    },
    /// Kind 13: one identity read's witness (`AET-WO-2204` D1): the whole
    /// log lines the device's model writes for the read, as the judge
    /// expects them, and every line Renode logged for the device with a
    /// witness's shape, in order, at most [`MAX_WITNESS_LINES`]; `None` when
    /// the log could not be read whole (past the filter's budget).
    I2cWitness {
        peripheral: String,
        reference: String,
        /// The model's path in Renode, `i2c1.u2`.
        device: String,
        expected: Vec<String>,
        observed: Option<Vec<String>>,
        /// The addresses the device's controller found no device at during
        /// the run, in order, at most [`MAX_WITNESS_LINES`]: attribution's
        /// evidence of a transfer to a wrong address, never a witness.
        absent: Vec<u8>,
    },
    /// Kind 14: the verdict, with each distinct finding kind, its severity and
    /// how many times it was found, sorted.
    Verdict {
        pass: bool,
        findings: Vec<(String, String, u64)>,
    },
    /// Kind 15: one solved I2C bus, from the solved topology alone
    /// (`AET-WO-3100` R1): its controller, its two lines, SCL then SDA, and
    /// its devices by reference, sorted. It joins a device or a controller to
    /// the bus kinds 7 to 9 name. Archived after kind 6, before any kind 7.
    I2cBusSolved {
        bus: String,
        peripheral: String,
        lines: Vec<BusLine>,
        devices: Vec<(String, u8)>,
    },
    /// Kind 16: the declared rail (`AET-WO-2301`, ruled D8): every value the
    /// rail was computed from, so the waveform is re-derivable from the trace.
    /// The droop's own values, then the domain's nominal and tolerance and the
    /// decoupling the emitter places on it.
    RailDeclared {
        droop: Droop,
        nominal_millivolts: u32,
        tolerance_permille: u32,
        decoupling_picofarads: u64,
    },
    /// Kind 17: `analog.rail_crossing_computed`, one crossing of one
    /// threshold's level, computed by ngspice from the declared values.
    RailCrossing {
        domain: String,
        level: String,
        microvolts: u64,
        at_ps: u64,
        falling: bool,
    },
    /// Kind 18: `bridge.reset_rendered`, a reset asserted or released: the
    /// model's rendering of the supervisor, never a measured reset. Renode
    /// models no BOR, PVD or reset cause; `rendering` names the monitor
    /// commands that stood in for it.
    ResetRendered {
        at_ns: u64,
        released: bool,
        cause: Vec<String>,
        rendering: String,
    },
    /// Kind 19: `firmware.observed` after a release, at the recovery bound:
    /// the boot word, the identity read's two words, and the controller's
    /// `CR2` and `TXDR`, which only a transfer on the bus sets after a reset
    /// (`AET-WO-2301` adversarial F1), as Renode read them.
    FirmwareObserved {
        at_ns: u64,
        boot_word: Reading,
        status: Reading,
        value: Reading,
        /// The first identity read's witness-shaped lines from the release
        /// to the probe (`AET-WO-2204`): `None` when the boot has no identity
        /// read, or when the log could not be read whole.
        witness: Option<Vec<String>>,
    },
    /// Kind 20: the scenario's `expect_counts`, sorted by kind, archived only
    /// when it declares them.
    ExpectedCounts { counts: Vec<(String, u64)> },
}

fn u(n: u64) -> Value {
    Value::Uint(n)
}

fn t(s: &str) -> Value {
    Value::Text(s.to_owned())
}

fn flag(b: bool) -> Value {
    u(u64::from(b))
}

fn opt(v: Option<u64>) -> Value {
    Value::Array(v.map(u).into_iter().collect())
}

fn texts(v: &[String]) -> Value {
    Value::Array(v.iter().map(|s| t(s)).collect())
}

fn opt_texts(v: Option<&[String]>) -> Value {
    Value::Array(v.map(texts).into_iter().collect())
}

/// Witness lines as a record carries them: at most [`MAX_WITNESS_LINES`],
/// each 1 to 256 printable ASCII bytes.
fn witness_lines_ok(v: &[String]) -> bool {
    v.len() <= MAX_WITNESS_LINES && v.iter().all(|s| text_ok(s, 1, 256))
}

/// The most witness lines a record carries: a judge sees two per identity
/// read, so a longer observation already differs from what it expects, and
/// cutting it here cannot make it match.
pub const MAX_WITNESS_LINES: usize = 16;

fn reading(r: &Reading) -> Value {
    Value::Array(vec![
        u(r.address.into()),
        u(r.mask.into()),
        u(r.expected.into()),
        opt(r.observed.map(u64::from)),
    ])
}

/// Reads fields in order; every accessor is `None` on a wrong type.
struct Fields<'a>(std::slice::Iter<'a, Value>);

impl<'a> Fields<'a> {
    fn next(&mut self) -> Option<&'a Value> {
        self.0.next()
    }
    fn uint(&mut self) -> Option<u64> {
        match self.next()? {
            Value::Uint(n) => Some(*n),
            _ => None,
        }
    }
    fn int(&mut self) -> Option<i64> {
        self.next()?.as_i64()
    }
    fn u32(&mut self) -> Option<u32> {
        u32::try_from(self.uint()?).ok()
    }
    fn u8(&mut self) -> Option<u8> {
        u8::try_from(self.uint()?).ok()
    }
    fn flag(&mut self) -> Option<bool> {
        match self.uint()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn text(&mut self) -> Option<String> {
        match self.next()? {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn hash(&mut self) -> Option<[u8; 32]> {
        match self.next()? {
            Value::Bytes(b) => <[u8; 32]>::try_from(b.as_slice()).ok(),
            _ => None,
        }
    }
    fn array(&mut self) -> Option<&'a [Value]> {
        match self.next()? {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    fn opt(&mut self) -> Option<Option<u64>> {
        match self.array()? {
            [] => Some(None),
            [Value::Uint(n)] => Some(Some(*n)),
            _ => None,
        }
    }
    fn opt_u32(&mut self) -> Option<Option<u32>> {
        match self.opt()? {
            None => Some(None),
            Some(n) => u32::try_from(n).ok().map(Some),
        }
    }
    fn texts(&mut self) -> Option<Vec<String>> {
        let mut inner = Fields(self.array()?.iter());
        let mut out = Vec::new();
        while inner.0.len() > 0 {
            out.push(inner.text()?);
        }
        Some(out)
    }
    fn opt_texts(&mut self) -> Option<Option<Vec<String>>> {
        let mut inner = Fields(self.array()?.iter());
        if inner.0.len() == 0 {
            return Some(None);
        }
        let v = inner.texts()?;
        inner.end()?;
        Some(Some(v))
    }
    fn reading(&mut self) -> Option<Reading> {
        let mut r = Fields(self.array()?.iter());
        let reading = Reading {
            address: r.u32()?,
            mask: r.u32()?,
            expected: r.u32()?,
            observed: r.opt_u32()?,
        };
        r.end()?;
        Some(reading)
    }
    fn end(&mut self) -> Option<()> {
        self.0.next().is_none().then_some(())
    }
}

fn override_value(o: &Override) -> Value {
    Value::Array(match o {
        Override::PullUpOhms { bus, ohms } => vec![u(0), t(bus), u((*ohms).into())],
        Override::MaxCapacitance { net, femtofarads } => vec![u(1), t(net), u(*femtofarads)],
        Override::LineHeldLow { bus, line } => vec![
            u(2),
            t(bus),
            u(match line {
                Line::Sda => 0,
                Line::Scl => 1,
            }),
        ],
        Override::Droop(d) => {
            let mut v = vec![u(3)];
            v.extend(droop_fields(d));
            v
        }
    })
}

fn droop_fields(d: &Droop) -> Vec<Value> {
    vec![
        t(&d.domain),
        u(d.floor_millivolts.into()),
        u(d.start_ns),
        u(d.fall_ns),
        u(d.hold_ns),
        u(d.rise_ns),
        u(d.source_milliohms.into()),
        u(d.load_milliamps.into()),
        u(d.bor_level.into()),
    ]
}

impl Fields<'_> {
    fn droop(&mut self) -> Option<Droop> {
        Some(Droop {
            domain: self.text()?,
            floor_millivolts: self.u32()?,
            start_ns: self.uint()?,
            fall_ns: self.uint()?,
            hold_ns: self.uint()?,
            rise_ns: self.uint()?,
            source_milliohms: self.u32()?,
            load_milliamps: self.u32()?,
            bor_level: self.u8()?,
        })
    }
}

fn override_from(v: &Value) -> Option<Override> {
    let Value::Array(items) = v else { return None };
    let mut f = Fields(items.iter());
    let o = match f.uint()? {
        0 => Override::PullUpOhms {
            bus: f.text()?,
            ohms: f.u32()?,
        },
        1 => Override::MaxCapacitance {
            net: f.text()?,
            femtofarads: f.uint()?,
        },
        2 => Override::LineHeldLow {
            bus: f.text()?,
            line: match f.uint()? {
                0 => Line::Sda,
                1 => Line::Scl,
                _ => return None,
            },
        },
        3 => Override::Droop(f.droop()?),
        _ => return None,
    };
    f.end()?;
    Some(o)
}

impl Record {
    pub fn kind(&self) -> u64 {
        FIRST_KIND
            + match self {
                Self::Scenario { .. } => 0,
                Self::Solved { .. } => 1,
                Self::RiseMeasured { .. } => 2,
                Self::LineStuckHigh { .. } => 3,
                Self::BusFault { .. } => 4,
                Self::BootObserved { .. } => 5,
                Self::RegisterObserved { .. } => 6,
                Self::I2cCheck { .. } => 7,
                Self::I2cWitness { .. } => 8,
                Self::Verdict { .. } => 9,
                Self::I2cBusSolved { .. } => 10,
                Self::RailDeclared { .. } => 11,
                Self::RailCrossing { .. } => 12,
                Self::ResetRendered { .. } => 13,
                Self::FirmwareObserved { .. } => 14,
                Self::ExpectedCounts { .. } => 15,
            }
    }

    /// The virtual time a record is archived at: 0 for what is decided
    /// before the boot, its own time for the droop's crossings, resets and
    /// observations, and `end_ns` for what the boot's end reads.
    pub fn timestamp(&self, end_ns: u64) -> u64 {
        match self {
            Self::RailCrossing { at_ps, .. } => at_ps / 1000,
            Self::ResetRendered { at_ns, .. } | Self::FirmwareObserved { at_ns, .. } => *at_ns,
            _ if self.before_boot() => 0,
            _ => end_ns,
        }
    }

    /// Whether the record is decided before the boot starts, at virtual time
    /// 0: the scenario, the topology, the solved buses, the analog records and
    /// the bus faults. The rest are read at the boot's end.
    pub fn before_boot(&self) -> bool {
        matches!(
            self,
            Self::Scenario { .. }
                | Self::Solved { .. }
                | Self::I2cBusSolved { .. }
                | Self::RiseMeasured { .. }
                | Self::LineStuckHigh { .. }
                | Self::BusFault { .. }
                | Self::RailDeclared { .. }
                | Self::ExpectedCounts { .. }
        )
    }

    /// The payload after the kind and the timestamp.
    pub fn fields(&self) -> Vec<Value> {
        match self {
            Self::Scenario {
                name,
                sha256,
                expect_pass,
                expect_findings,
                overrides,
            } => vec![
                t(name),
                Value::Bytes(sha256.to_vec()),
                flag(*expect_pass),
                texts(expect_findings),
                Value::Array(overrides.iter().map(override_value).collect()),
            ],
            Self::Solved {
                specification_sha256,
                topology_sha256,
            } => vec![
                Value::Bytes(specification_sha256.to_vec()),
                Value::Bytes(topology_sha256.to_vec()),
            ],
            Self::RiseMeasured {
                bus,
                net,
                rise_30_70_ps,
                complete,
                ceiling_ps,
                exceeds,
            } => vec![
                t(bus),
                t(net),
                u(*rise_30_70_ps),
                flag(*complete),
                u(*ceiling_ps),
                flag(*exceeds),
            ],
            Self::LineStuckHigh {
                bus,
                net,
                microvolts_at_release,
                ceiling_ps,
            } => vec![
                t(bus),
                t(net),
                Value::from_i64(*microvolts_at_release),
                u(*ceiling_ps),
            ],
            Self::BusFault {
                bus,
                fault,
                source,
                from,
                nets,
                firmware_sees,
            } => vec![
                t(bus),
                t(fault),
                t(source),
                texts(from),
                texts(nets),
                t(firmware_sees),
            ],
            Self::BootObserved {
                image_sha256,
                boot_word,
                instructions_expected,
                instructions_observed,
                pc,
            } => vec![
                Value::Bytes(image_sha256.to_vec()),
                reading(boot_word),
                u(*instructions_expected),
                opt(*instructions_observed),
                opt(pc.map(u64::from)),
            ],
            Self::RegisterObserved { name, reading: r } => vec![t(name), reading(r)],
            Self::I2cCheck {
                reference,
                address,
                register,
                expected,
                observed,
                status,
            } => vec![
                t(reference),
                u((*address).into()),
                u((*register).into()),
                u(*expected),
                opt(*observed),
                Value::Array(status.iter().map(|s| t(s)).collect()),
            ],
            Self::I2cWitness {
                peripheral,
                reference,
                device,
                expected,
                observed,
                absent,
            } => vec![
                t(peripheral),
                t(reference),
                t(device),
                texts(expected),
                opt_texts(observed.as_deref()),
                Value::Array(absent.iter().map(|&a| u(a.into())).collect()),
            ],
            Self::Verdict { pass, findings } => vec![
                flag(*pass),
                Value::Array(
                    findings
                        .iter()
                        .map(|(kind, severity, count)| {
                            Value::Array(vec![t(kind), t(severity), u(*count)])
                        })
                        .collect(),
                ),
            ],
            Self::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => vec![
                t(bus),
                t(peripheral),
                Value::Array(
                    lines
                        .iter()
                        .map(|l| {
                            Value::Array(vec![
                                t(&l.signal),
                                t(&l.net),
                                t(&l.pin),
                                u(l.alternate_function.into()),
                                u(l.ceiling_ps),
                            ])
                        })
                        .collect(),
                ),
                Value::Array(
                    devices
                        .iter()
                        .map(|(reference, address)| {
                            Value::Array(vec![t(reference), u((*address).into())])
                        })
                        .collect(),
                ),
            ],
            Self::RailDeclared {
                droop,
                nominal_millivolts,
                tolerance_permille,
                decoupling_picofarads,
            } => {
                let mut v = droop_fields(droop);
                v.extend([
                    u((*nominal_millivolts).into()),
                    u((*tolerance_permille).into()),
                    u(*decoupling_picofarads),
                ]);
                v
            }
            Self::RailCrossing {
                domain,
                level,
                microvolts,
                at_ps,
                falling,
            } => vec![
                t(domain),
                t(level),
                u(*microvolts),
                u(*at_ps),
                flag(*falling),
            ],
            Self::ResetRendered {
                at_ns,
                released,
                cause,
                rendering,
            } => vec![u(*at_ns), flag(*released), texts(cause), t(rendering)],
            Self::FirmwareObserved {
                at_ns,
                boot_word,
                status,
                value,
                witness,
            } => vec![
                u(*at_ns),
                reading(boot_word),
                reading(status),
                reading(value),
                opt_texts(witness.as_deref()),
            ],
            Self::ExpectedCounts { counts } => vec![Value::Array(
                counts
                    .iter()
                    .map(|(kind, n)| Value::Array(vec![t(kind), u(*n)]))
                    .collect(),
            )],
        }
    }

    /// The record of `kind` from its payload, or `None` if a field has the
    /// wrong type or the payload the wrong length. [`check`](Self::check)
    /// judges the values.
    pub fn from_fields(kind: u64, payload: &[Value]) -> Option<Self> {
        let mut f = Fields(payload.iter());
        let record = match kind {
            5 => Self::Scenario {
                name: f.text()?,
                sha256: f.hash()?,
                expect_pass: f.flag()?,
                expect_findings: f.texts()?,
                overrides: f
                    .array()?
                    .iter()
                    .map(override_from)
                    .collect::<Option<_>>()?,
            },
            6 => Self::Solved {
                specification_sha256: f.hash()?,
                topology_sha256: f.hash()?,
            },
            7 => Self::RiseMeasured {
                bus: f.text()?,
                net: f.text()?,
                rise_30_70_ps: f.uint()?,
                complete: f.flag()?,
                ceiling_ps: f.uint()?,
                exceeds: f.flag()?,
            },
            8 => Self::LineStuckHigh {
                bus: f.text()?,
                net: f.text()?,
                microvolts_at_release: f.int()?,
                ceiling_ps: f.uint()?,
            },
            9 => Self::BusFault {
                bus: f.text()?,
                fault: f.text()?,
                source: f.text()?,
                from: f.texts()?,
                nets: f.texts()?,
                firmware_sees: f.text()?,
            },
            10 => Self::BootObserved {
                image_sha256: f.hash()?,
                boot_word: f.reading()?,
                instructions_expected: f.uint()?,
                instructions_observed: f.opt()?,
                pc: f.opt_u32()?,
            },
            11 => Self::RegisterObserved {
                name: f.text()?,
                reading: f.reading()?,
            },
            12 => Self::I2cCheck {
                reference: f.text()?,
                address: f.u8()?,
                register: f.u8()?,
                expected: f.uint()?,
                observed: f.opt()?,
                status: match f.array()? {
                    [] => None,
                    [Value::Text(s)] => Some(s.clone()),
                    _ => return None,
                },
            },
            13 => Self::I2cWitness {
                peripheral: f.text()?,
                reference: f.text()?,
                device: f.text()?,
                expected: f.texts()?,
                observed: f.opt_texts()?,
                absent: {
                    let mut inner = Fields(f.array()?.iter());
                    let mut v = Vec::new();
                    while inner.0.len() > 0 {
                        v.push(inner.u8()?);
                    }
                    v
                },
            },
            14 => {
                let pass = f.flag()?;
                let mut findings = Vec::new();
                for item in f.array()? {
                    let Value::Array(triple) = item else {
                        return None;
                    };
                    let mut g = Fields(triple.iter());
                    findings.push((g.text()?, g.text()?, g.uint()?));
                    g.end()?;
                }
                Self::Verdict { pass, findings }
            }
            15 => {
                let bus = f.text()?;
                let peripheral = f.text()?;
                let mut lines = Vec::new();
                for item in f.array()? {
                    let Value::Array(line) = item else {
                        return None;
                    };
                    let mut g = Fields(line.iter());
                    lines.push(BusLine {
                        signal: g.text()?,
                        net: g.text()?,
                        pin: g.text()?,
                        alternate_function: g.u8()?,
                        ceiling_ps: g.uint()?,
                    });
                    g.end()?;
                }
                let mut devices = Vec::new();
                for item in f.array()? {
                    let Value::Array(pair) = item else {
                        return None;
                    };
                    let mut g = Fields(pair.iter());
                    devices.push((g.text()?, g.u8()?));
                    g.end()?;
                }
                Self::I2cBusSolved {
                    bus,
                    peripheral,
                    lines,
                    devices,
                }
            }
            16 => Self::RailDeclared {
                droop: f.droop()?,
                nominal_millivolts: f.u32()?,
                tolerance_permille: f.u32()?,
                decoupling_picofarads: f.uint()?,
            },
            17 => Self::RailCrossing {
                domain: f.text()?,
                level: f.text()?,
                microvolts: f.uint()?,
                at_ps: f.uint()?,
                falling: f.flag()?,
            },
            18 => Self::ResetRendered {
                at_ns: f.uint()?,
                released: f.flag()?,
                cause: f.texts()?,
                rendering: f.text()?,
            },
            19 => Self::FirmwareObserved {
                at_ns: f.uint()?,
                boot_word: f.reading()?,
                status: f.reading()?,
                value: f.reading()?,
                witness: f.opt_texts()?,
            },
            20 => {
                let mut counts = Vec::new();
                for item in f.array()? {
                    let Value::Array(pair) = item else {
                        return None;
                    };
                    let mut g = Fields(pair.iter());
                    counts.push((g.text()?, g.uint()?));
                    g.end()?;
                }
                Self::ExpectedCounts { counts }
            }
            _ => return None,
        };
        f.end()?;
        Some(record)
    }

    /// The field-level rules; the writer and the reader both apply them.
    pub fn check(&self) -> Result<(), &'static str> {
        let names = |v: &[String]| v.iter().all(|s| name_ok(s));
        let sorted = |v: &[String]| v.windows(2).all(|w| w[0] < w[1]);
        match self {
            Self::Scenario {
                name,
                expect_pass,
                expect_findings,
                overrides,
                ..
            } => {
                if !name_ok(name) {
                    return Err("scenario name is not 1 to 64 printable ASCII bytes");
                }
                if !names(expect_findings) || !sorted(expect_findings) {
                    return Err("expected finding kinds are not sorted unique names");
                }
                if *expect_pass != expect_findings.is_empty() {
                    return Err(
                        "a passing scenario expects no finding, a failing one at least one",
                    );
                }
                let ids_ok = overrides.iter().all(|o| match o {
                    Override::PullUpOhms { bus, .. } | Override::LineHeldLow { bus, .. } => {
                        name_ok(bus)
                    }
                    Override::MaxCapacitance { net, .. } => name_ok(net),
                    Override::Droop(d) => name_ok(&d.domain),
                });
                if !ids_ok {
                    return Err("an override's id is not 1 to 64 printable ASCII bytes");
                }
                Ok(())
            }
            Self::Solved { .. } => Ok(()),
            Self::RiseMeasured {
                bus,
                net,
                rise_30_70_ps,
                complete,
                ceiling_ps,
                exceeds,
            } => {
                if !name_ok(bus) || !name_ok(net) {
                    Err("bus or net name is not 1 to 64 printable ASCII bytes")
                } else if *exceeds != (!complete || rise_30_70_ps > ceiling_ps) {
                    Err("exceeds is not the rise against the ceiling")
                } else {
                    Ok(())
                }
            }
            Self::LineStuckHigh { bus, net, .. } => {
                if name_ok(bus) && name_ok(net) {
                    Ok(())
                } else {
                    Err("bus or net name is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::BusFault {
                bus,
                fault,
                source,
                from,
                nets,
                firmware_sees,
            } => {
                if !name_ok(bus) || !name_ok(fault) {
                    Err("bus or fault name is not 1 to 64 printable ASCII bytes")
                } else if !text_ok(source, 1, 128) || !text_ok(firmware_sees, 0, 1024) {
                    Err("fault source or rendering is not printable ASCII of its length")
                } else if from.is_empty() || !from.iter().all(|s| text_ok(s, 1, 64)) {
                    Err("fault causes are not 1 to 64 printable ASCII bytes each")
                } else if nets.is_empty() || !names(nets) || !sorted(nets) {
                    Err("fault nets are not sorted unique names")
                } else {
                    Ok(())
                }
            }
            Self::BootObserved { .. } => Ok(()),
            Self::RegisterObserved { name, .. } => {
                if name_ok(name) {
                    Ok(())
                } else {
                    Err("register name is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::I2cCheck {
                reference, status, ..
            } => {
                if !name_ok(reference) {
                    Err("device reference is not 1 to 64 printable ASCII bytes")
                } else if status.as_deref().is_some_and(|s| !name_ok(s)) {
                    Err("status is not 1 to 64 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::I2cWitness {
                peripheral,
                reference,
                device,
                expected,
                observed,
                absent,
            } => {
                if absent.len() > MAX_WITNESS_LINES || absent.iter().any(|&a| a > 0x7F) {
                    Err("absent addresses are not at most 16 seven-bit addresses")
                } else if !(name_ok(peripheral) && name_ok(reference) && name_ok(device)) {
                    Err("peripheral, reference or device is not 1 to 64 printable ASCII bytes")
                } else if !witness_lines_ok(expected)
                    || observed.as_deref().is_some_and(|o| !witness_lines_ok(o))
                {
                    Err("witness lines are not at most 16 lines of 1 to 256 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::Verdict { pass, findings } => {
                let pairs_sorted = findings
                    .windows(2)
                    .all(|w| (&w[0].0, &w[0].1) < (&w[1].0, &w[1].1));
                if !findings
                    .iter()
                    .all(|(k, s, n)| name_ok(k) && name_ok(s) && *n > 0)
                {
                    Err("a finding is not a kind, a severity and a count above 0")
                } else if !pairs_sorted {
                    Err("findings are not sorted and unique by kind and severity")
                } else if *pass != findings.iter().all(|(_, s, _)| s == "info") {
                    Err("pass is not the absence of a finding above info")
                } else {
                    Ok(())
                }
            }
            Self::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => {
                let ends = |l: &BusLine, line: &str| {
                    l.signal.strip_prefix(peripheral.as_str()) == Some(line)
                };
                if !name_ok(bus) || !name_ok(peripheral) {
                    Err("bus or peripheral name is not 1 to 64 printable ASCII bytes")
                } else if !lines
                    .iter()
                    .all(|l| name_ok(&l.signal) && name_ok(&l.net) && name_ok(&l.pin))
                {
                    Err("a line's signal, net or pin is not 1 to 64 printable ASCII bytes")
                } else if !(lines.len() == 2 && ends(&lines[0], "_SCL") && ends(&lines[1], "_SDA"))
                {
                    Err("a solved bus's lines are not its peripheral's SCL, then its SDA")
                } else if lines[0].net == lines[1].net || lines[0].pin == lines[1].pin {
                    Err("a solved bus's two lines share a net or a pin")
                } else if !lines.iter().all(|l| {
                    split_pin(&l.pin).is_some() && l.alternate_function <= MAX_ALTERNATE_FUNCTION
                }) {
                    Err(
                        "a line's pin is not P, a port letter and 0 to 15, or its alternate function is over 15",
                    )
                } else if !devices.iter().all(|(r, _)| name_ok(r))
                    || !devices.windows(2).all(|w| w[0].0 < w[1].0)
                {
                    Err("a solved bus's devices are not sorted unique references")
                } else {
                    Ok(())
                }
            }
            Self::RailDeclared {
                droop,
                nominal_millivolts,
                decoupling_picofarads,
                ..
            } => {
                if !name_ok(&droop.domain) {
                    Err("a droop's domain is not 1 to 64 printable ASCII bytes")
                } else if droop.floor_millivolts >= *nominal_millivolts
                    || *decoupling_picofarads == 0
                {
                    Err("a declared rail's floor is not below its nominal, or it has no decoupling")
                } else {
                    Ok(())
                }
            }
            Self::RailCrossing { domain, level, .. } => {
                if name_ok(domain) && name_ok(level) {
                    Ok(())
                } else {
                    Err("a crossing's domain or level is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::ResetRendered {
                cause, rendering, ..
            } => {
                if cause.is_empty() || !names(cause) || !sorted(cause) {
                    Err("a reset's causes are not sorted unique threshold names")
                } else if !text_ok(rendering, 1, 1024) {
                    Err("a reset's rendering is not 1 to 1024 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::FirmwareObserved { witness, .. } => {
                if witness.as_deref().is_some_and(|o| !witness_lines_ok(o)) {
                    Err("witness lines are not at most 16 lines of 1 to 256 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::ExpectedCounts { counts } => {
                if counts.is_empty() || !counts.iter().all(|(k, _)| name_ok(k)) {
                    Err("expected counts are not one or more finding kinds")
                } else if !counts.windows(2).all(|w| w[0].0 < w[1].0) {
                    Err("expected counts are not sorted unique by kind")
                } else {
                    Ok(())
                }
            }
        }
    }
}
