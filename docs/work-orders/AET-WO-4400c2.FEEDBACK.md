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
