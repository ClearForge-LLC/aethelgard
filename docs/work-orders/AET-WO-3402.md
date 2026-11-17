# AET-WO-3402 — Sentinel-01 before its order: a second GND test point, the ledger rows in FAB-NOTES, and each I2C bus at its own declared speed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3402-preorder`.
**Author:** Claude (architect) · **Builder:** strong tier (the `3401` builder, reused), with a targeted adversarial pass.
**Phase:** the first fabricated board (roadmap §3), before Scotty orders Sentinel-01. Written 2026-11-17 from `AET-WO-3401`'s close (PR #190, merged `ac5f8c8`). **Cadence: build, with one measured stop (§1.3).**

**Grounds:**
- **The review of 3401 (PR #190, comment 6940837600), R2:** `TestPointNetRepeated` (`ir/src/bench.rs:178`, `AET-WO-1312`) refuses a second test point on `GND`. Sentinel-01 has one GND pad, far from the I2C pads `BENCH.md` B6 probes. Ground is the net where several probe points are the norm.
- **3401's F3 (Q5):** `FAB-NOTES.md` lacks the acknowledged ledger rows (`acknowledge_unwitnessed`). `AET-WO-3401` §2.2 asked for them.
- **3401's F4 (Q8):** the generated firmware drives SENSORS (I2C1) at 100 kHz, not its declared 400 kHz. `svd-hal/src/board.rs`'s `generate` gives every I2C peripheral the lowest declared speed, because "the topology does not say which peripheral serves which bus". The solved topology does carry `i2c_timings` per peripheral (version 8), `i2c_speeds` per bus, and the pin assignments that put each bus's SCL and SDA on a peripheral's pins.

> **What this is:** the three fixes Sentinel-01 needs before its order:
> - a second GND test pad, near the I2C pads;
> - the ledger rows in `FAB-NOTES.md`;
> - firmware that runs each I2C bus at its own declared speed.
>
> Sentinel-01's package is then regenerated.
>
> It is NOT:
> - a change to ArduPilot's `HAL_I2C_MAX_CLOCK` rule (one clock by ArduPilot's design, ruled on #189);
> - a change to any gate's verdict on another board, except where §1.3 measures a byte move and the architect rules it.

## 1. Build

1. **`TestPointNetRepeated` exempts `GND`.**
   - A specification may name `GND` on any number of test points, each with its own reference. Every other net stays at one.
   - A tier-1 test covers both sides.
   - No specification version: this accepts what was refused and changes no shape.
2. **The ledger rows in `FAB-NOTES.md`.**
   - `verify --fab-out` writes the specification's `acknowledge_unwitnessed` entries into `FAB-NOTES.md`, each with its register, class, fields and reason, as `AET-WO-3401` §2.2 asked.
   - A board with none writes a line saying so.
3. **Each I2C peripheral at its own bus's declared speed. Measure first, then STOP.**
   - Derive which peripheral serves which bus from the solved topology alone (`RUL-16`): the bus's SCL and SDA nets, and the assignments that put them on a peripheral's pins. Where the derivation is ambiguous or absent, the generator refuses by name. It never falls back to the lowest speed (`INV-06`).
   - **STOP** with:
     - whether the derivation needs a solved-schema field;
     - the list of every committed board and golden whose bytes the change would move (expected: only boards with two I2C buses at different declared speeds);
     - each `TIMINGR` before and after.
   - After the ruling, build it. Every moved golden is listed in FEEDBACK.
4. **Regenerate Sentinel-01** (`examples/sentinel-01.json`) with a second `GND` test point near TP11–TP14, the EXT and SENSORS pads, that still passes every gate at two layers. Then regenerate its package through `bench-package` as `3401` did. Update:
   - `ORDER.md`: the test-point count;
   - `BENCH.md`: B6's ground spring to the new pad; B2's `TIMINGR` per peripheral, I2C1 at 400 kHz and I2C2 at 100 kHz.

## 2. Acceptance

- Sentinel-01 passes every gate at two layers, with two GND pads and its firmware's I2C1 at 400 kHz.
- Its package regenerates byte for byte in CI and binds per PR (`bench_board_tier1`).
- `FAB-NOTES.md` carries the ledger rows.
- No other board's bytes move except as the §1.3 ruling allows.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`, `INV-07`; `RUL-03`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-ir` (`bench.rs` validation; a solved-schema field only if ruled at §1.3);
  - `aethelgard-svd-hal` (`board.rs`'s I2C timing);
  - `aethelgard-eda` (`fab.rs`'s `FAB-NOTES`);
  - `examples/sentinel-01.json`;
  - `fab/sentinel-01-bench/`;
  - `boards/sentinel-01-bench/`;
  - tests;
  - FEEDBACK.
- **Running beside you** (coordinate through the bus):
  - `1313` (ir's `board` type and validation in `lib.rs`; `pcb.rs`; specification 15);
  - `4400b2` (pincheck; ir validation; one svd-hal `board.rs` test moves);
  - `1604` (CLI; `eda/src/fab.rs` at its §1.3 for fab-format files, the same file as your §1.2);
  - `2302b` (evolve, chaos);
  - `2303` (a spike).
- One PR, left unmerged. Write `~/.cache/aet-3402/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A second test point on a non-GND net** still refused.
- **A board whose two I2C buses share a peripheral,** or a bus with no derivable peripheral: refused by name, never the lowest speed.
- **A `TIMINGR` that runs a bus faster than its declared speed.**
- **A FAB-NOTES ledger row that differs from the specification's entry.**

## 7. Flag-and-stop conditions

- **§1.3 (the I2C derivation and its byte moves).**
- **The second GND pad does not route at two layers.**
