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

/// KiCad 10.0.7's layer id for each copper layer this engine writes
/// (measured, `AET-WO-4100` S1): `F.Cu` 0, `B.Cu` 2, `In1.Cu` 4, `In2.Cu` 6.
pub fn kicad_id(name: &str) -> Option<&'static str> {
    Some(match name {
        "F.Cu" => "0",
        "B.Cu" => "2",
        "In1.Cu" => "4",
        "In2.Cu" => "6",
        _ => return None,
    })
}

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
    /// The declared net classes and coupled pairs (`AET-WO-1308`), each net
    /// by the board's name for it. Empty for a specification that declares
    /// none, whose board is then today's.
    pub classes: Vec<Class>,
    pub pairs: Vec<Pair>,
    /// How each declared through-hole part's pads join a plane or zone of
    /// their own net, by reference (`AET-WO-1309` F5). A through-hole pad of
    /// a filled net with no entry here is refused by name (`INV-06`).
    pub joins: Vec<(String, Join)>,
    /// Each declared thermal-via array: the pad it is under and its rule
    /// (`AET-WO-1309` §1.2). The engine emits it; the gate judges it from
    /// this, never from the board.
    pub thermal: Vec<ThermalPad>,
}

/// How a through-hole pad meets a fill of its own net.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    Solid,
    Relief,
}

impl Join {
    /// The join a part row declares (`aethelgard-ir`'s connectors table).
    pub fn of(join: aethelgard_ir::connectors::PadJoin) -> Join {
        match join {
            aethelgard_ir::connectors::PadJoin::Solid => Join::Solid,
            aethelgard_ir::connectors::PadJoin::Relief => Join::Relief,
        }
    }

    /// The pad's own `zone_connect` that makes KiCad 10.0.7's refill join
    /// it so (measured, `AET-WO-1309` FEEDBACK B2): 2 solid, 1 relief.
    pub fn token(self) -> &'static str {
        match self {
            Join::Solid => "2",
            Join::Relief => "1",
        }
    }

    /// The join a pad's `zone_connect` token gives, if it is one of these.
    pub fn of_token(token: &str) -> Option<Join> {
        [Join::Solid, Join::Relief]
            .into_iter()
            .find(|j| j.token() == token)
    }
}

/// A declared thermal-via array, in nanometres: `columns` along the
/// footprint's x as drawn at rotation 0, `rows` along its y, centred on the
/// pad, `pitch_nm` apart both ways.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalPad {
    pub reference: String,
    pub pad: String,
    pub columns: usize,
    pub rows: usize,
    pub pitch_nm: i64,
    pub drill_nm: i64,
    pub via_nm: i64,
    /// The layers each via must join besides the pad's F.Cu.
    pub layers: Vec<String>,
}

impl ThermalPad {
    /// The array `rule` (an IR part row's, validated) declares under
    /// `reference`'s pad `pad`.
    pub fn of(
        reference: &str,
        pad: &str,
        rule: &aethelgard_ir::thermal::ThermalVias,
    ) -> ThermalPad {
        ThermalPad {
            reference: reference.to_owned(),
            pad: pad.to_owned(),
            columns: usize::from(rule.columns),
            rows: usize::from(rule.rows),
            pitch_nm: nm(rule.pitch_um),
            drill_nm: nm(rule.drill_um),
            via_nm: nm(rule.diameter_um()),
            layers: rule.layers.clone(),
        }
    }
}

/// The copper one net is laid with: its track width and its via.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rule {
    pub width_nm: i64,
    pub via_nm: i64,
    pub drill_nm: i64,
}

/// A declared net class: its nets' rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub rule: Rule,
    /// Whether the class declares its via; otherwise it is the board's.
    pub via_declared: bool,
    pub nets: Vec<String>,
}

/// A declared coupled pair: its positive and negative nets, the gap between
/// their copper and their width. Its nets' via is the board's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub name: String,
    pub nets: [String; 2],
    pub gap_nm: i64,
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
            classes: spec
                .net_classes
                .iter()
                .map(|c| Class {
                    name: c.name.clone(),
                    rule: Rule {
                        width_nm: nm(c.track_width_um),
                        via_nm: c
                            .via
                            .as_ref()
                            .map_or(crate::route::VIA_NM, |v| nm(v.diameter_um)),
                        drill_nm: c
                            .via
                            .as_ref()
                            .map_or(crate::route::DRILL_NM, |v| nm(v.drill_um)),
                    },
                    via_declared: c.via.is_some(),
                    nets: c.nets.iter().map(|n| board_net(spec, n)).collect(),
                })
                .collect(),
            pairs: spec
                .diff_pairs
                .iter()
                .map(|p| Pair {
                    name: p.name.clone(),
                    nets: [board_net(spec, &p.nets[0]), board_net(spec, &p.nets[1])],
                    gap_nm: nm(p.gap_um),
                    width_nm: nm(p.width_um),
                })
                .collect(),
            // The power tree's (`AET-WO-4300d2` §1.5, `1309` D1): the
            // power-in connector's declared join, and each buck's array from
            // its row's rule.
            joins: spec
                .power
                .iter()
                .filter_map(|p| {
                    let row = aethelgard_ir::connectors::power_in(&p.input.part)?;
                    Some((p.input.reference.clone(), Join::of(row.join)))
                })
                .collect(),
            thermal: aethelgard_ir::power::thermal_arrays(spec)
                .iter()
                .map(|(reference, pad, rule)| ThermalPad::of(reference, pad, rule))
                .collect(),
        }
    }

    /// The class or pair a net (by the board's name) is in: `class` or
    /// `pair`, its name, and the rule its nets are laid with (a pair's width
    /// with the board's via).
    pub fn owner(&self, net: &str) -> Option<(&'static str, &str, Rule)> {
        let has = |nets: &[String]| nets.iter().any(|n| n == net);
        if let Some(c) = self.classes.iter().find(|c| has(&c.nets)) {
            return Some(("class", &c.name, c.rule));
        }
        let p = self.pairs.iter().find(|p| has(&p.nets))?;
        let rule = Rule {
            width_nm: p.width_nm,
            ..self.default_rule()
        };
        Some(("pair", &p.name, rule))
    }

    /// The rule a net is laid with: its class's or pair's, or the board's.
    pub fn rule(&self, net: &str) -> Rule {
        self.owner(net).map_or(self.default_rule(), |(_, _, r)| r)
    }

    /// The board's own rule: the declared track width and the engine's via.
    pub fn default_rule(&self) -> Rule {
        Rule {
            width_nm: self.width_nm,
            via_nm: crate::route::VIA_NM,
            drill_nm: crate::route::DRILL_NM,
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

/// The board's name for a specification net: a rail (a voltage domain or
/// `GND`) by its own name, a signal by its root-sheet label (`pcb`).
fn board_net(spec: &Specification, net: &str) -> String {
    let rail = net == crate::schematic::GND || spec.voltage_domains.iter().any(|d| d.id == net);
    if rail {
        crate::pcb::escape(net)
    } else {
        crate::extract::board_net(net)
    }
}

/// The reference's declared board (`examples/sentinel-01-minimal.json`), for
/// tests. The CLI's `stackups_tier1` asserts it equals `Stack::of` on that
/// file, so the two cannot drift.
#[cfg(test)]
pub(crate) fn reference() -> Stack {
    let thickness_source = "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %";
    Stack {
        copper: vec!["F.Cu".into(), "B.Cu".into()],
        planes: vec![],
        thickness: Thickness {
            nominal_nm: 1_600_000,
            min_nm: 340_000,
            max_nm: 2_000_000,
            source: thickness_source.into(),
        },
        dielectrics: vec![(
            "F.Cu".into(),
            "B.Cu".into(),
            200_000,
            "implied: the finished minimum 340 um less twice the copper maximum 70 um".into(),
        )],
        copper_max_nm: 70_000,
        copper_source: "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)".into(),
        permittivity_permille: 5400,
        permittivity_source: "the IPC-4101 FR-4 maximum, specified at 1 MHz".into(),
        clearance_nm: 200_000,
        width_nm: 200_000,
        classes: vec![],
        pairs: vec![],
        joins: vec![],
        thermal: vec![],
    }
}
