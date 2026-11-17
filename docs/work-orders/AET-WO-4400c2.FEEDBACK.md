# AET-WO-4400c2 — FEEDBACK

Branch `wo/aet-4400c2-twin-dma`, from `main` at `26e4c620`. The work order is read from `~/.cache/aet-4400c2-wo.md`, the architect's copy; it lands by a docs PR and is not committed here. Every answer is labelled **measured**, **read at source** or **inferred**.

## Sources (read at source, nothing run)

- **Renode's models**, read at `renode-infrastructure` `cd4b002aac2398994c4a61a34a8b31b5700facf2`. That is the `src/Infrastructure` submodule of `renode` `f1dd1b4af` ("Add Changelog for v1.17.0", 2026-09-06), the commit the pinned tools image reports (`1.17.0+20260907gitf1dd1b4af`). Each file's sha256 is in `spike/4400c2/renode-src.SOURCE.txt`. `STM32H7_SPI.cs` has the same sha256 (`bc3f1348…`) as the `066a7f13` copy `4230` read.
  - `STM32H7_SPI.cs`: one request GPIO, `DMARecieve` (L85). It blinks when `RXDMAEN` is set and a word is received (L366–371). L163 reads "Transmitting with DMA doesn't require any special logic": no transmit line.
  - `STM32F7_USART.cs`: one request GPIO, `ReceiveDmaRequest` (L87), set while `DMAR` is set and the receive buffer is not empty (L109). `DMAT` is a plain flag (L187): no transmit line.
  - `STM32F7_I2C.cs`: one request GPIO, `DmaReceive` (L111), set while `RXDMAEN` is set and receive data is waiting (L390, L469). `TXDMAEN` is a tag (L133).
  - `STM32_Timer.cs`: **no DMA request GPIO.** `UDE`, `CC1DE`–`CC4DE` and `TDE` are tags (L224–230); its GPIOs are interrupts only (L572–582).
  - `STM32_DMAMUX.cs`: `OnGPIO(number)` sets every output channel whose `DMAREQ_ID` equals `number` (L25–35). So a line wired to `dmamux1@N` serves a firmware only if `N` is the silicon's request ID.
  - Renode's own `stm32h7.repl` wires exactly one such line, `spi4: DMARecieve -> dmamux1@83` (L197). It wires none for `i2c1`–`i2c3` or any USART.
- **The request IDs:** RM0433 Rev 9, Table 124, "DMAMUX1: assignment of multiplexer inputs to resources", p. 697 (local copy sha256 `672a9433d6602da224e5215f571beed2be0e683cb025526f4afe9db61c26b6a0`).
  - Receive: `spi1_rx_dma` 37, `spi2_rx_dma` 39, `spi3_rx_dma` 61, `spi4_rx_dma` 83, `spi5_rx_dma` 85;
  - `i2c1_rx_dma` 33, `i2c2_rx_dma` 35, `i2c3_rx_dma` 73;
  - `usart1_rx_dma` 41, `usart2_rx_dma` 43, `usart3_rx_dma` 45, `uart4_rx_dma` 63, `uart5_rx_dma` 65, `usart6_rx_dma` 71, `uart7_rx_dma` 79, `uart8_rx_dma` 81.
  - The table puts I2C4, SPI6 and LPUART1 on DMAMUX2/BDMA, not DMAMUX1.
- **ArduPilot's DMA map for Sentinel-02's generated board** (CI's `hwdef.h` from run 44051581563; `4400c` FEEDBACK):
  - SPI1 TX DMA1 s1, RX DMA2 s1;
  - SPI2 TX DMA1 s3, RX DMA1 s2;
  - USART1 RX DMA1 s0, TX DMA1 s4;
  - **I2C1 with none** (`HAL_I2C1_CONFIG … SHARED_DMA_NONE`).
- **What the twin emits** (`crates/aethelgard-twin/src/platform.rs`):
  - each SPI controller it adds (`spi1`, `spi2`, …) as `SPI.STM32H7_SPI`, with `IRQ` and no DMA line (L716–726);
  - the stock I2C and USART instances it uses, from `stm32h7.repl` and listed in `STOCK`, never restated.
- **The generated Rust firmware uses no DMA** (`svd-hal` README; `AET-WO-4300a`).

## Predictions — written before any run

| id | What | Prediction |
|---|---|---|
| Q1 | The SPI receive lines (`spiN: DMARecieve -> dmamux1@<spiN_rx>`), emitted in the platform file | `4400c`'s `r3` prints `SPI Flash 0x00EF4017 found` with no overlay, as the overlay did |
| Q2 | Without them | `Unknown SPI Flash 0x00F0BB03`, as `4400c`'s `r2` |
| Q3 | USART1's receive line (`usart1: ReceiveDmaRequest -> dmamux1@41`), for an injected MAVLink 2 `COMMAND_LONG` (`MAV_CMD_REQUEST_MESSAGE`, `AUTOPILOT_VERSION`) on SERIAL0 | with the line, ArduPilot answers with `COMMAND_ACK` (msgid 77) for command 512. Without it, no answer: ArduPilot receives SERIAL0 by DMA (DMA1 s0), and the request never reaches DMAMUX |
| Q4 | I2C1's receive line (`i2c1: DmaReceive -> dmamux1@33`) | no change on Sentinel-02: ArduPilot uses no DMA on I2C1 (`SHARED_DMA_NONE`). The barometers' traffic is byte-identical with and without it |
| Q5 | Transmit lines | none exist in any model, so none can be emitted. Transmit by DMA already works: console TX and SPI TX run without one |
| Q6 | Timers | no line exists in Renode's `STM32_Timer`. A firmware that drives DShot by timer DMA (ArduPilot does) cannot be served by any platform line. That is a **model change, §7's flag**, named and not attempted |
| Q7 | Moved goldens | `boards/sentinel-01/sentinel-01.repl` and `boards/sentinel-01-bench/sentinel-01-bench.repl` (both add `spi1`, and use `i2c1` and a USART only if declared); `crates/aethelgard-twin/src/tests.rs` L74–83 (the `spi1` text); and any CLI test that compares an emitted `.repl` byte for byte. No verdict of the generated firmware moves |

## §1.1 Results — measured after the predictions (`8bf1df18`)

**How it ran.**
- `4400c`'s driver, copied to `spike/4400c2/drive.py`, gained one addition: `PLATFORM` and USART1 byte injection (`INJECT_AT_S`, `INJECT_HEX`, one `WriteChar` per byte).
- It boots CI's unmodified ArduPilot ELF (sha256 `c8a2c9fa…`, run 44051581563) under the CLI's own monitor script for `sentinel-02-own`. That script was captured again from the unmodified CLI booting the board's own Rust firmware (`spike/4400c2/sh/gen.sh`: `pass`).
- **The candidates are platform files, not overlays.** `spike/4400c2/candidates.py` writes the emitted `sentinel-02-own.repl` with each candidate's lines in the file, as the emitter would write them (`spike/4400c2/out/candidates/`).
- Each run is 3 s in 500 ms steps; the same MAVLink 2 `COMMAND_LONG` is injected into USART1 at 1.5 s. Containers ran through `capped-run` with the caps read back (`out/caps.log`).
- One line per run is in `spike/4400c2/out/summary.jsonl`.

### The candidates

| Run | Lines in the platform | Dataflash (console) | Reply to one `COMMAND_LONG` (44 B) | Reply to three (132 B) | Barometer traffic (model lines) |
|---|---|---|---|---|---|
| `p0-emitted` | none (today's emitter) | `Unknown SPI Flash 0x00F0BB03` | none | none; **0 DMA reads** of USART1's `RDR` | first at 1.973600630 s |
| `p1-spi` | `spi1 … @37`, `spi2 … @39` | **`SPI Flash 0x00EF4017 found`** | none | — | first at 0.344289850 s |
| `p2-spi-usart` | `p1` + `usart1: ReceiveDmaRequest -> dmamux1@41` | found | none: 44 DMA reads, then no hand-off (below) | **`COMMAND_ACK` (cmd 512, result 0) and `AUTOPILOT_VERSION`**; 64 DMA reads; **then a hard fault at 1.601556130 s** (below) | same as `p1` |
| `p3-spi-usart-i2c` | `p2` + `i2c1: DmaReceive -> dmamux1@33` | found | none | — | **byte-identical to `p2`** (700 lines, sha256 `4138879f…`; console sha256 `399e6bb1…` identical) |
| `p4-usart` | `usart1 … @41` only | `Unknown SPI Flash 0x00F0BB03` | none | `COMMAND_ACK` and `AUTOPILOT_VERSION`; 64 DMA reads; no fault | first at 1.973609580 s |
| `p5-wrong` (adversarial) | `spi1 … @39`, `spi2 … @37`, `usart1 … @43` | `Unknown SPI Flash 0x00F0BB03` | — | none; 0 DMA reads | as `p0` |

- **SPI's receive line is needed (measured).** With it in the file and no overlay, ArduPilot finds the dataflash. Without it, or on the wrong number, it reads garbage. Q1 and Q2 hold, and §2's first acceptance line is met on a candidate file.
- **USART1's receive line is needed, but it is not enough (measured).** Without it the DMA never reads `RDR`, so ArduPilot can never receive on SERIAL0. With it the DMA moves every byte, but ArduPilot hands a buffer on only at transfer-complete (64 bytes, `RX_BOUNCE_BUFSIZE`, `UARTDriver.h` L25) or at the USART's IDLE interrupt (`UARTDriver.cpp` L592; the 1 kHz tick at L1113–1160 only re-arms a stopped stream). **Renode's `STM32F7_USART` never raises IDLE:** `IDLEIE`, `IDLE` and `IDLECF` are tags (L154, L231, L253). So a frame under 64 bytes is never delivered; one that fills the buffer is answered. That is a **model change, §7's flag**, outside the platform description.
- **I2C1's receive line changes nothing for this ELF (measured):** ArduPilot uses no I2C DMA here (`SHARED_DMA_NONE`). It is a candidate only in the work order's "wherever Renode's model has the line" sense.
- **A wrong number fails visibly (measured, `p5-wrong`):** swapping SPI1's and SPI2's numbers and moving USART1 to USART2's 43 brings back the garbled ID and stops every receive DMA. DMAMUX matches the request ID exactly (`STM32_DMAMUX.cs` L25–35).

### The hard fault behind USART1's line (measured; cause not established)

- **The fault.** In `p2-spi-usart` with three frames at 1.5 s, ArduPilot answers, then faults at **1.601556130 s**: `HardFault_Handler`, `CFSR` `0x00020000` (UsageFault `INVSTATE`: a branch to an address with the Thumb bit clear, so a corrupted code pointer), `HFSR` `0x40000000` (`FORCED`), `LR` `0xFFFFFFED`.
- **What prints.** CrashCatcher prints `CRASH ENCOUNTERED` / `Enable logging and then press any key to start dump.`, then polls USART1 (`CrashCatcher_getc`). With USART1's access logging on, that polling wrote a 5.4 GB log, which is not kept.
- **Deterministic:** two further runs (`crash-a`, `crash-b`) fault at the same instant.
- **Timing-dependent:**
  - injected at 2.5 s instead (`crash-late`), there is no fault: `COMMAND_ACK`, `AUTOPILOT_VERSION` and a STATUSTEXT;
  - without the SPI lines (`p4-usart`) there is no fault;
  - with one 44-byte frame (`p2-spi-usart`) there is no fault.

  1.6 s is where the dataflash logger starts `Chip erase` over SPI1's DMA in these runs.
- **What Renode logged at the injection instant:** `dma1: Attempting to create DMA request with no data to send. Ignoring operation.` and `dma1: Error in DMA configuration detected. Ignoring transfer request.` (`out/runs/p2-spi-usart-x3/dma-errors.log`).
- **Inferred, not established:** Renode's `STM32DMA` mishandles the receive stream's re-arm next to SPI DMA and corrupts memory. That would make it a twin artifact rather than ArduPilot's. A build that emits USART1's line exposes this for the first time.

### §1.1's table: each peripheral the twin emits or uses

| Peripheral (twin) | Renode 1.17.0 model and its request GPIO (`cd4b002a`) | RM0433 Rev 9 Table 124 (p. 697) | ArduPilot's use on Sentinel-02 (`hwdef.h`) | Needed? (measured) |
|---|---|---|---|---|
| SPI1–SPI5 (added `spiN`, `STM32H7_SPI`) | `DMARecieve`, receive only (L85, L366–371); transmit needs none (L163) | `spi1_rx_dma` 37, `spi2_rx_dma` 39, `spi3_rx_dma` 61, `spi4_rx_dma` 83 (Renode's own `spi4` wiring), `spi5_rx_dma` 85 | SPI1 RX DMA2 s1, SPI2 RX DMA1 s2 | **yes**: the dataflash is found only with it (`p1`) |
| USART1–3, UART4–5, USART6, UART7–8 (stock, `STM32F7_USART`) | `ReceiveDmaRequest`, receive only (L87, L109) | 41, 43, 45, 63, 65, 71, 79, 81 | USART1 RX DMA1 s0 | **yes**, to receive at all (`p0` against `p2`/`p4`). It is insufficient without IDLE (model), and it exposes the fault above |
| I2C1–3 (stock, `STM32F7_I2C`) | `DmaReceive`, receive only (L111, L390) | `i2c1_rx_dma` 33, `i2c2_rx_dma` 35, `i2c3_rx_dma` 73 | none | **no effect for this ELF** (`p3` byte-identical to `p2`) |
| I2C4, SPI6, LPUART1 | — | on DMAMUX2/BDMA, not in Table 124 | — | not applicable |
| Timers (stock, `STM32_Timer`) | **no request GPIO**: `UDE`, `CCxDE` and `TDE` are tags (L224–230) | e.g. `TIM1_UP` 15, `TIM5_UP` 59 | none on this board (no motor output) | **cannot be served by any platform line.** ArduPilot's DShot uses timer DMA, so a DShot board needs a model change (§7's flag) |
| Transmit, every peripheral | no transmit line in any model | the `_tx_dma` IDs exist (38, 40, 42, …) | SPI1/2 TX, USART1 TX | not needed: SPI and console transmit work without one (`4400c`, measured) |

## The lines proposed to emit (for the ruling; nothing built)

From ST's facts and the solved topology alone (`RUL-16`). Each number is quoted from RM0433 Rev 9 Table 124 as a cited constant table in `aethelgard-twin`, never typed without its source (`INV-06`):

1. **For each SPI controller the twin adds** (`spiN`, solved): `DMARecieve -> dmamux1@<spiN_rx_dma>`, inside its entry. **Measured needed.**
2. **For each solved serial port's USART/UART** (stock): an update block `usartN:` / `ReceiveDmaRequest -> dmamux1@<…_rx_dma>`, in the twin's existing update style (`flashController:`, `gpioPortA:`). **Measured needed to receive at all,** with the IDLE gap and the fault above named. The ruling decides whether it is emitted now, or held until the DMA artifact is understood.
3. **For each solved I2C controller** (stock): `i2cN:` / `DmaReceive -> dmamux1@<i2cN_rx_dma>`. **No measured effect here**; emitted only if the ruling wants every line Renode's model has.
4. **Nothing** for transmit or timers: no model line exists.

## Moved goldens (read at source)

| File | Moves under | Why |
|---|---|---|
| `boards/sentinel-01/sentinel-01.repl` | 1, 2, 3 | it adds `spi1`, and its specification (`examples/sentinel-01-minimal.json`) declares `CONSOLE`, I2C `SENSORS` and SPI `FLASH` |
| `boards/sentinel-01-bench/sentinel-01-bench.repl` | 1, 2, 3 | `examples/sentinel-01.json`: `CONSOLE`, I2C `SENSORS` and `EXT`, SPI `FLASH`. **`3402` re-blesses this board:** its inbox is told |
| `crates/aethelgard-twin/src/tests.rs` L74–83 | 1 | it states `spi1`'s emitted text byte for byte |
| `crates/aethelgard-cli/tests/generate_tier1.rs` L74–92 and the `attribute_seeded` regeneration guard (L54) | 1, 2, 3 | they compare `boards/sentinel-01/**` with a fresh `generate`; they move through the `.repl` above, not by code |

- No other file embeds `.repl` text (`git ls-files | xargs grep -l 'STM32H7_SPI @'`: the two boards, `twin/src/tests.rs`, Renode's vendored copy, and one FEEDBACK).
- Campaign archives name the topology and the firmware, not the platform, so no archive moves (inferred from `campaign.rs`'s header artifacts).
- **No generated-firmware verdict should move:** that firmware enables no DMA (`svd-hal` README). This is measured at the build.

## Each prediction beside its result

| id | Predicted | Result | |
|---|---|---|---|
| Q1 | SPI lines emitted: `SPI Flash 0x00EF4017 found` | yes (`p1`, `p2`, `p3`) | held |
| Q2 | without them: `Unknown SPI Flash 0x00F0BB03` | yes (`p0`, `p4`) | held |
| Q3 | USART1's line: ArduPilot answers an injected `COMMAND_LONG` | **only for a buffer-filling burst** (132 B), not for one 44 B frame: Renode's USART has no IDLE. And with the SPI lines, at 1.5 s, the run then hard-faults | missed in part |
| Q4 | I2C1's line: no change | byte-identical | held |
| Q5 | no transmit lines exist | none in any model | held |
| Q6 | timers have no line; a model change | `STM32_Timer` has none | held |
| Q7 | the goldens listed | as the table above, plus the CLI regeneration guards it did not name | held, widened |

## Decisions needed

- **D1 (architect).** Which of lines 1–3 to emit. Recommended: **1 now** (measured needed, no side effect seen). **2** with its fault named as a twin residual, or held until the fault is understood. **3** only for completeness.
- **D2 (architect).** USART IDLE and timer DMA are Renode model gaps (§7). Where they go: a model of our own, upstream Renode, or backlog until a board needs them (`4400d`'s console receive, a DShot board).
- **D3 (architect).** The hard fault: whether to chase it before line 2 is emitted.
