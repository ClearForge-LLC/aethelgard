//! `verify` with a seeded set `--against` a tree, two runs made once per
//! test (`AET-WO-1909` B): `AET-WO-3301`'s binding and byte-identity, and
//! `AET-WO-3300`'s package at two paths. The binding is asserted against the
//! seeded run: every gate in order, every campaign run in the set's order,
//! and each run's id and state hash against `campaign` run directly.
//!
//! The two tests shared one pair of runs with `--fab-out` (`AET-WO-1916` D2
//! (i), ruled 2026-11-23 06:35Z) until `AET-WO-4300k` (T), ruled
//! 2026-11-24T11:25Z: `--fab-out` refuses the reference (`FabricationNotReady`),
//! so the test split. The binding's point is the committed reference board
//! crate (`boards/sentinel-01`), so it stays on the reference, `--against` the
//! committed tree with no fab flag; the package moves to the ready fixture
//! (`common/ready.rs`), `--against` the tree `generate` and `emit` write for
//! it. `--against` with no fab flag is also `with_tools`' board-not-drawn
//! test (LP1). Each test reads what it checks from its runs and never writes
//! into them. Each `verify` peaks at about 3.5 GiB: a pair's two runs go at
//! once, and one pair at a time (`PAIR`), so at most two run together
//! (`AET-WO-1918` (3), ruled (ii)), under CI's Kani ceiling of 12 GiB.

#![cfg(feature = "with-tools")]

mod common;
#[path = "common/ready.rs"]
mod ready;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::verify::{
    SEEDED, SPEC, Shared, aethelgard, board, build, copy_tree, gate_reads, one, package_files, ran,
    root, row, scratch, sha,
};
use ready::{READY, READY_SCENARIOS};
use serde_json::Value;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Two runs: their exit codes and stdout, the `--against` tree and, with
/// `--fab-out`, the two package directories, at paths of different lengths.
struct Runs {
    base: PathBuf,
    tree: PathBuf,
    fab: Option<[PathBuf; 2]>,
    codes: [Option<i32>; 2],
    first: String,
    second: String,
}

/// One pair of runs at a time in this process, whichever test asks first:
/// the binary is not colocated (`AET-WO-1918` (3)), so both tests may share a
/// shard, and two pairs at once would be four verifies.
static PAIR: std::sync::Mutex<()> = std::sync::Mutex::new(());

static REFERENCE: Shared<Runs> =
    Shared::new("verify of the reference, seeded set, --against the committed tree, twice");
static PACKAGE: Shared<Runs> =
    Shared::new("verify of the ready fixture, seeded set, --against and --fab-out, at two paths");

/// `verify --spec spec --scenarios scenarios --against <tree>`, twice, each
/// with `--fab-out` at its own path when `fab`. The tree is `board_crate` as
/// `board/` and the KiCad files `emit` writes for `spec` as `kicad/`.
fn two_runs(label: &str, spec: &str, scenarios: &[&str], board_crate: &Path, fab: bool) -> Runs {
    let _pair = PAIR.lock().unwrap_or_else(|e| e.into_inner());
    let base = scratch(label);
    let tree = base.join("against");
    copy_tree(board_crate, &tree.join("board"));
    let kicad = tree.join("kicad");
    let (code, stdout) = aethelgard(&["emit", "--file", spec, "--out", kicad.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(0),
        "emit writes the --against tree's KiCad files: {stdout}"
    );
    let fab = fab.then(|| [base.join("a"), base.join("a-much/longer/path/b")]);
    let run = |i: usize| {
        let mut args = vec!["verify", "--spec", spec, "--scenarios"];
        args.extend(scenarios);
        args.extend(["--against", tree.to_str().unwrap()]);
        if let Some(dirs) = &fab {
            args.extend(["--fab-out", dirs[i].to_str().unwrap()]);
        }
        aethelgard(&args)
    };
    // The two runs are independent, so they run at once (`AET-WO-1918` (3),
    // ruled (ii)): the job's peak is two verifies', under CI's Kani ceiling
    // (`ci/kani-ceiling`, which `ci/ceiling.sh` holds for every shard).
    let ((code_a, first), (code_b, second)) = std::thread::scope(|s| {
        let b = s.spawn(|| run(1));
        (run(0), b.join().unwrap())
    });
    Runs {
        base,
        tree,
        fab,
        codes: [code_a, code_b],
        first,
        second,
    }
}

/// The reference's runs, `--against` the committed board, with no fab flag;
/// `property` is what the calling test checks, named if the runs failed.
fn reference(property: &str) -> &'static Runs {
    REFERENCE.get(property, || {
        two_runs("verify-2k2", SPEC, &SEEDED, &board(), false)
    })
}

/// The ready fixture's runs with `--fab-out`. It has no committed board
/// crate: its `board/` is the one `generate` writes, as `boards/sentinel-01`
/// is the reference's (`generate_tier1.rs`); its `kicad/` is `emit`'s.
fn package(property: &str) -> &'static Runs {
    PACKAGE.get(property, || {
        let generated = scratch("verify-2k2-ready-generate");
        let (code, out) = aethelgard(&[
            "generate",
            "--spec",
            READY,
            "--out",
            generated.to_str().unwrap(),
        ]);
        assert_eq!(
            code,
            Some(0),
            "generate writes the ready board crate: {out}"
        );
        two_runs(
            "verify-2k2-ready",
            READY,
            &READY_SCENARIOS,
            &generated.join("board"),
            true,
        )
    })
}

/// Each seeded scenario's `name`, in `SEEDED`'s order.
fn seeded_names() -> Vec<String> {
    SEEDED
        .iter()
        .map(|f| {
            let bytes = fs::read(root().join(f)).unwrap();
            serde_json::from_slice::<Value>(&bytes).unwrap()["name"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

/// §3.2 and §3.4 of `AET-WO-3301`: `verify` with the seeded set and
/// `--against` the committed board passes; its verdict names every file's
/// hash, the ELF's, every gate's result and every run; two runs print the
/// same bytes. With no fab flag (`AET-WO-4300k` (T)): the point is the
/// committed board, and the reference is not ready for a package.
#[test]
fn verify_binds_the_reference_and_two_runs_print_the_same_bytes() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let r = reference("verify binds the reference and two runs print the same bytes");
    let v = one(&r.first);
    assert_eq!(r.codes[0], Some(0), "the first run passes: {v}");
    assert_eq!(r.codes[1], Some(0), "the second run passes: {}", r.second);
    assert_eq!(r.first, r.second, "two runs print the same bytes");
    assert!(v.get("fab").is_none(), "no fab flag, no fab key: {v}");

    assert_eq!(v["verdict"], "pass", "the verdict is pass: {v}");
    assert_eq!(
        v["against"], "identical",
        "--against finds the committed tree identical: {v}"
    );
    assert_eq!(
        v["rise_times"], "computed from declared values",
        "rise times are computed from declared values: {v}"
    );
    assert_eq!(
        v["engine_version"],
        env!("CARGO_PKG_VERSION"),
        "the verdict names this engine's version: {v}"
    );
    assert_eq!(
        v["specification_sha256"],
        sha(&fs::read(root().join(SPEC)).unwrap()),
        "the verdict names the specification's sha256: {v}"
    );
    // Every file of the tree, by path, with the hash of the bytes compared.
    let mut want = serde_json::Map::new();
    let mut stack = vec![r.tree.clone()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p
                    .strip_prefix(&r.tree)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                want.insert(rel, sha(&fs::read(&p).unwrap()).into());
            }
        }
    }
    assert_eq!(
        v["files"],
        Value::Object(want),
        "the verdict names every file of the tree by the hash of its bytes"
    );
    let gates: Vec<(&str, &str)> = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| (g["gate"].as_str().unwrap(), g["verdict"].as_str().unwrap()))
        .collect();
    assert_eq!(
        gates,
        [
            ("erc", "pass"),
            ("drc", "pass"),
            ("support", "pass"),
            ("extract", "pass"),
            // One per harness: the scheduler, I2C1's transfer, since
            // `AET-WO-4200` SPI1's transfer and its frame bound, and since
            // `AET-WO-4300b` the console's: the reset clock check and
            // USART1's bring-up, generated for a board with a serial port;
            // since `AET-WO-4230` the brown-out word's one flash read; since
            // `AET-WO-3303b` the scheduler in two parts, I2C1's and SPI1's;
            // since `AET-WO-4240v` the supply and VOS Scale 2 sequence; since
            // `AET-WO-4240e` the kernel-clock switch.
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("build", "pass"),
            ("simulate", "pass"),
            // The seeded set's two faults detected by their mechanisms
            // (`AET-WO-3303` D1, `AET-WO-2302` D5), one gate each.
            ("attribute", "pass"),
            ("attribute", "pass"),
            ("campaign", "pass"),
        ],
        "every gate runs, in order, and passes"
    );
    let attributed: Vec<&str> = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["gate"] == "attribute")
        .map(|g| g["scenario"].as_str().unwrap())
        .collect();
    assert_eq!(
        attributed,
        ["spi-absent-u4", "serial-rx-open-console"],
        "each fault the seeded set detects by its mechanism is an attribute gate, in order"
    );
    // `AET-WO-3303b` §1.2: a board without a timer proves its schedule in
    // parts, each a gate; the whole harness is no longer a proof, so it is no
    // gate, and no harness drops out silently.
    let proved: Vec<&str> = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["gate"] == "prove")
        .map(|g| g["harness"].as_str().unwrap())
        .collect();
    assert_eq!(
        proved,
        [
            "proofs::every_valid_sequence_i2c1",
            "proofs::every_valid_sequence_spi1",
            "proofs::i2c1_transfer_is_bounded_and_ordered",
            "proofs::spi1_transfer_is_bounded_and_ordered",
            "proofs::spi1_transfer_refuses_too_many_frames",
            "proofs::reset_clock_is_ok_exactly_at_reset",
            "proofs::usart1_start_writes_the_solved_values",
            "proofs::flash_option_status_is_one_read",
            "proofs::pwr_scale2_is_once_in_order_and_bounded",
            // `AET-WO-4240e`: the kernel-clock switch.
            "proofs::spi_kernel_switch_is_in_order_bounded_and_named",
        ],
        "every harness of the generated crate is a prove gate"
    );
    // Each gate's boundary is what its tools' read grants make it (ruled on
    // PR #96): in the pinned image, Kani's kit is uid 1001's, so `prove` says
    // `toolchain`; under rootless podman every gate says `user_namespace`.
    for g in v["gates"].as_array().unwrap() {
        if g["gate"] == "prove" {
            assert_eq!(
                g["harness_provenance"], "generated",
                "each prove gate proves the generated harness: {g}"
            );
        }
        // `AET-WO-2308` L3: the campaign gate names the Renode build its
        // runs' archives record, the one the twin is pinned to.
        if g["gate"] == "campaign" {
            assert_eq!(
                g["tool_version"],
                aethelgard_twin::platform::RENODE_BUILD,
                "the campaign names its runs' Renode build: {g}"
            );
        }
        // An attribute gate judges the campaign's records and runs no tool,
        // so it has no uid boundary to report (null, never a value).
        if g["gate"] == "attribute" {
            assert_eq!(
                g["uid_boundary"],
                Value::Null,
                "attribute reports no uid boundary: {g}"
            );
            assert!(
                g.get("uid_boundary_unmet").is_none(),
                "attribute reports no unmet uid boundary: {g}"
            );
            continue;
        }
        // `AET-WO-1305`: extract is bound to the generated board, and runs no
        // tool, so it has no uid boundary to report (null, never a value).
        if g["gate"] == "extract" {
            assert_eq!(
                g["input_sha256"], v["files"]["kicad/sentinel-01.kicad_pcb"],
                "extract reads the generated board: {g}"
            );
            assert_eq!(
                g["uid_boundary"],
                Value::Null,
                "extract reports no uid boundary: {g}"
            );
            assert!(
                g.get("uid_boundary_unmet").is_none(),
                "extract reports no unmet uid boundary: {g}"
            );
            continue;
        }
        let (holds, unmet) = common::expected_boundary(&gate_reads(g["gate"].as_str().unwrap()));
        assert_eq!(
            g["uid_boundary"], holds,
            "the gate's uid boundary is its grants': {g}"
        );
        assert_eq!(
            g["uid_boundary_unmet"].as_str(),
            unmet,
            "the gate's unmet uid boundary is its grants': {g}"
        );
    }
    // The ELF is the one `build` gives for the committed board anywhere.
    let elf = r.base.join("reference.elf");
    let (_, bytes) = build(&board(), &elf);
    assert_eq!(
        v["elf_sha256"],
        sha(&bytes),
        "the verdict's ELF is the one build gives"
    );
    // The campaign ran every seeded scenario, in `SEEDED`'s order: the
    // tolerated faults and the baseline pass; the two detected faults fail as
    // each declares, and so match.
    let runs = v["campaign"].as_array().unwrap();
    let ran: Vec<(&str, &str, &str, bool)> = runs
        .iter()
        .map(|run| {
            (
                run["scenario"].as_str().unwrap(),
                run["verdict"].as_str().unwrap(),
                run["expect"].as_str().unwrap(),
                run["matched"].as_bool().unwrap(),
            )
        })
        .collect();
    let names = seeded_names();
    assert_eq!(
        ran,
        [
            (names[0].as_str(), "pass", "pass", true),
            (names[1].as_str(), "pass", "pass", true),
            (names[2].as_str(), "fail", "fail", true),
            (names[3].as_str(), "pass", "pass", true),
            (names[4].as_str(), "fail", "fail", true),
        ],
        "the campaign ran every seeded scenario, in order, each as it declares: {v}"
    );
    assert_eq!(
        names[3], "pullup-2k2",
        "the 2.2 kΩ baseline is seeded: {names:?}"
    );
    for run in runs {
        assert_eq!(
            run["run_id"].as_str().unwrap().len(),
            32,
            "each campaign run id is 32 characters: {run}"
        );
    }
    // The same run ids `campaign` gives that ELF and the seeded set directly.
    let archive = r.base.join("archive");
    let mut args = vec!["campaign", "--scenarios"];
    args.extend(SEEDED);
    args.extend([
        "--firmware",
        elf.to_str().unwrap(),
        "--archive",
        archive.to_str().unwrap(),
    ]);
    let (_, lines) = aethelgard(&args);
    let direct: Vec<Value> = lines
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|l| l.get("scenario").is_some())
        .collect();
    assert_eq!(
        direct.len(),
        runs.len(),
        "campaign gives one line per seeded scenario: {lines}"
    );
    for (run, line) in runs.iter().zip(&direct) {
        assert_eq!(
            run["scenario"], line["scenario"],
            "campaign runs the seeded set in verify's order"
        );
        assert_eq!(
            run["run_id"], line["run_id"],
            "verify's run id is the one campaign gives the ELF directly: {run}"
        );
        assert_eq!(
            run["state_sequence_hash"], line["state_sequence_hash"],
            "verify's state hash is the one campaign gives the ELF directly: {run}"
        );
    }
}

/// `AET-WO-3300` §3, the package probe, on the ready fixture (`AET-WO-4300k`
/// (T), ruled 11:25Z: the reference made ready for a package; the reference
/// itself is refused `FabricationNotReady`, `fab_tier1.rs`): `verify
/// --fab-out` with the seeded set passes and writes the package; the
/// verdict's `fab` hashes equal the files' sha256; two runs at two paths
/// print the same verdict and write the same bytes; the ELF is the one the
/// campaign ran and the BOM is `emit`'s; the manifest names the verdict line
/// its own run printed, without its `fab` key (that line carries `against`,
/// as the run does: `AET-WO-1909` B, ruled acceptable); B.Cu carries the GND
/// pour and no Gerber carries KiCad's clock (ruling on PR #103).
/// `--against-fab` is `against_fab_tier2.rs`.
#[test]
fn verify_fab_out_writes_the_package_its_verdict_names_at_any_path() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let r = package("verify --fab-out writes the package its verdict names at any path");
    let [a, b] = r.fab.as_ref().unwrap();
    let v = one(&r.first);
    assert_eq!(r.codes[0], Some(0), "the run at the first path passes: {v}");
    assert_eq!(
        r.codes[1],
        Some(0),
        "the run at the second path passes: {}",
        r.second
    );
    assert_eq!(
        r.first, r.second,
        "two runs at two paths print the same verdict"
    );
    let package = package_files(a);
    assert_eq!(
        package,
        package_files(b),
        "two runs at two paths write the same bytes"
    );

    assert_eq!(v["verdict"], "pass", "the verdict is pass: {v}");
    // `AET-WO-2306` §1.5: on the ready fixture every gate judged something, as
    // the inventory found, and the droop's recovery was witnessed. Read from
    // the table since `AET-WO-1610` (2) removed `judged_nothing`: no row is
    // `judged 0`.
    assert!(v.get("judged_nothing").is_none(), "{v}");
    let rows = ran(&v);
    assert!(
        rows.iter().all(|(_, s)| s != "judged 0"),
        "nothing judged nothing: {rows:?}"
    );
    // `AET-WO-1609` §2: the ready fixture's ran-or-not table, every
    // gate and part by name. It declares no drop limit, no crystal, no pair,
    // no power tree and no pin device, so each of those parts is off by its
    // key; its six scenarios ran (`AET-WO-4310` (5) added SENSORS' held-low
    // SDA), and its three detected faults were attributed.
    for (gate, state) in [
        ("extract.supply", "off: power.drop_limits"),
        ("extract.oscillator", "off: mcu.hse.crystal"),
        ("extract.pairs", "off: diff_pairs"),
        ("drc.pairs", "off: diff_pairs"),
        ("drc.power", "off: power"),
        ("drc.orientation", "off: spi_devices[].orientation"),
        ("drc.outline", "off: board.max_outline_um"),
        ("simulate.hse_clock", "off: mcu.hse"),
        ("gpio", "off: pin_devices[].signals"),
        ("adc", "off: pin_devices[].analog"),
        ("bridge", "off: pin_devices[].bridges"),
        ("campaign", "judged 6"),
        ("build", "judged 1"),
        ("simulate", "judged 1"),
    ] {
        assert_eq!(row(&rows, gate), state, "{gate}: {rows:?}");
    }
    assert_eq!(
        rows.iter()
            .filter(|(n, s)| n.starts_with("attribute#") && s == "judged 1")
            .count(),
        3,
        "{rows:?}"
    );
    assert!(
        row(&rows, "extract.lines").starts_with("judged "),
        "{rows:?}"
    );
    for g in v["gates"].as_array().unwrap() {
        assert!(g["judged"].as_u64() >= Some(1), "{g}");
        assert!(g.get("nothing_in_scope").is_none(), "{g}");
    }
    for run in v["campaign"].as_array().unwrap() {
        let w = &run["observation_witness"];
        if run["scenario"] == "droop-bor3" {
            assert!(
                w["judged"].as_u64() >= Some(1) && w.get("nothing_in_scope").is_none(),
                "{run}"
            );
        } else {
            assert!(w.is_null(), "no droop, no observation: {run}");
        }
    }
    let fab = &v["fab"];
    assert_eq!(fab["status"], "written", "the package is written: {fab}");
    assert_eq!(
        fab["tool_version"], "10.0.7",
        "the package names KiCad 10.0.7: {fab}"
    );
    assert_eq!(
        fab["board_sha256"], v["files"]["kicad/sentinel-01-ready.kicad_pcb"],
        "the package names the generated board: {fab}"
    );
    let drc = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["gate"] == "drc")
        .unwrap();
    assert_eq!(
        fab["uid_boundary"], drc["uid_boundary"],
        "the package's uid boundary is DRC's: {fab}"
    );
    // The verdict names every file written, and only those, by their bytes.
    let named: std::collections::BTreeMap<String, String> = fab["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, h)| (k.clone(), h.as_str().unwrap().to_owned()))
        .collect();
    let hashed: std::collections::BTreeMap<String, String> = package
        .iter()
        .map(|(k, bytes)| (k.clone(), sha(bytes)))
        .collect();
    assert_eq!(
        named, hashed,
        "the verdict names every file written, and only those, by sha256"
    );
    let mut names: Vec<&str> = package.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "FAB-NOTES.md",
            "MANIFEST.json",
            "sentinel-01-ready-B_Cu.gbl",
            "sentinel-01-ready-B_Mask.gbs",
            "sentinel-01-ready-B_Paste.gbp",
            "sentinel-01-ready-B_Silkscreen.gbo",
            "sentinel-01-ready-Edge_Cuts.gm1",
            "sentinel-01-ready-F_Cu.gtl",
            "sentinel-01-ready-F_Mask.gts",
            "sentinel-01-ready-F_Paste.gtp",
            "sentinel-01-ready-F_Silkscreen.gto",
            "sentinel-01-ready-jlcpcb-bom.csv",
            "sentinel-01-ready-jlcpcb-cpl.csv",
            "sentinel-01-ready-pos.csv",
            "sentinel-01-ready.bom.csv",
            "sentinel-01-ready.drl",
            "sentinel-01-ready.elf",
        ],
        "the package is exactly these seventeen files"
    );
    // The firmware is the ELF `build` gave, `simulate` booted and the
    // campaign's run was named by (verify binds the run id to it).
    assert_eq!(
        named["sentinel-01-ready.elf"],
        v["elf_sha256"].as_str().unwrap(),
        "the packaged ELF is the one verify built"
    );
    assert_eq!(
        v["campaign"][0]["verdict"], "pass",
        "the packaged ELF's campaign passed: {v}"
    );
    // The BOM is `emit`'s, byte for byte: the one the shared runs' `--against`
    // tree holds, which `emit` wrote and `--against` found identical.
    assert_eq!(
        package["sentinel-01-ready.bom.csv"],
        fs::read(r.tree.join("kicad/sentinel-01-ready.bom.csv")).unwrap(),
        "the packaged BOM is emit's, byte for byte"
    );
    // `AET-WO-1604` §1.3: the JLCPCB files are the package's own BOM and
    // placement file in JLCPCB's format, nothing else.
    let (jlcpcb_bom, jlcpcb_cpl) = aethelgard_eda::fab::jlcpcb(
        &package["sentinel-01-ready.bom.csv"],
        &package["sentinel-01-ready-pos.csv"],
    )
    .unwrap();
    assert_eq!(
        package["sentinel-01-ready-jlcpcb-bom.csv"],
        jlcpcb_bom.into_bytes()
    );
    assert_eq!(
        package["sentinel-01-ready-jlcpcb-cpl.csv"],
        jlcpcb_cpl.into_bytes()
    );
    // `AET-WO-2308` (1): U4's symbol stands in for the W25Q64JVSSIQ, and the
    // stand-in is labelled wherever the package lists the part by value: the
    // BOM, the JLCPCB BOM's `Note` and FAB-NOTES' parts with a note.
    let standin = "symbol Memory_Flash:W25Q32JVSS stands in for W25Q64JVSSIQ: same family, same SOIC-8 208-mil pinout (AET-WO-1300 D2)";
    let text = |name: &str| String::from_utf8(package[name].clone()).unwrap();
    assert!(
        text("sentinel-01-ready.bom.csv").contains(&format!("U4,W25Q64JVSSIQ,Package_SO:SOIC-8_5.3x5.3mm_P1.27mm,Memory_Flash:W25Q32JVSS,\"{standin}\"\r\n")),
        "the BOM labels U4"
    );
    assert!(
        text("sentinel-01-ready-jlcpcb-bom.csv").contains(&format!(
            "W25Q64JVSSIQ,\"U4\",SOIC-8_5.3x5.3mm_P1.27mm,,\"{standin}\"\r\n"
        )),
        "the JLCPCB BOM labels U4"
    );
    assert!(
        text("FAB-NOTES.md").contains("\n## Parts with a note\n")
            && text("FAB-NOTES.md").ends_with(&format!("- U4 (W25Q64JVSSIQ): {standin}\n")),
        "FAB-NOTES lists U4 last"
    );
    // The manifest: every other file, and the verdict line this run printed
    // without its `fab` key (`AET-WO-1916` D2: derived from its own line).
    let manifest: Value = serde_json::from_slice(&package["MANIFEST.json"]).unwrap();
    let line = r.first.trim_end();
    let without = format!("{}}}", &line[..line.rfind(",\"fab\":").unwrap()]);
    assert_eq!(
        manifest["verdict_sha256"],
        sha(without.as_bytes()),
        "the manifest names the verdict without its fab key"
    );
    assert_eq!(
        manifest["specification_sha256"], v["specification_sha256"],
        "the manifest names the specification"
    );
    assert_eq!(
        manifest["topology_sha256"], v["topology_sha256"],
        "the manifest names the topology"
    );
    let scenarios: Vec<Value> = READY_SCENARIOS
        .iter()
        .map(|f| {
            let bytes = fs::read(root().join(f)).unwrap();
            let name = serde_json::from_slice::<Value>(&bytes).unwrap()["name"].clone();
            serde_json::json!({"scenario": name, "sha256": sha(&bytes)})
        })
        .collect();
    assert_eq!(
        manifest["scenarios"],
        Value::Array(scenarios),
        "the manifest names each scenario and its sha256"
    );
    // `AET-WO-3303` §2: the ready fixture package's verdict, and its manifest,
    // list every precondition as met; FLASH's absent device and the
    // console's open RX line were each detected by its mechanism (each
    // `attribute` gate passed, `AET-WO-2302` D5).
    let met = serde_json::json!([
        {"precondition": "every_harness_proved", "state": "met"},
        {"precondition": "campaign_faults_every_bus_and_rail", "state": "met"},
        {"precondition": "mcu_support_network_judged", "state": "met"},
        {"precondition": "every_gate_passed", "state": "met"},
    ]);
    assert_eq!(fab["preconditions"], met, "{fab}");
    assert_eq!(manifest["preconditions"], met, "{manifest}");
    assert!(
        v["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["gate"] == "attribute"
                && g["scenario"] == "spi-absent-u4"
                && g["verdict"] == "pass"),
        "{v}"
    );
    // The console's port met by its open RX line, detected by `E3`
    // (`AET-WO-2302` D5).
    assert!(
        v["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["gate"] == "attribute"
                && g["scenario"] == "serial-rx-open-console"
                && g["verdict"] == "pass"),
        "{v}"
    );
    let mut listed = named.clone();
    listed.remove("MANIFEST.json");
    assert_eq!(
        manifest["files"],
        serde_json::to_value(&listed).unwrap(),
        "the manifest names every other file by sha256"
    );
    // Condition 8: B.Cu carries a region of the GND zone. No Gerber carries
    // KiCad's clock lines.
    let bcu = String::from_utf8(package["sentinel-01-ready-B_Cu.gbl"].clone()).unwrap();
    assert!(
        bcu.contains("%TO.N,GND*%\nG36*\n") && bcu.contains("\nG37*\n"),
        "B.Cu carries a region of the GND pour"
    );
    // FAB-NOTES.md names the two removed lines in prose, so only the Gerbers
    // are searched.
    for (name, bytes) in package.iter().filter(|(n, _)| n.contains(".g")) {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains("CreationDate") && !text.contains("Created by"),
            "no Gerber carries KiCad's clock lines: {name}"
        );
    }
    let notes = String::from_utf8(package["FAB-NOTES.md"].clone()).unwrap();
    // The specification's sha256, of the file `verify` read: computed, not
    // pinned, so a specification version bump does not move this test
    // (`AET-WO-4400d`: the literal prefix `b9e95cdc` was version 15's).
    let spec_sha256 = sha(&fs::read(root().join(READY)).unwrap());
    for want in [
        "70 um",
        "0.34 mm to 2 mm",
        "FR-4",
        "plated via, drilled 0.3 mm",
        spec_sha256.as_str(),
    ] {
        assert!(notes.contains(want), "FAB-NOTES.md states {want}");
    }
    // The flash is `nrnd`, acknowledged, and its lifecycle held
    // (`AET-WO-4211`): the lifecycle section is its one banner line, with no
    // unheld line (`verify`'s `lifecycle_unheld`).
    let lifecycle = notes
        .split_once("## Lifecycle\n")
        .and_then(|(_, rest)| rest.split_once("\n## "))
        .map(|(section, _)| section)
        .expect("FAB-NOTES.md has a lifecycle section");
    let lines: Vec<&str> = lifecycle.lines().filter(|l| l.starts_with("- ")).collect();
    assert_eq!(lines.len(), 1, "{lifecycle}");
    assert!(
        lines[0]
            .starts_with("- LIFECYCLE NRND: W25Q64JVxIQ (U4) is not recommended for new designs"),
        "{lifecycle}"
    );
    // `AET-WO-3402` §1.2: the acknowledged ledger rows, one line per entry of
    // the specification, in its order, each its register, class, fields and
    // reason exactly as written; a row that differs from its entry fails.
    let section = notes
        .split_once("## Acknowledged ledger rows\n")
        .and_then(|(_, rest)| rest.split_once("\n## "))
        .map(|(section, _)| section)
        .expect("FAB-NOTES.md has the acknowledged ledger rows");
    let written: Vec<&str> = section.lines().filter(|l| l.starts_with("- ")).collect();
    let spec: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root().join(READY)).unwrap()).unwrap();
    let want: Vec<String> = spec["acknowledge_unwitnessed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            let fields: Vec<String> = a["fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| format!("`{}`", f.as_str().unwrap()))
                .collect();
            format!(
                "- `{}` ({}): {}. {}",
                a["register"].as_str().unwrap(),
                a["class"].as_str().unwrap(),
                fields.join(", "),
                a["reason"].as_str().unwrap()
            )
        })
        .collect();
    assert!(!want.is_empty(), "the ready fixture acknowledges rows");
    assert_eq!(
        written, want,
        "FAB-NOTES.md's ledger rows are the specification's"
    );
    // The notes' file list describes every other file, and only those. It is
    // read from its own section: `AET-WO-3402` §1.2's acknowledged rows and
    // `AET-WO-1604` §1.3's JLCPCB section have bullets that start with a
    // backtick too.
    let files = notes
        .split_once("## Files\n")
        .and_then(|(_, rest)| rest.split_once("\n## "))
        .map(|(section, _)| section)
        .expect("FAB-NOTES.md has a file list");
    let described: Vec<&str> = files
        .lines()
        .filter_map(|l| l.strip_prefix("- `")?.split('`').next())
        .collect();
    let others: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| *n != "FAB-NOTES.md" && *n != "MANIFEST.json")
        .collect();
    assert_eq!(
        described, others,
        "FAB-NOTES.md describes every other file, and only those"
    );
}
