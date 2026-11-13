//! Trace items: the header, the events, and the manifest, each as a CBOR
//! array of the shape `README.md` states.

use crate::cbor::Value;
use crate::record::{self, Record};

/// The only time unit the trace admits.
pub const TIME_UNIT: &str = "ns";
/// The format version of the mirror record.
pub const VERSION: u64 = 1;
/// The format version of the trace header. 2 since `AET-WO-2300` added the
/// campaign's event kinds 5 to 14 ([`Record`]); 3 since `AET-WO-3100` added
/// kind 15, the solved I2C bus, and the run rules that join to it; 4 since
/// `AET-WO-2301` added kinds 16 to 20, a declared supply droop.
pub const TRACE_VERSION: u64 = 4;
/// The manifest's version. 2 since `AET-WO-0400a`: `state_hash` chains each
/// event's delta, not the whole state, so a version-1 manifest means a hash
/// this reader no longer computes.
pub const MANIFEST_VERSION: u64 = 2;
pub const TRACE_MAGIC: &str = "aethelgard-trace";
pub const MANIFEST_MAGIC: &str = "aethelgard-manifest";
pub const MIRROR_MAGIC: &str = "aethelgard-mirror";

/// One simulator event. The timestamp is in [`TIME_UNIT`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    RegisterWrite {
        timestamp: u64,
        address: u32,
        value: u32,
    },
    PinLevel {
        timestamp: u64,
        pin: String,
        level: u8,
    },
    Interrupt {
        timestamp: u64,
        line: u16,
    },
    AnalogSample {
        timestamp: u64,
        net: String,
        microvolts: i64,
    },
    FaultInjected {
        timestamp: u64,
        scenario: String,
        detail: String,
    },
    /// Kinds 5 to 20: a campaign run's records (`AET-WO-2300`, `AET-WO-3100`,
    /// `AET-WO-2301`).
    Record {
        timestamp: u64,
        record: Record,
    },
}

/// Why an item cannot be built from a decoded value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    UnknownKind(u64),
    Malformed(&'static str),
    Version(u64),
}

fn malformed<T>(reason: &'static str) -> Result<T, Fault> {
    Err(Fault::Malformed(reason))
}

/// A pin, net or scenario name: 1 to 64 bytes of printable ASCII, no space.
pub fn name_ok(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

/// Free text: up to `max` bytes of printable ASCII, space included.
pub fn text_ok(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

fn uint(v: &Value) -> Option<u64> {
    match v {
        Value::Uint(n) => Some(*n),
        _ => None,
    }
}

fn text(v: &Value) -> Option<&str> {
    match v {
        Value::Text(t) => Some(t),
        _ => None,
    }
}

impl Event {
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::RegisterWrite { timestamp, .. }
            | Self::PinLevel { timestamp, .. }
            | Self::Interrupt { timestamp, .. }
            | Self::AnalogSample { timestamp, .. }
            | Self::FaultInjected { timestamp, .. }
            | Self::Record { timestamp, .. } => *timestamp,
        }
    }

    /// The field-level rules of `README.md`; the writer and the reader both apply them.
    pub fn check(&self) -> Result<(), &'static str> {
        match self {
            Self::RegisterWrite { .. } => Ok(()),
            Self::PinLevel { pin, level, .. } => {
                if !name_ok(pin) {
                    Err("pin name is not 1 to 64 printable ASCII bytes")
                } else if *level > 1 {
                    Err("pin level is not 0 or 1")
                } else {
                    Ok(())
                }
            }
            Self::Interrupt { line, .. } => {
                if *line > 1023 {
                    Err("interrupt line exceeds 1023")
                } else {
                    Ok(())
                }
            }
            Self::AnalogSample { net, .. } => {
                if name_ok(net) {
                    Ok(())
                } else {
                    Err("net name is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::FaultInjected {
                scenario, detail, ..
            } => {
                if !name_ok(scenario) {
                    Err("scenario name is not 1 to 64 printable ASCII bytes")
                } else if !text_ok(detail, 0, 1024) {
                    Err("detail is not 0 to 1024 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::Record { record, .. } => record.check(),
        }
    }

    pub fn to_value(&self) -> Value {
        let t = |s: &str| Value::Text(s.to_owned());
        let u = |n: u64| Value::Uint(n);
        let items = match self {
            Self::RegisterWrite {
                timestamp,
                address,
                value,
            } => {
                vec![
                    u(0),
                    u(*timestamp),
                    u(u64::from(*address)),
                    u(u64::from(*value)),
                ]
            }
            Self::PinLevel {
                timestamp,
                pin,
                level,
            } => {
                vec![u(1), u(*timestamp), t(pin), u(u64::from(*level))]
            }
            Self::Interrupt { timestamp, line } => {
                vec![u(2), u(*timestamp), u(u64::from(*line))]
            }
            Self::AnalogSample {
                timestamp,
                net,
                microvolts,
            } => {
                vec![u(3), u(*timestamp), t(net), Value::from_i64(*microvolts)]
            }
            Self::FaultInjected {
                timestamp,
                scenario,
                detail,
            } => {
                vec![u(4), u(*timestamp), t(scenario), t(detail)]
            }
            Self::Record { timestamp, record } => {
                let mut items = vec![u(record.kind()), u(*timestamp)];
                items.extend(record.fields());
                items
            }
        };
        Value::Array(items)
    }

    pub fn from_value(value: &Value) -> Result<Self, Fault> {
        let Value::Array(items) = value else {
            return malformed("event is not an array");
        };
        let Some(kind) = items.first().and_then(uint) else {
            return malformed("event has no unsigned kind");
        };
        if kind > record::LAST_KIND {
            return Err(Fault::UnknownKind(kind));
        }
        let Some(timestamp) = items.get(1).and_then(uint) else {
            return malformed("event has no unsigned timestamp");
        };
        let event = match (kind, &items[2..]) {
            (0, [a, v]) => match (uint(a).map(u32::try_from), uint(v).map(u32::try_from)) {
                (Some(Ok(address)), Some(Ok(value))) => Self::RegisterWrite {
                    timestamp,
                    address,
                    value,
                },
                _ => return malformed("register write field is not a u32"),
            },
            (1, [p, l]) => match (text(p), uint(l).map(u8::try_from)) {
                (Some(pin), Some(Ok(level))) => Self::PinLevel {
                    timestamp,
                    pin: pin.to_owned(),
                    level,
                },
                _ => return malformed("pin level fields have the wrong types"),
            },
            (2, [l]) => match uint(l).map(u16::try_from) {
                Some(Ok(line)) => Self::Interrupt { timestamp, line },
                _ => return malformed("interrupt line is not a u16"),
            },
            (3, [n, m]) => match (text(n), m.as_i64()) {
                (Some(net), Some(microvolts)) => Self::AnalogSample {
                    timestamp,
                    net: net.to_owned(),
                    microvolts,
                },
                _ => return malformed("analog sample fields have the wrong types"),
            },
            (4, [s, d]) => match (text(s), text(d)) {
                (Some(scenario), Some(detail)) => Self::FaultInjected {
                    timestamp,
                    scenario: scenario.to_owned(),
                    detail: detail.to_owned(),
                },
                _ => return malformed("fault fields have the wrong types"),
            },
            (record::FIRST_KIND.., payload) => match Record::from_fields(kind, payload) {
                Some(record) => Self::Record { timestamp, record },
                None => return malformed("campaign record fields have the wrong types or length"),
            },
            _ => return malformed("event payload has the wrong length"),
        };
        match event.check() {
            Ok(()) => Ok(event),
            Err(reason) => malformed(reason),
        }
    }
}

/// The first item of `trace.cbor`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub run_id: [u8; 16],
    /// `[name, sha256]`, sorted by name, names unique (`INV-03`).
    pub artifacts: Vec<(String, [u8; 32])>,
}

impl Header {
    pub fn check(&self) -> Result<(), &'static str> {
        for (name, _) in &self.artifacts {
            if !text_ok(name, 1, 1024) {
                return Err("artifact name is not 1 to 1024 printable ASCII bytes");
            }
        }
        if self.artifacts.windows(2).any(|w| w[0].0 >= w[1].0) {
            return Err("artifacts are not sorted by unique name");
        }
        Ok(())
    }

    pub fn to_value(&self) -> Value {
        let artifacts = self
            .artifacts
            .iter()
            .map(|(name, hash)| {
                Value::Array(vec![Value::Text(name.clone()), Value::Bytes(hash.to_vec())])
            })
            .collect();
        Value::Array(vec![
            Value::Text(TRACE_MAGIC.to_owned()),
            Value::Uint(TRACE_VERSION),
            Value::Bytes(self.run_id.to_vec()),
            Value::Text(TIME_UNIT.to_owned()),
            Value::Array(artifacts),
        ])
    }

    pub fn from_value(value: &Value) -> Result<Self, Fault> {
        let items = envelope(value, TRACE_MAGIC, TRACE_VERSION)?;
        let [
            _,
            _,
            Value::Bytes(run_id),
            Value::Text(unit),
            Value::Array(list),
        ] = items
        else {
            return malformed("header has the wrong shape");
        };
        if unit != TIME_UNIT {
            return malformed("time unit is not ns");
        }
        let Ok(run_id) = <[u8; 16]>::try_from(run_id.as_slice()) else {
            return malformed("run id is not 16 bytes");
        };
        let mut artifacts = Vec::with_capacity(list.len());
        for entry in list {
            let Value::Array(pair) = entry else {
                return malformed("artifact is not an array");
            };
            let [Value::Text(name), Value::Bytes(hash)] = pair.as_slice() else {
                return malformed("artifact is not [name, sha256]");
            };
            let Ok(hash) = <[u8; 32]>::try_from(hash.as_slice()) else {
                return malformed("artifact hash is not 32 bytes");
            };
            artifacts.push((name.clone(), hash));
        }
        let header = Self { run_id, artifacts };
        match header.check() {
            Ok(()) => Ok(header),
            Err(reason) => malformed(reason),
        }
    }
}

/// Check the magic and the version of a `[magic, version, …]` item and
/// return its elements. The version is judged before the shape, so a future
/// format reads as an unknown version and not as a malformed one.
fn envelope<'a>(value: &'a Value, magic: &str, version: u64) -> Result<&'a [Value], Fault> {
    let Value::Array(items) = value else {
        return malformed("item is not an array");
    };
    if items.first().and_then(text) != Some(magic) {
        return malformed("magic text is wrong");
    }
    match items.get(1).and_then(uint) {
        Some(found) if found == version => Ok(items),
        Some(other) => Err(Fault::Version(other)),
        None => malformed("item has no unsigned version"),
    }
}

/// `manifest.cbor`: what makes a run committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub trace_bytes: u64,
    pub trace_sha256: [u8; 32],
    pub event_count: u64,
    pub state_hash: [u8; 32],
}

impl Manifest {
    pub fn to_value(&self) -> Value {
        Value::Array(vec![
            Value::Text(MANIFEST_MAGIC.to_owned()),
            Value::Uint(MANIFEST_VERSION),
            Value::Uint(self.trace_bytes),
            Value::Bytes(self.trace_sha256.to_vec()),
            Value::Uint(self.event_count),
            Value::Bytes(self.state_hash.to_vec()),
        ])
    }

    pub fn from_value(value: &Value) -> Result<Self, Fault> {
        let items = envelope(value, MANIFEST_MAGIC, MANIFEST_VERSION)?;
        let [
            _,
            _,
            Value::Uint(trace_bytes),
            Value::Bytes(sha),
            Value::Uint(event_count),
            Value::Bytes(state),
        ] = items
        else {
            return malformed("manifest has the wrong shape");
        };
        let (Ok(trace_sha256), Ok(state_hash)) = (
            <[u8; 32]>::try_from(sha.as_slice()),
            <[u8; 32]>::try_from(state.as_slice()),
        ) else {
            return malformed("manifest hash is not 32 bytes");
        };
        Ok(Self {
            trace_bytes: *trace_bytes,
            trace_sha256,
            event_count: *event_count,
            state_hash,
        })
    }
}
