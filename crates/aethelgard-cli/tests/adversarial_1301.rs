//! AET-WO-1301 §5, the adversarial pass through the CLI, tier 1 (no KiCad).
//! What KiCad said was measured in the tools image (`container/IMAGE`,
//! `--network none`) at 6b55cb3; the tier-2 half is
//! `adversarial_1301_tools.rs`. A test marked **red until fixed** pins an
//! attack that succeeded; it asserts the secure outcome and fails until the
//! code changes. The rest pin attacks the code withstood.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::pcb::{self, Rect};
use aethelgard_eda::sexpr::{self, Sx};
use serde_json::Value;

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Dir(PathBuf);

impl Dir {
    /// On disk (target/tmp), not in RAM-backed /tmp.
    fn new(tag: &str) -> Dir {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "aet-adv-1301-{tag}-{}-{}",
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

fn json_line(out: &Output) -> Value {
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout {stdout} stderr {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(&stdout).unwrap()
}

fn reference_spec() -> Value {
    serde_json::from_slice(&fs::read(example("sentinel-01-minimal.json")).unwrap()).unwrap()
}

fn write_spec(dir: &Path, name: &str, spec: &Value) -> PathBuf {
    let p = dir.join(format!("{name}.json"));
    fs::write(&p, serde_json::to_vec_pretty(spec).unwrap()).unwrap();
    p
}

fn emit(spec: &Path, out: &Path) -> Output {
    cli()
        .args(["emit", "--file"])
        .arg(spec)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

/// **Red until fixed.** KiCad takes two references for one when they differ
/// only in ASCII case or in leading zeros of their number. The specification
/// refuses only exact duplicate references, and the emitter skips only exact
/// matches when it numbers its passives and the MCU, so each specification
/// below validates and emits. Measured, with the reference specification's
/// second I2C device renamed: `u2` (beside device `U2`) and `r1` (beside
/// pull-up `R1`) fail the gate with `duplicate_footprints` and
/// `extra_footprint`; `U01` (beside the MCU's `U1`) and `U02` (beside `U2`)
/// with `extra_footprint`. A specification that validates must not emit a
/// board its own gate fails; such references should be refused (the
/// emitter's `design`, beside its `#` and exact-duplicate checks, and
/// `Builder::reference`).
#[test]
fn attack_references_kicad_takes_for_one_are_refused() {
    let d = Dir::new("case");
    let named = |r: &str| {
        let mut s = reference_spec();
        s["i2c_devices"][1]["reference"] = r.into();
        s
    };
    for (name, spec) in [
        ("u2", named("u2")),
        ("r1", named("r1")),
        ("U01", named("U01")),
        ("U02", named("U02")),
    ] {
        let out = emit(&write_spec(&d.0, name, &spec), &d.0.join(name));
        let v = json_line(&out);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{name}: a reference KiCad takes for a duplicate was emitted: {v}"
        );
        assert_eq!(v["error"]["kind"], "DuplicateReference", "{name}: {v}");
    }
}

fn nm(s: &Sx, i: usize) -> i64 {
    sexpr::nm(s.arg(i).unwrap()).unwrap()
}

/// KiCad 10.0.7's widest `RUL-14` glyph, `m`, advances 4/3 of the font
/// width (measured by Gerber plot in the tools image; see
/// `aethelgard-eda/tests/adversarial_1301.rs`). A centred reference of `n`
/// glyphs is at most this wide each side of its anchor, pen included.
fn reference_half_width(n: i64, font_w: i64, thick: i64) -> i64 {
    n * font_w * 2 / 3 + thick / 2
}

/// **Red until fixed.** Twenty-four I2C devices with 64-byte references of
/// `m`. By KiCad's ink, read back from the emitted board, every reference
/// stays inside the outline and clear of every other part's courtyard and
/// reference. Measured on this board: 9 `silk_edge_clearance`, 2
/// `silk_overlap` (U00…'s reference over R5's) and 20 `silk_over_copper`
/// (references over U1's and U4's pads), beyond the unconnected items.
#[test]
fn attack_a_wide_reference_stays_clear_of_every_other_part() {
    let d = Dir::new("wide");
    let mut s = reference_spec();
    let mut buses = Vec::new();
    let mut devices = Vec::new();
    let mut constraints = Vec::new();
    for b in 0..4 {
        buses.push(serde_json::json!({
            "id": format!("B{b}"), "peripheral": null, "speed_hz": 400000,
            "pull_up_ohms": 2200, "domain": "VDD_3V3",
            "sda": {"net": format!("SDA{b}"), "pin": null},
            "scl": {"net": format!("SCL{b}"), "pin": null}
        }));
        for net in [format!("SDA{b}"), format!("SCL{b}")] {
            constraints.push(serde_json::json!({
                "net_name": net, "max_capacitance_femtofarads": 100000,
                "max_rise_time_nanoseconds": 300, "impedance_ohms": null
            }));
        }
        for (part, address) in [
            ("BME280", 118),
            ("BME280", 119),
            ("TMP117", 72),
            ("TMP117", 73),
            ("TMP117", 74),
            ("TMP117", 75),
        ] {
            let i = devices.len();
            let mut reference = format!("U{i:02}{}", "m".repeat(64));
            reference.truncate(64);
            devices.push(serde_json::json!({
                "reference": reference, "part": part, "supply": "VDD_3V3",
                "bus": format!("B{b}"), "address": address
            }));
        }
    }
    s["i2c_buses"] = buses.into();
    s["i2c_devices"] = devices.into();
    s["net_constraints"] = constraints.into();
    // The control: the same parts with short references meet nothing.
    let mut short = s.clone();
    for (i, dev) in short["i2c_devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        dev["reference"] = format!("S{i}").into();
    }
    let control = reference_collisions(&d.0, "short", &short);
    assert!(control.is_empty(), "control: {control:#?}");
    let met = reference_collisions(&d.0, "wide", &s);
    assert!(met.is_empty(), "{} collisions: {met:#?}", met.len());
}

/// Emits `spec`; every reference text, by KiCad's ink, that leaves the
/// outline or meets another part's courtyard or reference.
fn reference_collisions(dir: &Path, name: &str, spec: &Value) -> Vec<String> {
    let out = emit(&write_spec(dir, name, spec), &dir.join(name));
    let r = json_line(&out);
    assert_eq!(out.status.code(), Some(0), "{r}");
    let board =
        sexpr::parse(&fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap()).unwrap();
    let edge = board.child("gr_rect").unwrap();
    let outline = Rect {
        x0: nm(edge.child("start").unwrap(), 1),
        y0: nm(edge.child("start").unwrap(), 2),
        x1: nm(edge.child("end").unwrap(), 1),
        y1: nm(edge.child("end").unwrap(), 2),
    };
    let shift = |r: Rect, (x, y): (i64, i64)| Rect {
        x0: r.x0 + x,
        y0: r.y0 + y,
        x1: r.x1 + x,
        y1: r.y1 + y,
    };
    // (reference, courtyard, reference text box), board coordinates.
    let mut parts: Vec<(String, Rect, Rect)> = Vec::new();
    for fp in board.children("footprint") {
        let reference = fp.property("Reference").unwrap().to_owned();
        let at = fp.child("at").unwrap();
        let origin = (nm(at, 1), nm(at, 2));
        let g = pcb::geometry(fp, &reference).unwrap();
        let prop = fp
            .children("property")
            .find(|p| p.arg(1) == Some("Reference"))
            .unwrap();
        let pat = prop.child("at").unwrap();
        let font = prop.child("effects").unwrap().child("font").unwrap();
        let (w, h) = (
            nm(font.child("size").unwrap(), 1),
            nm(font.child("size").unwrap(), 2),
        );
        let thick = nm(font.child("thickness").unwrap(), 1);
        assert_eq!(pat.arg(3).unwrap_or("0"), "0", "{reference}");
        let n = i64::try_from(reference.chars().count()).unwrap();
        let (hw, hh) = (reference_half_width(n, w, thick), h / 2 + thick);
        let (x, y) = (nm(pat, 1) + origin.0, nm(pat, 2) + origin.1);
        let text = Rect {
            x0: x - hw,
            y0: y - hh,
            x1: x + hw,
            y1: y + hh,
        };
        parts.push((reference, shift(g.courtyard, origin), text));
    }
    let mut met = Vec::new();
    for (i, (a, _, text)) in parts.iter().enumerate() {
        if !outline.contains(*text) {
            met.push(format!("{a}'s reference leaves the board"));
        }
        for (j, (b, courtyard, other_text)) in parts.iter().enumerate() {
            if i != j && (text.overlaps(*courtyard) || (j > i && text.overlaps(*other_text))) {
                met.push(format!("{a}'s reference meets {b}"));
            }
        }
    }
    met
}

/// Withstood. A specification equal as a value, whatever its keys' order,
/// its whitespace, the locale, the time zone or the environment, emits the
/// same board byte for byte (measured on the host too: `env -i`,
/// `LC_ALL=tr_TR.UTF-8`, `TZ=Pacific/Kiritimati`, another working
/// directory, an absolute out path and a 1 MiB stack all gave board sha256
/// 2e84b9e5…).
#[test]
fn the_board_does_not_depend_on_key_order_locale_or_environment() {
    fn reversed(v: &Value) -> String {
        match v {
            Value::Object(m) => format!(
                "{{ {} }}",
                m.iter()
                    .rev()
                    .map(|(k, x)| format!("{}:\n{}", Value::String(k.clone()), reversed(x)))
                    .collect::<Vec<_>>()
                    .join(" ,\n\t")
            ),
            Value::Array(a) => {
                format!("[{}]", a.iter().map(reversed).collect::<Vec<_>>().join(","))
            }
            x => x.to_string(),
        }
    }
    let d = Dir::new("det");
    let spec = reference_spec();
    let shuffled = d.0.join("shuffled.json");
    fs::write(&shuffled, reversed(&spec)).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&shuffled).unwrap()).unwrap(),
        spec
    );
    let board = |spec: &Path, out: &str, env: &[(&str, &str)]| {
        let mut c = cli();
        c.env_clear()
            .env("HOME", std::env::var_os("HOME").unwrap_or_default());
        for (k, v) in env {
            c.env(k, v);
        }
        let o = c
            .args(["emit", "--file"])
            .arg(spec)
            .arg("--out")
            .arg(d.0.join(out))
            .output()
            .unwrap();
        let r = json_line(&o);
        assert_eq!(o.status.code(), Some(0), "{r}");
        fs::read(r["board"]["path"].as_str().unwrap()).unwrap()
    };
    let plain = board(&example("sentinel-01-minimal.json"), "plain", &[]);
    let other = board(
        &shuffled,
        "other/deeper",
        &[
            ("LC_ALL", "tr_TR.UTF-8"),
            ("LANG", "tr_TR.UTF-8"),
            ("TZ", "Pacific/Kiritimati"),
            ("HOME", "/nonexistent"),
        ],
    );
    assert!(
        plain == other,
        "the board depends on more than the specification"
    );
}

/// Withstood. The schematic copied in for parity is checked like the board
/// before any tool is looked for: the board's own file, a directory and a
/// FIFO are refused, so the copies cannot collide and nothing blocks.
#[test]
fn a_schematic_that_is_the_board_a_directory_or_a_fifo_is_refused() {
    let d = Dir::new("sch");
    let empty = d.0.join("empty");
    fs::create_dir(&empty).unwrap();
    let board = d.0.join("b.kicad_pcb");
    fs::write(&board, "(kicad_pcb (version 20260206))\n").unwrap();
    let fifo = d.0.join("s.kicad_sch");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    for (sch, kind) in [
        (&board, "MalformedSchematic"),
        (&empty, "UnreadableInput"),
        (&fifo, "UnreadableInput"),
    ] {
        let out = cli()
            .env("PATH", &empty)
            .args(["check", "drc", "--board"])
            .arg(&board)
            .arg("--schematic")
            .arg(sch)
            .output()
            .unwrap();
        let v = json_line(&out);
        assert_eq!(out.status.code(), Some(3), "{}: {v}", sch.display());
        assert_eq!(v["error"]["kind"], kind, "{}: {v}", sch.display());
    }
}

/// **Red until fixed.** The gate refuses a board carrying `board_only`
/// before any tool runs, as it refuses its other waivers (exit 3). See
/// `aethelgard-eda/tests/adversarial_1301.rs` for what KiCad does with it.
#[test]
fn attack_check_drc_refuses_a_board_only_footprint_before_any_tool_runs() {
    let d = Dir::new("board-only");
    let empty = d.0.join("empty");
    fs::create_dir(&empty).unwrap();
    let r = json_line(&emit(&example("sentinel-01-minimal.json"), &d.0.join("o")));
    let text = fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap();
    let marked = text.replacen("(attr smd)", "(attr smd board_only)", 1);
    assert_ne!(marked, text);
    let board = d.0.join("b.kicad_pcb");
    fs::write(&board, marked).unwrap();
    let out = cli()
        .env("PATH", &empty)
        .args(["check", "drc", "--board"])
        .arg(&board)
        .arg("--schematic")
        .arg(r["schematic"]["path"].as_str().unwrap())
        .output()
        .unwrap();
    let v = json_line(&out);
    assert_eq!(
        out.status.code(),
        Some(3),
        "a board_only footprint reached the tool lookup: {v}"
    );
    assert_ne!(v["error"]["kind"], "MalformedBoard", "{v}");
}

/// The emitted reference board and schematic, as text.
fn emitted(d: &Dir) -> (String, String) {
    let r = json_line(&emit(&example("sentinel-01-minimal.json"), &d.0.join("o")));
    let read = |k: &str| fs::read_to_string(r[k]["path"].as_str().unwrap()).unwrap();
    (read("board"), read("schematic"))
}

/// `check drc --schematic` on texts, with no tool on PATH: a refusal must
/// come before any tool runs.
fn refused(d: &Dir, board: &str, sch: &str) -> (Option<i32>, Value) {
    let empty = d.0.join("empty");
    fs::create_dir_all(&empty).unwrap();
    let (b, s) = (d.0.join("b.kicad_pcb"), d.0.join("s.kicad_sch"));
    fs::write(&b, board).unwrap();
    fs::write(&s, sch).unwrap();
    let out = cli()
        .env("PATH", &empty)
        .args(["check", "drc", "--board"])
        .arg(&b)
        .arg("--schematic")
        .arg(&s)
        .output()
        .unwrap();
    (out.status.code(), json_line(&out))
}

/// Fixed (builder's F6, measured at 6b55cb3 in the tools image). KiCad's
/// parity check accepts a pad marked `no_connect` whose net starts with the
/// schematic's net and `_`. With `SPI_MOSI` renamed `I2C_SDA_X`, the MCU's
/// SDA pad moved onto `/I2C_SDA_X` with `(pintype "bidirectional+no_connect")`
/// passed parity with nothing beyond `unconnected_items` (65, as the
/// unedited board). The gate now refuses a `no_connect` pad on a real net.
#[test]
fn attack_a_no_connect_pad_moved_onto_another_net_is_refused() {
    let d = Dir::new("nc-pad");
    let (board, sch) = emitted(&d);
    let mcu = board.find("(footprint \"Package_QFP").unwrap();
    let at = mcu + board[mcu..].find("(net \"/I2C_SDA\")").unwrap();
    let moved = format!(
        "{}(net \"/I2C_SDA_X\")\n\t\t\t(pintype \"bidirectional+no_connect\"){}",
        &board[..at],
        &board[at + "(net \"/I2C_SDA\")".len()..]
    );
    let (code, v) = refused(&d, &moved, &sch);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "NoConnectPadOnNet", "{v}");
}

/// Fixed (builder's F7, measured at 6b55cb3). A rail named `unconnected-V`
/// is a power net KiCad's parity check matches by prefix: U4's supply pad
/// moved to `unconnected-V-cut`, off the rail, passed parity with nothing
/// beyond `unconnected_items` (65 → 64). The emitter now refuses such a
/// name, and the gate refuses a schematic whose label or value carries one.
#[test]
fn attack_a_rail_named_with_kicads_unconnected_prefix_is_refused() {
    let d = Dir::new("nc-rail");
    let spec = reference_spec()
        .to_string()
        .replace("\"VDD_3V3\"", "\"unconnected-V\"");
    let file = d.0.join("spec.json");
    fs::write(&file, spec).unwrap();
    let out = emit(&file, &d.0.join("o"));
    let v = json_line(&out);
    assert_eq!(out.status.code(), Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "ReservedName", "{v}");
    assert_eq!(v["error"]["id"], "unconnected-V", "{v}");

    let (board, sch) = emitted(&d);
    let (code, v) = refused(&d, &board, &sch.replace("\"VDD_3V3\"", "\"unconnected-V\""));
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "ReservedNetName", "{v}");
}
