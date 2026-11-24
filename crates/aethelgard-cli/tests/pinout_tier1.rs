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
    // the ELF's serial table: the reference topology's one port, its console
    // on USART1, TX PA9 and RX PA10 (`AET-WO-4410` R9), as ArduPilot writes
    // SERIAL0's entry (`SD1`, the lines `PAL_LINE(GPIOA,9U)` and `10U`); and
    // its serial parameters (`AET-WO-4405`), SERIAL0's alone.
    let mut elf = elf.clone();
    if header.is_some() && elf.serial.is_none() {
        elf.serial = Some(vec![common::serial_bytes(
            &aethelgard_pincheck::elf::SerialEntry {
                instance: 1,
                is_usb: false,
                tx_line: 0x5802_0009,
                rx_line: 0x5802_000a,
                empty: false,
                driver_is_instance: true,
            },
        )]);
        elf.var_info = Some(common::var_info(&common::SERIAL0_ONLY));
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

/// `AET-WO-1218` (`1217` R2): the reference topology with `FLASH_CS` moved
/// to `PC2`, whose pad takes only an analog input. On the H753 `validate`
/// refuses it (`AnalogPadPin`); with `mcu` renamed, the pads `validate`
/// reads are no row's, and the check refuses the topology by its part.
#[test]
fn a_pad_claim_is_refused_whatever_the_topology_calls_its_mcu() {
    let d = dir("pad-mcu");
    let mut t: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(TOPOLOGY).unwrap()).unwrap();
    t["gpio"]["FLASH_CS"]["pin"] = "PC2".into();
    for (mcu, kind) in [
        ("STM32H753ZITx", "AnalogPadPin"),
        ("STM32H743ZITx", "PartMismatch"),
        ("stm32h753zitx", "PartMismatch"),
    ] {
        t["mcu"] = mcu.into();
        let path = d.join(format!("{mcu}.json"));
        fs::write(&path, serde_json::to_vec(&t).unwrap()).unwrap();
        let (code, line) = run(&d, &Elf::default(), None, path.to_str().unwrap(), ALLOW);
        assert_eq!(code, 2, "{mcu}: {line}");
        assert!(
            line.contains(&format!(r#""kind":"{kind}""#)),
            "{mcu}: {line}"
        );
    }
}

/// `AET-WO-4410` (6), `1218` F4: every refusal the topology alone decides is
/// reported before the ELF is read, so an unreadable or missing ELF beside it
/// never hides it: `validate`'s own (`PinDeviceUndeclared`, a chip select
/// at `low`), `PartMismatch` (`1218`) and `SwdPinNamed` (`4410` (3)).
#[test]
fn a_topology_refusal_is_reported_beside_an_unreadable_elf() {
    let d = dir("order");
    let base: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(TOPOLOGY).unwrap()).unwrap();
    let truncated = d.join("truncated.elf");
    fs::write(&truncated, b"not an elf").unwrap();
    let missing = d.join("missing.elf");
    type Edit = fn(&mut serde_json::Value);
    let cases: [(&str, Edit, &str); 3] = [
        (
            "part",
            |t| t["mcu"] = "STM32H743ZITx".into(),
            "PartMismatch",
        ),
        (
            "low",
            |t| t["gpio"]["FLASH_CS"]["initial"] = "low".into(),
            "PinDeviceUndeclared",
        ),
        (
            "swd",
            |t| t["gpio"]["FLASH_CS"]["pin"] = "PA13".into(),
            "SwdPinNamed",
        ),
    ];
    for (name, edit, kind) in cases {
        let mut t = base.clone();
        edit(&mut t);
        let path = d.join(format!("{name}.json"));
        fs::write(&path, serde_json::to_vec(&t).unwrap()).unwrap();
        for elf in [&truncated, &missing] {
            let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
                .args(["check", "pinout", "--topology"])
                .arg(&path)
                .arg("--elf")
                .arg(elf)
                .arg("--hwdef-h")
                .arg(&missing)
                .output()
                .unwrap();
            let line = String::from_utf8(out.stdout).unwrap();
            assert_eq!(out.status.code(), Some(2), "{name}: {line}");
            assert!(
                line.contains(&format!(r#""kind":"{kind}""#)),
                "{name}: {line}"
            );
        }
    }
}
