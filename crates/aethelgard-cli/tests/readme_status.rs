//! Review B8 / A6, and the Grok review of 2026-11-16 (row 17, `AET-WO-1607`
//! (4)): the README's status line states each phase of `docs/roadmap.md` §5
//! as that phase stands. A phase whose section holds `**Exit gate: passed`
//! is named complete; every other phase is named, and never as complete or
//! passed; and the line names no phase §5 does not have. Both documents are
//! protected; this test only reads them.

use std::collections::BTreeMap;
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

/// Every phase of §5, by its `### P<n> —` heading, with whether its section
/// holds an `**Exit gate: passed`.
fn phases(roadmap: &str) -> BTreeMap<u32, bool> {
    let start = roadmap.find("\n## 5. ").expect("roadmap §5");
    let end = roadmap[start..].find("\n## 6. ").expect("roadmap §6") + start;
    let mut phases = BTreeMap::new();
    let mut current = None;
    for line in roadmap[start..end].lines() {
        if let Some(rest) = line.strip_prefix("### P") {
            let n = number(rest).expect("phase number");
            phases.insert(n, false);
            current = Some(n);
        } else if line.starts_with("**Exit gate: passed")
            && let Some(n) = current
        {
            phases.insert(n, true);
        }
    }
    phases
}

/// The leading digits of `s`, if any.
fn number(s: &str) -> Option<u32> {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The README's status line, from `**Status:**` to its first sentence that
/// names no phase: "phases P0 to P2 are complete. P3's work has merged, and
/// its exit gate waits on …. P4's … work is under way." Each sentence is
/// returned with the phases it names; a range `Pa to Pb` names every phase
/// between.
fn stated(readme: &str) -> Vec<(String, Vec<u32>)> {
    let line = readme
        .lines()
        .find(|l| l.starts_with("**Status:**"))
        .expect("a **Status:** line");
    let mut out = Vec::new();
    for sentence in line["**Status:**".len()..].split(". ") {
        let mut named = Vec::new();
        let mut rest = sentence;
        while let Some(at) = rest.find('P') {
            let tail = &rest[at + 1..];
            if let Some(n) = number(tail) {
                // A range: `Pa to Pb` names a..=b.
                let after = &tail[n.to_string().len()..];
                match after.strip_prefix(" to P").and_then(number) {
                    Some(m) => named.extend(n..=m),
                    None => named.push(n),
                }
            }
            rest = tail;
        }
        named.dedup();
        if named.is_empty() {
            break;
        }
        out.push((sentence.to_owned(), named));
    }
    out
}

/// The first way the status line disagrees with §5, if any.
fn disagreement(readme: &str, roadmap: &str) -> Option<String> {
    let phases = phases(roadmap);
    let stated = stated(readme);
    let says_done = |s: &str| s.contains("complete") || s.contains("passed");
    for (sentence, named) in &stated {
        if let Some(n) = named.iter().find(|n| !phases.contains_key(n)) {
            return Some(format!("names P{n}, which §5 does not have: {sentence:?}"));
        }
    }
    for (&n, &passed) in &phases {
        let about: Vec<&String> = stated
            .iter()
            .filter(|(_, named)| named.contains(&n))
            .map(|(s, _)| s)
            .collect();
        if about.is_empty() {
            return Some(format!("does not state P{n}"));
        }
        let done = about.iter().any(|s| says_done(s));
        if passed && !about.iter().any(|s| s.contains("complete")) {
            return Some(format!(
                "P{n}'s exit gate passed, but no sentence calls it complete: {about:?}"
            ));
        }
        if !passed && done {
            return Some(format!(
                "P{n}'s exit gate is open, but the line calls it complete or passed: {about:?}"
            ));
        }
    }
    None
}

#[test]
fn the_readme_states_every_phase_as_the_roadmap_does() {
    let roadmap = read("docs/roadmap.md");
    let phases = phases(&roadmap);
    assert!(
        phases.values().any(|&p| p),
        "the roadmap marks no phase passed"
    );
    assert!(
        phases.values().any(|&p| !p),
        "the roadmap marks every phase passed"
    );
    assert_eq!(
        disagreement(&read("README.md"), &roadmap),
        None,
        "README status line and docs/roadmap.md §5 disagree"
    );
}

/// Each way a status line can go wrong, on today's documents edited.
#[test]
fn a_stale_status_line_or_an_unmarked_phase_is_caught() {
    let roadmap = read("docs/roadmap.md");
    let readme = read("README.md");
    let edit = |text: &str, from: &str, to: &str| {
        let edited = text.replacen(from, to, 1);
        assert_ne!(edited, text, "the document no longer holds {from:?}");
        edited
    };
    let caught = |readme: &str, roadmap: &str, what: &str| {
        let d = disagreement(readme, roadmap);
        assert!(
            d.as_deref().is_some_and(|d| d.contains(what)),
            "{what}: {d:?}"
        );
    };
    // The review's finding: a README two phases behind.
    let stale = edit(
        &readme,
        "phases P0 to P2 are complete",
        "phase P1 is complete",
    );
    caught(&stale, &roadmap, "does not state P0");
    // A passed phase named, but not as complete.
    let vague = edit(
        &readme,
        "phases P0 to P2 are complete",
        "phases P0 to P2 are done",
    );
    caught(&vague, &roadmap, "P0's exit gate passed");
    // A roadmap whose P2 gate is not marked passed.
    let unmarked = edit(
        &roadmap,
        "**Exit gate: passed 2026-11-09",
        "**Exit gate: open",
    );
    caught(&readme, &unmarked, "P1's exit gate is open");
    // An open phase called complete, or passed.
    for to in ["P3 is complete", "P3's exit gate passed"] {
        let early = edit(&readme, "P3's work has merged, and its exit gate waits", to);
        caught(&early, &roadmap, "P3's exit gate is open");
    }
    // An open phase the line does not state at all.
    let silent = edit(&readme, " P4's flight-controller work is under way.", "");
    caught(&silent, &roadmap, "does not state P4");
    // A phase §5 does not have.
    let unknown = edit(
        &readme,
        "P4's flight-controller work is under way.",
        "P4's flight-controller work is under way. P5's work is under way.",
    );
    caught(&unknown, &roadmap, "names P5");
}
