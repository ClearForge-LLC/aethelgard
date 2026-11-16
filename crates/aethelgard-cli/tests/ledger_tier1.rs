//! `AET-WO-4230` §1.7, §5.4, tier 1: the witness ledger.
//!
//! - The reference's ledger has one row per field the generator lists, plus
//!   each SPI controller's `SR.MODF`, and nothing else: a row dropped, or one
//!   added, fails here. Its counts are pinned.
//! - Every specification in the repository that solves has a ledger: no
//!   field any of them writes is unclassified.
//! - Sentinel-02's scripted devices give their waits (`silicon-only`) and the
//!   registers their models answer 0x00 with a warning (`ignored-by-model`).
//! - The bench sheet's rows for the reference are pinned by hash.
//!
//! With `AET_LEDGER_DUMP=<dir>` set, the reference's row list and bench rows
//! are written there (the work order's report files).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_cli::ledger::{self, Class, Ledger};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use aethelgard_svd_hal::board::{FieldAccess, register_fields};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn solve(bytes: &[u8]) -> Option<(SolvedTopology, PinTable)> {
    let spec: Specification = serde_json::from_slice(bytes).ok()?;
    let sha: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).ok()?;
    let topology = aethelgard_ir::solve(&spec, &table, &sha).ok()?;
    Some((topology, table))
}

fn reference() -> (SolvedTopology, PinTable) {
    solve(&fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap()).unwrap()
}

fn fixture(name: &str) -> (SolvedTopology, PinTable) {
    solve(
        &fs::read(root().join(format!("crates/aethelgard-cli/tests/fixtures/{name}.json")))
            .unwrap(),
    )
    .unwrap()
}

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn reference_ledger() -> Ledger {
    let (t, table) = reference();
    ledger::build(&t, &table).unwrap()
}

#[test]
fn the_reference_has_a_row_for_every_listed_field_and_no_other() {
    let (t, table) = reference();
    let fields = register_fields(&t, &table).unwrap();
    let l = ledger::build(&t, &table).unwrap();
    // The generated Reset's FPU enable first (the tier-2 completeness test
    // found it), then the generator's list, then each controller's MODF.
    let mut want: Vec<String> = vec!["SCB.CPACR.CP10".to_owned(), "SCB.CPACR.CP11".to_owned()];
    want.extend(fields.iter().map(|f| match f.access {
        FieldAccess::Written => f.id(),
        FieldAccess::Reset => format!("{}@reset", f.id()),
    }));
    want.push("SPI1.SR.MODF@reset".to_owned());
    let have: Vec<String> = l.rows.iter().map(|r| r.id.clone()).collect();
    assert_eq!(have, want, "one row per field, in the generator's order");
    assert_eq!(l.total, l.rows.len());
    // Each row carries its field's place and value as the generator listed it.
    for (r, f) in l.rows[2..].iter().zip(&fields) {
        assert_eq!(
            (r.pos, r.width, r.value),
            (f.pos, f.width, f.value),
            "{}",
            r.id
        );
        assert_eq!(r.family, f.family, "{}", r.id);
    }
}

#[test]
fn the_reference_s_counts_are_pinned() {
    let l = reference_ledger();
    assert_eq!(l.total, 264);
    let by_class: BTreeMap<&str, usize> = l.counts.by_class.clone();
    assert_eq!(
        by_class,
        BTreeMap::from([
            ("independent-rule", 67),
            ("observed", 58),
            ("same-source", 59),
            ("ignored-by-model", 79),
            ("silicon-only", 1),
        ])
    );
    let by_family: BTreeMap<String, usize> = l.counts.by_family.clone();
    assert_eq!(
        by_family,
        BTreeMap::from([
            ("CORE".to_owned(), 2),
            ("FLASH".to_owned(), 1),
            ("GPIO".to_owned(), 95),
            ("I2C".to_owned(), 46),
            ("RCC".to_owned(), 27),
            ("SPI".to_owned(), 46),
            ("USART".to_owned(), 47),
        ])
    );
    // The counts are the rows'.
    for c in Class::ALL {
        assert_eq!(
            l.counts.by_class[c.name()],
            l.rows.iter().filter(|r| r.class == c).count()
        );
    }
    assert_eq!(l.evidence.len(), 5);
}

#[test]
fn the_families_the_work_order_names_are_classed_as_ruled() {
    let l = reference_ledger();
    let class = |id: &str| {
        l.rows
            .iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("{id}"))
            .class
    };
    for id in [
        "SPI1.CFG1.MBR",
        "SPI1.CFG2.SSM",
        "SPI1.CFG2.MASTER",
        "SPI1.CR1.SSI",
        "RCC.D2CCIP1R.SPI123SEL",
        "I2C1.CR1.DNF",
        "I2C1.TIMINGR.SCLL",
        "USART1.BRR.DIV_MANTISSA",
    ] {
        assert_eq!(class(id), Class::IndependentRule, "{id}");
    }
    assert_eq!(class("RCC.D2CCIP1R.SPI123SEL@reset"), Class::IgnoredByModel);
    assert_eq!(class("RCC.D1CCIPR.CKPERSEL@reset"), Class::IgnoredByModel);
    assert_eq!(class("SPI1.SR.MODF@reset"), Class::IgnoredByModel);
    assert_eq!(class("FLASH.OPTSR_CUR.BOR_LEV@reset"), Class::SiliconOnly);
    assert_eq!(class("GPIOB.OTYPER.OT6"), Class::IgnoredByModel);
    assert_eq!(class("I2C1.CR1.PE"), Class::SameSource);
    // Every `observed` row's judge cites what showed the model reacting.
    for r in l.rows.iter().filter(|r| r.class == Class::Observed) {
        assert!(
            r.judge.contains("FEEDBACK") || r.judge.contains(".cs L"),
            "{}: {}",
            r.id,
            r.judge
        );
    }
}

/// Every JSON file under `dir`, recursively.
fn jsons(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            jsons(&p, out);
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
}

#[test]
fn every_specification_that_solves_has_a_ledger() {
    let mut files = Vec::new();
    for dir in ["examples", "scenarios", "boards", "crates"] {
        jsons(&root().join(dir), &mut files);
    }
    let mut classified = 0;
    for path in files {
        if path.components().any(|c| c.as_os_str() == "target") {
            continue;
        }
        let Some((t, table)) = solve(&fs::read(&path).unwrap()) else {
            continue;
        };
        // A specification the generator refuses has no firmware to ledger.
        if register_fields(&t, &table).is_err() {
            continue;
        }
        let l = ledger::build(&t, &table)
            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.message()));
        assert!(l.total > 0, "{}", path.display());
        classified += 1;
    }
    assert!(classified >= 15, "{classified} specifications classified");
}

#[test]
fn sentinel_02_s_scripted_devices_give_their_waits_and_model_rows() {
    let (t, table) = fixture("sentinel-02-own");
    let l = ledger::build(&t, &table).unwrap();
    let waits: Vec<_> = l.rows.iter().filter(|r| r.family == "WAIT").collect();
    assert!(!waits.is_empty());
    for w in &waits {
        assert_eq!(w.class, Class::SiliconOnly, "{}", w.id);
        assert!(w.judge.contains("settle covers"), "{}", w.judge);
        assert!(w.register.is_empty());
    }
    let models: Vec<&str> = l
        .rows
        .iter()
        .filter(|r| r.family == "MODEL")
        .map(|r| r.id.as_str())
        .collect();
    assert!(
        models.iter().any(|m| m.ends_with(".reserved")),
        "{models:?}"
    );
    assert!(
        models.iter().any(|m| m.ends_with(".bank3,bank4")),
        "{models:?}"
    );
    assert!(
        l.rows
            .iter()
            .filter(|r| r.family == "MODEL")
            .all(|r| r.class == Class::IgnoredByModel)
    );
    // The reference runs no script and has no own model: neither family.
    let r = reference_ledger();
    assert!(
        !r.rows
            .iter()
            .any(|r| r.family == "WAIT" || r.family == "MODEL")
    );
}

#[test]
fn the_reference_s_bench_rows_are_pinned() {
    let l = reference_ledger();
    let rows = ledger::bench_rows(&l);
    let on: Vec<_> = l
        .rows
        .iter()
        .filter(|r| ledger::on_the_bench(&l, r))
        .collect();
    if let Some(dir) = std::env::var_os("AET_LEDGER_DUMP") {
        let dir = PathBuf::from(dir);
        fs::write(dir.join("bench-rows-reference.md"), &rows).unwrap();
        let mut tsv = String::from("id\tclass\tfamily\taccess\tvalue\n");
        for r in &l.rows {
            tsv += &format!(
                "{}\t{}\t{}\t{}\t{}\n",
                r.id,
                r.class.name(),
                r.family,
                r.access,
                r.value.map_or("run-time".to_owned(), |v| v.to_string())
            );
        }
        fs::write(dir.join("ledger-reference.tsv"), tsv).unwrap();
    }
    // No judged row and no run-time value is on the sheet.
    assert!(on.iter().all(|r| !r.class.judged() && r.value.is_some()));
    assert!(
        rows.contains(
            "| FLASH.OPTSR_CUR (`BOR_LEV`) | `0x5200201c` | `0x0000000c` | `0x0000000c` |"
        )
    );
    assert!(rows.contains("| SPI1.SR (`MODF`) | `0x40013014` | `0x00000200` | `0x00000000` |"));
    assert!(!rows.contains("BSRR") && !rows.contains("ICR"));
    // SPI123SEL is read at reset, then written: only the written row is judged.
    assert!(!rows.contains("D2CCIP1R"));
    assert_eq!(rows.lines().count(), on.len() + 2);
    assert_eq!(on.len(), 120);
    assert_eq!(
        sha(&rows),
        "f25e1c1a5a4e46b6c66cd46b766deb6cf0a5aaa08e76dcea772d9c12cf114664"
    );
}

/// The verdict carries the ledger and the silicon findings: `verify
/// --against` an empty directory differs before any gate runs, and prints
/// the verdict with both, its fab keys never after them. The largest
/// fixture's line stays far under the MCP path's 1 MiB stdout cap.
#[test]
fn verify_s_verdict_carries_the_ledger_and_the_silicon_findings() {
    let empty = std::env::temp_dir().join(format!("aet-4230-c2-{}", std::process::id()));
    fs::create_dir_all(&empty).unwrap();
    let run = |spec: &Path| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(["verify", "--spec"])
            .arg(spec)
            .arg("--against")
            .arg(&empty)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    };
    let line = run(&root().join("examples/sentinel-01-minimal.json"));
    let v: serde_json::Value = serde_json::from_slice(&line).unwrap();
    assert_eq!(v["against"], "differs");
    assert_eq!(v["ledger"]["total"], 264);
    assert_eq!(v["ledger"]["rows"].as_array().unwrap().len(), 264);
    assert_eq!(v["ledger"]["counts"]["by_class"]["independent-rule"], 67);
    assert_eq!(v["ledger"]["evidence"].as_object().unwrap().len(), 5);
    assert_eq!(v["silicon"]["holds"], true);
    assert_eq!(v["silicon"]["spi"][0]["peripheral"], "SPI1");
    assert_eq!(v["silicon"]["i2c"][0]["peripheral"], "I2C1");
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["kind"] != "silicon_rule_fails")
    );
    // In the line's own order: after the findings, the last keys without a
    // fab flag (the fab keys would follow them).
    let text = String::from_utf8(line).unwrap();
    let at = |key: &str| text.find(&format!("\"{key}\":")).unwrap();
    assert!(at("findings") < at("silicon") && at("silicon") < at("ledger"));
    assert!(!text.contains("\"against_fab\":") && !text.contains("\"fab\":"));
    let large = run(&root().join("crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json"));
    assert!(large.len() < 512 * 1024, "{} bytes", large.len());
    fs::remove_dir_all(&empty).unwrap();
}
