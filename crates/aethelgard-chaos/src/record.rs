//! A campaign run's records (`AET-WO-2300`): event kinds 5 to 14, one per
//! record `aethelgard campaign` archives. Every field is an integer, a byte
//! string, a text string or an array of them; nothing is a float or a map
//! (`RUL-06`). An absent value is an empty array, a present one an array of
//! one element.

use crate::cbor::Value;
use crate::event::{name_ok, text_ok};

/// The first campaign kind; [`Record::kind`] is this plus the variant's index.
pub const FIRST_KIND: u64 = 5;
/// The last campaign kind.
pub const LAST_KIND: u64 = 14;
/// The first kind read at the boot's end; kinds before it are decided before
/// the boot starts, at virtual time 0.
pub const FIRST_BOOT_KIND: u64 = 10;

/// One override a scenario applies to its specification (`AET-WO-2300` §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Override {
    /// `[0, bus, ohms]`.
    PullUpOhms { bus: String, ohms: u32 },
    /// `[1, net, femtofarads]`.
    MaxCapacitance { net: String, femtofarads: u64 },
    /// `[2, bus, line]`, `line` 0 for SDA and 1 for SCL.
    LineHeldLow { bus: String, line: Line },
}

/// One line of an I2C bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Sda,
    Scl,
}

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
    /// Kind 13: the controller state that witnesses the last read on a bus.
    I2cWitness {
        peripheral: String,
        reference: String,
        cr2: Reading,
        txdr: Reading,
    },
    /// Kind 14: the verdict, with each distinct finding kind, its severity and
    /// how many times it was found, sorted.
    Verdict {
        pass: bool,
        findings: Vec<(String, String, u64)>,
    },
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
    })
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
            }
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
                cr2,
                txdr,
            } => vec![t(peripheral), t(reference), reading(cr2), reading(txdr)],
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
                cr2: f.reading()?,
                txdr: f.reading()?,
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
                ..
            } => {
                if name_ok(peripheral) && name_ok(reference) {
                    Ok(())
                } else {
                    Err("peripheral or reference is not 1 to 64 printable ASCII bytes")
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
        }
    }
}
