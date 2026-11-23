# AET-WO-4240e §1.1 — the measurement, (a) against (b), and the rule's shape

Measured 2026-11-23 at `e67db6cd` (main `465bd860` plus the predictions), on Furnace. Harness: `measure.py`, output `out/measure.json`. Predictions: `PREDICTIONS.md`, committed first.

## 0. The answer

- **26 of the 32 committed specifications that generate start SPI1, SPI2 or SPI3 with no kernel clock.** That is every HSI board with an SPI controller on `SPI123SEL`. Each writes `SPI123SEL` = `100` (`per_ck`) while the mux's input `pll1_q_ck` has never run, so the switch does not take place (RM0433 Rev 9 §8.5.10, pp. 371–372).
- **Nothing else is unclocked.**
  - The 3 HSE boards' SPI1/SPI2 run on `pll1_q_ck` from `Hse::start`, with `DIVQ1EN` set (`PLLCFGR` `0x00030002`/`0x00030004`).
  - `SPI45SEL` (`rcc_pclk2` → `hsi_ker_ck`) and `SPI6SEL` (`rcc_pclk4` → `hsi_ker_ck`) switch between two running inputs.
  - Every I2C and USART selector is never written, and stays on its running reset input.
- **`SPI123SEL` is the only selector the generator moves away from a dead input.** No generated firmware enables RNG, FDCAN, LPUART, LPTIM, SAI, SDMMC, QSPI or USB.
- **The ADC differs from a ground of the WO (D3).** `d1-drv8876`'s ADC3 runs on `adc_sclk`: the bring-up's first write is `ADC3_CCR` = `0x00030000` (`CKMODE` `11`), and with `CKMODE` ≠ `00` the ADC does not take `adc_ker_ck_input` (§25.4.3, p. 920). `ADCSEL` stays at `pll2_p_ck`, which never runs, and nothing uses it. Tviburi's guard refuses this board at `ADC3EN` because it does not read `CKMODE`. No bench has run the ADC.
- **No §7 stop fires** (§5):
  - no held page contradicts p371;
  - no bench record shows a switch away from a dead input working: no generated `per_ck` firmware has run SPI1 on silicon;
  - neither option can move a pin or copper byte: `aethelgard-eda` reads no clock or kernel fact.
- **Lean: (a).** It moves no rate, no `MBR` and no `BENCH.md` prediction. (b) falsifies `BENCH.md` L196, which the fence forbids. (b)'s rule is simpler, though: it has no switch to order. That trade is D1.

## 1. Method

- **Which specifications.** Every git-tracked `.json` with a top-level `schema_version` (198 files). `aethelgard validate` accepts 32 of them, all at specification 29, and `aethelgard generate` writes each one's board crate and firmware.
- **The 166 skips, none a live board:**
  - 159 are not specifications at the current schema: scenario overlays (`scenarios/`, `spike/`, `docs/…repro`), solved topologies at solved schema 18 (`hwdef`, `pincheck`, `svd-hal ui` fixtures), the IR's data tables, and frozen old-schema CLI fixtures (versions 1, 2 and 6).
  - 7 are refused by design: three `Infeasible` solve fixtures, `d2-drv8833` (`PartLimitUndeclared`), `invalid-zero-domain` (`ZeroNominalVoltage`), and two `spike/` drafts (`SchemaViolation`).
- **What was read.** The emitted crate only, never the generator's source:
  - `board/firmware/src/main.rs` for the static call order: `reset_clock`, `hse.start`, `select_spi_kernels`, each `rcc.enable_*`, and each start (`init`, `configure`, `bring_up`, which precede the first `SPE`/`PE`/`UE`/`ADEN`);
  - `board/src/board.rs` for the words: the select writes, `PLLCFGR`, `ADCx_CCR`, and every read, write and modify of a CCIPR register.
- **The mux model.** It is simulated along that order by the p371 rule. A select write moves the input only when the current input and the new one both run; otherwise the input is "pending" and no clock reaches the peripheral.
  - The sources that run from reset: `HSION` 1 (p. 383), `per_ck` = `hsi_ker_ck` at `CKPERSEL` 00 (p. 412), and the bus clocks.
  - `Hse::start` adds `hse_ck` and the PLL1 outputs whose `DIVxEN` is set.
  - The generator never clears `HSION`: its `RCC_CR` words are `HSION` plus the bits set so far.
- **Every enable classified.** No `rcc.enable_*` falls outside the selector table, except GPIO ports and timers, which take no kernel mux.
- **No RCC write after bring-up.** The firmware never writes the clock tree after bring-up, so the clock at a peripheral's start is its clock for the run.
- **Cross-check.** Tviburi's 14 Renode-recorded boots (`TVI-WO-1300.rm0433.md` §5, at `75da8053`) agree with this static order on every board and peripheral. The one difference is the ADC row above.

## 2. The selectors, by page (held RM0433 Rev 9, §8.7.19–§8.7.22)

| Peripheral (generated) | Selector | Reset input (p.) | Generated write | Input at start, HSI board | Input at start, HSE board |
|---|---|---|---|---|---|
| SPI1–3 | `D2CCIP1R.SPI123SEL[14:12]` | `pll1_q_ck` (414) | `100` `per_ck`, HSI boards only | **pending `pll1_q_ck`→`per_ck`: none** | `pll1_q_ck`, running |
| SPI4/5 | `D2CCIP1R.SPI45SEL[18:16]` | APB (`rcc_pclk2`) (414) | `011` `hsi_ker_ck`, HSI only | `hsi_ker_ck` (switched) | APB |
| SPI6 | `D3CCIPR.SPI6SEL[30:28]` | `rcc_pclk4` (418) | `011` `hsi_ker_ck`, HSI only | `hsi_ker_ck` (switched) | `rcc_pclk4` |
| I2C1–3 | `D2CCIP2R.I2C123SEL[13:12]` | `rcc_pclk1` (416) | never | `rcc_pclk1` | `rcc_pclk1` |
| I2C4 | `D3CCIPR.I2C4SEL[9:8]` | `rcc_pclk4` (420) | never | `rcc_pclk4` | `rcc_pclk4` |
| USART1/6 | `D2CCIP2R.USART16SEL[5:3]` | `rcc_pclk2` (417) | never | `rcc_pclk2` | `rcc_pclk2` |
| USART2/3, UART4/5/7/8 | `D2CCIP2R.USART234578SEL[2:0]` | `rcc_pclk1` (417) | never | `rcc_pclk1` | `rcc_pclk1` |
| ADC1/2, ADC3 | `D3CCIPR.ADCSEL[17:16]` | `pll2_p_ck` (419), never running | never; `CCR.CKMODE` `11` | `adc_sclk` (p. 920), the mux unused | as HSI |
| `per_ck` | `D1CCIPR.CKPERSEL[29:28]` | `hsi_ker_ck` (412) | never | `hsi_ker_ck` | `hsi_ker_ck` |

Not generated, for the rule's table: FDCAN (`FDCANSEL` reset `hse_ck`, p. 413), RNG (`RNGSEL` reset `hsi48_ck`, printed "hsi8_ck", p. 417), SDMMC/SAI/SPDIF (`pll1_q_ck`), USB (disabled), CEC (`lse_ck`). Each of these resets onto a source that is off at reset.

**CCIPR accesses in the emitted board crate (shown, not inferred):**
- 23 boards: `D1CCIPR` read, `D2CCIP1R` read + write, `D2CCIP2R` read;
- 3 boards: the same plus a `D3CCIPR` read (I2C4);
- 5 boards: reads only (the HSE boards and the boards without SPI);
- 1 board (`pair-standin`, SPI6): `D3CCIPR` read + write.
- **No `D2CCIP2R` or `D1CCIPR` write anywhere.**

## 3. Per specification

| Specification | Clock | Selector writes (at main.rs line) | Kernel-clocked peripherals: clock at start | Unclocked | Tviburi (Renode-recorded) |
|---|---|---|---|---|---|
| `crates/aethelgard-cli/tests/fixtures/connectors.json` | HSI | D2CCIP1R=0x00004000 (171) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1, spi2 | — |
| `crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json` | HSI | D2CCIP1R=0x00004000 (174) | i2c1 yes; i2c2 yes; i2c3 yes; i2c4 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json` | HSI | D2CCIP1R=0x00034000 (178) | i2c1 yes; i2c2 yes; i2c3 yes; i2c4 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi3 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi4 yes | spi1, spi2, spi3 | — |
| `crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json` | HSI | none  | usart1 yes | 0 | agrees |
| `crates/aethelgard-cli/tests/fixtures/four-layer-power.json` | HSI | D2CCIP1R=0x00004000 (170) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/four-layer.json` | HSI | D2CCIP1R=0x00004000 (170) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/four-port.json` | HSI | D2CCIP1R=0x00004000 (181) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); uart4 yes; usart1 yes; usart2 yes; usart3 yes | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json` | HSE | none  | i2c1 yes; spi1 yes; spi2 yes; usart2 yes; usart3 yes | 0 | agrees |
| `crates/aethelgard-cli/tests/fixtures/hse-external-8.json` | HSE | none  | i2c1 yes; spi1 yes; spi2 yes; usart2 yes; usart3 yes | 0 | agrees |
| `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json` | HSI | D2CCIP1R=0x00004000 (186) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/logger-4600.json` | HSI | D2CCIP1R=0x00004000 (194) | i2c1 yes; i2c2 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/motors-dshot600.json` | HSI | D2CCIP1R=0x00004000 (171) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | agrees: unclocked |
| `crates/aethelgard-cli/tests/fixtures/motors-pwm.json` | HSI | D2CCIP1R=0x00004000 (171) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | agrees: unclocked |
| `crates/aethelgard-cli/tests/fixtures/pair-standin.json` | HSI | D3CCIPR=0x30000000 (170) | i2c1 yes; spi6 yes | 0 | — |
| `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json` | HSI | D2CCIP1R=0x00004000 (178) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` | HSI | D2CCIP1R=0x00004000 (194) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1, spi2 | — |
| `crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json` | HSI | D2CCIP1R=0x00004000 (196) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart2 yes; usart3 yes | spi1, spi2 | agrees: unclocked |
| `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` | HSI | D2CCIP1R=0x00004000 (200) | i2c1 yes; i2c2 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes; usart2 yes; usart3 yes | spi1, spi2 | agrees: unclocked |
| `crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json` | HSI | D2CCIP1R=0x00004000 (179) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | agrees: unclocked |
| `crates/aethelgard-cli/tests/fixtures/serial-four.json` | HSI | D2CCIP1R=0x00004000 (181) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); uart4 yes; usart1 yes; usart2 yes; usart3 yes | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/solve/stress.json` | HSI | D2CCIP1R=0x00034000 (178) | i2c1 yes; i2c2 yes; i2c3 yes; i2c4 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi3 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi4 yes | spi1, spi2, spi3 | — |
| `crates/aethelgard-cli/tests/fixtures/spi-only.json` | HSI | D2CCIP1R=0x00004000 (170) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-cli/tests/fixtures/two-bme.json` | HSI | D2CCIP1R=0x00004000 (170) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` | HSI | none  | adc3 yes; usart1 yes | 0 | differs: its guard refuses ADC3EN (ADCSEL = pll2_p_ck); CKMODE 11 is not read |
| `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` | HSI | D2CCIP1R=0x00004000 (187) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); spi2 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1, spi2 | — |
| `examples/sentinel-01-ext-bmp581.json` | HSI | D2CCIP1R=0x00004000 (194) | i2c1 yes; i2c2 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | agrees: unclocked |
| `examples/sentinel-01-minimal.json` | HSI | D2CCIP1R=0x00004000 (178) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | agrees: unclocked |
| `examples/sentinel-01.json` | HSI | D2CCIP1R=0x00004000 (179) | i2c1 yes; i2c2 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | agrees: unclocked |
| `examples/sentinel-02-bench-hse8.json` | HSE | none  | i2c1 yes; spi1 yes | 0 | agrees |
| `examples/sentinel-02-bench.json` | HSI | D2CCIP1R=0x00004000 (185) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | agrees: unclocked |
| `spike/4300j/open/motors-pwm-open.json` | HSI | D2CCIP1R=0x00004000 (171) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck) | spi1 | — |
| `spike/4300j/open/serial-dshot-9600-open.json` | HSI | D2CCIP1R=0x00004000 (179) | i2c1 yes; spi1 **no** (SPI123SEL pending:pll1_q_ck->per_ck); usart1 yes | spi1 | — |

The line in parentheses is `select_spi_kernels`'s line in that board's `main.rs`. Each SPI enable and start follows it, and no PLL is started on an HSI board.

## 4. The predictions, checked

| # | Result |
|---|---|
| P1.1 | **Held.** Writes only to `D2CCIP1R` and `D3CCIPR` (SPI6), only on HSI boards with SPI. |
| P1.2 | **Held.** No generated firmware enables RNG, FDCAN, LPUART, LPTIM, SAI, SDMMC, QSPI or USB. |
| P2.1, P2.2 | **Held** (static order; Tviburi's recorded order agrees). |
| P3.1 | **Held, larger:** 26 boards, the 9 Tviburi booted plus 17 committed specifications it did not (14 CLI fixtures, `ir`'s `sentinel-02-power`, and `spike/4300j/open/*`). |
| P3.2–P3.4 | **Held.** |
| P3.5 | **Held on the page and the bytes** (`CCR` = `0x00030000` written first in `bring_up`); unconfirmed on silicon. It contradicts the WO's ground from Tviburi, so it is D3, not a finding to file. |
| P4.1 | **Partly wrong.** (a) also moves solved topologies (D7) and, through WO (3), seven specifications' acknowledgements (D2). §6 has the measured list. |
| P4.2 | **Held, and more:** (b) falsifies `BENCH.md` L196. |
| P4.3 | **Held:** `aethelgard-eda` reads no `SolvedClock`, kernel or `spi_controllers` fact (grep over `crates/aethelgard-eda/src`), so no `kicad/` byte, Gerber or drill can move. |
| P5.1, P5.2 | **Held** (§5). |

## 5. The stop conditions (§7)

- **A held page contradicting p371:**
  - p371–372 is quoted in full in `TVI-WO-1300.rm0433.md` §2.3 and re-read here.
  - All four CCIPR descriptions carry the same footnote: "both the previous and the new clock sources are present during the switching, and for the whole transition time" (pp. 412, 413, 416, 418). `SPI123SEL`'s caution (p. 414) points back to the same section.
  - ES0392 Rev 16's only clock-switching erratum is 2.2.11 (LSE CSS), which does not apply.
  - **None contradicts.**
- **A bench record of a switch away from a dead input working:**
  - `3302` B8 (`bench-3302-results.md`): `SPI123SEL` = `000` with PLL1 off. No switch was attempted, SPI1 had no kernel clock, and `MODF` was set.
  - `3304` B7, `4240d` B8 and `fab/sentinel-01-bench/BENCH.md` B4 are prepared and never filled ("Filled at the bench").
  - The architect's logs record no later SPI1 run. **None.**
- **A pin or copper byte:** none under either option (P4.3).

## 6. (a) against (b)

Both options must start PLL1 from HSI, and neither can use PLL1's reset words.
- At reset, `DIVM1` = 32 gives `ref1_ck` = 2.00 MHz, the wide VCO's floor (`fPLL_IN` 2–16 MHz, DS12117 Rev 10 Table 39, p. 129). HSI's slow edge falls below it, and `PLL1RGE` resets to `00` (1–2 MHz, p. 401), which straddles the same edge. HSI's band is −25,894/+14,984 ppm (`Budget::HSI`), 62.34–64.96 MHz.
- So the bring-up writes `PLLCKSELR`, `PLLCFGR` and `PLL1DIVR` before `PLL1ON`. For example `DIVM1` 5 gives 12.47–12.99 MHz in `RGE` `11`, `DIVN1` 20 gives a 249–260 MHz VCO (192–836), and `DIVQ1` 4 gives `pll1_q_ck` ≈ HSI, under VOS2's 300 MHz (p. 130).
- The lock is bounded by `tLOCK` ≤ 150 µs (p. 130), the bound `Hse::start` already uses.

| | (a) PLL1 for the switch only | (b) PLL1 left running, `SPI123SEL` at reset |
|---|---|---|
| SPI1–3 kernel | `per_ck` = `hsi_ker_ck`, 64 MHz, unchanged | `pll1_q_ck` from HSI; 64 MHz only if `DIVQ1` makes it so |
| `SPI45SEL`/`SPI6SEL` | written as today | can stay at reset (APB = HSI): **no select written on any board** |
| What runs after bring-up | as today; PLL1 off | PLL1 for the whole run: its current, and a second boot dependency |
| New firmware steps | PLL words, `PLL1ON`, bounded `PLL1RDY` wait, select write, a transition wait (2 cycles of each clock, p. 371), `PLL1ON` cleared | PLL words, `PLL1ON`, bounded `PLL1RDY` wait, before `SPIxEN` (the HSE boards' order today) |
| **What the IR rule reads for "the old input ran at the switch"** | the solved clock's PLL1-from-HSI record and its window ("from before the selects to after them"). **The order of the writes inside that window exists only in the generator's sequence:** the rule judges the plan, and the emitted order is consistency (the Kani oracle), the twin (stock RCC has no mux model) and the bench row | nothing: there is no switch. The rule judges the steady state from the solved clock, as for the HSE boards |
| Solved topology | new PLL1-from-HSI record (solved schema 18 → 19); `kernel` unchanged | new record (→ 19); `kernel` `PerCk` → `Pll1Q` on 26 boards; `kernel_hz`/`mbr` unchanged at `DIVQ1` = 64 MHz |
| Committed solved topologies moved | the 8 at schema 18 (`hwdef` ×5, `svd-hal ui` ×2, `pincheck` ×1) by the version, if they are regenerated | the same 8, and their `kernel` |
| Generated crates moved | `boards/sentinel-01/**`, `boards/sentinel-01-bench/**`; `svd-hal ui` `motors-pwm`, `motors-pwm-open`, `serial-four` (`board.rs`, `proofs.rs`); `ui/fail/spi_kernels_without_reset_clock.stderr` if the API changes | the same, and `select_spi_kernels` goes away |
| Bench package | ELF; `MANIFEST.json` ELF, topology and verdict hashes; `BENCH.md` L5's ELF hash | the same **and `BENCH.md` L196 (`D2CCIP1R` `0x00004000` → `0x00000000`)**. The fence allows new rows only |
| `BENCH.md` `L…` citations into `main.rs` (L176–L252) | shift if `main.rs` gains a fault arm (D6). L5 already scopes them to the tree `3308` merged | shift the same way |
| Gerbers, drill, `kicad/` | none | none |

## 7. The rule's shape (`RUL-20` (1), `INV-07`)

- **Where.** A new module in `aethelgard-ir` (`kernel_clock`). It shares no code with `aethelgard-svd-hal`, nor with the solver's `solve::spi_kernel`. It has its own table, §2's rows, each cited by page.
- **Inputs:** the `SolvedTopology` and the held table. Never the generator, the generated crate or the firmware.
  - **Enabled kernel-clocked peripherals:** `spi_controllers`, the I2C controllers, `serial_ports` and the ADCs.
  - **A peripheral with no table row is refused by name** (`INV-06`), not passed.
  - **Each peripheral's selected input:** SPI from `spi_controllers[].kernel`; the ADC from a new solved fact (D3); I2C and USART at their reset input, since the topology selects none.
  - **The sources that run:**
    - from reset: `hsi_ker_ck`, `per_ck` at `CKPERSEL`'s reset, the bus clocks and `adc_sclk`;
    - with the HSE tree (as `silicon::spi_kernel_on` decodes it today): `hse_ck` and PLL1's outputs by `DIVxEN` and `PLLSRC`;
    - with the new PLL1-from-HSI record: PLL1's outputs, for its window (a) or for the whole run (b).
  - **PLL words** are judged against the manual: `RGE` against the reference across HSI's band, the VCO's range, `Q` ≤ the VOS maximum.
- **The p371 rule:**
  - A mux's input at entry is its reset input, after every system reset (p. 371: "After system reset, PLLs, HSE, CSI and HSI48 are switched OFF", with p. 372's USART1 example).
  - A selected input that is not the reset input requires both inputs running across the switch, for its whole transition (pp. 371, 413).
  - The input at the peripheral's start must run.
  - A PLL whose output an enabled peripheral's input still is must not be stopped.
- **Refusals by name:**
  - `KernelClockDeadSwitch { peripheral, selector, from, to, dead }`;
  - `KernelClockNotRunning { peripheral, selector, input }`;
  - `KernelClockUnknown { peripheral }`.
  Each carries its pages.
- **Where it runs:** where `check_spi` runs today. That is `svd-hal`'s `check_silicon` before generation (`SiliconRuleFails`) and the verdict's silicon findings. The ledger's selector rows cite it.
- **What it replaces:** `silicon::kernel_at_reset` gives `per_ck`, 64 MHz, for `SPI123SEL` = `100` "with every other RCC register at its reset value". That is the defect in rule form, and it is why `check_spi` passes all 26 boards today. The new rule makes `check_spi`'s kernel conditional on the switch, and `check_spi` keeps judging `MBR`.

## 8. Decisions for the architect

- **D1 — (a) or (b).** Lean **(a)**: no rate, `MBR` or `BENCH.md` prediction moves, and PLL1 is off once bring-up ends. Against it, the switch's order is judged by the plan, not by the rule reading the emitted order. (b) needs a ruling on `BENCH.md` L196, keeps PLL1 running, and moves every HSI SPI topology's `kernel`; for that, its rule has no order to trust.
- **D2 — the acknowledgement cascade of WO (3).**
  - Seven committed specifications acknowledge the reset selector rows `ignored-by-model`: `RCC.D1CCIPR` `CKPERSEL`, `RCC.D2CCIP1R` `SPI123SEL`/`SPI45SEL`, and `RCC.D2CCIP2R` `USART16SEL`/`USART28SEL`/`I2C123SEL`. They are `examples/sentinel-01.json`, `examples/sentinel-01-minimal.json`, and the CLI fixtures `driver-judges-nothing`, `four-layer`, `logger-4600`, `sentinel-01-bench` and `sentinel-03`.
  - Moving those rows to `independent-rule` makes each acknowledgement stale, and `ledger::stale` refuses it (`verify.rs` L1712, `main.rs` L2910). So the specifications must drop them.
  - `examples/` and the CLI fixtures are outside the fence. `sentinel-01`'s specification hash then moves `MANIFEST.json`, `FAB-NOTES.md` L3–4 and the generated headers, all hash lines.
  - Lean: **grant the seven acknowledgement deletions**, and nothing else in those files.
  - The alternative: move only the written rows and the HSE boards' reset rows, and leave the HSI reset rows `ignored-by-model`.
- **D3 — the ADC.** `aethelgard-ir` holds no `CKMODE` fact, so the rule cannot apply p920 without reading the generator. Three shapes:
  - (i) a solved ADC clock fact (`adc_sclk`/4) that the rule judges as running. `d1-drv8876` then passes, against Tviburi's refusal.
  - (ii) the rule requires `adc_ker_ck_input` to run. Then the generator must start PLL2 or PLL3, since `ADCSEL` cannot leave `pll2_p_ck` without it (p371 again).
  - (iii) bench only.
  - Lean **(i)**, a Nucleo ADC row with PLL2 off as its bench check, and a note to Tviburi that its guard does not read `CKMODE` (its fence, not mine).
- **D4 — the "planted specification" of §2.** No specification field picks a kernel or a selector. After the fix the solver never emits a dead switch, so no planted specification can reach the refusal.
  - Lean: **planted solved topologies** (the PLL1 record removed, its window closed early, `Pll1Q` with no PLL, a peripheral with no table row) through the rule and through `verify`'s silicon findings.
  - Also the pre-fix topologies of the 26 boards, regenerated at `465bd860`, as the scar's fixtures.
- **D5 — the refusal kinds and where they surface.** Lean: the three kinds in §7, in `generate` (as `SiliconRuleFails` does) and in the verdict.
- **D6 — (a)'s lock fault.**
  - Lean: `select_spi_kernels` returns a `Result`, and `main.rs` parks with a named boot word on `PLL1RDY`'s timeout (reusing `BOOT_PLL1_NOT_LOCKED`).
  - That adds lines before the `main.rs` lines `BENCH.md` cites. Lean: leave those citations as they are, since L5 scopes them to `3308`'s tree.
  - The alternative is a select that parks inside the board crate, as `reset_clock`'s failure does today with no boot word.
- **D7 — the solved schema.** Both options add a PLL1-from-HSI record to `SolvedClock`, so solved schema 18 → 19. The 8 committed topologies move by the version.

## 9. Not measured

- Any silicon. The Nucleo row is WO (4), Scotty's.
- Recorded mode of Tviburi's guard against regenerated firmware (§2's acceptance). That waits for the build and for `1300a`; I will ask on the bus when I reach it.
- The exact PLL words. §6's are one feasible set; the solver chooses them in the build.
