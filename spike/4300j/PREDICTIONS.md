# AET-WO-4300j — predictions, written before any run (§1.1)

Written 2026-11-23 on `main` `cd79024c`, before the kit in this directory had run.

**Grounds:**
- `4300i`'s record: FEEDBACK §2–§4, and `spike/4300i/out/` (motors-summary, pull-refs, null-sweep);
- code read on `cd79024c`: `motor.rs`, `offboard.rs`, `silicon::implied_pupd`, `hwdef::generate`, `SolvedMotorOutput`;
- the held pages, read rendered (`DERIVATION.md`).

No `validate`, `solve` or `emit` had run on this branch.

**Not blind:** P8 to P10 are computations, not forecasts. `derive.py` ran before this file was written, and its output is `derive.out`.

## The universe and the baseline

- **P1.** The committed current specifications number 34 (`universe.sh`, copied from `4300i`). This is `4300i`'s null-sweep count.
- **P2.** Every exit code and refusal kind equals `4300i`'s merged-head record (`spike/4300i/out/after/baseline.tsv`).
- **P3.** Every emitted tree equals that record's, hash for hash. Main has merged 1322, which reports "no emitted byte moved", and 2310 and 1914, which touch the twin and CI only.
- **P4.** Two runs of each command are byte-identical.

## The motor boards

- **P5.** 10 boards declare `motor_outputs`, with 40 outputs between them. Seven of those boards emit a PCB, with 28 drawn motor nets. `hse-external-8` and `d1-drv8876` validate but draw nothing, and `d2-drv8833` is refused at validate by design.
- **P6.** Every one of the 40 declares `CRCW0603_F_K` at 40 200 Ω to `GND`.
- **P7.** Every motor board's `mcu.supply` is `VDD_3V3`, at 3300 mV ±50 ‰. So one bound serves every committed board.

## The derivation (computed; `derive.out`)

- **P8.** The floor is 211 824 Ω, at VDD min 3135 mV, with a margin of 10 % of VDD.
- **P9.** The ceiling is 895 714 Ω, at VDD min, with the same margin.
- **P10.** The value is **221 kΩ** E96, `CRCW0603_F_K`. That is the smallest E96 value whose worst case (±1 % tolerance, +2 % drift) lies inside both bounds. No §7 E96 stop.

## Which bytes a build would move (forecast for §1.3, judged by the null sweep)

- **P11 · versions:**
  - specification 28 → 29;
  - solved topology 17 → 18 (the fitment rides in `SolvedMotorOutput`, because both generators read the topology alone, `RUL-16`);
  - power-parts 9 → 10 (a jumper row for the link, and the RPD band replaced by the bound's inputs);
  - `parts.json` (13) does not move.
- **P12 · specifications:** all 34 move by their `schema_version` line. The 10 motor boards also move in each output's `pull`: the value, the link and the fitment.
- **P13 · solves:**
  - every topology moves by its `schema_version` and its specification hash;
  - motor boards also move by the fitment on each `motor_outputs` entry;
  - no `/assignments/*/pin` moves on any board.
- **P14 · emits:** emitted files move on the 7 drawn motor boards only (schematic, PCB, BOM, netlist). On every other board emit is byte-identical, as it was across `4300i`'s version bump.
- **P15 · Sentinel-03's outline** grows by the four links and stays inside its 100 × 100 mm bound. The other drawn motor boards may grow too; RESULTS measures each.
- **P16 · generated firmware:**
  - svd-hal goldens move by their hash and version lines; a motor `PUPDR` word moves only on a fixture declared `open`;
  - hwdef fixtures with motors gain the comment line; `PULLDOWN` appears only on an `open` output.
- **P17 · `boards/` and `fab/sentinel-01-bench`** move by their specification and topology hash lines only. No Gerber, BOM, placement or ELF byte moves, and the package needs a dispatch.
- **P18 · no committed board's pin moves.** Neither the value nor the fitment enters the solver.
