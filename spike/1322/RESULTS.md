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

`crates/aethelgard-eda/tests/prepare_read_memory.rs` is one test in its own binary: a footprint whose reference is 10 kB, with 100 000 pads, and the peak (`VmHWM`) before and after `prepare_read`. Both binaries were run directly, debug.

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
