//! `AET-WO-3100` §3.4, tier 1: attribution on campaign runs written through
//! `aethelgard-chaos`'s own writer, with no Renode and no bridge. The healthy
//! run below is the reference board's, as `campaign` archives it for
//! `pullup-2k2` (the register addresses and values are the archived ones);
//! each test changes it the way a failure would, keeps it a run `replay`
//! accepts, and reads the attribution back.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::campaign::{self, run_id};
use aethelgard_chaos::record::{BusLine, Line, Override, Reading, Record};
use aethelgard_chaos::{Event, Header, Replayed, TraceWriter, replay_dir};
use aethelgard_evolve::{AttributeError, Attribution, Domain, Rule, attribute};

fn reading(address: u32, mask: u32, expected: u32, observed: u32) -> Reading {
    Reading {
        address,
        mask,
        expected,
        observed: Some(observed),
    }
}

fn line(peripheral: &str, suffix: &str, net: &str, pin: &str) -> BusLine {
    BusLine {
        signal: format!("{peripheral}{suffix}"),
        net: net.into(),
        pin: pin.into(),
        alternate_function: 4,
        ceiling_ps: 300_000,
    }
}

fn solved(
    bus: &str,
    peripheral: &str,
    nets: [&str; 2],
    pins: [&str; 2],
    devices: &[(&str, u8)],
) -> Record {
    Record::I2cBusSolved {
        bus: bus.into(),
        peripheral: peripheral.into(),
        lines: vec![
            line(peripheral, "_SCL", nets[0], pins[0]),
            line(peripheral, "_SDA", nets[1], pins[1]),
        ],
        devices: devices.iter().map(|(r, a)| ((*r).into(), *a)).collect(),
    }
}

fn rise(bus: &str, net: &str, ps: u64) -> Record {
    Record::RiseComputed {
        bus: bus.into(),
        net: net.into(),
        rise_30_70_ps: ps,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: ps > 300_000,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    }
}

fn fault(bus: &str, nets: &[&str]) -> Record {
    Record::BusFault {
        bus: bus.into(),
        fault: "absent".into(),
        source: "aethelgard".into(),
        from: vec!["analog.rise_time_computed > ceiling_ns".into()],
        nets: nets.iter().map(|n| (*n).into()).collect(),
        firmware_sees: "absent".into(),
    }
}

fn check(status: &str, observed: u64) -> Record {
    Record::I2cCheck {
        reference: "U2".into(),
        address: 0x76,
        step: "identity".into(),
        write: vec![0xD0],
        width: 1,
        expected: 0x60,
        mask: 0xFF,
        observed: Some(observed),
        status: Some(status.into()),
    }
}

/// U2's identity read as the BME280 model logs it (register 0xD0, value 0x60),
/// without the `[NOISY] i2c1.u2: ` every line starts with.
const OK: [&str; 2] = ["Write D0", "Read 60"];

/// A witness of `reference` on `peripheral` (`AET-WO-2204`): what the judge
/// expects the model to log, and what Renode logged (`None`: unreadable).
fn witness_with(
    peripheral: &str,
    reference: &str,
    expected: &[&str],
    observed: Option<&[&str]>,
) -> Record {
    let device = format!(
        "{}.{}",
        peripheral.to_ascii_lowercase(),
        reference.to_ascii_lowercase()
    );
    let lines = |v: &[&str]| -> Vec<String> {
        v.iter().map(|l| format!("[NOISY] {device}: {l}")).collect()
    };
    Record::I2cWitness {
        step: "identity".into(),
        peripheral: peripheral.into(),
        reference: reference.into(),
        expected: lines(expected),
        observed: observed.map(lines),
        device,
        absent: Vec::new(),
    }
}

/// U2's witness with the lines Renode logged for it: [`OK`] is a read that
/// went as solved, `Some(&[])` a device that logged nothing.
fn witness(observed: Option<&[&str]>) -> Record {
    witness_with("I2C1", "U2", &OK, observed)
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

/// The names `campaign` gives the registers it reads.
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
        executable: vec![(0, 0x1000_0000)],
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
    }
}

/// The healthy reference run: SENSORS on I2C1, both lines at 186 ns, U2 read.
fn healthy() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "pullup-2k2".into(),
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
        solved(
            "SENSORS",
            "I2C1",
            ["I2C_SCL", "I2C_SDA"],
            ["PB6", "PB7"],
            &[("U2", 0x76), ("U3", 0x48)],
        ),
        rise("SENSORS", "I2C_SCL", 186_402),
        rise("SENSORS", "I2C_SDA", 186_402),
        boot(0x2100_B007),
        register("GPIOA.MODER", None),
        register("GPIOB.MODER", None),
        register("GPIOB.AFRL", None),
        register("RCC.AHB4ENR", None),
        register("RCC.APB1LENR", None),
        check("ok", 0x60),
        witness(Some(&OK)),
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

fn is_check(r: &Record) -> bool {
    matches!(r, Record::I2cCheck { .. })
}
fn is_witness(r: &Record) -> bool {
    matches!(r, Record::I2cWitness { .. })
}
fn is_verdict(r: &Record) -> bool {
    matches!(r, Record::Verdict { .. })
}

/// Write `records` as a campaign run through the chaos writer, header and
/// run id consistent, and read it back as `replay` does.
fn archived(records: &[Record]) -> Replayed {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "aet-3100-evolve-{}-{}",
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
        tools: aethelgard_chaos::campaign::tools_of(records)
            .iter()
            .map(|n| ((*n).to_owned(), "test".to_owned()))
            .collect(),
        // `AET-WO-2310`: the header hashes ngspice's library and the toolchain's
        // sysroot whenever they judged the run; a fixed hash stands in.
        tool_sha256: aethelgard_chaos::campaign::tools_of(records)
            .iter()
            .filter(|n| {
                [
                    aethelgard_chaos::campaign::NGSPICE,
                    aethelgard_chaos::campaign::RUST_TOOLCHAIN,
                ]
                .contains(n)
            })
            .map(|n| ((*n).to_owned(), [7; 32]))
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

const UNBOUND: &str =
    "a solved line has no computed rise, stuck-high record or held-low fault, or more than one";
const BOOT_WORD: &str = "the verdict's boot_word_missing does not match the boot word";
const HALTED: &str = "the verdict's cpu_halted does not match the booted core's instruction count";

/// Why `attribute` refuses `records`: a rule `replay` judges by.
fn refused(records: &[Record]) -> &'static str {
    match attribute(&archived(records)) {
        Err(AttributeError::NotACampaignRun(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The reason `replay` refuses a verdict whose count of `kind` its records
/// do not bound (`aethelgard_chaos::campaign::SHOWN`, `AET-WO-2312`).
fn refusal(kind: &str) -> &'static str {
    aethelgard_chaos::campaign::SHOWN
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, why)| *why)
        .unwrap()
}

/// `AET-WO-4210` F-B10: an identity read's mismatch named as a data or
/// configuration step's selects nothing, and the run is `I1`; named as its
/// own step's, it is decided as before.
#[test]
fn an_identity_mismatch_dressed_as_another_steps_is_refused() {
    let mut r = healthy();
    replace(&mut r, is_check, check("ok", 0x61));
    replace(&mut r, is_verdict, verdict(&[("i2c_identity_mismatch", 1)]));
    assert_ne!(attributed(&r).rule, Rule::I1);
    // Dressed as another step's, the identity read's own goes unnamed: replay
    // refuses it by that kind, and attribute with it (`AET-WO-2312`).
    for dressed in ["i2c_data_mismatch", "i2c_configure_mismatch"] {
        replace(&mut r, is_verdict, verdict(&[(dressed, 1)]));
        assert_eq!(refused(&r), refusal("i2c_identity_mismatch"), "{dressed}");
    }
}

/// `AET-WO-3201` B2: a boot word missing that no finding names. It was
/// `I1`; `replay` now refuses it by name.
#[test]
fn a_boot_word_missing_unnamed_is_refused() {
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    assert_eq!(refused(&r), BOOT_WORD);
}

fn attributed(records: &[Record]) -> Attribution {
    let run = archived(records);
    let a = attribute(&run).unwrap();
    // Deterministic: a second call gives the same record.
    assert_eq!(attribute(&run).unwrap(), a);
    // Every index in range, its kind the record's, and one the rule allows.
    for e in a.evidence.iter().chain(&a.symptoms).chain(&a.masked) {
        assert_eq!(records[e.index as usize].kind(), e.kind, "{a:?}");
    }
    for e in &a.evidence {
        assert!(a.rule.evidence_kinds().contains(&e.kind), "{a:?}");
    }
    a
}

fn kinds(refs: &[aethelgard_evolve::EventRef]) -> Vec<u64> {
    refs.iter().map(|e| e.kind).collect()
}

/// The wrong-register shape: U2 times out waiting for the read, and the model
/// logged a write of register 0xD1 where 0xD0 was solved.
fn wrong_register(r: &mut [Record]) {
    replace(r, is_check, check("timeout_rxne", 0));
    replace(r, is_witness, witness(Some(&["Write D1"])));
    replace(r, is_verdict, verdict(&[("i2c_timeout", 1)]));
}

/// A 47 kΩ shape on SENSORS: both rises past the ceiling and the bus fault.
fn faulted_sensors(r: &mut Vec<Record>) {
    for net in ["I2C_SCL", "I2C_SDA"] {
        let i = find(
            r,
            |x| matches!(x, Record::RiseComputed { bus, net: n, .. } if bus == "SENSORS" && n == net),
        );
        r[i] = rise("SENSORS", net, 3_982_301);
    }
    insert_after_last(
        r,
        |x| {
            matches!(
                x,
                Record::RiseComputed { .. } | Record::LineStuckHigh { .. }
            )
        },
        fault("SENSORS", &["I2C_SCL", "I2C_SDA"]),
    );
}

#[test]
fn the_healthy_run_passes_and_is_none() {
    let a = attributed(&healthy());
    assert_eq!((a.domain, a.rule), (Domain::None, Rule::N1));
    assert!(a.evidence.is_empty() && a.symptoms.is_empty());
}

#[test]
fn a_wrong_register_on_a_measured_healthy_bus_is_software_by_the_witness() {
    let mut r = healthy();
    wrong_register(&mut r);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1));
    assert_eq!(kinds(&a.evidence), [7, 7, 13]);
    assert_eq!(kinds(&a.symptoms), [12]);
    assert!(a.masked.is_empty());
}

#[test]
fn the_same_wrong_register_on_a_faulted_bus_is_electrical_with_the_witness_masked() {
    let mut r = healthy();
    wrong_register(&mut r);
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E1));
    assert_eq!(kinds(&a.evidence), [7, 7, 9]);
    assert_eq!(kinds(&a.masked), [13]);
}

#[test]
fn a_txis_timeout_with_nothing_logged_is_indeterminate_not_software() {
    // A transfer that never got past the address leaves the device's log
    // empty, which a fault on the bus does too: it is no evidence about a TXIS
    // timeout. The bus is healthy, but nothing says which half.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I4));
    assert_eq!(kinds(&a.evidence), [7, 7]);
}

/// A line of a solved bus with no computed rise. It was `I2`; since
/// `AET-WO-3201` B1, `replay` refuses it, so it is never attributed.
#[test]
fn no_rise_measured_on_a_line_of_the_symptoms_bus_is_refused() {
    let mut r = healthy();
    wrong_register(&mut r);
    r.retain(|x| !matches!(x, Record::RiseComputed { net, .. } if net == "I2C_SDA"));
    assert_eq!(refused(&r), UNBOUND);
}

/// §5.3: removing all the electrical evidence never concludes either half:
/// since `AET-WO-3201` B1, the run is refused.
#[test]
fn removing_every_rise_never_turns_a_software_run_electrical_or_keeps_it_software() {
    let mut r = healthy();
    wrong_register(&mut r);
    r.retain(|x| !matches!(x, Record::RiseComputed { .. }));
    assert_eq!(refused(&r), UNBOUND);
}

/// A second bus, OTHER on I2C2, with no device, faulted past its ceiling.
fn other_bus_faulted(r: &mut Vec<Record>) {
    let at = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        solved("OTHER", "I2C2", ["O_SCL", "O_SDA"], ["PF1", "PF0"], &[]),
    );
    let i = find(r, |x| matches!(x, Record::RiseComputed { .. }));
    r.insert(i, rise("OTHER", "O_SCL", 900_000));
    r.insert(i + 1, rise("OTHER", "O_SDA", 900_000));
    let after = find(r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(after, fault("OTHER", &["O_SCL", "O_SDA"]));
}

#[test]
fn a_fault_on_another_bus_is_indeterminate_because_the_fault_is_elsewhere() {
    // R7: two buses through the chaos writer. U2 times out on SENSORS, which is
    // computed healthy (its rises are present); the only fault is on OTHER.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    other_bus_faulted(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I3));
    // The healthy symptom bus's rises and the other bus's faults.
    let buses: Vec<&str> = a
        .evidence
        .iter()
        .map(|e| match &r[e.index as usize] {
            Record::RiseComputed { bus, .. } | Record::BusFault { bus, .. } => bus.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(buses, ["OTHER", "OTHER", "SENSORS", "SENSORS", "OTHER"]);
}

#[test]
fn an_electrical_bus_and_a_software_bus_in_one_run_are_indeterminate_never_averaged() {
    let mut r = healthy();
    wrong_register(&mut r);
    other_bus_faulted(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I5));
    assert_eq!(kinds(&a.evidence), [7, 7, 7, 7, 9, 13]);
}

#[test]
fn a_finding_no_archived_event_shows_is_indeterminate_and_named() {
    let mut r = healthy();
    replace(&mut r, is_verdict, verdict(&[("renode_error_log", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I1));
    assert_eq!(a.unexplained, ["renode_error_log"]);
    assert!(a.evidence.is_empty());
}

#[test]
fn changing_only_the_verdicts_finding_kinds_does_not_move_the_domain() {
    // A wrong-register run (the read completed, the model logged Write D1) whose verdict
    // is rewritten to claim a timeout: no event shows one, so it is I1, not
    // whatever a timeout would have been attributed to.
    let mut r = healthy();
    replace(&mut r, is_check, check("ok", 0x00));
    replace(&mut r, is_witness, witness(Some(&["Write D1", "Read 0"])));
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("i2c_identity_mismatch", 1),
            ("i2c_transaction_unwitnessed", 1),
        ]),
    );
    let honest = attributed(&r);
    assert_eq!((honest.domain, honest.rule), (Domain::Software, Rule::S1));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    // No record shows a timeout: refused by that kind (`AET-WO-2312`).
    assert_eq!(refused(&r), refusal("i2c_timeout"));
    // And claiming the analog finding on a healthy bus is refused by replay's rules.
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2)]),
    );
    let run = archived(&r);
    assert!(attribute(&run).is_err());
}

#[test]
fn a_finding_count_the_events_do_not_reach_is_refused() {
    let mut r = healthy();
    wrong_register(&mut r);
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 2)]));
    assert_eq!(refused(&r), refusal("i2c_timeout"));
}

#[test]
fn a_witness_agreeing_with_a_wrong_register_leaves_no_software_evidence() {
    // The firmware's register was wrong, but the witness is rewritten to agree
    // with the solved read: attribution has nothing, and says so.
    let mut r = healthy();
    wrong_register(&mut r);
    replace(&mut r, is_witness, witness(Some(&OK)));
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_witness_whose_own_expectation_disagrees_with_the_solved_device_is_no_evidence() {
    let mut r = healthy();
    wrong_register(&mut r);
    let w = witness_with("I2C1", "U2", &["Write D1", "Read 60"], Some(&["Write D1"]));
    replace(&mut r, is_witness, w);
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_read_counts_only_once_the_model_logged_one() {
    // A timeout after the write: the model logged the right write and no
    // read, so nothing is wrong yet.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_rxne", 0));
    replace(&mut r, is_witness, witness(Some(&["Write D0"])));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    // Once it logged a read, another value is evidence.
    replace(&mut r, is_witness, witness(Some(&["Write D0", "Read 61"])));
    assert_eq!(attributed(&r).rule, Rule::S1);
}

#[test]
fn a_transfer_the_controller_found_no_device_for_is_evidence_unless_it_is_the_devices_own() {
    // `AET-WO-2204`: the seeded set's `wrong-address`. A TXIS timeout with
    // nothing logged by the model is no evidence alone; the controller's
    // `Unknown slave at address 119.` (0x77, not U2's 0x76) is the firmware's.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    let i = find(&r, is_witness);
    let absent = |r: &mut Vec<Record>, a: Vec<u8>| {
        if let Record::I2cWitness { absent, .. } = &mut r[i] {
            *absent = a;
        }
    };
    absent(&mut r, vec![0x77]);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1));
    // A faulted bus finds no device even at U2's own address: no evidence.
    absent(&mut r, vec![0x76]);
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_wrong_alternate_function_on_scl_is_software_by_its_register() {
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOB.AFRL"),
        register("GPIOB.AFRL", Some(0x4500_0000)),
    );
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1));
    assert_eq!(kinds(&a.evidence), [7, 7, 11]);
}

#[test]
fn a_skipped_controller_clock_is_software_and_explains_the_locked_access() {
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "RCC.APB1LENR"),
        register("RCC.APB1LENR", Some(0)),
    );
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("i2c_timeout", 1),
            ("locked_peripheral_access", 1),
            ("register_mismatch", 1),
        ]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1));
    // The device's log is empty, as a gated controller leaves it: no
    // evidence of an address or a register. The clock gate left off is.
    assert_eq!(kinds(&a.evidence), [7, 7, 11]);
}

#[test]
fn a_locked_access_with_no_clock_gate_left_off_is_unexplained() {
    let mut r = healthy();
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I1);
}

#[test]
fn a_register_no_bus_owns_is_board_software_only_when_every_bus_is_healthy() {
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOA.MODER"),
        register("GPIOA.MODER", Some(0x0000_A800)),
    );
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S2));
    other_bus_faulted(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("register_mismatch", 1)]),
    );
    let a = attributed(&r);
    assert_eq!(a.domain, Domain::Indeterminate);
}

/// `3100` F13, `AET-WO-2205` §1.5: kind 10 archives the executable
/// segments, so a core that ended outside them is software evidence on
/// healthy buses (`S2`), never by elimination.
#[test]
fn a_core_outside_the_archived_segments_is_software_on_healthy_buses() {
    let mut r = healthy();
    let outside = |pc: u32, executable: Vec<(u32, u32)>| Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0x2100_B007),
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(pc),
        executable,
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
    };
    let segments = vec![(0x0800_0000, 0x0800_1000)];
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        outside(0x7000_0000, segments.clone()),
    );
    replace(&mut r, is_verdict, verdict(&[("pc_outside_firmware", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S2));
    assert_eq!(kinds(&a.evidence), [7, 7, 10]);
    assert_eq!(kinds(&a.symptoms), [10]);
    // A faulted bus elsewhere: the boot is not on it, so the fault is elsewhere.
    other_bus_faulted(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("analog_rise_exceeds_ceiling", 2),
            ("pc_outside_firmware", 1),
        ]),
    );
    assert_eq!(attributed(&r).domain, Domain::Indeterminate);
}

#[test]
fn a_missing_boot_word_is_software_on_healthy_buses_and_never_by_elimination() {
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S2));
    assert_eq!(kinds(&a.evidence), [7, 7, 10]);
    // With no bus measured, nothing says the electrical layer is healthy,
    // and `replay` refuses the run (`AET-WO-3201` B1).
    r.retain(|x| !matches!(x, Record::RiseComputed { .. }));
    assert_eq!(refused(&r), UNBOUND);
    // With a faulted bus, the boot is not on it: the fault is elsewhere.
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("boot_word_missing", 1)]),
    );
    let a = attributed(&r);
    assert_eq!(a.domain, Domain::Indeterminate);
}

#[test]
fn a_held_low_line_is_electrical_by_its_fault() {
    let mut r = healthy();
    if let Record::Scenario { overrides, .. } = &mut r[0] {
        overrides.push(Override::LineHeldLow {
            bus: "SENSORS".into(),
            line: Line::Sda,
        });
    }
    r.retain(|x| !matches!(x, Record::RiseComputed { net, .. } if net == "I2C_SDA"));
    let at = find(&r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(
        at,
        Record::BusFault {
            bus: "SENSORS".into(),
            fault: "absent".into(),
            source: "scenario held".into(),
            from: vec![campaign::HELD_LOW.into()],
            nets: vec!["I2C_SDA".into()],
            firmware_sees: "absent".into(),
        },
    );
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_line_held_low", 1), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Electrical, Rule::E1));
    assert_eq!(kinds(&a.evidence), [9]);
    assert_eq!(kinds(&a.symptoms), [9, 12]);
}

#[test]
fn a_run_replay_refuses_is_refused_by_attribute() {
    let mut r = healthy();
    r.retain(|x| !matches!(x, Record::I2cBusSolved { .. }));
    let run = archived(&r);
    assert!(matches!(
        attribute(&run),
        Err(aethelgard_evolve::AttributeError::NotACampaignRun(_))
    ));
}

// ---- The adversarial pass (A1 to A15) and the mutation run ----

fn insert_after_last(r: &mut Vec<Record>, pred: impl Fn(&Record) -> bool, record: Record) {
    let i = r.iter().rposition(pred).unwrap();
    r.insert(i + 1, record);
}

/// A second bus, AUX on I2C2 (pins PF1 and PF0), sorted before SENSORS, with
/// U5 at 0x40; computed healthy when `measured`.
fn add_aux(r: &mut Vec<Record>, measured: bool) {
    let at = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        solved(
            "AUX",
            "I2C2",
            ["A_SCL", "A_SDA"],
            ["PF1", "PF0"],
            &[("U5", 0x40)],
        ),
    );
    if measured {
        let i = find(r, |x| matches!(x, Record::RiseComputed { .. }));
        r.insert(i, rise("AUX", "A_SCL", 186_402));
        r.insert(i + 1, rise("AUX", "A_SDA", 186_402));
    }
}

/// U5's identity read and its witness: `observed` is what the model logged.
fn u5_read(r: &mut Vec<Record>, status: &str, observed: &[&str]) {
    insert_after_last(
        r,
        is_check,
        Record::I2cCheck {
            reference: "U5".into(),
            address: 0x40,
            step: "identity".into(),
            write: vec![0xD0],
            width: 1,
            expected: 0x60,
            mask: 0xFF,
            observed: Some(0),
            status: Some(status.into()),
        },
    );
    insert_after_last(
        r,
        is_witness,
        witness_with("I2C2", "U5", &OK, Some(observed)),
    );
}

fn set_register(r: &mut [Record], name: &str, f: impl FnOnce(&mut Reading)) {
    let i = find(
        r,
        |x| matches!(x, Record::RegisterObserved { name: n, .. } if n == name),
    );
    let Record::RegisterObserved { reading, .. } = &mut r[i] else {
        unreachable!()
    };
    f(reading);
}

#[test]
fn a1_a_verdict_narrowed_or_passed_beside_a_failing_event_is_refused() {
    // The wrong-address run, its verdict rewritten to pass: the TXIS timeout
    // is still in the archive, so replay refuses it by that kind, and
    // attribute with it, never `none` (`AET-WO-2312`; before, `I1`).
    let mut r = healthy();
    wrong_register(&mut r);
    replace(&mut r, is_verdict, verdict(&[]));
    assert_eq!(refused(&r), refusal("i2c_timeout"));
    // The timeout moved to `info`: the same.
    let mut r = healthy();
    wrong_register(&mut r);
    replace(
        &mut r,
        is_verdict,
        Record::Verdict {
            pass: true,
            findings: vec![("i2c_timeout".into(), "info".into(), 1)],
        },
    );
    assert_eq!(refused(&r), refusal("i2c_timeout"));
    // Two buses: one faulted, the other's wrong address dropped from the
    // verdict. Without the converse rule this was a confident E1.
    let mut r = healthy();
    add_aux(&mut r, true);
    u5_read(&mut r, "timeout_rxne", &["Write D1"]);
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2)]),
    );
    assert_eq!(refused(&r), refusal("i2c_timeout"));
}

#[test]
fn a3_a_bus_the_header_cannot_join_is_never_healthy_and_refuses_nothing_else() {
    // An extra bus on a controller and a port the ST header lacks.
    let unjoinable = |r: &mut Vec<Record>| {
        let at = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
        r.insert(
            at + 1,
            solved("ZZZ", "I2C9", ["Z_SCL", "Z_SDA"], ["PZ0", "PZ1"], &[]),
        );
        let i = find(r, |x| matches!(x, Record::RiseComputed { .. }));
        r.insert(i + 2, rise("ZZZ", "Z_SCL", 186_402));
        r.insert(i + 3, rise("ZZZ", "Z_SDA", 186_402));
    };
    // The symptom's own bus is joined: still software.
    let mut r = healthy();
    wrong_register(&mut r);
    unjoinable(&mut r);
    assert_eq!(attributed(&r).rule, Rule::S1);
    // A symptom on the board needs every bus computed healthy: I2.
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    unjoinable(&mut r);
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I2, vec![15]));
}

#[test]
fn a4_one_devices_witness_never_explains_another_devices_timeout() {
    // U2 times out with nothing logged; U3, on the same bus, completed a
    // read of the wrong register, and only U3's witness shows it.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    insert_after_last(
        &mut r,
        is_check,
        Record::I2cCheck {
            reference: "U3".into(),
            address: 0x48,
            step: "identity".into(),
            write: vec![0x0F],
            width: 2,
            expected: 0x117,
            mask: 0xFFFF,
            observed: Some(0x00),
            status: Some("ok".into()),
        },
    );
    replace(&mut r, is_witness, witness(Some(&[])));
    insert_after_last(
        &mut r,
        is_witness,
        witness_with(
            "I2C1",
            "U3",
            &["Write F", "Read 1 17"],
            Some(&["Write E", "Read 0"]),
        ),
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("i2c_identity_mismatch", 1),
            ("i2c_timeout", 1),
            ("i2c_transaction_unwitnessed", 1),
        ]),
    );
    let a = attributed(&r);
    // U3's symptoms are software; U2's has no evidence of its own.
    assert_eq!(
        (a.domain, a.rule),
        (Domain::Indeterminate, Rule::I4),
        "{a:?}"
    );
}

#[test]
fn a5_a_witness_whose_lines_are_not_its_devices_is_no_evidence() {
    // The logged lines are another device's (`i2c1.u3`), not the witness's
    // own (`i2c1.u2`): nothing the judge can attribute to U2.
    let mut r = healthy();
    wrong_register(&mut r);
    let mut w = witness(Some(&["Write D1"]));
    if let Record::I2cWitness { observed, .. } = &mut w {
        *observed = Some(vec!["[NOISY] i2c1.u3: Write D1".into()]);
    }
    replace(&mut r, is_witness, w);
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a7_the_mode_a_pin_must_hold_is_kind_15s_not_the_readings() {
    // The reading's expectation forged to "output" while the pin is in its
    // alternate-function mode: no evidence either way.
    let mut r = healthy();
    set_register(&mut r, "GPIOB.MODER", |g| g.expected = 0x0000_9000);
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0x0000_A000));
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    // A genuine wrong mode, the high bit of the field (input, 0b00): S1.
    let mut r = healthy();
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0x0000_8000));
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S1, vec![7, 7, 11]));
}

/// A10: one event never explains a finding counted at two failing
/// severities. Since `AET-WO-3201` B3 a verdict has one failing severity,
/// `error`, so the second count cannot be archived at all.
#[test]
fn a10_one_event_never_explains_a_finding_counted_at_two_severities() {
    let two = Record::Verdict {
        pass: false,
        findings: vec![
            ("i2c_timeout".into(), "error".into(), 1),
            ("i2c_timeout".into(), "warning".into(), 1),
        ],
    };
    assert_eq!(
        two.check().unwrap_err(),
        "a finding is not a kind, a severity of error or info, and a count above 0"
    );
}

#[test]
fn a15_the_evidence_is_the_named_rules_alone() {
    // A wrong address on the bus (S1) and no boot word (S2): the run is S1,
    // and its evidence holds no kind 10.
    let mut r = healthy();
    wrong_register(&mut r);
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[("boot_word_missing", 1), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S1, vec![7, 7, 13]));
    assert_eq!(kinds(&a.symptoms), [10, 12]);
}

/// The run whose read completed (status ok, value 0x60) with `observed` logged
/// and the verdict naming only the unwitnessed read.
fn completed(observed: Option<&[&str]>) -> Rule {
    let mut r = healthy();
    replace(&mut r, is_witness, witness(observed));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    attributed(&r).rule
}

#[test]
fn every_witness_line_is_evidence_within_its_width() {
    // The register written, after the read completed with the right value.
    assert_eq!(completed(Some(&["Write D1", "Read 60"])), Rule::S1);
    // The value read.
    assert_eq!(completed(Some(&["Write D0", "Read 61"])), Rule::S1);
    // A read the record says completed and the model never logged.
    assert_eq!(completed(Some(&[])), Rule::S1);
    // A byte wider than a byte is no line the judge can read: no evidence.
    assert_eq!(completed(Some(&["Write D0", "Read 160"])), Rule::I4);
    assert_eq!(completed(Some(&["Write D01", "Read 60"])), Rule::I4);
    // A log that could not be read whole is no evidence.
    assert_eq!(completed(None), Rule::I4);
    // A witness whose own expectation is not the read's register is no evidence.
    let mut r = healthy();
    let w = witness_with("I2C1", "U2", &["Write D1", "Read 60"], Some(&["Write D2"]));
    replace(&mut r, is_witness, w);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_wrong_line_counts_on_any_timed_out_read() {
    // A TC timeout after the model logged a write of another register.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_tc", 0));
    replace(&mut r, is_witness, witness(Some(&["Write D1"])));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::S1);
    // A STOPF timeout after the model returned another value.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_stopf", 0));
    replace(&mut r, is_witness, witness(Some(&["Write D0", "Read 61"])));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::S1);
}

#[test]
fn an_alternate_function_wrong_in_its_high_bit_or_on_afrh_is_software() {
    let mut r = healthy();
    set_register(&mut r, "GPIOB.AFRL", |g| g.observed = Some(0x4C00_0000));
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    assert_eq!(attributed(&r).rule, Rule::S1);
    // A bus on PB8 and PB9, whose fields are in AFRH.
    let mut r = healthy();
    let at = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r[at] = solved(
        "SENSORS",
        "I2C1",
        ["I2C_SCL", "I2C_SDA"],
        ["PB8", "PB9"],
        &[("U2", 0x76)],
    );
    let afrh = Record::RegisterObserved {
        name: "GPIOB.AFRH".into(),
        reading: reading(0x5802_0424, 0x0000_00FF, 0x0000_0044, 0x0000_0045),
    };
    insert_after_last(
        &mut r,
        |x| matches!(x, Record::RegisterObserved { .. }),
        afrh,
    );
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S1, vec![7, 7, 11]));
}

#[test]
fn a_field_the_reading_covers_only_in_part_is_no_evidence() {
    let mut r = healthy();
    set_register(&mut r, "GPIOB.AFRL", |g| {
        g.mask = 0x0700_0000;
        g.expected = 0x0400_0000;
        g.observed = Some(0x0500_0000);
    });
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_ports_clock_gate_left_off_is_software_and_explains_the_locked_access() {
    let mut r = healthy();
    set_register(&mut r, "RCC.AHB4ENR", |g| g.observed = Some(0x1));
    // The judge names the locked accesses once, however many there were
    // (`AET-WO-2312`): the fixture's old count, three, is refused.
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 3), ("register_mismatch", 1)]),
    );
    assert_eq!(
        refused(&r),
        "the verdict names locked_peripheral_access more than once"
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 1), ("register_mismatch", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S1, vec![7, 7, 11]));
}

#[test]
fn a_locked_access_beside_only_a_gpio_mismatch_is_unexplained() {
    let mut r = healthy();
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0x0000_8000));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 1), ("register_mismatch", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I1);
}

#[test]
fn a_gate_two_buses_own_is_on_the_board_and_owned_so_indeterminate() {
    // AUX moves onto port B (PB10, PB11): both buses own GPIOB's gate.
    let mut r = healthy();
    let at = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        solved("AUX", "I2C2", ["A_SCL", "A_SDA"], ["PB10", "PB11"], &[]),
    );
    let i = find(&r, |x| matches!(x, Record::RiseComputed { .. }));
    r.insert(i, rise("AUX", "A_SCL", 186_402));
    r.insert(i + 1, rise("AUX", "A_SDA", 186_402));
    set_register(&mut r, "RCC.AHB4ENR", |g| g.observed = Some(0x1));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 1), ("register_mismatch", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I4, vec![7, 7, 7, 7]));
}

#[test]
fn a_register_wrong_inside_and_outside_a_buss_fields_is_on_the_board() {
    let mut r = healthy();
    set_register(&mut r, "RCC.AHB4ENR", |g| g.observed = Some(0x2));
    set_register(&mut r, "RCC.APB1LENR", |g| g.observed = Some(0x0000_0001));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 1), ("register_mismatch", 2)]),
    );
    // RCC.AHB4ENR: GPIOA's gate off (no bus owns it); APB1LENR: I2C1's gate
    // off, plus a bit no one owns. Both on the board: S2.
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S2, vec![7, 7, 11, 11]));
}

#[test]
fn a_line_stuck_high_and_an_incomplete_rise_are_electrical() {
    let mut r = healthy();
    r.retain(|x| !matches!(x, Record::RiseComputed { net, .. } if net == "I2C_SDA"));
    let i = find(&r, |x| matches!(x, Record::RiseComputed { .. }));
    r.insert(
        i + 1,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: 1_500_000,
            ceiling_ps: 300_000,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
    );
    r.insert(i + 2, fault("SENSORS", &["I2C_SDA"]));
    if let Record::BusFault { from, .. } = &mut r[i + 2] {
        *from = vec!["analog.line_stuck_high".into()];
    }
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_line_stuck_high", 1), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::E1, vec![8, 9]));

    let mut r = healthy();
    let i = find(&r, |x| matches!(x, Record::RiseComputed { .. }));
    r[i] = Record::RiseComputed {
        bus: "SENSORS".into(),
        net: "I2C_SCL".into(),
        rise_30_70_ps: 200_000,
        complete: false,
        ceiling_ps: 300_000,
        exceeds: true,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    };
    r.insert(i + 2, fault("SENSORS", &["I2C_SCL"]));
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 1), ("i2c_timeout", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::E1);
}

#[test]
fn each_finding_selects_only_the_events_that_show_it() {
    // An invalid record, as invalid: no evidence, I4. Claimed as a timeout,
    // no record shows one: refused by that kind (`AET-WO-2312`; before, I1).
    let mut r = healthy();
    replace(&mut r, is_check, check("invalid", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(refused(&r), refusal("i2c_timeout"));
    // An identity mismatch or an unwitnessed read claimed of a timed-out
    // read: the timeout is unnamed, and refused by its kind.
    let mut r = healthy();
    wrong_register(&mut r);
    for kind in ["i2c_identity_mismatch", "i2c_transaction_unwitnessed"] {
        replace(&mut r, is_verdict, verdict(&[(kind, 1)]));
        assert_eq!(refused(&r), refusal("i2c_timeout"), "{kind}");
    }
    // A boot word claimed missing that is there is refused by `replay`
    // (`AET-WO-3201` B2); so, on a booted run, is a halt claimed of a full
    // one (`AET-WO-2310`).
    let mut r = healthy();
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    assert_eq!(refused(&r), BOOT_WORD);
    replace(&mut r, is_verdict, verdict(&[("cpu_halted", 1)]));
    assert_eq!(refused(&r), HALTED);
    // A real halt is board software.
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0),
            instructions_expected: 10_000_000,
            instructions_observed: Some(9_999_999),
            pc: Some(0x0800_0596),
            executable: vec![(0, 0x1000_0000)],
            brown_out: aethelgard_chaos::record::Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[("boot_word_missing", 1), ("cpu_halted", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::S2);
    // Two register mismatches against a count of one.
    let mut r = healthy();
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0x0000_8000));
    set_register(&mut r, "GPIOB.AFRL", |g| g.observed = Some(0x4500_0000));
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    assert_eq!(refused(&r), refusal("register_mismatch"));
}

#[test]
fn a_board_symptom_names_its_rule_and_evidence() {
    // Faulted: I3 with the faults. Every bus healthy: S2 with every rise.
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    add_aux(&mut r, true);
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    let a = attributed(&r);
    assert_eq!(
        (a.rule, kinds(&a.evidence)),
        (Rule::S2, vec![7, 7, 7, 7, 10])
    );
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("boot_word_missing", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I3, vec![7, 7, 9]));
    // No bus at all: I2, with nothing to name.
    let mut r = healthy();
    r.retain(|x| {
        !matches!(
            x,
            Record::I2cBusSolved { .. }
                | Record::RiseComputed { .. }
                | Record::I2cCheck { .. }
                | Record::I2cWitness { .. }
        )
    });
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    let a = attributed(&r);
    assert_eq!((a.rule, a.evidence.len()), (Rule::I2, 0));
}

#[test]
fn the_fault_on_the_second_bus_is_elsewhere_for_the_first() {
    // AUX (index 0) healthy, U5 times out with the right address; SENSORS
    // (index 1) faulted.
    let mut r = healthy();
    add_aux(&mut r, true);
    u5_read(&mut r, "timeout_txis", &[]);
    faulted_sensors(&mut r);
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { reference, .. } if reference == "U2"),
        witness(Some(&[])),
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 2)]),
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I3, "{a:?}");
}

#[test]
fn the_lowest_indeterminate_rule_names_the_run_with_only_its_evidence() {
    // AUX on a port the header lacks (I2 for U5; unmeasured, it is refused
    // since `AET-WO-3201` B1); SENSORS healthy with U2's right-address
    // timeout (I4): the run is I2, with AUX's kind 15 alone.
    let mut r = healthy();
    let at = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        solved(
            "AUX",
            "I2C2",
            ["A_SCL", "A_SDA"],
            ["PZ0", "PZ1"],
            &[("U5", 0x40)],
        ),
    );
    let i = find(&r, |x| matches!(x, Record::RiseComputed { .. }));
    r.insert(i, rise("AUX", "A_SCL", 186_402));
    r.insert(i + 1, rise("AUX", "A_SDA", 186_402));
    u5_read(&mut r, "timeout_txis", &[]);
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { reference, .. } if reference == "U2"),
        witness(Some(&[])),
    );
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 2)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I2, vec![15]));
}

#[test]
fn i5_lists_both_halves_and_the_masked_witness() {
    // SENSORS faulted with U2's wrong address (masked); AUX healthy with U5's.
    let mut r = healthy();
    wrong_register(&mut r);
    add_aux(&mut r, true);
    u5_read(&mut r, "timeout_rxne", &["Write D1"]);
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 2)]),
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I5, "{a:?}");
    assert_eq!(kinds(&a.masked), [13]);
    assert!(kinds(&a.evidence).contains(&9) && kinds(&a.evidence).contains(&13));
}

#[test]
fn an_unexplained_finding_names_no_evidence_beside_a_decided_symptom() {
    let mut r = healthy();
    wrong_register(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_timeout", 1), ("renode_error_log", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, a.evidence.len()), (Rule::I1, 0));
}

#[test]
fn every_kind_of_failing_event_the_verdict_omits_is_refused_or_unexplained() {
    // Each event shows a failure, and the verdict passes. Since `AET-WO-2312`
    // replay refuses each kind its records bound, by that kind, and attribute
    // with it. A witness that differs without its read's witness being
    // unread binds no count (the archive cuts a long log), so it is left to
    // attribute, unexplained.
    type Edit = Box<dyn Fn(&mut Vec<Record>)>;
    let cases: Vec<(&str, Edit, Option<&str>)> = vec![
        (
            "invalid",
            Box::new(|r| replace(r, is_check, check("invalid", 0))),
            Some("i2c_record_invalid"),
        ),
        (
            "identity",
            Box::new(|r| replace(r, is_check, check("ok", 0x61))),
            Some("i2c_identity_mismatch"),
        ),
        // A short, whole witness that differs: the judge names it (or a
        // cut `[ERROR]` line's `step_error`), so it is refused jointly
        // (`AET-WO-2312` adversarial F2).
        (
            "unwitnessed",
            Box::new(|r| replace(r, is_witness, witness(Some(&["Write D1", "Read 60"])))),
            Some("i2c witness"),
        ),
        (
            "register",
            Box::new(|r| set_register(r, "GPIOA.MODER", |g| g.observed = Some(0))),
            Some("register_mismatch"),
        ),
        // A boot word missing with no finding is refused by `replay`
        // since `AET-WO-3201` B2 (`a_boot_word_missing_unnamed_is_refused`),
        // and a booted halt with none since `AET-WO-2310` (below). A clock
        // enable left off is a register's mismatch first.
        (
            "locked",
            Box::new(|r| set_register(r, "RCC.AHB4ENR", |g| g.observed = Some(0x1))),
            Some("register_mismatch"),
        ),
    ];
    for (name, edit, kind) in cases {
        let mut r = healthy();
        edit(&mut r);
        if let Some("i2c witness") = kind {
            assert_eq!(refused(&r), I2C_WITNESSES, "{name}");
            continue;
        }
        if let Some(kind) = kind {
            assert_eq!(refused(&r), refusal(kind), "{name}");
            continue;
        }
        let a = attributed(&r);
        assert_eq!(a.rule, Rule::I1, "{name}: {a:?}");
        assert!(
            a.unexplained.iter().all(|u| u.starts_with("event ")),
            "{name}: {a:?}"
        );
    }
    // A booted core that counted fewer instructions, with no `cpu_halted`,
    // is refused by `replay` (`AET-WO-2310`), not left unexplained.
    let mut r = healthy();
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0x2100_B007),
            instructions_expected: 10_000_000,
            instructions_observed: Some(9_999_999),
            pc: Some(0x0800_0596),
            executable: vec![(0, 0x1000_0000)],
            brown_out: aethelgard_chaos::record::Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
    );
    assert_eq!(refused(&r), HALTED);
    // Two such events of one kind: refused by it.
    let mut r = healthy();
    set_register(&mut r, "GPIOA.MODER", |g| g.observed = Some(0));
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0));
    assert_eq!(refused(&r), refusal("register_mismatch"));
}

#[test]
fn a_timeout_claimed_as_an_invalid_record_is_refused() {
    let mut r = healthy();
    wrong_register(&mut r);
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 1)]));
    assert_eq!(refused(&r), refusal("i2c_timeout"));
}

#[test]
fn each_part_of_a_sound_witness_is_required() {
    type Edit<'a> = &'a dyn Fn(&mut Vec<String>, &mut Option<Vec<String>>);
    let unwitnessed = |edit: Edit| {
        let mut r = healthy();
        let mut w = witness(Some(&["Write D1", "Read 60"]));
        if let Record::I2cWitness {
            expected, observed, ..
        } = &mut w
        {
            edit(expected, observed);
        }
        replace(&mut r, is_witness, w);
        replace(
            &mut r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
        attributed(&r).rule
    };
    assert_eq!(unwitnessed(&|_, _| {}), Rule::S1);
    // An expectation with no write of the check's register, or no read of its
    // value, or the read of another value.
    assert_eq!(unwitnessed(&|e, _| e.remove(0).clear()), Rule::I4);
    assert_eq!(unwitnessed(&|e, _| e.truncate(1)), Rule::I4);
    assert_eq!(
        unwitnessed(&|e, _| e[1] = "[NOISY] i2c1.u2: Read 61".into()),
        Rule::I4
    );
    // An expectation of another device's lines.
    assert_eq!(
        unwitnessed(&|e, _| e.iter_mut().for_each(|l| *l = l.replace("u2", "u3"))),
        Rule::I4
    );
    // A log that could not be read.
    assert_eq!(unwitnessed(&|_, o| *o = None), Rule::I4);
}

/// `AET-WO-2309`: replay refuses a register unread beside a read boot, so a
/// register holds nothing observed only in a boot that read nothing. The
/// shape is the wrong-register run killed before its reads, as the judge
/// archives it: none of its registers is evidence.
#[test]
fn a_register_with_nothing_observed_is_no_evidence() {
    let mut r = healthy();
    wrong_register(&mut r);
    for x in &mut r {
        match x {
            Record::BootObserved {
                boot_word,
                instructions_observed,
                pc,
                brown_out,
                flash_access,
                ..
            } => {
                boot_word.observed = None;
                *instructions_observed = None;
                *pc = None;
                brown_out.observed = None;
                *flash_access = None;
            }
            Record::RegisterObserved { reading, .. } => reading.observed = None,
            Record::I2cCheck {
                observed, status, ..
            } => {
                *observed = None;
                *status = None;
            }
            Record::I2cWitness {
                observed, absent, ..
            } => {
                *observed = None;
                absent.clear();
            }
            _ => {}
        }
    }
    replace(&mut r, is_verdict, verdict(&[("cpu_aborted", 1)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I1, vec![]), "{a:?}");
}

#[test]
fn a_register_wrong_on_the_second_bus_is_that_buss_alone() {
    let mut r = healthy();
    add_aux(&mut r, true);
    set_register(&mut r, "GPIOB.AFRL", |g| g.observed = Some(0x4500_0000));
    replace(&mut r, is_verdict, verdict(&[("register_mismatch", 1)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::S1, vec![7, 7, 11]));
}

/// `AET-WO-2205` §1.6: the seeded set reads each rule's evidence kinds from
/// here by the rule's id, so the id round-trips for every rule and no other.
#[test]
fn every_rule_is_found_by_its_id_and_nothing_else_is() {
    for r in Rule::ALL {
        assert_eq!(Rule::from_id(r.id()), Some(r), "{} round-trips", r.id());
    }
    let mut ids: Vec<&str> = Rule::ALL.iter().map(|r| r.id()).collect();
    ids.dedup();
    // Twenty-five since `AET-WO-2302b` added `S6` to `S11` and `H1` to `H4`
    // and removed `I7` (sixteen since `AET-WO-2302` added `E3`, fifteen
    // since `AET-WO-3201` added `I7`, fourteen since `AET-WO-4300b` added
    // `S5` and `I6`, twelve since `AET-WO-4300a` added `S4`, eleven since
    // `AET-WO-4200` added `E2` and `S3`).
    assert_eq!(ids.len(), 25, "twenty-five rules, twenty-five distinct ids");
    for other in ["", "e1", "E4", "S12", "I7", "I0", "H0", "H5", "N1 ", "h1"] {
        assert_eq!(Rule::from_id(other), None, "`{other}` names no rule");
    }
}

/// `AET-WO-2205` adversarial pass 2 (M3): software evidence on an
/// electrically faulted bus is listed in `masked` whatever rule names the
/// run. A wrong-register witness on the faulted SENSORS (E1) beside a boot
/// with no boot word (I3, the fault is elsewhere) is I3, the witness masked.
#[test]
fn a_witness_on_a_faulted_bus_stays_masked_when_the_run_is_indeterminate() {
    let mut r = healthy();
    wrong_register(&mut r);
    faulted_sensors(&mut r);
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("analog_rise_exceeds_ceiling", 2),
            ("boot_word_missing", 1),
            ("i2c_timeout", 1),
        ]),
    );
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I3));
    assert_eq!(
        kinds(&a.masked),
        [13],
        "the wrong witness on the faulted bus is masked"
    );
}

// ---- AET-WO-4210: one check and one witness per step of a device's script ----

/// U2's `data` step (register 0xF7, two bytes, 0x1234) as a check and its
/// witness, appended after the identity read's.
fn with_data_step(r: &mut Vec<Record>, observed_lines: &[&str]) {
    insert_after_last(
        r,
        is_check,
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x76,
            step: "data".into(),
            write: vec![0xF7],
            width: 2,
            expected: 0x1234,
            mask: 0xFFFF,
            observed: Some(0x1234),
            status: Some("ok".into()),
        },
    );
    let mut w = witness_with(
        "I2C1",
        "U2",
        &["Write F7", "Read 12 34"],
        Some(observed_lines),
    );
    if let Record::I2cWitness { step, .. } = &mut w {
        *step = "data".into();
    }
    insert_after_last(r, is_witness, w);
}

#[test]
fn a_data_steps_witness_that_differs_is_that_steps_event() {
    let mut r = healthy();
    with_data_step(&mut r, &["Write F7", "Read 12 35"]);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    let data_witness = r
        .iter()
        .rposition(|x| matches!(x, Record::I2cWitness { step, .. } if step == "data"))
        .unwrap() as u64;
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S1);
    assert_eq!(
        a.symptoms.iter().map(|e| e.index).collect::<Vec<_>>(),
        [data_witness]
    );
    // The same data step, read as solved, is no symptom of that finding.
    let mut r = healthy();
    with_data_step(&mut r, &["Write F7", "Read 12 34"]);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I1);
}

#[test]
fn a_masked_identity_check_counts_as_passing() {
    let masked = |observed: u64| {
        let mut r = healthy();
        replace(
            &mut r,
            is_check,
            Record::I2cCheck {
                reference: "U2".into(),
                address: 0x76,
                step: "identity".into(),
                write: vec![0xD0],
                width: 2,
                expected: 0x0008,
                mask: 0x3F,
                observed: Some(observed),
                status: Some("ok".into()),
            },
        );
        replace(
            &mut r,
            is_witness,
            witness_with(
                "I2C1",
                "U2",
                &["Write D0", "Read 00 08"],
                Some(&["Write D0", "Read 00 08"]),
            ),
        );
        replace(&mut r, is_verdict, verdict(&[("i2c_identity_mismatch", 1)]));
        r
    };
    // 0xFFC8 & 0x3F == 0x08: the claimed mismatch has no event to show it,
    // so replay refuses it by its kind (`AET-WO-2312`; before, I1).
    assert_eq!(refused(&masked(0xFFC8)), refusal("i2c_identity_mismatch"));
    // 0xFF00 & 0x3F == 0: a real mismatch.
    assert_ne!(attributed(&masked(0xFF00)).rule, Rule::I1);
}

/// `AET-WO-4230` D10: kind 10's brown-out word not the declared level, or
/// flash controller accesses the bring-up never makes, are the firmware's
/// (`S2`) on healthy buses, as the campaign's other boot failures are. A
/// verdict that does not name them is refused by replay before attribution
/// (`aethelgard-chaos` `check_run`).
#[test]
fn a_brown_out_word_or_a_flash_access_is_software_on_healthy_buses() {
    let edit = |word: u32, flash: Option<Vec<String>>| {
        let mut r = healthy();
        for x in &mut r {
            if let Record::BootObserved {
                brown_out,
                flash_access,
                ..
            } = x
            {
                brown_out.expected = 0xC;
                brown_out.observed = Some(word);
                *flash_access = flash.clone();
            }
        }
        r
    };
    for (word, flash, kind) in [
        (
            0x0406_AAF0,
            Some(Vec::new()),
            "brown_out_level_not_programmed",
        ),
        (
            0xFFFF_FFFF,
            Some(Vec::new()),
            "brown_out_level_not_programmed",
        ),
        (
            0x0406_AAFC,
            Some(vec!["the firmware wrote the flash controller".to_owned()]),
            "flash_controller_access",
        ),
        (0x0406_AAFC, None, "flash_controller_access"),
    ] {
        let mut r = edit(word, flash.clone());
        replace(&mut r, is_verdict, verdict(&[(kind, 1)]));
        let a = attributed(&r);
        assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S2), "{kind}");
        assert_eq!(kinds(&a.symptoms), [10], "{kind}");
    }
    // The declared level read back, every access the one read: a pass.
    let r = edit(0x0406_AAFC, Some(Vec::new()));
    assert_eq!(attributed(&r).rule, Rule::N1);
}

/// `AET-WO-3306` (F-COLLIDE): the reference run with a second BME280, U5,
/// at 0x77 on SENSORS, its identity read as solved; then `edit` applied.
/// Each of `others` is one more BME280 on the bus, at its address, read as
/// solved. U2's check is `ok`: the device that answered gave it the part's
/// value, as a second BME280 does.
fn two_bme(others: &[(&str, u8)], edit: impl Fn(&mut Vec<Record>)) -> Vec<Record> {
    let mut r = healthy();
    let mut devices = vec![("U2", 0x76), ("U3", 0x48), ("U5", 0x77)];
    devices.extend_from_slice(others);
    replace(
        &mut r,
        |x| matches!(x, Record::I2cBusSolved { .. }),
        solved(
            "SENSORS",
            "I2C1",
            ["I2C_SCL", "I2C_SDA"],
            ["PB6", "PB7"],
            &devices,
        ),
    );
    for &(reference, address) in [("U5", 0x77)].iter().chain(others) {
        let mut c = check("ok", 0x60);
        if let Record::I2cCheck {
            reference: x,
            address: a,
            ..
        } = &mut c
        {
            *x = reference.into();
            *a = address;
        }
        insert_after_last(&mut r, is_check, c);
        insert_after_last(
            &mut r,
            is_witness,
            witness_with("I2C1", reference, &OK, Some(&OK)),
        );
    }
    edit(&mut r);
    r
}

/// Set `reference`'s witness's observed lines.
fn observe(r: &mut [Record], reference: &str, lines: &[&str]) {
    let i = find(
        r,
        |x| matches!(x, Record::I2cWitness { reference: w, .. } if w == reference),
    );
    let with = witness_with("I2C1", reference, &OK, Some(lines));
    r[i] = with;
}

/// The joint witness refusal (`AET-WO-2312` adversarial F1-F3).
const I2C_WITNESSES: &str =
    "the verdict's i2c witness findings do not reach the I2C witnesses that show one";

/// `records` with its verdict naming each witness that differs from the
/// model's, as the judge names it (`campaign::shown`'s unwitnessed records),
/// when the verdict passes: the view tests read a witness's fields, and
/// replay refuses a differing witness left unnamed (`AET-WO-2312` F2).
fn named_witnesses(records: &[Record]) -> Vec<Record> {
    let mut r = records.to_vec();
    let n = campaign::shown(&r)
        .into_iter()
        .find(|s| s.kind == "i2c_transaction_unwitnessed")
        .map_or(0, |s| s.records.len() as u64);
    if n > 0 && matches!(r.last(), Some(Record::Verdict { pass: true, .. })) {
        replace(
            &mut r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", n)]),
        );
    }
    r
}

/// The fields `attribute`'s view gives witness `index`.
fn fields_of(records: &[Record], index: usize) -> Vec<(String, u32, u64, Option<u64>, bool)> {
    let run = archived(&named_witnesses(records));
    let e = aethelgard_evolve::EventRef {
        index: index as u64,
        kind: 13,
    };
    let aethelgard_evolve::View::Fields { fields, .. } = aethelgard_evolve::view(&run)
        .unwrap()
        .event(e)
        .unwrap()
        .view
    else {
        panic!("a witness's view is its fields")
    };
    fields
        .into_iter()
        .map(|f| (f.register, f.address, f.solved, f.observed, f.wrong))
        .collect()
}

fn witness_index(r: &[Record], reference: &str) -> usize {
    find(
        r,
        |x| matches!(x, Record::I2cWitness { reference: w, .. } if w == reference),
    )
}

/// F-COLLIDE, as `AET-WO-3305` §5 measured it: U2's identity read sent to
/// 0x77, where U5 answers. U2 logs nothing, and U5's log holds U2's write and
/// read beside its own. U5's whole log is judged against its script, so its
/// witness is the software's evidence, naming the transaction count at 0x77
/// and U2's step as the address the firmware sent it to. Read only by its
/// first write and read, U5's witness was its own read, and the run was I4.
#[test]
fn a_step_sent_to_another_devices_address_that_answers_is_software() {
    let r = two_bme(&[], |r| {
        observe(r, "U2", &[]);
        observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 2)]),
        );
    });
    let (u2, u5) = (witness_index(&r, "U2"), witness_index(&r, "U5"));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1), "{a:?}");
    let evidence: Vec<u64> = a.evidence.iter().map(|e| e.index).collect();
    assert!(
        evidence.contains(&(u5 as u64)),
        "U5's witness is evidence: {a:?}"
    );
    assert!(evidence.contains(&(u2 as u64)), "{a:?}");
    assert!(a.masked.is_empty(), "{a:?}");
    let wrong: Vec<_> = fields_of(&r, u5).into_iter().filter(|f| f.4).collect();
    assert_eq!(
        wrong,
        [
            ("i2c1.u5.transactions".to_owned(), 0x77, 1, Some(2), true),
            (
                "i2c1.u2.identity.address".to_owned(),
                0x76,
                0x76,
                Some(0x77),
                true
            ),
        ]
    );
}

/// The surplus alone, on a healthy bus where every other device read as
/// solved: no other step has the extra transaction's shape and lacks it, so
/// whose step it was is not named, only the address (§7). It is software
/// all the same, by U5's witness alone.
#[test]
fn a_transaction_no_scripted_step_lacks_is_software_by_its_address_alone() {
    let r = two_bme(&[], |r| {
        observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
    });
    let u5 = witness_index(&r, "U5");
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::S1, "{a:?}");
    assert_eq!(
        a.evidence
            .iter()
            .filter(|e| e.kind == 13)
            .map(|e| e.index)
            .collect::<Vec<_>>(),
        [u5 as u64]
    );
    let wrong: Vec<_> = fields_of(&r, u5).into_iter().filter(|f| f.4).collect();
    assert_eq!(
        wrong,
        [("i2c1.u5.transactions".to_owned(), 0x77, 1, Some(2), true)]
    );
}

/// §5: two other devices share the extra transaction's shape. The one whose
/// own witness holds its read is not named; with both lacking theirs, the
/// step is not named, only the address.
#[test]
fn a_shared_step_shape_names_only_the_step_that_lacks_its_transaction() {
    let collide = |u6: &'static [&'static str]| {
        two_bme(&[("U6", 0x75)], move |r| {
            observe(r, "U2", &[]);
            observe(r, "U6", u6);
            observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
            let n = if u6.is_empty() { 3 } else { 2 };
            replace(
                r,
                is_verdict,
                verdict(&[("i2c_transaction_unwitnessed", n)]),
            );
        })
    };
    let r = collide(&OK);
    let named: Vec<String> = fields_of(&r, witness_index(&r, "U5"))
        .into_iter()
        .filter(|f| f.4 && f.0.ends_with(".address"))
        .map(|f| f.0)
        .collect();
    assert_eq!(named, ["i2c1.u2.identity.address"]);
    let r = collide(&[]);
    assert!(
        fields_of(&r, witness_index(&r, "U5"))
            .iter()
            .all(|f| !(f.4 && f.0.ends_with(".address"))),
        "two steps of that shape lack their transaction: neither is named"
    );
    assert_eq!(attributed(&r).rule, Rule::S1);
}

/// §5: a script that repeats a step holds it twice, so its log is whole and
/// nothing is extra. A read repeated inside one transaction (the same write,
/// two reads), or a transaction missing beside an extra one, is not a
/// surplus: each witness keeps today's judgement.
#[test]
fn a_repeat_the_script_holds_or_a_wrong_transaction_is_not_a_surplus() {
    // The script holds the read twice: a pass.
    let mut r = two_bme(&[], |_| {});
    for x in &mut r {
        if let Record::I2cWitness {
            reference,
            expected,
            observed,
            ..
        } = x
            && reference == "U5"
        {
            *expected = [OK, OK]
                .concat()
                .iter()
                .map(|l| format!("[NOISY] i2c1.u5: {l}"))
                .collect();
            *observed = Some(expected.clone());
        }
    }
    assert!(
        fields_of(&r, witness_index(&r, "U5")).iter().all(|f| !f.4),
        "a repeat the script holds is no evidence"
    );
    // One write, two reads: one transaction, not the script's. Its first
    // write and read are the solved ones, so it stays I4 as before.
    let r = two_bme(&[], |r| {
        observe(r, "U5", &["Write D0", "Read 60", "Read 60"]);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
    });
    assert_eq!(attributed(&r).rule, Rule::I4);
    // An extra write with the script's own missing: not the script's
    // transactions with one more. U5's first write is wrong: S1 as before,
    // with no transaction count.
    let r = two_bme(&[], |r| {
        observe(r, "U5", &["Write D1", "Read 60"]);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
    });
    assert_eq!(attributed(&r).rule, Rule::S1);
    assert!(
        fields_of(&r, witness_index(&r, "U5"))
            .iter()
            .all(|f| !f.0.ends_with(".transactions"))
    );
    // A warning line among the device's lines: not a transaction.
    let r = two_bme(&[], |r| {
        let i = witness_index(r, "U5");
        if let Record::I2cWitness { observed, .. } = &mut r[i] {
            let mut o = observed.clone().unwrap();
            o.extend([
                "[WARNING] i2c1.u5: Write D0".to_owned(),
                "[NOISY] i2c1.u5: Read 60".to_owned(),
            ]);
            *observed = Some(o);
        }
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
    });
    assert_eq!(attributed(&r).rule, Rule::I4);
}

/// One more device's identity step (register 0xD0, a read of `width` bytes)
/// on `bus`, whose record claims the part's value while its device logged
/// nothing: by its records alone, sent to some other device that answered.
fn lacking(r: &mut Vec<Record>, peripheral: &str, reference: &str, address: u8, width: u8) {
    let read = format!("Read 60{}", " 00".repeat(usize::from(width) - 1));
    let script = ["Write D0", read.as_str()];
    insert_after_last(
        r,
        is_check,
        Record::I2cCheck {
            reference: reference.into(),
            address,
            step: "identity".into(),
            write: vec![0xD0],
            width,
            expected: 0x60,
            mask: 0xFF,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
    );
    insert_after_last(
        r,
        is_witness,
        witness_with(peripheral, reference, &script, Some(&[])),
    );
}

/// §5, the shared step shape: the step named is one on the same bus whose
/// read has the extra transaction's width. A device on another bus, or with
/// a longer read of the same register, lacking its own transaction, is not
/// a second candidate.
#[test]
fn the_step_named_is_on_the_same_bus_with_the_extra_reads_width() {
    let named = |r: &[Record]| -> Vec<String> {
        fields_of(r, witness_index(r, "U5"))
            .into_iter()
            .filter(|f| f.4 && f.0.ends_with(".address"))
            .map(|f| f.0)
            .collect()
    };
    let collide = |r: &mut Vec<Record>| {
        observe(r, "U2", &[]);
        observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
    };
    // U6 on SENSORS reads two bytes of 0xD0 and logged nothing.
    let r = two_bme(&[], |r| {
        collide(r);
        lacking(r, "I2C1", "U6", 0x75, 2);
        let i = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
        r[i] = solved(
            "SENSORS",
            "I2C1",
            ["I2C_SCL", "I2C_SDA"],
            ["PB6", "PB7"],
            &[("U2", 0x76), ("U3", 0x48), ("U5", 0x77), ("U6", 0x75)],
        );
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 3)]),
        );
    });
    assert_eq!(named(&r), ["i2c1.u2.identity.address"]);
    // U9, a BME280 at 0x76 on AUX, logged nothing for its identity read.
    let r = two_bme(&[], |r| {
        collide(r);
        let at = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
        r.insert(
            at,
            solved(
                "AUX",
                "I2C2",
                ["A_SCL", "A_SDA"],
                ["PF1", "PF0"],
                &[("U9", 0x76)],
            ),
        );
        let i = find(r, |x| matches!(x, Record::RiseComputed { .. }));
        r.insert(i, rise("AUX", "A_SCL", 186_402));
        r.insert(i + 1, rise("AUX", "A_SDA", 186_402));
        lacking(r, "I2C2", "U9", 0x76, 1);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 3)]),
        );
    });
    assert_eq!(named(&r), ["i2c1.u2.identity.address"]);
}

/// A device with two steps: each step's witness takes as many lines as it
/// expects and the last the rest, so U2's extra read, logged before U5's own
/// data read, lands in U5's data witness. The surplus is shown there, on the
/// witness its first line went to, and U5's identity witness, which holds
/// its own read, shows nothing.
#[test]
fn the_surplus_is_shown_on_the_witness_its_first_line_went_to() {
    let r = two_bme(&[], |r| {
        insert_after_last(
            r,
            is_check,
            Record::I2cCheck {
                reference: "U5".into(),
                address: 0x77,
                step: "data".into(),
                write: vec![0xF7],
                width: 2,
                expected: 0x1234,
                mask: 0xFFFF,
                observed: Some(0x1234),
                status: Some("ok".into()),
            },
        );
        let mut w = witness_with(
            "I2C1",
            "U5",
            &["Write F7", "Read 12 34"],
            Some(&["Write D0", "Read 60", "Write F7", "Read 12 34"]),
        );
        if let Record::I2cWitness { step, .. } = &mut w {
            *step = "data".into();
        }
        insert_after_last(r, is_witness, w);
        observe(r, "U2", &[]);
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 2)]),
        );
    });
    let identity = witness_index(&r, "U5");
    let data = r
        .iter()
        .rposition(|x| matches!(x, Record::I2cWitness { step, .. } if step == "data"))
        .unwrap();
    assert!(
        fields_of(&r, identity).iter().all(|f| !f.4),
        "U5's identity read is its own"
    );
    let shown: Vec<String> = fields_of(&r, data)
        .into_iter()
        .filter(|f| f.4)
        .map(|f| f.0)
        .collect();
    assert_eq!(
        shown,
        [
            "i2c1.u5.Write",
            "i2c1.u5.Read",
            "i2c1.u5.transactions",
            "i2c1.u2.identity.address"
        ]
    );
    assert_eq!(attributed(&r).rule, Rule::S1);
}

/// Two-bme with U5 a BMP581 at 0x47 (`spike/3306/spec/bme-bmp.json`), its
/// three steps' checks claiming `ok`: `u2` edits U2's records, and `u5` is
/// each of U5's witnesses' observed lines, whole. Its script is our model's
/// (`BMP581.cs` `FinishTransmission`).
fn bme_bmp(u2: impl Fn(&mut Vec<Record>), u5: [&[&str]; 3]) -> Vec<Record> {
    let mut r = two_bme(&[], |_| {});
    let i = find(&r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r[i] = solved(
        "SENSORS",
        "I2C1",
        ["I2C_SCL", "I2C_SDA"],
        ["PB6", "PB7"],
        &[("U2", 0x76), ("U3", 0x48), ("U5", 0x47)],
    );
    r.retain(|x| {
        !matches!(x, Record::I2cCheck { reference, .. } | Record::I2cWitness { reference, .. } if reference == "U5")
    });
    u2(&mut r);
    let line = |l: &str| format!("[NOISY] i2c1.u5: {l}");
    let [identity, configure, data] =
        u5.map(|lines| lines.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>());
    for (step, write, width, expected, mask, script, observed) in [
        (
            "identity",
            vec![0x01],
            1,
            0x50,
            0xFF,
            vec![line("read 0x01 -> 0x50")],
            identity,
        ),
        (
            "configure",
            vec![0x36, 0x40, 0x81],
            0,
            0,
            0,
            vec![line("write 0x36 <- 0x40 0x81")],
            configure,
        ),
        (
            "data",
            vec![0x1D],
            6,
            423_687_010,
            0xFFFF_FFFF_FFFF,
            vec![line("read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62")],
            data,
        ),
    ] {
        insert_after_last(
            &mut r,
            is_check,
            Record::I2cCheck {
                reference: "U5".into(),
                address: 0x47,
                step: step.into(),
                write,
                width,
                expected,
                mask,
                observed: Some(expected),
                status: Some("ok".into()),
            },
        );
        insert_after_last(
            &mut r,
            is_witness,
            Record::I2cWitness {
                peripheral: "I2C1".into(),
                reference: "U5".into(),
                device: "i2c1.u5".into(),
                step: step.into(),
                expected: script,
                observed: Some(observed),
                absent: Vec::new(),
            },
        );
    }
    r
}

/// `AET-WO-3306` F1, which `AET-WO-3307` answers: U2's identity read sent
/// to 0x47, where U5, a BMP581, answers. These are the records measured on
/// `spike/3306/spec/bme-bmp.json` (`spike/3306/out`; `spike/3307/out` on
/// `main` at `28fba951`). Our model logs a transaction as one line, `read
/// 0xRR -> 0xVV` or `write 0xRR <- …` (`BMP581.cs` `FinishTransmission`),
/// which `witness_fields` now reads. U5's configure witness holds U2's
/// transfer (`read 0x50 -> 0x00`: the model masks the pointer `0xD0` to 7
/// bits), not its own `write 0x36 <- 0x40 0x81`, so it is the software's
/// evidence beside U2's silent one, and the run is S1. Its identity witness
/// holds only a warning, which is no line it reads: sound, and nothing in it
/// wrong. Its data witness reads 6 bytes, wider than a witness judges.
#[test]
fn a_collide_onto_a_bmp581_is_software_by_its_witness() {
    let mut r = bme_bmp(
        |r| {
            replace(r, is_check, check("ok", 0));
            observe(r, "U2", &[]);
        },
        [
            &["[WARNING] i2c1.u5: access to unmodelled register 0x50"],
            &["[NOISY] i2c1.u5: read 0x50 -> 0x00"],
            &[
                "[NOISY] i2c1.u5: read 0x01 -> 0x50",
                "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81",
                "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62",
            ],
        ],
    );
    replace(
        &mut r,
        is_verdict,
        verdict(&[
            ("i2c_identity_mismatch", 1),
            ("i2c_transaction_unwitnessed", 4),
        ]),
    );
    let a = attributed(&r);
    let [identity, configure, data] = ["identity", "configure", "data"].map(|s| {
        find(
            &r,
            |x| matches!(x, Record::I2cWitness { reference, step, .. } if reference == "U5" && step == s),
        )
    });
    let u2 = witness_index(&r, "U2");
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1), "{a:?}");
    let mut evidence: Vec<usize> = a.evidence.iter().map(|e| e.index as usize).collect();
    evidence.retain(|&i| r[i].kind() == 13);
    evidence.sort_unstable();
    let mut want = vec![u2, configure];
    want.sort_unstable();
    assert_eq!(evidence, want, "{a:?}");
    assert_eq!(kinds(&a.evidence), [7, 7, 13, 13], "{a:?}");
    assert!(a.masked.is_empty(), "{a:?}");
    assert_eq!(
        fields_of(&r, configure),
        [
            ("i2c1.u5.address".to_owned(), 0x47, 0x47, None, false),
            (
                "i2c1.u5.Write".to_owned(),
                0x47,
                0x36_4081,
                Some(0x50),
                true
            ),
        ]
    );
    assert_eq!(
        fields_of(&r, identity),
        [
            ("i2c1.u5.address".to_owned(), 0x47, 0x47, None, false),
            ("i2c1.u5.Write".to_owned(), 0x47, 0x01, None, false),
            ("i2c1.u5.Read".to_owned(), 0x47, 0x50, None, false),
        ]
    );
    assert!(
        fields_of(&r, data).is_empty(),
        "a 6-byte read is no evidence"
    );
}

/// `AET-WO-3307` §5, targeted: a BMP581 witness reads only its own model's
/// lines, whole. U5's identity witness on two-bme with U5 a BMP581: our
/// model's line with a wrong register or value is the software's, and a
/// read too wide to judge still shows its pointer (adversarial 2). Every
/// other line is none it reads, so the witness shows nothing wrong, each
/// line below carrying a wrong register or value: a line another model
/// writes (the ICM42688P's `bank 0` line, the BME280's `Write`/`Read`), a
/// warning or another device's line, and a malformed one. A script in no
/// form it reads is no evidence at all, and a BME280's witness does not read
/// the BMP581's form.
#[test]
fn a_bmp581_witness_reads_only_its_models_lines_whole() {
    let identity = |r: &[Record]| {
        find(
            r,
            |x| matches!(x, Record::I2cWitness { reference, step, .. } if reference == "U5" && step == "identity"),
        )
    };
    let shown = |line: &str| {
        let r = bme_bmp(
            |_| {},
            [
                &[line],
                &["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"],
                &["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"],
            ],
        );
        fields_of(&r, identity(&r))
    };
    let wrong = |line: &str| -> Vec<(String, Option<u64>)> {
        shown(line)
            .into_iter()
            .filter(|f| f.4)
            .map(|f| (f.0, f.3))
            .collect()
    };
    assert!(wrong("[NOISY] i2c1.u5: read 0x01 -> 0x50").is_empty());
    // Its fixtures' passing verdict, a differing witness unnamed, is refused
    // (`fields_of` names it, `AET-WO-2312` adversarial F2).
    let r = bme_bmp(
        |_| {},
        [
            &["[NOISY] i2c1.u5: read 0x01 -> 0x51"],
            &["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"],
            &["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"],
        ],
    );
    assert_eq!(refused(&r), I2C_WITNESSES);
    assert_eq!(
        shown("[NOISY] i2c1.u5: read 0x01 -> 0x51 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00")
            .into_iter()
            .map(|f| (f.3, f.4))
            .collect::<Vec<_>>(),
        [(None, false), (Some(0x01), false), (None, false)],
        "the pointer of a read too wide to judge, and no value"
    );
    for (line, field, observed) in [
        ("[NOISY] i2c1.u5: read 0x01 -> 0x51", "Read", 0x51),
        ("[NOISY] i2c1.u5: read 0x02 -> 0x50", "Write", 0x02),
        ("[NOISY] i2c1.u5: write 0x02 <- (none)", "Write", 0x02),
        // A write of 0x01 then 0x50, shown under the step's one-byte mask.
        ("[NOISY] i2c1.u5: write 0x01 <- 0x50", "Write", 0x50),
        // Adversarial 2: a read too wide to judge keeps its pointer.
        (
            "[NOISY] i2c1.u5: read 0x02 -> 0x50 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00",
            "Write",
            0x02,
        ),
    ] {
        assert_eq!(
            wrong(line),
            [(format!("i2c1.u5.{field}"), Some(observed))],
            "{line}"
        );
    }
    for line in [
        // Another model's line, or another form.
        "[NOISY] i2c1.u5: bank 0 read 0x01 -> 0x51",
        "[NOISY] i2c1.u5: Write 02",
        "[NOISY] i2c1.u5: Read 51",
        "[NOISY] i2c1.u5: write to read-only register 0x02 ignored",
        "[NOISY] i2c1.u5: INT set",
        // Not this device's model's, at its level.
        "[WARNING] i2c1.u5: read 0x01 -> 0x51",
        "[NOISY] i2c1.u55: read 0x01 -> 0x51",
        "[NOISY] i2c1.u4: read 0x01 -> 0x51",
        // Malformed.
        "[NOISY] i2c1.u5: read 0x01 -> 0x5a",
        "[NOISY] i2c1.u5: read 0x01 -> 0x5",
        "[NOISY] i2c1.u5: read 0x01 -> 51",
        "[NOISY] i2c1.u5: read 0x01 -> 0X51",
        "[NOISY] i2c1.u5: read 0x001 -> 0x51",
        "[NOISY] i2c1.u5: read 0x01 -> (none)",
        "[NOISY] i2c1.u5: read 0x01 -> ",
        "[NOISY] i2c1.u5: read 0x01 ->",
        "[NOISY] i2c1.u5: read 0x01 -> 0x51 ",
        "[NOISY] i2c1.u5: read 0x01 -> 0x51  0x00",
        "[NOISY] i2c1.u5: read 0x81 -> 0x51",
        "[NOISY] i2c1.u5: read 0x02 <- 0x50",
        "[NOISY] i2c1.u5: write 0x02 -> 0x50",
        "[NOISY] i2c1.u5: Read 0x01 -> 0x51",
        "[NOISY] i2c1.u5: write 0x02 <- 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00",
    ] {
        let f = shown(line);
        assert!(
            f.len() == 3 && f.iter().all(|f| !f.4 && f.3.is_none()),
            "{line}: {f:?}"
        );
    }
    let mut r = bme_bmp(
        |_| {},
        [
            &["[NOISY] i2c1.u5: read 0x02 -> 0x51"],
            &["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"],
            &["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"],
        ],
    );
    let i = identity(&r);
    if let Record::I2cWitness { expected, .. } = &mut r[i] {
        *expected = vec!["[NOISY] i2c1.u5: read 0x01 -> 0x5".to_owned()];
    }
    assert!(fields_of(&r, i).is_empty(), "a malformed script");
    // A BMP581 witness that logged nothing where its check claims `ok` is
    // now the software's by silence, as a `Write`/`Read` witness is.
    let r = bme_bmp(
        |_| {},
        [
            &[],
            &["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"],
            &["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"],
        ],
    );
    assert_eq!(
        fields_of(&r, identity(&r))
            .into_iter()
            .map(|f| (f.3, f.4))
            .collect::<Vec<_>>(),
        [(None, false), (None, true), (None, true)],
        "silent"
    );
    // Nor does a `Write`/`Read` witness read our BMP581's form.
    let r = two_bme(&[], |r| observe(r, "U2", &["read 0xD1 -> 0x61"]));
    let f = fields_of(&r, witness_index(&r, "U2"));
    assert!(
        f.len() == 3 && f.iter().all(|f| !f.4 && f.3.is_none()),
        "{f:?}"
    );
}

/// Adversarial F1 (`AET-WO-3306` §6): a step is named as sent to the
/// surplus's address only if it went elsewhere. If its record claims `ok`
/// and its device's log holds its transaction, U2 was not the one at 0x77,
/// even with its value wrong. If it timed out with its step finding no
/// device at 0x75, it went to 0x75, not 0x77; with no record claiming
/// `ok`, nothing answered it. Each way U5's repeated read is still
/// software evidence, by the count alone.
#[test]
fn a_step_that_reached_its_own_device_or_no_device_is_not_named_as_the_surplus() {
    let held = two_bme(&[], |r| {
        replace(
            r,
            |x| matches!(x, Record::I2cCheck { reference, .. } if reference == "U2"),
            check("ok", 0x61),
        );
        observe(r, "U2", &["Write D0", "Read 61"]);
        observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
        replace(
            r,
            is_verdict,
            verdict(&[
                ("i2c_identity_mismatch", 1),
                ("i2c_transaction_unwitnessed", 2),
            ]),
        );
    });
    // Two more, each told apart by one fact alone: a step that timed out
    // with no absence logged, and one claiming `ok` whose step found no
    // device at 0x75.
    let elsewhere = |status: &'static str, absent: Vec<u8>| {
        two_bme(&[], move |r| {
            replace(
                r,
                |x| matches!(x, Record::I2cCheck { reference, .. } if reference == "U2"),
                check(status, if status == "ok" { 0x60 } else { 0 }),
            );
            let i = witness_index(r, "U2");
            if let Record::I2cWitness {
                observed,
                absent: a,
                ..
            } = &mut r[i]
            {
                *observed = Some(Vec::new());
                *a = absent.clone();
            }
            observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
            let findings: &[(&str, u64)] = if status == "ok" {
                &[("i2c_transaction_unwitnessed", 2)]
            } else {
                &[("i2c_timeout", 1), ("i2c_transaction_unwitnessed", 1)]
            };
            replace(r, is_verdict, verdict(findings));
        })
    };
    let timed_out = elsewhere("timeout_txis", vec![]);
    let to_0x75 = elsewhere("ok", vec![0x75]);
    // U2's own timeout with nothing logged and no absence is indeterminate
    // by itself (`a_txis_timeout_with_nothing_logged_is_indeterminate_not_software`),
    // and an indeterminate symptom names the run.
    for (r, rule) in [(held, Rule::S1), (timed_out, Rule::I4), (to_0x75, Rule::S1)] {
        let wrong: Vec<String> = fields_of(&r, witness_index(&r, "U5"))
            .into_iter()
            .filter(|f| f.4)
            .map(|f| f.0)
            .collect();
        assert_eq!(wrong, ["i2c1.u5.transactions"]);
        assert_eq!(attributed(&r).rule, rule);
    }
}

/// Adversarial F1: a step of the surplus's shape whose device cannot be read
/// whole (here an `[ERROR]` line in its log) might be the one, so no step
/// is named, only the address.
#[test]
fn a_candidate_that_cannot_be_told_names_no_step() {
    let r = two_bme(&[("U6", 0x75)], |r| {
        observe(r, "U2", &[]);
        let i = witness_index(r, "U6");
        if let Record::I2cWitness { observed, .. } = &mut r[i] {
            *observed = Some(vec!["[ERROR] i2c1.u6: bus error".to_owned()]);
        }
        observe(r, "U5", &["Write D0", "Read 60", "Write D0", "Read 60"]);
        // U6's step logged an `[ERROR]`: the judge names it
        // `i2c_step_error`, not unwitnessed (`AET-WO-2312` adversarial F3).
        replace(
            r,
            is_verdict,
            verdict(&[("i2c_step_error", 1), ("i2c_transaction_unwitnessed", 2)]),
        );
    });
    // The fixture's old verdict, three unwitnessed, is refused by it.
    let mut old = r.clone();
    replace(
        &mut old,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 3)]),
    );
    assert_eq!(refused(&old), refusal("i2c_step_error"));
    let wrong: Vec<String> = fields_of(&r, witness_index(&r, "U5"))
        .into_iter()
        .filter(|f| f.4)
        .map(|f| f.0)
        .collect();
    assert_eq!(wrong, ["i2c1.u5.transactions"]);
}

// ---- AET-WO-2312: attribute takes replay's derivation, and is bounded ----

/// Review round 3 A8 (`AET-WO-2312` (6)): the reviewer's four archives,
/// each this healthy run with one failure its verdict does not name, built
/// here by the reviewer's edits through `archived` (chaos holds only `.rs`
/// under `tests/`, so they are not committed as archives). They were `I1`
/// here while replay passed three of them. Replay now refuses each by the
/// kind its records show, and attribute with the same reason.
#[test]
fn the_round_3_archives_are_refused_as_replay_refuses_them() {
    let healthy_run = attribute(&archived(&healthy())).unwrap();
    assert_eq!(
        (healthy_run.domain, healthy_run.rule),
        (Domain::None, Rule::N1)
    );
    type Edit = fn(&mut Vec<Record>);
    let shapes: [(&str, Edit, &str); 4] = [
        (
            "reg_unnamed",
            |r| set_register(r, "GPIOB.AFRL", |g| g.observed = Some(0x4700_0000)),
            refusal("register_mismatch"),
        ),
        (
            "timeout_unnamed",
            |r| {
                replace(r, is_check, check("timeout_rxne", 0));
                replace(r, is_witness, witness(Some(&[])));
            },
            refusal("i2c_timeout"),
        ),
        (
            "idmismatch_unnamed",
            |r| {
                replace(r, is_check, check("ok", 0x58));
                replace(r, is_witness, witness(Some(&["Write D0", "Read 58"])));
            },
            refusal("i2c_identity_mismatch"),
        ),
        (
            "halted_unnamed",
            |r| {
                let b = find(r, |x| matches!(x, Record::BootObserved { .. }));
                if let Record::BootObserved {
                    instructions_observed,
                    ..
                } = &mut r[b]
                {
                    *instructions_observed = Some(5);
                }
            },
            HALTED,
        ),
    ];
    for (shape, edit, why) in shapes {
        let mut r = healthy();
        edit(&mut r);
        let run = archived(&r);
        assert_eq!(
            attribute(&run),
            Err(AttributeError::NotACampaignRun(why)),
            "{shape}"
        );
        assert_eq!(
            campaign::check_archived(&run.header, &run.state).unwrap_err(),
            why,
            "{shape}"
        );
    }
}

/// The records `attribute` takes a symptom from are replay's
/// (`campaign::shown`): before the boot word a read's value shows nothing,
/// as the judge names nothing of it, so the run is decided by its boot word,
/// not left unexplained. Evolve's own selection (before `AET-WO-2312`)
/// showed the read and made it `I1`.
#[test]
fn a_records_value_before_the_boot_word_shows_nothing_to_attribute() {
    let mut r = healthy();
    replace(&mut r, is_check, check("ok", 0x61));
    replace(
        &mut r,
        |x| matches!(x, Record::BootObserved { .. }),
        boot(0),
    );
    replace(&mut r, is_verdict, verdict(&[("boot_word_missing", 1)]));
    let a = attributed(&r);
    assert!(a.unexplained.is_empty(), "{a:?}");
    assert_ne!(a.rule, Rule::I1, "{a:?}");
}

/// Review round 3 A15: attribute's cost grows with symptoms × buses ×
/// records, and it is an MCP tool. Past `MAX_ATTRIBUTE_RECORDS` it refuses
/// by name; at it, it attributes.
#[test]
fn a_run_past_the_attribution_bound_is_refused_by_name() {
    use aethelgard_evolve::MAX_ATTRIBUTE_RECORDS;
    let padded = |total: usize| {
        let mut r = healthy();
        let at = find(&r, |x| matches!(x, Record::RegisterObserved { .. }));
        let pad = total - r.len();
        for i in 0..pad {
            r.insert(
                at,
                Record::RegisterObserved {
                    name: format!("GPIOK.PAD{i}"),
                    reading: reading(0x5802_2800 + 4 * i as u32, u32::MAX, 0, 0),
                },
            );
        }
        r
    };
    let at_bound = padded(MAX_ATTRIBUTE_RECORDS);
    assert_eq!(at_bound.len(), MAX_ATTRIBUTE_RECORDS);
    assert_eq!(attributed(&at_bound).rule, Rule::N1);
    let past = padded(MAX_ATTRIBUTE_RECORDS + 1);
    let run = archived(&past);
    // Replay judges it as before; attribute alone is bounded.
    assert!(campaign::check_archived(&run.header, &run.state).is_ok());
    assert_eq!(
        attribute(&run),
        Err(AttributeError::TooManyRecords {
            records: MAX_ATTRIBUTE_RECORDS + 1,
            limit: MAX_ATTRIBUTE_RECORDS,
        })
    );
}

/// A reached kind may pass its records in replay (`campaign::Bound::Reach`):
/// an unmodelled device's invalid read is named and not archived. Attribute
/// explains a kind only by as many records as the count, so the read with no
/// record leaves the kind unexplained, never decided by the other's.
#[test]
fn a_reached_count_past_its_records_is_unexplained() {
    let mut r = healthy();
    replace(&mut r, is_check, check("invalid", 0));
    replace(&mut r, is_witness, witness(Some(&[])));
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 2)]));
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
    let at = find(&r, is_check);
    assert_eq!(
        a.unexplained,
        [format!("event {at}"), "i2c_record_invalid".to_owned()],
        "{a:?}"
    );
}
