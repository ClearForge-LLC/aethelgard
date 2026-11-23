//! Every specification version's changes, one row per version from 2 to
//! [`SPECIFICATION_SCHEMA_VERSION`] (`AET-WO-1215`): the work order that made
//! it, and the key paths a file of the version before must gain, may gain,
//! must rename or must drop, and the refusals it added that judge content an
//! older file could already hold. This crate's README states each row in
//! prose; the table is the same fact for a program, cited row by row to it.
//!
//! A file of an older version is refused (`UnsupportedSchemaVersion`) with
//! every row after its own, so its author sees every change at once. Nothing
//! is migrated and no value is filled (`INV-06`): a row names paths, never a
//! value. A path is `.`-separated keys, `[]` for each element of an array,
//! and a tag of an externally tagged object as a key
//! (`mcu.hse.crystal.startup_max_us`). A required path is required wherever
//! its parent is present: `mcu.foreign_clock.vos` only under a declared
//! `mcu.foreign_clock`.
//!
//! The bump's work order writes its row by hand; a test fails CI when the
//! last row is not [`SPECIFICATION_SCHEMA_VERSION`].

use std::fmt;

use serde::Serialize;

use crate::SPECIFICATION_SCHEMA_VERSION;

/// A key a version renamed: `path` is refused now, and `by` is its
/// replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Replacement {
    pub path: &'static str,
    pub by: &'static str,
}

/// One version's changes against the version before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VersionChange {
    pub version: u32,
    /// The work order that made the version, as the README names it.
    pub work_order: &'static str,
    /// Keys new at this version that a file must carry.
    pub requires: &'static [&'static str],
    /// Keys new at this version that a file may leave absent.
    pub optional: &'static [&'static str],
    /// Keys of the version before that this one renamed.
    pub replaced: &'static [Replacement],
    /// Keys of the version before that this one no longer admits.
    pub removed: &'static [&'static str],
    /// Refusal kinds this version added that judge content a file of the
    /// version before could already hold.
    pub rules: &'static [&'static str],
}

/// Every version from 2, in order (version 1 is the first; it changed
/// nothing). README `## Specification`, the paragraph that starts "Version 2",
/// is the prose for each row.
pub const CHANGES: &[VersionChange] = &[
    row(2, "AET-WO-4100", &["board"], &[], &[], &[]),
    row(
        3,
        "AET-WO-4300c",
        &[],
        &[],
        &[],
        &[
            "MissingSpiConstraint",
            "SupplyOutsideSpeedTable",
            "AmbiguousPartName",
            "PartNameCase",
            "SupplyOutsidePart",
            "PartOnWrongBus",
            "I2cAddressNotOffered",
            "SpiClockAbovePart",
            "UnjudgedSpiConstraint",
        ],
    ),
    row(4, "AET-WO-4300f", &["connectors"], &[], &[], &[]),
    row(
        5,
        "AET-WO-1308",
        &["net_classes", "diff_pairs"],
        &[],
        &[],
        &[],
    ),
    row(
        6,
        "AET-WO-4300a",
        &["mcu.hse_hz", "motor_outputs"],
        &[],
        &[],
        &["InvalidIdentifier"],
    ),
    row(
        7,
        "AET-WO-4300d",
        &["mcu.bor_level", "power"],
        &[],
        &[],
        &[],
    ),
    row(
        8,
        "AET-WO-4210",
        &["accept_lifecycle"],
        &[],
        &[],
        &[
            "IoOutsidePart",
            "LifecycleNotAccepted",
            "LifecycleStatusMismatch",
        ],
    ),
    row(9, "AET-WO-4300b", &["serial_ports"], &[], &[], &[]),
    row(
        10,
        "AET-WO-4300d3",
        &[
            "power.conditions.dc_bias_derating",
            "power.input.hot_plug",
            "power.regulators[].junction",
            "power.regulators[].controls",
            "power.crossings[].controls",
        ],
        &[],
        &[],
        &[
            "SupplyFloorBelowBorRising",
            "EffectiveCapacitanceBelowMinimum",
            "DcBiasBeyondCurve",
            "BatteryInputUndamped",
            "ControlPinUnwired",
            "BuckJunctionOverLimit",
        ],
    ),
    row(
        11,
        "AET-WO-4300g",
        &["mcu.supply_source"],
        &[],
        &[],
        &[
            "NoProgrammingPath",
            "SupplyHasNoSource",
            "DuplicateNetName",
            "NetShadowsDomain",
        ],
    ),
    row(
        12,
        "AET-WO-4230",
        &[
            "serial_ports[].peer_deviation_ppm",
            "serial_ports[].peer_deviation_source",
            "acknowledge_unwitnessed",
        ],
        &[],
        &[],
        &[
            "SerialBaudTolerance",
            "SpiSpeedBelowKernelRange",
            "SpiKernelClockUnavailable",
            "PullUpBelowSinkFloor",
        ],
    ),
    row(
        13,
        "AET-WO-4220b",
        &[
            "i2c_devices[].pin_capacitance",
            "spi_devices[].pin_capacitance",
        ],
        &[],
        &[],
        &[],
    ),
    row(
        14,
        "AET-WO-1312",
        &[],
        &["test_points", "mcu.reset_switch"],
        &[],
        &["DuplicateNetName", "NetShadowsDomain"],
    ),
    row(15, "AET-WO-1313", &[], &["board.max_outline_um"], &[], &[]),
    row(
        16,
        "AET-WO-4400d",
        &[],
        &["board.forward", "spi_devices[].orientation"],
        &[],
        &[],
    ),
    row(17, "AET-WO-4400g", &[], &["mcu.foreign_clock"], &[], &[]),
    row(
        18,
        "AET-WO-2304b",
        &["power.battery.series_milliohms", "motor_outputs[].esc"],
        &[],
        &[],
        &[],
    ),
    row(
        19,
        "AET-WO-4240v",
        &["mcu.core_supply", "mcu.foreign_clock.vos"],
        &[],
        &[],
        &[],
    ),
    row(
        20,
        "AET-WO-4240a",
        &["mcu.hse"],
        &[],
        &[Replacement {
            path: "mcu.hse_hz",
            by: "mcu.hse",
        }],
        &[],
    ),
    row(
        21,
        "AET-WO-1210",
        &[],
        &["board.drawn"],
        &[],
        &["UnboundedPinCapacitance", "PartNotDrawable"],
    ),
    row(
        22,
        "AET-WO-1211a",
        &[
            "motor_outputs[].pwm.idle",
            "motor_outputs[].pwm.resolution_ps",
        ],
        &["pin_devices"],
        &[],
        &[],
    ),
    row(
        23,
        "AET-WO-4240b",
        &[
            "mcu.hse.crystal.startup_max_us",
            "mcu.hse.crystal.startup_source",
            "mcu.hse.external.startup_max_us",
            "mcu.hse.external.startup_source",
        ],
        &[],
        &[],
        &["SpiSpeedBelowKernelRange", "SerialBaudReadAsKbaud"],
    ),
    row(
        24,
        "AET-WO-1211e",
        &["pin_devices[].bulk"],
        &[],
        &[],
        &[
            "PullAboveMcuTolerance",
            "McuPinStructureUnheld",
            "TripReferenceGrounded",
            "TripSenseUnbound",
            "TripBelowFullScale",
        ],
    ),
    row(25, "AET-WO-1216b", &[], &[], &[], &["OffDomainAboveLimit"]),
    row(
        26,
        "AET-WO-1217",
        &[],
        &[],
        &[],
        &["AnalogPadPin", "PowerPartUnknown"],
    ),
    row(27, "AET-WO-1218", &[], &[], &[], &["PowerPartUnknown"]),
    row(
        28,
        "AET-WO-4300i",
        &["motor_outputs[].pull", "mcu.reserved_pins"],
        &[],
        &[],
        &[
            "ConnectorPinNotFt",
            "McuPinStructureUnheld",
            "MotorPinResetPull",
            "PowerPartUnknown",
        ],
    ),
    row(
        29,
        "AET-WO-4300j",
        &["motor_outputs[].pull.link", "motor_outputs[].pull.fitment"],
        &[],
        &[],
        &[
            "MotorPullOutsideBound",
            "MotorPullFloorUnreachable",
            "MotorPullBoundEmpty",
            "PowerPartUnknown",
        ],
    ),
];

/// A row with nothing removed; `replaced` is the only other column a version
/// has used (20).
const fn row(
    version: u32,
    work_order: &'static str,
    requires: &'static [&'static str],
    optional: &'static [&'static str],
    replaced: &'static [Replacement],
    rules: &'static [&'static str],
) -> VersionChange {
    VersionChange {
        version,
        work_order,
        requires,
        optional,
        replaced,
        removed: &[],
        rules,
    }
}

/// The rows a file of `version` lacks: every row after its own, in order.
/// Empty for the current version, a newer one, and 0, which never existed.
pub fn lacked_by(version: u32) -> &'static [VersionChange] {
    // Every row is a version above 1, so a current or newer file lacks none.
    if version == 0 {
        return &[];
    }
    let first = CHANGES.partition_point(|c| c.version <= version);
    &CHANGES[first..]
}

/// The refusal of a specification of `version`, which is not the current
/// one, carrying every row it lacks.
pub fn unsupported(version: u32) -> crate::IrValidationError {
    crate::IrValidationError::UnsupportedSchemaVersion {
        version,
        expected: SPECIFICATION_SCHEMA_VERSION,
        changes: lacked_by(version),
    }
}

/// The message's tail for `changes`: nothing when there are none, else each
/// row compactly, in order. Paths only; no value is suggested.
pub(crate) struct Lacking(pub &'static [VersionChange]);

impl fmt::Display for Lacking {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        write!(
            f,
            "; it lacks {} version change(s), each named by path, and nothing is filled:",
            self.0.len()
        )?;
        for (i, c) in self.0.iter().enumerate() {
            write!(
                f,
                "{} {} ({})",
                if i == 0 { "" } else { ";" },
                c.version,
                c.work_order
            )?;
            let mut parts = Vec::new();
            if !c.requires.is_empty() {
                parts.push(format!("requires [{}]", c.requires.join(", ")));
            }
            if !c.optional.is_empty() {
                parts.push(format!("optional [{}]", c.optional.join(", ")));
            }
            for r in c.replaced {
                parts.push(format!("{} replaced by {}", r.path, r.by));
            }
            if !c.removed.is_empty() {
                parts.push(format!("removes [{}]", c.removed.join(", ")));
            }
            if !c.rules.is_empty() {
                parts.push(format!("refuses by [{}]", c.rules.join(", ")));
            }
            write!(f, " {}", parts.join(", "))?;
        }
        Ok(())
    }
}
