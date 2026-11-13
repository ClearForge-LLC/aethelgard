# AET-WO-3302 — The bench comparison: the generated firmware and the reference bus on a real STM32H753, against the simulation

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`.
**Author:** Claude (architect) · **Bench:** Scotty (human-track); the architect prepares the inputs and folds the results.
**Phase:** P3. This work order closes **P3 gate clause 4**:

> "The bench comparison's disagreements are written into `northstar.md` §2 as the project's first scar, by amendment, whatever they are."

**Grounds:**
- **`RUL-19` (1).** The campaign's rise times are computed by ngspice from **declared** values (a 2.2 kΩ pull-up, 100 pF). `extract` bounds the routed board's capacitance, but nothing has yet been compared with a real bus. This is the project's only evidence about that gap.
- **`RUL-08`.** The reference target is the STM32H753ZI on the Nucleo-H753ZI, which arrived on 2026-11-13. It is not Sentinel-01's board: what transfers is the MCU, the generated firmware and the I2C bus physics, not the routed copper.
- **The solved pins:**
  - I2C1: SCL `PB6`, SDA `PB7`, AF4;
  - SPI1: SCK `PA5`, MISO `PA6`, MOSI `PA7`, AF5;
  - flash chip select: `PA0`.

  Source: `boards/sentinel-01/src/board.rs`.
- **The devices and the bus:**
  - BME280 at `0x76`; TMP117 at `0x48`;
  - 400 kHz;
  - 2.2 kΩ declared pull-up, 100 pF declared capacitance, 300 ns ceiling.
- **The scenarios:**
  - `pullup-2k2` passes;
  - `pullup-47k` fails with `analog_rise_exceeds_ceiling` and `i2c_timeout`;
  - `propose` answers 47 kΩ with 2.7 kΩ (`AET-WO-3200`).

> **What this is:** one evening at the bench. The generated firmware runs on the real chip, the reference bus is wired with real sensors, and three pull-ups are measured against what the campaign said.
>
> It is NOT:
> - Sentinel-01's board;
> - a fabrication order;
> - a fix for whatever disagrees. Disagreements are recorded, not explained away.

## 0. Before the bench (the architect, on Furnace)

1. **The ELF:** `sentinel-01.elf` from a passing `verify --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out`, with its sha256 as the verdict names it.
2. **The simulated expectations,** in `docs/work-orders/AET-WO-3302.bench.md`, the bench sheet:
   - the register image the boot judge checks (address, mask and value), from `simulate --firmware`;
   - the firmware's result words and their addresses;
   - the campaign's rise times and outcomes for 2.2 kΩ, 2.7 kΩ and 47 kΩ;
   - the 0.8473 × R × C prediction at the bench's own measured capacitance (filled in at B3).
3. **The bench sheet's blanks:** every measurement below has a row with its prediction beside it, **written before the bench**.

## 1. The bench (Scotty)

**Safety and setup:**
- Everything is 3.3 V; nothing touches 5 V.
- Power comes from the ST-LINK USB only. The bench supply is not used in this work order.
- Use a **10× scope probe.** A 1× probe adds about 100 pF and would swamp the measurement.
- Measure rise time from **30 % to 70 % of 3.3 V (0.99 V to 2.31 V)**, as UM10204 and our records define it.

**B0. Pin conflicts.** Before wiring, check each solved pin (`PB6`, `PB7`, `PA0`, `PA5`, `PA6`, `PA7`) against the Nucleo-H753ZI user manual (UM2407), and record every on-board use.
- Expected, to confirm: a user LED on `PB7` and the Ethernet PHY on `PA7`.
- A load on SDA changes the rise time itself. Disconnect it by its solder bridge, which is reversible, and note which ones were opened.
- If a conflict cannot be removed reversibly, **stop** (§7).

**B1. Flash.**
- Flash the ELF with `probe-rs`.
- Record the `probe-rs` version, the ELF's sha256, and the chip `probe-rs` reports.

**B2. The register image.** With no sensors wired, let the firmware run, then read every address on the sheet (`probe-rs read b32 <addr> 1`). Write down each value beside the simulated one.
- On real silicon the transfer will time out with no device present, so the firmware's I2C outcome here is expected to be a timeout.

**B3. 2.2 kΩ.**
- **Wire the bus:** the 3.3 V BME280 and TMP117 breakouts on `PB6` and `PB7`, with their own pull-ups removed. BME280's SDO goes low (0x76); TMP117's ADD0 goes to GND (0x48). Then one 2.2 kΩ to 3.3 V per line.
- Reset, and record the result words (the BME280 ID should read `0x60`).
- **Scope SDA and SCL:**
  - 30–70 % rise time on each line;
  - one screenshot per line;
  - the bit period, which shows whether the clock was stretched.
- **Compute the bench's own capacitance:** C = t_rise ÷ (0.8473 × 2,200 Ω).

**B4. 47 kΩ.** Replace both pull-ups.
- Record the result words, the rise times, the SCL period and whether the read succeeded.
- **The prediction to test:** the campaign models 47 kΩ as a timeout. A real STM32 I2C controller waits for SCL to go high (clock synchronisation), so the real bus may run slower and succeed, or fail on SDA timing instead.

**B5. 2.7 kΩ**, the proposal. Same records as B4.

**B6. Optional: at the declared 100 pF.** Add a C0G capacitor per line so that the bench's C plus the capacitor is about 100 pF. Repeat B3 to B5's rise times only.

Send the sheet, the screenshots and the `probe-rs` output to the architect.

## 2. Afterwards (the architect)

- **FEEDBACK:** every measurement beside its prediction, and every disagreement named with its size.
- **A draft amendment to `northstar.md` §2:** the first scar, worded as what the simulation claimed and what the bench showed. **Scotty ratifies it** (P3 gate clause 4). Then the P3 exit gate is assessed whole.
- **Work orders for what the scar implies:** for example, the campaign modelling clock stretching. These are written, not built here.

## 3. Acceptance

- B0 to B5 are recorded, with predictions written before the bench.
- The scar is drafted, and ratified or amended by Scotty.
- P3's exit gate is reconciled in `roadmap.md`.

## 7. Flag-and-stop conditions

- A solved pin conflicts with on-board hardware that cannot be disconnected reversibly. The architect then pins the bus elsewhere and regenerates the ELF before the bench.
- The ELF does not boot on the chip at all. Record what `probe-rs` shows (PC, fault registers), and do not debug at the bench beyond that.
- Anything at 5 V, or any doubt about it.
