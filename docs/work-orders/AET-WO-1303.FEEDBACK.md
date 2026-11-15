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

## Results — measured after the predictions commit (`2171536`)

### Where it ran

- **Host only, for every number below.** `aethelgard emit` and `extract` (debug build), and Python probes in a scratch venv (numpy 2.5.4, scipy 1.18.1). `emit` reproduces 1305's reference board, sha256 `7b9b0e3d…2070`, and the 4-layer fixture `6acc50fe…65d9` (`spike/1303/boards/`). `extract` on both writes `boards/*/extract.json`.
- **The container, once, stopped by its own guard.** `spike/1303/img.sh` runs the pinned image (`c69ff9bc…`) under rootless podman with `--memory 8g --memory-swap 8g --pids-limit -1`, and writes nothing under `/sys/fs/cgroup` (the architect's correction to kickoff rule 3). Its read-back (`spike/1303/caps.log`):
  - `memory.max=8589934592`;
  - **`memory.swap.max=8589934592`, not 0**;
  - `pids.max=max`.

  The guard killed the container before its command ran. Podman 5.7 sets `memory.swap.max` to the `--memory-swap` value; it does not set the difference. This is the global CLAUDE.md's F11 (Aethelgard `2101`), re-measured. So the one container step, **P4.7 (how KiCad's refill joins `In2` to a via), is blocked.** The harness that would produce it: `img.sh` running `kicad-cli pcb export gerbers --layers In2.Cu` with zone refill on the 4-layer board, then a point-in-region test at radius 0.4 mm around each own-net via and each foreign via. See D9.
- **Research** (a delegated read-only pass, `spike/1303/research-sources.md`). Every value cited below as [read] was read in the primary text by that pass or by me, and I spot-checked two by hand: USB 2.0 §7.1.6.1, and DS12117 Rev 10 pp. 215–216, rendered and read.

### Each prediction beside its result

| # | Prediction | Result | Label | Verdict |
|---|---|---|---|---|
| P1.1 | 4300c wants an upper C at row granularity | As predicted. 4300c's SPI lines are 16.9–20.0 pF without the MCU pin, so the 30 pF row with 10 pF to spare | read (PR #129) | right |
| P1.2 | USB wants a Z interval in 90 Ω ±15 %, with no PCB clause for full speed | **Wrong on the clause.** USB 2.0 §7.1.6.1 says "The transmission line between the receptacle and RS must be 90 Ω ±15%". But it sits in the downstream-port (hub/host) sentence, and the next paragraph budgets an **upstream full-speed device** at **25 pF per conductor** on that line, with D+/D− differing by under 10 %. Which one binds a device is a ruling (D2) | read | wrong |
| P1.3 | Upper R at about 10 mΩ precision | As predicted. The load is 658 mA at most (DS12117 Rev 10 Table 120, Tj 125 °C), so 10 mΩ is 6.6 mV of the 165 mV tolerance | read / measured | right |
| P1.4 | The I2C rise reads C; the ground return is unnamed | As predicted | read | right |
| P2.1 | 4300c already measured SPI | Yes: PR #129, `4cbe26d`, every SPI line on the reference, the 4-layer fixture and D1 | read | right |
| P2.2 | No UART or timer line on any board | None in `examples/` or the CLI fixtures | read | right |
| P2.3 | The refused shapes | Confirmed from `allowed()`, `TOP` and `route::Layer` (below) | read | right |
| P3.1 | Four bounds give a certified interval | Built: primal P1 in a grounded box, stream-function dual in an insulated box | measured | right |
| P3.2 | The shell trial puts the Z lower bound 15–25 % low | **Far worse.** On a single line, the shell's C is 2.52× the certified C and its C_air 1.63×, so Z0's lower bound is **50.7 % below** the truth | measured | wrong (too mild) |
| P3.3 | The bracket within 1 % at about 10⁴ nodes | 0.24–0.52 % wide at 25–47 k nodes, about 1 s per geometry | measured | right |
| P3.4 | Cohn's exact stripline lies inside | **Inside at all 8 points** (4 geometries × 2 meshes): 0.9908–0.99997 below, 1.0014–1.0109 above. This checks `stripline_odd`. `microstrip_odd`'s dual (a cut with duplicated nodes, its own code path) has no exact case. On nested uniform refinement (`s3/nested.out`) its bounds never cross, the lower rises and the upper falls at every level: the test that exposed F1. The first run failed Cohn: see F1 | measured | right, after F1 |
| P3.5 | IPC-2141 lies outside by more than 3 % at some point | Outside at **all six** fixed geometries, by −8.6 % to +3.9 % | measured | right |
| P3.6 | W = S = 0.2 mm gives about 110–135 Ω | 115.1–115.5 Ω at h 0.2 mm, εr 3.61, t 43 µm. Near 90 Ω needs W ≈ 0.4 mm at s = 0.4 mm, or 0.3–0.35 mm at s 0.15–0.2 mm | measured | right |
| P3.7 | Tolerances alone give about ±10–13 % | **±21–32 %**, so hi/lo is 1.53–1.93 against the 1.353 (103.5/76.5) any interval must meet to fit, whatever W. The bracket itself is under 0.5 % wide; the rest is input. Three inputs dominate (`s3/sens.out`): the solder mask (−13.8 % at OSH Park's cited 0.8 mil maximum, with εr 5.4 a condition), εr's IPC maximum (−15 %) and etch (±6 %). **No geometry fits,** even with etch halved (hi/lo 1.45) | measured | wrong (too mild) |
| P3.8 | No rigorous interval at 2 layers | Agreed. The reference's `B.Cu` carries 55 tracks (108 mm, 30 of them `VDD_3V3`) through the ground zone, and the fill is never read | measured / inferred | right |
| P3.9 | Full speed is electrically short | TFR/TFF 4–20 ns (USB 2.0 Table 7-9, read), about 0.6 m of FR-4 at 4 ns. The reference outline is 45.2 × 40.4 mm | read / inferred | right |
| P4.1 | No regulator on either board | None: `VDD_3V3` has 29 pads on U1 (16), U2 (3), U3, U4, C3–C6, R1, R2, R4 and R5 | measured | right |
| P4.2 | Tube flow, R ≤ ρL/(Wt) under a length condition | Derived. On the routed boards the condition and its companions (pad end, via entry) often fail at W. A path then narrows its tube (rigorous) or is refused by name. The first version checked only the bends, and allowed what the second forbids: F7 | measured | right, with F7 |
| P4.3 | Every input is on the wrong side as declared | Confirmed. Copper minimum, plating minimum, temperature and a material allowance are all new | read | right |
| P4.4 | Exact checks hold | Bar, annulus and barrel equal their closed forms; the 90° bend lies inside a certified bracket (corner 0.558–0.560 squares against the tube's 1) | measured | right |
| P4.5 | Reference 50–300 mΩ from the best stand-in | **9.1–125 mΩ** on the 7 of 16 pins it bounds; 9 pins are refused by name from every stand-in | measured | partly; coverage not predicted |
| P4.6 | 4-layer 10–40 mΩ | **8.1–116 mΩ** on 15 of 16 pins; U1.108 refused | measured | wrong high end |
| P4.7 | Refill joins vias solidly | **Blocked** (container guard) | — | not measured |
| P4.8 | Per-pin R × total load bounds the drop | Holds on 300 random networks: the maximum ratio is 1.000000 | measured | right |
| P5.1–P5.3 | Shape of the verdict and limits | Proposal below | inferred | — |

### S1 — the consumers, exactly

| Gate (state today) | Quantity, direction | Precision useful on Sentinel-02 | What the gate does today without it |
|---|---|---|---|
| `4300c` Fmax (stopped on PR #129, D1–D4 open) | C per driven line (SCK, MOSI, each chip select), **upper bound**, plus the receivers' pins | Row-level: C rounded up to 10/30/50 pF (DS12117 Table 149). The reference's SPI lines bound at 16.9–20.0 pF without the MCU pin, so a bound within about 1.5× of the truth is enough [read, PR #129] | Nothing; 4300c D2 offers a declared `net_constraint` instead |
| `4300e1` USB, static (unwritten) | **For a device:** C per D+/D− conductor on the board between receptacle and transceiver, **upper bound** ≤ 25 pF, and **|C+ − C−| < 10 %**, which needs a lower bound too (USB 2.0 §7.1.6.1, read). **Or**, if the 90 Ω sentence binds the device: Z_diff, a **two-sided interval** inside 76.5–103.5 Ω | C: the 4-layer per-length upper bound (285 fF/mm at 0.18 mm, 4100) allows about 80 mm of pair within 25 pF. Z: an interval of ±15 % at most, and today's inputs give ±22–33 % (S3) | Nothing (4300e1 is unwritten; 4300 S3 proposed "the 90 Ω pair from extraction") |
| `4300d` supply drop (unwritten) and the droop (`2301`) | R from the source pad to each MCU supply pin, **upper bound**; then the drop ≤ R × the declared load | 10 mΩ ≈ 6.6 mV at 658 mA (DS12117 Rev 10 Table 120, Tj 125 °C; 550 mA at Tj 105 °C from ITCM, Table 118) of a 165 mV tolerance (3.3 V ±5 %). BOR3 falls at 2.54 V minimum (read) | The droop takes a scenario's declared `source_milliohms` (500) and `load_milliamps` (100); nothing relates them to copper |
| I2C rise (`RUL-19` (1), merged) | C per line, upper | As today | Reads `extract` |
| **Ground return (found here)** | R of the GND path, pin to source, upper | As the supply | Nothing names it (D7) |

### S2 — capacitance on every signal line: cited, not re-run

- **Ruled on PR #129** (the architect's comment 6887905174, relayed to this builder's mailbox) [read]:
  - `4300c` widens `extract.rs` to SPI lines (SCK, MOSI, MISO, chip selects);
  - it adds a `pin_capacitance` group to `parts.json`, cited maxima only, and a part without one makes extraction refuse a driven line by name;
  - a driven line's load excludes the MCU's own pin (Table 149's C is external load);
  - I2C keeps UM10204's 10 pF per pin.

  This work order adds nothing to that.
- **4300c measured it** (PR #129, `4cbe26d`, `spike/4300c/probe-output.txt`):
  - the bound machinery is net-agnostic;
  - on the reference, the 4-layer fixture and D1, every SPI line bounds at 8.9–14.8 pF of copper;
  - only the line list and the pin term change.

  The ICM-20948 fixture has no board, and D1's I2C lines exceed their own declared bound [read].
- **No board carries a UART or timer line** [read], so there is nothing to measure.
- **What the reader refuses,** and so the bound never meets [read]:
  - a top-level item outside `TOP` (`extract.rs:663-676`): an `arc`, a `gr_poly`, a `gr_line` or a `gr_text` on copper;
  - a pad child not in `allowed("pad")` (`extract.rs:597-604`), which has no `drill`: so no through-hole pad and no padstack;
  - any pad shape but `rect` or `roundrect` (`extract.rs:764`): circle, oval, trapezoid and custom;
  - copper in a footprint outside its pads;
  - a zone other than the declared ground or planes (`route::connectivity`), and a filled zone;
  - a segment on any layer but `F.Cu` or `B.Cu` (`route.rs:2048-2053`), so no inner-layer track.

  An arc or an inner track would need its own shell. Every other refusal is copper of unknown shape.

### S3 — differential impedance: a certified bracket, and what it shows

**The bound** (`spike/1303/s3/bracket.py`).
- **Reduction.** By symmetry, odd-mode C is one strip, the plane and an electric wall at the pair's midline.
- **Upper C (Dirichlet, Chew 7.1.48).** The energy of the P1 minimiser with ψ = 1 on the strip and 0 on the wall, the plane and a *grounded* box. A grounded box only adds ground, so C_open ≤ C_box ≤ E.
- **Lower C (Thomson).** D = rot χ for continuous P1 χ is divergence-free with continuous normal flux. χ is constant on the *insulated* box and jumps by Q across a cut from the strip to the plane, so the flux off the strip is Q. Then C ≥ Q²/∫|D|²/ε, and D = 0 outside the box is admissible.
- **Interval.** Each C is taken at the geometry corner that raises it (W + δ, gap − δ about fixed centres; h minimum; t maximum; εr maximum; mask present) or lowers it, and the same for C_air. That gives Z_diff = 2/(c0 √(C·C_air)), quasi-TEM.
- **Monotonicity in each input is by containment:**
  - ground as a half-space nested in the wall position and in h;
  - the strip nested under the fixed-centre etch model;
  - ε pointwise.

**Checks.**
- **Exact:** Cohn's zero-thickness coupled stripline, odd mode, solved on the half-domain with the strip on the symmetry plane. It lies inside `stripline_odd`'s bracket at every point (`s3/exact.out`).
- **Consistency of `microstrip_odd`** (its dual is a separate code path): nested uniform refinement at two geometries, four levels each (`s3/nested.out`). The bounds never cross, the lower bound rises, and the upper falls.
- **Closed forms against the bracket at fixed geometry** (`s3/usb.out` A):
  - IPC-2141 (second-hand formula text; the standard is paywalled) is **outside at all six points**, by −8.6 % to +3.9 %.
  - No published coupled-microstrip table was reachable: Bryant & Weiss 1968 and Garg & Bahl are paywalled (research record items 3–4).
- **The interval over the 4-layer fixture's tolerances** (`s3/usb.out` B, s = 0.2 mm):

| W (mm) | εr taken | Z_diff interval (Ω) | hi/lo (fits only if ≤ 1.353) |
|---|---|---|---|
| 0.20 | 3.4–5.4 (5.4 the fixture's declared IPC maximum; 3.4 a stated condition, since no minimum is published) | 70.0–135.0 | 1.93 |
| 0.20 | 3.43–3.91 (±5 % about OSH Park's 3.61 at 1 GHz and Isola's 3.72 at 100 MHz) | 81.1–134.6 | 1.66 |
| 0.30 | 3.43–3.91 | 70.9–112.0 | 1.58 |
| 0.35 | 3.43–3.91 | 66.8–103.8 | 1.55 |
| 0.40 | 3.43–3.91 | 63.2–96.8 | 1.53 |
| 0.30, etch halved, t ≥ 35 µm | 3.43–3.91 | 73.3–107.4 | 1.47 |

The mask is taken at OSH Park's maximum, 0.6 ± 0.2 mil (≤ 0.021 mm) [read], with εr 5.4 (a condition). The etch is ±25 µm about fixed centres, and copper is 30–70 µm (conditions).

- **Why so wide** (`s3/sens.out`, one input at a time about W 0.3 mm, nominal 96.6 Ω):
  - h ±0.0202 mm: −3.9/+3.3 %;
  - εr ±5 %: ±2.5 %;
  - εr up to 5.4: −14.6 %;
  - t 30–70 µm: −4.9/+2.7 %;
  - etch ±25 µm: ±6 %;
  - **solder mask 0–21 µm at εr 5.4: −13.8 %** (50 µm: −17.2 %; 25 µm at εr 4.0: −10.8 %). In odd mode the field crowds into the gap the mask fills.

  An exact solver given the same inputs would be just as wide, because the bracket is under 0.5 % wide. The gate is limited by what the stackup declares, not by the bound.
- **Two layers:** refused. No plane lies under the pair, and the `B.Cu` ground zone is unfilled in the file and cut by 55 tracks.
- **Full speed is electrically short.** The edge is 4–20 ns (Table 7-9), and the board is 45 × 40 mm. The quantity §7.1.6.1 gives a device is capacitance, which the existing machinery bounds from above (D2).

### S4 — supply-path resistance: a Thomson bound, measured

**The bound** (`spike/1303/s4/rbound.py` and `rbound2.py`, each module doc). Terminals are the two pads, each one equipotential (its solder and lead), with unit current between them. Any divergence-free J confined to the copper gives R ≤ ∫ρ|J|². Deleting copper never lowers R (Rayleigh), so one path's flow bounds the whole net. The flow, piece by piece:
- **Track chain.** Uniform flow along the offset curves of the centreline. At a bend of deviation θ the inner offset loses 2y·tan(θ/2) and the outer gains yθ, so ∫ℓ(y)dy = LW + (θ − 2 tan(θ/2))W²/8 ≤ LW, and **R ≤ ρ·L_centre/(w·t_min)**. It is admissible while every segment, on each side, is at least (w/2)·Σtan(θ/2) over the bends whose inside is on that side. Otherwise the path's tube narrows to the w that is admissible.
- **Pad end.** The end cross-section (width w, normal to the track) lies inside the pad's rounded box (an erosion test). Otherwise w narrows.
- **Pass through a pad.** At 2 layers the supply is daisy-chained through pads, and a pad between two tracks is not an equipotential. It is priced as a straight virtual track between the two track ends, admissible when its stadium lies inside the pad's copper.
- **Via entry, the general fan.** The tube stops at any cross-section inside the via pad disc (r ≤ 0.3 mm), on one of the last three segments, with the centre ahead and the hole (r_d = 0.15 mm) clear. From that line, current runs along rays to the hole wall with angular density j = (1/w)|q − c|²/p. Its energy is (ρ/t)(1/w²)/p·∫|q − c|² ln(|q − c|/r_d) ds, and the region is the via pad, star-shaped about the centre. The cut minimizing tube plus fan is taken.
- **Barrel.** The plating shell carries J = (0, p·r/A, q/A), divergence-free when (p, q) is, with energy ≤ (ρ/A)∫(q² + r_d²p²). The entry density is carried along straight lines to uniform over a depth d; over the depth this integrates to ln(x)/(x − 1), leaving one quadrature. The flow is uniform in the middle and un-spread to the leaving arc. A via is used once.
- **Plane.** Uniform barrel current is un-spread to uniform over |φ| ≤ β facing the other via, then sent along rays to the bisector, mirrored. It is admissible if the rhombus is clear of foreign vias' 0.51 mm clearance discs, of every other own-net via's drilled hole, and of the edge. Plane to plane through via B joins two sector flows in B's barrel above the plane, f1 + f2, priced at twice their energies (|a + b|² ≤ 2|a|² + 2|b|²).
- **The search.** Dijkstra chooses a path under these rules: no pass across a hole, no hop through a pad. `path_bound` then recomputes the path's bound from its own geometry, and a piece that fails is banned and the search repeated (up to 40 times). **Every number below is a recomputed path bound.**

**Checks** (`s4/check_exact.out`):
- the bar, the radial annulus and the uniform barrel equal their closed forms;
- 90° bends of three shapes lie inside certified primal/dual brackets, with the corner at 0.5577–0.5599 squares; the tube counts 1 square there and holds;
- across 300 random resistor networks, drop / (R_eff × total load) ≤ 1.000000, which is P4.8;
- the general fan and spread reduce to the symmetric closed forms at s0 = 0. The spread matches to 15 digits. The fan is lower, because it integrates the log term the first version only bounded.
- The pad-copper test agrees with a 720-point brute force on 3,000 random discs.
- **The plane bowtie against an exact case** (`s4/check_fan_plane.out`). Two equipotential holes in an infinite sheet have R = (ρ/πt)·acosh(D/2r_d) exactly. The bowtie's flow is admissible there, and its bound holds at every β from 2° to 75° and every D from 1 to 30 mm. It sits 2.7–78× above the exact value: the narrow sectors are the loose ones.
- **The general fan against a certified lower bound.** Take the copper of a track ending at a via, terminals at the far end and at the hole rim. A P1 primal on a grid domain that contains the copper, with the hole terminal grown, gives R ≥ 1/E. Tube plus fan holds above it at a straight entry and at 45° and 90° turns 0.125 mm before the centre: 1.11×, 1.16× and 1.20× the lower bound.
- **Stated conditions the bound still rests on,** none checked from the board:
  - the plane's fill is the declared plane: the outline less 0.51 mm discs around foreign vias, meeting each own-net via all round (P4.7 was blocked, D9);
  - each pad is an equipotential terminal;
  - the via pad is concentric with its hole;
  - the plating, copper and temperature figures in `conditions.json`.

**Conditions taken** (`s4/conditions.json`, each with its source):
- ρ20: IACS 1.7241 µΩ·cm (NBS Handbook 100, read);
- α: 0.00393 (read);
- copper temperature: 105 °C (a condition). The 658 mA load is DS12117's figure at Tj 125 °C, so the junction is allowed hotter than the copper. The architect may want both at 125 °C, which raises every R by 6 %;
- material allowance: k = 1 (a condition; every R scales by k);
- outer copper minimum: 30 µm (a condition);
- plane copper minimum: 15 µm (a condition);
- plating minimum: 18 µm (a condition);
- barrel lengths: from the declared stack.

**Measured** (`s4/paths.out`, recomputed paths only), per MCU supply pin, from its best stand-in source:

| Board | Pins bounded | Best stand-in, range over the bounded pins | Pairs bounded | Drop at 658 mA, worst bounded pin | Refused by name |
|---|---|---|---|---|---|
| Reference (2 layers, 0.2 mm tracks) | **7 of 16** | 9.1–125.4 mΩ | 45 of 208 | 83 mV of 165 mV | 9 pins from every stand-in: U1.32, and 39–121 behind vias and pads. 156 pairs have no path the flow rules admit; the rest fail at a pad end |
| 4-layer fixture (`In2` plane) | **15 of 16** | 8.1–116.1 mΩ | 89 of 208 | 76 mV | U1.108 from every stand-in |

- **These are `main`'s routes (`121344d`).** `1307` (PR #126, unmerged) changes every routed board: the reference goes from 70 to 64 vias, and from 348.5 to 331.6 mm of track (its mailbox note, `bae55aa`). The numbers above will move. The build re-measures after `1307` merges, as the work order's cadence already says. The method and its checks do not depend on the routes.
- **What limits coverage:** the closed-form pieces, not the copper. A via entered after a turn inside its pad, a pad too narrow for the tube, or a plane hop blocked by crowded foreign vias each refuse a path that real current would use. Where the pieces do fit, the plane hop dominates at 4 layers: 6–44 mΩ per hop against 2.05 mΩ for an ideal two-via sheet at 10 mm (not a bound).
- **The first version's table** (`s4/paths-v1-inadmissible.out`: 7.7–229 / 8.1–53 mΩ) **was not a bound.** On the reference, 120 of its 208 paths hopped through an intermediate pad at zero cost, and it never checked via entry or pad ends (F7).

### S5 — where the bounds go (proposal)

- **The `extract` verdict.**
  - `lines` stays, byte-identical for Sentinel-01.
  - Two sibling arrays are added, each entry carrying its terms, the declared limit and its own `conditions`:
    - `pairs`: per USB pair, `z_diff_milliohms_min`/`_max` if D2 rules the impedance check; per-line `c_bound_femtofarads` and the mismatch bound if it rules capacitance;
    - `supplies`: per domain, per MCU supply pin, `r_bound_microohms` with the path's terms.
  - A refusal names its piece: `PlaneFlowBlocked`, `TubeInadmissible`, `NoPlaneUnderPair`, `PairNotParallel`.
- **`verify`.** It gates each entry as it gates `lines`: the bound at most the declared limit, finding by name. Nothing passes on an absent limit; a board feature with no limit is refused by name (`INV-06`).
- **The limits, as new specification fields** (a schema version):
  - `usb` (from `4300e1`) carries the pair's nets and, by D2, either `max_capacitance_femtofarads` per conductor (25,000) with a mismatch permille (100), or an impedance window `[min, max]` in ohms;
  - the unused `net_constraints[].impedance_ohms` is removed, or ruled the window's centre;
  - each domain gains `max_path_milliohms`, or a `max_drop_millivolts` with the load taken from the parts table's cited IDD maximum (DS12117 Table 120). The source pad is the domain's regulator output from `4300d`.
- **The stackup gains the other side of every bound,** each with its `source`, minima rounded down and maxima up (`RUL-14`):
  - `relative_permittivity_min_permille`;
  - each dielectric's `max_thickness_um`;
  - `copper_min_um` per outer and inner layer;
  - `plating_min_um`;
  - `solder_mask` thickness max and permittivity max;
  - `etch_tolerance_um`;
  - `temperature_max_c`;
  - a resistivity allowance.

## Recommendation per question

| Q | Recommendation | Rests on |
|---|---|---|
| S1 | Three consumers, each with its direction and precision as tabled. Add a fourth, the ground return (D7). `4300e1`'s quantity for a full-speed device is C per conductor (≤ 25 pF) and its mismatch, not 90 Ω (D2) | USB 2.0 §7.1.6.1; DS12117 Tables 118–120; PR #129 |
| S2 | Nothing to build here: `4300c`'s ruling widens the line list. Arcs, inner tracks and non-rectangular pads stay refused by name until a board needs them | PR #129 ruling; `extract.rs`, `route.rs` reads |
| S3 | If any impedance check stays: the certified 2D bracket (D1 (a)), never closed forms. For full speed, check C instead (D2 (b)). Refuse at 2 layers | S3 A, B, exact and nested checks |
| S4 | Thomson's principle with an explicit flow, reported per pin, refused by name where no admissible flow is found. The closed-form pieces are rigorous but cover 7/16 and 15/16 pins on today's routes. For the build, a certified discrete flow (D5 (b)), checked against the closed forms where both apply | S4 table and checks |
| S5 | `pairs` and `supplies` beside `lines`, gated like `lines`. New limit and stackup fields, a schema version, refused by name when absent | `INV-06`, `RUL-14` |

## Decisions needed

- **D1 · How Z is bounded, if it is bounded at all.**
  - **(a)** The certified 2D bracket built here: primal P1 plus a stream-function dual, both variational, on a cross-section. It is a numerical solve, which §1's "not a field solver" excludes, but every number it gives is a proven bound. It holds Cohn's exact case, and its microstrip dual passes nested refinement. **Recommended if D2 keeps an impedance check.**
  - **(b)** Closed forms (IPC-2141, Wadell). They are not bounds: IPC-2141 falls outside the bracket at every point tried, by up to 8.6 %. 1305 F1's ruling ranks them as approximations.
  - **(c)** No Z bound; see D2 (b).
- **D2 · What 4300e1 checks for a full-speed device.**
  - **(a)** Z_diff in 90 Ω ±15 %, reading §7.1.6.1's 90 Ω sentence as binding the device. With today's declarable inputs no geometry fits: hi/lo ≥ 1.45 against 1.353 (S3 B).
  - **(b)** §7.1.6.1's device budget: each conductor's C ≤ 25 pF, and |C+ − C−| < 10 %. **Recommended for the 25 pF half,** which today's upper-bound machinery delivers once widened to the pair. **The mismatch half is not delivered today.** It needs an upper bound on one line against a lower bound on the other. The shell trial is 1.6–2.5× above the truth, and the only closed lower bound, parallel-plate flux under the track, is far below. A 10 % mismatch is provable only with D1 (a)'s bracket on the parallel run, plus a stated condition that the two lines see the same stackup. Or the mismatch is declared a condition of the router's symmetric routing (D8).
  - **(c)** Both.
- **D3 · The stackup's other sides** (S5's list) become specification fields, each with a source, or a stated condition. **Recommended: fields, with refusal when a gate needs one and it is absent.** The mask (εr and thickness), the εr minimum and the etch decide whether any Z interval can pass at all. Copper and plating minima and a temperature decide S4.
- **D4 · The supply bound's terminals and source.** **Recommended as measured:**
  - pads are equipotentials, and a pass through a pad is priced, not assumed;
  - the source pad is `4300d`'s regulator output, and until a board has one there is no supply gate;
  - the drop is per-pin R × the total declared load (P4.8), with the load cited from the parts table (DS12117 Table 120: 658 mA at Tj 125 °C).
- **D5 · How the flow is built.**
  - **(a)** The closed-form pieces measured here. Rigorous, but they cover 7 of 16 pins on the reference and 15 of 16 on the 4-layer fixture, and the plane hop is 3–20× the ideal.
  - **(b)** A certified discrete Thomson flow over the copper: per-layer sheets with holes, as S3's stream-function dual, joined by barrels. It never refuses connected copper and is tighter, but it is numerical.
  - **Recommended: (b) for the build, with (a)'s closed forms as its exact-case tests.** D1 (a) puts a numerical variational solve in the engine anyway.
- **D6 · Material allowance and temperature.** No citable maximum resistivity for electrodeposited foil or barrel plating was reachable (IPC-4562 is paywalled). **Recommended:** k and the copper temperature are declared conditions with their sources, and the build cites IPC-4562 Table 3-2 once someone holds a copy. Every R scales linearly in k and in 1 + α(T − 20).
- **D7 · The ground return.** The MCU sees VDD − VSS, so a supply gate that bounds only VDD under-claims. **Recommended:** the same bound on GND (zone or plane plus its stitching), with the drop limit applied to the sum.
- **D8 · Router scope** (`1307` is changing it).
  - No net has its own width: every supply track is 0.2 mm.
  - No pair is routed coupled, with a fixed gap.
  - Both a supply gate and a pair check need these. **Recommended:** a router work order after `1307`. `1303`'s build gates what the router produces and refuses what it cannot bound.
- **D9 · Container caps.**
  - With equal flags, podman 5.7 leaves `memory.swap.max` at 8 GiB (read back). The guard stopped the run, and nothing was written under `/sys/fs/cgroup`. The correction's premise does not hold on this podman. `4210` measured the same, independently (its mailbox note, `540ae8f`: 8589934592, steady over 40 s).
  - **Recommended:** P4.7 stays a stated condition (the plane meets each own-net via all round, and keeps 0.2005 mm from foreign vias, as `4100` measured) until a capped-swap route is ruled.
- **D10 · The work order's S4 says "the copper maximum".** An upper R bound needs the copper **minimum**; the maximum bounds the wrong way. Read as a slip; please confirm.

## Findings

| # | Finding | Where | Type | Decision-needed |
|---|---|---|---|---|
| F1 | **My first dual bound was not admissible.** It left the strip's end nodes free, so χ varied along the insulating edge beside each tip. Cohn's check caught it: lower bound above upper, and falling on refinement. The fix fixes every node of an insulating piece, end points included. The same trap waits for the build | `s3/bracket.py` | process / lesson | no |
| F2 | A narrow-strip barrel flow is rigorous but 10× loose (30 mΩ against 2.9 exact). Spreading to uniform brings it to about 2× | `s4/rbound.py` | design | no |
| F3 | 1305's shell trial is 2.52× the certified C for a line over a plane at 0.18 mm. Its air-case looseness is 1.63×, the trial's own (as `4100` measured). The other 1.55× is the shell taking εr 5.4 in the air above the board, where the bracket's C/C_air gives an effective 3.48. Fine for an upper bound, useless for Z | S3 C | measured | no |
| F4 | The delegated research pass could not resolve DS12117's IDD columns and guessed 550 mA at Tj 125 °C. Rendering pp. 215–216 shows Typ, then Max at Tj 25/85/105/125 °C, so 550 mA is at Tj 105 °C (Table 118) and the maximum is 658 mA at 125 °C (Table 120). Its other READ items were spot-checked where used | research record | process | no |
| F5 | §7.1.6.1's 90 Ω sentence and the device's 25 pF budget sit in adjacent paragraphs. Which binds a device is a reading of the spec, not a measurement | USB 2.0 | ruling | D2 |
| F6 | OSH Park's 4-layer lots sometimes use an EM528 core (Dk 4.11) instead of FR408HR. That does not touch the F.Cu–In1 prepreg, but a stackup source should name the lot risk | research record item 5 | data | D3 |
| F7 | **My first S4 table was not a bound.** It let a path hop through an intermediate pad at zero cost (120 of the reference's 208 paths did) and pass across a via's hole on one layer. It ignored own-net via holes in the plane and never checked via entry or pad ends. The advisor's review found it before the push. The second version (`rbound2.py`) recomputes every reported bound from its path's geometry; the first output is kept as `paths-v1-inadmissible.out` | `s4/` | process / lesson | no |
| F8 | On today's routes the closed-form pieces refuse many paths real current uses: a turn inside a via pad, narrow pads, crowded plane rhombi. Coverage, not looseness, is their limit | S4 | measured | D5 |

## What a build deletes

`spike/1303/` (`roadmap.md` §6). The tables above are the record; every measurement a build relies on returns as a Rust test.
