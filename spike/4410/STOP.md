# AET-WO-4410 — STOP after §1.2

Stopped 2026-11-23 with the measurement (`RESULTS.md`), the line texts, (1)'s field shape and (7)'s answer. Nothing is built: no file under `crates/`, `fab/` or `docs/` has changed. Two §7 conditions fired, and three facts change what §2's acceptance can mean. Each is below with a lean.

## The two §7 stops

### S1 · (7) has no witness that `check pinout` reads — §7 "stop with the source lines"

- One writer of `SYSCFG->PMCR` exists at the tag, in source text (no raw-address store, assembly or Lua writer either; a run-time write through a generic memory interface is not covered): ChibiOS's `mac_lld_init` (`hal_mac_lld.c:306`, `:308`, `:310`). It is a masked read-modify-write of PA1SO (bit 25) and EPIS (bits 23:21). **Nothing at the tag writes PC2SO or PC3SO.**
- What `check pinout` reads holds no PMCR value. The writer's operands are instruction immediates, and no `hwdef.h` macro carries one (RESULTS M3).
- **D1 — what row L1129 becomes.** Options:
  - **(i) Lean.** Record it as a source fact: the pincheck README names the one writer and its bits at `fb773567`, with the re-read due on any ArduPilot re-pin. The row closes as "no writer at the tag; nothing to judge from the image". No code.
  - (ii) Pincheck reads `HAL_USE_MAC` in `hwdef.h` and names a board that links the writer. This is a proxy for the writer, not a witness of PC2SO, and it would refuse an Ethernet board whose writer is harmless.
  - (iii) The real witness is a write: the twin's foreign judge already logs peripheral writes (`2311`, `LogPeripheralAccess`), so it could name any PMCR write that sets bit 26 or 27. The twin is outside this fence: a backlog row.
  - The lean is (i) now and (iii) as the row's new owner.

### S2 · Sentinel-03's battery monitor cannot build in unmodified ArduPilot — a pin would have to move

- The solver puts `VBAT_SENSE` on **`PF3`, whose only channel is `ADC3_INP5`** (`adc_inputs`; the solver takes ADC1, then ADC3, `pin_devices.rs:2119`, `1211` Q8).
- ArduPilot parses `PF3 BATT_VOLTAGE_SENS ADC3 SCALE(1)`, but the build fails. Its H7 mcuconf enables ADC12 unconditionally (`stm32h7_mcuconf.h:420`), and ChibiOS then needs an ADC12 DMA stream, which the script allocates only from an ADC1 or ADC2 pin (`hal_adc_lld.h:339` `#error`, measured).
- Redefining `STM32_ADC_USE_ADC12 FALSE` is itself an error (`-Werror`, measured). No board in the tree is ADC3-only.
- On an ADC1 pin it builds (`s03adc1`). Its clean `check pinout` verdict says nothing about the sense: **today's judge does not see a sense pin**. (1)'s judge adds that: the `HAL_ANALOG_PINS` row and `HAL_BATT_VOLT_PIN` against the topology's `adc_inputs`.
- An ADC1-only rule, scratch, moves Sentinel-03's sense `PF3` → **`PC0`** (`ADC1_INP10`, position 26). Nothing else moves.
- **A specification cannot pin a sense today:** `power.sense` has no pin key (`power.rs:329-336`), and the solver says "a sense is never pinned".
- **D2 — where the pin moves, and who moves it.** In every option, `hwdef` refuses by name, in 4410, any topology whose emitted ADC lines include no `ADC1` line: a new refusal, say `AdcWithoutAdc1`, citing `stm32h7_mcuconf.h:420` and `hal_adc_lld.h:339`. The options differ in how Sentinel-03 then reaches an ADC1 pin:
  - **(a) Lean.** The solver's sense rule prefers the lowest-position free pin **with an ADC1 channel**, and falls back to ADC3 as today. This is an IR touch outside this fence (`solve.rs`, `pin_devices.rs`), for an IR work order or a fence widening. It moves `sentinel-03`'s and `sentinel-02-power`'s sense to `PC0` (scratch, measured: nothing else moves). No specification key is added. It is the order `choose_channel` already prefers, applied to the pin.
  - (b) A new specification key, `power.sense[].pin`, and Sentinel-03 declares an ADC1 pin. That is a specification schema bump, which this fence excludes.
  - (c) `4300k` reserves Sentinel-03's eight ADC3-only pins PF3–PF10 in `mcu.reserved_pins`. No schema change, but eight reservations to steer one net (measured: it lands on PC0).
  - (d) hwdef emits a second, unsolved ADC1 line to obtain the DMA stream. Rejected: that is a fact with no source (`INV-06`).

## What §2's acceptance meets

- **D3 — Sentinel-03 also needs two specification keys.** With the sense bypassed, Sentinel-03 is refused `ImuOrientationUndeclared` (`U5`). It declares no `orientation` and no `board.forward`. Adding `{x: right, z: up}` (as `sentinel-02-own` declares) and `forward: top` in a scratch copy, it generates, and no pin moves.
  - Both are specification keys, which this fence excludes. U5's real orientation is a fact of the drawn board, not mine to choose.
  - **Lean:** `4300k` declares both. Until then, 4410 proves "Sentinel-03's hwdef generates" on a test-derived copy built in memory: the committed specification plus the two keys, solved under D2's rule, never written. The committed Sentinel-03 generates when D2's pin and `4300k`'s keys land. Alternatively, the architect widens this fence to those two keys of `sentinel-03.json`.
- **D4 — `hse-crystal-16` cannot generate after (2):** it declares no console (`NoConsole`). **Lean:** the "16 MHz topology generates" test copies `hse-crystal-16`'s `mcu.hse.crystal` into `serial-dshot-9600` in memory and solves it. No new specification file is added.
- **D5 — the WO's `HAL_BATT_MONITOR_DEFAULT 4` is current sensing.** Voltage only is `3` (`AP_BattMonitor_Params.h:17-18`). **Lean:** `3`.

## The shapes asked for

- **(1) field shape. Lean:** `SolvedSense` gains the two resistances `power.sense` declares; the channel is already solved in `adc_inputs`.
  ```json
  "sense": { "VBAT_SENSE": { "pin": "PF3", "position": 13, "channels": ["ADC3_INP5"],
                             "top_ohms": 100000, "bottom_ohms": 10000 } }
  ```
  Solved 18 → 19. hwdef reads `adc_inputs[net]` for the ADC and channel, and `top_ohms` and `bottom_ohms` for the scale.
- **D6 — the scale's text.** Both forms build (`s03adc1`, `s03quot`):
  - **(a) Lean.** The exact quotient `define HAL_BATT_VOLT_SCALE (110000.0f/10000.0f)`, numerator `top + bottom` and denominator `bottom`. Pincheck reads two integers and compares them exactly with the topology, by its own arithmetic. Both are exact in `float` up to 2²⁴ Ω, and hwdef refuses a divider above that by name. The `float` quotient is then the correctly rounded ratio, which is what `VOLT_MULT` (an `AP_Float`) holds.
  - (b) A decimal literal (`11.0`, as 52 boards write it), the shortest that rounds to the same `float`. Pincheck then judges by comparing roundings. §5's "decimal rounding" and "100 k / 9.09 k" cases are where (b) can go wrong and (a) cannot.
- **(1) lines (lean):**
  ```
  <pin> BATT_VOLTAGE_SENS ADC1 SCALE(1)
  define HAL_BATT_VOLT_PIN <ADC1 channel>
  define HAL_BATT_VOLT_SCALE (<top+bottom>.0f/<bottom>.0f)
  define HAL_BATT_MONITOR_DEFAULT 3
  ```
  `define HAL_USE_ADC FALSE` is emitted only when no sense is solved.
- **(2) the MCU row. Lean:** `ardupilot.oscillators`, one entry per cited `OSCILLATOR_HZ`, each `{oscillator_hz, at, spi_kernel_hz}`. It replaces `oscillator_hz`, `oscillator_at` and `spi_kernel_hz`; power-parts 10 → 11.
  - `0`: SPI1–3 80 MHz, SPI4–6 72 MHz (cited as today, plus the PLL2 lines the SPI4 row lacks).
  - `16000000`: SPI1–3 80 MHz, SPI4–6 **100 MHz** (RESULTS (2)).
  - `OscillatorNotCited` then names the oscillator and the list of cited ones. hwdef picks the entry by `clock.hse_hz` exactly, so a 16 MHz topology is never judged against the `0` row's kernel clocks, nor the reverse (§5).
- **D7 — (3) text.** **Lean:** `PA13 JTMS-SWDIO SWD PULLUP` and `PA14 JTCK-SWCLK SWD PULLDOWN`: the pulls written out, as every other line states its pull. Both forms build and judge clean. ArduPilot's own 197 lines each are bare: no SWD line in the tree writes a pull.
  - Pincheck judges mode and AF only (alternate AF0, RM0433's reset), from its own reset table.
  - A topology or allow-list naming PA13 or PA14 is refused, never overwritten (§5).
  - The four CI hand boards (`pincheck/fixtures`) gain the same two lines (RESULTS M7).
  - **Measured on all nine CI boards** (four hand, five generated): every build passes, and CI's `ardupilot-pinout.sh` exits 0 with every verdict as on `main`. The SWD §7 condition does not fire.

## Findings beside the fence (for FEEDBACK)

- **F1 · ArduPilot's 16 MHz kernel clocks are not the IR's.** At 16 MHz ArduPilot runs SPI4–6, and USART1/6 and the other USARTs (all PLL2_Q), at 100 MHz, and I2C1–3 on PLL3_R. The IR's solved `achieved_hz`, BRR and I2C timings describe the IR's tree. hwdef writes only SPI rates, which `SpiBelowArduPilotDivider` bounds. The rest is `4240e`'s area, or a row.
- **F2 · `hse-external-8` and `sentinel-02-bench-hse8` hold 8 MHz oscillators,** which this work order does not cite. They stay `OscillatorNotCited` (and `NoConsole`/`NoSerialPort` besides).
- **F3 · `4400b` F2 is live on a committed board:** `solve-stress` is refused `LabelTakesAfPrefix` (`SPI_BUS_0_CS0_NET`). The mutant for (8) has a real board to fail on.

## Taken when ruled (no stop; measured)

- **R9.** Regenerating `pincheck/tests/common/topology.json` turns 9 of 24 `check.rs` tests red, each by the console's PA9 and PA10 read as inputs (RESULTS M5). The test ELFs gain the console's pins.
- **2311 F1.** SERIAL0 is pulled up through `GCS::setup_console` → `find_serial` (`GCS_Common.cpp:2810`, `AP_SerialManager.cpp:605`, `:656`, `:781-783`). The sentence will say every port ArduPilot opens.
- **(4)–(6), (8)** need no ruling. (6) is reproduced: `GpioNotJudged` is hidden behind `Truncated` and `UnreadableInput`.
