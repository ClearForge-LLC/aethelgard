# AET-WO-2302 FEEDBACK — the twin's windows

Branch `wo/aet-2302-windows`, cut from `main` at `1178ca8` (specification 12, trace 12, `TIER2_MIN` 160). Spike first, then stop for the ruling. Every answer is labelled **measured**, **read at source** or **inferred**.

## §0 Predictions (committed before any measured run, never edited)

Read at source before predicting:
- The generated bring-up (`svd-hal/src/board/firmware.rs`) runs: reset clock check, `OPTSR` read, clock enables, GPIO, I2C/SPI init, each serial port configured, **each port's pattern sent**, every identity and configure step, **one settle** (the largest `settle_us`, spun at the fastest HSI: 64.96 spins per µs), every data step, **`start_outputs`** per timer, **the boot word**, then **each port's echo**, then the self-loop.
- `settle_us` in `parts.json`: ICM20948 40 000, ICM42688P 20 000, BMP581 10 000, ICP10111 340; BME280, TMP117 and W25Q64JV none.
- Renode runs the settle spin about 1.92 times silicon's lower bound (`4220` F2); `4230` measured `imu-baro-1v8`'s boot word at 78.1 ms.
- The serial frame is injected at 100 µs, paced one quantum-rounded character apart; the wire budget (`wire_ns`) is silicon time: pattern (label + 5 + 1 chars) plus 8 paced frame bytes. Renode's USART transmits in zero virtual time.
- `DSHOT_WINDOW_NS` was set by log volume: four DShot600 channels for 100 ms are 139.6 MB of log and 10.8 s of timer events (`4300a` P5.2); `LOG_FILTER` is 8 MiB.

Predictions:

| # | Prediction |
|---|---|
| P1 | Reference (BME280, TMP117, W25Q64JV, CONSOLE 115200): the boot word is written between 0.2 and 1.0 ms of virtual time; every device step falls before 1 ms; the pattern is out before the first device step; the echo ends within 0.1 ms after the frame's last byte (800 µs). |
| P2 | `motors-dshot600`: `start_outputs` and the boot word fall between 0.2 and 1.0 ms, within 20 µs of each other. |
| P3 | `serial-dshot-9600`: refused before the boot today (`SerialWireTimePastWindow`; wire budget about 22.3 ms against 2 ms). Its twin phases match P2 plus the console's pattern. |
| P4 | `imu-baro-1v8`: identity and configure steps before 1 ms; the settle takes 76–78 ms; data steps and the boot word at 77–79 ms. |
| P5 | Sentinel-02-shaped (written here: `sentinel-02-own` + four DShot600 + MAVLINK 57600 + CRSF 420000): settle ≈ 38–39 ms; `start_outputs` and the boot word at 38–41 ms; wire budget ≈ 3.9 ms (MAVLINK) + 1.0 ms (CRSF) from the 100 µs injection, so the serial evidence ends by about 5 ms. |
| P6 | DShot's wall cost is the timer events, not the log: with the observers' logging off, four channels still cost ≈ 0.1 s of wall per ms of virtual time. Logging on adds ≈ 1.4 MB of log per ms. |
| P7 | (a) anchored windows on Sentinel-02-shaped: the boot ends at `start_outputs` + 2 ms ≈ 41 ms; wall ≈ the `imu-baro` boot's share for 41 ms plus 2.6 s of DShot; log ≈ 3 MB; the witnesses hold. Trace ≈ today's records plus one window record per witness: 1.5–3 KB per run. |
| P8 | (b) one 100 ms window with a DShot sub-window: DShot runs ≈ 60 ms after its sub-window; unless the observers are silenced at the sub-window's end, the log passes 8 MiB and the witness fails closed; silenced, the wall is ≈ 6–7 s more than (a). Trace bytes as (a). |
| P9 | (c) two boots: wall ≈ (b)'s device boot plus (a)'s output boot (the firmware is one ELF, so DShot runs in the device boot too); trace ≈ 2× one boot's records. |
| P10 | Recommendation expected: (a), with the DShot anchor bounded by the judge (after the last data step's witness, before the boot word, and early enough that anchor + window ≤ 100 ms). |
| P11 | Q6 under (a): a mixed board needs the output window to hold two of its slowest PWM periods (40 ms at 50 Hz); with DShot observers silenced after 2 ms, the cost is ≈ 4 s of wall per boot from DShot's timer events. |
| P12 | Q8: no override kind reaches a serial port; the seeded `serial_rx_dead` is a firmware mutant, which `verify`'s campaign (the generated ELF) cannot carry. `Target::Serial` needs an override kind the composition renders on the board side (the RX line open: no frame injected), detected by `serial_echo_missing`. |
| P13 | 3302 synchronisation: no verdict depends on the controller's SCL period; the twin's I2C transfer is instantaneous in virtual time, and the windows are ms-scale. Recorded as a model divergence. |
