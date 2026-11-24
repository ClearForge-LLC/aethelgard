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

/// `⌈e·√2⌉`, nm: the farthest a per-axis tolerance `e` moves a centre,
/// exact in integers (the float's ceiling raised until `2e² ≤ d²`). One
/// source for the via ring rule and the supply gate's holes.
pub fn diagonal_nm(e_nm: i64) -> i64 {
    let mut d = (e_nm as f64 * std::f64::consts::SQRT_2).ceil() as i64;
    // the float's ceiling can fall one short: first at e = 225 058 681 nm,
    // where e·√2 lies just above an integer (d² − 2e² = −1); within the
    // IR's 1 mm cap on a tolerance it never does (measured, every nm to
    // 3 mm), so this exactness does not rest on that cap
    while 2 * i128::from(e_nm).pow(2) > i128::from(d).pow(2) {
        d += 1;
    }
    d
}

/// The ring rule (`AET-WO-1314`, ruling 6982875594 D1): a via drilled `d`
/// finishes anywhere up to `d + plus`, its centre anywhere within
/// `position` per axis of its pad's, so up to `⌈position·√2⌉` off centre. A
/// pad of diameter
///
/// `D ≥ (d + plus) + 2⌈position·√2⌉ + 2·ring`, rounded up to 10 µm,
///
/// keeps at least `ring` of copper round the hole at every declared size
/// and position: the worst case is the largest hole at a corner of the
/// position box. The smaller tolerance (`hole_size_minus_um`) does not
/// enter: a smaller hole only widens the ring. All three inputs are the
/// specification's (`board.stackup.fab_tolerances`), never the engine's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingRule {
    pub plus_nm: i64,
    pub position_nm: i64,
    pub ring_nm: i64,
}

impl RingRule {
    /// The rule a specification declares: `None` where it declares no
    /// `fab_tolerances`; refused with the first field the rule needs that
    /// the block lacks (`RUL-14`: never defaulted).
    pub fn of(spec: &Specification) -> Result<Option<RingRule>, &'static str> {
        let Some(t) = &spec.board.stackup.fab_tolerances else {
            return Ok(None);
        };
        let need = |v: &Option<aethelgard_ir::board::Sourced>, field: &'static str| {
            v.as_ref().map(|s| nm(s.value)).ok_or(field)
        };
        Ok(Some(RingRule {
            plus_nm: need(
                &t.hole_size_plus_um,
                "board.stackup.fab_tolerances.hole_size_plus_um",
            )?,
            position_nm: need(
                &t.hole_position_um,
                "board.stackup.fab_tolerances.hole_position_um",
            )?,
            ring_nm: need(&t.via_ring_um, "board.stackup.fab_tolerances.via_ring_um")?,
        }))
    }

    /// The smallest pad, nm, that keeps the ring round a hole drilled
    /// `drill_nm`.
    pub fn pad_nm(&self, drill_nm: i64) -> i64 {
        let reach = diagonal_nm(self.position_nm);
        let d = drill_nm + self.plus_nm + 2 * reach + 2 * self.ring_nm;
        (d + 9_999) / 10_000 * 10_000
    }

    /// Each declared class whose via is smaller than the rule's pad for its
    /// drill: (class, its pad, the pad that holds the ring), nm.
    pub fn unheld(&self, stack: &Stack) -> Vec<(String, i64, i64)> {
        stack
            .classes
            .iter()
            .filter(|c| c.via_declared)
            .filter_map(|c| {
                let pad = self.pad_nm(c.rule.drill_nm);
                (c.rule.via_nm < pad).then(|| (c.name.clone(), c.rule.via_nm, pad))
            })
            .collect()
    }
}

/// The supply gate's junction regions, as the router keeps them apart
/// (`AET-WO-1324`, ruled 2026-11-24). `extract` prices each via of a net it
/// judges with a junction region of cells ([`crate::supply`]), and refuses
/// two that meet (`ViaRegionsOverlap`): its certified bound carries one
/// flow per piece of copper, and a region's field is written for one hole.
/// KiCad's DRC passes two such vias, so meeting regions are a limit of the
/// supply model, not a fabrication fault; the router keeps the model
/// unchanged by routing around that limit. Built only where the gate runs:
/// the specification declares `power.drop_limits` and each input the
/// region's reach reads. Every field is the specification's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunctionRule {
    /// The nets the gate judges, by the board's name: each limited domain
    /// and `GND`, its return.
    pub nets: Vec<String>,
    /// `fab_tolerances.hole_size_plus_um`, `fab_tolerances.hole_position_um`
    /// and `plating_min_um`, nm.
    pub hole_plus_nm: i64,
    pub hole_offset_nm: i64,
    pub plating_nm: i64,
}

impl JunctionRule {
    /// The rule a specification declares, or `None` where the gate does not
    /// run: no drop limit, or an input its region's reach reads undeclared
    /// (the gate refuses that by name, `SupplyInputUndeclared`).
    pub fn of(spec: &Specification) -> Option<JunctionRule> {
        let limits = spec.power.as_ref()?.drop_limits.as_ref()?;
        if limits.is_empty() {
            return None;
        }
        let st = &spec.board.stackup;
        let fab = st.fab_tolerances.as_ref()?;
        let mut nets: Vec<String> = limits.iter().map(|l| l.domain.clone()).collect();
        nets.push(crate::route::GROUND.to_owned());
        Some(JunctionRule {
            nets,
            hole_plus_nm: nm(fab.hole_size_plus_um.as_ref()?.value),
            hole_offset_nm: nm(fab.hole_position_um.as_ref()?.value),
            plating_nm: nm(st.plating_min_um.as_ref()?.value),
        })
    }

    /// The junction reach of a hole drilled `drill_nm`, as the gate prices
    /// it ([`crate::supply::junction_reach`] at [`crate::supply::BETA_NM`]).
    pub fn reach(&self, drill_nm: i64) -> i64 {
        crate::supply::junction_reach(
            drill_nm,
            self.hole_plus_nm,
            self.plating_nm,
            crate::supply::BETA_NM,
            self.hole_offset_nm,
        )
    }
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
    /// Each dielectric's declared maximum, in the same order, where the
    /// specification gives one (`AET-WO-1303b`): a via barrel prices that
    /// row at it instead of at the finished maximum less the other minima.
    pub dielectric_max_nm: Vec<Option<i64>>,
    pub copper_max_nm: i64,
    pub copper_source: String,
    /// Relative permittivity, at most, in thousandths.
    pub permittivity_permille: u32,
    pub permittivity_source: String,
    pub clearance_nm: i64,
    pub width_nm: i64,
    /// The board's own via, pad and drill: the engine's 0.6/0.3 mm, or the
    /// ring rule's pad where the specification declares the fabricator's
    /// tolerances (`AET-WO-1314`).
    pub via_nm: i64,
    pub drill_nm: i64,
    /// The nets (by the board's name) `extract` bounds: each I2C bus's SDA
    /// and SCL, each SPI bus's SCK and MOSI (`extract::lines`), each of
    /// which `validate` requires a capacitance bound for
    /// (`MissingBusConstraint`, `MissingSpiConstraint`). The capacitance
    /// ladder prices their vias (`AET-WO-1314`); a constrained net
    /// `extract` does not bound is not among them.
    pub bounded: Vec<String>,
    /// What the ring rule changed that the specification did not declare:
    /// each thermal array's via raised to it, for the verdict.
    pub notes: Vec<String>,
    /// The declared net classes and coupled pairs (`AET-WO-1308`), each net
    /// by the board's name for it. Empty for a specification that declares
    /// none, whose board is then today's.
    pub classes: Vec<Class>,
    pub pairs: Vec<Pair>,
    /// The oscillator nets beside a declared crystal, by the board's name
    /// (`AET-WO-4240c` §1.6): the router lays them right after the stitched
    /// nets, on F.Cu, and never offers them a via. Empty for a specification
    /// that declares no crystal, whose board is then today's.
    pub oscillator: Vec<String>,
    /// How each declared through-hole part's pads join a plane or zone of
    /// their own net, by reference (`AET-WO-1309` F5). A through-hole pad of
    /// a filled net with no entry here is refused by name (`INV-06`).
    pub joins: Vec<(String, Join)>,
    /// Each declared thermal-via array: the pad it is under and its rule
    /// (`AET-WO-1309` §1.2). The engine emits it; the gate judges it from
    /// this, never from the board.
    pub thermal: Vec<ThermalPad>,
    /// Each declared IMU's seat, by reference (`AET-WO-4400d`): the gate
    /// holds the board's footprint to it ([`crate::route::declared`]).
    pub imus: Vec<(String, crate::schematic::Seat)>,
    /// The supply gate's junction rule, where the gate runs.
    pub junction: Option<JunctionRule>,
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
    /// Refused with the first field the ring rule needs that a declared
    /// `fab_tolerances` block lacks ([`RingRule::of`]), never read with the
    /// engine's via instead (`AET-WO-1315` §1.6).
    pub fn of(spec: &Specification) -> Result<Stack, &'static str> {
        let b = &spec.board;
        let s = &b.stackup;
        let f = &s.finished_thickness_um;
        let mut st = Stack {
            via_nm: crate::route::VIA_NM,
            drill_nm: crate::route::DRILL_NM,
            notes: Vec::new(),
            bounded: spec
                .i2c_buses
                .iter()
                .flat_map(|b| [&b.sda.net, &b.scl.net])
                .chain(
                    spec.spi_buses
                        .iter()
                        .flat_map(|b| [&b.sck.net, &b.mosi.net]),
                )
                .map(|n| board_net(spec, n))
                .collect(),
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
            dielectric_max_nm: s
                .dielectrics
                .iter()
                .map(|d| d.max_thickness_um.map(nm))
                .collect(),
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
                    nets: c
                        .nets
                        .iter()
                        .map(|n| board_net(spec, n))
                        .chain(switch_nets(spec, &c.nets))
                        .collect(),
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
            oscillator: aethelgard_ir::names::oscillator_nets(spec)
                .iter()
                .map(|n| board_net(spec, n))
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
            // `validate` holds each seat's forward edge declared and its
            // part's axes cited, and the embedded parts table parsed.
            imus: crate::schematic::seats(spec)
                .expect("a validated specification's IMU seats are complete"),
            junction: JunctionRule::of(spec),
        };
        // The ring rule (`AET-WO-1314`, ruling 6982875594 D1, D2): the
        // board's own via is sized by it; a thermal array's via, the
        // engine's own part, is raised to it and noted. A class keeps its
        // declared via: an undersized one is refused before routing
        // ([`RingRule::unheld`]), never raised here.
        if let Some(rule) = RingRule::of(spec)? {
            st.via_nm = rule.pad_nm(st.drill_nm);
            // a class that declares no via is laid with the board's
            for c in st.classes.iter_mut().filter(|c| !c.via_declared) {
                c.rule.via_nm = st.via_nm;
                c.rule.drill_nm = st.drill_nm;
            }
            for t in &mut st.thermal {
                let pad = rule.pad_nm(t.drill_nm);
                if t.via_nm < pad {
                    st.notes.push(format!(
                        "via ring: {} pad {}'s thermal vias raised from {} to {} mm, the pad that keeps the declared ring round their {} mm drill",
                        t.reference,
                        t.pad,
                        crate::sexpr::mm(t.via_nm),
                        crate::sexpr::mm(pad),
                        crate::sexpr::mm(t.drill_nm)
                    ));
                    t.via_nm = pad;
                }
            }
        }
        Ok(st)
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
            via_nm: self.via_nm,
            drill_nm: self.drill_nm,
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

/// Each buck's switch node, by its board name, whose output domain is one of
/// `nets` (`AET-WO-4300d2` §1.4): the SW net carries the inductor's current,
/// the output's, so it is laid at the output's class. The emitter names it
/// `<reference>_SW` (`schematic::power`).
fn switch_nets(spec: &Specification, nets: &[String]) -> Vec<String> {
    let (Some(p), Ok(table)) = (&spec.power, aethelgard_ir::power_parts::table()) else {
        return Vec::new();
    };
    p.regulators
        .iter()
        .filter(|r| nets.contains(&r.output))
        .filter(|r| {
            table
                .get(&r.part)
                .is_some_and(|row| matches!(row.kind, aethelgard_ir::power_parts::Kind::Buck(_)))
        })
        .map(|r| crate::extract::board_net(&format!("{}_SW", r.reference)))
        .collect()
}

/// The board's name for a specification net: a rail (a voltage domain or
/// `GND`) by its own name, a signal by its root-sheet label (`pcb`).
fn board_net(spec: &Specification, net: &str) -> String {
    let rail = net == crate::schematic::GND
        || spec.voltage_domains.iter().any(|d| d.id == net)
        || (spec.power.is_some() && net == aethelgard_ir::power::BATTERY_NET);
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
        via_nm: crate::route::VIA_NM,
        drill_nm: crate::route::DRILL_NM,
        bounded: vec![],
        notes: vec![],
        copper: vec!["F.Cu".into(), "B.Cu".into()],
        planes: vec![],
        thickness: Thickness {
            nominal_nm: 1_600_000,
            min_nm: 340_000,
            max_nm: 2_000_000,
            source: thickness_source.into(),
        },
        dielectric_max_nm: vec![None],
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
        oscillator: vec![],
        joins: vec![],
        thermal: vec![],
        imus: vec![],
        junction: None,
    }
}

#[cfg(test)]
mod ring_tests {
    use super::*;

    /// JLCPCB's declared tolerances (+0.13 mm, ±0.075 mm per axis, so a
    /// 106 067 nm reach) and the rule's pad at each ring the stop measured.
    fn rule(ring_nm: i64) -> RingRule {
        RingRule {
            plus_nm: 130_000,
            position_nm: 75_000,
            ring_nm,
        }
    }

    /// The pad is the largest hole, twice the position's reach and twice the
    /// ring, rounded up to 10 µm: exactly, at each ring the stop measured.
    #[test]
    fn the_pad_holds_the_ring_at_the_largest_hole_and_the_worst_position() {
        // 300 000 + 130 000 + 2·106 067 = 642 134
        assert_eq!(rule(0).pad_nm(300_000), 650_000);
        assert_eq!(rule(28_000).pad_nm(300_000), 700_000);
        assert_eq!(rule(50_000).pad_nm(300_000), 750_000);
        assert_eq!(rule(0).pad_nm(254_000), 600_000);
        assert_eq!(rule(28_000).pad_nm(254_000), 660_000);
        assert_eq!(rule(50_000).pad_nm(254_000), 700_000);
        // the reach is ⌈e·√2⌉ exactly, and the rounding is up: at a hole that
        // lands on 10 µm the pad is exact, a nanometre more rounds up
        let r = RingRule {
            plus_nm: 0,
            position_nm: 0,
            ring_nm: 0,
        };
        assert_eq!(r.pad_nm(300_000), 300_000);
        assert_eq!(r.pad_nm(300_001), 310_000);
        let p = RingRule {
            plus_nm: 0,
            position_nm: 1,
            ring_nm: 0,
        };
        assert_eq!(
            p.pad_nm(299_996),
            300_000,
            "the reach of 1 nm per axis is 2 nm"
        );
        assert_eq!(p.pad_nm(299_997), 310_000);
        // where the float's ceiling of e·√2 falls one short (a Pell pair,
        // 318 281 039² − 2·225 058 681² = −1), the reach is still exact
        assert_eq!(diagonal_nm(225_058_681), 318_281_040);
        assert_eq!(diagonal_nm(75_000), 106_067);
    }

    /// A class whose declared via is under the rule's pad is named, with its
    /// pad and the pad that holds the ring; one at or over it, or a class
    /// that declares no via, is not.
    #[test]
    fn a_class_via_under_the_rule_is_named() {
        let mut st = reference();
        let class = |name: &str, via_nm: i64, declared: bool| Class {
            name: name.into(),
            rule: Rule {
                width_nm: 200_000,
                via_nm,
                drill_nm: 300_000,
            },
            via_declared: declared,
            nets: vec![],
        };
        st.classes = vec![
            class("SMALL", 700_000, true),
            class("EXACT", 750_000, true),
            class("UNDECLARED", 600_000, false),
        ];
        assert_eq!(
            rule(50_000).unheld(&st),
            [("SMALL".to_string(), 700_000, 750_000)]
        );
    }
}
