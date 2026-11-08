//! `aethelgard solve --file`, end to end against the real STM32H753ZITx pin
//! table: P1 gate clause 2, clause 4's solver half, each refusal, and the
//! bound. The four fixtures under `tests/fixtures/solve/` are the
//! `AET-WO-1200` spike's specifications, byte for byte, so the digests below
//! are the ones the spike measured on Furnace.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use aethelgard_ir::SolvedTopology;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// sha256 of the success line without its newline. The spike measured
/// `c7f59a49…` and `759f4146…`; `AET-WO-1202` added `i2c_ceilings` and the
/// I2C line constraints to both inputs (old and new in its FEEDBACK).
const REFERENCE_DIGEST: &str = "9e5b38ff615c734c23c7664486ee6d42476e4fdfd77be5243be4fc40a7991ec9";
const STRESS_DIGEST: &str = "77fd96e2d59875ae631b26b2a55085b099779bc62ce7c206653f0eafbb828310";

/// The 1 s bound of `AET-WO-1200` §0 S1. The measured stress time is about
/// 1.5 ms in a debug build, so this is a bound, not a benchmark.
const BOUND: Duration = Duration::from_secs(1);

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn reference() -> PathBuf {
    root().join("examples/sentinel-01-minimal.json")
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/solve/{name}.json"))
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A private directory per test, removed at the end.
struct Dir(PathBuf);

impl Dir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("aet-solve-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, name: &str, doc: &Value) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, serde_json::to_vec_pretty(doc).unwrap()).unwrap();
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn solve_in(cwd: &Path, file: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["solve", "--file"])
        .arg(file)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn solve(file: &Path) -> Output {
    solve_in(&std::env::temp_dir(), file)
}

/// stdout is exactly one line; returns it without the newline.
fn line(output: &Output) -> &str {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout:?}");
    stdout
        .strip_suffix('\n')
        .expect("a newline-terminated line")
}

fn solved(file: &Path) -> (SolvedTopology, String) {
    let output = solve(file);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = line(&output).to_owned();
    let topology: SolvedTopology = serde_json::from_str(&text).unwrap();
    assert_eq!(topology.validate(), Ok(()));
    (topology, text)
}

fn refused(file: &Path, kind: &str) -> Value {
    let output = solve(file);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let report: Value = serde_json::from_str(line(&output)).unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["error"]["kind"], kind, "{report}");
    report
}

/// The reference specification with `edit` applied, written to `dir`.
fn edited(dir: &Dir, edit: impl FnOnce(&mut Value)) -> PathBuf {
    let mut doc: Value = serde_json::from_slice(&std::fs::read(reference()).unwrap()).unwrap();
    edit(&mut doc);
    dir.write("spec.json", &doc)
}

#[test]
fn gate_clause_2_a_pin_set_twice_names_both_claimants_and_emits_nothing_else() {
    let dir = Dir::new("clause-2");
    let file = edited(&dir, |d| {
        d["i2c_buses"][0]["sda"]["pin"] = "PB9".into();
        d["spi_devices"][0]["cs"]["pin"] = "PB9".into();
    });
    let cwd = Dir::new("clause-2-cwd");
    let output = solve_in(&cwd.0, &file);
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_str(line(&output)).unwrap();
    assert_eq!(
        report,
        json!({"ok": false, "error": {
            "kind": "PinClaimedTwice",
            "id": "PB9",
            "claimants": ["i2c_buses[0].sda", "spi_devices[0].cs"],
            "message": "pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"
        }})
    );
    // No board files: nothing written where it ran, nor beside the input.
    assert_eq!(std::fs::read_dir(&cwd.0).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
fn gate_clause_4_two_solves_are_byte_identical_and_match_the_spike() {
    for (file, digest) in [
        (reference(), REFERENCE_DIGEST),
        (fixture("stress"), STRESS_DIGEST),
    ] {
        let (_, a) = solved(&file);
        let (_, b) = solved(&file);
        assert_eq!(a, b, "{}", file.display());
        assert_eq!(hex(a.as_bytes()), digest, "{}: {a}", file.display());
    }
}

#[test]
fn the_hash_is_of_the_bytes_as_read() {
    let (topology, _) = solved(&reference());
    assert_eq!(
        topology.specification_sha256,
        hex(&std::fs::read(reference()).unwrap())
    );
    // Reformatting the file changes the hash, not the assignments.
    let dir = Dir::new("reformatted");
    let file = edited(&dir, |_| {});
    let (other, _) = solved(&file);
    assert_ne!(other.specification_sha256, topology.specification_sha256);
    assert_eq!(other.assignments, topology.assignments);
    assert_eq!(other.gpio, topology.gpio);
}

#[test]
fn the_reference_solves_to_the_expected_pins() {
    let (t, _) = solved(&reference());
    let pin = |net: &str| t.assignments[net].pin.as_str();
    assert_eq!(
        [
            pin("I2C_SDA"),
            pin("I2C_SCL"),
            pin("SPI_SCK"),
            pin("SPI_MOSI"),
            pin("SPI_MISO")
        ],
        ["PB7", "PB6", "PA5", "PA7", "PA6"]
    );
    assert_eq!(t.assignments["I2C_SDA"].peripheral, "I2C1");
    assert_eq!(t.assignments["SPI_SCK"].signal, "SPI1_SCK");
    assert_eq!(t.gpio["FLASH_CS"], "PA0");
    // Both lines declare 300 ns: Fast-mode, 400 kHz, the bus's own speed.
    assert_eq!(t.i2c_ceilings, [("SENSORS".to_owned(), 400_000)].into());
}

#[test]
fn the_stress_specification_solves_within_the_bound() {
    let start = Instant::now();
    let (t, _) = solved(&fixture("stress"));
    assert!(start.elapsed() < BOUND, "{:?}", start.elapsed());
    assert_eq!(t.assignments.len(), 20);
    assert_eq!(t.gpio.len(), 8);
    assert_eq!(t.i2c_ceilings.len(), 4);
    assert!(t.i2c_ceilings.values().all(|hz| *hz == 400_000));
}

#[test]
fn the_adversarial_infeasible_specifications_refuse_within_the_bound() {
    for (name, claimants) in [
        (
            "infeasible-shared",
            json!(["I2C_BUS_0", "I2C_BUS_2", "I2C_BUS_3"]),
        ),
        (
            "pigeonhole-spi",
            json!([
                "SPI_BUS_0",
                "SPI_BUS_1",
                "SPI_BUS_2",
                "SPI_BUS_3",
                "SPI_BUS_4",
                "SPI_BUS_5",
                "SPI_BUS_6"
            ]),
        ),
        (
            "thrash-pins",
            json!(["SPI_BUS_3", "SPI_BUS_4", "SPI_BUS_5"]),
        ),
    ] {
        let start = Instant::now();
        let report = refused(&fixture(name), "Infeasible");
        assert!(start.elapsed() < BOUND, "{name}: {:?}", start.elapsed());
        assert_eq!(report["error"]["claimants"], claimants, "{name}");
        assert_eq!(report["error"]["id"], Value::Null);
    }
}

/// An edit of the reference, the refusal kind, its `id` and its `claimants`.
type Case = (fn(&mut Value), &'static str, Value, Value);

#[test]
fn each_refusal_before_the_search_names_its_cause() {
    let dir = Dir::new("refusals");
    let cases: [Case; 6] = [
        (
            |d| d["i2c_buses"][0]["scl"]["pin"] = "pb8".into(),
            "UnknownPin",
            json!("pb8"),
            json!(["i2c_buses[0].scl"]),
        ),
        (
            |d| d["spi_devices"][0]["cs"]["pin"] = "PA13".into(),
            "ReservedPin",
            json!("PA13"),
            json!(["spi_devices[0].cs"]),
        ),
        (
            |d| d["spi_buses"][0]["sck"]["pin"] = "PH0".into(),
            "ReservedPin",
            json!("PH0"),
            json!(["spi_buses[0].sck"]),
        ),
        (
            |d| d["i2c_buses"][0]["peripheral"] = "I2C5".into(),
            "UnknownPeripheral",
            json!("I2C5"),
            json!(["i2c_buses[0]"]),
        ),
        (
            |d| d["i2c_buses"][0]["peripheral"] = "SPI1".into(),
            "UnknownPeripheral",
            json!("SPI1"),
            json!(["i2c_buses[0]"]),
        ),
        (
            |d| d["i2c_buses"][0]["sda"]["pin"] = "PB6".into(),
            "NoInstance",
            json!("SENSORS"),
            json!(["SENSORS"]),
        ),
    ];
    for (edit, kind, id, claimants) in cases {
        let report = refused(&edited(&dir, edit), kind);
        assert_eq!(report["error"]["id"], id, "{report}");
        assert_eq!(report["error"]["claimants"], claimants, "{report}");
    }
    let report = refused(
        &edited(&dir, |d| {
            d["i2c_buses"][0]["peripheral"] = "I2C3".into();
            d["spi_devices"][0]["cs"]["pin"] = "PC9".into();
        }),
        "NoInstance",
    );
    assert_eq!(
        report["error"]["message"],
        "no instance can serve bus `SENSORS`: I2C3: every pin for `I2C3_SDA` is held by hand \
         (`PC9` by `spi_devices[0].cs`)"
    );
}

#[test]
fn two_buses_on_one_hand_set_instance_are_refused() {
    let dir = Dir::new("peripheral-twice");
    let file = edited(&dir, |d| {
        let mut second = d["i2c_buses"][0].clone();
        second["id"] = "OTHER".into();
        second["sda"]["net"] = "OTHER_SDA".into();
        second["scl"]["net"] = "OTHER_SCL".into();
        d["i2c_buses"].as_array_mut().unwrap().push(second);
        // The new bus's lines need constraints, or the ceiling check refuses first.
        let sda = d["net_constraints"][0].clone();
        for net in ["OTHER_SDA", "OTHER_SCL"] {
            let mut c = sda.clone();
            c["net_name"] = net.into();
            d["net_constraints"].as_array_mut().unwrap().push(c);
        }
        d["i2c_buses"][0]["peripheral"] = "I2C2".into();
        d["i2c_buses"][1]["peripheral"] = "I2C2".into();
    });
    let report = refused(&file, "PeripheralClaimedTwice");
    assert_eq!(
        report["error"]["claimants"],
        json!(["i2c_buses[0]", "i2c_buses[1]"])
    );
}

#[test]
fn an_unknown_part_is_exit_2() {
    let dir = Dir::new("unknown-part");
    let file = edited(&dir, |d| d["mcu"]["part"] = "STM32F103C8Tx".into());
    let report = refused(&file, "UnknownPart");
    assert_eq!(report["error"]["id"], "STM32F103C8Tx");
}

#[test]
fn an_invalid_specification_is_refused_as_validate_refuses_it() {
    let output = solve(&root().join("examples/invalid-zero-domain.json"));
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_str(line(&output)).unwrap();
    assert_eq!(report["error"]["kind"], "ZeroNominalVoltage");
    assert_eq!(report["error"]["id"], "VDD_3V3");
    assert!(report["error"].get("claimants").is_none());
}

#[test]
fn unreadable_input_is_exit_3_and_a_missing_flag_is_64() {
    let dir = Dir::new("unreadable");
    let missing = dir.0.join("absent.json");
    let output = solve(&missing);
    assert_eq!(output.status.code(), Some(3));
    let not_json = dir.0.join("not.json");
    std::fs::write(&not_json, b"{").unwrap();
    assert_eq!(solve(&not_json).status.code(), Some(3));
    let output = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .arg("solve")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn each_ceiling_refusal_is_exit_2_naming_the_bus() {
    let dir = Dir::new("ceilings");
    let cases: [(fn(&mut Value), &str, &str); 3] = [
        (
            |d| {
                d["net_constraints"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|c| c["net_name"] != "I2C_SCL");
            },
            "MissingBusConstraint",
            "I2C bus `SENSORS` has no net constraint on `I2C_SCL`, so it has no rise time to bound its speed",
        ),
        (
            |d| {
                for c in d["net_constraints"].as_array_mut().unwrap() {
                    c["max_rise_time_nanoseconds"] = 1001.into();
                }
            },
            "RiseTimeTooSlow",
            "I2C bus `SENSORS` allows a rise time of 1001 ns, slower than any I2C mode (1000 ns at most)",
        ),
        (
            |d| d["i2c_buses"][0]["speed_hz"] = 1_000_000.into(),
            "BusTooFast",
            "I2C bus `SENSORS` runs at 1000000 Hz, above its rise-time ceiling of 400000 Hz",
        ),
    ];
    for (edit, kind, message) in cases {
        let report = refused(&edited(&dir, edit), kind);
        assert_eq!(report["error"]["id"], "SENSORS", "{report}");
        assert_eq!(report["error"]["claimants"], json!(["SENSORS"]), "{report}");
        assert_eq!(report["error"]["message"], message, "{report}");
    }
}
