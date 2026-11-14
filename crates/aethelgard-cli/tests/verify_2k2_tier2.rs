//! `verify` on the reference with the 2.2 kΩ scenario, run twice and read by
//! two tests (`AET-WO-1909` B, ruled on PR #115): `AET-WO-3301`'s binding and
//! byte-identity, and `AET-WO-3300`'s package at two paths. Both runs carry
//! `--against` the committed tree and `--fab-out`, at two paths of different
//! lengths. The flags read and write disjoint state, and each verdict equals
//! the single-flag one apart from the keys the other flag adds
//! (`AET-WO-1909` FEEDBACK). Every tool run happens in the shared run, once.

#![cfg(feature = "with-tools")]

mod common;

use std::fs;
use std::path::PathBuf;

use common::verify::{
    SPEC, Shared, aethelgard, board, build, copy_tree, gate_reads, one, package_files, root,
    scratch, sha,
};
use serde_json::Value;

/// The two runs: their exit codes and stdout, the `--against` tree, and the
/// two `--fab-out` directories.
struct Runs {
    base: PathBuf,
    tree: PathBuf,
    a: PathBuf,
    b: PathBuf,
    codes: [Option<i32>; 2],
    first: String,
    second: String,
}

static RUNS: Shared<Runs> = Shared::new("verify 2k2 --against --fab-out, twice at two paths");

fn runs(property: &str) -> &'static Runs {
    RUNS.get(property, || {
        let base = scratch("verify-2k2");
        let tree = base.join("against");
        copy_tree(&board(), &tree.join("board"));
        let kicad = tree.join("kicad");
        let (code, stdout) =
            aethelgard(&["emit", "--file", SPEC, "--out", kicad.to_str().unwrap()]);
        assert_eq!(
            code,
            Some(0),
            "emit writes the --against tree's KiCad files: {stdout}"
        );
        let a = base.join("a");
        let b = base.join("a-much/longer/path/b");
        let run = |fab: &PathBuf| {
            aethelgard(&[
                "verify",
                "--spec",
                SPEC,
                "--scenarios",
                "scenarios/sentinel-01/pullup-2k2.json",
                "--against",
                tree.to_str().unwrap(),
                "--fab-out",
                fab.to_str().unwrap(),
            ])
        };
        let (code_a, first) = run(&a);
        let (code_b, second) = run(&b);
        Runs {
            base,
            tree,
            a,
            b,
            codes: [code_a, code_b],
            first,
            second,
        }
    })
}

/// §3.2 and §3.4 of `AET-WO-3301`: `verify` with the 2.2 kΩ scenario and
/// `--against` the committed board passes; its verdict names every file's
/// hash, the ELF's, every gate's result and the run; two runs print the same
/// bytes.
#[test]
fn verify_binds_the_reference_and_two_runs_print_the_same_bytes() {
    let r = runs("verify binds the reference and two runs print the same bytes");
    let v = one(&r.first);
    assert_eq!(r.codes[0], Some(0), "the first run passes: {v}");
    assert_eq!(r.codes[1], Some(0), "the second run passes: {}", r.second);
    assert_eq!(r.first, r.second, "two runs print the same bytes");

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
            ("extract", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("build", "pass"),
            ("simulate", "pass"),
            ("campaign", "pass"),
        ],
        "every gate runs, in order, and passes"
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
    let runs = v["campaign"].as_array().unwrap();
    assert_eq!(runs.len(), 1, "the campaign ran one scenario: {v}");
    assert_eq!(
        runs[0]["scenario"], "pullup-2k2",
        "the campaign ran pullup-2k2: {v}"
    );
    assert_eq!(runs[0]["verdict"], "pass", "the campaign's run passes: {v}");
    assert_eq!(
        runs[0]["run_id"].as_str().unwrap().len(),
        32,
        "the campaign's run id is 32 characters: {v}"
    );
    // The same run id `campaign` gives that ELF and scenario directly.
    let archive = r.base.join("archive");
    let (_, lines) = aethelgard(&[
        "campaign",
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        "--firmware",
        elf.to_str().unwrap(),
        "--archive",
        archive.to_str().unwrap(),
    ]);
    let line: Value = serde_json::from_str(lines.lines().next().unwrap()).unwrap();
    assert_eq!(
        runs[0]["run_id"], line["run_id"],
        "verify's run id is the one campaign gives the ELF directly"
    );
    assert_eq!(
        runs[0]["state_sequence_hash"], line["state_sequence_hash"],
        "verify's state hash is the one campaign gives the ELF directly"
    );
}

/// `AET-WO-3300` §3, the reference probe: `verify --fab-out` with the 2.2 kΩ
/// scenario passes and writes the package; the verdict's `fab` hashes equal
/// the files' sha256; two runs at two paths print the same verdict and write
/// the same bytes; the ELF is the one the campaign ran and the BOM is
/// `emit`'s; the manifest names the verdict the call prints without
/// `--fab-out`; B.Cu carries the GND pour and no Gerber carries KiCad's
/// clock (ruling on PR #103). `--against-fab` is `against_fab_tier2.rs`.
#[test]
fn verify_fab_out_writes_the_package_its_verdict_names_at_any_path() {
    let r = runs("verify --fab-out writes the package its verdict names at any path");
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
    let package = package_files(&r.a);
    assert_eq!(
        package,
        package_files(&r.b),
        "two runs at two paths write the same bytes"
    );

    assert_eq!(v["verdict"], "pass", "the verdict is pass: {v}");
    let fab = &v["fab"];
    assert_eq!(fab["status"], "written", "the package is written: {fab}");
    assert_eq!(
        fab["tool_version"], "10.0.7",
        "the package names KiCad 10.0.7: {fab}"
    );
    assert_eq!(
        fab["board_sha256"], v["files"]["kicad/sentinel-01.kicad_pcb"],
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
            "sentinel-01-B_Cu.gbl",
            "sentinel-01-B_Mask.gbs",
            "sentinel-01-B_Paste.gbp",
            "sentinel-01-B_Silkscreen.gbo",
            "sentinel-01-Edge_Cuts.gm1",
            "sentinel-01-F_Cu.gtl",
            "sentinel-01-F_Mask.gts",
            "sentinel-01-F_Paste.gtp",
            "sentinel-01-F_Silkscreen.gto",
            "sentinel-01-pos.csv",
            "sentinel-01.bom.csv",
            "sentinel-01.drl",
            "sentinel-01.elf",
        ],
        "the package is exactly these fifteen files"
    );
    // The firmware is the ELF `build` gave, `simulate` booted and the
    // campaign's run was named by (verify binds the run id to it).
    assert_eq!(
        named["sentinel-01.elf"],
        v["elf_sha256"].as_str().unwrap(),
        "the packaged ELF is the one verify built"
    );
    assert_eq!(
        v["campaign"][0]["verdict"], "pass",
        "the packaged ELF's campaign passed: {v}"
    );
    // The BOM is `emit`'s, byte for byte.
    let emitted = r.base.join("emit");
    let (code, out) = aethelgard(&["emit", "--file", SPEC, "--out", emitted.to_str().unwrap()]);
    assert_eq!(code, Some(0), "emit writes the BOM to compare: {out}");
    assert_eq!(
        package["sentinel-01.bom.csv"],
        fs::read(emitted.join("sentinel-01.bom.csv")).unwrap(),
        "the packaged BOM is emit's, byte for byte"
    );
    // The manifest: every other file, and the verdict without its `fab` key,
    // which is the line the same call prints without `--fab-out`.
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
    let scenario = fs::read(root().join("scenarios/sentinel-01/pullup-2k2.json")).unwrap();
    assert_eq!(
        manifest["scenarios"],
        serde_json::json!([{"scenario": "pullup-2k2", "sha256": sha(&scenario)}]),
        "the manifest names the scenario and its sha256"
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
    let bcu = String::from_utf8(package["sentinel-01-B_Cu.gbl"].clone()).unwrap();
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
    for want in [
        "70 um",
        "0.34 mm to 2 mm",
        "FR-4",
        "plated via, drilled 0.3 mm",
        "acd60081",
    ] {
        assert!(notes.contains(want), "FAB-NOTES.md states {want}");
    }
    // The notes describe every other file, and only those.
    let described: Vec<&str> = notes
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
