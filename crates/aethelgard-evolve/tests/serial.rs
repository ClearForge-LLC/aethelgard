//! `AET-WO-4300b`, tier 1: attribution of a serial port's symptoms
//! (`Rule::S5`, `Rule::I6`) on campaign runs written through
//! `aethelgard-chaos`'s own writer, with no Renode. The run is the reference
//! board's I2C run (as `attribute.rs` builds it) with two serial ports,
//! `CONSOLE` (USART1, PA9/PA10 at AF7) and `GPS` (USART2, PA2/PA3 at AF7),
//! added where `check_run` ranks their records.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::campaign::{
    self, SERIAL_ECHO_FINDING, SERIAL_FORGED_FINDING, SERIAL_PATTERN_FINDING,
    SERIAL_STATIC_FINDING, SERIAL_TAG_FINDING, run_id,
};
use aethelgard_chaos::record::{
    BusLine, Override, Reading, Record, SerialCapture, SerialFraming, SerialTags, serial_frame,
};
use aethelgard_chaos::{Event, Header, Replayed, TraceWriter, replay_dir};
use aethelgard_evolve::propose::{self, BusFacts, Input, Outcome, Rules};
use aethelgard_evolve::{Attribution, Domain, Evidence, Rule, attribute, view};

/// The firmware the archive's header names: each frame is derived from it.
const FIRMWARE: [u8; 32] = [9; 32];

fn register(name: &str, address: u32, mask: u32, expected: u32, observed: u32) -> Record {
    Record::RegisterObserved {
        name: name.into(),
        reading: Reading {
            address,
            mask,
            expected,
            observed: Some(observed),
        },
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
        step: "identity".into(),
        reference: "U2".into(),
        expected: lines.clone(),
        observed: Some(lines),
        device,
        absent: Vec::new(),
    }
}

/// A solved port at 115200 8N1: `BRR` 556, the nearest; or (`holds` false)
/// 557, one off, which silicon's check fails.
fn port(label: &str, index: u32, usart: &str, tx: &str, rx: &str, holds: bool) -> Record {
    let (brr, actual_millibaud, error_ppm) = figures(holds);
    Record::SerialPortSolved {
        label: label.into(),
        index,
        peripheral: usart.into(),
        tx: tx.into(),
        tx_af: 7,
        rx: rx.into(),
        rx_af: 7,
        role: if index == 0 { "console" } else { "gps" }.into(),
        framing: SerialFraming {
            baud: 115_200,
            data_bits: 8,
            parity: "none".into(),
            stop_bits: 1,
            inverted: false,
        },
        kernel_hz: 64_000_000,
        brr,
        actual_millibaud,
        error_ppm,
    }
}

fn figures(holds: bool) -> (u32, u64, i64) {
    if holds {
        (556, 115_107_913, -799)
    } else {
        (557, 114_901_256, -2593)
    }
}

/// The port's framing check: as its solved port gives it, or (`holds`
/// false) as a `BRR` of 557 gives it, with the bring-up's writes alone or
/// one to `PRESC` beside them.
fn check(label: &str, holds: bool, tags_hold: bool) -> Record {
    let (brr, actual_millibaud, error_ppm) = figures(holds);
    Record::SerialFramingCheck {
        label: label.into(),
        kernel_hz: 64_000_000,
        brr,
        actual_millibaud,
        error_ppm,
        tolerance: Some(("Table 407".into(), 0, 33_300)),
        holds,
        tags_needed: vec![],
        tags: Some(SerialTags {
            writes: 5,
            missing: vec![],
            unexpected: if tags_hold {
                vec![]
            } else {
                vec!["0x1 to 0x2C".into()]
            },
        }),
    }
}

/// How a port's witness went.
#[derive(Clone, Copy, PartialEq)]
enum Echo {
    Witnessed,
    /// The frame came back with the RX pin on AF8 at the injection.
    NotCounted,
    /// No frame came back.
    Missing,
    /// Nothing at all: the USART never sent (`UE` clear).
    Silent,
}

/// `tx`: the TX pin's `MODER` and `AFSEL` fields at the end, as kind 11 read
/// them (adversarial F11), and at its first `TDR` write.
fn witness(label: &str, echo: Echo, tx: (u8, u8)) -> Record {
    let p = (label.len() + 5) as u64;
    let (patterns, echoed) = match echo {
        Echo::Witnessed | Echo::NotCounted => (1, 8),
        Echo::Missing => (1, 0),
        Echo::Silent => (0, 0),
    };
    Record::SerialWitness {
        label: label.into(),
        capture: Some(SerialCapture {
            patterns,
            cut: 0,
            echoed,
            stray: false,
            len: patterns * p + echoed,
        }),
        frame: serial_frame(&FIRMWARE, label),
        echo: match echo {
            Echo::Witnessed => "witnessed",
            Echo::NotCounted => "not_counted",
            _ => "missing",
        }
        .into(),
        rx_at_injection: Some((0b10, if echo == Echo::NotCounted { 8 } else { 7 })),
        tx_at_end: Some(tx),
        tx_at_send: Some(tx),
        // Trace 13 (`AET-WO-2302` D4): the console's frame a quantum after the
        // boot word, the GPS port's once the console has echoed.
        injected_ns: Some(if label == "CONSOLE" {
            130_750
        } else {
            2_000_000
        }),
    }
}

/// What a firmware may have got wrong, and how each port's witness went.
struct Case {
    /// `GPIOA.MODER` as read: PA2, PA3, PA9 and PA10 in AF mode when right.
    moder: u32,
    /// `GPIOA.AFRH` as read: PA9 and PA10's fields, `0x770` when right.
    afrh: u32,
    /// `GPIOA.AFRL` as read: PA2 and PA3's fields, `0x7700` when right.
    afrl: u32,
    /// `USART1.CR1` as read: `UE | RE | TE` when right.
    usart1_cr1: u32,
    /// `RCC.APB2ENR` as read: `USART1EN` when right.
    apb2enr: u32,
    console: Echo,
    gps: Echo,
    console_holds: bool,
    console_tags_hold: bool,
}

impl Default for Case {
    fn default() -> Self {
        Self {
            moder: 0x0028_00A0,
            afrh: 0x770,
            afrl: 0x7700,
            usart1_cr1: 0xD,
            apb2enr: 0x10,
            console: Echo::Witnessed,
            gps: Echo::Witnessed,
            console_holds: true,
            console_tags_hold: true,
        }
    }
}

/// The `CR1` bits Renode's USART stores (`aethelgard_twin::boot`).
const CR1_STORED: u32 = 0x0400_87ED;

fn run(c: &Case, findings: &[(&str, u64)]) -> Vec<Record> {
    let mut findings: Vec<(String, String, u64)> = findings
        .iter()
        .map(|(k, n)| ((*k).into(), "error".into(), *n))
        .collect();
    findings.push(("device_unmodelled".into(), "info".into(), 1));
    findings.sort();
    vec![
        Record::Scenario {
            name: "serial".into(),
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
        port("CONSOLE", 0, "USART1", "PA9", "PA10", c.console_holds),
        port("GPS", 1, "USART2", "PA2", "PA3", true),
        rise("I2C_SCL"),
        rise("I2C_SDA"),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: Reading {
                address: 0x2000_0000,
                mask: u32::MAX,
                expected: 0x2100_B007,
                observed: Some(0x2100_B007),
            },
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
            0x003C_00F0,
            0x0028_00A0,
            c.moder,
        ),
        register("GPIOA.AFRL", 0x5802_0020, 0x0000_FF00, 0x7700, c.afrl),
        register("GPIOA.AFRH", 0x5802_0024, 0x0000_0FF0, 0x770, c.afrh),
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
            0x0022_0000,
            0x0022_0000,
        ),
        register("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0x10, c.apb2enr),
        register("USART1.CR1", 0x4001_1000, CR1_STORED, 0xD, c.usart1_cr1),
        register("USART2.CR1", 0x4000_4400, CR1_STORED, 0xD, 0xD),
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
        check("CONSOLE", c.console_holds, c.console_tags_hold),
        check("GPS", true, true),
        witness(
            "CONSOLE",
            c.console,
            (((c.moder >> 18) & 3) as u8, ((c.afrh >> 4) & 0xF) as u8),
        ),
        witness(
            "GPS",
            c.gps,
            (((c.moder >> 4) & 3) as u8, ((c.afrl >> 8) & 0xF) as u8),
        ),
        Record::Verdict {
            pass: findings.iter().all(|f| f.1 == "info"),
            findings,
        },
    ]
}

fn archived(records: &[Record]) -> Replayed {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "aet-4300b-evolve-serial-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let header = Header {
        run_id: run_id(&[1; 32], &[2; 32], &FIRMWARE),
        artifacts: vec![
            ("base".into(), [8; 32]),
            ("firmware".into(), FIRMWARE),
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
    let a = attributed_any(records);
    assert!(a.unexplained.is_empty(), "{a:?}");
    a
}

/// The evidence, a register by its name and a serial record by its kind
/// and label.
fn named(records: &[Record], a: &Attribution) -> Vec<String> {
    a.evidence
        .iter()
        .map(|e| match &records[e.index as usize] {
            Record::RegisterObserved { name, .. } => name.clone(),
            Record::SerialFramingCheck { label, .. } | Record::SerialWitness { label, .. } => {
                format!("kind {} {label}", e.kind)
            }
            r => format!("kind {}", r.kind()),
        })
        .collect()
}

#[test]
fn a_healthy_serial_run_passes_none() {
    let a = attributed(&run(&Case::default(), &[]));
    assert_eq!((a.domain, a.rule), (Domain::None, Rule::N1));
}

/// The seeded `serial_rx_dead`: PA10 (the console's RX) on AF8, not the
/// solved AF7, so no byte reached the receiver. The alternate-function
/// register is the firmware's own write, and nothing electrical is modelled
/// on the pin: software, S5, by the register and the failed echo.
#[test]
fn the_console_s_rx_on_the_wrong_alternate_function_is_software_s5() {
    let r = run(
        &Case {
            afrh: 0x870,
            console: Echo::NotCounted,
            ..Case::default()
        },
        &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5));
    assert_eq!(named(&r, &a), ["GPIOA.AFRH", "kind 29 CONSOLE"]);
}

#[test]
fn a_usart_register_or_its_clock_gate_is_the_port_s_own_evidence() {
    // `UE` never set: nothing sent, nothing echoed, `USART1.CR1` wrong.
    let r = run(
        &Case {
            usart1_cr1: 0xC,
            console: Echo::Silent,
            ..Case::default()
        },
        &[
            ("register_mismatch", 1),
            (SERIAL_ECHO_FINDING, 1),
            (SERIAL_PATTERN_FINDING, 1),
        ],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5));
    assert_eq!(named(&r, &a), ["USART1.CR1", "kind 29 CONSOLE"]);
    // The USART's clock gate left off.
    let r = run(
        &Case {
            apb2enr: 0,
            console: Echo::Silent,
            ..Case::default()
        },
        &[
            ("register_mismatch", 1),
            (SERIAL_ECHO_FINDING, 1),
            (SERIAL_PATTERN_FINDING, 1),
        ],
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S5);
    assert_eq!(named(&r, &a), ["RCC.APB2ENR", "kind 29 CONSOLE"]);
    // A wrong USART register beside a witness that passed: the register
    // alone; the passing witness is never evidence (`4300a` M).
    let r = run(
        &Case {
            usart1_cr1: 0x40D,
            ..Case::default()
        },
        &[("register_mismatch", 1)],
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S5);
    assert_eq!(named(&r, &a), ["USART1.CR1"]);
}

/// §5.4, a wrong attribution attempt: the GPS port's RX on the wrong AF
/// (its own register, its own S5) beside the console's dead echo with every
/// register of the console's right. The GPS field never explains the
/// console: its symptom stays indeterminate, so the run is.
#[test]
fn another_port_s_wrong_field_never_explains_this_port_s_dead_echo() {
    let r = run(
        &Case {
            afrl: 0x8700,
            console: Echo::Missing,
            ..Case::default()
        },
        &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
    assert_eq!(named(&r, &a), ["kind 29 CONSOLE"]);
    // The GPS field and the GPS echo, without the console's: S5 by GPS's
    // own evidence.
    let r = run(
        &Case {
            afrl: 0x8700,
            gps: Echo::NotCounted,
            ..Case::default()
        },
        &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S5);
    assert_eq!(named(&r, &a), ["GPIOA.AFRL", "kind 29 GPS"]);
}

#[test]
fn no_register_evidence_is_never_a_guess() {
    // Every owned register right and still no echo: nothing shows which half
    // failed, so the run is indeterminate, never software by elimination.
    let r = run(
        &Case {
            console: Echo::Missing,
            ..Case::default()
        },
        &[(SERIAL_ECHO_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I6));
    assert_eq!(named(&r, &a), ["kind 29 CONSOLE"]);
    // Renode's access log not read whole: neither the writes nor the TX pin
    // at its sends were rebuilt, so no write shows: I6, by both records.
    let mut r = run(
        &Case::default(),
        &[(SERIAL_TAG_FINDING, 1), (SERIAL_PATTERN_FINDING, 1)],
    );
    for x in r.iter_mut() {
        match x {
            Record::SerialFramingCheck { label, tags, .. } if label == "CONSOLE" => *tags = None,
            Record::SerialWitness {
                label, tx_at_send, ..
            } if label == "CONSOLE" => *tx_at_send = None,
            _ => {}
        }
    }
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I6));
    assert_eq!(named(&r, &a), ["kind 28 CONSOLE", "kind 29 CONSOLE"]);
}

/// Adversarial pass 2, M1 (the PM's ruling): a failing write judgement
/// (kind 28's tags, from Renode's access log) and a TX pin rebuilt unrouted
/// at a `TDR` write (kind 29's `tx_at_send`) are each direct evidence of a
/// firmware write: software, S5, citing them, every register right.
#[test]
fn a_failing_write_judgement_or_an_unrouted_send_is_software_s5() {
    // A write the bring-up never makes (`0x1 to 0x2C`): S5 by kind 28.
    let r = run(
        &Case {
            console_tags_hold: false,
            ..Case::default()
        },
        &[(SERIAL_TAG_FINDING, 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 CONSOLE"]);
    // PA9 an input at the pattern's sends, routed by the end (ADV-1): S5 by
    // kind 29, which the pattern's finding selects.
    let mut r = run(&Case::default(), &[(SERIAL_PATTERN_FINDING, 1)]);
    for x in r.iter_mut() {
        if let Record::SerialWitness {
            label, tx_at_send, ..
        } = x
            && label == "CONSOLE"
        {
            *tx_at_send = Some((0b00, 7));
        }
    }
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 29 CONSOLE"]);
    // `propose` says so, and reports nothing: the witness names no field.
    let archived = archived(&r);
    let v = view(&archived).unwrap();
    let evidence: Vec<Evidence> = a.evidence.iter().map(|e| v.event(*e).unwrap()).collect();
    let input = Input {
        attribution: &a,
        evidence: &evidence,
        buses: &v.buses(),
        facts: &BTreeMap::new(),
    };
    let ans = propose::propose(&[&Rules], &input);
    assert!(
        matches!(&ans.outcome, Outcome::NoProposal { reason, .. } if reason.contains("no generator report")),
        "{:?}",
        ans.outcome
    );
    // Both, beside a wrong register of the port's: all three cited.
    let mut r = run(
        &Case {
            console_tags_hold: false,
            usart1_cr1: 0x40D,
            ..Case::default()
        },
        &[
            ("register_mismatch", 1),
            (SERIAL_TAG_FINDING, 1),
            (SERIAL_PATTERN_FINDING, 1),
        ],
    );
    for x in r.iter_mut() {
        if let Record::SerialWitness {
            label, tx_at_send, ..
        } = x
            && label == "CONSOLE"
        {
            *tx_at_send = Some((0b10, 0));
        }
    }
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S5, "{a:?}");
    assert_eq!(
        named(&r, &a),
        ["USART1.CR1", "kind 28 CONSOLE", "kind 29 CONSOLE"]
    );
    // Another port's failing judgement never explains this port's echo.
    let mut r = run(
        &Case {
            console: Echo::Missing,
            ..Case::default()
        },
        &[(SERIAL_ECHO_FINDING, 1), (SERIAL_TAG_FINDING, 1)],
    );
    for x in r.iter_mut() {
        if let Record::SerialFramingCheck {
            label,
            tags: Some(t),
            ..
        } = x
            && label == "GPS"
        {
            t.unexpected = vec!["RCC.APB1LRSTR 0x20000 written, not an enable register".into()];
        }
    }
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
}

/// Adversarial pass 4, ADV4-4, in ADV3-5's shape: PA10 taken off AF7 after
/// the injection. The frame came back whole on a pin routed at the
/// injection, so the echo is `not_counted` and the interval rule's note
/// fails kind 28: the echo's finding and the tags' are the firmware's, S5 by
/// kind 28, with the witness the echo's finding selects.
#[test]
fn adversarial_4_an_rx_pin_unrouted_after_the_injection_is_software_s5() {
    let mut r = run(
        &Case {
            console_tags_hold: false,
            ..Case::default()
        },
        &[(SERIAL_TAG_FINDING, 1), (SERIAL_ECHO_FINDING, 1)],
    );
    for x in r.iter_mut() {
        match x {
            Record::SerialFramingCheck {
                label,
                tags: Some(t),
                ..
            } if label == "CONSOLE" => {
                t.unexpected = vec![
                    "PA10 MODER 0b00, AF7, OTYPER 0, OSPEEDR 0b00, PUPDR 0b00 after it was solved"
                        .into(),
                ];
            }
            Record::SerialWitness { label, echo, .. } if label == "CONSOLE" => {
                *echo = "not_counted".into();
            }
            _ => {}
        }
    }
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 CONSOLE", "kind 29 CONSOLE"]);
}

/// `4300a` F1: a failing static check is the topology's, not the
/// firmware's, whatever else of the port's is wrong.
#[test]
fn a_failing_static_framing_check_is_never_software() {
    let r = run(
        &Case {
            console_holds: false,
            afrh: 0x870,
            console: Echo::NotCounted,
            ..Case::default()
        },
        &[
            ("register_mismatch", 1),
            (SERIAL_ECHO_FINDING, 1),
            (SERIAL_STATIC_FINDING, 1),
        ],
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
    assert!(
        named(&r, &a).contains(&"kind 28 CONSOLE".to_owned()),
        "{a:?}"
    );
}

#[test]
fn a_rewritten_verdict_cannot_move_the_domain() {
    // The register is wrong but the verdict names only the dead echo: the
    // register's event shows a failure the verdict does not name.
    let r = run(
        &Case {
            afrh: 0x870,
            console: Echo::NotCounted,
            ..Case::default()
        },
        &[(SERIAL_ECHO_FINDING, 1)],
    );
    let a = attribute(&archived(&r)).unwrap();
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    // And the echo, dropped from the verdict, is replay's to refuse.
    let r = run(
        &Case {
            afrh: 0x870,
            console: Echo::NotCounted,
            ..Case::default()
        },
        &[("register_mismatch", 1)],
    );
    assert!(campaign::check_run(&r).is_err());
}

/// `propose` answers S5 with a generator report naming the board crate's
/// wrong alternate-function field, and nothing else (`INV-05`).
#[test]
fn propose_answers_the_dead_rx_with_a_generator_report_of_its_af_field() {
    let r = run(
        &Case {
            afrh: 0x870,
            console: Echo::NotCounted,
            ..Case::default()
        },
        &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 1)],
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
    assert_eq!(what, [("GPIOA.AFRH", 0xF00, 0x700, Some(0x800))]);
}

/// One register wrong in two ports' fields (`GPIOA.MODER`: PA10, the
/// console's RX, and PA3, GPS's RX, both analog): the first port stands for
/// the register, as a timer's first output does, and each port's dead echo
/// cites it as its own evidence. Never the board's, so never I4.
#[test]
fn a_register_wrong_in_two_ports_fields_is_both_ports_evidence() {
    let r = run(
        &Case {
            moder: 0x0038_00E0,
            console: Echo::NotCounted,
            gps: Echo::NotCounted,
            ..Case::default()
        },
        &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 2)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(
        named(&r, &a),
        ["GPIOA.MODER", "kind 29 CONSOLE", "kind 29 GPS"]
    );
}

/// A port whose USART left no capture at all is forged, a failure no
/// register explains: I6, by its witness.
#[test]
fn a_port_with_no_capture_is_indeterminate_by_its_witness() {
    let mut r = run(&Case::default(), &[(SERIAL_FORGED_FINDING, 1)]);
    for x in r.iter_mut() {
        if let Record::SerialWitness {
            label,
            capture,
            echo,
            ..
        } = x
            && label == "GPS"
        {
            *capture = None;
            *echo = "missing".into();
        }
    }
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
    assert_eq!(named(&r, &a), ["kind 29 GPS"]);
}

/// `r` with `port`'s RX line held open by its scenario (`AET-WO-2302` D5):
/// nothing went into it (no instant, no mux read at an injection), and the
/// port after it in the chain went in a quantum after the boot word. With
/// `declared` false the scenario does not say so.
fn open(mut r: Vec<Record>, port: &str, declared: bool) -> Vec<Record> {
    for x in r.iter_mut() {
        match x {
            Record::Scenario { overrides, .. } if declared => {
                overrides.push(Override::SerialRxOpen { port: port.into() });
            }
            Record::SerialWitness {
                label,
                rx_at_injection,
                injected_ns,
                ..
            } if label == port => {
                *rx_at_injection = None;
                *injected_ns = None;
            }
            Record::SerialWitness {
                label, injected_ns, ..
            } if port == "CONSOLE" && label == "GPS" => *injected_ns = Some(130_750),
            _ => {}
        }
    }
    r
}

/// `AET-WO-2302` D5, E3: the console's RX line open on the board, its echo
/// missing, and nothing else of the port's wrong: electrical, E3, by the
/// port's framing check and its witness alone (only that port as subject);
/// the GPS port after it, skipped to, is witnessed.
#[test]
fn an_open_rx_line_the_scenario_declares_is_electrical_e3_by_that_port_alone() {
    let missing = Case {
        console: Echo::Missing,
        ..Case::default()
    };
    let r = open(run(&missing, &[(SERIAL_ECHO_FINDING, 1)]), "CONSOLE", true);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E3), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 CONSOLE", "kind 29 CONSOLE"]);
    assert!(a.masked.is_empty(), "{a:?}");
    // The last port open: the console goes in as ever.
    let gps = Case {
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = open(run(&gps, &[(SERIAL_ECHO_FINDING, 1)]), "GPS", true);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E3), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 GPS", "kind 29 GPS"]);
    // The same records without the scenario's override: no positive
    // evidence of an open line, so I6, never electrical by elimination.
    let r = open(run(&gps, &[(SERIAL_ECHO_FINDING, 1)]), "GPS", false);
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
}

/// E3 holds only on a port whose framing, writes and pattern hold, with no
/// software cause: a firmware cause on the open port is S5 (the seeded
/// `serial_rx_dead` stays S5); the pattern missing too, or the writes not
/// read whole, leave the open line unshown (I6).
#[test]
fn an_open_rx_line_with_anything_else_of_the_port_s_wrong_is_not_e3() {
    let r = open(
        run(
            &Case {
                afrh: 0x870,
                console: Echo::Missing,
                ..Case::default()
            },
            &[("register_mismatch", 1), (SERIAL_ECHO_FINDING, 1)],
        ),
        "CONSOLE",
        true,
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(named(&r, &a), ["GPIOA.AFRH", "kind 29 CONSOLE"]);
    let silent = Case {
        console: Echo::Silent,
        ..Case::default()
    };
    let r = open(
        run(
            &silent,
            &[(SERIAL_ECHO_FINDING, 1), (SERIAL_PATTERN_FINDING, 1)],
        ),
        "CONSOLE",
        true,
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
    let missing = Case {
        console: Echo::Missing,
        ..Case::default()
    };
    // Renode's access log not read whole: neither the writes nor the TX pin
    // at its sends rebuilt.
    let mut r = open(
        run(
            &missing,
            &[
                (SERIAL_ECHO_FINDING, 1),
                (SERIAL_TAG_FINDING, 1),
                (SERIAL_PATTERN_FINDING, 1),
            ],
        ),
        "CONSOLE",
        true,
    );
    for x in r.iter_mut() {
        match x {
            Record::SerialFramingCheck { label, tags, .. } if label == "CONSOLE" => *tags = None,
            Record::SerialWitness {
                label, tx_at_send, ..
            } if label == "CONSOLE" => *tx_at_send = None,
            _ => {}
        }
    }
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
}

/// `propose` answers E3 with no rule: an open RX line is a board fault no
/// value change fixes.
#[test]
fn propose_leaves_an_open_rx_line_to_the_model_slot() {
    let missing = Case {
        console: Echo::Missing,
        ..Case::default()
    };
    let r = open(run(&missing, &[(SERIAL_ECHO_FINDING, 1)]), "CONSOLE", true);
    let replayed = archived(&r);
    let a = attribute(&replayed).unwrap();
    assert_eq!(a.rule, Rule::E3);
    let v = view(&replayed).unwrap();
    let evidence: Vec<Evidence> = a.evidence.iter().map(|e| v.event(*e).unwrap()).collect();
    let buses = v.buses();
    let input = Input {
        attribution: &a,
        evidence: &evidence,
        buses: &buses,
        facts: &BTreeMap::new(),
    };
    let ans = propose::propose(&[&Rules], &input);
    assert!(
        matches!(&ans.outcome, Outcome::NoProposal { reason, sizing }
            if reason == propose::MODEL_SLOT && sizing.is_empty()),
        "{:?}",
        ans.outcome
    );
}

/// Mutation sweep of `E3` (`AET-WO-2302` D5): an open RX line beside a port
/// whose silicon framing check fails is not shown to be the port's one
/// fault (I6, never electrical), and beside writes the access log shows
/// wrong it is the firmware's (S5): mutants `holds: true` and `t.holds()`
/// survived the cases above.
#[test]
fn an_open_rx_line_beside_a_failing_framing_check_or_wrong_writes_is_not_e3() {
    let rule = |case: Case, findings: &[(&str, u64)]| {
        let r = open(run(&case, findings), "CONSOLE", true);
        let a = attributed(&r);
        (a.domain, a.rule)
    };
    assert_eq!(
        rule(
            Case {
                console_holds: false,
                console: Echo::Missing,
                ..Case::default()
            },
            &[(SERIAL_ECHO_FINDING, 1), (SERIAL_STATIC_FINDING, 1)],
        ),
        (Domain::Indeterminate, Rule::I6)
    );
    assert_eq!(
        rule(
            Case {
                console_tags_hold: false,
                console: Echo::Missing,
                ..Case::default()
            },
            &[(SERIAL_ECHO_FINDING, 1), (SERIAL_TAG_FINDING, 1)],
        ),
        (Domain::Software, Rule::S5)
    );
}

// ---- AET-WO-2302b: the twin's serial window findings, refused as the
// harness's; and `motor_anchor_out_of_order` placed by the boot word ----

/// Sets each port's first frame byte's instant (kind 29's `injected_ns`).
fn injected(mut r: Vec<Record>, console: Option<u64>, gps: Option<u64>) -> Vec<Record> {
    for x in r.iter_mut() {
        if let Record::SerialWitness {
            label,
            injected_ns,
            rx_at_injection,
            ..
        } = x
        {
            let at = if label == "CONSOLE" { console } else { gps };
            *injected_ns = at;
            if at.is_none() {
                *rx_at_injection = None;
            }
        }
    }
    r
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

/// The console goes in at 98.3 ms: its frame and echo (886 876 ns at
/// 115 200 8N1) end inside the run, the GPS port's after it do not.
const LATE_CONSOLE: u64 = 98_300_000;

/// `H3`: the ports' wire time past the run is the twin's bound, and the
/// GPS port in it whose echo alone is missing is the same bound's: the run
/// is refused as the harness's, by the first injection (the boot word's
/// instant), the GPS port's witness and both ports' framings.
#[test]
fn h3_the_wire_time_past_the_run_is_the_harness_s_with_its_cut_echo() {
    let gps = Case {
        gps: Echo::Missing,
        ..Case::default()
    };
    let findings = [
        (SERIAL_ECHO_FINDING, 1),
        (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
    ];
    let r = injected(
        run(&gps, &findings),
        Some(LATE_CONSOLE),
        Some(LATE_CONSOLE + 900_000),
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::H3),
        "{a:?}"
    );
    assert_eq!(
        named(&r, &a),
        ["kind 27", "kind 27", "kind 29 CONSOLE", "kind 29 GPS"]
    );
    assert_eq!(
        Rule::H3.bound(),
        Some("the run's MAX_VIRTUAL_NS against the ports' wire time from the boot word")
    );
    // The console's own echo missing, its frame and echo inside the run: not
    // the bound's, so the port's (`I6`) names the run.
    let both = Case {
        console: Echo::Missing,
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = injected(
        run(
            &both,
            &[
                (SERIAL_ECHO_FINDING, 2),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        Some(LATE_CONSOLE),
        Some(LATE_CONSOLE + 900_000),
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I6),
        "{a:?}"
    );
    assert_eq!(named(&r, &a), ["kind 29 CONSOLE"]);
    // The GPS port in the sum, but silent: its pattern never went out, which
    // the bound does not explain, so its own `I6` names the run.
    let silent = Case {
        gps: Echo::Silent,
        ..Case::default()
    };
    let r = injected(
        run(
            &silent,
            &[
                (SERIAL_ECHO_FINDING, 1),
                (SERIAL_PATTERN_FINDING, 1),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        Some(LATE_CONSOLE),
        Some(LATE_CONSOLE + 900_000),
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I6, "{a:?}");
    assert_eq!(named(&r, &a), ["kind 29 GPS"]);
    // The GPS port in the sum, its frame back with its RX pin off its AF at
    // the injection (`not_counted`): the pin, not the bound, so its own
    // `I6` names the run, never the harness.
    let off = Case {
        gps: Echo::NotCounted,
        ..Case::default()
    };
    let r = injected(
        run(
            &off,
            &[
                (SERIAL_ECHO_FINDING, 1),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        Some(LATE_CONSOLE),
        Some(LATE_CONSOLE + 900_000),
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I6, "{a:?}");
    assert_eq!(named(&r, &a), ["kind 29 GPS"]);
}

/// `H3` never fires when the first port took no frame: the boot word's
/// instant the wire time runs from is then in no record, so the kind is
/// unexplained by name.
#[test]
fn h3_never_fires_without_the_boot_word_s_instant() {
    let none = Case {
        console: Echo::Missing,
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = injected(
        run(
            &none,
            &[
                (SERIAL_ECHO_FINDING, 2),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        None,
        None,
    );
    let a = attributed_any(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I1));
    assert_eq!(
        a.unexplained,
        [
            "serial_wire_past_run: the first port took no frame, so the boot word's instant the wire time runs from is in no record (kind 29's injected_ns)"
        ]
    );
}

/// A harness finding beside a fault of the device's never masks it: the
/// console's failing writes (`S5`), or the GPS port's RX line declared open
/// (`E3`), name the run, with their own evidence alone.
#[test]
fn h3_never_masks_a_serial_fault() {
    let c = Case {
        console_tags_hold: false,
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = injected(
        run(
            &c,
            &[
                (SERIAL_ECHO_FINDING, 1),
                (SERIAL_TAG_FINDING, 1),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        Some(LATE_CONSOLE),
        Some(LATE_CONSOLE + 900_000),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 CONSOLE"]);
    // The GPS port open, the console going in past the run's end less its
    // own wire time: its cut echo is the bound's, the open line electrical.
    let c = Case {
        console: Echo::Missing,
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = open(
        run(
            &c,
            &[
                (SERIAL_ECHO_FINDING, 2),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        "GPS",
        true,
    );
    let r = injected(r, Some(99_200_000), None);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E3), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 28 GPS", "kind 29 GPS"]);
}

/// `H4`: the judge's injection chain out of order is its own act, by the
/// ports it injected; with no port after the chain's first injected, no
/// record shows the chain, and the kind is unexplained by name. Beside the
/// console's failing writes, `S5` names the run.
#[test]
fn h4_an_injection_out_of_order_is_the_harness_s_by_the_chain() {
    let order = (campaign::SERIAL_INJECTION_ORDER_FINDING, 1);
    let r = run(&Case::default(), &[order]);
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::H4),
        "{a:?}"
    );
    assert_eq!(named(&r, &a), ["kind 29 CONSOLE", "kind 29 GPS"]);
    let gps = Case {
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = injected(
        run(&gps, &[(SERIAL_ECHO_FINDING, 1), order]),
        Some(130_750),
        None,
    );
    let a = attributed_any(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    assert_eq!(
        a.unexplained,
        [
            "serial_injection_out_of_order: named 1 time(s), but 0 port(s) after the chain's first took a frame (kind 29's injected_ns)"
        ]
    );
    let tags = Case {
        console_tags_hold: false,
        ..Case::default()
    };
    let r = run(&tags, &[(SERIAL_TAG_FINDING, 1), order]);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S5), "{a:?}");
}

/// [`run`] with a PWM output, `MOTOR1` (TIM1_CH1 on PA8 at AF1), witnessed
/// and anchored at `anchor`.
fn with_motor(c: &Case, anchor: u64, findings: &[(&str, u64)]) -> Vec<Record> {
    let mut r = run(c, findings);
    // Beside the solved ports, before the computed rises.
    let boot = r
        .iter()
        .position(|x| matches!(x, Record::RiseComputed { .. }))
        .unwrap();
    r.insert(
        boot,
        Record::MotorTimingCheck {
            label: "MOTOR1".into(),
            kernel_hz: 64_000_000,
            period_ps: 2_500_000_000,
            high_ps: 1_000_000_000,
            nominal_period_ps: 2_500_000_000,
            nominal_high_ps: 1_000_000_000,
            holds: true,
        },
    );
    r.insert(
        boot,
        Record::MotorOutputSolved {
            label: "MOTOR1".into(),
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
        },
    );
    let check = r
        .iter()
        .position(|x| matches!(x, Record::SerialFramingCheck { .. }))
        .unwrap();
    r.insert(
        check,
        Record::MotorWitness {
            label: "MOTOR1".into(),
            observer: "gpioPortA.pa8obs".into(),
            // At the 64 MHz kernel clock (`AET-WO-4400g`).
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
        },
    );
    r
}

/// `S8` where the records place the boot word (the console's injection
/// less a quantum, 30 750 ns): a timer anchored after it is out of order by
/// that instant, cited beside the boot and the output's witness. One
/// anchored before it shows nothing out of order in any record: the same
/// finding is then unexplained by name, never software.
#[test]
fn s8_an_anchor_after_the_boot_word_s_instant_cites_that_instant() {
    let order = [(campaign::MOTOR_ANCHOR_ORDER_FINDING, 1)];
    let r = with_motor(&Case::default(), 40_000, &order);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S8), "{a:?}");
    assert_eq!(named(&r, &a), ["kind 10", "kind 26", "kind 29 CONSOLE"]);
    let r = with_motor(&Case::default(), 6, &order);
    let a = attributed_any(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    assert!(
        a.unexplained[0]
            .starts_with("motor_anchor_out_of_order: named 1 time(s), 0 timer(s) anchored after"),
        "{a:?}"
    );
}

// ---- AET-WO-2302b fix round: the adversarial pass's findings, held ----

/// ADV-S4: replay refuses a first injection less than a quantum after the
/// boot's start: the judge injects the first port one quantum after the
/// boot word, which would then precede the run, and the wire time past the
/// run (kind 29's instant, `serial_wire_past_run`) would be unbound. At
/// exactly a quantum, the same run is accepted.
#[test]
fn adv_s4_replay_refuses_a_first_injection_under_a_quantum() {
    let gps = Case {
        gps: Echo::Missing,
        ..Case::default()
    };
    let at = |first: u64| {
        injected(
            run(&gps, &[(SERIAL_ECHO_FINDING, 1)]),
            Some(first),
            Some(2_000_000),
        )
    };
    assert!(campaign::check_run(&at(100_000)).is_ok());
    assert_eq!(
        campaign::check_run(&at(99_999)).err(),
        Some(
            "the first serial port's frame went in less than a quantum after the boot's start, before any boot word"
        )
    );
}

/// The GPS port's RX pin (PA3) in input mode at its injection, off its AF7
/// (kind 29's `rx_at_injection`).
fn gps_rx_unrouted(mut r: Vec<Record>) -> Vec<Record> {
    for x in r.iter_mut() {
        if let Record::SerialWitness {
            label,
            rx_at_injection,
            ..
        } = x
            && label == "GPS"
        {
            *rx_at_injection = Some((0b00, 0));
        }
    }
    r
}

/// ADV-S1: a port in the wire time past the run whose echo is missing with
/// its RX pin off its AF at the injection is the pin's, not the bound's:
/// `H3` never takes it, and its own `I6` names the run, as the same
/// records do with the wire inside the run; beside the console's failing
/// writes (`S5`) too, never the harness set aside for software.
#[test]
fn adv_s1_h3_never_takes_an_echo_whose_rx_pin_was_unrouted_at_the_injection() {
    for (case, more) in [
        (
            Case {
                gps: Echo::Missing,
                ..Case::default()
            },
            None,
        ),
        (
            Case {
                console_tags_hold: false,
                gps: Echo::Missing,
                ..Case::default()
            },
            Some((SERIAL_TAG_FINDING, 1)),
        ),
    ] {
        let inside: Vec<(&str, u64)> = [(SERIAL_ECHO_FINDING, 1)].into_iter().chain(more).collect();
        let past: Vec<(&str, u64)> = inside
            .iter()
            .copied()
            .chain([(campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1)])
            .collect();
        let control = attributed(&gps_rx_unrouted(run(&case, &inside)));
        assert_eq!(control.rule, Rule::I6, "{control:?}");
        let r = gps_rx_unrouted(injected(
            run(&case, &past),
            Some(LATE_CONSOLE),
            Some(LATE_CONSOLE + 900_000),
        ));
        let a = attributed(&r);
        assert_eq!(
            (a.domain, a.rule),
            (Domain::Indeterminate, Rule::I6),
            "{a:?}"
        );
        assert_eq!(named(&r, &a), ["kind 29 GPS"]);
        // Routed at the injection, the same echo is the bound's.
        let routed = injected(
            run(&case, &past),
            Some(LATE_CONSOLE),
            Some(LATE_CONSOLE + 900_000),
        );
        let a = attributed(&routed);
        let want = if more.is_some() { Rule::S5 } else { Rule::H3 };
        assert_eq!(a.rule, want, "{a:?}");
    }
    // A port the run ended before injecting has no RX pin read at an
    // injection: its missing echo stays the bound's.
    let none = Case {
        console: Echo::Missing,
        gps: Echo::Missing,
        ..Case::default()
    };
    let r = injected(
        run(
            &none,
            &[
                (SERIAL_ECHO_FINDING, 2),
                (campaign::SERIAL_WIRE_PAST_RUN_FINDING, 1),
            ],
        ),
        Some(99_500_000),
        None,
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::H3, "{a:?}");
}
