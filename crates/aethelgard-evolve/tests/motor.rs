//! `AET-WO-4300a` §1.5, tier 1: attribution of a motor output's symptoms
//! (`Rule::S4`) on campaign runs written through `aethelgard-chaos`'s own
//! writer, with no Renode. The run is the reference board's I2C run (as
//! `attribute.rs` builds it) with one motor output, `MOTOR1`, TIM1_CH1 on
//! PA8 at AF1, added where `check_run` ranks its records.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::campaign::{
    self, MOTOR_MISMATCH_FINDING, MOTOR_STATIC_FINDING, MOTOR_UNWITNESSED_FINDING, run_id,
};
use aethelgard_chaos::record::{BusLine, Override, Reading, Record};
use aethelgard_chaos::{Event, Header, Replayed, TraceWriter, replay_dir};
use std::collections::BTreeMap;

use aethelgard_evolve::propose::{self, BusFacts, Input, Outcome, Rules};
use aethelgard_evolve::{Attribution, Domain, Evidence, Rule, attribute, view};

fn reading(address: u32, mask: u32, expected: u32, observed: u32) -> Reading {
    Reading {
        address,
        mask,
        expected,
        observed: Some(observed),
    }
}

fn register(name: &str, address: u32, mask: u32, expected: u32, observed: u32) -> Record {
    Record::RegisterObserved {
        name: name.into(),
        reading: reading(address, mask, expected, observed),
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

fn rise(net: &str) -> Record {
    Record::RiseComputed {
        bus: "SENSORS".into(),
        net: net.into(),
        rise_30_70_ps: 186_402,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    }
}

fn i2c_witness() -> Record {
    let device = "i2c1.u2".to_owned();
    let lines = vec![
        format!("[NOISY] {device}: Write D0"),
        format!("[NOISY] {device}: Read 60"),
    ];
    Record::I2cWitness {
        peripheral: "I2C1".into(),
        reference: "U2".into(),
        step: "identity".into(),
        expected: lines.clone(),
        observed: Some(lines),
        device,
        absent: Vec::new(),
    }
}

/// What a firmware may have got wrong.
#[derive(Default)]
struct Wrong {
    afrh: Option<u32>,
    cr1: Option<u32>,
    psc: Option<u32>,
}

/// A witness of output `label` on `observer`, with counts its status allows
/// (replay re-derives the status from them), at the model's timing for
/// `ccr`.
fn witness_of(label: &str, observer: &str, status: &str, ccr: u32) -> Record {
    let edges = status == "witnessed" || status == "mismatch";
    Record::MotorWitness {
        label: label.into(),
        observer: observer.into(),
        period_ns: 639_744,
        high_ns: u64::from(ccr) * 256,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: status.into(),
        rises: if edges { 156 } else { 0 },
        falls: if edges { 155 } else { 0 },
        first_ns: edges.then_some(639_750),
        last_ns: edges.then_some(99_800_070),
        end_ns: 100_000_000,
        span_ns: edges.then_some(if status == "mismatch" { 400 } else { 8 }),
        off_grid: if edges { 311 } else { 0 },
        batched: 0,
        // Trace 13 (`AET-WO-2302`): every output of the timer names its
        // anchor, one period before the first rise.
        anchor_ns: Some(6),
    }
}

fn solved(label: &str, channel: u8, pin: &str, ccr: u32) -> Record {
    Record::MotorOutputSolved {
        label: label.into(),
        timer: "TIM1".into(),
        channel,
        pin: pin.into(),
        alternate_function: 1,
        protocol: "pwm".into(),
        psc: 63,
        arr: 2499,
        ccr,
        pwm: Some((400, 1000, 2000)),
    }
}

/// The static check of an output at `ccr`: it holds at the 1000 µs idle
/// pulse's 1000 ticks, and fails a tick short.
fn check(label: &str, ccr: u32) -> Record {
    Record::MotorTimingCheck {
        label: label.into(),
        kernel_hz: 64_000_000,
        period_ps: 2_500_000_000,
        high_ps: u64::from(ccr) * 1_000_000,
        nominal_period_ps: 2_500_000_000,
        nominal_high_ps: 1_000_000_000,
        holds: ccr == 1000,
    }
}

fn run(wrong: &Wrong, status: &str, holds: bool, findings: &[(&str, u64)]) -> Vec<Record> {
    let ok = |w: Option<u32>, v: u32| w.unwrap_or(v);
    let ccr = if holds { 1000 } else { 999 };
    let mut findings: Vec<(String, String, u64)> = findings
        .iter()
        .map(|(k, n)| ((*k).into(), "error".into(), *n))
        .collect();
    findings.push(("device_unmodelled".into(), "info".into(), 1));
    findings.sort();
    vec![
        Record::Scenario {
            name: "motors".into(),
            sha256: [1; 32],
            expect_pass: findings.iter().all(|f| f.1 == "info"),
            expect_findings: findings
                .iter()
                .filter(|f| f.1 != "info")
                .map(|f| f.0.clone())
                .collect(),
            overrides: vec![Override::PullUpOhms {
                bus: "SENSORS".into(),
                ohms: 2200,
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
        solved("MOTOR1", 1, "PA8", ccr),
        check("MOTOR1", ccr),
        rise("I2C_SCL"),
        rise("I2C_SDA"),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0x2100_B007),
            instructions_expected: 10_000_000,
            instructions_observed: Some(10_000_000),
            pc: Some(0x0800_0596),
            executable: vec![(0x0800_0000, 0x0800_1000)],
            brown_out: aethelgard_chaos::record::Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
        register(
            "GPIOA.MODER",
            0x5802_0000,
            0x0003_0000,
            0x0002_0000,
            0x0002_0000,
        ),
        register(
            "GPIOA.AFRH",
            0x5802_0024,
            0x0000_000F,
            0x1,
            ok(wrong.afrh, 0x1),
        ),
        register(
            "GPIOB.MODER",
            0x5802_0400,
            0x0000_F000,
            0x0000_A000,
            0x0000_A000,
        ),
        register(
            "GPIOB.AFRL",
            0x5802_0420,
            0xFF00_0000,
            0x4400_0000,
            0x4400_0000,
        ),
        register("RCC.AHB4ENR", 0x5802_44E0, u32::MAX, 0x3, 0x3),
        register(
            "RCC.APB1LENR",
            0x5802_44E8,
            u32::MAX,
            0x0020_0000,
            0x0020_0000,
        ),
        register("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0x1, 0x1),
        register("TIM1.CR1", 0x4001_0000, 0xFF, 0x81, ok(wrong.cr1, 0x81)),
        register("TIM1.PSC", 0x4001_0028, u32::MAX, 63, ok(wrong.psc, 63)),
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x76,
            step: "identity".into(),
            write: vec![0xD0],
            width: 1,
            expected: 0x60,
            mask: 0xFF,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
        i2c_witness(),
        witness_of("MOTOR1", "gpioPortA.pa8obs", status, ccr),
        Record::Verdict {
            pass: findings.iter().all(|f| f.1 == "info"),
            findings,
        },
    ]
}

fn archived(records: &[Record]) -> Replayed {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "aet-4300a-evolve-motor-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let firmware = [9u8; 32];
    let header = Header {
        run_id: run_id(&[1; 32], &[2; 32], &firmware),
        artifacts: vec![
            ("base".into(), [8; 32]),
            ("firmware".into(), firmware),
            ("scenario".into(), [1; 32]),
            ("specification".into(), [2; 32]),
            ("topology".into(), [3; 32]),
        ],
        tools: campaign::tools_of(records)
            .iter()
            .map(|n| ((*n).to_owned(), "test".to_owned()))
            .collect(),
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

fn attributed(records: &[Record]) -> Attribution {
    campaign::check_run(records).expect("the run is one check_run accepts");
    let a = attribute(&archived(records)).unwrap();
    for e in &a.evidence {
        assert!(a.rule.evidence_kinds().contains(&e.kind), "{a:?}");
    }
    assert!(a.unexplained.is_empty(), "{a:?}");
    a
}

fn named(records: &[Record], a: &Attribution) -> Vec<String> {
    a.evidence
        .iter()
        .map(|e| match &records[e.index as usize] {
            Record::RegisterObserved { name, .. } => name.clone(),
            r => format!("kind {}", r.kind()),
        })
        .collect()
}

#[test]
fn a_healthy_motor_run_passes_none() {
    let a = attributed(&run(&Wrong::default(), "witnessed", true, &[]));
    assert_eq!((a.domain, a.rule), (Domain::None, Rule::N1));
}

/// The seeded `motor_output_dead`: PA8 on AF2, not the solved AF1, so the
/// timer's channel never reaches the pin. The alternate-function register is
/// the firmware's own write, and nothing electrical is modelled on the pin:
/// software, S4, by the register and the unwitnessed output.
#[test]
fn a_pin_on_the_wrong_alternate_function_is_software_s4() {
    let r = run(
        &Wrong {
            afrh: Some(0x2),
            ..Wrong::default()
        },
        "unwitnessed",
        true,
        &[("register_mismatch", 1), (MOTOR_UNWITNESSED_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S4));
    assert_eq!(named(&r, &a), ["GPIOA.AFRH", "kind 26"]);
}

#[test]
fn adversarial_4_a_stopped_or_re_rated_timer_is_s4_by_its_own_register() {
    let stopped = run(
        &Wrong {
            cr1: Some(0x80),
            ..Wrong::default()
        },
        "unwitnessed",
        true,
        &[("register_mismatch", 1), (MOTOR_UNWITNESSED_FINDING, 1)],
    );
    let a = attributed(&stopped);
    assert_eq!(a.rule, Rule::S4);
    assert_eq!(named(&stopped, &a), ["TIM1.CR1", "kind 26"]);
    // The timer run at another rate, the claim kept: the edges miss the
    // model's sequence, and the prescaler is the evidence.
    let re_rated = run(
        &Wrong {
            psc: Some(31),
            ..Wrong::default()
        },
        "mismatch",
        true,
        &[("register_mismatch", 1), (MOTOR_MISMATCH_FINDING, 1)],
    );
    let a = attributed(&re_rated);
    assert_eq!(a.rule, Rule::S4);
    assert_eq!(named(&re_rated, &a), ["TIM1.PSC", "kind 26"]);
}

#[test]
fn adversarial_4_no_register_evidence_is_never_a_guess() {
    // Every owned register right and still no edges: nothing shows which half
    // failed, so the run is indeterminate, never software by elimination.
    let silent = run(
        &Wrong::default(),
        "unwitnessed",
        true,
        &[(MOTOR_UNWITNESSED_FINDING, 1)],
    );
    let a = attributed(&silent);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I4));
    // A static mismatch is the topology's, not the firmware's: indeterminate.
    let static_only = run(
        &Wrong::default(),
        "witnessed",
        false,
        &[(MOTOR_STATIC_FINDING, 1)],
    );
    assert_eq!(attributed(&static_only).rule, Rule::I4);
}

#[test]
fn adversarial_4_a_rewritten_verdict_cannot_move_the_domain() {
    // The register is wrong but the verdict names only the dead output: the
    // register's event shows a failure the verdict does not name.
    let r = run(
        &Wrong {
            afrh: Some(0x2),
            ..Wrong::default()
        },
        "unwitnessed",
        true,
        &[(MOTOR_UNWITNESSED_FINDING, 1)],
    );
    // check_run does not re-derive register_mismatch; attribution refuses
    // to explain the run.
    let a = attribute(&archived(&r)).unwrap();
    assert_eq!(a.rule, Rule::I1, "{a:?}");
}

/// `propose` answers S4 with a generator report naming the board crate's
/// wrong alternate-function field, and nothing else (`INV-05`).
#[test]
fn propose_answers_the_dead_output_with_a_generator_report_of_its_af_field() {
    let r = run(
        &Wrong {
            afrh: Some(0x2),
            ..Wrong::default()
        },
        "unwitnessed",
        true,
        &[("register_mismatch", 1), (MOTOR_UNWITNESSED_FINDING, 1)],
    );
    let archived = archived(&r);
    let a = attribute(&archived).unwrap();
    let v = view(&archived).unwrap();
    let evidence: Vec<Evidence> = a.evidence.iter().map(|e| v.event(*e).unwrap()).collect();
    let buses = v.buses();
    let facts = BTreeMap::from([(
        "SENSORS".to_owned(),
        BusFacts {
            pull_up_ohms: 2200,
            domain_millivolts: 3300,
            floor_ohms: 967,
        },
    )]);
    let input = Input {
        attribution: &a,
        evidence: &evidence,
        buses: &buses,
        facts: &facts,
    };
    let ans = propose::propose(&[&Rules], &input);
    let Outcome::GeneratorReport(report) = &ans.outcome else {
        panic!("{:?}", ans.outcome)
    };
    let what: Vec<(&str, u64, u64, Option<u64>)> = report
        .expectations
        .iter()
        .map(|e| (e.what.as_str(), e.mask.unwrap(), e.solved, e.observed))
        .collect();
    assert_eq!(what, [("GPIOA.AFRH", 0xF, 1, Some(2))]);
}

/// `run` with a second output, `MOTOR2` on TIM1_CH2 (PA9), witnessed as
/// `status`, and `TIM1.CCR2` read as `ccr2`, the verdict's findings as given.
fn with_motor2(status: &str, ccr2: u32, findings: &[(&str, u64)]) -> Vec<Record> {
    let mut r = run(&Wrong::default(), "witnessed", true, findings);
    let at = |r: &[Record], f: &dyn Fn(&Record) -> bool| r.iter().position(f).unwrap() + 1;
    let i = at(&r, &|x| matches!(x, Record::MotorOutputSolved { .. }));
    r.insert(i, solved("MOTOR2", 2, "PA9", 1000));
    let i = at(&r, &|x| matches!(x, Record::MotorTimingCheck { .. }));
    r.insert(i, check("MOTOR2", 1000));
    let i = at(
        &r,
        &|x| matches!(x, Record::RegisterObserved { name, .. } if name == "TIM1.PSC"),
    );
    r.insert(i, register("TIM1.CCR2", 0x4001_0038, u32::MAX, 1000, ccr2));
    let i = at(&r, &|x| matches!(x, Record::MotorWitness { .. }));
    r.insert(i, witness_of("MOTOR2", "gpioPortA.pa9obs", status, 1000));
    r
}

#[test]
fn adversarial_h3_a_channel_s_register_is_its_own_output_s_alone() {
    // Adversarial pass, H3: MOTOR1 dead with every register of its own
    // right, and another channel's CCR2 wrong. The wrong CCR2 is MOTOR2's
    // evidence only; MOTOR1's dead output stays indeterminate.
    let mut r = with_motor2(
        "mismatch",
        0x3e9,
        &[
            ("register_mismatch", 1),
            (MOTOR_MISMATCH_FINDING, 1),
            (MOTOR_UNWITNESSED_FINDING, 1),
        ],
    );
    let i = r
        .iter()
        .position(|x| matches!(x, Record::MotorWitness { label, .. } if label == "MOTOR1"))
        .unwrap();
    r[i] = witness_of("MOTOR1", "gpioPortA.pa8obs", "unwitnessed", 1000);
    let dead = |r: &[Record]| {
        let a = attribute(&archived(r)).unwrap();
        assert_eq!(
            (a.domain, a.rule),
            (Domain::Indeterminate, Rule::I4),
            "{a:?}"
        );
        // Only the failing record: MOTOR1's passing static check is not
        // evidence (second review, F6).
        assert_eq!(named(r, &a), ["kind 26"]);
        assert!(matches!(&r[a.evidence[0].index as usize],
                Record::MotorWitness { label, .. } if label == "MOTOR1"));
    };
    dead(&r);
    // MOTOR2 witnessed beside the wrong CCR2: still not MOTOR1's evidence.
    let alone = {
        let mut r = with_motor2(
            "witnessed",
            0x3e9,
            &[("register_mismatch", 1), (MOTOR_UNWITNESSED_FINDING, 1)],
        );
        r[i] = witness_of("MOTOR1", "gpioPortA.pa8obs", "unwitnessed", 1000);
        r
    };
    dead(&alone);
}

#[test]
fn adversarial_a_wrong_channel_register_cites_that_output_s_failing_witness_only() {
    // Adversarial pass, medium: CCR2 wrong, MOTOR2 a mismatch, MOTOR1
    // witnessed. The evidence is CCR2 and MOTOR2's witness, never MOTOR1's
    // passing one.
    let r = with_motor2(
        "mismatch",
        0x3e9,
        &[("register_mismatch", 1), (MOTOR_MISMATCH_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S4), "{a:?}");
    assert_eq!(named(&r, &a), ["TIM1.CCR2", "kind 26"]);
    let cited = a
        .evidence
        .iter()
        .find(|e| e.kind == 26)
        .map(|e| &r[e.index as usize]);
    assert!(
        matches!(cited, Some(Record::MotorWitness { label, .. }) if label == "MOTOR2"),
        "{cited:?}"
    );
    // CCR2 wrong and MOTOR2's witness passing: the register alone.
    let r = with_motor2("witnessed", 0x3e9, &[("register_mismatch", 1)]);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S4), "{a:?}");
    assert_eq!(named(&r, &a), ["TIM1.CCR2"]);
}

#[test]
fn a_failing_static_check_is_never_software_whatever_else_is_wrong() {
    // Second adversarial review, F1: the solver's CCR a tick short (a
    // failing static check, the topology's) beside an unrelated wrong bit
    // of the output's CR1 (URS) was attributed software, S4.
    let r = run(
        &Wrong {
            cr1: Some(0x85),
            ..Wrong::default()
        },
        "witnessed",
        false,
        &[("register_mismatch", 1), (MOTOR_STATIC_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I4),
        "{a:?}"
    );
    assert_eq!(named(&r, &a), ["kind 25"]);
}

/// `AET-WO-3201`, `AET-WO-4300a` Q7: the motor run made a DShot board's
/// (2 ms window) whose boot was read without its boot word, with U2's
/// identity record still as the judge filled it, and `findings` beside
/// `boot_word_missing`.
fn late_dshot(findings: &[(&str, u64)]) -> Vec<Record> {
    let mut all = vec![("boot_word_missing", 1)];
    all.extend(findings);
    let mut r = run(&Wrong::default(), "unwitnessed", true, &all);
    for x in &mut r {
        match x {
            Record::MotorOutputSolved {
                protocol,
                psc,
                arr,
                ccr,
                pwm,
                ..
            } => {
                (*protocol, *psc, *arr, *ccr, *pwm) = ("dshot600".into(), 0, 106, 40, None);
            }
            Record::MotorTimingCheck {
                period_ps,
                high_ps,
                nominal_period_ps,
                nominal_high_ps,
                ..
            } => {
                (*period_ps, *high_ps) = (1_671_875, 625_000);
                (*nominal_period_ps, *nominal_high_ps) = (1_666_666, 624_999);
            }
            // Trace 13 (`AET-WO-2302`): a timer that never started has no
            // anchor, and its witness runs to the boot's end.
            Record::MotorWitness {
                period_ns,
                high_ns,
                anchor_ns,
                ..
            } => (*period_ns, *high_ns, *anchor_ns) = (424, 160, None),
            Record::BootObserved {
                boot_word,
                instructions_expected,
                instructions_observed,
                ..
            } => {
                boot_word.observed = Some(0);
                (*instructions_expected, *instructions_observed) = (200_000, Some(200_000));
            }
            Record::I2cCheck {
                status, observed, ..
            } => (*status, *observed) = (Some("invalid".into()), Some(0xFFFF_FFFF)),
            _ => {}
        }
    }
    r
}

/// Q7: `identity_read_outside_window` is decided by the unset record, I7,
/// with that record as its evidence. Before, no rule selected it: `I1`.
#[test]
fn a_read_still_unset_when_the_short_window_ends_is_i7_by_its_record() {
    let r = late_dshot(&[("identity_read_outside_window", 1)]);
    let a = attributed(&r);
    let check = r
        .iter()
        .position(|x| matches!(x, Record::I2cCheck { .. }))
        .unwrap() as u64;
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I7),
        "{a:?}"
    );
    assert_eq!(
        a.evidence,
        [aethelgard_evolve::EventRef {
            index: check,
            kind: 12
        }]
    );
    // Proposes nothing (`INV-05`).
    assert!(Rule::I7.domain() == Domain::Indeterminate);
}

/// The selection is exact: the record must be the judge's fill, the boot
/// unbooted and the window short, or the finding is unexplained (`I1`).
#[test]
fn a_late_read_is_only_the_unset_record_of_an_unbooted_short_window() {
    let finding = [("identity_read_outside_window", 1)];
    // The record's value written: not the fill, so not the judge's unset
    // record, and the finding is unexplained.
    let mut r = late_dshot(&finding);
    for x in &mut r {
        if let Record::I2cCheck { observed, .. } = x {
            *observed = Some(0);
        }
    }
    let a = attribute(&archived(&r)).unwrap();
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    // The finding counted twice for one record.
    let r = late_dshot(&[("identity_read_outside_window", 2)]);
    assert!(attribute(&archived(&r)).is_err());
    // The unset record of a booted DShot run is the judge's
    // `i2c_record_invalid`, never this rule.
    let mut r = late_dshot(&[]);
    for x in &mut r {
        if let Record::BootObserved { boot_word, .. } = x {
            boot_word.observed = Some(boot_word.expected);
        }
    }
    let n = r.len() - 1;
    r[n] = Record::Verdict {
        pass: false,
        findings: vec![
            ("device_unmodelled".into(), "info".into(), 1),
            ("i2c_record_invalid".into(), "error".into(), 1),
            (MOTOR_UNWITNESSED_FINDING.into(), "error".into(), 1),
        ],
    };
    if let Record::Scenario {
        expect_findings, ..
    } = &mut r[0]
    {
        *expect_findings = vec![
            "i2c_record_invalid".into(),
            MOTOR_UNWITNESSED_FINDING.into(),
        ];
    }
    // The record is decided on its healthy bus (I4, its rises), beside the
    // dead output's I4: never `Window`.
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I4, "{a:?}");
    assert_eq!(
        a.evidence.iter().filter(|e| e.kind == 7).count(),
        2,
        "the record's own decision names its bus's rises: {a:?}"
    );
    // A boot word written after the window, with the finding beside it:
    // `replay` refuses the run.
    let n = r.len() - 1;
    r[n] = Record::Verdict {
        pass: false,
        findings: vec![
            ("device_unmodelled".into(), "info".into(), 1),
            ("identity_read_outside_window".into(), "error".into(), 1),
            (MOTOR_UNWITNESSED_FINDING.into(), "error".into(), 1),
        ],
    };
    assert!(attribute(&archived(&r)).is_err());
}

/// The same unset record on the PWM board's long window, unbooted, claimed
/// as invalid: the judge never judges a record before the boot word, so
/// `replay` refuses the verdict (adversarial F2), and it is never `I7`.
#[test]
fn an_unset_record_on_the_long_window_is_never_i7() {
    let mut r = run(
        &Wrong::default(),
        "unwitnessed",
        true,
        &[("boot_word_missing", 1), ("i2c_record_invalid", 1)],
    );
    for x in &mut r {
        match x {
            Record::BootObserved { boot_word, .. } => boot_word.observed = Some(0),
            Record::I2cCheck {
                status, observed, ..
            } => (*status, *observed) = (Some("invalid".into()), Some(0xFFFF_FFFF)),
            _ => {}
        }
    }
    assert!(matches!(
        attribute(&archived(&r)),
        Err(aethelgard_evolve::AttributeError::NotACampaignRun(
            "the verdict judges an identity read's record of a boot without its boot word"
        ))
    ));
}
