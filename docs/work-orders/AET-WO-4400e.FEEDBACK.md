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

## STOPPED at §1.1 — the stop, reported first

**The judge cannot reach "the ICM-42688-P read" on this twin.** Three blockers stand in the way, each labelled. The first meets the work order's §7 condition "the held USART line and its hard fault come first". The first two also meet "a witness needs a model change".

**B1 — measured: ArduPilot hard-faults 0.672 ms after the IMU probe, deterministically.**
- **When:** `HardFault_Handler` is entered at **3.570806620 s** virtual, every time. That holds in seven hooked runs (`r1`–`r7`) and in `j2`, which has no hook at all (`ICSR` `0x803` and `PC` in `CrashCatcher_getc` at 3.6 s).
- **Fault status:** `CFSR` `0x00020000` (UsageFault `INVSTATE`), `HFSR` `0x40000000` (`FORCED`), `MMFAR` and `BFAR` 0 (`r2`).
- **The frame:** `EXC_RETURN` `0xFFFFFFED`, so a thread frame on PSP. PSP is `0x30015E50`, and the frame reads `R0 30015E78`, `R1 300160E8`, `R2 30015EF8`, `LR 08000EDF` (`r3`).
- **What that is (read at source):**
  - The frame stands in ChibiOS's context switch, `__port_switch`, whose `ldmia sp!, {r4–r11, pc}` at `0x08000EC4` pops the next thread's saved PC.
  - **`R0`, the thread switched to, is `ch0`** (`0x30015E78`, ChibiOS's OS instance: the ready list's header), not a thread.
  - **`R1`, the thread switched from, is the idle thread** (inside `ch_c0_idle_thread_wa`).
  - `LR` is in `__port_switch_from_isr`.

  So a preemption switched from the idle thread to an empty ready list's header.
- **The sequence (`r6`, `r7`, every vector hooked):**
  - Each of the IMU's SPI transactions ends in DMA1 stream 2's interrupt (SPI2 receive, exception 29), then a preemption.
  - After the last configure write (`PWR_MGMT0 ← 0x0F` at 3.570800570 s), ArduPilot's main thread sleeps (`delay_microseconds(300)`, `AP_InertialSensor_Invensensev3.cpp` L901–902).
  - The idle thread's `svc 0` exits a preemption (exception 11 at 3.570805990 s). A second `chSchDoPreemption` follows at 3.570806100 s **with no interrupt before it**, and the fault comes 0.5 µs later.
  - The PC `0x08000EC6` in that frame falls inside the 4-byte `ldmia`, so it is Renode's rendering of the fault, not an instruction boundary. Nothing here is reasoned from it.
- **Narrowed, not ruled out:** ChibiOS's interrupt epilogue builds the thread-mode frame only when `ICSR.RETTOBASE` is 1 (`chcore.c` L315). At 35 of 35 entries of DMA1 stream 2's interrupt, Renode's state was consistent:
  - `EXC_RETURN` `0xFFFFFFED` (to thread mode);
  - `ICSR` `0x81D` (`VECTACTIVE` 29, `RETTOBASE` 1);
  - `SHCSR.SVCALLACT` 0, so not nested in the SVC handler.

  The epilogue reads `RETTOBASE` at the ISR's exit, which was not sampled, and TIM5's interrupts were not sampled at all.
- **Cause: not established.** It is either the twin's Cortex-M emulation (tlib's exception and FPU-frame handling, a Renode change) or ArduPilot/ChibiOS. Inferred, not measured: the same ArduPilot runs this driver on silicon at every boot of a board with an ICM-42688-P.
- **Its relation to `4400c2` D3** (the held USART line's fault): **inferred to be the same fault.**
  - It has the same `CFSR` and `HFSR`, also in `HardFault_Handler` after a run of DMA interrupts.
  - `4400c2` recorded no thread frame, so the identity is not shown.
  - **If it is the same fault, the USART receive line is not what manufactures it:** here no USART receive line is emitted (`4400c2`'s ruling held it).
  - One run settles it: `4400c2`'s repro (`spike/4400c2/sh/crash.sh` at `5a1bccaf`, reachable from PR #204's head `1061fd4a`, not from `main`, which squash-merged it) with this branch's PSP-frame hook (`spike/4400e/stubs-fault2.txt`). It is left for the ruling.
- **On the console:** `CRASH ENCOUNTERED` / `Enable logging and then press any key to start dump.` ArduPilot's CrashCatcher then polls USART1 for a key: the dump would need SERIAL0 receive. The judge does not.

**B2 — read at source; the boot never reached it: our ICM-42688-P model has no FIFO, and ArduPilot reads its IMU only through the FIFO.**
- `models/renode/ICM42688P.cs` L7–8: "Not modelled: the FIFO (FIFO_COUNT reads 0, FIFO_DATA reads its reset 0xFF)".
- ArduPilot's `read_fifo` reads `FIFO_COUNTH` (`0x2E`) and stops on a zero count (`AP_InertialSensor_Invensensev3.cpp` L583–590). It never reads the data registers (`0x1D`…) that the part table's data step names.
- `AP_InertialSensor::wait_for_sample` leaves its loop only when a sample has arrived (L2058–2120). So past B1 the main thread would wait in `init_gyro` for good.
- **P7 is unmeasured, not held:** B1 comes first, and no run reached `read_fifo` (0 `0x2E` reads; its hook never fired in `r3`).
- **The model change it needs:** a FIFO in `ICM42688P.cs` (`FIFO_COUNTH`/`L` and `FIFO_DATA` packets at the configured ODR). That is this project's own model (`AET-WO-4220`), outside this order's fence. Whether it counts as the work order's "Renode model change" is the architect's to say.
- **Not proposed** (for the ruling, not chosen): taking a `FIFO_COUNT` poll as the "read" witness. It is a weakening, since it carries no data.

**B3 — measured, a rule question rather than a blocker:**
- Renode's `STM32H7_SPI` logs `[ERROR] spi<n>: Attempted to set peripheral into SPI slave mode. Only master mode is supported` once per ArduPilot SPI transaction but one: SPI1 16 for 17 chip-select windows, SPI2 18 for 19 (`j1b`) and 34 for 35 (`j2`).
- Under today's rules each is `renode_error_log`, and inside a step `spi_step_error`.
- Which register write triggers it was not read: the pinned model's source is not on this host, and the run logs no SPI access.

**SERIAL0 receive:** **not needed** by any witness, measured. Every probe and read below happens with nothing sent to the board. Only CrashCatcher's dump waits for a key.

**What was not done:**
- Nothing was worked past B1.
- `r2`–`r7` are observation-only diagnostics made after the stop condition was met: execution hooks, plus USART1's access logging off. They change no state, and each stops at 3.6 s.
- No crate, model, workflow or `container/` file changed: `git diff --stat 0829fe83 -- crates models .github container` is empty.

## §1.1 Results — measured after the predictions (`62b2b444`)

**The ELF (not in git):** `arducopter.elf`, sha256 `a799dfb6633537e74a8c3b7d8b6a464f185c255e377a612b7f9fcf8378dab1ef`, 2,502,044 B.
- **Source:** CI's own, the artifact `generated-sentinel-02` of `ardupilot-image` run 44148995663 on `main` at `0829fe83`, the base of this branch.
- **Its `hwdef.dat`** (sha256 `8da4cb01…`) holds `IMU Invensensev3 SPI:icm42688 ROTATION_ROLL_180_YAW_90`. The ELF holds 31 `Invensensev3` symbols.
- **Its `PT_LOAD`s** (`readelf -lW`, read at source):
  - code in flash at `0x08000000` (`0x13CCA8` B);
  - `.data` at virtual `0x30002200`, physical `0x0813CCA8` (4,132 B);
  - `.crash_log`, `NOBITS` in flash at `0x0813DCCC` (`0x82334` B);
  - `.bss` at `0x30003228`, `.heap` at `0x30016284` to `0x30040000`;
  - `.mstack` and `.pstack` at `0x30000000`.

**How it ran (measured):**
- Every container went through `spike/4400e/sh/img.sh`, under `~/bin/capped-run --mem 8G`, one at a time. The caps were read back each time (`memory.max=8589934592 memory.swap.max=0 pids.max=8192`, `spike/4400e/out/caps.log`).
- **The CLI's script, not a reconstruction.** The unmodified CLI generated `sentinel-02-own`, built its Rust firmware and booted it: `simulate --for 100ms` gives `pass`, with `device_unmodelled` only (P13). Its Renode argv came from `/proc`, as `4400c` did (`spike/4400e/sh/gen.sh`, `out/renode-argv.txt`). The platform the CLI booted is byte-identical to the emitted `.repl`.
- `spike/4400e/drive.py` (from `4400c`'s) replays that script under the CLI's `renode.config`, with **only the ELF swapped**. Left out are the boot-word hook and its serial injection, and the judge's reads, which follow the run instead.
- **The judge's shape:** one `RunFor`, no hook, the CLI's own log levels and access logging. These are runs `j1` (3.570 s), `j1b` (3.5705 s) and `j2` (3.6 s).
- P1: unmodified, `simulate --firmware` refuses this ELF before any boot: `FirmwareOutsideMemory`, "a PT_LOAD of 4132 bytes at virtual address 0x30002200" (exit 3). **Held.**

### The witnesses, each at its virtual time (`j1b`, no hook; the models' own lines, matched as the judge matches them)

| Witness | Line (after the stamp) | Virtual time |
|---|---|---|
| The run's first model line | `[NOISY] spi1.spi1mux.u4: Transmitting data 0x5, current state: RecognizeOperation` (a status read) | 0.161918280 s |
| W25Q64JV (`u4`) identity, JEDEC `0x9F` | `…u4: Handled command: Operation: ReadID, returning 0xEF`, then `0x40`, `0x17` | 0.161938610 s |
| BME280 (`u2`) identity | `[NOISY] i2c1.u2: Write D0`, `[NOISY] i2c1.u2: Read 60` | 0.342585230 s |
| BMP581 (`u7`) identity | `[NOISY] i2c1.u7: read 0x01 -> 0x50` | 0.342780210 s |
| BMP581 first data read | `[NOISY] i2c1.u7: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62` (the stimulus, 25 °C and 101,325 Pa) | 0.382745850 s |
| BME280 first data read | `[NOISY] i2c1.u2: Write F7`, `[NOISY] i2c1.u2: Read 0 0 90 80 0 D0` | 0.382787270 s |
| **ICM-42688-P (`u5`) identity** | `[NOISY] spi2.spi2mux.u5: bank 0 read 0x75 -> 0x47` | **3.570134680 s** |
| ICM-42688-P first data read | **none** (B1, then B2) | — |

- The console says `SPI Flash 0x00EF4017 found pages=32768 erase=64k` (`4400c2`'s lines, no overlay), then `Dataflash: erase done (1795 ms)` and `No Compass backends available`. It prints nothing about the INS before B1.
- The barometers read on every 8–16 ms from their timer callbacks: 400 `0x1D` reads of the BMP581 and 200 `F7` reads of the BME280 by 3.5705 s.
- The configuration collapses repeated log entries, but no run's log holds a repeat marker (measured), so every count is whole.

### Each device against its `4210` script (`crates/aethelgard-ir/data/parts.json`)

| Part | Step | The script (the judge's expected line) | ArduPilot's traffic | |
|---|---|---|---|---|
| W25Q64JV | identity (`0x9F` → `EF4017`; `script: null`) | the flash model's `ReadID` lines | served, and the firmware received it | **satisfied**. The identity is in the device's **second** chip-select window, after a status read (`0x05`), so today's rule "the k-th window holds the k-th step" puts a status read where the identity is expected |
| BME280 | identity | `Write D0`, `Read 60` | exactly this, once | **satisfied** |
| BME280 | (no data step: `script: null`) | — | `Write 88` and 24 calibration bytes; `Write F4 57`; `Write F5 8`; then `Write F7` and 6 bytes, periodically | **no data step exists to judge** |
| BMP581 | identity | `read 0x01 -> 0x50` | exactly this | **satisfied** |
| BMP581 | configure (`0x36 ← 0x40 0x81`, a burst) | `write 0x36 <- 0x40 0x81` | `read 0x28`, `read 0x27`, then `write 0x37 <- 0x00`, `write 0x36 <- 0x72`, `write 0x37 <- 0x3D`: three single-byte writes | **violated** (`i2c_configure_mismatch`), as in `4400c` |
| BMP581 | data (`0x1D`, 6 bytes) | `read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62` | exactly this, from 0.3827 s | **satisfied** |
| ICM-42688-P | identity (`0xF5`, i.e. `0x75` read → `0x47`) | `bank 0 read 0x75 -> 0x47` | exactly this, at 3.5701 s | **satisfied** |
| ICM-42688-P | configure (`0x4C ← 0xC0`, `0x50 ← 0x66`, `0x4E ← 0x03`, each its own write) | those three lines | `0x16 ← 0x80`, `0x5F ← 0x07`, **`0x4C ← 0xC0`**, `0x4B ← 0x02`, `0x4E ← 0x00`, `0x4F ← 0x05`, `0x50 ← 0x05`, then the bank 1 and bank 2 filter writes through `0x76`, `0x4D` (read `0x91`, write `0x51`), and `0x4E ← 0x0F` | **violated:** `0x50` gets `0x05` (±16 g, 8 kHz) and `0x4E` gets `0x0F` (gyro and accelerometer low-noise), never `0x66` and `0x03`. **No model warning:** every register ArduPilot touches is modelled (P6's warning half missed) |
| ICM-42688-P | data (`0x9D`, i.e. `0x1D` read, 6 bytes) | `bank 0 read 0x1D -> …` | never (B1; at source, ArduPilot never reads `0x1D` at all) | **not reached** |

### The window: three bounds (`j1`, `j1b`, `j2`, one `RunFor`, no hook, Furnace)

| Run | Virtual | Wall in `RunFor` | Wall, Renode's start to quit | Peak RSS (tree, every 100 ms) | Log bytes | Bytes the CLI's filter counts (of 8 MiB) |
|---|---|---|---|---|---|---|
| `j1` | 3.570 s | 4.94 s | 8.63 s | 500.3 MiB (49 % of 1 GiB) | 4.50 MB | — |
| `j1b` | 3.5705 s (after the ICM identity, 0.3 ms before B1) | 5.02 s | 8.74 s | 500.5 MiB | 4.61 MB | 4.06 MB (48 %) |
| `j2` | 3.6 s (B1 inside) | 6.81 s | 10.44 s | 522.4 MiB | 117.5 MB | **4.13 MB**: CrashCatcher's `ISR` polling is an access line the judge never reads, discarded uncounted (`renode.rs` `LOG_FILTER`, `twin/src/serial.rs` `is_unjudged_access`) |

- **`j1b` is clean only because it ends 0.3 ms before B1.** No window is proposed that ends between the probe and the fault.
- **Today's judge would name B1 itself:** `core_in_exception` (`ICSR` `0x803` at `j2`'s end). It is not a log flood, since the flood is discarded.
- **Cost past the IMU's start is not measured**, because no run reached it.
  - Measured: SPI2 writes 210 log lines for 19 transactions, about 11 per transaction, at the CLI's levels.
  - Projected (inferred): ArduPilot's backend polls at its sample rate, about three transactions per millisecond (count, data, register check). That is about 3.6 MB per simulated second of IMU, so the 8 MiB budget, 4.06 MB of which is already used at 3.57 s, binds about 1.2 s after the IMU starts. Wall time binds later.
  - A window has to end within that. Any rule that "reads continue to the window's end" is bounded by it.
- The wall figures are Furnace's. The README records a boot Furnace finished that a CI shard killed at 30 s. A CI shard's time is unmeasured.

### Today's judge, rule by rule, on this ELF (from its finding kinds in source; observables read after `j1b`, `out/runs/j1b/judge-reads.json`)

| Rule (finding kind) | This boot | Proposal for a foreign judge (the weakening named, `INV-07`) |
|---|---|---|
| `PT_LOAD` only in flash or DTCM, both addresses (`FirmwareOutsideMemory`) | **refused before boot**, `.data` at `0x30002200` | **replace:** the regions below (memory rule). It guards loading, so no witness weakens |
| boot word (`boot_word_missing`) | `0x0`; ArduPilot writes none | **replace** by "booted" (below) |
| 100 MIPS × time (`cpu_halted`) | 73,148,106 of 357,050,000 (20.5 %); the core idles in `__idle_thread` | **drop. Weakens:** no witness that the core kept executing. Partly **replaced** by a liveness witness: each periodic device's data read inside the window's last *k* ms (the barometers' 8–16 ms period) |
| `ICSR.VECTACTIVE` 0 (`core_in_exception`) | `0x800` at 3.5705 s; **`0x803` at 3.6 s, which names B1** | **keep**, at the window's end |
| final `PC` in an executable `PT_LOAD` | `0x080F88FC` (`.text`) | **keep** |
| `PC` on a `yield; b.n` self-loop (`pc_not_on_self_loop`) | not a self-loop (RTOS) | **drop. Weakens** the "parked" witness |
| solved pins' `MODER`, `AFRL`, `AFRH` (`register_mismatch`) | **6 of 6 equal** | **keep** |
| USART1 `CR1`, `CR2`, `CR3`, `BRR` (`register_mismatch`) | `CR2` passes. `CR1` `0x10D` against `0x0D` (`PEIE`); `CR3` `0xC0` (DMA enables); `BRR` `0x271` against `0x22C` | **replace:** `CR1`'s frame bits (`M0`, `M1`, `PCE`, `PS`, `UE`, `TE`) and `CR2.STOP` against the solved framing. Drop `CR3` and `BRR`. **Weakens:** the baud is unjudged; `BRR` depends on ArduPilot's clock tree, and Renode's `usart1` is fixed at 200 MHz (the owned divergence row) |
| each cold RCC register, exactly (`register_mismatch`) | exact: 4 of 9 fail. **"The solved bits set": 9 of 9 pass.** Extra bits: DMA1/2, RNG, `GPIOC`–`K`, TIM5, `SYSCFG` | **replace** by "at least the solved bits". **Weakens:** an unsolved peripheral clocked is not caught |
| `SysTick.CSR` (mask `0x10003`) | `0x5` | **drop** (ArduPilot ticks on TIM5). **Weakens** "no interrupt armed" |
| `NVIC.ISER0`–`4` all 0 | `0x8080FFC0`, `0x02040139`, 0, `0x04000000`, 0 | **drop. Weakens** "no interrupt armed" |
| `--for` ≤ 100 ms | nothing ArduPilot does is witnessed by 100 ms (first model line at 0.16 s) | **replace** by the foreign window (below) |
| device identity and steps (`i2c_identity_mismatch`, `i2c_transaction_unwitnessed`, `i2c_configure_mismatch`, `i2c_data_mismatch`, `spi_identity_mismatch`, `spi_transaction_unwitnessed`, …) | see the script table | **replace** (per-device witness, below) |
| SPI's k-th chip-select window holds the k-th step | the W25Q's identity is in its second window | **replace:** a step is the first window whose line matches it, in order, never across devices. **Weakens:** extra transactions before a step are not refused |
| a device's `[WARNING]` fails its step | `u4` warns `Transmission finished in the unexpected state: RecognizeOperation` (`4210` F11's known line) | **keep**, with today's one exception |
| `renode_error_log`, `i2c_step_error`, `spi_step_error` | **B3:** one `[ERROR]` per SPI transaction but one | **decision needed** (D4): keep, so ArduPilot fails by name until the model stops logging it; or exempt that exact text, which weakens: the SPI model's own error channel is ignored for that message |
| `renode_log_flood` (8 MiB counted) | 4.06 MB at 3.57 s; the fault's flood is uncounted | **keep**; it bounds the window |
| `wall_bound_exceeded`, `memory_cap_exceeded`, `cpu_aborted` | 8.7–10.4 s and 500–522 MiB on Furnace | **keep** |
| `device_unmodelled` (U3, TMP117) | info, as the generated firmware's | **keep** |
| serial pattern and echo (`serial_pattern_missing`, `serial_echo_missing`, …) | SERIAL0 carries MAVLink 2; an echo needs receive | **drop** for a foreign firmware. **Weakens:** no serial witness. The framing check above stays, and receive stays held |

### The memory rule (read at source)

- A foreign firmware's `PT_LOAD`, at both its physical and its virtual address, must lie inside one of these. Each base is the header's define, and each size the one its comment states, as `elf.rs` takes DTCM's 128 KB today:
  - `FLASH_BANK1_BASE` to `FLASH_END`;
  - `D1_ITCMRAM_BASE` (64 KB);
  - `D1_DTCMRAM_BASE` (128 KB);
  - `D1_AXISRAM_BASE` (512 KB);
  - `D2_AHBSRAM_BASE` (288 KB);
  - `D3_SRAM_BASE` (64 KB);
  - `D3_BKPSRAM_BASE` (4 KB).
- Renode 1.17.0's `stm32h7.repl` maps each of these at the same base with the same size (`itcm`, `dtcm`, `axiSram`, `sram1`–`3` at `0x30000000`/`…20000`/`…40000` totalling 288 KB, `sram4`, `backupSram`, `flashBank1`/`2`).
- ArduPilot's segments all fit: flash, and D2 SRAM `0x30000000`–`0x30040000`.
- `D2_AXISRAM_BASE` (`0x10000000`, the H7's alias) is left out, since Renode maps nothing there.

### Moved bytes, by declaration (read at source)

| Declaration | What moves | |
|---|---|---|
| **(a) a `simulate` flag** (for example `--foreign`) | `crates/aethelgard-cli/tests/mcp-tools.json` (the `simulate` tool's schema gains a property, and `--for`'s description, "at most 100ms on every board", changes if the bound becomes per judge); the README's *Booting firmware* section | **no verdict, no specification, no board, no package** |
| (b) a specification field | specification 16 → 17: every `specification_sha256`, so both boards' generated crates, every fixture, the bench package's `MANIFEST.json`, and every golden that pins a specification digest (the `4400d` pattern) | everything |
| (c) a manifest beside the ELF | nothing committed, but a new input: a reader, a `RUL-13` bound and a new refusal | none, plus a new attack surface |

- Under any of the three, **no generated-firmware verdict moves**: today's judge is not touched.
- Measured on the current CLI: `sentinel-02-own`'s generated firmware gives `pass` with `device_unmodelled` only. Its verdict is the baseline a build compares byte for byte.

### Each prediction beside its result

| id | Predicted | Result | |
|---|---|---|---|
| P1 | `FirmwareOutsideMemory` at `0x3000xxxx` | exactly, `0x30002200`, exit 3 | held |
| P2 | no stall in start-up; the banner at about 9.4 ms | the banner and storage lines as in `4400c` | held |
| P3 | `SPI Flash 0x00EF4017 found`; first model line about 0.164 s, JEDEC about 0.180 s | found; first line 0.1619 s, JEDEC 0.16194 s | held; times earlier |
| P4 | BME280 about 0.344 s, BMP581 about 0.2 ms later, its data about 40 ms after | 0.342585 s, 0.342780 s, 0.382746 s | held |
| P5 | ICM identity `bank 0 read 0x75 -> 0x47` at 2.5–4.5 s | 3.570134680 s | held |
| P6 | configure violated; unmodelled-register warnings | violated; **no warning** | half held |
| P7 | no data read; `0x2E` polls; main thread waits in `init_gyro`; the stop | **not reached:** B1 at 3.5708 s comes first, so no `0x2E` poll and no `init_gyro`. Still a stop, by B1 | **missed** (B2 is still read at source) |
| P8 | the barometers read on | on until B1 | held until B1 |
| P9 | SERIAL0 receive not needed | not needed (only CrashCatcher's dump waits for a key) | held |
| P10 | window 4–5 s; wall 5–12 s; 450–650 MiB | to the ICM identity: 3.5705 s, 8.7 s, 500 MiB. Past it unmeasurable (B1) | held as far as reached |
| P11 | today's rules as in `4400c` | as `4400c`, plus `renode_error_log` (B3) and the SPI window rule | held, with two rules more |
| P12 | 1.25× clock; `BRR` `0x271` | `TIM5` `PSC` 199, `CNT` 4,462,994 µs at 3.5705 s (1.2499×); `BRR` `0x271` | held |
| P13 | the generated firmware `pass`, `device_unmodelled` only | exactly | held |

## The proposal (for the ruling; nothing built)

- **D1 — B1, the hard fault (the architect's).** It blocks the positive case of §2: no witness of "the ICM read" exists before it. Options:
  - **(a)** Chase it as its own order, in Renode's Cortex-M exception and FPU-frame handling, ChibiOS's port, or both. Start from this repro: `sentinel-02-own`, CI's ELF, 3.5708 s, with the hooks in `spike/4400e/stubs-*.txt`. First run the one `4400c2` repro that settles whether D3 is the same fault.
  - **(b)** Build the foreign judge now, with "booted" and "probed" only, and the IMU's read held. The positive case then cannot pass: ArduPilot fails by name (`core_in_exception`) inside any window past 3.5708 s. That is a judge that is correct but has no passing case.
- **D2 — B2, the FIFO (the architect's):** whether our own `ICM42688P.cs` gains a FIFO (outside this fence), or the IMU's read witness for a foreign firmware is something else. A `FIFO_COUNT` poll carries no data, so it is not proposed.
- **D3 — the declaration:** **recommended (a), a `simulate` flag.**
  - It is chosen on the command line, never inferred (`INV-06`). The verdict records `boot.judge: "foreign"`, so a foreign pass can never be read as a generated one.
  - It moves one golden, the MCP snapshot, and no specification or package.
  - Without the flag, ArduPilot is refused as today (`FirmwareOutsideMemory`). With it, the generated firmware is judged by the foreign judge, and its verdict says so.
- **D4 — B3, the SPI `[ERROR]`:** keep `renode_error_log` (ArduPilot fails by name until Renode's model changes), or exempt that one exact message for every judge (a weakening).
- **D5 — "booted":**
  - **recommended:** the first identity transaction a declared device's model logs (here the W25Q's, 0.1619 s), together with `core_in_exception` and the final `PC` in an executable `PT_LOAD` at the window's end;
  - **refused:** the console banner;
  - **not proposed:** `RCC_CFGR.SWS` (a model reaction, but to a register the firmware writes).
- **D6 — the per-device witness. "Identity and data only" is not firmware-neutral here:**
  - the BME280 has no data step in the part table;
  - the ICM's table data step (`0x1D`) is a read ArduPilot never makes, FIFO or not.
  - **(i)** The part table gains data steps for the reads a driver may make (the BME280's `0xF7`; the ICM's FIFO), outside this fence, in `aethelgard-ir`. Lost: the configure step, which witnesses that the driver set the part as the script requires (the BMP581's and the ICM's are both violated today).
  - **(ii)** A script per firmware, with ArduPilot's own sequences as data (the configure step kept for ArduPilot). Lost: firmware-independence, since each foreign firmware needs its own script.
- **D7 — the window:** the ICM's first data read plus the liveness margin. **Not proposed until B1 and B2 resolve:** past the IMU's start the 8 MiB budget, not wall time, binds first (projected about 1.2 s), and the cost there was not reachable.

## The owned roadmap §7.1 rows

- **TIM5 at a fixed 250 MHz** (ArduPilot's clock 1.25×): measured again (1.2499×).
  - The foreign judge's times are virtual, so the divergence only shifts when ArduPilot's own delays end (INS at 3.57 s virtual is 4.46 s on its clock).
  - **Re-owned** to architecture §2's divergence list; not closed.
- **`usart1` at 200 MHz:** `BRR` `0x271` measured again. The proposal drops `BRR` for a foreign firmware, so the row stays a bench row. **Re-owned.**
- **The BMP581 configure row:** measured violated again. It closes by D6: (i) drops the configure step for a foreign firmware (weakening named); (ii) carries ArduPilot's own sequence.
- **`4400c2` D2 and D3, the USART line:** receive is not needed (P9), so the line stays held. **But D3's hard fault is, inferred, B1, and B1 needs no USART line.** It is reported here and re-owned to D1.

## Evidence

- In git, under `spike/4400e/`:
  - `drive.py`, `analyze.py`, `budget.py`;
  - `sh/`;
  - the `stubs-*.txt` hook files;
  - `out/`, which holds:
    - the CLI's argv and both `simulate` outputs;
    - `caps.log`;
    - per run, `steps.json`, `setup.cmds` and the USART1 capture;
    - `j1b`'s judge reads and its device lines (`models.log`);
    - each diagnostic run's hook lines from 3.5694 s (`hooks-near-fault.log`);
    - the witness summaries `j1b-witness.json` and `j2-witness.json`.
- The ELF is not in git. It is named by sha256 above, with the CI run that built it.
- The Renode logs of `r1` (1.4 GB) and `j2` (117 MB) were deleted after their figures were taken.

**STOPPED at §1.1**, 2026-11-17, for the architect's ruling on D1–D7. `main` is `0829fe83`, unchanged since this branch was cut. Title and body: `~/.cache/aet-4400e/pr-title-body.md`.

# Resumed: the build without the FIFO (rulings on #212 comment 6958822008 and #215 comment 6961943794)

`main` `d0381216` (`4400e2` record, `4400e3` RETTOBASE correction, `1313c`) is merged in as a merge commit, and re-gated (132 suites, 2,178 passed, 0 failed). 4220c (#214, the ICM's FIFO) is not merged: the FIFO data step and D7 wait on it, as the resume note orders.

## Built

- **D3, the declaration:** `simulate --firmware F --spec S --foreign TIME`.
  - It is never inferred, and the verdict's `boot` carries `"judge": "foreign"`.
  - `--for` and `--foreign` together are refused (exit 64). `--for` keeps its 100 ms bound and its exit 64.
  - The foreign window's bound, `FOREIGN_MAX_VIRTUAL_NS`, is 4 s, **provisional until D7**.
  - A separate `ForeignBoot` is serialized under the same `boot` key, so a generated verdict is byte for byte what it was (measured below).
  - The MCP snapshot gains the `foreign` property.
- **The memory rule** (`elf::check_foreign`, `elf::foreign_memory`):
  - flash, plus the six RAM regions ST's header states;
  - each base is its `#define`, and each size is its comment's. `svd-hal` gains `region_size_kib`, which reads the header's three comment shapes. The generator's `region_kib` is untouched.
  - both addresses of every `PT_LOAD` are checked;
  - the overlap rule applies to file bytes only (provisional).
- **D5, "booted":** the first identity transaction a declared device's model logs. In addition, `core_in_exception` and `pc_outside_firmware` are kept. Otherwise the verdict names `foreign_not_booted`.
- **D6 (i):**
  - `parts.json` schema 9 → 10 adds `reads`, with a paired `reads` citation. The BME280 lists its burst from `0xF7`, at least 6 bytes, cited to BST-BME280-DS001-24 §4, p. 23.
  - `Part::steps` never reaches a listed read, so no generated firmware performs one (tested).
  - The twin's `Model::read_witness` gives the BME280's witness: `Write F7`, then a `Read` of at least 6 bytes. The values aren't judged, because the stock model answers from the configuration the firmware wrote (a named residual).
  - The configure step is not judged for a foreign firmware. The verdict's rule table names that.
- **D4:** the SPI slave-mode `[ERROR]` is exempt only directly after a `CFG2` write with `MASTER` 0 while the controller's last `CR1` write left `SPE` 0. It is read from each SPI controller's access log, which a foreign boot turns on, and counted in `boot.spi_errors_exempted`.
- **The rule table:** `renode::foreign::RULES`, serialized in every foreign verdict's `boot.rules`. Each of today's rules is kept, replaced or dropped, with what it judges or weakens. The rules the ruled table did not name are marked `provisional` and were sent to the architect's inbox (00:20 UTC):
  - overlapping zero-filled segments;
  - the brown-out record;
  - flash-controller access;
  - `locked_peripheral_access`;
  - the motor outputs;
  - the `untrusted` halted stop.
- **The judge is its own function** (`renode/foreign.rs`) with its own commands. A foreign boot writes no boot word and no records, and installs none of our hooks and no injection. It keeps the platform check, the analog pre-pass, the sandbox, the bounds, the `RETTOBASE` correction and the cold RCC writes.

## Measured

- **The thin slice:** this branch's CLI, CI's `sentinel-02-own` ArduPilot (`a799dfb6…`), `--foreign 4000ms`.
  - **The register rules hold:** every solved pin, RCC at-least and USART1's frame bits. The core ends idle (`ICSR` `0x800`, `PC` in `.text`).
  - **D4 exempted 909 slave-mode errors.**
  - **The verdict fails**, for two reasons:
    1. **`renode_log_flood`: the 8 MiB budget binds before 4 s**, so no witness is read and nothing is booted. Peak RSS was 860 MiB (82 % of 1 GiB).
    2. **`locked_peripheral_access`:** ArduPilot writes DMA1 (`0x40020010`) before its clock is on, which silicon ignores. That rule is provisional.
- **Where the budget goes** (the CLI's own `--foreign` script, captured from `/proc` and replayed to 4 s): 9.29 MB counted.
  - Renode's own `Unhandled write` warnings at their default level:
    - `nvic` 2.7 MB, from cache-maintenance writes (upstream `7b687540` "NVIC: Ignore cache maintenance operations", in no release);
    - `dma1` 1.4 MB;
    - `spi2` 1.1 MB;
    - `i2c1` 1.0 MB.
  - **D4's SPI access log:** 2.15 MB.
  - By second: 2.46, 0.48, 0.38 and **5.97 MB** (3–4 s, where the IMU starts).
  - On the whole log, all 1,073 slave-mode errors are exempt and no other `[ERROR]` occurs. The one the CLI run reported unexempted was a casualty of lines dropped past the budget, which fails closed.
  - **This is D7's ruling item.** The window's budget binds before wall time, even before the FIFO's reads. Without D4's access log it would be about 7.1 MB at 4 s.
- **No generated-firmware verdict moves:**
  - Each firmware is judged by `main` `d0381216`'s CLI and this branch's: both boards at 100 ms, and campaigns of all 25 scenarios.
  - **The 54 archive files are byte-identical.**
  - Both boards' `simulate` verdicts, each tree mounted at the same path, are **byte-identical outside `memory`**. A `main`-against-`main` control differs in `memory` too (the run-to-run sample).
- **Gates:** fmt and clippy clean (also with `with-tools`); `cargo test --workspace`: **132 suites, 2,189 passed, 0 failed**.
- **The new tests:**
  - `aethelgard-ir`'s `reads` test and its seven refusals;
  - `svd-hal`'s six header regions and their bad shapes;
  - the twin's read witness;
  - four unit tests in `renode/foreign.rs`: D4's exemption, the rule table, the register families and the step search;
  - five CLI tests in `boot_tier1.rs` through a foreign stub: the judge, booted and devices; no witness, `foreign_not_booted`; D4; an exception; and the declaration with the memory rule.

## Moved bytes (all named)

- `parts.json` (schema 10, sha256 `97e328f6…`) and `aethelgard-ir`'s pin. No generated file or verdict reads the parts table's bytes (searched).
- `cli/tests/mcp-tools.json` (the `foreign` property, and `firmware`'s wording).
- The `mcp.rs` unit list and the `strict_fence.rs` module list.

## Remaining (not done in this stage)

- **After 4220c merges:**
  - merge `main`;
  - add the ICM's FIFO read to `parts.json` `reads` and the twin's `read_witness`;
  - measure the window (D7) with the IMU's reads, and stop if it needs a ruling (the budget measured above says it may);
  - then build to acceptance:
    - CI runs the foreign judge on the generated `sentinel-02-own` ELF, which lives in the `ardupilot-image` workflow, apart from the tools image;
    - the WO's mutations, each refused by name;
    - the full adversarial pass and the reduction pass.
- **The provisional rules** wait on the architect's ruling.

## Predictions before re-measuring under the rulings (PR #212, comment 6965511448)

Written before the run. CI's ArduPilot ELF (a799dfb6), this CLI, `--foreign 4000ms`:

1. **Budget:** the counted log drops from 9.29 MB to under 2 MB. Of the 6.2 MB of Renode's own `Unhandled write` warnings, one line per kind remains (tens of kinds). D4's 2.15 MB access log becomes about 1,100 hook lines (about 0.1 MB). `renode_log_flood` is absent.
2. **D4:** every slave-mode `[ERROR]` is exempt from the hook's lines, about 1,070 of them by 4 s, as the whole-log replay found. No `renode_error_log`.
3. **Flash:** ArduPilot keeps its parameters in flash sectors outside its image, so every loaded flash segment hashes unchanged and there is no `flash_overwritten`.
4. **Locked:** DMA1 written before its clock is on, listed as `info`, so the verdict does not fail on it.
5. **Booted:** with the log under budget, the identity witnesses are read and the run boots (no `foreign_not_booted`). The ICM's data read stays unwitnessed until 4220c.
6. **Peak RSS** stays near 860 MiB. Wall time is within 20 % of the last run.

## The #212 rulings, built and measured (comment 6965511448)

### Built
- **`PT_LOAD` overlap, BOR, motors, `--foreign TIME`:** accepted as built; "provisional" is gone from those rows. The bound stays a provisional constant until D7 is measured with the ICM's reads.
- **`locked_peripheral_access`:** for a foreign firmware it is one `info` finding. `boot.locked` lists each access (peripheral and address) with its first virtual time and count.
- **`flash_controller_access`, replaced: no write inside its own loaded ranges.** Two witnesses, both from what the firmware cannot write, neither from the access log (labelled deviation **R1**, below):
  1. **Program:** after the run, Renode hashes each loaded flash segment's bytes (`ReadBytes` in the monitor). Any difference from the ELF's file bytes is `flash_overwritten`.
  2. **Erase:** a hook on the flash controller logs each write to `FLASH_CR1`/`FLASH_CR2` that sets `START`. A sector (`SER`, `SNB`) or bank (`BER`) erase overlapping a loaded flash segment is `flash_overwritten`, even when the bytes are reprogrammed alike. Every offset, bit, base and size is read from ST's header (`FLASH_TypeDef`, `FLASH_CR_*`, `FLASH_SECTOR_SIZE`, `FLASH_SECTOR_TOTAL`). `boot.erases` lists each erase.
  - Weakens: writes elsewhere in flash are not judged. An erase is judged when started; whether the silicon would have refused it (bank locked) is not.
- **D7 (a):** the foreign boot's log filter summarises Renode's `Unhandled write` warnings.
  - Key: the line after its timestamp, with every written value elided.
  - The first line of each kind is kept as written. Every later line is counted and dropped, uncounted against the budget.
  - `boot.warnings` lists each kind with its count and first and last virtual time.
  - Generated boots use the unchanged filter (`summarise_none`).
- **D7 (b):** D4 reads a hook on each SPI controller's writes, which logs only a `CFG2` write with the value written and `CR1` as it stands. The whole SPI access log is gone.

### Measured (CI's ArduPilot a799dfb6, `--foreign 4000ms`, this CLI)
| | before | after |
|---|---|---|
| counted log | 9.29 MB (replay), over 8 MiB | under 8 MiB: no `renode_log_flood` (measured); 0.76 MB by replaying the old full log through a model of the new filter (modelled, not the run's own count) |
| `Unhandled write` kinds | — | 68 kinds, 45,247 lines (measured; the replay model also finds 68) |
| D4 exemptions | 909 (lines dropped by the flood) | 1,073, equal to the 1,073 slave-mode errors in the old full log |
| booted | `foreign_not_booted` (witnesses past the budget) | U4 identity at 161.938610 ms |
| witness times | old full log | identical: U4 161938610, U2 342585230, U2 data 382787270, U7 342780210, U5 3569253830 ns. The hooks do not move the run. |
| flash | — | both loaded segments (0x08000000, 1,297,576 B; 0x0813cca8, 4,132 B) unchanged |
| locked | fail | `info`, 22 accesses (DMA1 and DMA2 streams at 0.92 ms, I2C1 at 165.5 ms) |
| erases | — | 4, of bank 2's sectors 6 and 7 (0x081C0000, 0x081E0000) in its first 15 ms: its parameter storage, outside its image; no finding |
| peak RSS | 860 MiB | 779 MiB (with both hooks) |

**What is left failing:** `spi_transaction_unwitnessed` for U5 (the ICM-42688-P). Its script's data read is not what ArduPilot does; the FIFO read waits on 4220c.

**Flash controls (real Renode, hand-built ELFs in `~/.cache/aet-4400e/selfwrite`):**
- `store` writes one word into its own image: the hash fails.
- `nostore` (its nop twin): no finding.
- `erase` erases sector 1, which holds its own segment of all 0xFF: the erase witness fails while the hash is unchanged.
- `erase-other` (the same erase, no segment there): the erase is listed and nothing fails.

### Predictions against outcomes
1. Budget under 2 MB, no flood: **held**. No flood was measured; 0.76 MB is modelled.
2. D4, about 1,070 exempt, no `renode_error_log`: **held** (1,073).
3. Flash unchanged: **held**.
4. Locked is DMA1 only: **missed.** There are 22 accesses, on DMA1, DMA2 and I2C1.
5. Booted, ICM data unwitnessed: **held**.
6. RSS near 860 MiB: **held** (779 MiB).

### Labelled deviation R1: the flash rule is not read from the access log
- The ruling called it cheap from the access log. Read at source (`STM32H7_FlashController.cs`, renode-infrastructure `cd4b002a`), that log cannot place a program write: program data are stores to the flash memory, not to the controller's registers (L182–192), and an erase is the model's own `SetRange` (L441).
- The controller's full access log also costs budget.
- The hash witnesses every changed byte. The `START` hook witnesses every erase of an own sector, the one way to rewrite flash and leave its hash unchanged.
- For the architect: accept R1, or rule the access log.

### Gates and the generated verdicts (this commit)
- **Gates:** fmt and clippy clean (with and without `with-tools`); 132 suites, 2,193 passed, 0 failed.
- **CI's package step, run locally:** every crate packages, and the CLI builds from the packages and validates.
- **Generated verdicts, `main`'s CLI against this branch's:**
  - the 54 campaign archive files are byte-identical;
  - 32 of the 34 verdict lines are identical;
  - the two `simulate` verdicts differ only in `input.path` (each tree's own) and the memory sample, as in the last comparison.

## R1 ruled (PR #212, comment 6966155027): the four conditions, built

1. **The oracle is the ELF, on the host.**
   - It already was: `elf::flash_loads` hashes each loaded flash segment's file bytes before the run, and the judge compares Renode's post-run read with that.
   - No pre-run read happens in the twin.
   - The tier-2 controls also hash each segment in the test itself and assert that it equals the verdict's `expected_sha256`.
2. **The weakening is named in the rule table:**
   - A store that changes a word of an own segment, and a later store that restores it with no erase between, passes.
   - Silicon cannot do that. Programming only clears bits, and "these operations do not support the return of a bit to its virgin state. In this case an erase operation of the entire sector is required" (RM0433 Rev 9 §4.3.9 "FLASH program operations", p. 161; the same page advises against overwriting a non-virgin flash word).
   - So any change and restore needs an erase, which the `START` hook fails.
3. **Every erase the header defines:**
   - sector (`SER` + `SNB`) and bank (`BER`) on `FLASH_CR1`/`CR2` with `START`;
   - mass erase, `MER` on `FLASH_OPTCR`. It sets `BER1`, `BER2`, `START1` and `START2`, per RM0433's `FLASH_OPTCR` description, so it is judged as both banks.
   - Offsets come from `FLASH_TypeDef`, bits from `FLASH_CR_*_Pos` and `FLASH_OPTCR_*_Pos`, and sizes from `FLASH_SECTOR_SIZE` × `FLASH_SECTOR_TOTAL`.
   - `OPTSTART` is the `info` finding `flash_option_change`, with each time in `boot.option_changes`.
   - Measured with the hook: ArduPilot's 4 s run starts no option change (`boot.option_changes` is empty) and the same four bank-2 erases. Nothing changes an own segment's protection, so no stop. Peak RSS 833 MiB.
4. **The controls are tier-2 tests** in `twin_tier2.rs`, each refused by name, on the reference platform under `--foreign 10ms`:
   - `a_foreign_firmware_that_stores_into_its_own_flash_fails_flash_overwritten`;
   - `a_foreign_firmware_that_erases_its_own_sector_fails_though_its_hash_holds`;
   - `a_foreign_firmware_that_erases_outside_its_own_flash_is_no_finding` (bank 2 sector 6, ArduPilot's real case).
   - All three passed in the tools image (3 passed, 11.6 s).
   - TIER2_MIN 183 → 186.
   - Gates after these: fmt and clippy (both feature sets) clean; 132 suites, 2,193 passed, 0 failed; CI's package step passes on the host (the tools image has no `jq`).
   - They join §2's mutation list: the hash compare, the hash-count check, the erase overlap, the `MER` bit, and `OPTSTART` as info.
   - Tier 1 adds the `MER` fail and the `OPTSTART` info, and requires the erase hook's exact argv.
