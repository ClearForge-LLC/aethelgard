//! `AET-WO-1917` (review 3, A16; ruling D4): `ci/shard.sh`'s `TIER2_RE`, the
//! tier-2 binaries whose passes count toward the floor, is checked, not
//! trusted. A with-tools file is any `tests/*.rs` of a crate or the bridge
//! whose source carries a `feature = "with-tools"` cfg, on the file or on an
//! item (`attribute_seeded.rs` gates a module). Each must be named in
//! `TIER2_RE`, and each name in it must be such a file: either miss fails by
//! name.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The binary names of `TIER2_RE='^tests/(a|b|…)[.]rs$'`, the one assignment
/// in `shard`. Any other form panics rather than being misread.
fn tier2_names(shard: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = shard
        .lines()
        .filter(|l| l.starts_with("TIER2_RE="))
        .collect();
    assert_eq!(lines.len(), 1, "want one TIER2_RE= line: {lines:?}");
    let body = lines[0]
        .strip_prefix("TIER2_RE='^tests/(")
        .and_then(|s| s.strip_suffix(")[.]rs$'"))
        .unwrap_or_else(|| panic!("TIER2_RE is not `^tests/(a|b|…)[.]rs$`: {}", lines[0]));
    let names: BTreeSet<String> = body.split('|').map(str::to_owned).collect();
    for n in &names {
        assert!(
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "TIER2_RE name `{n}` is not a plain binary name"
        );
    }
    names
}

/// Whether a source carries a `feature = "with-tools"` cfg outside `//`
/// comments, spaces ignored.
fn with_tools(source: &str) -> bool {
    source.lines().any(|l| {
        let code = l.split("//").next().unwrap_or("");
        let squeezed: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        squeezed.contains("feature=\"with-tools\"")
    })
}

/// `(binary name, source)` of every `tests/*.rs` of every crate and the bridge.
fn test_files(root: &Path) -> Vec<(String, String)> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .unwrap()
        .map(|e| e.unwrap().path().join("tests"))
        .collect();
    dirs.push(root.join("bridge/tests"));
    let mut out = Vec::new();
    for d in dirs.into_iter().filter(|d| d.is_dir()) {
        for e in fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rs") {
                let name = p.file_stem().unwrap().to_str().unwrap().to_owned();
                out.push((name, fs::read_to_string(&p).unwrap()));
            }
        }
    }
    out
}

/// The misses, by name: a with-tools file `TIER2_RE` does not name, and a
/// name with no with-tools file.
fn misses(names: &BTreeSet<String>, files: &[(String, String)]) -> Vec<String> {
    let gated: BTreeSet<&str> = files
        .iter()
        .filter(|(_, src)| with_tools(src))
        .map(|(n, _)| n.as_str())
        .collect();
    let mut out: Vec<String> = gated
        .iter()
        .filter(|n| !names.contains(**n))
        .map(|n| format!("tests/{n}.rs is built under with-tools but TIER2_RE does not name it"))
        .collect();
    out.extend(
        names
            .iter()
            .filter(|n| !gated.contains(n.as_str()))
            .map(|n| format!("TIER2_RE names {n}, which is no with-tools test file")),
    );
    out
}

#[test]
fn tier2_re_names_exactly_the_with_tools_test_files() {
    let shard = fs::read_to_string(root().join("ci/shard.sh")).unwrap();
    let names = tier2_names(&shard);
    let files = test_files(&root());
    // Not vacuous: twelve today (1917 §1.1), file-level and item-level both.
    assert!(names.len() >= 12, "{names:?}");
    assert!(names.contains("with_tools") && names.contains("attribute_seeded"));
    assert_eq!(misses(&names, &files), Vec::<String>::new());
}

#[test]
fn a_planted_miss_either_way_is_named() {
    let names = tier2_names("x\nTIER2_RE='^tests/(a_tier2|gone)[.]rs$'\ny\n");
    let files = [
        ("a_tier2", "#![cfg(feature = \"with-tools\")]\n"),
        ("new_tools", "#[cfg(feature=\"with-tools\")]\nmod tier2 {}\n"),
        ("commented", "// #![cfg(feature = \"with-tools\")]\n"),
        ("plain", "fn header_with_tools() {}\n"),
    ]
    .map(|(n, s)| (n.to_owned(), s.to_owned()));
    assert_eq!(
        misses(&names, &files),
        [
            "tests/new_tools.rs is built under with-tools but TIER2_RE does not name it",
            "TIER2_RE names gone, which is no with-tools test file",
        ]
    );
}

#[test]
fn a_tier2_re_of_another_form_is_refused() {
    for bad in [
        "TIER2_RE='^tests/(a|b)\\.rs$'\n",
        "TIER2_RE='^tests/(a|b)[.]rs$'\nTIER2_RE='^tests/(c)[.]rs$'\n",
        "TIER2_RE='^tests/(a|.*)[.]rs$'\n",
        "",
    ] {
        let r = std::panic::catch_unwind(|| tier2_names(bad));
        assert!(r.is_err(), "accepted: {bad}");
    }
}
