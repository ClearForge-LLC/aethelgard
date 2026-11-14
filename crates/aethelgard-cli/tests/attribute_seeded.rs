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
//! and each case's trace bytes and event count, which `OQ-03` asks for.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

mod common;

use common::verify::root;

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
    assert_eq!(m.schema_version, 1, "the manifest is schema version 1");
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
    assert!(
        electrical >= 8 && software >= 8,
        "at least eight electrical and eight software cases: {electrical} {software}"
    );
    assert!(
        electrical + software >= 20,
        "at least twenty failing cases: {electrical} {software}"
    );
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
        ("software", "S1"),
        "the 3k3 wrong-address trap is a software S1 case"
    );
    assert_eq!(
        trap(case("/pullup-47k.json", Some("wrong-address"))),
        ("electrical", "E1"),
        "the 47k wrong-address trap is an electrical E1 case"
    );
    assert_eq!(
        trap(case("seeded/pullup-3k3.json", None)),
        ("none", "N1"),
        "the unmutated 3k3 case is the none N1 trap"
    );
    for c in &m.cases {
        assert_eq!(
            c.findings.is_empty(),
            c.domain == "none",
            "a case has no findings exactly when its domain is none: {}",
            c.name
        );
        let ok = matches!(
            (c.domain.as_str(), c.rule.as_str()),
            ("electrical", "E1" | "E2") | ("software", "S1" | "S2" | "S3") | ("none", "N1")
        );
        assert!(
            ok,
            "each case carries a known domain and rule pair: {}: {} {}",
            c.name, c.domain, c.rule
        );
        if let Some(mutant) = &c.mutant {
            assert!(
                m.mutants.contains_key(mutant),
                "each case names a mutant the manifest defines: {}: {mutant}",
                c.name
            );
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
            "a case uses each of the six named software mutants: {name}"
        );
    }
}

#[test]
fn every_mutant_applies_exactly_once_to_the_generated_board_as_committed() {
    for (name, m) in &manifest().mutants {
        let text = fs::read_to_string(root().join(BOARD).join(&m.file)).unwrap();
        let patched = patch(name, &text, m).unwrap();
        assert_ne!(patched, text, "the mutant changes the board source: {name}");
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
    assert_eq!(
        patch("a", "x 0x76 y", &m).unwrap(),
        "x 0x77 y",
        "a patch matching once is applied"
    );
    for text in ["nothing", "0x76 and 0x76"] {
        let e = patch("wrong-address", text, &m).unwrap_err();
        assert!(
            e.contains("mutant `wrong-address`") && e.contains("not once"),
            "a patch matching zero or two times is refused naming the mutant: {e}"
        );
    }
    // The adversary's decoy: the code moved to a constant, the text kept in a comment.
    let decoy = "    // i2c1.transfer(&mut r, 0x76,\n    i2c1.transfer(&mut r, ADDR, &[0xd0])\n";
    let e = patch("wrong-address", decoy, &m).unwrap_err();
    assert!(
        e.contains("only in a comment"),
        "a match only inside a comment is refused as such: {e}"
    );
}

#[test]
fn every_unmutated_case_is_a_scenario_whose_own_expectation_is_the_manifests() {
    let m = manifest();
    for c in m.cases.iter().filter(|c| c.mutant.is_none()) {
        let s = read_json(&root().join(&c.scenario));
        let mut expect: Vec<String> = serde_json::from_value(s["expect_findings"].clone()).unwrap();
        expect.sort();
        assert_eq!(
            expect, c.findings,
            "an unmutated scenario expects the findings the manifest lists: {}",
            c.name
        );
        let fails = if c.findings.is_empty() {
            "pass"
        } else {
            "fail"
        };
        assert_eq!(
            s["expect"], fails,
            "an unmutated scenario expects fail exactly when the manifest lists findings: {}",
            c.name
        );
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

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn sha_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Every scenario file under `scenarios/`, relative to the root.
fn scenario_files() -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root().join("scenarios")];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let path = e.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "json") {
                let rel = path.strip_prefix(root()).unwrap().to_str().unwrap();
                out.push(rel.to_owned());
            }
        }
    }
    out.sort();
    out
}

/// The one scenario file whose bytes hash to `sha`.
fn scenario_with_sha(sha: &str) -> String {
    let found: Vec<String> = scenario_files()
        .into_iter()
        .filter(|f| sha_hex(&fs::read(root().join(f)).unwrap()) == sha)
        .collect();
    assert_eq!(found.len(), 1, "scenario sha256 {sha}: {found:?}");
    found.into_iter().next().unwrap()
}

/// `AET-WO-3200` §1.4, the half that needs no tool, so it runs in `gates`
/// too: every committed proposal is a proposal of pull-up changes only, the
/// proposed specification is the file `proposal.json` names and validates,
/// its scenario is in the tree, and its directory is named
/// `<project>-<scenario name>`. The tier-2 test re-runs it to its bytes.
#[test]
fn every_committed_proposal_is_pull_up_changes_named_for_its_scenario() {
    // Vacuous while `proposals/` is absent (`AET-WO-1909` F2): say so, once.
    let Ok(entries) = fs::read_dir(root().join("proposals")) else {
        use std::io::Write;
        let mut log = fs::OpenOptions::new()
            .append(true)
            .open("/dev/stderr")
            .unwrap();
        log.write_all(b"ANSWER 3200: proposals/ absent: nothing to check\n")
            .unwrap();
        return;
    };
    for entry in entries {
        let dir = entry.unwrap().path();
        let mut names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["proposal.json", "proposed-spec.json"],
            "a committed proposal directory holds exactly the proposal and its proposed spec: {}",
            dir.display()
        );
        let p = read_json(&dir.join("proposal.json"));
        assert_eq!(
            (&p["schema_version"], &p["outcome"], &p["proposer"]),
            (&1.into(), &"proposal".into(), &"rules".into()),
            "a proposal is schema 1, outcome proposal, proposed by rules: {}",
            dir.display()
        );
        let changes = p["changes"].as_array().unwrap();
        assert!(!changes.is_empty(), "a proposal makes at least one change");
        for c in changes {
            let mut keys: Vec<&String> = c.as_object().unwrap().keys().collect();
            keys.sort();
            assert_eq!(
                keys,
                ["bus", "pull_up_ohms"],
                "each change is a pull-up change naming a bus and an ohms value: {}",
                dir.display()
            );
        }
        let spec = fs::read(dir.join("proposed-spec.json")).unwrap();
        assert_eq!(
            p["proposed_specification_sha256"],
            sha_hex(&spec).as_str(),
            "the proposal records the sha256 of its proposed spec"
        );
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(["validate", "--file"])
            .arg(dir.join("proposed-spec.json"))
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "the proposed spec validates: {}",
            dir.display()
        );
        let scenario = scenario_with_sha(p["scenario"]["sha256"].as_str().unwrap());
        let named = read_json(&root().join(&scenario));
        assert_eq!(
            named["name"], p["scenario"]["name"],
            "the proposal names the scenario its sha256 selects"
        );
        let project: serde_json::Value = serde_json::from_slice(&spec).unwrap();
        let want = format!(
            "{}-{}",
            project["project_name"].as_str().unwrap(),
            named["name"].as_str().unwrap()
        );
        assert_eq!(
            dir.file_name().unwrap().to_str().unwrap(),
            want,
            "a proposal directory is named for its project and scenario"
        );
    }
}

#[cfg(feature = "with-tools")]
mod tier2 {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::OnceLock;

    use aethelgard_chaos::record::Record;
    use aethelgard_chaos::replay_dir;
    use serde_json::Value;

    use super::*;
    use crate::common::verify::copy_tree;

    fn answer(line: &str) {
        let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
        log.write_all(format!("ANSWER 3100: {line}\n").as_bytes())
            .unwrap();
    }

    fn scratch() -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("aet-3100-seeded-{}", std::process::id()))
    }

    /// The firmware of `mutant` (none: the generated firmware as committed),
    /// built by `aethelgard build` from a scratch copy of the board. `build`
    /// strips symbols, so its ELF is the same wherever the copy is
    /// (`AET-WO-3301`, `AET-WO-3200` F11): the unmutated firmware's sha256,
    /// and with it every run id, reproduces at any path.
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
                patch(name, &text, m).unwrap_or_else(|e| {
                    panic!("the mutant patch applies to the board source: {e}")
                }),
            )
            .unwrap();
        }
        let elf = dir.join("firmware.elf");
        let (code, stdout) = aethelgard(&[
            "build",
            "--crate",
            board.to_str().unwrap(),
            "--out",
            elf.to_str().unwrap(),
        ]);
        assert_eq!(
            code,
            Some(0),
            "aethelgard build succeeds on the case firmware: {label}: {}",
            String::from_utf8_lossy(&stdout)
        );
        elf
    }

    fn aethelgard(args: &[&str]) -> (Option<i32>, Vec<u8>) {
        aethelgard_in(&root(), args)
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
                assert_eq!(
                    summary["check"], "campaign",
                    "the campaign output ends with its campaign summary line: {label}: {lines:?}"
                );
                // A firmware whose every case is unmutated, or passes, matches all.
                let all = cases
                    .iter()
                    .all(|c| c.mutant.is_none() || c.findings.is_empty());
                if all {
                    assert_eq!(
                        code,
                        Some(0),
                        "a campaign whose cases all match exits 0: {label}: {lines:?}"
                    );
                }
                for c in cases {
                    let scenario = read_json(&root().join(&c.scenario));
                    let line = lines
                        .iter()
                        .find(|l| l["scenario"] == scenario["name"])
                        .unwrap_or_else(|| {
                            panic!(
                                "the campaign reports a line for every case scenario: {}: no line in {lines:?}",
                                c.name
                            )
                        })
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
            assert_eq!(
                findings, r.case.findings,
                "each case reports exactly its manifest's findings: {}",
                r.case.name
            );
            // A scenario run against the unmutated firmware matches its own `expect`.
            if r.case.mutant.is_none() {
                assert_eq!(
                    r.line["matched"], true,
                    "an unmutated case matches its scenario's own expectation: {}: {}",
                    r.case.name, r.line
                );
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
            assert_eq!(
                code,
                Some(0),
                "attribute exits 0 on every seeded run: {}: {a}",
                r.case.name
            );
            assert_eq!(
                (a["domain"].as_str().unwrap(), a["rule"].as_str().unwrap()),
                (r.case.domain.as_str(), r.case.rule.as_str()),
                "attribute names the manifest's domain and rule: {}: {a}",
                r.case.name
            );
            let records = replay_dir(&r.run).unwrap().state.records;
            let allowed: &[u64] = match r.case.rule.as_str() {
                "E1" => &[7, 8, 9],
                "E2" => &[22],
                "S1" => &[7, 11, 13],
                "S2" => &[7, 10, 11],
                "S3" => &[21, 22],
                _ => &[],
            };
            let evidence = a["evidence"].as_array().unwrap();
            assert_eq!(
                evidence.is_empty(),
                r.case.domain == "none",
                "attribute cites evidence exactly when the case is a fault: {}: {a}",
                r.case.name
            );
            for e in evidence {
                let (index, kind) = (e["index"].as_u64().unwrap(), e["kind"].as_u64().unwrap());
                assert!(
                    (index as usize) < records.len(),
                    "each evidence index points at a record of the run: {}: {a}",
                    r.case.name
                );
                assert_eq!(
                    records[index as usize].kind(),
                    kind,
                    "each evidence entry's kind is the kind of the record it cites: {}: {a}",
                    r.case.name
                );
                assert!(
                    allowed.contains(&kind),
                    "each evidence kind is one its rule allows: {}: {a}",
                    r.case.name
                );
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
        assert_eq!(
            soft.line["findings"],
            serde_json::json!(["i2c_timeout"]),
            "the 3.3 kOhm wrong-address trap fails with i2c_timeout, as the 47 kOhm run does"
        );
        let a = &json_lines(&attribute(&soft.run).1)[0];
        assert_eq!(
            (&a["domain"], &a["rule"]),
            (&"software".into(), &"S1".into()),
            "the 3.3 kOhm wrong-address trap is attributed to software, rule S1: {a}"
        );
        // The 47 kOhm run with the same mutant is electrical, its witness masked.
        assert_eq!(
            masked("trap-wrong-address-47k"),
            [13],
            "the 47 kOhm wrong-address run is electrical with its witness (kind 13) masked"
        );
        // The unmutated 47 kOhm run masks nothing: its witness agrees.
        assert!(
            masked("pullup-47k").is_empty(),
            "the unmutated 47 kOhm run masks nothing: its witness agrees"
        );
        let none = &json_lines(&attribute(&case("trap-unmutated-3k3").run).1)[0];
        assert_eq!(
            (&none["domain"], &none["rule"]),
            (&"none".into(), &"N1".into()),
            "the unmutated 3.3 kOhm run is attributed to none, rule N1: {none}"
        );
    }

    #[test]
    fn attribute_twice_is_identical_and_a_one_byte_change_is_refused_as_replay_refuses_it() {
        for r in ran() {
            assert_eq!(
                attribute(&r.run),
                attribute(&r.run),
                "attribute twice on the same run gives identical output: {}",
                r.case.name
            );
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
        assert_eq!(
            attributed, replayed,
            "attribute refuses a one-byte-changed trace exactly as replay does"
        );
        assert_eq!(
            attributed.0,
            Some(3),
            "a one-byte-changed trace is refused with exit 3"
        );
        assert_eq!(
            json_lines(&attributed.1)[0]["error"]["kind"],
            "HashMismatch",
            "a one-byte-changed trace is refused as a HashMismatch"
        );
    }

    // ---- `AET-WO-3200`: `propose` on every seeded run, and the re-gate ----

    /// An `ANSWER 3200` line on `/dev/stderr`, for FEEDBACK.
    fn note(line: &str) {
        let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
        log.write_all(format!("ANSWER 3200: {line}\n").as_bytes())
            .unwrap();
    }

    /// What `propose` must answer for each case, stated here and never read
    /// from the tool (`INV-07`).
    #[derive(Debug)]
    enum Want {
        /// A proposal of exactly this pull-up on `SENSORS`, which re-gates.
        Proposal(u64),
        /// A generator report naming the generator the mutant's file is in.
        Report,
        /// No rule: a held line is a board fault no value fixes.
        ModelSlot,
        /// A rule matched, but no E12 value lies at or above the floor.
        BelowFloor,
        /// A passing run: attribution precedes mutation (`INV-05`).
        Inv05,
    }

    fn want(c: &Case) -> Want {
        match (c.domain.as_str(), c.name.as_str()) {
            ("software", _) => Want::Report,
            ("none", _) => Want::Inv05,
            _ if c.findings.iter().any(|f| f == "i2c_line_held_low") => Want::ModelSlot,
            // A device absent behind its chip select: a board fault no value
            // fixes (`AET-WO-4200`, `E2`).
            (_, "spi-absent-u4") => Want::ModelSlot,
            // R_target = R × 240,000 / rise ≈ 2,832 Ω for every pull-up case,
            // the 47 kOhm trap included (E1 masks its wrong address).
            (
                _,
                "pullup-3k9"
                | "pullup-4k7"
                | "pullup-10k"
                | "pullup-47k"
                | "pullup-100k"
                | "trap-wrong-address-47k",
            ) => Want::Proposal(2700),
            // 2,200 × 240,000 / 410,089 ≈ 1,287 Ω.
            (_, "cap-220p-2k2") => Want::Proposal(1200),
            // 390 pF on SDA at 2.2 kOhm (AET-WO-1203's replacement for the
            // 470 pF case): about 727 ns, so R_target ≈ 726 Ω and E12 gives
            // 680 Ω, under the 967 Ω floor.
            (_, "cap-390p-sda-2k2") => Want::BelowFloor,
            (_, n) => panic!("case `{n}` has no expected proposal: state it here"),
        }
    }

    fn aethelgard_in(cwd: &Path, args: &[&str]) -> (Option<i32>, Vec<u8>) {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.code(), out.stdout)
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        v.sort();
        v
    }

    /// `propose` on `r`'s run into a fresh `out`, from the repository root.
    fn propose(r: &Ran, out: &Path) -> (Option<i32>, Value) {
        let _ = fs::remove_dir_all(out);
        let (code, stdout) = aethelgard(&[
            "propose",
            "--archive",
            r.run.to_str().unwrap(),
            "--scenario",
            &r.case.scenario,
            "--out",
            out.to_str().unwrap(),
        ]);
        (code, json_lines(&stdout).pop().unwrap())
    }

    /// Where each case's proposal is written, by case name: a fixed path, so
    /// the output a committed proposal is taken from can be found.
    fn proposals_out() -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("aet-3200-proposals")
    }

    #[test]
    fn p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says() {
        let m = manifest();
        for r in ran() {
            let name = &r.case.name;
            let out = proposals_out().join(name);
            let (code, line) = propose(r, &out);
            let p = read_json(&out.join("proposal.json"));
            note(&format!(
                "propose {name} exit={code:?} outcome={} changes={} sizing={} reason={}",
                p["outcome"], p["changes"], p["sizing"], p["reason"]
            ));
            // Twice: the same bytes (§3.5).
            let again = scratch().join("again").join(name);
            assert_eq!(
                propose(r, &again).0,
                code,
                "propose twice exits the same: {name}"
            );
            assert_eq!(
                names(&again),
                names(&out),
                "propose twice writes the same files: {name}"
            );
            for f in names(&out) {
                assert_eq!(
                    fs::read(out.join(&f)).unwrap(),
                    fs::read(again.join(&f)).unwrap(),
                    "propose twice writes the same bytes: {name}: {f}"
                );
            }
            match want(&r.case) {
                Want::Proposal(ohms) => {
                    assert_eq!(
                        code,
                        Some(0),
                        "a pull-up case is proposed with exit 0: {name}: {line}"
                    );
                    assert_eq!(
                        names(&out),
                        ["proposal.json", "proposed-spec.json"],
                        "a proposal writes proposal.json and proposed-spec.json only: {name}"
                    );
                    assert_eq!(
                        p["changes"],
                        serde_json::json!([{"bus": "SENSORS", "pull_up_ohms": ohms}]),
                        "the proposal changes exactly the SENSORS pull-up to the expected E12 value: {name}"
                    );
                    // The trap's wrong address is masked by the fault, and
                    // the proposal says so: its re-gate, on regenerated
                    // firmware, cannot speak to the firmware that failed.
                    let masked = p["attribution"]["masked"].as_array().unwrap();
                    assert_eq!(
                        masked
                            .iter()
                            .map(|e| e["kind"].as_u64().unwrap())
                            .collect::<Vec<_>>(),
                        if r.case.mutant.is_some() {
                            vec![13]
                        } else {
                            vec![]
                        },
                        "the proposal's masked witnesses are the wrong-address trap's kind 13 and nothing else: {name}"
                    );
                    regate(&out, &r.run);
                }
                Want::Report => {
                    assert_eq!(
                        code,
                        Some(6),
                        "a software case gets a generator report with exit 6: {name}: {line}"
                    );
                    assert_eq!(
                        names(&out),
                        ["generator-report.json", "proposal.json"],
                        "a generator report writes generator-report.json and proposal.json only: {name}"
                    );
                    // The generator the mutant's file belongs to: an oracle
                    // the manifest states, independent of the tool.
                    let file = &m.mutants[r.case.mutant.as_ref().unwrap()].file;
                    let generator = if file.starts_with("firmware/") {
                        "firmware"
                    } else {
                        assert_eq!(
                            file, "src/board.rs",
                            "a mutant file is under firmware/ or is src/board.rs: {name}"
                        );
                        "board_crate"
                    };
                    let g = read_json(&out.join("generator-report.json"));
                    assert_eq!(
                        g["generators"],
                        serde_json::json!([generator]),
                        "the report names the generator the mutant's file belongs to: {name}"
                    );
                    assert_eq!(
                        g["rule"],
                        r.case.rule.as_str(),
                        "the report names the case's rule: {name}"
                    );
                    assert_eq!(
                        p["changes"],
                        serde_json::json!([]),
                        "a software case proposes no changes: {name}"
                    );
                }
                Want::ModelSlot => {
                    assert_eq!(
                        code,
                        Some(1),
                        "a held-line case gets no proposal, exit 1: {name}: {line}"
                    );
                    assert_eq!(
                        names(&out),
                        ["proposal.json"],
                        "a held-line case writes proposal.json only: {name}"
                    );
                    assert_eq!(
                        p["reason"], "no rule; RUL-18 model proposer slot",
                        "a held-line case is left to the model proposer slot: {name}"
                    );
                }
                Want::BelowFloor => {
                    assert_eq!(
                        code,
                        Some(1),
                        "a case whose E12 value is below the floor gets no proposal, exit 1: {name}: {line}"
                    );
                    assert_eq!(
                        names(&out),
                        ["proposal.json"],
                        "a below-floor case writes proposal.json only: {name}"
                    );
                    let s = &p["sizing"][0];
                    assert!(
                        s["e12_ohms"].as_u64().unwrap() < s["rp_min_ohms"].as_u64().unwrap(),
                        "the sized E12 value lies below the minimum pull-up: {name}: {p}"
                    );
                    assert!(
                        p["reason"].as_str().unwrap().contains("floor"),
                        "the below-floor reason names the floor: {name}"
                    );
                }
                Want::Inv05 => {
                    assert_eq!(
                        code,
                        Some(1),
                        "a passing run is refused a proposal, exit 1: {name}: {line}"
                    );
                    assert_eq!(
                        names(&out),
                        ["proposal.json"],
                        "a refused passing run writes proposal.json only: {name}"
                    );
                    let reason = p["reason"].as_str().unwrap();
                    assert!(
                        reason.contains("N1") && reason.contains("INV-05"),
                        "the refusal cites N1 and INV-05: {name}"
                    );
                }
            }
        }
    }

    /// A committed proposal (`proposals/*/`) re-runs from the scenario its
    /// `proposal.json` names to its own bytes, and re-gates, so it cannot
    /// drift from the tool or slip past the gates (`AET-WO-3200` §1.4).
    #[test]
    fn every_committed_proposal_reruns_to_its_own_bytes_and_regates() {
        let Ok(entries) = fs::read_dir(root().join("proposals")) else {
            // Vacuous while `proposals/` is absent (`AET-WO-1909` F2).
            note("proposals/ absent: nothing to check");
            return;
        };
        for entry in entries {
            let committed = entry.unwrap().path();
            let p = read_json(&committed.join("proposal.json"));
            let scenario = scenario_with_sha(p["scenario"]["sha256"].as_str().unwrap());
            let r = ran()
                .iter()
                .find(|r| r.case.mutant.is_none() && r.case.scenario == scenario)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: no unmutated seeded case runs {scenario}",
                        committed.display()
                    )
                });
            let out = scratch()
                .join("committed")
                .join(committed.file_name().unwrap());
            let (code, line) = propose(r, &out);
            assert_eq!(
                code,
                Some(0),
                "propose on a committed proposal's scenario exits 0: {line}"
            );
            assert_eq!(
                names(&out),
                names(&committed),
                "a rerun writes the same files as the committed proposal: {}",
                committed.display()
            );
            for f in names(&committed) {
                assert_eq!(
                    fs::read(out.join(&f)).unwrap(),
                    fs::read(committed.join(&f)).unwrap(),
                    "{}: {f} is not what propose writes",
                    committed.display()
                );
            }
            regate(&committed, &r.run);
            note(&format!(
                "committed proposal {} reruns byte-identical and re-gates",
                committed.display()
            ));
        }
    }

    const HARNESS: &str = "proofs::every_valid_sequence";

    /// The re-gate runs, by the proposed specification's sha256: each distinct
    /// specification is judged once.
    static REGATED: std::sync::Mutex<BTreeMap<String, PathBuf>> =
        std::sync::Mutex::new(BTreeMap::new());

    /// `AET-WO-3200` §1.4: the proposal in `dir` through every gate a
    /// specification passes, then the checks that it passed for the right
    /// reason (§5.5) against the run that failed.
    fn regate(dir: &Path, failing: &Path) {
        let p = read_json(&dir.join("proposal.json"));
        let bytes = fs::read(dir.join("proposed-spec.json")).unwrap();
        let sha = sha_hex(&bytes);
        assert_eq!(
            p["proposed_specification_sha256"],
            sha.as_str(),
            "the proposal's recorded sha256 is that of its proposed-spec.json: {}",
            dir.display()
        );
        let run = {
            let mut done = REGATED.lock().unwrap_or_else(|e| e.into_inner());
            match done.get(&sha) {
                Some(run) => run.clone(),
                None => {
                    let run = every_gate(&sha, &bytes);
                    done.insert(sha.clone(), run.clone());
                    run
                }
            }
        };
        let passed = replay_dir(&run).unwrap().state.records;
        let failed = replay_dir(failing).unwrap().state.records;
        // The same solved buses, lines, ceilings and devices: the proposal
        // kept everything the gates judge, and changed only the pull-up.
        let solved = |r: &[Record]| -> Vec<Record> {
            r.iter()
                .filter(|x| matches!(x, Record::I2cBusSolved { .. }))
                .cloned()
                .collect()
        };
        assert_eq!(
            solved(&passed),
            solved(&failed),
            "the re-gate run solves the same buses, lines, ceilings and devices as the failing run: {}",
            dir.display()
        );
        // Every device the failing run read is read, and answers.
        let reads = |r: &[Record]| -> Vec<(String, Option<String>)> {
            r.iter()
                .filter_map(|x| match x {
                    Record::I2cCheck {
                        reference, status, ..
                    } => Some((reference.clone(), status.clone())),
                    _ => None,
                })
                .collect()
        };
        let (now, then) = (reads(&passed), reads(&failed));
        assert!(!now.is_empty(), "the re-gate run read no device");
        assert_eq!(
            now.iter().map(|x| &x.0).collect::<Vec<_>>(),
            then.iter().map(|x| &x.0).collect::<Vec<_>>(),
            "the re-gate run reads every device the failing run read"
        );
        assert!(
            now.iter().all(|x| x.1.as_deref() == Some("ok")),
            "every device the re-gate run reads answers ok: {now:?}"
        );
        // Each binding line's rise, computed from declared values, within 2 %
        // of its predicted rise.
        for s in p["sizing"].as_array().unwrap() {
            within_2_percent(dir, &sha, &passed, s);
        }
    }

    fn within_2_percent(dir: &Path, sha: &str, passed: &[Record], s: &Value) {
        let (net, predicted) = (
            s["binding_net"].as_str().unwrap(),
            s["predicted_rise_ps"].as_u64().unwrap(),
        );
        let rise = passed
            .iter()
            .find_map(|x| match x {
                Record::RiseMeasured {
                    net: n,
                    rise_30_70_ps,
                    complete: true,
                    exceeds: false,
                    ..
                } if n == net => Some(*rise_30_70_ps),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!("the re-gate run measures a complete rise within its ceiling on the binding net: {net}")
            });
        note(&format!(
            "regate {} spec={sha} {net} rise_30_70_ps={rise} predicted_rise_ps={predicted} off_by_ps={}",
            dir.display(),
            rise.abs_diff(predicted)
        ));
        assert!(
            rise.abs_diff(predicted) * 50 <= predicted,
            "the re-gate rise is within 2 % of the predicted rise: {net}: {rise} ps is more than 2 % from the predicted {predicted} ps"
        );
    }

    /// Prove one board crate at a time: each proof peaks at about 3.5 GB.
    static ONE_PROOF: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Every gate on one specification: `validate`, `solve`, `generate`, ERC, DRC
    /// with schematic parity on what `aethelgard generate` writes, the board
    /// crate proved with `--spec` (its harness `generated`), then a campaign
    /// of one `expect: pass` scenario on the firmware `aethelgard build`
    /// builds from it, whose run `attribute` names `N1`. Returns that run.
    fn every_gate(sha: &str, bytes: &[u8]) -> PathBuf {
        let dir = scratch().join(format!("regate-{}", &sha[..12]));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("proposed-spec.json"), bytes).unwrap();
        let ok = |args: &[&str]| -> Value {
            let (code, stdout) = aethelgard_in(&dir, args);
            let v = json_lines(&stdout).pop().unwrap();
            assert_eq!(
                code,
                Some(0),
                "each gate passes the proposed specification with exit 0: {args:?}: {v}"
            );
            note(&format!("regate {sha} {}: exit 0", args[..2].join(" ")));
            v
        };
        ok(&["validate", "--file", "proposed-spec.json"]);
        ok(&["solve", "--file", "proposed-spec.json"]);
        // Everything the specification produces, as `aethelgard generate`
        // writes it (`AET-WO-3301`; `AET-WO-3200` F12).
        ok(&["generate", "--spec", "proposed-spec.json", "--out", "gen"]);
        let spec: aethelgard_ir::Specification = serde_json::from_slice(bytes).unwrap();
        let sch = format!("gen/kicad/{}.kicad_sch", spec.project_name);
        let pcb = format!("gen/kicad/{}.kicad_pcb", spec.project_name);
        assert_eq!(
            ok(&["check", "erc", "--schematic", &sch])["verdict"],
            "pass",
            "ERC passes the proposed specification's schematic"
        );
        let drc = ok(&[
            "check",
            "drc",
            "--board",
            &pcb,
            "--spec",
            "proposed-spec.json",
            "--schematic",
            &sch,
        ]);
        assert_eq!(
            drc["verdict"], "pass",
            "DRC with schematic parity passes the proposed specification's board"
        );
        {
            let _one = ONE_PROOF.lock().unwrap_or_else(|e| e.into_inner());
            let v = ok(&[
                "prove",
                "--crate",
                "gen/board",
                "--harness",
                HARNESS,
                "--spec",
                "proposed-spec.json",
            ]);
            assert_eq!(
                v["verdict"], "pass",
                "the proposed specification's board crate is proved"
            );
            assert_eq!(
                v["harness_provenance"], "generated",
                "the proof ran the generated harness"
            );
        }
        // The firmware generated with the crate, built as the seeded set's is.
        let built = ok(&["build", "--crate", "gen/board", "--out", "firmware.elf"]);
        assert_eq!(
            built["verdict"], "pass",
            "the proposed specification's board crate builds"
        );
        let elf = dir.join("firmware.elf");

        let scenario = serde_json::json!({
            "schema_version": 1,
            "name": format!("regate-{}", &sha[..12]),
            "base": "proposed-spec.json",
            "expect": "pass",
            "expect_findings": [],
            "overrides": [],
        });
        fs::write(dir.join("regate.json"), scenario.to_string()).unwrap();
        let (code, stdout) = aethelgard_in(
            &dir,
            &[
                "campaign",
                "--scenarios",
                "regate.json",
                "--firmware",
                elf.to_str().unwrap(),
                "--archive",
                "archive",
            ],
        );
        let lines = json_lines(&stdout);
        assert_eq!(code, Some(0), "the re-gate campaign exits 0: {lines:?}");
        assert_eq!(
            lines[0]["matched"], true,
            "the re-gate scenario matches its expect: pass: {lines:?}"
        );
        let run = dir.join(lines[0]["run"].as_str().unwrap());
        let (code, stdout) = attribute(&run);
        let a = &json_lines(&stdout)[0];
        assert_eq!(code, Some(0), "attribute exits 0 on the re-gate run: {a}");
        assert_eq!(
            (&a["domain"], &a["rule"]),
            (&"none".into(), &"N1".into()),
            "attribute names the re-gate run none, rule N1: {a}"
        );
        note(&format!("regate {sha}: campaign matched, attribute N1"));
        run
    }
}
