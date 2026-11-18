# AET-WO-1303b — FEEDBACK

Labels:
- **[measured]**: by a run named beside it.
- **[read at source]**: from a file and line named beside it.
- **[inferred]**: from reasoning, neither read nor run.

Part B (the measured stop) and Part A (the §1.1 stop) are kept below as ruled records. Section numbers inside each part refer to that part. Their evidence under `spike/1303b/` was deleted with this build, as ruled in S5, and stays in this branch's history at `81a8e619`.

## 0. Status: READY

Built to every ruling on #222:

| Comment | Ruling |
|---|---|
| 6968894352 | the §1.1 stop (Part A) |
| 6969258830 | Q1 and Q2 |
| 6971664955 | the measured stop (Part B) |
| 6973083189 | the load |
| 6974760231 | the fabrication tolerances |
| 6976355662 | the fixture conflict, (A) |

**The gate.** For each domain the specification gives a drop limit (`power.drop_limits`), `extract` judges:
- every MCU supply and return pin's certified resistance (D5 (b)), with every fabrication tolerance the specification declares priced;
- the path from the regulator's current-entry pad (a buck's inductor pad on the rail; an LDO's OUT) to its own ground pin (`PGND` by the power-parts pin map; U11.1 on the fixtures, SNVSAN3F Table 6-1, S2);
- the drop: (worst supply pin + worst return pin) × the domain's load. The load is `loads()`' total: the MCU's 658 mA, child regulators' inputs, and every declared `power.loads` entry.

With no limit, no verdict key moves (§2.3).

**Coverage, every tolerance priced [measured, §2.1]:**

| Board | VDD_3V3 | GND | Drop at the load | Against 165 mV |
|---|---|---|---|---|
| s024l (4 layers, GND 0.7 mm vias) | 16/16, 33.2–58.5 mΩ | 11/11, 39.0–45.9 mΩ | 74.8 mV at 716 mA | pass, 2.2× |
| Sentinel-03 as drawn (GND 0.6 mm vias) | 16/16, 33.0–56.6 mΩ | 10/10 refused, `SupplyViaPadTooSmall`, "via pad >= 0.7 mm" | none | refused by name |
| Sentinel-03 with GND on 0.7 mm vias (not committed, ruled (A)) | 16/16, 33.6–68.1 mΩ | 10/10, 37.5–41.9 mΩ | 82.3 mV at 748 mA | 2.0×, but `IMU_SCK` fails 31.27 > 30 pF |
| The power fixture (2 layers, GND 0.7 mm vias) | 16/16, 581–1547 mΩ | 11/11 refused, `GroundReturnUnread` (ruled (iii)) | none | refused by name |

No 4-layer margin falls under 1.5×, so no pitch note fires.

**Acceptance (WO §2):**
- Each bound has an exact-case test and a mutant a test kills (§4.2).
- The power fixture gets bounds per supply pin, and its return is refused per pin by name.
- Sentinel-01's bytes are unchanged, and so is every board without a regulator, except the pair stand-in's `pairs` key (D2, §2.3).
- A missing minimum or tolerance is refused by name, never defaulted.
- CI is green and `TIER2_MIN` is exact (§5).

**Correction, as the architect asked.** At the measured stop I gave the junction as 1.4–3.6 % of each bound; that was the concentric hole. With S1's worst-case junction, the hole-size and etch tolerances and the 62.5 µm pitch together, the worst 4-layer VDD + VSS rises:
- s024l: from 62.3 mΩ to 104.4 mΩ (+68 %);
- Sentinel-03 (GND on 0.7 mm vias): from 65.0 mΩ to 110.0 mΩ.

I did not measure each lever's share separately; an earlier "25–35 %" for S1 alone was not measured on this head and is withdrawn.

## 1. What is built

### 1.1 `crates/aethelgard-eda/src/flow.rs`

The certified discrete Thomson flow is described in Part B §1. This build adds the worst case over the hole.

- **Position.** `junction_table` partitions the ±`hole_position_um` offset into 16 × 16 boxes. Over each box it bounds each face's `κ`, and records each box's own barrel widths.
- **Size.** The shell radius runs from the smallest finished hole to the largest, each plus the plating minimum. It is cut into 4 slices (`HOLE_SIZE_SLICES`). Each slice's table becomes further boxes (`JunctionTable::merge`), so `Element::Group` prices a via's joints and barrel cells together at the worst (size, position) box.
- **Junction coefficients as series.** The band coefficients and the plating term were differences of terms of order `r_d/β`, with values of order `β/r_d`. Over an interval of radii those differences lost everything to dependency; Sentinel-03 hit "division by an interval holding 0". They are now the exact series in `x = β/r_d` and `u = 1 − (1 − t_p/r_d)²`, derived beside the code:
  - `c_ff = Σ(−1)^{k+1} x^k/(k+2)`
  - `c_fg = Σ(−1)^{k+1} 2x^k/((k+1)(k+2))`
  - `c_gg = Σ(−1)^{k+1} 2x^k/(k(k+1)(k+2))`
  - `I/A² = Σ u^{m−2}/(m(m−1)(m−2))`

  Each tail is bounded. `r_d` enters once, so an interval stays tight. Tests:
  - the quadrature match within 0.05 % at a point is unchanged;
  - over a quarter of the size range, the interval encloses 17 sampled sizes and its upper end is within 2 % of their largest.
- **`Network::check`** refuses a node with edges but neither an element nor a terminal's role (the adversarial pass, L3).

### 1.2 `crates/aethelgard-eda/src/supply.rs`

**Engine constants**, documented beside the flow (S4): `PITCH_NM` 62.5 µm, `CROP_NM` 5 mm, `BETA_NM` 10 µm, `FILL_CLEARANCE_NM`, `OPENING_NM`, `EDGE_NM`, `ARC_NM`, `HOLE_SIZE_SLICES`. The hole position tolerance moved out of the engine, into the specification, as ruled.

**Hole size (ruling (1), option (a)):**
- The barrel is priced at the smallest shell (`barrel_hole`). Its coefficient `ρr/A(r)` falls as `r` grows, by `−t/(2r − t)²`.
- The junction's fit is checked at the largest shell plus the position's reach.
- Holes kept clear of the net's copper use the largest finished size (`hole_clear`).
- **No plating maximum is needed.** At least the minimum plating lines the finished wall, and the layer's copper meets it, so a shell at the finished radius plus the minimum is copper at every size. My bus note that VDD failed at 19 µm was wrong.

**A via that cannot fit** is left out. A pin it leaves unreachable is refused, `SupplyViaPadTooSmall`, and the message names the pad that would fit (`pad_needed`, rounded up to 10 µm). The message states that the gate left those vias out. It does not claim that they alone cut the path.

**Etch (ruling (2)).** Tracks are priced at `(1000 − track_width_permille)/1000` of their drawn width, rounded down. Pads and via pads at drawn size are a named condition.

**Plane ports, face by face.** On a plane a face is a port when every cell its column's hull meets is plane copper (`Region::hull_inside`). Before, a via joined a plane only if its whole junction region was plane copper.
- **Why it changed:** pricing the largest hole grew the regions, and Sentinel-03 lost 10 of 16 VDD pins and 4 of 10 GND pins: 33 VDD vias without a plane join, against 19 before [measured, a temporary probe].
- **Why it is sound:** each face's rays, its band sector and its plating column lie in the triangle from the hole's centre to the face. That triangle lies inside the hull of the face and the centre square, so a column whose hull is copper is copper wherever it carries current. The outer layers already used this test.
- **Test:** a foreign knockout reaching into one side of a region costs only that side's faces.

**The adversarial pass's guards (§4.1):**
- `SupplyTerminalMissing`;
- witness disc clearance;
- port hulls clear of holes;
- notes in the verdict;
- a non-finite bound refused;
- a hole that may finish at or below zero left out;
- the declared dielectric maximum bounding a barrel's row.

### 1.3 `crates/aethelgard-cli/src/extract.rs`

- **`supplies`** sits beside `lines`, with: verdict, resistance, source, return, load, limit and its source, the drop, each pin and return, the conditions, the pitch note, and the copper model's notes. It is absent without a limit.
- **`pairs` (D2).**
- **Findings:** `supply_pin_unbounded` and `supply_drop_over_limit`. `supply_drop_unbounded` is belt-and-suspenders and labelled so.

### 1.4 Specification fields

All optional, with no schema bump (ruled):

| Field | Validation |
|---|---|
| `board.stackup.outer_copper_min_um`, `plane_copper_min_um` | at most `copper_max_um` |
| `plating_min_um`, `copper_temperature_max_c` | |
| `Dielectric.max_thickness_um` | |
| `board.stackup.fab_tolerances`: `hole_position_um`, `hole_size_plus_um`, `hole_size_minus_um` | 0–1000 µm |
| `fab_tolerances.track_width_permille` | 0–999 |
| `power.drop_limits` | |
| `power.loads[].reference` | |

Each entry is `{value, source}`, and is refused by name when the gate reads it absent.

**Deviation, for the record.** The ruling named `track_width_fraction`. The IR holds integers only (`Sourced.value: u32`, `Eq`) and writes fractions per mille (`relative_permittivity_max_permille`), so the field is `track_width_permille`: 200 means ±20 %.

### 1.5 Fixtures (bytes that move)

**Both power fixtures' VDD_3V3 load entries** name every other sink, each cited, derived or declared (6973083189). Sentinel-03 has 90 mA and the power fixture 37 mA. Two hand-computed input-current goldens moved by exactly +37 mA.

**Both declare JLCPCB's tolerances** (6974760231). The sources cite jlcpcb.com/capabilities/pcb-capabilities, read 18 Nov 2026: Hole Position Tolerance "+/-0.075 mm"; Hole size Tolerance "Through-holes: +0.13 / -0.08 mm"; Track width tolerance "+/-20%". A source must be printable ASCII, so "±" is written "+/-".

**GND's via class:**
- **The power fixture puts GND in `SUPPLY_MCU`** (0.7/0.3 mm vias, 0.2 mm tracks, as the default), as the `-gnd` variant was measured. Its board's GND vias move. Its `extract` still passes, every line under its bound [measured]. `power_board_tier1`'s class list gains `GND`.
- **Sentinel-03 keeps GND on 0.6/0.3** (ruled (A)). No byte of its board moves.

**Sentinel-01 is untouched.**

## 2. Measured at the gate

### 2.1 Coverage [measured, `~/.cache/aet-1303b/cov/run.py`]

The run used HEAD's release binary at a 165 mV limit, on the power fixture as the CLI tests take it (`tests/common/power.rs`) and on s024l (the same at Sentinel-03's four layers and stackup, as tier 1 builds it). The table is in §0. Each board takes 3–4 min in release (9 min 19 s for three). The supply tier-1 file takes 4 min 54 s on this host.

**The pitch and the etch [measured, unit probe].** At 62.5 µm, a 45° 0.2 mm track etched to 0.16 mm no longer holds a face-connected chain of cells into a 0.7 mm via, and the path disconnects. A face-adjacent pair needs a strip about 0.133 mm wide, and alignment does the rest. No corpus pin is lost to it today: Sentinel-03's coverage is the same with the etch at 0. It is the S4 finer-pitch lever, in the §7.1 row.

### 2.2 The solver (S4's timing, kept here as ruled)

The potential solve is preconditioned by aggregation multigrid. A 1.76M-cell sheet solves in 7 s, against 400 s with Jacobi [measured, the removed timing test]. Nothing it computes enters the bound except as a proposal.

### 2.3 Byte-identity [measured, `~/.cache/aet-1303b/cmp/run.py`, main's binary against HEAD's on every committed specification]

25 committed specifications. Of the 23 without a regulator, 18 emit, and 17 of them have **board and `extract` verdict byte-identical** to main's, Sentinel-01 included (`examples/sentinel-01.json`, `examples/sentinel-01-minimal.json`, `sentinel-01-bench.json`). The exceptions:
- `pair-standin.json`: its board is identical, and its verdict gains `pairs` (key added, nothing else changed), D2 as ruled.
- 5 refuse at `emit` on both binaries alike: three solver fixtures meant to, `imu-baro-1v8.json` and `invalid-zero-domain.json`.

The two specifications with a regulator gain fields main's binary refuses as unknown, so each was emitted from its own branch's file:
- **Sentinel-03:** board **identical** (sha256 `7f259f04a192…` both).
- **The power fixture:** board differs (GND on 0.7 mm vias, ruled); its `extract` passes.

## 3. Rulings applied after the measured stop

- **The load (6973083189):** option 1, as §1.5 describes.
- **The tolerances (6974760231):** (1) (a) and (2) as recommended, §1.2. (b) and (c) were refused.
- **The fixture conflict (6976355662):** (A), §1.5.
  - With GND on 0.7 mm vias, Sentinel-03's routing moves and `IMU_SCK` measures 31.27 pF against its declared 30 pF (main: 20.43 pF; its vias 8.57 pF).
  - Sentinel-03 keeps 0.6, and its refusal is asserted through `extract` in tier 1. Its own specification declares no drop limit, so `verify` does not fail on it.
  - The priced 4-layer drop is gated on s024l in tier 1.
- **The pair stand-in.** It is the only committed specification with `diff_pairs`, and it has no regulator. Its `extract` verdict gains `pairs`, which is D2's delivery, as ruled at §1.1.

## 4. Adversarial pass, mutation, reduction

### 4.1 Adversarial pass (full, WO §5)

A fresh subagent ran it; I checked each finding.

**Fixed:**
- **H1.** A limit on a domain the load has no pin on judged nothing and passed. It is now refused, `SupplyTerminalMissing` (test). A `None` drop with no refused pin is a finding (belt-and-suspenders).
- **L1.** The witness disc's clearance part used the cover radius. It now uses the opening plus the arc error.
- **L2.** A port's ray hull is checked against holes (belt-and-suspenders: the outside cell's own hole clearance and DRC's hole-to-hole spacing keep holes out of every hull, §4.2).
- **L3.** `check` refuses a node with edges and no element.
- **L4.** The copper model's notes are in the verdict.
- **L5.** A non-finite bound refuses its pin. µΩ is rounded up past the product's rounding.
- **L6.** The declared dielectric maximum is plumbed into the barrel's rows. The atan documentation is corrected.
- **M1 (hole size) and M2 (etch):** raised, ruled (6974760231) and built.
- **M3 (`pairs` on `diff_pairs`):** §3.

**The WO's four classes:**
- **A constant from the wrong side.** `rho_max` takes IPC-MF-150F's foil maximum at the declared temperature. Hole size, position and etch are each priced at their worst.
- **A nominal used as a minimum.** The tests and coverage declare `plating_min_um` 18 µm, JLCPCB's stated average. A specification declares its own minimum with its source, and the gate is honest to it. The test figures rest on an average and are labelled so here.
- **A pad assumed and not priced.** Terminals are equipotential pads, a stated condition. A relieved pad joins no plane (left out, which only raises R). Pads at drawn size are a named condition.
- **A gate passing with a field absent.** Each of the eight inputs is refused by name, with a test each, and so is the sink list.
- **Copper the reader cannot bound.** It is refused per pin (`SupplyPinUnbounded`, `SupplyViaPadTooSmall`, `GroundReturnUnread`), each with a finding.
- **Sentinel-01's bytes:** §2.3.

### 4.2 Mutation [measured, `~/.cache/aet-1303b/mut/run.py`, one mutant per guard, run against its tests]

44 mutants, one per guard this order adds or changes, each run against the suites the guard lives in. **43 killed.** The survivor is the port hull's hole test (L2), labelled belt-and-suspenders at its site.

**The survivor's label first gave the wrong reason** ("DRC spacing keeps a hole out of pad copper": false for a hole of the same net). The true reason, measured after READY at `6d370e29`:
- a port's outside cell is already clear of every hole, so a hole crossing its hull would have to lie inside the junction region;
- DRC's hole-to-hole spacing forbids that;
- even a same-net pad hole 0.41 mm off, closer than DRC allows, is crossed by no port (test `a_port_whose_rays_cross_a_hole_of_the_net_is_not_joined`, a property test; the mutant still survives it).

Guards that first survived, each given a test that now kills it:
- the network's foreign-face check;
- a barrel's worst-corner pricing;
- the outer port's copper hull;
- a part named twice as a load;
- the plating, dielectric-maximum and finite-bound guards;
- the hole-size guard (a tolerance larger than the hole);
- the barrel at the smallest shell (`barrel_hole`, whose effect is about 2 % of a path and invisible to path tests);
- `β ≥ r_d`.

Not mutated: the two series' tail terms. They are about 1e-40 here, below any f64 a test can see; they are kept for rigour.

### 4.3 Reduction pass (diff)

A fresh subagent ran it; I checked each claim by grep before acting.

Reduction pass (diff): −227 net lines in the reduction commit (+64 −291) · 1 duplicate removed (`overlaps128` into `flow::overlaps`) · 7 dead items removed (the example driver, S5; `Iv::hull`, `Iv::clamp`, the concentric `junction` wrapper, `Bound::iterations`/`residual`, `Model::knockout`, the `named` helper) · 3 protected kept (`supply_drop_unbounded`, the port hull's hole test, `PinBound::via_transfer` which a test reads) · 2 findings below.

After the tolerance ruling, one fact written twice (the cleared hole's radius, at two sites) became one function, `hole_clear`.

**Findings:**
- The `conditions()` strings quote `rho_max`'s constants: one is prose for the verdict, the other arithmetic.
- `dielectric_max_nm` is a vector parallel to `dielectrics`. Folding it into the tuple touches `fab.rs`, outside the fence.

## 5. Gates

Local, on this head [measured]:
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`: clean. `cargo doc` raises no warning in `flow.rs` or `supply.rs`; `extract.rs`'s two `CONDITIONS` links predate this branch.
- `cargo test --workspace`: green (exit 0).
- CI's package step: every publishable crate packages (201 files, 3.1 MiB).

CI on the PR, head `bf6f6ae6`: run 44298800920 green (gates and every tools shard). The tier-2 canary aggregated `tier2_tests_passed=191 floor=191`. The engine and ArduPilot image runs (44298800883, 44298800885) are green too [measured].

`TIER2_MIN` stays 191: this order adds no tier-2 test. Tier 2 asserts nothing on `extract` for the changed fixtures (checked: `with_tools.rs` reads the pair stand-in's DRC only).

## 6. §7.1 rows (for the architect to apply)

| Item | From | Owner |
|---|---|---|
| A through-hole barrel priced as a via's; a relieved pad refused until its spokes are priced | 1309 D2 | **done, `1303b`:** an on-grid through-hole pad is a via at its drill. A relieved pad joins no plane and is left out with a note. Leaving it out only raises R, so no refusal is needed |
| A cited current-to-width source for power classes | 4300d2 K11, A7 | **re-owned:** the drop gate bounds resistance, not heating. An IPC-2152 heating gate is its own quantity. Trigger: Sentinel-02 (`4500`) |
| A stackup bound below the fabricator's minimum copper | review 2, fuzz | **partly done, `1303b`:** each declared minimum is at least 1 µm and at most `copper_max_um`. A floor for `copper_max_um` itself waits on a cited minimum: the IPC-6012 row (1303 D6) |
| Every pin a flat 10 pF in extraction | Grok review 2026-11-16 | **re-owned:** outside `1303b`'s list. `4300e1` (USB's static checks read the pair C) |
| The certified discrete Thomson flow (1303 D5 (b)) | 1308 B4 | **done, `1303b`** |
| A 2-layer board needing the supply gate must widen its supply tracks or carry a plane; measured at `1303b` S3 | 1303b S3 | trigger: a 2-layer board needs the gate |
| A finer pitch with one factorisation per net, or a larger via pad: the levers when a bound nears its limit. At 62.5 µm a 45° 0.2 mm track etched 20 % disconnects (§2.1) | 1303b S4 | trigger: a board within 1.5× of its limit, or a pin lost to an etched diagonal |
| A 2-layer ground return read from the filled zone (option (i)) instead of refused | 1303b ruling (iii) | trigger: a 2-layer board needs the gate |
| Other domains' return currents in the ground copper | 1303b conditions | trigger: a board with a second regulated domain under a drop limit |
| The default 0.6/0.3 mm via cannot keep its band at JLCPCB's worst hole size and position (needs 0.7 mm; a 0.554/0.254 mm thermal via needs 0.66 mm): a manufacturability finding for every board | 1303b, ruling 6974760231 | the next router work order |
| A cited pad etch tolerance (pads and via pads are taken at drawn size, a named condition) | 1303b, ruling 6974760231 | trigger: a fabricator states one |
| Sentinel-03's supply gate needs a ground via pad of at least 0.7 mm. At 0.7 its routing moves and `IMU_SCK`'s bound exceeds 30 pF (31.27 pF, measured at `1303b`): vias are not costed on capacitance-bounded nets (`1313` Q3) | 1303b, ruling 6976355662 | the next router work order, beside the default-via-pad row |

## 7. Record

- **Branch** `wo/aet-1303b-extraction-build`, PR #222, main `c4a6cc6f` merged in.
- **Evidence (Furnace):**
  - `~/.cache/aet-1303b/cov/` (coverage: `run.py`, `out3/`, `run3.log`);
  - `~/.cache/aet-1303b/s03drawn/` (Sentinel-03 as drawn, gated);
  - `~/.cache/aet-1303b/s03new/` (the GND 0.7 and tolerance-split diagnostics);
  - `~/.cache/aet-1303b/cmp/` (byte identity, `result.txt`);
  - `~/.cache/aet-1303b/mut/` (`run.py`, `final.log`).
- **The measured stop's evidence** (`spike/1303b/`) is deleted with this build; it is at `81a8e619`.

### Lessons

- **A closed form that is exact at a point can be useless over an interval.** The junction's band terms were differences of order-`p` terms; rewriting them as their series cost nothing at a point and made an interval of hole sizes tight.
- **An all-or-nothing admission rule can hide behind a tolerance.** The plane rule held while the regions were small, and failed the moment pricing the largest hole grew them. A probe counting refused joins found it in one run.
- **Measure a ruled fixture change before building on it.** GND on 0.7 mm vias moved Sentinel-03's routing and failed a gate the ruling did not mention.

# Part B: the measured stop after D5 (b) (record, ruled in comment 6971664955)

## B0. Status: STOPPED at the ruled measured stop (D5 (b))

The §1.1 stop was ruled on #222 (comment 6968894352, Part A below), and Q1 and Q2 were answered in comment 6969258830. The ruling asked for one measured stop "before any gate or field": coverage and the per-pin bound on sentinel-02-power, s024l and sentinel-03, with the VDD + VSS drop at the load. This is that stop.

- **D5 (b) is built** (`crates/aethelgard-eda/src/flow.rs`, §1): the certified discrete Thomson flow as Q2 accepted it. RT0 face fluxes on square cells wholly inside copper, made exactly divergence-free by integer quanta along a spanning tree, with the energy evaluated in outward-rounded intervals.
  - **The exact-case tests hold.** Bar and barrel give exactly their closed forms. Annulus, bounded two-hole sheet (with the refinement series) and a 45° track are above exact and converge. The bend is above `1303`'s certified bracket. The junction coefficient is within 0.1 % of a direct quadrature of its own field. A via in a disc is above its shorted floor. **No case is ever below exact.**
- **Coverage: every pin on every board [measured, §2.2].**

  | Board | VDD_3V3 | GND |
  |---|---|---|
  | sentinel-02-power (2 layers) | 16 of 16, 334–882 mΩ | refused by name, `GroundReturnUnread`, as ruled |
  | s024l (4 layers) | 16 of 16, 19.5–31.2 mΩ | 11 of 11, 21.7–31.1 mΩ |
  | sentinel-03 | 16 of 16, 18.1–34.2 mΩ | 10 of 10, 20.4–30.8 mΩ |

  Part A's closed forms (D5 (a)) covered 0, 10 and 6 of 16 VDD pins, at 142–892 mΩ.
- **The 4-layer drop is well under 165 mV [measured, §2.3].**
  - Worst VDD plus worst GND pin: 62.3 mΩ on s024l and 65.0 mΩ on sentinel-03.
  - That is 41.0 mV and 42.8 mV at the MCU's 658 mA. 165 mV holds to 2.65 A and 2.54 A, against the net's total load, which the fixtures will sum (ruling 5).
  - The ruling's "over 165 mV at 4 layers" branch does not fire.
- **The 2-layer VDD alone is over 165 mV** at 658 mA: 220–580 mV.
  - The labelled reference (§2.4) puts the copper itself at 206–544 mV, under the stated conditions: 30 µm, 125 °C, k, and the whole load through the worst pin.
  - Whether that is a design finding is the architect's call (§3, S3).
- **The copper the bound uses is copper [measured, §2.5].**
  - Every plane cell was checked against KiCad 10.0.7's own refill, in the pinned image.
  - Every outer cell and every via junction square was checked against an independent reading of the board, with drills subtracted.
  - All pass, except 8 single plane cells that are islands in the model as well. They carry no flux.
- **Q1:** every 4-layer bound rests on two via junctions, one near the source and one near the pin. The junctions carry 1.4–3.6 % of the bound.
  - A junction square fits outside the ±0.075 mm hole with a via pad of 0.80 mm at β 0.03 mm, or 0.71 mm at β 0.01 mm (§2.6).
  - The pads are 0.7 mm (VDD class) and 0.6 mm (GND default) today.
- **Not built, as ruled:** no gate, no field, no CLI subcommand. The measurement is an example binary (`crates/aethelgard-cli/examples/supply_stop.rs`).
- **For the ruling (§3):** S1 the Q1 choice; S2 the ground terminal's reading; S3 the 2-layer VDD; S4 the gate's parameters (pitch, crop, time); S5 the fence.

## B1. What is built

**`flow.rs`: the certified flow.** Its module documentation states the guarantee and how it is kept.
- **Elements:**
  - RT0 sheet cells. The energy `(ρ/t)[(a/b)(F_W² + F_W F_E + F_E²) + (b/a)(F_S² + F_S F_N + F_N²)]/3` is exact in the face fluxes.
  - Barrel cells: the plating unrolled. The `(ρ/A)∫(q² + r_d² p²)` shell bound of `1303` S4 is a sheet of `ρ r_d/A`, and a row whose height is a range is taken at its worse end.
  - Via junctions: per grid face of a square around the hole, `κ·F²`. Three regions, each written out:
    - rays to `r_d + β`;
    - a band that makes the arrival uniform per barrel column;
    - radial injection into the plating.

    Each is divergence-free and meets the next with equal normal flux, and each energy is in closed form.
- **Exactness:**
  - A two-point potential solve proposes the fluxes: conjugate gradients with an aggregation-multigrid preconditioner, capped in iterations.
  - The fluxes are rounded to integer quanta of `2⁻⁴⁰` A, and the residue is carried to the sink along a breadth-first spanning tree. Conservation is re-checked in `i128` at every node.
  - Before any flux, `Network::check` refuses a network whose elements do not price every edge at each end, or that joins two terminals directly.
- **Rounding:**
  - Interval arithmetic is widened one unit outward per operation.
  - `ln` and `atan` are own series with their remainders as intervals.
  - Quadrature is enclosed panel by panel.
  - Inputs that are not exact in binary enter as intervals.
  - The bound is the upper end. Nothing is rounded down.
- **Q2's three conditions, each tested:**
  - **Cells wholly inside copper.** `supply.rs` tests every cell exactly in integers, and §2.5 checks it against KiCad and an independent reader.
  - **The energy exact in the fluxes**, in intervals.
  - **The (a) pieces as exact cases:**

| Case (test in `flow.rs`) | Exact | (b) |
|---|---|---|
| bar 40 × 6 | 6.6667 squares | equal, to 1e-9 |
| barrel, 8 rows × 48 columns | ρH/(π t_p(2r_d − t_p)) | equal, to 1e-9; a row known only in a range is above |
| annulus 1–4, h = 1/8, 1/16, 1/32 | ln 4/2π = 0.220636 | 0.238358, 0.229001, 0.224472 |
| two-hole sheet (below), h = 1/8 … 1/128 | 2 acosh 2/π = 0.838401 | 1.001111, 0.909196, 0.870659, 0.854089, 0.846116 (+0.92 %) |
| 90° bend, arms 24, width 8 | 6.5577–6.5599 (`1303`'s bracket) | 6.5696 |
| 0.2 mm track at 45°, h = 31.25, 15.625 µm | 21.2132 squares | 30.3704 (×1.43), 23.5625 (×1.11) |
| junction coefficient, 3 faces | direct quadrature of its own field | +0.01 % to +0.05 % above it |
| via in a 2 mm disc, barrel 1.6 mm | shorted floor 114.0759 | 114.9660 |

- **The bounded two-hole sheet:**
  - The disc of radius √3 about the midpoint of two unit holes 4 apart, less the holes.
  - The disc meets both hole circles at right angles: it is a flux line of the infinite sheet's field, in bipolar coordinates. So its resistance is exactly 2 acosh(2)/π squares.
  - The series halves its excess at each refinement (first order). The two finest give 0.838143, against 0.838401 exact.
- **Curved electrodes:** a cell wholly inside an electrode never shares a face with a cell wholly in the copper. So a cell inside (copper ∪ electrode) counts as resistive copper, which can only raise the bound.

**`supply.rs`: one net's copper as a network.** Its module documentation names every inner approximation and stated condition.
- **On F.Cu and B.Cu,** a cell counts when it lies wholly inside one track, via pad or pad, or when its four or sixteen sub-squares each do, and it is clear of every own-net drill. The geometry is exact in integers.
- **On an own-net plane,** the outline is less 0.625 mm and less each knockout: d/2 + 0.2025 mm, rounded up to 10 µm, plus 0.125 mm. This is P4.7's measured clearance and F9's opening. It applies to foreign vias and to every drilled pad (own-net ones too: the join is not priced). Plane copper beyond 5 mm of the terminals' box is left out.
- **Every own-net via on the router's grid** is a junction square on each layer that holds it wholly, and a barrel through the stack:
  - outer rows: 30 µm to `copper_max_um`;
  - plane rows: 15 µm to `copper_max_um`;
  - each dielectric row: its declared minimum to the finished maximum less every other row's minimum.

  A via off the grid is left out, and its hole is kept clear: here, the 23 thermal vias under U11's pad, on a 0.1 mm grid.
- **Terminals** are pads taken as equipotentials: L2.2 for VDD, U11.1 for GND, and each U1 pad on the net.

**What it does not have yet:** unit tests of its own. Its geometry is checked externally (§2.5), not by tests in the crate (S5).

## B2. The measured stop [measured]

### B2.1 Runs

- **Boards:** Part A's (`emit` of `main` `002dae07`; s024l built by `spike/1303b/build-fixtures.py`).
- **Driver:** `crates/aethelgard-cli/examples/supply_stop.rs`, run by `spike/1303b/b/run.sh` at 62.5 and 31.25 µm with a 5 mm crop. The 2-layer board was also run at 15.625 µm.
- **Conditions:**
  - ρ_max = IACS 1.7241 µΩ·cm × (1 + 0.00393 × 105) × k 1.181 = 2.8764 × 10⁻⁵ Ω·mm (125 °C), k on plated copper being a stated condition;
  - outer copper ≥ 30 µm, plane copper ≥ 15 µm, plating ≥ 18 µm, all conditions as in Part A;
  - holes concentric (Q1, ruled for this stop).
- **Pitch:** a cell of 31.25 µm puts every via centre (on the 125 µm grid) on a cell corner, and its junction square has half-side 187.5 µm, with β = 30 µm.

**The ground terminal [read at source].** LMR33630 datasheet SNVSAN3F, Rev. F, Nov 2020 (sha256 `3b0920a4…c04a` as fetched):
- **Table 6-1, p. 5:** the HSOIC's thermal pad row reads "AGND … For the HSOIC package, the pad on the bottom of the device serves as both the AGND connection and a thermal connection to the heat sink ground plane". PGND is pin 1: "Power ground terminal".
- **Absolute maximum ratings, p. 6:** "AGND to PGND −0.3 0.3" V.
- **§10.1, p. 34:** "PGND pins are connected directly to the source of the low side MOSFET switch".

The datasheet does not say the pad connects to PGND inside; it names it a different node. The terminal is therefore U11.1 alone, and U11.9 is resistive copper (S2).

### B2.2 Per pin (`b/summary.out`, from `b/*.out`)

| Board, net | pins | 62.5 µm | 31.25 µm | 15.625 µm |
|---|---|---|---|---|
| sentinel-02-power VDD_3V3 | 16/16 | 497–1317 mΩ | 335–887 mΩ | 334–882 mΩ |
| s024l VDD_3V3 | 16/16 | 22.4–39.3 | 19.5–31.2 | |
| s024l GND | 11/11 | 28.1–41.5 | 21.7–31.1 | |
| sentinel-03 VDD_3V3 | 16/16 | 20.9–44.4 | 18.1–34.2 | |
| sentinel-03 GND | 10/10 | 26.8–41.5 | 20.4–30.8 | |

- **Vias:** every 4-layer pin's flow uses two vias, one near the source and one near the pin. Each carries all the current; no other via carries 1 %. The 2-layer pins use 4–16.
- **Crop:** 15 mm instead of 5 mm lowers the 62.5 µm bounds by 2.9–7.1 % (s024l VDD) and 1.7–4.8 % (sentinel-03 GND). Less plane only raises a bound.
- **The crop is built from the terminals named.** A pin run alone (`ref-*.out`) gets a smaller box than the same pin run with all its net's pins: U1.17 reads 32.378 mΩ alone and 31.216 mΩ with the rest. Both are bounds; the verdict must fix one rule (S4).
- **Shares** (`b/shares-31250.out`):

  | | Sheets | Junctions | Barrels |
  |---|---|---|---|
  | 4-layer worst and best pins | 9.1–27.2 mΩ | 0.43–0.73 mΩ | 4.0–8.6 mΩ |
  | 2-layer pins | 309–818 mΩ | 1.2–3.1 mΩ | 25–66 mΩ |

  - The VDD barrels run F.Cu to In2 through rows each priced at its own maximum: 2.30 mm summed, against the 1.851 mm finished maximum. The optional `dielectrics[].max_thickness_um` (ruling 4) would tighten that.

### B2.3 The drop

The drop is (worst VDD pin + worst GND pin) × load. The MCU ties its VDD and VSS inside, so the worst pair bounds it [inferred, as ruled]. The load here is the MCU's 658 mA (DS12117 Table 120) alone; the fixtures' `power.loads` VDD_3V3 entry will sum every sink (ruling 5).

| Board | bound | at 658 mA | 165 mV holds to |
|---|---|---|---|
| s024l | 31.2 + 31.1 = 62.3 mΩ | 41.0 mV | 2.65 A |
| sentinel-03 | 34.2 + 30.8 = 65.0 mΩ | 42.8 mV | 2.54 A |
| sentinel-02-power | VDD alone 334–882 mΩ; GND refused by name | 220–580 mV | — |

### B2.4 The reference (UNCERTIFIED, labelled so everywhere)

- **At 4 layers.** The 62.5 → 31.25 µm step cannot be extrapolated: a 0.2 mm track centred on the grid keeps 0.125 mm of whole cells at 62.5 µm but 0.1875 mm at 31.25 µm, a step and not a first-order trend.
  - Instead, each board's worst VDD and worst GND pin was run alone at 31.25 and 15.625 µm (`b/ref.sh`), where the kept width is the same. The bounds move −1.5 % to −3.1 %.
  - The first-order extrapolation from those two pitches sits 4.3 % (s024l) and 5.1 % (sentinel-03) below the same-crop bound: 41.3 mV and 41.4 mV at 658 mA.
  - It removes the plane's discretisation only. The track-width loss, the junction and the range worst cases remain in both.
- **At 2 layers.**
  - All 387 VDD_3V3 segments are axis-aligned, with centre lines on the 125 µm grid. So each 0.2 mm track keeps exactly 0.1875 mm at both fine pitches, and the 31.25 → 15.625 change (−0.32 % to −0.58 %) is not convergence.
  - A straight run is bounded at 0.2/0.1875 = 1.067× its sheet resistance. The reference, bound × 0.9375 [inferred: track runs only], is 313–827 mΩ: 206–544 mV at 658 mA.
- **Why the 2-layer number is high.** At 30 µm and 125 °C a 0.2 mm track is 4.8 mΩ/mm, and the routes from L2 to U1 run tens of millimetres. The copper, under the stated conditions, gives most of the 2-layer number; the bound adds the 6.7 % above. An upper bound cannot prove a failure: this is a reference, not a lower bound.

### B2.5 The copper is copper

- **Planes, against KiCad's own refill.**
  - Both 4-layer boards were prepared as the DRC gate prepares them (`1303` P4.7's `prep`, now `spike/1303b/fill/prep`).
  - In1/In2 were exported by KiCad 10.0.7 with `--check-zones` in the pinned tools image, under `~/bin/capped-run` (caps read back: `memory.max=8589934592`, `memory.swap.max=0`, `pids.max=8192`).
  - Every plane cell at both pitches was tested for containment (shapely, integer nm; `fill/check_cells.py`, `fill/out-*.txt`). The transform was verified on every via flash: 201/201 and 340/340.
  - **0 of 2.6 million cells at 62.5 µm and 8 of 10.5 million at 31.25 µm fall outside.** Each of the 8 is a single cell in a pocket between four GND thermal vias on In2. KiCad removes the pocket as an island, and in the model each is a component of its own, joined to nothing (`fill/islands.out`). The repair gives such a cell zero flux, so no bound counts it.
  - The build will drop plane islands as KiCad does (S4).
- **Outer layers and every junction square, against an independent reading.**
  - The independent reading is `spike/1303/s4`'s `pcb.py`, with widths, pads and via sizes as the file states them, not the router's rules. Discs are inscribed polygons and drills circumscribed ones (`fill/check_copper.py`, `fill/copper-*.out`).
  - A junction square less its own drill must be covered on every layer it joins: planes against the refill plus the via pads.
  - Each square is tested 1 nm inside its edges, because the reader converts mm to nm in floating point; without that, a cell edge exactly on a pad edge was flagged once (R40.1).
  - The polygons flag 6 outer cells. Each has a corner exactly on a pad's arc: TP23's 1 mm circle, J1's mounting pad, U3.4's rounded corners. The exact integer pass in the same script puts all 6 inside.
  - **Result: 0 outer cells and 0 junction squares outside copper, on all three boards, at every pitch measured.**

### B2.6 Q1: the hole at ±0.075 mm

- **Which bounds rest on a junction:** all of them. Every 4-layer pin uses two vias and every 2-layer pin 4–16. The junctions' share is 0.43–0.73 mΩ per 4-layer pin (§2.2).
- **The pad at which a junction square fits outside the worst-case hole edge.** The square's half-side `a` (a multiple of 31.25 µm) must exceed `r_d + 0.075 + β`, and its corner must lie inside the via pad, so pad ≥ 2√2·a:

  | Drill | β | a | Pad needed |
  |---|---|---|---|
  | 0.3 mm (VDD class 0.7, GND default 0.6) | 30 µm | 281.25 µm | **0.80 mm** |
  | 0.3 mm | 10 µm | 250 µm | **0.71 mm** |
  | 0.254 mm (U11's thermal array, 0.554) | 30 µm | 250 µm | 0.71 mm |

- **The square fits; the junction does not follow.** The coefficient `κ` is derived for a hole concentric with its square. An offset hole needs either its own derivation (rays to the hole's centre, a band of varying width) or a square re-centred off the grid.
- **What a smaller β costs** in κ is not measured.

## B3. For the ruling

- **S1, Q1's choice.** Either:
  - a router via-pad change to 0.8 mm (or 0.71 mm with β 0.01 mm), which moves bytes on boards that declare the class (`1308`'s option); or
  - a worst-case junction, which is a new derivation.

  The junction is 1.4–3.6 % of each 4-layer bound, so either costs little in the bound. Until ruled, the gate is not built on the concentric condition.
- **S2, the ground terminal.** On the datasheet's wording (§2.1), the exposed pad is AGND, not PGND, and nothing says they join inside. I read that as "not stated", so U11.1 alone. Including U11.9 could only lower the bound. Is that the reading the ruling meant?
- **S3, the 2-layer VDD.** 334–882 mΩ bound and 313–827 mΩ reference: 206–544 mV at 658 mA, over 165 mV before any GND. Under the stated conditions the 0.2 mm tracks themselves exceed it. Whether this is a design finding for the 2-layer board is yours. The 2-layer gate refuses GND by name (`GroundReturnUnread`) as ruled, so no 2-layer drop verdict exists either way.
- **S4, the gate's parameters, which set the verdict's bytes:**
  - **Pitch:** 31.25 µm, or 62.5 µm, whose bounds are 13–35 % higher (sentinel-03's worst pair 85.9 mΩ: 56.5 mV).
  - **Crop:** proposed as the net's terminals' box plus 5 mm, fixed per net. 15 mm costs 1.8–2.0× the cells for 1.7–7.1 %.
  - **Time:** 18–30 s per pin at 31.25 µm (3–6 s at 62.5 µm). A 4-layer board's 26–27 pins take 9–10 minutes, on one core, at about 1 GB. One proposal per pin is rebuilt today; one factorisation for all the net's pins is possible and not built.
  - **Islands:** dropped as KiCad drops them.
- **S5, the fence.**
  - The work order's fence names `extract.rs` and `stack.rs`. This build adds two modules beside them, `flow.rs` and `supply.rs`, the extraction's new engine.
  - The measuring driver is an example in the published CLI crate. It goes when the gate exists.
  - The fill check's `prep` is a nested crate under `spike/1303b/fill/` (outside the workspace, as `1303`'s was). It goes with `spike/1303b/`.
  - `supply.rs` has no unit tests yet: the build adds them with the gate (exact cases on constructed boards, and a mutant per guard).

**After the ruling (as already ruled):** the optional fields refused by name when absent; the `power.loads` VDD_3V3 entries summed from cited maxima; the gate and the verdict's `supplies` and `pairs`; the USB pair on `pair-standin.json`; `GroundReturnUnread`; the §7.1 rows, with the new ones (a 2-layer read fill; a multi-source ground). Then the adversarial pass and the reduction pass, `spike/1303b/` deleted, CI green with TIER2_MIN exact, and READY last.

## B4. Record

- **Base and branch:** `main` `002dae07`, branch `wo/aet-1303b-extraction-build`.
- **This stop adds:**
  - `flow.rs`, `supply.rs`, and their registration in `lib.rs`;
  - the example driver;
  - evidence under `spike/1303b/b/` (runs, summary, reference, shares) and `spike/1303b/fill/` (KiCad Gerbers, `prep`, both copper checks, islands).
- **Existing files changed:** `lib.rs` (two modules), `crates/aethelgard-cli/tests/strict_fence.rs` (both new modules declared, as reading no KiCad file: they take `route::read`'s `Board` and `Routed`), and this FEEDBACK. No golden moves, no field, no verdict key.
- **Host suite:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`, and `cargo test --workspace` on the host, plus CI.
- **One container step,** the refill export, run by a subagent under `capped-run`, one container at a time.

---

# Part A: the §1.1 stop (record, ruled)

## A0. Status at the §1.1 stop (ruled on #222, comment 6968894352)

- **Coverage is below every pin on every board, with the router's remedies applied [measured]. That is the ruling's own D5 (b) trigger** (comment 6888926396: "If coverage on Sentinel-02's supply pins is still below every pin with the router's remedies applied, (b) the certified discrete flow is ruled in then").
  - VDD_3V3: 0 of 16 pins on the 2-layer power fixture; 10 of 16 on the power fixture at four layers (12 with GND classed); 6 of 16 on Sentinel-03.
  - GND: 0 of 11 on both power fixtures; 10 of 10 on Sentinel-03.
- **Where pins are bounded, the bounds are too loose to gate [measured].**
  - They run 142–892 mΩ, dominated by the plane hops, and each hop's bound is 4.1–87× the exact two-hole sheet (median about 12).
  - Even Sentinel-03's best VDD and best GND pin sum to 313 mΩ: 206 mV at 658 mA, over the 165 mV tolerance.
  - So a gate on (a) alone would fail every board on its bound's looseness, not on its copper.
- **A §7 condition also fires:** the 2-layer ground return has no honest bound from the copper the reader can read. It runs through a `B.Cu` zone that is unfilled in the emitted file (§A3).
- **Not built:** the discrete flow, any field, any gate. This branch adds FEEDBACK and the evidence (`spike/1303b/`) only.
- **For the ruling:** D5 (b), the 2-layer ground (§7), the ground terminal, the fields and their schema version, the verdict's shape, the load, the USB pair and the §7.1 rows (§A5).

## A1. What was measured, and how

**The boards.** Every specification in the corpus with a regulator, emitted by `main` `002dae07`'s release CLI:
- **`sentinel-02-power`,** the power fixture at two layers, as committed (`crates/aethelgard-ir/tests/fixtures/`);
- **`s024l`,** the power fixture at four layers. No such fixture exists (no test turns `power_tree()` into four layers [read at source, `crates/aethelgard-cli/tests/common/power.rs`]). It is constructed: the committed fixture with Sentinel-03's `copper_layers`, `layers` and `stackup` (`spike/1303b/build-fixtures.py`);
- **`sentinel-03`,** the corpus's 4-layer board with the same regulators.

**The router's remedies, as the specifications declare them [read at source]:**
- Both fixtures already put VDD_3V3 in a `SUPPLY_MCU` class: 0.2 mm tracks and 0.7/0.3 mm vias (`AET-WO-1308`'s via-pad option).
- GND has no class. The `-gnd` variant adds it to the same class (`build-fixtures.py`).
- Wider supply tracks were not tried: 1308 measured that 0.25 mm loses coverage, because it moves the stitching (1308 FEEDBACK B4).

**The terminals (D4):**
- **VDD:** the pad where the regulator's current enters the net. U11 is a buck (LMR33630) with no pad on VDD_3V3, so that is its inductor's output pad, `L2.2`.
  - At DC, that node is the net's highest potential: capacitors carry no current, and every other pad is a sink.
  - So for any point s on the net, V_s − V_pin ≤ V_L2 − V_pin ≤ R(L2, pin) × the net's total current (P4.8). That covers the feedback divider's sense point too (R20) [inferred].
- **GND:** U11's ground pads (U11.1, and the thermal pad U11.9). The choice is a question (§A5.3).

**The bound.** `1303`'s closed-form pieces, unchanged in their formulas, with the changes these boards need (`spike/1303b/oracle.diff`, each commented at its site):
- The reader keeps circle and oval pads as copper the plane must clear. The spike's reader refused them, and these boards have connectors.
- Every plane obstacle clears by its own size: d/2 + 0.2025 mm (P4.7's measured fill clearance), rounded up to 0.01 mm, plus 0.125 mm (F9). That covers each foreign via, and every through-hole and NPTH pad of any net. Before, every obstacle cleared as a 0.6 mm via.
- **One via geometry per run.** A run uses the net's vias of one geometry: VDD_3V3 0.7/0.3; GND 0.6/0.3, or 0.7/0.3 in `-gnd`.
  - The other own-net vias are the 0.554/0.254 vias in the regulators' thermal pads.
  - Their copper is not used (Rayleigh), and their drilled holes stay obstacles.
  - A track ending at one is dropped, so no path crosses its hole as a plain point.
- **A pass through a pad keeps off every drilled hole** (any via, any through-hole pad) by w/2 + r_hole + the hole offset.

**The regression check [measured].** The modified oracle, run with the spike's own `run_paths2.py` on the spike's boards, reproduces `spike/1303/s4/paths.out` byte for byte (`out/regress-spike-paths.out`). The changes are neutral where the old boards have none of the new features.

**The conditions:** `spike/1303/s4/conditions.json` as the spike stated them.
- ρ_max is IACS × (1 + α·105) × k 1.181, at 125 °C;
- outer copper ≥ 30 µm, plane copper ≥ 15 µm, plating ≥ 18 µm, all conditions;
- barrel lengths from the declared finished thickness (0.34–2.0 mm at two layers, 1.25–1.851 mm at four).

Each run was made with the hole concentric and at JLCPCB's ±0.075 mm.

## A2. Coverage [measured]

MCU supply pins (U1) bounded, of all, as `concentric / ±0.075 mm`:

| Board | VDD_3V3 as declared | VDD_3V3 with GND classed | GND as declared (0.6/0.3 vias) | GND classed (0.7/0.3) |
|---|---|---|---|---|
| `sentinel-02-power`, 2 layers | **0 / 0 of 16** | 0 / 0 | **0 / 0 of 11** | 0 / 0 |
| `s024l`, power fixture at 4 layers | **10 / 10 of 16** (315–434 / 357–506 mΩ) | 12 / 12 (315–477 / 357–578 mΩ) | **0 / 0 of 11** | 0 / 0 |
| `sentinel-03`, 4 layers | **6 / 5 of 16** (171–697 / 215–509 mΩ) | 5 / 5 (172–262 / 216–333 mΩ) | **10 / 9 of 10** (142–529 / 400–893 mΩ) | 10 / 10 (162–584 / 420–1116 mΩ) |

**Each pin's bound is exactly R ≤ ρ_max·(a/t_out + b/t_plane + c/A(t_p)).** a, b and c are fitted on four thickness sets per path, with worst residual ≤ 4×10⁻¹⁵, and printed per pin. The milliohm figures evaluate that at the conditions above.

**Refused pins, with the piece that refused them** (`out/cov-0.out`; "the search" is the flow-rule Dijkstra, retried after banning each failed piece):

| Board, net | Refused | Piece |
|---|---|---|
| `sentinel-02-power` VDD_3V3 | 16 | **via entry and pad pass.** Every pin is connected (16 of 16 reachable when the flow rules are ignored), but each path the search found failed at a via entry or at a pass through a pad. The bans then exhaust the search: 1 to 40 pad passes, and 1 via entry, per pin |
| `sentinel-02-power` GND | 11 | **no copper the reader reads.** Each pin's stub ends at a stitching via whose other side is the `B.Cu` zone, unfilled in the file. That is §A3 |
| `s024l` VDD_3V3 | 6 | via entry (1 to 3 per pin; one pin also 13 pad passes) |
| `s024l` GND | 11 | **pad end at U11.1:** the tube cannot leave U11.1 at any width. U11.9, the thermal pad, has no edge: its own vias are the excluded 0.554/0.254 ones, and no track reaches it. So every GND row rests on one entry |
| `sentinel-03` VDD_3V3 | 10 | via entry (6: 2 to 5 per pin, one also a pad pass); **no plane edge** (4: each pin's stub reaches one via, and that via has no admissible plane hop) |
| `sentinel-03` GND | 0 / 1 | at ±0.075 mm, one pin: no path |

**Looseness, measured on every plane hop the bounded paths use** (`D`, β, the hop's bound, and the exact two-hole sheet (ρ/πt)·acosh(D/2r_d), printed per hop):
- the bound is **4.1–87× the exact value, median 9.4–12.3**, over 84 to 149 hops per run;
- the plane term ρ·b/t_plane is most of each bound. U1.6 on Sentinel-03, for example: 627 of 697 mΩ.

**The drop this implies** (an illustration at 658 mA, not a gate; the load question is §A5.5):
- Sentinel-03's best VDD and best GND pin sum to (171 + 142) mΩ → 206 mV;
- its worst pair sums to (697 + 529) mΩ → 806 mV;
- the tolerance is 165 mV.

Whether the board or the bound is at fault, (a) cannot tell: the exact two-hole figures say most of it is the bound.

**Two more facts behind the GND rows:**
- **The In1 barrel is priced at the board's full 1.851 mm.** That is the declared finished maximum: the stackup declares dielectric minima only, so no shorter maximum depth to In1 exists to use.
- **The spike's own pieces still hold.** Every bounded path was recomputed from its own geometry, as in the spike.

## A3. The §7 condition: the 2-layer ground return

At two layers GND is F.Cu stubs and stitching vias into a `B.Cu` zone. The emitted file holds the zone unfilled; the DRC gate refills it in the image [read at source, 1303 P4.7].
- The reader therefore has no copper to bound any GND path through it, and on this board every path goes through it (11 of 11).
- That is the work order's §7: "a quantity the gate needs has no honest bound from the copper the reader can read: stop". The work order names two remedies: declare it a condition, or drop the gate.

**Options, for the ruling (none chosen):**
- **(i) Read the fill.** `extract` reads the zone as KiCad refills it, from the DRC gate's prepared copy in the image. The supply gate would then need KiCad, which `extract` does not today.
- **(ii) Declare it.** At two layers the ground return is a stated, unverified condition, and the gate bounds VDD alone. That under-claims, as D7 says.
- **(iii) Refuse it.** The supply gate refuses a 2-layer board by name (`GroundReturnUnread`), until (i) or a ground plane.

## A4. What the build would move (the goldens)

- **Sentinel-01's bounds and `lines`:** byte-identical. Sentinel-01 has no regulator, so no supply entry is computed.
- **The extract verdict's keys:** `supplies` and `pairs` beside `lines`. No committed file or test pins the verdict's bytes: tests read `verdict["lines"]` by key [read at source, `crates/aethelgard-cli/tests/*.rs`].
- **`verify`** copies extract's `conditions` strings into the bench package's `FAB-NOTES.md` [read at source, `verify.rs` L1590]. So the build must add no condition to a board without a regulator, or that package moves. Campaign hashes cover the twin's states, not extract's output [read at source].
- **The specification's fields: two ways, for the ruling.**
  - **(i) Schema 17, every new key required** (`null` where nothing reads it, as `1308` F1 did). This moves:
    - the 25 specifications at version 16;
    - Sentinel-01-minimal's sha256, in `boards/sentinel-01/` (8 files), `aethelgard-hwdef`'s two fixtures and `aethelgard-svd-hal`'s `serial-four` UI test;
    - Sentinel-01's sha256, in `boards/sentinel-01-bench/` (8 files) and the bench package's `MANIFEST.json` and `FAB-NOTES.md`. The package is rewritten by CI's `bench-package` dispatch.

    No KiCad file or bound moves.
  - **(ii) No version bump:** optional fields, refused by name when a gate reads one and it is absent (`INV-06`), never defaulted. Nothing above moves; the verdict gains its keys only.

## A5. The proposal, for the ruling

1. **D5 (b): rule it in.** Coverage and looseness both show the trigger [measured, §A2].
   - The (a) pieces stay as its exact-case tests: bar, annulus, barrel, two-hole sheet and the bend bracket, as the ruling planned.
   - The flow: per-layer sheets with holes, as `1303` S3's stream-function dual, joined by barrels.
   - The discrete flow is not built before the ruling.
2. **The 2-layer ground (§A3):** (i), (ii) or (iii).
3. **The ground terminal (D7).** Return current leaves the GND net at the regulator's ground pads (U11.1, U11.9) and at the battery return (the input connector's ground pad, and the input capacitors' grounds). Which is the terminal is a question:
   - the pad the regulator references its feedback to (R21's ground);
   - the regulator's ground pad;
   - every sink with a multi-terminal bound.

   No choice made.
4. **The fields** (§A4 (i) or (ii); names and units proposed):
   - `board.stackup.outer_copper_min_um`: `{value, source}`, read by any supply path on an outer layer;
   - `board.stackup.plane_copper_min_um`: `{value, source}`, read by a plane hop;
   - `board.stackup.plating_min_um`: `{value, source}`, read by any barrel;
   - `board.stackup.copper_temperature_max_c`: `{value, source}`. For Sentinel-02 that is 125 °C, the load's Tj (DS12117 Table 120, D6);
   - `power.drop_limits[]`: `{domain, max_millivolts, source}`, the VDD + VSS limit per domain. 165 mV for 3.3 V ±5 %, cited.
   - Optionally `board.stackup.dielectrics[].max_thickness_um`, which would let a barrel to In1 be priced at its own depth instead of the whole board (§A2).

   k = 1.181 (IPC-MF-150F §3.8.1.2), IACS and α stay cited constants in the module's documentation, with k on plated copper a stated condition (D6). Each field is refused by name when the gate reads it and it is absent, never defaulted (`RUL-14`: a minimum, never a nominal).
5. **The load is the net's total current** (P4.8 holds for every sink on the net, not one).
   - `power.loads` has no VDD_3V3 entry today [read at source]: its entries are VDD_5V and VDD_1V8.
   - Proposed: the gate reads `power.loads`' entry for the domain, `{domain, milliamps, source}`, refused by name when absent. Its value is the sum of every sink's cited maximum: the MCU's 658 mA (DS12117 Table 120) plus the rest.
6. **The verdict's shape.** `supplies` beside `lines`, one entry per domain:
   - `{domain, source: {reference, pad}, return: {reference, pads}, load_milliamps, limit_millivolts, pins: [...], returns: [...], drop_bound_millivolts, conditions}`;
   - each pin is `{pin, net, r_bound_microohms, coefficients: {a, b, c}}` or `{pin, net, refused: {kind, piece}}`;
   - `drop_bound_millivolts` is (the largest VDD bound + the largest VSS bound) × load, and `null` with a finding when any pin is refused. The MCU ties its VDD and VSS pins inside, so the worst pair is the bound [inferred].
7. **The USB pair (D2).** `pairs` beside `lines`: each conductor of a declared `diff_pairs` entry gets `c_bound_femtofarads` from the existing machinery, against its net's `net_constraints[].max_capacitance_femtofarads` (25,000 for full speed). The mismatch is named in each entry as "unverified: a stated condition of symmetric pair routing (1303 D2)". It is tested on `pair-standin.json`. No new field.
8. **The §7.1 rows:**
   - **A through-hole barrel priced as a via's:** the barrel piece takes the pad's drill as r_d. With (b), a through-hole pad is a barrel like any via.
   - **A relieved pad on a supply path:** refused by name (`ReliefUnpriced`) until its spokes are priced (`1309` D2). Any pad with a thermal-relief zone connection on a bounded path is refused.
   - **A cited current-to-width source:** re-owned. The R bound is this order's quantity; a heating limit per width (IPC-2152) is a different gate, so a backlog row.
   - **A stackup bound below the fabricator's minimum:** the new minima fields validate `minimum ≤ copper_max_um`. `copper_max_um` accepted down to 1 then cannot undercut a declared minimum.

## A6. Record

- **Base:** `main` `002dae07`. Branch: FEEDBACK and `spike/1303b/` only. The build deletes `spike/1303b/` once cited.
- **Oracle:** `spike/1303/s4` at `d5bc8416`, unchanged in the tree. The copy that ran is `~/.cache/aet-1303b/s4`, and its changes are `spike/1303b/oracle.diff`.
- **Host only.** No KiCad run was needed for §1.1: the boards are `emit`'s output, and P4.7's fill measurement stands as the spike's.
