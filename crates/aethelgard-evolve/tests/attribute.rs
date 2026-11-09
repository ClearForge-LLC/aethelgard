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
    for net in ["I2C_SCL", "I2C_SDA"] {
        let i = find(
            r,
            |x| matches!(x, Record::RiseMeasured { bus, net: n, .. } if bus == "SENSORS" && n == net),
        );
        r[i] = rise("SENSORS", net, 3_982_301);
    }
    insert_after_last(
        r,
        |x| {
            matches!(
                x,
                Record::RiseMeasured { .. } | Record::LineStuckHigh { .. }
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

/// §5.3: removing all the electrical evidence never concludes either half.
#[test]
fn removing_every_rise_never_turns_a_software_run_electrical_or_keeps_it_software() {
    let mut r = healthy();
    wrong_address(&mut r);
    r.retain(|x| !matches!(x, Record::RiseMeasured { .. }));
    let a = attributed(&r);
    assert_eq!(a.domain, Domain::Indeterminate);
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
    // CR2 reads 0, its reset value: the firmware never wrote it, so it says
    // nothing of an address. The clock gate left off is the evidence.
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

// ---- The adversarial pass (A1 to A15) and the mutation run ----

fn insert_after_last(r: &mut Vec<Record>, pred: impl Fn(&Record) -> bool, record: Record) {
    let i = r.iter().rposition(pred).unwrap();
    r.insert(i + 1, record);
}

/// A second bus, AUX on I2C2 (pins PF1 and PF0), sorted before SENSORS, with
/// U5 at 0x40; measured healthy when `measured`.
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
        let i = find(r, |x| matches!(x, Record::RiseMeasured { .. }));
        r.insert(i, rise("AUX", "A_SCL", 186_402));
        r.insert(i + 1, rise("AUX", "A_SDA", 186_402));
    }
}

const CR2_U5: u32 = 0x0201_0480;

/// U5's identity read and its controller's witness.
fn u5_read(r: &mut Vec<Record>, status: &str, cr2: u32, txdr: u32) {
    insert_after_last(
        r,
        is_check,
        Record::I2cCheck {
            reference: "U5".into(),
            address: 0x40,
            register: 0xD0,
            expected: 0x60,
            observed: Some(0),
            status: Some(status.into()),
        },
    );
    insert_after_last(
        r,
        is_witness,
        Record::I2cWitness {
            peripheral: "I2C2".into(),
            reference: "U5".into(),
            cr2: reading(0x4000_5804, 0x02FF_07FF, CR2_U5, cr2),
            txdr: reading(0x4000_5828, 0xFF, 0xD0, txdr),
        },
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
fn a1_a_verdict_narrowed_or_passed_beside_a_failing_event_leaves_it_unexplained() {
    // The wrong-address run, its verdict rewritten to pass: the TXIS timeout
    // is still in the archive, so it is unexplained, never `none`.
    let mut r = healthy();
    wrong_address(&mut r);
    let timeout = find(&r, is_check);
    replace(&mut r, is_verdict, verdict(&[]));
    let a = attributed(&r);
    assert_eq!((a.domain, a.rule), (Domain::Indeterminate, Rule::I1));
    assert_eq!(a.unexplained, [format!("event {timeout}")]);
    // The timeout moved to `info`: the same.
    let mut r = healthy();
    wrong_address(&mut r);
    replace(
        &mut r,
        is_verdict,
        Record::Verdict {
            pass: true,
            findings: vec![("i2c_timeout".into(), "info".into(), 1)],
        },
    );
    assert_eq!(attributed(&r).rule, Rule::I1);
    // Two buses: one faulted, the other's wrong address dropped from the
    // verdict. Without the converse rule this was a confident E1.
    let mut r = healthy();
    add_aux(&mut r, true);
    u5_read(&mut r, "timeout_txis", 0x0001_0082, 0);
    faulted_sensors(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 2)]),
    );
    let a = attributed(&r);
    assert_eq!(a.rule, Rule::I1, "{a:?}");
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
        let i = find(r, |x| matches!(x, Record::RiseMeasured { .. }));
        r.insert(i + 2, rise("ZZZ", "Z_SCL", 186_402));
        r.insert(i + 3, rise("ZZZ", "Z_SDA", 186_402));
    };
    // The symptom's own bus is joined: still software.
    let mut r = healthy();
    wrong_address(&mut r);
    unjoinable(&mut r);
    assert_eq!(attributed(&r).rule, Rule::S1);
    // A symptom on the board needs every bus measured healthy: I2.
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
    // U2 times out with the right address; U3, on the same bus, completed a
    // read of the wrong register, and the bus's witness is U3's.
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_txis", 0));
    insert_after_last(
        &mut r,
        is_check,
        Record::I2cCheck {
            reference: "U3".into(),
            address: 0x48,
            register: 0x0F,
            expected: 0x117,
            observed: Some(0x00),
            status: Some("ok".into()),
        },
    );
    replace(
        &mut r,
        is_witness,
        Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: "U3".into(),
            cr2: reading(0x4000_5404, 0x02FF_07FF, 0x0202_0490, 0x0202_0490),
            txdr: reading(0x4000_5428, 0xFF, 0x0F, 0x0E),
        },
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
fn a5_a_witness_not_at_its_controllers_registers_is_no_evidence() {
    let mut r = healthy();
    wrong_address(&mut r);
    let mut w = witness(0x0001_00EE, 0);
    if let Record::I2cWitness { cr2, .. } = &mut w {
        cr2.address = 0x4000_5804;
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

#[test]
fn a10_one_event_never_explains_a_finding_counted_at_two_severities() {
    let mut r = healthy();
    wrong_address(&mut r);
    replace(
        &mut r,
        is_verdict,
        Record::Verdict {
            pass: false,
            findings: vec![
                ("i2c_timeout".into(), "error".into(), 1),
                ("i2c_timeout".into(), "warning".into(), 1),
            ],
        },
    );
    assert_eq!(attributed(&r).rule, Rule::I1);
}

#[test]
fn a15_the_evidence_is_the_named_rules_alone() {
    // A wrong address on the bus (S1) and no boot word (S2): the run is S1,
    // and its evidence holds no kind 10.
    let mut r = healthy();
    wrong_address(&mut r);
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

#[test]
fn every_witness_field_is_evidence_within_its_width() {
    // SADD's tenth bit, after the read completed with the right value.
    let completed = |cr2: u32| {
        let mut r = healthy();
        replace(&mut r, is_witness, witness(cr2, 0xD0));
        replace(
            &mut r,
            is_verdict,
            verdict(&[("i2c_transaction_unwitnessed", 1)]),
        );
        attributed(&r).rule
    };
    assert_eq!(completed(CR2_OK | 0x200), Rule::S1);
    // NBYTES's eighth bit.
    assert_eq!(completed(CR2_OK | 0x0080_0000), Rule::S1);
    // A CR2 reading that does not cover SADD and NBYTES is no evidence.
    let mut r = healthy();
    let mut w = witness(CR2_OK | 0x0080_0000, 0xD0);
    if let Record::I2cWitness { cr2, .. } = &mut w {
        cr2.mask = 0x0200_07FF;
        cr2.observed = Some(CR2_OK | 0x200);
    }
    replace(&mut r, is_witness, w);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I4);
    // A TXDR whose own expectation is not the read's register is no evidence.
    let mut r = healthy();
    let mut w = witness(CR2_OK, 0xD1);
    if let Record::I2cWitness { txdr, .. } = &mut w {
        txdr.expected = 0xD1;
        txdr.observed = Some(0xD2);
    }
    replace(&mut r, is_witness, w);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn txdr_counts_from_a_tc_timeout_and_nbytes_from_a_stopf_timeout() {
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_tc", 0));
    replace(&mut r, is_witness, witness(0x0001_20EC, 0xD1));
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::S1);
    let mut r = healthy();
    replace(&mut r, is_check, check("timeout_stopf", 0));
    replace(&mut r, is_witness, witness(0x0202_04EC, 0xD0));
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
    // The count of locked accesses is not compared: one finding, any number.
    replace(
        &mut r,
        is_verdict,
        verdict(&[("locked_peripheral_access", 3), ("register_mismatch", 1)]),
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
    let i = find(&r, |x| matches!(x, Record::RiseMeasured { .. }));
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
    r.retain(|x| !matches!(x, Record::RiseMeasured { net, .. } if net == "I2C_SDA"));
    let i = find(&r, |x| matches!(x, Record::RiseMeasured { .. }));
    r.insert(
        i + 1,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: 1_500_000,
            ceiling_ps: 300_000,
        },
    );
    r.insert(i + 2, fault("SENSORS", &["I2C_SDA"]));
    if let Record::BusFault { from, .. } = &mut r[i + 2] {
        *from = vec!["analog.line_stuck_high".into()];
    }
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_line_stuck_high", 1), ("i2c_timeout", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::E1, vec![8, 9]));

    let mut r = healthy();
    let i = find(&r, |x| matches!(x, Record::RiseMeasured { .. }));
    r[i] = Record::RiseMeasured {
        bus: "SENSORS".into(),
        net: "I2C_SCL".into(),
        rise_30_70_ps: 200_000,
        complete: false,
        ceiling_ps: 300_000,
        exceeds: true,
    };
    r.insert(i + 2, fault("SENSORS", &["I2C_SCL"]));
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("analog_rise_exceeds_ceiling", 1), ("i2c_timeout", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::E1);
}

#[test]
fn each_finding_selects_only_the_events_that_show_it() {
    // An invalid record, as invalid: no evidence, I4. Claimed as a timeout: I1.
    let mut r = healthy();
    replace(&mut r, is_check, check("invalid", 0));
    replace(&mut r, is_witness, witness(0x0001_00EC, 0));
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I4);
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I1);
    // An identity mismatch or an unwitnessed read claimed of a timed-out read.
    let mut r = healthy();
    wrong_address(&mut r);
    for kind in ["i2c_identity_mismatch", "i2c_transaction_unwitnessed"] {
        replace(&mut r, is_verdict, verdict(&[(kind, 1)]));
        assert_eq!(attributed(&r).rule, Rule::I1, "{kind}");
    }
    // A boot word claimed missing that is there, and a halt claimed of a full run.
    let mut r = healthy();
    for kind in ["boot_word_missing", "cpu_halted"] {
        replace(&mut r, is_verdict, verdict(&[(kind, 1)]));
        assert_eq!(attributed(&r).rule, Rule::I1, "{kind}");
    }
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
    assert_eq!(attributed(&r).rule, Rule::I1);
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
                | Record::RiseMeasured { .. }
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
    u5_read(&mut r, "timeout_txis", 0x0001_0080, 0);
    faulted_sensors(&mut r);
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { reference, .. } if reference == "U2"),
        witness(0x0001_00EC, 0),
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
    // AUX unmeasured (I2 for U5); SENSORS healthy with U2's right-address
    // timeout (I4): the run is I2, with AUX's kind 15 alone.
    let mut r = healthy();
    add_aux(&mut r, false);
    u5_read(&mut r, "timeout_txis", 0x0001_0080, 0);
    replace(&mut r, is_check, check("timeout_txis", 0));
    replace(
        &mut r,
        |x| matches!(x, Record::I2cWitness { reference, .. } if reference == "U2"),
        witness(0x0001_00EC, 0),
    );
    replace(&mut r, is_verdict, verdict(&[("i2c_timeout", 2)]));
    let a = attributed(&r);
    assert_eq!((a.rule, kinds(&a.evidence)), (Rule::I2, vec![15]));
}

#[test]
fn i5_lists_both_halves_and_the_masked_witness() {
    // SENSORS faulted with U2's wrong address (masked); AUX healthy with U5's.
    let mut r = healthy();
    wrong_address(&mut r);
    add_aux(&mut r, true);
    u5_read(&mut r, "timeout_txis", 0x0001_0082, 0);
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
    wrong_address(&mut r);
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_timeout", 1), ("renode_error_log", 1)]),
    );
    let a = attributed(&r);
    assert_eq!((a.rule, a.evidence.len()), (Rule::I1, 0));
}

#[test]
fn every_kind_of_failing_event_the_verdict_omits_is_unexplained() {
    // Each event shows a failure, and the verdict passes: every one is named.
    type Edit = Box<dyn Fn(&mut Vec<Record>)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "invalid",
            Box::new(|r| replace(r, is_check, check("invalid", 0))),
        ),
        (
            "identity",
            Box::new(|r| replace(r, is_check, check("ok", 0x61))),
        ),
        (
            "unwitnessed",
            Box::new(|r| replace(r, is_witness, witness(CR2_OK, 0xD1))),
        ),
        (
            "register",
            Box::new(|r| set_register(r, "GPIOA.MODER", |g| g.observed = Some(0))),
        ),
        (
            "boot word",
            Box::new(|r| replace(r, |x| matches!(x, Record::BootObserved { .. }), boot(0))),
        ),
        (
            "halted",
            Box::new(|r| {
                replace(
                    r,
                    |x| matches!(x, Record::BootObserved { .. }),
                    Record::BootObserved {
                        image_sha256: [4; 32],
                        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, 0x2100_B007),
                        instructions_expected: 10_000_000,
                        instructions_observed: Some(9_999_999),
                        pc: Some(0x0800_0596),
                    },
                )
            }),
        ),
        (
            "locked",
            Box::new(|r| set_register(r, "RCC.AHB4ENR", |g| g.observed = Some(0x1))),
        ),
    ];
    for (name, edit) in cases {
        let mut r = healthy();
        edit(&mut r);
        let a = attributed(&r);
        assert_eq!(a.rule, Rule::I1, "{name}: {a:?}");
        assert!(
            a.unexplained.iter().all(|u| u.starts_with("event ")),
            "{name}: {a:?}"
        );
    }
    // Two such events of one kind are both named.
    let mut r = healthy();
    set_register(&mut r, "GPIOA.MODER", |g| g.observed = Some(0));
    set_register(&mut r, "GPIOB.MODER", |g| g.observed = Some(0));
    assert_eq!(attributed(&r).unexplained.len(), 2);
}

#[test]
fn a_timeout_claimed_as_an_invalid_record_is_unexplained() {
    let mut r = healthy();
    wrong_address(&mut r);
    replace(&mut r, is_verdict, verdict(&[("i2c_record_invalid", 1)]));
    assert_eq!(attributed(&r).rule, Rule::I1);
}

#[test]
fn each_part_of_a_sound_witness_is_required() {
    let unwitnessed = |edit: &dyn Fn(&mut Reading, &mut Reading)| {
        let mut r = healthy();
        let mut w = witness(CR2_OK, 0xD1);
        if let Record::I2cWitness { cr2, txdr, .. } = &mut w {
            edit(cr2, txdr);
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
    assert_eq!(
        unwitnessed(&|_, t| t.address = 0x4000_5828 + 0x400),
        Rule::I4
    );
    assert_eq!(unwitnessed(&|_, t| t.mask = 0x0F), Rule::I4);
    // A CR2 reading that covers NBYTES but not SADD.
    assert_eq!(
        unwitnessed(&|c, _| {
            c.mask = 0x02FF_0400;
            c.expected = CR2_OK & 0x02FF_0400;
            c.observed = Some((CR2_OK & 0x02FF_0400) | 0xEE);
        }),
        Rule::I4
    );
    // A CR2 at its reset value, after a completed read: NBYTES says nothing.
    let mut r = healthy();
    replace(&mut r, is_witness, witness(0, 0xD0));
    replace(
        &mut r,
        is_verdict,
        verdict(&[("i2c_transaction_unwitnessed", 1)]),
    );
    assert_eq!(attributed(&r).rule, Rule::I4);
}

#[test]
fn a_register_with_nothing_observed_is_no_evidence() {
    let mut r = healthy();
    wrong_address(&mut r);
    set_register(&mut r, "GPIOB.AFRL", |g| g.observed = None);
    let a = attributed(&r);
    assert_eq!(kinds(&a.evidence), [7, 7, 13]);
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
