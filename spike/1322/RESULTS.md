# AET-WO-1322 — results, beside `PREDICTIONS.md`

Measured 2026-11-23 on Furnace. The base is a `git archive` of `main` `7fbe5d11`, and the branch a `git archive` of `29d137c0` (the code as it emits; later commits add tests and kits only, inside `#[cfg(test)]` and `spike/`). Each tree had its own target dir. Evidence is in `~/.cache/aet-1322/`.

## The baseline (§1.1)

- **B1, hit.** The sweep selects 34 specifications: 25 emit (exit 0) and 9 exit 2.
- **B2, hit.** `run.sh` ran unchanged on the base: `d4diff` 277 cases (926 files), `d1319` 84 cases (327 files), the sweep 280 files.
- **B3, hit.** `bulk.sh` base against base: `ceramic identical: emit exit 0, 3 files, the note in 1`; `electrolytic` the same.
- **B4, hit for every emitted byte; one refusal text moved, as (4) asks.** Branch against base:
  - `identical: 280 files` (the sweep);
  - `d4diff identical: 277 cases, 926 files`;
  - `bulk.sh`: both variants identical;
  - `d1319 DIFFERS: 1 entries, 1 cases`. That case is `d1-ipropi-undeclared/summary.txt`: it records `Drawing::of`'s refusal, whose text is (4)'s rewrite. No drawn file moved. I predicted all four identical, and did not see that `d1319` records this refusal's text.

## (2) `prepare_read` on a long name

`crates/aethelgard-eda/tests/prepare_read_memory.rs` is one test in its own binary: a footprint whose reference is 10 kB, with 100 000 pads, and the peak (`VmHWM`) before and after `prepare_read`. Both binaries were run directly, debug. This is the test's first form; its final form (ten nested footprints, 15 000 pads) is under Mutants below.

| | `main`'s `board.rs` | branch |
|---|---|---|
| peak growth in `prepare_read` | **982 968 KiB** (test fails) | under 2 MB (test passes) |
| process max RSS | 1 027 632 KB | 46 268 KB |
| time | 0.58 s | 0.07 s |

- **M1, hit:** about N × 10 kB (983 MB at N = 100 000).
- **M2, hit:** under 100 MB, and independent of the name's length (the name is held once).
- **M3, hit:** every `board::tests` case passes with its id unchanged.

## (3) The echo, and `MAX_ITEMS`

- **E1, hit.** `check drc --board`, no KiCad on `PATH`:

| probe | base stdout | branch stdout |
|---|---|---|
| a 14 MiB reference on a pad with a local clearance (`LocalClearance`, the name in `id` and `message`) | 29 360 327 B | 697 B |
| a 12 MiB layer name with `"` and `\n` in it (`MalformedBoard`, `Refusal::Layer`) | 13 693 350 B | 465 B |

  The branch writes the first 200 bytes as `{:?}` writes them, then `… (the first 200 of N bytes as escaped)`.
- **E2, hit.** Every committed expected message and id passes unchanged. That covers the eda tests, the CLI's `strict_tier1` corpus, `emit_tier1` and `through_hole_tier1` (workspace run, §7 of FEEDBACK).
- **E3, hit.** `spike/1322/count.py` over the 154 boards the base emitted (sweep, `d4diff`, `d1319`) and every committed `.kicad_pcb`:
  - the largest is `d3-24-i2c-12-spi`: 3 351 copper items, 800 courtyard edges and 0 keepouts, 4 151 in all;
  - so the cap of 20 000 is far off. `1311` F's board (19 803 copper items and a 90 000-point courtyard) is now refused by the count.
- **After the adversarial pass (B1, its finding):** the echo bound was per label, so 250 nested footprints still wrote 130 153 B on stdout (base: 5 007 153 B, measured by the adversary). `board.rs` now bounds the rendered chain whole (`MAX_CHAIN_BYTES`, 800 B, then `of … (N more carriers)`), tested by `board::tests::a_deep_chain_of_carriers_is_bounded_whole`.
- **The count, folded (reduction pass):** the early copper-only check and the total check were two mechanisms for one cap. There is now one check, after the parts are read; reading them is linear.

## (1) Unreachable guards

- **U1, missed in its premise.** `validate` runs in `place()` (`emit`, `emit_moved`, `placement`), not in `design()`.
  - `Drawing::of`, `decoupling_picofarads` and `extract`/`loops`/`supply` reach `design()` without it.
  - So the guards that `validate` shadows are reachable by a library caller. They are labelled `belt-and-suspenders:`, each with a test that reaches it directly, and none is deleted for that reason.
- **Only two guards were dead, both by code order:** `crossing`'s "bus has no device" and "no SPI bus" (`behind` and the SPI-device loop run first).
  - The device lookup is now one function, `device_on`, which `behind` and `crossing` share.
  - The bus lookup, and `crossing`'s second "not a translator", are `EmitError::Internal` with the reason stated.
- **The adversary's probes** (13 hand-written specifications, and 4 hand-edited topologies × 2 specifications) reached neither new `Internal`, and every output was identical to base.

## (4)–(7)

- **I1, hit.** The guard stays; its text names the tie and where `validate` admits it. `d1319`'s `d1-ipropi-undeclared` records the new text (B4 above).
- **N1, hit.** Already on `main` (`4300i`). The mutant that empties eda's claim loop is killed by `cli/tests/hot_plug_tier1.rs`.
- **D1, hit.** `1319`'s M2, restored (BMP581's second decoupler dropped, its `DECOUPLERS` count lowered to 1): run exit 0, **killed by bytes**. Against the branch's own output, sweep differences are on the BMP581 boards (`hse-crystal-16`, `logger-4600`, `sentinel-02-own`, `-shaped`, `sentinel-03`, `sentinel-02-bench`, …), `d4diff` differs in 35 cases and `d1319` in 3.
- **D2, hit.** A doubled entry is refused by `check`, and mutant D1 is killed by the new test only.
- **P1, hit.** `pcb::unroutable` is the mapping. Its test pins `kind`, `id` and the message. Mutant P1 is killed.

## Mutants (`spike/1322/mutants.sh`, sized to the diff)

**19 of 19 killed**, each by its named test failing, none by a build break: B1–B4, K1, K2, E1, E2, S1, R1, R2, C1, C2, D1, X1, L1, I1, P1, N1.
- B1 (a copy of the chain per child) **survived once**. The echo bound had made the flat test's labels 230 B, so a per-child copy cost 23 MB, under the test's bound.
  - The test now nests ten named footprints over 15 000 pads. B1 grows the peak by 39 640 KiB against a 16 MiB bound, and is killed.
  - `main`'s `board.rs` grows it by 1 467 944 KiB (run in a capped scope).
- An uncapped run of a 50-deep, 100 000-pad form against `main`'s `board.rs` grew the peak by **45.6 GB** on Furnace before failing. That is why the test is sized as it is.
