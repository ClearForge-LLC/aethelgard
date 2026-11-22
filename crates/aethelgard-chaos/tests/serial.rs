//! `AET-WO-4300b` §1.5: kinds 27 to 29 (a solved serial port, its static
//! framing judgement, its twin witness) through the archive, their field
//! rules, `check_run`'s joins and its re-derivation of the verdict's serial
//! findings from them, and `check_archived`'s frame.

mod common;

use aethelgard_chaos::campaign::{
    ARTIFACTS, MOTOR_ANCHOR_ORDER_FINDING, SERIAL_ECHO_FINDING, SERIAL_FORGED_FINDING,
    SERIAL_PATTERN_FINDING, SERIAL_STATIC_FINDING, SERIAL_TAG_FINDING,
    SERIAL_WIRE_PAST_RUN_FINDING, check_archived, check_run, check_serial_frames, released, run_id,
    serial_witness_findings,
};
use aethelgard_chaos::record::{
    MAX_SERIAL_UNEXPECTED, MAX_SERIAL_UNEXPECTED_BYTES, Reading, Record, SerialCapture,
    SerialFraming, SerialTags, serial_frame,
};
use aethelgard_chaos::{Event, Header, TraceWriter, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

/// The refusals the tests below assert by name (`AET-WO-2302b`: a test
/// that asserted only an error could pass on another guard's).
const SERIAL_REFUSED: &str = "the verdict's serial findings do not match the serial records";
const ECHO_REFUSED: &str = "a serial witness's echo is not what its capture and its RX pin give";
const CHECK_REFUSED: &str = "a serial framing check is not what its solved port gives";
const TX_END_REFUSED: &str =
    "a serial witness's TX pin at the end is not what kind 11 read of its MODER and AFR";
const CHECKS_REFUSED: &str =
    "the serial framing checks are not one per solved port, in the ports' order";
const WITNESSES_REFUSED: &str = "the serial witnesses are not one per solved port in the ports' order, or are absent from a run whose boot word is set";
const PORTS_REFUSED: &str = "the solved serial ports are not in the specification's order, each with its own label, USART and pins";
const TAGS_REFUSED: &str = "a serial framing check's tag judgement names a tag the framing does not need as missing, or too many or malformed unexpected ones";
const COUNTS_REFUSED: &str =
    "a serial witness's counts do not fit its capture's length and its pattern's";

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

fn tags(writes: u64) -> Option<SerialTags> {
    Some(SerialTags {
        writes,
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
        tx_at_send: Some((0b10, af)),
        // Port after port (`AET-WO-2302` D4): the console's frame a quantum
        // after the boot word, the next once the console has echoed.
        injected_ns: Some(if label == "CONSOLE" {
            130_750
        } else {
            2_000_000
        }),
    }
}

/// Kind 11's read of `name` (`GPIOA.MODER`): `observed` under its whole word.
fn read(name: &str, address: u32, observed: u32) -> Record {
    Record::RegisterObserved {
        name: name.into(),
        reading: Reading {
            address,
            mask: u32::MAX,
            expected: observed,
            observed: Some(observed),
        },
    }
}

/// The two ports' pins as the image reads them at the end: PA0 and PA1 on
/// AF8 (SBUS), PA9 and PA10 on AF7 (the console), all four in AF mode.
fn gpio_reads() -> [Record; 3] {
    [
        read("GPIOA.MODER", 0x5802_0000, 0x0028_000A),
        read("GPIOA.AFRL", 0x5802_0020, 0x88),
        read("GPIOA.AFRH", 0x5802_0024, 0x770),
    ]
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
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
    }
}

/// The positions of the records [`run`] builds.
const CONSOLE: usize = 2;
const BOOT: usize = 4;
/// Kind 11's read of `GPIOA.MODER`, then `AFRL` and `AFRH`.
const MODER: usize = 5;
const CHECK_CONSOLE: usize = 8;
const CHECK_SBUS: usize = 9;
const WITNESS_CONSOLE: usize = 10;
const WITNESS_SBUS: usize = 11;

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
        gpio_reads()[0].clone(),
        gpio_reads()[1].clone(),
        gpio_reads()[2].clone(),
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
        injected_ns,
        ..
    } = &mut r[at]
    else {
        panic!("not a witness");
    };
    f(capture, echo, rx_at_injection, tx_at_end);
    // The injection's instant goes with the RX pin's read there (trace 13).
    if rx_at_injection.is_none() {
        *injected_ns = None;
    }
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
    let kinds: Vec<u64> = records[2..12].iter().map(Record::kind).collect();
    assert_eq!(kinds, [27, 27, 10, 11, 11, 11, 28, 28, 29, 29]);
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
    let cases: [(&str, Forge, &[&str]); 6] = [
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
        assert_eq!(
            check_run(&judged(r.clone(), &[])).unwrap_err(),
            SERIAL_REFUSED,
            "{what}"
        );
        for k in kinds.iter().filter(|k| **k != SERIAL_FORGED_FINDING) {
            let twice: Vec<(&str, u64)> =
                kinds.iter().map(|x| (*x, 1 + u64::from(x == k))).collect();
            assert_eq!(
                check_run(&judged(r.clone(), &twice)).unwrap_err(),
                SERIAL_REFUSED,
                "{what}"
            );
        }
        let mut other = findings.clone();
        other.push((SERIAL_TAG_FINDING, 1));
        assert_eq!(
            check_run(&judged(r, &other)).unwrap_err(),
            SERIAL_REFUSED,
            "{what}"
        );
    }
}

/// The pattern counts only with the TX pin on its AF at the end, as kind 11
/// read it, and at the first `TDR` write of each boot (adversarial F4); a
/// TX pin at the end that contradicts kind 11's read is refused (F11).
#[test]
fn adversarial_f4_f11_the_tx_pin_counts_at_its_first_send_and_at_the_end() {
    let send = |m: Option<(u8, u8)>| {
        edit(|r| {
            let Record::SerialWitness { tx_at_send, .. } = &mut r[WITNESS_CONSOLE] else {
                panic!("not a witness");
            };
            *tx_at_send = m;
        })
    };
    for m in [Some((0b00, 0)), Some((0b10, 8)), Some((0b11, 7)), None] {
        let r = send(m);
        assert!(
            ok(&judged(r.clone(), &[(SERIAL_PATTERN_FINDING, 1)])),
            "{m:?}"
        );
        assert_eq!(
            check_run(&judged(r, &[])).unwrap_err(),
            SERIAL_REFUSED,
            "{m:?}"
        );
    }
    // PA9 analog at the end, as kind 11 read GPIOA.MODER too.
    let end = edit(|r| {
        r[MODER] = read("GPIOA.MODER", 0x5802_0000, 0x002C_000A);
        set_witness(r, WITNESS_CONSOLE, |_, _, _, tx| *tx = Some((0b11, 7)));
    });
    assert!(ok(&judged(end.clone(), &[(SERIAL_PATTERN_FINDING, 1)])));
    assert_eq!(check_run(&judged(end, &[])).unwrap_err(), SERIAL_REFUSED);
    // ADV-R1: kind 11 reads PA9 in AF mode, kind 29 says it ended an input.
    let contradicted = edit(|r| {
        set_witness(r, WITNESS_CONSOLE, |_, _, _, tx| *tx = Some((0b00, 7)));
    });
    for findings in [&[(SERIAL_PATTERN_FINDING, 1)][..], &[]] {
        let res = check_run(&judged(contradicted.clone(), findings));
        assert_eq!(
            res.as_ref().map(|s| s.matched),
            Err(
                &"a serial witness's TX pin at the end is not what kind 11 read of its MODER and AFR"
            ),
            "{res:?}"
        );
    }
    // The AF read contradicted, or a register kind 11 never read.
    let af = edit(|r| set_witness(r, WITNESS_CONSOLE, |_, _, _, tx| *tx = Some((0b10, 8))));
    assert_eq!(
        check_run(&judged(af, &[(SERIAL_PATTERN_FINDING, 1)])).unwrap_err(),
        TX_END_REFUSED
    );
    let unread = edit(|r| {
        r.remove(MODER + 2);
    });
    assert_eq!(check_run(&unread).unwrap_err(), TX_END_REFUSED);
    // Neither read nor witnessed at the end: tied, and the pattern fails.
    let neither = edit(|r| {
        r.drain(MODER..MODER + 3);
        for w in [WITNESS_CONSOLE - 3, WITNESS_SBUS - 3] {
            set_witness(r, w, |_, _, _, tx| *tx = None);
        }
    });
    assert!(ok(&judged(neither, &[(SERIAL_PATTERN_FINDING, 2)])));
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
    assert_eq!(
        check_run(&judged(cut.clone(), &[])).unwrap_err(),
        SERIAL_REFUSED
    );
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
            if let Record::SerialWitness { tx_at_send, .. } = &mut r[WITNESS_SBUS] {
                *tx_at_send = None;
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
        assert_eq!(
            check_run(&not_booted("not_judged", &[(f, 1)])).unwrap_err(),
            SERIAL_REFUSED,
            "{f}"
        );
    }
    // An echo judged without the boot word, or not judged with it, is not
    // the judge's.
    assert_eq!(
        check_run(&not_booted("missing", &[])).unwrap_err(),
        ECHO_REFUSED
    );
    let r = edit(|r| set_witness(r, WITNESS_CONSOLE, |_, e, _, _| *e = "not_judged".into()));
    assert_eq!(check_run(&r).unwrap_err(), ECHO_REFUSED);
    // Without the boot word the witnesses may be absent; with it, never.
    let mut r = not_booted("not_judged", &[]);
    r.drain(WITNESS_CONSOLE..=WITNESS_SBUS);
    assert!(check_run(&r).is_ok());
    let r = edit(|r| {
        r.drain(WITNESS_CONSOLE..=WITNESS_SBUS);
    });
    assert_eq!(check_run(&r).unwrap_err(), WITNESSES_REFUSED);
}

#[test]
fn a_port_s_tags_fail_when_missing_unexpected_or_unread() {
    // Unread: the access log was not whole, so neither the writes nor the
    // TX pin at its sends were rebuilt, and both fail.
    let unread = edit(|r| {
        if let Record::SerialFramingCheck { tags: t, .. } = &mut r[CHECK_SBUS] {
            *t = None;
        }
        if let Record::SerialWitness { tx_at_send, .. } = &mut r[WITNESS_SBUS] {
            *tx_at_send = None;
        }
    });
    assert!(ok(&judged(
        unread.clone(),
        &[(SERIAL_TAG_FINDING, 1), (SERIAL_PATTERN_FINDING, 1)]
    )));
    // Adversarial pass 2, L1: a TX pin rebuilt from a log the tags could not
    // read is a pair the judge never writes.
    let forged = edit(|r| {
        if let Record::SerialFramingCheck { tags: t, .. } = &mut r[CHECK_SBUS] {
            *t = None;
        }
    });
    for findings in [&[(SERIAL_TAG_FINDING, 1)][..], &[]] {
        let res = check_run(&judged(forged.clone(), findings));
        assert_eq!(
            res.as_ref().map(|s| s.matched),
            Err(
                &"a serial witness's TX pin at its sends is rebuilt from an access log its framing check could not read"
            ),
            "{res:?}"
        );
    }
    for tags in [
        Some(SerialTags {
            writes: 2,
            missing: vec!["CR1.M0".into()],
            unexpected: vec![],
        }),
        // Every form the access log's judge writes (adversarial F1-F3, pass
        // 2 A1, A2, L2), at its longest, through the archive's text rule, at
        // the most it carries.
        Some(SerialTags {
            writes: 4,
            missing: vec!["CR2.RXINV".into(), "CR2.TXINV".into()],
            unexpected: [
                "CR1 0xFFFFFFFF at UE, not 0xFFFFFFFF",
                "the last boot never set CR1.UE",
                "WriteUInt16 to 0xFFFFFFFF",
                "WriteByte to 0x1",
                "TDR before UE",
                "0xFFFFFFFF to 0x2C",
                "CR2 0x32000 after UE",
                "CR1.M1 set at UE",
                "RCC.APB1LRSTR 0x40000 written, not an enable register",
                "WriteUInt16 to RCC 0xFFFFFFFF",
                "PK15 OTYPER 1, OSPEEDR 0b11, PUPDR 0b11 at the injection",
                "PK15 OTYPER/OSPEEDR/PUPDR not rebuilt at the injection",
                "gpioPortK refused a write to 0xFFFFFFFF (a locked pin)",
                "CR2 0xFFFFFFFF set, a bit Renode does not store",
                "PK15 OT set in OTYPER, a bit Renode does not store",
            ]
            .iter()
            .cycle()
            .take(16)
            .map(|x| (*x).to_owned())
            .collect(),
        }),
    ] {
        let r = edit(|r| {
            if let Record::SerialFramingCheck { tags: t, .. } = &mut r[CHECK_SBUS] {
                *t = tags.clone();
            }
        });
        assert_eq!(r[CHECK_SBUS].check(), Ok(()), "{tags:?}");
        assert!(
            ok(&judged(r.clone(), &[(SERIAL_TAG_FINDING, 1)])),
            "{tags:?}"
        );
        assert_eq!(
            check_run(&judged(r, &[])).unwrap_err(),
            SERIAL_REFUSED,
            "{tags:?}"
        );
    }
    // Past the most kind 28 carries, the record is malformed.
    let r = edit(|r| {
        if let Record::SerialFramingCheck { tags: Some(t), .. } = &mut r[CHECK_SBUS] {
            t.unexpected = vec!["0x1 to 0x2C".into(); 17];
        }
    });
    assert_eq!(r[CHECK_SBUS].check(), Err(TAGS_REFUSED));
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
        assert_eq!(
            check_run(&judged(r, &f)).unwrap_err(),
            SERIAL_REFUSED,
            "booted {booted}"
        );
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
            }) == Err(CHECK_REFUSED)
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
    assert_eq!(check_run(&r).unwrap_err(), CHECK_REFUSED);
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
    assert_eq!(
        check_run(&r).unwrap_err(),
        CHECK_REFUSED,
        "holds on another kernel clock"
    );
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
        assert_eq!(
            check_run(&r).unwrap_err(),
            CHECK_REFUSED,
            "{millibaud} {ppm}"
        );
    }
}

#[test]
fn replay_ties_each_echo_to_its_capture_and_rx_pin() {
    type Forge = dyn FnOnce(
        &mut Option<SerialCapture>,
        &mut String,
        &mut Option<(u8, u8)>,
        &mut Option<(u8, u8)>,
    );
    let forge = |f: Box<Forge>| {
        let r = edit(|r| set_witness(r, WITNESS_CONSOLE, |c, e, rx, tx| f(c, e, rx, tx)));
        check_run(&r).map(|_| ())
    };
    // Witnessed with the RX pin off its AF, or not read at the injection.
    assert_eq!(
        forge(Box::new(|_, _, rx, _| *rx = Some((0b10, 8)))),
        Err(ECHO_REFUSED)
    );
    assert_eq!(
        forge(Box::new(|_, _, rx, _| *rx = Some((0b11, 7)))),
        Err(ECHO_REFUSED)
    );
    assert_eq!(forge(Box::new(|_, _, rx, _| *rx = None)), Err(ECHO_REFUSED));
    // Witnessed with part of the frame, or with stray bytes.
    assert_eq!(
        forge(Box::new(|c, _, _, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 7,
                stray: false,
                len: 19,
            });
        })),
        Err(ECHO_REFUSED)
    );
    assert_eq!(
        forge(Box::new(|c, _, _, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 0,
                stray: true,
                len: 20,
            });
        })),
        Err(ECHO_REFUSED)
    );
    // Not counted with the RX pin on its AF; partial with the whole frame;
    // missing with the whole frame back on a routed pin.
    for status in ["not_counted", "partial", "missing"] {
        assert_eq!(
            forge(Box::new(move |_, e, _, _| *e = status.into())),
            Err(ECHO_REFUSED),
            "{status}"
        );
    }
}

/// `AET-WO-2302` D4: each port goes in after the port before it, never
/// without it, and inside the boot; replay refuses any other order.
#[test]
fn the_ports_go_in_one_after_another_inside_the_boot() {
    assert!(ok(&edit(|_| {})));
    let at = |r: &mut [Record], i: usize, t: Option<u64>| {
        if let Record::SerialWitness { injected_ns, .. } = &mut r[i] {
            *injected_ns = t;
        }
    };
    for (console, sbus) in [
        (Some(2_000_000), Some(2_000_000)),
        (Some(2_000_000), Some(130_750)),
        (Some(130_750), Some(100_000_001)),
    ] {
        let r = edit(|r| {
            at(r, WITNESS_CONSOLE, console);
            at(r, WITNESS_SBUS, sbus);
        });
        assert_eq!(
            check_run(&r).unwrap_err(),
            "a serial port's frame went in before the port before it, without it, or past the boot",
            "{console:?} {sbus:?}"
        );
    }
}

/// `AET-WO-2302` D5: a port the scenario holds open (`serial_rx_open`)
/// gets nothing, and the chain skips it: the port after it goes in alone,
/// and the open port's echo is missing by its own record. Replay refuses a
/// frame or a mux read at an injection into an open port, an open port no
/// solved port names, and a skip the scenario does not declare.
#[test]
fn an_open_port_gets_nothing_and_the_chain_skips_it() {
    use aethelgard_chaos::record::Override;
    fn open(port: &str) -> impl Fn(&mut Vec<Record>) + '_ {
        move |r: &mut Vec<Record>| {
            if let Record::Scenario { overrides, .. } = &mut r[0] {
                overrides.push(Override::SerialRxOpen { port: port.into() });
            }
        }
    }
    let console_open = |r: &mut Vec<Record>| {
        set_witness(r, WITNESS_CONSOLE, |c, e, rx, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 0,
                stray: false,
                len: 12,
            });
            *e = "missing".into();
            *rx = None;
        });
    };
    // The console held open: nothing went in, its echo is missing, and SBUS
    // went in after the boot word with the console skipped.
    let skipped = |r: &mut Vec<Record>| {
        console_open(r);
        if let Record::SerialWitness { injected_ns, .. } = &mut r[WITNESS_SBUS] {
            *injected_ns = Some(130_750);
        }
    };
    let r = judged(edit(skipped), &[(SERIAL_ECHO_FINDING, 1)]);
    // Without the override, a port goes in only after the one before it.
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a serial port's frame went in before the port before it, without it, or past the boot"
    );
    let mut r = r;
    open("CONSOLE")(&mut r);
    let s = check_run(&r).unwrap();
    assert!(
        s.matched && s.failing_kinds == [SERIAL_ECHO_FINDING],
        "{s:?}"
    );
    // A frame, or a mux read at an injection, into the open port.
    let mut forged = r.clone();
    if let Record::SerialWitness { injected_ns, .. } = &mut forged[WITNESS_CONSOLE] {
        *injected_ns = Some(130_750);
    }
    assert_eq!(
        check_run(&forged).unwrap_err(),
        "a frame went into a serial port whose RX line the scenario holds open"
    );
    let mut forged = r.clone();
    if let Record::SerialWitness {
        rx_at_injection, ..
    } = &mut forged[WITNESS_CONSOLE]
    {
        *rx_at_injection = Some((0b10, 7));
    }
    assert_eq!(
        check_run(&forged).unwrap_err(),
        "a frame went into a serial port whose RX line the scenario holds open"
    );
    // An open port no solved port names.
    let mut unknown = r.clone();
    open("GPS")(&mut unknown);
    assert_eq!(
        check_run(&unknown).unwrap_err(),
        "an open serial RX line names no solved serial port"
    );
    // The last port held open: the console goes in as ever, SBUS gets
    // nothing.
    let mut last = judged(
        edit(|r| {
            set_witness(r, WITNESS_SBUS, |c, e, rx, _| {
                *c = Some(SerialCapture {
                    patterns: 1,
                    cut: 0,
                    echoed: 0,
                    stray: false,
                    len: 9,
                });
                *e = "missing".into();
                *rx = None;
            });
        }),
        &[(SERIAL_ECHO_FINDING, 1)],
    );
    open("SBUS")(&mut last);
    assert!(ok(&last));
}

#[test]
fn a_serial_record_that_joins_no_solved_port_is_refused() {
    assert!(check_run(&run()).is_ok());
    let refused = |f: &dyn Fn(&mut Vec<Record>)| {
        let mut r = run();
        f(&mut r);
        check_run(&r).map(|_| ())
    };
    // A check or a witness of a port nothing solved.
    assert_eq!(
        refused(&|r| {
            if let Record::SerialFramingCheck { label, .. } = &mut r[CHECK_SBUS] {
                *label = "GPS".into();
            }
        }),
        Err(CHECKS_REFUSED)
    );
    assert_eq!(
        refused(&|r| {
            if let Record::SerialWitness { label, .. } = &mut r[WITNESS_SBUS] {
                *label = "GPS".into();
            }
        }),
        Err(WITNESSES_REFUSED)
    );
    // A port with no check; a check twice; the checks out of the ports' order.
    assert_eq!(
        refused(&|r| {
            r.remove(CHECK_SBUS);
        }),
        Err(CHECKS_REFUSED)
    );
    assert_eq!(
        refused(&|r| r[CHECK_SBUS] = check_console()),
        Err(CHECKS_REFUSED)
    );
    assert_eq!(
        refused(&|r| r.swap(CHECK_CONSOLE, CHECK_SBUS)),
        Err(CHECKS_REFUSED)
    );
    // A witness twice, one missing, or out of order.
    assert_eq!(
        refused(&|r| r[WITNESS_SBUS] = witness("CONSOLE", 7)),
        Err(WITNESSES_REFUSED)
    );
    assert_eq!(
        refused(&|r| {
            r.remove(WITNESS_SBUS);
        }),
        Err(WITNESSES_REFUSED)
    );
    assert_eq!(
        refused(&|r| r.swap(WITNESS_CONSOLE, WITNESS_SBUS)),
        Err(WITNESSES_REFUSED)
    );
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
    assert_eq!(sbus_as!(peripheral => "USART1".into()), Err(PORTS_REFUSED));
    assert_eq!(sbus_as!(tx => "PA10".into()), Err(PORTS_REFUSED));
    assert_eq!(sbus_as!(label => "CONSOLE".into()), Err(PORTS_REFUSED));
    assert_eq!(sbus_as!(index => 0), Err(PORTS_REFUSED));
    // A witness after the verdict, a check before the boot.
    let order = Err("the records are not in the campaign's order");
    assert_eq!(
        refused(&|r| {
            let w = r.remove(WITNESS_SBUS);
            r.push(w);
        }),
        order
    );
    assert_eq!(refused(&|r| r.swap(BOOT, CHECK_CONSOLE)), order);
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
    assert_eq!(check_run(&judged(forged, &[])).unwrap_err(), SERIAL_REFUSED);
    // A run with no port has no capture directory, so none.
    let no_ports: Vec<Record> = run()
        .into_iter()
        .filter(|r| !matches!(r.kind(), 27..=29))
        .collect();
    assert!(check_run(&no_ports).is_ok());
    assert_eq!(
        check_run(&judged(no_ports, &[(SERIAL_FORGED_FINDING, 1)])).unwrap_err(),
        SERIAL_REFUSED
    );
    // Any serial finding at `info` is the wrong severity.
    let mut r = run();
    let last = r.len() - 1;
    r[last] = Record::Verdict {
        pass: true,
        findings: vec![(SERIAL_FORGED_FINDING.into(), "info".into(), 1)],
    };
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a serial finding has the wrong severity"
    );
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
                    writes: 0,
                    missing: vec!["CR1.M1".into()],
                    unexpected: vec![],
                })
            }
        }),
        check(&|r| {
            if let Record::SerialFramingCheck { tags, .. } = r {
                *tags = Some(SerialTags {
                    writes: 0,
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
        // Trace 13: the injection's instant goes with the RX pin's read.
        w(&|r| {
            if let Record::SerialWitness { injected_ns, .. } = r {
                *injected_ns = None
            }
        }),
        w(&|r| {
            if let Record::SerialWitness {
                rx_at_injection, ..
            } = r
            {
                *rx_at_injection = None
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
    // Each refused by its own rule, in `bad`'s order.
    let port_names = "a solved serial port's label, peripheral or pins are malformed, or its TX and RX share a pin";
    let role = "a solved serial port's role or parity is not one a specification declares";
    let framing_bad = "a solved serial port's framing is not a baud above 0, a 7- to 9-bit word and 1 or 2 stop bits";
    let needed = "a serial framing check's needed tags are not tags, in order, once each";
    let mux = "a serial witness's pin mode is above 3 or its alternate function above 15";
    let instant = "a serial witness names its injection's instant without the RX pin's mux there, or the mux without the instant";
    let why = [
        port_names,
        port_names,
        "a solved serial port's alternate function is above 15",
        role,
        role,
        framing_bad,
        framing_bad,
        "a solved serial port's kernel clock is 0 or its BRR outside 16 to 0xFFFF",
        needed,
        needed,
        needed,
        TAGS_REFUSED,
        TAGS_REFUSED,
        "a serial framing check's tolerance row is malformed",
        "a serial witness's echo is not one the judge gives",
        mux,
        mux,
        instant,
        instant,
        COUNTS_REFUSED,
        COUNTS_REFUSED,
        COUNTS_REFUSED,
        COUNTS_REFUSED,
    ];
    assert_eq!(bad.len(), why.len());
    for (r, why) in bad.iter().zip(why) {
        assert_eq!(r.check(), Err(why), "{r:?}");
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
    assert_eq!(r.check(), Err(COUNTS_REFUSED));
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
    assert_eq!(
        check_serial_frames(&FW, &run()),
        Err("a serial witness's frame is not the one its firmware and label give")
    );
}

/// Adversarial pass 4, ADV4-4 (the PM's ruling), in the shapes of pass 3's
/// ADV3-4 and ADV3-5: once the interval rule names a pin unrouted after it
/// was solved (kind 28), the witness says so too. The RX pin made an input
/// after the injection: the echo came back whole on a pin routed at the
/// injection, `not_counted`, and the run needs the echo's finding beside
/// the tags'; the TX pin made an input between `TDR` writes: `tx_at_send`
/// carries that mux, and the run needs the pattern's finding. The note, at
/// its longest, fits kind 28's text rule. A `not_counted` on a routed pin
/// with tags that hold is a record the judge never writes.
#[test]
fn adversarial_4_a_pin_unrouted_once_solved_is_never_witnessed() {
    let note = |pin: &str| {
        format!("{pin} MODER/AFR not rebuilt, OTYPER/OSPEEDR/PUPDR not rebuilt after it was solved")
    };
    let with = |pin: &str, f: &dyn Fn(&mut Vec<Record>)| {
        edit(|r| {
            if let Record::SerialFramingCheck { tags: Some(t), .. } = &mut r[CHECK_CONSOLE] {
                t.unexpected = vec![note(pin)];
            }
            f(r);
        })
    };
    // ADV3-5's shape: PA10 unrouted after the injection.
    let rx = with("PK15", &|r| {
        set_witness(r, WITNESS_CONSOLE, |_, e, _, _| *e = "not_counted".into());
    });
    assert_eq!(rx[CHECK_CONSOLE].check(), Ok(()));
    assert!(ok(&judged(
        rx.clone(),
        &[(SERIAL_TAG_FINDING, 1), (SERIAL_ECHO_FINDING, 1)]
    )));
    assert!(!ok(&judged(rx.clone(), &[(SERIAL_TAG_FINDING, 1)])));
    // ADV3-4's shape: PA9 an input between TDR writes, so `tx_at_send` is
    // its mux then.
    let tx = with("PK15", &|r| {
        if let Record::SerialWitness { tx_at_send, .. } = &mut r[WITNESS_CONSOLE] {
            *tx_at_send = Some((0b00, 7));
        }
    });
    assert!(ok(&judged(
        tx.clone(),
        &[(SERIAL_TAG_FINDING, 1), (SERIAL_PATTERN_FINDING, 1)]
    )));
    assert!(!ok(&judged(tx, &[(SERIAL_TAG_FINDING, 1)])));
    // Kind 28 at its most, 16 texts of 128 bytes, fits one archived event
    // and replays whole.
    let most = edit(|r| {
        if let Record::SerialFramingCheck { tags: Some(t), .. } = &mut r[CHECK_CONSOLE] {
            t.unexpected = vec!["x".repeat(MAX_SERIAL_UNEXPECTED_BYTES); MAX_SERIAL_UNEXPECTED];
        }
    });
    assert_eq!(most[CHECK_CONSOLE].check(), Ok(()));
    let events: Vec<Event> = most
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("serial-kind-28-at-its-most", 9, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, most);
    let over = edit(|r| {
        if let Record::SerialFramingCheck { tags: Some(t), .. } = &mut r[CHECK_CONSOLE] {
            t.unexpected = vec!["x".repeat(MAX_SERIAL_UNEXPECTED_BYTES + 1)];
        }
    });
    assert_eq!(over[CHECK_CONSOLE].check(), Err(TAGS_REFUSED));
    // `not_counted` on a pin routed at the injection, with tags that hold.
    let forged = edit(|r| {
        set_witness(r, WITNESS_CONSOLE, |_, e, _, _| *e = "not_counted".into());
    });
    assert_eq!(
        check_run(&judged(forged, &[(SERIAL_ECHO_FINDING, 1)])).map(|s| s.matched),
        Err("a serial witness's echo is not what its capture and its RX pin give")
    );
}

/// Sets both ports' injection instants in a [`run`].
fn injected(r: &mut [Record], console: Option<u64>, sbus: Option<u64>) {
    for (at, t) in [(WITNESS_CONSOLE, console), (WITNESS_SBUS, sbus)] {
        if let Record::SerialWitness { injected_ns, .. } = &mut r[at] {
            *injected_ns = t;
        }
    }
}

/// The replay error a verdict gets for either finding the boot word's
/// instant decides (A2-5).
const INSTANT_REFUSED: &str = "the verdict's serial_wire_past_run or motor_anchor_out_of_order does not match the boot word's instant";

/// Adversarial pass 2, A2-5 (`AET-WO-2302`): `serial_wire_past_run` is
/// bound by replay where the records decide it. The first port the chain
/// injects goes in one quantum after the boot word, so its kind 29
/// `injected_ns` places the boot word; from there the ports' frames and
/// echoes (kind 27's framings, each its 8 paced bytes and one echoed
/// character: the console's 8 x 100 000 + 86 876 = 886 876 ns at 115 200
/// 8N1, SBUS's 8 x 200 000 + 120 000 = 1 720 000 ns at 100 000 8E2,
/// 2 606 876 ns in all) must end inside the boot or the verdict names it
/// once. Without the first port's injection the instant is not in the
/// records, and the verdict names it at most once; before the boot word,
/// never.
#[test]
fn a2_5_replay_binds_serial_wire_past_run_from_the_records() {
    // The honest run: the boot word at 30 750 ns, the frames well inside.
    assert!(ok(&run()));
    let named = |r: Vec<Record>| judged(r, &[(SERIAL_WIRE_PAST_RUN_FINDING, 1)]);
    assert_eq!(check_run(&named(run())).unwrap_err(), INSTANT_REFUSED);
    // The console injected at 97 393 124 ns: the frames end at exactly
    // 100 000 000 ns, inside; one ns later they pass it, and the verdict
    // must name it, once.
    let fits = edit(|r| injected(r, Some(97_393_124), Some(99_000_000)));
    assert!(ok(&fits));
    assert_eq!(check_run(&named(fits)).unwrap_err(), INSTANT_REFUSED);
    let past = edit(|r| injected(r, Some(97_393_125), Some(99_000_000)));
    assert_eq!(check_run(&past).unwrap_err(), INSTANT_REFUSED);
    assert!(ok(&named(past.clone())));
    assert_eq!(
        check_run(&judged(past, &[(SERIAL_WIRE_PAST_RUN_FINDING, 2)])).unwrap_err(),
        INSTANT_REFUSED
    );
    // SBUS held open: its frame never goes in, and adds nothing; the console
    // alone ends at 98 280 001 ns.
    let mut open = edit(|r| {
        injected(r, Some(97_393_125), None);
        set_witness(r, WITNESS_SBUS, |c, e, rx, _| {
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 0,
                stray: false,
                len: 9,
            });
            *e = "missing".into();
            *rx = None;
        });
    });
    let mut open_named = judged(
        open.clone(),
        &[(SERIAL_ECHO_FINDING, 1), (SERIAL_WIRE_PAST_RUN_FINDING, 1)],
    );
    open = judged(open, &[(SERIAL_ECHO_FINDING, 1)]);
    for r in [&mut open, &mut open_named] {
        if let Record::Scenario { overrides, .. } = &mut r[0] {
            overrides.push(aethelgard_chaos::record::Override::SerialRxOpen {
                port: "SBUS".into(),
            });
        }
    }
    assert!(ok(&open));
    assert_eq!(check_run(&open_named).unwrap_err(), INSTANT_REFUSED);
    // No frame went in (the boot ended first): the instant is not in the
    // records, so the verdict may name it once, never twice.
    let none = edit(|r| {
        for (at, len) in [(WITNESS_CONSOLE, 12), (WITNESS_SBUS, 9)] {
            set_witness(r, at, |c, e, rx, _| {
                *c = Some(SerialCapture {
                    patterns: 1,
                    cut: 0,
                    echoed: 0,
                    stray: false,
                    len,
                });
                *e = "missing".into();
                *rx = None;
            });
        }
    });
    for n in [0, 1] {
        let mut f = vec![(SERIAL_ECHO_FINDING, 2)];
        if n > 0 {
            f.push((SERIAL_WIRE_PAST_RUN_FINDING, n));
        }
        assert!(ok(&judged(none.clone(), &f)), "{n}");
    }
    assert_eq!(
        check_run(&judged(
            none,
            &[(SERIAL_ECHO_FINDING, 2), (SERIAL_WIRE_PAST_RUN_FINDING, 2)]
        ))
        .unwrap_err(),
        INSTANT_REFUSED
    );
}

/// A PWM output on TIM1 (PA8, AF1) beside [`run`]'s ports: its solved
/// output and static check (kinds 24 and 25, which hold) and its witness
/// (kind 26, `witnessed` on the model's sequence) anchored at `anchor`.
fn with_motor(anchor: u64) -> Vec<Record> {
    let solved = Record::MotorOutputSolved {
        label: "M1".into(),
        timer: "TIM1".into(),
        channel: 1,
        pin: "PA8".into(),
        alternate_function: 1,
        protocol: "pwm".into(),
        psc: 63,
        arr: 2499,
        ccr: 1000,
        pwm: Some((400, 1000, 2000)),
        idle: Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
        resolution_ps: Some(1_000_000),
    };
    let check = Record::MotorTimingCheck {
        label: "M1".into(),
        kernel_hz: 64_000_000,
        period_ps: 2_500_000_000,
        high_ps: 1_000_000_000,
        nominal_period_ps: 2_500_000_000,
        nominal_high_ps: 1_000_000_000,
        holds: true,
    };
    let witness = Record::MotorWitness {
        label: "M1".into(),
        observer: "gpioPortA.pa8obs".into(),
        // At the 64 MHz kernel clock (`AET-WO-4400g`): a 1 µs tick, 2,499
        // ticks a period.
        period_ns: 2_499_000,
        high_ns: 1_000_000,
        grain_ns: 10,
        quantum_ns: 100_000,
        status: "witnessed".into(),
        rises: 40,
        falls: 39,
        first_ns: Some(2_499_006),
        last_ns: Some(2_499_006 + 39 * 2_499_000),
        end_ns: 100_000_000,
        span_ns: Some(8),
        off_grid: 79,
        batched: 0,
        anchor_ns: Some(anchor),
        edges_before: 0,
        edges_after: 0,
        cen_rises: 1,
    };
    let mut r = run();
    r.insert(CHECK_CONSOLE, witness);
    r.insert(BOOT, check);
    r.insert(BOOT, solved);
    r
}

/// Adversarial pass 2, A2-5: `motor_anchor_out_of_order`'s case of an
/// anchor after the boot word is bound by replay where the records place
/// the boot word (the first injected port's `injected_ns` less a quantum,
/// 30 750 ns here): a timer anchored after it must be named, at least once;
/// and no verdict names it more than once per anchored timer, nor before
/// the boot word. Its other cases (before the last device step's witness,
/// no boot word seen by its hook) are the boot's line order and stay
/// unbound: an anchor before the boot word may be named once.
#[test]
fn a2_5_replay_binds_an_anchor_after_the_boot_word() {
    let order = |n| vec![(MOTOR_ANCHOR_ORDER_FINDING, n)];
    let before = with_motor(6);
    assert!(ok(&before));
    assert!(ok(&judged(before.clone(), &order(1))));
    assert_eq!(
        check_run(&judged(before, &order(2))).unwrap_err(),
        INSTANT_REFUSED
    );
    let after = with_motor(40_000);
    assert_eq!(check_run(&after).unwrap_err(), INSTANT_REFUSED);
    assert!(ok(&judged(after.clone(), &order(1))));
    // One ns after the boot word is after it; at it, not.
    assert_eq!(check_run(&with_motor(30_751)).unwrap_err(), INSTANT_REFUSED);
    assert!(ok(&with_motor(30_750)));
    // No frame went in: the instant is unknown, and the anchor unbound
    // from below.
    let mut unknown = after;
    for at in [WITNESS_CONSOLE + 3, WITNESS_SBUS + 3] {
        set_witness(&mut unknown, at, |c, e, rx, _| {
            let len = c.map_or(0, |c| c.len - 8);
            *c = Some(SerialCapture {
                patterns: 1,
                cut: 0,
                echoed: 0,
                stray: false,
                len,
            });
            *e = "missing".into();
            *rx = None;
        });
    }
    assert!(ok(&judged(unknown, &[(SERIAL_ECHO_FINDING, 2)])));
    // Before the boot word, never.
    let mut unbooted = with_motor(40_000);
    unbooted[BOOT + 2] = boot(false);
    let n = unbooted.len() - 1;
    unbooted.truncate(n);
    unbooted.retain(|x| {
        !matches!(
            x,
            Record::SerialWitness { .. } | Record::MotorWitness { .. }
        )
    });
    unbooted.push(verdict(&[]));
    let unbooted = judged(unbooted, &[("boot_word_missing", 1)]);
    assert!(ok(&unbooted), "{:?}", check_run(&unbooted));
    let mut named = unbooted;
    let n = named.len() - 1;
    named[n] = verdict(&[("boot_word_missing", 1), (MOTOR_ANCHOR_ORDER_FINDING, 1)]);
    assert_eq!(check_run(&named).unwrap_err(), INSTANT_REFUSED);
    let mut wire = named;
    wire[n] = verdict(&[("boot_word_missing", 1), (SERIAL_WIRE_PAST_RUN_FINDING, 1)]);
    assert_eq!(check_run(&wire).unwrap_err(), INSTANT_REFUSED);
}
