# AET-WO-4400c2 — The twin's platform carries the DMA request lines a firmware's drivers use

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400c2-twin-dma`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400c` builder, reused), with a targeted adversarial pass.
**Phase:** P4, after the `4400c` spike (PR #202, ruling comment 6944720041, D2). Written 2026-11-17. **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **`4400c`'s stall 1, class (p), measured.** `GenericSpiFlash` serves JEDEC `EF 40 17`, but unmodified ArduPilot reads `0x00F0BB03`.
  - The twin's emitted `spi1` and `spi2` declare no `DMARecieve -> dmamux1@…` line. Renode 1.17.0's own `spi4` does (`stm32h7.repl` L197).
  - ArduPilot receives SPI1 on DMA2 stream 1, DMAMUX request 37 (`stm32_dmamux.h` L72).
  - With the overlay `spike/4400c/overlay-spi-dma.repl` (`spi1 … @37`, `spi2 … @39`), run `r3` prints `SPI Flash 0x00EF4017 found` and the logger comes up.
- **The same gap garbles the IMU's SPI2 reads** on any board whose `IMU` line exists (`4400d`).
- **The generated Rust firmware uses no DMA** (`svd-hal` README; `AET-WO-4300a`), so a request line it never asserts cannot change its verdict.

> **What this is:** the emitted platform wires each peripheral's DMA request lines into Renode's DMAMUX wherever Renode 1.17.0's own model has the line, so a firmware that uses DMA (ArduPilot) reaches its devices.
>
> It is NOT:
> - a change to any verdict of the generated firmware (no DMA there);
> - the foreign-firmware judge (a later order);
> - a model of our own.

## 1. Build

1. **Measure first, then STOP.** For each peripheral the twin emits (SPI, I2C, USART/UART, the timers), say:
   - whether Renode 1.17.0's model has DMA request connections, read at source in the pinned image;
   - which DMAMUX request number RM0433 gives it (cite the table);
   - whether ArduPilot uses DMA on it for Sentinel-02's generated board (read in `hwdef.h`'s DMA map and in ArduPilot's source at the pinned tag).

   Run `4400c`'s `r3` again with candidate lines, all emitted rather than overlaid, to measure which are needed. **STOP** with:
   - the table;
   - the lines you propose to emit;
   - every committed platform golden and `.repl` whose bytes would move (`boards/*/…repl`, the twin's tests).
2. **After the ruling,** `aethelgard-twin` emits the ruled lines from ST's facts and the solved topology alone (`RUL-16`). Each request number comes from a cited table, never typed in by hand without its source (`INV-06`). Every moved golden is listed in FEEDBACK.

## 2. Acceptance

- `4400c`'s `r3` on the emitted platform, with **no overlay**, prints `SPI Flash 0x00EF4017 found`. Record it as measured, with the spike's driver unchanged.
- Every generated-firmware verdict is unchanged: the reference, the bench board and every scenario. Bytes move only in the platform files, and only as listed.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`, `INV-07`; `RUL-05`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-twin` (`platform.rs`);
  - its tests and goldens;
  - `boards/*/*.repl`, where they move;
  - FEEDBACK.
- **Running beside you:**
  - `1313` (eda placement, specification 15);
  - `1604` (CLI);
  - `3402` (ir, svd-hal, eda's FAB-NOTES, Sentinel-01's package);
  - `2302b` (evolve, chaos).

  `3402` re-blesses `boards/sentinel-01-bench`. If your `.repl` change moves it, tell `3402` in its inbox. Whoever merges second regenerates.
- One PR, left unmerged. Write `~/.cache/aet-4400c2/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A request line wired to the wrong DMAMUX number:** a read that succeeds by accident, for example on SPI2 instead of SPI1.
- **A generated-firmware verdict that moves.**
- **A request number with no citation.**

## 7. Flag-and-stop conditions

- **§1.1 is the stop.**
- **A line needs a Renode model change** that cannot be made in the platform description. Stop and say what.
