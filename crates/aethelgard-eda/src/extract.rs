//! Extraction, first step (`AET-WO-1305`): an upper bound on each I2C
//! line's capacitance, from the routed board's own copper.
//!
//! **Why a bound is enough.** RC rise time grows with C. When a line's
//! bound is at most its declared `max_capacitance_femtofarads`, the
//! campaign, which simulated the declared value, judged a rise at least as
//! slow as the board's (`RUL-19` (1)).
//!
//! **The principle.** With one conductor at potential 1 and every other
//! conductor at 0, any trial potential `ψ` that takes those values gives
//! `C ≤ ∫ ε |∇ψ|² dV`, for any distribution of permittivity `ε` (W. C. Chew,
//! *Lectures on Theory of Microwave and Optical Waveguides*,
//! arXiv:2107.09672v2, §7.1.2, eqs. 7.1.48 and 7.1.51). A floating conductor
//! may take any constant, so the same `ψ`, which is 0 on it, still bounds.
//!
//! **The trial.** For each convex piece of the line's copper (a track, a
//! via, a pad), `ψ = max(0, 1 − d/S)`, with `d` the distance to the piece
//! and `S` = [`CLEARANCE_NM`]. `|∇ψ| = 1/S` in the shell `0 < d < S` and 0
//! elsewhere, so `C_piece ≤ ε_max · V_shell / S²`. `ψ` is 0 on every foreign
//! conductor, because each one is at least `S` from the piece:
//! - on the same copper layer, by [`route::check`], which `read` runs on
//!   exactly the shapes used here (a track's stadium, a via's disk, a pad's
//!   box);
//! - on the other layer, across a dielectric thicker than `S` (asserted);
//! - in the dielectric, where the only conductors are via barrels inside
//!   their checked disks;
//! - in the ground zone's fill, which keeps the board's clearance (the DRC
//!   gate refills and judges it).
//!
//! The line is the union of its pieces, and `max(ψ_a, ψ_b)` is a trial for
//! the union whose energy is at most the sum, so the line's bound is the
//! sum of its pieces' bounds (overlaps are counted twice, which only adds).
//!
//! **The shell's volume** around a prism of height `c` over a convex base
//! of area `A` and perimeter `p` (Steiner's formula, derived for a prism in
//! [`shell`]):
//!
//! `V = c·(p·S + π·S²) + 2·A·S + (π/2)·p·S² + (4/3)·π·S³`.
//!
//! Every term has a positive coefficient, so substituting upper bounds for
//! π, ε0, `ε_r`, the copper thickness and the via's height keeps a bound.
//! Every division rounds up. The result is integer femtofarads.
//!
//! **What it is not:** a field solver, an estimate of the typical value, or
//! any claim about resistance or inductance. It bounds the board's copper
//! as drawn; [`CONDITIONS`] lists what it assumes of the fabricated board.

use std::collections::{BTreeMap, BTreeSet};

use crate::pcb::escape;
use crate::route::{self, Board, CLEARANCE_NM, DRILL_NM, Routed, VIA_NM, WIDTH_NM};
use crate::sexpr::{self, Sx};

/// π from above: 355/113 = 3.14159292… > π.
pub const PI_NUM: u128 = 355;
pub const PI_DEN: u128 = 113;

/// ε0 from above, in femtofarads per nanometre: CODATA 2022 gives
/// 8.8541878188(14) × 10⁻¹² F/m (NIST, physics.nist.gov/cgi-bin/cuu/Value?ep0);
/// value plus one standard uncertainty is 8.8541878202 × 10⁻¹² F/m, and
/// 1 F/m = 10⁶ fF/nm.
pub const EPS0_NUM: u128 = 88_541_878_202;
pub const EPS0_DEN: u128 = 10_000_000_000_000_000;

/// The relative permittivity bound: 5.4, the IPC-4101 maximum for FR-4 at
/// 1 MHz, as quoted in the "IPC Specification" column of Panasonic's
/// R-1566W data sheet (IPC-4101E /127: "Permittivity, maximum at 1MHz
/// 5,4") and Bungard's FR4 data sheet (IPC 4101A: "Permittivity, spec
/// maximum, A. @ 1 MHz 5.4"). Air (1), solder mask and every typical FR-4
/// value lie below it, and `C` cannot fall when `ε` rises anywhere, so one
/// bound covers the whole space.
pub const EPS_R_NUM: u128 = 54;
pub const EPS_R_DEN: u128 = 10;

/// The board's declared thickness, as the generator writes it (`pcb`,
/// `(general (thickness 1.6))`). `read` refuses a board declaring another.
pub const BOARD_THICKNESS_NM: i64 = 1_600_000;

/// A via's copper height, bounded: the declared thickness plus a quarter,
/// for fabrication tolerance (a condition, [`CONDITIONS`]).
pub const VIA_HEIGHT_NM: i64 = BOARD_THICKNESS_NM + BOARD_THICKNESS_NM / 4;

/// Finished outer copper, bounded: 70 µm (2 oz), twice KiCad's default
/// 35 µm. The board declares no copper weight, so this is a condition.
pub const COPPER_NM: i64 = 70_000;

/// UM10204 rev. 7.0 (1 October 2021), Table 10 "Characteristics of the SDA
/// and SCL I/O stages", page 43: `Ci`, "capacitance for each I/O pin",
/// maximum 10 pF in Standard-mode, Fast-mode and Fast-mode Plus. Its note
/// [10]: "Special purpose devices such as multiplexers and switches may
/// exceed this capacitance because they connect multiple paths together."
pub const PIN_FF: u64 = 10_000;

/// What the bound assumes of the fabricated board and its parts.
pub const CONDITIONS: &[&str] = &[
    "laminate and solder mask relative permittivity at most 5.4 (IPC-4101 FR-4 maximum at 1 MHz)",
    "finished copper at most 70 um thick; finished board at most 2.0 mm thick",
    "zone fill keeps the board's clearance (the DRC gate refills and judges it)",
    "every I2C pin at most UM10204's Ci of 10 pF; no multiplexer or switch on the line",
    "conductors not in the board file (a part's terminals above its pads) are not modelled",
    "neighbouring nets are static while the line rises",
];

/// The shell's half-width: every foreign conductor is at least this far.
const S: u128 = CLEARANCE_NM as u128;

// The other layer's copper is beyond the shell: the dielectric between the
// two copper layers is thicker than `S`.
const _: () = assert!(BOARD_THICKNESS_NM - 2 * COPPER_NM > CLEARANCE_NM);
// A via's barrel lies inside its checked disk.
const _: () = assert!(DRILL_NM <= VIA_NM);

/// The common denominator of every area, perimeter and volume below: it
/// clears π's 113 twice, and the 4 and 3 of `W²/4` and `4/3`.
const D: u128 = PI_DEN * PI_DEN * 12;

/// `⌈a / b⌉`.
const fn ceil_div(a: u128, b: u128) -> u128 {
    a.div_ceil(b)
}

const fn mul(a: u128, b: u128) -> Option<u128> {
    a.checked_mul(b)
}

const fn add(a: u128, b: u128) -> Option<u128> {
    a.checked_add(b)
}

/// A convex base, as `D` times its area and `D` times its perimeter, each
/// with π taken from above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Base {
    area: u128,
    perimeter: u128,
}

impl Base {
    /// A `w` × `h` box.
    pub const fn boxed(w: u128, h: u128) -> Option<Base> {
        let Some(wh) = mul(w, h) else { return None };
        let Some(area) = mul(wh, D) else { return None };
        let Some(sum) = add(w, h) else { return None };
        let Some(p) = mul(sum, 2 * D) else {
            return None;
        };
        Some(Base { area, perimeter: p })
    }

    /// A disk of radius `r`: area `π r²`, perimeter `2 π r`.
    pub const fn disk(r: u128) -> Option<Base> {
        let Some(r2) = mul(r, r) else { return None };
        let Some(area) = mul(r2, PI_NUM * PI_DEN * 12) else {
            return None;
        };
        let Some(perimeter) = mul(r, 2 * PI_NUM * PI_DEN * 12) else {
            return None;
        };
        Some(Base { area, perimeter })
    }
}

/// `D` times the volume of the points within `s` of a prism of height `c`
/// over `base`, minus the prism. Derivation: the prism grown sideways by
/// `s` adds `c·(p·s + π s²)` (a convex region grown by `s` gains `p·s + π s²`
/// in area); each end adds `∫₀ˢ (A + p·√(s²−z²) + π(s²−z²)) dz` = `A·s +
/// π p s²/4 + (2/3) π s³`. For a box this is the familiar
/// `2(ab+bc+ca)s + π(a+b+c)s² + (4/3)π s³`.
pub const fn shell(base: Base, c: u128, s: u128) -> Option<u128> {
    let Some(s2) = mul(s, s) else { return None };
    let Some(s3) = mul(s2, s) else { return None };
    // c·p·s
    let Some(cs) = mul(c, s) else { return None };
    let Some(t1) = mul(cs, base.perimeter) else {
        return None;
    };
    // c·π·s²
    let Some(cs2) = mul(c, s2) else { return None };
    let Some(t2) = mul(cs2, PI_NUM * PI_DEN * 12) else {
        return None;
    };
    // 2·A·s
    let Some(t3) = mul(2 * s, base.area) else {
        return None;
    };
    // (π/2)·p·s², rounded up
    let Some(ps2) = mul(base.perimeter, s2) else {
        return None;
    };
    let Some(t4) = mul(ps2, PI_NUM) else {
        return None;
    };
    let t4 = ceil_div(t4, 2 * PI_DEN);
    // (4/3)·π·s³ · D = 4·355·113·4·s³
    let Some(t5) = mul(s3, 4 * PI_NUM * PI_DEN * 4) else {
        return None;
    };
    let Some(a) = add(t1, t2) else { return None };
    let Some(b) = add(a, t3) else { return None };
    let Some(c) = add(b, t4) else { return None };
    add(c, t5)
}

/// Femtofarads, rounded up, from `D` times a shell's volume in nm³:
/// `ε0 · ε_r · V / S²`.
pub const fn femtofarads(volume_d: u128) -> Option<u64> {
    let Some(a) = mul(volume_d, EPS0_NUM * EPS_R_NUM) else {
        return None;
    };
    let den = EPS0_DEN * EPS_R_DEN * D * S * S;
    let ff = ceil_div(a, den);
    if ff > u64::MAX as u128 {
        return None;
    }
    Some(ff as u64)
}

const fn must(v: Option<u64>) -> u64 {
    match v {
        Some(v) => v,
        None => panic!("a derived constant overflowed"),
    }
}

const fn must_base(v: Option<Base>) -> Base {
    match v {
        Some(v) => v,
        None => panic!("a derived base overflowed"),
    }
}

const fn must_vol(v: Option<u128>) -> u128 {
    match v {
        Some(v) => v,
        None => panic!("a derived volume overflowed"),
    }
}

/// The per-length bound, femtofarads per millimetre of a track's length,
/// rounded up: `d/dL` of a track's shell. A track of centre length `L` is
/// a stadium (`A = L·W + π W²/4`, `p = 2L + π W`), so
/// `dV/dL = 2·c·S + 2·W·S + π·S²`.
pub const PER_MM_FF: u64 = {
    let (c, w) = (COPPER_NM as u128, WIDTH_NM as u128);
    let per_nm = (2 * c * S + 2 * w * S) * D + S * S * PI_NUM * PI_DEN * 12;
    must(femtofarads(per_nm * 1_000_000))
};

/// Each track's two round ends: the shell of a track of length 0, a disk of
/// diameter `WIDTH_NM`, `COPPER_NM` high.
pub const SEGMENT_FF: u64 = {
    let base = must_base(Base::disk(WIDTH_NM as u128 / 2));
    must(femtofarads(must_vol(shell(base, COPPER_NM as u128, S))))
};

/// One via: its checked disk, diameter `VIA_NM`, through `VIA_HEIGHT_NM`.
/// The barrel and both pads lie inside it.
pub const VIA_FF: u64 = {
    let base = must_base(Base::disk(VIA_NM as u128 / 2));
    must(femtofarads(must_vol(shell(base, VIA_HEIGHT_NM as u128, S))))
};

const _: () = assert!(WIDTH_NM % 2 == 0 && VIA_NM % 2 == 0);

/// One pad's bound: its box (a superset of its copper, and the shape
/// `route::check` clears), `COPPER_NM` high.
pub fn pad_ff(rect: crate::pcb::Rect) -> Option<u64> {
    let w = u128::try_from(rect.x1.checked_sub(rect.x0)?).ok()?;
    let h = u128::try_from(rect.y1.checked_sub(rect.y0)?).ok()?;
    femtofarads(shell(Base::boxed(w, h)?, COPPER_NM as u128, S)?)
}

/// A refusal: a named kind and a message. No estimate follows one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub kind: &'static str,
    pub message: String,
}

fn refused(kind: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        kind,
        message: message.into(),
    }
}

/// The four terms of one line's bound, femtofarads, each rounded up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Terms {
    pub trace: u64,
    pub vias: u64,
    pub pads: u64,
    pub pins: u64,
}

/// One I2C line's bound and what it was computed from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBound {
    /// The specification's net name.
    pub net: String,
    /// The board's name for it.
    pub board_net: String,
    pub bus: String,
    pub c_bound_femtofarads: u64,
    pub terms: Terms,
    /// Routed length, summed over every track, nanometres.
    pub length_nm: u64,
    pub segments: u64,
    pub via_count: u64,
    /// Every copper pad on the line, pins included.
    pub pad_count: u64,
    /// Pads counted at `Ci`, as `<reference>.<pad>`.
    pub pins: Vec<String>,
    /// The one part on the line that is neither a declared device nor a
    /// pull-up: the controller.
    pub controller: String,
}

fn copper_layer(name: &str) -> bool {
    name.ends_with(".Cu")
}

/// True when `item` or any list inside it names a copper layer.
fn on_copper(item: &Sx) -> bool {
    if matches!(item.head(), Some("layer" | "layers"))
        && item.items()[1..]
            .iter()
            .filter_map(Sx::text)
            .any(copper_layer)
    {
        return true;
    }
    item.items().iter().any(on_copper)
}

/// The top-level items that hold no copper geometry, whatever they name.
const NO_GEOMETRY: &[&str] = &[
    "version",
    "generator",
    "generator_version",
    "general",
    "paper",
    "title_block",
    "layers",
    "setup",
    "property",
    "net",
    "embedded_fonts",
];

/// Reads a `.kicad_pcb` text and refuses, before any estimate, a board the
/// bound does not hold for: unparseable; another thickness; copper that
/// `route::read` would not see (an arc, a graphic or text on copper, copper
/// in a footprint other than its pads); copper whose labels disagree with
/// its connections (`route::connectivity`); or copper closer than
/// [`CLEARANCE_NM`] to another net's (`route::check`).
pub fn read(text: &str) -> Result<(Board, Routed), Refused> {
    let board = sexpr::parse(text).map_err(|e| refused("BoardUnreadable", e.to_string()))?;
    if board.head() != Some("kicad_pcb") {
        return Err(refused("BoardUnreadable", "not a kicad_pcb"));
    }
    let thickness = board
        .child("general")
        .and_then(|g| g.child("thickness"))
        .and_then(|t| t.arg(1))
        .and_then(sexpr::nm);
    if thickness != Some(BOARD_THICKNESS_NM) {
        return Err(refused(
            "BoardThickness",
            format!(
                "the board declares thickness {thickness:?} nm; the bound is derived for {BOARD_THICKNESS_NM} nm"
            ),
        ));
    }
    for item in board.items().iter().skip(1) {
        let head = item.head().unwrap_or("");
        match head {
            "segment" | "via" | "zone" => {}
            "footprint" => {
                let reference = item.property("Reference").unwrap_or("?");
                if let Some(other) = item
                    .items()
                    .iter()
                    .skip(1)
                    // A footprint's own `layer` names its side.
                    .find(|c| !matches!(c.head(), Some("pad" | "layer")) && on_copper(c))
                {
                    return Err(refused(
                        "UnmodelledCopper",
                        format!(
                            "{reference} has `{}` on a copper layer, which the bound does not model",
                            other.head().unwrap_or("?")
                        ),
                    ));
                }
            }
            h if NO_GEOMETRY.contains(&h) => {}
            h if on_copper(item) => {
                return Err(refused(
                    "UnmodelledCopper",
                    format!(
                        "a top-level `{h}` is on a copper layer, which the bound does not model"
                    ),
                ));
            }
            _ => {}
        }
    }
    let (board, routed) = route::read(&board).map_err(|e| refused("BoardUnreadable", e))?;
    route::connectivity(&board, &routed).map_err(|e| refused("NetLabelsDisagree", e))?;
    route::check(&board, &routed).map_err(|e| refused("ClearanceViolated", e))?;
    Ok((board, routed))
}

/// The board's name for a specification label (`pcb`'s own mapping).
pub fn board_net(label: &str) -> String {
    format!("/{}", escape(label))
}

/// One I2C bus as the bound needs it: its lines' specification net names,
/// its rail's, and its declared devices' references. The caller builds it
/// from the solved topology (`RUL-16`: this module never reads the
/// specification's device list).
#[derive(Clone, Debug)]
pub struct Bus<'a> {
    pub id: &'a str,
    pub sda: &'a str,
    pub scl: &'a str,
    /// The voltage domain's id, which names the rail net.
    pub domain: &'a str,
    pub devices: BTreeSet<&'a str>,
}

/// Every I2C line of `buses`, bus by bus, SDA then SCL, bounded on a board
/// `read` accepted.
///
/// The parts on a line are classified from the bus and the board alone: a
/// declared I2C device of the bus (each of its pads on the line is a pin);
/// a pull-up (a part with exactly two copper pads, the other on the bus's
/// rail; copper only); and exactly one other part, the controller (each of
/// its pads on the line is a pin). Any further part is refused, because its
/// capacitance is not known. Every pad's copper is counted, pins included:
/// `Ci` is the part's, not the land's.
pub fn bound(board: &Board, routed: &Routed, buses: &[Bus<'_>]) -> Result<Vec<LineBound>, Refused> {
    let mut out = Vec::new();
    for bus in buses {
        let rail = escape(bus.domain);
        let mut controllers = BTreeSet::new();
        for net in [bus.sda, bus.scl] {
            let line = line(board, routed, bus.id, net, &rail, &bus.devices)?;
            controllers.insert(line.controller.clone());
            out.push(line);
        }
        if controllers.len() != 1 {
            return Err(refused(
                "UnmodelledPart",
                format!(
                    "bus `{}`: its lines have different controllers {controllers:?}",
                    bus.id
                ),
            ));
        }
    }
    Ok(out)
}

fn overflow(net: &str) -> Refused {
    refused(
        "Overflow",
        format!("`{net}`: the bound does not fit in integer arithmetic"),
    )
}

fn line(
    board: &Board,
    routed: &Routed,
    bus: &str,
    net: &str,
    rail: &str,
    devices: &BTreeSet<&str>,
) -> Result<LineBound, Refused> {
    let board_net = board_net(net);
    let on = |n: &Option<String>| n.as_deref() == Some(board_net.as_str());
    // Each part with a pad on the line, and its pads there.
    let mut parts: BTreeMap<&str, (usize, Vec<&route::Pad>)> = BTreeMap::new();
    for (i, part) in board.parts.iter().enumerate() {
        let here: Vec<&route::Pad> = part.pads.iter().filter(|p| on(&p.net)).collect();
        if !here.is_empty() {
            if parts.contains_key(part.reference.as_str()) {
                return Err(refused(
                    "UnmodelledPart",
                    format!("`{net}`: two footprints are {}", part.reference),
                ));
            }
            parts.insert(&part.reference, (i, here));
        }
    }
    if parts.is_empty() {
        return Err(refused(
            "LineNotOnBoard",
            format!("`{net}` ({board_net}) has no pad on the board"),
        ));
    }
    for d in devices {
        if !parts.contains_key(d) {
            return Err(refused(
                "DeviceNotOnLine",
                format!("{d} is declared on bus `{bus}` but has no pad on `{net}`"),
            ));
        }
    }
    let pull_up = |i: usize| {
        let pads = &board.parts[i].pads;
        pads.len() == 2
            && pads.iter().filter(|p| on(&p.net)).count() == 1
            && pads.iter().any(|p| p.net.as_deref() == Some(rail))
    };
    let others: Vec<&str> = parts
        .iter()
        .filter(|(r, (i, _))| !devices.contains(*r) && !pull_up(*i))
        .map(|(r, _)| *r)
        .collect();
    let [controller] = others[..] else {
        return Err(refused(
            "UnmodelledPart",
            format!(
                "`{net}`: expected exactly one part besides the declared devices and pull-ups, found {others:?}"
            ),
        ));
    };

    let mut terms = Terms::default();
    let mut pins = Vec::new();
    let mut pad_count = 0u64;
    for (reference, (i, pads)) in &parts {
        let is_pin = !pull_up(*i);
        for pad in pads {
            let ff = pad_ff(pad.rect).ok_or_else(|| overflow(net))?;
            terms.pads = terms.pads.checked_add(ff).ok_or_else(|| overflow(net))?;
            pad_count += 1;
            if is_pin {
                pins.push(format!("{reference}.{}", pad.number));
                terms.pins = terms
                    .pins
                    .checked_add(PIN_FF)
                    .ok_or_else(|| overflow(net))?;
            }
        }
    }

    let mut length: u128 = 0;
    let mut segments = 0u64;
    for s in routed.segments.iter().filter(|s| s.net == board_net) {
        let d = |a: i64, b: i64| u128::from(a.abs_diff(b));
        length = length
            .checked_add(d(s.start.0, s.end.0) + d(s.start.1, s.end.1))
            .ok_or_else(|| overflow(net))?;
        segments += 1;
    }
    let per_length = length
        .checked_mul(u128::from(PER_MM_FF))
        .ok_or_else(|| overflow(net))?
        .div_ceil(1_000_000);
    terms.trace = u64::try_from(per_length)
        .ok()
        .and_then(|t| t.checked_add(segments.checked_mul(SEGMENT_FF)?))
        .ok_or_else(|| overflow(net))?;
    let via_count = routed.vias.iter().filter(|v| v.net == board_net).count() as u64;
    terms.vias = via_count.checked_mul(VIA_FF).ok_or_else(|| overflow(net))?;
    let total = [terms.trace, terms.vias, terms.pads, terms.pins]
        .into_iter()
        .try_fold(0u64, u64::checked_add)
        .ok_or_else(|| overflow(net))?;
    Ok(LineBound {
        net: net.to_owned(),
        board_net,
        bus: bus.to_owned(),
        c_bound_femtofarads: total,
        terms,
        length_nm: u64::try_from(length).map_err(|_| overflow(net))?,
        segments,
        via_count,
        pad_count,
        pins,
        controller: controller.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ε0 · ε_r in fF/nm, from CODATA's value (not its upper bound).
    const EPS: f64 = 8.854_187_818_8e-6 * 5.4;
    const PI: f64 = std::f64::consts::PI;

    /// The shell around a prism, in nm³, with true π.
    fn shell_f(area: f64, perimeter: f64, c: f64, s: f64) -> f64 {
        c * (perimeter * s + PI * s * s)
            + 2.0 * area * s
            + PI / 2.0 * perimeter * s * s
            + 4.0 / 3.0 * PI * s * s * s
    }

    fn ff(volume: f64) -> f64 {
        let s = CLEARANCE_NM as f64;
        EPS * volume / (s * s)
    }

    /// The integer constant bounds its formula from above, and by less than
    /// one femtofarad plus the slack of π's and ε0's upper bounds.
    fn pinned(name: &str, constant: u64, exact: f64) {
        let c = constant as f64;
        assert!(c >= exact, "{name}: {c} under {exact}");
        assert!(
            c - exact < 1.0 + exact * 1e-6,
            "{name}: {c} over {exact} by 1 fF or more"
        );
    }

    /// §1.4: each derived constant against its formula, and against the
    /// values the §1.3 measurement used, so a change to a router constant
    /// re-derives it here and fails loudly.
    #[test]
    fn each_constant_is_its_formula_rounded_up() {
        let (s, c, w) = (CLEARANCE_NM as f64, COPPER_NM as f64, WIDTH_NM as f64);
        let per_nm = 2.0 * c * s + 2.0 * w * s + PI * s * s;
        pinned("PER_MM_FF", PER_MM_FF, ff(per_nm * 1e6));
        let r = w / 2.0;
        pinned(
            "SEGMENT_FF",
            SEGMENT_FF,
            ff(shell_f(PI * r * r, 2.0 * PI * r, c, s)),
        );
        let r = VIA_NM as f64 / 2.0;
        pinned(
            "VIA_FF",
            VIA_FF,
            ff(shell_f(PI * r * r, 2.0 * PI * r, VIA_HEIGHT_NM as f64, s)),
        );
        for (pw, ph) in [
            (300_000, 1_475_000),
            (540_000, 640_000),
            (1, 1),
            (5_000_000, 3_000_000),
        ] {
            let rect = crate::pcb::Rect {
                x0: 7,
                y0: -3,
                x1: 7 + pw,
                y1: -3 + ph,
            };
            let (a, p) = ((pw * ph) as f64, 2.0 * (pw + ph) as f64);
            pinned("pad_ff", pad_ff(rect).unwrap(), ff(shell_f(a, p, c, s)));
        }
        assert_eq!(
            (PER_MM_FF, SEGMENT_FF, VIA_FF, PIN_FF),
            (280, 124, 1519, 10_000),
            "a constant moved: re-measure §1.3 before changing this line"
        );
    }

    /// The prism formula is Steiner's for a box, `2(ab+bc+ca)s + π(a+b+c)s² +
    /// (4/3)π s³`.
    #[test]
    fn the_prism_shell_is_steiners_formula_for_a_box() {
        let (a, b, c, s) = (540_000.0, 640_000.0, 70_000.0, 200_000.0);
        let steiner = 2.0 * (a * b + b * c + c * a) * s
            + PI * (a + b + c) * s * s
            + 4.0 / 3.0 * PI * s * s * s;
        let prism = shell_f(a * b, 2.0 * (a + b), c, s);
        assert!((steiner - prism).abs() < 1e-6 * steiner);
        // The integer shell, with π from above, is at least the exact one.
        let d = D as f64;
        let integer = shell(Base::boxed(540_000, 640_000).unwrap(), 70_000, 200_000).unwrap();
        assert!(integer as f64 / d >= prism);
    }

    fn agm(mut a: f64, mut b: f64) -> f64 {
        for _ in 0..64 {
            (a, b) = ((a + b) / 2.0, (a * b).sqrt());
        }
        a
    }

    /// The complete elliptic integral of the first kind, `K(k)`.
    fn k(k: f64) -> f64 {
        PI / (2.0 * agm(1.0, (1.0 - k * k).sqrt()))
    }

    /// The bound dominates the exact solutions it must: a zero-thickness
    /// coplanar strip of width `W` between ground planes at `S` on both
    /// sides, in a medium of `ε_max` everywhere (conformal mapping, exact:
    /// `4 ε K(k)/K(k')`, `k = W / (W + 2S)`), and a via's barrel inside a
    /// coaxial ground at `S` (`2π ε / ln((R + S)/R)` per length). Both are
    /// geometries in which every foreign conductor is at least `S` away, so
    /// the trial bounds them; a bound below either would be a defect.
    #[test]
    fn the_bound_dominates_the_exact_solutions_of_its_class() {
        let (w, s) = (WIDTH_NM as f64, CLEARANCE_NM as f64);
        let kk = w / (w + 2.0 * s);
        let cpw_per_mm = 4.0 * EPS * k(kk) / k((1.0 - kk * kk).sqrt()) * 1e6;
        assert!(PER_MM_FF as f64 >= cpw_per_mm, "{cpw_per_mm}");
        let r = VIA_NM as f64 / 2.0;
        let coax = 2.0 * PI * EPS / ((r + s) / r).ln() * VIA_HEIGHT_NM as f64;
        assert!(VIA_FF as f64 >= coax, "{coax}");
        // The side of the via's shell alone already dominates the coax.
        let side = ff((2.0 * PI * r * s + PI * s * s) * VIA_HEIGHT_NM as f64);
        assert!(side >= coax, "{side} {coax}");
    }

    #[test]
    fn a_pad_too_large_to_compute_is_none_not_a_wrap() {
        let huge = crate::pcb::Rect {
            x0: i64::MIN,
            y0: 0,
            x1: i64::MAX,
            y1: 1,
        };
        assert_eq!(pad_ff(huge), None);
        let inverted = crate::pcb::Rect {
            x0: 5,
            y0: 0,
            x1: 0,
            y1: 1,
        };
        assert_eq!(pad_ff(inverted), None);
    }
}
