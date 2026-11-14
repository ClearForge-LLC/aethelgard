//! The board a specification declares (`AET-WO-4100`): its copper layers and
//! planes, its stackup bounds and its design rules, in nanometres, as every
//! writer and reader of the board takes them. Built only from a validated
//! [`Specification`]; nothing here is read from a board.
//!
//! At 2 copper layers the board is today's: both layers signal, ground on an
//! unfilled `B.Cu` zone. At 4 it is S-P-P-S: `F.Cu` and `B.Cu` route, and
//! each inner layer is an unfilled plane of one net, exactly one of them
//! `GND` (ruled D1, D7). Vias are through vias at both counts.

use aethelgard_ir::Specification;
use aethelgard_ir::board::LayerRole;

/// The declared board, in nanometres.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    /// KiCad's copper layer names, top to bottom.
    pub copper: Vec<String>,
    /// Each plane as (layer, net), top to bottom. Empty at 2 layers.
    pub planes: Vec<(String, String)>,
    pub thickness: Thickness,
    /// Each dielectric's (upper layer, lower layer, minimum, source).
    pub dielectrics: Vec<(String, String, i64, String)>,
    pub copper_max_nm: i64,
    pub copper_source: String,
    /// Relative permittivity, at most, in thousandths.
    pub permittivity_permille: u32,
    pub permittivity_source: String,
    pub clearance_nm: i64,
    pub width_nm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thickness {
    pub nominal_nm: i64,
    pub min_nm: i64,
    pub max_nm: i64,
    pub source: String,
}

fn nm(um: u32) -> i64 {
    i64::from(um) * 1000
}

impl Stack {
    /// The declared board of a specification that `validate` accepted.
    pub fn of(spec: &Specification) -> Stack {
        let b = &spec.board;
        let s = &b.stackup;
        let f = &s.finished_thickness_um;
        Stack {
            copper: b.layers.iter().map(|l| l.name.clone()).collect(),
            planes: b
                .layers
                .iter()
                .filter(|l| l.role == LayerRole::Plane)
                .filter_map(|l| Some((l.name.clone(), l.net.clone()?)))
                .collect(),
            thickness: Thickness {
                nominal_nm: nm(f.nominal),
                min_nm: nm(f.min),
                max_nm: nm(f.max),
                source: f.source.clone(),
            },
            dielectrics: s
                .dielectrics
                .iter()
                .map(|d| {
                    (
                        d.between[0].clone(),
                        d.between[1].clone(),
                        nm(d.min_thickness_um),
                        d.source.clone(),
                    )
                })
                .collect(),
            copper_max_nm: nm(s.copper_max_um.value),
            copper_source: s.copper_max_um.source.clone(),
            permittivity_permille: s.relative_permittivity_max_permille.value,
            permittivity_source: s.relative_permittivity_max_permille.source.clone(),
            clearance_nm: nm(b.rules.clearance_um),
            width_nm: nm(b.rules.track_width_um),
        }
    }

    /// The number of copper layers.
    pub fn layers(&self) -> usize {
        self.copper.len()
    }

    /// Whether ground is a plane (4 layers) rather than the `B.Cu` zone.
    pub fn has_planes(&self) -> bool {
        !self.planes.is_empty()
    }

    /// The plane a net is, if any: its layer name.
    pub fn plane_of(&self, net: &str) -> Option<&str> {
        self.planes
            .iter()
            .find(|(_, n)| n == net)
            .map(|(l, _)| l.as_str())
    }

    /// The net a layer is a plane of, if it is one.
    pub fn plane_of_layer(&self, layer: &str) -> Option<&str> {
        self.planes
            .iter()
            .find(|(l, _)| l == layer)
            .map(|(_, n)| n.as_str())
    }

    /// The minimum dielectric under `F.Cu` and over `B.Cu`: how near the next
    /// copper layer can be to an outer piece.
    pub fn outer_dielectrics_nm(&self) -> (i64, i64) {
        let first = self.dielectrics.first().map_or(0, |d| d.2);
        let last = self.dielectrics.last().map_or(0, |d| d.2);
        (first, last)
    }
}

/// The reference's declared board, for tests.
#[cfg(test)]
pub(crate) fn reference() -> Stack {
    let spec: Specification =
        serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json")).unwrap();
    Stack::of(&spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_declares_todays_two_layer_board() {
        let spec: Specification =
            serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json"))
                .unwrap();
        spec.validate().unwrap();
        let s = Stack::of(&spec);
        assert_eq!(s.copper, ["F.Cu", "B.Cu"]);
        assert!(s.planes.is_empty());
        assert_eq!(
            (
                s.thickness.nominal_nm,
                s.thickness.min_nm,
                s.thickness.max_nm
            ),
            (1_600_000, 340_000, 2_000_000)
        );
        assert_eq!(s.outer_dielectrics_nm(), (200_000, 200_000));
        assert_eq!((s.copper_max_nm, s.permittivity_permille), (70_000, 5400));
        assert_eq!((s.clearance_nm, s.width_nm), (200_000, 200_000));
    }
}
