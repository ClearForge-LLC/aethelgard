//! The specification's `board` section (`AET-WO-4100`, ruled on PR #120):
//! the copper layers and what each carries, the stackup as bounds with their
//! sources, and the design rules. Every key is required (`INV-06`).
//!
//! **Units** are micrometres and thousandths (ruled D8). **A stackup value is
//! a bound, never a nominal:** a declared minimum dielectric is a lower bound
//! its `source` supports, which needs a published tolerance, and minima round
//! down and maxima round up (ruled D2). The validator cannot check a source;
//! `FAB-NOTES.md` states the rule.
//!
//! **What this engine builds** (ruled D1, D7 and the rules field):
//! - 2 copper layers, both signal; or 4, `F.Cu` and `B.Cu` signal and both
//!   inner layers planes, exactly one of them `GND` and the other a declared
//!   domain. Six layers are a later work order and are refused by name.
//! - clearance and track width at or above the engine floor;
//! - through vias of exactly the engine's 0.3 mm drill and 0.6 mm diameter.
//!   Any other via is refused as unsupported, never built differently.

use std::collections::BTreeSet;

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::IrValidationError;

/// The engine floor for clearance and track width: KiCad's default net-class
/// values, which the router, the gate and extraction were built and measured
/// at (`AET-WO-1300`, `AET-WO-1302`).
pub const FLOOR_UM: u32 = 200;
/// The one via this engine builds: drill and diameter, in micrometres.
pub const VIA_DRILL_UM: u32 = 300;
pub const VIA_DIAMETER_UM: u32 = 600;
/// The largest finished thickness, dielectric or copper weight accepted, so
/// that every derived length fits the engine's integer arithmetic.
pub const MAX_THICKNESS_UM: u32 = 10_000;
/// The largest relative permittivity accepted, in thousandths.
pub const MAX_PERMITTIVITY_PERMILLE: u32 = 100_000;
/// A source is cited in `FAB-NOTES.md`: 1 to 200 bytes of printable ASCII,
/// space included, with no space at either end.
pub const MAX_SOURCE: usize = 200;

/// The largest outline bound accepted, in micrometres: the placer searches
/// at most a metre around a part (`pcb.rs`'s `RINGS`).
pub const MAX_OUTLINE_UM: u32 = 1_000_000;

/// The copper layer counts this engine builds.
pub const LAYER_COUNTS: [u8; 2] = [2, 4];

/// KiCad 10.0.7's names for `n` copper layers, top to bottom (measured,
/// `AET-WO-4100` S1): `F.Cu`, `In1.Cu` … `In<n-2>.Cu`, `B.Cu`.
pub fn copper_names(n: u8) -> Vec<String> {
    let mut out = vec!["F.Cu".to_owned()];
    out.extend((1..n.saturating_sub(1)).map(|i| format!("In{i}.Cu")));
    out.push("B.Cu".to_owned());
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Board {
    pub copper_layers: u8,
    /// Top to bottom, one per copper layer.
    pub layers: Vec<CopperLayer>,
    pub stackup: Stackup,
    pub rules: DesignRules,
    /// The largest outline the placer may reach (`AET-WO-1313`), in
    /// micrometres (ruled F2: the board section's unit, `4100` D8). Optional
    /// under the precedent ruled on PR #191 (Q1): its absence is today's
    /// placer, judged as such, and is not written back. As for `test_points`
    /// and `mcu.reset_switch`, an explicit `null` is refused, not read as
    /// absent (ruled F4).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_outline_um: Option<OutlineBound>,
    /// The edge the vehicle's nose points to (`AET-WO-4400d`,
    /// [`crate::orientation`]). Optional, as `max_outline_um` is; required
    /// when a device declares an orientation (`ForwardUndeclared`).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub forward: Option<crate::orientation::Edge>,
}

/// A declared outline bound: the board's width and height at most, in
/// micrometres, each from 1 to [`MAX_OUTLINE_UM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct OutlineBound {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct CopperLayer {
    pub name: String,
    pub role: LayerRole,
    /// A plane's net; `null` on a signal layer. Required as a key.
    #[serde(deserialize_with = "Option::deserialize")]
    pub net: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerRole {
    Signal,
    Plane,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Stackup {
    pub finished_thickness_um: FinishedThickness,
    /// One per adjacent pair of copper layers, top to bottom.
    pub dielectrics: Vec<Dielectric>,
    /// Finished copper, at most, on every layer.
    pub copper_max_um: Sourced,
    /// The relative permittivity of laminate and solder mask, at most, in
    /// thousandths.
    pub relative_permittivity_max_permille: Sourced,
    /// The supply-path bound's inputs (`AET-WO-1303b`, D3/D6): finished
    /// copper at least this on F.Cu and B.Cu, and on an inner plane; hole
    /// plating at least this; copper no hotter than this, °C. Each optional:
    /// the supply gate refuses by name a specification that declares a drop
    /// limit and lacks one, and never defaults it (`RUL-14`).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub outer_copper_min_um: Option<Sourced>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub plane_copper_min_um: Option<Sourced>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub plating_min_um: Option<Sourced>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub copper_temperature_max_c: Option<Sourced>,
    /// The fabricator's tolerances the supply gate prices
    /// (`AET-WO-1303b`, ruling 6974760231): a fabricator's numbers, not the
    /// engine's. Optional, and each entry optional: refused by name when
    /// the gate reads one that is absent.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub fab_tolerances: Option<FabTolerances>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct FabTolerances {
    /// A hole's centre anywhere within this of its pad's, per axis, µm.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hole_position_um: Option<Sourced>,
    /// A finished hole up to this larger than drawn, µm.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hole_size_plus_um: Option<Sourced>,
    /// A finished hole up to this smaller than drawn, µm.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hole_size_minus_um: Option<Sourced>,
    /// A finished track up to this much narrower than drawn, in
    /// thousandths of its width (the IR's integers, as
    /// `relative_permittivity_max_permille`).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub track_width_permille: Option<Sourced>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct FinishedThickness {
    /// What the board file declares as its thickness.
    pub nominal: u32,
    pub min: u32,
    pub max: u32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Dielectric {
    /// The two copper layers it separates, upper first.
    pub between: Vec<String>,
    pub min_thickness_um: u32,
    pub source: String,
    /// Its thickness at most, under the same `source` (`AET-WO-1303b`):
    /// optional; without it a via barrel prices this dielectric at the
    /// finished maximum less every other layer's minimum.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_thickness_um: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Sourced {
    pub value: u32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct DesignRules {
    pub clearance_um: u32,
    pub track_width_um: u32,
    pub via_drill_um: u32,
    pub via_diameter_um: u32,
}

impl Board {
    /// Each plane, top to bottom, as (layer name, net).
    pub fn planes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.layers.iter().filter_map(|l| match (l.role, &l.net) {
            (LayerRole::Plane, Some(net)) => Some((l.name.as_str(), net.as_str())),
            _ => None,
        })
    }
}

impl Board {
    /// Step 13 of `Specification::validate`, in this order: the layer count,
    /// the layer table, each layer's role, the plane nets and their layout,
    /// the dielectrics, the finished thickness, the copper and permittivity
    /// bounds, every source, the rules, then the outline bound. `domains` are
    /// the declared voltage domain ids. Whether the bound can hold the parts
    /// is the emitter's to judge (`aethelgard-eda`'s `pcb::bound_holds`).
    pub fn validate(&self, domains: &BTreeSet<&str>) -> Result<(), IrValidationError> {
        use IrValidationError as E;
        let n = self.copper_layers;
        if !LAYER_COUNTS.contains(&n) {
            return Err(E::UnsupportedLayerCount { value: n });
        }
        let names = copper_names(n);
        if self.layers.len() != names.len() {
            return Err(E::LayerTable {
                path: "board.layers".into(),
                rule: "one entry per copper layer, top to bottom",
            });
        }
        for (i, (layer, name)) in self.layers.iter().zip(&names).enumerate() {
            if &layer.name != name {
                return Err(E::LayerTable {
                    path: format!("board.layers[{i}].name"),
                    rule: "KiCad's copper names in order: F.Cu, In1.Cu, In2.Cu, B.Cu",
                });
            }
        }
        for (i, layer) in self.layers.iter().enumerate() {
            let path = || format!("board.layers[{i}]");
            let outer = i == 0 || i + 1 == self.layers.len();
            match (layer.role, &layer.net) {
                (LayerRole::Signal, Some(_)) => {
                    return Err(E::LayerRole {
                        path: path(),
                        rule: "a signal layer carries no plane net (`net` is null)",
                    });
                }
                (LayerRole::Plane, None) => {
                    return Err(E::LayerRole {
                        path: path(),
                        rule: "a plane names its net",
                    });
                }
                (LayerRole::Plane, Some(_)) if outer => {
                    return Err(E::LayerRole {
                        path: path(),
                        rule: "F.Cu and B.Cu are signal layers",
                    });
                }
                (LayerRole::Signal, None) if !outer => {
                    return Err(E::LayerRole {
                        path: path(),
                        rule: "at 4 copper layers both inner layers are planes",
                    });
                }
                _ => {}
            }
        }
        let mut nets = BTreeSet::new();
        for (_, net) in self.planes() {
            if net != "GND" && !domains.contains(net) {
                return Err(E::PlaneNet { id: net.to_owned() });
            }
            if !nets.insert(net) {
                return Err(E::PlaneLayout {
                    rule: "one plane per net",
                });
            }
        }
        if n == 4 && !nets.contains("GND") {
            return Err(E::PlaneLayout {
                rule: "at 4 copper layers exactly one plane is `GND`",
            });
        }
        let s = &self.stackup;
        if s.dielectrics.len() != names.len() - 1 {
            return Err(E::DielectricTable {
                path: "board.stackup.dielectrics".into(),
                rule: "one dielectric per adjacent pair of copper layers, top to bottom",
            });
        }
        let mut sum: u64 = 0;
        for (i, d) in s.dielectrics.iter().enumerate() {
            if d.between != [names[i].clone(), names[i + 1].clone()] {
                return Err(E::DielectricTable {
                    path: format!("board.stackup.dielectrics[{i}].between"),
                    rule: "the two copper layers it separates, upper first, in order",
                });
            }
            bounded(
                d.min_thickness_um,
                MAX_THICKNESS_UM,
                format!("board.stackup.dielectrics[{i}].min_thickness_um"),
            )?;
            sum += u64::from(d.min_thickness_um);
        }
        let f = &s.finished_thickness_um;
        if f.min == 0 || f.max > MAX_THICKNESS_UM {
            return Err(E::FinishedThickness {
                rule: "min above 0 and max at most 10000 um",
            });
        }
        if !(f.min <= f.nominal && f.nominal <= f.max) {
            return Err(E::FinishedThickness {
                rule: "min <= nominal <= max",
            });
        }
        if u64::from(f.min) < sum {
            return Err(E::FinishedThickness {
                rule: "min is below the sum of the dielectric minima",
            });
        }
        bounded(
            s.copper_max_um.value,
            MAX_THICKNESS_UM,
            "board.stackup.copper_max_um.value".into(),
        )?;
        let er = s.relative_permittivity_max_permille.value;
        if !(1000..=MAX_PERMITTIVITY_PERMILLE).contains(&er) {
            return Err(E::StackupValue {
                path: "board.stackup.relative_permittivity_max_permille.value".into(),
                rule: "a relative permittivity from 1000 (vacuum) to 100000 thousandths",
            });
        }
        // At 2 layers `FAB-NOTES.md` and extraction's conditions state only the
        // finished thickness and the copper, so the declared dielectric must
        // follow from them: thickness at least min, copper at most max.
        if n == 2
            && u64::from(s.dielectrics[0].min_thickness_um) + 2 * u64::from(s.copper_max_um.value)
                > u64::from(f.min)
        {
            return Err(E::FinishedThickness {
                rule: "at 2 layers the dielectric minimum may not exceed min less twice copper_max_um",
            });
        }
        source(&f.source, "board.stackup.finished_thickness_um.source")?;
        for (i, d) in s.dielectrics.iter().enumerate() {
            source(&d.source, &format!("board.stackup.dielectrics[{i}].source"))?;
        }
        source(
            &s.copper_max_um.source,
            "board.stackup.copper_max_um.source",
        )?;
        source(
            &s.relative_permittivity_max_permille.source,
            "board.stackup.relative_permittivity_max_permille.source",
        )?;
        // the supply gate's inputs, each where declared (`AET-WO-1303b`): a
        // copper minimum above 0 and at most `copper_max_um` (the §7.1 row:
        // `copper_max_um` cannot undercut a declared minimum), the plating
        // above 0, the temperature from 1 to 300 °C, a dielectric's maximum
        // at least its minimum
        for (v, path) in [
            (&s.outer_copper_min_um, "board.stackup.outer_copper_min_um"),
            (&s.plane_copper_min_um, "board.stackup.plane_copper_min_um"),
        ] {
            if let Some(v) = v {
                bounded(v.value, MAX_THICKNESS_UM, format!("{path}.value"))?;
                if v.value > s.copper_max_um.value {
                    return Err(E::StackupValue {
                        path: format!("{path}.value"),
                        rule: "a copper minimum at most copper_max_um",
                    });
                }
                source(&v.source, &format!("{path}.source"))?;
            }
        }
        if let Some(v) = &s.plating_min_um {
            bounded(
                v.value,
                MAX_THICKNESS_UM,
                "board.stackup.plating_min_um.value".into(),
            )?;
            source(&v.source, "board.stackup.plating_min_um.source")?;
        }
        if let Some(v) = &s.copper_temperature_max_c {
            if !(1..=300).contains(&v.value) {
                return Err(E::StackupValue {
                    path: "board.stackup.copper_temperature_max_c.value".into(),
                    rule: "a temperature from 1 to 300 degrees C",
                });
            }
            source(&v.source, "board.stackup.copper_temperature_max_c.source")?;
        }
        if let Some(f) = &s.fab_tolerances {
            for (v, path, max, rule) in [
                (
                    &f.hole_position_um,
                    "hole_position_um",
                    1000,
                    "a tolerance from 0 to 1000 um",
                ),
                (
                    &f.hole_size_plus_um,
                    "hole_size_plus_um",
                    1000,
                    "a tolerance from 0 to 1000 um",
                ),
                (
                    &f.hole_size_minus_um,
                    "hole_size_minus_um",
                    1000,
                    "a tolerance from 0 to 1000 um",
                ),
                (
                    &f.track_width_permille,
                    "track_width_permille",
                    999,
                    "a fraction from 0 to 999 per mille",
                ),
            ] {
                if let Some(v) = v {
                    let path = format!("board.stackup.fab_tolerances.{path}");
                    if v.value > max {
                        return Err(E::StackupValue {
                            path: format!("{path}.value"),
                            rule,
                        });
                    }
                    source(&v.source, &format!("{path}.source"))?;
                }
            }
        }
        for (i, d) in s.dielectrics.iter().enumerate() {
            if let Some(m) = d.max_thickness_um {
                let path = format!("board.stackup.dielectrics[{i}].max_thickness_um");
                bounded(m, MAX_THICKNESS_UM, path.clone())?;
                if m < d.min_thickness_um {
                    return Err(E::StackupValue {
                        path,
                        rule: "a dielectric's maximum at least its minimum",
                    });
                }
            }
        }
        let r = &self.rules;
        for (value, path) in [
            (r.clearance_um, "board.rules.clearance_um"),
            (r.track_width_um, "board.rules.track_width_um"),
        ] {
            if value < FLOOR_UM {
                return Err(E::RuleBelowFloor {
                    path: path.into(),
                    value,
                    floor: FLOOR_UM,
                });
            }
            bounded(value, MAX_THICKNESS_UM, path.into())?;
        }
        for (value, required, path) in [
            (r.via_drill_um, VIA_DRILL_UM, "board.rules.via_drill_um"),
            (
                r.via_diameter_um,
                VIA_DIAMETER_UM,
                "board.rules.via_diameter_um",
            ),
        ] {
            if value != required {
                return Err(E::UnsupportedViaRule {
                    path: path.into(),
                    value,
                    required,
                });
            }
        }
        if let Some(b) = &self.max_outline_um {
            for (value, path) in [
                (b.width, "board.max_outline_um.width"),
                (b.height, "board.max_outline_um.height"),
            ] {
                if value == 0 || value > MAX_OUTLINE_UM {
                    return Err(E::OutlineBound {
                        path: path.into(),
                        value,
                        max: MAX_OUTLINE_UM,
                    });
                }
            }
        }
        Ok(())
    }
}

/// An optional key that is present must hold a value: `null` is refused as
/// the wrong type, as `test_points` and `mcu.reset_switch` refuse it.
pub(crate) fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn bounded(value: u32, max: u32, path: String) -> Result<(), IrValidationError> {
    if value == 0 || value > max {
        return Err(IrValidationError::StackupValue {
            path,
            rule: "above 0 and at most 10000 um",
        });
    }
    Ok(())
}

pub(crate) fn source(value: &str, path: &str) -> Result<(), IrValidationError> {
    let b = value.as_bytes();
    let ok = (1..=MAX_SOURCE).contains(&b.len())
        && b.iter().all(|c| (0x20..=0x7E).contains(c))
        && b[0] != b' '
        && b[b.len() - 1] != b' ';
    if ok {
        Ok(())
    } else {
        Err(IrValidationError::InvalidSource { path: path.into() })
    }
}

crate::object_only!(
    Board,
    CopperLayer,
    Stackup,
    FinishedThickness,
    Dielectric,
    Sourced,
    FabTolerances,
    DesignRules,
    OutlineBound
);
