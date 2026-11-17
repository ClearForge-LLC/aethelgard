# AET-WO-4400e — FEEDBACK (build with one measured stop, §1.1)

Branch `wo/aet-4400e-foreign-judge`, from `main` at `0829fe83` (`4400d` merged, #207). The work order is on `main` at `docs/work-orders/AET-WO-4400e.md`. Prototypes live under `spike/4400e/`; until the ruling, no crate, model, workflow or `container/` file changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

## §1.1 Predictions — written before any run

### What the predictions rest on (read at source, nothing run)

- **The board.** It is the `4400c` board plus two changes on `main`:
  - `4400c2` (#204): the emitted `spi1` and `spi2` carry `DMARecieve -> dmamux1@37` and `@39`;
  - `4400d` (#207): the generated `hwdef.dat` holds `IMU Invensensev3 SPI:icm42688 ROTATION_ROLL_180_YAW_90`, so the CI ELF compiles the Invensensev3 driver (`4400d`'s inbox note, measured there).
- **Our ICM-42688-P model** (`models/renode/ICM42688P.cs` L7–8, `AET-WO-4220`): "Not modelled: the FIFO (FIFO_COUNT reads 0, FIFO_DATA reads its reset 0xFF)". It logs one `Noisy` line per SPI transaction: `bank {b} read 0x{reg} -> {bytes}` or `bank {b} write 0x{reg} <- {bytes}` (L120–131).
- **ArduPilot's Invensensev3 driver** (Copter-4.7.2 `fb773567`, read in `container/ARDUPILOT-IMAGE`):
  - **Probe:** `hardware_init` → `check_whoami`, which reads `WHOAMI` `0x75` (L1003, L1093).
  - **Configure:** ArduPilot's own writes, not the `4210` script's. `PWR_MGMT0` (`0x4E`) gets `0x00`, then `0x0F`; `GYRO_CONFIG0` and `ACCEL_CONFIG0` (`0x4F`, `0x50`) get its ODR value; bank 1 and bank 2 get the anti-alias filters; `INTF_CONFIG0` (`0x4C`) gets `0xC0` in `fifo_reset` (L236–256, L844–901).
  - **Data: only through the FIFO.** `read_fifo` reads `FIFO_COUNTH` (`0x2E`, 2 bytes) and, when the count is zero, never reads `FIFO_DATA` (L583–590). It never reads the data registers `0x1D`… that the `4210` script's data step names.
- **ArduPilot's INS start** (`AP_InertialSensor.cpp`):
  - `init` calls `init_gyro` when the gyro count is non-zero and gyro calibration is not `NEVER` (L966–972; the default calibrates at start-up);
  - `_init_gyro` prints `Init Gyro` and waits on samples through `wait_for_sample`. Its `while (true)` loop leaves only when a gyro and an accelerometer sample have arrived (L2058–2120): **no timeout**.
- **Barometers:** both drivers accumulate on their own timer callbacks, independent of the main thread (`AP_Baro_BMP280.cpp`, `AP_Baro_BMP581.cpp`).
- **The judge** (`crates/aethelgard-cli/README.md`, *Booting firmware*; `elf.rs` L149–163): a `PT_LOAD` only in flash or DTCM RAM, at both addresses.
- **ST's header** (`crates/aethelgard-svd-hal/data/st-cmsis-device-h7/stm32h753xx.h` L2130–2148), the regions a foreign firmware's `PT_LOAD` could use:
  - `D1_ITCMRAM_BASE` `0x00000000` (64 KB);
  - `D1_DTCMRAM_BASE` `0x20000000` (128 KB);
  - `D1_AXISRAM_BASE` `0x24000000` (up to 512 KB);
  - `D2_AHBSRAM_BASE` `0x30000000` (up to 288 KB);
  - `D3_SRAM_BASE` `0x38000000` (64 KB);
  - `D3_BKPSRAM_BASE` `0x38800000` (4 KB);
  - `FLASH_BANK1_BASE` to `FLASH_END`.

  Each size is stated only in that line's comment.

### Predicted

| id | What | Prediction |
|---|---|---|
| P1 | `simulate --firmware` on the CI ELF, unmodified | refused before any boot as `FirmwareOutsideMemory`, `.data` at a `0x3000xxxx` virtual address (inferred from `4400c`) |
| P2 | Start-up and console | as `4400c`: no stall, the banner's first byte at about 9.4 ms |
| P3 | The dataflash, now with the emitted SPI receive lines | `SPI Flash 0x00EF4017 found` with no overlay (`4400c2`'s acceptance). The first model line of the run is `u4`'s, at about 0.164 s (status `0x05`), and its JEDEC identity `0x9F` at about 0.180 s |
| P4 | The barometers | identity reads: BME280 at about 0.344 s, BMP581 about 0.2 ms later. The BMP581's first data read about 40 ms after its identity (`4400c` `r3`) |
| P5 | **The ICM-42688-P's identity** | probed at `ins.init`: `[NOISY] spi2.spi2mux.u5: bank 0 read 0x75 -> 0x47`, at 2.5–4.5 s virtual (inferred: `4400c`'s 5.2 s, less the 1.6 s that the dataflash failure cost there) |
| P6 | The ICM's configure step against the `4210` script (`0x4C ← 0xC0`, `0x50 ← 0x66`, `0x4E ← 0x03`) | **violated:** ArduPilot writes `0x4E ← 0x00` first, then `0x0F` (never `0x03`), and `0x50` with its own ODR and full scale. Some of its bank 1 and bank 2 writes reach registers the model does not hold, so the model logs `access to unmodelled register` warnings |
| P7 | **The ICM's first data read** | **never.** ArduPilot reads `bank 0 read 0x2E -> 0x00 0x00` (`FIFO_COUNT`) at its backend rate and never reads `FIFO_DATA` or `0x1D`. No sample arrives, so the main thread never leaves `init_gyro` (`Init Gyro` printed, then nothing from the main thread). **That is the work order's §7 stop: the IMU's read witness needs a model change, our own `ICM42688P.cs` FIFO** |
| P8 | The barometers after P7 | read on, from their timer callbacks, while the main thread waits |
| P9 | SERIAL0 receive | **not needed** by any witness: every probe and read happens with nothing sent to the board |
| P10 | The window | the ICM identity plus a margin: about 4–5 s virtual. At 1.1–2.5 s of wall per simulated second, 5–12 s of wall, inside 30 s. Peak RSS 450–650 MiB, inside 1 GiB |
| P11 | The rules of today's judge | as `4400c` §1.5 (the boot word, instructions, self-loop, cold RCC, `SysTick`, `NVIC` and USART1 fail; pins pass), measured again on this ELF |
| P12 | The model divergences | ArduPilot's clock still runs 1.25× virtual time (TIM5 at a fixed 250 MHz); USART1 `BRR` is still `0x271` |
| P13 | The generated `sentinel-02-own` firmware under today's judge (the CLI script's capture) | `pass`, with `device_unmodelled` only |
