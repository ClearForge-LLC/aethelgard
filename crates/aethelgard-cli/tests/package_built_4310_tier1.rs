//! `AET-WO-4310` (4), ruled D6: the package says what it was built at. A
//! laddered board records its rung and pour step in its title block;
//! `extract` reads them, or states `not_run` for a board that runs no
//! ladder, and refuses a record that is missing, malformed or unexpected by
//! name; `verify --against-fab` refuses a package whose FAB-NOTES states
//! neither line.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(label: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("aet-4310-built-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let last = text.lines().last().unwrap_or_default();
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_str(last).unwrap_or_else(|e| panic!("{e}: {text}")),
    )
}

/// `spec`'s emitted board, as text, in `dir`.
fn emitted(spec: &str, dir: &Path) -> (PathBuf, String) {
    let out = dir.join("out");
    let s = root().join(spec);
    let (code, v) = run(&[
        "emit",
        "--file",
        s.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{v}");
    let board = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "kicad_pcb"))
        .unwrap();
    let text = std::fs::read_to_string(&board).unwrap();
    (board, text)
}

fn extract(board: &Path, spec: &str) -> (i32, Value) {
    let s = root().join(spec);
    run(&[
        "extract",
        "--board",
        board.to_str().unwrap(),
        "--spec",
        s.to_str().unwrap(),
    ])
}

const READY: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json";
const MINIMAL: &str = "examples/sentinel-01-minimal.json";

#[test]
fn a_laddered_board_records_its_rung_and_extract_reads_it_with_the_outline() {
    let d = scratch("ready");
    let (board, text) = emitted(READY, &d);
    assert!(
        text.contains("\t(title_block\n\t\t(comment 1 \"aethelgard capacitance_ladder rung "),
        "{}",
        &text[..400]
    );
    let (code, v) = extract(&board, READY);
    assert_eq!(code, 0, "{v}");
    let l = &v["capacitance_ladder"];
    assert!(l["rung"].as_u64().is_some(), "{v}");
    // No buck: the step is `none`.
    assert!(
        l["pour_step"].is_null() && l["pour_square_permille"].is_null(),
        "{l}"
    );
    assert_eq!(v["outline_mm"], "44.10 x 48.50");

    // The record removed: refused by name.
    let start = text.find("\t(title_block\n").unwrap();
    let end = start + text[start..].find("\t)\n").unwrap() + 3;
    let bare = d.join("bare.kicad_pcb");
    std::fs::write(&bare, format!("{}{}", &text[..start], &text[end..])).unwrap();
    let (code, v) = extract(&bare, READY);
    assert_ne!(code, 0);
    assert_eq!(v["error"]["kind"], "LadderUnrecorded", "{v}");

    // A record of no rung the ladder has: malformed.
    let bad = d.join("bad.kicad_pcb");
    std::fs::write(
        &bad,
        text.replacen("capacitance_ladder rung ", "capacitance_ladder rung 9", 1),
    )
    .unwrap();
    let (_, v) = extract(&bad, READY);
    assert_eq!(v["error"]["kind"], "LadderRecordMalformed", "{v}");

    // The comment with one more element, or the title block with a second
    // child: the block holds the record and nothing else.
    let rec_end = text[start..].find("\")\n").unwrap() + start + 1;
    for (label, planted) in [
        (
            "extra",
            format!("{} \"x\"{}", &text[..rec_end], &text[rec_end..]),
        ),
        (
            "second",
            text.replacen("\t(title_block\n", "\t(title_block\n\t\t(title \"t\")\n", 1),
        ),
    ] {
        let f = d.join(format!("{label}.kicad_pcb"));
        std::fs::write(&f, planted).unwrap();
        let (_, v) = extract(&f, READY);
        assert_eq!(v["error"]["kind"], "LadderRecordMalformed", "{label}: {v}");
    }
}

#[test]
fn a_board_that_runs_no_ladder_states_not_run_and_a_record_on_it_is_refused() {
    let d = scratch("minimal");
    let (board, text) = emitted(MINIMAL, &d);
    assert!(!text.contains("title_block"));
    let (code, v) = extract(&board, MINIMAL);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["capacitance_ladder"], "not_run");
    assert_eq!(v["outline_mm"], "44.10 x 48.50");
    let planted = d.join("planted.kicad_pcb");
    std::fs::write(
        &planted,
        text.replacen(
            "\t(paper \"A4\")\n",
            "\t(paper \"A4\")\n\t(title_block\n\t\t(comment 1 \"aethelgard capacitance_ladder rung 0 pour_step none\")\n\t)\n",
            1,
        ),
    )
    .unwrap();
    let (_, v) = extract(&planted, MINIMAL);
    assert_eq!(v["error"]["kind"], "LadderUnrecorded", "{v}");
}

#[test]
fn a_package_whose_notes_state_neither_built_line_is_refused_by_against_fab() {
    // A copy of a package written before (4): its FAB-NOTES names neither.
    let d = scratch("pkg");
    let pkg = d.join("package");
    std::fs::create_dir_all(&pkg).unwrap();
    let held = root().join("fab/sentinel-01-bench/package");
    for e in std::fs::read_dir(&held).unwrap() {
        let p = e.unwrap().path();
        std::fs::copy(&p, pkg.join(p.file_name().unwrap())).unwrap();
    }
    let spec = root().join("examples/sentinel-01.json");
    let scenario = root().join("scenarios/sentinel-01-bench/pullup-2k2.json");
    let (code, v) = run(&[
        "verify",
        "--spec",
        spec.to_str().unwrap(),
        "--scenarios",
        scenario.to_str().unwrap(),
        "--against-fab",
        pkg.to_str().unwrap(),
    ]);
    assert_ne!(code, 0);
    assert_eq!(v["error"]["kind"], "PackageBuiltUnstated", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("`- Outline as built:`") && m.contains("`- Capacitance ladder:`"),
        "{m}"
    );
}
