//! `board::prepare_read` holds a long name once, whatever the number of
//! children scanned inside it (`AET-WO-1322` (2); `1311` adversarial C: a
//! copy of the carrying footprint's name per child took 3.0 GB for a 10 kB
//! reference over 300 000 children, and a 100 kB one was killed at 24 GB).
//!
//! A refusal writes at most about 230 bytes of each name (`strict::shown`),
//! so what a copy per child would grow is the carrying chain: here 10
//! footprints nested, each named by a 10 kB reference, about 2.3 kB of
//! chain under each of 15 000 pads (35 MB), and before the bound 100 kB
//! (1.5 GB). It is sized so that a regression costs about that, no more.
//!
//! The test is alone in its binary, so the process's peak resident memory
//! (`VmHWM`, Linux) is this test's own.

use aethelgard_eda::{board, rules::SETUP};

/// The process's peak resident memory, in KiB.
fn peak_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    let line = status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .expect("VmHWM in /proc/self/status");
    line.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .expect("VmHWM's value in kB")
}

#[test]
fn a_10_kb_reference_over_15_000_pads_stays_bounded() {
    const PADS: usize = 15_000;
    const NESTED: usize = 10;
    let reference = "R".repeat(10_000);
    let mut text = "(kicad_pcb".to_owned();
    for _ in 0..NESTED {
        text.push_str(&format!(
            " (footprint \"X\" (property \"Reference\" \"{reference}\")"
        ));
    }
    for _ in 0..PADS {
        text.push_str(" (pad \"1\" smd rect)");
    }
    text.push_str(&")".repeat(NESTED + 1));
    text.push('\n');
    let tree = board::read(&text).unwrap();
    let before = peak_kib();
    let out = board::prepare_read(&tree, &text, SETUP).unwrap();
    let grown = peak_kib().saturating_sub(before);
    assert!(out.contains(SETUP));
    // Holding each label once leaves the stack's entries and the output,
    // under 1 MB.
    assert!(
        grown < 16 * 1024,
        "prepare_read grew the peak by {grown} KiB over {before} KiB"
    );
}
