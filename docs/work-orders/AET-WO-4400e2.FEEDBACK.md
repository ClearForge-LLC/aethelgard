# AET-WO-4400e2 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-4400e2-hard-fault`, from `main` at `0829fe83`. The work order is read from `~/.cache/aet-4400e2-wo.md` (docs PR #213). The ruling that ordered it is #212, comment 6958822008, D1 (a). Prototypes and every diagnostic deviation live under `spike/4400e2/`; no crate, model, workflow or `container/` file changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

## Predictions — written before any run

### What they rest on (read at source, nothing run)

**ChibiOS's port** (the pinned ArduPilot image, ChibiOS `d103983f`, `os/common/ports/ARMv7-M/chcore.c`):
- `__port_irq_epilogue` (L312–) runs at the end of every kernel ISR.
- **When `SCB->ICSR & RETTOBASE` is set (L315)**, it pushes an artificial exception frame on the PSP. That frame returns to `__port_switch_from_isr`, which calls `chSchDoPreemption`. Afterwards, `__port_exit_from_isr` and `svc 0` discard the frame.
- **The decision rests entirely on `RETTOBASE`:** 1 must mean "this ISR returns to thread mode".
- `CORTEX_SIMPLIFIED_PRIORITY` is `FALSE` (`chcore.h` L221), so kernel ISRs at different priorities can nest.

**Renode 1.17.0's NVIC** (`renode-infrastructure` `cd4b002a`, `src/Emulator/Cores/Arm-M/NVIC.cs` L1079):
```
.WithFlag(11, FieldMode.Read, valueProviderCallback: _ => activeIRQs.Intersect(Enum.GetValues(typeof(SystemException)).Cast<int>()).Count() <= 1, name: "RETTOBASE")
```
`SystemException` (L3195–) holds exceptions 1–15 only. **So an external interrupt active beneath the current one is not counted:** with two external IRQs nested, or one over SVC, `RETTOBASE` reads 1.
- **Armv7-M (`ICSR`):** `RETTOBASE` is 1 only when no exception other than the current one is active.
- **Upstream:** `e7a8584c` (2026-10-07, "[GH#941] NVIC: Make ICSR.RETTOBASE take external interrupts into account") changes it to `activeIRQs.Count() <= 1`. That is after v1.17.0 (2026-09-07), and no release has followed (read through the GitHub API, 2026-11-17).

**Renode's SPI model** (`STM32H7_SPI.cs`, sha256 `bc3f1348…`, L187–194): `CFG2.MASTER` reads 1 always, and a write of 0 logs `[ERROR] Attempted to set peripheral into SPI slave mode`.
- **ChibiOS:** `spi_lld_stop` writes `CR1 &= ~SPE`, then `CFG2 = 0U` (`os/hal/ports/STM32/LLD/SPIv3/hal_spi_lld.c` L736–746).
- **ArduPilot:** it calls `spiStart` per transaction (`SPIDevice.cpp` L386) and `spiStop` on release (L372).
- **RM0433 Rev 9** §50.11.4 (p. 2221): `SPI_CFG2` reset value `0x0000 0000`; "write protected when SPI is enabled"; bit 22 `MASTER`: 0 slave, 1 master.

### Predicted

| id | What | Prediction |
|---|---|---|
| P1 | **The identity** (`4400c2`'s `crash-a`, with `4400e`'s PSP-frame hook) | the same switch: `R0` = `ch0`, `R1` = the idle thread, `INVSTATE`, at `4400c2`'s 1.601556130 s. **The same fault** |
| P2 | **`RETTOBASE` at each ISR's exit** (`__port_irq_epilogue`'s entry, with every active exception read from `IABR0`–`4` and `SHCSR`) | in the faulting cycle, an epilogue runs with **two or more exceptions active and reads `RETTOBASE` 1**: one kernel ISR nested in another. No earlier epilogue does both |
| P3 | Every interrupt in the last millisecond | DMA1 stream 2 (SPI2) and TIM5; the nested pair is one of them inside the other |
| P4 | `FPCCR`, `FPCAR` and the frame type | `FPCCR` `ASPEN` and `LSPEN` set; every `EXC_RETURN` `0xFFFFFFED`, frames extended as it says. Lazy stacking is **not** the cause |
| P5 | `SVCall` and `PendSV` priorities (`SHPR2`, `SHPR3`) against the ISRs' (`NVIC_IPR`) | as `chcore.h` sets them; tlib honours them. Not the cause |
| P6 | **Deviation A:** `RETTOBASE` given its architectural value at the epilogue's `ICSR` load (a hook that rewrites the loaded register; the firmware unchanged) | **no fault** at 3.5708 s: ArduPilot reaches `init_gyro` and polls `FIFO_COUNT` (`0x2E`), then waits (`4400e` B2). `4400c2`'s repro also does not fault |
| P7 | **Deviation B:** ArduPilot built with lazy stacking off (`FPCCR.LSPEN` 0) in the pinned ArduPilot image | **still faults**, at or near the same instant |
| P8 | **Deviation C:** a Renode with `e7a8584c` (no release has it; a scratch image of a development build) | no fault (if a build is obtainable inside a scratch image) |
| P9 | **The class** | **twin emulation:** Renode 1.17.0's `ICSR.RETTOBASE`. **Smallest fix:** pin a Renode carrying `e7a8584c`, or carry that one-line patch in the pinned image's build. Weakens no witness |
| P10 | **D4, the SPI `[ERROR]`** | written by `spi_lld_stop`'s `CFG2 = 0U` after `SPE` is cleared: one per `spiStop`. On silicon, the reset value written to a disabled SPI. **A false alarm** of a model that supports only master mode |
| P11 | Cost | each diagnostic run under 3.6 s virtual: 10–60 s of wall (hooks at high rate are slow, `4400e` `r7`) and 500–550 MiB |
