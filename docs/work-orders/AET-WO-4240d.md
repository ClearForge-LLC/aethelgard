# AET-WO-4240d — The bring-up in bypass on the Nucleo: firmware, predictions and a bench sheet for Scotty

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4240d-bypass-bench`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4, before `4500`. Written 2026-11-22. **Cadence:** a build of the firmware and the sheet; the bench itself is Scotty's.

**Grounds** (`4240` D7 (c), ruled on #240; `4240` FEEDBACK §2 and §11; `4240b` merged as #293):
- `4240b` generates H1's bring-up from a declared HSE: bounded `HSERDY`/`PLL1RDY`/`SWS` waits, each with its own halt word, and CSS.
- Both HSE fixtures simulate to `pass`, but no silicon has run it yet.
- `4240` D7 (c) rules a **bench of the bring-up in bypass** on the NUCLEO-H753ZI before `4500`. The ST-LINK's `MCO` drives PH0 as an external 8 MHz clock, `mcu.hse = external`, and the firmware is a variant of `sentinel-02-bench`.
- **It witnesses:** the register sequence, `SWS` = PLL1, a clock out at the predicted frequency, and SPI's SCK at the predicted rate.
- **It does not witness a crystal's ppm,** because the source is the ST-LINK's. A crystal is first witnessed on Sentinel-02 hardware (`4240` D7 (d)).
- **Rows B-0 and B-1 have not been read.** B-0 is the ST-LINK firmware version; B-1 is the `MCO` frequency, either 8.000 MHz (HSI/2) or 8.33 MHz (25 MHz ÷ 3), per `4240` FEEDBACK §2. **B-1 gates the bench.**
- Read first:
  - `docs/work-orders/AET-WO-4240.FEEDBACK.md` §2, which holds B-0, B-1, the PLL words for 8 MHz and the options (a)–(c);
  - `AET-WO-3304.bench.md`, the sheet's model;
  - `AET-WO-4240b.FEEDBACK.md`.

> **What this is:**
> - **A bench specification,** `examples/sentinel-02-bench-hse8.json`:
>   - `examples/sentinel-02-bench.json` with `mcu.hse = {external: {hz 8 000 000, ppm, source, startup_max_us, startup_source}}` and `board.drawn: false`, since the Nucleo is the board (`4240b` D-9);
>   - `ppm` cited to the ST-LINK MCU's own clock source as B-1 will find it. If no held source states it, stop (§7).
> - **Its firmware,** generated and built by `main`'s CLI in the pinned image, twice at two paths with one sha, and its `simulate` verdict.
> - **`AET-WO-4240d.bench.md`,** a sheet in `3304`'s form. Every prediction is written before the bench, each labelled `twin`, `rule`, `generator`, `repeat` or `not predicted`. Never edit a prediction: a disagreement is the result (`INV-07`).
>   1. **B-0 and B-1 first, as the gate.** Their exact commands are in `4240` FEEDBACK §2, restated and re-read at source. **If B-1 is not 8.000 MHz within the analyzer's resolution, the sheet says stop: do not flash.**
>   2. **Flash and boot:** the boot words in order, and which `B0F4`–`B0F8` halt each failure would show.
>   3. **The register read-back after boot,** by `probe-rs read`, each address and expected word cited:
>      - `RCC_CR` (`HSEBYP`, `HSERDY`, `PLL1RDY`);
>      - `RCC_CFGR` `SWS` = PLL1;
>      - `PLLCKSELR` `0x0000_0052`, `PLLCFGR` `0x0003_0002` and `PLL1DIVR` `0x0103_069F` (`4240` §11 D7 (c));
>      - `FLASH_ACR`; the `PWR` VOS2 words.
>   4. **A clock out:** route `MCO1` = `pll1_q` ÷ N to `PA8` by `probe-rs` writes after boot, with N chosen so the FX2's 24 MSa/s counts it. Predict the edge count; say why that N.
>   5. **SPI SCK** on the bench's SPI device at the predicted rate, against `3304`'s measured rate at the HSI clock (`repeat` beside `generator`).
>   6. **Restore:** a reset returns every written register.
>
> It is NOT:
> - a crystal, a solder-bridge or jumper change, or any `5V`;
> - a change to the generator or the twin;
> - a committed bench package (`fab/`): this is a bench record, as `3304` was.

## 1. Build

1. **Measure first:** the specification validates and solves at `main`'s version. Predictions go in the sheet before any run.
2. **Generate, build twice, and simulate** in the pinned image. Record the ELF sha, the board crate's input hash, `simulate`'s image hash and its verdict.
3. **Write the sheet**, every row with its source label. Name what the bench cannot witness: the `HSERDY` timeout path, because no solder change is allowed; CSS, because the clock cannot be stopped without one; and ppm.
4. **Leave the ELF** at `~/.cache/aet-4240d/bench/`, with `SHA256SUMS`, as `3304` did.

## 2. Acceptance

- **The specification validates.** The `tests` that walk `examples/` hold it. `simulate` passes, or its finding is in FEEDBACK.
- **Every register, address and word on the sheet is cited** to RM0433 Rev 9 or DS12117 Rev 10, and re-read at source by an adversarial subagent.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:**
  - `examples/sentinel-02-bench-hse8.json`;
  - `docs/work-orders/AET-WO-4240d.bench.md` and FEEDBACK;
  - `spike/4240d/`;
  - a test row only where a test enumerates `examples/`.
- **Not:**
  - the generator, the IR or the twin;
  - `fab/`;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `1211b` (solved 16, in the package queue);
  - `1211e` (specification 24);
  - `2307`;
  - `1215`;
  - `1319` (eda).
  The specification's version follows `main` at your merge, by the kit. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4240d/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A sheet step that could harm the board,** or one that leaves a register changed after the reset.
- **A prediction labelled `rule` or `twin` that is really the generator's word.**
- **An address or bit position that differs from RM0433 Rev 9.**

## 7. Flag-and-stop conditions

- **No held source states the ST-LINK clock's deviation** for `ppm`. Stop and report what is held. Never invent a figure.
- **The bench needs any solder-bridge, jumper or `5V` change.** Stop.
- **`simulate` refuses or fails the variant.** Stop and report it.
