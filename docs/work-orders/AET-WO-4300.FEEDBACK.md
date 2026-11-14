# AET-WO-4300 FEEDBACK — flight I/O spike

**Builder:** Claude (strong tier), worktree `~/aethelgard-4300`, branch `wo/aet-4300-flight-io` from `main` `2967d16`.
**Cadence:** spike (§1), then STOP for the architect's ruling.

The predictions in this section were committed before any firmware build or any container run. They are not edited afterwards; corrections go in a later section.

Labels:
- **[read]** — read at source: Renode 1.17.0 at renode `ab721d88` / Infrastructure `066a7f13` (`~/.cache/aet-2301/src`), DS12117 Rev 10 and RM0433 Rev 9 text, or this repository at `2967d16`.
- **[measured]** — run in the pinned image under rootless podman, caps read back.
- **[inferred]** — neither; reasoning from the two.

`2204`'s FEEDBACK says the image's binary (`f1dd1b4af`) differs from the source read only in `MappedMemory.cs` and one unit test. No model read below is either of those.

## 0. Source reads that shape the predictions

Each item is **[read]**.

- **Timers.** `stm32h7.repl` binds `timer1`–`timer17` (except 6/7 outputs) to `Timers.STM32_Timer`, each with the fixed parameter `frequency: 250000000`, registered in `rcc` by alias.
  - The repl wires each channel to its alternate-function pins (`timer1: 0 -> gpioPortA#08@01 | …`). `STM32_GPIOPort.GPIOAlternateFunction.OnGPIO` forwards the channel's level to the pin only while the pin's MODER is AF and its AFR selects that function.
  - Each channel's output is set or cleared by the timer's own `LimitTimer` events: at update for PWM mode 1 (`Set`) and at compare match (`Unset`).
  - `BDTR.MOE` is a tagged flag; `CCxP` is read only for inputs (capture edge, gated mode). The output path never consults polarity.
  - `DIER.UDE`, `CC1DE`–`CC4DE`, `TDE` and `CR2.CCDS` are tagged. `DCR` (0x48) and `DMAR` (0x4C) are in the register enum but not defined, so they read 0 and log an unhandled access.
  - Logging: `Noisy` "IRQ pending" at each update; `Noisy` "cctimer{n}: Output Enable set to {v}" at a `CCER` write; `Noisy` compare-IRQ lines only when the channel's interrupt is enabled. Nothing at a pin edge.
- **Tagged fields** (`PeripheralRegister.Tag`) store nothing: a read returns the field's reset value, and a write is logged as unhandled.
- **GPIO.** `STM32_GPIOPort` `ODR` and `BSRR` writes call `WritePin` for every pin, whatever its mode. A pin in AF mode can therefore be driven by a store.
- **Miscellaneous.LED** logs "LED state changed to {state}" at `Noisy` on each change. It is the cheapest pin observer.
- **Log time.** The logger stamps each entry with `MasterTimeSource.ElapsedVirtualTime`, which advances at sync points; the default quantum is 100 µs (`TimeSourceBase.DefaultQuantum`). The CLI's config sets `log-timestamps-type = Host`, so its log carries no virtual time at all.
- **USARTs** are `UART.STM32F7_USART` with fixed `frequency` parameters (200 MHz for `usart1`/`6`, 125 MHz for `usart2`). `TransmitCharacter` raises `CharReceived` at once: no baud time. `SWAP`, `RXINV` and `TXINV` are tagged. Parity and stop bits are stored and reported as properties, but no byte carries framing. The model logs nothing per byte.
  - Capture is by an attached analyzer or `CreateFileBackend` (bytes to a file); injection is `WriteChar` (monitor).
- **USB.** `usb1` and `usb2` are `USB.STM_USB`, a Synopsys DWC2 OTG device model (`GOTGCTL` … `DCFG`), declared `@ sysbus` only: **not** registered in `rcc`, so no clock gating. Renode 1.17.0 has a host side: `USB.USB_UART`, a `USBHost` that enumerates a device, finds CDC and CDC-Data interfaces, sets DTR and exposes the data endpoints as a UART. Antmicro's own test `tests/platforms/nucleo_h753zi.robot` "Should Communicate Over USB" runs a Zephyr CDC shell on `usb2` through it.
- **SYSCFG** is `Miscellaneous.STM32_SYSCFG`. Its register collection defines only `EXTICR1`–`4`. `CCCSR` (0x20), `CCVR` and `CCCR` exist in the enum only. A write is dropped with an unhandled-write log; a read returns 0. `READY` (bit 8) therefore never reads 1.
- **DMA.** `dma1`/`dma2` are `DMA.STM32DMA`, fed by `dmamux1` (`STM32_DMAMUX`) through GPIO request lines. No timer is wired to `dmamux1` in the repl, and the timer raises no DMA request.
- **RCC.** `STM32H7_RCC` enables and disables each peripheral registered under an alias (`sysbus.SetPeripheralEnabled`) from its `*ENR` bit. The CLI already cold-writes `AHB4ENR`, `APB1LENR` and `APB2ENR` (0x580244E0/E8/F0) to 0 before the run.

## 1. Predictions (committed before any run)

Probe plan, so the predictions have something to point at. All firmware is the committed reference (`boards/sentinel-01/firmware`) plus raw register code inserted before the boot word, built offline in the pinned image (`4200`'s `fw.py` method). All runs use the CLI's composition (`2204`: complete `--config`, synchronous logging, `logLevel -1` per witness source) unless a row says "deviation".

- Observers: a `Miscellaneous.LED` on `PA8` (TIM1_CH1, AF1) added with the `|` form so the stock route `8 -> timer1@00` stays.
- TIM1 probes set `TIM1EN`, `PA8` to AF1, `PSC = 249` (1 MHz at Renode's 250 MHz), `CCMR1` PWM mode 1 with preload, `CCR1`, `CCER.CC1E`, `BDTR.MOE`, `EGR.UG`, `CR1.CEN | ARPE`.

### S1. Timers

| id | probe | prediction |
|---|---|---|
| P1.1 | `pwm50`: ARR 19999, CCR1 1500 (50 Hz, 1.5 ms), CLI composition | LED lines "LED state changed to True/False" alternate, two per period, about 8–10 in 100 ms. Host timestamps only: the edges are witnessed, their timing is not. |
| P1.2 | `pwm50`, deviation: `log-timestamps-type = Virtual` | Edge times are multiples of the 100 µs quantum: period 20.0 ms ± 0.1, high time 1.5 ms ± 0.1. |
| P1.3 | `pwm400`: ARR 2499, CCR1 1500 | 2.5 ms period, ≈ 75–80 LED lines in 100 ms; with virtual time, ± 0.1 ms. |
| P1.4 | `nomoe`: no `BDTR` write | Same edges as `pwm50`. `BDTR` reads 0 both ways. The twin drives a pin silicon would leave dead. |
| P1.5 | `cc1p`: `CCER = CC1E \| CC1P` | Same edges as `pwm50`, not inverted. Silicon inverts. |
| P1.6 | `nogate`: no `TIM1EN` | No LED line. One disabled-peripheral warning per TIM1 access (≥ 7). |
| P1.7 | `claim`: record only, no timer | No LED line. |
| P1.8 | `cen-off`: full setup, `CEN` never set | 0 or 1 LED line (the `UG` event), none periodic. |
| P1.9 | `bitbang-af`: `PA8` in AF1, timer off, `BSRR` toggles with a delay loop | LED lines of the same form as `pwm50`, and the same `MODER`/`AFR`. The pin is forgeable by a store. |
| P1.10 | `pwm600k`: PSC 0, ARR 416, CCR1 312 (DShot600 bit 1.67 µs, a "1" at 75 %), 20 ms, virtual time | Edges counted at the timer's rate (≈ 2 per 1.67 µs, > 20 000 lines), but timestamps collapse to 100 µs steps: bit time is not resolvable from the log. |
| P1.11 | DShot by DMA | **[read]** Not modelled: no DMA request from the timer, `DMAR` absent. DShot can be judged only statically: DMA stream and DMAMUX request registers, and `PSC`/`ARR` against the clock tree. |
| P1.12 | Clock | **[read]** The twin's timer clock is the repl's `frequency`, not the clock tree. A `PSC` computed for silicon (HSI 64 MHz here) runs 3.9× fast in the twin unless the emitted platform sets `frequency` per timer. |

Witness (P1, **[inferred]**): no timer log line marks an edge. The candidate witness is the pin observer's own line, and P1.9 predicts a store forges it. What a store cannot forge is the *timing*: P1.2 predicts timer edges land on a 100 µs grid from a deterministic clock, but a delay loop can land there too. Predicted conclusion: timer outputs are forgeable at the pin; the twin can check presence and period, not that the timer made them.

### S2. UARTs

| id | probe | prediction |
|---|---|---|
| P2.1 | `uart`: USART2 on PA2/PA3 AF7, 115200 8N1 (`BRR = 1085` for the repl's 125 MHz, see P2.6), sends `AET-UART\r\n` then `00 FE 09 FD 7F 80 FF 0D 0A` | The file backend holds the 19 bytes exactly. The model logs nothing per byte. No baud time passes. |
| P2.2 | `uart`, inject 4 bytes with `sysbus.usart2 WriteChar` between two `RunFor`s, after `RE` is set; the firmware echoes | The 4 bytes come back in the file after the pattern; the record counts 4. |
| P2.3 | `uart-sbus`: 100 kbaud, 8E2 (`M0`, `PCE`, `STOP = 10`), `TXINV \| RXINV` | Same bytes captured as P2.1. `CR2` reads back with `STOP = 10` and without `TXINV`/`RXINV`. One unhandled-write log for the tags. Framing faults cannot be rendered. |
| P2.4 | `uart-nogate`: no `USART2EN` | No byte captured; disabled-peripheral warnings. |
| P2.5 | witness | **[inferred]** A store cannot put bytes in the file backend. The bytes witness that the firmware wrote `TDR` with `UE`/`TE` set, not that a receiver accepted them; an echo witnesses `RDR` reads. |
| P2.6 | clock | **[read]** `BRR` is divided from the repl's fixed `frequency`, not the clock tree (as P1.12). |

### S3. USB

| id | probe | prediction |
|---|---|---|
| P3.1 | Antmicro's Zephyr `shell-cdc` ELF for `nucleo_h753zi` (third-party firmware), `USB_UART` registered on `usb2`, no network in the container | Enumerates; the shell prompt `uart:~$` is captured from `usb_uart` within 5 s of virtual time. |
| P3.2 | our firmware | **[inferred]** Feasible, but needs a USB device stack in the board crate: no USB crate is cached and none is in the lock file. Not measured with our firmware. |
| P3.3 | clock gating | **[read]** USB is not gated by `RCC` in the twin. |

### S4. Power and ADC — proposal only

- **P4.1 [read]** The `2301` bridge solves DC operating points of declared parts; a regulator's input range and dropout are datasheet fields, so a static check can read them without SPICE.
- **P4.2 [inferred]** ADC battery monitoring goes out of this work order: the twin's `STM32H7_ADC` value path is host-fed, so a measured voltage would be the judge's own number.

### S5. Compensation cell and pin speeds

| id | probe | prediction |
|---|---|---|
| P5.1 | `cccsr-bounded`: `SYSCFGEN`, `CCCSR = EN`, poll `READY` up to 100 000 times, record the outcome and `CCCSR` | Timeout recorded; `CCCSR` reads 0; unhandled write and read logs from `syscfg`; the boot word is written. |
| P5.2 | `cccsr-wait`: as P5.1 without the bound | Hangs: the boot word stays 0, PC inside the poll. **§7's second condition holds.** |
| P5.3 | Table 149 | **[read]** The speed-`10` and `11` rows carry note 5 (cell enabled). The datasheet gives no figure for those speeds with the cell off. |

### S6. Cold `RCC_*ENR`

- **P6.1 [read]** TIM1/8/15/16/17 and USART1/6 are in `APB2ENR`; TIM2–7/12–14, USART2/3 and UART4/5/7/8 in `APB1LENR`. Both are cold-written today. LPUART1 and SYSCFG are in `APB4ENR`; OTG1/2, DMA1/2 and ADC12 in `AHB1ENR`. Neither of those is cold-written today.
- **P6.2** `nogate` (P1.6) and `uart-nogate` (P2.4) measure that the twin drops a peripheral whose enable bit the firmware never set.
