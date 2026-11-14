//! The one shape every tool verdict takes (`crates/aethelgard-cli/README.md`).

use serde::Serialize;

use crate::input::Input;
use crate::run::{SandboxReport, Tool};

#[derive(Serialize)]
pub struct Verdict {
    pub ok: bool,
    pub verdict: &'static str,
    pub check: &'static str,
    pub input: InputReport,
    pub tool: ToolReport,
    pub sandbox: SandboxReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_severities: Option<&'static [&'static str]>,
    /// `check drc` only: whose rules judged, `specification` or `kicad-default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<&'static str>,
    /// `check drc --spec` only: the sha256 of the specification file as read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification_sha256: Option<String>,
    /// `check drc --schematic` only: the schematic parity was judged against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schematic: Option<InputReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    /// `prove` only (`AET-WO-3301` §1.4): `generated` when the crate copied is
    /// byte for byte the board crate `generate` writes for `--spec`, else
    /// `supplied`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_provenance: Option<&'static str>,
    /// `prove` only: the cover properties Kani reported for the harness, 0
    /// when it reported none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_count: Option<u32>,
    /// `build` only: the ELF written, and its sha256.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elf: Option<InputReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    /// `simulate --firmware` only: what the boot was judged by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<Boot>,
    /// `simulate --firmware` only: the analog pre-pass's measurements and its
    /// fault decision, each its own record, apart from `boot` (`AET-WO-2200`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog: Option<Vec<AnalogRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faults: Option<Vec<FaultRecord>>,
    /// A campaign droop only (`AET-WO-2301`): the declared rail, ngspice's
    /// crossings, the resets the bridge rendered and what Renode read after
    /// each release, each its own record with its source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub droop: Option<DroopReport>,
    pub findings: Vec<Finding>,
}

/// A droop's records, as `campaign` archives them (kinds 16 to 19).
#[derive(Serialize)]
pub struct DroopReport {
    pub rail: RailReport,
    pub crossings: Vec<CrossingReport>,
    pub resets: Vec<ResetReport>,
    pub observed: Vec<ObservedReport>,
}

/// `analog.rail_declared`: every value the rail was computed from.
#[derive(Serialize)]
pub struct RailReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub domain: String,
    pub nominal_millivolts: u32,
    pub tolerance_permille: u32,
    pub floor_millivolts: u32,
    pub start_ns: u64,
    pub fall_ns: u64,
    pub hold_ns: u64,
    pub rise_ns: u64,
    pub source_milliohms: u32,
    pub load_milliamps: u32,
    pub decoupling_picofarads: u64,
    pub bor_level: u8,
    /// The rail's lowest value, as ngspice computed it.
    pub minimum_microvolts: u64,
    /// Where the thresholds come from (`AET-WO-2301` D4, provisional on F1).
    pub thresholds: &'static str,
}

/// `analog.rail_crossing_computed`.
#[derive(Serialize)]
pub struct CrossingReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub level: String,
    pub microvolts: u64,
    pub at_ps: u64,
    pub falling: bool,
}

/// `bridge.reset_rendered`: the model's rendering, never a measured reset.
#[derive(Serialize)]
pub struct ResetReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub at_ns: u64,
    pub released: bool,
    pub cause: Vec<String>,
    pub rendering: &'static str,
}

/// `firmware.observed` at the recovery bound after a release.
#[derive(Serialize)]
pub struct ObservedReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub at_ns: u64,
    pub boot_word: Word,
    pub status: Word,
    pub value: Word,
    /// What Renode logged for the first identity read's device from the
    /// release to this read: only a transfer the controller runs calls the
    /// model (`AET-WO-2301` adversarial F1, `AET-WO-2204`). `null` when the
    /// boot has no identity read.
    pub witness: Option<WitnessReport>,
}

/// A boot's inputs, its expectations and what Renode read (`AET-WO-2100`).
/// Every expected value comes from the specification's solved topology and
/// the CLI's boot contract, never from the firmware (`INV-07`).
#[derive(Serialize)]
pub struct Boot {
    pub firmware: FirmwareReport,
    pub specification_sha256: String,
    pub seed: u32,
    pub virtual_time_ns: u64,
    /// `null` until Renode has read it, as is every `observed` below.
    pub mips: Option<u64>,
    pub instructions: Expected<u64>,
    /// Where the core was when the run ended, for a reader; not a witness.
    pub pc: Option<String>,
    pub boot_word: Word,
    pub registers: Vec<RegisterReport>,
    pub unobservable: Vec<String>,
    /// One per identity read the judge expects (`AET-WO-2202`).
    pub i2c: Vec<I2cReport>,
    /// What Renode logged for each identity read's device (`AET-WO-2204` D1).
    pub i2c_witness: Vec<WitnessReport>,
    /// `ICSR` after the run: `VECTACTIVE` [8:0] is 0 when the core is in
    /// thread mode (`AET-WO-2204` D3).
    pub icsr: Option<String>,
}

/// One identity read's witness: the device model's whole log lines the judge
/// expects, and every line Renode logged for the device with a witness's
/// shape (`AET-WO-2204` D1).
#[derive(Serialize)]
pub struct WitnessReport {
    pub peripheral: String,
    pub reference: String,
    /// The model's path in Renode, `i2c1.u2`.
    pub device: String,
    /// Empty when the model has no known witness format (a named failure).
    pub expected: Vec<String>,
    /// `null` until Renode has run, or when the log could not be read whole.
    pub observed: Option<Vec<String>>,
    /// The addresses the device's controller found no device at, in order:
    /// attribution's evidence, never part of the witness.
    pub absent: Vec<String>,
}

impl WitnessReport {
    /// `check`'s witness as the judge expects it, with what Renode logged.
    pub fn of(
        check: &aethelgard_twin::boot::I2cCheck,
        observed: Option<Vec<String>>,
        absent: Vec<String>,
    ) -> Self {
        WitnessReport {
            peripheral: check.peripheral.clone(),
            reference: check.reference.clone(),
            device: check.device.clone(),
            expected: check.witness.clone().unwrap_or_default(),
            observed,
            absent,
        }
    }
}

/// One identity read: what the part table says, and what the firmware's record held.
#[derive(Serialize)]
pub struct I2cReport {
    pub reference: String,
    pub address: String,
    pub register: String,
    pub expected: String,
    pub observed: Option<String>,
    /// `ok`, `timeout_txis`, `timeout_tc`, `timeout_rxne`, `timeout_stopf` or `invalid`;
    /// `null` until Renode has read the record.
    pub status: Option<&'static str>,
}

#[derive(Serialize)]
pub struct FirmwareReport {
    pub path: String,
    /// The file's bytes as copied.
    pub sha256: String,
    /// The `PT_LOAD` file bytes in physical-address order: what Renode loads.
    pub image_sha256: String,
}

#[derive(Serialize)]
pub struct Expected<T> {
    pub expected: T,
    pub observed: Option<T>,
}

#[derive(Serialize)]
pub struct Word {
    pub address: String,
    pub expected: String,
    pub observed: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterReport {
    pub name: String,
    pub address: String,
    pub mask: String,
    pub expected: String,
    pub observed: Option<String>,
}

/// `0x2100b007`.
pub fn word(v: u32) -> String {
    format!("{v:#010x}")
}

#[derive(Serialize)]
pub struct InputReport {
    pub path: String,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct ToolReport {
    pub name: &'static str,
    pub version: String,
    pub exit_code: i32,
}

#[derive(Serialize)]
pub struct Finding {
    pub kind: String,
    pub severity: String,
    pub message: String,
    #[serde(rename = "where")]
    pub location: Vec<Where>,
}

#[derive(Serialize)]
pub struct Where {
    pub description: String,
    pub x: f64,
    pub y: f64,
    pub uuid: String,
}

#[derive(Serialize)]
pub struct Machine {
    pub cpus: Vec<String>,
    pub peripherals: Vec<Peripheral>,
}

#[derive(Serialize)]
pub struct Peripheral {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

impl Finding {
    /// A finding with no location: the Kani and Renode kinds.
    pub fn plain(kind: &str, message: String) -> Self {
        Self {
            kind: kind.to_owned(),
            severity: "error".to_owned(),
            message,
            location: Vec::new(),
        }
    }

    /// A finding of severity `info`, which does not fail the verdict. Used for
    /// what the twin could not check, so it is reported and never passed
    /// silently (`AET-WO-2202` §1.2).
    pub fn info(kind: &str, message: String) -> Self {
        Self {
            severity: "info".to_owned(),
            ..Self::plain(kind, message)
        }
    }
}

impl Verdict {
    /// A verdict is pass exactly when it has no finding above severity `info`;
    /// the caller decides that only after both witnesses agreed.
    pub fn new(
        check: &'static str,
        input: &Input,
        tool: &Tool,
        version: String,
        code: i32,
        sandbox: SandboxReport,
        findings: Vec<Finding>,
    ) -> Self {
        // Only the boot path makes an `info` finding (KiCad's severities are
        // `error` and `warning`, which `kicad` refuses to widen).
        let pass = findings.iter().all(|f| f.severity == "info");
        Self {
            ok: pass,
            verdict: if pass { "pass" } else { "fail" },
            check,
            input: InputReport {
                path: input.original.display().to_string(),
                sha256: input.sha256.clone(),
            },
            tool: ToolReport {
                name: tool.name,
                version,
                exit_code: code,
            },
            sandbox,
            included_severities: None,
            rules: None,
            specification_sha256: None,
            schematic: None,
            harness: None,
            harness_provenance: None,
            cover_count: None,
            elf: None,
            machine: None,
            boot: None,
            analog: None,
            faults: None,
            droop: None,
            findings,
        }
    }
}

/// One I2C line's rise time as the bridge measured it (`AET-WO-2200`): the
/// evidence. Nothing in it comes from the firmware or from Renode.
#[derive(Serialize)]
pub struct AnalogRecord {
    /// `analog.rise_time_measured`, or `analog.line_stuck_high` for a line
    /// at or above 30 % when its driver released it.
    pub kind: &'static str,
    pub source: &'static str,
    pub bus: String,
    pub net: String,
    /// False when the line had not reached 70 % when the window ended, or
    /// never fell (ruled F7 (b) on PR #70).
    pub complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rise_30_70_ns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rise_30_70_ns_lower_bound: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volts_at_release: Option<f64>,
    pub ceiling_ns: u32,
    pub exceeds: bool,
    pub method: &'static str,
    pub ngspice: &'static str,
}

/// The decision a measurement past its ceiling forces on its bus, and what the
/// firmware is given to see of it, labelled as the model's rendering.
#[derive(Serialize)]
pub struct FaultRecord {
    pub kind: &'static str,
    /// `aethelgard` for a measured fault; `scenario <name>` for a line a
    /// campaign scenario holds low (`AET-WO-2300`).
    pub source: String,
    pub bus: String,
    pub fault: &'static str,
    /// Each distinct cause among the bus's lines.
    pub from: Vec<&'static str>,
    /// The bus's lines past their ceiling.
    pub nets: Vec<String>,
    pub firmware_sees: &'static str,
}
