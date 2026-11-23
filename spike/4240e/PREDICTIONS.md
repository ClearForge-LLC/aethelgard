# AET-WO-4240e §1.1 — predictions, written before the measurement

Written 2026-11-23 ~17:20Z at `465bd860`, before `measure.py` ran. Each is checked in `RESULTS.md`.

Ground read so far (not yet measured): the generator's `select_code` table, `Plan::selects_kernels`, the firmware's call order in `board/firmware.rs` (`main_rs`), RM0433 Rev 9 §8.5.10 pp. 370–372 (held text), and Tviburi's Renode-recorded order table for 14 boards (`TVI-WO-1300.rm0433.md` §5).

## P1. Which selectors the generated firmware writes

- **P1.1** Only `RCC_D2CCIP1R` (`SPI123SEL`, `SPI45SEL`) and `RCC_D3CCIPR` (`SPI6SEL`), and only on a board with an SPI controller and no HSE. `D1CCIPR`, `D2CCIP2R` (USART, I2C123, RNG, …) and every other `D3CCIPR` field (`ADCSEL`, `I2C4SEL`, `LPUART1SEL`) are read and never written.
- **P1.2** No committed specification declares a peripheral whose kernel the generator could select with RNG, FDCAN, LPUART, LPTIM, SAI, SDMMC, QSPI or USB: the generator enables none of them.

## P2. Order

- **P2.1** HSI boards with SPI: `ResetClock` read → `select_spi_kernels` (one whole-word write per register) → GPIO enables → peripheral enables (`SPIxEN` among them) → … → SPI `init`/`SPE`. No PLL is ever turned on.
- **P2.2** HSE boards: `ResetClock` read → `Hse::start` (`HSEON`, `PLL1ON` with `DIVQ1EN` set, `SW` = PLL1) → enables → starts. No select is written; `SPI123SEL` stays at `pll1_q_ck`.

## P3. The p371 outcome per peripheral

- **P3.1** Every HSI board with SPI1, SPI2 or SPI3: the switch `pll1_q_ck` → `per_ck` does not take place (PLL1 off); the SPI starts with **no kernel clock**. Expected count: the 9 boards Tviburi names, plus every committed specification Tviburi did not boot that has SPI1–3 on HSI.
- **P3.2** SPI4/5 (`SPI45SEL` `rcc_pclk2` → `hsi_ker_ck`) and SPI6 (`SPI6SEL` `rcc_pclk4` → `hsi_ker_ck`): both inputs run, the switch takes place, clocked.
- **P3.3** HSE boards' SPI1–3: `pll1_q_ck` runs from `Hse::start` before `SPIxEN`: clocked.
- **P3.4** I2C1–3 (`I2C123SEL` `rcc_pclk1`), I2C4 (`I2C4SEL` `rcc_pclk4`), USART1/6 (`rcc_pclk2`), USART2/3, UART4/5/7/8 (`rcc_pclk1`): never switched, clocked on every board.
- **P3.5** ADC1/ADC3: `ADCSEL` stays at `pll2_p_ck`, which never runs; **but** the generator writes `ADCx_CCR.CKMODE` = `11`, which takes `adc_sclk` and not `adc_ker_ck_input` (RM0433 Rev 9 §25.4.3, p. 920). So the ADC has a clock, and Tviburi's refusal of `d1-drv8876` at `ADC3EN` is the guard not reading `CKMODE`. To be stated as a finding, not acted on (Tviburi's fence).

## P4. Bytes each option would move

- **P4.1 (a)** (start PLL1 from HSI for the switch, then stop it): the generated board crate and firmware of every HSI-with-SPI board, so `boards/sentinel-01/**`, `boards/sentinel-01-bench/**`, the svd-hal `ui` goldens that carry an HSI SPI board (`motors-pwm`, `motors-pwm-open`, `serial-four` if it has SPI), the bench ELF and its two hash lines (`MANIFEST.json`, `BENCH.md` L5). **No solved-topology byte, no `kicad/` byte, no Gerber.**
- **P4.2 (b)** (PLL1 from HSI, left running, `SPI123SEL` left at `pll1_q_ck`): everything (a) moves, **plus** every committed solved topology of an HSI SPI board (`kernel` `PerCk` → `Pll1Q`; `kernel_hz`/`mbr` only if Q ≠ 64 MHz), and the `SolvedClock` gains a PLL1 description on HSI boards. Still no `kicad/` byte or Gerber.
- **P4.3** Neither option moves a pin: the solver's pin assignment does not read the kernel clock.

## P5. The stop conditions

- **P5.1** No held page contradicts p371–372; p413's footnote (“both the previous and the new clock sources are present during the switching”) agrees.
- **P5.2** No bench record shows a switch away from a dead input working: `3302` B8 shows `SPI123SEL` = `pll1_q_ck` and PLL1 off, no switch attempted.
