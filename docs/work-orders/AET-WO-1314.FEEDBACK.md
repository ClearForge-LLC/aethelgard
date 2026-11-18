# AET-WO-1314 — FEEDBACK

Labels:
- **[measured]**: by a run named beside it.
- **[read at source]**: from a page or file named beside it.
- **[inferred]**: from reasoning, neither read nor run.

Part B (the measured stop) is kept as a ruled record. Its evidence under `spike/1314/` is deleted with this build, as ruled, and stays in this branch's history at `69ae5142`.

## 0. Status: READY

Built to the rulings on #227: comment 6982875594 (D1–D5 accepted, with D3's three conditions) and comment 6987450886 (§8's stop: (A), with three conditions).

- **The ring rule.**
  - Where a specification declares `board.stackup.fab_tolerances`, every via keeps the declared ring (`via_ring_um`, a new optional field) round the largest finished hole at the worst declared position: `D ≥ (d + plus) + 2⌈position·√2⌉ + 2·ring`, rounded up to 10 µm.
  - The board's own via, and any class that declares none, takes the rule's pad. A thermal array's via is raised to it, with a note in the verdict.
  - A class declaring a smaller via is refused by name (`ViaRingUnheld`), and a missing tolerance the rule needs likewise (`FabToleranceUndeclared`).
- **The capacitance ladder.**
  - Such a board is routed at each of 8 engine rungs: today's routing; a bounded net's via at 48, 96 and 320; then each of those with bounded nets first.
  - Each rung is judged by `extract`'s own arithmetic on the emitted text. The rung with the largest smallest margin is emitted, and that margin is reported.
  - If no rung fits, the board is refused, naming every net over its bound with its figure (`CapacitanceOverBound`).
  - It chooses; `verify` still judges by running `extract` on its own. A test proves the second run passes and finds the same margin.
- **Acceptance (WO §2) [measured]:**
  - **Sentinel-03 with GND on rule-sized vias (0.75/0.3 mm, thermals 0.7/0.254):** every capacitance-bounded net passes in `extract`'s own run. The smallest margin is `IMU_SCK` at 24.945 of 30 pF (16.9 %), at rung 7: bounded nets first, via 320.
  - **Its supply gate** bounds every pin, 16/16 + 10/10, at 85.8 mV against 165 mV, with no `SupplyViaPadTooSmall` (§2).
  - **No board without `fab_tolerances` moves a byte:** Sentinel-01 and its bench package, and every fixture (§2.3).
  - **Each guard has a mutant a test kills** (§4.2).
  - **CI is green and `TIER2_MIN` is exact** (§5). Sentinel-03's tier-2 KiCad checks ran on the new board (§5).

## 1. What is built

### 1.1 `crates/aethelgard-eda/src/stack.rs`

- **`RingRule`**: `of(spec)`, `pad_nm`, `unheld`, and `diagonal_nm`, the ⌈e·√2⌉ reach, now one source for the rule and the supply gate's holes.
- **`Stack::of`**:
  - sizes the board's via and undeclared classes' vias by the rule;
  - raises thermal arrays, with a note in `Stack::notes`;
  - leaves a declared class's via as declared;
  - `Stack::bounded` is each I2C SDA/SCL and SPI SCK/MOSI net with a declared bound: exactly the nets `extract` bounds.

### 1.2 `crates/aethelgard-eda/src/route.rs`

- **`Rung`, `RUNGS` (engine constants beside `VIA_COST`) and `route_with`.**
- **`route(board)`** is `route_with(board, Rung::TODAY)`, today's router: every via 24, the order unchanged.

### 1.3 `crates/aethelgard-eda/src/schematic.rs`

- **`emit_moved`**: the ring refusals, then `laddered` on a board with tolerances, and `write_moved` at today's rung on any other.
- **`laddered`**: places once and routes the 8 rungs on threads, each result kept at its index, so the choice does not depend on timing.
  - It judges each rung with `extract::read`, `extract::lines` and `extract::bound`.
  - **`choose`** (a pure function, unit-tested) picks the largest smallest margin among the fitting rungs, the earlier on a tie, or else the nearest rung's overs.
  - **Unjudged boards:** where `extract::lines` refuses (the committed power fixture: `TranslatorPinUnbounded`), the board is routed as today and the ladder records `unjudged`. `extract` refuses it by the same name. That keeps `1310`'s ruled "the crossing is drawn".
  - **Rungs that fail to route** are reported (`unrouted_rungs`).
- **Refusals:**
  - `FabToleranceUndeclared` (`id` is the field's path);
  - `ViaRingUnheld` (`id` is the class);
  - `CapacitanceOverBound` (`id` is the first net);
  - `LadderRefused` (`extract`'s own kind, when its reader refuses the ladder's board).

### 1.4 Elsewhere

- **`crates/aethelgard-eda/src/pcb.rs`:** `placed` and `write_placed_at` split from `write_moved`, which takes a rung.
- **`crates/aethelgard-eda/src/rules.rs`:** the KiCad project's Default class via is the stack's, not a typed-in 0.6/0.3. This is just outside the WO's named fence; it is the adversarial pass's "constant typed in where a declared tolerance exists" (L5).
- **`crates/aethelgard-cli/src/main.rs`:** `emit`'s report gains `capacitance_ladder` (the rung, its price and order, `smallest_margin`, `unrouted_rungs`, `unjudged`, `judged_by`) and `notes`. Both are absent on a board without tolerances.
- **IR:** `fab_tolerances.via_ring_um` (0–1000 µm, cited). README row 13 and the optional-keys list are updated.
- **Fixtures** (Sentinel-03 and the power fixture):
  - each declares `via_ring_um` 50 µm, citing JLCPCB's Drilling note ("Via diameter should be 0.1mm(0.15mm preferred) larger than Via hole size"), read 18 Nov 2026;
  - `SUPPLY_MCU` declares 0.75/0.3.

## 2. Measured

### 2.1 Sentinel-03 [measured, this head's release binary]

- **Vias:** 301 at 0.75/0.3 and 24 thermal at 0.7/0.254. No other size.
- **`extract`, on its own:**

  | Net | pF | Bound |
  |---|---|---|
  | I2C_SDA | 72.3 | 100 |
  | I2C_SCL | 77.1 | 100 |
  | EXT_SDA | 66.8 | 100 |
  | EXT_SCL | 67.2 | 100 |
  | SPI_SCK | 12.9 | 30 |
  | SPI_MOSI | 14.3 | 30 |
  | IMU_SCK | 24.9 | 30 |
  | IMU_MOSI | 21.5 | 30 |

  All pass. Before, on today's 0.6 mm vias, `IMU_SCK` was 20.4 pF; with GND alone on 0.7 mm vias (1303b) it was 31.3 pF.
- **The supply gate** (limit 165 mV, the minima of 1303b's tests): pass, 16/16 VDD and 10/10 GND bounded, 85.8 mV.
- **Emit time:** about 25 s in release, the 8 rungs routed in parallel (the slowest rung about 20 s). On the same machine it was 2.85 s for a single routing.

### 2.2 The power fixture

- **As the CLI tests take it** (crossing and IMU removed), s024l (4 layers) and the 2-layer board: tier 1 passes. The s024l gate is unchanged in shape.
- **As committed:** `emit` routes it as today, with the vias the rule's, and records `unjudged: TranslatorPinUnbounded`. `extract` refuses it by that name (test).

### 2.3 Bytes [measured]

- **Every committed board without `fab_tolerances` is unchanged:**
  - the workspace suite is green on this head, with every byte golden and Sentinel-01's bench package included;
  - with tolerances absent, `emit` takes the old path at `Rung::TODAY`, and the KiCad project's via renders as before ("0.6", "0.3").
- **What moved, as ruled (D5):**
  - the two fixtures with tolerances (their boards);
  - `supply_tier1`'s (A) test, now `sentinel_03_on_rule_sized_vias_bounds_every_return`;
  - the remedy test, now on a declared ring of 0. A class under the rule is refused at `emit` (`ViaRingUnheld`), so the gate's remedy is reached through a ring smaller than the gate's need: GND vias at 0.65 mm, "via pad >= 0.7 mm". This is a deviation from the ruling's literal wording ("a class declares an undersized via"), for that reason;
  - the per-tolerance refusals: the rule's own (position, larger size, ring) refuse at `emit`, in `via_ring_tier1`; the gate's (smaller size, etch) still refuse in `extract`.

## 3. Rulings applied

All of 6982875594:
- **D1:** the field, 50 µm, cited.
- **D2:** `ViaRingUnheld`; thermal raised, with a note.
- **D3:** rungs as engine constants, the three knobs gone; the in-process check documented and tested as choosing, not witnessing; Sentinel-03's smallest margin reported.
- **D4:** tolerances only.
- **D5:** as above.

## 4. Adversarial pass, mutation, reduction

### 4.1 Adversarial pass (full, WO §5)

A fresh subagent found no high findings. I checked each one.

**Fixed:**
- **M1:** the ladder's `extract` refusals had become an anonymous `EmitterError`. `lines` refusing now routes the board as today and records `unjudged` (test above). A refusal reading back the ladder's own board keeps `extract`'s kind (`LadderRefused`).
- **L1:** `bounded` held every constrained net, so a net `extract` never judges was still priced. It now holds exactly the nets `extract` bounds.
- **L2:** a board with nothing bounded reported an empty margin. The margin is now optional.
- **L3:** a rung that failed to route was hidden behind one that routed. It is now reported.
- **L5:** the project file's via was typed in.

**Kept, by ruling:**
- **M2:** the ladder chooses on capacitance alone and may leave rung 0 when a later rung has more margin. The supply gate, DRC and pair checks are `verify`'s, judged on the emitted board as for any other. Documented on `laddered`.

**Recorded:**
- **L4:** 8 concurrent routes take about 8 times one route's memory. `speeds.rs` calls `emit` for its capacitance message and so now pays the ladder on a tolerance board.
- **L6:** through-hole pads' rings are outside this rule (a non-goal; a thermal via that no longer fits its pad is refused by `arrays()`, by name).

**The WO's four classes:**
- **A ring that holds at nominal, not at the worst:** the formula takes the largest hole at the position box's corner, exact in integers, and every via kind is sized by it (verified, with tests).
- **A bounded net routed over its bound:** every emitted rung is judged by `extract`'s arithmetic, and `extract` judges it again on its own (test).
- **A board with no tolerances whose bytes move:** none (§2.3).
- **A constant typed in where a declared tolerance exists:** none left. The KiCad project's via was one, and is fixed (L5).

### 4.2 Mutation [measured, `~/.cache/aet-1314/mut/run.py`, one mutant per guard]

23 mutants, one per guard this order adds or changes, each run against the suites the guard lives in. **23 killed.** The last two are the placement shortcut's (§8): the routed board nudged, and `placement` routing.

- **Not mutated: `diagonal_nm`'s integer correction loop.** It is labelled belt-and-suspenders: the float's ceiling of `e·√2` is exact for every `e` up to 3 mm (measured, every nanometre; the IR caps a tolerance at 1 mm), so no test input can reach it.
- **Guards added for the mutation run:**
  - `choose` became a pure function with two unit tests: the largest smallest margin, ties, a rung over its bound, an unrouted rung, no bounded net, the nearest rung's overs;
  - the project file's via has a unit test;
  - Sentinel-03's ladder choice is a golden: rung 7, `IMU_SCK` at 24 945 fF.
- **A harness filter named no test once.** The `ViaRingUnheld` mutant ran zero tests and read as a survivor; it was re-run with the right filter, and killed.
- **A guard removed instead of tested:** the filter keeping only constrained nets in `bounded`. `validate` already requires every line's bound (`MissingBusConstraint`, `MissingSpiConstraint`), so it was dead code.

### 4.3 Reduction pass (diff)

A fresh subagent ran it; I checked each claim.

Reduction pass (diff): −13 net lines · 2 duplicates removed (the ⌈e√2⌉ reach, now `diagonal_nm`, shared with 1303b's `hole_radius`; the tier-1 `root`/`scratch` helpers, now `common::verify`'s) · 1 dead field removed (`Stack::ring`) · 1 dead mechanism removed (a shortcut for a board with nothing bounded, which `choose` already handles) · 0 protected · 2 findings:
- 14 tier-1 files each define their own `scratch`/`root`/`run`, a repo-wide backlog;
- `write_moved` takes a rung only to route at it.

## 5. Gates

Local, on this head [measured]:
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -D warnings` are clean.
- `cargo test --workspace` is green.
- The byte comparison against main's binary over every committed specification (§2.3).

CI on the PR, and Sentinel-03's tier-2 KiCad checks on its new board (`board_tier2`, ruling D5), are recorded in the READY commit.

## 6. §7.1 rows (for the architect to apply)

| Item | From | Owner |
|---|---|---|
| The default 0.6/0.3 mm via cannot keep its band at the fabricator's worst hole (1303b) | 1303b, ruling 6974760231 | **done, `1314`:** the ring rule, where tolerances are declared |
| Sentinel-03's supply gate needs a ground via pad of at least 0.7 mm; at 0.7, `IMU_SCK` exceeds 30 pF (1313 Q3) | 1303b, ruling 6976355662 | **done, `1314`:** rule-sized vias with the ladder; `IMU_SCK` at 24.9 pF, the gate passes |
| The router adds vias without cost on capacitance-bounded nets | 1313 Q3 | **done for boards with tolerances, `1314`** (the ladder); boards without them as the next row |
| `d3-24-i2c-12-spi`, `serial-four`, `sentinel-02-shaped` and `solve/stress` fail a declared capacitance bound on today's routing (`extract`'s `capacitance_over_declared_bound`); the ladder would fix or refuse them | 1314 D4 | a later router work order, triggered when those fixtures' bytes may move |
| A fabrication package for a board with `fab_tolerances` uses the rule's via (`fab.rs` names the engine's 0.6/0.3) | 1314 ruling | `4500`'s fab step |
| A through-hole pad's ring at the declared tolerances | 1314 adversarial L6 | trigger: a through-hole part on a board with tolerances whose ring is in question |
| The ladder's cost per emit: a Sentinel-03 `emit` goes from 2.85 s (one routing) to about 25 s unpinned, 48.4 s on 4 CPUs (eight routings in parallel, 11–48 s each); every check that emits a board with tolerances pays it | 1314 §8, ruling 6987450886 | the architect (docs PR) |

## 7. Record

- **Branch** `wo/aet-1314-via-tolerance`, PR #227, base main `acae53f7`.
- **Evidence (Furnace):**
  - `~/.cache/aet-1314/final/` (Sentinel-03 emitted, extracted and gated on this head);
  - `~/.cache/aet-1314/cmp.txt` (byte identity);
  - `~/.cache/aet-1314/mut/` (`run.py`, `final.log`);
  - the measured stop's `~/.cache/aet-1314/m/`.

### Lessons

- **A filter in a mutation harness that names no test reads as a survivor.** Check that each filter matches a test before believing a result.
- **A class with no declared via must follow the board's via, not the engine's constant.** A unit test on `Class::via_declared` found it, before any board did.

## 8. The ladder's cost in `outline_tier1`: stopped at `c197dab3`, ruled (A) in 6987450886, built

The architect's note: `order_only_renamings_place_the_fixture_the_same_way` ran more than 16 min in shard 2 before the cancel. It had never run in this PR's CI before, because `bd85b33b` failed to compile.

**Measured [`~/.cache/aet-1314/order4*.log`, 4 CPUs pinned with `AllowedCPUs=0-3`, the test alone]:**

| | main `82925510` | this branch |
|---|---|---|
| the test | 42.7 s, pass | 543.4 s, pass (12.7×) |
| one release emit of Sentinel-03, unpinned | 2.85 s, 1 routing | 25 s, 8 routings in parallel |
| the same, on 4 CPUs | — | 48.4 s |

On 4 CPUs the 8 rungs, routed in parallel and contending, took:

| Rung | Time | Fits |
|---|---|---|
| 0 | 23.1 s | no: `IMU_MOSI` at 141 % |
| 1 | 48.0 s | yes |
| 2 | 23.1 s | no |
| 3 | 11.4 s | yes |
| 4 | 19.5 s | no |
| 5 | 19.8 s | no |
| 6 | 20.9 s | yes |
| 7 | 12.9 s | yes, best: `IMU_SCK` at 83 % |

**What this is:**
- **It terminates:** it is slow, not a hang. Under CI's shard concurrency it passes the 16 min mark.
- **Every seed places identically,** so there is no determinism finding: the test compares placement (`name_free`: each footprint's position and the outline), and it passes.
- **The cause is the ladder (D3) routing every Sentinel-03 emit 8 times,** in a test that emits the fixture 21 times to check placement only. Today's routing alone does not fit the fixture's bounds (rung 0, `IMU_MOSI` 141 %), so the ladder cannot be skipped for it.

**Options:**
- **(A) Recommended: place without routing in this test.** The assertion is placement only, so the test's cost should be placement's.
  - A small `aethelgard-eda` entry would emit the placed board without routing: `pcb::placed` exists, and the outline is the `gr_rect`.
  - The base and 20 seeds keep their full meaning, and the ladder is unchanged.
  - **This changes the test's cost,** so it is the architect's to rule.
  - `each_size_sample_places_with_its_gaps_or_is_refused_by_name` also emits Sentinel-03 three times and checks the routed loops, so it keeps routing.
- **(B) Stop at the first rung that fits.** No gain on this fixture: rung 1 fits first, and it is the slowest rung (48 s). It also changes D3's "best smallest margin". Not recommended.
- **(C) Fewer rungs.** It changes the ruled rungs, for a small gain. Not recommended.

None of these skips, ignores, cuts the seeds of, or re-shards the test.

**Built to the ruling, (A) with its three conditions:**
1. **One placement path.** `schematic::place` is what `emit` itself runs up to the router: validate, design, stack, the via ring rule's refusals, the declared bound, then `pcb::placed`.
   - `emit_moved` calls it, then routes, by the ladder or at today's rung.
   - `schematic::placement` calls it too, and writes the board through the same writer with routing off (`write_placed_at(…, None)`): no track, via or zone. Its documentation says it is the placed board, not a routed one, and the CLI does not expose it.
   - `pcb::write_moved` became test-only.
2. **The shortcut proven.** The test emits the base fixture once in full, ladder included, asserts the ladder ran, and asserts its `name_free` equals `placement`'s. The 20 seeds compare through `placement`.
   - A mutant that nudges the routed board's outline by one pitch fails that assertion by name ("emit's board places otherwise than its own placement").
   - A mutant that routes inside `placement` fails `placement_writes_no_copper_and_refuses_as_emit_does`, which also proves `placement` refuses a 60 × 60 mm bound with `emit`'s own `OutlineUnreachable`, word for word.
3. **The shard budget,** in §5.

**Measured after it [the test alone, 4 CPUs pinned]:** 50.6 s (main 42.7 s, the branch before it 543.4 s). The one full emit with the ladder is most of it.

# Part B: the measured stop (record, ruled in comment 6982875594)

## B0. Status: STOPPED at the measured stop (§1.1)

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

## B1. The ring rule [read at source, inferred]

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

## B2. Measured [`spike/1314/`]

### B2.1 How

The measurement knobs live on this branch, read from the environment, and are for the stop only:
- `AET_RING_UM` sizes vias by the rule where `fab_tolerances` is declared: the board's own via, each class's via raised to at least the rule's pad, and each thermal array's via the same.
- `AET_BOUNDED_VIA_COST` prices a via on a net with a declared capacitance bound (today every via costs 24).
- `AET_BOUNDED_FIRST` routes bounded nets right after the stitched ones.

The boards measured are Sentinel-03 (4 layers) and the power fixture as the CLI tests take it (2 layers). Each emit takes about 3 s; a gated extract about 3 min.

### B2.2 Every bounded net, before and after [measured, `sweep1.jsonl`, `sweep2.jsonl`]

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

### B2.3 The costings tried, on Sentinel-03 [measured]

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

### B2.4 The supply gate on rule-sized vias [measured, `gated.jsonl`; limit 165 mV, minima as 1303b's tests]

| Board, ring, cost | `extract` | VDD | GND | Drop |
|---|---|---|---|---|
| Sentinel-03, 0 µm, 48 | lines pass | 16/16 | 0/10, `SupplyViaPadTooSmall` | none |
| Sentinel-03, 28 µm, 48 | pass | 16/16 | 10/10 (≤ 50.4 mΩ) | 84.6 mV |
| Sentinel-03, 50 µm, 48 | pass | 16/16 | 10/10 (≤ 55.9 mΩ) | 89.6 mV |
| Sentinel-03, 50 µm, 320 | pass | 16/16 | 10/10 (≤ 52.7 mΩ) | 85.8 mV |
| power fixture, 28 or 50 µm, 48 | lines pass | 16/16 | refused, `GroundReturnUnread` (2 layers, ruled 1303b (iii)) | none |

### B2.5 Bytes [measured]

- **With no knob set,** Sentinel-03's board is byte-identical to main's (`cmp`).
- **The ring rule acts only where `fab_tolerances` is declared:** today the two regulator fixtures.
- **The fixture-emitting tier-1 tests** under a 50 µm ring and cost 48 (`goldens.out`): `outline_tier1`, `power_board_tier1` and `campaign_tier1` pass. `supply_tier1` moves in two tests (D5).

## B3. For the ruling

### B3 D1. The ring and where it lives

**Recommend 50 µm, from JLCPCB's stated via rule, held at the worst declared size and position.** It is the only cited figure for a via's ring, and it covers the supply gate's own 28 µm with 22 µm to spare.

**Recommend it as a new optional field,** `fab_tolerances.via_ring_um {value, source}`. Ruling 6974760231 puts the fabricator's numbers in the specification, not the engine, and this is one of them. It is refused by name when the rule needs it and it is absent. The two fixtures would declare JLCPCB's figure, cited and dated.

The alternative is an engine constant citing JLCPCB. The WO's adversarial list names that very class: "a constant typed in where a declared tolerance exists".

### B3 D2. Which vias, and a declared via smaller than the rule

**Measured as built for the stop:** the board's own via, each class's via and each thermal array's via are raised to at least the rule's pad.

**Recommend:**
- the board's own via is sized by the rule;
- a class whose declared via is smaller than the rule is refused by name (`ViaRingUnheld`, with the class and the pad that holds the ring), never silently raised. The fixtures' `SUPPLY_MCU` (0.7/0.3) would declare 0.75/0.3;
- a thermal array's via, whose ring is the engine's parts table's, is raised to the rule, with a note.

### B3 D3. The costing

**Recommend a deterministic ladder of routing attempts**, each judged by `extract`'s own per-net figure, computed in process on the emitted board text (`extract::read`, then `lines` and `bound`, the same code `extract` runs):
1. as today;
2. bounded-net via cost 48;
3. 96;
4. 320;
5. bounded nets first, at each of those.

**Emit the attempt with the largest smallest margin** (fixed order, ties to the earlier). If none fits, refuse by name with each net over its bound and its figure. A board is never emitted over its bound.

**Why not a single price:** §2.3. A cap per net (vias ≤ headroom ÷ per-via figure) was not tried: the measured failures are detours as much as vias, and the in-process check covers both.

**Cost:** up to 8 emits of about 3 s each on a 4-layer board, only when the first attempt is over.

### B3 D4. Scope

**Recommend: the ladder only where `fab_tolerances` is declared.**

Four committed fixtures fail a capacitance bound on today's routing, measured: `d3-24-i2c-12-spi`, `serial-four`, `sentinel-02-shaped` and `solve/stress`. Each fails with `capacitance_over_declared_bound`. Applied everywhere, the ladder would re-route them or turn them into emit refusals, and the WO forbids moving any board without tolerances.

The refusal ("never routed over its bound") would then hold for boards with tolerances only. Today's boards keep `extract`'s finding as their judge.

### B3 D5. What moves

- **The two fixtures with `fab_tolerances`:** the Sentinel-03 board and the power fixture's board (vias resized, routing moved).
- **`supply_tier1`:**
  - the ruled (A) test (Sentinel-03 refused, "via pad >= 0.7 mm") inverts: Sentinel-03 now passes the gate. The refusal and its remedy are then tested on a specification whose class declares a via under the gate's need, if D2's refusal leaves one, or on a constructed one;
  - each per-tolerance refusal test emits its own board, because a board sized by the rule is unreadable under a specification without the tolerance (`BoardUnreadable`, measured).
- **Tier 2:** Sentinel-03's KiCad checks (`board_tier2`) run on the new board in the build. Not run at the stop.
- **No byte moves on Sentinel-01, its bench package, or any board without `fab_tolerances`.**

### Outside the fence, for the record

`fab.rs` (the fabrication package's drill text) names the engine's 0.6/0.3 mm via. Only Sentinel-01's bench board has a package, and it declares no tolerances, so nothing is wrong today. A package for a board with tolerances would need the rule's via there too. That belongs to a fab work order.

## B4. Record

- **Branch** `wo/aet-1314-via-tolerance`, base main `acae53f7`.
- **Evidence:** `spike/1314/` (scripts and outputs) and `~/.cache/aet-1314/m/` (boards).
- **The knobs** (`AET_RING_UM`, `AET_BOUNDED_VIA_COST`, `AET_BOUNDED_FIRST`) are in `stack.rs` and `route.rs`, labelled "MEASURED STOP ONLY". They go with the build.
