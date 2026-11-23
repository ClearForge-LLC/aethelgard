//! Fault scenarios and the campaign's run rules (`AET-WO-2300`).
//!
//! A scenario is a strict JSON file, authored apart from anything generated
//! (`INV-07`): a name, the specification it starts from, what it expects, and
//! closed-world overrides. [`Scenario::parse`] reads it; the runner, in the
//! CLI, applies the overrides to the specification, because nothing in this
//! crate reads a `Specification` (`RUL-16`, `AET-WO-2203`). [`check_run`] and
//! [`check_archived`] are what `replay` judges a campaign run by, from its
//! header and records alone.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

use crate::event::{Header, name_ok};
use crate::power::{self, Band};
use crate::record::{
    self, ANALOG_MAX_MILLIVOLTS, BatterySag, BusLine, Droop, Level, Line, Override, RailDroop,
    Reading, Record, SERIAL_FRAME_LEN, SerialCapture, SerialFraming, realized_ns, split_pin,
    step_index,
};
use crate::replay::State;

/// The scenario format versions read: 1 (static scenarios, unchanged); 2,
/// since `AET-WO-2301`, which admits the `droop` override and `expect_counts`,
/// and since `AET-WO-4200` the `spi_device_absent` override; and 3, since
/// `AET-WO-2302` (D5), which also admits the `serial_rx_open` override; 4,
/// since `AET-WO-2304b`, which also admits `battery_sag` and
/// `rail_out_of_band`; 5, since `AET-WO-1211c`, which also admits a pin
/// device's overrides, `pin_held` and `analog_value`; and 6, since
/// `AET-WO-2310` ([`PLANTED_CHANGES`]), the only version that reads a droop
/// or a battery sag ([`PLANTED_NEEDS_VERSION`]). A file may use only what its
/// version admits.
pub const SCENARIO_VERSIONS: [u32; 6] = [1, 2, 3, 4, 5, 6];
/// The scenario version a planted event (a droop, a battery sag) needs.
pub const PLANTED_VERSION: u32 = 6;
/// A droop or a battery sag in a scenario older than [`PLANTED_VERSION`].
pub const PLANTED_NEEDS_VERSION: &str = "PlantedEventNeedsVersion6";
/// What changed for a planted event at [`PLANTED_VERSION`], as its refusal
/// says (`AET-WO-1215`'s pattern).
pub const PLANTED_CHANGES: &str = "scenario 6 (AET-WO-2310): a droop's and a battery_sag's `start_ns` from reset is `from_boot_word_ns`, signed ns from the undisturbed boot word, and a droop states no `bor_level` (the specification's mcu.bor_level is the level)";
/// A planted event the undisturbed boot word cannot place: its instant
/// before reset or past the run, or no boot word to place it from
/// (`AET-WO-2310`, ruled Q-A (a)).
pub const PLANTED_UNPLACED: &str = "PlantedEventUnplaced";
/// At most this many overrides, so the scenario record stays inside the
/// reader's 4,096-byte event bound.
pub const MAX_OVERRIDES: usize = 32;
/// At most this many expected finding kinds, for the same reason.
pub const MAX_EXPECTED: usize = 16;
/// Why a held-low line faults its bus, as the fault record's `from` names it.
pub const HELD_LOW: &str = "scenario.line_held_low";
/// The finding a held-low line fails its run with.
pub const HELD_LOW_FINDING: &str = "i2c_line_held_low";
/// The findings the analog pre-pass fails a run with, one per line.
pub const PAST_CEILING_FINDING: &str = "analog_rise_exceeds_ceiling";
/// The boot judge's finding for a core that executed fewer instructions than
/// the run holds: it slept or stopped (`renode::judge_boot`), with or without
/// the boot word (`AET-WO-2310`, overturning `AET-WO-2100`'s ruled A10).
pub const HALTED_FINDING: &str = "cpu_halted";
/// One per reset a droop rendered (`info`): the reset itself is the
/// supervisor doing its job, so it never fails a run.
pub const RESET_FINDING: &str = "brownout_reset";
/// One per reset the firmware did not recover from within
/// its recovery bound ([`power::recovery_bound`]) of its release's boot, or
/// that was never released, or whose bound could not be read.
pub const UNRECOVERED_FINDING: &str = "brownout_unrecovered";
/// A droop whose rail left the domain's declared tolerance (`info`).
pub const BELOW_TOLERANCE_FINDING: &str = "rail_below_tolerance";
pub const STUCK_HIGH_FINDING: &str = "analog_line_stuck_high";
/// The boot judge's finding for a core that ended outside every executable
/// segment of the firmware, which kind 10 archives (`AET-WO-2205` §1.5).
pub const PC_OUTSIDE_FINDING: &str = "pc_outside_firmware";
/// The boot judge's finding for a core parked inside the firmware but off
/// every self-loop, made only when `pc` is inside (`AET-WO-2204` D3).
pub const SELF_LOOP_FINDING: &str = "pc_not_on_self_loop";
/// The boot judge's finding for a boot record whose brown-out word is not
/// the declared level, or was never recorded (`AET-WO-4230` §1.6, D10:
/// `BrownOutLevelNotProgrammed`), which kind 10 archives.
pub const BROWN_OUT_FINDING: &str = "brown_out_level_not_programmed";
/// The boot judge's finding for a flash controller accessed otherwise than
/// by the bring-up's one read of `FLASH_OPTSR_CUR` (`AET-WO-4230` D10).
pub const FLASH_ACCESS_FINDING: &str = "flash_controller_access";
// belt-and-suspenders: `aethelgard-twin` `boot::RECORD_UNSET`, which this
// crate does not depend on; the CLI's tests archive the twin's fill.
/// The judge's fill of the boot record's words before the core runs: a
/// brown-out word still at it was never recorded, whatever its `BOR_LEV`
/// bits (3) say.
pub const RECORD_UNSET: u32 = 0xFFFF_FFFF;

/// True when kind 10's brown-out word, read, is the judge's fill or its
/// `BOR_LEV` is not the declared level (`AET-WO-4230` D10).
pub fn brown_out_fails(r: &record::Reading) -> bool {
    r.observed
        .is_some_and(|o| o == RECORD_UNSET || o & r.mask != r.expected)
}

/// The boot judge's finding for a boot that was read and whose boot word is
/// not the firmware's, made once (`AET-WO-3201` B2).
pub const BOOT_WORD_FINDING: &str = "boot_word_missing";
/// A boot whose machine aborted and never ended (`renode::bounded`).
pub const CPU_ABORTED_FINDING: &str = "cpu_aborted";
/// A firmware Renode could not load (`renode::judge_boot`).
pub const FIRMWARE_LOAD_FINDING: &str = "firmware_load_error";
/// A boot killed at its memory cap (`renode::bounded`).
pub const MEMORY_CAP_FINDING: &str = "memory_cap_exceeded";
/// A platform Renode could not load (`renode::judge_boot`).
pub const PLATFORM_LOAD_FINDING: &str = "platform_load_error";
/// A boot killed at its wall-clock bound (`renode::bounded`).
pub const WALL_BOUND_FINDING: &str = "wall_bound_exceeded";
/// The findings of a boot that read nothing, one of which the judge names
/// (`AET-WO-3201` B2): killed at a bound (the CLI's `renode::bounded`), or a
/// platform or firmware that did not load (`renode::judge_boot`; adversarial
/// pass, F1). The judge names each by the constant above (`AET-WO-2310`).
pub const UNREAD_FINDINGS: [&str; 5] = [
    CPU_ABORTED_FINDING,
    FIRMWARE_LOAD_FINDING,
    MEMORY_CAP_FINDING,
    PLATFORM_LOAD_FINDING,
    WALL_BOUND_FINDING,
];
/// The identity reads' findings the judge makes only once the boot word is
/// set (`renode::judge_boot`): a record is judged at the fixed point, and
/// before it only a timeout is named (`AET-WO-3201`; adversarial pass, F2).
pub const BOOTED_READ_FINDINGS: [&str; 10] = [
    "i2c_configure_mismatch",
    "i2c_data_mismatch",
    "i2c_identity_mismatch",
    "i2c_record_invalid",
    "i2c_transaction_unwitnessed",
    "spi_configure_mismatch",
    "spi_data_mismatch",
    "spi_identity_mismatch",
    "spi_record_invalid",
    "spi_transaction_unwitnessed",
];
/// A read's status in kinds 12 and 21 when it completed (`renode::status_name`).
pub const STATUS_OK: &str = "ok";
/// The judge's findings for an `ok` read whose witness was not the model's
/// or could not be read (`renode::witness_finding`), two of
/// [`BOOTED_READ_FINDINGS`].
pub const UNWITNESSED_I2C: &str = "i2c_transaction_unwitnessed";
pub const UNWITNESSED_SPI: &str = "spi_transaction_unwitnessed";

/// How many `ok` reads of `bus` (kind 12 for `i2c`, 21 for `spi`) have their
/// own witness (kind 13 or 22, by reference and step) unread.
fn unread_ok_witnesses(records: &[Record], bus: &str) -> u64 {
    let unread = |reference: &str, step: &str| {
        records.iter().any(|w| match (bus, w) {
            (
                "i2c",
                Record::I2cWitness {
                    reference: r,
                    step: s,
                    observed,
                    ..
                },
            )
            | (
                "spi",
                Record::SpiWitness {
                    reference: r,
                    step: s,
                    observed,
                    ..
                },
            ) => r == reference && s == step && observed.is_none(),
            _ => false,
        })
    };
    records
        .iter()
        .filter(|x| match (bus, x) {
            (
                "i2c",
                Record::I2cCheck {
                    reference,
                    step,
                    status,
                    ..
                },
            )
            | (
                "spi",
                Record::SpiCheck {
                    reference,
                    step,
                    status,
                    ..
                },
            ) => status.as_deref() == Some(STATUS_OK) && unread(reference, step),
            _ => false,
        })
        .count() as u64
}

/// A scenario refused, with the kind `aethelgard` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioError {
    pub kind: &'static str,
    pub id: Option<String>,
    pub message: String,
}

impl ScenarioError {
    pub fn new(kind: &'static str, id: Option<&str>, message: String) -> Self {
        Self {
            kind,
            id: id.map(str::to_owned),
            message,
        }
    }
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Expect {
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LineName {
    Sda,
    Scl,
}

/// Present, even as `null`, is `Some`; so a key a variant does not use is
/// refused whatever its value.
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<serde_json::Value>, D::Error> {
    serde_json::Value::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawOverride {
    #[serde(default, deserialize_with = "present")]
    bus: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    net: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    pull_up_ohms: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    max_capacitance_femtofarads: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    line_held_low: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    droop: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    spi_device_absent: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    serial_rx_open: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    battery_sag: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    rail_out_of_band: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    pin_held: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "present")]
    analog_value: Option<serde_json::Value>,
}

/// `{"net": ..., "level": "low" | "high"}`, nothing else
/// (`AET-WO-1211c`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawPinHeld {
    net: String,
    level: Level,
}

/// `{"net": ..., "millivolts": n}`, nothing else (`AET-WO-1211c`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawAnalogValue {
    net: String,
    millivolts: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawBatterySag {
    series_milliohms: u64,
    from_boot_word_ns: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawDroop {
    domain: String,
    floor_millivolts: u64,
    from_boot_word_ns: i64,
    fall_ns: u64,
    hold_ns: u64,
    rise_ns: u64,
    source_milliohms: u64,
    load_milliamps: u64,
}

/// `expect_counts`: an object of finding kinds to counts, in the file's order;
/// a duplicate key is refused rather than one silently winning.
fn counts<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<(String, u64)>>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<Vec<(String, u64)>>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an object of finding kinds to counts")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out: Vec<(String, u64)> = Vec::new();
            while let Some((k, n)) = map.next_entry::<String, u64>()? {
                if out.iter().any(|(seen, _)| *seen == k) {
                    return Err(serde::de::Error::custom(format!(
                        "expect_counts names `{k}` twice"
                    )));
                }
                out.push((k, n));
            }
            Ok(Some(out))
        }
    }
    d.deserialize_map(V)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
struct RawScenario {
    schema_version: u32,
    name: String,
    base: String,
    expect: Expect,
    expect_findings: Vec<String>,
    #[serde(default, deserialize_with = "counts")]
    expect_counts: Option<Vec<(String, u64)>>,
    overrides: Vec<RawOverride>,
}

/// The derives above are `remote = "Self"`; these wrap them so each type
/// deserializes only from an object, never from a positional array, as
/// `aethelgard-ir`'s types do.
macro_rules! object_only {
    ($($ty:ident),*) => {$(
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;
                impl<'de> Visitor<'de> for ObjectVisitor {
                    type Value = $ty;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " object"))
                    }
                    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<$ty, A::Error> {
                        $ty::deserialize(MapAccessDeserializer::new(map))
                    }
                }
                deserializer.deserialize_map(ObjectVisitor)
            }
        }
    )*};
}

object_only!(
    RawOverride,
    RawScenario,
    RawDroop,
    RawBatterySag,
    RawPinHeld,
    RawAnalogValue
);

/// One scenario file, parsed and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub name: String,
    /// The specification it starts from, relative to the scenario file's
    /// directory; the caller resolves and bounds it.
    pub base: String,
    pub expect_pass: bool,
    /// Sorted and unique; empty exactly when `expect_pass`.
    pub expect_findings: Vec<String>,
    /// In the file's order.
    pub overrides: Vec<Override>,
    /// Exact counts of finding kinds, any severity, sorted by kind; empty
    /// when the file declares none (`AET-WO-2300` F11, `AET-WO-2301` §2.3).
    pub expect_counts: Vec<(String, u64)>,
    /// The file's sha256.
    pub sha256: [u8; 32],
}

fn schema(message: String) -> ScenarioError {
    ScenarioError::new("SchemaViolation", None, message)
}

fn override_from(raw: RawOverride, version: u32) -> Result<Override, ScenarioError> {
    use serde_json::Value as J;
    let text = |v: J, key: &str| match v {
        J::String(s) if name_ok(&s) => Ok(s),
        other => Err(schema(format!(
            "override `{key}` is not 1 to 64 printable ASCII bytes: {other}"
        ))),
    };
    let int = |v: J, key: &str, max: u64| match v.as_u64() {
        Some(n) if n <= max && v.is_u64() => Ok(n),
        _ => Err(schema(format!(
            "override `{key}` is not an integer from 0 to {max}: {v}"
        ))),
    };
    let RawOverride {
        bus,
        net,
        pull_up_ohms,
        max_capacitance_femtofarads,
        line_held_low,
        droop,
        spi_device_absent,
        serial_rx_open,
        battery_sag,
        rail_out_of_band,
        pin_held,
        analog_value,
    } = raw;
    // The pin device's overrides (`AET-WO-1211c`) are read first, and each
    // is refused beside any other key: none of the branches below knows
    // them, so one left to them would be dropped unread.
    let old_keys = bus.is_some()
        || net.is_some()
        || pull_up_ohms.is_some()
        || max_capacitance_femtofarads.is_some()
        || line_held_low.is_some()
        || droop.is_some()
        || spi_device_absent.is_some()
        || serial_rx_open.is_some()
        || battery_sag.is_some()
        || rail_out_of_band.is_some();
    if let Some(p) = pin_held {
        if old_keys || analog_value.is_some() {
            return Err(schema(
                "a pin_held override is exactly {pin_held: {net, level}}".to_owned(),
            ));
        }
        return pin_held_from(p);
    }
    if let Some(a) = analog_value {
        if old_keys {
            return Err(schema(
                "an analog_value override is exactly {analog_value: {net, millivolts}}".to_owned(),
            ));
        }
        return analog_value_from(a);
    }
    let alone = |key: &str, others: bool| {
        if others {
            Err(schema(format!(
                "a {key} override is exactly {{{key}: ...}}"
            )))
        } else {
            Ok(())
        }
    };
    let rest = |a: bool, b: bool| {
        a || b
            || bus.is_some()
            || net.is_some()
            || pull_up_ohms.is_some()
            || max_capacitance_femtofarads.is_some()
            || line_held_low.is_some()
            || droop.is_some()
            || spi_device_absent.is_some()
            || serial_rx_open.is_some()
    };
    if (battery_sag.is_some() || droop.is_some()) && version < PLANTED_VERSION {
        return Err(ScenarioError::new(
            PLANTED_NEEDS_VERSION,
            None,
            format!(
                "scenario schema_version {version} reads no droop or battery_sag: {PLANTED_CHANGES}"
            ),
        ));
    }
    if let Some(b) = battery_sag {
        alone("battery_sag", rest(false, rail_out_of_band.is_some()))?;
        return battery_sag_from(b).map(Override::BatterySag);
    }
    if let Some(d) = rail_out_of_band {
        alone("rail_out_of_band", rest(false, false))?;
        return Ok(Override::RailOutOfBand {
            domain: text(d, "rail_out_of_band")?,
        });
    }
    let others = bus.is_some()
        || net.is_some()
        || pull_up_ohms.is_some()
        || max_capacitance_femtofarads.is_some()
        || line_held_low.is_some()
        || droop.is_some();
    if let Some(p) = serial_rx_open {
        if others || spi_device_absent.is_some() {
            return Err(schema(
                "an open serial RX line override is exactly {serial_rx_open: port}".to_owned(),
            ));
        }
        return Ok(Override::SerialRxOpen {
            port: text(p, "serial_rx_open")?,
        });
    }
    if let Some(r) = spi_device_absent {
        if others {
            return Err(schema(
                "an absent SPI device override is exactly {spi_device_absent: reference}"
                    .to_owned(),
            ));
        }
        return Ok(Override::SpiDeviceAbsent {
            reference: text(r, "spi_device_absent")?,
        });
    }
    if let Some(d) = droop {
        if bus.is_some()
            || net.is_some()
            || pull_up_ohms.is_some()
            || max_capacitance_femtofarads.is_some()
            || line_held_low.is_some()
        {
            return Err(schema(
                "a droop override is exactly {droop: {...}}".to_owned(),
            ));
        }
        return droop_from(d).map(Override::Droop);
    }
    match (bus, net, pull_up_ohms, max_capacitance_femtofarads, line_held_low) {
        (Some(bus), None, Some(ohms), None, None) => Ok(Override::PullUpOhms {
            bus: text(bus, "bus")?,
            ohms: u32::try_from(int(ohms, "pull_up_ohms", u32::MAX.into())?)
                .expect("bounded by u32::MAX"),
        }),
        (None, Some(net), None, Some(ff), None) => Ok(Override::MaxCapacitance {
            net: text(net, "net")?,
            femtofarads: int(ff, "max_capacitance_femtofarads", u64::MAX)?,
        }),
        (Some(bus), None, None, None, Some(line)) => Ok(Override::LineHeldLow {
            bus: text(bus, "bus")?,
            line: match serde_json::from_value::<LineName>(line.clone()) {
                Ok(LineName::Sda) => Line::Sda,
                Ok(LineName::Scl) => Line::Scl,
                Err(_) => {
                    return Err(schema(format!(
                        "override `line_held_low` is not \"sda\" or \"scl\": {line}"
                    )));
                }
            },
        }),
        _ => Err(schema(
            "an override is exactly {bus, pull_up_ohms}, {net, max_capacitance_femtofarads}, {bus, line_held_low}, {droop}, {spi_device_absent}, {serial_rx_open}, {battery_sag}, {rail_out_of_band}, {pin_held} or {analog_value}"
                .to_owned(),
        )),
    }
}

/// The bounds of a droop's unsigned integers (`AET-WO-2301` §5.4), each
/// checked on its own before any arithmetic: the run is 100 ms, a source is
/// at most 1 kΩ, a load at most 10 A. Its offset is [`offset_from`]'s.
const DROOP_BOUNDS: [(&str, u64, u64); 6] = [
    ("floor_millivolts", 1, 5_000),
    ("fall_ns", 1, 10_000_000),
    ("hold_ns", 1, 100_000_000),
    ("rise_ns", 1, 10_000_000),
    ("source_milliohms", 1, 1_000_000),
    ("load_milliamps", 0, 10_000),
];

/// A planted event's `from_boot_word_ns` (`AET-WO-2310`): within a run's
/// length either way of the boot word ([`record::MAX_PLANT_OFFSET_NS`]) and
/// a whole number of 10 ns instructions, as the boot word is. Judged before
/// any boot, from the offset alone: the instant from reset is judged once
/// the undisturbed boot word places it ([`PLANTED_UNPLACED`]).
fn offset_from(what: &str, v: i64) -> Result<i64, ScenarioError> {
    let max = record::MAX_PLANT_OFFSET_NS;
    if !record::plant_offset_ok(v) {
        return Err(schema(format!(
            "{what} `from_boot_word_ns` is not a multiple of 10 from {} to {max}: {v} (judged before the boot, from the offset alone; its instant from reset is judged once the undisturbed boot word places it)",
            -max
        )));
    }
    Ok(v)
}

fn droop_from(v: serde_json::Value) -> Result<Droop, ScenarioError> {
    let raw: RawDroop = serde_json::from_value(v).map_err(|e| schema(format!("droop: {e}")))?;
    if !name_ok(&raw.domain) {
        return Err(schema(format!(
            "droop domain `{}` is not 1 to 64 printable ASCII bytes",
            raw.domain.escape_debug()
        )));
    }
    let values = [
        raw.floor_millivolts,
        raw.fall_ns,
        raw.hold_ns,
        raw.rise_ns,
        raw.source_milliohms,
        raw.load_milliamps,
    ];
    for ((key, lo, hi), v) in DROOP_BOUNDS.iter().zip(values) {
        if !(*lo..=*hi).contains(&v) {
            return Err(schema(format!(
                "droop `{key}` is not from {lo} to {hi}: {v}"
            )));
        }
    }
    let narrow = |v: u64| u32::try_from(v).expect("bounded above");
    Ok(Droop {
        from_boot_word_ns: offset_from("droop", raw.from_boot_word_ns)?,
        domain: raw.domain,
        floor_millivolts: narrow(raw.floor_millivolts),
        fall_ns: raw.fall_ns,
        hold_ns: raw.hold_ns,
        rise_ns: raw.rise_ns,
        source_milliohms: narrow(raw.source_milliohms),
        load_milliamps: narrow(raw.load_milliamps),
    })
}

fn battery_sag_from(v: serde_json::Value) -> Result<BatterySag, ScenarioError> {
    let raw: RawBatterySag =
        serde_json::from_value(v).map_err(|e| schema(format!("battery_sag: {e}")))?;
    let (lo, hi) = (1, crate::battery::SERIES_MILLIOHMS_MAX);
    if !(lo..=hi).contains(&raw.series_milliohms) {
        return Err(schema(format!(
            "battery_sag `series_milliohms` is not from {lo} to {hi}: {}",
            raw.series_milliohms
        )));
    }
    Ok(BatterySag {
        series_milliohms: u32::try_from(raw.series_milliohms).expect("bounded above"),
        from_boot_word_ns: offset_from("battery_sag", raw.from_boot_word_ns)?,
    })
}

fn pin_held_from(v: serde_json::Value) -> Result<Override, ScenarioError> {
    let raw: RawPinHeld =
        serde_json::from_value(v).map_err(|e| schema(format!("pin_held: {e}")))?;
    if !name_ok(&raw.net) {
        return Err(schema(format!(
            "pin_held net `{}` is not 1 to 64 printable ASCII bytes",
            raw.net.escape_debug()
        )));
    }
    Ok(Override::PinHeld {
        net: raw.net,
        level: raw.level,
    })
}

fn analog_value_from(v: serde_json::Value) -> Result<Override, ScenarioError> {
    let raw: RawAnalogValue =
        serde_json::from_value(v).map_err(|e| schema(format!("analog_value: {e}")))?;
    if !name_ok(&raw.net) {
        return Err(schema(format!(
            "analog_value net `{}` is not 1 to 64 printable ASCII bytes",
            raw.net.escape_debug()
        )));
    }
    match u32::try_from(raw.millivolts) {
        Ok(millivolts) if millivolts <= ANALOG_MAX_MILLIVOLTS => Ok(Override::AnalogValue {
            net: raw.net,
            millivolts,
        }),
        _ => Err(schema(format!(
            "analog_value `millivolts` is not from 0 to {ANALOG_MAX_MILLIVOLTS}: {}",
            raw.millivolts
        ))),
    }
}

/// The rail's time constant R·C in ns, rounded up.
pub fn tau_ns(source_milliohms: u32, decoupling_picofarads: u64) -> u64 {
    // mΩ · pF = 1e-15 s = 1e-6 ns.
    (u128::from(source_milliohms) * u128::from(decoupling_picofarads))
        .div_ceil(1_000_000)
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The rail's lowest and resting levels, in µV: the floor and the nominal,
/// each less the load's I·R drop. `None` when the drop reaches the floor.
pub fn rail_levels(droop: &Droop, nominal_millivolts: u32) -> Option<(u64, u64)> {
    let drop = droop.drop_microvolts();
    let floor = u64::from(droop.floor_millivolts) * 1000;
    let rest = u64::from(nominal_millivolts) * 1000;
    // A drop that takes the rail to 0 V is no rail at all: refused.
    Some((
        floor.checked_sub(drop).filter(|f| *f > 0)?,
        rest.checked_sub(drop)?,
    ))
}

/// The domain's tolerance floor in µV: its nominal less its tolerance.
pub fn tolerance_floor_uv(nominal_millivolts: u32, tolerance_permille: u32) -> u64 {
    u64::from(nominal_millivolts) * u64::from(1000u32.saturating_sub(tolerance_permille))
}

/// The time after which the rail does nothing more: its sag ends, the rail
/// settles (10 τ), BOR0's temporization runs, the release's nudge, and the
/// grid's rounding on both edges. The recovery bound is not in it: it is
/// read from the boot the twin ran (`AET-WO-2305` §1.2), and the probe it
/// places is checked against the run once it is known.
pub fn droop_end_ns(rail: &RailDroop, decoupling_picofarads: u64) -> u64 {
    let droop = &rail.droop;
    let settle = tau_ns(droop.source_milliohms, decoupling_picofarads).saturating_mul(10);
    [
        rail.start_ns,
        droop.fall_ns,
        droop.hold_ns,
        droop.rise_ns,
        settle,
        power::T_RSTTEMPO_NS,
        power::NUDGE_NS,
        2 * power::RESOLUTION_NS,
    ]
    .into_iter()
    .fold(0u64, u64::saturating_add)
}

/// A droop the runner can render, once it knows the domain's nominal and the
/// decoupling on it (ruled D3, D7): refused when its rail floor, or its
/// resting level for a threshold it crosses, lies inside that threshold's
/// min–max band, because the part's behaviour there is not stated; when the
/// resting rail is not above every active threshold, so the part would not
/// run at all; when the floor is held less than 10 τ, so the rail never
/// reaches it; and when its effects run past the boot's `span_ns`. The
/// instant and the level are the rendered rail's ([`RailDroop`]): the
/// instant placed from the undisturbed boot word, the level the
/// specification's (`AET-WO-2310`).
pub fn check_droop(
    rail: &RailDroop,
    nominal_millivolts: u32,
    decoupling_picofarads: u64,
    span_ns: u64,
) -> Result<(), ScenarioError> {
    check_droop_shape(
        &rail.droop,
        rail.bor_level,
        nominal_millivolts,
        decoupling_picofarads,
    )?;
    let end = droop_end_ns(rail, decoupling_picofarads);
    if end > span_ns {
        return Err(ScenarioError::new(
            "DroopRefused",
            Some(&rail.droop.domain),
            format!(
                "the droop's effects run to {end} ns, past the boot's {span_ns} ns (its instant {} ns, placed from the undisturbed boot word)",
                rail.start_ns
            ),
        ));
    }
    Ok(())
}

/// [`check_droop`] before the boot, when only the offset is known
/// (`AET-WO-2310`): every clause but the instant's, at the specification's
/// BOR level. The clause of its instant waits for the undisturbed boot word.
pub fn check_droop_shape(
    droop: &Droop,
    bor_level: u8,
    nominal_millivolts: u32,
    decoupling_picofarads: u64,
) -> Result<(), ScenarioError> {
    let refuse = |m: String| ScenarioError::new("DroopRefused", Some(&droop.domain), m);
    if droop.floor_millivolts >= nominal_millivolts {
        return Err(refuse(format!(
            "the floor {} mV is not below the domain's nominal {nominal_millivolts} mV",
            droop.floor_millivolts
        )));
    }
    if decoupling_picofarads == 0 {
        return Err(refuse("the domain has no decoupling".to_owned()));
    }
    let (floor, rest) = rail_levels(droop, nominal_millivolts)
        .ok_or_else(|| refuse("the load's I·R drop reaches the floor".to_owned()))?;
    let tau = tau_ns(droop.source_milliohms, decoupling_picofarads);
    if droop.hold_ns < tau.saturating_mul(10) {
        return Err(refuse(format!(
            "the floor is held {} ns, less than 10 τ ({} ns), so the rail does not reach it",
            droop.hold_ns,
            tau.saturating_mul(10)
        )));
    }
    for t in power::active(bor_level) {
        if power::band(rest, t.falling_uv) != Band::Above {
            return Err(refuse(format!(
                "the resting rail, {rest} µV, is not above {}'s falling band",
                t.name
            )));
        }
        // The rail's floor must lie clear of the threshold's whole hysteresis
        // region, from the falling band's minimum to the rising band's
        // maximum: inside it the part's behaviour is not stated, and near a
        // level ngspice computes, the rail's ringing dithers across it
        // (`AET-WO-2301` adversarial F4).
        let region = [t.falling_uv[0], 0, t.rising_uv[2]];
        if power::band(floor, region) == Band::Inside
            && power::band(floor, t.falling_uv) != Band::Inside
        {
            return Err(refuse(format!(
                "the rail floor, {floor} µV, lies inside {}'s hysteresis region {:?} µV, near the levels ngspice computes",
                t.name,
                [region[0], region[2]]
            )));
        }
        match power::band(floor, t.falling_uv) {
            Band::Inside => {
                return Err(refuse(format!(
                    "the rail floor, {floor} µV, lies inside {}'s falling band {:?} µV, where the part's behaviour is not stated",
                    t.name,
                    [t.falling_uv[0], t.falling_uv[2]]
                )));
            }
            Band::Below if power::band(rest, t.rising_uv) != Band::Above => {
                return Err(refuse(format!(
                    "the resting rail, {rest} µV, is not above {}'s rising band, so the release is not stated",
                    t.name
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

/// What an override sets, so two overrides of one thing are refused.
fn target(o: &Override) -> (u8, &str) {
    match o {
        Override::PullUpOhms { bus, .. } => (0, bus),
        Override::MaxCapacitance { net, .. } => (1, net),
        Override::LineHeldLow { bus, line } => (
            match line {
                Line::Sda => 2,
                Line::Scl => 3,
            },
            bus,
        ),
        // One droop per scenario, whatever its domain.
        Override::Droop(_) => (4, ""),
        Override::SpiDeviceAbsent { reference } => (5, reference),
        Override::SerialRxOpen { port } => (6, port),
        // One battery sag per scenario; refused as combined before this.
        Override::BatterySag(_) => (7, ""),
        Override::RailOutOfBand { domain } => (8, domain),
        Override::PinHeld { net, .. } => (9, net),
        Override::AnalogValue { net, .. } => (10, net),
    }
}

impl Scenario {
    /// Parse one scenario file's bytes. Unknown keys, a positional array, a
    /// duplicate key, a wrong type, an unknown version, a name outside
    /// `RUL-14`'s set, an expectation that contradicts itself and two
    /// overrides of one thing are each refused.
    pub fn parse(bytes: &[u8]) -> Result<Self, ScenarioError> {
        let raw: RawScenario = serde_json::from_slice(bytes).map_err(|e| {
            if e.classify() == serde_json::error::Category::Data {
                schema(e.to_string())
            } else {
                ScenarioError::new("InvalidJson", None, e.to_string())
            }
        })?;
        if !SCENARIO_VERSIONS.contains(&raw.schema_version) {
            return Err(ScenarioError::new(
                "UnsupportedSchemaVersion",
                None,
                format!(
                    "scenario schema_version {} is not one of {SCENARIO_VERSIONS:?}",
                    raw.schema_version
                ),
            ));
        }
        if !name_ok(&raw.name) {
            return Err(schema(format!(
                "scenario name `{}` is not 1 to 64 printable ASCII bytes",
                raw.name.escape_debug()
            )));
        }
        if raw.base.is_empty() {
            return Err(schema("scenario base is empty".to_owned()));
        }
        if raw.overrides.len() > MAX_OVERRIDES {
            return Err(schema(format!(
                "a scenario has at most {MAX_OVERRIDES} overrides"
            )));
        }
        if raw.expect_findings.len() > MAX_EXPECTED {
            return Err(schema(format!(
                "a scenario expects at most {MAX_EXPECTED} finding kinds"
            )));
        }
        let mut expect_findings = raw.expect_findings;
        if let Some(bad) = expect_findings.iter().find(|k| !name_ok(k)) {
            return Err(schema(format!(
                "expected finding kind `{}` is not 1 to 64 printable ASCII bytes",
                bad.escape_debug()
            )));
        }
        let given = expect_findings.len();
        expect_findings.sort();
        expect_findings.dedup();
        if expect_findings.len() != given {
            return Err(schema(
                "an expected finding kind is listed twice".to_owned(),
            ));
        }
        let expect_pass = raw.expect == Expect::Pass;
        if expect_pass != expect_findings.is_empty() {
            return Err(ScenarioError::new(
                "ExpectContradiction",
                Some(&raw.name),
                "`expect: pass` takes no expect_findings, and `expect: fail` names at least one"
                    .to_owned(),
            ));
        }
        let overrides = raw
            .overrides
            .into_iter()
            .map(|o| override_from(o, raw.schema_version))
            .collect::<Result<Vec<_>, _>>()?;
        let mut expect_counts = raw.expect_counts.unwrap_or_default();
        let v2_only = overrides
            .iter()
            .any(|o| matches!(o, Override::Droop(_) | Override::SpiDeviceAbsent { .. }))
            || !expect_counts.is_empty();
        if v2_only && raw.schema_version < 2 {
            return Err(schema(
                "a droop override, an absent SPI device and expect_counts need schema_version 2"
                    .to_owned(),
            ));
        }
        let v3_only = overrides
            .iter()
            .any(|o| matches!(o, Override::SerialRxOpen { .. }));
        if v3_only && raw.schema_version < 3 {
            return Err(schema(
                "an open serial RX line needs schema_version 3".to_owned(),
            ));
        }
        let sags = overrides
            .iter()
            .filter(|o| matches!(o, Override::BatterySag(_)))
            .count();
        let v4_only = sags > 0
            || overrides
                .iter()
                .any(|o| matches!(o, Override::RailOutOfBand { .. }));
        if v4_only && raw.schema_version < 4 {
            return Err(schema(
                "a battery sag and a rail out of band need schema_version 4".to_owned(),
            ));
        }
        let v5_only = overrides
            .iter()
            .any(|o| matches!(o, Override::PinHeld { .. } | Override::AnalogValue { .. }));
        if v5_only && raw.schema_version < 5 {
            return Err(schema(
                "a pin_held or analog_value override needs schema_version 5".to_owned(),
            ));
        }
        // §1.6: two sags, or a sag with a droop, interact in ways the chain
        // does not model.
        let droops = overrides
            .iter()
            .filter(|o| matches!(o, Override::Droop(_)))
            .count();
        if sags > 1 || (sags == 1 && droops > 0) {
            return Err(ScenarioError::new(
                crate::battery::REFUSED,
                Some(&raw.name),
                "combined: more than one battery_sag, or a battery_sag with a droop, in one scenario; their interaction is not modelled".to_owned(),
            ));
        }
        // §1.7: one rail out of band, rendered by its consumers' absences and
        // nothing else (adversarial 1 and 3), so every clause judges it.
        check_rail_out_of_band(&overrides)
            .map_err(|m| ScenarioError::new(RAIL_REFUSED, Some(&raw.name), m.to_owned()))?;
        if expect_counts.len() > MAX_EXPECTED {
            return Err(schema(format!(
                "a scenario expects at most {MAX_EXPECTED} counts"
            )));
        }
        if let Some((bad, _)) = expect_counts.iter().find(|(k, _)| !name_ok(k)) {
            return Err(schema(format!(
                "expect_counts kind `{}` is not 1 to 64 printable ASCII bytes",
                bad.escape_debug()
            )));
        }
        expect_counts.sort();
        let mut seen = BTreeSet::new();
        for o in &overrides {
            if !seen.insert(target(o)) {
                return Err(ScenarioError::new(
                    "DuplicateOverride",
                    Some(target(o).1),
                    format!("two overrides set the same thing: {o:?}"),
                ));
            }
        }
        Ok(Self {
            name: raw.name,
            base: raw.base,
            expect_pass,
            expect_findings,
            overrides,
            expect_counts,
            sha256: Sha256::digest(bytes).into(),
        })
    }

    /// The scenario's battery sag, if it declares one.
    pub fn battery_sag(&self) -> Option<&BatterySag> {
        self.overrides.iter().find_map(|o| match o {
            Override::BatterySag(b) => Some(b),
            _ => None,
        })
    }

    /// The rail the scenario holds out of its band, if it declares one.
    pub fn rail_out_of_band(&self) -> Option<&str> {
        self.overrides.iter().find_map(|o| match o {
            Override::RailOutOfBand { domain } => Some(domain.as_str()),
            _ => None,
        })
    }

    /// The nets the scenario holds at a level, in the file's order
    /// (`AET-WO-1211c`).
    pub fn pins_held(&self) -> Vec<(&str, Level)> {
        self.overrides
            .iter()
            .filter_map(|o| match o {
                Override::PinHeld { net, level } => Some((net.as_str(), *level)),
                _ => None,
            })
            .collect()
    }

    /// The nets the scenario drives at a voltage, in mV, in the file's order
    /// (`AET-WO-1211c`).
    pub fn analog_values(&self) -> Vec<(&str, u32)> {
        self.overrides
            .iter()
            .filter_map(|o| match o {
                Override::AnalogValue { net, millivolts } => Some((net.as_str(), *millivolts)),
                _ => None,
            })
            .collect()
    }

    /// The scenario's droop, if it declares one.
    pub fn droop(&self) -> Option<&Droop> {
        self.overrides.iter().find_map(|o| match o {
            Override::Droop(d) => Some(d),
            _ => None,
        })
    }

    /// Kind 20, when the scenario declares `expect_counts`.
    pub fn counts_record(&self) -> Option<Record> {
        (!self.expect_counts.is_empty()).then(|| Record::ExpectedCounts {
            counts: self.expect_counts.clone(),
        })
    }

    /// The scenario's record, the first of its run.
    pub fn record(&self) -> Record {
        Record::Scenario {
            name: self.name.clone(),
            sha256: self.sha256,
            expect_pass: self.expect_pass,
            expect_findings: self.expect_findings.clone(),
            overrides: self.overrides.clone(),
        }
    }
}

/// A run's id: a function of what it judged, so a second campaign over the
/// same inputs names the same run (and an archive refuses it as existing).
pub fn run_id(scenario: &[u8; 32], specification: &[u8; 32], firmware: &[u8; 32]) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"aethelgard-run\0");
    h.update(scenario);
    h.update(specification);
    h.update(firmware);
    let digest: [u8; 32] = h.finalize().into();
    digest[..16].try_into().expect("16 of 32 bytes")
}

/// What `replay` reports of a campaign run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSummary {
    pub name: String,
    pub expect_pass: bool,
    pub expect_findings: Vec<String>,
    pub pass: bool,
    /// The verdict's finding kinds above `info`, sorted and unique.
    pub failing_kinds: Vec<String>,
    /// The verdict is the one expected, for the reasons expected.
    pub matched: bool,
}

/// Judge a campaign run from its records alone: their order, and that every
/// fault, measurement and verdict agrees with the evidence beside it. A run
/// that fails any of these is refused with the reason.
///
/// The order is: the scenario, its expected counts (kind 20, if any), the
/// solved topology, the solved I2C buses (kind 15, sorted by bus), the analog
/// records (kinds 7 and 8), the bus faults, the declared rail (kind 16), the
/// droop's crossings, resets and observations (kinds 17 to 19) in time
/// order: the boot's witnesses (kind 10, then 11, 12, 13, 21, 22, 26, 28 and
/// 29 in that order), and the verdict, last and once. The solved motor
/// outputs (kind 24) and their static checks (kind 25) sit with the solved
/// buses (`AET-WO-4300a`), and so do the solved serial ports (kind 27,
/// `AET-WO-4300b`) and a pin device's declared GPIO and ADC signals (kinds
/// 33 and 35, `AET-WO-1211c`). Its witnesses (kinds 34 and 36) and a level
/// output's (kind 37) sit with the motor witnesses (kind 26), in any order
/// among them.
pub fn check_run(records: &[Record]) -> Result<RunSummary, &'static str> {
    fn rank(r: &Record) -> u8 {
        match r {
            Record::Scenario { .. } => 0,
            Record::ExpectedCounts { .. } => 1,
            Record::Solved { .. } => 2,
            Record::I2cBusSolved { .. }
            | Record::SpiBusSolved { .. }
            | Record::MotorOutputSolved { .. }
            | Record::MotorTimingCheck { .. }
            | Record::SerialPortSolved { .. }
            | Record::PinGpioDeclared { .. }
            | Record::PinAdcDeclared { .. } => 3,
            Record::RiseComputed { .. } | Record::LineStuckHigh { .. } => 4,
            Record::BusFault { .. } => 5,
            Record::RailDeclared { .. }
            | Record::BatterySagDeclared { .. }
            | Record::RecoveryBound { .. } => 6,
            Record::RailCrossing { .. }
            | Record::ResetRendered { .. }
            | Record::FirmwareObserved { .. } => 7,
            Record::BootObserved { .. } => 8,
            Record::RegisterObserved { .. } => 9,
            Record::I2cCheck { .. } => 10,
            Record::I2cWitness { .. } => 11,
            Record::SpiCheck { .. } => 12,
            Record::SpiWitness { .. } => 13,
            Record::MotorWitness { .. }
            | Record::PinGpioWitness { .. }
            | Record::PinAdcWitness { .. }
            | Record::MotorLevelWitness { .. } => 14,
            Record::JudgeStop { .. } => 15,
            Record::SerialFramingCheck { .. } => 16,
            Record::SerialWitness { .. } => 17,
            Record::Verdict { .. } => 18,
        }
    }
    /// Within rank 7: time in ps, then crossing, reset, observation.
    fn when(r: &Record) -> (u64, u8) {
        match r {
            Record::RailCrossing { at_ps, .. } => (*at_ps, 0),
            Record::ResetRendered { at_ns, .. } => (at_ns.saturating_mul(1000), 1),
            Record::FirmwareObserved { at_ns, .. } => (at_ns.saturating_mul(1000), 2),
            _ => (0, 0),
        }
    }
    let count_rank = |n: u8| records.iter().filter(|r| rank(r) == n).count();
    let ordered = records.windows(2).all(|w| {
        let (a, b) = (rank(&w[0]), rank(&w[1]));
        a < b || (a == b && (a != 7 || when(&w[0]) <= when(&w[1])))
    });
    if records.is_empty() || !ordered {
        return Err("the records are not in the campaign's order");
    }
    if !(count_rank(0) == 1 && count_rank(2) == 1 && count_rank(8) == 1 && count_rank(18) == 1) {
        return Err("a run holds exactly one scenario, topology, boot and verdict");
    }
    // Kind 32 shares rank 6 after the rail; it is held to once by
    // `check_droop_records` (`AET-WO-2305` §1.2).
    let declared = records
        .iter()
        .filter(|r| {
            matches!(
                r,
                Record::RailDeclared { .. } | Record::BatterySagDeclared { .. }
            )
        })
        .count();
    if count_rank(1) > 1 || declared > 1 {
        return Err(
            "a run holds at most one declared rail or battery sag and one set of expected counts",
        );
    }
    // The scenario's own shape before any join: a rail out of band names
    // the consumers it renders (`AET-WO-2304b` §1.7).
    if let Record::Scenario { overrides, .. } = &records[0] {
        check_rail_out_of_band(overrides)?;
    }
    let booted = records.iter().any(|r| {
        matches!(r, Record::BootObserved { boot_word, .. }
            if boot_word.observed == Some(boot_word.expected))
    });
    check_read_with_the_boot(records)?;
    check_joins(records, booted)?;
    check_serial_joins(records, booted)?;
    let Record::Scenario {
        name,
        expect_pass,
        expect_findings,
        overrides,
        ..
    } = &records[0]
    else {
        unreachable!("ranked first");
    };
    let Some(Record::Verdict { pass, findings }) = records.last() else {
        unreachable!("ranked last");
    };

    // Each line that faults its bus, by (bus, net), and how many of each finding they need.
    let mut causes: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut needed: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::RiseComputed {
                bus,
                net,
                exceeds: true,
                ..
            } => {
                causes.insert((bus, net));
                *needed.entry(PAST_CEILING_FINDING).or_default() += 1;
            }
            Record::LineStuckHigh { bus, net, .. } => {
                causes.insert((bus, net));
                *needed.entry(STUCK_HIGH_FINDING).or_default() += 1;
            }
            _ => {}
        }
    }
    // Each held-low override's net: its line of its solved bus, SCL first
    // (`AET-WO-3201` B1: the fault binds that line, and no other).
    let held_nets: BTreeSet<(&str, &str)> = overrides
        .iter()
        .filter_map(|o| match o {
            Override::LineHeldLow { bus, line } => records.iter().find_map(|r| match r {
                Record::I2cBusSolved { bus: b, lines, .. } if b == bus => {
                    let net = &lines[usize::from(matches!(line, Line::Sda))].net;
                    Some((bus.as_str(), net.as_str()))
                }
                _ => None,
            }),
            _ => None,
        })
        .collect();
    let mut faulted: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut held_faults = 0u64;
    for r in records {
        let Record::BusFault {
            bus, from, nets, ..
        } = r
        else {
            continue;
        };
        if from.iter().any(|f| f == HELD_LOW) {
            if from.len() != 1
                || nets.len() != 1
                || !held_nets.contains(&(bus.as_str(), nets[0].as_str()))
            {
                return Err("a held-low fault names no held-low override of its bus");
            }
            held_faults += 1;
        } else {
            for net in nets {
                if !causes.contains(&(bus.as_str(), net.as_str())) {
                    return Err("a bus fault names a net with no measurement past its ceiling");
                }
                faulted.insert((bus, net));
            }
        }
    }
    if faulted != causes {
        return Err("a measurement past its ceiling faulted no bus");
    }
    let held_lines = overrides
        .iter()
        .filter(|o| matches!(o, Override::LineHeldLow { .. }))
        .count() as u64;
    if held_faults != held_lines {
        return Err("a held-low override faulted no bus, or one bus twice");
    }
    if held_faults > 0 {
        needed.insert(HELD_LOW_FINDING, held_faults);
    }
    // Each line of each solved bus is bound by exactly one record: its
    // computed rise (kind 7), its stuck-high record (kind 8), or, for a
    // line a scenario holds low, which the analog pass never measures, its
    // held-low fault (kind 9). A run that dropped a line's measurement is
    // refused here, never replayed as a pass (`AET-WO-3201` B1).
    for r in records {
        let Record::I2cBusSolved { bus, lines, .. } = r else {
            continue;
        };
        for line in lines {
            let bound = records
                .iter()
                .filter(|x| match x {
                    Record::RiseComputed { bus: b, net, .. }
                    | Record::LineStuckHigh { bus: b, net, .. } => b == bus && *net == line.net,
                    Record::BusFault {
                        bus: b, from, nets, ..
                    } => b == bus && from.iter().any(|f| f == HELD_LOW) && nets.contains(&line.net),
                    _ => false,
                })
                .count();
            if bound != 1 {
                return Err(
                    "a solved line has no computed rise, stuck-high record or held-low fault, or more than one",
                );
            }
        }
    }
    let count = |kind: &str| -> u64 {
        findings
            .iter()
            .filter(|(k, s, _)| k == kind && s != "info")
            .map(|(_, _, n)| *n)
            .sum()
    };
    for kind in [PAST_CEILING_FINDING, STUCK_HIGH_FINDING, HELD_LOW_FINDING] {
        if count(kind) != needed.get(kind).copied().unwrap_or(0) {
            return Err("the verdict's analog and held-low findings do not match the records");
        }
    }
    let any_severity = |kind: &str| -> u64 {
        findings
            .iter()
            .filter(|(k, _, _)| k == kind)
            .map(|(_, _, n)| *n)
            .sum()
    };
    // The boot's `pc` against the segments kind 10 archives: the verdict
    // names a core outside the firmware exactly when they place it there.
    let outside = records.iter().any(|r| {
        matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
            if !record::inside(executable, *pc))
    });
    // At any severity: the judge makes it an error or not at all
    // (`AET-WO-2205` adversarial 2).
    if any_severity(PC_OUTSIDE_FINDING) != u64::from(outside)
        || count(PC_OUTSIDE_FINDING) != u64::from(outside)
    {
        return Err("the verdict's pc_outside_firmware does not match the boot's pc and segments");
    }
    // The judge looks for the self-loop only inside the firmware
    // (`AET-WO-2205` adversarial 3).
    let read_inside = records.iter().any(|r| {
        matches!(r, Record::BootObserved { pc: Some(pc), executable, .. }
            if record::inside(executable, *pc))
    });
    if any_severity(SELF_LOOP_FINDING) > 0 && !read_inside {
        return Err(
            "the verdict names pc_not_on_self_loop for a pc outside the firmware or unread",
        );
    }
    // The brown-out word and the flash controller (`AET-WO-4230` D10): once
    // the boot word is set, each finding exactly when kind 10 shows it, as
    // an error; before it, none.
    let (brown, flash) = records
        .iter()
        .find_map(|r| match r {
            Record::BootObserved {
                brown_out,
                flash_access,
                ..
            } => Some((
                booted && brown_out_fails(brown_out),
                booted && flash_access.as_deref() != Some(&[]),
            )),
            _ => None,
        })
        .unwrap_or_default();
    for (kind, want) in [(BROWN_OUT_FINDING, brown), (FLASH_ACCESS_FINDING, flash)] {
        if any_severity(kind) != u64::from(want) || count(kind) != u64::from(want) {
            return Err(
                "the verdict's brown-out and flash controller findings do not match the boot's",
            );
        }
    }
    // The boot word as the judge names it (`AET-WO-3201` B2): a boot that
    // was read names `boot_word_missing` exactly when its word is not the
    // firmware's, as an error; a boot that read nothing was killed at a
    // bound or did not load, and names why, once (adversarial F1).
    let read = records.iter().any(
        |r| matches!(r, Record::BootObserved { boot_word, .. } if boot_word.observed.is_some()),
    );
    let missing = u64::from(read && !booted);
    if any_severity(BOOT_WORD_FINDING) != missing || count(BOOT_WORD_FINDING) != missing {
        return Err("the verdict's boot_word_missing does not match the boot word");
    }
    let unread: u64 = UNREAD_FINDINGS.iter().map(|k| any_severity(k)).sum();
    let unread_failing: u64 = UNREAD_FINDINGS.iter().map(|k| count(k)).sum();
    if unread != u64::from(!read) || unread_failing != unread {
        return Err(
            "a boot that read nothing does not name once why (a bound or a load error), or a read one does",
        );
    }
    // A booted core that slept (`AET-WO-2310`, overturning `AET-WO-2100`'s
    // ruled A10): the verdict names `cpu_halted` once, as an error, exactly
    // when kind 10 counts fewer instructions than the run holds.
    if booted {
        let halted = records.iter().any(|r| {
            matches!(r, Record::BootObserved { instructions_observed: Some(n), instructions_expected, .. }
                if n < instructions_expected)
        });
        let want = u64::from(halted);
        if any_severity(HALTED_FINDING) != want || count(HALTED_FINDING) != want {
            return Err(
                "the verdict's cpu_halted does not match the booted core's instruction count",
            );
        }
    }
    // A record is judged only at the fixed point: without the boot word, no
    // finding of a record's value, validity or witness (adversarial F2), as
    // for the motor outputs and serial ports below.
    if !booted && BOOTED_READ_FINDINGS.iter().any(|k| any_severity(k) > 0) {
        return Err("the verdict judges an identity read's record of a boot without its boot word");
    }
    // An `ok` read whose own witness is unread (`AET-WO-2310`, `2309` §5): at
    // the fixed point the judge names `{bus}_transaction_unwitnessed` for
    // each, so a witness blanked without its finding is refused.
    if booted {
        for (bus, kind) in [("i2c", UNWITNESSED_I2C), ("spi", UNWITNESSED_SPI)] {
            let unwitnessed = unread_ok_witnesses(records, bus);
            if count(kind) < unwitnessed {
                return Err(
                    "an ok read's witness is unread and the verdict does not name it unwitnessed",
                );
            }
        }
    }
    // The motor outputs (`AET-WO-4300a`): the verdict names a static
    // mismatch once per check that fails, and, once the boot word is set,
    // each witness's finding by its status, exactly; before it, none.
    let mut motor_needed: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::MotorTimingCheck { holds: false, .. } => {
                *motor_needed.entry(MOTOR_STATIC_FINDING).or_default() += 1;
            }
            Record::MotorWitness { status, .. } if booted => {
                let kind = match status.as_str() {
                    "unwitnessed" | "unreadable" => Some(MOTOR_UNWITNESSED_FINDING),
                    "mismatch" | "batched" | "stopped" => Some(MOTOR_MISMATCH_FINDING),
                    "static_only" => Some(MOTOR_STATIC_ONLY_FINDING),
                    "unanchored" => Some(MOTOR_ANCHOR_UNOBSERVED_FINDING),
                    // Judged by its kind 37 instead (`AET-WO-1211c`).
                    "level" => None,
                    _ => None,
                };
                if let Some(kind) = kind {
                    *motor_needed.entry(kind).or_default() += 1;
                }
            }
            Record::MotorLevelWitness { status, .. } if booted && status != "witnessed" => {
                *motor_needed.entry(MOTOR_LEVEL_FINDING).or_default() += 1;
            }
            _ => {}
        }
    }
    // Each DShot timer's window and stop (`AET-WO-2302`), from the records
    // once the boot word is set: a window past the boot, or an anchored
    // window inside it with no judge's stop, each named once per timer; a
    // stop that ran, cleared `CEN` and read a register back off its solved
    // value, once per such register (and once for registers it did not
    // read); a timer whose `CEN` rose more than once, once; and each
    // output's edges before its anchor or after its window, once each
    // (adversarial pass 1, L3).
    if booted {
        for (kind, n) in dshot_timer_findings(records)
            .into_iter()
            .chain(anchor_record_findings(records))
        {
            *motor_needed.entry(kind).or_default() += n;
        }
    }
    for kind in MOTOR_FINDINGS {
        if any_severity(kind) != motor_needed.get(kind).copied().unwrap_or(0) {
            return Err("the verdict's motor findings do not match the motor records");
        }
    }
    if findings.iter().any(|(k, s, _)| {
        (k == MOTOR_STATIC_ONLY_FINDING) != (s == "info") && MOTOR_FINDINGS.contains(&k.as_str())
    }) {
        return Err("a motor finding has the wrong severity");
    }
    // The serial ports (`AET-WO-4300b`), as the judge decides them: a
    // failing silicon check always; once the boot word is set, a port's tags
    // (kind 28) and its witness (kind 29), exactly. A capture directory
    // holding what no port's backend wrote is forged too and names no port,
    // so the records bound `serial_capture_forged` from below only; with no
    // port there is no capture directory, and none.
    let serial = serial_findings(records, booted);
    for kind in SERIAL_FINDINGS {
        let (have, want) = (any_severity(kind), serial.get(kind).copied().unwrap_or(0));
        let ports = records
            .iter()
            .any(|r| matches!(r, Record::SerialPortSolved { .. }));
        let ok = if kind == SERIAL_FORGED_FINDING && ports {
            have >= want
        } else {
            have == want
        };
        if !ok {
            return Err("the verdict's serial findings do not match the serial records");
        }
    }
    if findings
        .iter()
        .any(|(k, s, _)| s == "info" && SERIAL_FINDINGS.contains(&k.as_str()))
    {
        return Err("a serial finding has the wrong severity");
    }
    // What the boot word's instant decides (`AET-WO-2302` adversarial pass
    // 2, A2-5), once the boot word is set; before it, neither is named.
    let wire = if booted {
        wire_past_run_ports(records).map_or((0, 1), |p| {
            let n = u64::from(!p.is_empty());
            (n, n)
        })
    } else {
        (0, 0)
    };
    let order = if booted {
        anchor_order_bounds(records)
    } else {
        (0, 0)
    };
    for (kind, (least, most)) in [
        (SERIAL_WIRE_PAST_RUN_FINDING, wire),
        (MOTOR_ANCHOR_ORDER_FINDING, order),
    ] {
        let n = any_severity(kind);
        if n < least || n > most || count(kind) != n {
            return Err(
                "the verdict's serial_wire_past_run or motor_anchor_out_of_order does not match the boot word's instant",
            );
        }
    }
    check_battery_sag_records(records, overrides)?;
    // The pin devices (`AET-WO-1211c`): the joins first, then the verdict's
    // GPIO and ADC findings, each exactly its records' count and an error.
    let pins = check_pin_device_records(records, overrides, booted)?;
    for kind in PIN_DEVICE_FINDINGS {
        if any_severity(kind) != pins.get(kind).copied().unwrap_or(0)
            || count(kind) != any_severity(kind)
        {
            return Err("the verdict's GPIO and ADC findings do not match the pin device records");
        }
    }
    let droop_counts = check_droop_records(records, overrides)?;
    for (kind, want) in droop_counts {
        if any_severity(kind) != want {
            return Err("the verdict's droop findings do not match the droop's records");
        }
    }
    if findings
        .iter()
        .any(|(k, s, _)| (k == RESET_FINDING || k == BELOW_TOLERANCE_FINDING) && s != "info")
        || findings
            .iter()
            .any(|(k, s, _)| k == UNRECOVERED_FINDING && s == "info")
    {
        return Err("a droop finding has the wrong severity");
    }
    let expected_counts: &[(String, u64)] = records
        .iter()
        .find_map(|r| match r {
            Record::ExpectedCounts { counts } => Some(counts.as_slice()),
            _ => None,
        })
        .unwrap_or(&[]);
    let counts_match = expected_counts
        .iter()
        .all(|(kind, n)| any_severity(kind) == *n);
    let failing_kinds: Vec<String> = findings
        .iter()
        .filter(|(_, s, _)| s != "info")
        .map(|(k, _, _)| k.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // A run matches when it fails for exactly the kinds named, no more and no
    // fewer, `info` never counting (`AET-WO-2300` §5.4). The records' own rules
    // make that the whole test: a passing verdict has no finding above `info`,
    // and a passing scenario names none.
    let matched = failing_kinds == *expect_findings && counts_match;
    Ok(RunSummary {
        name: name.clone(),
        expect_pass: *expect_pass,
        expect_findings: expect_findings.clone(),
        pass: *pass,
        failing_kinds,
        matched,
    })
}

/// Kind 10's "all read or all unread" across the boot's reads (`AET-WO-2309`,
/// from `AET-WO-3201` adversarial 5.8): the judge reads every solved register
/// (kind 11) and every identity read's record (kinds 12 and 21, its value and
/// its status) in the read that reads the boot, so each is read exactly when
/// the boot is. A witness's log lines and its attribution evidence (kinds 13
/// and 22) are unread when the boot is; a read boot may still hold a witness
/// unread, when Renode's log passed the filter's budget (`AET-WO-2204` D2).
/// And what only a boot Renode read can give, its motor, serial, pin and
/// level witnesses and its judge's stops (kinds 26, 29, 30, 34, 36 and 37),
/// a boot that read nothing never holds (ruled D3).
fn check_read_with_the_boot(records: &[Record]) -> Result<(), &'static str> {
    let read = records.iter().any(
        |r| matches!(r, Record::BootObserved { boot_word, .. } if boot_word.observed.is_some()),
    );
    let together = records.iter().all(|r| match r {
        Record::RegisterObserved { reading, .. } => reading.observed.is_some() == read,
        Record::I2cCheck {
            observed, status, ..
        }
        | Record::SpiCheck {
            observed, status, ..
        } => observed.is_some() == read && status.is_some() == read,
        Record::I2cWitness {
            observed, absent, ..
        } => read || (observed.is_none() && absent.is_empty()),
        Record::SpiWitness {
            observed, unrouted, ..
        } => read || (observed.is_none() && unrouted.is_empty()),
        _ => true,
    });
    if !together {
        return Err(
            "the boot's registers, identity reads and witnesses are not all read or all unread with its boot word",
        );
    }
    let renode_read = records.iter().any(|r| {
        matches!(
            r,
            Record::MotorWitness { .. }
                | Record::SerialWitness { .. }
                | Record::JudgeStop { .. }
                | Record::PinGpioWitness { .. }
                | Record::PinAdcWitness { .. }
                | Record::MotorLevelWitness { .. }
        )
    });
    if !read && renode_read {
        return Err(
            "a boot that read nothing holds a motor, serial, pin or level witness, or a judge's stop",
        );
    }
    Ok(())
}

/// `AET-WO-3100` R2: every bus, net, device and controller the other records
/// name is joined by exactly one solved bus (kind 15), so a dropped or forged
/// join is refused here and never reaches attribution as a guess.
/// Each observation's readings are the boot's own (`AET-WO-2301` adversarial
/// F5): the boot word's address and expectation and the identity read's
/// expected value, each at the mask the boot record judges it by; the
/// record's status and value are full words. An observation's witness lines
/// are judged against the first identity read's witness record (kind 13) and
/// exist exactly when it does (`AET-WO-2204`). Replay trusts no expectation
/// an observation could carry for itself.
fn check_observed_against_the_boot(records: &[Record]) -> Result<(), &'static str> {
    let boot = records.iter().find_map(|r| match r {
        Record::BootObserved { boot_word, .. } => Some(boot_word),
        _ => None,
    });
    // The first identity read: the first I2C one, or the first SPI one on a
    // board with none, as the checks are ordered (`AET-WO-4200` adversarial A1).
    let check = records
        .iter()
        .find_map(|r| match r {
            Record::I2cCheck { expected, .. } => Some(*expected),
            _ => None,
        })
        .or_else(|| {
            records.iter().find_map(|r| match r {
                Record::SpiCheck { expected, .. } => Some(*expected),
                _ => None,
            })
        });
    let has_witness = !first_script(records).is_empty();
    let same = |a: &Reading, b: &Reading| {
        (a.address, a.mask, a.expected) == (b.address, b.mask, b.expected)
    };
    for r in records {
        let Record::FirmwareObserved {
            boot_word,
            status,
            value,
            witness,
            ..
        } = r
        else {
            continue;
        };
        let ok = boot.is_some_and(|b| same(boot_word, b))
            && status.mask == u32::MAX
            && value.mask == u32::MAX
            && check.is_none_or(|v| u64::from(value.expected) == v)
            && (has_witness || witness.is_none());
        if !ok {
            return Err("an observation's readings are not the boot's own");
        }
    }
    Ok(())
}

/// The first identity read's device's steps' expected lines, in the
/// archive's (the script's) order: the first kind 13's device, or with no
/// I2C read the first kind 22's, which a droop's observation must show since
/// its release (`AET-WO-2305` §1.3). Empty without one.
fn first_script(records: &[Record]) -> Vec<&[String]> {
    let i2c = records.iter().filter_map(|r| match r {
        Record::I2cWitness {
            device, expected, ..
        } => Some((device, expected.as_slice())),
        _ => None,
    });
    let spi = records.iter().filter_map(|r| match r {
        Record::SpiWitness {
            device, expected, ..
        } => Some((device, expected.as_slice())),
        _ => None,
    });
    let steps: Vec<(&String, &[String])> = match i2c.clone().next() {
        Some(_) => i2c.collect(),
        None => spi.collect(),
    };
    let Some((first, _)) = steps.first().copied() else {
        return Vec::new();
    };
    steps
        .into_iter()
        .filter(|(d, _)| *d == first)
        .map(|(_, e)| e)
        .collect()
}

/// A reading as the judge sees it: observed, and equal to the expected
/// value under the mask.
fn reads_as_expected(r: &Reading) -> bool {
    r.observed
        .is_some_and(|o| o & r.mask == r.expected & r.mask)
}

/// The undisturbed boot word a planted run's instants are placed from
/// (`AET-WO-2310`, ruled Q-A (a)): a run with a droop or a battery sag holds
/// exactly one kind 32, and it holds the word (a run whose undisturbed boot
/// wrote none is refused by name before it is archived); a run with neither
/// holds no kind 32.
fn planted_reference(
    records: &[Record],
    overrides: &[Override],
) -> Result<Option<u64>, &'static str> {
    let planted = overrides
        .iter()
        .any(|o| matches!(o, Override::Droop(_) | Override::BatterySag(_)));
    let words: Vec<Option<u64>> = records
        .iter()
        .filter_map(|r| match r {
            Record::RecoveryBound { boot_word_ns, .. } => Some(*boot_word_ns),
            _ => None,
        })
        .collect();
    match (planted, words.as_slice()) {
        (false, []) => Ok(None),
        (true, [Some(word)]) => Ok(Some(*word)),
        (true, [None]) => Err("a planted run's reference holds no undisturbed boot word"),
        _ => Err(
            "a planted run does not hold exactly one kind 32, or a run with none planted holds one",
        ),
    }
}

/// The BOR level kind 10's brown-out reading declares: its expected word
/// under its own mask, shifted down to the field (`AET-WO-2310`). `None` when
/// the reading is not a field's value.
pub fn declared_bor_level(brown_out: &Reading) -> Option<u8> {
    let shift = brown_out.mask.trailing_zeros();
    (brown_out.mask != 0 && brown_out.expected & !brown_out.mask == 0)
        .then(|| brown_out.expected >> shift)
        .and_then(|v| u8::try_from(v).ok())
}

/// The droop's records against the scenario and against each other, and the
/// counts of the droop's findings they require (`AET-WO-2301` §2.2, §2.3, §5):
/// - kinds 16 to 19 appear only with a droop override, and kind 16 then
///   exactly once, carrying the override's own values;
/// - each active threshold's level is crossed exactly when the declared rail
///   floor is below it: once falling while the source falls, once rising
///   while it rises (within 10 τ of each ramp), at the table's typical value;
/// - the resets are exactly [`power::schedule`] of those crossings, each
///   with its fixed rendering;
/// - the rail's instant is the undisturbed boot word (kind 32, which every
///   planted run holds, [`planted_reference`]) plus the scenario's offset,
///   and its BOR level the one kind 10 declares (`AET-WO-2310`);
/// - each probe lies inside the run (`AET-WO-2305` §1.2);
/// - each release has exactly one observation, at its probe (the release, its
///   nudge and the bound),
///   unless the boot was killed before its reads (its boot record observed
///   nothing), when there is none and every release counts as unrecovered.
fn check_droop_records(
    records: &[Record],
    overrides: &[Override],
) -> Result<[(&'static str, u64); 3], &'static str> {
    let droop = overrides.iter().find_map(|o| match o {
        Override::Droop(d) => Some(d),
        _ => None,
    });
    let declared = records.iter().find_map(|r| match r {
        Record::RailDeclared {
            droop,
            nominal_millivolts,
            tolerance_permille,
            decoupling_picofarads,
        } => Some((
            droop,
            *nominal_millivolts,
            *tolerance_permille,
            *decoupling_picofarads,
        )),
        _ => None,
    });
    let crossings: Vec<power::Crossing> = records
        .iter()
        .filter_map(|r| match r {
            Record::RailCrossing {
                level,
                microvolts,
                at_ps,
                falling,
                ..
            } => Some(power::Crossing {
                level: level.clone(),
                microvolts: *microvolts,
                at_ps: *at_ps,
                falling: *falling,
            }),
            _ => None,
        })
        .collect();
    let resets: Vec<(u64, bool, &[String], &str)> = records
        .iter()
        .filter_map(|r| match r {
            Record::ResetRendered {
                at_ns,
                released,
                cause,
                rendering,
            } => Some((*at_ns, *released, cause.as_slice(), rendering.as_str())),
            _ => None,
        })
        .collect();
    let script = first_script(records);
    let reference = planted_reference(records, overrides)?;
    let observed: Vec<(u64, bool)> = records
        .iter()
        .filter_map(|r| match r {
            Record::FirmwareObserved {
                at_ns,
                boot_word,
                status,
                value,
                witness,
            } => Some((
                *at_ns,
                [boot_word, status, value]
                    .iter()
                    .all(|r| reads_as_expected(r))
                    && match witness {
                        // No identity read: nothing to witness, vacuously.
                        None => script.is_empty(),
                        Some(w) => power::witness_recovers(&script, w),
                    },
            )),
            _ => None,
        })
        .collect();
    check_observed_against_the_boot(records)?;
    let (droop, (rail, nominal, tolerance, decoupling)) = match (droop, declared) {
        (None, None) => {
            if !(crossings.is_empty() && resets.is_empty() && observed.is_empty()) {
                return Err("a run without a droop holds a droop's records");
            }
            return Ok([
                (RESET_FINDING, 0),
                (UNRECOVERED_FINDING, 0),
                (BELOW_TOLERANCE_FINDING, 0),
            ]);
        }
        (Some(d), Some(r)) => (d, r),
        _ => return Err("a droop override and its declared rail do not come together"),
    };
    if rail.droop != *droop {
        return Err("the declared rail is not the scenario's droop");
    }
    // Its instant, placed from the undisturbed boot word (ruled Q-A (a)), and
    // its level, the specification's, which kind 10 declares (`AET-WO-2310`).
    let word = reference.ok_or("a droop's run holds no undisturbed boot word")?;
    if record::resolve_instant(word, droop.from_boot_word_ns) != Some(rail.start_ns) {
        return Err(
            "the declared rail's instant is not the undisturbed boot word plus the scenario's offset",
        );
    }
    let level = records.iter().find_map(|r| match r {
        Record::BootObserved { brown_out, .. } => declared_bor_level(brown_out),
        _ => None,
    });
    if level != Some(rail.bor_level) {
        return Err("the declared rail's BOR level is not the one kind 10 declares");
    }
    if records
        .iter()
        .any(|r| matches!(r, Record::RailCrossing { domain, .. } if *domain != droop.domain))
    {
        return Err("a crossing names another domain");
    }
    let (floor, _) =
        rail_levels(droop, nominal).ok_or("the declared rail's drop reaches its floor")?;
    let settle_ps = tau_ns(droop.source_milliohms, decoupling)
        .saturating_mul(10)
        .saturating_mul(1000);
    let ps = |ns: u64| ns.saturating_mul(1000);
    let fall = (
        ps(rail.start_ns),
        ps(rail.start_ns + droop.fall_ns).saturating_add(settle_ps),
    );
    let rise_from = rail.start_ns + droop.fall_ns + droop.hold_ns;
    let rise = (
        ps(rise_from),
        ps(rise_from + droop.rise_ns).saturating_add(settle_ps),
    );
    for (name, uv) in power::levels(rail.bor_level) {
        let here: Vec<&power::Crossing> = crossings.iter().filter(|c| c.level == name).collect();
        let within = |c: &power::Crossing, (a, b): (u64, u64)| a < c.at_ps && c.at_ps <= b;
        let ok = if floor < uv {
            matches!(here.as_slice(), [f, r]
                if f.falling && !r.falling && f.microvolts == uv && r.microvolts == uv
                    && within(f, fall) && within(r, rise))
        } else {
            here.is_empty()
        };
        if !ok {
            return Err("a threshold's crossings do not follow from the declared rail");
        }
    }
    if crossings.len()
        != crossings
            .iter()
            .filter(|c| {
                power::levels(rail.bor_level)
                    .iter()
                    .any(|(n, _)| *n == c.level)
            })
            .count()
    {
        return Err("a crossing names a level no active threshold has");
    }
    let schedule = power::schedule(rail.bor_level, &crossings)?;
    let mut want: Vec<(u64, bool, &[String], &str)> = Vec::new();
    for r in &schedule {
        want.push((r.asserted_ns, false, &r.cause, power::ASSERT_RENDERING));
        if let Some(at) = r.released_ns {
            want.push((at, true, &r.cause, power::RELEASE_RENDERING));
        }
    }
    if resets != want {
        return Err("the rendered resets are not the supervisor's rule applied to the crossings");
    }
    // The recovery bound (`AET-WO-2305` §1.2), kind 32's, which every planted
    // run holds (`AET-WO-2310`): each probe at the release, its nudge and the
    // bound, inside the run. A droop that releases nothing probes nothing.
    let releases: Vec<u64> = schedule.iter().filter_map(|r| r.released_ns).collect();
    let bound = if releases.is_empty() {
        None
    } else {
        power::recovery_bound(word)
    };
    let probes: Vec<u64> = match bound {
        Some(b) => releases
            .iter()
            .map(|&at| power::probe_ns(at, b).filter(|&p| p <= TWIN_MAX_VIRTUAL_NS))
            .collect::<Option<_>>()
            .ok_or("a release's probe lies past the run")?,
        None => Vec::new(),
    };
    let boot_read = records.iter().any(|r| {
        matches!(
            r,
            Record::BootObserved {
                instructions_observed: Some(_),
                ..
            }
        )
    });
    let at: Vec<u64> = observed.iter().map(|o| o.0).collect();
    let unrecovered_probes = if bound.is_none() {
        if !at.is_empty() {
            return Err("a droop with no recovery bound holds an observation");
        }
        releases.len() as u64
    } else if boot_read {
        if at != probes {
            return Err("each release has not exactly one observation at the recovery bound");
        }
        observed.iter().filter(|o| !o.1).count() as u64
    } else {
        if !at.is_empty() {
            return Err("a boot that read nothing holds an observation");
        }
        probes.len() as u64
    };
    let unrecovered =
        unrecovered_probes + schedule.iter().filter(|r| r.released_ns.is_none()).count() as u64;
    let tolerance_floor = tolerance_floor_uv(nominal, tolerance);
    Ok([
        (RESET_FINDING, schedule.len() as u64),
        (UNRECOVERED_FINDING, unrecovered),
        (BELOW_TOLERANCE_FINDING, u64::from(floor < tolerance_floor)),
    ])
}

/// Kind 31 against its scenario (`AET-WO-2304b` §1.9), re-judged from its
/// own fields with the same function the runner used
/// ([`crate::battery::check_battery_sag`]): the sag and its declaration come
/// together and agree; every refusal clause, the static operating point,
/// the binding floor, and the deck's solved minimum, which must lie at or
/// above every floor and within [`crate::battery::SOLVE_TOLERANCE_UV`] of
/// the static point. A record any clause would have refused is refused.
fn check_battery_sag_records(
    records: &[Record],
    overrides: &[Override],
) -> Result<(), &'static str> {
    let sag = overrides.iter().find_map(|o| match o {
        Override::BatterySag(b) => Some(b),
        _ => None,
    });
    let declared = records
        .iter()
        .find(|r| matches!(r, Record::BatterySagDeclared { .. }));
    // §1.6 combined, judged again from the scenario's own record.
    let sags = overrides
        .iter()
        .filter(|o| matches!(o, Override::BatterySag(_)))
        .count();
    if sags > 1 || (sags == 1 && overrides.iter().any(|o| matches!(o, Override::Droop(_)))) {
        return Err("a battery sag is combined with another sag or a droop");
    }
    let (sag, declared) = match (sag, declared) {
        (None, None) => return Ok(()),
        (Some(s), Some(d)) => (s, d),
        _ => return Err("a battery sag override and its declaration do not come together"),
    };
    let Record::BatterySagDeclared {
        sag: declared_sag,
        start_ns,
        v_open_microvolts,
        healthy_milliohms,
        sinks,
        capacitance_picofarads,
        static_microvolts,
        solved_minimum_microvolts,
        binding,
    } = declared
    else {
        unreachable!("matched above");
    };
    if declared_sag != sag {
        return Err("the declared battery sag is not the scenario's");
    }
    // Its instant, placed from the undisturbed boot word (`AET-WO-2310`).
    let word = planted_reference(records, overrides)?
        .ok_or("a battery sag's run holds no undisturbed boot word")?;
    if record::resolve_instant(word, sag.from_boot_word_ns) != Some(*start_ns) {
        return Err(
            "the declared battery sag's instant is not the undisturbed boot word plus the scenario's offset",
        );
    }
    let chain = crate::battery::SagChain {
        v_open_microvolts: *v_open_microvolts,
        healthy_milliohms: *healthy_milliohms,
        sinks: sinks.clone(),
        capacitance_picofarads: *capacitance_picofarads,
    };
    let at = crate::battery::check_battery_sag(sag, Some(*start_ns), &chain, TWIN_MAX_VIRTUAL_NS)
        .map_err(|_| "a battery sag's declaration is one its checks refuse")?;
    if at != *static_microvolts {
        return Err("a battery sag's static operating point is not the one its fields give");
    }
    if crate::battery::binding(sinks).map(|k| &k.name) != Some(binding) {
        return Err("a battery sag's binding floor is not its highest");
    }
    let floor = sinks
        .iter()
        .map(record::SagSink::floor_microvolts)
        .max()
        .unwrap_or(0);
    if *solved_minimum_microvolts < floor
        || solved_minimum_microvolts.abs_diff(at) > crate::battery::SOLVE_TOLERANCE_UV
    {
        return Err(
            "a battery sag's solved minimum is below a floor, or not its static operating point",
        );
    }
    Ok(())
}

/// A pin device's records (`AET-WO-1211c`), joined to each other, to kind 26
/// and to the scenario's overrides, and the GPIO and ADC findings they give
/// (`PIN_DEVICE_FINDINGS`; the motor level finding is counted with the motor
/// findings). Every signal is declared once, by net, and every witness joins
/// one declaration: a witness is shaped as its signal's direction, and,
/// `witnessed`, is what the declaration says, so a verdict cannot call a
/// forged read a match. Once `booted`, every declared signal has its witness.
/// Each `pin_held` names an input kind 33 holding the override's level, and
/// each `analog_value` an ADC kind 35 holding its voltage; each held
/// declaration has its override, so a held level is never invented nor an
/// override dropped. A kind 26 with status `level` has exactly one kind 37
/// and each kind 37 its kind 26.
fn check_pin_device_records(
    records: &[Record],
    overrides: &[Override],
    booted: bool,
) -> Result<BTreeMap<&'static str, u64>, &'static str> {
    let mut gpio: BTreeMap<&str, &Record> = BTreeMap::new();
    let mut adc: BTreeMap<&str, &Record> = BTreeMap::new();
    for r in records {
        match r {
            Record::PinGpioDeclared { net, .. } if gpio.insert(net, r).is_some() => {
                return Err("a GPIO signal is declared twice");
            }
            Record::PinAdcDeclared { net, .. } if adc.insert(net, r).is_some() => {
                return Err("an ADC input is declared twice");
            }
            _ => {}
        }
    }
    // A pin device's pin is its own (adversarial F5): no two of kinds 33 and
    // 35 share one, and none is a solved peripheral's (an I2C line, an SPI
    // chip select, a motor output, a serial port's TX or RX), so a forged
    // archive cannot hold a peripheral's pin under a pin device's name.
    let mut taken: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        match r {
            Record::I2cBusSolved { lines, .. } => {
                taken.extend(lines.iter().map(|l| l.pin.as_str()))
            }
            Record::SpiBusSolved { devices, .. } => {
                taken.extend(devices.iter().map(|d| d.2.as_str()));
            }
            Record::MotorOutputSolved { pin, .. } => {
                taken.insert(pin);
            }
            Record::SerialPortSolved { tx, rx, .. } => taken.extend([tx.as_str(), rx.as_str()]),
            _ => {}
        }
    }
    for r in gpio.values().chain(adc.values()) {
        let (Record::PinGpioDeclared { pin, .. } | Record::PinAdcDeclared { pin, .. }) = r else {
            continue;
        };
        if !taken.insert(pin) {
            return Err("a pin device's pin is another signal's or a peripheral's");
        }
    }
    let mut out: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut gpio_witnessed: BTreeSet<&str> = BTreeSet::new();
    let mut adc_witnessed: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        match r {
            Record::PinGpioWitness {
                net,
                status,
                bsrr,
                ordered,
                level,
            } => {
                let Some(Record::PinGpioDeclared {
                    pin,
                    output,
                    level: declared,
                    active,
                    ..
                }) = gpio.get(net.as_str()).copied()
                else {
                    return Err("a GPIO witness names no declared GPIO signal");
                };
                if !gpio_witnessed.insert(net) {
                    return Err("a GPIO signal has two witnesses");
                }
                let shaped = if *output {
                    level.is_none()
                } else {
                    bsrr.is_none() && ordered.is_none()
                };
                if !shaped {
                    return Err("a GPIO witness is not shaped as its signal's direction");
                }
                // An input's mismatch reads a level, and not the driven one
                // (adversarial F6: a forged failure).
                if !*output && status == "mismatch" && level.is_none_or(|l| l == *declared) {
                    return Err("an input's mismatch reads no level, or the driven one");
                }
                if status == "witnessed" {
                    let fits = if *output {
                        let bit = split_pin(pin).map(|(_, n)| u32::from(n));
                        let mask = bit.and_then(|n| match declared {
                            1 => Some((1u32 << n, 1u32 << (n + 16))),
                            0 => Some((1u32 << (n + 16), 1u32 << n)),
                            _ => None,
                        });
                        *ordered == Some(true)
                            && mask
                                .zip(*bsrr)
                                .is_some_and(|((set, other), w)| w & set != 0 && w & other == 0)
                    } else {
                        level.is_some_and(|l| l == *declared)
                    };
                    if !fits {
                        return Err(
                            "a witnessed GPIO signal is not what its declaration says: an output's first BSRR word before its MODER write setting its declared level, an input's level its declared one",
                        );
                    }
                }
                if booted {
                    let kind = match status.as_str() {
                        "unwitnessed" | "unreadable" => Some(GPIO_UNWITNESSED_FINDING),
                        "mismatch" => Some(GPIO_MISMATCH_FINDING),
                        "witnessed" if !*output && *level == *active => {
                            Some(GPIO_INPUT_ASSERTED_FINDING)
                        }
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        *out.entry(kind).or_default() += 1;
                    }
                }
            }
            Record::PinAdcWitness {
                net, status, code, ..
            } => {
                let Some(Record::PinAdcDeclared { code: declared, .. }) =
                    adc.get(net.as_str()).copied()
                else {
                    return Err("an ADC witness names no declared ADC input");
                };
                if !adc_witnessed.insert(net) {
                    return Err("an ADC input has two witnesses");
                }
                if status == "witnessed" && *code != Some(*declared) {
                    return Err(
                        "a witnessed ADC input's code is not the one its declaration gives",
                    );
                }
                // A mismatch reads a code, and not the declared one
                // (adversarial F6: a forged failure).
                if status == "mismatch" && code.is_none_or(|c| c == *declared) {
                    return Err("an ADC input's mismatch reads no code, or the declared one");
                }
                if booted {
                    let kind = match status.as_str() {
                        "unwitnessed" => Some(ADC_UNWITNESSED_FINDING),
                        "mismatch" => Some(ADC_MISMATCH_FINDING),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        *out.entry(kind).or_default() += 1;
                    }
                }
            }
            _ => {}
        }
    }
    if booted && (gpio_witnessed.len() != gpio.len() || adc_witnessed.len() != adc.len()) {
        return Err(
            "a declared GPIO signal or ADC input has no witness, though the boot word is set",
        );
    }
    // The overrides against the declarations they set.
    let mut held_nets: BTreeSet<&str> = BTreeSet::new();
    let mut analog_nets: BTreeSet<&str> = BTreeSet::new();
    for o in overrides {
        match o {
            Override::PinHeld { net, level } => {
                if !held_nets.insert(net) {
                    return Err("a scenario holds one net twice");
                }
                match gpio.get(net.as_str()).copied() {
                    None => return Err("a pin_held override names no declared GPIO signal"),
                    Some(Record::PinGpioDeclared { output: true, .. }) => {
                        return Err("PinHeldIsOutput: a pin_held override names a GPIO output");
                    }
                    Some(Record::PinGpioDeclared { level: l, held, .. })
                        if !*held || *l != u8::from(*level == Level::High) =>
                    {
                        return Err(
                            "a pin_held override is not held at its level by its signal's declaration",
                        );
                    }
                    Some(_) => {}
                }
            }
            Override::AnalogValue { net, millivolts } => {
                if !analog_nets.insert(net) {
                    return Err("a scenario sets one analog net twice");
                }
                match adc.get(net.as_str()).copied() {
                    Some(Record::PinAdcDeclared {
                        held: true,
                        microvolts,
                        ..
                    }) if *microvolts == u64::from(*millivolts) * 1000 => {}
                    _ => {
                        return Err(
                            "an analog_value override names no ADC input held at its voltage",
                        );
                    }
                }
            }
            _ => {}
        }
    }
    for r in gpio.values() {
        if matches!(r, Record::PinGpioDeclared { net, held: true, .. } if !held_nets.contains(net.as_str()))
        {
            return Err("a held GPIO signal has no pin_held override");
        }
    }
    for r in adc.values() {
        if matches!(r, Record::PinAdcDeclared { net, held: true, .. } if !analog_nets.contains(net.as_str()))
        {
            return Err("a held ADC input has no analog_value override");
        }
    }
    // Kind 26's `level` and kind 37 are each other's, by label.
    let judged: BTreeSet<&str> = records
        .iter()
        .filter_map(|r| match r {
            Record::MotorWitness { label, status, .. } if status == "level" => Some(label.as_str()),
            _ => None,
        })
        .collect();
    let mut levelled: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        if let Record::MotorLevelWitness { label, .. } = r
            && (!judged.contains(label.as_str()) || !levelled.insert(label))
        {
            return Err("a level witness names no kind 26 judged by level, or one twice");
        }
    }
    // Each level witness against its output (adversarial F1, F9): its level
    // is kind 24's declared idle level; its settled instant is kind 26's
    // anchor, plus one period and one quantum for an idle high (`1211` Q10);
    // its counts are within kind 26's; and witnessed, the window holds a
    // period after it, and kind 26 shows no edge but an idle high's one rise.
    for r in records {
        let Record::MotorLevelWitness {
            label,
            level,
            settle_ns,
            rises_after,
            falls_after,
            status,
            ..
        } = r
        else {
            continue;
        };
        let idle = records.iter().find_map(|s| match s {
            Record::MotorOutputSolved { label: l, idle, .. } if l == label => Some(*idle),
            _ => None,
        });
        let high = match idle {
            Some(Some(record::MotorIdle::Low)) => false,
            Some(Some(record::MotorIdle::High)) => true,
            _ => return Err("a level witness's output does not idle at a static level"),
        };
        if *level != u8::from(high) {
            return Err("a level witness's level is not its output's declared idle");
        }
        let Some(Record::MotorWitness {
            anchor_ns,
            period_ns,
            quantum_ns,
            end_ns,
            rises,
            falls,
            ..
        }) = records
            .iter()
            .find(|w| matches!(w, Record::MotorWitness { label: l, .. } if l == label))
        else {
            return Err("a motor witness judged by level has no level witness");
        };
        let settle = anchor_ns.map(|a| a + if high { period_ns + quantum_ns } else { 0 });
        if *settle_ns != settle {
            return Err(
                "a level witness's settled instant is not its anchor's, plus a period and a quantum when high",
            );
        }
        if rises_after > rises || falls_after > falls {
            return Err("a level witness counts more edges than its motor witness");
        }
        let still = match high {
            false => *rises == 0 && *falls == 0,
            true => *rises <= 1 && *falls == 0,
        };
        let window = settle.is_some_and(|s| s.saturating_add(*period_ns) <= *end_ns);
        if status == "witnessed" && !(still && window) {
            return Err(
                "a witnessed level's motor witness shows an edge past its idle's, or its window holds no period after the settled instant",
            );
        }
    }
    if levelled != judged {
        return Err("a motor witness judged by level has no level witness");
    }
    Ok(out)
}

/// The refusal of a rail out of band (`AET-WO-2304b` §1.7), one kind with a
/// message per clause.
pub const RAIL_REFUSED: &str = "RailOutOfBandRefused";

/// A rail out of band is rendered through the compositions the scenario
/// lists beside it (`AET-WO-2304b` §1.7, K1): one rail, at least one
/// consumer's absence, a `serial_rx_open` or a `spi_device_absent`, and
/// nothing else faulted with it. Which consumers the specification gives is
/// the runner's check; the records carry no specification (`RUL-16`). The
/// parser and replay both apply it.
pub fn check_rail_out_of_band(overrides: &[Override]) -> Result<(), &'static str> {
    let rails = overrides
        .iter()
        .filter(|o| matches!(o, Override::RailOutOfBand { .. }))
        .count();
    if rails == 0 {
        return Ok(());
    }
    if rails > 1 {
        return Err(
            "more than one rail out of band in one scenario: their interaction is not modelled",
        );
    }
    let rendered = |o: &Override| {
        matches!(
            o,
            Override::SerialRxOpen { .. } | Override::SpiDeviceAbsent { .. }
        )
    };
    let only = overrides
        .iter()
        .all(|o| rendered(o) || matches!(o, Override::RailOutOfBand { .. }));
    if !only || !overrides.iter().any(rendered) {
        return Err(
            "a rail out of band is not rendered by its consumers' absence alone, or by none",
        );
    }
    Ok(())
}

/// The motor output findings the boot judge gives (`AET-WO-4300a`).
pub const MOTOR_STATIC_FINDING: &str = "motor_timing_static_mismatch";
pub const MOTOR_UNWITNESSED_FINDING: &str = "motor_output_unwitnessed";
pub const MOTOR_MISMATCH_FINDING: &str = "motor_timing_mismatch";
/// `info`: the witness fit the model's sequence with every edge on the grid.
pub const MOTOR_STATIC_ONLY_FINDING: &str = "motor_witness_static_only";

/// A motor output held at a static level (`AET-WO-1211c`, kind 37) that did
/// not stay at it once its timer settled: one per kind 37 whose status is
/// not `witnessed`, once the boot word is set.
pub const MOTOR_LEVEL_FINDING: &str = "motor_level_mismatch";

/// The pin devices' findings the boot judge gives (`AET-WO-1211c`), each
/// once the boot word is set: an input the firmware's `IDR` read at its
/// active level (kind 34, `witnessed`, the level equal to kind 33's
/// `active`); a GPIO signal unwitnessed or unreadable, or witnessed not as
/// declared; an ADC input unread, or read as another code (kind 36).
pub const GPIO_INPUT_ASSERTED_FINDING: &str = "pin_device_input_asserted";
pub const GPIO_UNWITNESSED_FINDING: &str = "gpio_unwitnessed";
pub const GPIO_MISMATCH_FINDING: &str = "gpio_mismatch";
pub const ADC_UNWITNESSED_FINDING: &str = "adc_read_unwitnessed";
pub const ADC_MISMATCH_FINDING: &str = "adc_value_mismatch";

/// Every GPIO and ADC finding kind the records explain exactly, each an
/// error. Apart from [`BOOTED_READ_FINDINGS`], whose refusal speaks of
/// identity reads: these are counted from their records, so a boot word not
/// set is a count of 0 here.
const PIN_DEVICE_FINDINGS: [&str; 5] = [
    GPIO_INPUT_ASSERTED_FINDING,
    GPIO_UNWITNESSED_FINDING,
    GPIO_MISMATCH_FINDING,
    ADC_UNWITNESSED_FINDING,
    ADC_MISMATCH_FINDING,
];

/// The anchors' findings the boot judge gives (`AET-WO-2302`, ruled (a-i)):
/// each motor timer's window anchors at its first `CEN` rise in the boot
/// the judge reads, which must be observed (edges with none:
/// `unanchored`), once, after the boot's last device step's witness and
/// before its boot word; a DShot window must end inside the run, its stop
/// must run and read back the solved registers; and no edge may fall
/// before its anchor or after its window.
pub const MOTOR_ANCHOR_UNOBSERVED_FINDING: &str = "motor_anchor_unobserved";
pub const MOTOR_ANCHOR_REPEATED_FINDING: &str = "motor_anchor_repeated";
pub const MOTOR_ANCHOR_ORDER_FINDING: &str = "motor_anchor_out_of_order";
pub const MOTOR_WINDOW_PAST_RUN_FINDING: &str = "motor_window_past_run";
pub const MOTOR_EDGE_BEFORE_FINDING: &str = "motor_edge_before_anchor";
pub const MOTOR_EDGE_AFTER_FINDING: &str = "motor_edge_after_window";
pub const JUDGE_STOP_MISSING_FINDING: &str = "judge_stop_missing";
pub const JUDGE_STOP_MISMATCH_FINDING: &str = "judge_stop_mismatch";
/// The ports' summed wire time from one quantum after the boot word, past
/// the run's end (`AET-WO-2302` D4): known only after the run.
pub const SERIAL_WIRE_PAST_RUN_FINDING: &str = "serial_wire_past_run";
/// A port's frame byte injected before the port before it in the chain had
/// echoed its whole frame (`AET-WO-2302` D4, adversarial pass 1, L2),
/// recounted from the logged `TDR` bytes.
pub const SERIAL_INJECTION_ORDER_FINDING: &str = "serial_injection_out_of_order";

/// The serial port findings the boot judge gives (`AET-WO-4300b`): the
/// silicon check of a port's `BRR` and framing, and the twin's witness of its
/// pattern, its echo, its capture and its tags.
pub const SERIAL_STATIC_FINDING: &str = "serial_framing_static_mismatch";
pub const SERIAL_PATTERN_FINDING: &str = "serial_pattern_missing";
pub const SERIAL_ECHO_FINDING: &str = "serial_echo_missing";
pub const SERIAL_FORGED_FINDING: &str = "serial_capture_forged";
pub const SERIAL_TAG_FINDING: &str = "serial_tag_mismatch";

/// Every serial finding kind the records explain.
const SERIAL_FINDINGS: [&str; 5] = [
    SERIAL_STATIC_FINDING,
    SERIAL_PATTERN_FINDING,
    SERIAL_ECHO_FINDING,
    SERIAL_FORGED_FINDING,
    SERIAL_TAG_FINDING,
];

/// Each serial finding kind the records give, and how many: a failing
/// static check (kind 28's `holds`) always; once `booted`, each port's tags
/// that failed or were not read (kind 28), and each witness's findings
/// ([`serial_witness_findings`]).
fn serial_findings(records: &[Record], booted: bool) -> BTreeMap<&'static str, u64> {
    let releases = released(records);
    let mut out: BTreeMap<&'static str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::SerialFramingCheck { holds, tags, .. } => {
                if !holds {
                    *out.entry(SERIAL_STATIC_FINDING).or_default() += 1;
                }
                if booted && tags.as_ref().is_none_or(|t| !t.holds()) {
                    *out.entry(SERIAL_TAG_FINDING).or_default() += 1;
                }
            }
            Record::SerialWitness { label, .. } if booted => {
                let port = records
                    .iter()
                    .find(|p| matches!(p, Record::SerialPortSolved { label: l, .. } if l == label));
                for kind in port
                    .map(|p| serial_witness_findings(p, r, releases))
                    .unwrap_or_default()
                {
                    *out.entry(kind).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    out
}

/// The resets the run rendered and released (kind 18): the boots after the
/// first.
pub fn released(records: &[Record]) -> u64 {
    records
        .iter()
        .filter(|r| matches!(r, Record::ResetRendered { released: true, .. }))
        .count() as u64
}

/// Whether a pin's `MODER` and `AFSEL` fields, as kind 29 carries them, put
/// it in alternate-function mode (`10`, RM0433 Rev 9 §11.4.1) on `af`.
pub fn routed(mux: Option<(u8, u8)>, af: u8) -> bool {
    mux.is_some_and(|(mode, a)| mode == 0b10 && a == af)
}

/// The finding kinds kind 29 `witness` gives, joined to kind 27 `port`, in a
/// run whose boot word is set, with `releases` boots after the first, as the
/// boot judge gives them: no capture is forged and nothing else; a capture
/// with stray bytes, or more patterns (whole and cut) than boots, is forged;
/// the pattern is out when at least one whole pattern was captured and the
/// TX pin was on its AF at the first `TDR` write of each boot (adversarial
/// F4) and at the end; and the echo fails unless witnessed.
pub fn serial_witness_findings(
    port: &Record,
    witness: &Record,
    releases: u64,
) -> Vec<&'static str> {
    let (
        Record::SerialPortSolved { tx_af, .. },
        Record::SerialWitness {
            capture,
            echo,
            tx_at_end,
            tx_at_send,
            ..
        },
    ) = (port, witness)
    else {
        return Vec::new();
    };
    let Some(c) = capture else {
        return vec![SERIAL_FORGED_FINDING];
    };
    let mut out = Vec::new();
    let boots = releases.saturating_add(1);
    if c.stray || c.patterns.saturating_add(c.cut) > boots {
        out.push(SERIAL_FORGED_FINDING);
    }
    if !(c.patterns > 0 && routed(*tx_at_send, *tx_af) && routed(*tx_at_end, *tx_af)) {
        out.push(SERIAL_PATTERN_FINDING);
    }
    if echo != "witnessed" && echo != "not_judged" {
        out.push(SERIAL_ECHO_FINDING);
    }
    out
}

/// Kinds 28 and 29 each join one solved serial port (kind 27) by label
/// (`AET-WO-4300b`): the ports are in the specification's order, each on
/// its own USART and pins; every port has exactly one framing check, in the
/// ports' order, which is what its solved port gives silicon; and its
/// witnesses are in the ports' order too, every port's or none, and every
/// port's once the boot word is set, each echo the status its own counts and
/// pins give, and its TX pin at its sends rebuilt only where its framing
/// check's writes were (both come from one access log; adversarial pass 2,
/// L1).
fn check_serial_joins(records: &[Record], booted: bool) -> Result<(), &'static str> {
    let ports: Vec<&Record> = records
        .iter()
        .filter(|r| matches!(r, Record::SerialPortSolved { .. }))
        .collect();
    let mut labels = BTreeSet::new();
    let mut peripherals = BTreeSet::new();
    let mut pins = BTreeSet::new();
    let mut last_index = None;
    for p in &ports {
        let Record::SerialPortSolved {
            label,
            index,
            peripheral,
            tx,
            rx,
            ..
        } = p
        else {
            unreachable!("filtered");
        };
        if !labels.insert(label.as_str())
            || !peripherals.insert(peripheral.as_str())
            || !pins.insert(tx.as_str())
            || !pins.insert(rx.as_str())
            || last_index.is_some_and(|l| l >= *index)
        {
            return Err(
                "the solved serial ports are not in the specification's order, each with its own label, USART and pins",
            );
        }
        last_index = Some(*index);
    }
    let label_of = |r: &Record| match r {
        Record::SerialPortSolved { label, .. }
        | Record::SerialFramingCheck { label, .. }
        | Record::SerialWitness { label, .. } => label.clone(),
        _ => String::new(),
    };
    let order: Vec<String> = ports.iter().map(|p| label_of(p)).collect();
    let of = |pick: fn(&Record) -> bool| -> Vec<&Record> {
        records.iter().filter(|r| pick(r)).collect()
    };
    let checks = of(|r| matches!(r, Record::SerialFramingCheck { .. }));
    let witnesses = of(|r| matches!(r, Record::SerialWitness { .. }));
    if checks.iter().map(|r| label_of(r)).collect::<Vec<_>>() != order {
        return Err("the serial framing checks are not one per solved port, in the ports' order");
    }
    let witnessed: Vec<String> = witnesses.iter().map(|r| label_of(r)).collect();
    if witnessed != order && (booted || !witnessed.is_empty()) {
        return Err(
            "the serial witnesses are not one per solved port in the ports' order, or are absent from a run whose boot word is set",
        );
    }
    for (port, check) in ports.iter().zip(&checks) {
        if !serial_silicon(port, check) {
            return Err("a serial framing check is not what its solved port gives");
        }
    }
    for (check, witness) in checks.iter().zip(&witnesses) {
        if matches!(
            (check, witness),
            (
                Record::SerialFramingCheck { tags: None, .. },
                Record::SerialWitness {
                    tx_at_send: Some(_),
                    ..
                }
            )
        ) {
            return Err(
                "a serial witness's TX pin at its sends is rebuilt from an access log its framing check could not read",
            );
        }
    }
    for ((port, check), witness) in ports.iter().zip(&checks).zip(&witnesses) {
        if !serial_echo(port, check, witness, booted) {
            return Err("a serial witness's echo is not what its capture and its RX pin give");
        }
        if !serial_tx_read(port, witness, records) {
            return Err(
                "a serial witness's TX pin at the end is not what kind 11 read of its MODER and AFR",
            );
        }
    }
    // A port the scenario holds open (`AET-WO-2302` D5) is a solved port,
    // and nothing went into it; the chain skips it.
    let open = open_ports(records);
    if open.iter().any(|p| !labels.contains(p)) {
        return Err("an open serial RX line names no solved serial port");
    }
    if witnesses.iter().any(|w| {
        matches!(w, Record::SerialWitness { label, injected_ns, rx_at_injection, .. }
            if open.contains(label.as_str()) && (injected_ns.is_some() || rx_at_injection.is_some()))
    }) {
        return Err("a frame went into a serial port whose RX line the scenario holds open");
    }
    // The injections are sequential (`AET-WO-2302` D4): a port goes in only
    // after the port before it that is not held open, inside the boot.
    let injected: Vec<Option<u64>> = witnesses
        .iter()
        .filter_map(|w| match w {
            Record::SerialWitness { label, .. } if open.contains(label.as_str()) => None,
            Record::SerialWitness { injected_ns, .. } => Some(*injected_ns),
            _ => None,
        })
        .collect();
    let sequential = injected.windows(2).all(|w| match (w[0], w[1]) {
        (Some(a), Some(b)) => a < b,
        (None, Some(_)) => false,
        _ => true,
    });
    if !sequential || injected.iter().flatten().any(|&t| t > TWIN_MAX_VIRTUAL_NS) {
        return Err(
            "a serial port's frame went in before the port before it, without it, or past the boot",
        );
    }
    // The first port goes in one quantum after the boot word, which the
    // boot writes after its start (`AET-WO-2302b` ADV-S4): earlier, the boot
    // word's instant would precede the run, and `boot_word_instant` finds none.
    if injected
        .first()
        .copied()
        .flatten()
        .is_some_and(|t| t < TWIN_QUANTUM_NS)
    {
        return Err(
            "the first serial port's frame went in less than a quantum after the boot's start, before any boot word",
        );
    }
    Ok(())
}

/// The labels of the ports whose RX line the scenario (kind 5) holds open
/// (`AET-WO-2302` D5).
pub fn open_ports(records: &[Record]) -> BTreeSet<&str> {
    match records.first() {
        Some(Record::Scenario { overrides, .. }) => overrides
            .iter()
            .filter_map(|o| match o {
                Override::SerialRxOpen { port } => Some(port.as_str()),
                _ => None,
            })
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// The first port the judge's chain injects, the first not held open, as
/// its kind 29's index and `injected_ns`. `None` when that port took no
/// frame, or there is none.
pub fn first_injection(records: &[Record]) -> Option<(usize, u64)> {
    let open = open_ports(records);
    records
        .iter()
        .enumerate()
        .find_map(|(i, r)| match r {
            Record::SerialWitness {
                label, injected_ns, ..
            } if !open.contains(label.as_str()) => Some(injected_ns.map(|t| (i, t))),
            _ => None,
        })
        .flatten()
}

/// The boot word's instant as the records give it (`AET-WO-2302`
/// adversarial pass 2, A2-5): the first port the chain injects takes its
/// frame one quantum after the boot word, so its kind 29 `injected_ns`
/// less `QUANTUM_NS`. `None` when that port took no frame, or there is none.
pub fn boot_word_instant(records: &[Record]) -> Option<u64> {
    first_injection(records)?.1.checked_sub(TWIN_QUANTUM_NS)
}

/// One character's time on the wire in ns, rounded up, at a solved actual
/// baud in thousandths, and a port's wire time after the boot word: its
/// frame, `SERIAL_FRAME_LEN` bytes each a character rounded up to a whole
/// quantum apart, and one echoed character (A2-3). `None` for a baud of 0
/// or an overflow.
// belt-and-suspenders: `aethelgard_twin::serial::{char_ns, spacing_ns}` and
// `aethelgard-cli`'s `frame_echo_ns` state these; replay checks an archive
// without the twin or the CLI, so it states them again.
fn frame_echo_ns(framing: &SerialFraming, actual_millibaud: u64) -> Option<u64> {
    let bits = 1 + u64::from(framing.word_bits()) + u64::from(framing.stop_bits);
    let char_ns = (actual_millibaud > 0)
        .then(|| bits.checked_mul(1_000_000_000_000))
        .flatten()?
        .div_ceil(actual_millibaud);
    char_ns
        .div_ceil(TWIN_QUANTUM_NS)
        .checked_mul(TWIN_QUANTUM_NS)?
        .checked_mul(SERIAL_FRAME_LEN as u64)?
        .checked_add(char_ns)
}

/// The ports, by label, whose frames and echoes end past the boot, as the
/// judge sums them for `serial_wire_past_run` of a booted run (A2-5): from
/// the first port the chain injects, one quantum after the boot word, the
/// ports' frames and echoes (kind 27's framings, summed in the
/// specification's order over the ports not held open), from the first
/// whose sum ends past the boot to the last. Empty with no port the chain
/// injects, or when every frame and echo ends inside the boot; `None` when
/// the first port took no frame, so the instant is not in the records.
pub fn wire_past_run_ports(records: &[Record]) -> Option<Vec<&str>> {
    let open = open_ports(records);
    let ports: Vec<(&str, &SerialFraming, u64)> = records
        .iter()
        .filter_map(|r| match r {
            Record::SerialPortSolved {
                label,
                framing,
                actual_millibaud,
                ..
            } if !open.contains(label.as_str()) => {
                Some((label.as_str(), framing, *actual_millibaud))
            }
            _ => None,
        })
        .collect();
    if ports.is_empty() {
        return Some(Vec::new());
    }
    let injected = boot_word_instant(records)?.checked_add(TWIN_QUANTUM_NS)?;
    let mut end = Some(injected);
    Some(
        ports
            .into_iter()
            .filter(|&(_, f, baud)| {
                end = end
                    .zip(frame_echo_ns(f, baud))
                    .and_then(|(e, w)| e.checked_add(w));
                !end.is_some_and(|e| e <= TWIN_MAX_VIRTUAL_NS)
            })
            .map(|(label, ..)| label)
            .collect(),
    )
}

/// The timer of the motor output `label`, joined from kind 24.
fn timer_of<'a>(records: &'a [Record], label: &str) -> Option<&'a str> {
    records.iter().find_map(|m| match m {
        Record::MotorOutputSolved {
            label: l, timer, ..
        } if l == label => Some(timer.as_str()),
        _ => None,
    })
}

/// The bounds on `motor_anchor_out_of_order` in a booted run the records
/// give (A2-5): at most once per timer with an anchor (kind 26, joined to
/// kind 24's timer); and, where the boot word's instant is in the records
/// ([`boot_word_instant`]), at least once per timer whose anchor is after
/// it, since the bring-up writes the boot word last. Its other cases, an
/// anchor before the last device step's witness or no boot word seen by its
/// hook, are the order of the boot's log lines, which no record carries.
fn anchor_order_bounds(records: &[Record]) -> (u64, u64) {
    let mut anchors: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        let Record::MotorWitness {
            label,
            anchor_ns: Some(a),
            ..
        } = r
        else {
            continue;
        };
        if let Some(t) = timer_of(records, label) {
            anchors.insert(t, *a);
        }
    }
    let after = boot_word_instant(records)
        .map_or(0, |b| anchors.values().filter(|&&a| a > b).count() as u64);
    (after, anchors.len() as u64)
}

/// Whether kind 29 `witness`'s TX pin at the end is the `MODER` and `AFSEL`
/// fields of kind 11's reads of the pin's `GPIOx.MODER` and `GPIOx.AFRL` or
/// `AFRH`, joined to kind 27 `port` (adversarial F11): both present and
/// equal, or both absent (the image read neither register).
// belt-and-suspenders: the judge takes both from one read
// (`aethelgard_twin::serial::PinMux`); replay checks an archive without the
// twin, so it splits the pin and places its fields again.
fn serial_tx_read(port: &Record, witness: &Record, records: &[Record]) -> bool {
    let (Record::SerialPortSolved { tx, .. }, Record::SerialWitness { tx_at_end, .. }) =
        (port, witness)
    else {
        return false;
    };
    let mut chars = tx.chars();
    let (Some('P'), Some(letter @ 'A'..='K')) = (chars.next(), chars.next()) else {
        return false;
    };
    let Some(index) = chars.as_str().parse::<u32>().ok().filter(|i| *i < 16) else {
        return false;
    };
    let read = |name: String| {
        records.iter().find_map(|r| match r {
            Record::RegisterObserved { name: n, reading } if *n == name => reading.observed,
            _ => None,
        })
    };
    let moder = read(format!("GPIO{letter}.MODER"));
    let afr = read(format!(
        "GPIO{letter}.{}",
        if index < 8 { "AFRL" } else { "AFRH" }
    ));
    match (tx_at_end, moder.zip(afr)) {
        (None, None) => moder.is_none() && afr.is_none(),
        (Some((mode, af)), Some((m, a))) => {
            u32::from(*mode) == (m >> (2 * index)) & 0b11
                && u32::from(*af) == (a >> (4 * (index % 8))) & 0xF
        }
        _ => false,
    }
}

/// RM0433 Rev 9 §48.5.8, Tables 406 (`BRR[3:0]` = 0) and 407, column
/// `OVER8` = 0, `ONEBIT` = 0: the receiver's tolerance in ppm by `M[1:0]`.
// belt-and-suspenders: `aethelgard_ir::serial::TOLERANCE` is the judge's
// table; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// table again.
const SERIAL_TOLERANCE: [(&str, u8, u32); 6] = [
    ("Table 406", 0b00, 37_500),
    ("Table 406", 0b01, 34_100),
    ("Table 406", 0b10, 41_600),
    ("Table 407", 0b00, 33_300),
    ("Table 407", 0b01, 30_300),
    ("Table 407", 0b10, 37_000),
];

/// Whether kind 28 `check` is exactly the static framing check of kind 27
/// `port`: the figures, the tolerance row, `holds`, and the tags the framing
/// needs, re-derived (as kind 25's are: adversarial pass on `4300a`).
// belt-and-suspenders: `aethelgard_ir::serial::check_silicon` is the judge's
// check; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// same exact-integer rules again: the kernel clock at reset (RM0433 Rev 9
// §8.7.21, every prescaler 1), `BRR` the nearest whole `USARTDIV` from 16 to
// `0xFFFF`, a 7- to 9-bit word and 1 or 2 stop bits, and an error strictly
// below the row's tolerance.
fn serial_silicon(port: &Record, check: &Record) -> bool {
    let (
        Record::SerialPortSolved {
            framing,
            kernel_hz: k_port,
            brr: b_port,
            actual_millibaud: a_port,
            error_ppm: e_port,
            ..
        },
        Record::SerialFramingCheck {
            kernel_hz,
            brr,
            actual_millibaud,
            error_ppm,
            tolerance,
            holds,
            tags_needed,
            ..
        },
    ) = (port, check)
    else {
        return false;
    };
    let (k, b, baud) = (
        u128::from(*kernel_hz),
        u128::from(*brr),
        u128::from(framing.baud),
    );
    let d = b * baud;
    let actual = u64::try_from(k * 1000 / b.max(1)).ok();
    // The error, rounded half away from zero.
    let error = (d > 0).then(|| {
        let num =
            (i128::try_from(k).unwrap_or(i128::MAX) - i128::try_from(d).unwrap_or(0)) * 1_000_000;
        let d = i128::try_from(d).unwrap_or(i128::MAX);
        let q = (2 * num.abs() + d) / (2 * d);
        i64::try_from(if num < 0 { -q } else { q }).unwrap_or(i64::MAX)
    });
    let m_bits = match framing.word_bits() {
        8 => Some(0b00),
        9 => Some(0b01),
        7 => Some(0b10),
        _ => None,
    };
    let table = if brr & 0xF == 0 {
        "Table 406"
    } else {
        "Table 407"
    };
    let row = m_bits.and_then(|m| {
        SERIAL_TOLERANCE
            .iter()
            .find(|(t, rm, _)| *t == table && *rm == m)
    });
    let solved = (baud > 0)
        .then(|| (2 * k + baud) / (2 * baud))
        .is_some_and(|n| n == b && (16..=0xFFFF).contains(&n));
    let within = row.is_some_and(|(_, _, ppm)| k.abs_diff(d) * 1_000_000 < u128::from(*ppm) * d);
    let want_holds =
        *kernel_hz == RESET_KERNEL_HZ && solved && (1..=2).contains(&framing.stop_bits) && within;
    (kernel_hz, brr) == (k_port, b_port)
        && actual == Some(*actual_millibaud)
        && *a_port == *actual_millibaud
        && error.unwrap_or(i64::MAX) == *error_ppm
        && *e_port == *error_ppm
        && tolerance.as_ref().map(|(t, m, p)| (t.as_str(), *m, *p))
            == row.map(|&(t, m, p)| (t, m, p))
        && want_holds == *holds
        && *tags_needed == framing.tags()
}

/// Whether kind 29 `witness`'s echo is the status the judge gives its
/// capture and its RX pin at the injection, joined to kind 27 `port`: not
/// judged without the boot word; else the whole frame after the patterns
/// with the RX pin read is witnessed when that pin was on its AF and not
/// counted otherwise, or not counted when kind 28 `check`'s tags were read
/// and failed (the interval rule named the RX pin changed after it was solved);
/// some of it is partial; anything else is missing.
fn serial_echo(port: &Record, check: &Record, witness: &Record, booted: bool) -> bool {
    let (
        Record::SerialPortSolved { rx_af, .. },
        Record::SerialWitness {
            capture,
            echo,
            rx_at_injection,
            ..
        },
    ) = (port, witness)
    else {
        return false;
    };
    let clean = |c: &&SerialCapture| !c.stray;
    let echoed = capture.as_ref().filter(clean).map_or(0, |c| c.echoed);
    let want = match (booted, rx_at_injection) {
        (false, _) => "not_judged",
        (true, Some(_)) if echoed == SERIAL_FRAME_LEN as u64 => {
            if routed(*rx_at_injection, *rx_af) {
                "witnessed"
            } else {
                "not_counted"
            }
        }
        (true, Some(_)) if echoed > 0 => "partial",
        (true, _) => "missing",
    };
    // Adversarial pass 4, ADV4-4: a frame back whole on a pin routed at the
    // injection is not counted when the RX pin was unrouted after it, which
    // the interval rule names among kind 28's unexpected writes: the tags
    // must have been read and failed.
    let unrouted_after =
        matches!(check, Record::SerialFramingCheck { tags: Some(t), .. } if !t.holds());
    echo == want || (want == "witnessed" && echo == "not_counted" && unrouted_after)
}

/// Each serial port's frame, kind 29's, against the one the judge injects
/// into the firmware the header names (`artifacts` "firmware"): the frame is
/// re-derived from that sha256 and the port's label, never read from the
/// record alone (`AET-WO-4300b`).
pub fn check_serial_frames(firmware: &[u8; 32], records: &[Record]) -> Result<(), &'static str> {
    let forged = records.iter().any(|r| {
        matches!(r, Record::SerialWitness { label, frame, .. }
            if *frame != record::serial_frame(firmware, label))
    });
    if forged {
        Err("a serial witness's frame is not the one its firmware and label give")
    } else {
        Ok(())
    }
}

/// Every motor finding kind the records explain exactly.
/// `motor_anchor_out_of_order` is bounded apart ([`anchor_order_bounds`]):
/// its cases other than an anchor after the boot word are the order of the
/// boot's log lines, which no record carries.
const MOTOR_FINDINGS: [&str; 12] = [
    MOTOR_STATIC_FINDING,
    MOTOR_UNWITNESSED_FINDING,
    MOTOR_MISMATCH_FINDING,
    MOTOR_STATIC_ONLY_FINDING,
    MOTOR_ANCHOR_UNOBSERVED_FINDING,
    MOTOR_WINDOW_PAST_RUN_FINDING,
    JUDGE_STOP_MISSING_FINDING,
    JUDGE_STOP_MISMATCH_FINDING,
    MOTOR_ANCHOR_REPEATED_FINDING,
    MOTOR_EDGE_BEFORE_FINDING,
    MOTOR_EDGE_AFTER_FINDING,
    MOTOR_LEVEL_FINDING,
];

/// The anchors' findings kinds 26 and 30 carry (`AET-WO-2302` adversarial
/// pass 1, L3), as the judge names them once the boot word is set:
/// `judge_stop_mismatch` once per register a stop that left `CEN` cleared
/// read back off its solved value under its mask, and once more when it
/// read fewer registers back than its timer has; `motor_anchor_repeated`
/// once per timer whose `CEN` rose more than once (kind 26's `cen_rises`,
/// one per timer, [`check_motor_joins`]); `motor_edge_before_anchor` and
/// `motor_edge_after_window` once per output with such edges.
fn anchor_record_findings(records: &[Record]) -> Vec<(&'static str, u64)> {
    let (mut mismatch, mut before, mut after) = (0, 0, 0);
    let mut rises: BTreeMap<&str, u64> = BTreeMap::new();
    for r in records {
        match r {
            Record::JudgeStop {
                read_back,
                cr1_after: Some(c),
                ..
            } if c & 1 == 0 => {
                let off = read_back
                    .iter()
                    .filter(|g| g.observed.is_some_and(|o| o & g.mask != g.expected))
                    .count() as u64;
                let short = read_back.iter().any(|g| g.observed.is_none());
                mismatch += off + u64::from(short);
            }
            Record::MotorWitness {
                label,
                edges_before,
                edges_after,
                cen_rises,
                ..
            } => {
                before += u64::from(*edges_before > 0);
                after += u64::from(*edges_after > 0);
                if let Some(t) = timer_of(records, label) {
                    rises.insert(t, *cen_rises);
                }
            }
            _ => {}
        }
    }
    let repeated = rises.values().filter(|&&n| n > 1).count() as u64;
    vec![
        (JUDGE_STOP_MISMATCH_FINDING, mismatch),
        (MOTOR_ANCHOR_REPEATED_FINDING, repeated),
        (MOTOR_EDGE_BEFORE_FINDING, before),
        (MOTOR_EDGE_AFTER_FINDING, after),
    ]
}

/// Each DShot timer's window findings, from kinds 24, 26 and 30
/// (`AET-WO-2302`): its anchored window past the boot
/// (`motor_window_past_run`), or inside it with no judge's stop for that
/// anchor (`judge_stop_missing`; a kind 30 whose `CR1` after it still has
/// `CEN` set is no stop).
fn dshot_timer_findings(records: &[Record]) -> Vec<(&'static str, u64)> {
    let mut anchors: BTreeMap<&str, Option<u64>> = BTreeMap::new();
    for r in records {
        let Record::MotorOutputSolved {
            label,
            timer,
            protocol,
            ..
        } = r
        else {
            continue;
        };
        if !protocol.starts_with("dshot") {
            continue;
        }
        let anchor = records.iter().find_map(|w| match w {
            Record::MotorWitness {
                label: l,
                anchor_ns,
                ..
            } if l == label => *anchor_ns,
            _ => None,
        });
        let slot = anchors.entry(timer.as_str()).or_insert(None);
        *slot = slot.or(anchor);
    }
    let (mut past, mut missing) = (0, 0);
    for (timer, anchor) in anchors {
        let Some(at) = anchor else { continue };
        if dshot_window_past_run(at) {
            past += 1;
        } else if !records.iter().any(|r| {
            matches!(r, Record::JudgeStop { timer: t, anchor_ns, cr1_after: Some(c), .. }
                if t == timer && *anchor_ns == at && c & 1 == 0)
        }) {
            missing += 1;
        }
    }
    vec![
        (MOTOR_WINDOW_PAST_RUN_FINDING, past),
        (JUDGE_STOP_MISSING_FINDING, missing),
    ]
}

/// Kinds 25 and 26 each join one solved motor output (kind 24) by label,
/// every output has exactly one static check and at most one witness, and
/// exactly one in a booted run (`AET-WO-2302` A2-2), and a witness's
/// observer is its output's pin's (`AET-WO-4300a`).
fn check_motor_joins(records: &[Record], booted: bool) -> Result<(), &'static str> {
    let mut solved: BTreeMap<&str, &Record> = BTreeMap::new();
    for r in records {
        if let Record::MotorOutputSolved { label, .. } = r
            && solved.insert(label.as_str(), r).is_some()
        {
            return Err("a motor output is solved twice");
        }
    }
    // Each witness's window, from the records (`AET-WO-2302`): it starts at
    // its anchor, which all outputs of a timer share, and ends
    // `DSHOT_WINDOW_NS` after it for DShot and at the boot's end, 100 ms on
    // every board, for PWM and without an anchor. Kind 10's instruction
    // count is not tied to it: a droop's boot counts from the last reset's
    // release (third adversarial review, F1).
    let mut timer_anchor: BTreeMap<&str, Option<u64>> = BTreeMap::new();
    let mut timer_rises: BTreeMap<&str, u64> = BTreeMap::new();
    // Each output's timer kernel clock, from its kind 25: the frequency the
    // twin builds the timer at, and so its witness's timing (trace 14).
    let kernel: BTreeMap<&str, u64> = records
        .iter()
        .filter_map(|r| match r {
            Record::MotorTimingCheck {
                label, kernel_hz, ..
            } => Some((label.as_str(), *kernel_hz)),
            _ => None,
        })
        .collect();
    let mut checked: BTreeSet<&str> = BTreeSet::new();
    let mut witnessed: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        match r {
            Record::MotorTimingCheck { label, .. } => {
                let Some(m) = solved.get(label.as_str()) else {
                    return Err("a motor timing check names no solved output");
                };
                if !checked.insert(label) {
                    return Err("a motor timing check names one output twice");
                }
                if silicon_check(m, r) != Some(true) {
                    return Err("a motor timing check is not what its solved output gives");
                }
            }
            Record::MotorWitness {
                label, observer, ..
            } => {
                let Some(m @ Record::MotorOutputSolved { pin, .. }) =
                    solved.get(label.as_str()).copied()
                else {
                    return Err("a motor witness names no solved output");
                };
                let Record::MotorOutputSolved {
                    timer, protocol, ..
                } = m
                else {
                    unreachable!("kind 24 by its label");
                };
                let Record::MotorWitness {
                    anchor_ns,
                    end_ns,
                    first_ns,
                    cen_rises,
                    ..
                } = r
                else {
                    unreachable!("matched");
                };
                let window = match (anchor_ns, protocol.starts_with("dshot")) {
                    (Some(a), true) => a.checked_add(TWIN_DSHOT_WINDOW_NS),
                    _ => Some(TWIN_MAX_VIRTUAL_NS),
                };
                if window != Some(*end_ns)
                    || anchor_ns.is_some_and(|a| a >= *end_ns)
                    || first_ns.zip(*anchor_ns).is_some_and(|(f, a)| f < a)
                {
                    return Err(
                        "a motor witness's window is not its anchor's, or its first rise is before it",
                    );
                }
                if *timer_anchor.entry(timer.as_str()).or_insert(*anchor_ns) != *anchor_ns
                    || *timer_rises.entry(timer.as_str()).or_insert(*cen_rises) != *cen_rises
                {
                    return Err("two outputs of one timer name different anchors or CEN rises");
                }
                let Some(&kernel_hz) = kernel.get(label.as_str()) else {
                    return Err("a solved motor output has no static timing check");
                };
                if !witness_consistent(m, kernel_hz, r) {
                    return Err(
                        "a motor witness's status is not what its counts and its output's timing give",
                    );
                }
                // belt-and-suspenders: the twin's `timer::observer_name`
                // states this path; replay checks an archive without the
                // twin, so it states it again.
                let want = split_pin(pin).map(|(port, n)| {
                    format!("gpioPort{port}.p{}{n}obs", port.to_ascii_lowercase())
                });
                if want.as_deref() != Some(observer.as_str()) || !witnessed.insert(label) {
                    return Err(
                        "a motor witness is not its output's pin's observer, or is there twice",
                    );
                }
            }
            _ => {}
        }
    }
    if checked.len() != solved.len() {
        return Err("a solved motor output has no static timing check");
    }
    if booted && witnessed.len() != solved.len() {
        return Err("a booted run's solved motor output has no witness");
    }
    // Each judge's stop (kind 30): a DShot timer's, once, at its witnesses'
    // anchor plus `DSHOT_WINDOW_NS`, inside the boot.
    let mut stopped: BTreeSet<&str> = BTreeSet::new();
    for r in records {
        let Record::JudgeStop {
            timer,
            anchor_ns,
            at_ns,
            ..
        } = r
        else {
            continue;
        };
        let dshot = solved.values().any(|m| {
            matches!(m, Record::MotorOutputSolved { timer: t, protocol, .. }
                if t == timer && protocol.starts_with("dshot"))
        });
        if !dshot || !stopped.insert(timer.as_str()) {
            return Err("a judge's stop names no DShot timer, or one twice");
        }
        if timer_anchor.get(timer.as_str()) != Some(&Some(*anchor_ns))
            || anchor_ns.checked_add(TWIN_DSHOT_WINDOW_NS) != Some(*at_ns)
            || *at_ns > TWIN_MAX_VIRTUAL_NS
        {
            return Err(
                "a judge's stop is not at its timer's witnesses' anchor plus DSHOT_WINDOW_NS, inside the boot",
            );
        }
        check_stop_read_back(records, r)?;
    }
    Ok(())
}

/// Kind 30 `stop`'s read-back is the run's own (`AET-WO-2302` adversarial
/// pass 2, A2-1): exactly the timer's solved registers kind 11 lists (each
/// named `TIMn.` and the register), in kind 11's order, each with kind 11's
/// address, mask and expected value. The one difference is the judge's own
/// act: a stop that left `CEN` cleared makes kind 11's `CR1` expect `CEN`
/// clear after the boot, while the stop read `CR1` back against the solved
/// value, `CEN` set, before it cleared `CEN`.
fn check_stop_read_back(records: &[Record], stop: &Record) -> Result<(), &'static str> {
    let Record::JudgeStop {
        timer,
        read_back,
        cr1_after,
        ..
    } = stop
    else {
        return Ok(());
    };
    let prefix = format!("{timer}.");
    let cr1 = format!("{timer}.CR1");
    let solved: Vec<(&str, &Reading)> = records
        .iter()
        .filter_map(|r| match r {
            Record::RegisterObserved { name, reading } if name.starts_with(&prefix) => {
                Some((name.as_str(), reading))
            }
            _ => None,
        })
        .collect();
    if solved.len() != read_back.len() {
        return Err(
            "a judge's stop does not read back exactly its timer's solved registers that kind 11 lists",
        );
    }
    let stopped = cr1_after.is_some_and(|c| c & 1 == 0);
    for ((name, k11), got) in solved.iter().zip(read_back) {
        if got.address != k11.address || got.mask != k11.mask {
            return Err(
                "a judge's stop reads a register back at an address or under a mask that is not kind 11's for it",
            );
        }
        // A stop follows its timer's anchor, a `CEN` rise to the solved
        // `CR1`, which sets `CEN`: the stop expects it set, kind 11 cleared.
        let same = if *name == cr1 && stopped {
            got.expected & 1 == 1 && got.expected & !1 == k11.expected
        } else {
            got.expected == k11.expected
        };
        if !same {
            return Err(
                "a judge's stop reads a register back against a value that is not kind 11's for it",
            );
        }
    }
    Ok(())
}

/// The stamp grain (10 ns) and the quantum (100 µs) the judge witnesses
/// with.
// belt-and-suspenders: `aethelgard-cli`'s `STAMP_GRAIN_NS` and
// `aethelgard_twin::boot::QUANTUM_NS` state these; replay checks an archive
// without the twin or the CLI, so it states them again.
const TWIN_GRAIN_NS: u64 = 10;
const TWIN_QUANTUM_NS: u64 = 100_000;

/// Every board's boot, 100 ms, and a DShot output's window from its anchor,
/// 2 ms (`AET-WO-2302`); and every timer's kernel clock, the reset clock's
/// 64 MHz (RM0433 Table 57), the only clock a solved topology admits.
// belt-and-suspenders: `aethelgard_twin::boot::{MAX_VIRTUAL_NS,
// DSHOT_WINDOW_NS}` and `aethelgard_ir::SolvedClock::reset` state
// these; replay checks an archive without the twin or `aethelgard-ir`
// (RUL-16), so it states them again.
pub const TWIN_MAX_VIRTUAL_NS: u64 = 100_000_000;
/// A DShot output's window from its anchor, 2 ms (`AET-WO-2302`), as
/// [`TWIN_MAX_VIRTUAL_NS`] states it.
pub const TWIN_DSHOT_WINDOW_NS: u64 = 2_000_000;

/// Whether a DShot window from `anchor` ends past the run's end.
pub fn dshot_window_past_run(anchor: u64) -> bool {
    anchor.saturating_add(TWIN_DSHOT_WINDOW_NS) > TWIN_MAX_VIRTUAL_NS
}
const RESET_KERNEL_HZ: u64 = 64_000_000;

/// Whether kind 25 `check` is exactly the static check of kind 24 `m` on its
/// own kernel clock: the figures and `holds`, re-derived (adversarial pass:
/// a forged `holds` beside its own failing figures).
// belt-and-suspenders: `aethelgard_ir::motor::check_silicon` is the judge's
// check; replay never depends on `aethelgard-ir` (RUL-16), so it states the
// same exact-integer rules again.
fn silicon_check(m: &Record, check: &Record) -> Option<bool> {
    let (
        Record::MotorOutputSolved {
            protocol,
            psc,
            arr,
            ccr,
            pwm,
            idle,
            resolution_ps,
            ..
        },
        Record::MotorTimingCheck {
            kernel_hz,
            period_ps,
            high_ps,
            nominal_period_ps,
            nominal_high_ps,
            holds,
            ..
        },
    ) = (m, check)
    else {
        return None;
    };
    if *kernel_hz != RESET_KERNEL_HZ {
        return Some(false);
    }
    let kernel = u128::from(*kernel_hz);
    let (psc1, arr1, ccr) = (u128::from(*psc) + 1, u128::from(*arr) + 1, u128::from(*ccr));
    let ps = |cycles: u128| u64::try_from(cycles * 1_000_000_000_000 / kernel.max(1)).ok();
    let bit_rate = match protocol.as_str() {
        "dshot300" => Some(300_000u128),
        "dshot600" => Some(600_000),
        _ => None,
    };
    let (want_holds, nominal, nominal_high) = match (bit_rate, pwm) {
        (Some(rate), None) => {
            let period_ok = (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
            let high_ok = (8 * ccr * psc1 * rate).abs_diff(3 * kernel) <= 4 * psc1 * rate;
            let n = 1_000_000_000_000 / rate;
            (period_ok && high_ok, n, n * 3 / 8)
        }
        // As `aethelgard_ir::motor::check_silicon` judges it since
        // `AET-WO-1211a`: the declared tick exactly, and the declared idle
        // (CCR 0 for low, ARR + 1 for high, else the pulse), from kind 24's
        // own fields (trace 18).
        (None, Some((rate, _, hi))) => {
            let (Some(idle), Some(resolution_ps)) = (idle, resolution_ps) else {
                return Some(false);
            };
            let rate = u128::from(*rate);
            let period_ok =
                rate > 0 && (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
            let tick_ok = psc1 * 1_000_000_000_000 == u128::from(*resolution_ps) * kernel;
            let (high_ok, nominal_high) = match idle {
                record::MotorIdle::Low => (ccr == 0, 0),
                record::MotorIdle::High => (ccr == arr1, 1_000_000_000_000 / rate.max(1)),
                record::MotorIdle::PulseUs(us) => (
                    ccr * psc1 * 1_000_000 == u128::from(*us) * kernel,
                    u128::from(*us) * 1_000_000,
                ),
            };
            let max_ok = u128::from(*hi) * kernel < arr1 * psc1 * 1_000_000;
            (
                period_ok && tick_ok && high_ok && max_ok,
                1_000_000_000_000 / rate.max(1),
                nominal_high,
            )
        }
        _ => return Some(false),
    };
    Some(
        ps(arr1 * psc1) == Some(*period_ps)
            && ps(ccr * psc1) == Some(*high_ps)
            && u64::try_from(nominal).ok() == Some(*nominal_period_ps)
            && u64::try_from(nominal_high).ok() == Some(*nominal_high_ps)
            && want_holds == *holds,
    )
}

/// Whether kind 26 `w` is consistent with kind 24 `m` at its timer's kernel
/// clock `kernel_hz` (kind 25's) and with its own counts: the model's timing
/// is the output's, `ARR` and `CCR` ticks of `PSC + 1` cycles each as
/// [`realized_ns`] gives it (`AET-WO-4400g` §1.3), and a status is one its counts allow. A passing status
/// needs two rises or more, alternating counts, no shared stamp, a spread
/// under the grain, the last rise within a period and a grain of the
/// window's end, and edges off the grid exactly when `witnessed`.
fn witness_consistent(m: &Record, kernel_hz: u64, w: &Record) -> bool {
    let (
        Record::MotorOutputSolved { psc, arr, ccr, .. },
        Record::MotorWitness {
            period_ns,
            high_ns,
            grain_ns,
            quantum_ns,
            status,
            rises,
            falls,
            first_ns,
            last_ns,
            end_ns,
            span_ns,
            off_grid,
            batched,
            anchor_ns,
            ..
        },
    ) = (m, w)
    else {
        return false;
    };
    let cycles = u64::from(*psc) + 1;
    let realized = |ticks: u32| realized_ns(u64::from(ticks) * cycles, kernel_hz);
    if Some(*period_ns) != realized(*arr)
        || Some(*high_ns) != realized(*ccr)
        || *grain_ns != TWIN_GRAIN_NS
        || *quantum_ns != TWIN_QUANTUM_NS
    {
        return false;
    }
    // The fields agree with each other (adversarial F3, on `a2c4fa4`): the
    // first rise no later than the last, which is no later than the end;
    // no more edges off the grid than edges; and a passing sequence's last
    // rise exactly `rises - 1` periods after its first, within its span.
    if first_ns.zip(*last_ns).is_some_and(|(f, l)| f > l)
        || last_ns.is_some_and(|l| l > *end_ns)
        || rises
            .checked_add(*falls)
            .is_none_or(|edges| *off_grid > edges)
    {
        return false;
    }
    let none = *rises == 0 && *falls == 0 && first_ns.is_none();
    let shaped = *rises >= 2
        && *batched == 0
        && first_ns.is_some()
        && span_ns.is_some_and(|s| s < *grain_ns);
    let covers =
        last_ns.is_some_and(|l| l.saturating_add(*period_ns).saturating_add(*grain_ns) >= *end_ns);
    let continuous = witness_continuous(w);
    let passing = shaped && continuous && covers;
    match status.as_str() {
        "witnessed" => passing && *off_grid > 0,
        "static_only" => passing && *off_grid == 0,
        "unwitnessed" => *rises < 2,
        "unreadable" => none,
        "batched" => *rises >= 2 && *batched > 0,
        // The judge's `witness_motor` gives `stopped` only to edges on the
        // model's sequence, as a passing one (`AET-WO-2302b` ADV-M1).
        "stopped" => shaped && continuous && !covers,
        "unanchored" => {
            anchor_ns.is_none()
                && first_ns.is_none()
                && last_ns.is_none()
                && span_ns.is_none()
                && rises.checked_add(*falls).is_some_and(|e| e > 0)
        }
        // An unreadable log, or edges off the model's sequence.
        "mismatch" => none || *rises >= 2,
        // Judged by its kind 37, whatever its counts (`AET-WO-1211c`); the
        // timing and window above still bind it.
        "level" => true,
        _ => false,
    }
}

/// Whether kind 26 `w`'s edges are continuous on the model's sequence, as
/// the judge's `witness_motor` gives every status past `mismatch`: its last
/// rise exactly `rises - 1` periods after its first, within its span (a
/// rise every period between them), and its edges alternating (as many
/// falls as rises, or one fewer). `false` for any other record.
pub fn witness_continuous(w: &Record) -> bool {
    let Record::MotorWitness {
        period_ns,
        rises,
        falls,
        first_ns,
        last_ns,
        span_ns,
        ..
    } = w
    else {
        return false;
    };
    // Checked: a forged count must not wrap onto the honest spacing, nor
    // panic (third adversarial review, F2).
    let spaced = first_ns
        .zip(*last_ns)
        .zip(*span_ns)
        .is_some_and(|((f, l), s)| {
            rises
                .checked_sub(1)
                .and_then(|n| n.checked_mul(*period_ns))
                .is_some_and(|due| l.checked_sub(f).is_some_and(|d| d.abs_diff(due) <= s))
        });
    let alternating = *falls == *rises || rises.checked_sub(1) == Some(*falls);
    spaced && alternating
}

fn check_joins(records: &[Record], booted: bool) -> Result<(), &'static str> {
    check_motor_joins(records, booted)?;
    /// A kind 15's bus, peripheral, lines and devices.
    type Solved<'a> = (&'a str, &'a str, &'a [BusLine], &'a [(String, u8)]);
    let solved: Vec<Solved> = records
        .iter()
        .filter_map(|r| match r {
            Record::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => Some((bus.as_str(), peripheral.as_str(), &lines[..], &devices[..])),
            _ => None,
        })
        .collect();
    if !solved.windows(2).all(|w| w[0].0 < w[1].0) {
        return Err("the solved buses are not sorted by unique bus");
    }
    // No net, pin or controller belongs to two buses, and no two devices on
    // one bus share an address (`AET-WO-3100` adversarial A6).
    fn unique<T: Ord>(items: Vec<T>) -> bool {
        let n = items.len();
        items.into_iter().collect::<BTreeSet<_>>().len() == n
    }
    let lines = || solved.iter().flat_map(|b| b.2.iter());
    if !unique(lines().map(|l| l.net.as_str()).collect())
        || !unique(lines().map(|l| l.pin.as_str()).collect())
        || !unique(solved.iter().map(|b| b.1).collect())
    {
        return Err("two solved buses share a net, a pin or a controller");
    }
    if !solved
        .iter()
        .all(|b| unique(b.3.iter().map(|d| d.1).collect()))
    {
        return Err("two devices on one solved bus share an address");
    }
    // One measurement per line, one read per step of a device's script and
    // one witness per step, as the campaign writes them (adversarial A2;
    // `AET-WO-2204`: a bus with two devices has two witnesses;
    // `AET-WO-4210`: a device's steps run identity, configure, data, each
    // at most once, the first being identity; `AET-WO-4220`: several
    // configurations each by its own name, in order).
    let mut seen: BTreeSet<(u8, &str, &str)> = BTreeSet::new();
    let mut last_step: BTreeMap<(u8, &str), usize> = BTreeMap::new();
    for r in records {
        let keys: Vec<(u8, &str, &str)> = match r {
            Record::RiseComputed { bus, net, .. } | Record::LineStuckHigh { bus, net, .. } => {
                vec![(0, bus, net)]
            }
            Record::I2cCheck {
                reference, step, ..
            } => vec![(1, reference, step)],
            Record::I2cWitness {
                reference, step, ..
            } => vec![(2, reference, step)],
            Record::SpiCheck {
                reference, step, ..
            } => vec![(3, reference, step)],
            Record::SpiWitness {
                reference, step, ..
            } => vec![(4, reference, step)],
            _ => vec![],
        };
        for key in keys {
            if !seen.insert(key) {
                return Err("a line measured twice, or a device step read or witnessed twice");
            }
            if key.0 != 0 {
                let Some(index) = step_index(key.2) else {
                    return Err("a step is not identity, a configuration or data");
                };
                let previous = last_step.insert((key.0, key.1), index);
                if previous.map_or(index != 0, |p| index <= p) {
                    return Err("a device's steps are not in the order identity, configure, data");
                }
            }
        }
    }
    let bus = |id: &str| {
        solved
            .iter()
            .find(|b| b.0 == id)
            .ok_or("an analog record or a bus fault names a bus no kind 15 solves")
    };
    let line = |id: &str, net: &str| -> Result<&BusLine, &'static str> {
        bus(id)?
            .2
            .iter()
            .find(|l| l.net == net)
            .ok_or("an analog record or a bus fault names a net that is not a line of its bus")
    };
    // Each device on exactly one bus: (bus's peripheral, address).
    let mut device: BTreeMap<&str, (&str, u8)> = BTreeMap::new();
    for (_, peripheral, _, devices) in &solved {
        for (reference, address) in *devices {
            if device
                .insert(reference.as_str(), (*peripheral, *address))
                .is_some()
            {
                return Err("a device is on two solved buses");
            }
        }
    }
    for r in records {
        match r {
            Record::RiseComputed {
                bus,
                net,
                ceiling_ps,
                ..
            }
            | Record::LineStuckHigh {
                bus,
                net,
                ceiling_ps,
                ..
            } => {
                if line(bus, net)?.ceiling_ps != *ceiling_ps {
                    return Err("an analog record's ceiling is not its solved line's");
                }
            }
            Record::BusFault { bus, nets, .. } => {
                for net in nets {
                    line(bus, net)?;
                }
            }
            Record::I2cCheck {
                reference, address, ..
            } => {
                if device.get(reference.as_str()).map(|d| d.1) != Some(*address) {
                    return Err("an identity read names no device of a solved bus at its address");
                }
            }
            Record::I2cWitness {
                peripheral,
                reference,
                device: path,
                ..
            } => match device.get(reference.as_str()) {
                None => return Err("a witness names no device of a solved bus"),
                Some((p, _)) if p != peripheral => {
                    return Err("a witness is not on its device's solved bus");
                }
                Some(_) if !path.ends_with(&format!(".{}", reference.to_ascii_lowercase())) => {
                    return Err("a witness's model path is not its device's emitted name");
                }
                Some(_) => {}
            },
            _ => {}
        }
    }
    // Each step's read is witnessed, and each witness is of a step's read,
    // joined on (reference, step) (`AET-WO-2204` D1, `AET-WO-4210`).
    let named = |witness: bool| -> BTreeSet<(&str, &str)> {
        records
            .iter()
            .filter_map(|r| match r {
                Record::I2cCheck {
                    reference, step, ..
                } if !witness => Some((reference.as_str(), step.as_str())),
                Record::I2cWitness {
                    reference, step, ..
                } if witness => Some((reference.as_str(), step.as_str())),
                _ => None,
            })
            .collect()
    };
    if named(false) != named(true) {
        return Err("a step's read without its witness, or a witness without its read");
    }
    // The same for SPI (`AET-WO-4200`, adversarial A3): each read joins its
    // solved device (kind 23) at its address and chip select, is witnessed
    // once under its own name behind its controller's multiplexer, and every
    // multiplexer line a witness carries is that multiplexer's; an absent
    // device is a solved one, and its model logged nothing; and no device is
    // both an I2C and an SPI one.
    let mut solved_spi: BTreeMap<&str, (&str, u8, &str)> = BTreeMap::new();
    for r in records {
        if let Record::SpiBusSolved {
            peripheral,
            devices,
        } = r
        {
            for (reference, address, select) in devices {
                if solved_spi
                    .insert(
                        reference.as_str(),
                        (peripheral.as_str(), *address, select.as_str()),
                    )
                    .is_some()
                {
                    return Err("an SPI device is on two solved controllers");
                }
            }
        }
    }
    let mut spi: BTreeMap<&str, &str> = BTreeMap::new();
    let mut spi_steps: BTreeSet<(&str, &str)> = BTreeSet::new();
    for r in records {
        if let Record::SpiCheck {
            reference,
            peripheral,
            select,
            address,
            step,
            ..
        } = r
        {
            if solved_spi.get(reference.as_str())
                != Some(&(peripheral.as_str(), *address, select.as_str()))
            {
                return Err("an SPI read names no solved device at its address and chip select");
            }
            spi.insert(reference, peripheral);
            spi_steps.insert((reference, step));
        }
    }
    let absent: BTreeSet<&str> = records
        .iter()
        .filter_map(|r| match r {
            Record::Scenario { overrides, .. } => Some(overrides),
            _ => None,
        })
        .flatten()
        .filter_map(|o| match o {
            Override::SpiDeviceAbsent { reference } => Some(reference.as_str()),
            _ => None,
        })
        .collect();
    if absent.iter().any(|r| !solved_spi.contains_key(r)) {
        return Err("an absent SPI device is no solved one");
    }
    let mut witnessed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for r in records {
        let Record::SpiWitness {
            peripheral,
            reference,
            device: path,
            step,
            observed,
            unrouted,
            ..
        } = r
        else {
            continue;
        };
        // belt-and-suspenders: the twin's `platform::spi_device_path` states
        // this path; replay checks an archive without the twin, so it states
        // it again (this crate never depends on the twin).
        let c = peripheral.to_ascii_lowercase();
        let mux = format!("{c}.{c}mux");
        let want = format!("{mux}.{}", reference.to_ascii_lowercase());
        if spi.get(reference.as_str()) != Some(&peripheral.as_str()) || *path != want {
            return Err(
                "an SPI witness is not its read's device behind its controller's multiplexer",
            );
        }
        let line = format!("[WARNING] {mux}: Tried to transmit byte 0x");
        if !unrouted.iter().all(|l| l.starts_with(&line)) {
            return Err("an SPI witness carries a line that is not its multiplexer's");
        }
        if absent.contains(reference.as_str()) && observed.as_ref().is_some_and(|o| !o.is_empty()) {
            return Err("an absent SPI device's model logged a transfer");
        }
        witnessed.insert((reference, step));
    }
    if witnessed != spi_steps || spi.keys().any(|r| device.contains_key(r)) {
        return Err("an SPI read without its witness, or a device on two buses");
    }
    Ok(())
}

/// The header artifacts of a campaign run, in the header's sorted order.
pub const ARTIFACTS: [&str; 5] = ["base", "firmware", "scenario", "specification", "topology"];

/// The header's tool names (`AET-WO-2205` §1.1): the engine and Renode judge
/// every run; the bridge only a run that computed a line or a rail (kinds 7,
/// 8 or 16), since it runs only then; and ngspice only a run with an answer
/// that named it, a line (kinds 7 and 8) or a crossing (kind 17).
pub const ENGINE: &str = "aethelgard";
pub const RENODE: &str = "renode";
pub const BRIDGE: &str = "aethelgard-bridge";
pub const NGSPICE: &str = "ngspice";
/// The Rust toolchain on the host that ran the campaign (`AET-WO-2310`, ruled
/// Q-T T1): `rustc -V`'s version and a digest of its whole active sysroot.
/// It is the host's toolchain, not proven to be the one that built the ELF.
pub const RUST_TOOLCHAIN: &str = "rust-toolchain";
/// [`RUST_TOOLCHAIN`]'s version on a host with none, with no hash.
pub const TOOLCHAIN_ABSENT: &str = "absent";

/// The tools a campaign run's header names, sorted: every run names the
/// host's Rust toolchain too (`AET-WO-2310`).
pub fn tools_of(records: &[Record]) -> Vec<&'static str> {
    fn line(r: &Record) -> bool {
        matches!(
            r,
            Record::RiseComputed { .. } | Record::LineStuckHigh { .. }
        )
    }
    let any = |f: fn(&Record) -> bool| records.iter().any(f);
    let mut tools = vec![ENGINE];
    // Kind 31 carries the deck's solved minimum, so the bridge and ngspice
    // both judged it (`AET-WO-2304b` §1.9).
    fn sag(r: &Record) -> bool {
        matches!(r, Record::BatterySagDeclared { .. })
    }
    if any(|r| line(r) || sag(r) || matches!(r, Record::RailDeclared { .. })) {
        tools.push(BRIDGE);
    }
    if any(|r| line(r) || sag(r) || matches!(r, Record::RailCrossing { .. })) {
        tools.push(NGSPICE);
    }
    tools.push(RENODE);
    tools.push(RUST_TOOLCHAIN);
    tools
}

/// Judge an archived campaign run, after the strict reader: it holds no
/// event but campaign records, its header names exactly what its records
/// name, and its `run_id` is the one those hashes give (`AET-WO-2300`
/// adversarial F4, F5). Then [`check_run`].
pub fn check_archived(header: &Header, state: &State) -> Result<RunSummary, &'static str> {
    let raw = !state.registers.is_empty()
        || !state.pins.is_empty()
        || !state.interrupts.is_empty()
        || !state.nets.is_empty()
        || state.faults != 0;
    if raw {
        return Err("a campaign run holds an event that is not a campaign record");
    }
    let summary = check_run(&state.records)?;
    let names: Vec<&str> = header.artifacts.iter().map(|(n, _)| n.as_str()).collect();
    if names != ARTIFACTS {
        return Err("the header's artifacts are not a campaign run's");
    }
    let tools: Vec<&str> = header.tools.iter().map(|(n, _)| n.as_str()).collect();
    if tools != tools_of(&state.records) {
        return Err("the header's tools are not the ones its records say judged the run");
    }
    // The content hashes (`AET-WO-2310`): ngspice's library whenever ngspice
    // judged the run, and the toolchain's sysroot unless the host had none;
    // no other tool's.
    let hashed: Vec<&str> = header.tool_sha256.iter().map(|(n, _)| n.as_str()).collect();
    let want: Vec<&str> = header
        .tools
        .iter()
        .filter(|(n, v)| n == NGSPICE || (n == RUST_TOOLCHAIN && v != TOOLCHAIN_ABSENT))
        .map(|(n, _)| n.as_str())
        .collect();
    if hashed != want {
        return Err(
            "the header's tool hashes are not ngspice's library and the toolchain's sysroot",
        );
    }
    let hash = |i: usize| &header.artifacts[i].1;
    // The topology follows the scenario, and its expected counts if any.
    let solved = state
        .records
        .iter()
        .find(|r| matches!(r, Record::Solved { .. }));
    let (
        Record::Scenario { sha256, .. },
        Some(Record::Solved {
            specification_sha256,
            topology_sha256,
        }),
    ) = (&state.records[0], solved)
    else {
        unreachable!("check_run puts the scenario first and holds one topology");
    };
    if hash(2) != sha256 || hash(3) != specification_sha256 || hash(4) != topology_sha256 {
        return Err("the header's hashes are not the ones its records name");
    }
    if run_id(hash(2), hash(3), hash(1)) != header.run_id {
        return Err("the run id is not the one its scenario, specification and firmware give");
    }
    check_serial_frames(hash(1), &state.records)?;
    Ok(summary)
}
