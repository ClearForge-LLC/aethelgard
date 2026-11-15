//! `AET-WO-4300b` §1.5: kinds 27 to 29 (a solved serial port, its static
//! framing judgement, its twin witness) through the archive, their field
//! rules, `check_run`'s joins and its re-derivation of the verdict's serial
//! findings from them, and `check_archived`'s frame.

mod common;

use aethelgard_chaos::campaign::{
    ARTIFACTS, SERIAL_ECHO_FINDING, SERIAL_FORGED_FINDING, SERIAL_PATTERN_FINDING,
    SERIAL_STATIC_FINDING, SERIAL_TAG_FINDING, check_archived, check_run, check_serial_frames,
    released, run_id, serial_witness_findings,
};
use aethelgard_chaos::record::{
    Reading, Record, SerialCapture, SerialFraming, SerialTags, serial_frame,
};
use aethelgard_chaos::{Event, Header, TraceWriter, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

fn framing(baud: u32, parity: &str, stop_bits: u8, inverted: bool) -> SerialFraming {
    SerialFraming {
        baud,
        data_bits: 8,
        parity: parity.into(),
        stop_bits,
        inverted,
    }
}

/// The console: USART1 on PA9/PA10 (AF7), 115200 8N1, `BRR` 556 on 64 MHz.
fn console() -> Record {
    Record::SerialPortSolved {
        label: "CONSOLE".into(),
        index: 0,
        peripheral: "USART1".into(),
        tx: "PA9".into(),
        tx_af: 7,
        rx: "PA10".into(),
        rx_af: 7,
        role: "console".into(),
        framing: framing(115_200, "none", 1, false),
        kernel_hz: 64_000_000,
        brr: 556,
        actual_millibaud: 115_107_913,
        error_ppm: -799,
    }
}

/// SBUS: UART4 on PA0/PA1 (AF8), 100000 8E2 inverted, `BRR` 640 exactly.
fn sbus() -> Record {
    Record::SerialPortSolved {
        label: "SBUS".into(),
        index: 1,
        peripheral: "UART4".into(),
        tx: "PA0".into(),
        tx_af: 8,
        rx: "PA1".into(),
        rx_af: 8,
        role: "rc_sbus".into(),
        framing: framing(100_000, "even", 2, true),
        kernel_hz: 64_000_000,
        brr: 640,
        actual_millibaud: 100_000_000,
        error_ppm: 0,
    }
}

fn tags(warnings: u64) -> Option<SerialTags> {
    Some(SerialTags {
        warnings,
        missing: vec![],
        unexpected: vec![],
    })
}

/// Each port's framing check as its solved port gives it, the tags read and
/// holding.
fn check_console() -> Record {
    Record::SerialFramingCheck {
        label: "CONSOLE".into(),
        kernel_hz: 64_000_000,
        brr: 556,
        actual_millibaud: 115_107_913,
        error_ppm: -799,
        tolerance: Some(("Table 407".into(), 0, 33_300)),
        holds: true,
        tags_needed: vec![],
        tags: tags(0),
    }
}

fn check_sbus() -> Record {
    Record::SerialFramingCheck {
        label: "SBUS".into(),
        kernel_hz: 64_000_000,
        brr: 640,
        actual_millibaud: 100_000_000,
        error_ppm: 0,
        tolerance: Some(("Table 406".into(), 1, 34_100)),
        holds: true,
        tags_needed: vec!["CR1.M0".into(), "CR2.RXINV".into(), "CR2.TXINV".into()],
        tags: tags(3),
    }
}

/// A witnessed port: its pattern once, the whole frame, both pins on `af`.
fn witness(label: &str, af: u8) -> Record {
    Record::SerialWitness {
        label: label.into(),
        capture: Some(SerialCapture {
            patterns: 1,
            cut: 0,
            echoed: 8,
            stray: false,
            len: (label.len() + 5 + 8) as u64,
        }),
        frame: [0x5A; 8],
        echo: "witnessed".into(),
        rx_at_injection: Some((0b10, af)),
        tx_at_end: Some((0b10, af)),
    }
}

fn boot(booted: bool) -> Record {
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: Reading {
            address: 0x2000_0000,
            mask: u32::MAX,
            expected: BOOT_WORD,
            observed: Some(if booted { BOOT_WORD } else { 0 }),
        },
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(0x0800_0100),
        executable: vec![(0x0800_0000, 0x0801_0000)],
    }
}

/// The positions of the records [`run`] builds.
const CONSOLE: usize = 2;
const BOOT: usize = 4;
const CHECK_CONSOLE: usize = 5;
const CHECK_SBUS: usize = 6;
const WITNESS_CONSOLE: usize = 7;
const WITNESS_SBUS: usize = 8;

/// A booted run of the two ports, both witnessed, passing.
fn run() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "serial".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        console(),
        sbus(),
        boot(true),
        check_console(),
        check_sbus(),
        witness("CONSOLE", 7),
        witness("SBUS", 8),
        verdict(&[]),
    ]
}

fn verdict(findings: &[(&str, u64)]) -> Record {
    Record::Verdict {
        pass: findings.is_empty(),
        findings: {
            let mut f: Vec<(String, String, u64)> = findings
                .iter()
                .map(|(k, n)| ((*k).to_owned(), "error".to_owned(), *n))
                .collect();
            f.sort();
            f
        },
    }
}

/// `r` with its verdict naming `findings`, and its scenario expecting them.
fn judged(mut r: Vec<Record>, findings: &[(&str, u64)]) -> Vec<Record> {
    let last = r.len() - 1;
    r[last] = verdict(findings);
    let mut kinds: Vec<String> = findings.iter().map(|(k, _)| (*k).to_owned()).collect();
    kinds.sort();
    kinds.dedup();
    r[0] = Record::Scenario {
        name: "serial".into(),
        sha256: [1; 32],
        expect_pass: kinds.is_empty(),
        expect_findings: kinds,
        overrides: vec![],
    };
    r
}

fn edit(f: impl FnOnce(&mut Vec<Record>)) -> Vec<Record> {
    let mut r = run();
    f(&mut r);
    r
}

fn set_witness(
    r: &mut [Record],
    at: usize,
    f: impl FnOnce(
        &mut Option<SerialCapture>,
        &mut String,
        &mut Option<(u8, u8)>,
        &mut Option<(u8, u8)>,
    ),
) {
    let Record::SerialWitness {
        capture,
        echo,
        rx_at_injection,
        tx_at_end,
        ..
    } = &mut r[at]
    else {
        panic!("not a witness");
    };
    f(capture, echo, rx_at_injection, tx_at_end);
}

fn ok(r: &[Record]) -> bool {
    check_run(r).is_ok_and(|s| s.matched)
}

#[test]
fn the_serial_records_round_trip_through_the_archive() {
    let records = run();
    let events: Vec<Event> = records
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("serial-roundtrip", 9, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, records);
    let kinds: Vec<u64> = records[2..9].iter().map(Record::kind).collect();
    assert_eq!(kinds, [27, 27, 10, 28, 28, 29, 29]);
    assert!(records[CONSOLE].before_boot());
    assert!(!records[CHECK_CONSOLE].before_boot() && !records[WITNESS_CONSOLE].before_boot());
}

#[test]
fn a_witnessed_run_passes_and_each_failure_needs_its_own_finding() {
    assert!(ok(&run()));
    // Each witness, forged as the judge would read a failing port, with the
    // findings the judge gives it.
    type Forge =
        fn(&mut Option<SerialCapture>, &mut String, &mut Option<(u8, u8)>, &mut Option<(u8, u8)>);
    let cases: [(&str, Forge, &[&str]); 7] = [
        (
            "no echo",
            |c, e, _, _| {
                *c = Some(SerialCapture {
                    patterns: 1,
                    cut: 0,
                    echoed: 0,
                    stray: false,
                    len: 12,
                });
                *e = "missing".into();
            },
            &[SERIAL_ECHO_FINDING],
        ),
        (
            "part of the echo",
            |c, e, _, _| {
                *c = Some(SerialCapture {
                    patterns: 1,
                    cut: 0,
                    echoed: 3,
                    stray: false,
                    len: 15,
                });
                *e = "partial".into();
            },
            &[SERIAL_ECHO_FINDING],
        ),
        (
            "the RX pin off its AF at the injection",
            |_, e, rx, _| {
                *rx = Some((0b10, 8));
                *e = "not_counted".into();
            },
            &[SERIAL_ECHO_FINDING],
        ),
        (
            "the TX pin off its AF at the end",
            |_, _, _, tx| *tx = Some((0b11, 7)),
            &[SERIAL_PATTERN_FINDING],
        ),
        (
            "no capture",
            |c, e, _, _| {
                *c = None;
                *e = "missing".into();
            },
            &[SERIAL_FORGED_FINDING],
        ),
        (
            "stray bytes",
            |c, e, _, _| {
                *c = Some(SerialCapture {
                    patterns: 1,
                    cut: 0,
                    echoed: 0,
                    stray: true,
                    len: 21,
                });
                *e = "missing".into();
            },
            &[SERIAL_ECHO_FINDING, SERIAL_FORGED_FINDING],
        ),
        (
            "two patterns in one boot",
            |c, _, _, _| {
                *c = Some(SerialCapture {
                    patterns: 2,
                    cut: 0,
                    echoed: 8,
                    stray: false,
                    len: 32,
                });
            },
            &[SERIAL_FORGED_FINDING],
        ),
    ];
    for (what, forge, kinds) in cases {
        let r = edit(|r| set_witness(r, WITNESS_CONSOLE, forge));
        let findings: Vec<(&str, u64)> = kinds.iter().map(|k| (*k, 1)).collect();
        assert!(ok(&judged(r.clone(), &findings)), "{what}");
        // The verdict that drops one, names it twice, or names another
        // kind is refused: the records explain the findings exactly.
        assert!(check_run(&judged(r.clone(), &[])).is_err(), "{what}");
        for k in kinds.iter().filter(|k| **k != SERIAL_FORGED_FINDING) {
            let twice: Vec<(&str, u64)> =
                kinds.iter().map(|x| (*x, 1 + u64::from(x == k))).collect();
            assert!(check_run(&judged(r.clone(), &twice)).is_err(), "{what}");
        }
        let mut other = findings.clone();
        other.push((SERIAL_TAG_FINDING, 1));
        assert!(check_run(&judged(r, &other)).is_err(), "{what}");
    }
}

/// The ruling on C1's parser: a pattern cut short counts as a boot, so it
/// passes only where a released reset gave the boot it was cut from; an
/// unreleased one gives none.
#[test]
fn a_cut_pattern_needs_a_released_reset() {
    let cut = edit(|r| {
        set_witness(r, WITNESS_CONSOLE, |c, _, _, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 1,
                echoed: 8,
                stray: false,
                len: 23,
            });
        });
    });
    assert!(ok(&judged(cut.clone(), &[(SERIAL_FORGED_FINDING, 1)])));
    assert!(check_run(&judged(cut.clone(), &[])).is_err());
    let reset = |released: bool| Record::ResetRendered {
        at_ns: 1_000_000,
        released,
        cause: vec!["VBOR3".into()],
        rendering: "machine Reset".into(),
    };
    let w = &cut[WITNESS_CONSOLE];
    for (resets, findings) in [
        (vec![], vec![SERIAL_FORGED_FINDING]),
        (vec![reset(false)], vec![SERIAL_FORGED_FINDING]),
        (vec![reset(true)], vec![]),
        (vec![reset(true), reset(true)], vec![]),
    ] {
        let releases = released(&resets);
        assert_eq!(
            serial_witness_findings(&console(), w, releases),
            findings,
            "{resets:?}"
        );
    }
}

#[test]
fn a_witness_and_tags_are_judged_only_once_the_boot_word_is_set() {
    let not_booted = |echo: &str, findings: &[(&str, u64)]| {
        let r = edit(|r| {
            r[BOOT] = boot(false);
            for at in [WITNESS_CONSOLE, WITNESS_SBUS] {
                let echo = echo.to_owned();
                set_witness(r, at, |c, e, _, _| {
                    *c = None;
                    *e = echo;
                });
            }
            if let Record::SerialFramingCheck { tags, .. } = &mut r[CHECK_SBUS] {
                *tags = None;
            }
        });
        let mut f = vec![("boot_word_missing", 1)];
        f.extend_from_slice(findings);
        judged(r, &f)
    };
    // No capture, no tags, nothing judged: no port's serial finding is due
    // (a foreign file in the capture directory is forged booted or not, so
    // `serial_capture_forged` is bounded from below only: its own test).
    assert!(check_run(&not_booted("not_judged", &[])).is_ok());
    for f in [
        SERIAL_TAG_FINDING,
        SERIAL_ECHO_FINDING,
        SERIAL_PATTERN_FINDING,
    ] {
        assert!(
            check_run(&not_booted("not_judged", &[(f, 1)])).is_err(),
            "{f}"
        );
    }
    // An echo judged without the boot word, or not judged with it, is not
    // the judge's.
    assert!(check_run(&not_booted("missing", &[])).is_err());
    let r = edit(|r| set_witness(r, WITNESS_CONSOLE, |_, e, _, _| *e = "not_judged".into()));
    assert!(check_run(&r).is_err());
    // Without the boot word the witnesses may be absent; with it, never.
    let mut r = not_booted("not_judged", &[]);
    r.drain(WITNESS_CONSOLE..=WITNESS_SBUS);
    assert!(check_run(&r).is_ok());
    let r = edit(|r| {
        r.drain(WITNESS_CONSOLE..=WITNESS_SBUS);
    });
    assert!(check_run(&r).is_err());
}

#[test]
fn a_port_s_tags_fail_when_missing_unexpected_or_unread() {
    for tags in [
        None,
        Some(SerialTags {
            warnings: 2,
            missing: vec!["CR1.M0".into()],
            unexpected: vec![],
        }),
        Some(SerialTags {
            warnings: 4,
            missing: vec![],
            unexpected: vec!["SWAP (0x1) at 0x4".into()],
        }),
    ] {
        let r = edit(|r| {
            if let Record::SerialFramingCheck { tags: t, .. } = &mut r[CHECK_SBUS] {
                *t = tags.clone();
            }
        });
        assert!(
            ok(&judged(r.clone(), &[(SERIAL_TAG_FINDING, 1)])),
            "{tags:?}"
        );
        assert!(check_run(&judged(r, &[])).is_err(), "{tags:?}");
    }
}

/// `BRR` 557 for 115200 on 64 MHz: inside the tolerance, but not the
/// nearest whole `USARTDIV`, so silicon's check fails.
fn console_557() -> (Record, Record) {
    let Record::SerialPortSolved {
        label,
        index,
        peripheral,
        tx,
        tx_af,
        rx,
        rx_af,
        role,
        framing,
        kernel_hz,
        ..
    } = console()
    else {
        unreachable!()
    };
    let port = Record::SerialPortSolved {
        label,
        index,
        peripheral,
        tx,
        tx_af,
        rx,
        rx_af,
        role,
        framing,
        kernel_hz,
        brr: 557,
        actual_millibaud: 114_901_256,
        error_ppm: -2593,
    };
    let check = Record::SerialFramingCheck {
        label: "CONSOLE".into(),
        kernel_hz: 64_000_000,
        brr: 557,
        actual_millibaud: 114_901_256,
        error_ppm: -2593,
        tolerance: Some(("Table 407".into(), 0, 33_300)),
        holds: false,
        tags_needed: vec![],
        tags: tags(0),
    };
    (port, check)
}

#[test]
fn a_failing_static_check_is_named_once_booted_or_not() {
    let (port, check) = console_557();
    for booted in [true, false] {
        let r = edit(|r| {
            r[CONSOLE] = port.clone();
            r[CHECK_CONSOLE] = check.clone();
            if !booted {
                r[BOOT] = boot(false);
                r.drain(WITNESS_CONSOLE..=WITNESS_SBUS);
            }
        });
        let mut f = vec![(SERIAL_STATIC_FINDING, 1)];
        if !booted {
            f.push(("boot_word_missing", 1));
        }
        assert!(check_run(&judged(r.clone(), &f)).is_ok(), "booted {booted}");
        f.retain(|x| x.0 != SERIAL_STATIC_FINDING);
        assert!(check_run(&judged(r, &f)).is_err(), "booted {booted}");
    }
}

#[test]
fn replay_re_derives_the_static_check_from_the_solved_port() {
    let (port, check) = console_557();
    let base = edit(|r| {
        r[CONSOLE] = port.clone();
        r[CHECK_CONSOLE] = check.clone();
    });
    let base = judged(base, &[(SERIAL_STATIC_FINDING, 1)]);
    assert!(ok(&base));
    let forge = |f: &dyn Fn(&mut Record)| {
        let mut r = base.clone();
        f(&mut r[CHECK_CONSOLE]);
        check_run(&r).map(|_| ())
    };
    macro_rules! forged {
        ($field:ident => $value:expr) => {
            forge(&|r| {
                if let Record::SerialFramingCheck { $field, .. } = r {
                    *$field = $value;
                }
            })
            .is_err()
        };
    }
    // A forged `holds` beside its own failing figures.
    assert!(forged!(holds => true));
    // Figures that are not the port's.
    assert!(forged!(brr => 556));
    assert!(forged!(actual_millibaud => 114_901_257));
    assert!(forged!(error_ppm => -2592));
    // Another row of the table, or none.
    assert!(forged!(tolerance => Some(("Table 406".into(), 0, 37_500))));
    assert!(forged!(tolerance => None));
    // A tag the framing does not need.
    assert!(forged!(tags_needed => vec!["CR1.M0".into()]));
    // Another kernel clock than the port's.
    assert!(forged!(kernel_hz => 32_000_000));
    // And SBUS's tags are what its framing needs: none dropped.
    let r = edit(|r| {
        if let Record::SerialFramingCheck { tags_needed, .. } = &mut r[CHECK_SBUS] {
            tags_needed.pop();
        }
    });
    assert!(check_run(&r).is_err());
}

#[test]
fn a_port_whose_kernel_clock_is_not_the_reset_clock_fails_silicon() {
    // A consistent port and check on a 32 MHz kernel clock: the figures are
    // right for it, but the only clock a solved topology admits is 64 MHz.
    let port = Record::SerialPortSolved {
        label: "CONSOLE".into(),
        index: 0,
        peripheral: "USART1".into(),
        tx: "PA9".into(),
        tx_af: 7,
        rx: "PA10".into(),
        rx_af: 7,
        role: "console".into(),
        framing: framing(115_200, "none", 1, false),
        kernel_hz: 32_000_000,
        brr: 278,
        actual_millibaud: 115_107_913,
        error_ppm: -799,
    };
    let mut check = check_console();
    if let Record::SerialFramingCheck { kernel_hz, brr, .. } = &mut check {
        (*kernel_hz, *brr) = (32_000_000, 278);
    }
    let r = edit(|r| {
        r[CONSOLE] = port;
        r[CHECK_CONSOLE] = check.clone();
    });
    assert!(check_run(&r).is_err(), "holds on another kernel clock");
    if let Record::SerialFramingCheck { holds, .. } = &mut check {
        *holds = false;
    }
    let mut r = r;
    r[CHECK_CONSOLE] = check;
    assert!(ok(&judged(r, &[(SERIAL_STATIC_FINDING, 1)])));
}

/// The baud and the error a `BRR` gives are re-derived, not only matched
/// between the port and its check: both forged alike are refused.
#[test]
fn a_port_and_its_check_forged_alike_are_refused() {
    for (millibaud, ppm) in [(115_107_914, -799), (115_107_913, -800)] {
        let r = edit(|r| {
            for at in [CONSOLE, CHECK_CONSOLE] {
                match &mut r[at] {
                    Record::SerialPortSolved {
                        actual_millibaud,
                        error_ppm,
                        ..
                    }
                    | Record::SerialFramingCheck {
                        actual_millibaud,
                        error_ppm,
                        ..
                    } => (*actual_millibaud, *error_ppm) = (millibaud, ppm),
                    _ => unreachable!(),
                }
            }
        });
        assert!(check_run(&r).is_err(), "{millibaud} {ppm}");
    }
}

#[test]
fn replay_ties_each_echo_to_its_capture_and_rx_pin() {
    let forge = |f: Box<
        dyn FnOnce(
            &mut Option<SerialCapture>,
            &mut String,
            &mut Option<(u8, u8)>,
            &mut Option<(u8, u8)>,
        ),
    >| {
        let r = edit(|r| set_witness(r, WITNESS_CONSOLE, |c, e, rx, tx| f(c, e, rx, tx)));
        check_run(&r).map(|_| ())
    };
    // Witnessed with the RX pin off its AF, or not read at the injection.
    assert!(forge(Box::new(|_, _, rx, _| *rx = Some((0b10, 8)))).is_err());
    assert!(forge(Box::new(|_, _, rx, _| *rx = Some((0b11, 7)))).is_err());
    assert!(forge(Box::new(|_, _, rx, _| *rx = None)).is_err());
    // Witnessed with part of the frame, or with stray bytes.
    assert!(
        forge(Box::new(|c, _, _, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 7,
                stray: false,
                len: 19,
            });
        }))
        .is_err()
    );
    assert!(
        forge(Box::new(|c, _, _, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 0,
                stray: true,
                len: 20,
            });
        }))
        .is_err()
    );
    // Not counted with the RX pin on its AF; partial with the whole frame;
    // missing with the whole frame back on a routed pin.
    for status in ["not_counted", "partial", "missing"] {
        assert!(
            forge(Box::new(move |_, e, _, _| *e = status.into())).is_err(),
            "{status}"
        );
    }
}

#[test]
fn a_serial_record_that_joins_no_solved_port_is_refused() {
    assert!(check_run(&run()).is_ok());
    let refused = |f: &dyn Fn(&mut Vec<Record>)| {
        let mut r = run();
        f(&mut r);
        check_run(&r).is_err()
    };
    // A check or a witness of a port nothing solved.
    assert!(refused(&|r| {
        if let Record::SerialFramingCheck { label, .. } = &mut r[CHECK_SBUS] {
            *label = "GPS".into();
        }
    }));
    assert!(refused(&|r| {
        if let Record::SerialWitness { label, .. } = &mut r[WITNESS_SBUS] {
            *label = "GPS".into();
        }
    }));
    // A port with no check; a check twice; the checks out of the ports' order.
    assert!(refused(&|r| {
        r.remove(CHECK_SBUS);
    }));
    assert!(refused(&|r| r[CHECK_SBUS] = check_console()));
    assert!(refused(&|r| r.swap(CHECK_CONSOLE, CHECK_SBUS)));
    // A witness twice, one missing, or out of order.
    assert!(refused(&|r| r[WITNESS_SBUS] = witness("CONSOLE", 7)));
    assert!(refused(&|r| {
        r.remove(WITNESS_SBUS);
    }));
    assert!(refused(&|r| r.swap(WITNESS_CONSOLE, WITNESS_SBUS)));
    // Two ports on one USART, one pin, or one label, or out of the
    // specification's order.
    macro_rules! sbus_as {
        ($field:ident => $value:expr) => {
            refused(&|r| {
                if let Record::SerialPortSolved { $field, .. } = &mut r[CONSOLE + 1] {
                    *$field = $value;
                }
            })
        };
    }
    assert!(sbus_as!(peripheral => "USART1".into()));
    assert!(sbus_as!(tx => "PA10".into()));
    assert!(sbus_as!(label => "CONSOLE".into()));
    assert!(sbus_as!(index => 0));
    // A witness after the verdict, a check before the boot.
    assert!(refused(&|r| {
        let w = r.remove(WITNESS_SBUS);
        r.push(w);
    }));
    assert!(refused(&|r| r.swap(BOOT, CHECK_CONSOLE)));
}

#[test]
fn serial_capture_forged_is_bounded_from_below_and_never_info() {
    // A foreign file in the capture directory names no port: the verdict may
    // name more forged captures than the witnesses give, never fewer.
    let r = edit(|_| {});
    assert!(check_run(&judged(r.clone(), &[(SERIAL_FORGED_FINDING, 1)])).is_ok());
    let forged = edit(|r| {
        set_witness(r, WITNESS_CONSOLE, |c, e, _, _| {
            *c = None;
            *e = "missing".into();
        });
    });
    assert!(check_run(&judged(forged.clone(), &[(SERIAL_FORGED_FINDING, 2)])).is_ok());
    assert!(check_run(&judged(forged, &[])).is_err());
    // A run with no port has no capture directory, so none.
    let no_ports: Vec<Record> = run()
        .into_iter()
        .filter(|r| !matches!(r.kind(), 27..=29))
        .collect();
    assert!(check_run(&no_ports).is_ok());
    assert!(check_run(&judged(no_ports, &[(SERIAL_FORGED_FINDING, 1)])).is_err());
    // Any serial finding at `info` is the wrong severity.
    let mut r = run();
    let last = r.len() - 1;
    r[last] = Record::Verdict {
        pass: true,
        findings: vec![(SERIAL_FORGED_FINDING.into(), "info".into(), 1)],
    };
    assert!(check_run(&r).is_err());
}

#[test]
fn the_field_rules_hold_on_write_and_read() {
    let port = |f: &dyn Fn(&mut Record)| {
        let mut r = console();
        f(&mut r);
        r
    };
    let mut bad = vec![
        port(&|r| {
            if let Record::SerialPortSolved { rx, .. } = r {
                *rx = "PA9".into()
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { tx, .. } = r {
                *tx = "PA09".into()
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { rx_af, .. } = r {
                *rx_af = 16
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { role, .. } = r {
                *role = "telemetry".into()
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { framing, .. } = r {
                framing.parity = "mark".into()
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { framing, .. } = r {
                framing.data_bits = 9;
                framing.parity = "odd".into()
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { framing, .. } = r {
                framing.stop_bits = 3
            }
        }),
        port(&|r| {
            if let Record::SerialPortSolved { brr, .. } = r {
                *brr = 15
            }
        }),
    ];
    let check = |f: &dyn Fn(&mut Record)| {
        let mut r = check_sbus();
        f(&mut r);
        r
    };
    bad.extend([
        check(&|r| {
            if let Record::SerialFramingCheck { tags_needed, .. } = r {
                tags_needed.reverse()
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tags_needed, .. } = r {
                tags_needed.push("CR2.TXINV".into())
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tags_needed, .. } = r {
                tags_needed[0] = "CR1.UE".into()
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tags, .. } = r {
                *tags = Some(SerialTags {
                    warnings: 0,
                    missing: vec!["CR1.M1".into()],
                    unexpected: vec![],
                })
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tags, .. } = r {
                *tags = Some(SerialTags {
                    warnings: 0,
                    missing: vec![],
                    unexpected: vec!["x".into(); 17],
                })
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tolerance, .. } = r {
                *tolerance = Some(("Table 406".into(), 3, 1))
            }
        }),
    ]);
    let w = |f: &dyn Fn(&mut Record)| {
        let mut r = witness("CONSOLE", 7);
        f(&mut r);
        r
    };
    bad.extend([
        w(&|r| {
            if let Record::SerialWitness { echo, .. } = r {
                *echo = "fine".into()
            }
        }),
        w(&|r| {
            if let Record::SerialWitness {
                rx_at_injection, ..
            } = r
            {
                *rx_at_injection = Some((4, 7))
            }
        }),
        w(&|r| {
            if let Record::SerialWitness { tx_at_end, .. } = r {
                *tx_at_end = Some((2, 16))
            }
        }),
        // Counts that do not fit the length: a byte short, a byte over, more
        // of the frame than it has, a stray capture with a cut.
        w(&|r| {
            if let Record::SerialWitness {
                capture: Some(c), ..
            } = r
            {
                c.len -= 1
            }
        }),
        w(&|r| {
            if let Record::SerialWitness {
                capture: Some(c), ..
            } = r
            {
                c.len += 1
            }
        }),
        w(&|r| {
            if let Record::SerialWitness {
                capture: Some(c), ..
            } = r
            {
                c.echoed = 9;
                c.len = 21
            }
        }),
        w(&|r| {
            if let Record::SerialWitness {
                capture: Some(c), ..
            } = r
            {
                c.stray = true;
                c.cut = 1
            }
        }),
    ]);
    for r in &bad {
        assert!(r.check().is_err(), "{r:?}");
    }
    for r in [
        console(),
        sbus(),
        check_console(),
        check_sbus(),
        witness("CONSOLE", 7),
    ] {
        assert!(r.check().is_ok(), "{r:?}");
    }
    // A cut pattern is 1 to P - 1 bytes: both ends fit.
    for len in [20 + 1, 20 + 11] {
        let r = w(&|r| {
            if let Record::SerialWitness {
                capture: Some(c), ..
            } = r
            {
                c.cut = 1;
                c.len = len
            }
        });
        assert!(r.check().is_ok(), "{len}");
    }
    let r = w(&|r| {
        if let Record::SerialWitness {
            capture: Some(c), ..
        } = r
        {
            c.cut = 1;
            c.len = 20 + 12
        }
    });
    assert!(r.check().is_err());
}

#[test]
fn the_frame_is_the_domain_s_hash_of_the_firmware_and_the_label() {
    let fw = [0x11u8; 32];
    let mut h = Sha256::new();
    h.update(b"aethelgard-serial-frame");
    h.update(fw);
    h.update(b"CONSOLE");
    assert_eq!(serial_frame(&fw, "CONSOLE")[..], h.finalize()[..8]);
    assert_ne!(serial_frame(&fw, "CONSOLE"), serial_frame(&fw, "SBUS"));
    assert_ne!(
        serial_frame(&fw, "CONSOLE"),
        serial_frame(&[0x12; 32], "CONSOLE")
    );
}

/// `check_archived` re-derives each witness's frame from the header's
/// firmware: a frame copied from another firmware or port is refused.
#[test]
fn an_archived_witness_must_carry_its_firmware_s_frame() {
    const SCEN: [u8; 32] = [1; 32];
    const SPEC: [u8; 32] = [2; 32];
    const TOPO: [u8; 32] = [3; 32];
    const FW: [u8; 32] = [4; 32];
    let header = Header {
        run_id: run_id(&SCEN, &SPEC, &FW),
        artifacts: ARTIFACTS
            .iter()
            .zip([[5; 32], FW, SCEN, SPEC, TOPO])
            .map(|(n, h)| ((*n).to_owned(), h))
            .collect(),
        tools: vec![
            ("aethelgard".into(), "0.1.0".into()),
            ("renode".into(), "1.17.0".into()),
        ],
    };
    let archived = |name: &str, frames: [[u8; 8]; 2]| {
        let mut records = run();
        for (at, f) in [WITNESS_CONSOLE, WITNESS_SBUS].into_iter().zip(frames) {
            if let Record::SerialWitness { frame, .. } = &mut records[at] {
                *frame = f;
            }
        }
        let dir = scratch(name).join("run");
        let mut w = TraceWriter::create(&dir, &header).unwrap();
        for record in records {
            let timestamp = if record.before_boot() { 0 } else { 100_000_000 };
            w.append(&Event::Record { timestamp, record }).unwrap();
        }
        w.commit().unwrap();
        let r = replay_dir(&dir).unwrap();
        check_archived(&r.header, &r.state).map(|_| ())
    };
    let right = [serial_frame(&FW, "CONSOLE"), serial_frame(&FW, "SBUS")];
    assert_eq!(archived("frame-right", right), Ok(()));
    for (name, frames) in [
        (
            "frame-other-fw",
            [serial_frame(&[6; 32], "CONSOLE"), right[1]],
        ),
        ("frame-other-port", [right[0], right[0]]),
    ] {
        assert_eq!(
            archived(name, frames),
            Err("a serial witness's frame is not the one its firmware and label give"),
            "{name}"
        );
    }
    assert!(check_serial_frames(&FW, &run()).is_err());
}
