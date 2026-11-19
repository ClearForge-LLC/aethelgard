# AET-WO-3304 — Bench preparation: the generated firmware reads the real BMP581 and ICM-42688-P on the Nucleo, every prediction written before the bench

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3304-bench-sensors`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P3 bench (`AET-WO-3302`'s method) and P4 (the flight sensors). Written 2026-11-19. **Cadence: preparation, ending in a stop.** Nothing here is run on silicon; the bench is Scotty's.

**Grounds:**
- **The method is `3302`'s.** The generated firmware runs on the real STM32H753 (NUCLEO-H753ZI, rev V die), and every row of the bench sheet is predicted by the twin **before** the bench. A disagreement is the result (`INV-07`; `northstar.md` §2's scars). Read `AET-WO-3302.md` and `AET-WO-3302.bench.md` first.
- **The parts are on the bench** (Scotty, 2026-11-18/19): a BMP581 breakout and an ICM-42688-P breakout, both 3.3 V; the full E-series resistor kit and capacitors.
- **The instruments:**
  - ST-LINK V3 with `probe-rs` 0.32.0 on t9o777.
  - The FNIRSI 2C53T scope (50 MHz, 250 MSa/s, firmware V1.2.0).
  - **New:** an FX2 logic analyzer (Saleae Logic clone, 8 channels, 24 MSa/s, 1.6 V threshold), driven by `sigrok-cli` 0.7.2 on t9o777. It decodes I2C and SPI to bytes, so the bench can compare **the transaction on the wire** with the twin's prediction, not only the register image.
- **The generated firmware already reads both identities.** Sentinel-02's firmware reads the BMP581 over I2C and the ICM-42688-P over SPI (`4400e`, `4220c`; identity records, trace kind 13). The question is whether that firmware, or a bench specification built the same way, runs on the Nucleo with the breakouts wired to header pins.

> **What this is:** the bench sheet for the two flight sensors on the Nucleo, produced by the engine and the twin:
> - the ELF and its hash;
> - the pin map to the Nucleo's headers, with UM2407's conflicts;
> - the register image and identity records predicted;
> - the bus transactions predicted, byte for byte, as `sigrok-cli`'s I2C and SPI decoders will print them;
> - the exact commands to run.
>
> It is NOT:
> - firmware written by hand. Capability enters through the specification; no engine code names a board (roadmap §6);
> - a change to any crate, model or golden;
> - the bench run itself.

## 1. Preparation

1. **Which firmware.**
   - Establish whether Sentinel-02's generated ELF (`sentinel-02-own.json`, or the fixture CI's `generated (sentinel-02)` job builds) can drive both sensors on the Nucleo:
     - which pins and peripherals it uses;
     - whether each pin reaches a Nucleo header (UM2407: Arduino, ST morpho), and what else sits on it (Ethernet PHY, LEDs, USB, the ST-LINK VCP; `3302` B0 is the precedent).
   - **If it can:** use it.
   - **If it cannot:** propose a bench specification (`examples/` or `boards/`, your call with a reason) that the engine solves onto header pins with the same parts.
     - It is solved, generated and simulated like any other board, and nothing in the engine names it.
     - Its SPI clock is one the analyzer can decode: at most about 4 MHz at 24 MSa/s, so state the rate the solve picks.
   - Either way, the ELF is built by `main`'s CLI in the pinned tools image, and its sha256 is recorded.
2. **The predictions, from the twin, before any bench:**
   - The register rows `3302` B2 checked, for the peripherals this firmware uses, masked as `3302` did.
   - **Each identity record** (address, expected and observed lines, status codes):
     - as the twin reports it with the device modelled;
     - the timeout code with nothing wired, as `3302`'s BME280 row did.
   - **The transactions, byte for byte, as `sigrok-cli` will print them:**
     - the I2C decoder's address, R/W, ACK/NAK and data for the BMP581's identity read (`CHIP_ID` 0x01, expected 0x50; DS-000347 for the ICM's `WHO_AM_I` 0x75, expected 0x47);
     - the SPI decoder's MOSI/MISO pairs for the ICM.
     - Give the exact decoder options (`-P i2c:scl=D0:sda=D1`, `spi:clk=…:mosi=…:miso=…:cs=…:cpol=…:cpha=…`). Name the channel assignment as the analyzer's CH1–CH8, which is sigrok's D0–D7.
   - **Timing that the analyzer can check and the twin predicts:** the I2C SCL period, the SPI clock and the inter-transaction gaps where the twin pins them.
     - Say which of these are model conventions (the `4300a` timer conventions, `4400f`'s whole-ns realization), and which the twin does not claim at all.
3. **The bench sheet:** `docs/work-orders/AET-WO-3304.bench.md`, in `3302.bench.md`'s form.
   - Predictions are written first; blank columns are for the bench. Never edit a prediction.
   - Rows: flash and verify, the register image, each identity record, each decoded transaction, and the pull-ups.
     - **The pull-ups:** a breakout carries its own. Read them off the breakout or measure them with the meter. Say what the campaign predicts for the bus rise at that value, with the bench's C taken from `3302`'s measured 55 pF and the analyzer's 5 pF input added.
   - **Wiring:** a table of breakout pin → Nucleo header pin → analyzer channel, with ground first and the analyzer's PWR and CLK pins left unconnected.
   - **The commands:** `probe-rs` and `sigrok-cli`, copy-paste ready for t9o777. `probe-rs` is at `~/tools/probe-rs/bin/probe-rs`, and the `claude` user can open the analyzer through the `sigrok` group.

## 2. Stop

**STOP** with FEEDBACK holding:
- the firmware choice and its reason;
- every prediction and its source run;
- the pin conflicts;
- anything the twin cannot predict, said plainly.

Push an `AET-WO-3304 STOPPED:` commit and open a PR. It is merged as the bench record after the architect's review. Write `~/.cache/aet-3304/pr-title-body.md`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-20`.
- **Fence:**
  - `docs/work-orders/AET-WO-3304.bench.md` and FEEDBACK;
  - a bench specification under `examples/` or `boards/`, if §1.1 needs one;
  - nothing else.
  - **No crate, model, golden, test, workflow, `ci/` or `container/` file.** A gap in the engine is a finding, not a fix.
- **Running beside you:**
  - `AET-WO-4400g` (the twin's clocks): your predictions of timer-derived timing may move when it merges. Name which.
  - `AET-WO-1910` (CI and tests).
  - `AET-WO-4201` (a parts spike).
  - You share no file with any of them.
- You run the tools image: one container of yours at a time, under `capped-run`. **No access to t9o777 or the bench hardware.**
- One PR.

## 5. Adversarial pass, targeted

- **A prediction that could not fail:** one written after a run that already shows the answer, or a band so wide no reading misses it.
- **A decoder line that differs from the twin's transaction in a way the sheet would call agreement** (a repeated start shown as stop-start, for instance).
- **A pin that is on a header but already driven by something on the Nucleo.**

## 7. Flag-and-stop conditions

- §2 is the stop.
- **Neither firmware can reach both sensors on header pins:** stop with the pin table and the options.
- **The twin's prediction for a row depends on the ICM model's undocumented behaviour** (`4220c` B1–B4): say so in that row. Do not choose for the bench.
