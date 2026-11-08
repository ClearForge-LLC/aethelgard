//! `aethelgard emit`'s placed board and `check drc --schematic`, without
//! KiCad (`AET-WO-1301` §1.5, tier 1). What KiCad's parity check says about
//! the board is tier 2 (`with_tools.rs`).

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::pcb::{self, Rect};
use aethelgard_eda::rules::FLOOR_NM;
use aethelgard_eda::sexpr::{self, Sx};
use serde_json::{Value, json};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let d = std::env::temp_dir().join(format!(
            "aet-1301-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    c.stdin(Stdio::null());
    c
}

fn json_line(out: &Output, code: i32) -> Value {
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout {stdout} stderr {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    v
}

/// Emits `spec` into `out`; the board, the schematic and the report.
fn emit(spec: &Path, out: &Path) -> (String, String, Value) {
    let r = json_line(
        &cli()
            .args(["emit", "--file", spec.to_str().unwrap(), "--out"])
            .arg(out)
            .output()
            .unwrap(),
        0,
    );
    let read = |key: &str| fs::read_to_string(r[key]["path"].as_str().unwrap()).unwrap();
    (read("board"), read("schematic"), r)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn nm(s: &Sx, i: usize) -> i64 {
    sexpr::nm(s.arg(i).unwrap()).unwrap()
}

/// Two processes, in different environments, working directories and out
/// paths, write the same board byte for byte.
#[test]
fn emit_writes_the_board_and_two_runs_are_byte_identical() {
    let d = Dir::new("twice");
    let spec = example("sentinel-01-minimal.json");
    let cwd = d.0.join("cwd");
    fs::create_dir(&cwd).unwrap();
    let mut boards = Vec::new();
    for (out, env, dir) in [
        ("a", vec![], d.0.clone()),
        (
            "b/deeper",
            vec![
                ("LC_ALL", "tr_TR.UTF-8"),
                ("TZ", "Pacific/Kiritimati"),
                ("SOURCE_DATE_EPOCH", "1"),
            ],
            cwd.clone(),
        ),
    ] {
        let o = d.0.join(out);
        let r = json_line(
            &cli()
                .env_clear()
                .envs(env)
                .current_dir(dir)
                .args(["emit", "--file", spec.to_str().unwrap(), "--out"])
                .arg(&o)
                .output()
                .unwrap(),
            0,
        );
        let path = o.join("sentinel-01.kicad_pcb");
        assert_eq!(r["board"]["path"], path.to_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        assert_eq!(r["board"]["sha256"], sha256_hex(&bytes));
        boards.push(bytes);
    }
    assert!(!boards[0].is_empty());
    assert_eq!(boards[0], boards[1], "two emissions of the board differ");
}

fn outline(board: &Sx) -> Rect {
    let rect = board.child("gr_rect").unwrap();
    let layer = rect.child("layer").and_then(|l| l.arg(1));
    assert_eq!(layer, Some("Edge.Cuts"));
    let (s, e) = (rect.child("start").unwrap(), rect.child("end").unwrap());
    Rect {
        x0: nm(s, 1),
        y0: nm(s, 2),
        x1: nm(e, 1),
        y1: nm(e, 2),
    }
}

fn shifted(r: Rect, (x, y): (i64, i64)) -> Rect {
    Rect {
        x0: r.x0 + x,
        y0: r.y0 + y,
        x1: r.x1 + x,
        y1: r.y1 + y,
    }
}

/// By the emitter's own geometry, read back from the board it wrote: no two
/// courtyards overlap, and each is inside the outline by the board-edge
/// clearance. The board holds no tracks, vias or zones.
#[test]
fn no_courtyards_overlap_and_every_one_is_inside_the_outline() {
    let d = Dir::new("courtyards");
    let (text, _, _) = emit(&example("sentinel-01-minimal.json"), &d.0);
    let board = sexpr::parse(&text).unwrap();
    assert_eq!(
        board.child("version").and_then(|v| v.arg(1)),
        Some("20260206")
    );
    for head in ["segment", "via", "zone", "arc", "setup", "net"] {
        assert!(board.child(head).is_none(), "the board holds a `{head}`");
    }
    let edge = outline(&board);
    let inner = Rect {
        x0: edge.x0 + FLOOR_NM,
        y0: edge.y0 + FLOOR_NM,
        x1: edge.x1 - FLOOR_NM,
        y1: edge.y1 - FLOOR_NM,
    };
    let mut courtyards = Vec::new();
    for fp in board.children("footprint") {
        let reference = fp.property("Reference").unwrap().to_owned();
        let at = fp.child("at").unwrap();
        assert_eq!(at.items().len(), 3, "{reference} is rotated");
        let g = pcb::geometry(fp, &reference).unwrap();
        let c = shifted(g.courtyard, (nm(at, 1), nm(at, 2)));
        assert!(
            inner.contains(c),
            "{reference}'s courtyard {c:?} is not inside {inner:?}"
        );
        courtyards.push((reference, c));
    }
    assert_eq!(courtyards.len(), 15);
    for (i, (a, ca)) in courtyards.iter().enumerate() {
        for (b, cb) in &courtyards[i + 1..] {
            assert!(!ca.overlaps(*cb), "{a} and {b} overlap");
        }
    }
    eprintln!(
        "AET-WO-1301 board {} x {} mm",
        sexpr::mm(edge.x1 - edge.x0),
        sexpr::mm(edge.y1 - edge.y0)
    );
}

/// What each symbol pin of the emitted sheet connects to, read from the
/// sheet's own geometry: `/label`, a power symbol's value, or `None` for a
/// no-connect. Keyed by (reference, pin number).
fn sheet_connections(sch: &Sx) -> BTreeMap<(String, String), Option<String>> {
    let lib: BTreeMap<&str, &Sx> = sch
        .child("lib_symbols")
        .unwrap()
        .children("symbol")
        .map(|s| (s.arg(1).unwrap(), s))
        .collect();
    let point = |s: &Sx| {
        let a = s.child("at").unwrap();
        (nm(a, 1), nm(a, 2))
    };
    let mut named: BTreeMap<(i64, i64), String> = BTreeMap::new();
    let mut pins: Vec<((String, String), (i64, i64))> = Vec::new();
    for inst in sch.children("symbol") {
        let reference = inst.property("Reference").unwrap().to_owned();
        let (x, y) = point(inst);
        let sym = lib[inst.child("lib_id").unwrap().arg(1).unwrap()];
        for p in aethelgard_eda::schematic::pins(sym).unwrap() {
            let at = (x + p.at.0, y - p.at.1);
            if reference.starts_with('#') {
                named.insert(at, inst.property("Value").unwrap().to_owned());
            } else {
                pins.push(((reference.clone(), p.number), at));
            }
        }
    }
    for label in sch.children("label") {
        named.insert(point(label), format!("/{}", label.arg(1).unwrap()));
    }
    let wires: BTreeMap<(i64, i64), (i64, i64)> = sch
        .children("wire")
        .map(|w| {
            let pts: Vec<(i64, i64)> = w
                .child("pts")
                .unwrap()
                .children("xy")
                .map(|xy| (nm(xy, 1), nm(xy, 2)))
                .collect();
            (pts[0], pts[1])
        })
        .collect();
    pins.into_iter()
        .map(|(key, at)| {
            let net = wires.get(&at).map(|end| named[end].clone());
            (key, net)
        })
        .collect()
}

/// Every pin the sheet connects has its net on the matching pad, every
/// no-connect pin's pad carries KiCad's `unconnected-(…)` net for it, and
/// a pad with no pin carries none. Each footprint carries its symbol's
/// reference, value, footprint, fields and path.
#[test]
fn every_schematic_pin_net_is_on_its_pad_and_every_footprint_is_its_symbol() {
    let d = Dir::new("nets");
    let (text, sch_text, _) = emit(&example("sentinel-01-minimal.json"), &d.0);
    let board = sexpr::parse(&text).unwrap();
    let sch = sexpr::parse(&sch_text).unwrap();
    let mut pads: BTreeMap<(String, String), Option<String>> = BTreeMap::new();
    let fps: BTreeMap<String, &Sx> = board
        .children("footprint")
        .map(|fp| (fp.property("Reference").unwrap().to_owned(), fp))
        .collect();
    for (reference, fp) in &fps {
        for pad in fp.children("pad").filter(|p| p.arg(1) != Some("")) {
            let net = pad.child("net").map(|n| n.arg(1).unwrap().to_owned());
            pads.insert((reference.clone(), pad.arg(1).unwrap().to_owned()), net);
        }
    }
    let sheet = sheet_connections(&sch);
    assert!(sheet.len() > 144);
    for ((reference, number), net) in &sheet {
        let key = (reference.clone(), number.clone());
        let pad = pads
            .remove(&key)
            .unwrap_or_else(|| panic!("{reference} pin {number} has no pad"));
        match net {
            Some(net) => assert_eq!(pad.as_ref(), Some(net), "{reference} pad {number}"),
            None => {
                let pad = pad.unwrap_or_else(|| panic!("{reference} pad {number} has no net"));
                assert!(
                    pad.starts_with(&format!("unconnected-({reference}-"))
                        && pad.ends_with(&format!("-Pad{number})")),
                    "{reference} pad {number}: {pad}"
                );
            }
        }
    }
    // What is left is pads with no symbol pin: WSON-6's exposed pad (B8).
    let left: Vec<_> = pads.iter().collect();
    assert_eq!(
        left,
        [(&("U3".to_owned(), "7".to_owned()), &None)],
        "pads with no pin"
    );

    for inst in sch.children("symbol") {
        let reference = inst.property("Reference").unwrap();
        if reference.starts_with('#') {
            continue;
        }
        let fp = fps[reference];
        assert_eq!(fp.arg(1), inst.property("Footprint"), "{reference}");
        assert_eq!(fp.property("Value"), inst.property("Value"), "{reference}");
        let uuid = inst.child("uuid").unwrap().arg(1).unwrap();
        assert_eq!(
            fp.child("path").and_then(|p| p.arg(1)),
            Some(format!("/{uuid}").as_str()),
            "{reference}"
        );
        for prop in inst.children("property") {
            let name = prop.arg(1).unwrap();
            if !matches!(name, "Reference" | "Value" | "Footprint") {
                assert_eq!(fp.property(name), prop.arg(2), "{reference} field {name}");
            }
        }
    }
}

// ---- check drc --schematic, with a stand-in kicad-cli ----

/// A `kicad-cli` that checks the scratch copy, then writes a clean report;
/// exit 9x names the check that failed.
fn stand_in(dir: &Path, checks: &str) -> PathBuf {
    let tb = dir.join("toolbox");
    fs::create_dir_all(&tb).unwrap();
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = version ]; then echo 10.0.7; exit 0; fi
out=; prev=; last=; parity=
for a in "$@"; do
  if [ "$prev" = -o ]; then out=$a; fi
  if [ "$a" = --schematic-parity ]; then parity=1; fi
  prev=$a; last=$a
done
d=$(dirname "$last")
{checks}
printf '%s' '{{"$schema":"https://schemas.kicad.org/drc.v1.json","coordinate_units":"mm","date":"x","included_severities":["error","warning"],"kicad_version":"10.0.7","source":"input.kicad_pcb","violations":[],"unconnected_items":[],"schematic_parity":[]}}' > "$out"
exit 0
"#
    );
    let stub = tb.join("kicad-cli");
    fs::write(&stub, script).unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    tb
}

fn drc(board: &Path, schematic: Option<&Path>, path: &Path) -> Output {
    let mut c = cli();
    c.env("PATH", path)
        .args(["check", "drc", "--board"])
        .arg(board);
    if let Some(s) = schematic {
        c.arg("--schematic").arg(s);
    }
    c.output().unwrap()
}

/// With `--schematic`, the schematic is copied beside the board under the
/// board's stem, KiCad is asked for parity, and the verdict names the
/// schematic's bytes. Without it, nothing changes.
#[test]
fn check_drc_runs_parity_against_a_copy_of_the_schematic_beside_the_board() {
    let d = Dir::new("parity");
    let (board_text, sch_text, r) = emit(&example("sentinel-01-minimal.json"), &d.0.join("o"));
    let board = PathBuf::from(r["board"]["path"].as_str().unwrap());
    let sch = PathBuf::from(r["schematic"]["path"].as_str().unwrap());
    let with = stand_in(
        &d.0.join("with"),
        &format!(
            // The sandbox lets the tool read only the scratch copy, so the
            // copy is checked by its size and title.
            "[ \"$parity\" = 1 ] || exit 90\n[ \"$(wc -c < \"$d/input.kicad_sch\")\" -eq {} ] || exit 91\ngrep -q '(title \"sentinel-01\")' \"$d/input.kicad_sch\" || exit 92\n[ \"$(basename \"$last\")\" = input.kicad_pcb ] || exit 93",
            sch_text.len()
        ),
    );
    let v = json_line(&drc(&board, Some(&sch), &with), 0);
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_eq!(v["schematic"]["sha256"], sha256_hex(sch_text.as_bytes()));
    assert_eq!(
        v["schematic"]["path"],
        fs::canonicalize(&sch).unwrap().to_str().unwrap()
    );
    assert_eq!(v["input"]["sha256"], sha256_hex(board_text.as_bytes()));

    let without = stand_in(
        &d.0.join("without"),
        "[ -z \"$parity\" ] || exit 94\n[ -e \"$d/input.kicad_sch\" ] && exit 95\ntrue",
    );
    let v = json_line(&drc(&board, None, &without), 0);
    assert!(v.get("schematic").is_none(), "{v}");
}

/// A schematic that is not one flat KiCad schematic is refused before any
/// tool runs: a hierarchical sheet names a file the gate never copied.
#[test]
fn a_schematic_that_is_not_one_flat_kicad_schematic_is_refused() {
    let d = Dir::new("sheet");
    let empty = d.0.join("empty");
    fs::create_dir(&empty).unwrap();
    let board = d.0.join("b.kicad_pcb");
    fs::write(&board, "(kicad_pcb (version 20260206))\n").unwrap();
    let cases = [
        (
            "(kicad_sch (version 20260306) (sheet (at 0 0) (property \"Sheetfile\" \"/etc/x.kicad_sch\")))",
            "HierarchicalSchematic",
        ),
        (
            "(kicad_sch (lib_symbols) (x (sheet (property \"Sheetfile\" \"y.kicad_sch\"))))",
            "HierarchicalSchematic",
        ),
        ("(kicad_pcb)", "MalformedSchematic"),
        ("(kicad_sch\n#\"\n)", "MalformedSchematic"),
        ("(kicad_sch", "MalformedSchematic"),
    ];
    for (text, kind) in cases {
        let sch = d.0.join("s.kicad_sch");
        fs::write(&sch, text).unwrap();
        let v = json_line(&drc(&board, Some(&sch), &empty), 3);
        assert_eq!(v["error"]["kind"], kind, "{text}: {v}");
    }
    let v = json_line(&drc(&board, Some(&d.0.join("absent.kicad_sch")), &empty), 3);
    assert_eq!(v["error"]["kind"], "UnreadableInput", "{v}");
}

/// `check_drc` over MCP takes the schematic too.
#[test]
fn the_mcp_check_drc_tool_takes_a_schematic() {
    let mut child = cli()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        for msg in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        ] {
            writeln!(stdin, "{msg}").unwrap();
        }
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let list: Value = text
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|v| v["id"] == 2)
        .unwrap();
    let drc = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "check_drc")
        .unwrap();
    assert_eq!(
        drc["inputSchema"]["properties"]["schematic"]["type"],
        "string"
    );
    assert_eq!(drc["inputSchema"]["required"], json!(["board"]));
}
