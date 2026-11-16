//! `AET-WO-4230` §1.7, §5.4, tier 1: the witness ledger.
//!
//! - The reference's ledger has one row per field the generator lists, plus
//!   each SPI controller's `SR.MODF`, and nothing else: a row dropped, or one
//!   added, fails here. Its counts are pinned.
//! - Every specification in the repository that solves has a ledger: no
//!   field any of them writes is unclassified.
//! - Sentinel-02's scripted devices give their waits (`silicon-only`) and the
//!   registers their models answer 0x00 with a warning (`ignored-by-model`),
//!   as twin limits outside the ledger (the architect's ruling on Q-I).
//! - The bench sheet's rows for the reference are pinned by hash.
//! - `AET-WO-4230` R1: the reference acknowledges every unwitnessed row by
//!   register and class; a field the ledger newly has is not acknowledged;
//!   an entry naming what the ledger lacks is stale. R5: a motor output's
//!   `PUPDR` field is the independent rule's.
//!
//! With `AET_LEDGER_DUMP=<dir>` set, the reference's row list and bench rows
//! are written there (the work order's report files).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_cli::ledger::{self, Class, Ledger};
use aethelgard_ir::{PinTable, SolvedTopology, Specification, UnwitnessedAcknowledgement};
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

fn reference_spec() -> Specification {
    serde_json::from_slice(&fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap())
        .unwrap()
}

/// One entry per register and unwitnessed class of `l`, naming every field.
fn every_row_acknowledged(l: &Ledger) -> Vec<UnwitnessedAcknowledgement> {
    let mut by: BTreeMap<(String, &str), Vec<String>> = BTreeMap::new();
    for r in &l.rows {
        if let Some(c) = r.class.unwitnessed() {
            by.entry((r.register.clone(), c.name()))
                .or_default()
                .push(r.field.clone());
        }
    }
    by.into_iter()
        .map(|((register, class), fields)| UnwitnessedAcknowledgement {
            register,
            class: class.to_owned(),
            fields,
            reason: "a test's".to_owned(),
        })
        .collect()
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
    // `AET-WO-4230` R5: `PUPDR` is written whole per port, 16 rows each on
    // ports A and B (24 more than the 8 solved pins' rows); R7: `D2CKRDY`.
    assert_eq!(l.total, 289);
    let by_class: BTreeMap<&str, usize> = l.counts.by_class.clone();
    assert_eq!(
        by_class,
        BTreeMap::from([
            ("independent-rule", 67),
            ("observed", 58),
            ("same-source", 83),
            ("ignored-by-model", 80),
            ("silicon-only", 1),
        ])
    );
    let by_family: BTreeMap<String, usize> = l.counts.by_family.clone();
    assert_eq!(
        by_family,
        BTreeMap::from([
            ("CORE".to_owned(), 2),
            ("FLASH".to_owned(), 1),
            ("GPIO".to_owned(), 119),
            ("I2C".to_owned(), 46),
            ("RCC".to_owned(), 28),
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
        "USART1.BRR.DIV_FRACTION",
        "SPI1.CFG2.AFCNTR",
    ] {
        assert_eq!(class(id), Class::IndependentRule, "{id}");
    }
    assert_eq!(class("RCC.D2CCIP1R.SPI123SEL@reset"), Class::IgnoredByModel);
    assert_eq!(class("RCC.D1CCIPR.CKPERSEL@reset"), Class::IgnoredByModel);
    assert_eq!(class("SPI1.SR.MODF@reset"), Class::IgnoredByModel);
    assert_eq!(class("FLASH.OPTSR_CUR.BOR_LEV@reset"), Class::SiliconOnly);
    assert_eq!(class("GPIOB.OTYPER.OT6"), Class::IgnoredByModel);
    assert_eq!(class("I2C1.CR1.PE"), Class::SameSource);
    assert_eq!(class("RCC.CR.D2CKRDY@reset"), Class::IgnoredByModel);
    // R3: BRR's judge is the independent inequality, not the solver's.
    let brr = l
        .rows
        .iter()
        .find(|r| r.id == "USART1.BRR.DIV_MANTISSA")
        .unwrap();
    assert!(
        brr.judge.starts_with("aethelgard_ir::silicon::check_brr"),
        "{}",
        brr.judge
    );
    // The reference has no motor output: every PUPDR field is same-source.
    assert!(
        l.rows
            .iter()
            .filter(|r| r.register.ends_with(".PUPDR"))
            .all(|r| r.class == Class::SameSource)
    );
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

/// The architect's ruling on Q-I: the ledger covers register fields only.
/// Sentinel-02's scripted devices' waits and model rows are twin limits,
/// never acknowledged: a specification acknowledging every ledger row
/// leaves nothing unacknowledged, so a scripted board is not blocked by rows
/// no acknowledgement can name. The wait is one (R6), the largest settle.
#[test]
fn sentinel_02_s_scripted_devices_give_their_waits_and_model_rows() {
    let (t, table) = fixture("sentinel-02-own");
    let mut l = ledger::build(&t, &table).unwrap();
    assert!(
        l.rows.iter().all(|r| !r.register.is_empty()),
        "register fields only"
    );
    let limits = ledger::twin_limits(&t).unwrap();
    let waits: Vec<_> = limits.iter().filter(|r| r.family == "WAIT").collect();
    assert!(waits.len() >= 2, "{waits:?}");
    let largest = waits.iter().filter_map(|w| w.value).max().unwrap();
    for w in &waits {
        assert_eq!(w.class, Class::SiliconOnly, "{}", w.id);
        assert!(w.judge.contains("settle covers"), "{}", w.judge);
        assert!(w.register.is_empty());
        assert!(
            w.note
                .starts_with(&format!("the bring-up waits once, after every scripted device's configuration and before any data read, for the largest settle_us among them ({largest} µs)")),
            "{}",
            w.note
        );
        assert_eq!(w.acknowledged, None);
    }
    let mut spec: Specification = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json"))
            .unwrap(),
    )
    .unwrap();
    spec.acknowledge_unwitnessed = every_row_acknowledged(&l);
    assert_eq!(ledger::stale(&l, &spec), None);
    assert_eq!(ledger::acknowledge(&mut l, &spec), Vec::<String>::new());
    assert_eq!(l.counts.unacknowledged, 0);
    let models: Vec<&str> = limits
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
        limits
            .iter()
            .filter(|r| r.family == "MODEL")
            .all(|r| r.class == Class::IgnoredByModel && r.acknowledged.is_none())
    );
    assert_eq!(limits.len(), waits.len() + models.len());
    // The reference runs no script and has no own model: no twin limit.
    assert!(ledger::twin_limits(&reference().0).unwrap().is_empty());
}

/// `AET-WO-4230` R1: the reference acknowledges every unwitnessed row of its
/// ledger, one entry per register and class (27: the ruling's 26 and R7's
/// `RCC.CR` `D2CKRDY`), exact fields (164), each with its own reason; no
/// entry is stale, and each row carries its entry's reason. Judged rows
/// carry no acknowledgement.
#[test]
fn the_reference_acknowledges_every_unwitnessed_row_by_register_and_class() {
    let spec = reference_spec();
    let mut l = reference_ledger();
    assert_eq!(ledger::stale(&l, &spec), None);
    assert_eq!(ledger::acknowledge(&mut l, &spec), Vec::<String>::new());
    assert_eq!(l.counts.unacknowledged, 0);
    let entries = &spec.acknowledge_unwitnessed;
    assert_eq!(entries.len(), 27);
    let per_class = |c: &str| entries.iter().filter(|a| a.class == c).count();
    assert_eq!(
        (
            per_class("same-source"),
            per_class("ignored-by-model"),
            per_class("silicon-only")
        ),
        (15, 11, 1)
    );
    assert_eq!(entries.iter().map(|a| a.fields.len()).sum::<usize>(), 164);
    // The hand-written form is the ledger's, entry for entry.
    let want = every_row_acknowledged(&l);
    let mut have: Vec<_> = entries
        .iter()
        .map(|a| (a.register.clone(), a.class.clone(), a.fields.clone()))
        .collect();
    have.sort();
    let mut want: Vec<_> = want
        .into_iter()
        .map(|a| (a.register, a.class, a.fields))
        .collect();
    want.sort();
    assert_eq!(have, want);
    // Each reason is its own.
    let reasons: std::collections::BTreeSet<&str> =
        entries.iter().map(|a| a.reason.as_str()).collect();
    assert_eq!(reasons.len(), entries.len());
    for r in &l.rows {
        match r.class.unwitnessed() {
            Some(c) => {
                assert_eq!(r.acknowledged, Some(true), "{}", r.id);
                let entry = spec.unwitnessed_acknowledgement(&r.register, c).unwrap();
                assert_eq!(r.reason.as_deref(), Some(entry.reason.as_str()), "{}", r.id);
            }
            None => assert_eq!(
                (r.acknowledged, r.reason.as_deref()),
                (None, None),
                "{}",
                r.id
            ),
        }
    }
}

/// R1, fail closed both ways at the ledger: a field the generator newly
/// writes in an acknowledged register is not acknowledged, and nothing is
/// dropped; an entry naming a field the ledger lacks for that register and
/// class (or a register it lacks) is stale, named with every such field.
#[test]
fn a_new_field_is_unacknowledged_and_an_entry_beyond_the_ledger_is_stale() {
    let spec = reference_spec();
    let mut l = reference_ledger();
    let mut extra = l
        .rows
        .iter()
        .find(|r| r.id == "GPIOA.MODER.MODER0")
        .unwrap()
        .clone();
    extra.id = "GPIOA.MODER.MODER1".into();
    extra.field = "MODER1".into();
    l.rows.push(extra);
    assert_eq!(ledger::acknowledge(&mut l, &spec), ["GPIOA.MODER.MODER1"]);
    assert_eq!(l.counts.unacknowledged, 1);
    assert_eq!(l.rows.len(), 290);
    let new = l.rows.last().unwrap();
    assert_eq!(
        (new.acknowledged, new.reason.as_deref()),
        (Some(false), None)
    );

    let l = reference_ledger();
    let stale = |edit: &dyn Fn(&mut Specification)| {
        let mut s = reference_spec();
        edit(&mut s);
        ledger::stale(&l, &s)
    };
    let at = |s: &Specification, register: &str, class: &str| {
        s.acknowledge_unwitnessed
            .iter()
            .position(|a| a.register == register && a.class == class)
            .unwrap()
    };
    // A field the register does not have in the ledger.
    let found = stale(&|s| {
        let i = at(s, "GPIOA.MODER", "same-source");
        s.acknowledge_unwitnessed[i]
            .fields
            .extend(["MODER3".into(), "MODER4".into()]);
    })
    .unwrap();
    assert_eq!(found.register, "GPIOA.MODER");
    assert_eq!(found.class, "same-source");
    assert_eq!(found.fields, ["MODER3", "MODER4"]);
    assert_eq!(
        found.path(),
        format!("acknowledge_unwitnessed[{}].fields", found.index)
    );
    assert!(
        found
            .message()
            .contains("`MODER3`, `MODER4` of register `GPIOA.MODER`")
    );
    // A field the ledger has, under another class.
    let found = stale(&|s| {
        let i = at(s, "I2C1.CR2", "same-source");
        s.acknowledge_unwitnessed[i].fields.push("SADD".into());
    })
    .unwrap();
    assert_eq!(found.fields, ["SADD"]);
    // A register the ledger lacks: every field.
    let found = stale(&|s| {
        s.acknowledge_unwitnessed.push(UnwitnessedAcknowledgement {
            register: "SPI2.CFG1".into(),
            class: "ignored-by-model".into(),
            fields: vec!["MBR".into(), "FTHLV".into()],
            reason: "r".into(),
        });
    })
    .unwrap();
    assert_eq!(
        (found.register.as_str(), found.fields.len()),
        ("SPI2.CFG1", 2)
    );
    assert_eq!(found.index, 27);
}

/// R5 (the architect's ruling on Q-G): each motor output's `PUPDR` field is
/// independent-rule, pulled down (`10`) by `silicon::motor_output_pupd`;
/// every other `PUPDR` field stays same-source. A generator leaving another
/// value there is refused by name (`LedgerRuleDisagrees`).
#[test]
fn a_motor_output_s_pull_down_is_the_independent_rule_s() {
    for name in ["motors-pwm", "motors-dshot600"] {
        let (t, table) = fixture(name);
        let l = ledger::build(&t, &table).unwrap();
        let motor: std::collections::BTreeSet<String> = t
            .motor_outputs
            .keys()
            .map(|net| {
                let pin = &t.assignments[net].pin;
                format!("GPIO{}.PUPDR.PUPD{}", &pin[1..2], &pin[2..])
            })
            .collect();
        assert_eq!(motor.len(), 4, "{name}");
        for r in l.rows.iter().filter(|r| r.register.ends_with(".PUPDR")) {
            if motor.contains(&r.id) {
                assert_eq!(r.class, Class::IndependentRule, "{name} {}", r.id);
                assert_eq!(r.value, Some(0b10), "{name} {}", r.id);
                assert!(r.judge.contains("motor_output_pupd"), "{}", r.judge);
            } else {
                assert_eq!(r.class, Class::SameSource, "{name} {}", r.id);
            }
        }
        // The generator's value is the rule's, or the ledger refuses.
        let mut fields = register_fields(&t, &table).unwrap();
        let first = motor.iter().next().unwrap().clone();
        let f = fields.iter_mut().find(|f| f.id() == first).unwrap();
        f.value = Some(0b01);
        let mut rows = ledger::register_rows(&fields, &ledger::TABLE).unwrap();
        let err = ledger::motor_pupd(&mut rows, &t).unwrap_err();
        assert_eq!(err.kind(), "LedgerRuleDisagrees");
        assert_eq!(err.id(), Some(first.as_str()));
    }
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
    // `AET-WO-4230` R7: `D2CKRDY` is a bench row (the architect's ruling on
    // Q-F).
    assert!(rows.contains("| RCC.CR (`D2CKRDY`) | `0x58024400` | `0x00008000` | `0x00008000` |"));
    assert_eq!(on.len(), 145);
    assert_eq!(
        sha(&rows),
        "af6187acf7a3a1cb3a446fbc81d7c30c62d1fac63cf4a71853f7df41922e083f"
    );
}

/// The verdict carries the ledger and the silicon findings: `verify
/// --against` an empty directory differs before any gate runs, and prints
/// the verdict with both, its fab keys never after them. The largest
/// fixture's line stays far under the MCP path's 1 MiB stdout cap.
///
/// `AET-WO-4230` R1: every row stays, each unwitnessed one `acknowledged`
/// with its reason. R2: every declared peer deviation is named with its
/// figure, a cited one never. R3: `check_brr` for every USART. Q-I: the twin
/// limits beside the ledger.
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
    assert_eq!(v["ledger"]["total"], 289);
    assert_eq!(v["ledger"]["rows"].as_array().unwrap().len(), 289);
    assert_eq!(v["ledger"]["counts"]["by_class"]["independent-rule"], 67);
    assert_eq!(v["ledger"]["evidence"].as_object().unwrap().len(), 5);
    assert_eq!(v["silicon"]["holds"], true);
    assert_eq!(v["silicon"]["spi"][0]["peripheral"], "SPI1");
    assert_eq!(v["silicon"]["i2c"][0]["peripheral"], "I2C1");
    assert_eq!(
        v["silicon"]["usart"],
        serde_json::json!([{"port": "CONSOLE", "peripheral": "USART1", "kernel_hz": 64000000,
            "baud": 115200, "brr": 556, "residue": 51200, "nearest": true, "in_range": true,
            "holds": true}])
    );
    let rows = v["ledger"]["rows"].as_array().unwrap();
    let acknowledged: Vec<_> = rows.iter().filter(|r| r["acknowledged"] == true).collect();
    assert_eq!(acknowledged.len(), 164);
    assert!(
        acknowledged
            .iter()
            .all(|r| r["reason"].as_str().is_some_and(|x| !x.is_empty()))
    );
    assert!(rows.iter().all(|r| r["acknowledged"] != false));
    assert_eq!(
        rows.iter()
            .filter(|r| r.get("acknowledged").is_none())
            .count(),
        67 + 58,
        "judged rows carry no acknowledgement"
    );
    assert_eq!(v["ledger"]["counts"]["unacknowledged"], 0);
    assert_eq!(
        v["declared_peers"],
        serde_json::json!([{"port": "CONSOLE", "peer_deviation_ppm": 5000}])
    );
    assert_eq!(v["twin_limits"], serde_json::json!([]));
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
    assert!(at("findings") < at("declared_peers") && at("declared_peers") < at("silicon"));
    assert!(at("ledger") < at("twin_limits"));
    assert!(!text.contains("\"against_fab\":") && !text.contains("\"fab\":"));
    let large = run(&root().join("crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json"));
    assert!(large.len() < 512 * 1024, "{} bytes", large.len());
    // Three ports declared, the CRSF port's cited spread never named; with no
    // acknowledgement every unwitnessed row is marked, none dropped.
    let four = run(&root().join("crates/aethelgard-cli/tests/fixtures/serial-four.json"));
    let w: serde_json::Value = serde_json::from_slice(&four).unwrap();
    assert_eq!(
        w["declared_peers"],
        serde_json::json!([
            {"port": "MAVLINK", "peer_deviation_ppm": 5000},
            {"port": "GPS", "peer_deviation_ppm": 5000},
            {"port": "SBUS", "peer_deviation_ppm": 5000},
        ])
    );
    assert_eq!(w["silicon"]["usart"].as_array().unwrap().len(), 4);
    let rows = w["ledger"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), w["ledger"]["total"].as_u64().unwrap() as usize);
    let unmarked = rows.iter().filter(|r| r["acknowledged"] == false).count();
    assert!(unmarked > 0);
    assert_eq!(w["ledger"]["counts"]["unacknowledged"], unmarked);
    assert!(rows.iter().all(|r| r["acknowledged"] != true));
    fs::remove_dir_all(&empty).unwrap();
}

/// `AET-WO-4230` R3: the verdict's silicon findings hold `check_brr` for
/// every serial port's USART, and a BRR one off (inside the tolerance the
/// solver's rule allows) fails `holds` and is named.
#[test]
fn silicon_judges_every_usart_s_brr_by_the_independent_inequality() {
    let (mut t, _) = fixture("serial-four");
    let s = ledger::silicon(&t);
    assert!(s.holds);
    let ports: Vec<&str> = s.usart.iter().map(|f| f.port.as_str()).collect();
    assert_eq!(ports, ["CRSF", "GPS", "MAVLINK", "SBUS"]);
    assert!(ledger::silicon_failures(&s).is_empty());
    t.serial_ports.get_mut("GPS").unwrap().brr += 1;
    let s = ledger::silicon(&t);
    assert!(!s.holds);
    let failing: Vec<&str> = s
        .usart
        .iter()
        .filter(|f| !f.brr.holds)
        .map(|f| f.port.as_str())
        .collect();
    assert_eq!(failing, ["GPS"]);
    let named = ledger::silicon_failures(&s);
    assert_eq!(named.len(), 1);
    assert!(
        named[0].contains("(GPS): aethelgard_ir::silicon::check_brr does not hold"),
        "{}",
        named[0]
    );
}
