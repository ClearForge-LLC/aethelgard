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
| P3.4 | Cohn's exact stripline lies inside | **Inside at all 8 points** (4 geometries × 2 meshes): 0.9908–0.99997 below, 1.0014–1.0109 above. The first run failed it: see F1 | measured | right, after F1 |
| P3.5 | IPC-2141 lies outside by more than 3 % at some point | Outside at **all six** fixed geometries, by −8.6 % to +3.9 % | measured | right |
| P3.6 | W = S = 0.2 mm gives about 110–135 Ω | 115.1–115.5 Ω at h 0.2 mm, εr 3.61, t 43 µm. Near 90 Ω needs W ≈ 0.4 mm at s = 0.4 mm, or 0.3–0.35 mm at s 0.15–0.2 mm | measured | right |
| P3.7 | Tolerances alone give about ±10–13 % | **±22–33 %.** The bracket itself is under 0.5 % wide; the rest is input. The solder mask (−17 % if 50 µm at εr 5.4 is allowed), εr's IPC maximum (−15 %) and etch (±6 %) dominate (`s3/sens.out`). **No geometry fits 76.5–103.5 Ω** under the conditions taken | measured | wrong (too mild) |
| P3.8 | No rigorous interval at 2 layers | Agreed. The reference's `B.Cu` carries 55 tracks (108 mm, 30 of them `VDD_3V3`) through the ground zone, and the fill is never read | measured / inferred | right |
| P3.9 | Full speed is electrically short | TFR/TFF 4–20 ns (USB 2.0 Table 7-9, read), about 0.6 m of FR-4 at 4 ns. The reference outline is 45.2 × 40.4 mm | read / inferred | right |
| P4.1 | No regulator on either board | None: `VDD_3V3` has 29 pads on U1 (16), U2 (3), U3, U4, C3–C6, R1, R2, R4 and R5 | measured | right |
| P4.2 | Tube flow, R ≤ ρL/(Wt) under a length condition | Derived. The condition held on every path found (no tube narrowed) | measured | right |
| P4.3 | Every input is on the wrong side as declared | Confirmed. Copper minimum, plating minimum, temperature and a material allowance are all new | read | right |
| P4.4 | Exact checks hold | Bar, annulus and barrel equal their closed forms; the 90° bend lies inside a certified bracket (corner 0.558–0.560 squares against the tube's 1) | measured | right |
| P4.5 | Reference 50–300 mΩ from the best stand-in | **7.7–229 mΩ** per pin from its best stand-in; up to 481 mΩ from the worst | measured | range low end wrong |
| P4.6 | 4-layer 10–40 mΩ | **8.1–53.2 mΩ**; worst 152 mΩ. Two stand-ins are refused (an isolated via, below) | measured | close; high end wrong |
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
- **What the reader refuses,** and so the bound never meets [read, `extract.rs`]:
  - a top-level `arc`, `gr_poly`, `gr_line`, `gr_text` or any other item outside `TOP`, which covers copper arcs and graphics;
  - a pad with `drill`, which covers through-hole pads; any pad shape but `rect` or `roundrect`, so circle, oval and custom; a padstack or any child not in `allowed("pad")`;
  - copper in a footprint outside its pads;
  - a zone other than the declared ground or planes (`route::connectivity`), and a filled zone;
  - an inner-layer track, since `route::Layer` is `F` or `B`.

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
- **Exact:** Cohn's zero-thickness coupled stripline, odd mode, solved on the half-domain with the strip on the symmetry plane. It lies inside the bracket at every point (`s3/exact.out`).
- **Closed forms against the bracket at fixed geometry** (`s3/usb.out` A):
  - IPC-2141 (second-hand formula text; the standard is paywalled) is **outside at all six points**, by −8.6 % to +3.9 %.
  - No published coupled-microstrip table was reachable: Bryant & Weiss 1968 and Garg & Bahl are paywalled (research record items 3–4).
- **The interval over the 4-layer fixture's tolerances** (`s3/usb.out` B, s = 0.2 mm):

| W (mm) | εr taken | Z_diff interval (Ω) | inside 76.5–103.5? |
|---|---|---|---|
| 0.20 | 3.4–5.4 (5.4 the fixture's declared IPC maximum; 3.4 a stated condition, since no minimum is published) | 67.8–135.0 | no |
| 0.20 | 3.43–3.91 (±5 % about OSH Park's 3.61 at 1 GHz and Isola's 3.72 at 100 MHz) | 78.8–134.6 | no |
| 0.30 | 3.43–3.91 | 69.1–112.0 | no |
| 0.35 | 3.43–3.91 | 65.2–103.8 | no |
| 0.40 | 3.43–3.91 | 61.7–96.8 | no |

- **Why so wide** (`s3/sens.out`, one input at a time about W 0.3 mm, nominal 96.6 Ω):
  - h ±0.0202 mm: −3.9/+3.3 %;
  - εr ±5 %: ±2.5 %;
  - εr up to 5.4: −14.6 %;
  - t 30–70 µm: −4.9/+2.7 %;
  - etch ±25 µm: ±6 %;
  - **solder mask 0–50 µm at εr 5.4: −17.2 %** (at 25 µm and εr 4.0: −10.8 %).

  An exact solver given the same inputs would be just as wide, because the bracket is under 0.5 % wide. The gate is limited by what the stackup declares, not by the bound.
- **Two layers:** refused. No plane lies under the pair, and the `B.Cu` ground zone is unfilled in the file and cut by 55 tracks.
- **Full speed is electrically short.** The edge is 4–20 ns (Table 7-9), and the board is 45 × 40 mm. The quantity §7.1.6.1 gives a device is capacitance, which the existing machinery bounds from above (D2).

### S4 — supply-path resistance: a Thomson bound, measured

**The bound** (`spike/1303/s4/rbound.py`, module doc).
- **Terminals.** Each pad is one equipotential (its solder and lead), with unit current between two. Any divergence-free J confined to the copper gives R ≤ ∫ρ|J|²; deleting copper never lowers R (Rayleigh). So one path's flow bounds the whole net.
- **Track chain.** Uniform flow along the offset curves of the centreline. At a bend of deviation θ the inner offset loses 2y·tan(θ/2) and the outer gains yθ, so ∫ℓ(y)dy = LW + (θ − 2 tan(θ/2))W²/8 ≤ LW, and **R ≤ ρ·L_centre/(W·t_min)**. It needs, on each side of each segment, length ≥ (W/2)·Σ tan(θ/2) over the bends whose inside is on that side; otherwise the tube narrows. It held on every path.
- **Via, outer layer.** From the cross-section at x_e = 0.3 mm, rays go to the hole wall with angular density j = x_e/(W cos²φ), |φ| ≤ atan(W/2x_e). This is the closed form `fan`, 0.89 mΩ.
- **Barrel.** The plating shell carries J = (0, p·r/A, q/A), divergence-free when (p, q) is, with energy ≤ (ρ/A)∫(q² + r_d²p²). The entry density is carried along straight lines to uniform over a depth d, then back to the leaving arc. Over the depth, the integral is ln(x)/(x − 1) in closed form, leaving one smooth quadrature in φ. The result is 6.2 mΩ through the reference and 3.7 mΩ to the 4-layer `In2`, against 2.9 and 2.7 mΩ for exact uniform current. A first, unspread strip gave 30 mΩ.
- **Plane.** From uniform barrel current, un-spread to uniform over |φ| ≤ β facing the other via, then rays to the bisector, mirrored. That is 2(ρ/t)(1/2β)·ln(D/(2r_d cos β)) plus the un-spread. It is admissible only if the rhombus holds plane copper: no foreign via's 0.51 mm clearance disc, and no edge. The probe takes the cheapest clear β from 75° down to 2°.

**Checks** (`s4/check_exact.out`):
- the bar, the radial annulus and the uniform barrel equal their closed forms;
- 90° bends of three shapes lie inside certified primal/dual brackets, with the corner at 0.5577–0.5599 squares; the tube bound counts 1 square there and holds;
- across 300 random resistor networks, drop / (R_eff × total load) ≤ 1.000000, which is P4.8.

**Conditions taken** (`s4/conditions.json`, each with its source):
- ρ20: IACS 1.7241 µΩ·cm (NBS Handbook 100, read);
- α: 0.00393 (read);
- temperature: 105 °C (condition);
- material allowance: k = 1 (condition; IPC-4562's foil maximum was not reached, and no cited maximum exists for plated copper; every R scales by k);
- outer copper minimum: 30 µm (condition; OSH Park gives 43 µm nominal with no tolerance);
- plane copper minimum: 15 µm (condition);
- plating minimum: 18 µm (condition; JLCPCB states an 18 µm average [read]; IPC-6012 class 2 thin-area 18 µm is second-hand);
- barrel lengths: from the declared stack.

**Measured** (`s4/paths.out`), per MCU supply pin, from its best and worst stand-in source:

| Board | Best stand-in, over the 16 pins | Worst stand-in, over the 16 pins | Drop at 658 mA, best stand-in, worst pin | Refused |
|---|---|---|---|---|
| Reference (2 layers, 0.2 mm tracks) | 7.7–229.0 mΩ | up to 480.8 mΩ | 151 mV of 165 mV | none |
| 4-layer fixture (`In2` plane) | 8.1–53.2 mΩ | up to 152.4 mΩ | 35 mV | 2 of 13 stand-ins: their stitching vias' rhombi are all blocked |

- **Composition at 4 layers.**
  - Each plane hop costs 6–44 mΩ, depending on the clear β. The ideal two-via sheet resistance at D = 10 mm would be 2.05 mΩ (not a bound).
  - Crowded foreign vias block 182 of 300 rhombi.
  - The bound is about 3–20× the ideal there. It is the loose term.
- **These are `main`'s routes (`121344d`).** `1307` (PR #126, unmerged) changes every routed board: the reference goes from 70 to 64 vias, and from 348.5 to 331.6 mm of track (its mailbox note, `bae55aa`). The numbers above will move. The build re-measures after `1307` merges, as the work order's cadence already says. The method and its checks do not depend on the routes.
- **At 2 layers the tracks dominate.** The reference's 0.2 mm supply tracks, with up to 4 vias on a path, give bounds that alone eat most of the tolerance at the MCU's maximum load. The router has one width for every net.

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

## Decisions needed

- **D1 · How Z is bounded, if it is bounded at all.**
  - **(a)** The certified 2D bracket built here: primal P1 plus a stream-function dual, both variational, on a cross-section. It is a numerical solve, which §1's "not a field solver" excludes, but every number it gives is a proven bound, and it holds Cohn's exact case. **Recommended if D2 keeps the impedance check.**
  - **(b)** Closed forms (IPC-2141, Wadell). They are not bounds: IPC-2141 falls outside the bracket at every point tried, by up to 8.6 %. 1305 F1's ruling ranks them as approximations.
  - **(c)** No Z bound; see D2 (b).
- **D2 · What 4300e1 checks for a full-speed device.**
  - **(a)** Z_diff in 90 Ω ±15 %, reading §7.1.6.1's 90 Ω sentence as binding the device. With today's declarable inputs, no geometry fits (S3 B).
  - **(b)** §7.1.6.1's device budget: each conductor's C ≤ 25 pF, by today's machinery widened to the pair, and |C+ − C−| < 10 %. The mismatch needs a lower bound on C too: the parallel-plate flux under the track, or D1 (a) on the parallel run. **Recommended:** it is the clause written for a device, and the pair is electrically short.
  - **(c)** Both.
- **D3 · The stackup's other sides** (S5's list) are new specification fields, each with a source, or a stated condition. The mask, εr minimum and etch decide whether any Z interval can pass at all.
- **D4 · The supply bound's terminals and source.**
  - Pads are taken as equipotentials.
  - The source pad is `4300d`'s regulator output. Until then there is no supply gate, since no board has a source.
  - The drop is per-pin R × the total declared load (P4.8).
- **D5 · Plane flow.**
  - **(a)** The closed-form bowtie measured here. It is rigorous but 3–20× loose, and it refuses 2 of 13 stand-ins.
  - **(b)** A certified discrete Thomson flow in the plane sheet (the S3 dual, in 2D with holes). It is tighter, never refuses a connected plane, and is numerical.
  - (a) is enough for Sentinel-02 if its rail budget holds at 4 layers (35 mV of 165 mV here).
- **D6 · Material allowance and temperature.** No citable maximum resistivity for electrodeposited foil or barrel plating was reachable (IPC-4562 is paywalled). k and the copper temperature are conditions until one is cited. Every R scales linearly in k and in 1 + α(T − 20).
- **D7 · The ground return.** The MCU sees VDD − VSS, so a supply gate that bounds only VDD under-claims. The same bound applies to GND (the zone or plane, and its stitching).
- **D8 · Router scope** (`1307` is changing it).
  - No net has its own width: every supply track is 0.2 mm.
  - No pair is routed coupled, with a fixed gap.
  - Both a supply gate and any pair check need these. Whose work order owns them?
- **D9 · Container caps.**
  - With equal flags, podman 5.7 leaves `memory.swap.max` at 8 GiB (read back). The guard stopped the run, and nothing was written under `/sys/fs/cgroup`.
  - Either a capped-swap route is ruled (for example a furnace777 apply script, or a podman option that reaches `memory.swap.max = 0`), or P4.7 stays a stated condition: the plane meets each own-net via all round.
- **D10 · The work order's S4 says "the copper maximum".** An upper R bound needs the copper **minimum**; the maximum bounds the wrong way. Read as a slip; please confirm.

## Findings

| # | Finding | Where | Type | Decision-needed |
|---|---|---|---|---|
| F1 | **My first dual bound was not admissible.** It left the strip's end nodes free, so χ varied along the insulating edge beside each tip. Cohn's check caught it: lower bound above upper, and falling on refinement. The fix fixes every node of an insulating piece, end points included. The same trap waits for the build | `s3/bracket.py` | process / lesson | no |
| F2 | A narrow-strip barrel flow is rigorous but 10× loose (30 mΩ against 2.9 exact). Spreading to uniform brings it to about 2× | `s4/rbound.py` | design | no |
| F3 | 1305's shell trial is 2.5× above the certified C for a line over a plane at 0.18 mm, because it ignores the plane's screening of the far field. Fine for an upper bound, useless for Z | S3 C | measured | no |
| F4 | The delegated research pass misread DS12117's IDD columns, putting Typ in the Max columns; rendering the page settled it. Its other READ items were spot-checked where used | research record | process | no |
| F5 | §7.1.6.1's 90 Ω sentence and the device's 25 pF budget sit in adjacent paragraphs. Which binds a device is a reading of the spec, not a measurement | USB 2.0 | ruling | D2 |
| F6 | OSH Park's 4-layer lots sometimes use an EM528 core (Dk 4.11) instead of FR408HR. That does not touch the F.Cu–In1 prepreg, but a stackup source should name the lot risk | research record item 5 | data | D3 |

## What a build deletes

`spike/1303/` (`roadmap.md` §6). The tables above are the record; every measurement a build relies on returns as a Rust test.
