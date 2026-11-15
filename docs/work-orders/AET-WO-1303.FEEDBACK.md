# AET-WO-1303 — FEEDBACK (spike, then STOP for the architect's ruling)

**Branch:** `wo/aet-1303-extraction` from `origin/main` `121344d` · **Work order:** `origin/claude/argon-repo-docs-synthesis-rjrb0d`, `docs/work-orders/AET-WO-1303.md` (lands through docs PR #128; not committed here) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then STOP. No build code before the ruling.

The predictions below were committed before any measured run. They are not edited afterwards; the results section sets each result beside its prediction.

**Labels.** **[read]** read at source (this repository at `121344d`, PR #129's branch at `4cbe26d`, or a cited document). **[measured]** run here, the command and output kept under `spike/1303/`. **[inferred]** reasoned, neither read nor run.

**Seen before these were written (orientation, not measurement).** `cargo build -p aethelgard-cli` (host, capped scope), which measures nothing. `4300c`'s FEEDBACK on PR #129 (`4cbe26d`), whose numbers are that work order's measurements, quoted here as [read]. `1305` and `4100` FEEDBACK. No board was emitted and nothing was extracted before this commit.

## Predictions — committed before any measured run

### What the predictions rest on [read]

- `extract` bounds one quantity: each I2C line's capacitance, from above, by Dirichlet's principle (Chew 7.1.48) with the trial `ψ = max(0, 1 − d/S)`, plus the near-plane cap of `4100` (`extract.rs` module doc, `line_with`). It has no lower bound on anything, and no resistance.
- `Stack` declares one side of each quantity: dielectric **minima**, permittivity **maximum**, copper **maximum**, finished thickness min and max (`stack.rs`). Each is the side a capacitance upper bound needs. A resistance upper bound and an impedance interval need the other sides too.
- `route::Layer` is `F` or `B`. Tracks never run on an inner layer; at 4 layers each inner layer is an unfilled plane, and each plane-net pad is joined by an `F.Cu` stub to a through via (`route.rs` L4–6, L999–1003). At 2 layers every non-ground net, `VDD_3V3` included, is routed as tracks at the one declared width (0.2 mm on both boards).
- `net_constraints[].impedance_ohms` exists and is validated non-zero (`lib.rs` L1208); nothing reads it.
- The droop scenarios declare `source_milliohms` (500) and `load_milliamps` (100) per scenario (`scenarios/sentinel-01/droop-*.json`); the bridge takes them as given (`analog.rs` L297). Nothing relates them to the board.
- `4300c` is stopped on PR #129. `4300d` and `4300e1` are intent lines in `roadmap.md` §5 P4, not written work orders. So no S1 consumer exists on `main`.

### S1 — the consumers

| id | prediction | label |
|---|---|---|
| P1.1 | Three consumers, none on `main`. `4300c` Fmax wants an **upper** bound on C per driven line, resolved to its row (10/30/50 pF): a bound useful at row granularity, so ±10 % suffices near a boundary and nothing finer matters. Without it the check has no C (4300c D2 offers a declared `net_constraint` instead). | read / inferred |
| P1.2 | `4300e1` wants a **two-sided interval** on the USB pair's differential impedance, inside the USB window. Sentinel-02 is the STM32H753's on-chip **full-speed** PHY (no ULPI part on the board), and for full speed the USB 2.0 figure is the cable's 90 Ω ±15 %, with no PCB-trace clause. Today the gate does not exist. | inferred |
| P1.3 | The supply gate wants an **upper** bound on the copper resistance from the source pad to each MCU supply pin, at a precision of about 10 mΩ: at 0.5 A that is 5 mV of a 165 mV (5 %) tolerance. Today nothing gates it; the droop takes a declared `source_milliohms`. | inferred |
| P1.4 | A fourth consumer: the I2C rise (`RUL-19` (1)) already reads the C bound; the ground return of the supply path (VSS) is a fifth quantity that nobody has named. | read / inferred |

### S2 — capacitance on every signal line

| id | prediction | label |
|---|---|---|
| P2.1 | Already done by `4300c` (PR #129, `4cbe26d`) for SPI on the reference, the 4-layer fixture and `D1`: the bound is net-agnostic, and only the line list and the pin term change. No new run is needed. | read |
| P2.2 | No board in the repository carries a UART or timer line, so there is nothing to measure for them. | inferred |
| P2.3 | The shapes the reader refuses, so the bound never meets them: arcs, through-hole pads (any `drill` on a pad), circle, oval and custom pads, zones on signal nets, a filled zone, inner-layer tracks, and copper text or graphics. | inferred from `allowed()` and `TOP` |

### S3 — differential impedance

| id | prediction | label |
|---|---|---|
| P3.1 | A **certified** interval needs four bounds: C and C_air, each from above and below. Dirichlet gives the upper bounds; Thomson's principle, with a divergence-free trial flux, gives the lower ones. Then `Z_odd = 1/(c0 √(C·C_air))` and `Z_diff = 2 Z_odd` (quasi-TEM). | inferred |
| P3.2 | `1305`'s shell trial alone is useless here: 1.3–1.7× above exact (4100 S4) puts the lower Z bound 15–25 % below the truth, wider than the whole ±15 % window. | inferred |
| P3.3 | A two-dimensional variational pair, P1 Dirichlet energy in a grounded box (upper C) and a P1 stream-function energy in an insulated box (lower C), brackets C within 1 % on a graded mesh of about 10⁴ nodes. It is a numerical solve, which the work order says this is not; that is a decision, not a fork. | inferred |
| P3.4 | **Exact check:** the bracket holds Cohn's exact zero-thickness coupled stripline (odd mode), within 1 % on each side, at every geometry tried. | inferred |
| P3.5 | The closed forms (IPC-2141, Wadell) are approximations with a stated accuracy, not bounds. Against the bracket, at least one point of the S3 grid falls outside it by more than 3 %. | inferred |
| P3.6 | On the 4-layer fixture's stackup at the reference rules (W = S = 0.2 mm, h about 0.18–0.22 mm, εr 3.6–5.4), the pair is about 110–135 Ω, so it **fails a 90 Ω ±15 % window**. 90 Ω needs a wider track (about 0.3 mm at this prepreg). | inferred |
| P3.7 | The stackup tolerances alone widen the interval to about ±10–13 % around its centre, even with an exact solver: εr from 3.6 to 5.4 accounts for about ±8 %, h for about ±5 %. With the declared side only (εr max 5.4, minimum not declared), no upper Z bound exists. | inferred |
| P3.8 | At 2 layers no rigorous interval exists: the only reference is the `B.Cu` ground zone, 1.2 mm or more away, cut by `B.Cu` tracks, and never read from the board. The gate refuses USB at 2 layers. | inferred |
| P3.9 | For full speed the pair is **electrically short** on any board here: a 4 ns minimum edge is about 0.6 m in FR-4, and a trace under about 0.1 m (a sixth of that) behaves as a lumped load. So the useful full-speed check may be a length and C bound, not 90 Ω. Raised as a decision. | inferred |

### S4 — supply-path resistance

| id | prediction | label |
|---|---|---|
| P4.1 | **Neither board has a regulator** or any other source part on `VDD_3V3`. The stand-in source is each non-MCU pad on the rail in turn. The probe reports, per MCU supply pin, the bound from the best and from the worst stand-in. Flagged as a decision: the real source pad arrives with `4300d`. | inferred |
| P4.2 | The bound is Thomson's principle for resistance (any divergence-free unit current flow gives `R ≤ ∫ρ|J|²`), applied to a flow along one copper path. Rayleigh monotonicity then covers the rest of the copper. For a bend in a track, a flow along the offset curves of the centreline gives `R ≤ ρ·L_centre/(W·t)` exactly, the corner term being `(θ − 2 tan(θ/2))W²/8 ≤ 0`. That needs each segment at least `(W/2)·tan(θ/2)` long; shorter ones need their own term. | inferred |
| P4.3 | Every input is on the wrong side for resistance as declared: an upper R bound needs the minimum finished copper, the minimum etched width, the minimum barrel plating and the maximum temperature. `Stack` has none of them. Each becomes a new field with a source, or a condition. | read / inferred |
| P4.4 | **Exact checks:** a straight bar (`ρL/(Wt)`), a radial annulus (`ρ ln(b/a)/(2πt)`) and a barrel (`ρH/(π t_p (2r − t_p))`): each bound equals or exceeds the exact value. | inferred |
| P4.5 | Reference (2 layers, 0.2 mm tracks), at 30 µm minimum copper, 18 µm plating and copper at 105 °C with a foil allowance: about 4–5 mΩ per mm of track and about 3–10 mΩ per via. MCU supply pins bound at **50–300 mΩ** from the best stand-in. At 0.5 A that is 25–150 mV, comparable to the 165 mV tolerance, so the gate is useful, and 0.2 mm supply tracks are marginal. | inferred |
| P4.6 | The 4-layer fixture: each pin's stub, two vias and the `In2` plane bound at **10–40 mΩ**, dominated by the stubs and vias. The plane itself contributes under 10 mΩ. | inferred |
| P4.7 | KiCad's refill joins a plane to its own net's via with **solid** copper (no thermal spokes), and keeps the declared clearance (about 0.2 mm) from a foreign via. Measured once in the image. | inferred |
| P4.8 | Per-pin R times the total load bounds that pin's drop: in a resistive network the transfer resistance is at most the driving-point resistance. Checked on random networks. | inferred |

### S5 — where the bounds go

| id | prediction | label |
|---|---|---|
| P5.1 | `extract`'s verdict keeps `lines` byte-identical for Sentinel-01 and gains two sibling arrays, `pairs` and `supplies`, each entry with its terms and its declared limit. | inferred |
| P5.2 | Limits: impedance as a declared window (minimum and maximum ohms) on a new `differential_pairs` entry, replacing the single, unused `impedance_ohms`; supply as a declared maximum path resistance per domain, or a maximum drop with a cited load. Each is refused by name when the board has the feature and the limit is absent (`INV-06`). | inferred |
| P5.3 | The stackup gains the other side of each bound (dielectric maxima, permittivity minimum, copper minimum, plating minimum, temperature maximum), each with a source. That is a specification version. | inferred |
