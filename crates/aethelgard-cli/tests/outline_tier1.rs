//! `AET-WO-1313`, tier 1: `board.max_outline_um`. The Sentinel-03 fixture
//! emits inside its declared 100 x 100 mm bound with its connectors on the
//! side edges; a bound that cannot hold the parts is refused by `validate`
//! before any placement, and one the placer cannot reach is refused by
//! `emit` by name, writing nothing (§5: never a partial board).

mod common;

use std::path::PathBuf;
use std::process::{Command, Stdio};

use aethelgard_eda::pcb::{self, GAP_NM, IC_GAP_NM, Rect};
use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::{library, sexpr, strict};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use common::rename;
use serde_json::{Value, json};

const FIXTURE: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(FIXTURE)).unwrap()).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

/// The committed fixture through `emit`, the ladder's eight routings, made
/// once per process (`AET-WO-1910` D2) and read by each test that judges that
/// board; a run that fails fails each of them by its property
/// (`common::verify::Shared`).
type Emitted = (Specification, SolvedTopology, PinTable, schematic::Emission);
static EMITTED: common::verify::Shared<Emitted> =
    common::verify::Shared::new("the committed Sentinel-03 through emit");

fn emitted(property: &str) -> &'static Emitted {
    EMITTED.get(property, || {
        let (spec, solved, table) = solved(&fixture());
        let e = schematic::emit(&spec, &solved, &table).unwrap();
        (spec, solved, table, e)
    })
}

/// The fixture with a bound of `width` x `height` whole millimetres.
fn with_bound(width: u32, height: u32) -> Value {
    let mut v = fixture();
    v["board"]["max_outline_um"] = json!({"width": width * 1000, "height": height * 1000});
    v
}

/// The board's outline (width, height) and each connector's rotation.
fn outline_and_rotations(board: &str) -> ((f64, f64), Vec<(String, String)>) {
    let tree = strict::board(board).unwrap();
    let rect = tree.root().all("gr_rect").next().unwrap();
    let xy = |head: &str| -> (f64, f64) {
        let n = rect.one(head).unwrap().unwrap();
        (
            n.arg(1).unwrap().parse().unwrap(),
            n.arg(2).unwrap().parse().unwrap(),
        )
    };
    let ((x0, y0), (x1, y1)) = (xy("start"), xy("end"));
    let mut rotations = Vec::new();
    for fp in tree.root().all("footprint") {
        let r = fp.field("Reference").unwrap().unwrap().to_owned();
        if r.starts_with('J') {
            let at = fp.one("at").unwrap().unwrap();
            rotations.push((r, at.arg(3).unwrap_or("0").to_owned()));
        }
    }
    ((x1 - x0, y1 - y0), rotations)
}

fn run(args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn scratch(label: &str) -> PathBuf {
    let dir = root()
        .join("target")
        .join(format!("aet-1313-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_fixture_fits_its_bound_with_connectors_on_the_side_edges() {
    let (spec, _, _, e) = emitted("the fixture fits its bound with connectors on the side edges");
    assert!(spec.board.max_outline_um.is_some());
    let ((w, h), rotations) = outline_and_rotations(&e.board);
    assert!(w <= 100.0 && h <= 100.0, "{w} x {h}");
    assert!(!rotations.is_empty());
    for (r, angle) in &rotations {
        assert!(angle == "90" || angle == "270", "{r} at {angle}");
    }
    // The same specification with no bound is today's board: every
    // connector on the bottom edge (D4; every corpus file is checked
    // byte for byte in FEEDBACK §5).
    let mut v = fixture();
    v["board"].as_object_mut().unwrap().remove("max_outline_um");
    let (spec, solved, table) = self::solved(&v);
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let ((w, _), rotations) = outline_and_rotations(&e.board);
    assert!(w > 100.0, "{w}");
    assert!(rotations.iter().all(|(_, a)| a == "0"));
}

#[test]
fn a_bound_the_placer_cannot_reach_is_refused_by_name_never_a_partial_board() {
    // 60 x 60 mm holds the keep-out boxes' sum, but not the placement.
    let v = with_bound(60, 60);
    let (spec, solved, table) = solved(&v);
    schematic::Drawing::of(&spec, &solved, &table)
        .unwrap()
        .bound_holds()
        .unwrap();
    match schematic::emit(&spec, &solved, &table) {
        Err(e @ EmitError::OutlineUnreachable { .. }) => {
            assert_eq!(e.kind(), "OutlineUnreachable");
            assert!(e.to_string().contains("best outline"), "{e}");
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
    let dir = scratch("unreachable");
    let spec_path = dir.join("spec.json");
    std::fs::write(&spec_path, serde_json::to_string(&v).unwrap()).unwrap();
    let out = dir.join("out");
    let (code, verdict) = run(&[
        "emit",
        "--file",
        spec_path.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(2), "{verdict}");
    assert_eq!(verdict["error"]["kind"], "OutlineUnreachable", "{verdict}");
    assert!(!out.exists(), "emit wrote into {}", out.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_refuses_a_bound_that_cannot_hold_the_parts() {
    let dir = scratch("too-small");
    for (w, h, id) in [(40, 40, Value::Null), (20, 200, json!("U1"))] {
        let path = dir.join(format!("spec-{w}-{h}.json"));
        std::fs::write(&path, serde_json::to_string(&with_bound(w, h)).unwrap()).unwrap();
        let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
        assert_eq!(code, Some(2), "{verdict}");
        assert_eq!(verdict["error"]["kind"], "OutlineTooSmall", "{verdict}");
        assert_eq!(verdict["error"]["id"], id, "{verdict}");
    }
    let path = dir.join("spec-100.json");
    std::fs::write(&path, serde_json::to_string(&fixture()).unwrap()).unwrap();
    let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{verdict}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_version_14_file_is_refused_by_its_version_before_its_shape() {
    let dir = scratch("v14");
    let mut v = fixture();
    v["schema_version"] = json!(14);
    let path = dir.join("v14.json");
    std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
    let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{verdict}");
    assert_eq!(
        verdict["error"]["kind"], "UnsupportedSchemaVersion",
        "{verdict}"
    );
    assert_eq!(
        verdict["error"]["message"], "schema_version is 14, not 22",
        "{verdict}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A board's outline and every footprint's library name, origin and
/// rotation, sorted: the placement with every name taken out.
fn name_free(board: &str) -> Vec<String> {
    let tree = sexpr::parse(board).unwrap();
    let rect = tree.child("gr_rect").unwrap();
    let mut v: Vec<String> = tree
        .children("footprint")
        .map(|fp| {
            let at = fp.child("at").unwrap();
            format!(
                "{} {} {} {}",
                fp.arg(1).unwrap(),
                at.arg(1).unwrap(),
                at.arg(2).unwrap(),
                at.arg(3).unwrap_or("0")
            )
        })
        .collect();
    v.sort();
    v.insert(
        0,
        sexpr::write(rect.child("start").unwrap()).trim().to_owned(),
    );
    v.insert(
        1,
        sexpr::write(rect.child("end").unwrap()).trim().to_owned(),
    );
    v
}

/// The outline's width and height in nanometres.
fn outline_nm(board: &str) -> Rect {
    let tree = sexpr::parse(board).unwrap();
    let rect = tree.child("gr_rect").unwrap();
    let at = |head: &str, i: usize| sexpr::nm(rect.child(head).unwrap().arg(i).unwrap()).unwrap();
    Rect {
        x0: at("start", 1),
        y0: at("start", 2),
        x1: at("end", 1),
        y1: at("end", 2),
    }
}

/// `AET-WO-1313c` §5, `1313b`'s `gaps.py` as a test: every keep-out box,
/// rebuilt from the board and the library as the placer drew it, is inside
/// the outline, and every two are at least [`GAP_NM`] apart, [`IC_GAP_NM`]
/// when both are ICs (the placer's `is_ic`: neither a `Device:` part, a
/// test point nor a switch, read from the schematic's symbols).
fn assert_gaps(e: &schematic::Emission) {
    let sch = sexpr::parse(&e.schematic).unwrap();
    let lib_of: std::collections::BTreeMap<String, String> = sch
        .children("symbol")
        .map(|s| {
            (
                s.property("Reference").unwrap().to_owned(),
                s.child("lib_id").unwrap().arg(1).unwrap().to_owned(),
            )
        })
        .collect();
    let ic = |lib: &str| {
        !lib.starts_with("Device:") && lib != "Connector:TestPoint" && !lib.starts_with("Switch:")
    };
    let board = sexpr::parse(&e.board).unwrap();
    let outline = outline_nm(&e.board);
    let mut keeps: Vec<(String, bool, Rect)> = Vec::new();
    for fp in board.children("footprint") {
        let reference = fp.property("Reference").unwrap().to_owned();
        let at = fp.child("at").unwrap();
        let theta: u16 = at.arg(3).unwrap_or("0").parse().unwrap();
        let (x, y) = (
            sexpr::nm(at.arg(1).unwrap()).unwrap(),
            sexpr::nm(at.arg(2).unwrap()).unwrap(),
        );
        // The library's footprint, with its reference on the layer the
        // board prints it on, as the placer drew it (`reference_on_fab`).
        let layer = |f: &sexpr::Sx| {
            f.children("property")
                .find(|p| p.arg(1) == Some("Reference"))
                .and_then(|p| p.child("layer"))
                .and_then(|l| l.arg(1))
                .map(str::to_owned)
        };
        let mut lib = library::footprint(fp.arg(1).unwrap()).unwrap();
        let on = layer(fp).unwrap();
        for c in lib.items_mut().unwrap().iter_mut() {
            if c.head() == Some("property") && c.arg(1) == Some("Reference") {
                for f in c.items_mut().unwrap().iter_mut() {
                    if f.head() == Some("layer") {
                        *f = sexpr::Sx::List(vec![sexpr::Sx::atom("layer"), sexpr::Sx::str(&on)]);
                    }
                }
            }
        }
        assert_eq!(layer(&lib).as_deref(), Some(on.as_str()), "{reference}");
        let k = pcb::geometry_at(&lib, &reference, theta).unwrap().keep;
        let k = Rect {
            x0: k.x0 + x,
            y0: k.y0 + y,
            x1: k.x1 + x,
            y1: k.y1 + y,
        };
        assert!(outline.contains(k), "{reference} leaves the outline");
        keeps.push((reference.clone(), ic(&lib_of[&reference]), k));
    }
    assert!(keeps.len() > 50, "{} footprints", keeps.len());
    for (i, (ra, ia, a)) in keeps.iter().enumerate() {
        for (rb, ib, b) in &keeps[i + 1..] {
            let gap = if *ia && *ib { IC_GAP_NM } else { GAP_NM };
            assert!(
                !a.grow(gap).overlaps(*b),
                "{ra} and {rb} are closer than {gap} nm"
            );
        }
    }
}

/// `AET-WO-1313c` §5: every loop rule of the power tree (each buck's input
/// capacitors, CBOOT, the inductor, ...) holds on the placed board, judged
/// by the loop gate `check drc --spec` runs (`loops::judge`), after
/// compaction moved each buck's group whole.
fn assert_loops(spec: &Specification, solved: &SolvedTopology, table: &PinTable, board: &str) {
    let rules = aethelgard_eda::loops::rules(spec, solved, table).unwrap();
    assert!(!rules.is_empty());
    let tree = strict::board(board).unwrap();
    let (b, _) =
        aethelgard_eda::route::read(&tree, &aethelgard_eda::stack::Stack::of(spec).unwrap())
            .unwrap();
    for l in aethelgard_eda::loops::judge(&b, &rules).unwrap() {
        assert_eq!(
            l.verdict, "pass",
            "{} beside {}: {:?}",
            l.part, l.ic, l.pads
        );
    }
}

/// The fixture's placement with every name taken out ([`name_free`]).
/// `AET-WO-1313b`'s prototype placed it at `5e30de78…` (`457d644`, FEEDBACK
/// §1.3); re-pinned by `AET-WO-1315` from a measured run, the hot-plug bulk
/// capacitor C1 drawn behind U10's ceramics (its FEEDBACK §2); re-pinned by
/// `AET-WO-4202` from a measured run, U2 gone (its FEEDBACK §2.3). Re-pin it
/// only with the change that moves it, named in that change's FEEDBACK.
const FIXTURE_PLACEMENT_SHA256: &str =
    "bd276856d6578b8c095b8ade3794acae7e4081e6c6fdc159e7f59a11c02a55c3";

/// `AET-WO-1313c` §2: the fixture places at its measured outline, with
/// every placer gap and every loop rule kept after compaction: 85.25 x
/// 80.95 mm at `1313b`'s (FEEDBACK §2.1, (k)+(m)+(s)), 96.50 x 69.35 mm
/// since `AET-WO-1315` drew C1 (its FEEDBACK §2, measured), 95.80 x 67.60
/// mm since `AET-WO-4202` took U2 away (its FEEDBACK §2.3, measured).
#[test]
fn the_fixture_places_at_the_spikes_outline_with_every_gap_kept() {
    use sha2::{Digest, Sha256};
    let (spec, solved, table, e) =
        emitted("the fixture places at the spike's outline with every gap kept");
    let o = outline_nm(&e.board);
    assert_eq!((o.x1 - o.x0, o.y1 - o.y0), (95_800_000, 67_600_000));
    let digest: String = Sha256::digest(name_free(&e.board).join("\n").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(digest, FIXTURE_PLACEMENT_SHA256);
    assert_gaps(e);
    assert_loops(spec, solved, table, &e.board);
}

/// `AET-WO-1313c` §2: twenty order-only renamings (references permuted
/// within their letters and length, bus nets prefixed) place the fixture
/// identically once names are taken out. The assertion is placement alone,
/// so the seeds are placed by `schematic::placement`, `emit`'s own path up
/// to the router (`AET-WO-1314`, ruling 6987450886): routing each of them
/// eight times, as the capacitance ladder does, bought this test nothing.
/// The shortcut is proven, not assumed: the base fixture's full `emit`,
/// ladder included, places exactly as `placement` does.
#[test]
fn order_only_renamings_place_the_fixture_the_same_way() {
    let place = |v: &Value| {
        let (spec, solved, table) = solved(v);
        name_free(&schematic::placement(&spec, &solved, &table).unwrap())
    };
    let base = fixture();
    let want = place(&base);
    let (_, _, _, emitted) = emitted("order-only renamings place the fixture the same way");
    assert!(emitted.ladder.is_some(), "the fixture is laddered");
    assert_eq!(
        name_free(&emitted.board),
        want,
        "emit's board places otherwise than its own placement"
    );
    let seeds: Vec<u64> = (1..=20).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for chunk in seeds.chunks(seeds.len().div_ceil(threads)) {
            let (base, want) = (&base, &want);
            s.spawn(move || {
                for &seed in chunk {
                    let renamed = rename::order_only(base, seed);
                    assert_ne!(&renamed, base, "seed {seed} renames nothing");
                    assert_eq!(&place(&renamed), want, "seed {seed}");
                }
            });
        }
    });
}

/// `schematic::placement` (`AET-WO-1314`, ruling 6987450886) is the placed
/// board, not a routed one: no track, via or zone; and it refuses as `emit`
/// does before routing (a bound the placer cannot reach).
#[test]
fn placement_writes_no_copper_and_refuses_as_emit_does() {
    let (spec, topology, table) = solved(&fixture());
    let board = schematic::placement(&spec, &topology, &table).unwrap();
    // the board's own items, not text inside a footprint
    let tree = sexpr::parse(&board).unwrap();
    for head in ["segment", "via", "zone", "arc"] {
        assert_eq!(tree.children(head).count(), 0, "{head}");
    }
    assert!(tree.child("gr_rect").is_some());
    assert!(tree.children("footprint").count() > 0);
    let (spec, topology, table) = solved(&with_bound(60, 60));
    let placed = schematic::placement(&spec, &topology, &table).unwrap_err();
    let emitted = schematic::emit(&spec, &topology, &table)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(placed.kind(), "OutlineUnreachable");
    assert_eq!(placed.to_string(), emitted.to_string());
}

/// `AET-WO-1313c` §2, size samples: every reference prefixed by 2, 6 or 8
/// characters (`rename_len.py`). The `extract` margin is the geometry's,
/// not the method's, so no size is promised: each sample either places
/// inside the bound with every placer gap kept (its gates are tier 2's), or
/// is refused by name, writing nothing.
///
/// `AET-WO-1315` (ruled (a), C1 anchored to U10): 1 character places, at its
/// measured outline, so the pass branch stays exercised; 2, 6 and 8 were
/// refused (the x headroom fell from 14.75 to 3.5 mm, its FEEDBACK D1).
/// `AET-WO-1317`: 2 and 6 place on the reseeded rank's second seed, at their
/// measured outlines (its FEEDBACK §4.3); 8 is still refused, every seed over.
#[test]
fn each_size_sample_places_with_its_gaps_or_is_refused_by_name() {
    for (len, placed) in [
        (1, Some((98_800_000, 69_850_000))),
        (2, Some((89_200_000, 83_200_000))),
        (6, Some((98_150_000, 87_450_000))),
        (8, None),
    ] {
        let v = rename::sized(&fixture(), len);
        let (spec, solved, table) = solved(&v);
        match (schematic::emit(&spec, &solved, &table), placed) {
            (Ok(e), Some(outline)) => {
                let o = outline_nm(&e.board);
                assert_eq!((o.x1 - o.x0, o.y1 - o.y0), outline, "{len}");
                assert_gaps(&e);
                assert_loops(&spec, &solved, &table, &e.board);
            }
            (Err(e), None) => {
                assert!(
                    matches!(e.kind(), "OutlineUnreachable" | "OutlineTooSmall"),
                    "{len}: {e}"
                );
                // `AET-WO-1317`: the refusal names the least-over of the nine
                // attempts (seed 0 was 118.500 x 79.450 mm).
                assert!(
                    e.to_string().contains(
                        "the placer's best outline is 102.150 x 88.200 mm, over by 2.150 x 0.000 mm"
                    ),
                    "{len}: {e}"
                );
            }
            (r, _) => panic!("{len}: {:?}", r.map(|e| outline_nm(&e.board))),
        }
    }
}

/// The fixture with `board.forward` "top" and U5 turned so its x axis points
/// `x` (`AET-WO-4400d`'s variant).
fn turned(mut v: Value, x: &str) -> Value {
    v["board"]["forward"] = json!("top");
    for key in ["spi_devices", "i2c_devices"] {
        for d in v[key].as_array_mut().unwrap() {
            if d["reference"] == "U5" {
                d["orientation"] = json!({"x": x, "z": "up"});
            }
        }
    }
    v
}

/// `AET-WO-1313c` with `AET-WO-4400d`: under the bound, the IMU turned each
/// of its four ways (its declared orientation, the board's forward edge at
/// the top) is placed at that rotation and compacted whole with its group:
/// every placer gap and loop rule kept, and each keep-out zone it carries
/// inside the outline (the zones are drawn from the part's origin and
/// rotation, so a group moved whole carries them).
///
/// `AET-WO-1317`: the forward turn, refused since U2 left (`AET-WO-4202`),
/// places on the reseeded rank's first seed (its FEEDBACK §4.3).
#[test]
fn a_turned_imu_is_compacted_whole_with_its_keepouts_inside_the_bound() {
    for (x, rotation) in [
        ("right", "0"),
        ("forward", "90"),
        ("left", "180"),
        ("aft", "270"),
    ] {
        let (spec, solved, table) = solved(&turned(fixture(), x));
        let e = schematic::emit(&spec, &solved, &table).unwrap();
        let tree = sexpr::parse(&e.board).unwrap();
        let u5 = tree
            .children("footprint")
            .find(|f| f.property("Reference") == Some("U5"))
            .unwrap();
        assert_eq!(
            u5.child("at").unwrap().arg(3).unwrap_or("0"),
            rotation,
            "{x}"
        );
        let o = outline_nm(&e.board);
        assert!(
            o.x1 - o.x0 <= 100_000_000 && o.y1 - o.y0 <= 100_000_000,
            "{x}: {o:?}"
        );
        let mut points = 0;
        for zone in u5.children("zone") {
            let pts = zone.child("polygon").unwrap().child("pts").unwrap();
            for xy in pts.children("xy") {
                let p = (
                    sexpr::nm(xy.arg(1).unwrap()).unwrap(),
                    sexpr::nm(xy.arg(2).unwrap()).unwrap(),
                );
                assert!(
                    o.x0 <= p.0 && p.0 <= o.x1 && o.y0 <= p.1 && p.1 <= o.y1,
                    "{x}: U5's keep-out point {p:?} is outside {o:?}"
                );
                points += 1;
            }
        }
        assert!(points > 0, "{x}: U5 carries no keep-out zone");
        assert_gaps(&e);
        assert_loops(&spec, &solved, &table, &e.board);
    }
}

/// The fixture as it was before `AET-WO-4202` took U2 away: 03d89554's hunk
/// reversed (U2's device and load rows back, the flash pull-ups' load rows
/// renumbered R7 and R8 again).
fn with_u2(mut v: Value) -> Value {
    v["i2c_devices"].as_array_mut().unwrap().insert(
        0,
        json!({"reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS",
               "address": 118, "pin_capacitance": null}),
    );
    let loads = v["power"]["loads"].as_array_mut().unwrap();
    let flash: Vec<usize> = (0..loads.len())
        .filter(|&i| {
            loads[i]["source"]
                .as_str()
                .unwrap()
                .contains("W25Q64JV rev J 9.4 ILI")
        })
        .collect();
    assert_eq!(flash.len(), 2);
    for (i, r) in flash.into_iter().zip(["R7", "R8"]) {
        loads[i]["reference"] = json!(r);
    }
    let after = loads.iter().position(|l| l["reference"] == "R20").unwrap() + 1;
    loads.insert(
        after,
        json!({"domain": "VDD_3V3", "milliamps": 1, "reference": "U2",
               "source": "Bosch BST-BME280-DS001-24 rev 1.24 Table 1 p. 8: IDDP current during pressure measurement 714 uA (Max value at -40 C); rounded up"}),
    );
    v
}

/// Whether `v` places inside its bound: `Some(true)` placed, `Some(false)`
/// refused for its outline, `None` refused before placement (by `validate`
/// or the solver: not a placement to judge).
fn fits(v: &Value) -> Option<bool> {
    let spec: Specification = serde_json::from_value(v.clone()).ok()?;
    spec.validate().ok()?;
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).ok()?;
    match schematic::placement(&spec, &solved, &table) {
        Ok(_) => Some(true),
        Err(e) if e.kind() == "OutlineUnreachable" => Some(false),
        Err(e) => panic!("{e}"),
    }
}

/// `AET-WO-1317` §1.3, the sweep's slice: from each bounded variant that
/// places, remove each declared I2C or SPI device, connector and test point in
/// turn (with its load rows), as a specification would. Every removal that
/// `validate` and the solver accept still places inside the bound. On `main`'s
/// placer the forward turn and sizes 2 and 6 do not place whole; of the rest,
/// four removals stop fitting: U2, TP14 and TP24 from the forward turn with U2,
/// and Q_TP2 from size sample 1 (FEEDBACK §4.4). The variants are the fixture,
/// its four turns, size samples 1, 2 and 6, and the forward turn with U2.
#[test]
fn removing_one_declared_part_never_stops_a_bounded_board_fitting() {
    let mut variants: Vec<(String, Value)> = vec![("fixture".into(), fixture())];
    for x in ["right", "forward", "left", "aft"] {
        variants.push((format!("{x} turn"), turned(fixture(), x)));
    }
    for len in [1, 2, 6] {
        variants.push((format!("size {len}"), rename::sized(&fixture(), len)));
    }
    variants.push((
        "forward turn with U2".into(),
        turned(with_u2(fixture()), "forward"),
    ));
    let mut cases = Vec::new();
    for (name, v) in &variants {
        assert_eq!(fits(v), Some(true), "{name} places whole");
        for key in ["i2c_devices", "spi_devices", "connectors", "test_points"] {
            for i in 0..v[key].as_array().unwrap().len() {
                let mut r = v.clone();
                let gone = r[key].as_array_mut().unwrap().remove(i);
                let reference = gone["reference"].clone();
                if let Some(loads) = r["power"]["loads"].as_array_mut() {
                    loads.retain(|l| l["reference"] != reference);
                }
                cases.push((format!("{name} less {reference}"), r));
            }
        }
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let placed = std::sync::atomic::AtomicUsize::new(0);
    let refused: std::sync::Mutex<Vec<String>> = std::sync::Mutex::default();
    std::thread::scope(|s| {
        for chunk in cases.chunks(cases.len().div_ceil(threads)) {
            let (placed, refused) = (&placed, &refused);
            s.spawn(move || {
                for (name, r) in chunk {
                    match fits(r) {
                        Some(true) => {
                            placed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Some(false) => refused.lock().unwrap().push(name.clone()),
                        None => {}
                    }
                }
            });
        }
    });
    let refused = refused.into_inner().unwrap();
    assert!(refused.is_empty(), "stopped fitting: {refused:?}");
    // 33 removals from each variant (34 with U2), 5 each refused by
    // `validate` (a connector a serial port or the debug port needs, or the
    // flash an acknowledgement names): 8 x 28 + 29 placed.
    assert_eq!(placed.into_inner(), 8 * 28 + 29);
}

/// `AET-WO-1317` §1.3: a board placed on a reseeded rank (the forward turn,
/// on seed 1) is the same bytes on two runs, and the same placement under
/// order-only renamings once names are taken out (`AET-WO-1313c`).
#[test]
fn a_reseeded_placement_is_deterministic_and_name_free() {
    let place = |v: &Value| {
        let (spec, solved, table) = solved(v);
        schematic::placement(&spec, &solved, &table).unwrap()
    };
    let base = turned(fixture(), "forward");
    let once = place(&base);
    assert_eq!(place(&base), once, "two runs");
    let want = name_free(&once);
    for seed in 1..=5 {
        let renamed = rename::order_only(&base, seed);
        assert_ne!(renamed, base, "seed {seed} renames nothing");
        assert_eq!(name_free(&place(&renamed)), want, "seed {seed}");
    }
}
