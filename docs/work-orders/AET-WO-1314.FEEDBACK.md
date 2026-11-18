# AET-WO-1314 — FEEDBACK

Labels:
- **[measured]**: by a run under `spike/1314/`.
- **[read at source]**: from a page or file named beside it.
- **[inferred]**: from reasoning, neither read nor run.

## 0. Status: STOPPED at the measured stop (§1.1)

**The ring rule:** a via's pad keeps a minimum copper ring round the largest finished hole at the worst declared position.

**The supply gate passes once the ring is at least 28 µm** [measured, `gated.jsonl`]. Sentinel-03, with GND on rule-sized vias, then bounds every supply and return pin with no `SupplyViaPadTooSmall`:

| Ring | 0.3 mm drill pad | 0.254 mm thermal pad | Sentinel-03 drop at 748 mA |
|---|---|---|---|
| 28 µm | 0.70 mm | 0.66 mm | 84.6 mV (16/16 + 10/10) |
| 50 µm | 0.75 mm | 0.70 mm | 85.8–89.6 mV (16/16 + 10/10) |

A ring of 0 µm (0.65 mm pads) leaves GND refused, as 1303b predicts.

**No flat via price is reliable on Sentinel-03, measured.** The rule's larger vias push `IMU_SCK` and `IMU_MOSI` over 30 pF on today's routing: 31.3 and 42.4 pF at a 50 µm ring. A fixed via cost, or routing bounded nets first, fixes some combinations and breaks others (§2.3). The router trades vias for detours, and track costs capacitance too.

**Every ring has a passing combination** of the measured costings. `extract` passes every bounded net, `IMU_SCK` included, for example at a 50 µm ring with a bounded-net via cost of 48 (§2.2).

**Proposed costing (D3):** a deterministic ladder of routing attempts, each judged by `extract`'s own per-net figure on the emitted board in process. Emit the best one. Refuse by name, with the net and its figure, when none fits.

**Nothing moves on a board without `fab_tolerances`** [measured]. With every knob unset, Sentinel-03's board is byte-identical to main's. The knobs act only where tolerances are declared, or where set by hand for the sweep. D4 keeps the costing to boards that declare tolerances, because four committed fixtures fail a capacitance bound today (§3.4).

**For the ruling (§3):**
- D1: the ring and its source.
- D2: which vias the rule sizes, and a declared via smaller than the rule.
- D3: the costing.
- D4: the costing's scope.
- D5: the bytes and goldens that move.

## 1. The ring rule [read at source, inferred]

**The rule.** A via drilled `d` is finished anywhere from `d − minus` to `d + plus` (`fab_tolerances.hole_size_*`). Its centre lies anywhere within `position` per axis (`hole_position_um`), so up to `⌈position·√2⌉` from the pad's centre. A pad of diameter `D` keeps a ring of at least `r` at every size and position when

`D ≥ (d + plus) + 2⌈position·√2⌉ + 2r`, rounded up to 10 µm.

- **Worst case:** the largest hole at a diagonal corner of the position box.
- **`minus` does not enter:** a smaller hole only widens the ring.

With JLCPCB's declared figures (+0.13 mm; ±0.075 mm per axis, so a 106.07 µm reach):

| Ring `r` | Source | 0.3 mm drill | 0.254 mm drill |
|---|---|---|---|
| 0 µm | the hole stays inside the pad. IPC-6012's class allowance for via breakout is not cited: the standard is not held (roadmap row, 1303 D6) | 0.65 mm | 0.60 mm |
| 28 µm | the supply gate's own need: the band β 10 µm plus the declared plating minimum 18 µm (1303b `pad_needed`) | 0.70 mm | 0.66 mm |
| 50 µm | JLCPCB, jlcpcb.com/capabilities/pcb-capabilities, read 18 Nov 2026, Drilling: "Via diameter should be 0.1mm(0.15mm preferred) larger than Via hole size". That is a 0.05 mm ring (0.075 preferred), stated at nominal size; held here at the worst | 0.75 mm | 0.70 mm |

JLCPCB's "PTH annular ring ≧0.20mm" row is for component holes, not vias [read at source].

## 2. Measured [`spike/1314/`]

### 2.1 How

The measurement knobs live on this branch, read from the environment, and are for the stop only:
- `AET_RING_UM` sizes vias by the rule where `fab_tolerances` is declared: the board's own via, each class's via raised to at least the rule's pad, and each thermal array's via the same.
- `AET_BOUNDED_VIA_COST` prices a via on a net with a declared capacitance bound (today every via costs 24).
- `AET_BOUNDED_FIRST` routes bounded nets right after the stitched ones.

The boards measured are Sentinel-03 (4 layers) and the power fixture as the CLI tests take it (2 layers). Each emit takes about 3 s; a gated extract about 3 min.

### 2.2 Every bounded net, before and after [measured, `sweep1.jsonl`, `sweep2.jsonl`]

Values are pF, with the net's via count in brackets. The run labels used below:
- **today:** main's routing.
- **r28 / r50:** the ring rule at 28 / 50 µm.
- **c48 / c96:** a bounded-net via cost of 48 / 96.
- **BF:** bounded nets routed first.

**Sentinel-03:**

| net | bound | today | r28 | r28 c48 | r50 | r50 c48 | r50 BF c96 |
|---|---|---|---|---|---|---|---|
| I2C_SDA | 100 | 84.48 (12) | 100.22 (13) **over** | 77.57 (7) | 81.39 (8) | 82.90 (10) | 77.49 (6) |
| I2C_SCL | 100 | 75.94 (9) | 80.32 (9) | 82.77 (11) | 89.75 (12) | 83.83 (9) | 73.05 (6) |
| EXT_SDA | 100 | 80.02 (12) | 90.71 (16) | 75.84 (9) | 91.19 (12) | 77.41 (7) | 71.53 (6) |
| EXT_SCL | 100 | 88.03 (16) | 88.20 (13) | 75.33 (8) | 77.83 (8) | 94.32 (14) | 83.55 (10) |
| SPI_SCK | 30 | 12.89 (0) | 12.89 (0) | 12.89 (0) | 12.89 (0) | 13.38 (0) | 12.89 (0) |
| SPI_MOSI | 30 | 17.91 (2) | 19.84 (2) | 19.84 (2) | 29.93 (6) | 24.71 (2) | 17.15 (2) |
| IMU_SCK | 30 | 20.43 (2) | 32.48 (6) **over** | 28.03 (4) | 31.25 (6) **over** | 27.47 (4) | 28.20 (4) |
| IMU_MOSI | 30 | 20.76 (2) | 29.67 (4) | 22.77 (2) | 42.44 (8) **over** | 29.13 (4) | 21.40 (2) |

**The power fixture** (no net over its bound in any of the 40 runs):

| net | bound | today | r28 | r28 c48 | r50 | r50 c48 | r50 BF c96 |
|---|---|---|---|---|---|---|---|
| I2C_SDA | 100 | 60.88 (10) | 50.47 (3) | 55.68 (5) | 62.30 (7) | 58.74 (7) | 55.05 (3) |
| I2C_SCL | 100 | 44.54 (2) | 48.22 (3) | 44.28 (2) | 56.25 (6) | 44.60 (2) | 48.27 (4) |
| SPI_SCK | 30 | 12.89 (0) | 12.89 (0) | 12.89 (0) | 18.14 (2) | 18.14 (2) | 13.95 (0) |
| SPI_MOSI | 30 | 14.12 (0) | 14.51 (0) | 14.51 (0) | 13.91 (0) | 13.91 (0) | 13.49 (0) |

**Each via's own figure,** from `extract`'s `via_ff` on Sentinel-03: 1429 fF at 0.6 mm, 1534 at 0.65, 1641 at 0.70, 1749 at 0.75.

**The cost is congestion, not the price of a via.** At a 50 µm ring with no extra cost, `IMU_SCK` keeps 6 vias where today has 2, and its track grows from 6.7 to 9.9 pF. At cost 96–160 it still keeps 6, and its track grows to 13.4 pF. The larger stitched GND and default vias, laid before any track, block its short path.

### 2.3 The costings tried, on Sentinel-03 [measured]

Which runs passed every bounded net, by via cost:

| Ring | Order | 24 (today) | 48 | 96 | 160 | 320 |
|---|---|---|---|---|---|---|
| none | as today | pass | pass | pass | pass | pass |
| 0 µm | as today | fail | pass | pass | fail | pass |
| 28 µm | as today | fail | pass | pass | pass | pass |
| 50 µm | as today | fail | pass | fail | fail | pass |
| none | bounded first | pass | pass | pass | pass | pass |
| 0 µm | bounded first | pass | pass | pass | pass | pass |
| 28 µm | bounded first | fail | pass | pass | pass | pass |
| 50 µm | bounded first | fail | fail | pass | pass | pass |

**No single setting passes every ring.** Every ring passes at some setting.

### 2.4 The supply gate on rule-sized vias [measured, `gated.jsonl`; limit 165 mV, minima as 1303b's tests]

| Board, ring, cost | `extract` | VDD | GND | Drop |
|---|---|---|---|---|
| Sentinel-03, 0 µm, 48 | lines pass | 16/16 | 0/10, `SupplyViaPadTooSmall` | none |
| Sentinel-03, 28 µm, 48 | pass | 16/16 | 10/10 (≤ 50.4 mΩ) | 84.6 mV |
| Sentinel-03, 50 µm, 48 | pass | 16/16 | 10/10 (≤ 55.9 mΩ) | 89.6 mV |
| Sentinel-03, 50 µm, 320 | pass | 16/16 | 10/10 (≤ 52.7 mΩ) | 85.8 mV |
| power fixture, 28 or 50 µm, 48 | lines pass | 16/16 | refused, `GroundReturnUnread` (2 layers, ruled 1303b (iii)) | none |

### 2.5 Bytes [measured]

- **With no knob set,** Sentinel-03's board is byte-identical to main's (`cmp`).
- **The ring rule acts only where `fab_tolerances` is declared:** today the two regulator fixtures.
- **The fixture-emitting tier-1 tests** under a 50 µm ring and cost 48 (`goldens.out`): `outline_tier1`, `power_board_tier1` and `campaign_tier1` pass. `supply_tier1` moves in two tests (D5).

## 3. For the ruling

### D1. The ring and where it lives

**Recommend 50 µm, from JLCPCB's stated via rule, held at the worst declared size and position.** It is the only cited figure for a via's ring, and it covers the supply gate's own 28 µm with 22 µm to spare.

**Recommend it as a new optional field,** `fab_tolerances.via_ring_um {value, source}`. Ruling 6974760231 puts the fabricator's numbers in the specification, not the engine, and this is one of them. It is refused by name when the rule needs it and it is absent. The two fixtures would declare JLCPCB's figure, cited and dated.

The alternative is an engine constant citing JLCPCB. The WO's adversarial list names that very class: "a constant typed in where a declared tolerance exists".

### D2. Which vias, and a declared via smaller than the rule

**Measured as built for the stop:** the board's own via, each class's via and each thermal array's via are raised to at least the rule's pad.

**Recommend:**
- the board's own via is sized by the rule;
- a class whose declared via is smaller than the rule is refused by name (`ViaRingUnheld`, with the class and the pad that holds the ring), never silently raised. The fixtures' `SUPPLY_MCU` (0.7/0.3) would declare 0.75/0.3;
- a thermal array's via, whose ring is the engine's parts table's, is raised to the rule, with a note.

### D3. The costing

**Recommend a deterministic ladder of routing attempts**, each judged by `extract`'s own per-net figure, computed in process on the emitted board text (`extract::read`, then `lines` and `bound`, the same code `extract` runs):
1. as today;
2. bounded-net via cost 48;
3. 96;
4. 320;
5. bounded nets first, at each of those.

**Emit the attempt with the largest smallest margin** (fixed order, ties to the earlier). If none fits, refuse by name with each net over its bound and its figure. A board is never emitted over its bound.

**Why not a single price:** §2.3. A cap per net (vias ≤ headroom ÷ per-via figure) was not tried: the measured failures are detours as much as vias, and the in-process check covers both.

**Cost:** up to 8 emits of about 3 s each on a 4-layer board, only when the first attempt is over.

### D4. Scope

**Recommend: the ladder only where `fab_tolerances` is declared.**

Four committed fixtures fail a capacitance bound on today's routing, measured: `d3-24-i2c-12-spi`, `serial-four`, `sentinel-02-shaped` and `solve/stress`. Each fails with `capacitance_over_declared_bound`. Applied everywhere, the ladder would re-route them or turn them into emit refusals, and the WO forbids moving any board without tolerances.

The refusal ("never routed over its bound") would then hold for boards with tolerances only. Today's boards keep `extract`'s finding as their judge.

### D5. What moves

- **The two fixtures with `fab_tolerances`:** the Sentinel-03 board and the power fixture's board (vias resized, routing moved).
- **`supply_tier1`:**
  - the ruled (A) test (Sentinel-03 refused, "via pad >= 0.7 mm") inverts: Sentinel-03 now passes the gate. The refusal and its remedy are then tested on a specification whose class declares a via under the gate's need, if D2's refusal leaves one, or on a constructed one;
  - each per-tolerance refusal test emits its own board, because a board sized by the rule is unreadable under a specification without the tolerance (`BoardUnreadable`, measured).
- **Tier 2:** Sentinel-03's KiCad checks (`board_tier2`) run on the new board in the build. Not run at the stop.
- **No byte moves on Sentinel-01, its bench package, or any board without `fab_tolerances`.**

### Outside the fence, for the record

`fab.rs` (the fabrication package's drill text) names the engine's 0.6/0.3 mm via. Only Sentinel-01's bench board has a package, and it declares no tolerances, so nothing is wrong today. A package for a board with tolerances would need the rule's via there too. That belongs to a fab work order.

## 4. Record

- **Branch** `wo/aet-1314-via-tolerance`, base main `acae53f7`.
- **Evidence:** `spike/1314/` (scripts and outputs) and `~/.cache/aet-1314/m/` (boards).
- **The knobs** (`AET_RING_UM`, `AET_BOUNDED_VIA_COST`, `AET_BOUNDED_FIRST`) are in `stack.rs` and `route.rs`, labelled "MEASURED STOP ONLY". They go with the build.
