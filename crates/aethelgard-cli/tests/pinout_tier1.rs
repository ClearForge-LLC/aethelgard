//! Tier 1: `aethelgard check pinout`'s exit codes and its one JSON line
//! (`AET-WO-4400a`), on the synthetic ELF of `aethelgard-pincheck`'s tests and
//! the solver's real pin table. The real ArduPilot ELFs are judged in CI.

#[path = "../../aethelgard-pincheck/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Elf, step_g_words};

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pinout-{name}"));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

const TOPOLOGY: &str = "../aethelgard-pincheck/tests/common/topology.json";
const ALLOW: &str = "../aethelgard-pincheck/fixtures/step-g.allow.json";

/// Runs the check; returns the exit code and stdout's one line.
fn run(
    d: &Path,
    elf: &Elf,
    header: Option<[[u32; 7]; 11]>,
    topology: &str,
    allow: &str,
) -> (i32, String) {
    // With hwdef.h the identity judge runs (`AET-WO-4400b2`), and it reads
    // the ELF's serial table: the reference topology has no serial port, so
    // one empty entry, as ArduPilot writes `{}`.
    let mut elf = elf.clone();
    if header.is_some() && elf.serial.is_none() {
        elf.serial = Some(vec![[0; 44]]);
    }
    fs::write(d.join("fw.elf"), elf.bytes()).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.args(["check", "pinout", "--topology", topology, "--elf"])
        .arg(d.join("fw.elf"));
    cmd.args(["--allow", allow]);
    if let Some(words) = header {
        fs::write(d.join("hwdef.h"), common::header(&words)).unwrap();
        cmd.arg("--hwdef-h").arg(d.join("hwdef.h"));
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "one JSON line: {stdout}");
    (out.status.code().unwrap(), stdout)
}

#[test]
fn every_exit_code_of_check_pinout() {
    let d = dir("codes");
    let good = Elf::default();

    // 0: step G, both decodes, one `ok` key (a flattened verdict once printed two).
    let (code, line) = run(&d, &good, Some(step_g_words()), TOPOLOGY, ALLOW);
    assert_eq!(code, 0, "{line}");
    assert_eq!(line.matches("\"ok\":").count(), 1, "{line}");
    assert!(line.contains("\"ok\":true") && line.contains("\"decodes\":\"elf and hwdef.h agree\""));

    // 1: PI0 an output, which the real LQFP144 pin table does not have.
    let mut w = step_g_words();
    w[8][0] = 1;
    let (code, line) = run(
        &d,
        &Elf {
            words: w,
            ..Elf::default()
        },
        None,
        TOPOLOGY,
        ALLOW,
    );
    assert_eq!(code, 1, "{line}");
    assert!(
        line.contains(r#""check":"package","kind":"NotInPackage","pin":"PI0""#),
        "{line}"
    );

    // 5: hwdef.h says AF6 where the ELF says AF4.
    let mut other = step_g_words();
    other[1][5] = 0x6600_0000;
    let (code, line) = run(&d, &good, Some(other), TOPOLOGY, ALLOW);
    assert_eq!(code, 5, "{line}");
    assert!(
        line.contains(r#""kind":"DecodesDisagree","id":"GPIOB_AFRL""#),
        "{line}"
    );

    // 3: a malformed ELF, and an allow-list naming a topology pin.
    let (code, line) = run(
        &d,
        &Elf {
            sym_size: 304,
            ..Elf::default()
        },
        None,
        TOPOLOGY,
        ALLOW,
    );
    assert_eq!(code, 3, "{line}");
    assert!(line.contains(r#""kind":"SymbolSize""#), "{line}");
    let hide = d.join("hide.json");
    fs::write(
        &hide,
        r#"{"allow":{"PB6":{"mode":"alternate","alternate_function":4,"why":"hide"}}}"#,
    )
    .unwrap();
    let (code, line) = run(&d, &good, None, TOPOLOGY, hide.to_str().unwrap());
    assert_eq!(code, 3, "{line}");
    assert!(line.contains(r#""kind":"AllowNamesTopologyPin""#), "{line}");

    // 2: a topology of the wrong shape.
    let bad = d.join("topology.json");
    fs::write(&bad, r#"{"schema_version":3}"#).unwrap();
    let (code, line) = run(&d, &good, None, bad.to_str().unwrap(), ALLOW);
    assert_eq!(code, 2, "{line}");
    assert!(line.contains(r#""kind":"SchemaViolation""#), "{line}");
}
