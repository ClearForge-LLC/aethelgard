# AET-WO-1313c — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read at source]** from a file named beside it; **[inferred]** from arithmetic or reasoning.

## 0. Status: READY on `main` `0829fe8` (`4400d` merged in, §6)

- **F1, its own commit, byte-neutral:** anchored parts by the emitter's draw sequence (`Anchor::seq`); the corpus 24/24 byte-identical against `main`.
- **(k)+(m)+(s), under a declared bound only:** Sentinel-03 at four layers places at **85.25 × 80.95 mm**, the spike's outline and its every position, and passes every gate, `extract`'s least slack 9.2 pF [measured].
- **Order independence, as tests:** parts order (unit), every reference class reversed (unit), 20 order-only renamings of the fixture (tier 1); 10 shuffles of the real fixture [measured].
- **Size samples, no margin claimed:** at 2 characters `extract` refuses the board by name (−5.1 pF); at 6 every gate passes (5.4 pF); at 8 `emit` refuses it (`OutlineUnreachable`).
- **Adversarial:** one name dependence found and fixed (A1, §3); compaction keeps the gaps, the approach strips, the outline and every loop rule; 15 mutants, all killed, and on the merge two more on `shift`: one killed, the other keeps compaction from terminating (no assertion fails; CI's 45-minute shard timeout is what would stop it).
- **No bound declared:** every byte unchanged; only the fixture's board moves.
- **After the merge with `4400d` (§6):** the fixture's placement and outline are unchanged under specification 16 (it declares no orientation), so nothing is re-pinned. A turned IMU is compacted whole, its keep-outs inside the outline, at each of its four turns, and every gate passes at each [measured].

## 1. What is built

### 1.1 F1, its own commit (`dc6f2994`)

- **`Anchor::seq`** carries each anchored part's place in its IC's draw sequence. One `Builder::anchor_last` sets it and replaces the three owner-setting copies in `schematic.rs`, `schematic_power.rs` and `schematic_support.rs`.
- **`place` takes an IC's anchored parts by `(seq, reference)`**, never by list position. `seq` is unique per IC in the emitter, so the reference never decides there; it only makes a hand-built design with a repeated `seq` deterministic.
- **Byte-neutral [measured]:** the 24-specification corpus, `emit` and `generate`, is byte-identical against `main` `457d644`'s binary (24/24, 371 files), and the bounded fixture's `emit` is byte-identical too.
- **`with_anchored_parts_the_board_does_not_depend_on_the_parts_order`** extends the parts-order test to a design whose draw sequence runs against reference order: every rotation of the list and its reverse write the same bytes, and the first in the sequence has first pick by its pin. It fails with the anchored parts in list order, and with them in reference order (both mutants run).

### 1.2 (k)+(m)+(s), under a declared bound only (`pcb.rs`)

- **(k) `canonical_rank`.** Colour refinement (three rounds) over symbol and footprint, pad numbers, nets as multisets, ownership (owner's colour, `decouples`, `seq`, the anchor's pad pairs). The rank: MCU, ICs, passives, bench parts (a `GND` test point last); IC owners first ((r)); larger courtyard; nearer the MCU over label nets; colour; **the colours of the parts it owns** (§3, A1, new in the build); reference last.
  - It decides the placing order, the anchor tie-break, the bench parts, the connectors' edges, and the compaction order. An IC's anchored parts go by `seq`, as F1 has them.
  - It replaces (r)'s `bounded` key in `placing_order`, which the rank includes.
  - A net's kind (label net, rail, ground, unconnected) is read from the form of its name, never its text.
- **(m) `centroid_target`:** the centroid of the anchor's pads on every shared net.
- **(s) `compact`:** each group (an IC other than the MCU, with every non-IC part placed beside it), rigidly, a `STEP` at a time toward the MCU's courtyard centre, with a sideways offset of up to 10 mm when the step in y is blocked. A move keeps the placer's gaps (`Rule::gap`), stays out of every approach strip, and stays inside the x-range the parts had before compaction. The termination argument is a comment at the loop: each group's potential (|dy|, |dx|) falls strictly with each move.
- **Refusals unchanged:** `OutlineTooSmall` and `OutlineUnreachable`, by name, nothing written.
- **No environment switch:** none of the prototype's `AET_1313B*` variables survive (`INV-06`); the declared bound is the only switch.

### 1.3 Ported from the prototype, byte for byte [measured]

`spike/1313b/placer.patch` applied at `457d644` and run with `AET_1313B=kms` is the oracle. It is pre-`4400d` and reads specification 15 only, so the byte comparison holds for the build at `457d644`; after the merge, §6 says what still bridges to it. The build's `emit` is byte-identical to it on the fixture, and on the 2-, 6-, 8- and 12-character size samples (8 and 12 refused by both with the same `OutlineUnreachable` message). The one addition, the owned-parts key (§3, A1), moves no byte of any of these.

## 2. Acceptance (§2), measured

### 2.1 The fixture at four layers [measured]

| | Outline (mm) | Every gate | Least `extract` slack | `emit` |
|---|---|---|---|---|
| `main` `457d644` (`1313b`'s figure) | 85.25 × 99.05 | yes | 8.8 pF | 1.4 s (release) |
| **1313c** | **85.25 × 80.95** | **yes** | **9.2 pF** | 2.9 s (release), 3.8 s (debug, as the tests run it) |

- **Every gate** = `validate`, `generate`, `emit`, ERC, DRC by the specification's rules with and without schematic parity, `check support` and `extract`, each with no finding: the 1313 runner (`run.sh`), KiCad in the pinned image under `capped-run`, one container at a time.
- **The loop rules hold:** all 15 (both bucks' input capacitors, CBOOT, inductor, VCC, feedback, bulk input; the MCU's reservoir) pass in `check drc --spec`'s power section, and in the tier-1 test (`loops::judge` on the emitted board).
- **The outline is the spike's exactly**, and the placement too: the tier-1 test pins the name-free placement's digest, which equals the one computed apart (Python) from the prototype's board.
- **`emit` doubles** (1.4 → 2.9 s release): compaction's clearance check is members × parts per candidate step. It is under the bound only.

### 2.2 Order independence, as tests

- **Parts shuffles:** `under_a_bound_the_board_does_not_depend_on_the_parts_order`: every rotation of a 16-part design's list, and each reversed, places every part the same, and the reversed list writes the same bytes (compared once, §5). The design has two owner ICs with anchored parts, one with a loop rule, draw sequences against reference order, an IC that owns none, two passives that tie but for their value, a signal and a `GND` test point, and two side connectors.
- **On the real fixture [measured, not a test]:** 10 shuffles of the parts list (an uncommitted xorshift hack in `write_moved`, seeds 1-10) gave byte-identical boards, 10/10, and 10/10 on the 2- and 6-character samples too. A real design's parts list cannot be reached from a test without a seam, and a seam would be a switch (`INV-06`).
- **The reversed tie-break:** `under_a_bound_no_reference_decides_a_placement` renames every reference class in reverse (so the last tie-break, and every order references could set, runs backwards) and finds the same name-free placement. With no bound it finds a different one, so the check is not empty.
- **Order-only renamings, an identity test:** `order_only_renamings_place_the_fixture_the_same_way`, 20 seeded renamings of the fixture (`rename_perm.py`'s, in `tests/common/rename.rs`), each emitted, each the same name-free placement.

### 2.3 Size samples [measured; tier 1 and tier 2]

References prefixed by `len` characters (`rename_len.py`'s, ported). The tier-1 test checks each sample's placement (gaps, loops) or its refusal; the tier-2 test runs every gate and prints each outcome.

| `len` | Outcome | Outline (mm) | Least `extract` slack |
|---|---|---|---|
| 2 | **refused by `extract`** (`IMU_SCK` over its 30 pF by 5.1 pF); every other gate passes | 89.20 × 84.95 | −5.1 pF |
| 6 | **passes every gate** | 96.90 × 87.95 | 5.4 pF |
| 8 | **refused by `emit`** (`OutlineUnreachable`, 101.90 × 88.20, nothing written; `generate` the same) | — | — |

- **No margin is claimed.** The 2-character sample places with every gap and loop kept, and `extract` refuses it by name: a refused board, not a wrong one (ruling R1). The `extract` margin moves with geometry; only a capacitance-aware step could hold it (backlog, R4).
- The 6-character sample here is `rename_len.py`'s (every reference, connectors and test points included), not 1307's renamer, so its outline is not `1313b`'s 93.70 × 87.95.

### 2.4 No bound declared [measured]

- The 24-specification corpus (every example and fixture with a board and an MCU, no bound), `emit` and `generate`, is byte-identical against `main` `457d644`'s binary: 24/24.
- Only the fixture's board moves: no other specification declares a bound (`grep max_outline_um`), so the bench package does not move.

## 3. Adversarial pass (§5)

- **A1. A name still decided a placement under a bound [measured; fixed].** Two ICs alike in footprint and nets (two sensors on one bus) that own different anchored parts tied in the prototype's rank: an IC's colour sees its nets, never what it owns. The reference then chose which went first, and the placements differed by name. `under_a_bound_no_reference_decides_a_placement` found it on its first run.
  - **Fix:** a key after the colour, the multiset of colours of the parts an item owns. It splits only ties the colour leaves, so it moves no byte of the fixture or any size sample (re-checked against the oracle).
  - The fixture never showed it: its ICs differ in their nets. The mutant that drops the key fails that test.
- **No name or list position decides a placement under a bound, in what is tested:**
  - parts order: every rotation and reverse of the 16-part design, the same placement (and the same bytes, reversed); 10 shuffles of the real fixture [measured]; the mutant that breaks the rank's last tie by list position fails it;
  - every reference class reversed: the same name-free placement;
  - 20 order-only renamings of the fixture, whose bus-net prefixes also reorder the nets' names: the same name-free placement.
  - The anchor tie-break, the connectors' edges, the anchored parts' order and the target each have a mutant that puts a name back, and a test fails for each.
  - **What is not proved:** a design the tests do not reach could still tie in the rank between parts that are not alike. Only the reference then decides, and the outcome is the same name-free placement only if the parts are truly alike.
- **Compaction against the placer's gaps, the approach strips and the outline:**
  - `compaction_keeps_the_placers_gaps_and_the_approach_strips` (unit; U2's pins beside a declared pair's, so compaction would cross the strip);
  - `the_fixture_places_at_the_spikes_outline_with_every_gap_kept` and the size samples (tier 1): every keep-out box is rebuilt from the emitted board and the library as the placer drew it (reference on F.Fab where the board prints it there), inside the outline, at 0.5 mm, 1 mm between ICs.
  - Mutants that drop the approach check, zero the gap, drop the x-range limit, or make the IC gap 0.5 mm each fail a test.
- **Loop rules after compaction:** every rule (both bucks' input capacitors, CBOOT, inductor, VCC, feedback; the MCU's reservoir) passes on the fixture and on each sample that places, judged by `loops::judge` at tier 1 and by `check drc --spec` in the image. Compaction moves each group whole, so an anchored part keeps its place by its pins.
- **No bound, moved bytes:** the corpus, 24/24 byte-identical against `main`; the workspace's goldens unchanged (`cargo test --workspace`).
- **Mutation (diff-sized), 13 mutants on the build and F1's two, all killed:** approach check, compaction gap, x-range, owned-parts key, centroid target, anchor tie, connectors' order, compaction off, sidestep off, ownership in the colour, the distance key, `Rule::gap`'s IC gap, the rank's last tie by list position; and F1's two (anchored parts in list order; in reference order, re-run after the reduction). The distance key first survived: it moves bytes but not the outline, so the placement digest pin was added. A fourteenth, the anchored parts' tie by rank or by reference, survived because `seq` never ties in the emitter; that branch was removed (§4).

- **After the merge, on the guard the turned IMU depends on (`shift`) [measured]:** the mutant that leaves a group's origin behind fails five tier-1 tests, the turned-IMU test among them; the one that leaves its keep-out box behind keeps compaction from terminating: no assertion fails, the harness's eda and tier-1 runs each hit its 900 s timeout, and a re-run of the eda unit tests ran past 120 s against about 1 s. In CI only the `tools-shard` job's 45-minute timeout (`ci.yml`, a hang guard) would stop it. The likely cause is that compaction's potential reads the box, which then never moves [inferred]. The centroid-target and compaction-off mutants, re-run on the merge, are still killed.

- **`COLOUR_ROUNDS` is not free [measured]:** at 4 rounds instead of 3, the fixture and every size sample move, because the colours' values order parts, not only tell them apart. Three is kept, as measured in `1313b`; the comment says so.

## 4. Reduction pass

Reduction pass (diff): −52 net (55 lines in, 107 out, over `pcb.rs`, `outline_tier1.rs` and `rename.rs`) · 4 duplicates removed · 2 dead paths removed · 0 protected kept · 3 findings below.

Run by a fresh subagent over the diff, read-only; each proposal applied was checked by the suites, the fixture and the four size samples byte-identical to the oracle, the corpus 24/24, and the touched guards' mutants (all killed).
- **Duplicates removed:**
  - the gap rule, written in `place`'s `taken` and again in `Rule::gap`: one `Rule::gap` (no-bound path; bytes unchanged);
  - "anchored to this IC", in four places: one `anchored_to`;
  - the MCU test, `lib_id.starts_with("MCU_")`, in five places: one `is_mcu`;
  - the grown-rectangle check in the tier-1 test: `Rect::grow`, now `pub`; also the tests' anchored-part builder and rotate-and-reverse loop, one each.
- **Dead paths removed:**
  - compaction's connector filter (connectors are not placed when it runs);
  - the anchored parts' tie by rank (`seq` never ties; the reference only keeps a hand-built design deterministic, as F1 had it).
- **The merge's addition** (the turned-IMU test and its weight row): nothing to remove; the test reuses the file's fixture, outline, gap and loop helpers, and reads U5's node once for its rotation and zones.
- **Findings, not changed:**
  - `tests/common/rename.rs`'s FNV and reference-class key repeat `pcb.rs`'s, across the crate boundary; joining them needs public API.
  - `outline_tier1`'s `ic` closure restates `is_ic` over library names.
  - The size samples run twice, at tier 1 (placement, gaps, loops) and tier 2 (gates). Different layers; kept.

## 5. Record

- **Base:** `457d644` (`main` after `1313b`). Commits: F1 `dc6f2994`; the build `58ee9fa0`; this FEEDBACK with `spike/1313b/` deleted; `main` `0829fe8` merged by `f6333d00`; the turned-IMU test `90a54c0b`.
- **`spike/1313b/` is deleted** (WO §1.4). Its figures are in `AET-WO-1313b.FEEDBACK.md`, and its files stay in history at `457d644`. `rename_perm.py` and `rename_len.py` live on as test code, `crates/aethelgard-cli/tests/common/rename.rs`. The headroom figure (89.20 × 86.25 mm with the witness port, USB FS and E0 as blocks) is `1313b`'s, outline only (ruling R5): the blocks were a prototype switch and are not rebuilt.
- **Gates on the branch after the merge:** fmt; clippy `-D warnings` with `with-tools`; `cargo test --workspace` 2176 passed, 0 failed; in the pinned image, the two Sentinel-03 tier-2 tests (2 passed, 28.2 s) and `4400d`'s `a_turned_keepout_is_where_kicad_draws_it` (passed); `ci/shard.sh plan` in the image: guard ok, 2372 tests over 8 shards, 180 of them tier 2; CI's package step run locally: every crate packages and the CLI builds from the packages. CI on `90a54c0b` (the merge and the turned-IMU test) green: run 44158034450, `ANSWER tier2_tests_passed=180 floor=180`. Before the merge: 2145 passed, 0 failed.
- **Outside the WO's literal fence, and why:**
  - the CLI's tests (`outline_tier1.rs`, `board_tier2.rs`, `tests/common/rename.rs`), which hold the acceptance's tier-1 and tier-2 tests;
  - `.github/workflows/ci.yml` (`TIER2_MIN` 180 after the merge: 178, plus `4400d`'s one, plus this WO's one, a comment line naming each) and `ci/test-weights.tsv` (seven estimated rows, `4400d`'s row kept beside them), as every new test needs;
  - the eda README's placer section;
  - `Rect::grow`, made `pub` for the tier-1 gap check (reduction, §4).
- **Not built (WO "is NOT"):** the test-pad rule; a capacitance-aware step (backlog R4: the 2-character sample is its first case); router changes; bottom-side parts.
- **`emit` under a bound takes about twice as long** (1.4 → 2.9 s release on the fixture). Writing a bounded board in a test build is slower still on a large outline: `under_a_bound_the_board_does_not_depend_on_the_parts_order` compares layouts for every order and bytes once, because each write of its 100 mm-wide board takes about 1.6 s in a test build (the writer's cost, not compaction's: compaction takes 1.5 ms on the fixture).

## 6. The merge with `4400d` (`main` `0829fe8`, the architect's 17:04Z items)

`main` merged by a merge commit (`f6333d00`). The only conflicts were `ci.yml` and `ci/test-weights.tsv`, resolved as ruled (item 1). `pcb.rs` merged without conflict. Before `4400d` landed, a trial merge with its branch head (`51357430`) was measured; `main` since then differs from it in no placer file and no file of this WO (an `aethelgard-ir` test, docs and the bench package), and every figure below was re-run on the merged branch itself.

**Item 2: a turned IMU moves whole, with its zones [read at source; measured].**
- `4400d` gives `Item` a `rotation` from the IMU's seat and stores its pads and its geometry at that turn (`items`). `Placed` gained no positional field: its `rotation` was already there, and a translation leaves it unchanged. `shift` moves all three positional fields (origin, keep-out box, courtyard).
- A footprint's zones are not stored with the placement. They are drawn when the board is written, from the placed origin and rotation (`footprint`, `Some("zone") => to_board(&mut c, origin, rotation, …)`), so a group moved whole carries them.
- `centroid_target` reads the anchor's pads, which are now at the turn, so an IMU's capacitors target its turned pins.
- **Test:** `a_turned_imu_is_compacted_whole_with_its_keepouts_inside_the_bound` (tier 1). The fixture with `board.forward` at the top and U5 turned each of its four ways: U5 at its declared rotation, the outline inside the bound, every point of each keep-out zone inside the outline, every placer gap and loop rule held. Its weight row is an estimate (10.90 s on the host, times 2.8).
- Every gate in the pinned image, on the merged binary [measured]:

| U5's X | Turn | Outline (mm) | Every gate | Least `extract` slack |
|---|---|---|---|---|
| right | 0 | 85.25 × 80.95 | yes | 9.2 pF (`IMU_MOSI`) |
| forward | 90 | 85.25 × 81.45 | yes | 4.2 pF (`IMU_MOSI`) |
| left | 180 | 85.25 × 80.95 | yes | 6.0 pF (`IMU_SCK`) |
| aft | 270 | 85.25 × 80.20 | yes | 0.8 pF (`IMU_SCK`) |

No margin is claimed: at 270° the slack is 0.8 pF, so a turn can bring a board near `extract`'s bound, as the size samples do (backlog R4).

**Item 3: the fixture under specification 16 [measured]: nothing re-pinned.** The fixture declares no `board.forward` and no IMU orientation, so U5 stays at 0. The outline is 85.25 × 80.95 mm and the name-free placement digest is unchanged (the tier-1 pin passes as it stood). Every gate passes in the image (tier-2 test, least slack 9.2 pF). The size samples are unchanged too: 2 refused by `extract` (−5.1 pF, 89.20 × 84.95), 6 passing (5.4 pF, 96.90 × 87.95), 8 refused by `emit` (`OutlineUnreachable`).

**Item 4: the oracle is pre-`4400d`.** The prototype oracle (`457d644` with `placer.patch`) reads specification 15 only, so its byte comparison (§1.3) no longer applies to the merged branch. What still bridges to it:
- the placement digest pinned at tier 1 was computed from the oracle's board, and is unchanged on the merge;
- the size samples' outlines and slacks are unchanged;
- with no bound, the corpus (24 specifications, all at version 16) is byte-identical against `main` `0829fe8`'s own binary, built apart: 24/24, 371 files. The fixture is still the only specification that declares a bound.
