//! `AET-WO-1302`'s adversarial pass, tier 2: what KiCad's gate does not judge
//! on a routed board, pinned as measured for a ruling (FEEDBACK A2, A3). Each
//! test turns red if KiCad, or the gate, starts to see it. The router's own
//! judges refuse both boards (`routing_tier1.rs`).
#![cfg(feature = "with-tools")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-1302-adv-{tag}"));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Temp(d)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn record(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1302 adversarial {line}\n").as_bytes());
}

fn aethelgard(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    (out.status.code().unwrap(), v)
}

/// The emitted reference board and schematic, as text, and the spec.
fn reference(t: &Temp) -> (String, String, PathBuf) {
    let spec = example("sentinel-01-minimal.json");
    let out = t.0.join("ref");
    let (code, r) = aethelgard(&[
        "emit",
        "--file",
        spec.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{r}");
    let read = |k: &str| fs::read_to_string(r[k]["path"].as_str().unwrap()).unwrap();
    (read("board"), read("schematic"), spec)
}

/// `check drc --spec --schematic` on board text: (exit code, finding kinds).
fn drc(dir: &Path, board: &str, sch: &str, spec: &Path) -> (i32, Vec<String>) {
    fs::create_dir_all(dir).unwrap();
    let (b, s) = (dir.join("b.kicad_pcb"), dir.join("s.kicad_sch"));
    fs::write(&b, board).unwrap();
    fs::write(&s, sch).unwrap();
    let (code, v) = aethelgard(&[
        "check",
        "drc",
        "--board",
        b.to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
        "--schematic",
        s.to_str().unwrap(),
    ]);
    assert!(code == 0 || code == 1, "{v}");
    let kinds = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap().to_owned())
        .collect();
    (code, kinds)
}

/// Pinned as measured (A2, for a ruling). KiCad 10.0.7 takes a track's and
/// a via's net from the pads its copper reaches, so a board whose copper is
/// labelled with the wrong net passes the gate: every `VDD_3V3` segment and
/// via relabelled `GND` gives exit 0 and no finding. `1303` reads these
/// labels.
#[test]
fn kicad_does_not_judge_the_net_a_track_is_labelled_with() {
    let t = Temp::new("relabel");
    let (board, sch, spec) = reference(&t);
    let mut out = String::new();
    let mut copper = false;
    for line in board.lines() {
        match line {
            "\t(segment" | "\t(via" => copper = true,
            "\t)" => copper = false,
            _ => {}
        }
        if copper && line == "\t\t(net \"VDD_3V3\")" {
            out.push_str("\t\t(net \"GND\")\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    assert_ne!(out, board);
    let (code, kinds) = drc(&t.0.join("b"), &out, &sch, &spec);
    record(&format!("VDD copper labelled GND: exit {code}, {kinds:?}"));
    assert_eq!(
        (code, kinds),
        (0, vec![]),
        "KiCad now judges copper labels; revisit A2"
    );
}

/// Pinned as measured (A3, for a ruling). A closed loop of four F.Cu
/// segments on a net no pad carries, touching nothing, has no dangling end
/// and no ratsnest: exit 0, no finding.
#[test]
fn kicad_does_not_see_a_floating_loop_on_a_net_no_pad_carries() {
    let t = Temp::new("loop");
    let (board, sch, spec) = reference(&t);
    let seg = |a: (&str, &str), b: (&str, &str), i: usize| {
        format!(
            "\t(segment (start {} {}) (end {} {}) (width 0.2) (layer \"F.Cu\") (net \"/NOPE\") (uuid \"00000000-0000-4000-8000-00000000000{i}\"))\n",
            a.0, a.1, b.0, b.1
        )
    };
    let (p, q, r, s) = (("74", "78"), ("76", "78"), ("76", "79"), ("74", "79"));
    let ring = [seg(p, q, 1), seg(q, r, 2), seg(r, s, 3), seg(s, p, 4)].concat();
    let at = board.rfind("\t(embedded_fonts no)").unwrap();
    let edited = format!("{}{ring}{}", &board[..at], &board[at..]);
    let (code, kinds) = drc(&t.0.join("b"), &edited, &sch, &spec);
    record(&format!("floating loop on /NOPE: exit {code}, {kinds:?}"));
    assert_eq!(
        (code, kinds),
        (0, vec![]),
        "KiCad now sees the loop; revisit A3"
    );
}
