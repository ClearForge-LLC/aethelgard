//! AET-WO-1300 §5, the adversarial pass through the CLI, as regression tests.
//! Each `attack_*` case was measured against KiCad 10.0.7 in the tools image
//! and failed open before the fix; each asserts the secure outcome.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::sexpr;
use serde_json::{Value, json};

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
            "aet-adv-{tag}-{}-{}",
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
    serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
        panic!(
            "stdout {stdout:?} stderr {}",
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn sentinel() -> Value {
    serde_json::from_slice(&fs::read(example("sentinel-01-minimal.json")).unwrap()).unwrap()
}

fn emit(spec: &Value, d: &Path, out: &str) -> (Output, PathBuf) {
    let p = d.join(format!("{out}.json"));
    fs::write(&p, serde_json::to_vec(spec).unwrap()).unwrap();
    let o = d.join(out);
    let r = cli()
        .args([
            "emit",
            "--file",
            p.to_str().unwrap(),
            "--out",
            o.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    (r, o)
}

/// A stand-in `kicad-cli`: `version` prints 10.0.7; a run executes
/// `checks` (with `$last` the board copy, `$d` its directory) and then
/// writes a clean DRC report and exits 0.
fn checking_kicad(dir: &Path, checks: &str) -> PathBuf {
    let tb = dir.join("toolbox");
    fs::create_dir_all(&tb).unwrap();
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = version ]; then echo 10.0.7; exit 0; fi
out=; prev=; last=
for a in "$@"; do
  if [ "$prev" = -o ]; then out=$a; fi
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

// ---- the DRC gate, end to end ----

/// The board's own setup (board minimum 0.01 mm) and a pad's 0.01 mm local
/// clearance survive into the copy KiCad judges, behind `#` comment lines,
/// and the verdict says `"rules": "engine-floor"`.
#[test]
fn attack_a_board_hiding_its_rules_behind_hash_comments_gets_an_engine_floor_verdict() {
    let d = Dir::new("hash");
    let board = d.0.join("hidden.kicad_pcb");
    fs::write(
        &board,
        "(kicad_pcb\n\t(version 20241229)\n\t(generator \"pcbnew\")\n#\"\n\t(setup (pad_to_mask_clearance 0) (clearance_min 0.01))\n#\"\n\t(footprint \"R:R\"\n\t\t(layer \"F.Cu\")\n\t\t(property \"Reference\" \"R7\")\n#\"\n\t\t(pad 1 smd rect (at 0 0) (size 1 1) (layers F.Cu) (clearance 0.01))\n#\"\n\t)\n\t(net 0 \"\")\n)\n",
    )
    .unwrap();
    // The stand-in refuses to judge unless the board's rules reached it.
    let tb = checking_kicad(
        &d.0,
        r#"grep -q '^	(setup (pad_to_mask_clearance 0) (clearance_min 0.01))$' "$last" || exit 71
grep -q '(clearance 0.01))$' "$last" || exit 72"#,
    );
    let out = cli()
        .args([
            "check",
            "drc",
            "--board",
            board.to_str().unwrap(),
            "--spec",
            example("sentinel-01-minimal.json").to_str().unwrap(),
        ])
        .env("PATH", &tb)
        .output()
        .unwrap();
    let v = json_line(&out);
    assert_eq!(
        out.status.code(),
        Some(3),
        "the gate judged a board whose own rules reached KiCad: {v}"
    );
}

#[test]
fn withstood_a_board_symlink_and_its_sidecars_never_reach_the_judge() {
    let d = Dir::new("sidecar");
    let real = d.0.join("real");
    fs::create_dir(&real).unwrap();
    fs::write(real.join("b.kicad_pcb"), "(kicad_pcb (version 20241229))\n").unwrap();
    fs::write(
        real.join("b.kicad_pro"),
        "{\"board\":{\"design_settings\":{\"rules\":{\"min_clearance\":0.01}}}}",
    )
    .unwrap();
    fs::write(
        real.join("b.kicad_dru"),
        "(version 1)\n(rule own (constraint clearance (min 0.01mm)))\n",
    )
    .unwrap();
    fs::write(
        real.join("other.kicad_dru"),
        "(version 1)\n(rule own (constraint clearance (min 0.01mm)))\n",
    )
    .unwrap();
    // The board is reached through a symlink whose own directory holds a
    // symlinked project named after the link.
    std::os::unix::fs::symlink(real.join("b.kicad_pcb"), d.0.join("link.kicad_pcb")).unwrap();
    std::os::unix::fs::symlink(real.join("b.kicad_pro"), d.0.join("link.kicad_pro")).unwrap();
    let tb = checking_kicad(
        &d.0,
        r#"grep -q 'own\|0.01' "$d/input.kicad_pro" "$d/input.kicad_dru" "$last" && exit 71
[ "$(ls "$d" | sort | tr '\n' ' ')" = "fp-lib-table input.kicad_dru input.kicad_pcb input.kicad_pro lib sym-lib-table " ] || exit 72"#,
    );
    let out = cli()
        .args([
            "check",
            "drc",
            "--board",
            d.0.join("link.kicad_pcb").to_str().unwrap(),
        ])
        .env("PATH", &tb)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", json_line(&out));
}

// ---- names ----

/// A device reference in the emitter's own power and flag namespaces. The
/// sheet then holds two symbols with one reference, and `#` marks a symbol
/// virtual to KiCad. Measured on 10.0.7: ERC passes for `#PWR001` and
/// `#U9`, and the part is absent from `kicad-cli sch export netlist` and
/// `sch export bom` (14 rows against the emitter's 15).
#[test]
fn attack_a_reference_in_the_emitters_hash_namespace_is_emitted() {
    let d = Dir::new("hashref");
    for reference in ["#PWR001", "#FLG001", "#U9"] {
        let mut spec = sentinel();
        spec["i2c_devices"][1]["reference"] = json!(reference);
        let (out, o) = emit(&spec, &d.0, "o");
        if out.status.code() == Some(0) {
            let text = fs::read_to_string(o.join("sentinel-01.kicad_sch")).unwrap();
            let sch = sexpr::parse(&text).unwrap();
            let n = sch
                .children("symbol")
                .filter(|s| s.property("Reference") == Some(reference))
                .count();
            panic!("`{reference}` emitted; {n} symbol(s) carry that reference");
        }
        assert_eq!(out.status.code(), Some(2));
    }
}

/// Every identifier character RUL-14 admits, in the project name, the
/// references and the nets: the schematic parses back to the same names
/// and every BOM row reads back to five fields.
#[test]
fn withstood_every_admitted_identifier_character_round_trips_through_sch_and_bom() {
    let admitted: String = (0x21u8..=0x7e)
        .filter(|b| !b"/{}$[]".contains(b))
        .map(char::from)
        .collect();
    assert_eq!(admitted.len(), 88);
    let (a, b) = admitted.split_at(44);
    let mut spec = sentinel();
    spec["project_name"] = json!(format!("p{}", &a[..40]));
    spec["i2c_devices"][0]["reference"] = json!(format!("U{a}"));
    spec["i2c_devices"][1]["reference"] = json!(format!("U{b}"));
    spec["spi_devices"][0]["reference"] = json!("U\"\\,\"");
    spec["i2c_buses"][0]["sda"]["net"] = json!(format!("N{a}"));
    spec["i2c_buses"][0]["scl"]["net"] = json!(format!("N{b}"));
    spec["net_constraints"][0]["net_name"] = json!(format!("N{a}"));
    spec["net_constraints"][1]["net_name"] = json!(format!("N{b}"));
    spec["spi_devices"][0]["cs"]["net"] = json!("\\\"(;#");
    let d = Dir::new("names");
    let (out, o) = emit(&spec, &d.0, "o");
    let v = json_line(&out);
    assert_eq!(out.status.code(), Some(0), "{v}");
    let project = spec["project_name"].as_str().unwrap();
    let text = fs::read_to_string(o.join(format!("{project}.kicad_sch"))).unwrap();
    // No raw control byte and no line that starts with `#` (KiCad's comment).
    assert!(!text.bytes().any(|b| b < 0x20 && b != b'\n' && b != b'\t'));
    assert!(!text.lines().any(|l| l.trim_start().starts_with('#')));
    let sch = sexpr::parse(&text).unwrap();
    let refs: Vec<&str> = sch
        .children("symbol")
        .filter_map(|s| s.property("Reference"))
        .collect();
    for r in [format!("U{a}"), format!("U{b}"), "U\"\\,\"".into()] {
        assert!(refs.contains(&r.as_str()), "{r} not in {refs:?}");
    }
    let labels: Vec<&str> = sch.children("label").filter_map(|l| l.arg(1)).collect();
    for n in [format!("N{a}"), format!("N{b}"), "\\\"(;#".into()] {
        assert!(labels.contains(&n.as_str()), "{n} not in labels");
    }
    let title = sch
        .child("title_block")
        .and_then(|t| t.child("title"))
        .and_then(|t| t.arg(1));
    assert_eq!(title, Some(project));
    // The BOM: RFC 4180, CRLF rows, five fields each.
    let bom = fs::read_to_string(o.join(format!("{project}.bom.csv"))).unwrap();
    let rows = parse_csv(&bom);
    assert!(rows.iter().all(|r| r.len() == 5), "{rows:?}");
    let col0: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    for r in [format!("U{a}"), format!("U{b}"), "U\"\\,\"".into()] {
        assert!(col0.contains(&r.as_str()), "{r} not in {col0:?}");
    }
}

fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') => {
                assert_eq!(chars.next(), Some('\n'));
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (false, c) => {
                assert_ne!(c, '\n', "a bare LF in {text:?}");
                field.push(c);
            }
        }
    }
    assert!(field.is_empty() && row.is_empty(), "no final CRLF");
    rows
}

#[test]
fn withstood_dot_project_names_stay_inside_out() {
    let d = Dir::new("dots");
    for name in [".", "..", "...", "-", "~"] {
        let mut spec = sentinel();
        spec["project_name"] = json!(name);
        let (out, o) = emit(&spec, &d.0, "o");
        let v = json_line(&out);
        assert_eq!(out.status.code(), Some(0), "{name}: {v}");
        assert!(o.join(format!("{name}.kicad_sch")).is_file(), "{name}");
        assert_eq!(
            Path::new(v["schematic"]["path"].as_str().unwrap()).parent(),
            Some(o.as_path())
        );
    }
}

// ---- determinism ----

/// Two emissions under different environments, working directories, out
/// paths and spec byte layouts are byte-identical.
#[test]
fn withstood_emission_ignores_env_locale_time_cwd_and_spec_layout() {
    let d = Dir::new("det");
    let spec = sentinel();
    let compact = d.0.join("compact.json");
    let pretty = d.0.join("pretty.json");
    fs::write(&compact, serde_json::to_vec(&spec).unwrap()).unwrap();
    fs::write(&pretty, serde_json::to_vec_pretty(&spec).unwrap()).unwrap();
    let cwd2 = d.0.join("cwd2");
    fs::create_dir(&cwd2).unwrap();
    type Run<'a> = (&'a Path, &'a str, &'a [(&'a str, &'a str)], &'a Path);
    let runs: [Run; 3] = [
        (&compact, "a", &[], &d.0),
        (
            &pretty,
            "b/c",
            &[
                ("LC_ALL", "tr_TR.UTF-8"),
                ("TZ", "Pacific/Kiritimati"),
                ("LANG", "de_DE.UTF-8"),
            ],
            &cwd2,
        ),
        (
            &compact,
            "x",
            &[("SOURCE_DATE_EPOCH", "1"), ("HOME", "/nonexistent")],
            Path::new("/"),
        ),
    ];
    let mut outputs = Vec::new();
    for (file, out, env, cwd) in runs {
        let o = d.0.join(out);
        let r = cli()
            .env_clear()
            .env("HOME", std::env::var_os("HOME").unwrap_or_default())
            .envs(env.iter().copied())
            .current_dir(cwd)
            .args([
                "emit",
                "--file",
                file.to_str().unwrap(),
                "--out",
                o.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert_eq!(
            r.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&r.stdout)
        );
        outputs.push((
            fs::read(o.join("sentinel-01.kicad_sch")).unwrap(),
            fs::read(o.join("sentinel-01.bom.csv")).unwrap(),
        ));
    }
    assert!(outputs.windows(2).all(|w| w[0] == w[1]));
}
