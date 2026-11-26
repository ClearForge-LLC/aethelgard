//! The fabrication-readiness gate and the board facts it rests on
//! (`AET-WO-4300k`): `readiness::check` / `conditions` / `stale` on
//! Sentinel-03 and edited copies, and the emitted board's pour, zone-connect
//! and connector-opening facts.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use aethelgard_cli::readiness;
use aethelgard_eda::stack::Stack;
use aethelgard_ir::{SolvedTopology, Specification};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn base_json() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("examples/sentinel-03.json")).unwrap())
        .unwrap()
}

fn spec_of(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).unwrap()
}

fn solve(spec: &Specification) -> SolvedTopology {
    let sha: String = Sha256::digest(serde_json::to_vec(spec).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(spec, &table, &sha).unwrap()
}

/// The base specification, its solved topology and its conditions, once.
fn base() -> &'static (
    Specification,
    SolvedTopology,
    std::collections::BTreeMap<String, String>,
) {
    static B: OnceLock<(
        Specification,
        SolvedTopology,
        std::collections::BTreeMap<String, String>,
    )> = OnceLock::new();
    B.get_or_init(|| {
        let spec = spec_of(&base_json());
        let t = solve(&spec);
        let c = readiness::conditions(&spec).unwrap();
        (spec, t, c)
    })
}

/// The first gap of `edit` applied to the base, judged against the base's
/// topology and conditions (the key checks read neither).
fn gap_of(edit: impl Fn(&mut Value)) -> readiness::Gap {
    let (_, t, c) = base();
    let mut v = base_json();
    edit(&mut v);
    readiness::check(&spec_of(&v), t, c).expect_err("the edited specification must not pass")
}

fn stackup(v: &mut Value) -> &mut serde_json::Map<String, Value> {
    v["board"]["stackup"].as_object_mut().unwrap()
}

/// 76 since the power review (`AET-WO-4300k` §3): five named assumptions
/// added, the supervisor's lifecycle retired (held p. 16, active). 75 since
/// `AET-WO-4411`: U11's loss is read at its solved load, so no
/// `loss_current_gap:U11`. 96 since the module's own rail (ruled
/// 2026-11-25T21:17Z): U13's seventeen, as U11's, and four named rows,
/// U11's EN after U13's PG and U30's power-down order, Ioff buffer and
/// monitor.
#[test]
fn the_base_passes_the_gate_with_96_conditions_from_every_source_and_none_stale() {
    let (spec, t, c) = base();
    assert_eq!(readiness::check(spec, t, c), Ok(()));
    assert_eq!(c.len(), 96);
    for id in [
        "buck_junction_screened:U10",
        "supervisor_hysteresis_typical:U12",
        "stackup_condition:outer-copper-min",
        "hse_safety_factor_unmeasured:HSE",
        "pin_capacitance_declared:U5",
        "offboard_declared:J8.2",
        "usb_pair_undeclared:PA11",
        "theta_ja_copper_mismatch:U10",
        "supervisor_delay_unstated:U12",
        "buck_junction_screened:U13",
        "control_after:U11",
        "radio_power_down_order:U30",
        "radio_rail_ioff:U30",
        "radio_rail_monitor:U30",
    ] {
        assert!(c.contains_key(id), "{id} missing from {:?}", c.keys());
    }
    assert_eq!(readiness::stale(spec, c), None);
}

#[test]
fn absent_fab_tolerances_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| {
        stackup(v).remove("fab_tolerances");
    });
    assert_eq!(g.id, "board.stackup.fab_tolerances");
}

#[test]
fn absent_fabricator_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| {
        let s = stackup(v);
        s.remove("fabricator");
        // Strip every bound's fabricator citation so the gate, not the
        // structure of the citations, is what refuses.
        fn strip(v: &mut Value) {
            match v {
                Value::Object(m) => {
                    if m.get("cites")
                        .is_some_and(|c| c.get("fabricator").is_some())
                    {
                        m.remove("cites");
                    }
                    m.values_mut().for_each(strip);
                }
                Value::Array(a) => a.iter_mut().for_each(strip),
                _ => {}
            }
        }
        strip(&mut v["board"]["stackup"]);
    });
    assert_eq!(g.id, "board.stackup.fabricator");
}

#[test]
fn an_empty_dc_bias_derating_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| v["power"]["conditions"]["dc_bias_derating"] = json!([]));
    assert_eq!(g.id, "power.conditions.dc_bias_derating");
}

#[test]
fn empty_loads_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| v["power"]["loads"] = json!([]));
    assert_eq!(g.id, "power.loads");
}

#[test]
fn empty_net_classes_with_power_declared_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| v["net_classes"] = json!([]));
    assert_eq!(g.id, "net_classes");
}

#[test]
fn empty_drop_limits_is_a_gap_named_for_the_key() {
    let g = gap_of(|v| v["power"]["drop_limits"] = json!([]));
    assert_eq!(g.id, "power.drop_limits");
}

#[test]
fn an_imu_with_no_orientation_is_a_hwdef_gap_naming_the_refusal() {
    let mut v = base_json();
    let mut hit = 0;
    for key in ["spi_devices", "i2c_devices"] {
        for d in v[key].as_array_mut().unwrap() {
            if d["reference"] == "U5" {
                d.as_object_mut().unwrap().remove("orientation").unwrap();
                hit += 1;
            }
        }
    }
    assert_eq!(hit, 1, "U5 must carry one orientation to remove");
    let spec = spec_of(&v);
    let c = readiness::conditions(&spec).unwrap();
    let g = readiness::check(&spec, &solve(&spec), &c).unwrap_err();
    assert_eq!(g.id, "hwdef");
    assert!(
        g.message.contains("ImuOrientationUndeclared"),
        "{}",
        g.message
    );
}

#[test]
fn a_bounded_device_above_the_reset_floor_is_a_gap_named_for_the_device() {
    let g = {
        let (_, t, c) = base();
        let mut v = base_json();
        v["mcu"].as_object_mut().unwrap().remove("reset_supervisor");
        readiness::check(&spec_of(&v), t, c).unwrap_err()
    };
    assert_eq!(g.id, "U4");
}

#[test]
fn a_missing_acknowledgement_is_a_gap_named_for_that_condition() {
    let (_, t, c) = base();
    let mut v = base_json();
    let a = v["acknowledge_conditions"].as_array_mut().unwrap();
    let at = a
        .iter()
        .position(|e| e["id"] == "buck_junction_screened:U10")
        .unwrap();
    a.remove(at);
    let g = readiness::check(&spec_of(&v), t, c).unwrap_err();
    assert_eq!(g.id, "buck_junction_screened:U10");
    assert!(g.message.contains("buck_junction_screened:U10"));
}

#[test]
fn an_acknowledgement_no_condition_produces_is_stale() {
    let (_, _, c) = base();
    let mut v = base_json();
    v["acknowledge_conditions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "nothing:Z9", "reason": "no such condition"}));
    assert_eq!(readiness::stale(&spec_of(&v), c), Some("nothing:Z9"));
}

// ---- the emitted board ----

fn run_emit(spec: &str, out: &std::path::Path) {
    let o = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["emit", "--file", spec, "--out", out.to_str().unwrap()])
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );
}

fn emitted(example: &'static str, cell: &'static OnceLock<String>) -> &'static str {
    cell.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("aet-4300k-ready-{example}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        run_emit(&format!("examples/{example}.json"), &dir);
        let pcb = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.extension().is_some_and(|x| x == "kicad_pcb"))
            .unwrap();
        std::fs::read_to_string(pcb).unwrap()
    })
}

fn board03() -> &'static str {
    static C: OnceLock<String> = OnceLock::new();
    emitted("sentinel-03", &C)
}

fn board01() -> &'static str {
    static C: OnceLock<String> = OnceLock::new();
    emitted("sentinel-01", &C)
}

/// `(footprint ...)` blocks of a board's text, each beginning at its header.
fn footprints(text: &str) -> Vec<&str> {
    let starts: Vec<usize> = text
        .match_indices("\n\t(footprint ")
        .map(|(i, _)| i + 1)
        .collect();
    let tail = text.find("\n\t(zone").map_or(text.len(), |i| i + 1);
    starts
        .iter()
        .enumerate()
        .map(|(k, s)| &text[*s..starts.get(k + 1).copied().unwrap_or(tail)])
        .collect()
}

fn reference(fp: &str) -> &str {
    let at = fp.find("(property \"Reference\" \"").unwrap() + "(property \"Reference\" \"".len();
    &fp[at..at + fp[at..].find('"').unwrap()]
}

fn pads(fp: &str) -> Vec<&str> {
    let starts: Vec<usize> = fp
        .match_indices("\n\t\t(pad ")
        .map(|(i, _)| i + 1)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let end = starts.get(k + 1).copied().unwrap_or_else(|| {
                fp[*s..]
                    .find("\n\t\t(embedded_fonts")
                    .map_or(fp.len(), |e| s + e + 1)
            });
            &fp[*s..end]
        })
        .collect()
}

fn pad_name(pad: &str) -> &str {
    let at = pad.find('"').unwrap() + 1;
    &pad[at..at + pad[at..].find('"').unwrap()]
}

fn small_passive(fp: &str) -> bool {
    let lib = fp.split('"').nth(1).unwrap();
    (lib.starts_with("Resistor_SMD:") || lib.starts_with("Capacitor_SMD:"))
        && (lib.contains("_0402_") || lib.contains("_0603_"))
}

fn read_board(text: &str) -> aethelgard_eda::extract::ReadBoard {
    let stack = Stack::of(&base().0).unwrap();
    aethelgard_eda::extract::read(text, &stack)
        .map_err(|e| format!("{}: {}", e.kind, e.message))
        .unwrap()
}

#[test]
fn the_bucks_join_at_least_their_floor_of_copper_on_exactly_the_three_layers() {
    let read = read_board(board03());
    let pads: Vec<(String, String)> = read
        .board
        .stack
        .thermal
        .iter()
        .map(|t| (t.reference.clone(), t.pad.clone()))
        .collect();
    let joined = aethelgard_eda::pour::joined(&read.board, &read.routed, &pads);
    // `AET-WO-4411` (b), ruled 2026-11-25T21:17Z: with three bucks, each
    // keeps its screen's need on F.Cu too (U10 15 cm², U11 and U13 7 cm²).
    for (r, floor) in [("U10", 1500), ("U11", 700), ("U13", 700)] {
        let j = joined
            .iter()
            .find(|j| j.reference == r)
            .unwrap_or_else(|| panic!("{r} not joined: {joined:?}"));
        let names: Vec<&str> = j.layers.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(names, ["F.Cu", "B.Cu", "In1.Cu"], "{r}");
        for (l, a) in &j.layers {
            assert!(*a >= floor, "{r} on {l}: {a} mm2 < {floor}");
        }
    }
}

#[test]
fn smd_ground_pads_join_the_pour_solid_and_small_passives_keep_a_relief() {
    let text = board03();
    let fps = footprints(text);
    let u10 = fps.iter().find(|f| reference(f) == "U10").unwrap();
    let nine = pads(u10).into_iter().find(|p| pad_name(p) == "9").unwrap();
    assert!(nine.contains("(zone_connect 2)"), "{nine}");
    let mut small_gnd = 0;
    for fp in fps.iter().filter(|f| small_passive(f)) {
        for p in pads(fp).into_iter().filter(|p| p.contains("(net \"GND\")")) {
            small_gnd += 1;
            assert!(
                p.contains("(zone_connect 0)"),
                "{} pad {}: {p}",
                reference(fp),
                pad_name(p)
            );
        }
    }
    assert!(
        small_gnd >= 10,
        "only {small_gnd} small-passive GND pads found"
    );
}

#[test]
fn a_board_with_no_pours_joins_no_ground_pad_to_anything() {
    let text = board01();
    // Its GND zone is the plane (B.Cu); nothing is poured on the outer layer.
    assert!(
        !text.contains("\t(zone\n\t\t(net \"GND\")\n\t\t(layer \"F.Cu\")"),
        "sentinel-01 is expected to pour nothing on F.Cu"
    );
    let mut ground = 0;
    for fp in footprints(text) {
        for p in pads(fp).into_iter().filter(|p| p.contains("(net \"GND\")")) {
            ground += 1;
            assert!(
                !p.contains("zone_connect"),
                "{} pad {}: {p}",
                reference(fp),
                pad_name(p)
            );
        }
    }
    assert!(ground > 10, "only {ground} GND pads found");
}

fn judge(text: &str) -> (usize, Option<aethelgard_eda::opening::Inward>) {
    let read = read_board(text);
    aethelgard_eda::opening::judge(&read.board, |r| {
        read.footprints.get(r).map(|(_, v)| v.clone())
    })
}

/// The board `text` reads as, with part `r` turned by `f` (the courtyard is
/// already in board coordinates, so only the opening's direction changes).
fn judge_turned(
    text: &str,
    r: &str,
    f: impl Fn(u16) -> u16,
) -> Option<aethelgard_eda::opening::Inward> {
    let mut read = read_board(text);
    let part = read
        .board
        .parts
        .iter_mut()
        .find(|p| p.reference == r)
        .unwrap();
    part.rotation = f(part.rotation).rem_euclid(360);
    aethelgard_eda::opening::judge(&read.board, |x| {
        read.footprints.get(x).map(|(_, v)| v.clone())
    })
    .1
}

#[test]
fn every_connector_opens_off_the_board_on_the_emitted_sentinel_03() {
    let (n, inward) = judge(board03());
    assert_eq!(n, 8);
    assert_eq!(inward, None);
}

#[test]
fn a_connector_turned_to_face_into_the_board_is_judged_inward_and_named() {
    let text = board03();
    for rot in [270, 0] {
        let inward = judge_turned(text, "J4", |_| rot);
        assert_eq!(
            inward.map(|i| i.reference).as_deref(),
            Some("J4"),
            "J4 at {rot}"
        );
    }
    let inward = judge_turned(text, "J1", |r| r + 180);
    assert_eq!(inward.map(|i| i.reference).as_deref(), Some("J1"));
}

#[test]
fn a_zone_the_specification_does_not_declare_is_refused_as_planes_disagree() {
    let text = board03();
    let start = text
        .find("\t(zone\n\t\t(net \"GND\")\n\t\t(layer \"F.Cu\")")
        .unwrap();
    let block = &text[start..start + text[start..].find("\n\t)\n").unwrap() + 4];
    let planted = block.replace("(net \"GND\")", "(net \"VDD_3V3\")").replace(
        "075b57cb-ccd1-81fa-919f-3b444289ed74",
        "075b57cb-ccd1-81fa-919f-3b444289ed75",
    );
    assert_ne!(planted, block);
    let bad = text.replacen(
        "\t(embedded_fonts no)\n)",
        &format!("{planted}\t(embedded_fonts no)\n)"),
        1,
    );
    assert_ne!(bad, text);
    let stack = Stack::of(&base().0).unwrap();
    let e = aethelgard_eda::extract::read(&bad, &stack)
        .err()
        .expect("must refuse");
    assert_eq!(e.kind, "PlanesDisagree", "{}", e.message);
}
