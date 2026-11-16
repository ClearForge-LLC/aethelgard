//! `AET-WO-4300a` §1.4: kinds 24 to 26 (a solved motor output, its static
//! check for silicon, its twin witness) through the archive, their field
//! rules, and `check_run`'s joins and its re-derivation of the verdict's
//! motor findings from them. `AET-WO-2302`: each witness's window derived
//! from its anchor, and kind 30, the judge's stop.

mod common;

use aethelgard_chaos::campaign::{
    JUDGE_STOP_MISSING_FINDING, MOTOR_ANCHOR_UNOBSERVED_FINDING, MOTOR_MISMATCH_FINDING,
    MOTOR_STATIC_FINDING, MOTOR_STATIC_ONLY_FINDING, MOTOR_UNWITNESSED_FINDING,
    MOTOR_WINDOW_PAST_RUN_FINDING, check_run,
};
use aethelgard_chaos::record::{Reading, Record};
use aethelgard_chaos::{Event, replay_dir};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

/// The CCR of an output whose static check holds (the 1000 µs idle pulse at
/// a 1 µs tick), and of one whose check fails (a tick short).
const CCR_HOLDS: u32 = 1000;
const CCR_FAILS: u32 = 999;

fn solved(label: &str, pin: &str, channel: u8) -> Record {
    solved_ccr(label, pin, channel, CCR_HOLDS)
}

fn solved_ccr(label: &str, pin: &str, channel: u8, ccr: u32) -> Record {
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

/// The static check of an output solved at [`CCR_HOLDS`], or at
/// [`CCR_FAILS`] when it does not hold: its figures are what replay
/// re-derives from kind 24.
fn check(label: &str, holds: bool) -> Record {
    let ccr = u64::from(if holds { CCR_HOLDS } else { CCR_FAILS });
    Record::MotorTimingCheck {
        label: label.into(),
        kernel_hz: 64_000_000,
        period_ps: 2_500_000_000,
        high_ps: ccr * 1_000_000,
        nominal_period_ps: 2_500_000_000,
        nominal_high_ps: 1_000_000_000,
        holds,
    }
}

/// The PWM timer's anchor, its first `CEN` rise, one period before the first
/// rise (`AET-WO-2302`): every output of the timer names it.
const PWM_ANCHOR: u64 = 6;

/// A witness with counts the status allows: a full window of edges on the
/// model's sequence for `witnessed`, and each other status's own departure.
fn witness(label: &str, observer: &str, status: &str) -> Record {
    witness_ccr(label, observer, status, CCR_HOLDS)
}

fn witness_ccr(label: &str, observer: &str, status: &str, ccr: u32) -> Record {
    let (mut rises, mut falls, mut first_ns, mut last_ns, mut span_ns) =
        (156, 155, Some(639_750), Some(99_800_070), Some(8));
    let (mut off_grid, mut batched) = (311, 0);
    match status {
        "static_only" => off_grid = 0,
        "unwitnessed" | "unreadable" => {
            (rises, falls, first_ns, last_ns, span_ns, off_grid) = (0, 0, None, None, None, 0);
        }
        "mismatch" => span_ns = Some(400),
        "batched" => batched = 3,
        "stopped" => last_ns = Some(50_000_000),
        "unanchored" => (first_ns, last_ns, span_ns) = (None, None, None),
        _ => {}
    }
    Record::MotorWitness {
        label: label.into(),
        observer: observer.into(),
        period_ns: 639_744,
        high_ns: u64::from(ccr) * 256,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: status.into(),
        rises,
        falls,
        first_ns,
        last_ns,
        end_ns: 100_000_000,
        span_ns,
        off_grid,
        batched,
        anchor_ns: (status != "unanchored").then_some(PWM_ANCHOR),
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
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
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
    let m2_ccr = if m2_holds { CCR_HOLDS } else { CCR_FAILS };
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
        solved_ccr("M2", "PA9", 2, m2_ccr),
        check("M1", true),
        check("M2", m2_holds),
        boot(Some(if booted { BOOT_WORD } else { 0 })),
        witness("M1", "gpioPortA.pa8obs", "witnessed"),
        witness_ccr("M2", "gpioPortA.pa9obs", m2_status, m2_ccr),
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
        ("stopped", MOTOR_MISMATCH_FINDING),
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

#[test]
fn replay_re_derives_the_static_check_and_the_witness_status() {
    // Adversarial pass: replay took kinds 25 and 26 at their word, so a
    // forged `holds` or `status` beside its own failing figures passed.
    let base = run("witnessed", true, true, vec![]);
    let edit = |f: &dyn Fn(&mut Vec<Record>)| {
        let mut r = base.clone();
        f(&mut r);
        check_run(&r).map(|_| ())
    };
    assert_eq!(edit(&|_| {}), Ok(()));
    // A check that claims to hold beside its own figures a tick short.
    let mut short = run("witnessed", false, true, vec![]);
    if let Record::MotorTimingCheck { holds, .. } = &mut short[5] {
        *holds = true;
    }
    assert!(check_run(&short).is_err());
    // A check whose figures are not its output's.
    assert!(
        edit(
            &|r| if let Record::MotorTimingCheck { high_ps, .. } = &mut r[5] {
                *high_ps += 1;
            }
        )
        .is_err()
    );
    let forge = |f: &dyn Fn(&mut Record)| edit(&|r| f(&mut r[8]));
    // A witnessed status over each departure the judge fails.
    for status in [
        "static_only",
        "unwitnessed",
        "mismatch",
        "batched",
        "stopped",
    ] {
        assert!(
            forge(&|w| {
                *w = witness("M2", "gpioPortA.pa9obs", status);
                if let Record::MotorWitness { status, .. } = w {
                    *status = "witnessed".into();
                }
            })
            .is_err(),
            "{status}"
        );
    }
    // A witness of another timing, grain or quantum than the judge's.
    for f in [
        (|w: &mut Record| {
            if let Record::MotorWitness { period_ns, .. } = w {
                *period_ns += 4;
            }
        }) as fn(&mut Record),
        |w| {
            if let Record::MotorWitness { high_ns, .. } = w {
                *high_ns += 4;
            }
        },
        |w| {
            if let Record::MotorWitness { grain_ns, .. } = w {
                *grain_ns = 1_000;
            }
        },
        |w| {
            if let Record::MotorWitness { quantum_ns, .. } = w {
                *quantum_ns = 10;
            }
        },
        |w| {
            if let Record::MotorWitness { falls, .. } = w {
                *falls = 10;
            }
        },
    ] {
        assert!(forge(&f).is_err());
    }
}

#[test]
fn replay_ties_a_witness_s_fields_its_window_and_the_kernel_clock() {
    // Second adversarial review (on `a2c4fa4`), F3 and F4: each forgery
    // below passed `check_run` before.
    let base = run("witnessed", true, true, vec![]);
    let forged = |f: &dyn Fn(&mut Vec<Record>)| {
        let mut r = base.clone();
        f(&mut r);
        check_run(&r).map(|_| ())
    };
    assert_eq!(forged(&|_| {}), Ok(()));
    let w = |f: fn(&mut Record)| move |r: &mut Vec<Record>| f(&mut r[8]);
    for f in [
        // Two rises spanning 155 periods (everything else consistent).
        w(|x| {
            if let Record::MotorWitness {
                rises,
                falls,
                off_grid,
                ..
            } = x
            {
                (*rises, *falls, *off_grid) = (2, 2, 4);
            }
        }),
        // Rises on the model's spacing that run past the window's end.
        w(|x| {
            if let Record::MotorWitness {
                rises,
                falls,
                last_ns,
                ..
            } = x
            {
                (*rises, *falls) = (160, 159);
                *last_ns = Some(639_750 + 159 * 639_744);
            }
        }),
        // A window of its own: two rises ending a period apart, at 2 ms.
        w(|x| {
            if let Record::MotorWitness {
                rises,
                falls,
                last_ns,
                end_ns,
                off_grid,
                ..
            } = x
            {
                (*rises, *falls, *off_grid) = (2, 2, 4);
                *last_ns = Some(639_750 + 639_744);
                // Within a period of the last rise: covered, but not the board's.
                *end_ns = 1_900_000;
            }
        }),
        // More edges off the grid than edges.
        w(|x| {
            if let Record::MotorWitness { off_grid, .. } = x {
                *off_grid = 1_000_000;
            }
        }),
    ] {
        assert!(forged(&f).is_err());
    }
    // A boot that counts fewer instructions than the window, as a droop's
    // does from its last reset's release, is the run's own (third review,
    // F1: tying it to the window refused every droop on a motor board).
    assert_eq!(
        forged(&|r| if let Record::BootObserved {
            instructions_expected,
            instructions_observed,
            ..
        } = &mut r[6]
        {
            (*instructions_expected, *instructions_observed) = (3_970_000, Some(3_970_000));
        }),
        Ok(())
    );
    // Counts that would overflow, or wrap onto the honest spacing (third
    // review, F2): refused, never a panic.
    for f in [
        w(|x| {
            if let Record::MotorWitness { rises, falls, .. } = x {
                (*rises, *falls) = ((1 << 56) + 156, (1 << 56) + 155);
            }
        }),
        w(|x| {
            if let Record::MotorWitness { status, rises, .. } = x {
                (*status, *rises) = ("mismatch".into(), u64::MAX);
            }
        }),
        w(|x| {
            if let Record::MotorWitness { falls, .. } = x {
                *falls = u64::MAX;
            }
        }),
    ] {
        assert!(forged(&f).is_err());
    }
    // A kernel clock other than the reset clock's, every record consistent
    // with it: PSC 0 at a claimed 1 MHz is 400 Hz on paper and 25.6 kHz on
    // silicon (the review's forgery).
    let mut clock = base.clone();
    if let Record::MotorOutputSolved { psc, .. } = &mut clock[3] {
        *psc = 0;
    }
    clock[5] = Record::MotorTimingCheck {
        label: "M2".into(),
        kernel_hz: 1_000_000,
        period_ps: 2_500_000_000,
        high_ps: 1_000_000_000,
        nominal_period_ps: 2_500_000_000,
        nominal_high_ps: 1_000_000_000,
        holds: true,
    };
    if let Record::MotorWitness {
        period_ns,
        high_ns,
        rises,
        falls,
        first_ns,
        last_ns,
        span_ns,
        ..
    } = &mut clock[8]
    {
        (*period_ns, *high_ns) = (2499 * 4, 1000 * 4);
        (*rises, *falls) = (10_004, 10_003);
        *first_ns = Some(10_000);
        *last_ns = Some(10_000 + 10_003 * 9996);
        *span_ns = Some(8);
    }
    assert!(check_run(&clock).is_err());
    if let Record::MotorTimingCheck { kernel_hz, .. } = &mut clock[5] {
        *kernel_hz = 64_000_000;
    }
    // At the true clock the same figures do not hold, so it is refused too:
    // the forgery needs the false clock, and the pin refuses that.
    assert!(check_run(&clock).is_err());
}

/// `AET-WO-2302`: a DShot output's window is its anchor plus
/// `DSHOT_WINDOW_NS`, derived by replay from the records, never the board's
/// whole boot; the judge's stop (kind 30) sits at its end; and a window past
/// the boot, or one with no stop, is named once per timer.
fn dshot_run(anchor: Option<u64>, stop: Option<(u64, u32)>, findings: Vec<(&str, u64)>) -> Vec<Record> {
    let mut r = run("witnessed", true, true, vec![]);
    r[3] = Record::MotorOutputSolved {
        label: "M2".into(),
        timer: "TIM2".into(),
        channel: 1,
        pin: "PA9".into(),
        alternate_function: 1,
        protocol: "dshot600".into(),
        psc: 0,
        arr: 106,
        ccr: 40,
        pwm: None,
    };
    r[5] = Record::MotorTimingCheck {
        label: "M2".into(),
        kernel_hz: 64_000_000,
        period_ps: 1_671_875,
        high_ps: 625_000,
        nominal_period_ps: 1_666_666,
        nominal_high_ps: 624_999,
        holds: true,
    };
    // Its edges from its anchor: the first rise one period after it, the
    // last within a period of its window's end.
    let at = anchor.unwrap_or(0);
    let rises = 4716;
    r[8] = Record::MotorWitness {
        label: "M2".into(),
        observer: "gpioPortA.pa9obs".into(),
        period_ns: 424,
        high_ns: 160,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: if anchor.is_some() { "witnessed" } else { "unanchored" }.into(),
        rises,
        falls: rises,
        first_ns: anchor.map(|a| a + 490),
        last_ns: anchor.map(|a| a + 490 + (rises - 1) * 424),
        end_ns: anchor.map_or(100_000_000, |a| a + 2_000_000),
        span_ns: anchor.map(|_| 8),
        off_grid: 2 * rises - 1,
        batched: 0,
        anchor_ns: anchor,
    };
    let mut at_end = r.split_off(9);
    if let Some((stop_at, cr1)) = stop {
        r.push(Record::JudgeStop {
            timer: "TIM2".into(),
            anchor_ns: at,
            at_ns: stop_at,
            read_back: vec![Reading {
                address: 0x4000_0000,
                mask: 0xFF,
                expected: 0x81,
                observed: Some(0x81),
            }],
            cr1_after: Some(cr1),
        });
    }
    r.append(&mut at_end);
    let failing: Vec<&str> = findings.iter().map(|f| f.0).collect();
    r[0] = scenario(findings.is_empty(), &failing);
    let last = r.len() - 1;
    r[last] = Record::Verdict {
        pass: findings.is_empty(),
        findings: findings
            .into_iter()
            .map(|(k, n)| (k.to_owned(), "error".to_owned(), n))
            .collect(),
    };
    r
}

#[test]
fn a_dshot_output_s_window_is_its_anchor_plus_dshot_window_ns() {
    let a = 30_670;
    let end = a + 2_000_000;
    let ok = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    assert_eq!(check_run(&ok).map(|s| s.pass), Ok(true));
    // Kind 30 round-trips through the archive, after kind 26.
    let events: Vec<Event> = ok
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("stop-roundtrip", 7, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, ok);
    assert_eq!(ok[9].kind(), 30);
    // The witness ending anywhere but its anchor plus 2 ms is refused: the
    // board's boot (100 ms, the old 2 ms) is no DShot window.
    for wrong_end in [100_000_000, 2_000_000, end - 10] {
        let mut r = ok.clone();
        if let Record::MotorWitness { end_ns, .. } = &mut r[8] {
            *end_ns = wrong_end;
        }
        assert!(check_run(&r).is_err(), "{wrong_end}");
    }
    // A first rise before the anchor is refused.
    let mut r = ok.clone();
    if let Record::MotorWitness { first_ns, .. } = &mut r[8] {
        *first_ns = Some(a - 1);
    }
    assert!(check_run(&r).is_err());
    // The stop anywhere but the window's end, of another anchor, of a PWM
    // timer, or twice, is refused.
    for f in [
        |s: &mut Record| {
            if let Record::JudgeStop { at_ns, .. } = s {
                *at_ns -= 10;
            }
        },
        |s: &mut Record| {
            if let Record::JudgeStop { anchor_ns, at_ns, .. } = s {
                (*anchor_ns, *at_ns) = (*anchor_ns + 10, *at_ns + 10);
            }
        },
        |s: &mut Record| {
            if let Record::JudgeStop { timer, .. } = s {
                *timer = "TIM1".into();
            }
        },
    ] {
        let mut r = ok.clone();
        f(&mut r[9]);
        assert!(check_run(&r).is_err());
    }
    let mut twice = ok.clone();
    twice.insert(9, ok[9].clone());
    assert!(check_run(&twice).is_err());
    // No stop, or one that left CEN set: `judge_stop_missing`, once.
    let missing = vec![(JUDGE_STOP_MISSING_FINDING, 1)];
    assert!(check_run(&dshot_run(Some(a), None, vec![])).is_err());
    assert!(check_run(&dshot_run(Some(a), None, missing.clone())).unwrap().matched);
    assert!(check_run(&dshot_run(Some(a), Some((end, 0x81)), missing)).unwrap().matched);
    // F2's late anchor: 99.07 ms plus 2 ms is past the boot, named.
    let late = 99_072_350;
    let mut r = dshot_run(Some(late), None, vec![(MOTOR_WINDOW_PAST_RUN_FINDING, 1)]);
    if let Record::MotorWitness {
        status,
        rises,
        falls,
        last_ns,
        off_grid,
        ..
    } = &mut r[8]
    {
        // Its edges to the boot's end: 2187 rises, short of its window.
        (*status, *rises, *falls, *off_grid) = ("stopped".into(), 2187, 2187, 4373);
        *last_ns = Some(late + 490 + 2186 * 424);
    }
    r[0] = scenario(false, &[MOTOR_MISMATCH_FINDING, MOTOR_WINDOW_PAST_RUN_FINDING]);
    let n = r.len() - 1;
    r[n] = Record::Verdict {
        pass: false,
        findings: vec![
            (MOTOR_MISMATCH_FINDING.into(), "error".into(), 1),
            (MOTOR_WINDOW_PAST_RUN_FINDING.into(), "error".into(), 1),
        ],
    };
    assert!(check_run(&r).unwrap().matched);
    // Edges with no anchor: `unanchored`, named `motor_anchor_unobserved`,
    // the window the boot's end.
    let unanchored = dshot_run(None, None, vec![(MOTOR_ANCHOR_UNOBSERVED_FINDING, 1)]);
    assert!(check_run(&unanchored).unwrap().matched);
    assert!(check_run(&dshot_run(None, None, vec![])).is_err());
}
