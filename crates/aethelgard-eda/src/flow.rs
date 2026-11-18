//! A certified upper bound on the resistance between two terminals of a
//! copper network (`AET-WO-1303b`, ruling D5 (b) on PR #222).
//!
//! **The principle (Thomson).** For any current density `J` confined to the
//! conductor, divergence-free, carrying unit current from one terminal to
//! the other, `R ≤ ∫ ρ |J|² dV`. Deleting copper never lowers `R`
//! (Rayleigh), so a flow on any subset of the copper bounds the whole. The
//! bound is exact for the flow it uses: it is not an approximation, however
//! poor the flow.
//!
//! **The flow.** The copper is cut into elements, each carrying a field of
//! a fixed form, determined by the fluxes through its faces:
//! - a rectangle of sheet copper (a cell, `a × b`, thickness `t`) carries
//!   the lowest-order Raviart–Thomas (RT0) field `J = (α + βx, γ + δy)/t`.
//!   Its normal component on each face is constant and equals that face's
//!   flux over its length, so neighbouring cells match pointwise. Its
//!   divergence is the cell's net outflow over its area, uniform. Its
//!   energy is exactly
//!   `(ρ/t)·[(a/b)(F_W² + F_W·F_E + F_E²) + (b/a)(F_S² + F_S·F_N + F_N²)]/3`
//!   ([`Network::energy`]). **Every cell that carries flux lies wholly
//!   inside copper** (the builder's inner approximation, [`crate::supply`]);
//!   a face on a copper boundary or a hole has no edge and carries 0;
//! - a barrel cell ([`Element::Tall`], [`barrel`]) is the same field on the
//!   via's plating unrolled, its height known only within a range;
//! - a joint carrying `κ·F²` for the flux `F` of one edge: a grid face of a
//!   via's junction square into its barrel column ([`junction`], whose
//!   three regions' fields are written out there and whose energy is
//!   evaluated in closed form).
//!
//! A terminal (a pad taken as an equipotential) is a node with no element.
//! [`Network::check`] refuses, before any flux, a network whose elements do
//! not price every edge exactly once per end, or that joins two terminals
//! directly.
//!
//! **Exactness.** A potential solve proposes the face fluxes (two-point
//! conductances, conjugate gradients preconditioned by aggregation
//! multigrid, capped in iterations: nothing it computes enters the bound
//! except as a proposal). They are then made **exactly** divergence-free:
//! each is rounded to an integer number of quanta (`2⁻⁴⁰` of the unit
//! current), and the rounding residue is carried to the sink terminal
//! along a breadth-first spanning tree, so every node's net outflow is
//! exactly its target in integers ([`Network::bound`] re-checks it in
//! `i128`). Each element's field is then exactly divergence-free in the
//! reals (a cell with a junction's injection has the uniform divergence
//! that the plating's radial field cancels, [`junction`]), whatever the
//! solve's error: a poor solve gives a loose bound, never a wrong one.
//!
//! **Rounding.** The energy of that exact field is evaluated in interval
//! arithmetic ([`Iv`]): every operation's result is widened outward by one
//! unit in the last place, so the interval holds the exact real value; a
//! flux in quanta over `2⁴⁰` is exact in binary. Inputs that are not exact
//! in binary (ρ, a drill's radius, thicknesses) enter as intervals that hold
//! them; `ln` and `atan` are series with their remainders added as
//! intervals ([`Iv::ln`], [`Iv::atan`]); [`junction`]'s integrals are
//! enclosed panel by panel. A range (a barrel row's height) is taken at its
//! worse end. The bound is the upper end of the total's interval: **nothing
//! is rounded down**.
//!
//! **Checked against exact cases** (the tests): a bar and a barrel, at
//! their exact value; an annulus, a bounded two-hole sheet (with a
//! refinement series) and a 45° track, above it and converging; a 90° bend,
//! above `1303`'s certified bracket; a junction coefficient, against a
//! direct quadrature of its own field; a via in a disc, above its shorted
//! floor.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// A closed interval of reals, `[lo, hi]`, with every operation rounded
/// outward: the exact result of the operation on any reals inside the
/// operands lies inside the result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}

// The operations are named methods on purpose: each rounds outward, and
// an operator would read as exact arithmetic.
#[allow(clippy::should_implement_trait)]
impl Iv {
    /// An exact binary value: the interval `[x, x]`.
    pub fn exact(x: f64) -> Iv {
        Iv { lo: x, hi: x }
    }

    /// The interval that holds the decimal `num / den`, both exact integers
    /// below 2⁵³.
    pub fn ratio(num: i64, den: i64) -> Iv {
        Iv::exact(num as f64).div(Iv::exact(den as f64))
    }

    pub fn add(self, o: Iv) -> Iv {
        Iv {
            lo: (self.lo + o.lo).next_down(),
            hi: (self.hi + o.hi).next_up(),
        }
    }

    pub fn sub(self, o: Iv) -> Iv {
        Iv {
            lo: (self.lo - o.hi).next_down(),
            hi: (self.hi - o.lo).next_up(),
        }
    }

    pub fn mul(self, o: Iv) -> Iv {
        let p = [
            self.lo * o.lo,
            self.lo * o.hi,
            self.hi * o.lo,
            self.hi * o.hi,
        ];
        let lo = p.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = p.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Iv {
            lo: lo.next_down(),
            hi: hi.next_up(),
        }
    }

    /// Division by an interval that does not contain 0.
    pub fn div(self, o: Iv) -> Iv {
        assert!(
            o.lo > 0.0 || o.hi < 0.0,
            "division by an interval holding 0"
        );
        let p = [
            self.lo / o.lo,
            self.lo / o.hi,
            self.hi / o.lo,
            self.hi / o.hi,
        ];
        let lo = p.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = p.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Iv {
            lo: lo.next_down(),
            hi: hi.next_up(),
        }
    }

    pub fn scale(self, k: f64) -> Iv {
        self.mul(Iv::exact(k))
    }

    /// The square, which is never negative.
    pub fn sqr(self) -> Iv {
        let lo2 = self.lo * self.lo;
        let hi2 = self.hi * self.hi;
        if self.lo >= 0.0 {
            Iv {
                lo: lo2.next_down().max(0.0),
                hi: hi2.next_up(),
            }
        } else if self.hi <= 0.0 {
            Iv {
                lo: hi2.next_down().max(0.0),
                hi: lo2.next_up(),
            }
        } else {
            Iv {
                lo: 0.0,
                hi: lo2.max(hi2).next_up(),
            }
        }
    }

    /// The square root of a non-negative interval. IEEE 754 rounds `sqrt`
    /// correctly, so one unit outward holds the exact value.
    pub fn sqrt(self) -> Iv {
        assert!(self.lo >= 0.0, "sqrt of a negative interval");
        Iv {
            lo: self.lo.sqrt().next_down().max(0.0),
            hi: self.hi.sqrt().next_up(),
        }
    }

    /// The smallest interval holding both.
    pub fn hull(self, o: Iv) -> Iv {
        Iv {
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
        }
    }

    /// `max(lo_bound, min(hi_bound, x))` for every `x` in the interval.
    pub fn clamp(self, lo: f64, hi: f64) -> Iv {
        Iv {
            lo: self.lo.clamp(lo, hi),
            hi: self.hi.clamp(lo, hi),
        }
    }

    /// `ln x` for `x > 0`, without the platform's `ln` (whose rounding std
    /// does not promise): `x = m·2^k` with `m` in [1, 2), then
    /// `ln m = 2 atanh w`, `w = (m − 1)/(m + 1)` in [0, 1/3), as the series
    /// `2 Σ w^(2i+1)/(2i+1)` to 30 terms, whose remainder is at most
    /// `2 w^61 / (61 (1 − w²)) < 1e−30`, added as an interval; and
    /// `ln 2` the same way at `m = 2`. `ln` is increasing, so the ends are
    /// enough.
    pub fn ln(self) -> Iv {
        assert!(self.lo > 0.0, "ln of a non-positive interval");
        Iv {
            lo: ln_point(self.lo).lo,
            hi: ln_point(self.hi).hi,
        }
    }

    /// `atan x`, without the platform's `atan`: two halvings
    /// `atan x = 2 atan(x / (1 + √(1 + x²)))` bring `|x|` under 0.2, then the
    /// alternating series to 40 terms, whose remainder is at most the next
    /// term, added as an interval. Increasing, so the ends are enough.
    pub fn atan(self) -> Iv {
        Iv {
            lo: atan_point(self.lo).lo,
            hi: atan_point(self.hi).hi,
        }
    }
}

fn ln_point(x: f64) -> Iv {
    // x = m 2^k with m in [1, 2): exact in binary
    let mut k = 0i32;
    let mut m = x;
    while m >= 2.0 {
        m /= 2.0;
        k += 1;
    }
    while m < 1.0 {
        m *= 2.0;
        k -= 1;
    }
    let ln_m = atanh_series(Iv::exact(m));
    let ln2 = atanh_series(Iv::exact(2.0));
    ln_m.add(ln2.mul(Iv::exact(k as f64)))
}

/// `ln m = 2 atanh((m − 1)/(m + 1))` for `m` in [1, 2].
fn atanh_series(m: Iv) -> Iv {
    let w = m.sub(Iv::exact(1.0)).div(m.add(Iv::exact(1.0)));
    let w2 = w.sqr();
    let mut term = w;
    let mut sum = Iv::exact(0.0);
    for i in 0..30 {
        sum = sum.add(term.div(Iv::exact((2 * i + 1) as f64)));
        term = term.mul(w2);
    }
    // remainder: Σ_{i ≥ 30} w^(2i+1)/(2i+1) ≤ w^61 / (61 (1 − w²))
    let rem_hi = term.hi / 61.0 / (1.0 - w2.hi);
    let rem = Iv {
        lo: 0.0,
        hi: rem_hi.next_up().next_up(),
    };
    sum.add(rem).scale(2.0)
}

fn atan_point(x: f64) -> Iv {
    let mut z = Iv::exact(x);
    let mut times = 1.0;
    for _ in 0..2 {
        let d = Iv::exact(1.0).add(Iv::exact(1.0).add(z.sqr()).sqrt());
        z = z.div(d);
        times *= 2.0;
    }
    // |z| < 0.2: atan z = Σ (−1)^i z^(2i+1)/(2i+1)
    let z2 = z.sqr();
    let mut term = z;
    let mut sum = Iv::exact(0.0);
    for i in 0..40 {
        let t = term.div(Iv::exact((2 * i + 1) as f64));
        sum = if i % 2 == 0 { sum.add(t) } else { sum.sub(t) };
        term = term.mul(z2);
    }
    let next = term.lo.abs().max(term.hi.abs()) / 81.0;
    let rem = Iv {
        lo: -next.next_up(),
        hi: next.next_up(),
    };
    sum.add(rem).scale(times)
}

/// One element's energy form over the fluxes of the edges it touches.
#[derive(Clone, Debug)]
pub enum Element {
    /// A rectangle `a × b` of copper `t` thick (`rho_over_t` = ρ/t), the
    /// field of node `node`, with the edges through its west, east, south
    /// and north faces. Each face's flux in the +x (W, E) or +y (S, N)
    /// direction follows from the edge's orientation relative to `node`, so
    /// no sign is supplied. A face with no edge is insulating and carries 0.
    Cell {
        node: usize,
        a: Iv,
        b: Iv,
        rho_over_t: Iv,
        w: Option<usize>,
        e: Option<usize>,
        s: Option<usize>,
        n: Option<usize>,
    },
    /// A joint carrying `kappa · F²`, `F` the flux of one edge.
    Joint { edge: usize, kappa: Iv },
    /// A cell whose height `b` lies anywhere in `[b_lo, b_hi]` (a barrel
    /// row, its length declared only by bounds). Its energy
    /// `c₁/b + c₂·b` is convex in `b`, so the larger of the two ends bounds
    /// it at every height between: only the upper end of its interval is
    /// certified. Its node may also take [`Element::Joint`] edges that are
    /// not faces: the plating's radial injection ([`junction`]), whose
    /// divergence the cell's uniform divergence meets.
    Tall {
        node: usize,
        a: Iv,
        b_lo: Iv,
        b_hi: Iv,
        rho_over_t: Iv,
        w: Option<usize>,
        e: Option<usize>,
        s: Option<usize>,
        n: Option<usize>,
    },
}

/// A network: nodes (cells, terminals), edges (faces, joints), each edge
/// with an approximate conductance for the proposing solve only, and the
/// elements whose energy certifies the bound.
#[derive(Clone, Debug, Default)]
pub struct Network {
    pub nodes: usize,
    /// Each edge: from node, to node, conductance (siemens, approximate).
    pub edges: Vec<(usize, usize, f64)>,
    pub elements: Vec<Element>,
}

/// The outcome of [`Network::bound`].
#[derive(Clone, Debug)]
pub struct Bound {
    /// The certified upper bound on `R`, ohms: an interval whose upper end
    /// is the bound.
    pub energy: Iv,
    /// Conjugate-gradient iterations used, and the final relative residual.
    pub iterations: usize,
    pub residual: f64,
    /// The certified flow itself: each edge's flux, in quanta.
    pub flux: Vec<i64>,
}

/// Why [`Network::bound`] gave no bound.
#[derive(Clone, Debug, PartialEq)]
pub enum FlowError {
    /// The terminals are not connected through copper.
    Disconnected,
    /// The elements do not account for the graph (a builder's error).
    Malformed(String),
}

/// The number of quanta in the unit current: fluxes are integers of
/// `2⁻⁴⁰` amperes.
pub const QUANTA: i64 = 1 << 40;

impl Network {
    pub fn node(&mut self) -> usize {
        self.nodes += 1;
        self.nodes - 1
    }

    pub fn edge(&mut self, from: usize, to: usize, g: f64) -> usize {
        self.edges.push((from, to, g));
        self.edges.len() - 1
    }

    /// The certified bound on `R` between `source` and `sink`, after a
    /// potential solve of at most `max_iter` iterations. Refused when the
    /// two are not connected, or when the elements do not account for the
    /// graph exactly ([`Network::check`]).
    pub fn bound(
        &self,
        source: usize,
        sink: usize,
        max_iter: usize,
        tol: f64,
    ) -> Result<Bound, FlowError> {
        self.check(source, sink)?;
        let (u, iterations, residual) = self
            .solve(source, sink, max_iter, tol)
            .ok_or(FlowError::Disconnected)?;
        let mut flux: Vec<f64> = self
            .edges
            .iter()
            .map(|&(a, b, g)| g * (u[a] - u[b]))
            .collect();
        let current: f64 = self
            .edges
            .iter()
            .zip(&flux)
            .map(|(&(a, b, _), f)| {
                if a == source {
                    *f
                } else if b == source {
                    -f
                } else {
                    0.0
                }
            })
            .sum();
        if current.is_nan() || current <= 0.0 {
            return Err(FlowError::Disconnected);
        }
        for f in &mut flux {
            *f /= current;
        }
        let q = self
            .repair(&flux, source, sink)
            .ok_or(FlowError::Disconnected)?;
        self.conserves(&q, source, sink)?;
        Ok(Bound {
            energy: self.energy(&q),
            flux: q,
            iterations,
            residual,
        })
    }

    /// The elements account for the graph exactly, before any flux:
    /// - each element's face edges touch its node, no edge twice;
    /// - no node has two elements, and neither terminal has one;
    /// - every edge at a node with a [`Element::Cell`] is one of its faces;
    ///   at a [`Element::Tall`] node, every edge is a face or a joint (the
    ///   plating's injection);
    /// - every joint names an edge, at most one joint per edge;
    /// - every edge is a face of an element at one end at least, or a joint:
    ///   an edge no element prices would carry flux for nothing.
    ///
    /// The bound's validity rests on these: a flux no element prices, or a
    /// node whose divergence no field carries, would lower it unsoundly.
    pub fn check(&self, source: usize, sink: usize) -> Result<(), FlowError> {
        let bad = |m: String| Err(FlowError::Malformed(m));
        let mut owner: Vec<Option<(usize, bool)>> = vec![None; self.nodes]; // (element, tall)
        let mut joint = vec![false; self.edges.len()];
        let mut faced = vec![0u8; self.edges.len()];
        for (k, el) in self.elements.iter().enumerate() {
            let (node, faces, tall) = match el {
                Element::Cell {
                    node, w, e, s, n, ..
                } => (*node, [*w, *e, *s, *n], false),
                Element::Tall {
                    node, w, e, s, n, ..
                } => (*node, [*w, *e, *s, *n], true),
                Element::Joint { edge, .. } => {
                    if *edge >= self.edges.len() || joint[*edge] {
                        return bad(format!("joint {k} names edge {edge} twice or out of range"));
                    }
                    joint[*edge] = true;
                    continue;
                }
            };
            if node == source || node == sink {
                return bad(format!("element {k} is on a terminal"));
            }
            if owner[node].replace((k, tall)).is_some() {
                return bad(format!("node {node} has two elements"));
            }
            let mut mine: Vec<usize> = faces.iter().flatten().copied().collect();
            for &e in &mine {
                let (a, b, _) = self.edges[e];
                if a != node && b != node {
                    return bad(format!(
                        "element {k}'s face edge {e} does not touch node {node}"
                    ));
                }
                faced[e] += 1;
            }
            mine.sort_unstable();
            if mine.windows(2).any(|w| w[0] == w[1]) {
                return bad(format!("element {k} names a face twice"));
            }
        }
        // every edge at an element node is its face (or a joint at a Tall)
        let mut at: Vec<Vec<usize>> = vec![Vec::new(); self.nodes];
        for (e, &(a, b, _)) in self.edges.iter().enumerate() {
            if a == b {
                return bad(format!("edge {e} is a loop"));
            }
            at[a].push(e);
            at[b].push(e);
        }
        for (node, o) in owner.iter().enumerate() {
            let Some((k, tall)) = *o else { continue };
            let faces: Vec<usize> = match &self.elements[k] {
                Element::Cell { w, e, s, n, .. } | Element::Tall { w, e, s, n, .. } => {
                    [*w, *e, *s, *n].iter().flatten().copied().collect()
                }
                Element::Joint { .. } => unreachable!(),
            };
            for &e in &at[node] {
                if !faces.contains(&e) && !(tall && joint[e]) {
                    return bad(format!("edge {e} at node {node} is no face of element {k}"));
                }
            }
        }
        for e in 0..self.edges.len() {
            if faced[e] == 0 && !joint[e] {
                return bad(format!("edge {e} is priced by no element"));
            }
        }
        Ok(())
    }

    /// Every node's net outflow is exactly its target in quanta.
    fn conserves(&self, q: &[i64], source: usize, sink: usize) -> Result<(), FlowError> {
        let mut out = vec![0i128; self.nodes];
        for (e, &(a, b, _)) in self.edges.iter().enumerate() {
            out[a] += q[e] as i128;
            out[b] -= q[e] as i128;
        }
        for (i, &o) in out.iter().enumerate() {
            let target = if i == source {
                QUANTA as i128
            } else if i == sink {
                -(QUANTA as i128)
            } else {
                0
            };
            if o != target {
                return Err(FlowError::Malformed(format!("node {i} does not conserve")));
            }
        }
        Ok(())
    }

    /// The potentials with the source at 1 and the sink at 0, by
    /// Jacobi-preconditioned conjugate gradients on the graph Laplacian.
    /// Nodes not connected to either terminal keep potential 0 and carry no
    /// flux.
    fn solve(
        &self,
        source: usize,
        sink: usize,
        max_iter: usize,
        tol: f64,
    ) -> Option<(Vec<f64>, usize, f64)> {
        let n = self.nodes;
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for &(a, b, g) in &self.edges {
            adj[a].push((b, g));
            adj[b].push((a, g));
        }
        // the component of the source; it must hold the sink
        let mut seen = vec![false; n];
        let mut q = VecDeque::from([source]);
        seen[source] = true;
        while let Some(x) = q.pop_front() {
            for &(y, _) in &adj[x] {
                if !seen[y] {
                    seen[y] = true;
                    q.push_back(y);
                }
            }
        }
        if !seen[sink] {
            return None;
        }
        let free: Vec<usize> = (0..n)
            .filter(|&i| seen[i] && i != source && i != sink)
            .collect();
        let mut index = vec![usize::MAX; n];
        for (k, &i) in free.iter().enumerate() {
            index[i] = k;
        }
        let m = free.len();
        // A x = b on the free nodes: the graph Laplacian, b from the source at 1
        let mut b = vec![0.0; m];
        let mut ptr = vec![0usize; m + 1];
        let mut col = Vec::new();
        let mut val = Vec::new();
        let mut diag = vec![0.0; m];
        for (k, &i) in free.iter().enumerate() {
            for &(j, g) in &adj[i] {
                diag[k] += g;
                if j == source {
                    b[k] += g;
                }
                let jk = index[j];
                if jk != usize::MAX {
                    col.push(jk as u32);
                    val.push(-g);
                }
            }
            ptr[k + 1] = col.len();
        }
        let a = Csr {
            ptr,
            col,
            val,
            diag,
        };
        let amg = Amg::new(a);
        let a = &amg.levels[0];
        let mut x = vec![0.0; m];
        let mut r = b.clone();
        let mut z = vec![0.0; m];
        amg.precondition(&r, &mut z);
        let mut p = z.clone();
        let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let bnorm = b
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt()
            .max(f64::MIN_POSITIVE);
        let mut ap = vec![0.0; m];
        let mut it = 0;
        let mut res = r.iter().map(|v| v * v).sum::<f64>().sqrt() / bnorm;
        while it < max_iter && res > tol {
            a.mul(&p, &mut ap);
            let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
            if pap <= 0.0 {
                break;
            }
            let alpha = rz / pap;
            for k in 0..m {
                x[k] += alpha * p[k];
                r[k] -= alpha * ap[k];
            }
            amg.precondition(&r, &mut z);
            let rz2: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
            let beta = rz2 / rz;
            rz = rz2;
            for k in 0..m {
                p[k] = z[k] + beta * p[k];
            }
            it += 1;
            res = r.iter().map(|v| v * v).sum::<f64>().sqrt() / bnorm;
        }
        let mut u = vec![0.0; n];
        u[source] = 1.0;
        for (k, &i) in free.iter().enumerate() {
            u[i] = x[k];
        }
        Some((u, it, res))
    }

    /// Integer fluxes, exactly conserving: every node's net outflow is 0,
    /// the source's is `QUANTA`, the sink's `-QUANTA`. Non-tree edges keep
    /// their rounded flux; the tree edges (a breadth-first tree from the
    /// sink) carry each node's residue to it.
    fn repair(&self, flux: &[f64], source: usize, sink: usize) -> Option<Vec<i64>> {
        let n = self.nodes;
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (e, &(a, b, _)) in self.edges.iter().enumerate() {
            adj[a].push(e);
            adj[b].push(e);
        }
        let mut parent_edge = vec![usize::MAX; n];
        let mut order = Vec::new();
        let mut seen = vec![false; n];
        let mut q = VecDeque::from([sink]);
        seen[sink] = true;
        while let Some(x) = q.pop_front() {
            order.push(x);
            for &e in &adj[x] {
                let (a, b, _) = self.edges[e];
                let y = if a == x { b } else { a };
                if !seen[y] {
                    seen[y] = true;
                    parent_edge[y] = e;
                    q.push_back(y);
                }
            }
        }
        if !seen[source] {
            return None;
        }
        let mut q: Vec<i64> = flux
            .iter()
            .map(|f| (f * QUANTA as f64).round() as i64)
            .collect();
        // edges out of the sink's component carry nothing
        for (e, &(a, _, _)) in self.edges.iter().enumerate() {
            if !seen[a] {
                q[e] = 0;
            }
        }
        // net outflow of every node, tree edges excluded
        let mut out = vec![0i64; n];
        for (e, &(a, b, _)) in self.edges.iter().enumerate() {
            if parent_edge[a] == e || parent_edge[b] == e {
                continue;
            }
            out[a] += q[e];
            out[b] -= q[e];
        }
        // leaves first: each node's tree edge to its parent makes its net
        // outflow its target, and the residue moves to the parent
        for &x in order.iter().rev() {
            if x == sink {
                continue;
            }
            let target = if x == source { QUANTA } else { 0 };
            let e = parent_edge[x];
            let need = target - out[x]; // flux the tree edge must carry out of x
            let (a, b, _) = self.edges[e];
            let p = if a == x { b } else { a };
            q[e] = if a == x { need } else { -need };
            out[x] += need;
            out[p] -= need;
        }
        Some(q)
    }

    /// The exact field's energy for unit current, in an interval.
    pub fn energy(&self, q: &[i64]) -> Iv {
        let [a, b, c] = self.energy_by_kind(q);
        a.add(b).add(c)
    }

    /// [`Network::energy`] by element kind: sheet cells, junction joints,
    /// barrel cells (for the record of what a bound rests on).
    pub fn energy_by_kind(&self, q: &[i64]) -> [Iv; 3] {
        // the flux of edge `e` in the +x or +y direction at `node`'s face:
        // `into` for a W or S face (flux entering the node), else leaving;
        // q / 2^40 is exact in binary
        let face = |node: usize, e: Option<usize>, into: bool| -> Iv {
            match e {
                Some(i) => {
                    let out = if self.edges[i].0 == node { q[i] } else { -q[i] };
                    let v = if into { -out } else { out };
                    Iv::exact(v as f64 / QUANTA as f64)
                }
                None => Iv::exact(0.0),
            }
        };
        let quad = |x: Iv, y: Iv| x.mul(x).add(x.mul(y)).add(y.mul(y));
        let third = Iv::ratio(1, 3);
        let mut total = [Iv::exact(0.0); 3];
        for el in &self.elements {
            let term = match el {
                Element::Cell {
                    node,
                    a,
                    b,
                    rho_over_t,
                    w,
                    e,
                    s,
                    n,
                } => {
                    let x = a
                        .div(*b)
                        .mul(quad(face(*node, *w, true), face(*node, *e, false)));
                    let y = b
                        .div(*a)
                        .mul(quad(face(*node, *s, true), face(*node, *n, false)));
                    rho_over_t.mul(x.add(y)).mul(third)
                }
                Element::Joint { edge, kappa } => {
                    let v = Iv::exact(q[*edge] as f64 / QUANTA as f64);
                    kappa.mul(v.sqr())
                }
                Element::Tall {
                    node,
                    a,
                    b_lo,
                    b_hi,
                    rho_over_t,
                    w,
                    e,
                    s,
                    n,
                } => {
                    let at = |b: Iv| {
                        let x = a
                            .div(b)
                            .mul(quad(face(*node, *w, true), face(*node, *e, false)));
                        let y = b
                            .div(*a)
                            .mul(quad(face(*node, *s, true), face(*node, *n, false)));
                        rho_over_t.mul(x.add(y)).mul(third)
                    };
                    let (lo, hi) = (at(*b_lo), at(*b_hi));
                    Iv {
                        lo: lo.lo.min(hi.lo),
                        hi: lo.hi.max(hi.hi),
                    }
                }
            };
            let k = match el {
                Element::Cell { .. } => 0,
                Element::Joint { .. } => 1,
                Element::Tall { .. } => 2,
            };
            total[k] = total[k].add(term);
        }
        total
    }
}

/// A symmetric matrix: its diagonal, and its off-diagonal entries by row.
struct Csr {
    ptr: Vec<usize>,
    col: Vec<u32>,
    val: Vec<f64>,
    diag: Vec<f64>,
}

impl Csr {
    fn n(&self) -> usize {
        self.diag.len()
    }

    fn mul(&self, x: &[f64], y: &mut [f64]) {
        for i in 0..self.n() {
            let mut s = self.diag[i] * x[i];
            for k in self.ptr[i]..self.ptr[i + 1] {
                s += self.val[k] * x[self.col[k] as usize];
            }
            y[i] = s;
        }
    }

    /// Pairs each node with its strongest unpaired neighbour: the
    /// aggregate of every node, and their count.
    fn pairs(&self) -> (Vec<usize>, usize) {
        let n = self.n();
        let mut agg = vec![usize::MAX; n];
        let mut count = 0;
        for i in 0..n {
            if agg[i] != usize::MAX {
                continue;
            }
            let mut best = None;
            let mut w = 0.0;
            for k in self.ptr[i]..self.ptr[i + 1] {
                let j = self.col[k] as usize;
                if agg[j] == usize::MAX && j != i && -self.val[k] > w {
                    w = -self.val[k];
                    best = Some(j);
                }
            }
            agg[i] = count;
            if let Some(j) = best {
                agg[j] = count;
            }
            count += 1;
        }
        (agg, count)
    }

    /// `Pᵀ A P` for the piecewise-constant prolongation of `agg`.
    fn coarsen(&self, agg: &[usize], nc: usize) -> Csr {
        let mut rows: Vec<HashMap<u32, f64>> = vec![HashMap::new(); nc];
        let mut diag = vec![0.0; nc];
        for i in 0..self.n() {
            let ci = agg[i];
            diag[ci] += self.diag[i];
            for k in self.ptr[i]..self.ptr[i + 1] {
                let cj = agg[self.col[k] as usize];
                if cj == ci {
                    diag[ci] += self.val[k];
                } else {
                    *rows[ci].entry(cj as u32).or_insert(0.0) += self.val[k];
                }
            }
        }
        let mut ptr = vec![0usize; nc + 1];
        let mut col = Vec::new();
        let mut val = Vec::new();
        for (c, row) in rows.into_iter().enumerate() {
            let mut r: Vec<(u32, f64)> = row.into_iter().collect();
            r.sort_unstable_by_key(|e| e.0);
            for (j, v) in r {
                col.push(j);
                val.push(v);
            }
            ptr[c + 1] = col.len();
        }
        Csr {
            ptr,
            col,
            val,
            diag,
        }
    }
}

/// An aggregation multigrid preconditioner (pairwise matching twice per
/// level, Galerkin coarse operators, a symmetric V-cycle with damped Jacobi
/// smoothing). It only speeds the proposing solve: nothing it computes
/// enters the bound.
struct Amg {
    levels: Vec<Csr>,
    aggs: Vec<Vec<usize>>,
}

impl Amg {
    fn new(a: Csr) -> Amg {
        let mut levels = vec![a];
        let mut aggs = Vec::new();
        while levels.last().map_or(0, Csr::n) > 400 && levels.len() < 30 {
            let top = levels.last().unwrap();
            let (a1, n1) = top.pairs();
            let c1 = top.coarsen(&a1, n1);
            let (a2, n2) = c1.pairs();
            if n2 * 10 > top.n() * 9 {
                break; // no longer coarsening
            }
            let c2 = c1.coarsen(&a2, n2);
            aggs.push(a1.iter().map(|&x| a2[x]).collect());
            levels.push(c2);
        }
        Amg { levels, aggs }
    }

    fn precondition(&self, r: &[f64], z: &mut [f64]) {
        let x = self.cycle(0, r);
        z.copy_from_slice(&x);
    }

    fn cycle(&self, l: usize, b: &[f64]) -> Vec<f64> {
        let a = &self.levels[l];
        let n = a.n();
        let mut x = vec![0.0; n];
        let mut t = vec![0.0; n];
        let jacobi = |x: &mut Vec<f64>, t: &mut Vec<f64>, sweeps: usize| {
            for _ in 0..sweeps {
                a.mul(x, t);
                for i in 0..n {
                    x[i] += 0.6 * (b[i] - t[i]) / a.diag[i];
                }
            }
        };
        if l + 1 == self.levels.len() {
            jacobi(&mut x, &mut t, 60);
            return x;
        }
        jacobi(&mut x, &mut t, 2);
        a.mul(&x, &mut t);
        let agg = &self.aggs[l];
        let nc = self.levels[l + 1].n();
        let mut rc = vec![0.0; nc];
        for i in 0..n {
            rc[agg[i]] += b[i] - t[i];
        }
        let ec = self.cycle(l + 1, &rc);
        for i in 0..n {
            x[i] += 1.6 * ec[agg[i]];
        }
        jacobi(&mut x, &mut t, 2);
        x
    }
}

/// The coefficient `κ` of one grid face of a via's junction square: the
/// energy `κ·F²` of carrying that face's flux `F` into its own barrel
/// column (`AET-WO-1303b`).
///
/// **Geometry.** The via's hole (radius `r_d`, the drill's) is concentric
/// with the square (a stated condition, ruled for the measured stop). The
/// face lies on `y = a`; this grid face is its part `x` in `[u, v]`, and its
/// barrel column is the wedge of the rays through `(u, a)` and `(v, a)`,
/// angle `Δθ`. The square less the circle `r0 = r_d + β` is the layer's
/// copper, `t` thick (`rho_over_t` = ρ/t, the declared minimum).
///
/// **The field, in three regions, each divergence-free, meeting with equal
/// normal flux:**
/// 1. **Rays,** from the face to `r0`: the grid cell beside the face
///    carries a constant normal component `F/len` on it (RT0), and each
///    ray tube keeps its flux, so no flux crosses a ray. With
///    `R² = x² + a²`, the energy is `(ρ/t)(F/len)²(1/a)∫R²·ln(R/r0) dx`.
/// 2. **The band** `r_d ≤ r ≤ r0`, where the arrival density per radian
///    `f(θ) = (F/len)R²/a` is made uniform, `g = F/Δθ`:
///    `r J_r = [f u + g (1 − u)]/t`, `u = (r − r_d)/β`, and
///    `J_θ = −C(θ)/(β t)`, `C(θ) = ∫(f − g)`, which vanishes at both rays,
///    so nothing leaves the wedge sideways. Divergence-free by
///    construction. Its energy is exactly
///    `(ρ/t)∫[c_ff f² + c_fg f g + c_gg g² + C²(r0² − r_d²)/(2β²)] dθ`, the
///    `c` from `∫(α + γr)²/r dr` over the band (`α + γr` being `r J_r t`).
/// 3. **Into the plating** (radii `r_i = r_d − t_p … r_d`,
///    `A = (r_d² − r_i²)/2` per radian), over the layer's thickness `t`:
///    `J_r = c(r² − r_i²)/(2r)`, `c = F/(AΔθt)`, zero at the inner surface,
///    matching the arrival `g/(r_d t)` at the wall, with divergence `c`,
///    uniform. The barrel's cell for this column on this layer's row has
///    uniform divergence `−c` ([`barrel`]), and the two are orthogonal
///    (radial, in-shell), so their energies add. Its energy is exactly
///    `(ρ/t)·I/(A²Δθ)·F²`, `I = ∫(r² − r_i²)²/(4r) dr`.
///
/// The energy decreases as `t` grows, term by term, so `t` is the layer's
/// minimum.
///
/// **Rounding.** Every integral is a sum over `panels` sub-intervals of the
/// width times the interval extension of the integrand over the
/// sub-interval: an enclosure of the integral, with no assumption on the
/// integrand's shape. `atan` and `ln` are [`Iv`]'s own.
#[allow(clippy::too_many_arguments)]
pub fn junction(
    a: f64,
    u: f64,
    v: f64,
    r_d: Iv,
    beta: Iv,
    rho_over_t: Iv,
    t_p: Iv,
    panels: usize,
) -> Iv {
    let ai = Iv::exact(a);
    let len = Iv::exact(v).sub(Iv::exact(u));
    let r0 = r_d.add(beta);
    assert!(r0.hi < a, "the band reaches the square's face");
    let ln_r0 = r0.ln();
    let theta = |x: Iv| x.div(ai).atan();
    let th_u = theta(Iv::exact(u));
    let dth = theta(Iv::exact(v)).sub(th_u);
    let mut i_ray = Iv::exact(0.0);
    let mut i_f2 = Iv::exact(0.0);
    let mut i_c2 = Iv::exact(0.0);
    let width = (v - u) / panels as f64;
    for p in 0..panels {
        let lo = u + width * p as f64;
        let hi = if p + 1 == panels {
            v
        } else {
            u + width * (p + 1) as f64
        };
        let x = Iv { lo, hi };
        let dx = Iv::exact(hi).sub(Iv::exact(lo));
        let r2 = x.sqr().add(ai.sqr());
        // R² (ln R − ln r0) = R² (½ ln R² − ln r0)
        i_ray = i_ray.add(r2.mul(r2.ln().scale(0.5).sub(ln_r0)).mul(dx));
        i_f2 = i_f2.add(r2.mul(dx));
        // C / F = (x − u)/len − (θ(x) − θ_u)/Δθ, dθ = a dx / R²
        let phi = x
            .sub(Iv::exact(u))
            .div(len)
            .sub(theta(x).sub(th_u).div(dth));
        i_c2 = i_c2.add(phi.sqr().mul(ai.div(r2)).mul(dx));
    }
    let len2a = len.sqr().mul(ai);
    let ray = i_ray.div(len2a);
    // ∫f² dθ = (1/len²)(1/a)∫R² dx;  ∫f g dθ = ∫g² dθ = 1/Δθ  (per F²)
    let f2 = i_f2.div(len2a);
    let g2 = Iv::exact(1.0).div(dth);
    // r J_r t = α + γ r, α = g(1 + p) − f p, γ = (f − g)/β, p = r_d/β:
    // ∫(α + γr)²/r dr = α² L + 2αγβ + γ² D, L = ln(r0/r_d), D = (r0² − r_d²)/2
    let one = Iv::exact(1.0);
    let p = r_d.div(beta);
    let l = r0.ln().sub(r_d.ln());
    let d = r0.sqr().sub(r_d.sqr()).scale(0.5);
    let db2 = d.div(beta.sqr());
    let c_ff = p.sqr().mul(l).sub(p.scale(2.0)).add(db2);
    let c_fg = p
        .mul(one.add(p))
        .mul(l)
        .scale(-2.0)
        .add(one.add(p.scale(2.0)).scale(2.0))
        .sub(db2.scale(2.0));
    let c_gg = one.add(p).sqr().mul(l).sub(one.add(p).scale(2.0)).add(db2);
    let band = c_ff
        .mul(f2)
        .add(c_fg.mul(g2))
        .add(c_gg.mul(g2))
        .add(db2.mul(i_c2));
    // the plating: I = ¼[(r_d⁴ − r_i⁴)/4 − r_i²(r_d² − r_i²) + r_i⁴ ln(r_d/r_i)]
    let r_i = r_d.sub(t_p);
    let (d2, i2) = (r_d.sqr(), r_i.sqr());
    let quarter = d2.sqr().sub(i2.sqr()).scale(0.25);
    let i_p = quarter
        .sub(i2.mul(d2.sub(i2)))
        .add(i2.sqr().mul(r_d.ln().sub(r_i.ln())))
        .scale(0.25);
    let area = d2.sub(i2).scale(0.5);
    let plating = i_p.div(area.sqr()).mul(g2);
    rho_over_t.mul(ray.add(band).add(plating))
}

/// A via's barrel: the plating shell (radii `r_d − t_p … r_d`) as a periodic
/// sheet in `(s, z)`, `s = r_d θ`, with the shell field
/// `J = (0, p·r/A, q/A)` of `AET-WO-1303` S4, whose energy is at most
/// `(ρ/A)∫(q² + r_d² p²) dθ dz` = `(ρ r_d / A)∫|K|² ds dz` for the sheet
/// current `K = (p, q/r_d)`: a sheet of `ρ/t = ρ r_d/A`
/// (`rho_over_t`). Its columns are `widths` (each `r_d Δθ_k`, closing the
/// circle), its rows top to bottom have heights in `rows[i]` (an interval
/// of the declared range). `top` and `bottom`, when given, are terminal
/// nodes joined to the first and last rows through their outer faces.
/// Returns each cell's node, `[row][column]`. A layer joins a row through
/// [`junction`] edges into its cells, whose divergence they carry.
pub fn barrel(
    net: &mut Network,
    widths: &[Iv],
    rows: &[(Iv, Iv)],
    rho_over_t: Iv,
    top: Option<usize>,
    bottom: Option<usize>,
) -> Vec<Vec<usize>> {
    let k = widths.len();
    let cells: Vec<Vec<usize>> = rows
        .iter()
        .map(|_| (0..k).map(|_| net.node()).collect())
        .collect();
    // approximate conductances for the proposing solve
    let g_s = |i: usize, c: usize| {
        let (lo, hi) = rows[i];
        let b = (lo.hi + hi.hi) / 2.0;
        let a = widths[c].hi;
        b / (a * rho_over_t.hi)
    };
    let g_z = |i: usize, c: usize| {
        let (lo, hi) = rows[i];
        let b = (lo.hi + hi.hi) / 2.0;
        widths[c].hi / (b * rho_over_t.hi)
    };
    let mut east = vec![vec![usize::MAX; k]; rows.len()];
    let mut south = vec![vec![usize::MAX; k]; rows.len()];
    for i in 0..rows.len() {
        for c in 0..k {
            let d = (c + 1) % k;
            let g = 2.0 / (1.0 / g_s(i, c) + 1.0 / g_s(i, d));
            east[i][c] = net.edge(cells[i][c], cells[i][d], g);
            if i + 1 < rows.len() {
                let g = 2.0 / (1.0 / g_z(i, c) + 1.0 / g_z(i + 1, c));
                south[i][c] = net.edge(cells[i][c], cells[i + 1][c], g);
            }
        }
    }
    let last = rows.len() - 1;
    let ends: Vec<(Option<usize>, Option<usize>)> = (0..k)
        .map(|c| {
            let t = top.map(|n| net.edge(n, cells[0][c], 2.0 * g_z(0, c)));
            let b = bottom.map(|n| net.edge(cells[last][c], n, 2.0 * g_z(last, c)));
            (t, b)
        })
        .collect();
    for i in 0..rows.len() {
        for c in 0..k {
            let w = (c + k - 1) % k;
            // z grows downward: "n" is the face toward the next row (+z)
            net.elements.push(Element::Tall {
                node: cells[i][c],
                a: widths[c],
                b_lo: rows[i].0,
                b_hi: rows[i].1,
                rho_over_t,
                w: Some(east[i][w]),
                e: Some(east[i][c]),
                s: if i > 0 {
                    Some(south[i - 1][c])
                } else {
                    ends[c].0
                },
                n: if i < last {
                    Some(south[i][c])
                } else {
                    ends[c].1
                },
            });
        }
    }
    cells
}

/// A cell's lower-left corner, in cells.
pub type Cell = (i64, i64);

/// One layer's copper as square cells of one size, indexed by their lower
/// left corner in cells: copper cells, terminal cells (each holding a
/// terminal node: equipotential, no resistance) and ports (a cell's face
/// joined to another element, a via junction's [`junction`]). Built in two
/// steps, [`Sheet::new`] then any [`Sheet::port`]s, then [`Sheet::finish`],
/// which writes each copper cell's [`Element::Cell`].
pub struct Sheet {
    /// Each cell's node, copper and terminal.
    pub id: BTreeMap<(i64, i64), usize>,
    /// Each face's edge, keyed by the two cells it lies between, west or
    /// south first; a port's far cell is the one it faces.
    faces: BTreeMap<(Cell, Cell), usize>,
    own: Vec<(i64, i64)>,
    rho_over_t: Iv,
}

impl Sheet {
    /// The cells' nodes and every face between two of them, except between
    /// two cells of one terminal. `terminals` maps a cell to its terminal's
    /// node; a terminal cell listed in `cells` too is a terminal.
    pub fn new(
        net: &mut Network,
        cells: &[(i64, i64)],
        terminals: &BTreeMap<(i64, i64), usize>,
        rho_over_t: Iv,
    ) -> Sheet {
        let mut id = terminals.clone();
        let mut own = Vec::new();
        let mut sorted = cells.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for c in sorted {
            if let std::collections::btree_map::Entry::Vacant(v) = id.entry(c) {
                v.insert(net.node());
                own.push(c);
            }
        }
        let terminal: BTreeSet<usize> = terminals.values().copied().collect();
        let g = 1.0 / rho_over_t.hi; // between two square cells
        let mut faces = BTreeMap::new();
        for (&c, &a) in &id {
            for d in [(1, 0), (0, 1)] {
                let o = (c.0 + d.0, c.1 + d.1);
                if let Some(&b) = id.get(&o) {
                    if a == b && terminal.contains(&a) {
                        continue;
                    }
                    faces.insert((c, o), net.edge(a, b, g));
                }
            }
        }
        Sheet {
            id,
            faces,
            own,
            rho_over_t,
        }
    }

    /// Joins copper cell `cell`'s face toward `toward` (a neighbouring
    /// position holding no cell) to node `to`, with approximate
    /// conductance `g`; the edge is that face's.
    pub fn port(
        &mut self,
        net: &mut Network,
        cell: (i64, i64),
        toward: (i64, i64),
        to: usize,
        g: f64,
    ) -> usize {
        assert!(!self.id.contains_key(&toward), "a port faces a cell");
        let d = (toward.0 - cell.0, toward.1 - cell.1);
        assert!(d.0.abs() + d.1.abs() == 1, "a port faces a neighbour");
        let key = if d.0 + d.1 > 0 {
            (cell, toward)
        } else {
            (toward, cell)
        };
        let e = net.edge(self.id[&cell], to, g);
        assert!(self.faces.insert(key, e).is_none());
        e
    }

    /// Writes every copper cell's element.
    pub fn finish(self, net: &mut Network) {
        let one = Iv::exact(1.0);
        for &c in &self.own {
            let get = |from: (i64, i64), to: (i64, i64)| self.faces.get(&(from, to)).copied();
            net.elements.push(Element::Cell {
                node: self.id[&c],
                a: one,
                b: one,
                rho_over_t: self.rho_over_t,
                w: get((c.0 - 1, c.1), c),
                e: get(c, (c.0 + 1, c.1)),
                s: get((c.0, c.1 - 1), c),
                n: get(c, (c.0, c.1 + 1)),
            });
        }
    }
}

/// A via's hole: drill radius `r_d`, band width `beta` ([`junction`]),
/// the minimum plating `t_p`, and the copper's resistivity bound `rho`.
#[derive(Clone, Copy, Debug)]
pub struct Hole {
    pub r_d: Iv,
    pub beta: Iv,
    pub t_p: Iv,
    pub rho: Iv,
}

impl Hole {
    /// The plating's cross-section per radian, `t_p(2r_d − t_p)/2`.
    pub fn area(&self) -> Iv {
        self.t_p.mul(self.r_d.scale(2.0).sub(self.t_p)).scale(0.5)
    }
}

/// A layer a via joins: its sheet, the barrel row that is its copper (the
/// row's height range must be the layer's thickness range) and its minimum
/// thickness, at which [`junction`]'s coefficients are taken.
pub struct Join<'a> {
    pub sheet: &'a mut Sheet,
    pub row: usize,
    pub t_min: Iv,
}

/// A via centred on cell corner `corner`, its junction square `m` cells
/// each side of it (half-side `a = m·h`, `h` the cell side in the units of
/// `hole`), its barrel ([`barrel`]) with one column per grid face around
/// the square, counter-clockwise from the east face's south end, and a
/// [`junction`] joint from each copper or terminal cell beside the square
/// on each joined layer to its column on the layer's row. The square's own
/// cells must be absent from every joined sheet. Returns the barrel's
/// cells, `[row][column]`, and each join's port edges (oriented from the
/// layer into the barrel).
#[allow(clippy::too_many_arguments)]
pub fn via_ports(
    net: &mut Network,
    corner: (i64, i64),
    m: i64,
    h: f64,
    hole: &Hole,
    joins: Vec<Join<'_>>,
    rows: &[(Iv, Iv)],
    top: Option<usize>,
    bottom: Option<usize>,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let (cx, cy) = corner;
    let a = m as f64 * h;
    // (outside cell, the square's cell it faces, u, v along the face)
    let mut ports = Vec::new();
    let off = |k: i64| ((k as f64) * h, ((k + 1) as f64) * h);
    for j in cy - m..cy + m {
        let (u, v) = off(j - cy);
        ports.push(((cx + m, j), (cx + m - 1, j), u, v));
    }
    for i in (cx - m..cx + m).rev() {
        let (u, v) = off(i - cx);
        ports.push(((i, cy + m), (i, cy + m - 1), u, v));
    }
    for j in (cy - m..cy + m).rev() {
        let (u, v) = off(j - cy);
        ports.push(((cx - m - 1, j), (cx - m, j), u, v));
    }
    for i in cx - m..cx + m {
        let (u, v) = off(i - cx);
        ports.push(((i, cy - m - 1), (i, cy - m), u, v));
    }
    let ai = Iv::exact(a);
    let dth = |u: f64, v: f64| Iv::exact(v).div(ai).atan().sub(Iv::exact(u).div(ai).atan());
    let widths: Vec<Iv> = ports
        .iter()
        .map(|&(_, _, u, v)| hole.r_d.mul(dth(u, v)))
        .collect();
    let rho_barrel = hole.rho.mul(hole.r_d).div(hole.area());
    let cells = barrel(net, &widths, rows, rho_barrel, top, bottom);
    let mut all = Vec::new();
    for join in joins {
        let mut edges = Vec::new();
        for i in cx - m..cx + m {
            for j in cy - m..cy + m {
                assert!(
                    !join.sheet.id.contains_key(&(i, j)),
                    "a cell inside a via's junction square"
                );
            }
        }
        let rho_over_t = hole.rho.div(join.t_min);
        let mut kappa: BTreeMap<(u64, u64), Iv> = BTreeMap::new();
        for (k, &(cell, toward, u, v)) in ports.iter().enumerate() {
            if !join.sheet.id.contains_key(&cell) {
                continue; // no copper beside this face: it carries nothing
            }
            // κ depends on |u|, |v| only: the face's two halves mirror
            let key = (
                u.abs().min(v.abs()).to_bits(),
                u.abs().max(v.abs()).to_bits(),
            );
            let kap = *kappa.entry(key).or_insert_with(|| {
                let (p, q) = if u < 0.0 { (-v, -u) } else { (u, v) };
                junction(a, p, q, hole.r_d, hole.beta, rho_over_t, hole.t_p, 256)
            });
            let e = join
                .sheet
                .port(net, cell, toward, cells[join.row][k], 1.0 / kap.hi);
            net.elements.push(Element::Joint {
                edge: e,
                kappa: kap,
            });
            edges.push(e);
        }
        all.push(edges);
    }
    (cells, all)
}

/// A sheet between two terminals, each a set of cells: the network and the
/// two terminal nodes.
pub fn sheet(
    cells: &[(i64, i64)],
    source: &[(i64, i64)],
    sink: &[(i64, i64)],
    rho_over_t: Iv,
) -> (Network, usize, usize) {
    let mut net = Network::default();
    let src = net.node();
    let snk = net.node();
    let mut terminals = BTreeMap::new();
    for &c in source {
        terminals.insert(c, src);
    }
    for &c in sink {
        terminals.insert(c, snk);
    }
    Sheet::new(&mut net, cells, &terminals, rho_over_t).finish(&mut net);
    (net, src, snk)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar: `L × W` cells between two terminal columns carries the
    /// uniform field, which lies in the elements' space, so the bound is
    /// `ρL/(Wt)` exactly, up to outward rounding.
    #[test]
    fn a_bar_is_bounded_at_its_exact_resistance() {
        let (l, w) = (40i64, 6i64);
        let mut cells = Vec::new();
        for x in 0..l {
            for y in 0..w {
                cells.push((x, y));
            }
        }
        let src: Vec<_> = (0..w).map(|y| (-1, y)).collect();
        let snk: Vec<_> = (0..w).map(|y| (l, y)).collect();
        let rho_over_t = Iv::exact(1.0);
        let (net, s, t) = sheet(&cells, &src, &snk, rho_over_t);
        let b = net.bound(s, t, 10_000, 1e-14).unwrap();
        let exact = l as f64 / w as f64;
        assert!(b.energy.hi >= exact, "{:?} under {exact}", b.energy);
        assert!(
            b.energy.hi <= exact * (1.0 + 1e-9),
            "{:?} over {exact}",
            b.energy
        );
    }

    /// A 90° bend of a strip `W` wide: each arm `L` long from the inside
    /// corner. The corner's exact excess is −0.4407 squares, so the
    /// resistance is `2L/W − 1 + 0.5590…` squares (1303's certified bracket,
    /// 0.5577–0.5599 for the corner square). The bound must hold above the
    /// bracket's lower end and is close to it.
    #[test]
    fn a_right_angle_bend_is_bounded_above_its_bracket() {
        let (l, w) = (24i64, 8i64);
        let mut cells = Vec::new();
        for x in 0..l + w {
            for y in 0..w {
                cells.push((x, y));
            }
        }
        for x in l..l + w {
            for y in w..l + w {
                cells.push((x, y));
            }
        }
        let src: Vec<_> = (0..w).map(|y| (-1, y)).collect();
        let snk: Vec<_> = (l..l + w).map(|x| (x, l + w)).collect();
        let (net, s, t) = sheet(&cells, &src, &snk, Iv::exact(1.0));
        let b = net.bound(s, t, 20_000, 1e-13).unwrap();
        // squares: arms 2 L / W, corner square 1 replaced by 0.5577..0.5599
        let lower = 2.0 * l as f64 / w as f64 + 0.5577;
        let upper = 2.0 * l as f64 / w as f64 + 0.5599;
        assert!(
            b.energy.hi >= lower,
            "{:?} under the bracket {lower}",
            b.energy
        );
        assert!(
            b.energy.hi <= upper + 0.03,
            "{:?} far over {upper}",
            b.energy
        );
        eprintln!(
            "bend: bound {:.6} exact {:.4}..{:.4}",
            b.energy.hi, lower, upper
        );
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Copper,
        Source,
        Sink,
    }

    /// The cells of side `h` over `[-span, span]²` that `kind` places, by
    /// their four corners' coordinates.
    fn classify(
        h: f64,
        span: f64,
        kind: impl Fn(&[(f64, f64); 4]) -> Option<Kind>,
    ) -> (Network, usize, usize) {
        let n = (span / h).ceil() as i64;
        let (mut cells, mut src, mut snk) = (Vec::new(), Vec::new(), Vec::new());
        for i in -n..n {
            for j in -n..n {
                let (x0, y0) = (i as f64 * h, j as f64 * h);
                let c = [(x0, y0), (x0 + h, y0), (x0, y0 + h), (x0 + h, y0 + h)];
                match kind(&c) {
                    Some(Kind::Copper) => cells.push((i, j)),
                    Some(Kind::Source) => src.push((i, j)),
                    Some(Kind::Sink) => snk.push((i, j)),
                    None => {}
                }
            }
        }
        sheet(&cells, &src, &snk, Iv::exact(1.0))
    }

    fn within(c: &[(f64, f64); 4], (cx, cy): (f64, f64), r: f64) -> bool {
        c.iter().all(|&(x, y)| (x - cx).hypot(y - cy) <= r)
    }

    /// The nearest distance from the square to a point outside it is at a
    /// corner or an edge; all four corners beyond `r` and the point not
    /// level with an edge within `r` is enough here (the centre is never
    /// level with a cell straddling the circle at a flat side: tested by
    /// the min over the edge's closest point).
    fn beyond(c: &[(f64, f64); 4], (cx, cy): (f64, f64), r: f64) -> bool {
        let (x0, y0, x1, y1) = (c[0].0, c[0].1, c[3].0, c[3].1);
        let dx = (x0 - cx).max(0.0).max(cx - x1);
        let dy = (y0 - cy).max(0.0).max(cy - y1);
        dx.hypot(dy) >= r
    }

    /// The annulus `1 ≤ r ≤ 4` between a disc electrode and the plane
    /// outside: exactly `ln 4 / 2π` squares. A cell holding any of the
    /// electrodes' copper counts as resistive copper (more energy, still a
    /// bound); only a cell wholly inside an electrode is a terminal.
    #[test]
    fn an_annulus_is_bounded_at_or_above_its_exact_resistance() {
        let exact = 4f64.ln() / (2.0 * std::f64::consts::PI);
        let mut last = f64::INFINITY;
        for h in [1.0 / 8.0, 1.0 / 16.0, 1.0 / 32.0] {
            let (net, s, t) = classify(h, 4.2, |c| {
                if within(c, (0.0, 0.0), 1.0) {
                    Some(Kind::Source)
                } else if beyond(c, (0.0, 0.0), 4.0) {
                    Some(Kind::Sink)
                } else {
                    Some(Kind::Copper)
                }
            });
            let b = net.bound(s, t, 2000, 1e-12).unwrap();
            eprintln!("annulus h {h}: bound {:.6} exact {exact:.6}", b.energy.hi);
            assert!(b.energy.hi >= exact);
            assert!(b.energy.hi < last);
            last = b.energy.hi;
        }
        assert!(last < exact * 1.05, "{last} not near {exact}");
    }

    /// The two-hole sheet, bounded: two equipotential holes of radius 1,
    /// centres 4 apart, in the disc of radius `√3` about their midpoint, which
    /// meets both hole circles at right angles (it is a flux line of the
    /// infinite sheet's field, in bipolar coordinates). Exactly
    /// `2 acosh(2)/π` squares. The refinement series is printed.
    #[test]
    fn the_two_hole_sheet_is_bounded_and_converges_under_refinement() {
        let exact = 2.0 * 2f64.acosh() / std::f64::consts::PI;
        let disc = 3f64.sqrt();
        let mut last = f64::INFINITY;
        for h in [1.0 / 8.0, 1.0 / 16.0, 1.0 / 32.0, 1.0 / 64.0, 1.0 / 128.0] {
            let (net, s, t) = classify(h, 3.0, |c| {
                if within(c, (-2.0, 0.0), 1.0) {
                    Some(Kind::Source)
                } else if within(c, (2.0, 0.0), 1.0) {
                    Some(Kind::Sink)
                } else if within(c, (0.0, 0.0), disc) {
                    Some(Kind::Copper)
                } else {
                    None
                }
            });
            let b = net.bound(s, t, 2000, 1e-12).unwrap();
            eprintln!(
                "two-hole h {h}: bound {:.6} exact {exact:.6} ratio {:.4}",
                b.energy.hi,
                b.energy.hi / exact
            );
            assert!(b.energy.hi >= exact);
            assert!(b.energy.hi < last);
            last = b.energy.hi;
        }
        assert!(last < exact * 1.15, "{last} not near {exact}");
    }

    /// The barrel alone, end to end: the plating's cross-section
    /// `π t_p (2 r_d − t_p)` carries the current over its length `H`, so
    /// `R = ρH/(π t_p (2r_d − t_p))` exactly, and the uniform axial field
    /// lies in the elements' space.
    #[test]
    fn a_barrel_is_bounded_at_its_exact_resistance() {
        let (rho, r_d, t_p, h) = (1.0, 0.15, 0.018, 1.6);
        let area = Iv::exact(t_p).mul(Iv::exact(2.0 * r_d - t_p)).scale(0.5);
        let rho_over_t = Iv::exact(rho * r_d).div(area);
        let k = 48;
        let two_pi = Iv::exact(1.0).atan().scale(8.0);
        let widths: Vec<Iv> = (0..k)
            .map(|_| two_pi.mul(Iv::exact(r_d)).div(Iv::exact(k as f64)))
            .collect();
        let rows: Vec<(Iv, Iv)> = (0..8)
            .map(|_| (Iv::exact(h / 8.0), Iv::exact(h / 8.0)))
            .collect();
        let mut net = Network::default();
        let (top, bottom) = (net.node(), net.node());
        barrel(
            &mut net,
            &widths,
            &rows,
            rho_over_t,
            Some(top),
            Some(bottom),
        );
        let b = net.bound(top, bottom, 2000, 1e-13).unwrap();
        let exact = rho * h / (std::f64::consts::PI * t_p * (2.0 * r_d - t_p));
        assert!(b.energy.hi >= exact, "{:?} under {exact}", b.energy);
        assert!(
            b.energy.hi <= exact * (1.0 + 1e-9),
            "{:?} over {exact}",
            b.energy
        );
        // a row whose height is only known in a range takes the worse end
        let rows: Vec<(Iv, Iv)> = (0..8)
            .map(|_| (Iv::exact(h / 16.0), Iv::exact(h / 8.0)))
            .collect();
        let mut net = Network::default();
        let (top, bottom) = (net.node(), net.node());
        barrel(
            &mut net,
            &widths,
            &rows,
            rho_over_t,
            Some(top),
            Some(bottom),
        );
        let b = net.bound(top, bottom, 2000, 1e-13).unwrap();
        assert!(b.energy.hi >= exact);
    }

    /// A via in a disc of copper: the outer rim (radius 2 mm) an
    /// electrode, the via's barrel 1.6 mm long below the layer, its bottom
    /// the other electrode. Shorting the hole wall at the layer lowers `R`
    /// to the sheet's `ρ ln(b/r_d)/(2πt)` plus the barrel's
    /// `ρH/(π t_p (2r_d − t_p))`: a floor the bound must hold above.
    #[test]
    fn a_via_in_a_disc_is_bounded_above_its_shorted_floor() {
        let (rho, t, t_p, r_d, b, hb) = (1.0, 0.03, 0.018, 0.15, 2.0, 1.6);
        let h: f64 = 1.0 / 32.0;
        let m = 6;
        let mut net = Network::default();
        let rim = net.node();
        let bottom = net.node();
        let n = (2.1 / h).ceil() as i64;
        let mut cells = Vec::new();
        let mut terminals = BTreeMap::new();
        for i in -n..n {
            for j in -n..n {
                if (-m..m).contains(&i) && (-m..m).contains(&j) {
                    continue;
                }
                let (x0, y0) = (i as f64 * h, j as f64 * h);
                let c = [(x0, y0), (x0 + h, y0), (x0, y0 + h), (x0 + h, y0 + h)];
                if beyond(&c, (0.0, 0.0), b) {
                    terminals.insert((i, j), rim);
                } else {
                    cells.push((i, j));
                }
            }
        }
        let rho_over_t = Iv::exact(rho).div(Iv::exact(t));
        let mut sheet = Sheet::new(&mut net, &cells, &terminals, rho_over_t);
        let hole = Hole {
            r_d: Iv::exact(r_d),
            beta: Iv::exact(0.03),
            t_p: Iv::exact(t_p),
            rho: Iv::exact(rho),
        };
        let mut rows = vec![(Iv::exact(t), Iv::exact(t))];
        rows.extend((0..8).map(|_| (Iv::exact(hb / 8.0), Iv::exact(hb / 8.0))));
        let joins = vec![Join {
            sheet: &mut sheet,
            row: 0,
            t_min: Iv::exact(t),
        }];
        via_ports(
            &mut net,
            (0, 0),
            m,
            h,
            &hole,
            joins,
            &rows,
            None,
            Some(bottom),
        );
        sheet.finish(&mut net);
        let bd = net.bound(rim, bottom, 2000, 1e-12).unwrap();
        let pi = std::f64::consts::PI;
        let sheet_r = rho * (b / r_d).ln() / (2.0 * pi * t);
        let barrel_r = rho * hb / (pi * t_p * (2.0 * r_d - t_p));
        let floor = sheet_r + barrel_r;
        eprintln!(
            "via in disc: bound {:.5} floor {floor:.5} (sheet {sheet_r:.5}, barrel {barrel_r:.5}), excess {:.5}",
            bd.energy.hi,
            bd.energy.hi - floor
        );
        assert!(bd.energy.hi >= floor);
        assert!(
            bd.energy.hi <= floor * 1.01,
            "{} far over {floor}",
            bd.energy.hi
        );
    }

    /// [`junction`]'s coefficient holds its own field's energy, computed
    /// here directly by midpoint quadrature over the field as defined (in
    /// polar coordinates), and is within 0.1 % of it: the enclosure's own
    /// width. A term dropped or mis-signed moves it outside.
    #[test]
    fn a_junction_coefficient_holds_its_fields_energy() {
        let (rho, t, t_p, r_d, beta, a) = (1.0f64, 0.03f64, 0.018f64, 0.15f64, 0.03f64, 0.1875f64);
        let r_i = r_d - t_p;
        let area = (r_d * r_d - r_i * r_i) / 2.0;
        let r0 = r_d + beta;
        for (u, v) in [(0.0, 0.03125), (0.0625, 0.09375), (0.15625, 0.1875)] {
            let len = v - u;
            let (tu, tv) = ((u / a).atan(), (v / a).atan());
            let dth = tv - tu;
            let g = 1.0 / dth;
            let n = 4000;
            let (mut ray, mut band, mut plat) = (0.0, 0.0, 0.0);
            for k in 0..n {
                let th = tu + dth * (k as f64 + 0.5) / n as f64;
                let x = a * th.tan();
                let rr = a / th.cos();
                let f = rr * rr / (a * len);
                let c = (x - u) / len - (th - tu) / dth;
                ray += f * f * (rr / r0).ln() * dth / n as f64;
                let nr = 400;
                for j in 0..nr {
                    let r = r_d + beta * (j as f64 + 0.5) / nr as f64;
                    let w = (r - r_d) / beta;
                    let kr = (f * w + g * (1.0 - w)) / r;
                    let kt = c / beta;
                    band += (kr * kr + kt * kt) * r * (beta / nr as f64) * (dth / n as f64);
                }
            }
            let cc = 1.0 / (area * dth * t);
            let nr = 4000;
            for j in 0..nr {
                let r = r_i + t_p * (j as f64 + 0.5) / nr as f64;
                let jr = cc * (r * r - r_i * r_i) / (2.0 * r);
                plat += jr * jr * r * (t_p / nr as f64);
            }
            plat *= rho * dth * t;
            let (ray, band) = (rho / t * ray, rho / t * band);
            let direct = ray + band + plat;
            let rho_over_t = Iv::exact(rho).div(Iv::exact(t));
            let k = junction(
                a,
                u,
                v,
                Iv::exact(r_d),
                Iv::exact(beta),
                rho_over_t,
                Iv::exact(t_p),
                256,
            );
            eprintln!(
                "junction [{u}, {v}]: κ {:.5} direct {direct:.5} (rays {ray:.5}, band {band:.5}, plating {plat:.5})",
                k.hi
            );
            // the quadrature here is midpoint, good to about 1e-6
            assert!(
                k.hi >= direct * (1.0 - 1e-5),
                "κ {} under the field's energy {direct}",
                k.hi
            );
            assert!(
                k.hi <= direct * 1.001,
                "κ {} over the field's energy {direct}",
                k.hi
            );
        }
    }

    /// An edge no element prices, an element on a terminal, or a face that
    /// does not touch its node is refused before any flux.
    #[test]
    fn a_network_its_elements_do_not_account_for_is_refused() {
        let (net, s, t) = sheet(&[(0, 0), (1, 0)], &[(-1, 0)], &[(2, 0)], Iv::exact(1.0));
        assert!(net.bound(s, t, 100, 1e-12).is_ok());
        // a direct short between the terminals
        let mut short = net.clone();
        short.edge(s, t, 1.0);
        assert!(matches!(
            short.bound(s, t, 100, 1e-12),
            Err(FlowError::Malformed(_))
        ));
        // a cell's face swapped for an edge that is not its own
        let mut wrong = net.clone();
        if let Element::Cell { e, .. } = &mut wrong.elements[0] {
            *e = Some(0);
        }
        assert!(matches!(
            wrong.bound(s, t, 100, 1e-12),
            Err(FlowError::Malformed(_))
        ));
        // a face dropped from its cell: that edge is priced by one end only
        // or by none
        let mut dropped = net.clone();
        for el in &mut dropped.elements {
            if let Element::Cell { w, .. } = el {
                *w = None;
            }
        }
        assert!(matches!(
            dropped.bound(s, t, 100, 1e-12),
            Err(FlowError::Malformed(_))
        ));
    }

    /// A 0.2 mm track at 45° on 31.25 µm cells: the staircase of whole
    /// cells inside it loses copper, and this measures how much. Terminals
    /// are the half-planes beyond the track's two end lines; exactly
    /// `length / width` squares.
    #[test]
    fn a_diagonal_track_is_bounded_and_its_staircase_measured() {
        let (w, d) = (0.2f64, 3.0f64);
        let exact = (2.0 * d / 2f64.sqrt()) / w;
        for h in [1.0 / 32.0, 1.0 / 64.0] {
            let (net, s, t) = classify(h, d + 0.5, |c| {
                let strip = c
                    .iter()
                    .all(|&(x, y)| (x - y).abs() / 2f64.sqrt() <= w / 2.0);
                if !strip {
                    return None;
                }
                if c.iter().all(|&(x, y)| x + y <= 0.0) {
                    Some(Kind::Source)
                } else if c.iter().all(|&(x, y)| x + y >= 2.0 * d) {
                    Some(Kind::Sink)
                } else if c
                    .iter()
                    .all(|&(x, y)| x + y >= -0.2 && x + y <= 2.0 * d + 0.2)
                {
                    Some(Kind::Copper)
                } else {
                    None
                }
            });
            let b = net.bound(s, t, 2000, 1e-12).unwrap();
            eprintln!(
                "45° track h {h}: bound {:.4} exact {exact:.4} ratio {:.3}",
                b.energy.hi,
                b.energy.hi / exact
            );
            assert!(b.energy.hi >= exact);
        }
    }

    /// The interval functions hold the true values (checked against the
    /// platform's, which agree to a few units in the last place).
    #[test]
    fn the_interval_functions_hold_their_values() {
        for x in [1e-3, 0.137, 0.5, 1.0, 1.7, 2.0, 3.3, 47.0, 1e6] {
            let l = Iv::exact(x).ln();
            assert!(
                l.lo <= x.ln() + 1e-15 && x.ln() - 1e-15 <= l.hi,
                "ln {x}: {l:?}"
            );
            assert!(
                l.hi - l.lo < 1e-13 * (1.0 + x.ln().abs()),
                "ln {x} too wide: {l:?}"
            );
        }
        for x in [-1.0, -0.4142, -0.01, 0.0, 0.3, 0.7, 1.0] {
            let a = Iv::exact(x).atan();
            assert!(
                a.lo <= x.atan() + 1e-15 && x.atan() - 1e-15 <= a.hi,
                "atan {x}: {a:?}"
            );
            assert!(a.hi - a.lo < 1e-13, "atan {x} too wide: {a:?}");
        }
        let pi = Iv::exact(1.0).atan().scale(4.0);
        assert!(pi.lo <= std::f64::consts::PI && std::f64::consts::PI <= pi.hi);
    }

    /// The integer repair makes every node conserve exactly, whatever the
    /// proposal: a deliberately wrong proposal still certifies.
    #[test]
    fn a_poor_proposal_still_gives_a_bound_at_or_above_the_exact_value() {
        let (l, w) = (20i64, 4i64);
        let mut cells = Vec::new();
        for x in 0..l {
            for y in 0..w {
                cells.push((x, y));
            }
        }
        let src: Vec<_> = (0..w).map(|y| (-1, y)).collect();
        let snk: Vec<_> = (0..w).map(|y| (l, y)).collect();
        let (net, s, t) = sheet(&cells, &src, &snk, Iv::exact(1.0));
        // three iterations only
        let b = net.bound(s, t, 3, 0.0).unwrap();
        assert!(b.energy.hi >= l as f64 / w as f64);
    }
}
