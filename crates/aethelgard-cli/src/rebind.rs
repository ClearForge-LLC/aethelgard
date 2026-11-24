//! `verify --fab-out OUT --rebind COMMITTED` (`AET-WO-4300k`, ruled
//! 2026-11-24T11:25Z): a committed package re-bound to its specification
//! after a change that moves no hardware byte (a specification version
//! literal). The readiness gate is not judged. Instead, the package derived as
//! `--fab-out` derives it must equal `COMMITTED` byte for byte, but for
//! what binds it: `MANIFEST.json`'s specification, topology, verdict and
//! scenario hashes and its `FAB-NOTES.md` hash, and `FAB-NOTES.md`'s
//! specification sha256 line. Any other moved byte (a Gerber, the BOM, the
//! ELF, a precondition row) refuses it and nothing is written. A new board has
//! no committed package, so it cannot re-bind.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::run::Stop;

/// `COMMITTED` holds no committed package.
pub const NO_PACKAGE: &str = "RebindNoPackage";
/// `COMMITTED`'s package is another board's.
pub const OTHER_BOARD: &str = "RebindOtherBoard";
/// The derived package moves more than its binding.
pub const MOVED: &str = "RebindMoved";

const MANIFEST: &str = "MANIFEST.json";
const NOTES: &str = "FAB-NOTES.md";
/// The `MANIFEST.json` keys a re-bind may move.
const BINDING: [&str; 3] = ["specification_sha256", "topology_sha256", "verdict_sha256"];
/// The line before `FAB-NOTES.md`'s specification sha256.
const NOTES_LEAD: &str = "from the specification whose sha256 is";

fn manifest_of(dir: &Path) -> Result<Value, Stop> {
    let path = dir.join(MANIFEST);
    let none = |why: String| {
        Stop::refused(
            NO_PACKAGE,
            format!(
                "no package: {} holds no committed package to re-bind ({why})",
                dir.display()
            ),
        )
    };
    let bytes = std::fs::read(&path).map_err(|e| none(format!("{}: {e}", path.display())))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|e| none(format!("{} is not JSON: {e}", path.display())))?;
    if !v["project"].is_string() {
        return Err(none(format!("{} names no project", path.display())));
    }
    Ok(v)
}

/// Before any gate: `dir` holds a committed package, and it is `project`'s.
pub fn committed(dir: &Path, project: &str) -> Result<(), Stop> {
    crate::input::dir_check(dir)?;
    let m = manifest_of(dir)?;
    match m["project"].as_str() {
        Some(p) if p == project => Ok(()),
        p => Err(Stop::refused(
            OTHER_BOARD,
            format!(
                "no package: {} is the package of `{}`, not of `{project}`: a re-bind keeps its board",
                dir.display(),
                p.unwrap_or_default()
            ),
        )),
    }
}

/// After every gate: `derived` (every file, `MANIFEST.json` included) equals
/// the package in `dir` but for its binding, or the re-bind is refused,
/// naming each moved file or key.
pub fn matches(dir: &Path, derived: &BTreeMap<String, Vec<u8>>) -> Result<(), Stop> {
    let mut held = BTreeMap::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Stop::refused(NO_PACKAGE, format!("cannot read {}: {e}", dir.display())))?;
    for e in entries {
        let e = e.map_err(|e| {
            Stop::refused(NO_PACKAGE, format!("cannot read {}: {e}", dir.display()))
        })?;
        let name = e.file_name().to_string_lossy().into_owned();
        let bytes = std::fs::read(e.path()).map_err(|err| {
            Stop::refused(
                NO_PACKAGE,
                format!("cannot read {}: {err}", e.path().display()),
            )
        })?;
        held.insert(name, bytes);
    }
    let moved = differences(&held, derived);
    if moved.is_empty() {
        return Ok(());
    }
    Err(Stop::refused(
        MOVED,
        format!(
            "no package: re-binding {} would move more than its binding: {}; a re-bind moves only MANIFEST.json's {}, scenario and FAB-NOTES.md hashes and FAB-NOTES.md's specification line",
            dir.display(),
            moved.join("; "),
            BINDING.join(", ")
        ),
    ))
}

/// Every file or key of `derived` that differs from `held` beyond the binding.
fn differences(
    held: &BTreeMap<String, Vec<u8>>,
    derived: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let names: BTreeSet<&String> = held.keys().chain(derived.keys()).collect();
    let mut moved = Vec::new();
    for name in names {
        match (held.get(name), derived.get(name)) {
            (Some(_), None) => moved.push(format!("{name} is not derived")),
            (None, Some(_)) => moved.push(format!("{name} is not in the committed package")),
            (Some(a), Some(b)) if name == MANIFEST => moved.extend(manifest_moved(a, b)),
            (Some(a), Some(b)) if name == NOTES => {
                if !notes_bound_alike(a, b) {
                    moved.push(format!("{NOTES} differs beyond its specification line"));
                }
            }
            (Some(a), Some(b)) if a != b => moved.push(format!("{name} differs")),
            _ => {}
        }
    }
    moved
}

/// The MANIFEST keys that differ, once its binding is taken from `held`.
fn manifest_moved(held: &[u8], derived: &[u8]) -> Vec<String> {
    let (Ok(h), Ok(mut d)) = (
        serde_json::from_slice::<Value>(held),
        serde_json::from_slice::<Value>(derived),
    ) else {
        return vec![format!("{MANIFEST} is not JSON")];
    };
    for key in BINDING {
        if let (Some(obj), Some(v)) = (d.as_object_mut(), h.get(key)) {
            if obj.contains_key(key) {
                obj.insert(key.to_owned(), v.clone());
            }
        }
    }
    // A scenario keeps its name and place; only its hash may move.
    if let (Some(ds), Some(hs)) = (d["scenarios"].as_array_mut(), h["scenarios"].as_array()) {
        if ds.len() == hs.len() {
            for (x, y) in ds.iter_mut().zip(hs) {
                if x.get("scenario") == y.get("scenario") && x.get("sha256").is_some() {
                    x["sha256"] = y["sha256"].clone();
                }
            }
        }
    }
    if let (Some(df), Some(hf)) = (d["files"].as_object_mut(), h["files"].as_object()) {
        if let (true, Some(v)) = (df.contains_key(NOTES), hf.get(NOTES)) {
            df.insert(NOTES.to_owned(), v.clone());
        }
    }
    let (Some(h), Some(d)) = (h.as_object(), d.as_object()) else {
        return vec![format!("{MANIFEST} is not an object")];
    };
    let keys: BTreeSet<&String> = h.keys().chain(d.keys()).collect();
    keys.into_iter()
        .filter(|k| h.get(*k) != d.get(*k))
        .map(|k| format!("{MANIFEST} `{k}` differs"))
        .collect()
}

/// `FAB-NOTES.md` alike but for the 64-hex line after [`NOTES_LEAD`].
fn notes_bound_alike(held: &[u8], derived: &[u8]) -> bool {
    let (Ok(h), Ok(d)) = (std::str::from_utf8(held), std::str::from_utf8(derived)) else {
        return false;
    };
    let (h, d): (Vec<&str>, Vec<&str>) = (h.split('\n').collect(), d.split('\n').collect());
    h.len() == d.len()
        && h.iter().zip(&d).enumerate().all(|(i, (a, b))| {
            a == b
                || (i > 0
                    && h[i - 1] == d[i - 1]
                    && h[i - 1].trim_end().ends_with(NOTES_LEAD)
                    && sha_line(a)
                    && sha_line(b))
        })
}

/// "`<64 lowercase hex>`." and nothing else.
fn sha_line(line: &str) -> bool {
    line.strip_prefix('`')
        .and_then(|l| l.strip_suffix("`."))
        .is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(notes: &str, manifest: Value, gerber: &str) -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (NOTES.to_owned(), notes.as_bytes().to_vec()),
            (MANIFEST.to_owned(), serde_json::to_vec(&manifest).unwrap()),
            ("b-F_Cu.gtl".to_owned(), gerber.as_bytes().to_vec()),
        ])
    }

    fn manifest(spec: char, scenario: char, notes: char, row: &str, gerber: char) -> Value {
        let h = |c: char| c.to_string().repeat(64);
        serde_json::json!({
            "project": "b",
            "specification_sha256": h(spec), "topology_sha256": h(spec), "verdict_sha256": h(spec),
            "scenarios": [{"scenario": "s", "sha256": h(scenario)}],
            "preconditions": [{"precondition": row, "state": "met"}],
            "files": {NOTES: h(notes), "b-F_Cu.gtl": h(gerber)},
        })
    }

    fn notes(spec: char) -> String {
        format!(
            "x\nGenerated by aethelgard 0.2.0 {NOTES_LEAD}\n`{}`.\nend\n",
            spec.to_string().repeat(64)
        )
    }

    #[test]
    fn only_the_binding_may_move() {
        let held = pkg(&notes('a'), manifest('a', 'a', 'a', "met", 'c'), "G");
        assert!(differences(&held, &held).is_empty());
        let rebound = pkg(&notes('b'), manifest('b', 'b', 'b', "met", 'c'), "G");
        assert_eq!(differences(&held, &rebound), Vec::<String>::new());
        // A Gerber, its MANIFEST hash, a precondition row, a scenario's name,
        // FAB-NOTES beyond its line: each moved byte is named.
        let gerber = pkg(&notes('b'), manifest('b', 'b', 'b', "met", 'd'), "H");
        assert_eq!(
            differences(&held, &gerber),
            ["MANIFEST.json `files` differs", "b-F_Cu.gtl differs"]
        );
        let row = pkg(&notes('b'), manifest('b', 'b', 'b', "other", 'c'), "G");
        assert_eq!(
            differences(&held, &row),
            ["MANIFEST.json `preconditions` differs"]
        );
        let mut renamed = manifest('b', 'b', 'b', "met", 'c');
        renamed["scenarios"][0]["scenario"] = "t".into();
        let renamed = pkg(&notes('b'), renamed, "G");
        assert_eq!(
            differences(&held, &renamed),
            ["MANIFEST.json `scenarios` differs"]
        );
        let text = pkg(
            &notes('b').replace("end", "End"),
            manifest('b', 'b', 'b', "met", 'c'),
            "G",
        );
        assert_eq!(
            differences(&held, &text),
            ["FAB-NOTES.md differs beyond its specification line"]
        );
        let version = pkg(
            &notes('b').replace("0.2.0", "0.2.1"),
            manifest('b', 'b', 'b', "met", 'c'),
            "G",
        );
        assert_eq!(
            differences(&held, &version),
            ["FAB-NOTES.md differs beyond its specification line"]
        );
        let mut extra = rebound.clone();
        extra.insert("x.drl".to_owned(), vec![]);
        assert_eq!(
            differences(&held, &extra),
            ["x.drl is not in the committed package"]
        );
        let mut short = rebound;
        short.remove("b-F_Cu.gtl");
        assert!(differences(&held, &short).contains(&"b-F_Cu.gtl is not derived".to_owned()));
    }

    #[test]
    fn the_specification_line_is_a_sha_and_nothing_else() {
        assert!(sha_line(&format!("`{}`.", "0".repeat(64))));
        for bad in [
            format!("`{}`.", "0".repeat(63)),
            format!("`{}`.", "G".repeat(64)),
            format!("{}", "0".repeat(64)),
        ] {
            assert!(!sha_line(&bad), "{bad}");
        }
    }
}
