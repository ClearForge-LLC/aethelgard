//! `AET-WO-4240c`: the HSE support group drawn, tier 1 (no tool runs). The
//! crystal fixture is the reference specification with `4240a`'s 16 MHz
//! crystal declared in `mcu.hse`, built here so that no committed
//! specification moves. `emit` is driven at library level (the CLI refuses
//! the fixture `HseNotGenerated` until `AET-WO-4240b`, ruled Q2); `extract`
//! is run through the CLI, which reads it. The figures pinned below were
//! recorded in `spike/4240c/RESULTS.md` before they were pinned here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use aethelgard_eda::extract::{self, ReadBoard};
use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{route, strict, support};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

#[path = "common/netlist.rs"]
mod netlist;
use netlist::{Pads, netlist_text, pads_of};

const REFERENCE: &str = "examples/sentinel-01-minimal.json";
const CRYSTAL: &str = "ABM8AIG-16.000MHZ-12-2Z-T3";
const OSC_IN: &str = "/U1_OSC_IN";
const OSC_OUT: &str = "/U1_OSC_OUT";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn repo(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap()
}

/// The reference with `hse` in place of `hse_hz`.
fn with_hse(hse: Value) -> Value {
    let mut v: Value = serde_json::from_str(&repo(REFERENCE)).unwrap();
    let mcu = v["mcu"].as_object_mut().unwrap();
    mcu.remove("hse_hz");
    mcu.insert("hse".into(), hse);
    v
}

/// `4240a`'s 16 MHz crystal declaration, on the real part, with `caps`.
fn crystal_with(caps: [u64; 2]) -> Value {
    with_hse(
        json!({"crystal": {"part": CRYSTAL, "load_caps_femtofarads": caps,
        "pull_ppm": 30, "temperature_celsius": {"min": -20, "max": 85}, "life_hours": 8000,
        "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}}),
    )
}

fn crystal() -> Value {
    crystal_with([18_000, 18_000])
}

fn solved(v: Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

fn emitted(v: Value) -> Result<schematic::Emission, EmitError> {
    let (spec, solved, table) = solved(v);
    schematic::emit(&spec, &solved, &table)
}

struct Drawn {
    spec: Specification,
    solved: SolvedTopology,
    table: PinTable,
    board: String,
    schematic: String,
    read: ReadBoard,
}

/// The crystal fixture, emitted and read back once.
fn drawn() -> &'static Drawn {
    static D: OnceLock<Drawn> = OnceLock::new();
    D.get_or_init(|| {
        let (spec, solved, table) = solved(crystal());
        let e = schematic::emit(&spec, &solved, &table).unwrap();
        let read = extract::read(&e.board, &Stack::of(&spec).unwrap()).unwrap();
        Drawn {
            spec,
            solved,
            table,
            board: e.board,
            schematic: e.schematic,
            read,
        }
    })
}

fn oscillator(read: &ReadBoard) -> Vec<extract::Oscillator> {
    let d = drawn();
    extract::oscillator(&d.spec, &d.solved, &d.table, read).unwrap()
}

#[test]
fn the_crystal_is_drawn_on_ph0_and_ph1_with_its_capacitors() {
    let d = drawn();
    let y = d
        .read
        .board
        .parts
        .iter()
        .find(|p| p.reference == "Y1")
        .unwrap();
    let nets: BTreeMap<&str, &str> = y
        .pads
        .iter()
        .map(|p| (p.number.as_str(), p.net.as_deref().unwrap()))
        .collect();
    assert_eq!(
        nets,
        BTreeMap::from([("1", OSC_IN), ("2", "GND"), ("3", OSC_OUT), ("4", "GND")])
    );
    assert_eq!(
        d.read.footprints["Y1"],
        (
            "Crystal:Crystal_SMD_3225-4Pin_3.2x2.5mm".to_owned(),
            CRYSTAL.to_owned()
        )
    );
    // PH0 is LQFP-144 position 23 and PH1 24 in ST's pin table, as the MCU
    // row's own groups say.
    let u1 = d
        .read
        .board
        .parts
        .iter()
        .find(|p| p.reference == "U1")
        .unwrap();
    let on = |n: &str| {
        u1.pads
            .iter()
            .find(|p| p.number == n)
            .unwrap()
            .net
            .as_deref()
    };
    assert_eq!((on("23"), on("24")), (Some(OSC_IN), Some(OSC_OUT)));
    for (pin, position) in [("PH0", 23), ("PH1", 24)] {
        assert_eq!(d.table.pins[pin].position, position);
    }
    let row = support::row("STM32H753ZITx").unwrap();
    assert_eq!(row.group("OSC_IN"), ["23"]);
    assert_eq!(row.group("OSC_OUT"), ["24"]);
    // The symbol's two private `KLC_` notes stay in the library symbol the
    // sheet embeds, and reach no instance and no footprint.
    assert_eq!(d.schematic.matches("\"KLC_").count(), 2);
    assert!(!d.board.contains("KLC_"));
}

/// Each oscillator net as reported: `(board net, bound fF, vias, length nm,
/// pads, crystal gap µm)`.
type Row = (String, u64, u64, u64, u64, u64);

fn rows(o: &[extract::Oscillator]) -> Vec<Row> {
    o.iter()
        .map(|o| {
            let b = &o.line;
            (
                b.board_net.clone(),
                b.c_bound_femtofarads,
                b.via_count,
                b.length_nm,
                b.pad_count,
                o.crystal_gap_um,
            )
        })
        .collect()
}

#[test]
fn the_oscillator_nets_carry_no_via_and_are_reported() {
    let o = oscillator(&drawn().read);
    assert_eq!(
        rows(&o),
        [
            (OSC_IN.to_owned(), 7_473, 0, 10_750_000, 3, GAP_IN),
            (OSC_OUT.to_owned(), 6_016, 0, 6_875_000, 3, GAP_OUT),
        ]
    );
    // Copper only: no pin is counted (DS12117 states CIO as typical only).
    assert!(
        o.iter()
            .all(|o| o.line.terms.pins == 0 && o.line.pins.is_empty())
    );
    assert!(o.iter().all(|o| o.refusal().is_none() && o.crystal == "Y1"));
}

/// The crystal's gap to the MCU on each net, µm, as reported (RESULTS: 5.3
/// and 3.6 mm).
const GAP_IN: u64 = 5_310;
const GAP_OUT: u64 = 3_614;

#[test]
fn a_via_on_osc_in_is_refused_by_name() {
    // The board as read, with one via added on OSC_IN at its corner by the
    // crystal: the judge counts the net's vias, wherever they stand.
    let d = drawn();
    let mut read = d.read.clone();
    read.routed.vias.push(route::Via {
        net: OSC_IN.to_owned(),
        at: (83_000_000, 99_375_000),
    });
    let o = oscillator(&read);
    let r = o[0].refusal().unwrap();
    assert_eq!(r.kind, "OscillatorNetVia");
    assert!(r.message.contains("U1_OSC_IN"), "{}", r.message);
    assert!(o[1].refusal().is_none());
}

/// The board with the crystal and its copper moved by `dx` nm: the reported
/// gap and length follow it (ruled Q5: reported, never judged).
#[test]
fn the_crystal_moved_away_moves_the_reported_gap() {
    let d = drawn();
    let mut moved = d.read.clone();
    let dx = -5_000_000;
    let y = moved
        .board
        .parts
        .iter_mut()
        .find(|p| p.reference == "Y1")
        .unwrap();
    for p in &mut y.pads {
        p.rect.x0 += dx;
        p.rect.x1 += dx;
    }
    let far = oscillator(&moved);
    let gaps: Vec<u64> = far.iter().map(|o| o.crystal_gap_um).collect();
    // Copper edge to copper edge, so 5 mm along x is not 5,000 µm more where
    // the pads are also apart in y.
    assert_eq!(gaps, [10_305, 8_606]);
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-4240c-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `aethelgard extract` on `board` against the crystal fixture.
fn cli_extract(label: &str, board: &str) -> (i32, Value) {
    let dir = scratch(label);
    let spec = dir.join("spec.json");
    let pcb = dir.join("board.kicad_pcb");
    std::fs::write(&spec, serde_json::to_string(&crystal()).unwrap()).unwrap();
    std::fs::write(&pcb, board).unwrap();
    let r = run(&[
        "extract",
        "--board",
        pcb.to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
    ]);
    std::fs::remove_dir_all(&dir).unwrap();
    r
}

/// Ruled A1: "`Cs` dropped from the report" fails here. Each oscillator net's
/// bound, its gap and the typical `CIO` are in `extract`'s report, and the
/// pin condition is among its conditions.
#[test]
fn extract_reports_each_oscillator_net_and_the_pin_condition() {
    let d = drawn();
    let (code, v) = cli_extract("report", &d.board);
    assert_eq!(code, 0, "{v}");
    let o = v["oscillator"].as_array().unwrap();
    let got: Vec<(&str, u64, u64)> = o
        .iter()
        .map(|n| {
            (
                n["board_net"].as_str().unwrap(),
                n["c_bound_femtofarads"].as_u64().unwrap(),
                n["crystal_gap_um"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(got, [(OSC_IN, 7_473, GAP_IN), (OSC_OUT, 6_016, GAP_OUT)]);
    assert!(
        o.iter()
            .all(|n| n["mcu_pin_cio_typical_femtofarads"] == 5_000)
    );
    let conditions: Vec<&str> = v["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(conditions.contains(&extract::OSC_PIN_CONDITION));
    assert_eq!(v["findings"], json!([]), "{v}");
}

#[test]
fn a_board_with_no_crystal_reports_no_oscillator() {
    let dir = scratch("null");
    let (spec, solved, table) = solved(serde_json::from_str(&repo(REFERENCE)).unwrap());
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let pcb = dir.join("board.kicad_pcb");
    std::fs::write(&pcb, &e.board).unwrap();
    let (code, v) = run(&[
        "extract",
        "--board",
        pcb.to_str().unwrap(),
        "--spec",
        REFERENCE,
    ]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(code, 0, "{v}");
    assert!(v.get("oscillator").is_none(), "{v}");
    assert!(
        !v["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == extract::OSC_PIN_CONDITION)
    );
}

#[test]
fn an_external_clock_and_an_unstocked_load_value_are_refused_by_name() {
    let external = with_hse(
        json!({"external": {"hz_num": 8_000_000, "hz_den": 1, "ppm": 50,
        "source": "a declared bench oscillator (test)",
        "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}}),
    );
    let e = emitted(external).unwrap_err();
    assert_eq!((e.kind(), e.id()), ("HseSourceNotDrawn", Some("mcu.hse")));
    let e = emitted(crystal_with([15_000, 15_000])).unwrap_err();
    assert_eq!(e.kind(), "LoadCapacitorNoPart");
    assert!(matches!(
        e,
        EmitError::LoadCapacitorNoPart {
            femtofarads: 15_000
        }
    ));
}

#[test]
fn no_committed_specification_without_a_crystal_draws_one() {
    // Every committed specification: no crystal declared, so no oscillator
    // net named, none for the router to keep via-free, none reported, and
    // nothing of the crystal's in the design. The bytes themselves are
    // `spike/4240c/RESULTS.md`'s sweep (29 specifications, every file).
    let (mut checked, mut hse) = (0, 0);
    for dir in ["examples", "crates/aethelgard-cli/tests/fixtures"] {
        let mut stack = vec![root().join(dir)];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Ok(spec) =
                    serde_json::from_slice::<Specification>(&std::fs::read(&path).unwrap())
                else {
                    continue;
                };
                // `AET-WO-4240b`'s two HSE fixtures (`hse-crystal-16`,
                // `hse-external-8`) are judged by their own tests; every
                // other committed specification declares none.
                if spec.mcu.hse.is_some() {
                    hse += 1;
                    continue;
                }
                let own = aethelgard_ir::names::own_nets(&spec).unwrap();
                assert!(
                    !own.iter().any(|n| n.contains("_OSC_")),
                    "{}",
                    path.display()
                );
                let (Ok(table), Ok(stack_of)) = (
                    aethelgard_svd_hal::pin_table(&spec.mcu.part),
                    Stack::of(&spec),
                ) else {
                    continue;
                };
                assert!(stack_of.oscillator.is_empty(), "{}", path.display());
                let Ok(solved) = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)) else {
                    continue;
                };
                // `oscillator_lines` runs the emitter's own design.
                assert!(
                    extract::oscillator_lines(&spec, &solved, &table)
                        .unwrap()
                        .is_empty()
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 20, "{checked}");
    assert_eq!(hse, 2, "only AET-WO-4240b's two HSE fixtures declare one");
}

// ---- the support gate's step 7 ----

/// The support gate's findings on `pads`, all of them.
fn findings(spec: &Specification, pads: &Pads) -> Vec<&'static str> {
    let tree = strict::netlist(&netlist_text(pads)).unwrap();
    let netlist = support::netlist(&tree).unwrap();
    let placed: route::Board = drawn().read.board.clone();
    let r = support::judge(spec, &netlist, &placed).unwrap();
    let mut kinds: Vec<&'static str> = r.findings.iter().map(|f| f.kind).collect();
    kinds.sort_unstable();
    kinds
}

fn renet(pads: &mut Pads, reference: &str, pad: &str, to: &str) {
    pads.1
        .insert((reference.to_owned(), pad.to_owned()), to.to_owned());
}

/// The load capacitor on `net`.
fn cap_on(pads: &Pads, net: &str) -> String {
    pads.1
        .iter()
        .find(|((r, _), n)| r.starts_with('C') && n.as_str() == net)
        .map(|((r, _), _)| r.clone())
        .unwrap()
}

#[test]
fn the_gate_judges_the_crystal_and_names_each_fault() {
    let d = drawn();
    let base = pads_of(&d.board);
    let gate = |edit: &dyn Fn(&mut Pads)| {
        let mut p = base.clone();
        edit(&mut p);
        findings(&d.spec, &p)
    };
    assert!(gate(&|_| {}).is_empty(), "{:?}", gate(&|_| {}));
    // The crystal's ground pads left floating.
    assert_eq!(
        gate(&|p| renet(p, "Y1", "4", "unconnected-(Y1-Pad4)")),
        ["hse_crystal_ground_floating"]
    );
    // A crystal ground pad on another net, the supply.
    assert_eq!(
        gate(&|p| renet(p, "Y1", "2", "VDD_3V3")),
        ["hse_crystal_ground_floating"]
    );
    // A capacitor on the wrong net: OSC_OUT's moved to OSC_IN.
    let c_out = cap_on(&base, OSC_OUT);
    assert_eq!(
        gate(&|p| renet(p, &c_out, "1", OSC_IN)),
        ["hse_load_capacitor_missing", "hse_load_capacitor_missing"]
    );
    // Another MCU pin, a GPIO, on an oscillator net (adversarial F1).
    assert_eq!(
        gate(&|p| renet(p, "U1", "1", OSC_IN)),
        ["hse_net_extra_part"]
    );
    // The crystal's end on a ground pad.
    assert_eq!(
        gate(&|p| renet(p, "Y1", "3", "GND")),
        ["hse_crystal_off_net"]
    );
    // A wrong value.
    let c_in = cap_on(&base, OSC_IN);
    let value = base.0[&c_in].clone();
    let part = value.split_once(' ').unwrap().1.to_owned();
    assert_eq!(
        gate(&|p| {
            p.0.insert(c_in.clone(), format!("22p {part}"));
        }),
        ["hse_load_capacitor_value"]
    );
    // A capacitor of the wrong dielectric: its value names an X7R part.
    assert_eq!(
        gate(&|p| {
            p.0.insert(c_in.clone(), "18p GRM188R71C105KA12".into());
        }),
        ["hse_load_capacitor_dielectric"]
    );
    // No crystal at all.
    assert_eq!(
        gate(&|p| {
            p.0.remove("Y1");
            p.1.retain(|(r, _), _| r != "Y1");
        }),
        ["hse_crystal_missing"]
    );
}

#[test]
fn a_crystal_drawn_on_a_board_that_declares_none_is_named() {
    let d = drawn();
    let (spec, _, _) = solved(serde_json::from_str(&repo(REFERENCE)).unwrap());
    let kinds = findings(&spec, &pads_of(&d.board));
    assert!(kinds.contains(&"hse_crystal_undeclared"), "{kinds:?}");
}
