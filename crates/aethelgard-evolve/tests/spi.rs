//! `AET-WO-4200` D4, tier 1: attribution of an SPI device's identity read
//! (`Rule::E2`, `Rule::S3`) on campaign runs written through
//! `aethelgard-chaos`'s own writer, with no Renode. The base run is the
//! reference board's I2C run (as `attribute.rs` builds it) with U4's kind 21
//! and kind 22 added where `check_run` ranks them: after the I2C records and
//! before the verdict. The log lines are the ones Renode 1.17.0 measured.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::campaign::{self, run_id};
use aethelgard_chaos::record::{BusLine, Override, Reading, Record};
use aethelgard_chaos::{Event, Header, Replayed, TraceWriter, replay_dir};
use aethelgard_evolve::propose::{self, BusFacts, Input, MODEL_SLOT, Outcome, Rules};
use aethelgard_evolve::{Attribution, Domain, EventRef, Evidence, Rule, attribute, view};

const DEV: &str = "spi1.spi1mux.u4";
const MUX: &str = "spi1.spi1mux";
const EXPECTED_ID: u64 = 0x00EF_4017;

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

fn rise(net: &str) -> Record {
    Record::RiseMeasured {
        bus: "SENSORS".into(),
        net: net.into(),
        rise_30_70_ps: 186_402,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
    }
}

fn register(name: &str) -> Record {
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
            observed: Some(expected),
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
        expected: lines.clone(),
        observed: Some(lines),
        device,
        absent: Vec::new(),
    }
}

/// The healthy I2C part of the run, up to U2's witness; the SPI records and
/// the verdict follow.
fn base() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "spi-reference".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
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
        // U4 on SPI1 at its multiplexer's address 0, chip select PA0 (kind 23).
        Record::SpiBusSolved {
            peripheral: "SPI1".into(),
            devices: vec![("U4".into(), 0, "PA0".into())],
        },
        rise("I2C_SCL"),
        rise("I2C_SDA"),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0x2100_B007),
            instructions_expected: 10_000_000,
            instructions_observed: Some(10_000_000),
            pc: Some(0x0800_0596),
        },
        register("GPIOA.MODER"),
        register("GPIOB.MODER"),
        register("GPIOB.AFRL"),
        register("RCC.AHB4ENR"),
        register("RCC.APB1LENR"),
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x76,
            register: 0xD0,
            expected: 0x60,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
        i2c_witness(),
    ]
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|l| (*l).to_owned()).collect()
}

/// The flash's witness for U4, as Renode logged it: 8 lines.
fn flash_ok() -> Vec<String> {
    lines(&[
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x9F, current state: RecognizeOperation",
        "[NOISY] spi1.spi1mux.u4: Decoded operation: Operation: ReadID, write enabled False",
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand",
        "[NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0xEF",
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand",
        "[NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x40",
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand",
        "[NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x17",
    ])
}

/// The model's lines for a read of 0x90 (a wrong command), then three zeros.
fn flash_wrong_command() -> Vec<String> {
    let mut v = lines(&[
        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x90, current state: RecognizeOperation",
    ]);
    for _ in 0..3 {
        v.extend(lines(&[
            "[NOISY] spi1.spi1mux.u4: Transmitting data 0x0, current state: HandleCommand",
            "[NOISY] spi1.spi1mux.u4: Handled command: Operation: None, returning 0x0",
        ]));
    }
    v
}

const DUMMY: &str = "- ignoring transfer and returning a dummy byte";

/// The multiplexer's four lines (0x9F, then 0x0 three times) for a device
/// absent at `address`.
fn not_connected(address: u8) -> Vec<String> {
    [0x9Fu8, 0, 0, 0]
        .iter()
        .map(|b| {
            format!(
                "[WARNING] {MUX}: Tried to transmit byte {b:#x} to device {address:#x}, but it's not connected {DUMMY}"
            )
        })
        .collect()
}

/// The four lines for a chip select never asserted.
fn no_device_selected() -> Vec<String> {
    [0x9Fu8, 0, 0, 0]
        .iter()
        .map(|b| {
            format!(
                "[WARNING] {MUX}: Tried to transmit byte {b:#x}, but no device is currently selected {DUMMY}"
            )
        })
        .collect()
}

/// Another part's chip select held beside this one's.
fn multiple_selected() -> Vec<String> {
    vec![format!(
        "[WARNING] {MUX}: Tried to transmit byte 0x9F, but multiple devices are currently selected ([0x0, 0x1]) {DUMMY}"
    )]
}

fn spi_check(status: &str, observed: Option<u64>) -> Record {
    Record::SpiCheck {
        reference: "U4".into(),
        peripheral: "SPI1".into(),
        select: "PA0".into(),
        address: 0,
        command: 0x9F,
        expected: EXPECTED_ID,
        observed,
        status: Some(status.into()),
    }
}

fn spi_witness(observed: Option<Vec<String>>, unrouted: Vec<String>) -> Record {
    Record::SpiWitness {
        peripheral: "SPI1".into(),
        reference: "U4".into(),
        device: DEV.into(),
        expected: flash_ok(),
        observed,
        unrouted,
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

const BOTH: [(&str, u64); 2] = [
    ("spi_identity_mismatch", 1),
    ("spi_transaction_unwitnessed", 1),
];

/// The base run with U4's read, its witness and the verdict.
fn run(check: Record, witness: Record, findings: &[(&str, u64)]) -> Vec<Record> {
    let mut r = base();
    r.extend([check, witness, verdict(findings)]);
    r
}

/// Index of the first record of `kind`.
fn at(r: &[Record], kind: u64) -> u64 {
    r.iter().position(|x| x.kind() == kind).unwrap() as u64
}

fn archived(records: &[Record]) -> Replayed {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "aet-4200-evolve-spi-{}-{}",
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

/// `check_run` accepts the records, then they are attributed.
fn attributed(records: &[Record]) -> Attribution {
    campaign::check_run(records).expect("the run is one check_run accepts");
    let run = archived(records);
    let a = attribute(&run).unwrap();
    assert_eq!(attribute(&run).unwrap(), a);
    for e in a.evidence.iter().chain(&a.symptoms).chain(&a.masked) {
        assert_eq!(records[e.index as usize].kind(), e.kind, "{a:?}");
    }
    for e in &a.evidence {
        assert!(a.rule.evidence_kinds().contains(&e.kind), "{a:?}");
    }
    assert!(a.unexplained.is_empty(), "{a:?}");
    a
}

fn ev(index: u64, kind: u64) -> EventRef {
    EventRef { index, kind }
}

/// The absent device: Renode routed to address 0 and found nothing.
fn absent_run() -> Vec<Record> {
    run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), not_connected(0)),
        &BOTH,
    )
}

#[test]
fn a_device_absent_at_its_own_address_is_electrical_e2_by_its_witness() {
    let r = absent_run();
    let a = attributed(&r);
    let (c, w) = (at(&r, 21), at(&r, 22));
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E2), "{a:?}");
    assert_eq!(a.evidence, vec![ev(w, 22)]);
    assert_eq!(a.symptoms, vec![ev(c, 21), ev(w, 22)]);
    assert!(a.masked.is_empty());
}

#[test]
fn a_chip_select_never_asserted_is_software_s3() {
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), no_device_selected()),
        &BOTH,
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S3), "{a:?}");
    assert_eq!(a.evidence, vec![ev(at(&r, 22), 22)]);
}

#[test]
fn another_parts_chip_select_held_is_software_s3() {
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), multiple_selected()),
        &BOTH,
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S3), "{a:?}");
    assert_eq!(a.evidence, vec![ev(at(&r, 22), 22)]);
}

#[test]
fn a_wrong_command_the_model_logged_is_software_s3() {
    let r = run(
        spi_check("ok", Some(0)),
        spi_witness(Some(flash_wrong_command()), vec![]),
        &BOTH,
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S3), "{a:?}");
    assert_eq!(a.evidence, vec![ev(at(&r, 22), 22)]);
}

#[test]
fn a_claimed_read_with_nothing_logged_or_routed_is_s3_naming_the_claim() {
    // The record says the read completed with the right value (so only the
    // witness finding fires) and nothing reached the model or the mux: only
    // the firmware's own claim can say that.
    let r = run(
        spi_check("ok", Some(EXPECTED_ID)),
        spi_witness(Some(vec![]), vec![]),
        &[("spi_transaction_unwitnessed", 1)],
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S3), "{a:?}");
    assert_eq!(
        a.evidence,
        vec![ev(at(&r, 21), 21), ev(at(&r, 22), 22)],
        "{a:?}"
    );
}

#[test]
fn a_not_connected_line_for_another_address_is_not_electrical() {
    // The mux routed the read to device 0x1, not U4's 0x0, and the model
    // logged nothing. Not E2: E2 needs the device's OWN address. Nor S3: no
    // S3 condition holds (no no-device or multiple-device line, the model
    // logged nothing wrong, and the unrouted list is not empty so it is not a
    // bare claim). So the code gives I4 with no evidence. (I2C counts a wrong
    // address as software evidence; SPI does not, which is reported as an
    // observation, not a contradiction of a stated rule.)
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), not_connected(1)),
        &BOTH,
    );
    let a = attributed(&r);
    assert_ne!(a.domain, Domain::Electrical, "{a:?}");
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I4),
        "{a:?}"
    );
    assert!(a.evidence.is_empty());
}

#[test]
fn electrical_and_software_evidence_on_one_device_is_e2_with_the_software_masked() {
    let mut unrouted = not_connected(0);
    unrouted.extend(no_device_selected());
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), unrouted),
        &BOTH,
    );
    let a = attributed(&r);
    let w = at(&r, 22);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E2), "{a:?}");
    assert_eq!(a.evidence, vec![ev(w, 22)]);
    // Both halves live in the one witness, so the masked software event is
    // that same kind 22.
    assert_eq!(a.masked, vec![ev(w, 22)]);
}

#[test]
fn a_witness_that_could_not_be_read_is_no_evidence_and_i4() {
    // `observed: None`: `spi_evidence` returns nothing, so `decide` falls to
    // I4 (the device's bus is not in question and no evidence exists). The
    // run is not I1 because both failing findings do select events.
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(None, vec![]),
        &BOTH,
    );
    let a = attributed(&r);
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I4),
        "{a:?}"
    );
    assert!(a.evidence.is_empty() && a.masked.is_empty());
}

fn answer(records: &[Record]) -> (Attribution, propose::Answer) {
    let run = archived(records);
    let a = attribute(&run).unwrap();
    let v = view(&run).unwrap();
    let evidence: Vec<Evidence> = a.evidence.iter().map(|e| v.event(*e).unwrap()).collect();
    let buses = v.buses();
    let facts: BTreeMap<String, BusFacts> = BTreeMap::from([(
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
    (a, ans)
}

#[test]
fn e2_has_no_rule_and_no_proposal() {
    let (a, ans) = answer(&absent_run());
    assert_eq!(a.rule, Rule::E2);
    assert_eq!(ans.proposer, None, "{ans:?}");
    match ans.outcome {
        Outcome::NoProposal { reason, sizing } => {
            assert_eq!(reason, MODEL_SLOT);
            assert!(sizing.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn s3_is_answered_by_rules_and_never_as_a_proposal() {
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), no_device_selected()),
        &BOTH,
    );
    let (a, ans) = answer(&r);
    assert_eq!(a.rule, Rule::S3);
    assert_eq!(ans.proposer, Some("rules"), "{ans:?}");
    assert!(!matches!(ans.outcome, Outcome::Proposal(_)), "{ans:?}");
}

#[test]

fn s3_proposes_a_generator_report() {
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), no_device_selected()),
        &BOTH,
    );
    let (_, ans) = answer(&r);
    assert_eq!(ans.proposer, Some("rules"));
    assert!(
        matches!(ans.outcome, Outcome::GeneratorReport(_)),
        "{ans:?}"
    );
}

/// `AET-WO-4200` adversarial A3: replay joins every SPI record to the solved
/// controller (kind 23) and refuses what does not, as it refuses an I2C read
/// at an address kind 15 does not give. Each hostile archive here was
/// accepted by the build at `1fe460d`, and one attributed `E2`.
#[test]
fn replay_refuses_spi_records_that_do_not_join_the_solved_controller() {
    let refused = |records: &[Record], why: &str| {
        let err = campaign::check_run(records).unwrap_err();
        assert!(err.contains(why), "{err}");
    };
    let absent = || {
        run(
            spi_check("ok", Some(0xFF_FFFF)),
            spi_witness(Some(vec![]), not_connected(0)),
            &BOTH,
        )
    };
    // H3: the read at address 1, its solved address 0, with `not connected` at 1.
    let mut r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), not_connected(1)),
        &BOTH,
    );
    for x in r.iter_mut() {
        if let Record::SpiCheck { address, .. } = x {
            *address = 1;
        }
    }
    refused(&r, "no solved device at its address");
    // H11: a chip select the solved controller does not give.
    let mut r = absent();
    for x in r.iter_mut() {
        if let Record::SpiCheck { select, .. } = x {
            *select = "PZ99".into();
        }
    }
    refused(&r, "no solved device at its address");
    // H2: a `not connected` line from a source that is not the multiplexer.
    let other = vec![format!(
        "[WARNING] i2c1: Tried to transmit byte 0x9F to device 0x0, but it's not connected {DUMMY}"
    )];
    let r = run(
        spi_check("ok", Some(0xFF_FFFF)),
        spi_witness(Some(vec![]), other),
        &BOTH,
    );
    refused(&r, "not its multiplexer's");
    // H15: an absent device that no solved controller has.
    let mut r = absent();
    if let Record::Scenario { overrides, .. } = &mut r[0] {
        overrides.push(Override::SpiDeviceAbsent {
            reference: "U9".into(),
        });
    }
    refused(&r, "no solved one");
    // H14: the scenario leaves U4 absent, yet its model logged the read.
    let mut r = run(
        spi_check("ok", Some(EXPECTED_ID)),
        spi_witness(Some(flash_ok()), vec![]),
        &[],
    );
    if let Record::Scenario { overrides, .. } = &mut r[0] {
        overrides.push(Override::SpiDeviceAbsent {
            reference: "U4".into(),
        });
    }
    refused(&r, "absent SPI device's model logged");
    // The absent run itself, with its override, is one replay accepts.
    let mut r = absent();
    if let Record::Scenario { overrides, .. } = &mut r[0] {
        overrides.push(Override::SpiDeviceAbsent {
            reference: "U4".into(),
        });
    }
    campaign::check_run(&r).unwrap();
}
