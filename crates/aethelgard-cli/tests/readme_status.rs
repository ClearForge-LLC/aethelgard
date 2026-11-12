//! Review B8 / A6: the README's status line names, as complete, exactly the
//! phases `docs/roadmap.md` §5 marks "Exit gate: passed". Both documents are
//! protected; this test only reads them.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn read(rel: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel),
    )
    .unwrap()
}

/// The phases whose `### P<n> —` section in §5 holds an `**Exit gate: passed`.
fn passed_phases(roadmap: &str) -> BTreeSet<u32> {
    let start = roadmap.find("\n## 5. ").expect("roadmap §5");
    let end = roadmap[start..].find("\n## 6. ").expect("roadmap §6") + start;
    let mut passed = BTreeSet::new();
    let mut current = None;
    for line in roadmap[start..end].lines() {
        if let Some(rest) = line.strip_prefix("### P") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            current = Some(digits.parse::<u32>().expect("phase number"));
        } else if line.starts_with("**Exit gate: passed")
            && let Some(n) = current
        {
            passed.insert(n);
        }
    }
    passed
}

/// The phases the README's status line calls complete: "phases P0 to P2 are
/// complete" or "phase P1 is complete".
fn claimed_phases(readme: &str) -> BTreeSet<u32> {
    let line = readme
        .lines()
        .find(|l| l.starts_with("**Status:**"))
        .expect("a **Status:** line");
    let number = |s: &str| -> u32 {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().expect("phase number")
    };
    if let Some(at) = line.find("phases P") {
        let rest = &line[at + "phases P".len()..];
        let (from, tail) = rest.split_once(" to P").expect("`Pa to Pb`");
        assert!(
            tail.contains(" are complete"),
            "status does not say `are complete`: {line}"
        );
        (number(from)..=number(tail)).collect()
    } else if let Some(at) = line.find("phase P") {
        let rest = &line[at + "phase P".len()..];
        assert!(rest.contains(" is complete"), "{line}");
        BTreeSet::from([number(rest)])
    } else {
        panic!("the status line names no phase: {line}");
    }
}

#[test]
fn the_readme_status_names_exactly_the_phases_the_roadmap_passed() {
    let roadmap = passed_phases(&read("docs/roadmap.md"));
    assert!(!roadmap.is_empty(), "the roadmap marks no phase passed");
    assert_eq!(
        claimed_phases(&read("README.md")),
        roadmap,
        "README status line and docs/roadmap.md §5 disagree"
    );
}

#[test]
fn a_stale_status_line_or_an_unmarked_phase_is_caught() {
    let roadmap = read("docs/roadmap.md");
    let readme = read("README.md");
    // The review's finding: a README two phases behind.
    let stale = readme.replacen("phases P0 to P2 are complete", "phase P1 is complete", 1);
    assert_ne!(
        stale, readme,
        "the README no longer says `phases P0 to P2 are complete`"
    );
    assert_ne!(claimed_phases(&stale), passed_phases(&roadmap));
    // A roadmap whose P2 gate is not marked passed.
    let unmarked = roadmap.replacen("**Exit gate: passed 2026-11-09", "**Exit gate: open", 1);
    assert_ne!(passed_phases(&unmarked), passed_phases(&roadmap));
}
