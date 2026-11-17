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
        edges_before: 0,
        edges_after: 0,
        cen_rises: 1,
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

/// An unset record, unbooted, claimed as invalid: the judge never judges a
/// record before the boot word, so `replay` refuses the verdict
/// (adversarial F2), and `attribute` never reads it.
#[test]
fn an_unset_record_claimed_invalid_before_the_boot_word_is_refused() {
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

// ---- AET-WO-2302b: the twin's window findings, attributed or refused ----

/// TIM2's solved registers in [`dshot`]: name, address, mask, value (`CR1`
/// with `CEN` set), as kind 11 lists them and the judge's stop (kind 30)
/// reads them back.
const TIM2_SOLVED: [(&str, u32, u32, u32); 10] = [
    ("TIM2.CR1", 0x4000_0000, 0xFF, 0x81),
    ("TIM2.CR2", 0x4000_0004, 0x80, 0),
    ("TIM2.SMCR", 0x4000_0008, 0x77, 0),
    ("TIM2.DIER", 0x4000_000C, 0x5F, 0),
    ("TIM2.CCMR1", 0x4000_0018, 0x7373, 0x60),
    ("TIM2.CCMR2", 0x4000_001C, 0x7373, 0),
    ("TIM2.CCER", 0x4000_0020, 0xBBBB, 0x1),
    ("TIM2.PSC", 0x4000_0028, u32::MAX, 0),
    ("TIM2.ARR", 0x4000_002C, u32::MAX, 106),
    ("TIM2.CCR1", 0x4000_0034, u32::MAX, 40),
];

/// `M2`'s anchor in [`dshot`]: its window, 2 ms from it, inside the run.
const ANCHOR: u64 = 30_670;

/// `M2`'s witness, DShot600 on the model's sequence from `anchor`: its first
/// rise one period after it, a rise every period to its window's end.
fn dshot_witness(anchor: Option<u64>) -> Record {
    let rises = 4716;
    Record::MotorWitness {
        label: "M2".into(),
        observer: "gpioPortA.pa0obs".into(),
        period_ns: 424,
        high_ns: 160,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: if anchor.is_some() {
            "witnessed"
        } else {
            "unanchored"
        }
        .into(),
        rises,
        falls: rises,
        first_ns: anchor.map(|a| a + 490),
        last_ns: anchor.map(|a| a + 490 + (rises - 1) * 424),
        end_ns: anchor.map_or(100_000_000, |a| a + 2_000_000),
        span_ns: anchor.map(|_| 8),
        off_grid: 2 * rises - 1,
        batched: 0,
        anchor_ns: anchor,
        edges_before: 0,
        edges_after: 0,
        cen_rises: u64::from(anchor.is_some()),
    }
}

/// [`run`]'s PWM output `MOTOR1` (TIM1, anchored at 6 ns, as `wrong` and
/// `status` say) with a DShot output beside it, `M2` (TIM2_CH1 on PA0 at
/// AF1): its solved output and static check, TIM2's solved registers
/// (kind 11), its witness anchored at [`ANCHOR`], and the judge's stop at
/// its window's end that read every register back and left `CEN` cleared
/// (kind 30). The verdict's findings as given; `edit` then changes the
/// records ([`set_dshot_witness`], [`set_stop`], [`stop_left_set`]).
fn dshot_with(
    wrong: &Wrong,
    status: &str,
    findings: &[(&str, u64)],
    edit: &dyn Fn(&mut Vec<Record>),
) -> Vec<Record> {
    let mut r = run(wrong, status, true, findings);
    let after = |r: &[Record], f: &dyn Fn(&Record) -> bool| r.iter().rposition(f).unwrap() + 1;
    let i = after(&r, &|x| matches!(x, Record::MotorOutputSolved { .. }));
    r.insert(
        i,
        Record::MotorOutputSolved {
            label: "M2".into(),
            timer: "TIM2".into(),
            channel: 1,
            pin: "PA0".into(),
            alternate_function: 1,
            protocol: "dshot600".into(),
            psc: 0,
            arr: 106,
            ccr: 40,
            pwm: None,
        },
    );
    let i = after(&r, &|x| matches!(x, Record::MotorTimingCheck { .. }));
    r.insert(
        i,
        Record::MotorTimingCheck {
            label: "M2".into(),
            kernel_hz: 64_000_000,
            period_ps: 1_671_875,
            high_ps: 625_000,
            nominal_period_ps: 1_666_666,
            nominal_high_ps: 624_999,
            holds: true,
        },
    );
    // Kind 11's `CR1` expects `CEN` cleared after a stop that cleared it,
    // the judge's own act.
    let i = after(&r, &|x| matches!(x, Record::RegisterObserved { .. }));
    for (n, &(name, address, mask, value)) in TIM2_SOLVED.iter().enumerate() {
        let value = if name == "TIM2.CR1" {
            value & !1
        } else {
            value
        };
        r.insert(i + n, register(name, address, mask, value, value));
    }
    let i = after(&r, &|x| matches!(x, Record::MotorWitness { .. }));
    r.insert(i, dshot_witness(Some(ANCHOR)));
    r.insert(
        i + 1,
        Record::JudgeStop {
            timer: "TIM2".into(),
            anchor_ns: ANCHOR,
            at_ns: ANCHOR + 2_000_000,
            read_back: TIM2_SOLVED
                .iter()
                .map(|&(_, address, mask, value)| Reading {
                    address,
                    mask,
                    expected: value,
                    observed: Some(value),
                })
                .collect(),
            cr1_after: Some(0x80),
        },
    );
    edit(&mut r);
    r
}

/// [`dshot_with`] beside a witnessed `MOTOR1` with every register right.
fn dshot(findings: &[(&str, u64)], edit: &dyn Fn(&mut Vec<Record>)) -> Vec<Record> {
    dshot_with(&Wrong::default(), "witnessed", findings, edit)
}

fn set_dshot_witness(r: &mut [Record], f: impl Fn(&mut Record)) {
    let i = r
        .iter()
        .position(|x| matches!(x, Record::MotorWitness { label, .. } if label == "M2"))
        .unwrap();
    f(&mut r[i]);
}

fn set_stop(r: &mut [Record], f: impl Fn(&mut Vec<Reading>, &mut Option<u32>)) {
    for x in r.iter_mut() {
        if let Record::JudgeStop {
            read_back,
            cr1_after,
            ..
        } = x
        {
            f(read_back, cr1_after);
        }
    }
}

/// The stop ran but left `CEN` set: kind 11's `CR1` then expects the solved
/// value, as the judge does.
fn stop_left_set(r: &mut Vec<Record>) {
    set_stop(r, |_, cr1| *cr1 = Some(0x81));
    tim2_cr1_expected(r, 0x81);
}

/// No stop at all (`judge_stop_missing`).
fn no_stop(r: &mut Vec<Record>) {
    r.retain(|x| !matches!(x, Record::JudgeStop { .. }));
    tim2_cr1_expected(r, 0x81);
}

fn tim2_cr1_expected(r: &mut [Record], value: u32) {
    for x in r.iter_mut() {
        if let Record::RegisterObserved { name, reading } = x
            && name == "TIM2.CR1"
        {
            (reading.expected, reading.observed) = (value, Some(value));
        }
    }
}

/// `M2` anchored at `anchor` 99.07 ms, its window 2 ms past the run's end
/// (`AET-WO-2302` F2): its edges to the run's end, `rises` of them, no stop
/// (none is due past the run), `stopped`.
fn late(rises: u64) -> impl Fn(&mut Vec<Record>) {
    move |r: &mut Vec<Record>| {
        let late = 99_072_350;
        no_stop(r);
        set_dshot_witness(r, |w| {
            *w = dshot_witness(Some(late));
            if let Record::MotorWitness {
                status,
                rises: n,
                falls,
                last_ns,
                off_grid,
                ..
            } = w
            {
                (*status, *n, *falls, *off_grid) = ("stopped".into(), rises, rises, 2 * rises - 1);
                *last_ns = Some(late + 490 + (rises - 1) * 424);
            }
        });
    }
}

/// The attribution of a run `check_run` accepts, whatever it leaves
/// unexplained, with every evidence kind one its rule allows.
fn attributed_any(records: &[Record]) -> Attribution {
    campaign::check_run(records).expect("the run is one check_run accepts");
    let a = attribute(&archived(records)).unwrap();
    for e in &a.evidence {
        assert!(a.rule.evidence_kinds().contains(&e.kind), "{a:?}");
    }
    a
}

fn index_of(r: &[Record], f: impl Fn(&Record) -> bool) -> u64 {
    r.iter().position(f).unwrap() as u64
}

fn m2_witness(r: &[Record]) -> u64 {
    index_of(
        r,
        |x| matches!(x, Record::MotorWitness { label, .. } if label == "M2"),
    )
}

fn m2_solved(r: &[Record]) -> u64 {
    index_of(
        r,
        |x| matches!(x, Record::MotorOutputSolved { label, .. } if label == "M2"),
    )
}

fn motor1_witness(r: &[Record]) -> u64 {
    index_of(
        r,
        |x| matches!(x, Record::MotorWitness { label, .. } if label == "MOTOR1"),
    )
}

fn stop_at(r: &[Record]) -> u64 {
    index_of(r, |x| matches!(x, Record::JudgeStop { .. }))
}

fn boot_at(r: &[Record]) -> u64 {
    index_of(r, |x| matches!(x, Record::BootObserved { .. }))
}

fn indices(a: &[aethelgard_evolve::EventRef]) -> Vec<u64> {
    a.iter().map(|e| e.index).collect()
}

/// The base: a DShot output inside its window, stopped by the judge, beside
/// the PWM output, passes.
#[test]
fn a_dshot_output_witnessed_inside_its_window_passes_none() {
    let a = attributed(&dshot(&[], &|_| {}));
    assert_eq!((a.domain, a.rule), (Domain::None, Rule::N1));
}

/// `S6`: an output with edges and no `CEN` rise is the firmware's, by its
/// witness alone.
#[test]
fn s6_edges_with_no_anchor_are_software_by_the_witness() {
    let r = dshot(&[(campaign::MOTOR_ANCHOR_UNOBSERVED_FINDING, 1)], &|r| {
        no_stop(r);
        set_dshot_witness(r, |w| *w = dshot_witness(None));
    });
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S6), "{a:?}");
    assert_eq!(indices(&a.evidence), [m2_witness(&r)]);
    assert_eq!(indices(&a.symptoms), [m2_witness(&r)]);
}

/// `S6` never fires on an output with no edges: no anchor and nothing on
/// the pin is `motor_output_unwitnessed`, decided by the output's registers
/// (here none wrong, `I4`).
#[test]
fn s6_never_fires_on_an_output_with_no_edges() {
    let r = run(
        &Wrong::default(),
        "unwitnessed",
        true,
        &[(MOTOR_UNWITNESSED_FINDING, 1)],
    );
    let r: Vec<Record> = r
        .into_iter()
        .map(|mut x| {
            if let Record::MotorWitness {
                anchor_ns,
                cen_rises,
                ..
            } = &mut x
            {
                (*anchor_ns, *cen_rises) = (None, 0);
            }
            x
        })
        .collect();
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I4, "{a:?}");
}

/// `S7`: a timer whose `CEN` rose twice, by its witness; the other timer's
/// single rise is never cited.
#[test]
fn s7_a_timer_started_twice_is_software_by_its_own_witness_alone() {
    let r = dshot(&[(campaign::MOTOR_ANCHOR_REPEATED_FINDING, 1)], &|r| {
        set_dshot_witness(r, |w| {
            if let Record::MotorWitness { cen_rises, .. } = w {
                *cen_rises = 2;
            }
        });
    });
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S7), "{a:?}");
    assert_eq!(indices(&a.evidence), [m2_witness(&r)]);
    assert!(!indices(&a.evidence).contains(&motor1_witness(&r)));
}

/// `S8`: both anchored timers named out of order: the records say which
/// (every anchored timer), so each is the firmware's, with the boot word
/// set (kind 10). Named once of two, with no boot word's instant in the
/// records (no serial port), they cannot say which: unexplained by name,
/// never a guess.
#[test]
fn s8_out_of_order_is_software_only_where_the_records_say_which_timer() {
    let r = dshot(&[(campaign::MOTOR_ANCHOR_ORDER_FINDING, 2)], &|_| {});
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S8), "{a:?}");
    assert_eq!(
        indices(&a.evidence),
        [boot_at(&r), motor1_witness(&r), m2_witness(&r)]
    );
    let r = dshot(&[(campaign::MOTOR_ANCHOR_ORDER_FINDING, 1)], &|_| {});
    let a = attributed_any(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I1));
    assert_eq!(a.unexplained.len(), 1, "{a:?}");
    assert!(
        a.unexplained[0].starts_with(
            "motor_anchor_out_of_order: named 1 time(s) of 2 anchored timer(s), 0 anchored after the boot word's instant"
        ),
        "{a:?}"
    );
}

/// `S9`: edges before the anchor, by the output's witness.
#[test]
fn s9_edges_before_the_anchor_are_software_by_the_witness() {
    let r = dshot(&[(campaign::MOTOR_EDGE_BEFORE_FINDING, 1)], &|r| {
        set_dshot_witness(r, |w| {
            if let Record::MotorWitness { edges_before, .. } = w {
                *edges_before = 3;
            }
        });
    });
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S9), "{a:?}");
    assert_eq!(indices(&a.evidence), [m2_witness(&r)]);
}

/// `S10`: edges after the window the judge's stop closed (`CEN` cleared,
/// kind 30), by the witness and the stop. Not when the stop left `CEN` set:
/// the timer may still be running, so the kind is unexplained by name, and
/// `judge_stop_missing` beside it is the judge's (`H2`), never the run's
/// rule while anything is unexplained.
#[test]
fn s10_edges_after_the_window_are_software_only_once_the_stop_cleared_cen() {
    let after = |r: &mut Vec<Record>| {
        set_dshot_witness(r, |w| {
            if let Record::MotorWitness { edges_after, .. } = w {
                *edges_after = 5;
            }
        });
    };
    let r = dshot(&[(campaign::MOTOR_EDGE_AFTER_FINDING, 1)], &after);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S10), "{a:?}");
    assert_eq!(indices(&a.evidence), [m2_witness(&r), stop_at(&r)]);
    let r = dshot(
        &[
            (campaign::JUDGE_STOP_MISSING_FINDING, 1),
            (campaign::MOTOR_EDGE_AFTER_FINDING, 1),
        ],
        &|r| {
            after(r);
            stop_left_set(r);
        },
    );
    let a = attributed_any(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I1));
    assert_eq!(
        a.unexplained,
        [
            "motor_edge_after_window: M2: no judge's stop of its timer at its anchor left CEN cleared (kind 30), so its edges after the window may be the timer still running"
        ]
    );
    assert!(a.evidence.is_empty());
}

/// `S10` on a PWM output: its window is the boot's, and no judge's stop
/// closes it, so edges after it are unexplained by name.
#[test]
fn s10_never_fires_with_no_stop_to_cite() {
    let r = dshot(&[(campaign::MOTOR_EDGE_AFTER_FINDING, 1)], &|r| {
        for x in r.iter_mut() {
            if let Record::MotorWitness {
                label, edges_after, ..
            } = x
                && label == "MOTOR1"
            {
                *edges_after = 2;
            }
        }
    });
    let a = attributed_any(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    assert!(
        a.unexplained[0].starts_with("motor_edge_after_window: MOTOR1: no judge's stop"),
        "{a:?}"
    );
}

/// `S11`: the stop read a solved register back off its value (the firmware
/// wrote `CCR1` inside the window), by the stop; `propose` answers with a
/// generator report naming that register. A register the stop did not
/// read back shows no write: unexplained by name, never software.
#[test]
fn s11_a_register_read_back_off_its_value_is_software_by_the_stop() {
    let r = dshot(&[(campaign::JUDGE_STOP_MISMATCH_FINDING, 1)], &|r| {
        set_stop(r, |g, _| g[9].observed = Some(41));
    });
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S11), "{a:?}");
    assert_eq!(indices(&a.evidence), [stop_at(&r)]);
    let Outcome::GeneratorReport(report) = &proposed(&r, &a) else {
        panic!("{:?}", proposed(&r, &a))
    };
    let what: Vec<(&str, u64, u64, Option<u64>)> = report
        .expectations
        .iter()
        .map(|e| (e.what.as_str(), e.mask.unwrap(), e.solved, e.observed))
        .collect();
    assert_eq!(what, [("TIM2.CCR1", u64::from(u32::MAX), 40, Some(41))]);
    let r = dshot(&[(campaign::JUDGE_STOP_MISMATCH_FINDING, 1)], &|r| {
        set_stop(r, |g, _| g[9].observed = None);
    });
    let a = attributed_any(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    assert_eq!(
        a.unexplained,
        [
            "judge_stop_mismatch: TIM2: its stop read back fewer registers than the timer's (kind 30's read_back), and an unread register shows no write of the firmware's"
        ]
    );
}

/// `H1`: the DShot window past the run is the twin's bound, and the
/// output's `stopped` witness, whose last rise reaches the run's end, is
/// the same bound's: the run is refused as the harness's, never software.
/// A witness that stopped well before the run's end is the output's own
/// (`I4` here, its registers right), and the run takes it.
#[test]
fn h1_a_window_past_the_run_is_the_harness_s_with_its_cut_witness() {
    let findings = [
        (MOTOR_MISMATCH_FINDING, 1),
        (campaign::MOTOR_WINDOW_PAST_RUN_FINDING, 1),
    ];
    let r = dshot(&findings, &late(2187));
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::H1),
        "{a:?}"
    );
    assert_eq!(indices(&a.evidence), [m2_solved(&r), m2_witness(&r)]);
    assert_eq!(
        Rule::H1.bound(),
        Some(
            "the DShot window's end, its anchor plus DSHOT_WINDOW_NS, past the run's MAX_VIRTUAL_NS"
        )
    );
    // Stopped 0.9 ms before the run's end: not the bound's.
    let r = dshot(&findings, &late(100));
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I4),
        "{a:?}"
    );
    assert_eq!(indices(&a.evidence), [m2_witness(&r)]);
}

/// `H2`: the judge's stop that did not run, or left `CEN` set, is the
/// judge's own act: refused as the harness's, with the stop when there is
/// one.
#[test]
fn h2_a_missing_stop_is_the_harness_s() {
    let missing = [(campaign::JUDGE_STOP_MISSING_FINDING, 1)];
    let r = dshot(&missing, &no_stop);
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::H2),
        "{a:?}"
    );
    assert_eq!(indices(&a.evidence), [m2_solved(&r), m2_witness(&r)]);
    let r = dshot(&missing, &stop_left_set);
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::H2, "{a:?}");
    assert_eq!(
        indices(&a.evidence),
        [m2_solved(&r), m2_witness(&r), stop_at(&r)]
    );
}

/// A harness finding beside a fault of the device's never masks it, and is
/// never attributed to it: the run takes the fault's rule, and its
/// evidence is the fault's alone.
#[test]
fn a_harness_finding_never_masks_a_fault_nor_joins_its_evidence() {
    // Beside the dead PWM output (`S4`, its AF wrong).
    let r = dshot_with(
        &Wrong {
            afrh: Some(0x2),
            ..Wrong::default()
        },
        "unwitnessed",
        &[
            ("register_mismatch", 1),
            (campaign::JUDGE_STOP_MISSING_FINDING, 1),
            (MOTOR_UNWITNESSED_FINDING, 1),
        ],
        &no_stop,
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S4), "{a:?}");
    assert_eq!(named(&r, &a), ["GPIOA.AFRH", "kind 26"]);
    assert!(!indices(&a.evidence).contains(&m2_witness(&r)));
    assert!(indices(&a.symptoms).contains(&m2_witness(&r)));
    // Beside a faulted bus (`E1`): electrical, never `I5`.
    let mut r = dshot(
        &[
            (campaign::PAST_CEILING_FINDING, 2),
            (MOTOR_MISMATCH_FINDING, 1),
            (campaign::MOTOR_WINDOW_PAST_RUN_FINDING, 1),
        ],
        &late(2187),
    );
    fault_sensors(&mut r);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E1), "{a:?}");
    assert!(a.evidence.iter().all(|e| [7, 9].contains(&e.kind)), "{a:?}");
    // Beside a dead output with no evidence (`I4`): the run is that
    // indeterminate symptom's, never the harness's.
    let r = dshot_with(
        &Wrong::default(),
        "unwitnessed",
        &[
            (MOTOR_UNWITNESSED_FINDING, 1),
            (campaign::JUDGE_STOP_MISSING_FINDING, 1),
        ],
        &no_stop,
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I4, "{a:?}");
    assert_eq!(indices(&a.evidence), [motor1_witness(&r)]);
}

/// A new software rule beside a faulted bus is split between the halves
/// (`I5`), never averaged; beside `S4`, the lower rule names the run.
#[test]
fn a_window_software_rule_keeps_the_existing_precedence() {
    let edges = |r: &mut Vec<Record>| {
        set_dshot_witness(r, |w| {
            if let Record::MotorWitness { edges_before, .. } = w {
                *edges_before = 3;
            }
        });
    };
    let mut r = dshot(
        &[
            (campaign::PAST_CEILING_FINDING, 2),
            (campaign::MOTOR_EDGE_BEFORE_FINDING, 1),
        ],
        &edges,
    );
    fault_sensors(&mut r);
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I5),
        "{a:?}"
    );
    assert!(indices(&a.evidence).contains(&m2_witness(&r)), "{a:?}");
    assert!(a.evidence.iter().any(|e| e.kind == 9), "{a:?}");
    let r = dshot_with(
        &Wrong {
            afrh: Some(0x2),
            ..Wrong::default()
        },
        "unwitnessed",
        &[
            ("register_mismatch", 1),
            (campaign::MOTOR_EDGE_BEFORE_FINDING, 1),
            (MOTOR_UNWITNESSED_FINDING, 1),
        ],
        &edges,
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S4, "{a:?}");
    assert_eq!(named(&r, &a), ["GPIOA.AFRH", "kind 26"]);
}

/// `propose` proposes nothing for a harness rule, naming its bound; and a
/// window software rule with no solved expectation unmet in its evidence
/// gives no generator report.
#[test]
fn propose_answers_a_harness_rule_with_its_bound_and_nothing_else() {
    let r = dshot(&[(campaign::JUDGE_STOP_MISSING_FINDING, 1)], &no_stop);
    let a = attributed(&r);
    let Outcome::NoProposal { reason, .. } = proposed(&r, &a) else {
        panic!()
    };
    assert_eq!(
        reason,
        "attribution rule H2 (`indeterminate`): the judge's stop at its anchor plus DSHOT_WINDOW_NS is the twin's own bound, never the device's, so nothing is proposed (INV-05)"
    );
    let r = dshot(&[(campaign::MOTOR_EDGE_BEFORE_FINDING, 1)], &|r| {
        set_dshot_witness(r, |w| {
            if let Record::MotorWitness { edges_before, .. } = w {
                *edges_before = 3;
            }
        });
    });
    let a = attributed(&r);
    let Outcome::NoProposal { reason, .. } = proposed(&r, &a) else {
        panic!()
    };
    assert_eq!(
        reason,
        "no evidence event shows an unmet solved expectation, so no generator report is made"
    );
}

/// The past-ceiling fault of the reference bus: both lines' rises past
/// their ceiling (kind 7) and the bus fault they cause (kind 9).
fn fault_sensors(r: &mut Vec<Record>) {
    for x in r.iter_mut() {
        if let Record::RiseComputed {
            rise_30_70_ps,
            exceeds,
            ..
        } = x
        {
            (*rise_30_70_ps, *exceeds) = (3_982_301, true);
        }
    }
    let i = r
        .iter()
        .rposition(|x| matches!(x, Record::RiseComputed { .. }))
        .unwrap();
    r.insert(
        i + 1,
        Record::BusFault {
            bus: "SENSORS".into(),
            fault: "absent".into(),
            source: "aethelgard".into(),
            from: vec!["analog.rise_time_computed > ceiling_ns".into()],
            nets: vec!["I2C_SCL".into(), "I2C_SDA".into()],
            firmware_sees: "absent".into(),
        },
    );
}

fn proposed(r: &[Record], a: &Attribution) -> Outcome {
    let archived = archived(r);
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
        attribution: a,
        evidence: &evidence,
        buses: &buses,
        facts: &facts,
    };
    propose::propose(&[&Rules], &input).outcome
}
