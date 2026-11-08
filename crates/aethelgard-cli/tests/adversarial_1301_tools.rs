//! AET-WO-1301 §5, the adversarial pass on `check drc --schematic` and the
//! placed board, tier 2: KiCad 10.0.7 judges. Every row was first measured
//! in the tools image (`container/IMAGE`, `--network none`) at 6b55cb3 with
//! the host-built CLI. A test marked **red until fixed** pins an attack that
//! succeeded there and asserts the secure outcome; the rest pin attacks the
//! gate withstood, each with exactly the finding kinds KiCad gave.
#![cfg(feature = "with-tools")]

use std::collections::BTreeSet;
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
    /// On disk (target/tmp), not in RAM-backed /tmp.
    fn new(tag: &str) -> Temp {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-adv-1301-{tag}"));
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
        .write_all(format!("AET-WO-1301 adversarial {line}\n").as_bytes());
}

/// The CLI's exit code and its one JSON line.
fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
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

fn emit_to(spec: &Path, out: &Path) -> Value {
    let (code, v) = run(&[
        "emit",
        "--file",
        spec.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{v}");
    v
}

/// Writes `board` and `sch` into `dir` and runs `check drc --spec --schematic`.
fn drc(dir: &Path, board: &str, sch: &str, spec: &Path) -> (i32, Value) {
    fs::create_dir_all(dir).unwrap();
    let (b, s) = (dir.join("b.kicad_pcb"), dir.join("s.kicad_sch"));
    fs::write(&b, board).unwrap();
    fs::write(&s, sch).unwrap();
    run(&[
        "check",
        "drc",
        "--board",
        b.to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
        "--schematic",
        s.to_str().unwrap(),
    ])
}

/// The kinds of every finding but `unconnected_items`.
fn beyond(v: &Value) -> BTreeSet<String> {
    v["findings"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|f| f["kind"].as_str().unwrap().to_owned())
                .filter(|k| k != "unconnected_items")
                .collect()
        })
        .unwrap_or_default()
}

/// A verdict that would let the board through once routed: the gate judged
/// it and found nothing but unconnected items.
fn passes_parity(code: i32, v: &Value) -> bool {
    matches!(code, 0 | 1) && v["verdict"].is_string() && beyond(v).is_empty()
}

/// The emitted reference design: (board, schematic, spec).
fn reference(t: &Temp) -> (String, String, PathBuf) {
    let spec = example("sentinel-01-minimal.json");
    let r = emit_to(&spec, &t.0.join("ref"));
    let read = |k: &str| fs::read_to_string(r[k]["path"].as_str().unwrap()).unwrap();
    (read("board"), read("schematic"), spec)
}

/// The byte span of the footprint whose reference is `reference`, in the
/// layout `sexpr::write` gives the emitted board.
fn fp_span(board: &str, reference: &str) -> (usize, usize) {
    let needle = format!("(property \"Reference\" \"{reference}\"");
    let mut from = 0;
    while let Some(i) = board[from..].find("\n\t(footprint ") {
        let s = from + i + 1;
        let e = s + board[s..].find("\n\t)\n").unwrap() + 3;
        if board[s..e].contains(&needle) {
            return (s, e);
        }
        from = e;
    }
    panic!("no footprint {reference}");
}

fn edit_fp(board: &str, reference: &str, f: impl Fn(&str) -> String) -> String {
    let (s, e) = fp_span(board, reference);
    let new = f(&board[s..e]);
    assert_ne!(new, board[s..e], "{reference}: nothing changed");
    format!("{}{new}{}", &board[..s], &board[e..])
}

/// R5 copied as `new_ref` to a spot measured free on the reference board,
/// with fresh UUIDs, its attributes `attr` and its value `value`.
fn r5_copy(board: &str, new_ref: &str, attr: &str, value: &str) -> String {
    let (s, e) = fp_span(board, "R5");
    let mut fp = board[s..e]
        .replace(
            "(property \"Reference\" \"R5\"",
            &format!("(property \"Reference\" \"{new_ref}\""),
        )
        .replace("(attr smd)", &format!("({attr})"))
        .replace(
            "(property \"Value\" \"10k\"",
            &format!("(property \"Value\" \"{value}\""),
        );
    let at = fp.find("\t\t(at ").unwrap();
    let at_end = at + fp[at..].find(')').unwrap() + 1;
    fp.replace_range(at..at_end, "\t\t(at 115 105)");
    let mut out = String::new();
    let mut rest = fp.as_str();
    let mut n = 0;
    while let Some(i) = rest.find("(uuid \"") {
        n += 1;
        out.push_str(&rest[..i]);
        out.push_str(&format!("(uuid \"00000000-0000-4000-8000-{n:012}\")"));
        rest = &rest[i + rest[i..].find(')').unwrap() + 1..];
    }
    out.push_str(rest);
    let i = board.find("\t(gr_rect").unwrap();
    format!("{}{out}{}", &board[..i], &board[i..])
}

/// **Red until fixed.** KiCad's parity check skips a `board_only`
/// footprint: an extra part with real copper on the schematic's nets, or a
/// second R5 valued 0R, each judged with nothing beyond `unconnected_items`
/// (65 → 67). The secure outcome is either layer's: the gate refuses it
/// (exit 3) or its verdict carries a finding beyond the unconnected items.
#[test]
fn attack_a_board_only_footprint_does_not_pass_parity() {
    let t = Temp::new("board-only");
    let (board, sch, spec) = reference(&t);
    for (name, b) in [
        (
            "extra R99",
            r5_copy(&board, "R99", "attr smd board_only", "10k"),
        ),
        (
            "second R5 at 0R",
            r5_copy(&board, "R5", "attr smd board_only", "0R"),
        ),
    ] {
        let (code, v) = drc(&t.0.join(name.replace(' ', "-")), &b, &sch, &spec);
        record(&format!(
            "board_only {name}: exit {code}, beyond {:?}",
            beyond(&v)
        ));
        assert!(
            !passes_parity(code, &v),
            "{name}: a board_only footprint passed parity: {v}"
        );
    }
}

/// **Red until fixed.** KiCad accepts a pad net that starts with
/// `unconnected-` and with the schematic's net for the pin, so two pins the
/// schematic leaves unconnected can share one net when one's KiCad name is a
/// prefix of the other's. `RUL-14` admits `)` in a reference: devices `T`
/// and `T-ALERT-Pad3)` name their ALERT pins `unconnected-(T-ALERT-Pad3)`
/// and `unconnected-(T-ALERT-Pad3)-ALERT-Pad3)`. Measured: with T's pad 3 on
/// the second name, nothing beyond `unconnected_items` (72 → 73, the join
/// awaiting a track). The gate could refuse an `unconnected-` net on more
/// than one pad, which KiCad never names so.
#[test]
fn attack_two_unconnected_pins_joined_on_one_net_do_not_pass_parity() {
    let t = Temp::new("nc-join");
    let mut s: Value =
        serde_json::from_slice(&fs::read(example("sentinel-01-minimal.json")).unwrap()).unwrap();
    s["i2c_devices"][1]["reference"] = "T".into();
    s["i2c_devices"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "reference": "T-ALERT-Pad3)", "part": "TMP117", "supply": "VDD_3V3",
            "bus": "SENSORS", "address": 73
        }));
    let spec = t.0.join("spec.json");
    fs::write(&spec, serde_json::to_vec(&s).unwrap()).unwrap();
    let r = emit_to(&spec, &t.0.join("o"));
    let read = |k: &str| fs::read_to_string(r[k]["path"].as_str().unwrap()).unwrap();
    let (board, sch) = (read("board"), read("schematic"));
    let (code, v) = drc(&t.0.join("emitted"), &board, &sch, &spec);
    assert!(passes_parity(code, &v), "the emitted board: {v}");
    let joined = board.replace(
        "(net \"unconnected-(T-ALERT-Pad3)\")",
        "(net \"unconnected-(T-ALERT-Pad3)-ALERT-Pad3)\")",
    );
    assert_ne!(joined, board);
    let (code, v) = drc(&t.0.join("joined"), &joined, &sch, &spec);
    record(&format!("nc join: exit {code}, beyond {:?}", beyond(&v)));
    assert!(
        !passes_parity(code, &v),
        "two unconnected pins on one net passed parity: {v}"
    );
}

/// **Red until fixed.** Twenty-four I2C devices whose references are 64
/// bytes of `m`, KiCad's widest glyph: the emitted board must carry nothing
/// beyond unconnected items, as the reference board does. Measured: 9
/// `silk_edge_clearance`, 2 `silk_overlap`, 20 `silk_over_copper`. The same
/// parts with short references are the control, measured clean.
#[test]
fn attack_wide_references_leave_the_emitted_board_clean() {
    let t = Temp::new("wide");
    let mut s: Value =
        serde_json::from_slice(&fs::read(example("sentinel-01-minimal.json")).unwrap()).unwrap();
    let (mut buses, mut devices, mut constraints) = (vec![], vec![], vec![]);
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
            devices.push(serde_json::json!({
                "reference": "", "part": part, "supply": "VDD_3V3",
                "bus": format!("B{b}"), "address": address
            }));
        }
    }
    s["i2c_buses"] = buses.into();
    s["net_constraints"] = constraints.into();
    for (name, wide) in [("short", false), ("wide", true)] {
        for (i, d) in devices.iter_mut().enumerate() {
            // `U01` would be KiCad's `U1` (see the tier-1 reference test),
            // so the short control has no leading zero; the wide names end
            // in `m`, so KiCad reads no number in them.
            d["reference"] = if wide {
                format!("U{i:02}{}", "m".repeat(61))
            } else {
                format!("S{i}")
            }
            .into();
        }
        s["i2c_devices"] = devices.clone().into();
        let spec = t.0.join(format!("{name}.json"));
        fs::write(&spec, serde_json::to_vec(&s).unwrap()).unwrap();
        let r = emit_to(&spec, &t.0.join(name));
        let read = |k: &str| fs::read_to_string(r[k]["path"].as_str().unwrap()).unwrap();
        let (code, v) = drc(
            &t.0.join(format!("{name}-drc")),
            &read("board"),
            &read("schematic"),
            &spec,
        );
        record(&format!(
            "{name} references: exit {code}, beyond {:?}",
            beyond(&v)
        ));
        assert!(passes_parity(code, &v), "{name}: {:?}", beyond(&v));
    }
}

/// A limit of KiCad's parity check, pinned as measured (FEEDBACK A5, for a
/// ruling; low). Board-side differences it does not see, each with nothing
/// beyond `unconnected_items`: a field the symbol does not have, an empty
/// field (`Datasheet ""`) removed, the footprint's `(path …)` pointing at no
/// symbol, `exclude_from_pos_files`, and an unconnected pin's single-pad net
/// renamed under its own `unconnected-(…)` prefix. None changes
/// connectivity. Closing them needs the gate's own field and path
/// comparison, beyond `AET-WO-1301`; if KiCad starts to see one, this test
/// says so.
#[test]
fn kicad_parity_does_not_see_metadata_only_differences() {
    let t = Temp::new("metadata");
    let (board, sch, spec) = reference(&t);
    let rows: [(&str, String); 5] = [
        (
            "extra field",
            edit_fp(&board, "C1", |f| {
                f.replacen(
                    "\t\t(path ",
                    "\t\t(property \"Voltage\" \"50V\"\n\t\t\t(at 0 0 0)\n\t\t\t(layer \"F.Fab\")\n\t\t\t(hide yes)\n\t\t\t(effects (font (size 1 1) (thickness 0.15)))\n\t\t)\n\t\t(path ",
                    1,
                )
            }),
        ),
        (
            "empty field removed",
            edit_fp(&board, "C1", |f| {
                let s = f.find("\t\t(property \"Datasheet\" \"\"").unwrap();
                let e = s + f[s..].find("\n\t\t)\n").unwrap() + 5;
                format!("{}{}", &f[..s], &f[e..])
            }),
        ),
        (
            "path changed",
            edit_fp(&board, "R5", |f| {
                let s = f.find("(path \"/").unwrap();
                let e = s + f[s..].find(')').unwrap() + 1;
                format!(
                    "{}(path \"/00000000-0000-0000-0000-000000000000\"){}",
                    &f[..s],
                    &f[e..]
                )
            }),
        ),
        (
            "exclude_from_pos_files",
            edit_fp(&board, "R5", |f| {
                f.replace("(attr smd)", "(attr smd exclude_from_pos_files)")
            }),
        ),
        (
            "unconnected net suffixed",
            board.replace(
                "(net \"unconnected-(U3-ALERT-Pad3)\")",
                "(net \"unconnected-(U3-ALERT-Pad3)-joined\")",
            ),
        ),
    ];
    let mut seen = Vec::new();
    for (i, (name, b)) in rows.into_iter().enumerate() {
        assert_ne!(b, board, "{name}");
        let (code, v) = drc(&t.0.join(format!("row{i}")), &b, &sch, &spec);
        record(&format!(
            "metadata {name}: exit {code}, beyond {:?}",
            beyond(&v)
        ));
        if !passes_parity(code, &v) {
            seen.push(name);
        }
    }
    assert!(
        seen.is_empty(),
        "KiCad's parity now sees {seen:?}; revisit A5"
    );
}

/// Withstood: each board or schematic that differs from the other fails
/// with exactly the finding kinds KiCad gave when measured.
#[test]
fn parity_fails_every_other_mismatch_with_kicads_finding() {
    let t = Temp::new("withstood");
    let (board, sch, spec) = reference(&t);
    let set = |k: &[&str]| k.iter().map(|s| (*s).to_owned()).collect::<BTreeSet<_>>();
    let swap_pads = |f: &str| {
        let mut nets = f.match_indices("(net \"").map(|(i, _)| {
            let s = i + 6;
            f[s..s + f[s..].find('"').unwrap()].to_owned()
        });
        let (a, b) = (nets.next().unwrap(), nets.next().unwrap());
        f.replace("(pad \"1\" smd", "(pad \"@@\" smd")
            .replace("(pad \"2\" smd", "(pad \"1\" smd")
            .replace("(pad \"@@\" smd", "(pad \"2\" smd")
            .replace(&format!("(net \"{a}\")"), "(net \"@@\")")
            .replace(&format!("(net \"{b}\")"), &format!("(net \"{a}\")"))
            .replace("(net \"@@\")", &format!("(net \"{b}\")"))
    };
    let without = |b: &str, r: &str| {
        let (s, e) = fp_span(b, r);
        format!("{}{}", &b[..s], &b[e..])
    };
    let on_board_no = {
        let i = sch.find("(property \"Reference\" \"R5\"").unwrap();
        let j = sch[..i].rfind("(on_board yes)").unwrap();
        format!(
            "{}(on_board no){}",
            &sch[..j],
            &sch[j + "(on_board yes)".len()..]
        )
    };
    let empty_sch = "(kicad_sch\n\t(version 20260306)\n\t(generator \"eeschema\")\n\t(generator_version \"10.0\")\n\t(uuid \"11111111-1111-4111-8111-111111111111\")\n\t(paper \"A4\")\n)\n".to_owned();
    let label = sch.find("(label \"I2C_SDA\"\n").unwrap() + "(label \"I2C_SDA\"\n".len();
    let netclass_sch = format!(
        "{}\t\t(property \"Netclass\" \"HV\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n{}",
        &sch[..label],
        &sch[label..]
    );
    let rows: Vec<(&str, String, String, BTreeSet<String>)> = vec![
        (
            "values swapped R1/R3",
            edit_fp(
                &edit_fp(&board, "R1", |f| {
                    f.replace("\"Value\" \"2k2\"", "\"Value\" \"10k\"")
                }),
                "R3",
                |f| f.replace("\"Value\" \"10k\"", "\"Value\" \"2k2\""),
            ),
            sch.clone(),
            set(&["footprint_symbol_mismatch"]),
        ),
        (
            "field altered",
            edit_fp(&board, "C1", |f| {
                f.replace("\"Unpolarized capacitor\"", "\"Polarized capacitor\"")
            }),
            sch.clone(),
            set(&["footprint_symbol_field_mismatch"]),
        ),
        (
            "library id not in the library",
            edit_fp(&board, "R1", |f| {
                f.replacen("R_0402_1005Metric\"", "R_0402_1005Metric_X\"", 1)
            }),
            sch.clone(),
            set(&["lib_footprint_issues", "footprint_symbol_mismatch"]),
        ),
        (
            "library id of another vendored footprint",
            edit_fp(&board, "R1", |f| {
                f.replacen(
                    "\"Resistor_SMD:R_0402_1005Metric\"",
                    "\"Capacitor_SMD:C_0402_1005Metric\"",
                    1,
                )
            }),
            sch.clone(),
            set(&["lib_footprint_mismatch", "footprint_symbol_mismatch"]),
        ),
        (
            "pads 1 and 2 swapped with their nets",
            edit_fp(&board, "R1", swap_pads),
            sch.clone(),
            set(&["lib_footprint_mismatch"]),
        ),
        (
            "pad renumbered",
            edit_fp(&board, "R1", |f| {
                f.replacen("(pad \"1\" smd", "(pad \"1A\" smd", 1)
            }),
            sch.clone(),
            set(&["lib_footprint_mismatch", "net_conflict"]),
        ),
        (
            "pad resized",
            edit_fp(&board, "R1", |f| {
                f.replacen("(size 0.54 0.64)", "(size 0.2 0.2)", 1)
            }),
            sch.clone(),
            set(&["lib_footprint_mismatch"]),
        ),
        (
            "pad made a bare hole",
            edit_fp(&board, "R1", |f| {
                f.replacen(
                    "(pad \"2\" smd roundrect",
                    "(pad \"2\" np_thru_hole circle (drill 0.3)",
                    1,
                )
            }),
            sch.clone(),
            set(&["lib_footprint_mismatch", "net_conflict"]),
        ),
        (
            "unconnected pin's pad deleted",
            edit_fp(&board, "U3", |f| {
                let s = f.find("\t\t(pad \"3\" smd").unwrap();
                let e = s + f[s..].find("\n\t\t)\n").unwrap() + 5;
                format!("{}{}", &f[..s], &f[e..])
            }),
            sch.clone(),
            set(&["lib_footprint_mismatch", "net_conflict"]),
        ),
        (
            "reference changed",
            edit_fp(&board, "R5", |f| {
                f.replace("\"Reference\" \"R5\"", "\"Reference\" \"R9\"")
            }),
            sch.clone(),
            set(&["missing_footprint", "extra_footprint"]),
        ),
        (
            "reference in lower case",
            edit_fp(&board, "R5", |f| {
                f.replace("\"Reference\" \"R5\"", "\"Reference\" \"r5\"")
            }),
            sch.clone(),
            set(&["missing_footprint", "extra_footprint"]),
        ),
        (
            "footprint missing",
            without(&board, "R5"),
            sch.clone(),
            set(&["missing_footprint"]),
        ),
        (
            "extra footprint",
            r5_copy(&board, "R99", "attr smd", "10k"),
            sch.clone(),
            set(&["extra_footprint"]),
        ),
        (
            "duplicate reference",
            r5_copy(&board, "R5", "attr smd", "10k"),
            sch.clone(),
            set(&["duplicate_footprints"]),
        ),
        (
            "do not populate on the board only",
            edit_fp(&board, "R5", |f| f.replace("(attr smd)", "(attr smd dnp)")),
            sch.clone(),
            set(&["footprint_symbol_mismatch"]),
        ),
        (
            "excluded from the BOM on the board only",
            edit_fp(&board, "R5", |f| {
                f.replace("(attr smd)", "(attr smd exclude_from_bom)")
            }),
            sch.clone(),
            set(&["footprint_symbol_mismatch"]),
        ),
        (
            "schematic: R5 not on the board",
            board.clone(),
            on_board_no,
            set(&["extra_footprint"]),
        ),
        (
            "schematic: empty",
            board.clone(),
            empty_sch,
            set(&["extra_footprint"]),
        ),
        (
            "schematic: a netclass on a label carries no rule in",
            board.clone(),
            netclass_sch,
            set(&[]),
        ),
    ];
    for (i, (name, b, s, kinds)) in rows.into_iter().enumerate() {
        assert!(b != board || s != sch, "{name}: nothing changed");
        let (code, v) = drc(&t.0.join(format!("row{i}")), &b, &s, &spec);
        record(&format!(
            "withstood {name}: exit {code}, beyond {:?}",
            beyond(&v)
        ));
        assert_eq!(code, 1, "{name}: {v}");
        assert_eq!(beyond(&v), kinds, "{name}");
    }
    // Two unconnected pins on one KiCad name: measured at 6b55cb3 as KiCad's
    // `net_conflict`; the gate now refuses it before KiCad runs (F2's fix).
    let joined = board.replacen(
        "(net \"unconnected-(U1-PE2-Pad1)\")",
        "(net \"unconnected-(U3-ALERT-Pad3)\")",
        1,
    );
    assert_ne!(joined, board);
    let (code, v) = drc(&t.0.join("joined"), &joined, &sch, &spec);
    record(&format!(
        "withstood two unconnected pins on one KiCad name: exit {code}"
    ));
    assert_eq!(code, 3, "{v}");
    assert_eq!(v["error"]["kind"], "SharedUnconnectedNet", "{v}");
}

/// Withstood: a schematic the gate's reader accepts but KiCad will not load
/// is no verdict (exit 5, `ToolError`, KiCad writes no report), never a
/// pass. That includes a sheet spelled `Sheet` or `"sheet"`, which the
/// gate's `(sheet` scan does not match and KiCad does not read as one.
#[test]
fn a_schematic_kicad_cannot_load_is_no_verdict() {
    let t = Temp::new("unloadable");
    let (board, sch, spec) = reference(&t);
    let anchor = "\t(sheet_instances";
    assert!(sch.contains(anchor));
    let sheet = |head: &str| {
        sch.replacen(
            anchor,
            &format!(
                "\t({head} (at 10 10) (size 10 10) (fields_autoplaced yes) (stroke (width 0.1524) (type solid)) (fill (color 0 0 0 0.0)) (uuid \"22222222-2222-4222-8222-222222222222\") (property \"Sheetname\" \"x\" (at 10 9 0) (effects (font (size 1.27 1.27)))) (property \"Sheetfile\" \"input.kicad_pcb\" (at 10 21 0) (effects (font (size 1.27 1.27)))))\n{anchor}"
            ),
            1,
        )
    };
    let version = sch.find("\t(version ").unwrap();
    let version_end = version + sch[version..].find('\n').unwrap() + 1;
    let rows = [
        ("bare", "(kicad_sch)\n".to_owned()),
        (
            "a future version",
            format!(
                "{}\t(version 29991231)\n{}",
                &sch[..version],
                &sch[version_end..]
            ),
        ),
        (
            "no version",
            format!("{}{}", &sch[..version], &sch[version_end..]),
        ),
        ("a sheet spelled Sheet", sheet("Sheet")),
        ("a sheet spelled \"sheet\"", sheet("\"sheet\"")),
    ];
    for (i, (name, s)) in rows.into_iter().enumerate() {
        let (code, v) = drc(&t.0.join(format!("row{i}")), &board, &s, &spec);
        record(&format!(
            "unloadable {name}: exit {code}, {}",
            v["error"]["kind"]
        ));
        assert_eq!(code, 5, "{name}: {v}");
        assert_eq!(v["error"]["kind"], "ToolError", "{name}: {v}");
    }
}
