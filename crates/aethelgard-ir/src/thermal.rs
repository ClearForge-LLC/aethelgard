//! The rule a part row may declare for a thermal-via array under an exposed
//! pad (`AET-WO-1309` §1.2, FEEDBACK D1 and F4). Units are micrometres. The
//! rule has no JSON form yet: nothing reads it from a specification, and a
//! later work order's regulators table will hold rows that carry it.
//!
//! **What `ThermalVias::validate` refuses**, in this order, each by name:
//! - a `columns`, `rows`, `pitch_um` or `drill_um` of zero
//!   (`ThermalViasZero`, naming the first);
//! - a source that is empty or only whitespace, in the order count, pitch,
//!   drill, ring, layers (`ThermalViasUnsourced`);
//! - a ring under the board rules' own via ring, `(via_diameter_um -
//!   via_drill_um) / 2` (`ThermalViaRingUnderRules`; the same rule as
//!   `classes`' `ViaRingUnderRules`);
//! - a pitch under `drill_um + HOLE_GAP_UM`, so two holes of the array keep
//!   KiCad's hole-to-hole gap (`ThermalViaPitch`);
//! - a layer list that is empty, repeats a layer, names `F.Cu` (the pad's own
//!   layer), or names a layer the vias cannot join: anything but a declared
//!   plane layer, or `B.Cu` on a two-layer board, where ground is a `B.Cu`
//!   zone (`ThermalViaLayers`).

use crate::IrValidationError;
use crate::board::Board;

/// KiCad's default hole-to-hole minimum, edge to edge.
pub const HOLE_GAP_UM: u32 = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalVias {
    /// Along the pad's x, as the footprint is drawn at rotation 0.
    pub columns: u8,
    /// Along the pad's y.
    pub rows: u8,
    /// Centre to centre, both axes.
    pub pitch_um: u32,
    pub drill_um: u32,
    /// Annular ring; the via pad is `drill + 2 * ring`.
    pub ring_um: u32,
    /// Copper layers each via must join besides the pad's `F.Cu`.
    pub layers: Vec<String>,
    pub sources: ThermalSources,
}

/// The datasheet citation of each field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalSources {
    pub count: String,
    pub pitch: String,
    pub drill: String,
    pub ring: String,
    pub layers: String,
}

impl ThermalVias {
    /// The via pad's diameter, saturating at `u32::MAX`.
    pub fn diameter_um(&self) -> u32 {
        self.ring_um.saturating_mul(2).saturating_add(self.drill_um)
    }

    /// Refuses, by name, in the order the module documents. `part` is the
    /// row that carries the rule.
    pub fn validate(&self, part: &str, board: &Board) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        let zeros = [
            ("columns", u32::from(self.columns)),
            ("rows", u32::from(self.rows)),
            ("pitch_um", self.pitch_um),
            ("drill_um", self.drill_um),
        ];
        if let Some((field, _)) = zeros.iter().find(|(_, v)| *v == 0) {
            return Err(E::ThermalViasZero {
                part: part.to_owned(),
                field,
            });
        }

        let s = &self.sources;
        let sources = [
            ("count", &s.count),
            ("pitch", &s.pitch),
            ("drill", &s.drill),
            ("ring", &s.ring),
            ("layers", &s.layers),
        ];
        if let Some((field, _)) = sources.iter().find(|(_, v)| v.trim().is_empty()) {
            return Err(E::ThermalViasUnsourced {
                part: part.to_owned(),
                field,
            });
        }

        // The board's own ring is (diameter - drill) / 2. Rounded up, so an
        // odd difference does not lose its half micrometre.
        let min_ring = board
            .rules
            .via_diameter_um
            .saturating_sub(board.rules.via_drill_um)
            .div_ceil(2);
        if self.ring_um < min_ring {
            return Err(E::ThermalViaRingUnderRules {
                part: part.to_owned(),
                ring_um: self.ring_um,
                min_um: min_ring,
            });
        }

        let min_pitch = u64::from(self.drill_um) + u64::from(HOLE_GAP_UM);
        if u64::from(self.pitch_um) < min_pitch {
            return Err(E::ThermalViaPitch {
                part: part.to_owned(),
                pitch_um: self.pitch_um,
                min_um: min_pitch,
            });
        }

        self.check_layers(part, board)
    }

    fn check_layers(&self, part: &str, board: &Board) -> Result<(), IrValidationError> {
        let refuse = |layer: &str, rule: &'static str| IrValidationError::ThermalViaLayers {
            part: part.to_owned(),
            layer: layer.to_owned(),
            rule,
        };
        if self.layers.is_empty() {
            return Err(refuse(
                "",
                "no layer is named; the vias must join at least one",
            ));
        }
        for (i, layer) in self.layers.iter().enumerate() {
            if self.layers[..i].contains(layer) {
                return Err(refuse(layer, "the layer is named twice"));
            }
            if layer == "F.Cu" {
                return Err(refuse(layer, "F.Cu is the pad's own layer, joined already"));
            }
            let is_plane = board.planes().any(|(name, _)| name == layer);
            let is_ground_zone = layer == "B.Cu" && board.copper_layers == 2;
            if !is_plane && !is_ground_zone {
                return Err(refuse(
                    layer,
                    "the layer is not one the vias can join: a declared plane layer, or B.Cu on a 2-layer board",
                ));
            }
        }
        Ok(())
    }
}
