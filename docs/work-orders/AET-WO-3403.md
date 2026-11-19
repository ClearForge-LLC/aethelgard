# AET-WO-3403 — Sentinel-01 as ordered: the bench plan made able to fail, U2 left empty, and a BMP581 on J4 through a bench specification

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3403-s01-bench-plan`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P3 bench (`3302`'s method) for Sentinel-01, the first fabricated board. Written 2026-11-19. **Cadence: build. The boards are in fabrication now, and this lands before they arrive.**

**Grounds:**
- **Scotty ordered Sentinel-01 on 2026-11-19:**
  - the bare boards from `fab/sentinel-01-bench/package/`;
  - the parts on `ORDER.md`.

  He ordered before the order-gate review (roadmap §5, line 434). The review was held after.
- **The review** (a fresh read-only session; `~/.cache/aet-rev-s01/REVIEW.md` on Furnace) found **no reorder**:
  - the copper was checked against the makers' drawings;
  - a label-free connectivity pass found 0 shorts and 0 opens;
  - the package bytes equal `MANIFEST.json`.

  **The plan for the bench is not ready.** These are the review's findings this order owns:
  - **F1 (bench-blocking):** B1 pass 3, B4 and B8.1 expect the console to print a boot record. It never does.
    - The console sends `CONSOLE` + `00 FE 7F 80 FF` and echoes.
    - The records live in RAM (`AETHELGARD_BOOT`) and are read over SWD.
  - **F2 (bench-blocking, B7):** B7's method folds the I2C controller's tSYNC2 delay and the HSI's error into the threshold it claims to measure. Its uncertainty exceeds the ±0.2 V band. The fix it names is two edge rates, solved jointly.
  - **F3:** B1 passes with NRST open on J2.
  - **F4:** B6 has only an upper bound, so a 2.2 kΩ fitted where 10 kΩ belongs passes.
  - **F5:** B8.1 cannot see a missing R8, and B8.2 cannot fail.
  - **F6:** no row witnesses I2C1 (PB6/PB7 AF4) on silicon, and B2 is common-mode with the generator.
  - **F9–F11:** supply and cable hazards:
    - J3 feeds VDD with no protection, 4.0 V absolute;
    - three identical GH-4 headers sit side by side;
    - J2 is the project's own pinout, not Dronecode's;
    - SW1's pairing is unread.
  - **F14** (`4240`'s F-VOS): the ordered ELF runs at VOS3. B2 records `PWR_CR3`, `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR`.
  - **U1:** U2 left empty boots. The ordered ELF NACKs 0x76 with a bounded wait (`3302` measured that path on silicon). U3, the TMP117 at 0x48, is a debugger-driven I2C1 witness with no firmware change: register 0x0F reads 0x0117.
  - **U3:** a BMP581 breakout on J4 needs no copper, but needs three things:
    - firmware that addresses it, from a bench-only specification as `3304` did (`examples/sentinel-02-bench.json`);
    - EXT pinned to PB10/PB11, so the solver cannot move the bus off J4;
    - a hand-pinned GH→Qwiic cable. A straight adapter reverses the breakout's supply.
- **The BME280 is out of stock everywhere.** Scotty chose the BMP581 (#237, comment 7013295272). It does not fit U2's LGA-8 2.5 × 2.5 mm pads. So U2 stays empty on these boards.

> **What this is:**
> - `BENCH.md` corrected so every row can fail for the reason it names, with the review's F1–F6 and F9–F11 applied, and each row's source class stated (`3304`'s labels);
> - the U2-empty path written into the plan: what boots, what is lost, and the TMP117 witness over SWD;
> - a bench specification for a BMP581 on J4, built by `main`'s CLI, with its ELF's sha256, predictions and the cable's pin map;
> - the review stored as the record in `docs/reviews/`.
>
> It is NOT:
> - a change to the ordered copper, `package/`, `MANIFEST.json` or `examples/sentinel-01.json`;
> - the engine work the review recommends (F7: a second AF source for `verify`; a device declared behind a connector). Those are §7.1 rows;
> - the bench run. That is Scotty's.

## 1. Build

1. **Store the review.** Copy `~/.cache/aet-rev-s01/REVIEW.md` verbatim to `docs/reviews/2026-11-19-sentinel-01-as-ordered.md`. Add one header line naming its source path and session. Change nothing else in it.
2. **`fab/sentinel-01-bench/BENCH.md`, row by row.** Apply each of the review's recommendations, or say in FEEDBACK why not:
   - **F1:**
     - the console's 12 bytes `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF` at 115200 8N1;
     - the records read over SWD: word 0 = `0x2100B007`; record 1 = (`0x22020000`, `0x00EF4017`); record 0 per U1;
     - the exact `probe-rs` read commands.
   - **F2:** B7 rewritten:
     - two edge rates (J4 empty, then a known C0G on TP11), solved jointly for threshold and delay;
     - the kernel clock calibrated from the SCL low phase;
     - a band derived and stated, not assumed.
     - If the derivation cannot reach a band a correct board passes, say so and make the row a measurement with no pass line.
   - **F3:** TP3 at or below 0.3 V while the probe holds reset.
   - **F4:**
     - every pull-up read from its test pad to TP4 before power (2.2 k / 10 k);
     - the implied C recorded on every B6 line, pass or fail.
   - **F5:** TP2 to TP1 = 10 kΩ ±1 % unpowered, or the 100 kΩ drive test.
   - **F6:** a new row:
     - a scope or the FX2 analyzer on TP13/TP14 across reset;
     - the `0xEC` address phase at 400 kHz;
     - with U2 empty: NACK, and record 0 = (`0x22020001`, 0);
     - the TMP117 read over SWD (U1) as the positive I2C1 witness.
   - **F9–F11:** a "before power" section:
     - supply at most 3.6 V, current-limited;
     - each cable labelled for its header;
     - J1 and J3 never swapped;
     - J2's own pinout (not a Dronecode cable);
     - SW1's pads checked with a DMM before fitting.
   - **F14:** B2 gains the three PWR registers, labelled as `4240`'s bench row B-2.
   - **Every row** carries its source class: twin, rule, generator, repeat, or not predicted. A prediction that could not fail is rewritten or labelled so.
3. **U2 empty.** In `BENCH.md` and `ORDER.md`, say U2 is not fitted on these boards and why. State what each row expects with it empty, from the ordered ELF's code path, cited by `file:line`.
4. **A BMP581 on J4: the bench specification.**
   - `examples/sentinel-01-ext-bmp581.json`: `examples/sentinel-01.json` plus one BMP581 on `EXT` at the address its breakout's SDO strap gives.
     - EXT's pins are pinned to PB10/PB11.
     - Say which address you assume and how Scotty confirms it.
   - **It describes a board that was never made.** Its own `verify` verdict says nothing about J4's copper with a breakout on it. Say so in the specification's purpose and in `BENCH.md`.
   - Generate and build it with `main`'s CLI in the pinned image, and record the ELF's sha256.
   - **The twin fault:** a BMP581 through `droop-bor3` is stamped 1 ns early (`4201`, #237). Run `verify` with that scenario.
     - If it refuses, say so and run the rest.
     - Do not change the twin: it is `4400g`'s area, and the fault is a §7.1 row.
   - **Predictions,** in `3304.bench.md`'s form, as a section of `BENCH.md` or its own `fab/sentinel-01-bench/BENCH-EXT-BMP581.md`, your call:
     - the BMP581's identity record;
     - the I2C2 transaction as `sigrok-cli` prints it;
     - the pull-up arithmetic with the breakout's pull-ups in parallel with R3/R4, and the bench reads the breakout's values.
   - **The cable:** a table, J4 pin → breakout pin, by function. Never "straight through".
5. **Specification 17.** `4400g` (#238) bumps every specification to 17 and makes `examples/` walkers refuse an unparseable one.
   - If it merges before you are READY, merge `main` and bump your new specification.
   - If not, write it at 16 and say so on the bus. The architect sequences the merges.

## 2. Acceptance

- **Every BENCH row can fail for the reason it names.** FEEDBACK lists each row, the fault that fails it, and its source class.
- **Nothing ordered moves:** `git diff main -- fab/sentinel-01-bench/package examples/sentinel-01.json boards/` is empty.
- **The new specification:**
  - `validate`s;
  - generates, builds and simulates in the pinned image;
  - its ELF sha256 is recorded;
  - its `verify` runs and the result is reported as it is.
- **The four tests that walk `examples/`** pass with the new file in place.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-20`.
- **Fence:**
  - `fab/sentinel-01-bench/BENCH.md`, `ORDER.md`, and a new `BENCH-EXT-BMP581.md` if you choose one;
  - `examples/sentinel-01-ext-bmp581.json`;
  - `docs/reviews/2026-11-19-sentinel-01-as-ordered.md`;
  - FEEDBACK.
  - **Not:** `fab/sentinel-01-bench/package/**`, `examples/sentinel-01.json`, `boards/**`, any crate, model, golden, test, workflow, `ci/` or `container/`.
- **Running beside you:**
  - `4400g` (#238: specification 17; it re-derives `package/MANIFEST.json` and `FAB-NOTES.md`). Never touch `package/`.
  - `1911` (CI routing). A docs-and-fab PR may route differently after it lands. Say what CI ran.
  - `1315`, `4404` and `4600`. You share no file with them.
- One container of yours at a time, under `capped-run`, `--mem 16G` for `verify`, at `nice -n 10`. No access to t9o777 or the bench hardware.
- One PR, left unmerged. Write `~/.cache/aet-3403/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A pass line that a real fault slips through** (the review's F3–F5 pattern), on every row, not only the ones it named.
- **A prediction written from a run that already shows the answer.**
- **A cable table that a reader could wire backwards.**
- **The bench specification passing for a reason that does not hold on the ordered copper.**

## 7. Flag-and-stop conditions

- **A review finding you judge wrong at source:** stop with the evidence. The architect rules.
- **B7 cannot be made to fail only for its named reason:** say so, make it a measurement, and carry on.
- **The bench specification needs an engine change** (for example, the solver refuses EXT pinned to PB10/PB11): stop and say which.
