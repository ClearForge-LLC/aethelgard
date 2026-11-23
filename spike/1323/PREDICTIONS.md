# AET-WO-1323 — predictions, committed before any run

Written 2026-11-23 ~09:45Z on `wo/aet-1323-router-backlog` at `main` `cd79024c`. Nothing has been run or built yet.

## 0. Readings taken before the predictions

- **R1 · The fab package's specification** [read]. `fab/sentinel-01-bench/package/MANIFEST.json` names `specification_sha256` `e635e940…`, the sha256 of `examples/sentinel-01.json`. A `fab/` byte can move only if that specification's emitted `.kicad_pcb` or `.kicad_sch` moves (the Gerbers, drill and position files are KiCad's from them).
- **R2 · Pairs** [read]. Of every committed specification, only `crates/aethelgard-cli/tests/fixtures/pair-standin.json` declares a non-empty `diff_pairs` (1 pair). `route_pair` runs only for a declared pair.
- **R3 · Vendored keepouts** [read, by regex over the footprint text; to be confirmed through `route::read` on emitted boards]. Three footprints carry keepout zones, five zones in all, each a rectangle, and each lies inside its own courtyard:

  | footprint | courtyard (mm) | pad field (mm) | keepouts (mm) |
  |---|---|---|---|
  | `InvenSense_QFN-24_3x3mm_P0.4mm` (kicad 10.0.7) | ±2.05 | ±1.775 × ±1.6 | F.Cu ±0.875 × ±0.795 |
  | `TDK_LGA-14_3x2.5mm_P0.5mm` (ours) | ±1.8 × ±1.55 | ±1.4 × ±1.15 | F.Cu ±0.925 × ±0.675; B.Cu ±1.55 × ±1.3 |
  | `Bosch_LGA-10_2x2mm_P0.5mm` (ours) | ±1.3 | ±0.925 | F.Cu ±0.6; B.Cu ±1.05 |

  The two B.Cu keepouts reach past the pad field but not past the courtyard, so (3)'s rule must admit "inside the courtyard or the field", never the field alone. **No §7 stop on (3)** on this reading.
- **R4 · How CI runs these tests** [read]. `cargo test` in the dev profile, with `aethelgard-eda` at `opt-level = 2` (`Cargo.toml`); `supply_tier1`'s tests share `s024l_board()`'s `OnceLock` in one process and are co-located in one shard (`ci/test-weights.tsv` L39).
- **R5 · The `1307` A4 adversary is not kept** (no `~/.cache/aet-1307` kit; FEEDBACK A4 describes it: a 249 mm board, 4 LQFP-144 copies, 288 random two-pad nets, built as `route::Board`). It is reconstructed here with a fixed seed and labelled a reconstruction. `1310`'s U1 patch is kept (`~/.cache/aet-1310/spike-1310.patch`): `unhop` takes `soft` and judges the flattened hop by `hard_open` when soft.

## 1. The baseline (§1.1)

- **B1.** `spike/1211d/measure/sweep.sh` selects **34** committed specifications at `cd79024c`, **25** emit (exit 0) and **9** exit 2, as at `1322`.
- **B2.** `spike/1319/harness/run.sh` runs unchanged on a `git archive` of `cd79024c`; base against base (two runs) is identical, so the router is deterministic in this harness.
- **B3.** A sha256 per emitted file is written beside the sweep (`spike/1323/sha256-base.txt`).

## 2. (1) The work bound

- **W1. The unit.** The count is A* expansions: each state `search()` pops and does not discard as stale, summed over every `search()` of one `route_with` call (pair runs, breakouts, escapes' searches if any, every rip-up's re-route). The bound is per routing, not per `emit` (the capacitance ladder may route a board up to 8 times).
- **W2. Where the time goes** (on the adversary, by a profile before the counter is fixed): **`search` dominates (> 80 %)**. If instead `zone_blockers`, `components` or `Router::new` dominate, an expansion count does not bound the time, and that goes to the stop.
- **W3. Determinism.** The count is a pure function of the board: identical in debug and release and on a second run.
- **W4. The heaviest committed routing** is a Sentinel-03 rung (`sentinel-03.json`, the 11–48 s routings `1314` measured) or `d3-24-i2c-12-spi`: on the order of **10⁸** expansions. The reference board (1.0 s) is on the order of 10⁶–10⁷. Low confidence in the magnitudes; high in the order (Sentinel-03 or d3 heaviest).
- **W5. The heaviest routing anywhere in `cargo test --workspace`** (logged by the instrumented build) is the same board class as W4 or a test board within 2× of it.
- **W6. The adversary** (reconstruction) fails closed by `RIPUPS` as in `1307` (a few hundred seconds on Furnace, hundreds of MB), with a work count **at least 10× the heaviest committed routing**, so a bound at a 2–4× margin refuses it well before `RIPUPS` and its test costs well under a minute in release.
- **W7. The name.** A new refusal kind, `RouterWorkExceeded` (an `EmitError` in `schematic.rs`, mapped in `pcb::unroutable`), naming the bound and the net being routed when it was crossed.

## 3. (2) U1

- **U1-a. Movers in the sweep:** `d3-24-i2c-12-spi` moves (as `1310` measured); every other specification that routes today, and **`examples/sentinel-01.json` in particular, is byte-identical**. Medium confidence for the sentinel-01 half; if it moves, (2) is not built (§7).
- **U1-b. No board that routes today stops routing**; no board refused today starts routing in the sweep.
- **U1-c.** A board on which bans alone (U1 reverted) fails and U1 routes may not exist among committed boards (`1310`: both route d1). If none is found, U1's mutant is killed only by bytes, and that is said at the stop.

## 4. (4) The pair's stub

- **S1.** `1311` F2 reproduces on `main`: `paired(12 mm)` plus `S`'s `/S` pads at x = 7 mm, y = 3 and 8 mm, `route` refused by `connectivity` with "/N: track F … joined to nothing".
- **S2. Who lays it** (low confidence): `route_pair` itself, while the pair is laid on the bare board (`S`'s pads change the run or the breakout), not a later net's rip-up, since fixed nets are never ripped. A bisect (the segments after the pair loop, and after each later commit or rip) decides it.
- **S3.** In a sweep of `S`'s placements, a minority stub today.
- **S4. Movers:** only boards with a declared pair can move; among committed specifications that is `pair-standin.json` alone, and its bytes stay identical (its pair routes with no stub today). **No `fab/` byte moves.**

## 5. (5) The supply gate's split

- **P1.** `extract` on `s024l_board()` builds `copper()` once per net (VDD_3V3, GND) and calls `bound()` once per pin (16 + 11), each building and solving its own network. The solve (`Network::bound`) dominates (> 70 %), the network build next; the 60 mV and 165 mV runs split identically (the limit enters only the verdict).
- **P2.** The test-side `emit` of `s024l_board()` (paid once per process) is a minority of the tests' time.
- **P3.** Any cut that keeps every verdict, number and note bit-identical shares work across pins of one net within one `extract` (the network's structure is the same for every sink of a net). Whether the solver's iteration can be shared without moving a float is unknown until read; if it cannot, the only cut left may be structural sharing of the network build, and the gain is the build's share only.

## 6. Stops expected at §1.2

The planned stop after §1.2 fires in any case. No other §7 condition is predicted to fire.
