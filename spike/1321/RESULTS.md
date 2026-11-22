# AET-WO-1321 — results, beside `PREDICTIONS.md`

Measured 2026-11-22 on `main` `a41a8a8d` (base) and the branch at `d514f7a0` (the code head; later commits are the kit and docs). Evidence is in `~/.cache/aet-1321/`.
- **Base:** a `git archive` of `a41a8a8d`.
- **Branch:** a `git archive` of `d514f7a0`.
- Each tree had its own target dir, and each log names its own `Compiling aethelgard-eda (<tree>)`.

## The baseline

- **B1, hit.** The sweep ran 34 specifications.
- **B2, hit.** `run.sh` ran unchanged, with the shim's anchor in place: `d4diff` 277 cases and `d1319` 84.
  - `d1319` writes 331 files; `1320` counted 328. Both trees write 331, and the files compare identical.
- **B3, missed.** `d1-drv8876.json` is in the sweep, but it declares `board.drawn: false`, so `emit` exits 2 (`DrawsNoBoard`). **The sweep never draws `declared_bulk`.**
  - So the fold's bytes are judged by `bulk.sh`.
  - It emits that fixture on a drawn board, as `pin_devices_drawn_tier1.rs`'s `d1_drawn()` makes it (the `drawn` key removed).
  - It does so with the ceramic bulk and with that test's electrolytic, on both binaries, and runs `diff -r` on the outputs.
  - Base against base: identical, `emit` exits 0, and CVM2's note is in the BOM.

## (1) The reverse check

- **C1, hit.** `check(rows, counts)`. The `const` assertion over `ARMS` and `DECOUPLERS` compiles, and no row is touched.
- **C2, hit.** Every existing arm test passes, each with its expected message unchanged.
- **C3, hit after a fix.** The new test, `a_decoupler_entry_with_no_bus_row_is_refused`, refuses its three bad tables at compile time and accepts the reversed control.
- **C4, hit after the same fix.** `mutants.sh`: R1, R2 and R3 are all **KILLED** by the new test.
  - At `9dad9223`, **R2 survived.** `FIRST` named `"DRV8876"`, but the pin row's part is `"DRV8876PWPR"`. The entry had no row of any kind, so the "a pin row is not a bus row" case never ran.
  - The fresh reviewer found it on its own; its report was CONFIRMED by planting.
  - Fixed at `d514f7a0` (`DRV8876.part`); R2 is killed now.

## (2) The fold

- **F1, hit.** `two_pin(.., note)`. The power tree's three callers pass `None`, and `declared_bulk` calls `two_pin`.
  - The removed `Some(row)` pattern always matched: the `match` above it returns on `None`.
- **F2, hit.** N1 (no note) is killed by `d1_is_drawn_with_its_declared_bulk`, and by `bulk.sh`: both variants differ, 2 entries each.

## (4) `1319`'s mutants, with the path corrected

| mutant | run exit | verdict | by |
|---|---|---|---|
| M1, WP and HOLD swapped | 0 | KILLED | all three: the sweep (23 specifications move), `d4diff` (73 cases), `d1319` (6 cases) |
| M2, BMP581's second decoupler dropped | 101 | KILLED, **at compile time** | `error[E0080]: evaluation panicked: a bus row whose decoupler count is not its part's` (`1320`'s check). No bytes were measured; `compare.sh` reports `0 entries, 0 cases` over missing output. |
| M3, CVCP and CVINT reordered | 0 | KILLED | `d1319` DIFFERS: 130 entries, 32 cases (sweep and `d4diff` identical) |

As predicted (M). Since `1320`, M2 no longer tests the harness's bytes; the compiler refuses it first.

## Re-run

- **R1, hit.** Branch against base:
  - `identical: 280 files`;
  - `d4diff identical: 277 cases, 926 files`;
  - `d1319 identical: 84 cases, 331 files`;
  - `bulk.sh`: `ceramic identical: emit exit 0, 3 files, the note in 1`; `electrolytic identical: …` the same.
- **R2.** See FEEDBACK §6.
- **R3, hit.** `crates/`: +97 −35, net **+62**.
