# aethelgard-renode-models

Aethelgard's own [Renode](https://renode.io) 1.17.0 peripheral models, written for this project in C# (`RUL-12` as amended, `AET-WO-4220`):

| File | Part | Bus | Identity |
|---|---|---|---|
| `ICM42688P.cs` | TDK InvenSense ICM-42688-P (6-axis IMU) | SPI | `WHO_AM_I` 0x75 = 0x47 |
| `BMP581.cs` | Bosch Sensortec BMP581 (barometer) | I2C | `CHIP_ID` 0x01 = 0x50 |
| `AethelgardSTM32Timer.cs` | STM32H7 TIM1–TIM17 (a DShot output's timer, foreign platforms only) | — | stock `STM32_Timer`'s registers and conventions; `UDE` raises the update DMA request (RM0433 Rev 9 Table 124, p. 697), `DCR`/`DMAR` bursts (pp. 1586, 1615–1617), `OCxPE` preload (p. 1603) |
| `AethelgardSTM32DMA.cs` | STM32H7 DMA1/DMA2 (with the timer above) | — | stock `STM32DMA`'s behaviour, but a memory-to-peripheral stream whose DMAMUX1 request is a timer's moves one item per request and ends at `NDTR` 0 with `TCIF` (pp. 654, 665) |

Data only: the crate exports each file's text as `FILES`. `aethelgard-twin` pins every file by sha256, the emitted platform names each model it uses with that sha256, and the CLI writes its own copy, hashes it again and includes it before Renode loads the platform. A changed model is refused until its pin moves with it.

Each model logs one `Noisy` line per transaction with every byte moved, and nothing on an empty finish. It samples on its own timer at the configured output data rate, raises its data-ready status and interrupt pin itself, and rounds a stimulus to register counts half away from zero.

The ICM-42688-P also has its FIFO (`AET-WO-4220c`), as DS-000347 Rev 1.9 defines it:
- **Modelled:** the 16-bit packets 1 to 3 in STOP-on-FULL mode, filled from the same samples as the data registers, in virtual time. The count is in records or bytes, in either byte order, and latched by its high register. A burst read holds at `FIFO_DATA`. Also modelled: `FIFO_FLUSH`, packet 3's ODR timestamp and `FIFO_FULL_INT`.
- **Left out, each named by a warning when a firmware selects it:** the 20-bit packet, Stream-to-FIFO, FSYNC in the FIFO, the watermark and FIFO interrupts on a pin. `FIFO_LOST_PKT` stays an unmodelled register, which warns on access.
- **Sources:** where the datasheet is silent, the model follows TDK InvenSense's own ICM-426xx driver (eMD 2.0.9). Where both are silent, a rule is labelled inferred in the source.

The timer and the DMA (`AET-WO-4412` D4) are emitted only into a foreign platform (`<project>.foreign.repl`), only for the timers of DShot outputs, by cancel-and-restate: the stock entry cancelled, the model declared at its address under its name. The timer is eager (an event at every update) while a DMA request or interrupt is enabled, and lazy otherwise; on `arm12` it equals stock `STM32_Timer` access for access, and on `dshot12` it equals its own eager run (`spike/4412/proto/NOTES.md`).

Licensed `MIT OR Apache-2.0`, like the rest of the workspace. No Renode source is copied into these files.
