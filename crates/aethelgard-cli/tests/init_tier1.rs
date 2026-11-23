//! `aethelgard init --template <name> --out <file>` (`AET-WO-1608`): the
//! template is its example less its placeholders, field for field; each
//! placeholder alone is refused by `validate` by its own kind and path;
//! filled, it passes `validate` and `solve`; and `init` never overwrites.

mod common;

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::Scratch;
use serde_json::Value;

const TEMPLATE: &str = "i2c-spi-bench";

/// The placeholders as ruled (`AET-WO-1608` D1, 2026-11-23), each with the
/// kind `validate` refuses it by when it alone is left empty.
const PLACEHOLDERS: [(&str, &str); 10] = [
    ("project_name", "InvalidIdentifier"),
    ("mcu.core_supply.source", "InvalidSource"),
    (
        "i2c_devices[0].pin_capacitance.declared.condition",
        "InvalidSource",
    ),
    (
        "i2c_devices[0].pin_capacitance.declared.source",
        "InvalidSource",
    ),
    (
        "spi_devices[0].pin_capacitance.declared.condition",
        "InvalidSource",
    ),
    (
        "spi_devices[0].pin_capacitance.declared.source",
        "InvalidSource",
    ),
    (
        "board.stackup.finished_thickness_um.source",
        "InvalidSource",
    ),
    ("board.stackup.dielectrics[0].source", "InvalidSource"),
    ("board.stackup.copper_max_um.source", "InvalidSource"),
    (
        "board.stackup.relative_permittivity_max_permille.source",
        "InvalidSource",
    ),
];

fn aethelgard(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .output()
        .unwrap()
}

/// stdout is exactly one JSON line; returns it with the exit code.
fn line(out: &Output) -> (i32, Value) {
    let stdout = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout:?}");
    (
        out.status.code().unwrap(),
        serde_json::from_str(stdout).unwrap(),
    )
}

fn init(out: &Path) -> (i32, Value) {
    line(&aethelgard(&[
        "init",
        "--template",
        TEMPLATE,
        "--out",
        out.to_str().unwrap(),
    ]))
}

/// The JSON pointer of a placeholder path such as `a[0].b`.
fn pointer(path: &str) -> String {
    path.split('.')
        .flat_map(|seg| match seg.split_once('[') {
            Some((key, rest)) => vec![key.to_owned(), rest.trim_end_matches(']').to_owned()],
            None => vec![seg.to_owned()],
        })
        .fold(String::new(), |acc, seg| format!("{acc}/{seg}"))
}

/// Every string leaf's pointer whose value is `""`.
fn empties(v: &Value, at: String, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => m
            .iter()
            .for_each(|(k, c)| empties(c, format!("{at}/{k}"), out)),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, c)| empties(c, format!("{at}/{i}"), out)),
        Value::String(s) if s.is_empty() => out.push(at),
        _ => {}
    }
}

/// `init` into a fresh scratch directory: the written file's JSON.
fn written(label: &str) -> (Scratch, PathBuf, Value) {
    let s = Scratch::new(label);
    let out = s.0.join("spec.json");
    let (code, v) = init(&out);
    assert_eq!(code, 0, "{v}");
    let spec = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    (s, out, spec)
}

/// The template with every placeholder but `skip` filled with a test string.
fn filled(spec: &Value, skip: Option<&str>) -> Value {
    let mut v = spec.clone();
    for (p, _) in PLACEHOLDERS {
        if Some(p) == skip {
            continue;
        }
        let fill = if p == "project_name" {
            "init-test"
        } else {
            "a test string standing for a citation"
        };
        *v.pointer_mut(&pointer(p)).unwrap() = fill.into();
    }
    v
}

fn validate(dir: &Path, spec: &Value) -> (i32, Value) {
    let f = dir.join("filled.json");
    std::fs::write(&f, serde_json::to_vec_pretty(spec).unwrap()).unwrap();
    line(&aethelgard(&["validate", "--file", f.to_str().unwrap()]))
}

#[test]
fn init_prints_the_template_the_file_and_every_placeholder() {
    let s = Scratch::new("init-line");
    let out = s.0.join("spec.json");
    let (code, v) = init(&out);
    assert_eq!(code, 0, "{v}");
    let paths: Vec<&str> = PLACEHOLDERS.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        v,
        serde_json::json!({
            "ok": true,
            "template": TEMPLATE,
            "out": out.to_str().unwrap(),
            "placeholders": paths,
        })
    );
    // Written verbatim: the bytes held in the crate.
    assert_eq!(
        std::fs::read(&out).unwrap(),
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/i2c-spi-bench.json"))
            .unwrap()
    );
}

#[test]
fn the_template_is_its_example_less_the_placeholders() {
    let (_s, _out, spec) = written("init-example");
    let example: Value = serde_json::from_str(
        &std::fs::read_to_string(common::example("sentinel-02-bench.json")).unwrap(),
    )
    .unwrap();
    let mut expected = example.clone();
    for (p, _) in PLACEHOLDERS {
        let at = expected.pointer_mut(&pointer(p)).unwrap();
        assert!(at.is_string(), "{p} is a string in the example");
        assert_ne!(at, "", "{p} is written in the example");
        *at = "".into();
    }
    assert_eq!(spec, expected, "the template drifted from its example");
    // `""` exactly at the placeholders, nowhere else.
    let mut found = Vec::new();
    empties(&spec, String::new(), &mut found);
    let mut ruled: Vec<String> = PLACEHOLDERS.iter().map(|(p, _)| pointer(p)).collect();
    found.sort();
    ruled.sort();
    assert_eq!(found, ruled);
}

#[test]
fn the_template_is_at_the_current_specification_version() {
    let (_s, _out, spec) = written("init-version");
    assert_eq!(
        spec["schema_version"],
        aethelgard_ir::SPECIFICATION_SCHEMA_VERSION
    );
}

#[test]
fn unfilled_it_is_refused_at_its_first_placeholder() {
    let (s, out, _) = written("init-unfilled");
    let (code, v) = line(&aethelgard(&["validate", "--file", out.to_str().unwrap()]));
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "InvalidIdentifier", "{v}");
    assert_eq!(v["error"]["path"], "project_name", "{v}");
    // The message ends with the fix the crate-name rule accepts (D3).
    let message = v["error"]["message"].as_str().unwrap();
    assert!(
        message.ends_with("; the fix: \"project_name\": \"<1 to 58 bytes of a-z, 0-9 and -, a letter first, not ending with -, with no -- and no date shape dddd-dd-dd, and not a Windows device name (con, prn, aux, nul, com0 to com9, lpt0 to lpt9)>\""),
        "{message}"
    );
    drop(s);
}

#[test]
fn each_placeholder_alone_is_refused_by_its_own_kind_and_path() {
    let (s, _out, spec) = written("init-each");
    for (p, kind) in PLACEHOLDERS {
        let (code, v) = validate(&s.0, &filled(&spec, Some(p)));
        assert_eq!(code, 2, "{p}: {v}");
        assert_eq!(v["error"]["kind"], kind, "{p}: {v}");
        assert_eq!(v["error"]["path"], p, "{p}: {v}");
    }
}

#[test]
fn filled_it_passes_validate_and_solve() {
    let (s, _out, spec) = written("init-filled");
    let (code, v) = validate(&s.0, &filled(&spec, None));
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["project_name"], "init-test");
    let f = s.0.join("filled.json");
    let out = aethelgard(&["solve", "--file", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

/// `init` refuses `out`, exit 3, `UnwritableOutput`, with one line.
fn refused(out: &Path) {
    let (code, v) = init(out);
    assert_eq!(code, 3, "{}: {v}", out.display());
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "UnwritableOutput", "{v}");
}

#[test]
fn it_never_overwrites_a_file() {
    let s = Scratch::new("init-file");
    let out = s.0.join("mine.json");
    std::fs::write(&out, b"a person's own work").unwrap();
    refused(&out);
    assert_eq!(std::fs::read(&out).unwrap(), b"a person's own work");
    // Nor an empty one.
    let empty = s.0.join("empty.json");
    std::fs::write(&empty, b"").unwrap();
    refused(&empty);
    assert_eq!(std::fs::read(&empty).unwrap(), b"");
}

#[test]
fn it_never_follows_a_symlink() {
    let s = Scratch::new("init-link");
    let target = s.0.join("target.json");
    std::fs::write(&target, b"kept").unwrap();
    let link = s.0.join("link.json");
    symlink(&target, &link).unwrap();
    refused(&link);
    assert_eq!(std::fs::read(&target).unwrap(), b"kept");
    // A dangling one is not created through.
    let dangling = s.0.join("dangling.json");
    symlink(s.0.join("absent.json"), &dangling).unwrap();
    refused(&dangling);
    assert!(!s.0.join("absent.json").exists());
    // Nor a path through a dangling directory link.
    let dir_link = s.0.join("gone");
    symlink(s.0.join("no-such-dir"), &dir_link).unwrap();
    refused(&dir_link.join("spec.json"));
    assert!(!s.0.join("no-such-dir").exists());
}

#[test]
fn a_fifo_a_directory_or_a_missing_directory_is_refused() {
    let s = Scratch::new("init-special");
    let fifo = s.0.join("fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::from_raw_mode(0o600),
    )
    .unwrap();
    // Refused at once: never opened, so never waited on.
    refused(&fifo);
    refused(&s.0);
    refused(&s.0.join("absent-dir/spec.json"));
    assert!(!s.0.join("absent-dir").exists(), "no directory is created");
}

#[test]
fn a_path_through_a_live_directory_link_writes_inside_its_target() {
    // As `build --out`: only the last component is never followed.
    let s = Scratch::new("init-dirlink");
    std::fs::create_dir(s.0.join("real")).unwrap();
    symlink(s.0.join("real"), s.0.join("alias")).unwrap();
    let (code, v) = init(&s.0.join("alias/spec.json"));
    assert_eq!(code, 0, "{v}");
    assert!(s.0.join("real/spec.json").is_file());
}

#[test]
fn an_unknown_template_is_a_usage_error() {
    let s = Scratch::new("init-unknown");
    let out = s.0.join("spec.json");
    let (code, v) = line(&aethelgard(&[
        "init",
        "--template",
        "sentinel-01",
        "--out",
        out.to_str().unwrap(),
    ]));
    assert_eq!(code, 64, "{v}");
    assert_eq!(v["error"]["kind"], "UsageError", "{v}");
    assert!(!out.exists());
}
