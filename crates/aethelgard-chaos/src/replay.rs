//! The replay fold and the state-sequence hash (`INV-04`).

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::ArchiveError;
use crate::cbor::{self, Value};
use crate::event::Event;

/// The state a trace folds into.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub registers: BTreeMap<u32, u32>,
    pub pins: BTreeMap<String, u8>,
    pub interrupts: BTreeMap<u32, u64>,
    pub nets: BTreeMap<String, i64>,
    pub faults: u64,
}

impl State {
    fn apply(&mut self, event: &Event) {
        match event {
            Event::RegisterWrite { address, value, .. } => {
                self.registers.insert(*address, *value);
            }
            Event::PinLevel { pin, level, .. } => {
                self.pins.insert(pin.clone(), *level);
            }
            Event::Interrupt { line, .. } => {
                *self.interrupts.entry(u32::from(*line)).or_insert(0) += 1;
            }
            Event::AnalogSample {
                net, microvolts, ..
            } => {
                self.nets.insert(net.clone(), *microvolts);
            }
            Event::FaultInjected { .. } => self.faults += 1,
        }
    }

    /// The profile encoding, every map an array of `[key, value]` pairs in key order.
    pub fn encode(&self, out: &mut Vec<u8>) {
        fn pairs<K, V>(
            map: &BTreeMap<K, V>,
            key: impl Fn(&K) -> Value,
            val: impl Fn(&V) -> Value,
        ) -> Value {
            Value::Array(
                map.iter()
                    .map(|(k, v)| Value::Array(vec![key(k), val(v)]))
                    .collect(),
            )
        }
        let state = Value::Array(vec![
            pairs(
                &self.registers,
                |k| Value::Uint(u64::from(*k)),
                |v| Value::Uint(u64::from(*v)),
            ),
            pairs(
                &self.pins,
                |k| Value::Text(k.clone()),
                |v| Value::Uint(u64::from(*v)),
            ),
            pairs(
                &self.interrupts,
                |k| Value::Uint(u64::from(*k)),
                |v| Value::Uint(*v),
            ),
            pairs(
                &self.nets,
                |k| Value::Text(k.clone()),
                |v| Value::from_i64(*v),
            ),
            Value::Uint(self.faults),
        ]);
        cbor::encode(&state, out);
    }
}

/// The incremental fold. The writer and the reader both drive it, so a
/// trace's `state_hash` is computed one way only.
pub struct Replayer {
    state: State,
    hash: [u8; 32],
    count: u64,
    last_timestamp: u64,
    scratch: Vec<u8>,
}

impl Replayer {
    /// `header_bytes` are the header item's encoding; `h0` is their sha256.
    pub fn new(header_bytes: &[u8]) -> Self {
        Self {
            state: State::default(),
            hash: Sha256::digest(header_bytes).into(),
            count: 0,
            last_timestamp: 0,
            scratch: Vec::new(),
        }
    }

    /// Fold one event; refuses a timestamp that goes backwards.
    pub fn apply(&mut self, event: &Event) -> Result<(), ArchiveError> {
        if event.timestamp() < self.last_timestamp {
            return Err(ArchiveError::TimestampRegressed { index: self.count });
        }
        self.last_timestamp = event.timestamp();
        self.state.apply(event);
        self.scratch.clear();
        self.state.encode(&mut self.scratch);
        let mut hasher = Sha256::new();
        hasher.update(self.hash);
        hasher.update(&self.scratch);
        self.hash = hasher.finalize().into();
        self.count += 1;
        Ok(())
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn event_count(&self) -> u64 {
        self.count
    }

    /// The state-sequence hash so far; with no events, `h0`.
    pub fn state_hash(&self) -> [u8; 32] {
        self.hash
    }
}
