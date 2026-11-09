//! `AET-WO-3100` §1.4 and §3: the seeded set, `scenarios/sentinel-01/seeded/cases.json`.
//!
//! Tier 1 (always): the manifest seeds at least twenty failing cases, at least
//! eight in each domain, and the three trap cases; every mutant is a patch that
//! applies exactly once to the generated board as committed, so a generator
//! change fails here loudly and never silently un-seeds a case; every
//! electrical case is a scenario whose own expectation is its manifest's.
//!
//! Tier 2 (`with-tools`, the CI `tools` job): each mutant is built from a
//! scratch copy of `boards/sentinel-01`, every case runs through `campaign`
//! with the real bridge and Renode, and `attribute` on its run names the
//! manifest's domain and rule, with evidence of the kinds that rule allows.
//! Each run writes `ANSWER 3100` lines to `/dev/stderr`: every measured rise,
//! and the trace bytes `OQ-03` asks for.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

const MANIFEST: &str = "scenarios/sentinel-01/seeded/cases.json";
const BOARD: &str = "boards/sentinel-01";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    #[allow(dead_code)]
    about: String,
    mutants: BTreeMap<String, Mutant>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutant {
    #[allow(dead_code)]
    why: String,
    file: String,
    find: String,
    replace: String,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    scenario: String,
    mutant: Option<String>,
    domain: String,
    rule: String,
    findings: Vec<String>,
}

fn manifest() -> Manifest {
    let m: Manifest = serde_json::from_slice(&fs::read(root().join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(m.schema_version, 1);
    m
}

/// `text` with `find` replaced, refused unless `find` occurs exactly once, in
/// code: a match inside a `//` comment would patch the comment and leave the
/// firmware as it was (adversarial A9).
fn patch(name: &str, text: &str, m: &Mutant) -> Result<String, String> {
    let in_comment = text
        .lines()
        .any(|l| l.contains(m.find.as_str()) && l.trim_start().starts_with("//"));
    match text.matches(m.find.as_str()).count() {
        1 if in_comment => Err(format!(
            "mutant `{name}`: `{}` occurs only in a comment in {}: this case is no longer seeded",
            m.find, m.file
        )),
        1 => Ok(text.replacen(m.find.as_str(), &m.replace, 1)),
        n => Err(format!(
            "mutant `{name}`: `{}` occurs {n} times in {}, not once: the generator changed, and this case is no longer seeded",
            m.find, m.file
        )),
    }
}

#[test]
fn the_manifest_seeds_twenty_failing_cases_eight_per_domain_and_the_three_traps() {
    let m = manifest();
    let count = |d: &str| m.cases.iter().filter(|c| c.domain == d).count();
    let (electrical, software) = (count("electrical"), count("software"));
    assert!(electrical >= 8 && software >= 8, "{electrical} {software}");
    assert!(electrical + software >= 20);
    assert_eq!(
        m.cases.len(),
        electrical + software + count("none"),
        "every case names a domain"
    );
    let mut names: Vec<&str> = m.cases.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), m.cases.len(), "case names are unique");
    let case = |scenario: &str, mutant: Option<&str>| {
        m.cases
            .iter()
            .find(|c| c.scenario.ends_with(scenario) && c.mutant.as_deref() == mutant)
            .unwrap_or_else(|| panic!("no case {scenario} {mutant:?}"))
    };
    fn trap(c: &Case) -> (&str, &str) {
        (c.domain.as_str(), c.rule.as_str())
    }
    assert_eq!(
        trap(case("seeded/pullup-3k3.json", Some("wrong-address"))),
        ("software", "S1")
    );
    assert_eq!(
        trap(case("/pullup-47k.json", Some("wrong-address"))),
        ("electrical", "E1")
    );
    assert_eq!(trap(case("seeded/pullup-3k3.json", None)), ("none", "N1"));
    for c in &m.cases {
        assert_eq!(c.findings.is_empty(), c.domain == "none", "{}", c.name);
        let ok = matches!(
            (c.domain.as_str(), c.rule.as_str()),
            ("electrical", "E1") | ("software", "S1" | "S2") | ("none", "N1")
        );
        assert!(ok, "{}: {} {}", c.name, c.domain, c.rule);
        if let Some(mutant) = &c.mutant {
            assert!(m.mutants.contains_key(mutant), "{}: {mutant}", c.name);
        }
    }
    // The software cases are the six the work order names, and more.
    for name in [
        "wrong-address",
        "wrong-register",
        "wrong-nbytes",
        "i2c-clock-skipped",
        "scl-af-wrong",
        "no-boot-word",
    ] {
        assert!(
            m.cases.iter().any(|c| c.mutant.as_deref() == Some(name)),
            "{name}"
        );
    }
}

#[test]
fn every_mutant_applies_exactly_once_to_the_generated_board_as_committed() {
    for (name, m) in &manifest().mutants {
        let text = fs::read_to_string(root().join(BOARD).join(&m.file)).unwrap();
        let patched = patch(name, &text, m).unwrap();
        assert_ne!(patched, text, "{name}");
    }
}

#[test]
fn a_patch_that_matches_zero_or_two_times_is_refused_naming_the_mutant() {
    let m = Mutant {
        why: String::new(),
        file: "f.rs".into(),
        find: "0x76".into(),
        replace: "0x77".into(),
    };
    assert_eq!(patch("a", "x 0x76 y", &m).unwrap(), "x 0x77 y");
    for text in ["nothing", "0x76 and 0x76"] {
        let e = patch("wrong-address", text, &m).unwrap_err();
        assert!(
            e.contains("mutant `wrong-address`") && e.contains("not once"),
            "{e}"
        );
    }
    // The adversary's decoy: the code moved to a constant, the text kept in a comment.
    let decoy = "    // i2c1.transfer(&mut r, 0x76,\n    i2c1.transfer(&mut r, ADDR, &[0xd0])\n";
    let e = patch("wrong-address", decoy, &m).unwrap_err();
    assert!(e.contains("only in a comment"), "{e}");
}

#[test]
fn every_unmutated_case_is_a_scenario_whose_own_expectation_is_the_manifests() {
    let m = manifest();
    for c in m.cases.iter().filter(|c| c.mutant.is_none()) {
        let s: serde_json::Value =
            serde_json::from_slice(&fs::read(root().join(&c.scenario)).unwrap()).unwrap();
        let mut expect: Vec<String> = serde_json::from_value(s["expect_findings"].clone()).unwrap();
        expect.sort();
        assert_eq!(expect, c.findings, "{}", c.name);
        let fails = if c.findings.is_empty() {
            "pass"
        } else {
            "fail"
        };
        assert_eq!(s["expect"], fails, "{}", c.name);
    }
    // Every seeded scenario is some case's.
    for entry in fs::read_dir(root().join("scenarios/sentinel-01/seeded")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name == "cases.json" {
            continue;
        }
        let path = format!("scenarios/sentinel-01/seeded/{name}");
        assert!(
            m.cases.iter().any(|c| c.scenario == path),
            "{path} is no case's"
        );
    }
}

#[cfg(feature = "with-tools")]
mod tier2 {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::OnceLock;

    use aethelgard_chaos::record::Record;
    use aethelgard_chaos::replay_dir;
    use serde_json::Value;

    use super::*;

    fn answer(line: &str) {
        let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
        log.write_all(format!("ANSWER 3100: {line}\n").as_bytes())
            .unwrap();
    }

    fn scratch() -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("aet-3100-seeded-{}", std::process::id()))
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let (src, dst) = (entry.path(), to.join(entry.file_name()));
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&src, &dst);
            } else {
                fs::copy(&src, &dst).unwrap();
            }
        }
    }

    /// The firmware of `mutant` (none: the generated firmware as committed),
    /// built offline from a scratch copy of the board.
    fn firmware(label: &str, mutant: Option<(&str, &Mutant)>) -> PathBuf {
        let dir = scratch().join(label);
        let _ = fs::remove_dir_all(&dir);
        let board = dir.join("board");
        copy_tree(&root().join(BOARD), &board);
        if let Some((name, m)) = mutant {
            let file = board.join(&m.file);
            let text = fs::read_to_string(&file).unwrap();
            fs::write(
                &file,
                patch(name, &text, m).unwrap_or_else(|e| panic!("{e}")),
            )
            .unwrap();
        }
        // The scratch copy is its own workspace root, wherever the target
        // directory is: CI's is inside the repository, under the root
        // workspace, which does not list it (FEEDBACK F17).
        let manifest = board.join("firmware/Cargo.toml");
        let mut toml = fs::read_to_string(&manifest).unwrap();
        toml.push_str("\n[workspace]\n");
        fs::write(&manifest, toml).unwrap();
        let target = dir.join("target");
        let out = Command::new(env!("CARGO"))
            .args(["build", "--release", "--offline", "--locked"])
            .current_dir(board.join("firmware"))
            .env("CARGO_TARGET_DIR", &target)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        target.join("thumbv7em-none-eabihf/release/sentinel-01-board-firmware")
    }

    fn aethelgard(args: &[&str]) -> (Option<i32>, Vec<u8>) {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(args)
            .current_dir(root())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.code(), out.stdout)
    }

    fn json_lines(bytes: &[u8]) -> Vec<Value> {
        std::str::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    struct Ran {
        case: Case,
        /// The campaign's line for the case's scenario.
        line: Value,
        run: PathBuf,
    }

    /// Every case, run: one campaign per firmware, over that firmware's cases.
    fn ran() -> &'static Vec<Ran> {
        static RAN: OnceLock<Vec<Ran>> = OnceLock::new();
        RAN.get_or_init(|| {
            let m = manifest();
            let mut by_firmware: BTreeMap<Option<String>, Vec<Case>> = BTreeMap::new();
            for c in &m.cases {
                by_firmware
                    .entry(c.mutant.clone())
                    .or_default()
                    .push(c.clone());
            }
            let mut out = Vec::new();
            let mut images: BTreeMap<Vec<u8>, String> = BTreeMap::new();
            for (mutant, cases) in by_firmware {
                let label = mutant.clone().unwrap_or_else(|| "unmutated".into());
                let elf = firmware(&label, mutant.as_deref().map(|n| (n, &m.mutants[n])));
                // Every mutant builds a firmware of its own: a patch that
                // changed nothing the compiler keeps is not a seeded case (A9).
                if let Some(other) = images.insert(fs::read(&elf).unwrap(), label.clone()) {
                    panic!("{label} builds the same firmware as {other}");
                }
                let archive = scratch().join(&label).join("archive");
                let mut args = vec!["campaign", "--scenarios"];
                args.extend(cases.iter().map(|c| c.scenario.as_str()));
                args.extend([
                    "--firmware",
                    elf.to_str().unwrap(),
                    "--archive",
                    archive.to_str().unwrap(),
                ]);
                let (code, stdout) = aethelgard(&args);
                let lines = json_lines(&stdout);
                let summary = lines.last().unwrap();
                assert_eq!(summary["check"], "campaign", "{label}: {lines:?}");
                // A firmware whose every case is unmutated, or passes, matches all.
                let all = cases
                    .iter()
                    .all(|c| c.mutant.is_none() || c.findings.is_empty());
                if all {
                    assert_eq!(code, Some(0), "{label}: {lines:?}");
                }
                for c in cases {
                    let scenario: Value =
                        serde_json::from_slice(&fs::read(root().join(&c.scenario)).unwrap())
                            .unwrap();
                    let line = lines
                        .iter()
                        .find(|l| l["scenario"] == scenario["name"])
                        .unwrap_or_else(|| panic!("{}: no line in {lines:?}", c.name))
                        .clone();
                    answer(&format!(
                        "case {} trace_bytes={} event_count={}",
                        c.name, line["trace_bytes"], line["event_count"]
                    ));
                    let run = PathBuf::from(line["run"].as_str().unwrap());
                    out.push(Ran { case: c, line, run });
                }
            }
            out
        })
    }

    fn attribute(run: &Path) -> (Option<i32>, Vec<u8>) {
        aethelgard(&["attribute", "--archive", run.to_str().unwrap()])
    }

    #[test]
    fn every_case_fails_the_way_its_manifest_says() {
        for r in ran() {
            let findings: Vec<String> = serde_json::from_value(r.line["findings"].clone()).unwrap();
            assert_eq!(findings, r.case.findings, "{}", r.case.name);
            // A scenario run against the unmutated firmware matches its own `expect`.
            if r.case.mutant.is_none() {
                assert_eq!(r.line["matched"], true, "{}: {}", r.case.name, r.line);
            }
            // Every measured rise, recorded (FEEDBACK, §1.4).
            for record in replay_dir(&r.run).unwrap().state.records {
                if let Record::RiseMeasured {
                    net,
                    rise_30_70_ps,
                    complete,
                    ceiling_ps,
                    exceeds,
                    ..
                } = record
                {
                    answer(&format!(
                        "rise {} {net} rise_30_70_ps={rise_30_70_ps} complete={complete} ceiling_ps={ceiling_ps} exceeds={exceeds}",
                        r.case.name
                    ));
                }
            }
        }
    }

    #[test]
    fn p3_gate_clause_1_attribute_names_every_case_with_evidence_its_rule_allows() {
        for r in ran() {
            let (code, stdout) = attribute(&r.run);
            let a = &json_lines(&stdout)[0];
            assert_eq!(code, Some(0), "{}: {a}", r.case.name);
            assert_eq!(
                (a["domain"].as_str().unwrap(), a["rule"].as_str().unwrap()),
                (r.case.domain.as_str(), r.case.rule.as_str()),
                "{}: {a}",
                r.case.name
            );
            let records = replay_dir(&r.run).unwrap().state.records;
            let allowed: &[u64] = match r.case.rule.as_str() {
                "E1" => &[7, 8, 9],
                "S1" => &[7, 11, 13],
                "S2" => &[7, 10, 11],
                _ => &[],
            };
            let evidence = a["evidence"].as_array().unwrap();
            assert_eq!(
                evidence.is_empty(),
                r.case.domain == "none",
                "{}: {a}",
                r.case.name
            );
            for e in evidence {
                let (index, kind) = (e["index"].as_u64().unwrap(), e["kind"].as_u64().unwrap());
                assert!((index as usize) < records.len(), "{}: {a}", r.case.name);
                assert_eq!(records[index as usize].kind(), kind, "{}: {a}", r.case.name);
                assert!(allowed.contains(&kind), "{}: {a}", r.case.name);
            }
            answer(&format!(
                "attribute {} domain={} rule={} evidence={} symptoms={} masked={}",
                r.case.name, a["domain"], a["rule"], a["evidence"], a["symptoms"], a["masked"]
            ));
        }
    }

    fn case(name: &str) -> &'static Ran {
        ran().iter().find(|r| r.case.name == name).unwrap()
    }

    fn masked(name: &str) -> Vec<u64> {
        let a = &json_lines(&attribute(&case(name).run).1)[0];
        a["masked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["kind"].as_u64().unwrap())
            .collect()
    }

    #[test]
    fn the_trap_the_wrong_address_shares_i2c_timeout_with_47k_and_is_told_apart() {
        // Both fail with i2c_timeout at TXIS; 3.3 kOhm is measured inside the ceiling.
        let soft = case("trap-wrong-address-3k3");
        assert_eq!(soft.line["findings"], serde_json::json!(["i2c_timeout"]));
        let a = &json_lines(&attribute(&soft.run).1)[0];
        assert_eq!(
            (&a["domain"], &a["rule"]),
            (&"software".into(), &"S1".into()),
            "{a}"
        );
        // The 47 kOhm run with the same mutant is electrical, its witness masked.
        assert_eq!(masked("trap-wrong-address-47k"), [13]);
        // The unmutated 47 kOhm run masks nothing: its witness agrees.
        assert!(masked("pullup-47k").is_empty());
        let none = &json_lines(&attribute(&case("trap-unmutated-3k3").run).1)[0];
        assert_eq!(
            (&none["domain"], &none["rule"]),
            (&"none".into(), &"N1".into())
        );
    }

    #[test]
    fn attribute_twice_is_identical_and_a_one_byte_change_is_refused_as_replay_refuses_it() {
        for r in ran() {
            assert_eq!(attribute(&r.run), attribute(&r.run), "{}", r.case.name);
        }
        let run = &case("trap-wrong-address-3k3").run;
        let copy = scratch().join("flipped").join(run.file_name().unwrap());
        let _ = fs::remove_dir_all(&copy);
        fs::create_dir_all(&copy).unwrap();
        for name in ["trace.cbor", "manifest.cbor"] {
            fs::copy(run.join(name), copy.join(name)).unwrap();
        }
        let mut trace = fs::read(copy.join("trace.cbor")).unwrap();
        let at = trace.len() / 2;
        trace[at] ^= 1;
        fs::write(copy.join("trace.cbor"), &trace).unwrap();
        let replayed = aethelgard(&["replay", "--archive", copy.to_str().unwrap()]);
        let attributed = attribute(&copy);
        assert_eq!(attributed, replayed);
        assert_eq!(attributed.0, Some(3));
        assert_eq!(
            json_lines(&attributed.1)[0]["error"]["kind"],
            "HashMismatch"
        );
    }

    #[test]
    fn oq_03_the_reference_runs_bytes_with_kind_15() {
        for name in ["unmutated-2k2", "pullup-47k"] {
            let r = case(name);
            answer(&format!(
                "OQ-03 {name} trace_bytes={} event_count={}",
                r.line["trace_bytes"], r.line["event_count"]
            ));
        }
    }
}
