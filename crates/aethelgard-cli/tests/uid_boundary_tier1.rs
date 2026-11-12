//! Tier 1 of `AET-WO-1601`: the uid boundary, the private scratch base, the
//! hierarchical-sheet refusal and the file-size cap, against a `kicad-cli`
//! stub. The stub reports what it sees as the findings of a KiCad report, so
//! each test reads the tool's own view. The suite runs as a user on a host and
//! as root in the image; each test asserts what its identity must give.

mod common;

use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::{Scratch, private_tmp};
use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_aethelgard");
const TOOL_UID: u32 = 65534;
const CAP: u64 = 64 * 1024 * 1024;

fn root() -> bool {
    rustix::process::geteuid().is_root()
}

/// A toolbox holding a `kicad-cli` stub that answers `version`, runs `probe`
/// (shell, with `$in` the copy it was handed and `$out` its report), then
/// writes an ERC report whose findings are the lines `probe` appended to
/// `$HOME/said`, and exits 5 when there are any, as KiCad does. A refusal of
/// any kind but `ToolUnusable` or `ToolError` shows no tool ran: the version
/// probe is always the first.
fn toolbox(dir: &Scratch, probe: &str) -> PathBuf {
    let tb = dir.0.join("toolbox");
    fs::create_dir_all(&tb).unwrap();
    let body = format!(
        r#"#!/bin/sh
if [ "$1" = version ]; then echo 10.0.7; exit 0; fi
out=; prev=; in=
for a in "$@"; do [ "$prev" = -o ] && out=$a; prev=$a; in=$a; done
said="$HOME/said"; : > "$said"
{probe}
v=; n=0
while IFS= read -r line; do
  [ -n "$v" ] && v="$v,"
  v="$v{{\"type\":\"probe\",\"severity\":\"error\",\"description\":\"$line\",\"items\":[]}}"
  n=$((n+1))
done < "$said"
printf '{{"$schema":"https://schemas.kicad.org/erc.v1.json","coordinate_units":"mm","date":"x","ignored_checks":[],"included_severities":["error","warning"],"kicad_version":"10.0.7","source":"s","sheets":[{{"path":"/","uuid_path":"/x","violations":[%s]}}]}}' "$v" > "$out"
[ "$n" -gt 0 ] && exit 5
exit 0
"#,
    );
    let stub = tb.join("kicad-cli");
    fs::write(&stub, body).unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    tb
}

fn schematic(dir: &Scratch, text: &str) -> PathBuf {
    let p = dir.0.join("in.kicad_sch");
    fs::write(&p, text).unwrap();
    p
}

const FLAT: &str = "(kicad_sch (version 20250114))\n";

/// `check erc` with `PATH` the toolbox, `HOME` this process's, and `TMPDIR`
/// as given (`None` leaves it unset).
fn erc(tb: &Path, sch: &Path, tmpdir: Option<&Path>, home: Option<&Path>) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(["check", "erc", "--schematic"])
        .arg(sch)
        .env_clear()
        .env("PATH", tb)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(t) = tmpdir {
        cmd.env("TMPDIR", t);
    }
    match home {
        Some(h) => cmd.env("HOME", h),
        None => cmd.env("HOME", std::env::var_os("HOME").unwrap_or_default()),
    };
    cmd.output().unwrap()
}

fn json(out: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{e}: {stdout} / {}", String::from_utf8_lossy(&out.stderr)))
}

fn refused(out: &Output, code: i32, kind: &str) -> Value {
    let v = json(out);
    assert_eq!(out.status.code(), Some(code), "{v}");
    assert_eq!(v["ok"], false, "{v}");
    assert_eq!(v["error"]["kind"], kind, "{v}");
    v
}

/// The stub's lines, from a verdict whose findings it wrote.
fn said(v: &Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"))
        .iter()
        .map(|f| f["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn a_dir(parent: &Path, name: &str, mode: u32) -> PathBuf {
    let p = parent.join(name);
    fs::DirBuilder::new().mode(mode).create(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
    p
}

#[test]
fn a_scratch_path_with_a_space_or_a_semicolon_is_refused_before_any_tool_runs() {
    let dir = Scratch::new("uid-chars");
    let tb = toolbox(&dir, "");
    let sch = schematic(&dir, FLAT);
    let base = private_tmp("chars");
    let mode = if root() { 0o711 } else { 0o700 };
    for name in ["a b", "a;b", "a'b"] {
        let tmp = a_dir(&base.0, name, mode);
        let v = refused(&erc(&tb, &sch, Some(&tmp), None), 4, "ScratchUnsafe");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("[A-Za-z0-9/._-]"),
            "{v}"
        );
        assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0, "scratch was made");
    }
}

#[test]
fn a_scratch_base_another_user_could_write_is_refused() {
    let dir = Scratch::new("uid-writable");
    let tb = toolbox(&dir, "");
    let sch = schematic(&dir, FLAT);
    // /tmp is 1777: anyone can plant /tmp/.cargo/config.toml (AET-WO-3301 F13).
    let v = refused(
        &erc(&tb, &sch, Some(Path::new("/tmp")), None),
        4,
        "ScratchUnsafe",
    );
    assert_eq!(v["error"]["id"], "/tmp", "{v}");
    // Group-writable, at the base or above it.
    let base = private_tmp("writable");
    let open = a_dir(&base.0, "open", 0o770);
    let v = refused(&erc(&tb, &sch, Some(&open), None), 4, "ScratchUnsafe");
    assert_eq!(v["error"]["id"], open.display().to_string(), "{v}");
    let below = a_dir(&open, "below", 0o700);
    let v = refused(&erc(&tb, &sch, Some(&below), None), 4, "ScratchUnsafe");
    assert_eq!(v["error"]["id"], open.display().to_string(), "{v}");
    if root() {
        // As root: a base another uid owns, and one the tool uid cannot pass.
        let theirs = a_dir(&base.0, "theirs", 0o755);
        std::os::unix::fs::chown(&theirs, Some(4242), Some(4242)).unwrap();
        let v = refused(&erc(&tb, &sch, Some(&theirs), None), 4, "ScratchUnsafe");
        assert_eq!(v["error"]["id"], theirs.display().to_string(), "{v}");
        let shut = a_dir(&base.0, "shut", 0o700);
        let v = refused(&erc(&tb, &sch, Some(&shut), None), 4, "ScratchUnsafe");
        assert_eq!(v["error"]["id"], shut.display().to_string(), "{v}");
    }
    // A base that is not there is not made.
    let gone = base.0.join("gone");
    refused(&erc(&tb, &sch, Some(&gone), None), 4, "ScratchUnsafe");
    assert!(!gone.exists());
}

#[test]
fn without_tmpdir_scratch_is_under_var_lib_as_root_and_home_otherwise() {
    let dir = Scratch::new("uid-default");
    let tb = toolbox(&dir, r#"echo "home=$HOME" >> "$said""#);
    let sch = schematic(&dir, FLAT);
    let home = private_tmp("home");
    let out = erc(&tb, &sch, None, Some(&home.0));
    let v = json(&out);
    assert_eq!(out.status.code(), Some(1), "{v}");
    let want = if root() {
        "home=/var/lib/aethelgard/aethelgard-".to_owned()
    } else {
        format!("home={}/.cache/aethelgard/aethelgard-", home.0.display())
    };
    let lines = said(&v);
    assert!(lines[0].starts_with(&want), "{lines:?} vs {want}");
    if !root() {
        // No HOME and no TMPDIR: nowhere private, so a refusal, not /tmp.
        let mut cmd = Command::new(BIN);
        let out = cmd
            .args(["check", "erc", "--schematic"])
            .arg(&sch)
            .env_clear()
            .env("PATH", &tb)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let v = refused(&out, 4, "ScratchUnsafe");
        assert_eq!(v["error"]["id"], "HOME", "{v}");
    }
}

/// The tool's ids, its sandbox's `uid_boundary`, and, as root, the modes the
/// boundary rests on and what the tool uid cannot do to its own copy.
#[test]
fn uid_boundary_is_true_exactly_when_the_tool_ran_under_the_tool_uid() {
    let dir = Scratch::new("uid-ids");
    let probe = r#"echo "ids=$(/usr/bin/id -u) $(/usr/bin/id -g) $(/usr/bin/id -G)" >> "$said"
echo "copy=$(/usr/bin/stat -c '%u %a' "$in")" >> "$said"
echo "home=$(/usr/bin/stat -c '%u %g %a' "$HOME")" >> "$said"
echo "root=$(/usr/bin/stat -c '%u %g %a' "$HOME/..")" >> "$said"
if ( : >> "$in" ) 2>/dev/null; then echo "write=yes" >> "$said"; else echo "write=no" >> "$said"; fi
if /bin/mv "$in" "$in.aside" 2>/dev/null; then echo "rename=yes" >> "$said"; else echo "rename=no" >> "$said"; fi"#;
    let tb = toolbox(&dir, probe);
    let sch = schematic(&dir, FLAT);
    let out = erc(&tb, &sch, None, None);
    let v = json(&out);
    assert_eq!(out.status.code(), Some(1), "{v}");
    let lines = said(&v);
    let me = rustix::process::geteuid().as_raw();
    if root() {
        assert_eq!(v["sandbox"]["uid_boundary"], true, "{v}");
        assert_eq!(
            lines,
            [
                format!("ids={TOOL_UID} {TOOL_UID} {TOOL_UID}"),
                "copy=0 444".to_owned(),
                format!("home=0 {TOOL_UID} 1770"),
                format!("root=0 {TOOL_UID} 710"),
                "write=no".to_owned(),
                "rename=no".to_owned(),
            ]
        );
    } else {
        assert_eq!(v["sandbox"]["uid_boundary"], false, "{v}");
        assert!(lines[0].starts_with(&format!("ids={me} ")), "{lines:?}");
    }
}

#[test]
fn a_schematic_naming_a_hierarchical_sheet_is_refused_before_erc_runs() {
    let dir = Scratch::new("uid-sheet");
    let tb = toolbox(&dir, "");
    for text in [
        "(kicad_sch (version 20250114) (sheet (at 0 0) (property \"Sheetfile\" \"sub.kicad_sch\")))\n",
        "(kicad_sch (version 20250114) (x (y (sheet))))\n",
    ] {
        let sch = schematic(&dir, text);
        let v = refused(&erc(&tb, &sch, None, None), 3, "HierarchicalSchematic");
        assert!(v.get("verdict").is_none(), "{v}");
    }
    // A schematic the reader cannot read could hide a sheet: refused too.
    let sch = schematic(
        &dir,
        "# a comment KiCad skips\n(kicad_sch (version 20250114))\n",
    );
    refused(&erc(&tb, &sch, None, None), 3, "MalformedSchematic");
    // The flat control runs.
    let sch = schematic(&dir, FLAT);
    let out = erc(&tb, &sch, None, None);
    assert_eq!(out.status.code(), Some(0), "{}", json(&out));
}

#[test]
fn the_emitted_schematic_names_no_sheet() {
    let dir = Scratch::new("uid-emit");
    let tb = toolbox(&dir, "");
    let spec =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let out = Command::new(BIN)
        .args(["emit", "--file"])
        .arg(&spec)
        .arg("--out")
        .arg(dir.0.join("out"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", json(&out));
    let sch = dir.0.join("out/sentinel-01.kicad_sch");
    let out = erc(&tb, &sch, None, None);
    assert_eq!(out.status.code(), Some(0), "{}", json(&out));
}

/// Past `RLIMIT_FSIZE` a tool's output cannot be read as pass or fail: a
/// child cut off while the tool itself exits clean, and the tool cut off
/// itself, both end as `ToolError`, exit 5. The cap is the hard limit too.
#[test]
fn a_tool_writing_past_the_file_size_cap_is_a_tool_error() {
    let dir = Scratch::new("uid-fsize");
    let child = format!(
        r#"echo "fsize=$(ulimit -f) hard=$(ulimit -H -f)" >> "$said"
if (ulimit -f unlimited) 2>/dev/null; then echo raised >> "$said"; fi
/usr/bin/yes | /usr/bin/head -c {} > "$HOME/big""#,
        CAP + 1
    );
    let tb = toolbox(&dir, &child);
    let sch = schematic(&dir, FLAT);
    let v = refused(&erc(&tb, &sch, None, None), 5, "ToolError");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(message.contains("file-size cap"), "{v}");
    // Under the cap, the same stub's report is read: the limit is the cap, in
    // 512-byte blocks, soft and hard, and the tool cannot raise it.
    let small = toolbox(
        &dir,
        r#"echo "fsize=$(ulimit -f) hard=$(ulimit -H -f)" >> "$said"
if (ulimit -f unlimited) 2>/dev/null; then echo raised >> "$said"; fi
/usr/bin/yes | /usr/bin/head -c 4096 > "$HOME/small""#,
    );
    let v = json(&erc(&small, &sch, None, None));
    let blocks = CAP / 512;
    assert_eq!(said(&v), [format!("fsize={blocks} hard={blocks}")], "{v}");
    // The tool itself cut off: the stub's own shell writes, so SIGXFSZ ends it.
    let own = toolbox(
        &dir,
        r#"s=x; i=0; while [ $i -lt 12 ]; do s=$s$s; i=$((i+1)); done
while :; do printf '%s' "$s"; done > "$HOME/big""#,
    );
    let v = refused(&erc(&own, &sch, None, None), 5, "ToolError");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("file-size cap") || message.contains("signal 25"),
        "{v}"
    );
}
