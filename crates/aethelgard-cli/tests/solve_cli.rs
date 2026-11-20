//! `aethelgard solve --file`, end to end against the real STM32H753ZITx pin
//! table: P1 gate clause 2, clause 4's solver half, each refusal, and the
//! bound. The four fixtures under `tests/fixtures/solve/` are the
//! `AET-WO-1200` spike's specifications, byte for byte, so the digests below
//! are the ones the spike measured on Furnace.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use aethelgard_ir::SolvedTopology;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// sha256 of the success line without its newline. The spike measured
/// `c7f59a49…` and `759f4146…`; `AET-WO-1202` added `i2c_ceilings` and the
/// I2C line constraints to both inputs (old and new in its FEEDBACK), and
/// `AET-WO-2200` added `analog_lines` (`9e5b38ff…` and `77fd96e2…` before;
/// old and new in its FEEDBACK), and `AET-WO-2203` added `i2c_devices`
/// (`1b98f4c6…` and `d5f27416…` before; old and new in its FEEDBACK), and
/// `AET-WO-1205` added `i2c_speeds` and made the version 2 (`d684cd6d…` and
/// `dd192c61…` before; old and new in its FEEDBACK), and `AET-WO-4200` added
/// `spi_devices` and made the version 3 (`c655bf2e…` and `6eda552c…` before;
/// old and new in its FEEDBACK); re-taken at its merge of `AET-WO-4100`, whose
/// specification version 2 and board section moved them on `main` too
/// (`53ac16ac…` and `0c9ce020…` there); and re-taken in `AET-WO-4300c`, whose
/// specification version 3, solved version 4 (`spi_lines`) and `W25Q64JVxIQ`
/// moved them (`3c39199f…` and `3169ae88…` before); and re-taken in
/// `AET-WO-4300f`, whose specification version 4 (`connectors`) moved only the
/// specification's sha256: with the old hash put back, each output's digest is
/// the old one exactly (`ac9c9299…` and `a8606bf8…` before; FEEDBACK); and
/// re-taken in `AET-WO-1308` (version 5, `net_classes` and `diff_pairs`) the same
/// way (`26771d90…` and `6e4af649…` before); and re-taken in `AET-WO-4300a`,
/// whose specification version 6 (`motor_outputs`, `mcu.hse_hz`) and solved
/// version 5 (`clock`, `motor_outputs`) moved them (`8100aadf…` and
/// `798ae634…` before); and in `AET-WO-4300d`, whose specification version
/// 7 (`power`, `mcu.bor_level`) and solved version 6 (`power`) moved them
/// (`e61864b9…` and `33b079d1…` before); and in `AET-WO-4210`, whose
/// specification version 8 (`accept_lifecycle`) moved only the specification's
/// sha256 after `4300d`'s (`c7688dca…` and `fe4252eb…` before); and
/// re-taken in `AET-WO-4300b`, merging after both (specification version 9,
/// solved version 7, `serial_ports`, and the reference's console port),
/// which moved them (`8646b4b0…41bd` and `c7df9caf…b4d8` before): with the
/// version set back to 6, `serial_ports` and the console's two assignments
/// removed and `main`'s specification sha256 put back, each output's digest
/// is `main`'s exactly (`the_reference_s_console_moves_no_other_assignment`).
/// Re-taken for the serial port's required `negotiated` key (ruled on PR
/// #152, §9 Q1; `7faed6e0…276a` before): with `,"negotiated":null` removed and
/// the specification's old sha256 put back, the output's digest is the old
/// one exactly. Re-taken in `AET-WO-4230` (solved version 8:
/// `spi_controllers`, `i2c_timings`, `bor_level`; `5c2aeff0…3dd22` and
/// `84de8b88…3b016d` before): with the version set back to 7 and the three
/// keys removed, each output's digest is the old one exactly. Moved again in
/// `AET-WO-4230` slice A2a (`peer_deviation_ppm` on the console, which moved
/// the specification's sha256 and the topology's `serial_ports`; `8c6a0647…56f8`
/// and `e79f725a…286b` before). Moved once more by the merge of `AET-WO-4300d3`
/// (specification version 10; `4230` renumbered to 11), which moved only the
/// specification's sha256 (`3427fca7…a0ff` and `6fe3171b…922b` before).
/// Moved again in `AET-WO-4230` round 2 (`acknowledge_unwitnessed` and each
/// port's `peer_deviation_source`), which moved only the specification's
/// sha256: with `9832b91e…8081` put back, the reference's output hashes to
/// `447caa7d…46bc`, the old digest, exactly; with `3236b135…76c0` put back,
/// the stress fixture's hashes to `6d3bb760…8b02`, its old digest, exactly.
/// Moved once more by the reference's acknowledgements (`AET-WO-4230` R1,
/// slice R2-cli), its specification's sha256 alone: with `7f2be55f…dd9e` put
/// back, the reference's output hashes to `f83c83a0…e8ae`, the old digest.
/// Moved once more by the merge of `AET-WO-4300g` (specification version 11,
/// `mcu.supply_source`; `4230` renumbered to 12), which moved only the
/// specification's sha256: with `f3798b1f…b999` put back, the reference's
/// output hashes to `42f26fce…ad34`, and with `17b53057…0f8a` put back, the
/// stress fixture's to `1725d651…0d1f`, the old digests, exactly.
/// Moved once more by `AET-WO-4211` (the flash's `nrnd` acknowledgement),
/// which moved only the specification's sha256: with `c1072b71…2e3e` put
/// back, the reference's output hashes to `5d64214d…d4fb`, and with
/// `4c9aa762…762e` put back, the stress fixture's to `15dd2336…10c8`, the old
/// digests, exactly.
/// Moved once more by `AET-WO-4220b` (specification version 13, each
/// device's `pin_capacitance`), which moved only the specification's sha256:
/// with `3f89077c…aee4` put back, the reference's output hashes to
/// `d751e1f4…0dcf`, and with `7f8f17ed…baf9` put back, the stress fixture's
/// to `00be83e1…eef6`, the old digests, exactly.
/// Moved once more by `AET-WO-1312` (specification version 14), which moved
/// only the specification's sha256: with `87967ab4…a794` put back, the
/// reference's output hashes to `2e19ebee…31d3`, and with `8e9f9215…39cc`
/// put back, the stress fixture's to `a59e85d7…7af7`, the old digests,
/// exactly.
/// Moved by `AET-WO-4400b` (solved schema 9): with `"schema_version":8` put
/// back and the trailing `mcu` and `pin_speeds` keys removed, the
/// reference's output hashes to `9cfa82d3…2ed9` and the stress fixture's to
/// `81831b32…1121`, the old digests, exactly.
/// Moved by `AET-WO-4400d` (specification 16, solved 10), which moved only
/// the specification's sha256 and `schema_version`: with `b9e95cdc…74cd` and
/// version 9 put back, the reference's output hashes to `e7b7564b…5296`, and
/// with the stress fixture's old sha256 (`d76ac8a1…c51c`) and version 9 put
/// back, its to `237ae5f7…1a36`, the old digests, exactly.
/// Moved by `AET-WO-4400g` (specification 17, solved 11), which moved only
/// the specification's sha256 and `schema_version` and added a trailing
/// `"foreign_clock":null`: with `21e1b9d5…d511` and version 10 put back and
/// that key removed, the reference's output hashes to `783af0f1…eed5`, and
/// with the stress fixture's old sha256 (`942bd804…`) likewise, its to
/// `4e3de62b…7174`, the old digests, exactly.
/// Moved by `AET-WO-2304b` (specification 18, solved 11 unchanged), which
/// moved only the specification's sha256: with `89865e50…f771c` put back,
/// the reference's output hashes to `6bbb87ea…fe03`, and with the stress
/// fixture's old sha256 (`aa765a92…aebb`) its to `22bb9cb6…e6b8`, the old
/// digests, exactly (measured).
/// Moved by `AET-WO-4240v` (specification 19, solved 12), which moved only
/// the specification's sha256 and `schema_version` and added `"vos":"scale2"`
/// to the clock and `"core_supply":"ldo"`: with `2304b`'s sha256 and version
/// 11 put back and those keys removed, the reference's output is byte for
/// byte main's, hashing to `c7cc4fc4…5a1c`, and the stress fixture's likewise
/// to `316928c0…d502`, the old digests, exactly (measured).
const REFERENCE_DIGEST: &str = "fa009ce7c4a850f0aedb713d998042a109885a361783914f4b8d6fe369849a16";
const STRESS_DIGEST: &str = "a62263f73d5262b3a83511603fa501606f6cfe9becbf3b9befa829ae0416ba9f";

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
    assert_eq!(t.i2c_speeds, [("SENSORS".to_owned(), 400_000)].into());
    assert_eq!(t.schema_version, aethelgard_ir::SOLVED_SCHEMA_VERSION);
}

#[test]
fn the_reference_s_console_moves_no_other_assignment() {
    // `AET-WO-4300b` §1.7: the console takes USART1 beside what the
    // reference solved before it, which is what it solves without it.
    let (with, _) = solved(&reference());
    let dir = Dir::new("no-console");
    let file = edited(&dir, |d| {
        d["serial_ports"] = json!([]);
        // The console's connector goes with it; the programming path and the
        // supply input stay (`AET-WO-4300g`).
        let kept: Vec<serde_json::Value> = d["connectors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| {
                !c["pins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["net"].as_str().is_some_and(|n| n.starts_with("CONSOLE_")))
            })
            .cloned()
            .collect();
        d["connectors"] = json!(kept);
    });
    let (without, _) = solved(&file);
    let mut rest = with.assignments.clone();
    let tx = rest.remove("CONSOLE_TX").unwrap();
    let rx = rest.remove("CONSOLE_RX").unwrap();
    assert_eq!(rest, without.assignments);
    assert_eq!(with.gpio, without.gpio);
    assert_eq!(
        (tx.pin.as_str(), tx.signal.as_str(), tx.alternate_function),
        ("PA9", "USART1_TX", 7)
    );
    assert_eq!(
        (rx.pin.as_str(), rx.signal.as_str(), rx.alternate_function),
        ("PA10", "USART1_RX", 7)
    );
    let console = &with.serial_ports["CONSOLE"];
    assert_eq!((console.brr, console.error_ppm), (556, -799));
    assert!(without.serial_ports.is_empty());
}

#[test]
fn the_four_port_fixture_solves_on_distinct_peripherals() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/serial-four.json");
    let (t, a) = solved(&file);
    let (_, b) = solved(&file);
    assert_eq!(a, b);
    let ports: Vec<_> = t
        .serial_ports_in_order()
        .into_iter()
        .map(|(label, p, tx, rx)| {
            (
                label,
                p.peripheral.as_str(),
                tx.pin.as_str(),
                rx.pin.as_str(),
                p.brr,
            )
        })
        .collect();
    assert_eq!(
        ports,
        [
            ("MAVLINK", "USART1", "PA9", "PA10", 1111),
            ("GPS", "USART2", "PA2", "PA3", 556),
            ("CRSF", "USART3", "PB10", "PB11", 152),
            ("SBUS", "UART4", "PA0", "PA1", 640),
        ]
    );
    // UART4 took PA0, so the chip select takes the next free pin.
    assert_eq!(t.gpio["FLASH_CS"], "PA11");
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
            |d| d["spi_buses"][0]["sck"]["pin"] = "PC14".into(),
            "ReservedPin",
            json!("PC14"),
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

/// An edit of the reference, the refusal kind, and its message.
type CeilingCase = (fn(&mut Value), &'static str, &'static str);

#[test]
fn each_ceiling_refusal_is_exit_2_naming_the_bus() {
    let dir = Dir::new("ceilings");
    let cases: [CeilingCase; 4] = [
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
        (
            |d| d["i2c_buses"][0]["pull_up_ohms"] = 24.into(),
            "PullUpBelowSinkFloor",
            "I2C bus `SENSORS` has a pull-up of 24 ohms, below the 1022 ohm floor its rail and mode set (UM10204 sink current)",
        ),
    ];
    for (edit, kind, message) in cases {
        let report = refused(&edited(&dir, edit), kind);
        assert_eq!(report["error"]["id"], "SENSORS", "{report}");
        assert_eq!(report["error"]["claimants"], json!(["SENSORS"]), "{report}");
        assert_eq!(report["error"]["message"], message, "{report}");
    }
}

#[test]
fn a_line_over_the_bus_capacitance_limit_is_refused_by_net() {
    let dir = Dir::new("cap-limit");
    let report = refused(
        &edited(&dir, |d| {
            for c in d["net_constraints"].as_array_mut().unwrap() {
                if c["net_name"] == "I2C_SDA" {
                    c["max_capacitance_femtofarads"] = 401_000.into();
                }
            }
        }),
        "BusCapacitanceOverLimit",
    );
    assert_eq!(report["error"]["id"], "I2C_SDA", "{report}");
    assert_eq!(report["error"]["claimants"], json!(["I2C_SDA"]), "{report}");
}

/// `AET-WO-4300d`: the power tree (`common::power`), written to `dir`.
fn power_tree(dir: &Path) -> PathBuf {
    let v = common::power::power_tree();
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("power-tree.json");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    path
}

#[test]
fn a_power_tree_solves_into_the_topology_and_emits_its_board() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("aet-4300d-power-tree");
    let _ = std::fs::remove_dir_all(&dir);
    let file = power_tree(&dir);
    let (t, _) = solved(&file);
    let p = t.power.expect("the solved tree");
    let d = &p.domains["VDD_3V3"];
    assert_eq!(
        (d.vout_min_microvolts, d.vout_max_microvolts),
        (3_144_727, 3_417_204)
    );
    assert_eq!(
        (d.source.reference.as_str(), d.source.pad.as_str()),
        ("R20", "1")
    );
    assert_eq!((d.ret.reference.as_str(), d.ret.pad.as_str()), ("R21", "2"));
    assert_eq!(
        // the fixture's cited VDD_3V3 sinks add 35 mA (`AET-WO-1303b`): its
        // 37 less the removed crossing's U13 and R40 (`AET-WO-1209` D4)
        (p.input_steady_milliamps, p.input_fault_milliamps),
        (2714, 10_100)
    );
    // `AET-WO-4300d2` §1.7: the sense net's pin, the lowest package position
    // among the free pins that carry a single-ended ADC input.
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").unwrap();
    let sense = &p.sense["VBAT_SENSE"];
    let pin = &table.pins[&sense.pin];
    assert_eq!(sense.position, pin.position);
    assert_eq!(
        sense.channels,
        pin.adc_channels.iter().cloned().collect::<Vec<_>>()
    );
    assert!(!sense.channels.is_empty());
    let taken: Vec<&str> = t
        .assignments
        .values()
        .map(|a| a.pin.as_str())
        .chain(t.gpio.values().map(String::as_str))
        .collect();
    assert!(!taken.contains(&sense.pin.as_str()));
    let lowest = table
        .pins
        .iter()
        .filter(|(n, q)| {
            !q.adc_channels.is_empty()
                && !aethelgard_ir::RESERVED_PINS.contains(&n.as_str())
                && !taken.contains(&n.as_str())
        })
        .map(|(_, q)| q.position)
        .min();
    assert_eq!(lowest, Some(sense.position));
    // `AET-WO-4300d2`: the tree is drawn and placed on a routed board.
    let out = dir.join("emitted");
    let output = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["emit", "--file"])
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let board = std::fs::read_to_string(out.join("sentinel-02-power.kicad_pcb")).unwrap();
    for r in [
        "J1", "U10", "U11", "U12", "L1", "L2", "R10", "R11", "R50", "R51", "C50",
    ] {
        assert!(
            board.contains(&format!("(property \"Reference\" \"{r}\"")),
            "{r} is not on the board"
        );
    }
    // Each buck's 4 x 3 array of 0.254 mm vias (`Stack::of`, `1309` D1), and
    // J1's through-hole pads joined solid (`1309` D2).
    assert_eq!(board.matches("(drill 0.254)").count(), 24);
    let j1 = board
        .split("\t(footprint ")
        .find(|f| f.contains("(property \"Reference\" \"J1\""))
        .expect("J1's footprint");
    // Its two pins and its two plated pegs.
    assert_eq!(j1.matches("thru_hole").count(), 4);
    assert_eq!(j1.matches("(zone_connect 2)").count(), 4);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// belt-and-suspenders: the BOR thresholds' falling maxima live in two
/// crates that may not depend on each other (`workspace_shape`): the power
/// parts table's MCU row (DS12117 Table 115) for the supply floor, and
/// `aethelgard_chaos::power::BOR` (Table 16) for the scenario's resets. This
/// holds the two equal, so neither drifts alone (`AET-WO-4300d` reduction).
#[test]
fn the_power_rows_bor_maxima_equal_the_supervisors() {
    let table = aethelgard_ir::power_parts::table().unwrap();
    let Some(aethelgard_ir::power_parts::Kind::Mcu(m)) =
        table.get("STM32H753ZITx").map(|r| &r.kind)
    else {
        panic!("the MCU row");
    };
    let rows: Vec<i64> = m
        .bor_falling_max_millivolts
        .iter()
        .map(|l| l.value)
        .collect();
    let chaos: Vec<i64> = aethelgard_chaos::power::BOR
        .iter()
        .map(|t| i64::try_from(t.falling_uv[2] / 1000).unwrap())
        .collect();
    assert_eq!(rows, chaos);
}

/// D2 (`AET-WO-3303`): the README's example success line carries the solved
/// schema version and the reference's specification hash; neither may drift.
#[test]
fn the_readme_example_line_names_the_real_version_and_hash() {
    let readme = std::fs::read_to_string(root().join("crates/aethelgard-cli/README.md")).unwrap();
    let line = readme
        .lines()
        .find(|l| l.starts_with("{\"schema_version\":") && l.contains("\"specification_sha256\":"))
        .expect("the README's example solved-topology line");
    assert!(
        line.starts_with(&format!(
            "{{\"schema_version\":{},",
            aethelgard_ir::SOLVED_SCHEMA_VERSION
        )),
        "{line}"
    );
    let (topology, _) = solved(&reference());
    let prefix = &topology.specification_sha256[..8];
    assert!(
        line.contains(&format!("\"specification_sha256\":\"{prefix}…\"")),
        "README hash prefix is not {prefix}: {line}"
    );
}
