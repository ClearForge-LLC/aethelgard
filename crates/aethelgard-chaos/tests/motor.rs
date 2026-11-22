//! `AET-WO-4300a` §1.4: kinds 24 to 26 (a solved motor output, its static
//! check for silicon, its twin witness) through the archive, their field
//! rules, and `check_run`'s joins and its re-derivation of the verdict's
//! motor findings from them. `AET-WO-2302`: each witness's window derived
//! from its anchor, and kind 30, the judge's stop.

mod common;

use aethelgard_chaos::campaign::{
    BOOT_WORD_FINDING, JUDGE_STOP_MISMATCH_FINDING, JUDGE_STOP_MISSING_FINDING,
    MOTOR_ANCHOR_ORDER_FINDING, MOTOR_ANCHOR_REPEATED_FINDING, MOTOR_ANCHOR_UNOBSERVED_FINDING,
    MOTOR_EDGE_AFTER_FINDING, MOTOR_EDGE_BEFORE_FINDING, MOTOR_MISMATCH_FINDING,
    MOTOR_STATIC_FINDING, MOTOR_STATIC_ONLY_FINDING, MOTOR_UNWITNESSED_FINDING,
    MOTOR_WINDOW_PAST_RUN_FINDING, SERIAL_INJECTION_ORDER_FINDING, SERIAL_WIRE_PAST_RUN_FINDING,
    check_run,
};
use aethelgard_chaos::record::{Reading, Record};
use aethelgard_chaos::{Event, replay_dir};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

/// The CCR of an output whose static check holds (the 1000 µs idle pulse at
/// a 1 µs tick), and of one whose check fails (a tick short).
const CCR_HOLDS: u32 = 1000;
const CCR_FAILS: u32 = 999;

/// The refusals the tests below assert by name (`AET-WO-2302b`: a test
/// that asserted only an error could pass on another guard's).
const MOTOR_REFUSED: &str = "the verdict's motor findings do not match the motor records";
const STATUS_REFUSED: &str =
    "a motor witness's status is not what its counts and its output's timing give";
const CHECK_REFUSED: &str = "a motor timing check is not what its solved output gives";
const WINDOW_REFUSED: &str =
    "a motor witness's window is not its anchor's, or its first rise is before it";
const ANCHOR_FIELDS_REFUSED: &str = "a motor witness names an anchor without a CEN rise, a rise without an anchor, or edges outside a window it does not have";
const INSTANT_REFUSED: &str = "the verdict's serial_wire_past_run or motor_anchor_out_of_order does not match the boot word's instant";

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
        idle: Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
        resolution_ps: Some(1_000_000),
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

/// The model's PWM period at the 64 MHz kernel clock (`AET-WO-4400g`): 2,499
/// ticks of a 1 µs tick, `realized_ns` exactly, as `motors-pwm` measured
/// (`spike/4400f/out/pwm/pwm-a`: 40 rises in 100 ms).
const PERIOD: u64 = 2_499_000;
const FIRST: u64 = PWM_ANCHOR + PERIOD;

fn witness_ccr(label: &str, observer: &str, status: &str, ccr: u32) -> Record {
    let (mut rises, mut falls, mut first_ns, mut last_ns, mut span_ns) =
        (40, 39, Some(FIRST), Some(FIRST + 39 * PERIOD), Some(8));
    let (mut off_grid, mut batched) = (79, 0);
    match status {
        "static_only" => off_grid = 0,
        "unwitnessed" | "unreadable" => {
            (rises, falls, first_ns, last_ns, span_ns, off_grid) = (0, 0, None, None, None, 0);
        }
        "mismatch" => span_ns = Some(400),
        "batched" => batched = 3,
        // A rise every period to 50 ms, short of the window's end.
        "stopped" => {
            (rises, falls, last_ns, off_grid) = (20, 19, Some(FIRST + 19 * PERIOD), 39);
        }
        "unanchored" => (first_ns, last_ns, span_ns) = (None, None, None),
        _ => {}
    }
    Record::MotorWitness {
        label: label.into(),
        observer: observer.into(),
        period_ns: PERIOD,
        high_ns: u64::from(ccr) * 1_000,
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
        edges_before: 0,
        edges_after: 0,
        cen_rises: u64::from(status != "unanchored"),
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
            assert_eq!(
                check_run(&run(status, true, true, wrong)).unwrap_err(),
                MOTOR_REFUSED,
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
    assert_eq!(
        check_run(&as_error).unwrap_err(),
        "a motor finding has the wrong severity"
    );
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
    assert_eq!(check_run(&named).unwrap_err(), MOTOR_REFUSED);
}

#[test]
fn a_static_mismatch_is_named_once_per_failing_check_booted_or_not() {
    for booted in [true, false] {
        let mut findings = vec![(MOTOR_STATIC_FINDING, "error", 1)];
        if !booted {
            findings.push(("boot_word_missing", "error", 1));
        }
        assert!(check_run(&run("witnessed", false, booted, findings.clone())).is_ok());
        // The static mismatch dropped, the boot word's finding kept: refused
        // by the motor findings, never by the boot word's (`AET-WO-2302b`).
        findings.retain(|f| f.0 != MOTOR_STATIC_FINDING);
        assert_eq!(
            check_run(&run("witnessed", false, booted, findings)).unwrap_err(),
            MOTOR_REFUSED,
            "booted {booted}"
        );
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
    assert_eq!(
        edit(&|r| r[8] = witness("M2", "gpioPortA.pa8obs", "witnessed")),
        Err("a motor witness is not its output's pin's observer, or is there twice")
    );
    // A witness or a check of an output nothing solved.
    assert_eq!(
        edit(&|r| r[8] = witness("M9", "gpioPortA.pa9obs", "witnessed")),
        Err("a motor witness names no solved output")
    );
    assert_eq!(
        edit(&|r| r[5] = check("M9", true)),
        Err("a motor timing check names no solved output")
    );
    // A solved output with no static check.
    assert_eq!(
        edit(&|r| {
            r.remove(5);
        }),
        Err("a solved motor output has no static timing check")
    );
    // Two witnesses of one output.
    assert_eq!(
        edit(&|r| r[8] = witness("M1", "gpioPortA.pa8obs", "witnessed")),
        Err("a motor witness is not its output's pin's observer, or is there twice")
    );
    // A witness out of its rank, after the verdict.
    assert_eq!(
        edit(&|r| {
            let w = r.remove(8);
            r.push(w);
        }),
        Err("the records are not in the campaign's order")
    );
}

#[test]
fn the_field_rules_hold_on_write_and_read() {
    let bad = [
        (
            "a solved motor output's channel is not 1 to 4 or its AF above 15",
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
                idle: Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
                resolution_ps: Some(1_000_000),
            },
        ),
        (
            "a solved motor output's protocol is unknown, or its PWM timing, idle or tick is not present exactly for PWM",
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
                idle: Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
                resolution_ps: Some(1_000_000),
            },
        ),
        (
            "a solved motor output's CCR is not its idle's: 0 for low, ARR + 1 for high, else 1 to ARR - 1",
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
                idle: Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
                resolution_ps: Some(1_000_000),
            },
        ),
        (
            "a motor timing check's label is malformed or its timer clock 0",
            Record::MotorTimingCheck {
                label: "M1".into(),
                kernel_hz: 0,
                period_ps: 1,
                high_ps: 1,
                nominal_period_ps: 1,
                nominal_high_ps: 1,
                holds: true,
            },
        ),
        (
            "a motor witness's status is not one the judge gives",
            witness("M1", "gpioPortA.pa8obs", "fine"),
        ),
    ];
    for (why, r) in bad {
        assert_eq!(r.check(), Err(why), "{r:?}");
    }
    assert!(solved("M1", "PA8", 1).check().is_ok());
    // `AET-WO-1211c`: kind 24's idle and tick (trace 18). An idle level is
    // CCR 0 (low) or ARR + 1 (high); a pulse is inside the period; a DShot
    // output carries neither, and a PWM output both.
    use aethelgard_chaos::record::MotorIdle;
    let with = |idle: Option<MotorIdle>, ccr: u32, resolution_ps: Option<u64>| {
        let mut r = solved("M1", "PA8", 1);
        if let Record::MotorOutputSolved {
            idle: i,
            ccr: c,
            resolution_ps: t,
            ..
        } = &mut r
        {
            *i = idle;
            *c = ccr;
            *t = resolution_ps;
        }
        r
    };
    let ccr_why = "a solved motor output's CCR is not its idle's: 0 for low, ARR + 1 for high, else 1 to ARR - 1";
    let shape_why = "a solved motor output's protocol is unknown, or its PWM timing, idle or tick is not present exactly for PWM";
    let tick = Some(1_000_000);
    assert_eq!(with(Some(MotorIdle::Low), 0, tick).check(), Ok(()));
    assert_eq!(with(Some(MotorIdle::High), 2500, tick).check(), Ok(()));
    assert_eq!(with(Some(MotorIdle::Low), 1000, tick).check(), Err(ccr_why));
    assert_eq!(
        with(Some(MotorIdle::High), 2499, tick).check(),
        Err(ccr_why)
    );
    assert_eq!(
        with(Some(MotorIdle::PulseUs(1000)), 0, tick).check(),
        Err(ccr_why)
    );
    assert_eq!(with(None, 1000, tick).check(), Err(shape_why));
    assert_eq!(
        with(Some(MotorIdle::PulseUs(1000)), 1000, None).check(),
        Err(shape_why)
    );
    assert_eq!(
        with(Some(MotorIdle::PulseUs(1000)), 1000, Some(0)).check(),
        Err(shape_why)
    );
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
    assert_eq!(check_run(&short).unwrap_err(), CHECK_REFUSED);
    // A check whose figures are not its output's.
    assert_eq!(
        edit(
            &|r| if let Record::MotorTimingCheck { high_ps, .. } = &mut r[5] {
                *high_ps += 1;
            }
        ),
        Err(CHECK_REFUSED)
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
        assert_eq!(
            forge(&|w| {
                *w = witness("M2", "gpioPortA.pa9obs", status);
                if let Record::MotorWitness { status, .. } = w {
                    *status = "witnessed".into();
                }
            }),
            Err(STATUS_REFUSED),
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
        assert_eq!(forge(&f), Err(STATUS_REFUSED));
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
    for (why, f) in [
        // Two rises spanning 39 periods (everything else consistent).
        (
            STATUS_REFUSED,
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
        ),
        // Rises on the model's spacing that run past the window's end.
        (
            STATUS_REFUSED,
            w(|x| {
                if let Record::MotorWitness {
                    rises,
                    falls,
                    last_ns,
                    ..
                } = x
                {
                    (*rises, *falls) = (42, 41);
                    *last_ns = Some(FIRST + 41 * PERIOD);
                }
            }),
        ),
        // A window of its own: two rises ending a period apart, at 2 ms.
        (
            WINDOW_REFUSED,
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
                    *last_ns = Some(FIRST + PERIOD);
                    // Within a period of the last rise: covered, but not the board's.
                    *end_ns = 1_900_000;
                }
            }),
        ),
        // More edges off the grid than edges.
        (
            STATUS_REFUSED,
            w(|x| {
                if let Record::MotorWitness { off_grid, .. } = x {
                    *off_grid = 1_000_000;
                }
            }),
        ),
    ] {
        assert_eq!(forged(&f), Err(why));
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
                (*rises, *falls) = ((1 << 56) + 40, (1 << 56) + 39);
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
        assert_eq!(forged(&f), Err(STATUS_REFUSED));
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
    assert_eq!(check_run(&clock).unwrap_err(), CHECK_REFUSED);
    if let Record::MotorTimingCheck { kernel_hz, .. } = &mut clock[5] {
        *kernel_hz = 64_000_000;
    }
    // At the true clock the same figures do not hold, so it is refused too:
    // the forgery needs the false clock, and the pin refuses that.
    assert_eq!(check_run(&clock).unwrap_err(), CHECK_REFUSED);
}

/// The DShot timer's solved registers in `dshot_run`: name, address, mask,
/// value (`CR1` with `CEN` set), as kind 11 lists them and kind 30 reads them
/// back.
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

/// Where `dshot_run` puts its DShot output's witness (kind 26) and its
/// judge's stop (kind 30): after the boot (kind 10), the timer's ten kind 11s
/// and `M1`'s witness.
const DSHOT_WITNESS: usize = 8 + TIM2_SOLVED.len();
const DSHOT_STOP: usize = DSHOT_WITNESS + 1;

/// A DShot600 output's period at the 64 MHz kernel clock, as Renode
/// realizes it, and its first rise's lead over its anchor.
const DSHOT_PERIOD: u64 = 1_657;
const DSHOT_LEAD: u64 = 1_700;

/// `AET-WO-2302`: a DShot output's window is its anchor plus
/// `DSHOT_WINDOW_NS`, derived by replay from the records, never the board's
/// whole boot; the judge's stop (kind 30) sits at its end; and a window past
/// the boot, or one with no stop, is named once per timer.
fn dshot_run(
    anchor: Option<u64>,
    stop: Option<(u64, u32)>,
    findings: Vec<(&str, u64)>,
) -> Vec<Record> {
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
        idle: None,
        resolution_ps: None,
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
    // Its edges from its anchor: the first rise a period and a little after
    // it, the last within a period of its window's end. At the 64 MHz kernel
    // clock (`AET-WO-4400g`) a DShot600 bit of 106 ticks runs 1,657 ns and
    // its 40-tick high time 625 ns (`realized_ns`), and the 2 ms window
    // holds 1,206 rises, as `motors-dshot600` measured.
    let at = anchor.unwrap_or(0);
    let rises = 1206;
    r[8] = Record::MotorWitness {
        label: "M2".into(),
        observer: "gpioPortA.pa9obs".into(),
        period_ns: DSHOT_PERIOD,
        high_ns: 625,
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
        first_ns: anchor.map(|a| a + DSHOT_LEAD),
        last_ns: anchor.map(|a| a + DSHOT_LEAD + (rises - 1) * DSHOT_PERIOD),
        end_ns: anchor.map_or(100_000_000, |a| a + 2_000_000),
        span_ns: anchor.map(|_| 8),
        off_grid: 2 * rises - 1,
        batched: 0,
        anchor_ns: anchor,
        edges_before: 0,
        edges_after: 0,
        cen_rises: u64::from(anchor.is_some()),
    };
    // Kind 11, the timer's solved registers after the boot: `CR1` expects
    // `CEN` cleared after a stop that cleared it, the judge's own act.
    let stopped = stop.is_some_and(|(_, cr1)| cr1 & 1 == 0);
    let registers: Vec<Record> = TIM2_SOLVED
        .iter()
        .map(|&(name, address, mask, value)| {
            let expected = if name == "TIM2.CR1" && stopped {
                value & !1
            } else {
                value
            };
            Record::RegisterObserved {
                name: name.into(),
                reading: Reading {
                    address,
                    mask,
                    expected,
                    observed: Some(expected),
                },
            }
        })
        .collect();
    let mut at_end = r.split_off(7);
    r.extend(registers);
    let mut tail = at_end.split_off(2);
    r.append(&mut at_end);
    if let Some((stop_at, cr1)) = stop {
        r.push(Record::JudgeStop {
            timer: "TIM2".into(),
            anchor_ns: at,
            at_ns: stop_at,
            read_back: TIM2_SOLVED
                .iter()
                .map(|&(_, address, mask, value)| Reading {
                    address,
                    mask,
                    expected: value,
                    observed: Some(value),
                })
                .collect(),
            cr1_after: Some(cr1),
        });
    }
    r.append(&mut tail);
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
    assert_eq!(ok[DSHOT_STOP].kind(), 30);
    // The witness ending anywhere but its anchor plus 2 ms is refused: the
    // board's boot (100 ms, the old 2 ms) is no DShot window.
    for wrong_end in [100_000_000, 2_000_000, end - 10] {
        let mut r = ok.clone();
        if let Record::MotorWitness { end_ns, .. } = &mut r[DSHOT_WITNESS] {
            *end_ns = wrong_end;
        }
        assert_eq!(check_run(&r).unwrap_err(), WINDOW_REFUSED, "{wrong_end}");
    }
    // A first rise before the anchor is refused.
    let mut r = ok.clone();
    if let Record::MotorWitness { first_ns, .. } = &mut r[DSHOT_WITNESS] {
        *first_ns = Some(a - 1);
    }
    assert_eq!(check_run(&r).unwrap_err(), WINDOW_REFUSED);
    // The stop anywhere but the window's end, of another anchor, of a PWM
    // timer, or twice, is refused.
    let at = "a judge's stop is not at its timer's witnesses' anchor plus DSHOT_WINDOW_NS, inside the boot";
    let timer = "a judge's stop names no DShot timer, or one twice";
    for (why, f) in [
        (
            at,
            (|s: &mut Record| {
                if let Record::JudgeStop { at_ns, .. } = s {
                    *at_ns -= 10;
                }
            }) as fn(&mut Record),
        ),
        (at, |s: &mut Record| {
            if let Record::JudgeStop {
                anchor_ns, at_ns, ..
            } = s
            {
                (*anchor_ns, *at_ns) = (*anchor_ns + 10, *at_ns + 10);
            }
        }),
        (timer, |s: &mut Record| {
            if let Record::JudgeStop { timer, .. } = s {
                *timer = "TIM1".into();
            }
        }),
    ] {
        let mut r = ok.clone();
        f(&mut r[DSHOT_STOP]);
        assert_eq!(check_run(&r).unwrap_err(), why);
    }
    let mut twice = ok.clone();
    twice.insert(DSHOT_STOP, ok[DSHOT_STOP].clone());
    assert_eq!(check_run(&twice).unwrap_err(), timer);
    // No stop, or one that left CEN set: `judge_stop_missing`, once.
    let missing = vec![(JUDGE_STOP_MISSING_FINDING, 1)];
    assert_eq!(
        check_run(&dshot_run(Some(a), None, vec![])).unwrap_err(),
        MOTOR_REFUSED
    );
    assert!(
        check_run(&dshot_run(Some(a), None, missing.clone()))
            .unwrap()
            .matched
    );
    assert!(
        check_run(&dshot_run(Some(a), Some((end, 0x81)), missing))
            .unwrap()
            .matched
    );
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
    } = &mut r[DSHOT_WITNESS]
    {
        // Its edges to the boot's end: 559 rises, short of its window.
        (*status, *rises, *falls, *off_grid) = ("stopped".into(), 559, 559, 1117);
        *last_ns = Some(late + DSHOT_LEAD + 558 * DSHOT_PERIOD);
    }
    r[0] = scenario(
        false,
        &[MOTOR_MISMATCH_FINDING, MOTOR_WINDOW_PAST_RUN_FINDING],
    );
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
    assert_eq!(
        check_run(&dshot_run(None, None, vec![])).unwrap_err(),
        MOTOR_REFUSED
    );
}

/// Adversarial pass 1, L3 (R1, R2): replay binds what kinds 26 and 30 can
/// carry. A stop that cleared `CEN` and read a register back off its solved
/// value names `judge_stop_mismatch` once per such register; kind 26's
/// `edges_before`, `edges_after` and `cen_rises` name
/// `motor_edge_before_anchor`, `motor_edge_after_window` (once per output)
/// and `motor_anchor_repeated` (once per timer). A verdict that adds or
/// drops any of them beside its records is refused. What no record
/// carries stays unbound, documented: `motor_anchor_out_of_order`'s cases
/// other than an anchor after the boot word (the boot's line order; at
/// most once per anchored timer), `serial_injection_out_of_order` (the
/// logged `TDR` bytes). With no serial port, `serial_wire_past_run` is
/// never named (adversarial pass 2, A2-5).
#[test]
fn replay_binds_the_anchor_findings_kinds_26_and_30_carry() {
    let a = 30_670;
    let end = a + 2_000_000;
    let run = |f: &dyn Fn(&mut Vec<Record>), findings: Vec<(&str, u64)>| {
        let mut r = dshot_run(Some(a), Some((end, 0x80)), findings);
        f(&mut r);
        check_run(&r).map(|s| s.matched)
    };
    let none = |_: &mut Vec<Record>| {};
    assert_eq!(run(&none, vec![]), Ok(true));
    // R2: each bound kind named beside clean records is refused.
    for kind in [
        JUDGE_STOP_MISMATCH_FINDING,
        MOTOR_ANCHOR_REPEATED_FINDING,
        MOTOR_EDGE_BEFORE_FINDING,
        MOTOR_EDGE_AFTER_FINDING,
    ] {
        assert_eq!(run(&none, vec![(kind, 1)]), Err(MOTOR_REFUSED), "{kind}");
    }
    // The unbound ones are neither required nor refused.
    for kind in [MOTOR_ANCHOR_ORDER_FINDING, SERIAL_INJECTION_ORDER_FINDING] {
        assert_eq!(run(&none, vec![(kind, 1)]), Ok(true), "{kind}");
    }
    // Two timers anchored (TIM1's PWM, TIM2's DShot): out of order at most
    // twice. No port: no wire time past the run.
    assert_eq!(run(&none, vec![(MOTOR_ANCHOR_ORDER_FINDING, 2)]), Ok(true));
    assert_eq!(
        run(&none, vec![(MOTOR_ANCHOR_ORDER_FINDING, 3)]),
        Err(INSTANT_REFUSED)
    );
    assert_eq!(
        run(&none, vec![(SERIAL_WIRE_PAST_RUN_FINDING, 1)]),
        Err(INSTANT_REFUSED)
    );
    // R1: the read-back off its solved value: named once, or refused.
    let off = |r: &mut Vec<Record>| {
        if let Record::JudgeStop { read_back, .. } = &mut r[DSHOT_STOP] {
            read_back[0].observed = Some(0x00);
        }
    };
    assert_eq!(run(&off, vec![]), Err(MOTOR_REFUSED));
    assert_eq!(run(&off, vec![(JUDGE_STOP_MISMATCH_FINDING, 1)]), Ok(true));
    assert_eq!(
        run(&off, vec![(JUDGE_STOP_MISMATCH_FINDING, 2)]),
        Err(MOTOR_REFUSED)
    );
    // A register the stop did not read back: once.
    let short = |r: &mut Vec<Record>| {
        if let Record::JudgeStop { read_back, .. } = &mut r[DSHOT_STOP] {
            read_back[0].observed = None;
        }
    };
    assert_eq!(run(&short, vec![]), Err(MOTOR_REFUSED));
    assert_eq!(
        run(&short, vec![(JUDGE_STOP_MISMATCH_FINDING, 1)]),
        Ok(true)
    );
    // A stop that left CEN set is `judge_stop_missing`, never a mismatch;
    // kind 11's `CR1` then expects the solved value, CEN set, as the judge
    // does (A2-1: kind 11 still expecting it cleared is refused).
    let left_only = |r: &mut Vec<Record>| {
        off(r);
        if let Record::JudgeStop { cr1_after, .. } = &mut r[DSHOT_STOP] {
            *cr1_after = Some(0x81);
        }
    };
    let left = |r: &mut Vec<Record>| {
        left_only(r);
        cr1_expected(r, 0x81);
    };
    assert_eq!(run(&left, vec![(JUDGE_STOP_MISSING_FINDING, 1)]), Ok(true));
    assert_eq!(
        run(&left_only, vec![(JUDGE_STOP_MISSING_FINDING, 1)]),
        Err(STOP_VALUE_REFUSED)
    );
    // Kind 26's counts.
    let witness = |before: u64, after: u64, rises: u64| {
        move |r: &mut Vec<Record>| {
            if let Record::MotorWitness {
                edges_before,
                edges_after,
                cen_rises,
                ..
            } = &mut r[DSHOT_WITNESS]
            {
                (*edges_before, *edges_after, *cen_rises) = (before, after, rises);
            }
        }
    };
    for (w, kind) in [
        (witness(3, 0, 1), MOTOR_EDGE_BEFORE_FINDING),
        (witness(0, 5, 1), MOTOR_EDGE_AFTER_FINDING),
        (witness(0, 0, 2), MOTOR_ANCHOR_REPEATED_FINDING),
    ] {
        assert_eq!(run(&w, vec![]), Err(MOTOR_REFUSED), "{kind}");
        assert_eq!(run(&w, vec![(kind, 1)]), Ok(true), "{kind}");
    }
    // Before the boot word, none is named.
    let unbooted = |r: &mut Vec<Record>| {
        witness(3, 5, 2)(r);
        off(r);
        if let Some(Record::BootObserved { boot_word, .. }) = r
            .iter_mut()
            .find(|x| matches!(x, Record::BootObserved { .. }))
        {
            boot_word.observed = Some(0);
        }
    };
    assert_eq!(run(&unbooted, vec![(BOOT_WORD_FINDING, 1)]), Ok(true));
    // Kind 26 itself, as the archive's reader checks it: an anchor without
    // a rise, a rise without one, or edges outside a window it does not
    // have, are refused.
    let mut r = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    assert_eq!(r[DSHOT_WITNESS].check(), Ok(()));
    witness(0, 0, 0)(&mut r);
    assert_eq!(r[DSHOT_WITNESS].check(), Err(ANCHOR_FIELDS_REFUSED));
    let mut r = dshot_run(None, None, vec![(MOTOR_ANCHOR_UNOBSERVED_FINDING, 1)]);
    assert!(check_run(&r).unwrap().matched);
    assert_eq!(r[DSHOT_WITNESS].check(), Ok(()));
    for (before, after, rises) in [(0, 0, 1), (1, 0, 0), (0, 1, 0)] {
        witness(before, after, rises)(&mut r);
        assert_eq!(
            r[DSHOT_WITNESS].check(),
            Err(ANCHOR_FIELDS_REFUSED),
            "{before} {after} {rises}"
        );
    }
}

/// Replay's refusals of a judge's stop's read-back that is not the run's
/// own (adversarial pass 2, A2-1).
const STOP_COUNT_REFUSED: &str =
    "a judge's stop does not read back exactly its timer's solved registers that kind 11 lists";
const STOP_PLACE_REFUSED: &str = "a judge's stop reads a register back at an address or under a mask that is not kind 11's for it";
const STOP_VALUE_REFUSED: &str =
    "a judge's stop reads a register back against a value that is not kind 11's for it";

/// Sets kind 11's `TIM2.CR1` expected value in a `dshot_run`.
fn cr1_expected(r: &mut [Record], value: u32) {
    for x in r.iter_mut() {
        if let Record::RegisterObserved { name, reading } = x
            && name == "TIM2.CR1"
        {
            reading.expected = value;
        }
    }
}

/// Adversarial pass 2, A2-1: kind 30's read-back carried its own address,
/// mask and expected value, so a run could forge a stop's `CR1` read back
/// as 0 against an expected 0, under a mask of 0, or at another address,
/// and name no `judge_stop_mismatch`; or read back one register of the
/// timer's ten. Replay now binds each entry to kind 11's for the same
/// register (its `CR1` with `CEN` cleared, the judge's own act) and the
/// read-back to exactly the timer's registers kind 11 lists, and refuses
/// anything else by name, whatever the verdict says.
#[test]
fn replay_binds_a_stop_s_read_back_to_kind_11() {
    let a = 30_670;
    let end = a + 2_000_000;
    let run = |f: &dyn Fn(&mut Reading), findings: Vec<(&str, u64)>| {
        let mut r = dshot_run(Some(a), Some((end, 0x80)), findings);
        if let Record::JudgeStop { read_back, .. } = &mut r[DSHOT_STOP] {
            f(&mut read_back[0]);
        }
        check_run(&r).map(|s| s.matched)
    };
    assert_eq!(run(&|_| {}, vec![]), Ok(true));
    // The judge's own act: kind 11's `CR1` expects CEN cleared, the stop's
    // read-back the solved value, CEN set.
    let k11 = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    assert!(
        k11.iter()
            .any(|x| matches!(x, Record::RegisterObserved { name, reading }
        if name == "TIM2.CR1" && reading.expected == 0x80))
    );
    // CR1 observed 0 against an expected forged to 0: no mismatch by its
    // own fields, refused by kind 11's.
    let zero = |g: &mut Reading| (g.expected, g.observed) = (0, Some(0));
    for findings in [vec![], vec![(JUDGE_STOP_MISMATCH_FINDING, 1)]] {
        assert_eq!(run(&zero, findings), Err(STOP_VALUE_REFUSED));
    }
    // Under a mask of 0, every value matches: refused.
    let unmasked = |g: &mut Reading| (g.mask, g.expected, g.observed) = (0, 0, Some(0));
    assert_eq!(run(&unmasked, vec![]), Err(STOP_PLACE_REFUSED));
    // At another address (another timer's `CR1`): refused.
    let moved = |g: &mut Reading| g.address = 0x4000_0400;
    assert_eq!(run(&moved, vec![]), Err(STOP_PLACE_REFUSED));
    // One register of the timer's ten, or one more than kind 11 lists: refused.
    for keep in [1, TIM2_SOLVED.len() - 1] {
        let mut r = dshot_run(Some(a), Some((end, 0x80)), vec![]);
        if let Record::JudgeStop { read_back, .. } = &mut r[DSHOT_STOP] {
            read_back.truncate(keep);
        }
        assert_eq!(check_run(&r).map(|s| s.matched), Err(STOP_COUNT_REFUSED));
    }
    let mut r = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    if let Record::JudgeStop { read_back, .. } = &mut r[DSHOT_STOP] {
        read_back.push(read_back[1].clone());
    }
    assert_eq!(check_run(&r).map(|s| s.matched), Err(STOP_COUNT_REFUSED));
    // Kind 11 in another order than the read-back's: refused.
    let mut r = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    r.swap(7, 8);
    assert_eq!(check_run(&r).map(|s| s.matched), Err(STOP_PLACE_REFUSED));
    // Kind 11 forged instead, to expect CEN set after the stop cleared it,
    // or both forged to CEN set: refused.
    let mut r = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    cr1_expected(&mut r, 0x81);
    assert_eq!(check_run(&r).map(|s| s.matched), Err(STOP_VALUE_REFUSED));
    // The read-back's CR1 expected forged to CEN clear beside a timer read
    // back stopped (kind 11's own value): refused, since the stop reads the
    // solved CR1, CEN set.
    let cleared = |g: &mut Reading| (g.expected, g.observed) = (0x80, Some(0x80));
    assert_eq!(run(&cleared, vec![]), Err(STOP_VALUE_REFUSED));
}

/// Mutation sweep of `AET-WO-2302`'s replay joins: each refusal of a
/// witness's window, of two outputs of one timer, of a judge's stop, and of
/// the verdict's `motor_anchor_out_of_order` count, named exactly. Each
/// forgery below breaks one condition alone, so a refusal by another
/// condition's message is a failure (the `is_err` asserts beside them cannot
/// tell the conditions apart).
#[test]
fn replay_refuses_each_window_and_stop_join_by_its_own_message() {
    const WINDOW: &str =
        "a motor witness's window is not its anchor's, or its first rise is before it";
    const TIMER: &str = "two outputs of one timer name different anchors or CEN rises";
    const NOT_DSHOT: &str = "a judge's stop names no DShot timer, or one twice";
    const STOP_AT: &str = "a judge's stop is not at its timer's witnesses' anchor plus DSHOT_WINDOW_NS, inside the boot";
    const ORDER: &str = "the verdict's serial_wire_past_run or motor_anchor_out_of_order does not match the boot word's instant";
    let verdict = |r: &[Record]| check_run(r).map(|s| s.matched);
    let base = run("witnessed", true, true, vec![]);
    let both = |f: &dyn Fn(&mut Record)| {
        let mut r = base.clone();
        f(&mut r[7]);
        f(&mut r[8]);
        r
    };
    // An anchor at the run's end: the window is the board's (PWM), so only
    // "the anchor is before its window's end" refuses it; nothing is
    // witnessed, so no first rise is before it.
    let at_end = both(&|w| {
        if let Record::MotorWitness {
            status,
            rises,
            falls,
            first_ns,
            last_ns,
            span_ns,
            off_grid,
            anchor_ns,
            ..
        } = w
        {
            (*status, *rises, *falls, *off_grid) = ("unwitnessed".into(), 0, 0, 0);
            (*first_ns, *last_ns, *span_ns) = (None, None, None);
            *anchor_ns = Some(100_000_000);
        }
    });
    assert_eq!(verdict(&at_end), Err(WINDOW));
    // A first rise before its anchor, the window itself right.
    let before = both(&|w| {
        if let Record::MotorWitness { anchor_ns, .. } = w {
            *anchor_ns = Some(FIRST + 1);
        }
    });
    assert_eq!(verdict(&before), Err(WINDOW));
    // Two outputs of TIM1 naming different counts of its CEN rises.
    let mut rises = base.clone();
    if let Record::MotorWitness { cen_rises, .. } = &mut rises[8] {
        *cen_rises = 2;
    }
    assert_eq!(verdict(&rises), Err(TIMER));
    // A stop of a PWM timer, anchored where its witnesses are, at the
    // window's length after it: it names no DShot timer.
    let a = 30_670;
    let end = a + 2_000_000;
    let ok = dshot_run(Some(a), Some((end, 0x80)), vec![]);
    assert_eq!(verdict(&ok), Ok(true));
    let mut pwm = ok.clone();
    if let Record::JudgeStop {
        timer,
        anchor_ns,
        at_ns,
        read_back,
        ..
    } = &mut pwm[DSHOT_STOP]
    {
        *timer = "TIM1".into();
        (*anchor_ns, *at_ns) = (PWM_ANCHOR, PWM_ANCHOR + 2_000_000);
        read_back.truncate(1);
    }
    assert_eq!(verdict(&pwm), Err(NOT_DSHOT));
    // A stop of another anchor than its timer's witnesses', at its own
    // anchor plus the window.
    let mut other = ok.clone();
    if let Record::JudgeStop {
        anchor_ns, at_ns, ..
    } = &mut other[DSHOT_STOP]
    {
        (*anchor_ns, *at_ns) = (a - 1_000, end - 1_000);
    }
    assert_eq!(verdict(&other), Err(STOP_AT));
    // A stop past the boot: the anchor at 99 ms, its window's end at 101 ms.
    let late = 99_000_000;
    let mut past = dshot_run(
        Some(late),
        Some((late + 2_000_000, 0x80)),
        vec![(MOTOR_WINDOW_PAST_RUN_FINDING, 1)],
    );
    if let Record::MotorWitness {
        status,
        rises,
        falls,
        last_ns,
        off_grid,
        ..
    } = &mut past[DSHOT_WITNESS]
    {
        (*status, *rises, *falls, *off_grid) = ("stopped".into(), 603, 603, 1205);
        *last_ns = Some(late + DSHOT_LEAD + 602 * DSHOT_PERIOD);
    }
    past[0] = scenario(
        false,
        &[MOTOR_MISMATCH_FINDING, MOTOR_WINDOW_PAST_RUN_FINDING],
    );
    let n = past.len() - 1;
    past[n] = Record::Verdict {
        pass: false,
        findings: vec![
            (MOTOR_MISMATCH_FINDING.into(), "error".into(), 1),
            (MOTOR_WINDOW_PAST_RUN_FINDING.into(), "error".into(), 1),
        ],
    };
    assert_eq!(verdict(&past), Err(STOP_AT));
    // Two outputs of one timer, both anchored: `motor_anchor_out_of_order`
    // at most once (per timer, not per output), and at error severity.
    let named = |n: u64, severity: &str| {
        verdict(&run(
            "witnessed",
            true,
            true,
            vec![(MOTOR_ANCHOR_ORDER_FINDING, severity, n)],
        ))
    };
    assert_eq!(named(1, "error"), Ok(true));
    assert_eq!(named(2, "error"), Err(ORDER));
    assert_eq!(named(1, "info"), Err(ORDER));
}

/// `AET-WO-1211c` (trace 18): kind 24 carries the declared idle and tick, and
/// replay re-judges kind 25 as `check_silicon` has since `AET-WO-1211a`: an
/// output idling low at CCR 0 holds with a high time of 0, one idling high
/// at ARR + 1 holds with a high time of the whole period, and a kind 25
/// whose figures are not its idle's is refused.
#[test]
fn replay_re_derives_the_static_check_from_the_declared_idle() {
    use aethelgard_chaos::record::MotorIdle;
    let at_idle = |idle: MotorIdle, ccr: u32, high_ps: u64| {
        let mut r = run("witnessed", true, true, vec![]);
        if let Record::MotorOutputSolved {
            idle: i, ccr: c, ..
        } = &mut r[3]
        {
            *i = Some(idle);
            *c = ccr;
        }
        if let Record::MotorTimingCheck {
            high_ps: h,
            nominal_high_ps: n,
            ..
        } = &mut r[5]
        {
            *h = high_ps;
            *n = high_ps;
        }
        r
    };
    let judged = |r: &[Record]| {
        let solved = r[3].clone();
        let check = r[5].clone();
        // The joins of kinds 24 and 25 alone: the witness of M2 is the
        // pulse's, which a level output does not have, so only the static
        // check's refusal is read here.
        check_run(r)
            .err()
            .filter(|e| *e == CHECK_REFUSED)
            .map(|_| (solved, check))
    };
    assert!(judged(&at_idle(MotorIdle::Low, 0, 0)).is_none());
    assert!(judged(&at_idle(MotorIdle::High, 2500, 2_500_000_000)).is_none());
    assert!(judged(&at_idle(MotorIdle::Low, 0, 1)).is_some());
    assert!(judged(&at_idle(MotorIdle::High, 2500, 0)).is_some());
    assert!(judged(&at_idle(MotorIdle::PulseUs(1000), 1000, 0)).is_some());
}
