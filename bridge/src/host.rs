//! What ngspice's callbacks reach (`ffi`): the lines it prints, an exit it
//! signals, and the waveforms of the external sources the host drives. The
//! state is behind one `Mutex`; each function here takes it for its own
//! body only, so a callback never finds it held.

use std::sync::{Mutex, MutexGuard};

use crate::request::Pwl;

/// More printed lines than any measurement needs; past it, lines are dropped
/// and counted.
const LOG_CAP: usize = 4096;

struct State {
    log: Vec<String>,
    dropped: usize,
    exit: Option<i32>,
    sources: Vec<(String, Pwl)>,
    missing: Vec<String>,
}

static STATE: Mutex<State> = Mutex::new(State {
    log: Vec::new(),
    dropped: 0,
    exit: None,
    sources: Vec::new(),
    missing: Vec::new(),
});

fn state() -> MutexGuard<'static, State> {
    // A panic while held can only come from this module's own bodies, which
    // leave the state consistent, so a poisoned lock is still usable.
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// `SendChar`: one line, prefixed `stdout ` or `stderr ` by ngspice.
pub fn print(line: &str) {
    let mut s = state();
    if s.log.len() < LOG_CAP {
        s.log.push(line.to_owned());
    } else {
        s.dropped += 1;
    }
}

/// `ControlledExit`: ngspice wants to quit. Every later result is refused.
pub fn exited(status: i32) {
    state().exit = Some(status);
}

/// `GetVSRCData`: the value of the external source `name` at `time` seconds.
pub fn source(name: &str, time: f64) -> Option<f64> {
    let mut s = state();
    let found = s
        .sources
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, w)| w.at(time));
    if found.is_none() {
        s.missing.push(name.to_owned());
    }
    found
}

/// `GetISRCData`: no external current source is ever driven.
pub fn source_missing(name: &str) {
    state().missing.push(name.to_owned());
}

pub fn set_source(name: &str, waveform: Pwl) {
    let mut s = state();
    s.sources.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
    s.sources.push((name.to_owned(), waveform));
}

pub fn clear_sources() {
    let mut s = state();
    s.sources.clear();
    s.missing.clear();
}

/// The printed lines since the last call, and how many were dropped.
pub fn take_log() -> (Vec<String>, usize) {
    let mut s = state();
    (std::mem::take(&mut s.log), std::mem::take(&mut s.dropped))
}

/// The sources ngspice asked for that the host does not drive.
pub fn take_missing() -> Vec<String> {
    std::mem::take(&mut state().missing)
}

pub fn exit() -> Option<i32> {
    state().exit
}
