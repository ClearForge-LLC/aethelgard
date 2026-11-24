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

/// The README's status line, after `**Status:**`, as its sentences that
/// name a phase: "phases P0 to P2 are complete. P3's work has merged, and
/// its exit gate waits on …. P4's … work is under way." Each sentence comes
/// with the phases it names; a range `Pa to Pb` names every phase between.
/// Every sentence of the line is read, so a claim cannot hide after one that
/// names no phase.
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
        if !named.is_empty() {
            out.push((sentence.to_owned(), named));
        }
    }
    out
}

/// The sentence's words, lower-cased, so `incomplete` is not `complete`.
fn words(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// A sentence that says its phases are done: a word of done, and no word
/// that takes it back ("has not passed", "is incomplete", "waits").
fn says_done(sentence: &str) -> bool {
    const DONE: [&str; 6] = ["complete", "completed", "passed", "done", "finished", "met"];
    const OPEN: [&str; 8] = [
        "not",
        "incomplete",
        "open",
        "waits",
        "waiting",
        "pending",
        "under",
        "until",
    ];
    let w = words(sentence);
    w.iter().any(|x| DONE.contains(&x.as_str())) && !w.iter().any(|x| OPEN.contains(&x.as_str()))
}

/// The first way the status line disagrees with §5, if any: a phase §5 does
/// not have; a phase it does not state; a passed phase any sentence states
/// as other than done; or an open phase any sentence states as done.
fn disagreement(readme: &str, roadmap: &str) -> Option<String> {
    let phases = phases(roadmap);
    let stated = stated(readme);
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
        if passed && let Some(s) = about.iter().find(|s| !says_done(s)) {
            return Some(format!(
                "P{n}'s exit gate passed, but the line does not call it done: {s:?}"
            ));
        }
        if !passed && let Some(s) = about.iter().find(|s| says_done(s)) {
            return Some(format!(
                "P{n}'s exit gate is open, but the line calls it done: {s:?}"
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
    // A passed phase named, but not as done.
    let vague = edit(
        &readme,
        "phases P0 to P2 are complete",
        "phases P0 to P2 have merged",
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
    // The adversarial pass's five (AET-WO-1607 F1): a negation, other words
    // of done, a contradiction, and a claim after a sentence naming no phase.
    let negated = edit(
        &readme,
        "phases P0 to P2 are complete",
        "phases P0 to P2 are incomplete",
    );
    caught(&negated, &roadmap, "P0's exit gate passed");
    for to in ["P3 is done and its gate is met", "P3's work is finished"] {
        let early = edit(&readme, "P3's work has merged, and its exit gate waits", to);
        caught(&early, &roadmap, "P3's exit gate is open");
    }
    let finished = edit(
        &readme,
        "P4's flight-controller work is under way",
        "P4's flight-controller work is finished",
    );
    caught(&finished, &roadmap, "P4's exit gate is open");
    let contradicted = edit(
        &readme,
        "phases P0 to P2 are complete.",
        "phases P0 to P2 are complete. P1's exit gate has not passed.",
    );
    caught(&contradicted, &roadmap, "P1's exit gate passed");
    let hidden = edit(
        &readme,
        "it is the supported interface.**",
        "it is the supported interface.** P3 is complete.",
    );
    caught(&hidden, &roadmap, "P3's exit gate is open");
    // An open phase the line does not state at all.
    let silent = edit(&readme, " P4's flight-controller work is under way.", "");
    caught(&silent, &roadmap, "does not state P4");
    // A phase §5 does not have.
    let unknown = edit(
        &readme,
        "P4's flight-controller work is under way.",
        "P4's flight-controller work is under way. P6's work is under way.",
    );
    caught(&unknown, &roadmap, "names P6");
}
