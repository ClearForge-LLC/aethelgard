//! `AET-WO-1917` (`4408` Q4; ruling D5): `container/ardupilot-pinout.sh`
//! judges the boards the workflow's own matrices name, not a list it keeps.
//! A `generated` row is judged unless it has a `drop:` (a mutant); an
//! `ardupilot` board must be one of the hand boards the judge has findings
//! for. Before any board is judged, the boards directory must hold exactly
//! those: a matrix board with no directory, a directory no matrix names, and
//! an empty matrix are each refused by name. The set check runs here with no
//! `aethelgard` and no `jq`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const WORKFLOW: &str = ".github/workflows/ardupilot-image.yml";
const JUDGE: &str = "container/ardupilot-pinout.sh";

/// `bash ardupilot-pinout.sh ARGS…`: (exit, stdout, stderr).
fn judge(args: &[&str]) -> (i32, String, String) {
    let out = Command::new("bash")
        .arg(root().join(JUDGE))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-1917-pinout-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// A boards directory holding one directory per name.
fn boards(dir: &Path, names: &[&str]) -> String {
    let b = dir.join("boards");
    for n in names {
        fs::create_dir_all(b.join(n)).unwrap();
    }
    fs::create_dir_all(&b).unwrap();
    b.to_str().unwrap().to_owned()
}

const HAND: [&str; 4] = ["step-g", "seed-pin", "seed-af", "seed-pkg"];
const GENERATED: [&str; 5] = [
    "sentinel-01",
    "sentinel-02",
    "serial-dshot-9600",
    "four-port",
    "sentinel-01-full",
];

#[test]
fn the_judge_reads_the_workflows_matrices() {
    let wf = root().join(WORKFLOW);
    let wf = wf.to_str().unwrap();
    let (code, out, err) = judge(&["hand", wf]);
    assert_eq!(
        (code, out.lines().collect::<Vec<_>>()),
        (0, HAND.to_vec()),
        "{err}"
    );
    // sentinel-02-noimu, the mutant (`drop: IMU`), is out of reach.
    let (code, out, err) = judge(&["generated", wf]);
    assert_eq!(
        (code, out.lines().collect::<Vec<_>>()),
        (0, GENERATED.to_vec()),
        "{err}"
    );
    let d = scratch("real");
    let all: Vec<&str> = HAND.iter().chain(GENERATED.iter()).copied().collect();
    let (code, out, err) = judge(&["boards", wf, &boards(&d, &all)]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("judged set: 9 boards"), "{out}");
    fs::remove_dir_all(&d).unwrap();
}

/// The workflow with one more `generated` row, planted after `four-port`.
fn with_row(row: &str) -> String {
    let wf = fs::read_to_string(root().join(WORKFLOW)).unwrap();
    let at = "          - board: four-port\n            spec: crates/aethelgard-cli/tests/fixtures/four-port.json\n";
    assert_eq!(wf.matches(at).count(), 1);
    wf.replace(at, &format!("{at}{row}"))
}

fn refused(name: &str, workflow: &str, dirs: &[&str], want: &str) {
    let d = scratch(name);
    let wf = d.join("workflow.yml");
    fs::write(&wf, workflow).unwrap();
    let (code, out, err) = judge(&["boards", wf.to_str().unwrap(), &boards(&d, dirs)]);
    assert_eq!(code, 1, "{name}: {out}{err}");
    assert!(err.contains(want), "{name}: want `{want}` in: {err}");
    // The main mode refuses before judging: BIN is never run.
    let (code, _, err) = judge(&[
        "/nonexistent-bin",
        "t",
        &boards(&d, dirs),
        "a",
        wf.to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "{name}");
    assert!(err.contains(want), "{name}: main: {err}");
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn a_planted_matrix_row_with_no_board_is_refused_by_name() {
    let all: Vec<&str> = HAND.iter().chain(GENERATED.iter()).copied().collect();
    refused(
        "row",
        &with_row("          - board: ghost\n            spec: examples/sentinel-01.json\n"),
        &all,
        "in the workflow's matrix, but no board reached the judge: ghost",
    );
    // A planted mutant row is not expected.
    let d = scratch("mutant");
    let wf = d.join("workflow.yml");
    fs::write(
        &wf,
        with_row("          - board: ghost\n            spec: x.json\n            drop: IMU\n"),
    )
    .unwrap();
    let (code, _, err) = judge(&["boards", wf.to_str().unwrap(), &boards(&d, &all)]);
    assert_eq!(code, 0, "{err}");
    fs::remove_dir_all(&d).unwrap();
}

#[test]
fn a_board_no_matrix_names_is_refused_by_name() {
    let mut all: Vec<&str> = HAND.iter().chain(GENERATED.iter()).copied().collect();
    all.push("stray");
    let wf = fs::read_to_string(root().join(WORKFLOW)).unwrap();
    refused(
        "stray",
        &wf,
        &all,
        "a board no matrix of the workflow names: stray",
    );
}

#[test]
fn a_hand_board_the_judge_has_no_findings_for_is_refused_by_name() {
    let wf = fs::read_to_string(root().join(WORKFLOW)).unwrap().replace(
        "board: [step-g, seed-pin, seed-af, seed-pkg]",
        "board: [step-g, seed-pin, seed-af, seed-pkg, seed-new]",
    );
    assert!(wf.contains("seed-new"));
    let all: Vec<&str> = HAND.iter().chain(GENERATED.iter()).copied().collect();
    refused(
        "hand",
        &wf,
        &all,
        "seed-new: in the ardupilot matrix, but this judge has no expected findings for it",
    );
}

#[test]
fn an_empty_generated_matrix_is_refused() {
    let wf = fs::read_to_string(root().join(WORKFLOW)).unwrap();
    let start = wf.find("        include:\n").unwrap();
    let end = start + wf[start..].find("    steps:\n").unwrap();
    let empty = format!("{}        include: []\n{}", &wf[..start], &wf[end..]);
    refused(
        "empty",
        &empty,
        &HAND,
        "the generated job's matrix names no board",
    );
}
