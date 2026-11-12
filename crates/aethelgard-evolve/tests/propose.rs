//! `AET-WO-3200` §1.1 and §1.2, tier 1: the proposer interface and the rules
//! proposer, on campaign runs written through `aethelgard-chaos`'s own
//! writer and attributed by `attribute`, with no Renode and no bridge. The
//! healthy run is `attribute.rs`'s, the reference board as `campaign`
//! archives it. The floor is handed in as the integer the CLI takes from
//! `aethelgard_ir::electrical` (967 Ω at 3.3 V and 400 kHz).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::campaign::{self, run_id};
use aethelgard_chaos::record::{BusLine, Line, Override, Reading, Record};
use aethelgard_chaos::{Event, Header, Replayed, TraceWriter, replay_dir};
use aethelgard_evolve::propose::{
    self, Answer, BusFacts, Change, Generator, Input, MODEL_SLOT, Outcome, PAST_CEILING_CAUSE,
    Proposal, Proposer, Rules, Sizing, largest_e12_at_most,
};
use aethelgard_evolve::{
    Attribution, Domain, EventRef, Evidence, LineRises, Rise, Rule, SolvedBus, View, attribute,
    view,
};

const CR2_OK: u32 = 0x0201_04EC;
const FLOOR: u32 = 967;

fn reading(address: u32, mask: u32, expected: u32, observed: u32) -> Reading {
    Reading {
        address,
        mask,
        expected,
        observed: Some(observed),
    }
}

fn line(suffix: &str, net: &str, pin: &str) -> BusLine {
    BusLine {
        signal: format!("I2C1{suffix}"),
        net: net.into(),
        pin: pin.into(),
        alternate_function: 4,
        ceiling_ps: 300_000,
    }
}

fn rise(net: &str, ps: u64) -> Record {
    Record::RiseMeasured {
        bus: "SENSORS".into(),
        net: net.into(),
        rise_30_70_ps: ps,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: ps > 300_000,
    }
}

fn fault(nets: &[&str], from: &str) -> Record {
    Record::BusFault {
        bus: "SENSORS".into(),
        fault: "absent".into(),
        source: "aethelgard".into(),
        from: vec![from.into()],
        nets: nets.iter().map(|n| (*n).into()).collect(),
        firmware_sees: "absent".into(),
    }
}

fn check(status: &str, observed: u64) -> Record {
    Record::I2cCheck {
        reference: "U2".into(),
        address: 0x76,
        register: 0xD0,
        expected: 0x60,
        observed: Some(observed),
        status: Some(status.into()),
    }
}

fn witness(cr2: u32, txdr: u32) -> Record {
    Record::I2cWitness {
        peripheral: "I2C1".into(),
        reference: "U2".into(),
        cr2: reading(0x4000_5404, 0x02FF_07FF, CR2_OK, cr2),
        txdr: reading(0x4000_5428, 0xFF, 0xD0, txdr),
    }
}

fn verdict(findings: &[(&str, u64)]) -> Record {
    let mut findings: Vec<(String, String, u64)> = findings
        .iter()
        .map(|(k, n)| ((*k).into(), "error".into(), *n))
        .collect();
    findings.push(("device_unmodelled".into(), "info".into(), 1));
    findings.sort();
    Record::Verdict {
        pass: findings.iter().all(|f| f.1 == "info"),
        findings,
    }
}

fn register(name: &str, observed: Option<u32>) -> Record {
    let (address, mask, expected) = match name {
        "GPIOA.MODER" => (0x5802_0000, 0x0000_FC03, 0x0000_A801),
        "GPIOB.MODER" => (0x5802_0400, 0x0000_F000, 0x0000_A000),
        "GPIOB.AFRL" => (0x5802_0420, 0xFF00_0000, 0x4400_0000),
        "RCC.AHB4ENR" => (0x5802_44E0, u32::MAX, 0x3),
        "RCC.APB1LENR" => (0x5802_44E8, u32::MAX, 0x0020_0000),
        _ => panic!("{name}"),
    };
    Record::RegisterObserved {
        name: name.into(),
        reading: Reading {
            address,
            mask,
            expected,
            observed: Some(observed.unwrap_or(expected)),
        },
    }
}

fn boot(word: u32) -> Record {
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, word),
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(0x0800_0596),
    }
}

/// The healthy reference run at `ohms`: SENSORS on I2C1, both lines at
/// 186 ns, U2 read.
fn healthy(ohms: u32) -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "case".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![Override::PullUpOhms {
                bus: "SENSORS".into(),
                ohms,
            }],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        Record::I2cBusSolved {
            bus: "SENSORS".into(),
            peripheral: "I2C1".into(),
            lines: vec![
                line("_SCL", "I2C_SCL", "PB6"),
                line("_SDA", "I2C_SDA", "PB7"),
            ],
            devices: vec![("U2".into(), 0x76), ("U3".into(), 0x48)],
        },
        rise("I2C_SCL", 186_402),
        rise("I2C_SDA", 186_402),
        boot(0x2100_B007),
        register("GPIOA.MODER", None),
        register("GPIOB.MODER", None),
        register("GPIOB.AFRL", None),
        register("RCC.AHB4ENR", None),
        register("RCC.APB1LENR", None),
        check("ok", 0x60),
        witness(CR2_OK, 0xD0),
        verdict(&[]),
    ]
}

fn find(r: &[Record], pred: impl Fn(&Record) -> bool) -> usize {
    r.iter().position(pred).unwrap()
}

fn replace(r: &mut [Record], pred: impl Fn(&Record) -> bool, with: Record) {
    let i = find(r, pred);
    r[i] = with;
}

fn is_rise_on(net: &'static str) -> impl Fn(&Record) -> bool {
    move |x| matches!(x, Record::RiseMeasured { net: n, .. } if n == net)
}

/// The bus's lines measured at `scl` and `sda` picoseconds; each line past
/// its ceiling is in the bus fault, and the device times out.
fn rises(r: &mut Vec<Record>, scl: u64, sda: u64) {
    replace(r, is_rise_on("I2C_SCL"), rise("I2C_SCL", scl));
    replace(r, is_rise_on("I2C_SDA"), rise("I2C_SDA", sda));
    let past: Vec<&str> = [("I2C_SCL", scl), ("I2C_SDA", sda)]
        .into_iter()
        .filter(|(_, ps)| *ps > 300_000)
        .map(|(n, _)| n)
        .collect();
    let at = find(r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(at, fault(&past, PAST_CEILING_CAUSE));
    replace(
        r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EC, 0),
    );
    replace(
        r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[
            ("analog_rise_exceeds_ceiling", past.len() as u64),
            ("i2c_timeout", 1),
        ]),
    );
}

/// Write `records` as a campaign run through the chaos writer and read it
/// back as `replay` does.
fn archived(records: &[Record]) -> Replayed {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "aet-3200-propose-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let (
        Record::Scenario { sha256, .. },
        Record::Solved {
            specification_sha256,
            topology_sha256,
        },
    ) = (&records[0], &records[1])
    else {
        panic!("a run starts with its scenario and topology")
    };
    let firmware = [9u8; 32];
    let header = Header {
        run_id: run_id(sha256, specification_sha256, &firmware),
        artifacts: vec![
            ("base".into(), [8; 32]),
            ("firmware".into(), firmware),
            ("scenario".into(), *sha256),
            ("specification".into(), *specification_sha256),
            ("topology".into(), *topology_sha256),
        ],
    };
    let mut w = TraceWriter::create(&dir, &header).unwrap();
    for record in records.iter().cloned() {
        let timestamp = if record.before_boot() { 0 } else { 100_000_000 };
        w.append(&Event::Record { timestamp, record }).unwrap();
    }
    w.commit().unwrap();
    let run = replay_dir(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    run
}

fn facts(ohms: u32) -> BTreeMap<String, BusFacts> {
    BTreeMap::from([(
        "SENSORS".to_owned(),
        BusFacts {
            pull_up_ohms: ohms,
            domain_millivolts: 3300,
            floor_ohms: FLOOR,
        },
    )])
}

/// Attribute the run, view its evidence, and ask `proposers` with `facts`.
fn ask_with(
    records: &[Record],
    facts: &BTreeMap<String, BusFacts>,
    proposers: &[&dyn Proposer],
) -> (Attribution, Answer) {
    let run = archived(records);
    let a = attribute(&run).unwrap();
    let v = view(&run).unwrap();
    let evidence: Vec<Evidence> = a.evidence.iter().map(|e| v.event(*e).unwrap()).collect();
    let buses = v.buses();
    let input = Input {
        attribution: &a,
        evidence: &evidence,
        buses: &buses,
        facts,
    };
    let answer = propose::propose(proposers, &input);
    // Deterministic: a second call answers the same.
    assert_eq!(propose::propose(proposers, &input), answer);
    (a, answer)
}

fn ask(records: &[Record], ohms: u32) -> (Attribution, Answer) {
    ask_with(records, &facts(ohms), &[&Rules])
}

fn proposal(answer: &Answer) -> &Proposal {
    assert_eq!(answer.proposer, Some("rules"), "{answer:?}");
    match &answer.outcome {
        Outcome::Proposal(p) => p,
        other => panic!("not a proposal: {other:?}"),
    }
}

fn no_proposal(answer: &Answer) -> (&str, &[Sizing]) {
    match &answer.outcome {
        Outcome::NoProposal { reason, sizing } => (reason, sizing),
        other => panic!("not a NoProposal: {other:?}"),
    }
}

#[test]
fn the_47k_run_proposes_exactly_one_change_2700_with_the_worked_numbers() {
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    let (a, answer) = ask(&r, 47_000);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E1));
    let p = proposal(&answer);
    assert_eq!(
        p.changes,
        [Change {
            bus: "SENSORS".into(),
            pull_up_ohms: 2700
        }]
    );
    // R_target = 47,000 × 240,000 / 3,982,301 = 2,832.5, so E12 gives 2.7 kΩ.
    assert_eq!(
        p.sizing,
        [Sizing {
            bus: "SENSORS".into(),
            r_now_ohms: 47_000,
            binding_net: "I2C_SCL".into(),
            rise_worst_ps: 3_982_301,
            ceiling_ps: 300_000,
            r_target_ohms: 2832,
            rp_min_ohms: FLOOR,
            e12_ohms: Some(2700),
            // 3,982,301 × 2,700 / 47,000 = 228,770.5, rounded up.
            predicted_rise_ps: Some(228_771),
        }]
    );
}

#[test]
fn every_seeded_pull_up_and_the_220_pf_case_propose_by_the_same_rule() {
    // The rises of AET-WO-3100's FEEDBACK, each past the 300 ns ceiling.
    for (ohms, ps, want) in [
        (3_900, 330_443, 2700),
        (4_700, 398_230, 2700),
        (10_000, 847_298, 2700),
        (100_000, 8_472_980, 2700),
        // 220 pF on both lines at 2.2 kΩ: 2,200 × 240,000 / 410,089 = 1,287.5.
        (2_200, 410_089, 1200),
    ] {
        let mut r = healthy(ohms);
        rises(&mut r, ps, ps);
        let (_, answer) = ask(&r, ohms);
        assert_eq!(proposal(&answer).changes[0].pull_up_ohms, want, "{ohms}");
    }
}

#[test]
fn a_capacitance_no_floor_value_can_fix_proposes_nothing_and_shows_its_numbers() {
    // 390 pF on SDA at 2.2 kΩ (about 727 ns): R_target is 726 Ω, and the
    // largest E12 at or below it, 680 Ω, is under the 967 Ω floor.
    let mut r = healthy(2200);
    rises(&mut r, 186_402, 727_000);
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::E1);
    assert_eq!(answer.proposer, Some("rules"));
    let (reason, sizing) = no_proposal(&answer);
    assert!(reason.contains("[967, 726]"), "{reason}");
    assert!(reason.contains("below the pull-up floor"), "{reason}");
    assert_eq!(sizing.len(), 1);
    assert_eq!(
        (
            sizing[0].binding_net.as_str(),
            sizing[0].r_target_ohms,
            sizing[0].e12_ohms
        ),
        ("I2C_SDA", 726, Some(680))
    );
    // At exactly the floor's E12 neighbour the rule does propose: 1.0 kΩ
    // at a floor of 1,000 Ω, never 820 Ω.
    let mut r = healthy(2200);
    rises(&mut r, 186_402, 500_000);
    let mut f = facts(2200);
    f.get_mut("SENSORS").unwrap().floor_ohms = 1000;
    // 2,200 × 240,000 / 500,000 = 1,056: E12 gives 1,000.
    let (_, answer) = ask_with(&r, &f, &[&Rules]);
    assert_eq!(proposal(&answer).changes[0].pull_up_ohms, 1000);
    f.get_mut("SENSORS").unwrap().floor_ohms = 1001;
    let (_, answer) = ask_with(&r, &f, &[&Rules]);
    assert!(matches!(answer.outcome, Outcome::NoProposal { .. }));
}

#[test]
fn the_binding_line_is_the_least_ceiling_over_rise_not_the_slowest() {
    // SCL: 330 ns against 300 ns (0.909); SDA: 280 ns against 250 ns (0.893).
    // SDA binds though SCL is slower.
    let mut r = healthy(3900);
    rises(&mut r, 330_000, 280_000);
    let i = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    if let Record::I2cBusSolved { lines, .. } = &mut r[i] {
        lines[1].ceiling_ps = 250_000;
    }
    replace(
        &mut r,
        is_rise_on("I2C_SDA"),
        Record::RiseMeasured {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            rise_30_70_ps: 280_000,
            complete: true,
            ceiling_ps: 250_000,
            exceeds: true,
        },
    );
    let at = find(&r, |x| matches!(x, Record::BusFault { .. }));
    r[at] = fault(&["I2C_SCL", "I2C_SDA"], PAST_CEILING_CAUSE);
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 1)]),
    );
    let (_, answer) = ask(&r, 3900);
    let p = proposal(&answer);
    assert_eq!(p.sizing[0].binding_net, "I2C_SDA");
    assert_eq!(p.sizing[0].ceiling_ps, 250_000);
    // 3,900 × 200,000 / 280,000 = 2,785.7: 2.7 kΩ. By SCL it would be 2,836.
    assert_eq!(p.sizing[0].r_target_ohms, 2785);
}

fn held_low(r: &mut Vec<Record>) {
    if let Record::Scenario { overrides, .. } = &mut r[0] {
        overrides.push(Override::LineHeldLow {
            bus: "SENSORS".into(),
            line: Line::Sda,
        });
    }
    r.retain(|x| !is_rise_on("I2C_SDA")(x));
    let at = find(r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(at, fault(&["I2C_SDA"], campaign::HELD_LOW));
    replace(
        r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EC, 0),
    );
    replace(
        r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("i2c_line_held_low", 1), ("i2c_timeout", 1)]),
    );
}

#[test]
fn a_held_low_line_has_no_rule_and_names_the_model_slot() {
    let mut r = healthy(2200);
    held_low(&mut r);
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::E1);
    assert_eq!(answer.proposer, None);
    assert_eq!(no_proposal(&answer), (MODEL_SLOT, &[][..]));
}

#[test]
fn a_stuck_high_line_an_incomplete_rise_or_a_fault_of_another_cause_has_no_rule() {
    // Stuck high on SDA.
    let mut r = healthy(2200);
    r.retain(|x| !is_rise_on("I2C_SDA")(x));
    let i = find(&r, is_rise_on("I2C_SCL"));
    r.insert(
        i + 1,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: 1_500_000,
            ceiling_ps: 300_000,
        },
    );
    r.insert(i + 2, fault(&["I2C_SDA"], "analog.line_stuck_high"));
    replace(
        &mut r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EC, 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("analog_line_stuck_high", 1), ("i2c_timeout", 1)]),
    );
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::E1);
    assert_eq!(no_proposal(&answer).0, MODEL_SLOT);

    // An incomplete rise past the ceiling: a lower bound sizes nothing.
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    let i = find(&r, is_rise_on("I2C_SCL"));
    if let Record::RiseMeasured { complete, .. } = &mut r[i] {
        *complete = false;
    }
    let (a, answer) = ask(&r, 47_000);
    assert_eq!(a.rule, Rule::E1);
    assert_eq!(no_proposal(&answer).0, MODEL_SLOT);

    // A bus fault whose cause is not only a rise past its ceiling. (An empty
    // cause list is refused by the archive's reader.)
    for from in [
        vec![PAST_CEILING_CAUSE.to_owned(), campaign::HELD_LOW.to_owned()],
        vec!["analog.rise_time_measured".to_owned()],
    ] {
        let mut r = healthy(47_000);
        rises(&mut r, 3_982_301, 3_982_301);
        let i = find(&r, |x| matches!(x, Record::BusFault { .. }));
        if let Record::BusFault { from: f, .. } = &mut r[i] {
            *f = from.clone();
        }
        let run = archived(&r);
        if attribute(&run).is_err() {
            continue; // refused before any proposer: also never a proposal
        }
        let (_, answer) = ask(&r, 47_000);
        assert_eq!(no_proposal(&answer).0, MODEL_SLOT, "{from:?}");
    }
}

#[test]
fn a_line_with_no_rise_beside_one_past_its_ceiling_sizes_nothing() {
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    r.retain(|x| !is_rise_on("I2C_SDA")(x));
    let at = find(&r, |x| matches!(x, Record::BusFault { .. }));
    r[at] = fault(&["I2C_SCL"], PAST_CEILING_CAUSE);
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("analog_rise_exceeds_ceiling", 1), ("i2c_timeout", 1)]),
    );
    let (a, answer) = ask(&r, 47_000);
    assert_eq!(a.rule, Rule::E1);
    assert_eq!(no_proposal(&answer).0, MODEL_SLOT);
}

#[test]
fn a_wrong_address_is_a_firmware_generator_report_with_no_change() {
    let mut r = healthy(2200);
    replace(
        &mut r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EE, 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("i2c_timeout", 1)]),
    );
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::S1);
    let Outcome::GeneratorReport(g) = &answer.outcome else {
        panic!("{answer:?}")
    };
    assert_eq!(answer.proposer, Some("rules"));
    assert_eq!(g.expectations.len(), 1, "{g:?}");
    let e = &g.expectations[0];
    assert_eq!(e.generator, Generator::Firmware);
    assert_eq!(e.what, "I2C1.CR2");
    assert_eq!(e.address, Some(0x4000_5404));
    // SADD holds the 7-bit address shifted once: 0x76 solved, 0x77 observed.
    assert_eq!(
        (e.mask, e.solved, e.observed),
        (Some(0x3FF), 0xEC, Some(0xEE))
    );
}

#[test]
fn a_clock_gate_left_off_is_a_board_crate_report_and_a_boot_word_a_firmware_one() {
    let mut r = healthy(2200);
    replace(
        &mut r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "RCC.APB1LENR"),
        register("RCC.APB1LENR", Some(0)),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0, 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[
            ("i2c_timeout", 1),
            ("locked_peripheral_access", 1),
            ("register_mismatch", 1),
        ]),
    );
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::S1);
    let Outcome::GeneratorReport(g) = &answer.outcome else {
        panic!("{answer:?}")
    };
    let got: Vec<_> = g
        .expectations
        .iter()
        .map(|e| (e.generator, e.what.as_str(), e.mask, e.solved, e.observed))
        .collect();
    assert_eq!(
        got,
        [(
            Generator::BoardCrate,
            "RCC.APB1LENR",
            Some(1 << 21),
            1 << 21,
            Some(0)
        )]
    );

    let mut r = healthy(2200);
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("boot_word_missing", 1)]),
    );
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::S2);
    let Outcome::GeneratorReport(g) = &answer.outcome else {
        panic!("{answer:?}")
    };
    let got: Vec<_> = g
        .expectations
        .iter()
        .map(|e| (e.generator, e.what.as_str(), e.solved, e.observed))
        .collect();
    assert_eq!(
        got,
        [(Generator::Firmware, "boot_word", 0x2100_B007, Some(0))]
    );
}

#[test]
fn an_indeterminate_or_passing_run_proposes_nothing_and_names_its_rule() {
    // N1: the healthy run.
    let (a, answer) = ask(&healthy(2200), 2200);
    assert_eq!(a.rule, Rule::N1);
    assert_eq!(answer.proposer, Some("rules"));
    let (reason, _) = no_proposal(&answer);
    assert!(
        reason.contains("N1") && reason.contains("INV-05"),
        "{reason}"
    );
    // I4: a TXIS timeout on a healthy bus with a correct witness.
    let mut r = healthy(2200);
    replace(
        &mut r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EC, 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("i2c_timeout", 1)]),
    );
    let (a, answer) = ask(&r, 2200);
    assert_eq!(a.rule, Rule::I4);
    let (reason, _) = no_proposal(&answer);
    assert!(
        reason.contains("I4") && reason.contains("indeterminate"),
        "{reason}"
    );
    // I2: the faulted bus's rises dropped, so nothing is measured.
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    r.retain(|x| !matches!(x, Record::RiseMeasured { .. } | Record::BusFault { .. }));
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("i2c_timeout", 1)]),
    );
    let (a, answer) = ask(&r, 47_000);
    assert_eq!(a.rule, Rule::I2);
    assert!(no_proposal(&answer).0.contains("I2"));
}

/// A proposer that answers whatever it is built with.
struct Fixed(Outcome);

impl Proposer for Fixed {
    fn id(&self) -> &'static str {
        "fixed"
    }
    fn propose(&self, _: &Input) -> Option<Outcome> {
        Some(self.0.clone())
    }
}

struct Silent;

impl Proposer for Silent {
    fn id(&self) -> &'static str {
        "silent"
    }
    fn propose(&self, _: &Input) -> Option<Outcome> {
        None
    }
}

fn change(bus: &str, ohms: u32) -> Outcome {
    Outcome::Proposal(Proposal {
        changes: vec![Change {
            bus: bus.into(),
            pull_up_ohms: ohms,
        }],
        sizing: vec![],
    })
}

#[test]
fn the_first_proposer_that_answers_wins_and_none_answering_is_the_model_slot() {
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    let (_, answer) = ask_with(&r, &facts(47_000), &[&Silent, &Rules]);
    assert_eq!(proposal(&answer).changes[0].pull_up_ohms, 2700);
    let (_, answer) = ask_with(
        &r,
        &facts(47_000),
        &[&Fixed(change("SENSORS", 3300)), &Rules],
    );
    assert_eq!(answer.proposer, Some("fixed"));
    let (_, answer) = ask_with(&r, &facts(47_000), &[&Silent]);
    assert_eq!(
        (answer.proposer, no_proposal(&answer).0),
        (None, MODEL_SLOT)
    );
    let (_, answer) = ask_with(&r, &facts(47_000), &[]);
    assert_eq!(no_proposal(&answer).0, MODEL_SLOT);
}

#[test]
fn no_proposer_may_propose_for_a_run_that_is_not_electrical_or_below_a_floor() {
    let refused = |answer: &Answer| {
        assert_eq!(answer.proposer, None, "{answer:?}");
        let (reason, _) = no_proposal(answer);
        assert!(
            reason.contains("proposer `fixed`") && reason.contains("refused"),
            "{reason}"
        );
    };
    // A proposal for a passing run, and for a software run.
    let (_, answer) = ask_with(
        &healthy(2200),
        &facts(2200),
        &[&Fixed(change("SENSORS", 2700))],
    );
    refused(&answer);
    let mut sw = healthy(2200);
    replace(
        &mut sw,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(
        &mut sw,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("boot_word_missing", 1)]),
    );
    let (_, answer) = ask_with(&sw, &facts(2200), &[&Fixed(change("SENSORS", 2700))]);
    refused(&answer);
    // On an electrical run: below the floor, on a bus with no facts, twice
    // on one bus, or no change at all.
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    let f = facts(47_000);
    for bad in [
        change("SENSORS", FLOOR - 1),
        change("OTHER", 2700),
        Outcome::Proposal(Proposal {
            changes: vec![
                Change {
                    bus: "SENSORS".into(),
                    pull_up_ohms: 2700,
                },
                Change {
                    bus: "SENSORS".into(),
                    pull_up_ohms: 3300,
                },
            ],
            sizing: vec![],
        }),
        Outcome::Proposal(Proposal {
            changes: vec![],
            sizing: vec![],
        }),
    ] {
        let (_, answer) = ask_with(&r, &f, &[&Fixed(bad.clone())]);
        refused(&answer);
    }
    // At the floor it stands.
    let (_, answer) = ask_with(&r, &f, &[&Fixed(change("SENSORS", FLOOR))]);
    assert_eq!(answer.proposer, Some("fixed"));
    // A generator report for an electrical run is refused too.
    let report = Outcome::GeneratorReport(propose::GeneratorReport {
        expectations: vec![],
    });
    let (_, answer) = ask_with(&r, &f, &[&Fixed(report)]);
    refused(&answer);
}

/// The change type holds a bus and a pull-up and nothing else: this
/// destructuring stops compiling the day a field is added (`INV-07`).
#[test]
fn a_change_can_name_only_a_bus_and_its_pull_up() {
    let Change { bus, pull_up_ohms } = Change {
        bus: "SENSORS".into(),
        pull_up_ohms: 2700,
    };
    assert_eq!((bus.as_str(), pull_up_ohms), ("SENSORS", 2700));
}

#[test]
fn the_e12_value_is_the_largest_at_or_below_the_target() {
    for (limit, want) in [
        (2832, Some(2700)),
        (2700, Some(2700)),
        (2699, Some(2200)),
        (1287, Some(1200)),
        (726, Some(680)),
        (10, Some(10)),
        (9, None),
        (0, None),
        (u64::from(u32::MAX), Some(3_900_000_000)),
        (u64::MAX, Some(3_900_000_000)),
    ] {
        assert_eq!(largest_e12_at_most(limit), want, "{limit}");
    }
}

#[test]
fn the_view_names_each_events_bus_and_refuses_an_event_the_run_lacks() {
    let mut r = healthy(47_000);
    rises(&mut r, 3_982_301, 3_982_301);
    let run = archived(&r);
    let v = view(&run).unwrap();
    let rise = v.event(EventRef { index: 3, kind: 7 }).unwrap();
    assert_eq!(rise.bus.as_deref(), Some("SENSORS"));
    assert!(matches!(
        rise.view,
        View::Rise {
            rise_ps: 3_982_301,
            complete: true,
            exceeds: true,
            ..
        }
    ));
    let fault = v.event(EventRef { index: 5, kind: 9 }).unwrap();
    assert_eq!(
        fault.view,
        View::BusFault {
            from: vec![PAST_CEILING_CAUSE.into()],
            nets: vec!["I2C_SCL".into(), "I2C_SDA".into()]
        }
    );
    // A kind the event is not, or an index past the run, is no event.
    assert_eq!(v.event(EventRef { index: 3, kind: 8 }), None);
    assert_eq!(v.event(EventRef { index: 99, kind: 7 }), None);
    let buses = v.buses();
    assert_eq!(buses.len(), 1);
    assert_eq!(
        buses[0]
            .lines
            .iter()
            .map(|l| (l.net.as_str(), l.ceiling_ps, l.rises.len()))
            .collect::<Vec<_>>(),
        [("I2C_SCL", 300_000, 1), ("I2C_SDA", 300_000, 1)]
    );
    // An owned register's fields carry their solved and observed values.
    let i = find(
        &r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOB.AFRL"),
    );
    let e = v
        .event(EventRef {
            index: i as u64,
            kind: 11,
        })
        .unwrap();
    let View::Fields {
        device: None,
        fields,
    } = &e.view
    else {
        panic!("{e:?}")
    };
    assert_eq!(
        e.bus.as_deref(),
        None,
        "a register nothing is wrong in is on no bus"
    );
    assert_eq!(fields.len(), 2);
    assert!(
        fields
            .iter()
            .all(|f| !f.wrong && f.observed == Some(f.solved))
    );
}

// ---- Hand-built inputs: the rows `attribute` never hands over (mutation run) ----

fn attribution(rule: Rule) -> Attribution {
    Attribution {
        run_id: [0; 16],
        state_hash: [0; 32],
        domain: rule.domain(),
        rule,
        evidence: vec![],
        symptoms: vec![],
        masked: vec![],
        unexplained: vec![],
    }
}

fn rise_ev(bus: &str, index: u64, net: &str, ps: u64, complete: bool, exceeds: bool) -> Evidence {
    Evidence {
        event: EventRef { index, kind: 7 },
        bus: Some(bus.into()),
        view: View::Rise {
            net: net.into(),
            rise_ps: ps,
            complete,
            ceiling_ps: 300_000,
            exceeds,
        },
    }
}

fn fault_ev(bus: &str, from: &[&str]) -> Evidence {
    Evidence {
        event: EventRef { index: 9, kind: 9 },
        bus: Some(bus.into()),
        view: View::BusFault {
            from: from.iter().map(|c| (*c).into()).collect(),
            nets: vec!["SCL".into(), "SDA".into()],
        },
    }
}

/// A solved bus whose two lines, SCL and SDA, have `scl` and `sda` as their
/// measured rises `(ps, complete)`.
fn solved_bus(bus: &str, scl: &[(u64, bool)], sda: &[(u64, bool)]) -> SolvedBus {
    let line = |net: &str, rises: &[(u64, bool)]| LineRises {
        net: net.into(),
        ceiling_ps: 300_000,
        rises: rises
            .iter()
            .map(|&(rise_ps, complete)| Rise { rise_ps, complete })
            .collect(),
    };
    SolvedBus {
        bus: bus.into(),
        lines: vec![line("SCL", scl), line("SDA", sda)],
    }
}

fn bus_facts(entries: &[(&str, u32)]) -> BTreeMap<String, BusFacts> {
    entries
        .iter()
        .map(|&(bus, ohms)| {
            (
                bus.to_owned(),
                BusFacts {
                    pull_up_ohms: ohms,
                    domain_millivolts: 3300,
                    floor_ohms: FLOOR,
                },
            )
        })
        .collect()
}

fn hand(
    rule: Rule,
    evidence: &[Evidence],
    buses: &[SolvedBus],
    facts: &BTreeMap<String, BusFacts>,
) -> Answer {
    let a = attribution(rule);
    propose::propose(
        &[&Rules],
        &Input {
            attribution: &a,
            evidence,
            buses,
            facts,
        },
    )
}

const PAST: (u64, bool) = (3_982_301, true);

fn e1(evidence: &[Evidence], buses: &[SolvedBus], facts: &BTreeMap<String, BusFacts>) -> Answer {
    hand(Rule::E1, evidence, buses, facts)
}

fn the_47k_evidence() -> Vec<Evidence> {
    vec![
        rise_ev("SENSORS", 3, "SCL", PAST.0, true, true),
        rise_ev("SENSORS", 4, "SDA", PAST.0, true, true),
        fault_ev("SENSORS", &[PAST_CEILING_CAUSE]),
    ]
}

#[test]
fn the_hand_built_47k_row_proposes_2700_so_the_rows_below_differ_only_where_named() {
    let buses = [solved_bus("SENSORS", &[PAST], &[PAST])];
    let answer = e1(
        &the_47k_evidence(),
        &buses,
        &bus_facts(&[("SENSORS", 47_000)]),
    );
    assert_eq!(proposal(&answer).changes[0].pull_up_ohms, 2700);
}

#[test]
fn evidence_that_is_not_a_complete_rise_past_its_ceiling_or_its_fault_has_no_rule() {
    let buses = [solved_bus("SENSORS", &[PAST], &[PAST])];
    let facts = bus_facts(&[("SENSORS", 47_000)]);
    let stuck = Evidence {
        event: EventRef { index: 5, kind: 8 },
        bus: Some("SENSORS".into()),
        view: View::StuckHigh { net: "SDA".into() },
    };
    for (what, swap) in [
        (
            "incomplete",
            rise_ev("SENSORS", 3, "SCL", PAST.0, false, true),
        ),
        ("within", rise_ev("SENSORS", 3, "SCL", PAST.0, true, false)),
        ("no cause", fault_ev("SENSORS", &[])),
        (
            "a second cause",
            fault_ev("SENSORS", &[PAST_CEILING_CAUSE, campaign::HELD_LOW]),
        ),
        ("stuck high", stuck),
    ] {
        let mut evidence = the_47k_evidence();
        match swap.event.kind {
            7 => evidence[0] = swap,
            9 => evidence[2] = swap,
            _ => evidence.push(swap),
        }
        let answer = e1(&evidence, &buses, &facts);
        assert_eq!(
            (answer.proposer, no_proposal(&answer).0),
            (None, MODEL_SLOT),
            "{what}"
        );
    }
}

#[test]
fn a_bus_whose_lines_are_not_each_measured_once_and_completely_has_no_rule() {
    let facts = bus_facts(&[("SENSORS", 47_000)]);
    for (what, sda) in [
        ("two rises", vec![PAST, PAST]),
        ("none", vec![]),
        ("incomplete", vec![(186_402, false)]),
        ("zero", vec![(0, true)]),
    ] {
        let buses = [solved_bus("SENSORS", &[PAST], &sda)];
        let evidence = &the_47k_evidence()[..1];
        let answer = e1(evidence, &buses, &facts);
        assert_eq!(no_proposal(&answer).0, MODEL_SLOT, "{what}");
    }
    // The binding line exactly at its ceiling is not past it.
    let buses = [solved_bus(
        "SENSORS",
        &[(300_000, true)],
        &[(300_000, true)],
    )];
    let answer = e1(&the_47k_evidence(), &buses, &facts);
    assert_eq!(no_proposal(&answer).0, MODEL_SLOT);
}

#[test]
fn each_faulted_bus_is_sized_by_its_own_lines_and_facts() {
    // AUX sorts first and is healthy; SENSORS, second, is the faulted bus.
    let buses = [
        solved_bus("AUX", &[(186_402, true)], &[(186_402, true)]),
        solved_bus("SENSORS", &[(398_230, true)], &[(398_230, true)]),
    ];
    let facts = bus_facts(&[("AUX", 2200), ("SENSORS", 4700)]);
    let answer = e1(&the_47k_evidence(), &buses, &facts);
    let p = proposal(&answer);
    assert_eq!(
        p.changes,
        [Change {
            bus: "SENSORS".into(),
            pull_up_ohms: 2700
        }]
    );
    assert_eq!(
        (p.sizing[0].r_now_ohms, p.sizing[0].rise_worst_ps),
        (4700, 398_230)
    );
    // With no facts for the faulted bus, nothing is proposed, and it says so.
    let answer = e1(&the_47k_evidence(), &buses, &bus_facts(&[("AUX", 2200)]));
    assert_eq!(no_proposal(&answer).0, "no solved facts for bus `SENSORS`");
    // Both buses faulted and both below the floor: the first is named.
    let mut evidence = the_47k_evidence();
    evidence.push(rise_ev("AUX", 1, "SCL", 9_000_000, true, true));
    let buses = [
        solved_bus("AUX", &[(9_000_000, true)], &[(186_402, true)]),
        solved_bus("SENSORS", &[(9_000_000, true)], &[(9_000_000, true)]),
    ];
    let facts = bus_facts(&[("AUX", 2200), ("SENSORS", 2200)]);
    let answer = e1(&evidence, &buses, &facts);
    let (reason, sizing) = no_proposal(&answer);
    assert!(reason.starts_with("bus `AUX`"), "{reason}");
    assert_eq!(sizing.len(), 2);
}

#[test]
fn a_target_below_every_e12_value_proposes_nothing() {
    // 100 Ω × 240,000 / 3,982,301 = 6 Ω: no E12 value at all.
    let buses = [solved_bus("SENSORS", &[PAST], &[PAST])];
    let answer = e1(&the_47k_evidence(), &buses, &bus_facts(&[("SENSORS", 100)]));
    let (reason, sizing) = no_proposal(&answer);
    assert!(reason.contains("floor"), "{reason}");
    assert_eq!(
        (
            sizing[0].r_target_ohms,
            sizing[0].e12_ohms,
            sizing[0].predicted_rise_ps
        ),
        (6, None, None)
    );
}

/// The E12 series, written out here, not read from the code under test.
const E12_SERIES: [u64; 12] = [10, 12, 15, 18, 22, 27, 33, 39, 47, 56, 68, 82];

#[test]
fn every_e12_value_is_its_own_answer_and_one_ohm_less_is_the_one_before() {
    let mut previous = None;
    let mut decade = 1u64;
    loop {
        for m in E12_SERIES {
            let v = m * decade;
            if v > u64::from(u32::MAX) {
                // 4.7 GΩ is past a u32: 3.9 GΩ stays the answer.
                assert_eq!(largest_e12_at_most(v), previous, "{v}");
                return;
            }
            assert_eq!(largest_e12_at_most(v), Some(v as u32), "{v}");
            assert_eq!(largest_e12_at_most(v - 1), previous, "{v} - 1");
            previous = Some(v as u32);
        }
        decade *= 10;
    }
}

fn boot_ev(word_ok: bool, observed: Option<u64>) -> Evidence {
    Evidence {
        event: EventRef { index: 5, kind: 10 },
        bus: None,
        view: View::Boot {
            boot_word: aethelgard_evolve::Field {
                register: "boot_word".into(),
                address: 0x2000_0000,
                mask: u32::MAX,
                solved: 0x2100_B007,
                observed: Some(if word_ok { 0x2100_B007 } else { 0 }),
                wrong: !word_ok,
            },
            instructions_expected: 10_000_000,
            instructions_observed: observed,
        },
    }
}

#[test]
fn a_short_boot_is_a_firmware_report_of_its_instruction_count() {
    let answer = hand(Rule::S2, &[boot_ev(true, Some(5))], &[], &BTreeMap::new());
    let Outcome::GeneratorReport(g) = &answer.outcome else {
        panic!("{answer:?}")
    };
    let got: Vec<_> = g
        .expectations
        .iter()
        .map(|e| {
            (
                e.generator,
                e.what.as_str(),
                e.address,
                e.mask,
                e.solved,
                e.observed,
            )
        })
        .collect();
    assert_eq!(
        got,
        [(
            Generator::Firmware,
            "instructions",
            None,
            None,
            10_000_000,
            Some(5)
        )]
    );
}

#[test]
fn evidence_that_shows_no_unmet_expectation_makes_no_report() {
    // A boot whose word is right and whose instruction count is unobserved.
    let answer = hand(Rule::S2, &[boot_ev(true, None)], &[], &BTreeMap::new());
    assert_eq!(
        no_proposal(&answer).0,
        "evidence event 5 (kind 10) shows no unmet solved expectation, so no generator report is made"
    );
}

#[test]
fn the_view_reads_every_field_of_a_rise_a_stuck_line_a_boot_and_a_witness() {
    let mut r = healthy(2200);
    replace(
        &mut r,
        |x| matches!(x, Record::I2cCheck { .. }),
        check("timeout_txis", 0),
    );
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { .. }),
        witness(0x0001_00EE, 0),
    );
    let i = find(&r, |x| matches!(x, Record::BootObserved { .. }));
    if let Record::BootObserved {
        boot_word,
        instructions_observed,
        ..
    } = &mut r[i]
    {
        boot_word.observed = None;
        *instructions_observed = Some(7);
    }
    let i = find(&r, is_rise_on("I2C_SDA"));
    r[i] = Record::RiseMeasured {
        bus: "SENSORS".into(),
        net: "I2C_SDA".into(),
        rise_30_70_ps: 200_000,
        complete: false,
        ceiling_ps: 300_000,
        exceeds: true,
    };
    let b = find(&r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(b, fault(&["I2C_SDA"], PAST_CEILING_CAUSE));
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("analog_rise_exceeds_ceiling", 1), ("i2c_timeout", 1)]),
    );
    let run = archived(&r);
    let v = view(&run).unwrap();
    let at = |pred: &dyn Fn(&Record) -> bool| {
        let i = r.iter().position(pred).unwrap();
        v.event(EventRef {
            index: i as u64,
            kind: r[i].kind(),
        })
        .unwrap()
    };
    assert_eq!(
        at(&is_rise_on("I2C_SDA")).view,
        View::Rise {
            net: "I2C_SDA".into(),
            rise_ps: 200_000,
            complete: false,
            ceiling_ps: 300_000,
            exceeds: true
        }
    );
    assert_eq!(
        at(&is_rise_on("I2C_SCL")).view,
        View::Rise {
            net: "I2C_SCL".into(),
            rise_ps: 186_402,
            complete: true,
            ceiling_ps: 300_000,
            exceeds: false
        }
    );
    let boot = at(&|x| matches!(x, Record::BootObserved { .. }));
    let View::Boot {
        boot_word,
        instructions_expected,
        instructions_observed,
    } = &boot.view
    else {
        panic!("{boot:?}")
    };
    assert!(
        boot_word.wrong,
        "an unobserved boot word is not the solved one"
    );
    assert_eq!(
        (*instructions_expected, *instructions_observed),
        (10_000_000, Some(7))
    );
    let w = at(&|x| matches!(x, Record::I2cWitness { .. }));
    assert_eq!(w.bus.as_deref(), Some("SENSORS"));
    let View::Fields { device, fields } = &w.view else {
        panic!("{w:?}")
    };
    assert_eq!(device.as_deref(), Some("U2"));
    let got: Vec<_> = fields
        .iter()
        .map(|f| (f.register.as_str(), f.mask, f.solved, f.observed, f.wrong))
        .collect();
    assert_eq!(
        got,
        [
            ("I2C1.CR2", 0x3FF, 0xEC, Some(0xEE), true),
            (
                "I2C1.CR2",
                0x00FF_0000,
                0x0001_0000,
                Some(0x0001_0000),
                false
            ),
            ("I2C1.TXDR", 0xFF, 0xD0, Some(0), false),
        ]
    );

    // A line stuck high.
    let mut r = healthy(2200);
    r.retain(|x| !is_rise_on("I2C_SDA")(x));
    let i = find(&r, is_rise_on("I2C_SCL"));
    r.insert(
        i + 1,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: 1_500_000,
            ceiling_ps: 300_000,
        },
    );
    r.insert(i + 2, fault(&["I2C_SDA"], "analog.line_stuck_high"));
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("analog_line_stuck_high", 1)]),
    );
    let run = archived(&r);
    let e = view(&run)
        .unwrap()
        .event(EventRef {
            index: i as u64 + 1,
            kind: 8,
        })
        .unwrap();
    assert_eq!(
        e.view,
        View::StuckHigh {
            net: "I2C_SDA".into()
        }
    );
}

#[test]
fn the_view_joins_a_register_only_by_its_name_address_and_covered_fields() {
    let mut r = healthy(2200);
    // Port B's gate is owned by both pins: said once. The I2C gate, in
    // another RCC register, is not this register's.
    let i = find(
        &r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "RCC.AHB4ENR"),
    );
    // A reading of AFRL covering PB6's field alone, and a GPIOA register
    // wrong in bits no bus owns.
    let j = find(
        &r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOB.AFRL"),
    );
    if let Record::RegisterObserved { reading, .. } = &mut r[j] {
        reading.mask = 0x0F00_0000;
        reading.expected = 0x0400_0000;
        reading.observed = Some(0x0400_0000);
    }
    let k = find(
        &r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOA.MODER"),
    );
    if let Record::RegisterObserved { reading, .. } = &mut r[k] {
        reading.observed = Some(0x0000_A803);
    }
    replace(
        &mut r,
        |x| matches!(x, Record::Verdict { .. }),
        verdict(&[("register_mismatch", 1)]),
    );
    let run = archived(&r);
    let v = view(&run).unwrap();
    let fields = |i: usize| match v
        .event(EventRef {
            index: i as u64,
            kind: 11,
        })
        .unwrap()
        .view
    {
        View::Fields { fields, .. } => fields
            .into_iter()
            .map(|f| (f.register, f.address, f.mask, f.solved, f.observed, f.wrong))
            .collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        fields(i),
        [(
            "RCC.AHB4ENR".to_owned(),
            0x5802_44E0,
            0x2,
            0x2,
            Some(0x2),
            false
        )]
    );
    assert_eq!(
        fields(j),
        [(
            "GPIOB.AFRL".to_owned(),
            0x5802_0420,
            0x0F00_0000,
            0x0400_0000,
            Some(0x0400_0000),
            false
        )]
    );
    // GPIOA's MODER bit 1 is wrong and no bus owns it.
    assert_eq!(
        fields(k),
        [(
            "GPIOA.MODER".to_owned(),
            0x5802_0000,
            0x2,
            0x0,
            Some(0x2),
            true
        )]
    );
    // The same name at another address joins nothing.
    let mut r2 = r.clone();
    if let Record::RegisterObserved { reading, .. } = &mut r2[i] {
        reading.address += 4;
    }
    let run2 = archived(&r2);
    match view(&run2)
        .unwrap()
        .event(EventRef {
            index: i as u64,
            kind: 11,
        })
        .unwrap()
        .view
    {
        View::Fields { fields, .. } => assert!(fields.is_empty(), "{fields:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn on_two_buses_the_view_puts_each_event_and_rise_on_its_own_bus() {
    let mut r = healthy(2200);
    let at = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        Record::I2cBusSolved {
            bus: "AUX".into(),
            peripheral: "I2C2".into(),
            lines: vec![
                BusLine {
                    signal: "I2C2_SCL".into(),
                    net: "A_SCL".into(),
                    pin: "PF1".into(),
                    alternate_function: 4,
                    ceiling_ps: 300_000,
                },
                BusLine {
                    signal: "I2C2_SDA".into(),
                    net: "A_SDA".into(),
                    pin: "PF0".into(),
                    alternate_function: 4,
                    ceiling_ps: 300_000,
                },
            ],
            devices: vec![("U5".into(), 0x40)],
        },
    );
    let i = find(&r, |x| matches!(x, Record::RiseMeasured { .. }));
    for (n, (net, ps)) in [("A_SCL", 111_111), ("A_SDA", 222_222)]
        .into_iter()
        .enumerate()
    {
        r.insert(
            i + n,
            Record::RiseMeasured {
                bus: "AUX".into(),
                net: net.into(),
                rise_30_70_ps: ps,
                complete: true,
                ceiling_ps: 300_000,
                exceeds: false,
            },
        );
    }
    let last_check = r
        .iter()
        .rposition(|x| matches!(x, Record::I2cCheck { .. }))
        .unwrap();
    r.insert(
        last_check + 1,
        Record::I2cCheck {
            reference: "U5".into(),
            address: 0x40,
            register: 0xD0,
            expected: 0x60,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
    );
    let last_witness = r
        .iter()
        .rposition(|x| matches!(x, Record::I2cWitness { .. }))
        .unwrap();
    r.insert(
        last_witness + 1,
        Record::I2cWitness {
            peripheral: "I2C2".into(),
            reference: "U5".into(),
            cr2: reading(0x4000_5804, 0x02FF_07FF, 0x0201_0480, 0x0201_0480),
            txdr: reading(0x4000_5828, 0xFF, 0xD0, 0xD0),
        },
    );
    let run = archived(&r);
    let v = view(&run).unwrap();
    let w = r
        .iter()
        .position(|x| matches!(x, Record::I2cWitness { reference, .. } if reference == "U5"))
        .unwrap();
    let e = v
        .event(EventRef {
            index: w as u64,
            kind: 13,
        })
        .unwrap();
    assert_eq!(e.bus.as_deref(), Some("AUX"));
    let View::Fields { fields, .. } = &e.view else {
        panic!("{e:?}")
    };
    assert_eq!(fields[0].register, "I2C2.CR2");
    let rises: Vec<(String, Vec<u64>)> = v
        .buses()
        .iter()
        .flat_map(|b| {
            b.lines.iter().map(move |l| {
                (
                    format!("{}/{}", b.bus, l.net),
                    l.rises.iter().map(|x| x.rise_ps).collect(),
                )
            })
        })
        .collect();
    assert_eq!(
        rises,
        [
            ("AUX/A_SCL".to_owned(), vec![111_111]),
            ("AUX/A_SDA".to_owned(), vec![222_222]),
            ("SENSORS/I2C_SCL".to_owned(), vec![186_402]),
            ("SENSORS/I2C_SDA".to_owned(), vec![186_402]),
        ]
    );
}
