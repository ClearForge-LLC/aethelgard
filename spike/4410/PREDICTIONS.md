# AET-WO-4410 — predictions, written before any run (§1.1)

Written 2026-11-23 on `main` `465bd860`, before any `solve`, `generate hwdef` or `check pinout` ran on this branch.

**Grounds:** code read on `465bd860`: `hwdef::generate` (`hwdef/src/lib.rs:409-605`), `pincheck::judge` (`pincheck/src/lib.rs:377-579`), `cli/src/pinout.rs::check`, `SolvedSense` (`ir/src/lib.rs:772-776`), the MCU row's `ardupilot` block (`ir/data/power-parts.json:1366-1400`), `svd-hal/src/board/firmware.rs:183-200`, and each committed specification's `mcu`, `clock` and `power.sense`, read by a one-line script (no engine run).

**Not blind:** P1 is a count (`universe.sh`, copied from `4300j`), and P2's two sense boards are read from the files.

## The universe

- **P1.** The committed current specifications number 35 (`universe.sh`).
- **P2.** Two declare a `power.sense`: `sentinel-03` and `sentinel-02-power` (`VBAT_SENSE` each). No other does.

## `generate hwdef` on each solved specification (today's `main`)

- **P3.** `solve` refuses `invalid-zero-domain`, `solve-infeasible-shared`, `solve-pigeonhole-spi` and `d2-drv8833` (each by design), so no hwdef is reached for them.
- **P4.** `generate hwdef` refuses, by name:
  - `sentinel-03` and `sentinel-02-power`: `SenseNotEmitted` (`VBAT_SENSE`);
  - `hse-crystal-16`: `OscillatorNotCited` (16000000, cited 0);
  - `hse-external-8` and `sentinel-02-bench-hse8`: `OscillatorNotCited` (8000000, cited 0);
  - `d1-drv8876`: `GpioNotEmitted` (a pin device).
- **P5.** Every other board either generates or is refused for a serial reason (`NoSerialPort` or `NoConsole`) where it declares no console. I do not forecast which, board by board.
- **P6.** With `SenseNotEmitted` bypassed in a scratch build, Sentinel-03 **generates**: `PinNotEmitted` walks `assignments` and `gpio` only, and the sense pin is in neither. Its file then says `HAL_USE_ADC FALSE` beside a solved sense pin, which is the defect (1) removes.

## The ArduPilot texts (to be read at `fb773567`)

- **P7.** The SWD lines are `PA13 JTMS-SWDIO SWD` and `PA14 JTCK-SWCLK SWD`, written by most H7 boards at the tag; PA13 with `PULLUP` and PA14 with `PULLDOWN` on some.
- **P8.** The battery line is `<pin> BATT_VOLTAGE_SENS ADC1 SCALE(1)`, with `HAL_BATT_VOLT_PIN` the pin's ADC channel number and `HAL_BATT_VOLT_SCALE` the divider ratio `(top + bottom) / bottom` as a decimal literal; `HAL_BATT_MONITOR_DEFAULT 4` (analog voltage only).
- **P9.** `stm32h7_mcuconf.h` has a `STM32_HSECLK == 16000000U` branch whose PLL1 runs at the same VCO as the HSI branch, so SPI1–3 on PLL1_Q stay at 80 MHz; SPI4/5 on their own source may differ. Low confidence.
- **P10 (Q3).** No ArduPilot or ChibiOS code at the tag writes `SYSCFG->PMCR` with a value from `hwdef.h`, so `check pinout` has no witness for PC2SO/PC3SO. Low confidence: if so, (7) is a §7 stop.

## Which committed bytes a build would move

- **P11 · versions:** solved topology 18 → 19 (the divider in `SolvedSense`); power-parts 10 → 11 (the MCU row's per-oscillator kernel clocks). Specification (29) and `parts.json` do not move.
- **P12 · topologies:** every committed `SolvedTopology` moves by `schema_version`: the five `hwdef/tests/fixtures/*.topology.json`, `pincheck/tests/common/topology.json` (and, by `4300j` R9, it moves further, regenerated), and the two `svd-hal/tests/ui/*/topology.json`. A sense topology gains the divider; none of these eight holds a sense.
- **P13 · hwdef goldens:** the five `hwdef/tests/fixtures/*.hwdef.dat` move by the two SWD lines (and the width of the source column if a new line is the longest); none by (1), since none has a sense; the 2311 F1 sentence moves in those that emit a `DEFAULT_SERIALn_PROTOCOL`.
- **P14 · `fab/`:** only `sentinel-01-bench/package/MANIFEST.json` moves: `topology_sha256`, and each scenario's `sha256` if a verdict embeds the topology's hash. No Gerber, drill, BOM, CPL, `FAB-NOTES.md` or ELF byte moves: the ELF's provenance names the specification's hash (`svd-hal/src/board/firmware.rs:191`), the specification does not move, and `SOLVED_SCHEMA_VERSION` appears in svd-hal only under `#[cfg(test)]` (`board.rs:8593`, after `:8572`).
- **P15 · pins:** no pin of any board moves. Nothing here touches `solve`.
