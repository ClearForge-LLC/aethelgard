# AET-WO-3401 — Sentinel-01 as ordered ("V0+"): the lean probe with its bench access, its fabrication package, the order sheet and the bench plan

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3401-sentinel-01`.
**Author:** Claude (architect) · **Builder:** strong tier (the `3400` builder, reused), with a targeted adversarial pass.
**Phase:** the first fabricated board (roadmap §3). Written 2026-11-17. **Cadence:** build. §1 runs now, and §2 after `AET-WO-1312` merges.

**Grounds:**
- **Scotty, 2026-11-17, ruling `3400` Q6:**
  - Path (a), the lean probe now, "with the caveat that we still add in the systems for testing that help us with Sentinel-02".
  - Then (c), the loaded board as Sentinel-03 at four layers.
- **The architect's scoping, which Scotty accepted:** Sentinel-01 is V0 plus the cheap bench access. The power tree and the flight sensors stay on Sentinel-03.
- **`3400` FEEDBACK:**
  - V0 (`examples/sentinel-01-minimal.json`) passes every gate at two and four layers.
  - NRST on J2 passes (probe `p-nrst`).
  - The EXT bus (10 kΩ at 100 kHz on a DS-009 I2C port) passes in V5.
  - V0 already takes `VDD_3V3` from J3 pin 1, so a bench supply can sweep the MCU rail for the BOR thresholds.

> **What this is:** the board Scotty orders, as a specification of its own, with every gate passing. It also carries the papers the order and the bench need.
>
> It is NOT:
> - a change to `examples/sentinel-01-minimal.json`, which stays the quickstart's reference and P1's byte-for-byte fixture;
> - the power tree, the flight sensors or the witness (Sentinel-03).

## 1. Now: the specification without bench access

1. **`examples/sentinel-01.json`.** Start from `sentinel-01-minimal.json` and add:
   - NRST on J2;
   - a second I2C bus (`EXT`, 10 kΩ pull-ups, 100 kHz, declared rise time and capacitance, as V5) on a DS-009 I2C connector, with no device on it. This is the weak-pull-up operating point; a sensor or a coupon plugs in.
   - Keep `bor_level`, the BOR acknowledgement and every other V0 field.
2. **Measure:** run the five commands, plus parity, support and `extract`, at two layers. Record the outline.
3. **Decide where its fabrication package lives.** Either `boards/sentinel-01/` changes, which moves P1's byte-for-byte fixture, or the board has a directory of its own (`boards/sentinel-01-bench/`, or another name you justify), generated and checked by a tier-2 test as `boards/sentinel-01/` is. **Measure the CI cost of a second regenerated board and choose. STOP if it adds more than 3 minutes to any shard.**

## 2. After `1312`: bench access, then the papers

1. **Add to the specification:**
   - a test point on every declared net: both buses' SDA and SCL, the four SPI lines, flash CS, console TX and RX, `VDD_3V3`, NRST and BOOT0;
   - at least two `GND` test points;
   - BOOT0 on a connector pin;
   - `mcu.reset_switch: true`.

   Every gate passes at two layers.
2. **The fabrication package:** `verify --fab-out` emits it, and it is committed where §1.3 decided, with `FAB-NOTES.md` (stackup, BOR programming, the acknowledged ledger rows).
3. **`ORDER.md`, beside it.** For Scotty, who orders.
   - **The bare board:** outline, area, layer count, thickness, finish and the files to upload.
   - **The BOM:** reference, value, footprint, maker part number, and one distributor link per line, dated. A line with no part chosen yet is flagged, not guessed.
   - **Assembly:**
     - the fabricator's BOM and placement (CPL) files, if `verify --fab-out` can emit them. If it cannot, say what is missing, as a finding;
     - which lines are hand-solder-friendly;
     - the plan: some boards fab-assembled as the control, the rest built by Scotty.
   - **The open quotes:** the rows from `3400` FEEDBACK §10.5 that this board needs.
4. **`BENCH.md`.** The rows to run when the boards arrive, each tied to the prediction it tests:
   - **`3302`'s first-silicon rows, now on our copper:** programming by SWD with connect-under-reset, the register image read back, `BOR_LEV`, and the SPI1 `MODF` fix (the flash's JEDEC ID reads back).
   - **The BOR sweep:** a bench supply on J3 lowered until reset, against DS12117 Table 115 and `bor_level`.
   - **Rise time at both operating points** (2.2 kΩ/400 kHz and 10 kΩ/100 kHz) on the test points, against `extract`'s bound and the campaign's prediction (`RUL-19`).
   - **The H7's I2C clock-high threshold** (northstar §2's second divergence) on the weak bus.
   - **USART boot through BOOT0 and the console.**

   Each row names its instrument, its pass line and the corrections-table entry it would add on a disagreement.

## 3. Acceptance

- `examples/sentinel-01.json` passes every gate at two layers, with no finding, parity, support and `extract` included.
- The fabrication package regenerates byte for byte in CI.
- `ORDER.md` and `BENCH.md` are complete, or name each gap.
- `sentinel-01-minimal.json` and `boards/sentinel-01/` are byte-identical, unless §1.3 chose otherwise and the architect agreed.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 4. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`, `INV-07`; `RUL-03`, `RUL-19`, `RUL-20`.
- **Fence:**
  - `examples/`;
  - the new board directory;
  - tests;
  - `ORDER.md` and `BENCH.md`;
  - FEEDBACK.

  No crate, unless a gate needs a test fixture's path.
- **Running beside you:**
  - `AET-WO-1312` (the engine half): merge `main` when it lands, then do §2.
  - `AET-WO-1801` (the release) moves the engine version in every emitted board. Merge it and regenerate.
- One PR, left unmerged. Write `~/.cache/aet-3401/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A gate that passes only because the new bus has no device.** Plug in a declared off-board load within its budget and re-gate.
- **A BOM line whose maker part does not match the footprint emitted.**
- **A bench row whose pass line cannot fail.**

## 7. Flag-and-stop conditions

- **§1.3's CI cost exceeds 3 minutes on a shard.**
- **The board does not route at two layers once the test points are in.**
- **The fabrication package cannot be made byte-stable.**
