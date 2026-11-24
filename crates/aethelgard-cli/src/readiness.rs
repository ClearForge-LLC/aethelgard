//! The fabrication-readiness gate (`AET-WO-4300k` (1), B1, ruled (c) and
//! (d)): a precondition `verify --fab-out` checks before any gate, by name,
//! with no `MANIFEST.json` row. `--against-fab` does not apply it: the
//! package it re-derives binds the specification's sha256, so the
//! specification that passed when the package was written is the only one
//! that can match (ruled (c)).
//!
//! It refuses, in this order:
//! - **a key whose absence or emptiness turns a gate off or relaxes it**
//!   (the key-deletion sweep, `spike/4300k/sweep/`, `sweep-img/`):
//!   `board.stackup.fab_tolerances` and `board.stackup.fabricator` absent;
//!   with `power`, `power.drop_limits`, `power.conditions.dc_bias_derating`,
//!   `power.loads` or `net_classes` empty;
//! - **a hwdef that does not generate** (`aethelgard_hwdef::generate` on the
//!   solved topology);
//! - **a supply-bounded device whose VCC minimum is above the reset's
//!   falling minimum** (`aethelgard_ir::supervisor`, ruled D12);
//! - **a condition no `acknowledge_conditions` entry names**.
//!
//! [`stale`] (any `verify`) refuses an entry no condition produces.

use std::collections::BTreeMap;

use aethelgard_ir::{SolvedTopology, Specification};

/// The refusal's kind for every readiness gap.
pub const KIND: &str = "FabricationNotReady";
/// The refusal's kind for an acknowledgement no condition produces.
pub const STALE: &str = "ConditionAcknowledgementStale";

/// One readiness gap: what is missing, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub id: String,
    pub message: String,
}

/// Every condition a package of `spec` rests on, by id, with its text: the
/// power tree's screened conditions, the reset supervisor's, the stackup's
/// `{condition}` bounds, the crystal's unmeasured steps, and the values the
/// specification declares where no datasheet prints one.
pub fn conditions(
    spec: &Specification,
) -> Result<BTreeMap<String, String>, aethelgard_ir::IrValidationError> {
    let mut out = BTreeMap::new();
    let checked = aethelgard_ir::power::check(spec)?;
    if let Some(c) = &checked {
        out.extend(c.screened.clone());
    }
    out.extend(aethelgard_ir::supervisor::check(spec, checked.as_ref())?);
    out.extend(aethelgard_ir::board::conditions(&spec.board.stackup));
    out.extend(aethelgard_ir::clock::conditions(spec.mcu.hse.as_ref()));
    let devices = spec
        .declared_i2c_devices()
        .iter()
        .map(|d| (&d.reference, &d.pin_capacitance))
        .chain(
            spec.declared_spi_devices()
                .iter()
                .map(|d| (&d.reference, &d.pin_capacitance)),
        );
    for (reference, c) in devices {
        if let Some(c) = c {
            out.insert(
                format!("pin_capacitance_declared:{reference}"),
                format!(
                    "{reference}: each of its pins on a bounded line is counted at {} fF, a value the specification declares, not a maximum the datasheet prints ({})",
                    c.declared.femtofarads, c.declared.source
                ),
            );
        }
    }
    for conn in spec.declared_connectors() {
        for pin in &conn.pins {
            if let Some(o) = &pin.offboard {
                out.insert(
                    format!("offboard_declared:{}.{}", conn.reference, pin.pin),
                    format!(
                        "{}.{}: beyond the board it adds {} fF, declared ({})",
                        conn.reference, pin.pin, o.capacitance_femtofarads, o.source
                    ),
                );
            }
        }
    }
    let reserved = |p: &str| spec.mcu.reserved_pins.iter().any(|r| r.pin == p);
    if spec.diff_pairs.is_empty() && reserved("PA11") && reserved("PA12") {
        out.insert(
            "usb_pair_undeclared:PA11".to_owned(),
            "PA11/PA12: the USB pins are reserved and no differential pair is declared, so the pair gates (drc.pairs, extract.pairs) judge nothing (AET-WO-4300k (d))".to_owned(),
        );
    }
    Ok(out)
}

/// The first `acknowledge_conditions` entry no condition produces.
pub fn stale<'a>(
    spec: &'a Specification,
    conditions: &BTreeMap<String, String>,
) -> Option<&'a str> {
    spec.acknowledge_conditions
        .iter()
        .map(|a| a.id.as_str())
        .find(|id| !conditions.contains_key(*id))
}

/// The first readiness gap of `spec`, in the order the module states.
pub fn check(
    spec: &Specification,
    topology: &SolvedTopology,
    conditions: &BTreeMap<String, String>,
) -> Result<(), Gap> {
    let gap = |id: &str, message: String| {
        Err(Gap {
            id: id.to_owned(),
            message,
        })
    };
    let stack = &spec.board.stackup;
    if stack.fab_tolerances.is_none() {
        return gap(
            "board.stackup.fab_tolerances",
            "no package: `board.stackup.fab_tolerances` is absent, so the via-ring rule and the capacitance ladder are off (AET-WO-4300k (2), A15)".to_owned(),
        );
    }
    if stack.fabricator.is_none() {
        return gap(
            "board.stackup.fabricator",
            "no package: `board.stackup.fabricator` is absent, so no one fabricator stands behind the stackup's bounds (AET-WO-4300k (3))".to_owned(),
        );
    }
    if let Some(p) = &spec.power {
        for (empty, key, what) in [
            (
                p.drop_limits.is_empty(),
                "power.drop_limits",
                "the supply gate is off",
            ),
            (
                p.conditions.dc_bias_derating.is_empty(),
                "power.conditions.dc_bias_derating",
                "a ceramic with no maker's curve is judged at its rated capacitance",
            ),
            (
                p.loads.is_empty(),
                "power.loads",
                "every load-bound check runs without the declared loads",
            ),
            (
                spec.net_classes.is_empty(),
                "net_classes",
                "check drc's power classes are off",
            ),
        ] {
            if empty {
                return gap(
                    key,
                    format!(
                        "no package: `{key}` is empty with `power` declared, so {what} (AET-WO-4300k (d))"
                    ),
                );
            }
        }
    }
    if let Err(e) = aethelgard_hwdef::generate(topology) {
        return gap(
            "hwdef",
            format!(
                "no package: the hwdef does not generate: {} ({e})",
                e.kind()
            ),
        );
    }
    let floors = aethelgard_ir::supervisor::floors_above_reset(spec).map_err(|e| Gap {
        id: e.kind().to_owned(),
        message: e.to_string(),
    })?;
    if let Some(f) = floors.first() {
        return gap(&f.device, format!("no package: {}", f.text()));
    }
    let acknowledged: std::collections::BTreeSet<&str> = spec
        .acknowledge_conditions
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    let missing: Vec<&String> = conditions
        .keys()
        .filter(|id| !acknowledged.contains(id.as_str()))
        .collect();
    if let Some(first) = missing.first() {
        return gap(
            first,
            format!(
                "no package: {} condition(s) no `acknowledge_conditions` entry names: {}",
                missing.len(),
                missing
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    Ok(())
}
