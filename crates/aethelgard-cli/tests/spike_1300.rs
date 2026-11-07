//! AET-WO-1300 spike hook (deleted by the build half): runs the probes in
//! `spike/1300/` inside CI's `tools` job, where KiCad exists. Each answer is
//! an `ANSWER` line on stderr; this test fails only on a harness error, never
//! on what KiCad measured.

#![cfg(feature = "with-tools")]

#[path = "../../../spike/1300/mod.rs"]
mod probe;

use std::path::Path;

#[test]
fn spike_1300_answers() {
    let errors = probe::main(
        Path::new(env!("CARGO_BIN_EXE_aethelgard")),
        Path::new(env!("CARGO_TARGET_TMPDIR")),
    );
    assert!(errors.is_empty(), "harness errors: {errors:?}");
}
