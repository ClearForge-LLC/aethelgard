# AET-WO-1317 — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read]** at the source named beside it; **[inferred]** from arithmetic or reasoning.

## Status: READY — (a) built

- **The mechanism, named [measured, §2].**
  - (k)'s canonical rank breaks ties between parts that are equal in class, area and label-net distance by a colour: an FNV hash refined over every pad of every net, ground and the rails included.
  - So removing any part re-rolls those comparisons, and greedy first-fit placement (`spot`) and greedy compaction (s) turn a re-rolled order into a different local optimum.
  - With U2 gone, the forward turn's U11 group stalls behind U5's group after 17 of its 46 steps left, and the right edge stays 7.25 mm out.
  - The first divergence is `spot`, at U7, with the order identical up to there. Transplanting the with-U2 order restores the fit, so (k) decides. The width is lost in (s).
- **The sweep counts it [measured, §3].**
  - Parts level: 14 of 485 removals from a fitting board stop it fitting (2.9 %).
  - Spec level: 1 of 140.
  - Two-entry removals: 53 of 2,296.
  - Add-one: 4 refused boards fit once a part is added.
  - Soft growth is everywhere: about 73 % of removals widen or heighten the outline by more than 1 mm.
- **The fix, (a) [measured, §4].**
  - Under a declared bound, a placement that misses the bound is placed again with the canonical rank reseeded, seeds 1 to 8 in turn, and the first that fits is kept.
  - A board that fits the first time is the board returned, so no committed board moves. That covers 41 specifications through `generate`, Sentinel-01 and the bench package among them, and the five passing `outline_tier1` cases.
  - The forward turn fits (95.05 × 67.85 mm). Size samples 2 and 6 fit; 8 is still refused.
  - Hard cases fall:
    - parts level: 14/485 to 5/485, every one C4's, an MCU support capacitor no specification can remove;
    - spec level: 1 to 0;
    - two-entry: 53 to 0 of 3,430;
    - add-one: 4 to 0.
- **What it does not do.** Soft growth, the hash re-roll and C4 are untouched (§6, F1–F3). A cure for the re-roll itself would move the five pinned placements; it is the architect's row.

## 0. Predictions (`spike/1317/PREDICTIONS.md`, e5a061bf, before the first run), scored

| # | Predicted | Measured | Score |
|---|---|---|---|
| P1 | With U2 back: 96.50 × 69.35 and all four turns fit. Without: forward refused at 103.500 × 67.100 | exactly (`out/calibration.tsv`) | right |
| P2 | First divergence in the order (k), before U2's step (p 0.6); else a part taking U2's freed spot (p 0.3) | the order is identical through U3; the first placement to differ is U7, the step after U2's, taking the ring spot U2 had | **wrong**; the 0.3 alternative was right |
| P3a | The 3.5 mm is in x and already in the core before compaction (p 0.65) | in x, but **not** before compaction: the pre-compaction cores are 80.325 and 80.242 mm wide, the same right edge (154.475). Compaction pulls the right edge in to 142.975 with U2, to 150.225 without | **wrong** |
| P3b | Transplanting the with-U2 order restores the fit (p 0.55); (m) does not decide | fits, 96.300 × 67.850. With (m) knocked out both runs are refused alike (102.000 / 101.750) | right |
| P4 | ≥ 3 hard cases (p 0.7); ≥ 10 % soft at 1 mm (p 0.6); grows and shrinks; persists with (k) knocked out (p 0.7) | 14 hard (R), 1 (S); 73 % soft; 86 % shrink in some dimension; with (k) knocked out 10 hard, soft 638 → 143 | right, all four |
| P5 | C1's move is not this mechanism; it is U10's reserve (p 0.6) | not this trigger, but **not the reserve**: with C1 out of U10's reserve the outline is identical (95.80 × 67.60). It is the order: an anchored part goes straight after its IC (§2.3) | half right |
| P6 | No (k)/(m)/(s) change keeps the five passing cases identical; a refusal-only fallback can; ends in (a) (p 0.5) | built as predicted; the rank change was not built (inferred, §6 F2) | right |
| P7 | Sweep under 30 min, peak under 2 GiB | 33 s (R), 30 s (R+), 8 s (S); peak 13 MiB (driver) | right |

## 1. Method

- **Harness** (`spike/1317/harness/`, in a scratch copy of `main` ff7fd53d):
  - `hooks.patch` adds trace lines and knock-out knobs to `pcb.rs`.
  - `spike1317.rs` removes parts from the list `design` returns.
  - `driver.rs` builds the variants with `outline_tier1`'s own edits and `common::rename::sized`.
  - With U2 is 03d89554's fixture hunk reversed onto today's fixture.
  - With the hooks in and nothing knocked out, `outline_tier1` passes 9/9, placement digest included [measured], so the hooks move nothing.
- **Removal rules:**
  - **R (as committed):** a part with the parts it owns.
  - **R+ (amended after calibration, labelled so):** also every part left with a label net no other part shares.
  - **S (the committed fallback):** remove one declared entry (I2C or SPI device, connector, test point) and its load rows from the specification.
- **Calibration** decided between them:
  - R's U2 (U2, C37) fits at 85.2 × 79.7, because R5, U2's SDO strap, is not owned and stayed.
  - Removing U2, C37 and R5 reproduces 103.500 × 67.100 exactly, without 4202's renumbering of R6–R8 and the capacitors. So references play no part.
- **Excluded:** the MCU only. Five spec-level removals per variant are refused by `validate`, their own class, never counted: U4 (the flash an acknowledgement names), and the debug connector and the three serial ports' connectors.

## 2. The mechanism

### 2.1 The step diff: the forward turn, with U2 and without [measured, `out/trace-u2-forward-{with,without}.txt`]

Parts are matched by reference: the parts-level removal renames nothing.

| Stage | With U2 (fits) | Without U2, C37, R5 (refused) |
|---|---|---|
| Placing order | … U4 U5 **U2** U7 U3 **C14–C17 C24–C27** R51 R50 … | … U4 U5 U7 U3 **C24–C27 C14–C17** R50 R51 …; the bench parts reordered too |
| Steps U1 … U5 (0–40) | — | identical, origin for origin |
| First difference | U7 at (85.75, 87.50) | U7 at (85.75, 93.25), the spot U2 took; then U3 takes U7's, and 44 of 90 steps differ |
| Core before compaction | 80.325 × 74.780, right edge 154.475 | 80.242 × 72.780, right edge 154.475 |
| The right-edge group | U11 with L2, C22, C23, R20, R21 (L2's keep-out at 154.475 in both) | same |
| U11's compaction moves | 46 down, then 46 left | 46 down, then **17 left**: blocked by U5's group (U5, C43, C44 at x ≤ 134.25 in U11's band) |
| Core after compaction | 68.825 wide, right edge 142.975 | 75.992 wide, right edge 150.225 |
| Right connector edge | 152.000 | 159.250 (+7.25) |
| Outline | 96.300 × 69.100 | 103.500 × 67.100 |

**Which of (k), (m), (s) decides** [measured, `out/ablation.txt`]:

| Run | With U2 | Without |
|---|---|---|
| as built | fit 96.30 × 69.10 | refused 103.50 × 67.10 |
| without's parts, **with's order** (transplant) | — | **fit 96.30 × 67.85** |
| (k) knocked out (reference order) | fit 97.55 × 69.10 | fit 96.05 × 67.85 |
| (m) knocked out | refused 102.00 | refused 101.75 |
| (s) knocked out | refused 107.80 | refused 107.75 |

- **(s) is where the width is lost.** Without compaction the two runs differ by 0.05 mm.
- **(k) decides which local optimum compaction reaches.** Its order, transplanted, restores the fit.
- **(m) does not decide:** without it both are refused alike.
- **No single swap carries it.** Swapping only the C14–C17 / C24–C27 block back gives 102.300 (`out/order-swap-block`).
  - A prefix bisection (the with-U2 order for the first k entries, the reduced order after) flips fit and refusal four times between k = 69 and k = 90 (`out/prefix-bisection.txt`). All of those steps are test points.
  - A test point placed beside an IC joins that IC's compaction group, so its spot changes what the group collides with.
  - Fit is a chaotic function of the order.

### 2.2 Why the order re-rolls [read, `pcb.rs` `canonical_rank`]

- The rank keys are:
  1. the MCU;
  2. the ICs;
  3. the passives;
  4. the bench parts;
  5. the owners of anchored parts;
  6. courtyard area;
  7. label-net distance from the MCU;
  8. **the colour**;
  9. the owned parts' colours;
  10. the reference.
- C14–C17 (1210 on VDD_5V) and C24–C27 (1210 on VDD_3V3) tie on every key before the colour. Rails only, so their distance is the same, unreachable.
- The colour is FNV over the multiset of pad colours on each net, GND and the rails included, refined three rounds. U2 has pads on VDD_3V3 and GND, so removing it changes both nets' colours, and with them every part's.
- A hash comparison between two parts flips with probability about ½ per tie. Removing any part on GND (all of them) re-rolls every tie at once.
- The same happens with C4, another case: removing C4 (on VDD_3V3) flips U10 against U11. Both are regulators owning anchored parts, of equal area and distance. U11 then goes first, lands below the MCU, and drags U10 to the far left, and compaction makes 15 moves where it made 131: 112.25 wide (`out/trace-base-without-C4.txt`).

### 2.3 The related row: C1's group move (`1315` (c)) [measured, `out/c1.txt`, `out/trace-base-C1-*.txt`]

| Fixture | C1 anchored (as built) | C1 unanchored (owner none) | Anchored, C1 out of U10's reserve |
|---|---|---|---|
| today's (no U2) | 95.80 × 67.60 | 86.70 × 80.95 | 95.80 × 67.60 |
| with U2 (1315's tree) | 96.50 × 69.35 | **86.70 × 81.70** (1315 measured the same) | 96.50 × 69.35 |
| size 2 | refused 102.00 | fit 89.45 × 83.20 | refused 102.00 |

- **Not the same trigger, and not the reserve.**
  - Anchored, C1 (CP_Elec_8x10, 10.8 × 8.9 mm keep-out) is placed straight after U10, at step 30, by `4300d2` §1.3's rule that an anchored part follows its IC.
  - That puts it before U11, in the ring spot by U10. U11's first fit is then (145.05, 94.55), the right-hand column 1315 saw, where it was (99.30, 49.05), the top row.
  - The width is set before compaction: the core's right edge is 154.475 against 131.925.
- **What is shared** is the primitive: greedy first-fit `spot` packs whatever comes first nearest. Here a deliberate order rule, not a hash, decides what comes first.
- The reseeded rank does not touch it: C1 fits either way, and the anchored-after-IC grouping is applied after the rank.

## 3. The sweep [measured]

The committed fixture, `outline_tier1`'s four turns and size samples 1, 2, 6 and 8, each with one removal at a time.
- **R:** 97 removals per variant.
- **S:** 33 per variant, of which 28 are placed and 5 are refused by `validate`.
- **Hard:** the full board fits and the reduced one does not.
- **Soft:** the reduced outline is wider or taller by more than 1.000 mm.

`main`'s placer is in the first three result columns; the reseeded placer (8 seeds) in the last two.

| Variant | Full board, `main` | R: fit / hard / soft | S: fit / hard / soft | R, reseeded: hard | S, reseeded: hard |
|---|---|---|---|---|---|
| fixture | fit 95.80 × 67.60 | 95 / 2 / 76 | 28 / 0 / 19 | 1 | 0 |
| right turn | fit 95.80 × 67.60 | 95 / 2 / 76 | 28 / 0 / 19 | 1 | 0 |
| forward turn | refused 103.50 × 67.10 | 91 / 0 / 63 | 25 / 0 / 18 | 1 (now fits whole) | 0 |
| left turn | fit 95.80 × 67.60 | 95 / 2 / 76 | 28 / 0 / 18 | 1 | 0 |
| aft turn | fit 96.25 × 67.35 | 95 / 2 / 73 | 28 / 0 / 19 | 1 | 0 |
| size 1 | fit 98.80 × 69.85 | 91 / 6 / 67 | 27 / **1** / 19 | 1 | 0 |
| size 2 | refused 102.00 × 71.10 | 60 / 0 / 73 | 18 / 0 / 21 | 1 (now fits whole) | 0 |
| size 6 | refused 114.35 × 73.60 | 60 / 0 / 63 | 18 / 0 / 19 | 2 (now fits whole) | 0 |
| size 8 | refused 118.50 × 79.45 | 8 / 0 / 71 | 3 / 0 / 19 | 0 (still refused) | 0 |
| **total** | | 690 / **14** / 638 of 873 | 203 / **1** / 171 of 252 | **9** | **0** |

- **As a share of removals from a fitting board:**
  - R: 14/485 (2.9 %) on `main`; 9/776 (1.2 %) reseeded. On the five boards that fit on `main` it is 5/485, every one C4's.
  - S: 1/140 on `main` (size 1 less its second test point, 102.80 × 69.85); 0/224 reseeded.
- **R's hard removals on `main`:** C4 ×5, C22 ×5, C30, C21, Q_J1, Q_TP2 (size 1). R+ has the same 14.
- **C4** is an MCU support capacitor. The emitter always draws it, and no specification can remove it, so its 5 survive on the parts level only (§6 F3).
- **Knock-outs, R** (`out/sweep-R-no{k,m,s}.tsv`):
  - (k) out: hard 10, soft 143.
  - (m) out: the full board fits in 1 variant, hard 2, soft 701.
  - (s) out: the full board fits in none, soft 611.
  - Reference order is far steadier (soft 638 → 143), because removing a part shifts no other part's rank. But references are what `1313c` took out, and the hard cases persist.
- **Seeds, measured before freezing at 8** (`spike/1317/out/fb-*`):
  - 4 seeds: 19/776 hard. More boards fit whole, so more removals can be hard.
  - 8 seeds: 9/776.
  - 16 seeds: 9/776.
  - Colour rounds 0–2 and 4–8 instead of seeds: 9/776. Rounds were rejected (§4.1).
- **Cost:**
  - R: 32.7 s wall, 13 MiB peak.
  - R+: 29.7 s.
  - S: 8.4 s.
  - Reseeded R: 43–73 s for 4–16 seeds.
  - Host release build, nice 10, capped scope.

## 4. The fix (a) [measured]

### 4.1 What is built (`crates/aethelgard-eda/src/pcb.rs`)

- `canonical_rank(items, seed)`: seed 0 is `1313c`'s rank byte for byte. Any other seed is hashed in ahead of every symbol and footprint.
  - Two colours are equal under every seed or under none, a 64-bit collision aside. So a seed reorders only what colours already order, and the reference decides the same ties it did.
  - Colour rounds would not have kept that. At 0 rounds, C14–C17 and C24–C27 tie and the reference decides.
- `layout_of` places once (seed 0).
  - Only under a declared bound, and only on `OutlineUnreachable`, it places again with seeds 1 to `RESEEDS` (8) and returns the first that fits.
  - Any other refusal, on any attempt, is returned as it is, `CompactionUnbounded` included.
  - When every seed misses, the refusal names the least-over attempt (the larger overrun, then the sum, then the earlier seed), so "the placer's best outline" stays true. Size 8: 102.150 × 88.200; seed 0 was 118.500 × 79.450.
- Unbounded placement, every board under `boards/` and `examples/`, never reaches a retry.

### 4.2 Bytes: nothing committed moves [measured, `spike/1317/baseline.sh`]

`aethelgard generate` writes into one fixed scratch path for each of 41 specifications: every `examples/*.json`, every top-level test fixture, and the Sentinel-03 variants. Every file it writes and every verdict are hashed, `main`'s release binary against the branch's (`out/baseline-main/`):

- **Identical:** every file of every specification that generated on `main`. That includes Sentinel-01 (`examples/sentinel-01.json`, whose project is `sentinel-01-bench`, the bench package's board), Sentinel-03, its right, left and aft turns, size 1, and both with-U2 variants.
- **Moved:** only the three that `main` refused, which now generate: the forward turn, size 2 and size 6. Size 8's refusal now names its least-over attempt.

### 4.3 The re-judged cases [measured]

| Case | `main` | Branch | Held by |
|---|---|---|---|
| forward turn | refused 103.500 × 67.100 | **95.05 × 67.85**, seed 1 | `outline_tier1::a_turned_imu_…`: back to the placed-outline, keep-out, gap and loop checks the other turns get |
| size 2 | refused 102.000 × 71.100 | **89.20 × 83.20**, seed 2 | `each_size_sample_…`: outline, gaps, loops |
| size 6 | refused 114.350 × 73.600 | **98.15 × 87.45**, seed 2 | the same |
| size 8 | refused 118.500 × 79.450 | refused, best 102.150 × 88.200 | the same, its figures pinned |

TIER2_GATES_PLACEHOLDER

### 4.4 Tests

- **`outline_tier1::removing_one_declared_part_never_stops_a_bounded_board_fitting`** (tier 1, 4.6 s on the host):
  - It is the S sweep's slice over every variant that places whole: the fixture, the four turns, sizes 1, 2 and 6, and the forward turn with U2.
  - Every declared entry is removed in turn, 253 placed, and each must still fit.
  - On `main`'s placer it fails at its first precondition: the forward turn does not place whole, and nor do sizes 2 and 6 [measured, the branch's test against `main`'s `pcb.rs`].
  - With that precondition made a skip, four removals stop fitting on `main` [measured]:
    - U2, TP14 and TP24 from the forward turn with U2;
    - Q_TP2 from size 1.

    The S sweep in §3 has only the last, because it did not include the with-U2 variant.
- **`outline_tier1::a_reseeded_placement_is_deterministic_and_name_free`** (tier 1, 0.7 s): the forward turn, placed on seed 1, gives the same bytes twice, and the same name-free placement under 5 order-only renamings.
- **`each_size_sample_…`** pins sizes 2 and 6 at their outlines, and size 8's least-over figures.

### 4.5 Mutants [measured, `spike/1317/harness/mutants.sh`]

Each is one replacement in the final `pcb.rs`, judged by `outline_tier1` whole (debug, host, 4 threads):

| Mutant | Result | Failing tests |
|---|---|---|
| none (control) | 11/11 pass | — |
| M1 `RESEEDS = 0` (the fix reverted: `main`'s behaviour) | **killed** | the slice, the reseed test, the turned IMU, the size samples |
| M2 the seed ignored (every attempt is seed 0) | **killed** | the same four |
| M3 the most-over attempt reported instead of the least | **killed** | the size samples (size 8's pinned figures) |
| M4 ties in overrun go to the later seed | **survives** | — |
| M5 a fit is not kept (the loop runs on past it) | **killed** | the same four as M1 |

- **M4 survives** because no measured refusal has two attempts with exactly equal overruns. Size 8's nine attempts all differ, and so do the 60 × 60 test's.
- The earlier-seed tie rule is a stated choice that keeps the refusal deterministic. It is not held by a test. Constructing a tie would need a hand-built design, so it is left as a finding (F6), not a test written to kill a mutant.

## 5. Adversarial pass (§5, full)

1. **A fix that fits by shrinking a gap, a keep-out or a clearance.** None is touched [read: the diff changes `canonical_rank`'s starting hash, `place`'s signature and `layout_of`'s retry, nothing else]. The re-judged boards pass the gap and loop checks of every other bounded board (§4.3), and the gates in the image (§4.3).
2. **Monotone on the sweep, not beyond it** [measured, `out/adv-*.tsv`, public API only, `main` and the branch from the same driver]:

   | Probe | `main` | Branch |
   |---|---|---|
   | Remove two declared entries: 10 variants, 528–561 pairs each, 3,808 placed | 53 hard of 2,296 from a fitting board (turn-aft 8, size 1 11, the forward turn with U2 34) | **0 of 3,430** |
   | Add one (U2, or a test point beside each existing one): placed cases | 4 refused boards fit once a part is added (the forward turn +U2 and +a GND test point; sizes 2 and 6 +a GND test point) | 0 |
   | Soft growth over 1 mm, pairs | 2,266 | 2,043 |

   - **Beyond the sweep it holds for every probe tried. That does not prove it in general:** it is eight more draws, not an order that cannot re-roll.
   - A board that misses with all nine seeds and fits when a part is added is possible. Size 8 is the nearest measured: no addition made it fit.
3. **A placement that a reference name or list position now decides.**
   - A seed is a number, and the retry's trigger is the bound alone.
   - The renaming test holds the forward turn, placed on seed 1, to its name-free placement under 5 order-only renamings. `order_only_renamings_place_the_fixture_the_same_way` holds seed 0 under 20.
   - The removal slice's two with-and-without-U2 cases carry 4202's renumbering (R6–R8, C37–C44) and agree with the parts-level runs, which rename nothing.
4. **Compaction's bound (`1313d`) reached on a case that fitted before.**
   - No case that fitted before runs anything new: seed 0 is the first attempt.
   - Every reseeded attempt runs the same `compact`, with its own bound checked as it moves.
   - No `CompactionUnbounded` appeared in any sweep: 873 × 3 R runs, the reseeded runs, 3,808 pairs on the branch [measured, the class columns].

## 6. Findings

- **F1. The re-roll stays; soft growth is untouched.** About 73 % of one-part removals still move the outline by over 1 mm, and the reseeds change only refused boards. A board inside its bound can still grow when a part leaves. [measured, §3]
- **F2. The cure for the re-roll is a removal-stable tie key, and it moves the pinned placements.**
  - A rank whose order between two parts does not flip when an unrelated part leaves would need keys that change little with one part. Examples: a rail's degree or its ICs, in place of a hash over all of GND and the rails.
  - Any such key reorders today's ties, so the fixture's placement digest and the four other passing cases move.
  - `1317` forbids that, so it was not built. The architect's §7.1 row. [inferred; the knock-out shows the cost: reference order's soft growth is 143 against 638, §3]
- **F3. C4 and other MCU support parts.**
  - Removing C4 swaps U10 and U11 in the rank, and no seed of the nine recovers it: 112.25 wide on the fixture.
  - No specification can remove C4, so this is a parts-level case only. It is the same re-roll, at the regulators.
- **F4. Test time.**
  - `outline_tier1`'s size test now emits sizes 2 and 6 with the eight-rung ladder: 23 s alone on the host. The forward turn also emits.
  - In tier 2, `board_tier2::each_size_sample_…` now runs every gate on sizes 2 and 6. Its weight row is 1.5 s, from when all three were refused.
  - `ci/test-weights.tsv` and `ci/colocate.tsv` (`outline_tier1` 152 s) are outside this fence unless the plan's guard asks. CI_PLACEHOLDER
- **F6. The tie rule in the least-over choice is untested** (mutant M4 survives, §4.5).
- **F5. A board refused under a bound now costs nine placements before the refusal** (the 60 × 60 test, size 8): about 0.3 s in release.

## 7. Reduction pass

Diff-sized, by a fresh subagent over `git diff origin/main -- crates/`, then checked here.

- **Taken: a dead guard.** `layout_of`'s first match tested `bound.is_some() &&`. `OutlineUnreachable` is raised only under a bound, so the guard is always true where it is reached. Removed (−1 line); the mutants and the suite ran after it.
- **Not taken: folding seed 0 into the retry loop.** It saves about 2 more lines, but needs an `Option` best and an `expect` in library code.
- **Fixed: a misplaced doc comment.** `a_turned_imu_…`'s doc had run on into the new `turned` helper; it is reattached, and its stale 4202 history is cut to one line.
- **Checked, kept:**
  - `fits` overlaps `solved`, but must not panic on a `validate` refusal.
  - `match seed { 0 => FNV_BASIS, … }` keeps seed 0 byte-identical.
- **Spike data:**
  - The two-removal TSVs keep only their placed non-fit rows (736 K → 104 K); the driver regenerates the rest.
  - `fb-Rplus-salt_1-8.tsv` (identical in result to R's) is deleted.

Reduction pass (diff): −1 line net in code · 1 duplicate removed (spike data) · 1 dead path removed · 0 protected kept · 0 findings below.

## 8. Gates

GATES_PLACEHOLDER
