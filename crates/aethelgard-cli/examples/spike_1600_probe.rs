//! AET-WO-1600 spike: the in-sandbox probe, `spike/1600/probe.rs`. The build
//! deletes this file with the rest of the spike.

#![forbid(unsafe_code)]

#[path = "../../../spike/1600/probe.rs"]
mod probe;

fn main() {
    probe::main();
}
