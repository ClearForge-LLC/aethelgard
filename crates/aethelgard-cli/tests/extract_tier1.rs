//! `aethelgard extract` (`AET-WO-1305`), tier 1: no tool runs. The board is
//! the one `emit` writes for the reference; the numbers are those recorded
//! in the work order's FEEDBACK (§1.3) before this file was written.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::Value;

const SPEC: &str = "examples/sentinel-01-minimal.json";
const PCB: &str = "sentinel-01.kicad_pcb";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1305-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

/// The reference board's text, emitted once.
fn reference() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let dir = scratch("emit");
        let (code, v) = run(&["emit", "--file", SPEC, "--out", dir.to_str().unwrap()]);
        assert_eq!(code, 0, "{v}");
        let text = fs::read_to_string(dir.join(PCB)).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        text
    })
}

/// `extract` on `board` against `spec` (a path from the root).
fn extract(label: &str, board: &str, spec: &str) -> (i32, Value) {
    let dir = scratch(label);
    let path = dir.join(PCB);
    fs::write(&path, board).unwrap();
    let out = run(&["extract", "--board", path.to_str().unwrap(), "--spec", spec]);
    fs::remove_dir_all(&dir).unwrap();
    out
}

/// A specification file: the reference with `edit` applied.
fn spec_with(label: &str, edit: impl Fn(&mut Value)) -> PathBuf {
    let mut v: Value =
        serde_json::from_str(&fs::read_to_string(root().join(SPEC)).unwrap()).unwrap();
    edit(&mut v);
    let path =
        std::env::temp_dir().join(format!("aet-1305-spec-{label}-{}.json", std::process::id()));
    fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    path
}

fn declared(v: &mut Value, ff: u64) {
    for c in v["net_constraints"].as_array_mut().unwrap() {
        c["max_capacitance_femtofarads"] = ff.into();
    }
}

fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn terms(line: &Value) -> [u64; 5] {
    let t = &line["terms"];
    [
        t["trace"].as_u64().unwrap(),
        t["vias"].as_u64().unwrap(),
        t["pads"].as_u64().unwrap(),
        t["pins"].as_u64().unwrap(),
        line["c_bound_femtofarads"].as_u64().unwrap(),
    ]
}

fn refused(v: &Value) -> &str {
    assert_eq!(v["ok"], false, "{v}");
    assert!(v.get("lines").is_none(), "no estimate after a refusal: {v}");
    v["error"]["kind"].as_str().unwrap()
}

/// §3.1: the reference passes, every term as §1.3 recorded it, and the
/// verdict names both inputs' hashes and what the number is.
#[test]
fn the_reference_passes_with_every_term() {
    let (code, v) = extract("reference", reference(), SPEC);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["check"], "extract");
    assert_eq!(
        v["capacitance"],
        "upper bound estimated from routed geometry"
    );
    assert_eq!(v["input"]["sha256"], sha(reference().as_bytes()));
    assert_eq!(
        v["specification_sha256"],
        sha(&fs::read(root().join(SPEC)).unwrap())
    );
    assert!(!v["conditions"].as_array().unwrap().is_empty());
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    let (sda, scl) = (&lines[0], &lines[1]);
    assert_eq!(
        (sda["net"].as_str(), scl["net"].as_str()),
        (Some("I2C_SDA"), Some("I2C_SCL"))
    );
    assert_eq!(terms(sda), [9_146, 6_076, 1_578, 30_000, 46_800]);
    assert_eq!(terms(scl), [16_024, 10_633, 1_578, 30_000, 58_235]);
    for (line, length, segments, vias) in [(sda, 24_250_000, 19, 4), (scl, 43_500_000, 31, 7)] {
        assert_eq!(line["verdict"], "pass");
        assert_eq!(line["declared_femtofarads"], 100_000);
        assert_eq!(line["length_nm"], length);
        assert_eq!(line["segments"], segments);
        assert_eq!(line["via_count"], vias);
        assert_eq!(line["pad_count"], 4);
        assert_eq!(line["pins"].as_array().unwrap().len(), 3);
        assert_eq!(line["pull_ups"].as_array().unwrap().len(), 1);
    }
    assert_eq!(v["findings"], Value::Array(vec![]));
}

/// §3.2 and the §1.3 prediction: declared 20 pF fails both lines on their
/// pins alone, exit 1, each named with its numbers; 50 pF fails SCL only.
#[test]
fn a_declared_bound_below_the_bound_fails_naming_each_line() {
    let spec = spec_with("20p", |v| declared(v, 20_000));
    let (code, v) = extract("20p", reference(), spec.to_str().unwrap());
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "fail");
    let messages: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            assert_eq!(f["kind"], "capacitance_over_declared_bound");
            f["message"].as_str().unwrap()
        })
        .collect();
    assert_eq!(
        messages,
        [
            "`I2C_SDA`: bound 46800 fF (trace 9146, vias 6076, pads 1578, pins 30000) exceeds the declared 20000 fF",
            "`I2C_SCL`: bound 58235 fF (trace 16024, vias 10633, pads 1578, pins 30000) exceeds the declared 20000 fF",
        ]
    );

    let spec = spec_with("50p", |v| declared(v, 50_000));
    let (code, v) = extract("50p", reference(), spec.to_str().unwrap());
    assert_eq!(code, 1, "{v}");
    let verdicts: Vec<&str> = v["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["verdict"].as_str().unwrap())
        .collect();
    assert_eq!(verdicts, ["pass", "fail"]);
    assert_eq!(v["findings"].as_array().unwrap().len(), 1);

    // At the bound exactly, the line passes.
    let spec = spec_with("at", |v| {
        for c in v["net_constraints"].as_array_mut().unwrap() {
            let at = if c["net_name"] == "I2C_SDA" {
                46_800
            } else {
                58_235
            };
            c["max_capacitance_femtofarads"] = at.into();
        }
    });
    let (code, v) = extract("at", reference(), spec.to_str().unwrap());
    assert_eq!(code, 0, "{v}");
}

/// §3.3: a board whose net labels disagree with its copper is refused by
/// name, before any estimate.
#[test]
fn labels_that_disagree_with_the_copper_are_refused() {
    let text = reference();
    let at = text.find("(net \"/I2C_SDA\")\n\t\t(uuid").unwrap();
    let relabelled = format!(
        "{}(net \"/I2C_SCL\"){}",
        &text[..at],
        &text[at + "(net \"/I2C_SDA\")".len()..]
    );
    let (code, v) = extract("labels", &relabelled, SPEC);
    assert_eq!(code, 3, "{v}");
    assert_eq!(refused(&v), "NetLabelsDisagree");
}

/// One segment of the board's text: its `(start …)` and `(end …)` in mm,
/// its net, and where each of the two lines is.
struct Seg {
    start: (f64, f64),
    end: (f64, f64),
    net: String,
    start_at: (usize, usize),
    end_at: (usize, usize),
}

fn segments(text: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = text[from..].find("\t(segment\n") {
        let at = from + i;
        let body = &text[at..at + text[at..].find("\n\t)").unwrap()];
        let xy = |head: &str| {
            let open = body.find(&format!("({head} ")).unwrap();
            let s = open + head.len() + 2;
            let e = s + body[s..].find(')').unwrap();
            let mut it = body[s..e].split(' ').map(|n| n.parse::<f64>().unwrap());
            (
                (it.next().unwrap(), it.next().unwrap()),
                (at + open, at + e + 1),
            )
        };
        let (start, start_at) = xy("start");
        let (end, end_at) = xy("end");
        let n = body.find("(net \"").unwrap() + 6;
        let net = body[n..n + body[n..].find('"').unwrap()].to_owned();
        out.push(Seg {
            start,
            end,
            net,
            start_at,
            end_at,
        });
        from = at + 1;
    }
    out
}

/// §3.4: a hand-edited board with one I2C segment doubled in length raises
/// that line's trace term by the per-length bound times the added length,
/// to the femtofarad, and changes nothing else. The connectivity rule joins
/// a track only at its ends, so a segment is doubled away from an end where
/// another track's end, not a pad or a via, joins it. A doubling that runs
/// into another net's copper is refused (`ClearanceViolated`), never
/// estimated.
#[test]
fn a_longer_route_raises_the_trace_term_by_the_per_length_bound() {
    const PER_MM_FF: u64 = 280;
    let text = reference();
    let (_, base) = extract("base", text, SPEC);
    let (mut doubled, mut exact, mut clearance) = (0, 0, 0);
    for (i, s) in segments(text).iter().enumerate() {
        if !s.net.starts_with("/I2C_") {
            continue;
        }
        let far = |a: (f64, f64), b: (f64, f64)| (2.0 * b.0 - a.0, 2.0 * b.1 - a.1);
        for (head, (from, to), at) in [
            ("end", (s.start, s.end), s.end_at),
            ("start", (s.end, s.start), s.start_at),
        ] {
            let moved = far(from, to);
            let edited = format!(
                "{}({head} {} {}){}",
                &text[..at.0],
                moved.0,
                moved.1,
                &text[at.1..]
            );
            let (code, v) = extract(&format!("long-{i}-{head}"), &edited, SPEC);
            if code == 3 && v["error"]["kind"] == "ClearanceViolated" {
                clearance += 1;
            }
            if code != 0 {
                continue;
            }
            let k = usize::from(s.net == "/I2C_SCL");
            let (old, new) = (&base["lines"][k], &v["lines"][k]);
            let length = |l: &Value| l["length_nm"].as_u64().unwrap();
            let added = length(new) - length(old);
            assert_eq!(
                added as f64,
                ((s.end.0 - s.start.0).abs() + (s.end.1 - s.start.1).abs()) * 1e6
            );
            let (t_old, t_new) = (terms(old), terms(new));
            // The trace term is ⌈per-length × length⌉ plus each track's ends:
            // the rise is exactly the difference of the two ceilings, which is
            // the per-length bound times the added length to within the one
            // rounding, and exact when that product is whole femtofarads, as
            // it is for every length on the router's 0.125 mm grid (280 fF/mm
            // is 35 fF per pitch).
            let ceil = |nm: u64| (PER_MM_FF * nm).div_ceil(1_000_000);
            assert_eq!(t_new[0] - t_old[0], ceil(length(new)) - ceil(length(old)));
            assert!((t_new[0] - t_old[0]).abs_diff(PER_MM_FF * added / 1_000_000) <= 1);
            if (PER_MM_FF * added).is_multiple_of(1_000_000) {
                assert_eq!(t_new[0] - t_old[0], PER_MM_FF * added / 1_000_000);
                exact += 1;
            }
            assert_eq!(t_new[1..4], t_old[1..4], "only the trace term moves");
            assert_eq!(v["lines"][1 - k], base["lines"][1 - k]);
            doubled += 1;
        }
    }
    assert!(exact > 0, "no I2C segment could be doubled");
    assert!(clearance > 0, "no doubling ran into another net");
    eprintln!("ANSWER 1305: doubled={doubled} exact={exact} clearance_refused={clearance}");
}

/// What the bound does not model, or could read otherwise than KiCad, is
/// refused before any estimate (the adversarial pass's classes A, C3, C4
/// and E8): a repeated child (KiCad reads the last, `route::read` the
/// first), a child or item the reader does not model, a value count other
/// than the reader's, a pad with no copper layer or another shape, copper
/// outside pads, tracks and vias, and a coordinate too large to square.
#[test]
fn what_the_reader_does_not_model_is_refused() {
    let text = reference();
    let tail = text.rfind("\t(embedded_fonts").unwrap();
    let insert = |item: &str| format!("{}{item}\n{}", &text[..tail], &text[tail..]);
    let once = |from: &str, to: &str| text.replacen(from, to, 1);
    let outline = text.find("(gr_rect\n\t\t(start ").unwrap() + "(gr_rect\n\t\t".len();
    let huge = format!(
        "{}(start -9000000 0){}",
        &text[..outline],
        &text[outline + text[outline..].find(')').unwrap() + 1..]
    );
    let unmodelled = "UnmodelledCopper";
    for (label, board, kind) in [
        (
            "arc",
            insert(
                "\t(arc (start 90 90) (mid 91 91) (end 92 90) (width 0.2) (layer \"F.Cu\") (net \"/I2C_SDA\"))",
            ),
            unmodelled,
        ),
        (
            "gr_line",
            insert(
                "\t(gr_line (start 90 90) (end 92 90) (stroke (width 0.2) (type solid)) (layer \"B.Cu\"))",
            ),
            unmodelled,
        ),
        (
            "setup",
            insert("\t(setup (stackup (layer \"F.Cu\" (type \"copper\") (thickness 0.105))))"),
            unmodelled,
        ),
        (
            "fp_copper",
            once(
                "\t\t(attr ",
                "\t\t(fp_rect (start 0 0) (end 0.1 0.1) (layer \"F.Cu\"))\n\t\t(attr ",
            ),
            unmodelled,
        ),
        (
            "segment_width_twice",
            once(
                "\t\t(width 0.2)\n\t\t(layer",
                "\t\t(width 0.2)\n\t\t(width 2)\n\t\t(layer",
            ),
            unmodelled,
        ),
        (
            "segment_width_arity",
            once(
                "\t\t(width 0.2)\n\t\t(layer",
                "\t\t(width 0.2 2)\n\t\t(layer",
            ),
            unmodelled,
        ),
        (
            "via_size_twice",
            once("(size 0.6)", "(size 0.6)\n\t\t(size 2.4)"),
            unmodelled,
        ),
        (
            "via_padstack",
            once(
                "(size 0.6)",
                "(size 0.6)\n\t\t(padstack (mode front_inner_back) (layer \"B.Cu\" (size 2.4)))",
            ),
            unmodelled,
        ),
        (
            "pad_no_layers",
            once("\t\t\t(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")\n", ""),
            unmodelled,
        ),
        (
            "pad_user_layer",
            once(
                "(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")",
                "(layers \"TOP\" \"F.Mask\" \"F.Paste\")",
            ),
            unmodelled,
        ),
        (
            "pad_circle",
            once("smd roundrect", "smd circle"),
            unmodelled,
        ),
        (
            "pad_drill_offset",
            once(
                "(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")",
                "(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")\n\t\t\t(drill 0 (offset 0 2.5))",
            ),
            unmodelled,
        ),
        (
            "footprint_at_twice",
            once("\t\t(descr ", "\t\t(at 0 0)\n\t\t(descr "),
            unmodelled,
        ),
        (
            "property_twice",
            once(
                "\t\t(attr ",
                "\t\t(property \"Value\" \"100n\")\n\t\t(attr ",
            ),
            unmodelled,
        ),
        (
            "thickness_twice",
            once("(thickness 1.6)", "(thickness 1.6)\n\t\t(thickness 0.2)"),
            unmodelled,
        ),
        (
            "zone_fill",
            once(
                "\t\t(min_thickness 0.25)",
                "\t\t(min_thickness 0.25)\n\t\t(filled_polygon (layer \"B.Cu\") (pts (xy 0 0) (xy 1 0) (xy 1 1)))",
            ),
            unmodelled,
        ),
        (
            "zone_clearance",
            once(
                "\t\t(min_thickness 0.25)",
                "\t\t(min_thickness 0.25)\n\t\t(connect_pads (clearance 0.01))",
            ),
            unmodelled,
        ),
        (
            "thickness",
            once("(thickness 1.6)", "(thickness 1.0)"),
            "BoardThickness",
        ),
        ("huge", huge, "BoardTooLarge"),
        (
            "outline_layer_twice",
            once(
                "\t\t(layer \"Edge.Cuts\")\n",
                "\t\t(layer \"Edge.Cuts\")\n\t\t(layer \"F.Cu\")\n",
            ),
            unmodelled,
        ),
        (
            "general_twice",
            insert("\t(general (thickness 0.2))"),
            unmodelled,
        ),
        (
            "footprint_unknown_child",
            once(
                "\t\t(attr ",
                "\t\t(fp_circle (center 0 0) (end 0.1 0) (layer \"F.SilkS\"))\n\t\t(attr ",
            ),
            unmodelled,
        ),
        ("garbage", "(kicad_pcb (".to_owned(), "BoardUnreadable"),
    ] {
        assert_ne!(board, text, "{label}: the edit did not apply");
        let (code, v) = extract(label, &board, SPEC);
        assert_eq!(code, 3, "{label}: {v}");
        assert_eq!(refused(&v), kind, "{label}: {v}");
    }
}

/// The parts on each line are exactly the specification's design: each by
/// reference, footprint and value, each with exactly its pads there (the
/// adversarial pass's class B). A part renamed, a pull-up given another
/// value, a declared device missing, or a part the specification does not
/// put there is refused.
#[test]
fn the_parts_on_a_line_are_the_designs() {
    let text = reference();
    // R1's pad on SDA, numbered 3: its copper is where the design's pad 2 is.
    let r1 = text.find("(property \"Reference\" \"R1\"").unwrap();
    let pad = r1 + text[r1..].find("(pad \"2\"").unwrap();
    let renumbered = format!(
        "{}(pad \"3\"{}",
        &text[..pad],
        &text[pad + "(pad \"2\"".len()..]
    );
    for (label, board, kind) in [
        (
            "renamed",
            text.replacen(
                "(property \"Reference\" \"R1\"",
                "(property \"Reference\" \"C99\"",
                1,
            ),
            "DeviceNotOnLine",
        ),
        ("pads", renumbered, "PartMismatch"),
        (
            "value",
            text.replacen(
                "(property \"Value\" \"2k2\"",
                "(property \"Value\" \"100n\"",
                1,
            ),
            "PartMismatch",
        ),
    ] {
        assert_ne!(board, text, "{label}: the edit did not apply");
        let (code, v) = extract(label, &board, SPEC);
        assert_eq!(code, 3, "{label}: {v}");
        assert_eq!(refused(&v), kind, "{label}: {v}");
    }

    let missing = spec_with("missing", |v| {
        v["i2c_devices"][1]["reference"] = "U9".into();
    });
    let (code, v) = extract("missing", text, missing.to_str().unwrap());
    assert_eq!(code, 3, "{v}");
    assert_eq!(refused(&v), "DeviceNotOnLine");

    let unaccounted = spec_with("unaccounted", |v| {
        v["i2c_devices"].as_array_mut().unwrap().pop();
    });
    let (code, v) = extract("unaccounted", text, unaccounted.to_str().unwrap());
    assert_eq!(code, 3, "{v}");
    assert_eq!(refused(&v), "UnmodelledPart");

    let renamed = spec_with("line", |v| {
        v["i2c_buses"][0]["sda"]["net"] = "I2C_DATA".into();
        v["net_constraints"][0]["net_name"] = "I2C_DATA".into();
    });
    let (code, v) = extract("line", text, renamed.to_str().unwrap());
    assert_eq!(code, 3, "{v}");
    assert_eq!(refused(&v), "LineNotOnBoard");

    // A specification `solve` accepts and the design refuses is exit 2.
    let address = spec_with("address", |v| {
        v["i2c_devices"][0]["address"] = 80.into();
    });
    let (code, v) = extract("address", text, address.to_str().unwrap());
    assert_eq!(code, 2, "{v}");
    assert_eq!(refused(&v), "UnsupportedAddress");
}
