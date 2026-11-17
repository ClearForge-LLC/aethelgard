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
- **The request IDs:** RM0433 Rev 9, Table 124, "DMAMUX1: assignment of multiplexer inputs to resources", p. 697. Read from `pdftotext` of the PDF with sha256 `672a9433d6602da224e5215f571beed2be0e683cb025526f4afe9db61c26b6a0` (`~/.cache/aet-4230/ds/rm0433-rev9.pdf`). RM0433 covers the H742, H743/753 and H750, with one DMAMUX1 mapping (§17.3.2: "hardwired"; channels 0–7 to DMA1, 8–15 to DMA2). Table 125 (p. 698) bears the same title; every request ID used here reads the same in both tables' rows (measured by extracting both). ChibiOS's `stm32_dmamux.h` L72–77 agrees (37–42).
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
- **USART1's receive line is needed, but it is not enough (measured).** Without it the DMA never reads `RDR`, so ArduPilot can never receive on SERIAL0. With it the DMA moves every byte, but ArduPilot hands a buffer on only at transfer-complete (64 bytes, `RX_BOUNCE_BUFSIZE`, `UARTDriver.h` L25) or at the USART's IDLE interrupt (`UARTDriver.cpp` L592; the 1 kHz tick at L1113–1160 only re-arms a stopped stream). **Renode's `STM32F7_USART` never raises IDLE:** `IDLEIE`, `IDLE` and `IDLECF` are tags (L154, L231, L253). Measured: ArduPilot writes `CR1` = `0x2000013D` at 3.2 ms, and Renode warns `Unhandled bits: [4, 29] … Tags: IDLEIE` (`out/runs/p2-spi-usart/usart1.log`). So a frame under 64 bytes is never delivered; one that fills the buffer is answered. That is a **model change, §7's flag**, outside the platform description.
- **I2C1's receive line changes nothing for this ELF (measured):** ArduPilot uses no I2C DMA here (`SHARED_DMA_NONE`). It is a candidate only in the work order's "wherever Renode's model has the line" sense.
- **A wrong number fails visibly (measured, `p5-wrong`):** swapping SPI1's and SPI2's numbers and moving USART1 to USART2's 43 brings back the garbled ID and stops every receive DMA. DMAMUX matches the request ID exactly (`STM32_DMAMUX.cs` L25–35).

### The hard fault behind USART1's line (measured; cause not established)

| Run | Lines | Injection at 1.5 s | Result |
|---|---|---|---|
| `p2-spi-usart-x3` | SPI + USART1 | 132 B written at one instant | `COMMAND_ACK` and `AUTOPILOT_VERSION`, **then HardFault at 1.601556130 s** |
| `crash-a`, `crash-b` | the same | the same | **HardFault at 1.601556130 s** in both: deterministic |
| `crash-late` | the same | at 2.5 s instead | no fault; `COMMAND_ACK`, `AUTOPILOT_VERSION`, a STATUSTEXT |
| `p4-usart-x3` | USART1 only | 132 B at one instant | no fault; `COMMAND_ACK` |
| **`paced`** | SPI + USART1 | **132 B paced at the declared baud** (86,806 ns a byte, 115200 8N1) | **HardFault at 1.511036822 s**: 11.04 ms in, at about byte 127, the second 64-byte buffer's turnover |
| `paced-p4-x3` | USART1 only | 132 B paced | no fault; two `COMMAND_ACK`s and two `AUTOPILOT_VERSION`s (two buffers handed on) |
| `paced-p2-x2` | SPI + USART1 | 88 B paced (one turnover) | no fault; one `COMMAND_ACK` |

- **Every fault looks the same:** `HardFault_Handler`, `CFSR` `0x00020000` (UsageFault `INVSTATE`: a branch to an address with the Thumb bit clear, so a corrupted code pointer), `HFSR` `0x40000000` (`FORCED`), `MMFAR` and `BFAR` 0, `LR` `0xFFFFFFED`. CrashCatcher prints `CRASH ENCOUNTERED` / `Enable logging and then press any key to start dump.` and polls USART1 (`CrashCatcher_getc`). With USART1's access logging on, that polling wrote a 5.4 GB log in `p2-spi-usart-x3`; only its head, up to the fault, is kept (`out/runs/p2-spi-usart-x3/usart1-to-fault.log`).
- **When it happens.** It needs the SPI lines, USART1's line, a second receive-buffer turnover, and the dataflash's SPI DMA busy: the chip erase runs from 0.167124 s to 1.602072 s in every run with the SPI lines (`erase started` and `erase done` on the console). The unpaced fault (1.601556 s) falls 0.5 ms before the erase ends; the paced one at the second turnover. An injection after the erase does not fault.
- **The stimulus is not the cause.** Paced at the declared baud, the fault comes earlier, not later, so it is not an artifact of writing every byte at one instant.
- **Renode's two DMA error lines do not discriminate.** `dma1: Attempting to create DMA request with no data to send` and `Error in DMA configuration detected` appear in every unpaced burst, faulting (`crash-a`) or not (`p4-usart-x3`, `crash-late`), and in no paced run.
- **Inferred, not established:** Renode's `STM32DMA` mishandles USART1's receive stream (DMA1 stream 0) re-arming while SPI1's streams (DMA1 s1, DMA2 s1) are active, and corrupts memory. That would make it a twin artifact. The other reading, an ArduPilot fault on silicon, is not excluded.

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

## Moved goldens (read at source; not measured by a prototype emitter. Where lines 2 and 3 land in each file is unchecked)

| File | Moves under | Why |
|---|---|---|
| `boards/sentinel-01/sentinel-01.repl` | 1, 2, 3 | it adds `spi1`, and its specification (`examples/sentinel-01-minimal.json`) declares `CONSOLE`, I2C `SENSORS` and SPI `FLASH` |
| `boards/sentinel-01-bench/sentinel-01-bench.repl` | 1, 2, 3 | `examples/sentinel-01.json`: `CONSOLE`, I2C `SENSORS` and `EXT`, SPI `FLASH`. **`3402` re-blesses this board:** its inbox was told (`~/.cache/aet-bus/3402.md`, an appended note) that nothing has changed yet |
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
- **D3 (architect).** The hard fault: whether to chase it, a Renode DMA question, before line 2 is emitted.

## Outside the fence, named

- **`spike/4400c2/`** (`drive.py`, `candidates.py`, `summarize.py`, `sh/`, `out/`) is outside this work order's fence (`platform.rs`, its tests and goldens, `boards/*/*.repl`, FEEDBACK). It holds §1.1's measurements and their evidence.
- **The build deletes it** (roadmap §6: a spike's build deletes it); FEEDBACK keeps the figures.
- **§2's acceptance run uses `4400c`'s driver unchanged** (as merged with #202), not this one, which adds `PLATFORM` and injection.

**STOPPED at §1.1**, 2026-11-17, for the architect's ruling on D1–D3. `main` at `7b1ce095` (#202) is merged in, and the scratch trees (7.1 GB) are removed. Title and body: `~/.cache/aet-4400c2/pr-title-body.md`.

# The build (§2), to the ruling on PR #204 (comment 6945751393)

## The ruling, applied

- **D1, line 1 (SPI receive): built** (below).
- **D1, line 2 (USART receive): held, not emitted.** It manufactures a deterministic hard fault. The repro for the backlog row the ruling names, in full, since a squash merge drops `5a1bccaf`:
  - the platform: today's emitted `sentinel-02-own.repl` plus `DMARecieve -> dmamux1@37` in `spi1`, `@39` in `spi2`, and the block `usart1:` / `    ReceiveDmaRequest -> dmamux1@41`;
  - the firmware: CI's ArduPilot ELF (sha256 `c8a2c9fa…`, run 44051581563), under the CLI's own monitor script;
  - the stimulus: at 1.5 s virtual, the 44-byte MAVLink 2 frame `fd20000000ffbe4c00000000144300000000000000000000000000000000000000000000000000020101632f` three times, either at one instant (`HardFault` at 1.601556130 s) or one byte per 86,806 ns (`HardFault` at 1.511036822 s);
  - the signature: `CFSR` `0x00020000` (`INVSTATE`), `HFSR` `0x40000000`.

  The scripts are at `5a1bccaf` (`spike/4400c2/`, deleted after).
- **D1, line 3 (I2C receive): not emitted.** Recorded: no measured effect for this ELF (`p3` byte-identical to `p2`).
- **D2, USART IDLE and timer DMA:** to the backlog and to architecture §2 as model divergences. Those are the architect's documents; they are recorded here and nothing protected was edited.
- **`MANIFEST.json` (read at source):**
  - `fab/sentinel-01-bench/package/MANIFEST.json` holds no `.repl` or board hash of its own (keys: `project`, `specification_sha256`, `topology_sha256`, `scenarios`, `preconditions`, `verdict_sha256`, `files`).
  - Its `verdict_sha256` is the sha256 of `verify`'s verdict line (`fab.rs` L689), which lists every generated file's sha256 by path (`verify.rs` L70), the board's `.repl` included. **So `verdict_sha256` moves when the package is next written.**
  - This PR does not rewrite the package. `bench_board_tier1.rs` does not bind `verdict_sha256`, so per-PR CI stays green.
  - Main's `bench-package` job (`--against-fab`) would see the difference until the package is rewritten. 3402 re-blesses that package; whoever merges second regenerates (§3; 3402's inbox was told).
  - **This PR merges second** (3402 merged first, #203, `7d0763eb`), so it carries the regenerated package. The architect dispatches `bench-package` (`write`) on this branch after the AWAITING-PACKAGE push, and the artifact is committed unchanged (below).

## Outside the work order's stated fence, and why

The fence is `aethelgard-twin` (`platform.rs`), its tests and goldens, `boards/*/*.repl` and FEEDBACK. Also changed:
- **`crates/aethelgard-twin/src/lib.rs`:** the `MalformedDmaTable` error and its kind, for refusing a malformed row by name (`INV-06`).
- **`crates/aethelgard-twin/data/dmamux1-spi-rx.tsv`, new:** the ruling's "numbers from the cited table as data".
- **`crates/aethelgard-twin/README.md`:** one sentence naming the table.
- **`crates/aethelgard-svd-hal/tests/order_2203.rs`:** it pins the emitted platform's digest, so it is a golden of the twin's output. 3402, which changes svd-hal, was told which two pins moved.
- **`crates/aethelgard-cli/tests/workspace_shape.rs`:** it fixes the twin crate's files, so the new data file must be named there. It is in 1604's fence; 1604 was told.

## What was built

- **`crates/aethelgard-twin/data/dmamux1-spi-rx.tsv`:** the SPI receive rows of RM0433 Rev 9 Table 124 (SPI1 37, SPI2 39, SPI3 61, SPI4 83, SPI5 85), each with the table's resource name. The citation, the PDF's sha256, and why SPI6 has no row are in the file.
- **`platform.rs`:**
  - `DMAMUX1_SPI_RX` (`include_str!`) and `dmamux1_spi_rx()`;
  - `spi_rx_request(peripheral)`, which is `None` for SPI6 (DMAMUX2, which Renode does not declare) and refuses any other SPI with no row (`INV-06`);
  - each SPI controller the twin adds gets `    DMARecieve -> dmamux1@<n>` after its `IRQ`, and its comment names the table; SPI6's comment names the absence instead.
- **`lib.rs`:** the error `MalformedDmaTable`, for a row that is not three tab-separated fields with a decimal input.
- **The emitter reads only the topology's solved SPI controllers and the cited table** (`RUL-16`).

## Moved goldens (final, measured)

| File | Change |
|---|---|
| `boards/sentinel-01/sentinel-01.repl` | `spi1`'s comment line reworded, and `    DMARecieve -> dmamux1@37` added (2 lines; regenerated with `AETHELGARD_BLESS=1 cargo test -p aethelgard-svd-hal --test board`) |
| `boards/sentinel-01-bench/sentinel-01-bench.repl` | the same 2 lines (3402 told) |
| `crates/aethelgard-twin/src/tests.rs` | `spi1`'s and `spi2`'s exact blocks gain their lines; three tests added (below) |
| `crates/aethelgard-svd-hal/tests/order_2203.rs` | the two platform digests. Before `1313` merged: `a81a65b3…` → `ea5846df…` and `292791aa…` → `72948365…`. After merging `1313` (specification 15, `f845b6fd`), whose own pins are `244791ab…` and `6cad35bc…`, they are `b2bc0dcf…` and `bfa37e13…`. **Proved both times:** with this work order's two edits undone, the new texts hash to `main`'s pins exactly (a throwaway test, measured and then deleted). `main.rs` and the boot image did not move |
| `crates/aethelgard-cli/tests/workspace_shape.rs` | the twin's allowed files name `data/dmamux1-spi-rx.tsv` (`TWIN_DATA`) |
| `fab/sentinel-01-bench/package/MANIFEST.json` | not in this PR; its `verdict_sha256` moves at the next package write (above) |

- §1.1's list missed `order_2203.rs` and `workspace_shape.rs` (F1).
- `generate_tier1.rs` and the `attribute_seeded` guard did not need editing: they compare against the committed board, which was regenerated.

## Acceptance (measured)

- **`4400c`'s `r3` on the emitted platform, with no overlay,** using `4400c`'s driver unchanged (`spike/4400c/drive.py`, sha256 `13a4f58b…`, on `main` through #202) and the CLI built from this branch: `SPI Flash 0x00EF4017 found pages=32768 erase=64k`. The emitted `sentinel-02-own.repl` carries `spi1 … @37` (L20) and `spi2 … @39` (L28).
- **Generated-firmware verdicts are unchanged.** `simulate --for 100ms` on `boards/sentinel-01` and `boards/sentinel-01-bench`, `main`'s CLI and platform against this branch's: `pass` on both sides, for both boards. The verdicts differ only at `/input/sha256` (the platform), `/input/path` and `/boot/firmware/path`. The board's own Rust firmware on `sentinel-02-own`: `pass`, `device_unmodelled` only. The campaigns' scenarios are covered by CI's tier-2 suites, whose expectations this PR does not touch.
- **Gates** (the tools image, target inside the repository as CI has it): `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace`: 128 suites, 2,109 passed, 0 failed, measured after merging `main` at `f845b6fd` (#197, `1313`). Before that merge: CI green on `3146ea18` (run 44073801009: `gates`, `shards` and `tools` with all 8 shards; tier-2 `tools_total_passed=2298` against `TIER2_MIN` 177; `ardupilot-image` 44073800942 and `engine-image` 44073801164 green).
- **`TIER2_MIN` is `main`'s 178** (`1313`): this work order adds no tier-2 test; the three new tests are tier 1.

## Adversarial pass, targeted (§5): one mutant per guard, against `aethelgard-twin`'s tests

| Mutant | Attacks | Killed by |
|---|---|---|
| M1: the table's 37 and 39 swapped | a line on the wrong DMAMUX number, a read on SPI2 instead of SPI1 | the table test (its sha256 pin), the per-controller test, and both exact-block tests |
| M2: the table's citation lines deleted | a number with no citation | the table test (sha256 pin) |
| M3: SPI6 given a guessed number (11) | a fabricated line where Renode has no DMAMUX2 | the per-controller test |
| M4: the emitted line dropped | the fix silently undone | the per-controller test and both exact-block tests |
| M5: the parser's digit check removed (`+37` accepted) | a malformed row read as a number | `a_malformed_dma_row_is_refused` |

- **A verdict that moves:** measured above (none).
- **Residual, named:**
  - SPI1, 2, 3 and 5's numbers have no CI witness independent of RM0433. The tests' literals and the table are read from the same source, so a transcription error copied into both would pass.
  - What holds them: the sha256 pin (any edit must be read again from the table), §1.1's runtime boot (wrong numbers fail visibly, `p5-wrong`) and ChibiOS's `stm32_dmamux.h` (37, 39 agree, read in the pinned ArduPilot image).
  - SPI4's number is checked in CI against Renode's own file.

## Reduction pass

- `spike/4400c2/` is deleted (roadmap §6); its evidence stays at `5a1bccaf`.
- One request lookup per added SPI controller (computed once, not twice).
- Nothing else is removable: each of the three functions has a caller or a test that needs it.

## Findings (build)

- **F1 · §1.1's moved-golden list missed two guards · process · low.** `order_2203.rs` pins sha256 digests of the emitted platform, and `workspace_shape.rs` fixes the twin's files. Both were found by the gates, not by the grep for `.repl` text. Lesson: search for pinned digests of emitted output, not only for its text.
- **F2 · `fab_tier1` refuses `BaseOutsideRoot` when `CARGO_TARGET_DIR` is outside the repository · environment · info.** Its scratch base must sit under the current directory. CI keeps `target/` in the repository; the gates here were rerun that way.

## Question for the architect

- **Q1.** The emitted `.repl` header pins each Renode file and each model by sha256, but not RM0433, whose numbers it now carries (the table file pins the PDF's sha256 in its own comment). Should the header name the table file and its sha256? It would move every golden again, so it was not done here.

## Evidence

Kept under `~/.cache/aet-4400c2/evidence/` on Furnace:
- `mutants.txt`, the five mutants rerun to file;
- `accept-r3/`, the acceptance run's steps, USART1 capture and console text;
- `verdicts/`, the four `simulate` verdicts, main against branch;
- `gates.log` and `caps.log`.

**The package gate** (`ci.yml`'s `cargo package --locked --no-verify` over every publishable crate, run here): `aethelgard-twin-0.2.0.crate` holds `data/dmamux1-spi-rx.tsv` and `tests/renode-1.17.0/platforms/cpus/stm32h7.repl`, so both `include_str!`s resolve from the package (the `AET-WO-1207` lesson).

**Tier 2** compiles to no test locally (the `with-tools` feature), so CI is its first run of this change. No tier-2 source pins the old platform's sha256 or digests (searched for each).

**CI on `7931bce8`** (`main` `f845b6fd` merged):
- `ci` run 44080759063 is **green**: `gates`, `shards`, and `tools` with all 8 shards; tier-2 `tools_total_passed=2309`, against `TIER2_MIN` 178.
- `engine-image` 44080759128 is green.
- **`ardupilot-image` 44080759085 was cancelled in one job:** `generated (serial-dshot-9600)` hung in its own "build the image" step after step 12/12 (`WORKDIR /opt/ardupilot`, 12:14:57) until its 20-minute limit (12:32:20, "The operation was canceled"). Every other job of that run passed: `generated (sentinel-01)` and `(sentinel-02)`, the four `ardupilot` seeds, `image`, `pinned`, `cli` and `source`.
- **Not this change** (inferred from the log): the step builds the ArduPilot image and touches nothing this work order changes. The same job passed on `3146ea18` (run 44073800942).
- The read-only token cannot re-run a job. This READY commit's own push starts a fresh `ardupilot-image` run.

**READY**, 2026-11-17: built to the ruling, with no line held back unruled. Title and body: `~/.cache/aet-4400c2/pr-title-body.md`.

# After 3402 merged: AWAITING-PACKAGE

The architect's note (14:34 UTC, `~/.cache/aet-bus/4400c2.md`): 3402 merged (#203, `main` `7d0763eb`); merge it, re-gate, push AWAITING-PACKAGE, and commit the dispatched package unchanged before READY.

- **Merged** `main` `7d0763eb` as a merge commit, with no conflicts. 3402 moved the bench board's Rust and its package, not its `.repl`, and the board goldens bind unchanged after the merge (`board` test, no re-bless).
- **Gates** (tools image, target inside the repository): `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace --no-fail-fast`: 130 suites, 2,141 passed, 0 failed.
- **The package:** `fab/sentinel-01-bench/package/` is still `main`'s (3402's run 44091740015). With this branch's `.repl` its `verdict_sha256` differs, so it waits on the `bench-package` `write` dispatch on this branch. The READY commit carries that artifact unchanged.

