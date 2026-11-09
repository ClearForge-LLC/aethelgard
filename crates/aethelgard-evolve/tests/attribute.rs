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
use aethelgard_evolve::{Attribution, Domain, Rule, attribute};

const CR2_OK: u32 = 0x0201_04EC;

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
    Record::RiseMeasured {
        bus: bus.into(),
        net: net.into(),
        rise_30_70_ps: ps,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: ps > 300_000,
    }
}

fn fault(bus: &str, nets: &[&str]) -> Record {
    Record::BusFault {
        bus: bus.into(),
        fault: "absent".into(),
        source: "aethelgard".into(),
        from: vec!["analog.rise_time_measured > ceiling_ns".into()],
        nets: nets.iter().map(|n| (*n).into()).collect(),
        firmware_sees: "absent".into(),
    }
}

fn check(status: &str, observed: u64) -> Record {
    Record::I2cCheck {
        reference: "U2".into(),
        address: 0x76,
        register: 0xD0,
        expected: 0x60,
        observed: Some(observed),
        status: Some(status.into()),
    }
}

fn witness(cr2: u32, txdr: u32) -> Record {
    Record::I2cWitness {
        peripheral: "I2C1".into(),
        reference: "U2".into(),
        cr2: reading(0x4000_5404, 0x02FF_07FF, CR2_OK, cr2),
        txdr: reading(0x4000_5428, 0xFF, 0xD0, txdr),
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
        witness(CR2_OK, 0xD0),
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

/// The wrong-address shape: U2 times out at TXIS, `CR2.SADD` holds 0x77.
fn wrong_address(r: &mut [Record]) {
    replace(r, is_check, check("timeout_txis", 0));
    replace(r, is_witness, witness(0x0001_00EE, 0));
    replace(r, is_verdict, verdict(&[("i2c_timeout", 1)]));
}

/// A 47 kΩ shape on SENSORS: both rises past the ceiling and the bus fault.
fn faulted_sensors(r: &mut Vec<Record>) {
    let i = find(r, |x| matches!(x, Record::RiseMeasured { .. }));
    r[i] = rise("SENSORS", "I2C_SCL", 3_982_301);
    r[i + 1] = rise("SENSORS", "I2C_SDA", 3_982_301);
    r.insert(i + 2, fault("SENSORS", &["I2C_SCL", "I2C_SDA"]));
}

#[test]
fn the_healthy_run_passes_and_is_none() {
    let a = attributed(&healthy());
    assert_eq!((a.domain, a.rule), (Domain::None, Rule::N1));
    assert!(a.evidence.is_empty() && a.symptoms.is_empty());
}

#[test]
fn a_wrong_address_on_a_measured_healthy_bus_is_software_by_the_witness() {
    let mut r = healthy();
    wrong_address(&mut r);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Software, Rule::S1));
    assert_eq!(kinds(&a.evidence), [7, 7, 13]);
    assert_eq!(kinds(&a.symptoms), [12]);
    assert!(a.masked.is_empty());
}

#[test]
fn the_same_wrong_address_on_a_faulted_bus_is_electrical_with_the_witness_masked() {
    let mut r = healthy();
    wrong_address(&mut r);
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
fn a_txis_timeout_with_a_correct_address_is_indeterminate_not_software() {
    // TXDR is written only after TXIS, and CR2 still holds the write phase:
    // neither is evidence about a TXIS timeout. The bus is healthy and the
    // firmware asked for the right device, so nothing says which half.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I4));
    assert_eq!(kinds(&a.evidence), [7, 7]);
}

#[test]
fn no_rise_measured_on_a_line_of_the_symptoms_bus_is_indeterminate() {
    let mut r = healthy();
    wrong_address(&mut r);
    r.retain(|x| !matches!(x, Record::RiseMeasured { net, .. } if net == "I2C_SDA"));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I2));
    assert_eq!(kinds(&a.evidence), [15]);
}

#[test]
fn removing_every_rise_never_turns_a_software_run_electrical_or_keeps_it_software() {
    let mut r = healthy();
    wrong_address(&mut r);
    r.retain(|x| !matches!(x, Record::RiseMeasured { .. }));
    let a = attributed(&r);
    assert_eq!(a.domain, Domain::Indeterminate);
}

#[test]
fn a_line_measured_twice_is_not_measured_healthy() {
    let mut r = healthy();
    wrong_address(&mut r);
    let i = find(&r, |x| matches!(x, Record::RiseMeasured { .. }));
    let dup = r[i].clone();
    r.insert(i, dup);
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I2));
}

/// A second bus, OTHER on I2C2, with no device, faulted past its ceiling.
fn other_bus_faulted(r: &mut Vec<Record>) {
    let at = find(r, |x| matches!(x, Record::I2cBusSolved { .. }));
    r.insert(
        at,
        solved("OTHER", "I2C2", ["O_SCL", "O_SDA"], ["PF1", "PF0"], &[]),
    );
    let i = find(r, |x| matches!(x, Record::RiseMeasured { .. }));
    r.insert(i, rise("OTHER", "O_SCL", 900_000));
    r.insert(i + 1, rise("OTHER", "O_SDA", 900_000));
    let after = find(r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(after, fault("OTHER", &["O_SCL", "O_SDA"]));
}

#[test]
fn a_fault_on_another_bus_is_indeterminate_because_the_fault_is_elsewhere() {
    // R7: two buses through the chaos writer. U2 times out on SENSORS, which is
    // measured healthy (its rises are present); the only fault is on OTHER.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
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
            Record::RiseMeasured { bus, .. } | Record::BusFault { bus, .. } => bus.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(buses, ["OTHER", "OTHER", "SENSORS", "SENSORS", "OTHER"]);
}

#[test]
fn an_electrical_bus_and_a_software_bus_in_one_run_are_indeterminate_never_averaged() {
    let mut r = healthy();
    wrong_address(&mut r);
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
    // A wrong-register run (the read completed, TXDR holds 0xd1) whose verdict
    // is rewritten to claim a timeout: no event shows one, so it is I1, not
    // whatever a timeout would have been attributed to.
    let mut r = healthy();
    replace(&mut r, is_check, check("ok", 0x00));
    replace(&mut r, is_witness, witness(CR2_OK, 0xD1));
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
    let forged = attributed(&r);
    assert_eq!(
        (forged.domain, forged.rule),
        (Domain::Indeterminate, Rule::I1)
    );
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
fn a_finding_count_the_events_do_not_reach_is_unexplained() {
    let mut r = healthy();
    wrong_address(&mut r);
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 2)]));
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I1);
}

#[test]
fn a_witness_agreeing_with_a_wrong_address_leaves_no_software_evidence() {
    // The firmware's address was wrong, but the witness is rewritten to agree
    // with the solved one: attribution has nothing, and says so.
    let mut r = healthy();
    wrong_address(&mut r);
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_witness_whose_own_expectation_disagrees_with_the_solved_device_is_no_evidence() {
    let mut r = healthy();
    wrong_address(&mut r);
    let mut w = witness(0x0001_00EE, 0);
    if let Record::I2cWitness { cr2, .. } = &mut w {
        cr2.expected = 0x0201_04EE;
    }
    replace(&mut r, is_witness, w);
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn nbytes_counts_only_once_the_read_phase_began() {
    // A TC timeout: CR2 holds the write phase, NBYTES 1 against a 2-byte read.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_tc", 0));
    let mut w = witness(0x0001_00EC, 0xD0);
    if let Record::I2cWitness { cr2, .. } = &mut w {
        cr2.expected = 0x0202_04EC;
    }
    replace(&mut r, is_witness, w);
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    // Past the read's start, the same NBYTES is evidence.
    replace(&mut r, is_check, check("timeout_rxne", 0));
    assert_eq!(attributed(&r).rule, Rule::S1);
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
    replace(&mut r, is_witness, witness(0, 0));
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
    assert_eq!(kinds(&a.evidence), [7, 7, 11, 13]);
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
    // With no bus measured, nothing says the electrical layer is healthy.
    r.retain(|x| !matches!(x, Record::RiseMeasured { .. }));
    assert_eq!(attributed(&r).rule, Rule::I2);
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
    r.retain(|x| !matches!(x, Record::RiseMeasured { net, .. } if net == "I2C_SDA"));
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
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
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
