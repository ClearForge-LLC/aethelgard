//! `board::prepare_read` holds a long name once, whatever the number of
//! children scanned inside it (`AET-WO-1322` (2); `1311` adversarial C: a
//! copy of the carrying footprint's name per child took 3.0 GB for a 10 kB
//! reference over 300 000 children, and a 100 kB one was killed at 24 GB).
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
fn a_10_kb_reference_over_100_000_pads_stays_bounded() {
    const PADS: usize = 100_000;
    let reference = "R".repeat(10_000);
    let mut text = format!("(kicad_pcb (footprint \"X\" (property \"Reference\" \"{reference}\")");
    for _ in 0..PADS {
        text.push_str(" (pad \"1\" smd rect)");
    }
    text.push_str("))\n");
    let tree = board::read(&text).unwrap();
    let before = peak_kib();
    let out = board::prepare_read(&tree, &text, SETUP).unwrap();
    let grown = peak_kib().saturating_sub(before);
    assert!(out.contains(SETUP));
    // A copy per pad is 100 000 x 10 kB, about 1 GB; holding the name once
    // leaves the stack's entries and the output, a few MB.
    assert!(
        grown < 64 * 1024,
        "prepare_read grew the peak by {grown} KiB over {before} KiB"
    );
}
