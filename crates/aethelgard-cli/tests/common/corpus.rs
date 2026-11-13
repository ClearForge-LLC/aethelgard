//! The strict reader's corpus (`AET-WO-1306` §1.4): one edit file per
//! board or schematic under `tests/fixtures/strict/<kind>/<class>/`, each an
//! exact find-and-replace on the generated reference, with the verdict main
//! gave it and the verdict each gate must give it now.
//!
//! An edit file reads:
//!
//! ```text
//! # class: repeated-child
//! # note: what KiCad reads, and where that is established
//! # main: drc 0 pass; extract 3 UnmodelledCopper
//! # drc: 3 MalformedBoard kicad_pcb/general[1]
//! # extract: 3 MalformedBoard kicad_pcb/general[1]
//! @@ find
//! <lines that occur exactly once in the reference>
//! @@ replace
//! <their replacement>
//! @@ end
//! ```
//!
//! A gate line is `<exit> <verdict or refusal kind> [<id>]`. The gates are
//! `drc` (`check drc`), `drc-parity` (`check drc --spec --schematic`, as
//! `verify` runs it), `extract`, and `erc` (`check erc`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

pub const SPEC: &str = "examples/sentinel-01-minimal.json";

/// The classes a corpus file may name, by kind.
pub const BOARD_CLASSES: &[&str] = &[
    "repeated-child",
    "escape",
    "field-twice",
    "keyword",
    "unread-child",
    "unmodelled-shape",
    "unmodelled-item",
    "parts",
    "connectivity",
    "coordinate",
    "control",
];
pub const SCHEMATIC_CLASSES: &[&str] = &[
    "repeated-child",
    "field-twice",
    "field-spelling",
    "escape",
    "private-flag",
    "control",
];

/// One gate's expected outcome.
#[derive(Clone, Debug)]
pub struct Expect {
    pub gate: String,
    pub code: i32,
    /// `pass`, `fail`, or a refusal's kind.
    pub what: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Case {
    pub board: bool,
    pub class: String,
    pub name: String,
    pub text: String,
    pub expect: Vec<Expect>,
}

pub fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/strict")
}

/// Every corpus file, applied to `board` or `schematic` (the reference).
pub fn cases(board: &str, schematic: &str) -> Vec<Case> {
    let mut out = Vec::new();
    for (kind, reference) in [("board", board), ("schematic", schematic)] {
        let mut classes: Vec<_> = fs::read_dir(dir().join(kind))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        classes.sort();
        for class_dir in classes {
            let class = class_dir.file_name().unwrap().to_str().unwrap().to_owned();
            let mut files: Vec<_> = fs::read_dir(&class_dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            files.sort();
            for file in files {
                let name = file.file_stem().unwrap().to_str().unwrap().to_owned();
                assert_eq!(file.extension().unwrap(), "edit", "{}", file.display());
                out.push(parse(
                    &fs::read_to_string(&file).unwrap(),
                    kind == "board",
                    class.clone(),
                    name,
                    reference,
                ));
            }
        }
    }
    out
}

fn parse(src: &str, board: bool, class: String, name: String, reference: &str) -> Case {
    let mut text = reference.to_owned();
    let mut expect = Vec::new();
    let mut header_class = None;
    let lines: Vec<&str> = src.split('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(c) = line.strip_prefix("# class: ") {
            header_class = Some(c.to_owned());
        } else if line.starts_with("# note: ") || line.starts_with("# main: ") {
        } else if let Some(rest) = line.strip_prefix("# ") {
            let (gate, outcome) = rest.split_once(": ").unwrap();
            let mut f = outcome.splitn(3, ' ');
            expect.push(Expect {
                gate: gate.to_owned(),
                code: f.next().unwrap().parse().unwrap(),
                what: f.next().unwrap().to_owned(),
                id: f.next().map(str::to_owned),
            });
        } else if line == "@@ find" {
            let replace = i + lines[i..].iter().position(|l| *l == "@@ replace").unwrap();
            let end = replace
                + lines[replace..]
                    .iter()
                    .position(|l| *l == "@@ end")
                    .unwrap();
            let find = format!("{}\n", lines[i + 1..replace].join("\n"));
            let with = format!("{}\n", lines[replace + 1..end].join("\n"));
            assert_eq!(
                text.matches(&find).count(),
                1,
                "{class}/{name}: the edit does not apply:\n{find}"
            );
            text = text.replacen(&find, &with, 1);
            i = end;
        } else {
            assert!(line.is_empty(), "{class}/{name}: unexpected line {line:?}");
        }
        i += 1;
    }
    assert_eq!(
        header_class.as_deref(),
        Some(class.as_str()),
        "{name} names another class"
    );
    let classes = if board {
        BOARD_CLASSES
    } else {
        SCHEMATIC_CLASSES
    };
    assert!(classes.contains(&class.as_str()), "unknown class {class}");
    let gates: &[&str] = if board {
        &["drc", "drc-parity", "extract"]
    } else {
        &["erc", "drc-parity"]
    };
    let named: Vec<&str> = expect.iter().map(|e| e.gate.as_str()).collect();
    assert_eq!(named, gates, "{class}/{name}: one line per gate");
    assert!(
        class == "control" || text != reference,
        "{class}/{name} changes nothing"
    );
    Case {
        board,
        class,
        name,
        text,
        expect,
    }
}

/// The reference board and schematic, as `emit` writes them.
pub fn reference(scratch: &Path) -> (String, String) {
    let (code, v) = run(
        &["emit", "--file", SPEC, "--out", scratch.to_str().unwrap()],
        None,
    );
    assert_eq!(code, 0, "{v}");
    (
        fs::read_to_string(scratch.join("sentinel-01.kicad_pcb")).unwrap(),
        fs::read_to_string(scratch.join("sentinel-01.kicad_sch")).unwrap(),
    )
}

/// Runs the binary from the repository root; `path` replaces `PATH`.
pub fn run(args: &[&str], path: Option<&str>) -> (i32, Value) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.args(args).current_dir(root()).stdin(Stdio::null());
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let last = stdout.lines().last().unwrap_or_default();
    (
        out.status.code().unwrap(),
        serde_json::from_str(last).unwrap_or(Value::Null),
    )
}

/// Runs `gate` on `case`, written into `scratch`; `reference` supplies the
/// other file of a parity run.
pub fn gate(
    case: &Case,
    gate: &str,
    scratch: &Path,
    reference: (&str, &str),
    path: Option<&str>,
) -> (i32, Value) {
    let ext = if case.board { "kicad_pcb" } else { "kicad_sch" };
    let file = scratch.join(format!("{}.{ext}", case.name));
    fs::write(&file, &case.text).unwrap();
    let (board, schematic) = if case.board {
        (file.clone(), scratch.join("reference.kicad_sch"))
    } else {
        (scratch.join("reference.kicad_pcb"), file.clone())
    };
    fs::write(scratch.join("reference.kicad_pcb"), reference.0).unwrap();
    fs::write(scratch.join("reference.kicad_sch"), reference.1).unwrap();
    let (b, s, f) = (
        board.to_str().unwrap(),
        schematic.to_str().unwrap(),
        file.to_str().unwrap(),
    );
    let args: Vec<&str> = match gate {
        "drc" => vec!["check", "drc", "--board", f],
        "drc-parity" => vec![
            "check",
            "drc",
            "--board",
            b,
            "--spec",
            SPEC,
            "--schematic",
            s,
        ],
        "extract" => vec!["extract", "--board", f, "--spec", SPEC],
        "erc" => vec!["check", "erc", "--schematic", f],
        other => panic!("no gate {other}"),
    };
    run(&args, path)
}

/// A verdict's `pass`/`fail`, or a refusal's kind, and its id.
pub fn outcome(v: &Value) -> (String, Option<String>) {
    match v.get("error") {
        Some(e) => (
            e["kind"].as_str().unwrap_or_default().to_owned(),
            e["id"].as_str().map(str::to_owned),
        ),
        None => (v["verdict"].as_str().unwrap_or_default().to_owned(), None),
    }
}
