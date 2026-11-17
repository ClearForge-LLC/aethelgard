//! Bench access (`AET-WO-1312`): test points on declared nets, `BOOT0` on a
//! connector pin, and the MCU's reset push-button.
//!
//! - A **test point** is a bare pad on one net a connector pin could name: a
//!   declared signal, a voltage domain, `GND`, or one of the MCU's debug and
//!   boot nets ([`crate::DEBUG_NETS`]). It is drawn, placed, routed and judged
//!   like any pad (`aethelgard_eda`).
//! - **`BOOT0`** is wired to `GND` by the MCU row's strap (RM0433 p. 138,
//!   Table 9). When a connector pin or a test point names it ([`carries_boot0`]),
//!   the emitter draws the pin on its own `BOOT0` net with a pull-down of
//!   [`BOOT0_PULL_DOWN_OHMS`] to `GND`, so a probe can drive it high and the
//!   board still boots from flash with nothing attached; [`judge_boot0_pull_down`]
//!   judges that value against DS12117. When nothing names it, the wire stays.
//! - **`mcu.reset_switch`** adds a push-button from `NRST` to `GND`, numbered
//!   [`switch_reference`].

use std::collections::BTreeMap;
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::names::reference_key;
use crate::{BOOT0, IrValidationError as E, Specification, object_only};

/// One test point: the net its pad is on and its reference (`TP1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct TestPoint {
    pub net: String,
    pub reference: String,
}

object_only!(TestPoint);

/// The BOOT0 pull-down the emitter draws when `BOOT0` is carried: 10 kΩ,
/// which [`judge_boot0_pull_down`] passes over the MCU's whole supply range.
pub const BOOT0_PULL_DOWN_OHMS: u32 = 10_000;

/// DS12117 Rev 10, Table 49 (pp. 136–137, read on the rendered page): the
/// BOOT0 pin's leakage, `VPP (BOOT0 alternate function)` at
/// `0 < VIN ≤ VDD`, at most 15 nA.
pub const BOOT0_LEAKAGE_NANOAMPS: u64 = 15;

/// The current a driver may source at its high level: DS12117 Rev 10,
/// Table 50 (p. 139), an I/O sources `IIO = 4 mA` at `VOH ≥ VDD − 0.4 V` over
/// `1.62 V ≤ VDD ≤ 3.6 V`. A probe on the MCU's supply that drives as the
/// MCU's own I/O does can hold BOOT0 high through the pull-down.
pub const DRIVER_MICROAMPS: u64 = 4_000;

/// The driver's drop below its supply at [`DRIVER_MICROAMPS`] (Table 50).
pub const DRIVER_DROP_MILLIVOLTS: u64 = 400;

/// BOOT0's `VIL` maximum at `vdd`, `0.19·VDD + 0.1 V` (Table 49), rounded down.
pub fn boot0_vil_max_millivolts(vdd: u64) -> u64 {
    vdd * 19 / 100 + 100
}

/// BOOT0's `VIH` minimum at `vdd`, `0.17·VDD + 0.6 V` (Table 49), rounded up.
pub fn boot0_vih_min_millivolts(vdd: u64) -> u64 {
    (vdd * 17).div_ceil(100) + 600
}

/// Judges a pull-down of `ohms` from BOOT0 to `GND` over the MCU's supply
/// range `vdd_min..=vdd_max` (millivolts), or says which bound fails:
/// 1. **unplugged**, the pin's leakage through the pull-down stays at most
///    `VIL` max at the lowest supply, where `VIL` max is least, so the board
///    boots from flash;
/// 2. **driven**, a driver on the supply holds the pin at `VDD − 0.4 V`, at
///    least `VIH` min, at the lowest supply, where the margin `0.83·VDD − 1 V`
///    is least, while it sources at most [`DRIVER_MICROAMPS`] into the
///    pull-down at the highest supply.
///
/// A wire (`ohms == 0`) fails 2: no driver holds it high.
pub fn judge_boot0_pull_down(ohms: u64, vdd_min: u64, vdd_max: u64) -> Result<(), String> {
    // 1: nA × Ω = nV; rounded up to millivolts. A product past `u64` is past
    // any VIL (adversarial F3: it wrapped, and a 1.2 EΩ value passed).
    let floating = BOOT0_LEAKAGE_NANOAMPS
        .checked_mul(ohms)
        .map_or(u64::MAX, |nv| nv.div_ceil(1_000_000));
    let vil = boot0_vil_max_millivolts(vdd_min);
    if floating > vil {
        return Err(format!(
            "unplugged, BOOT0 sits at up to {floating} mV ({BOOT0_LEAKAGE_NANOAMPS} nA through {ohms} ohms), above its VIL maximum of {vil} mV at {vdd_min} mV (DS12117 Table 49)"
        ));
    }
    // 2
    let high = vdd_min.saturating_sub(DRIVER_DROP_MILLIVOLTS);
    let vih = boot0_vih_min_millivolts(vdd_min);
    if high < vih {
        return Err(format!(
            "a driver at {high} mV is below BOOT0's VIH minimum of {vih} mV at {vdd_min} mV (DS12117 Tables 49 and 50)"
        ));
    }
    let microamps = vdd_max
        .saturating_mul(1_000)
        .checked_div(ohms)
        .unwrap_or(u64::MAX);
    if microamps > DRIVER_MICROAMPS {
        return Err(format!(
            "driving BOOT0 high at {vdd_max} mV sources {} uA into {ohms} ohms, above the {DRIVER_MICROAMPS} uA a driver holds VOH at (DS12117 Table 50)",
            if microamps == u64::MAX {
                "unbounded".to_owned()
            } else {
                microamps.to_string()
            }
        ));
    }
    Ok(())
}

/// Whether a declared connector pin or test point names `BOOT0`, so the
/// emitter draws it on its own net through [`BOOT0_PULL_DOWN_OHMS`].
pub fn carries_boot0(spec: &Specification) -> bool {
    spec.declared_connectors()
        .iter()
        .flat_map(|c| &c.pins)
        .any(|p| p.net.as_deref() == Some(BOOT0))
        || spec.declared_test_points().iter().any(|t| t.net == BOOT0)
}

/// The reset switch's reference when `mcu.reset_switch` is set: `SW<n>`, the
/// first `n` from 1 whose `SW<n>` no reference of the specification spells
/// exactly, as the MCU's `U<n>` is numbered ([`crate::names::mcu_reference`]).
pub fn switch_reference(spec: &Specification) -> Option<String> {
    if !spec.mcu.reset_switch {
        return None;
    }
    let spelled = crate::names::references(spec);
    (1..)
        .map(|n| format!("SW{n}"))
        .find(|r| !spelled.contains(&r.as_str()))
}

/// Whether the emitter numbers references of this KiCad key itself: `U`, `R`
/// and `C` followed by digits, as [`crate::connectors`] holds for a
/// connector (`ConnectorReferenceReserved`), and `SW` followed by digits for
/// the reset switch.
fn emitter_prefix(key: &str) -> bool {
    let stem = key.trim_end_matches(|c: char| c.is_ascii_digit());
    stem.len() < key.len() && matches!(stem, "U" | "R" | "C" | "SW")
}

/// Step 15a: each test point by document order. Its net must be one a
/// connector pin may name ([`crate::connectors::nameable`]); its reference
/// distinct, under KiCad's key, from every other reference the specification
/// declares and outside the emitter's numbering; and no net carries two.
pub(crate) fn validate(spec: &Specification) -> Result<(), E> {
    let nameable = crate::connectors::nameable(spec);
    // Every other reference: `names::references` lists the test points last.
    let all = crate::names::references(spec);
    let others = &all[..all.len() - spec.declared_test_points().len()];
    let mut by_key: BTreeMap<String, &str> =
        others.iter().map(|r| (reference_key(r), *r)).collect();
    let mut nets: BTreeMap<&str, &str> = BTreeMap::new();
    for t in spec.declared_test_points() {
        if !nameable.contains(t.net.as_str()) {
            return Err(E::TestPointNetUnknown {
                id: t.reference.clone(),
                net: t.net.clone(),
            });
        }
        let key = reference_key(&t.reference);
        if emitter_prefix(&key) {
            return Err(E::TestPointReferenceTaken {
                id: t.reference.clone(),
                other: "the emitter's own numbering (U, R, C or SW and a number)".into(),
            });
        }
        if let Some(other) = by_key.insert(key, &t.reference) {
            return Err(E::TestPointReferenceTaken {
                id: t.reference.clone(),
                other: format!("`{other}`"),
            });
        }
        if let Some(first) = nets.insert(t.net.as_str(), t.reference.as_str()) {
            return Err(E::TestPointNetRepeated {
                id: t.net.clone(),
                first: first.to_owned(),
                second: t.reference.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The MCU's supply range, DS12117 Table 111 (`power-parts.json`).
    const VDD: (u64, u64) = (1_620, 3_600);

    #[test]
    fn the_pull_down_passes_over_the_whole_supply_range() {
        assert_eq!(
            judge_boot0_pull_down(u64::from(BOOT0_PULL_DOWN_OHMS), VDD.0, VDD.1),
            Ok(())
        );
    }

    #[test]
    fn a_wire_or_a_value_out_of_range_fails_by_its_bound() {
        let wire = judge_boot0_pull_down(0, VDD.0, VDD.1).unwrap_err();
        assert!(wire.contains("unbounded"), "{wire}");
        // 3.6 V / 4 mA = 900 ohms is the least a driver holds.
        assert!(judge_boot0_pull_down(900, VDD.0, VDD.1).is_ok());
        assert!(
            judge_boot0_pull_down(899, VDD.0, VDD.1)
                .unwrap_err()
                .contains("4000 uA")
        );
        // VIL max at 1.62 V is 407 mV; 15 nA reaches it at 27.1 Mohm.
        assert_eq!(boot0_vil_max_millivolts(1_620), 407);
        // Past u64, refused, never wrapped (adversarial F3).
        assert!(
            judge_boot0_pull_down(1_229_782_938_247_303_442, VDD.0, VDD.1)
                .unwrap_err()
                .contains("VIL")
        );
        assert!(judge_boot0_pull_down(u64::MAX, VDD.0, VDD.1).is_err());
        assert!(judge_boot0_pull_down(27_000_000, VDD.0, VDD.1).is_ok());
        assert!(
            judge_boot0_pull_down(27_200_000, VDD.0, VDD.1)
                .unwrap_err()
                .contains("VIL")
        );
    }

    #[test]
    fn the_thresholds_are_table_49s() {
        assert_eq!(boot0_vil_max_millivolts(3_300), 727);
        assert_eq!(boot0_vih_min_millivolts(3_300), 1_161);
        assert_eq!(boot0_vih_min_millivolts(1_620), 876);
        // A driver below VIH: a supply too low for the 0.4 V drop.
        assert!(
            judge_boot0_pull_down(10_000, 1_200, 3_600)
                .unwrap_err()
                .contains("VIH")
        );
    }

    #[test]
    fn the_emitters_prefixes_are_held_and_others_are_not() {
        for k in ["U1", "R12", "C3", "SW1"] {
            assert!(emitter_prefix(k), "{k}");
        }
        for k in ["TP1", "SW", "U", "J1", "SWD1", "BOOT1"] {
            assert!(!emitter_prefix(k), "{k}");
        }
    }
}
