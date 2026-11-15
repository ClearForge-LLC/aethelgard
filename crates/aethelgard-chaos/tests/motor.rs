//! `AET-WO-4300a` §1.4: kinds 24 to 26 (a solved motor output, its static
//! check for silicon, its twin witness) through the archive, their field
//! rules, and `check_run`'s joins and its re-derivation of the verdict's
//! motor findings from them.

mod common;

use aethelgard_chaos::campaign::{
    MOTOR_MISMATCH_FINDING, MOTOR_STATIC_FINDING, MOTOR_STATIC_ONLY_FINDING,
    MOTOR_UNWITNESSED_FINDING, check_run,
};
use aethelgard_chaos::record::{Reading, Record};
use aethelgard_chaos::{Event, replay_dir};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

fn solved(label: &str, pin: &str, channel: u8) -> Record {
    Record::MotorOutputSolved {
        label: label.into(),
        timer: "TIM1".into(),
        channel,
        pin: pin.into(),
        alternate_function: 1,
        protocol: "pwm".into(),
        psc: 63,
        arr: 2499,
        ccr: 1000,
        pwm: Some((400, 1000, 2000)),
    }
}

fn check(label: &str, holds: bool) -> Record {
    Record::MotorTimingCheck {
        label: label.into(),
        kernel_hz: 64_000_000,
        period_ps: 2_500_000_000,
        high_ps: 1_000_000_000,
        nominal_period_ps: 2_500_000_000,
        nominal_high_ps: 1_000_000_000,
        holds,
    }
}

fn witness(label: &str, observer: &str, status: &str) -> Record {
    Record::MotorWitness {
        label: label.into(),
        observer: observer.into(),
        period_ns: 639_744,
        high_ns: 256_000,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: status.into(),
        rises: 156,
        falls: 155,
        first_ns: Some(639_750),
        span_ns: Some(8),
        off_grid: 311,
        batched: 0,
    }
}

fn boot(word: Option<u32>) -> Record {
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: Reading {
            address: 0x2000_0000,
            mask: u32::MAX,
            expected: BOOT_WORD,
            observed: word,
        },
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(0x0800_0100),
        executable: vec![(0x0800_0000, 0x0801_0000)],
    }
}

fn scenario(expect_pass: bool, findings: &[&str]) -> Record {
    Record::Scenario {
        name: "motors".into(),
        sha256: [1; 32],
        expect_pass,
        expect_findings: findings.iter().map(|s| (*s).to_owned()).collect(),
        overrides: vec![],
    }
}

/// A run of two outputs: `M1` witnessed, `M2` with the given status and
/// static check, booted or not, and the verdict's findings as given.
fn run(
    m2_status: &str,
    m2_holds: bool,
    booted: bool,
    findings: Vec<(&str, &str, u64)>,
) -> Vec<Record> {
    let pass = findings.iter().all(|f| f.1 == "info");
    let failing: Vec<&str> = findings
        .iter()
        .filter(|f| f.1 != "info")
        .map(|f| f.0)
        .collect();
    vec![
        scenario(pass, &failing),
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        solved("M1", "PA8", 1),
        solved("M2", "PA9", 2),
        check("M1", true),
        check("M2", m2_holds),
        boot(Some(if booted { BOOT_WORD } else { 0 })),
        witness("M1", "gpioPortA.pa8obs", "witnessed"),
        witness("M2", "gpioPortA.pa9obs", m2_status),
        Record::Verdict {
            pass,
            findings: findings
                .into_iter()
                .map(|(k, s, n)| (k.to_owned(), s.to_owned(), n))
                .collect(),
        },
    ]
}

#[test]
fn the_motor_records_round_trip_through_the_archive() {
    let records = run("witnessed", true, true, vec![]);
    let events: Vec<Event> = records
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("motor-roundtrip", 7, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, records);
    let kinds: Vec<u64> = records[2..6].iter().map(Record::kind).collect();
    assert_eq!(kinds, [24, 24, 25, 25]);
    assert_eq!(records[7].kind(), 26);
}

#[test]
fn a_witnessed_run_passes_and_each_status_needs_its_own_finding() {
    let summary = check_run(&run("witnessed", true, true, vec![])).unwrap();
    assert!(summary.pass && summary.matched);
    for (status, kind) in [
        ("unwitnessed", MOTOR_UNWITNESSED_FINDING),
        ("unreadable", MOTOR_UNWITNESSED_FINDING),
        ("mismatch", MOTOR_MISMATCH_FINDING),
        ("batched", MOTOR_MISMATCH_FINDING),
    ] {
        let ok = run(status, true, true, vec![(kind, "error", 1)]);
        assert!(check_run(&ok).unwrap().matched, "{status}");
        // The verdict that drops it, names it twice, or names another kind
        // is refused: the records explain the findings exactly.
        for wrong in [
            vec![],
            vec![(kind, "error", 2)],
            vec![(MOTOR_STATIC_FINDING, "error", 1)],
        ] {
            assert!(
                check_run(&run(status, true, true, wrong)).is_err(),
                "{status}"
            );
        }
    }
    let info = run(
        "static_only",
        true,
        true,
        vec![(MOTOR_STATIC_ONLY_FINDING, "info", 1)],
    );
    assert!(check_run(&info).unwrap().pass);
    // As an error it is the wrong severity.
    let as_error = run(
        "static_only",
        true,
        true,
        vec![(MOTOR_STATIC_ONLY_FINDING, "error", 1)],
    );
    assert!(check_run(&as_error).is_err());
}

#[test]
fn a_witness_is_judged_only_once_the_boot_word_is_set() {
    // The firmware never reached its fixed point: no witness finding is due,
    // so one named is refused, and none named is the records' account.
    let not_booted = run(
        "unwitnessed",
        true,
        false,
        vec![("boot_word_missing", "error", 1)],
    );
    assert!(check_run(&not_booted).is_ok());
    let named = run(
        "unwitnessed",
        true,
        false,
        vec![
            ("boot_word_missing", "error", 1),
            (MOTOR_UNWITNESSED_FINDING, "error", 1),
        ],
    );
    assert!(check_run(&named).is_err());
}

#[test]
fn a_static_mismatch_is_named_once_per_failing_check_booted_or_not() {
    for booted in [true, false] {
        let mut findings = vec![(MOTOR_STATIC_FINDING, "error", 1)];
        if !booted {
            findings.push(("boot_word_missing", "error", 1));
        }
        assert!(check_run(&run("witnessed", false, booted, findings)).is_ok());
        assert!(check_run(&run("witnessed", false, booted, vec![])).is_err());
    }
}

#[test]
fn a_motor_record_that_joins_no_solved_output_is_refused() {
    let base = run("witnessed", true, true, vec![]);
    let edit = |f: &dyn Fn(&mut Vec<Record>)| {
        let mut r = base.clone();
        f(&mut r);
        check_run(&r).map(|_| ())
    };
    assert_eq!(edit(&|_| {}), Ok(()));
    // A witness on another pin's observer.
    assert!(edit(&|r| r[8] = witness("M2", "gpioPortA.pa8obs", "witnessed")).is_err());
    // A witness or a check of an output nothing solved.
    assert!(edit(&|r| r[8] = witness("M9", "gpioPortA.pa9obs", "witnessed")).is_err());
    assert!(edit(&|r| r[5] = check("M9", true)).is_err());
    // A solved output with no static check.
    assert!(
        edit(&|r| {
            r.remove(5);
        })
        .is_err()
    );
    // Two witnesses of one output.
    assert!(edit(&|r| r[8] = witness("M1", "gpioPortA.pa8obs", "witnessed")).is_err());
    // A witness out of its rank, after the verdict.
    assert!(
        edit(&|r| {
            let w = r.remove(8);
            r.push(w);
        })
        .is_err()
    );
}

#[test]
fn the_field_rules_hold_on_write_and_read() {
    let bad = [
        Record::MotorOutputSolved {
            label: "M1".into(),
            timer: "TIM1".into(),
            channel: 5,
            pin: "PA8".into(),
            alternate_function: 1,
            protocol: "pwm".into(),
            psc: 63,
            arr: 2499,
            ccr: 1000,
            pwm: Some((400, 1000, 2000)),
        },
        Record::MotorOutputSolved {
            label: "M1".into(),
            timer: "TIM1".into(),
            channel: 1,
            pin: "PA8".into(),
            alternate_function: 1,
            protocol: "dshot600".into(),
            psc: 0,
            arr: 106,
            ccr: 40,
            pwm: Some((400, 1000, 2000)),
        },
        Record::MotorOutputSolved {
            label: "M1".into(),
            timer: "TIM1".into(),
            channel: 1,
            pin: "PA8".into(),
            alternate_function: 1,
            protocol: "pwm".into(),
            psc: 63,
            arr: 2499,
            ccr: 2499,
            pwm: Some((400, 1000, 2000)),
        },
        Record::MotorTimingCheck {
            label: "M1".into(),
            kernel_hz: 0,
            period_ps: 1,
            high_ps: 1,
            nominal_period_ps: 1,
            nominal_high_ps: 1,
            holds: true,
        },
        witness("M1", "gpioPortA.pa8obs", "fine"),
    ];
    for r in bad {
        assert!(r.check().is_err(), "{r:?}");
    }
    assert!(solved("M1", "PA8", 1).check().is_ok());
    assert!(witness("M1", "gpioPortA.pa8obs", "batched").check().is_ok());
}
