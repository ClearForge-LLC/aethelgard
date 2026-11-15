//! Extraction, first step (`AET-WO-1305`): an upper bound on each I2C
//! line's capacitance, from the routed board's own copper. `AET-WO-4300c`
//! widens the line list to each SPI bus's SCK and MOSI, whose declared
//! capacitance chooses its pin's output speed: the copper is bounded the
//! same way, and only the pins are counted differently (see [`lines`]).
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
//! and `S` the specification's declared clearance. `|∇ψ| = 1/S` in the shell
//! `0 < d < S` and 0 elsewhere, so `C_piece ≤ ε_max · V_shell / S²`. `ψ` is 0
//! on every foreign conductor, because each one is at least `S` from the
//! piece:
//! - on the same copper layer, by [`route::check`], which `read` runs on
//!   exactly the shapes used here (a track's stadium, a via's disk, a pad's
//!   box);
//! - on the next copper layer, across a dielectric whose declared minimum
//!   `h` is at least `S`; when it is less, the cap below applies;
//! - in the dielectric, where the only conductors are via barrels inside
//!   their checked disks;
//! - in a zone's or plane's fill, which keeps the board's clearance from
//!   the line's vias (the DRC gate refills and judges it).
//!
//! **The cap, for an outer piece over a dielectric thinner than `S`**
//! (`AET-WO-4100` S4, ruled on PR #120). A piece is a prism: a convex base
//! `B` (area `A`, perimeter `p`), height `c`, bottom face at `z = 0`, the
//! dielectric below, and the next copper layer, plane or signal, no nearer
//! than `h < S` (the declared minimum of that dielectric). Take
//! `ρ = d/S` for `z ≥ 0` (today's trial) and `ρ = √((d_B/S)² + (z/h)²)` for
//! `z < 0`, with `d_B` the planar distance to `B`, and `ψ = max(0, 1 − ρ)`:
//! 1. `ψ = 1` on the piece;
//! 2. `ψ` is continuous (at `z = 0` both forms are `d_B/S`) and Lipschitz;
//! 3. below, `z² < h²(1 − d_B²/S²) ≤ S² − d_B²`, so the support lies inside
//!    today's `S`-shell, and every conductor that argument excludes stays
//!    excluded;
//! 4. below, the support ends at `|z| < h`, so `ψ` is 0 on the next copper
//!    layer, whatever it carries.
//!
//! Above and beside, the energy is today's. Below, directly under `B`,
//! `|∇ρ|² = 1/h²` over `A·h`; beyond it, at `d_B = s` (area `(p + 2πs) ds`,
//! Steiner), with `s = Su`, `|z| = hv` and polar `(u, v) = r(cos θ, sin θ)`,
//! `|∇ρ|² = cos²θ/S² + sin²θ/h²`. Integrating:
//!
//! `lower(A, p, h) = A/h + πph/(8S) + πpS/(8h) + 4πh/9 + 2πS²/(9h)`,
//!
//! which at `h = S` is today's bottom end cap `A/S + πp/4 + 2πS/3`, term by
//! term. So the bound is today's shell plus `S²·(lower(h) − lower(S))`
//! ([`cap`]), an exact rational that is 0 at `h = S`. **When the declared
//! minimum is at least `S`, today's integer path runs unchanged** (ruled
//! D5). Checked against a quadrature of the same `ψ`, the exact
//! zero-thickness stripline (Cohn), and the identity at `h = S` (tests).
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
//! any claim about resistance or inductance. It bounds the bare board's
//! copper as [`read`] reads it (a reader that refuses whatever KiCad could
//! read otherwise), plus each pin's `Ci`. Part bodies, solder and a part's
//! terminations above its pads are outside it; [`CONDITIONS`] states that
//! scope and what the bound assumes of the fabricated board.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::{PinTable, SolvedTopology, Specification};

use crate::pcb::escape;
use crate::route::{self, Board, DRILL_NM, Routed, VIA_NM};
use crate::schematic::Conn;
use crate::sexpr;
use crate::stack::Stack;
use crate::strict::{self, Node};

/// π from above: 355/113 = 3.14159292… > π.
pub const PI_NUM: u128 = 355;
pub const PI_DEN: u128 = 113;

/// ε0 from above, in femtofarads per nanometre: CODATA 2022 gives
/// 8.8541878188(14) × 10⁻¹² F/m (NIST, physics.nist.gov/cgi-bin/cuu/Value?ep0);
/// value plus one standard uncertainty is 8.8541878202 × 10⁻¹² F/m, and
/// 1 F/m = 10⁶ fF/nm.
pub const EPS0_NUM: u128 = 88_541_878_202;
pub const EPS0_DEN: u128 = 10_000_000_000_000_000;

/// The bound's physical inputs, from the specification's declared board
/// (`AET-WO-4100`, ruled D2 and D5). Each is a bound with a source, and
/// [`conditions`] states them:
/// - `S`, the declared clearance;
/// - `W`, the declared track width;
/// - `c`, the finished copper's maximum (`AET-WO-1305`: 70 µm on the
///   reference);
/// - `ε_r`, the laminate's and mask's relative permittivity maximum, as a
///   reduced fraction (`C` cannot fall when `ε` rises anywhere, so one bound
///   covers the board and the air around it; part bodies are outside the
///   scope);
/// - a via's height: the declared maximum finished thickness. The barrel
///   and both pads lie within the finished board, so the maximum bounds it
///   and no margin is added (`AET-WO-1305` added a quarter to a 1.6 mm board
///   that declared no maximum; the reference now declares 2.0 mm, the same
///   height);
/// - `h_top` and `h_bottom`: the declared minimum of the dielectric under
///   `F.Cu` and over `B.Cu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physics {
    s: u128,
    w: u128,
    /// A via's pad diameter: the engine's, or the line's class's.
    via: u128,
    c: u128,
    er_num: u128,
    er_den: u128,
    via_h: u128,
    h_top: u128,
    h_bottom: u128,
    /// Every piece takes the cap path, at `min(h, S)`: only
    /// [`bound_by_cap_path`] sets it, to show the two paths agree at `h = S`.
    force_cap: bool,
}

impl Physics {
    pub fn of(stack: &Stack) -> Physics {
        let u = |v: i64| u128::try_from(v).unwrap_or(0);
        let (top, bottom) = stack.outer_dielectrics_nm();
        let num = u128::from(stack.permittivity_permille);
        let g = gcd(num, 1000);
        Physics {
            s: u(stack.clearance_nm),
            w: u(stack.width_nm),
            via: u(VIA_NM),
            c: u(stack.copper_max_nm),
            er_num: num / g,
            er_den: 1000 / g,
            via_h: u(stack.thickness.max_nm),
            h_top: u(top),
            h_bottom: u(bottom),
            force_cap: false,
        }
    }

    /// The same for a line laid by `rule` (`AET-WO-1308`): a class's or a
    /// pair's width, and a class's via. The board's own rule changes nothing.
    pub fn laid_by(self, rule: crate::stack::Rule) -> Physics {
        let u = |v: i64| u128::try_from(v).unwrap_or(0);
        Physics {
            w: u(rule.width_nm),
            via: u(rule.via_nm),
            ..self
        }
    }

    /// The dielectric's minimum under a piece on `layer`.
    fn h(&self, layer: route::Layer) -> u128 {
        match layer {
            route::Layer::F => self.h_top,
            route::Layer::B => self.h_bottom,
        }
    }
}

const fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// UM10204 rev. 7.0 (1 October 2021), Table 10 "Characteristics of the SDA
/// and SCL I/O stages", page 43: `Ci`, "capacitance for each I/O pin",
/// maximum 10 pF in Standard-mode, Fast-mode and Fast-mode Plus. Its note
/// [10]: "Special purpose devices such as multiplexers and switches may
/// exceed this capacitance because they connect multiple paths together."
pub const PIN_FF: u64 = 10_000;

/// `nm` in millimetres, at least one decimal: `2_000_000` is `2.0`.
fn mm1(nm: i64) -> String {
    let s = crate::sexpr::mm(nm);
    if s.contains('.') { s } else { format!("{s}.0") }
}

/// Thousandths as a decimal: `5400` is `5.4`.
pub fn permille(v: u32) -> String {
    let frac = format!("{:03}", v % 1000);
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        (v / 1000).to_string()
    } else {
        format!("{}.{frac}", v / 1000)
    }
}

/// What the bound assumes of a connector's pins (`AET-WO-4300f` D8), stated
/// beside [`conditions`] when a line carries a declared off-board budget, as
/// [`SPI_CONDITION`] is, so every other verdict and the fabrication notes of a
/// board without connectors are unchanged.
pub const OFFBOARD: &str = "beyond the board, each connector pin on a line adds the capacitance its specification declares for the mated contact, the cable and the remote pins, as declared with its source (the line's `offboard`); nothing here measures it";

/// What the bound assumes of the fabricated board and its parts, from the
/// declared stack. At 2 layers the words are `AET-WO-1305`'s, so the
/// reference's `extract` output and `FAB-NOTES.md` are unchanged (ruled D5):
/// there the dielectric follows from the finished minimum less both copper
/// maxima, which `validate` requires. At 4 each dielectric's minimum is
/// stated as well.
pub fn conditions(stack: &Stack) -> Vec<String> {
    let mut out = vec![
        "scope: the bare board's copper and each I2C pin's Ci; part bodies, solder and terminations above the pads are not included".to_owned(),
        format!(
            "laminate and solder mask relative permittivity at most {} ({})",
            permille(stack.permittivity_permille),
            stack.permittivity_source
        ),
        format!(
            "finished copper at most {} um thick; finished board from {} mm to {} mm thick",
            stack.copper_max_nm / 1000,
            mm1(stack.thickness.min_nm),
            mm1(stack.thickness.max_nm)
        ),
    ];
    if stack.has_planes() {
        let each: Vec<String> = stack
            .dielectrics
            .iter()
            .map(|(a, b, min, _)| format!("{a} to {b} {} mm", crate::sexpr::mm(*min)))
            .collect();
        out.push(format!(
            "each dielectric at least its declared minimum: {}",
            each.join(", ")
        ));
    }
    out.extend(
        [
            "zone fill keeps the board's clearance (the DRC gate refills and judges it)",
            "every I2C pin at most UM10204's Ci of 10 pF; no multiplexer or switch on the line",
            "neighbouring nets are static while the line rises",
        ]
        .map(str::to_owned),
    );
    out
}

/// What the bound assumes of an SPI line's pins, stated beside
/// [`conditions`] when the specification has SPI lines (`AET-WO-4300c` D3).
/// [`conditions`] itself is unchanged, so the fabrication notes are too.
pub const SPI_CONDITION: &str = "on SCK and MOSI, each device pin at its part's cited maximum pin capacitance (parts table); the MCU's own pin drives the line and is not counted (DS12117 Table 149's C is the external load)";

// A via's barrel lies inside its checked disk.
const _: () = assert!(DRILL_NM <= VIA_NM);

/// The common denominator of every area, perimeter and volume below: it
/// clears π's 113 twice, and the 4 and 3 of `W²/4` and `4/3`.
const D: u128 = PI_DEN * PI_DEN * 12;

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
    let t4 = t4.div_ceil(2 * PI_DEN);
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
/// `ε0 · ε_r · V / S²`, as one exact division. When the product overflows
/// (a permittivity whose thousandths do not reduce, on a large piece), the
/// same value in two divisions, each rounded up, which can only add.
pub const fn femtofarads(ph: &Physics, volume_d: u128) -> Option<u64> {
    let Some(a) = mul(volume_d, EPS0_NUM) else {
        return None;
    };
    let Some(a) = mul(a, ph.er_num) else {
        return staged(ph, volume_d);
    };
    let Some(den) = mul(EPS0_DEN, ph.er_den) else {
        return None;
    };
    let Some(den) = mul(den, D) else { return None };
    let Some(s2) = mul(ph.s, ph.s) else {
        return None;
    };
    let Some(den) = mul(den, s2) else { return None };
    let ff = a.div_ceil(den);
    if ff > u64::MAX as u128 {
        return None;
    }
    Some(ff as u64)
}

/// `⌈⌈V·ε0 / (ε0_den·D·S²)⌉ · ε_r_num / ε_r_den⌉`: at least the exact value.
const fn staged(ph: &Physics, volume_d: u128) -> Option<u64> {
    let Some(a) = mul(volume_d, EPS0_NUM) else {
        return None;
    };
    let Some(den) = mul(EPS0_DEN, D) else {
        return None;
    };
    let Some(s2) = mul(ph.s, ph.s) else {
        return None;
    };
    let Some(den) = mul(den, s2) else { return None };
    let Some(x) = mul(a.div_ceil(den), ph.er_num) else {
        return None;
    };
    let ff = x.div_ceil(ph.er_den);
    if ff > u64::MAX as u128 {
        return None;
    }
    Some(ff as u64)
}

/// `D · S² · (lower(A, p, h) − lower(A, p, S))`, rounded up, signed: what the
/// cap adds to today's shell, in the units [`shell`] returns. `area_d` and
/// `perimeter_d` are `D` times the base's (a [`Base`]); `ends` includes the
/// two terms that do not grow with a base (`4πh/9`, `2πS²/(9h)`), which a
/// per-length derivative leaves out. Term by term the difference factors:
///
/// - `A·S²·(1/h − 1/S) = A·S·(S − h)/h`;
/// - `(πp/8)·S²·(h/S + S/h − 2) = (πp/8)·S·(S − h)²/h`;
/// - `S²·(2π/9)·(2h + S²/h − 3S) = S²·(2π/9)·(S − h)(S − 2h)/h`;
///
/// so, with π = 355/113,
///
/// `cap = (S − h)·[72·113·A_d·S + 9·355·p_d·S·(S − h)
///   + ends·16·355·D·S²·(S − 2h)] / (72·113·h)`,
///
/// which is 0 at `h = S` by its first factor. `None` on overflow or `h = 0`.
pub fn cap(area_d: u128, perimeter_d: u128, ends: bool, h: u128, s: u128) -> Option<i128> {
    let i = |v: u128| i128::try_from(v).ok();
    let (h, s, a, p) = (i(h)?, i(s)?, i(area_d)?, i(perimeter_d)?);
    if h <= 0 {
        return None;
    }
    let gap = s.checked_sub(h)?;
    let pi = i(PI_NUM)?;
    let first = (72 * i(PI_DEN)?).checked_mul(a)?.checked_mul(s)?;
    let second = (9 * pi).checked_mul(p)?.checked_mul(s)?.checked_mul(gap)?;
    let mut sum = first.checked_add(second)?;
    if ends {
        let third = (16 * pi)
            .checked_mul(i(D)?)?
            .checked_mul(s.checked_mul(s)?)?
            .checked_mul(s.checked_sub(h.checked_mul(2)?)?)?;
        sum = sum.checked_add(third)?;
    }
    let num = gap.checked_mul(sum)?;
    let den = (72 * i(PI_DEN)?).checked_mul(h)?;
    // Rounded up: toward +∞ for either sign.
    Some(num.div_euclid(den) + i128::from(num.rem_euclid(den) != 0))
}

/// A piece's shell, `D` times nm³, with the cap when the dielectric below it
/// is thinner than `S` (else today's shell alone).
fn capped(ph: &Physics, base: Base, c: u128, h: u128, ends: bool) -> Option<u128> {
    let v = shell(base, c, ph.s)?;
    if h >= ph.s && !ph.force_cap {
        return Some(v);
    }
    let extra = cap(base.area, base.perimeter, ends, h.min(ph.s), ph.s)?;
    let total = i128::try_from(v).ok()?.checked_add(extra)?;
    u128::try_from(total).ok()
}

/// The per-length bound, femtofarads per millimetre of a track's length,
/// rounded up: `d/dL` of a track's shell. A track of centre length `L` is
/// a stadium (`A = L·W + π W²/4`, `p = 2L + πW`), so
/// `dV/dL = 2·c·S + 2·W·S + π·S²`, plus the cap's `dA/dL = W`, `dp/dL = 2`
/// terms when the dielectric below is thinner than `S`.
pub fn per_mm_ff(ph: &Physics, h: u128) -> Option<u64> {
    let (c, w, s) = (ph.c, ph.w, ph.s);
    let per_nm = (2 * c * s + 2 * w * s)
        .checked_mul(D)?
        .checked_add(s.checked_mul(s)?.checked_mul(PI_NUM * PI_DEN * 12)?)?;
    let per_nm = if h >= s && !ph.force_cap {
        per_nm
    } else {
        let extra = cap(w.checked_mul(D)?, 2 * D, false, h.min(s), s)?;
        u128::try_from(i128::try_from(per_nm).ok()?.checked_add(extra)?).ok()?
    };
    femtofarads(ph, per_nm.checked_mul(1_000_000)?)
}

/// Each track's two round ends: the shell of a track of length 0, a disk of
/// diameter `W`, `c` high.
pub fn segment_ff(ph: &Physics, h: u128) -> Option<u64> {
    let base = Base::disk(ph.w / 2)?;
    femtofarads(ph, capped(ph, base, ph.c, h, true)?)
}

/// One via: its checked disk, its pad's diameter (`VIA_NM`, or its class's),
/// through the declared maximum finished thickness. The barrel and both pads
/// lie inside it.
pub fn via_ff(ph: &Physics) -> Option<u64> {
    let base = Base::disk(ph.via / 2)?;
    femtofarads(ph, shell(base, ph.via_h, ph.s)?)
}

const _: () = assert!(VIA_NM % 2 == 0);

/// One pad's bound: its box (a superset of its copper, and the shape
/// `route::check` clears), `c` high, on F.Cu.
pub fn pad_ff(ph: &Physics, rect: crate::pcb::Rect) -> Option<u64> {
    femtofarads(ph, capped(ph, base_of(rect)?, ph.c, ph.h_top, true)?)
}

/// A pad's box as the bound's base.
fn base_of(rect: crate::pcb::Rect) -> Option<Base> {
    let w = u128::try_from(rect.x1.checked_sub(rect.x0)?).ok()?;
    let h = u128::try_from(rect.y1.checked_sub(rect.y0)?).ok()?;
    Base::boxed(w, h)
}

/// One through-hole pad (`AET-WO-1309`): its box through the declared
/// maximum finished thickness, as [`via_ff`] bounds a via. The barrel and
/// the pad's copper on every layer lie inside it.
pub fn through_hole_ff(ph: &Physics, rect: crate::pcb::Rect) -> Option<u64> {
    femtofarads(ph, shell(base_of(rect)?, ph.via_h, ph.s)?)
}

/// A pad's bound on a line: a through-hole pad's [`through_hole_ff`], an
/// SMD pad's [`pad_ff`].
pub fn pad_bound(ph: &Physics, pad: &route::Pad) -> Option<u64> {
    match pad.drill_nm {
        Some(_) => through_hole_ff(ph, pad.rect),
        None => pad_ff(ph, pad.rect),
    }
}

/// A refusal: a named kind and a message. No estimate follows one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub kind: &'static str,
    /// The path to the refused node, when the strict reader refused one.
    pub id: Option<String>,
    pub message: String,
}

fn refused(kind: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        kind,
        id: None,
        message: message.into(),
    }
}

/// The strict reader's refusal: an unreadable file or another kind is
/// `BoardUnreadable`, as before it; anything else is its own
/// `MalformedBoard`, naming the path.
fn malformed(r: strict::Refusal) -> Refused {
    match r {
        strict::Refusal::Unreadable(..) | strict::Refusal::NotThisKind(_) => {
            refused("BoardUnreadable", r.to_string())
        }
        r => Refused {
            kind: r.kind(),
            id: r.id().map(str::to_owned),
            message: r.to_string(),
        },
    }
}

fn invalid(e: &crate::schematic::EmitError) -> Refused {
    refused(e.kind(), e.to_string())
}

/// The terms of one line's bound, femtofarads, each rounded up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Terms {
    pub trace: u64,
    pub vias: u64,
    pub pads: u64,
    pub pins: u64,
    /// The off-board capacitance its connectors declare (`AET-WO-4300f` D8).
    pub offboard: u64,
}

/// One line's bound and what it was computed from.
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
    /// Pads counted as a pin, as `<reference>.<pad>`.
    pub pins: Vec<String>,
    /// `SCK` or `MOSI` for an SPI line; `None` for I2C.
    pub spi_signal: Option<&'static str>,
    /// The driver's pads, whose copper is counted and whose pin is not: the
    /// MCU's, on an SPI line (DS12117 Table 149's C is the external load).
    pub drivers: Vec<String>,
    /// The line's pull-ups, by reference: copper only.
    pub pull_ups: Vec<String>,
    /// Each connector pad on the line and the off-board capacitance it
    /// declares, as `<reference>.<pad>: <fF> fF (<source>)`.
    pub offboard: Vec<String>,
}

/// The largest coordinate or size the reader accepts, so that every
/// distance `route::check` squares fits in an `i64`.
pub const MAX_COORD_NM: i64 = 250_000_000;

/// A pad layer by KiCad's canonical name (a user name such as `TOP` could
/// stand for a copper layer the reader would not see as one).
fn canonical_layer(name: &str) -> bool {
    let Some((side, kind)) = name.split_once('.') else {
        return false;
    };
    matches!(side, "F" | "B" | "*")
        && matches!(
            kind,
            "Cu" | "Mask" | "Paste" | "SilkS" | "Fab" | "CrtYd" | "Adhes"
        )
}

/// True when `item` or any list inside it names a copper layer.
fn on_copper(item: &Node<'_>) -> bool {
    if matches!(item.head(), Some("layer" | "layers")) && item.texts().any(strict::copper_pad_layer)
    {
        return true;
    }
    item.lists().any(|c| on_copper(&c))
}

/// For each item `read` judges: the children it may hold, each with the
/// number of plain arguments it takes (`None`: any), and whether it may
/// repeat. Derived from the boards the generator writes and the vendored
/// footprints. Anything else is refused: a child `route` does not read (a
/// padstack, a drill offset, a fill) is copper or rule the bound would not
/// see. A repeated child KiCad holds once was already refused by the strict
/// reader (`AET-WO-1306`); the flags here refuse what KiCad may repeat but
/// the bound reads once.
fn allowed(parent: &str) -> Option<&'static [(&'static str, Option<usize>, bool)]> {
    Some(match parent {
        "general" => &[
            ("thickness", Some(1), false),
            ("legacy_teardrops", None, false),
        ],
        "segment" => &[
            ("start", Some(2), false),
            ("end", Some(2), false),
            ("width", Some(1), false),
            ("layer", Some(1), false),
            ("net", Some(1), false),
            ("uuid", Some(1), false),
        ],
        "via" => &[
            ("at", Some(2), false),
            ("size", Some(1), false),
            ("drill", Some(1), false),
            ("layers", Some(2), false),
            ("net", Some(1), false),
            ("uuid", Some(1), false),
        ],
        "pad" => &[
            ("at", None, false),
            ("size", Some(2), false),
            ("layers", None, false),
            ("net", Some(1), false),
            ("roundrect_rratio", Some(1), false),
            ("property", None, false),
            ("zone_connect", Some(1), false),
            ("uuid", Some(1), false),
            // A through-hole pad (`AET-WO-1309`): its round drill, and every
            // layer's copper kept, as `route::part` requires.
            ("drill", Some(1), false),
            ("remove_unused_layers", Some(1), false),
        ],
        "zone" => &[
            ("net", Some(1), false),
            ("layer", Some(1), false),
            ("uuid", Some(1), false),
            ("hatch", Some(2), false),
            ("min_thickness", Some(1), false),
            ("filled_areas_thickness", Some(1), false),
            ("fill", Some(0), false),
            ("polygon", Some(0), false),
        ],
        "fill" => &[
            ("thermal_gap", Some(1), false),
            ("thermal_bridge_width", Some(1), false),
            ("island_removal_mode", Some(1), false),
        ],
        "gr_rect" => &[
            ("start", Some(2), false),
            ("end", Some(2), false),
            ("stroke", Some(0), false),
            ("fill", Some(1), false),
            ("layer", Some(1), false),
            ("uuid", Some(1), false),
        ],
        "stroke" => &[("width", Some(1), false), ("type", Some(1), false)],
        "polygon" => &[("pts", Some(0), false)],
        "pts" => &[("xy", Some(2), true)],
        _ => return None,
    })
}

/// The children a footprint may hold besides `pad` (judged by `allowed`),
/// and whether each may repeat (a field named twice was refused by the strict
/// reader).
const FOOTPRINT: &[(&str, bool)] = &[
    ("at", false),
    ("layer", false),
    ("uuid", false),
    ("path", false),
    ("sheetname", false),
    ("sheetfile", false),
    ("descr", false),
    ("tags", false),
    ("attr", false),
    ("duplicate_pad_numbers_are_jumpers", false),
    ("embedded_fonts", false),
    ("model", false),
    ("property", true),
    ("fp_line", true),
    ("fp_rect", true),
    ("fp_poly", true),
    ("fp_text", true),
    ("pad", true),
];

/// Top-level items `read` accepts; each but `footprint`, `segment`, `via`
/// and `zone` at most once. Any other item is refused, so no copper or rule
/// reaches KiCad that the bound has not read.
const TOP: &[&str] = &[
    "version",
    "generator",
    "generator_version",
    "general",
    "paper",
    "layers",
    "gr_rect",
    "embedded_fonts",
    "footprint",
    "segment",
    "via",
    "zone",
];

fn unmodelled(what: String) -> Refused {
    refused("UnmodelledCopper", what)
}

/// Every coordinate and size a geometry child gives is within
/// [`MAX_COORD_NM`].
fn bounded(item: &Node<'_>, what: &str) -> Result<(), Refused> {
    if !matches!(item.head(), Some("at" | "start" | "end" | "size" | "xy")) {
        return Ok(());
    }
    // An `at` holds a rotation after its two coordinates: 270 is an angle,
    // not 270 mm (`AET-WO-1309` adversarial F7).
    let n = if item.head() == Some("at") {
        2
    } else {
        usize::MAX
    };
    for a in item.texts().take(n) {
        if sexpr::nm(a).is_some_and(|v| v.abs() > MAX_COORD_NM) {
            return Err(refused(
                "BoardTooLarge",
                format!("{what}: `{a}` mm is beyond {MAX_COORD_NM} nm"),
            ));
        }
    }
    Ok(())
}

/// Each child list's keyword. A list KiCad would not read as a keyword (a
/// quoted or missing head) is refused, not skipped.
fn keyword<'a>(child: &Node<'a>, what: &str) -> Result<&'a str, Refused> {
    child
        .head()
        .ok_or_else(|| unmodelled(format!("{what} holds a list with no keyword")))
}

/// Judges one item's children against `allowed`, recursively.
fn modelled(item: &Node<'_>, what: &str) -> Result<(), Refused> {
    let parent = item.head().unwrap_or("");
    let Some(rules) = allowed(parent) else {
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    for child in item.lists() {
        let head = keyword(&child, what)?;
        let Some(&(_, arity, repeats)) = rules.iter().find(|r| r.0 == head) else {
            return Err(unmodelled(format!(
                "{what} has `{head}`, which the bound does not read"
            )));
        };
        if !repeats && !seen.insert(head) {
            return Err(unmodelled(format!("{what} has `{head}` twice")));
        }
        let args = child.texts().count();
        if arity.is_some_and(|n| n != args) {
            return Err(unmodelled(format!("{what}: `{head}` has {args} values")));
        }
        bounded(&child, what)?;
        modelled(&child, what)?;
    }
    Ok(())
}

/// A footprint's children, its pads' (each with `layers` by canonical name, and a `rect` or
/// `roundrect` shape, the two the bound boxes exactly), and no copper but
/// its pads. Returns its library id and `Value`. A field named twice was
/// refused by the strict reader.
fn footprint(fp: &Node<'_>) -> Result<(String, (String, String)), Refused> {
    let reference = fp.field("Reference").map_err(malformed)?.unwrap_or("?");
    let what = format!("footprint {reference}");
    let mut seen = BTreeSet::new();
    for child in fp.lists() {
        let head = keyword(&child, &what)?;
        let Some(&(_, repeats)) = FOOTPRINT.iter().find(|r| r.0 == head) else {
            return Err(unmodelled(format!(
                "{what} has `{head}`, which the bound does not read"
            )));
        };
        if !repeats && !seen.insert(head) {
            return Err(unmodelled(format!("{what} has `{head}` twice")));
        }
        bounded(&child, &what)?;
        match head {
            "pad" => {
                let pad = format!("{what} pad {}", child.arg(1).unwrap_or("?"));
                let canonical = child
                    .one("layers")
                    .map_err(malformed)?
                    .is_some_and(|l| l.texts().all(canonical_layer));
                if !canonical {
                    return Err(unmodelled(format!(
                        "{pad} has no `layers`, or names a layer by other than its canonical name"
                    )));
                }
                // A drill is a through-hole pad's alone: on an SMD pad its
                // offset would move the copper off the pad's `at`.
                if child.arg(2) != Some("thru_hole")
                    && child.one("drill").map_err(malformed)?.is_some()
                {
                    return Err(unmodelled(format!(
                        "{pad} is drilled and not a through-hole pad"
                    )));
                }
                // KiCad draws a circle as a disk of its x size (`1305` A11),
                // which lies inside the box the bound prices only when the
                // size is square (`AET-WO-1309`).
                let square = child
                    .one("size")
                    .map_err(malformed)?
                    .is_some_and(|z| z.arg(1).is_some() && z.arg(1) == z.arg(2));
                let circle = child.arg(3) == Some("circle") && square;
                if !circle && !matches!(child.arg(3), Some("rect" | "roundrect")) {
                    return Err(unmodelled(format!("{pad} has shape {:?}", child.arg(3))));
                }
                modelled(&child, &pad)?;
            }
            // A footprint's own `layer` names its side.
            "layer" => {}
            _ if on_copper(&child) => {
                return Err(unmodelled(format!("{what} has `{head}` on a copper layer")));
            }
            _ => {}
        }
    }
    let lib = fp.arg(1).unwrap_or("").to_owned();
    let value = fp
        .field("Value")
        .map_err(malformed)?
        .unwrap_or("")
        .to_owned();
    Ok((reference.to_owned(), (lib, value)))
}

/// A board as `read` accepted it: the board and copper as `route` reads
/// them, and each footprint's library id and `Value`, by reference.
pub struct ReadBoard {
    pub board: Board,
    pub routed: Routed,
    pub footprints: BTreeMap<String, (String, String)>,
}

/// Reads a `.kicad_pcb` text and refuses, before any estimate, a board the
/// bound does not hold for or could misread: unparseable; copper layers,
/// zones or a thickness other than the specification declares
/// ([`route::declared`]: `LayersDisagree`, `PlanesDisagree`,
/// `ThicknessDisagrees`); what the strict reader refuses (`MalformedBoard`); an item,
/// child, headless list, repeated child or value count the bound does not
/// model; a coordinate beyond [`MAX_COORD_NM`]; copper outside pads,
/// tracks, vias and the zone; two footprints with one reference; net
/// labels that disagree with the copper's connections (`route::connectivity`);
/// or copper closer than the declared clearance to another net's
/// (`route::check`).
pub fn read(text: &str, stack: &Stack) -> Result<ReadBoard, Refused> {
    read_tree(&strict::board(text).map_err(malformed)?, stack)
}

/// [`read`] on a board the strict reader has already read: `check drc
/// --spec` runs it on its own strict reading before it judges a declared
/// pair, so the pair judge sees only copper this allowlist models
/// (`AET-WO-3303` §1.3).
pub fn read_tree(tree: &strict::Tree, stack: &Stack) -> Result<ReadBoard, Refused> {
    strict::version(tree).map_err(malformed)?;
    let board = tree.root();
    let mut once = BTreeSet::new();
    let mut footprints = BTreeMap::new();
    for item in board.lists() {
        let head = keyword(&item, "the board")?;
        if !TOP.contains(&head) {
            return Err(unmodelled(format!(
                "a top-level `{head}`, which the bound does not read"
            )));
        }
        if !matches!(head, "footprint" | "segment" | "via" | "zone") && !once.insert(head) {
            return Err(unmodelled(format!("a second top-level `{head}`")));
        }
        match head {
            "footprint" => {
                let (reference, identity) = footprint(&item)?;
                if footprints.insert(reference.clone(), identity).is_some() {
                    return Err(refused(
                        "UnmodelledPart",
                        format!("two footprints are {reference}"),
                    ));
                }
            }
            "gr_rect" | "segment" | "via" | "zone" | "general" => {
                for child in item.lists() {
                    bounded(&child, head)?;
                }
                modelled(&item, head)?;
            }
            _ => {}
        }
    }
    route::declared(tree, stack).map_err(|d| refused(d.kind, d.message))?;
    let (board, routed) = route::read(tree, stack).map_err(|e| refused("BoardUnreadable", e))?;
    route::connectivity(&board, &routed).map_err(|e| refused("NetLabelsDisagree", e))?;
    route::check(&board, &routed).map_err(|e| refused("ClearanceViolated", e))?;
    Ok(ReadBoard {
        board,
        routed,
        footprints,
    })
}

/// The board's name for a specification label (`pcb`'s own mapping).
pub fn board_net(label: &str) -> String {
    format!("/{}", escape(label))
}

/// What one part on one line is, from the specification's own design.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Role {
    footprint: String,
    value: String,
    /// The part's pads on the line, by number.
    pads: BTreeSet<String>,
    load: Load,
    /// A connector's declared off-board capacitance for each of its pads on
    /// the line, as `(pad, femtofarads, source)`; empty for any other part.
    offboard: Vec<(String, u64, String)>,
}

/// How a part's pads load a line, beyond their copper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Load {
    /// A pin, counted at this many femtofarads per pad.
    Pin(u64),
    /// The bus's pull-up: copper only.
    PullUp,
    /// The driver of an SPI line, the MCU: copper only.
    Driver,
    /// An off-board connector (`AET-WO-4300f` D8): copper, no `Ci`, and the
    /// off-board capacitance its pins declare (`Role::offboard`).
    Connector,
}

/// A connector's declared off-board capacitance for each of `pads`, the pads
/// it has on `net`.
fn offboard_of(
    part: &crate::schematic::Part,
    pads: &BTreeSet<String>,
    net: &str,
) -> Result<Vec<(String, u64, String)>, Refused> {
    // belt-and-suspenders: `validate` refuses a connector pin on an I2C or
    // SPI line without a budget (`ConnectorBudgetMissing`), so no test
    // reaches the refusal below.
    pads.iter()
        .map(|p| {
            let (ff, source) = part.offboard.get(p).cloned().ok_or_else(|| {
                refused(
                    "UnmodelledPart",
                    format!(
                        "{}.{p} is on `{net}` with no declared off-board capacitance",
                        part.reference
                    ),
                )
            })?;
            Ok((p.clone(), ff, source))
        })
        .collect()
}

/// A device's pin on an SPI line the MCU drives: its part's cited maximum,
/// or a refusal by name when the table cites none (`AET-WO-4300c` D3). No
/// drawable part lacks one today (the ICM-20948 has no vendored symbol), so
/// only its unit test reaches the refusal.
fn device_pin(
    table: &aethelgard_ir::parts::PartsTable,
    reference: &str,
    part: &str,
    net: &str,
) -> Result<Load, Refused> {
    match table.get(part).and_then(|p| p.pin_capacitance_femtofarads) {
        Some(ff) => Ok(Load::Pin(ff)),
        None => Err(refused(
            "UnboundedPinCapacitance",
            format!(
                "the design puts {reference} ({part}) on `{net}`, a line the MCU drives, and the parts table cites no maximum for its pin capacitance"
            ),
        )),
    }
}

/// One line and every part the specification puts on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    net: String,
    bus: String,
    spi_signal: Option<&'static str>,
    parts: BTreeMap<String, Role>,
}

/// The pads of `part` the design puts on board net `on`.
fn pads_on(part: &crate::schematic::Part, on: &str) -> Result<BTreeSet<String>, Refused> {
    let pin_nets = crate::pcb::pin_nets(part).map_err(|e| invalid(&e))?;
    Ok(pin_nets
        .into_iter()
        .filter(|(_, n)| n == on)
        .map(|(p, _)| p)
        .collect())
}

/// Each I2C line of `spec`, bus by bus in document order, SDA then SCL,
/// then each SPI bus's SCK and MOSI (`AET-WO-4300c` D2), with the parts the
/// generator's design puts on it (`schematic::design`, from the
/// specification and its solved topology: `RUL-16`).
///
/// - **On an I2C line** a part is a pin part when it is not a passive (the
///   MCU and the declared devices), each pin at UM10204's `Ci` (unchanged),
///   and a pull-up when it is the resistor between the line and the bus's
///   rail.
/// - **On an SPI line** the MCU drives the edge, so its pin is not counted
///   (Table 149's C is the load it drives); each declared device's pin is
///   counted at its part's cited maximum (`parts.json` `pin_capacitance`,
///   `AET-WO-4300c` D3). A device whose part cites no maximum is refused by
///   name: no typical value stands in for one.
///
/// A passive that is not a pull-up is refused: its capacitance is not known.
pub fn lines(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<Vec<Line>, Refused> {
    let (parts, _) = crate::schematic::design(spec, solved, table).map_err(|e| invalid(&e))?;
    let mut out = Vec::new();
    for bus in &spec.i2c_buses {
        for net in [&bus.sda.net, &bus.scl.net] {
            let on = board_net(net);
            let mut roles = BTreeMap::new();
            for part in &parts {
                let pads = pads_on(part, &on)?;
                if pads.is_empty() {
                    continue;
                }
                let passive =
                    part.lib_id == crate::schematic::R || part.lib_id == crate::schematic::C;
                let connector = crate::schematic::is_connector(part);
                let load = if connector {
                    Load::Connector
                } else if passive {
                    Load::PullUp
                } else {
                    Load::Pin(PIN_FF)
                };
                let pull_up = part.lib_id == crate::schematic::R
                    && part
                        .conns
                        .values()
                        .filter(|c| **c == Conn::Net(net.clone()))
                        .count()
                        == 1
                    && part
                        .conns
                        .values()
                        .filter(|c| **c == Conn::Rail(bus.domain.clone()))
                        .count()
                        == 1
                    && part.conns.len() == 2;
                // belt-and-suspenders: today's generator puts no passive but
                // the pull-up on an I2C line; a later one that does (a series
                // resistor, a capacitor) must not have it counted as copper.
                if passive && !pull_up {
                    return Err(refused(
                        "UnmodelledPart",
                        format!(
                            "the design puts {} on `{net}`, and its capacitance is not known",
                            part.reference
                        ),
                    ));
                }
                let offboard = if connector {
                    offboard_of(part, &pads, net)?
                } else {
                    Vec::new()
                };
                roles.insert(
                    part.reference.clone(),
                    Role {
                        footprint: part.footprint.clone(),
                        value: part.value.clone(),
                        pads,
                        load,
                        offboard,
                    },
                );
            }
            out.push(Line {
                net: net.clone(),
                bus: bus.id.clone(),
                spi_signal: None,
                parts: roles,
            });
        }
    }
    let table = aethelgard_ir::parts::table().map_err(|e| refused(e.kind(), e.to_string()))?;
    for bus in &spec.spi_buses {
        for (signal, net) in [("SCK", &bus.sck.net), ("MOSI", &bus.mosi.net)] {
            let on = board_net(net);
            let mut roles = BTreeMap::new();
            for part in &parts {
                let pads = pads_on(part, &on)?;
                if pads.is_empty() {
                    continue;
                }
                // The solved topology's copy of the device (`RUL-16`).
                let device = solved
                    .spi_devices
                    .get(&part.reference)
                    .filter(|d| d.bus == bus.id);
                let load = if part.lib_id == crate::schematic::MCU {
                    Load::Driver
                } else if let Some(d) = device {
                    device_pin(table, &part.reference, &d.part, net)?
                } else if crate::schematic::is_connector(part) {
                    Load::Connector
                } else {
                    // belt-and-suspenders: today's generator puts only the
                    // MCU and the bus's devices on SCK and MOSI.
                    return Err(refused(
                        "UnmodelledPart",
                        format!(
                            "the design puts {} on `{net}`, and its capacitance is not known",
                            part.reference
                        ),
                    ));
                };
                let offboard = if load == Load::Connector {
                    offboard_of(part, &pads, net)?
                } else {
                    Vec::new()
                };
                roles.insert(
                    part.reference.clone(),
                    Role {
                        footprint: part.footprint.clone(),
                        value: part.value.clone(),
                        pads,
                        load,
                        offboard,
                    },
                );
            }
            out.push(Line {
                net: net.clone(),
                bus: bus.id.clone(),
                spi_signal: Some(signal),
                parts: roles,
            });
        }
    }
    Ok(out)
}

/// Every line of `lines`, bounded on a board `read` accepted. The parts on
/// each line must be exactly the design's: each by reference, footprint and
/// `Value`, each with exactly the design's pads on the line. Every pad's
/// copper is counted, pins included: `Ci` is the part's, not the land's.
pub fn bound(read: &ReadBoard, lines: &[Line]) -> Result<Vec<LineBound>, Refused> {
    lines.iter().map(|l| line(read, l)).collect()
}

fn overflow(net: &str) -> Refused {
    refused(
        "Overflow",
        format!("`{net}`: the bound does not fit in integer arithmetic"),
    )
}

/// [`bound`], with every piece taken through the cap path at `min(h, S)`.
/// At a dielectric of at least `S` it must give [`bound`]'s integers, since
/// the cap adds exactly 0 at `h = S` (ruled D5: the old and new paths agree on
/// the reference). For that test only.
#[doc(hidden)]
pub fn bound_by_cap_path(read: &ReadBoard, lines: &[Line]) -> Result<Vec<LineBound>, Refused> {
    lines.iter().map(|l| line_with(read, l, true)).collect()
}

fn line(read: &ReadBoard, want: &Line) -> Result<LineBound, Refused> {
    line_with(read, want, false)
}

fn line_with(read: &ReadBoard, want: &Line, force_cap: bool) -> Result<LineBound, Refused> {
    let (board, routed, net) = (&read.board, &read.routed, want.net.as_str());
    let board_net = board_net(net);
    let ph = Physics {
        force_cap,
        ..Physics::of(&board.stack)
    }
    .laid_by(board.stack.rule(&board_net));
    let mut on_board: BTreeMap<&str, Vec<&route::Pad>> = BTreeMap::new();
    for part in &board.parts {
        let here: Vec<&route::Pad> = part
            .pads
            .iter()
            .filter(|p| p.net.as_deref() == Some(board_net.as_str()))
            .collect();
        if !here.is_empty() {
            on_board.insert(&part.reference, here);
        }
    }
    if on_board.is_empty() {
        return Err(refused(
            "LineNotOnBoard",
            format!("`{net}` ({board_net}) has no pad on the board"),
        ));
    }
    for reference in want.parts.keys() {
        if !on_board.contains_key(reference.as_str()) {
            return Err(refused(
                "DeviceNotOnLine",
                format!(
                    "the specification puts {reference} on `{net}`, but the board has none of its pads there"
                ),
            ));
        }
    }
    let mut terms = Terms::default();
    let (mut pins, mut pull_ups, mut pad_count) = (Vec::new(), Vec::new(), 0u64);
    let (mut drivers, mut offboard) = (Vec::new(), Vec::new());
    for (reference, pads) in &on_board {
        let Some(role) = want.parts.get(*reference) else {
            return Err(refused(
                "UnmodelledPart",
                format!(
                    "{reference} has a pad on `{net}`, but the specification puts no {reference} there"
                ),
            ));
        };
        let identity = read.footprints.get(*reference);
        let numbers: BTreeSet<String> = pads.iter().map(|p| p.number.clone()).collect();
        if identity != Some(&(role.footprint.clone(), role.value.clone()))
            || numbers != role.pads
            || numbers.len() != pads.len()
        {
            return Err(refused(
                "PartMismatch",
                format!(
                    "{reference} on `{net}` is {identity:?} with pads {numbers:?}; the specification's is ({:?}, {:?}) with pads {:?}",
                    role.footprint, role.value, role.pads
                ),
            ));
        }
        if role.load == Load::PullUp {
            pull_ups.push((*reference).to_owned());
        }
        for (pad, ff, source) in &role.offboard {
            terms.offboard = terms
                .offboard
                .checked_add(*ff)
                .ok_or_else(|| overflow(net))?;
            offboard.push(format!("{reference}.{pad}: {ff} fF ({source})"));
        }
        for pad in pads {
            let ff = pad_bound(&ph, pad).ok_or_else(|| overflow(net))?;
            terms.pads = terms.pads.checked_add(ff).ok_or_else(|| overflow(net))?;
            pad_count += 1;
            match role.load {
                Load::Pin(ff) => {
                    pins.push(format!("{reference}.{}", pad.number));
                    terms.pins = terms.pins.checked_add(ff).ok_or_else(|| overflow(net))?;
                }
                Load::Driver => drivers.push(format!("{reference}.{}", pad.number)),
                Load::PullUp | Load::Connector => {}
            }
        }
    }

    // Per layer, each track's length times its layer's per-length bound,
    // summed, then rounded up once: at 2 layers both layers' bounds are the
    // same, so this is today's `⌈length × per-length⌉`.
    let mut length: u128 = 0;
    let mut weighted: u128 = 0;
    let mut segments = 0u64;
    let mut ends: u64 = 0;
    for s in routed.segments.iter().filter(|s| s.net == board_net) {
        let d = |a: i64, b: i64| u128::from(a.abs_diff(b));
        let here = d(s.start.0, s.end.0) + d(s.start.1, s.end.1);
        let h = ph.h(s.layer);
        let per = per_mm_ff(&ph, h).ok_or_else(|| overflow(net))?;
        let end = segment_ff(&ph, h).ok_or_else(|| overflow(net))?;
        length = length.checked_add(here).ok_or_else(|| overflow(net))?;
        weighted = here
            .checked_mul(u128::from(per))
            .and_then(|x| weighted.checked_add(x))
            .ok_or_else(|| overflow(net))?;
        ends = ends.checked_add(end).ok_or_else(|| overflow(net))?;
        segments += 1;
    }
    let per_length = weighted.div_ceil(1_000_000);
    terms.trace = u64::try_from(per_length)
        .ok()
        .and_then(|t| t.checked_add(ends))
        .ok_or_else(|| overflow(net))?;
    let via_count = routed.vias.iter().filter(|v| v.net == board_net).count() as u64;
    let via = via_ff(&ph).ok_or_else(|| overflow(net))?;
    terms.vias = via_count.checked_mul(via).ok_or_else(|| overflow(net))?;
    // A thermal array's vias, each at its own size (`AET-WO-1309`).
    for a in routed.arrays.iter().filter(|a| a.net == board_net) {
        let own = Physics {
            via: u128::try_from(a.via_nm).map_err(|_| overflow(net))?,
            ..ph
        };
        let each = via_ff(&own).ok_or_else(|| overflow(net))?;
        terms.vias = (a.at.len() as u64)
            .checked_mul(each)
            .and_then(|v| terms.vias.checked_add(v))
            .ok_or_else(|| overflow(net))?;
    }
    let total = [
        terms.trace,
        terms.vias,
        terms.pads,
        terms.pins,
        terms.offboard,
    ]
    .into_iter()
    .try_fold(0u64, u64::checked_add)
    .ok_or_else(|| overflow(net))?;
    Ok(LineBound {
        net: net.to_owned(),
        board_net,
        bus: want.bus.clone(),
        c_bound_femtofarads: total,
        terms,
        length_nm: u64::try_from(length).map_err(|_| overflow(net))?,
        segments,
        via_count,
        pad_count,
        pins,
        spi_signal: want.spi_signal,
        drivers,
        pull_ups,
        offboard,
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

    fn reference() -> Physics {
        Physics::of(&crate::stack::reference())
    }

    fn ff(volume: f64) -> f64 {
        let s = 200_000.0;
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

    /// §1.4: each derived value against its formula on the reference's
    /// declared board, and against the values the §1.3 measurement used.
    #[test]
    fn each_constant_is_its_formula_rounded_up() {
        let ph = reference();
        let h = ph.h_top;
        let (s, c, w) = (200_000.0, 70_000.0, 200_000.0);
        let per_nm = 2.0 * c * s + 2.0 * w * s + PI * s * s;
        let per_mm = per_mm_ff(&ph, h).unwrap();
        pinned("per-length", per_mm, ff(per_nm * 1e6));
        let r = w / 2.0;
        let segment = segment_ff(&ph, h).unwrap();
        pinned(
            "segment",
            segment,
            ff(shell_f(PI * r * r, 2.0 * PI * r, c, s)),
        );
        let r = VIA_NM as f64 / 2.0;
        let via = via_ff(&ph).unwrap();
        pinned(
            "via",
            via,
            ff(shell_f(PI * r * r, 2.0 * PI * r, 2_000_000.0, s)),
        );
        for (pw, phh) in [
            (300_000, 1_475_000),
            (540_000, 640_000),
            (1, 1),
            (5_000_000, 3_000_000),
        ] {
            let rect = crate::pcb::Rect {
                x0: 7,
                y0: -3,
                x1: 7 + pw,
                y1: -3 + phh,
            };
            let (a, p) = ((pw * phh) as f64, 2.0 * (pw + phh) as f64);
            pinned("pad", pad_ff(&ph, rect).unwrap(), ff(shell_f(a, p, c, s)));
        }
        assert_eq!(
            (per_mm, segment, via, PIN_FF),
            (280, 124, 1519, 10_000),
            "a value moved: re-measure §1.3 before changing this line"
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

    /// `K(k')` from `k` itself, `π / (2 AGM(1, k))`, so a tiny `k` does not
    /// lose `k'` to rounding.
    fn k_prime(k: f64) -> f64 {
        PI / (2.0 * agm(1.0, k))
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
        let ph = reference();
        let (w, s, height) = (200_000.0, 200_000.0, 2_000_000.0);
        let kk = w / (w + 2.0 * s);
        let cpw_per_mm = 4.0 * EPS * k(kk) / k((1.0 - kk * kk).sqrt()) * 1e6;
        assert!(
            per_mm_ff(&ph, ph.h_top).unwrap() as f64 >= cpw_per_mm,
            "{cpw_per_mm}"
        );
        let r = VIA_NM as f64 / 2.0;
        let coax = 2.0 * PI * EPS / ((r + s) / r).ln() * height;
        assert!(via_ff(&ph).unwrap() as f64 >= coax, "{coax}");
        // The side of the via's shell alone already dominates the coax.
        let side = ff((2.0 * PI * r * s + PI * s * s) * height);
        assert!(side >= coax, "{side} {coax}");
    }

    /// `lower(A, p, h)` of the module documentation, per `ε`.
    fn lower(a: f64, p: f64, h: f64, s: f64) -> f64 {
        a / h
            + PI * p * h / (8.0 * s)
            + PI * p * s / (8.0 * h)
            + 4.0 * PI * h / 9.0
            + 2.0 * PI * s * s / (9.0 * h)
    }

    /// The identity the cap rests on (ruled D5): at `h = S` the cap adds
    /// exactly nothing, for any base, so the old and new paths give equal
    /// integers; and `lower(S)` is today's bottom end cap term by term.
    #[test]
    fn at_h_equal_s_the_cap_adds_nothing() {
        for (w, l) in [
            (1, 1),
            (200_000, 200_000),
            (540_000, 640_000),
            (5_000_000, 3_000_000),
        ] {
            let base = Base::boxed(w, l).unwrap();
            for ends in [false, true] {
                assert_eq!(
                    cap(base.area, base.perimeter, ends, 200_000, 200_000),
                    Some(0)
                );
            }
            let (a, p, s) = ((w * l) as f64, 2.0 * (w + l) as f64, 200_000.0);
            let today = a / s + PI * p / 4.0 + 2.0 * PI * s / 3.0;
            assert!((lower(a, p, s, s) - today).abs() < 1e-9 * today);
        }
        let ph = reference();
        let mut at_s = ph;
        at_s.h_top = 199_999;
        // Just under `S` the cap path runs, and differs from today's only by
        // the cap's own few femtofarads per millimetre.
        let under = per_mm_ff(&at_s, at_s.h_top).unwrap();
        assert!((280..=281).contains(&under), "{under}");
    }

    /// The integer cap against its real formula: `D · S² · (lower(h) −
    /// lower(S))`, rounded up, for bases and depths across the range.
    #[test]
    fn the_integer_cap_is_its_formula_rounded_up() {
        let d = D as f64;
        for (w, l) in [
            (200_000, 1_000_000),
            (540_000, 640_000),
            (300_000, 1_475_000),
        ] {
            let base = Base::boxed(w, l).unwrap();
            let (a, p, s) = ((w * l) as f64, 2.0 * (w + l) as f64, 200_000.0);
            for h in [50_000u128, 99_000, 179_000, 199_000] {
                let hf = h as f64;
                let want = d * s * s * (lower(a, p, hf, s) - lower(a, p, s, s));
                let got = cap(base.area, base.perimeter, true, h, 200_000).unwrap() as f64;
                assert!(
                    got >= want * (1.0 - 1e-6) && got <= want * (1.0 + 1e-3) + 1.0,
                    "{h}: {got} {want}"
                );
            }
        }
    }

    /// The exact case for the cap (`AET-WO-4100` S4): a zero-thickness strip
    /// midway between two planes `2h` apart, in `ε` everywhere, is exactly
    /// `4ε K(k')/K(k)`, `k = sech(πW/4h)` (Cohn). The trial with a cap on both
    /// sides bounds it from above at every depth; the modulus is checked by
    /// the wide strip's limit `2W/h + 8 ln 2 / π`.
    #[test]
    fn the_capped_trial_dominates_the_exact_stripline() {
        let exact = |w: f64, h: f64| {
            let x = PI * w / (4.0 * h);
            let kk = 2.0 * (-x).exp() / (1.0 + (-2.0 * x).exp());
            4.0 * k_prime(kk) / k(kk)
        };
        let wide = exact(4.0, 0.1) - 2.0 * 4.0 / 0.1;
        assert!((wide - 8.0 * 2f64.ln() / PI).abs() < 1e-6, "{wide}");
        let s = 0.2;
        for w in [0.1, 0.2, 0.5, 1.0] {
            for h in [0.05, 0.075, 0.0994, 0.1, 0.15, 0.179, 0.2] {
                // Per length: dA/dL = W and dp/dL = 2 in `lower`, both sides.
                let trial = 2.0 * (w / h + PI * 2.0 * h / (8.0 * s) + PI * 2.0 * s / (8.0 * h));
                assert!(
                    trial >= exact(w, h),
                    "W {w} h {h}: {trial} < {}",
                    exact(w, h)
                );
            }
        }
    }

    /// The cap's closed form against a direct quadrature of the same `ψ`
    /// over a track's cross-section below its bottom face (`z < 0`), per
    /// length: `∫∫ |∇ρ|²` over `(d_B/S)² + (z/h)² < 1`.
    #[test]
    fn the_cap_is_its_quadrature() {
        let (w, s) = (0.2f64, 0.2f64);
        for h in [0.075, 0.1, 0.179] {
            let n = 2000;
            let (gx, gz) = ((w / 2.0 + s) / n as f64, h / n as f64);
            let psi = |x: f64, z: f64| {
                let db = (x.abs() - w / 2.0).max(0.0);
                (1.0 - ((db / s).powi(2) + (z / h).powi(2)).sqrt()).max(0.0)
            };
            let mut e = 0.0;
            for i in 0..n {
                for j in 0..n {
                    let (x, z) = (i as f64 * gx, -(j as f64) * gz);
                    let dx = (psi(x + gx, z) - psi(x, z)) / gx;
                    let dz = (psi(x, z - gz) - psi(x, z)) / gz;
                    e += (dx * dx + dz * dz) * gx * gz;
                }
            }
            let quad = 2.0 * e;
            let closed = w / h + PI * h / (4.0 * s) + PI * s / (4.0 * h);
            assert!(
                (quad - closed).abs() < 2e-3 * closed,
                "h {h}: {quad} vs {closed}"
            );
        }
    }

    /// Adversarial A2: a permittivity whose thousandths do not reduce
    /// (4.663) once overflowed the exact product on a 3.3 mm pad, giving
    /// `None`. The staged division bounds it from above instead.
    #[test]
    fn an_irreducible_permittivity_still_bounds_a_large_pad() {
        let mut stack = crate::stack::reference();
        stack.permittivity_permille = 4663;
        stack.dielectrics[0].2 = 179_000;
        let ph = Physics::of(&stack);
        for side in [3_300_000i64, 10_000_000, 40_000_000] {
            let rect = crate::pcb::Rect {
                x0: 0,
                y0: 0,
                x1: side,
                y1: side,
            };
            let got = pad_ff(&ph, rect).unwrap_or_else(|| panic!("{side}: None"));
            let (a, p, s, c, h) = (
                (side * side) as f64,
                4.0 * side as f64,
                200_000.0,
                70_000.0,
                179_000.0,
            );
            let exact = 8.854_187_818_8e-6
                * 4.663
                * (shell_f(a, p, c, s) / (s * s) - lower(a, p, s, s) + lower(a, p, h, s));
            assert!(got as f64 >= exact, "{side}: {got} < {exact}");
        }
    }

    #[test]
    fn a_pad_too_large_to_compute_is_none_not_a_wrap() {
        let ph = reference();
        let huge = crate::pcb::Rect {
            x0: i64::MIN,
            y0: 0,
            x1: i64::MAX,
            y1: 1,
        };
        assert_eq!(pad_ff(&ph, huge), None);
        let inverted = crate::pcb::Rect {
            x0: 5,
            y0: 0,
            x1: 0,
            y1: 1,
        };
        assert_eq!(pad_ff(&ph, inverted), None);
    }

    /// `AET-WO-4300c` D3: a device pin on SCK or MOSI counts at its part's
    /// cited maximum; a part with only a typical value (the ICM-20948's
    /// "< 10" pF) is refused by name, never given that value.
    #[test]
    fn a_device_pin_is_its_cited_maximum_or_refused() {
        let table = aethelgard_ir::parts::table().unwrap();
        assert_eq!(
            device_pin(table, "U4", "W25Q64JVxIQ", "SPI_SCK"),
            Ok(Load::Pin(8_000))
        );
        let err = device_pin(table, "U5", "ICM20948", "SPI_SCK").unwrap_err();
        assert_eq!(err.kind, "UnboundedPinCapacitance");
        assert!(
            err.message.contains("U5 (ICM20948) on `SPI_SCK`"),
            "{}",
            err.message
        );
        assert_eq!(
            device_pin(table, "U9", "UNLISTED", "SPI_SCK")
                .unwrap_err()
                .kind,
            "UnboundedPinCapacitance"
        );
    }
}
