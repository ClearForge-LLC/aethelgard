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

## 2. Runs

- Four containers of the pinned image (`container/IMAGE`), one at a time, each under `spike/4300/img.sh` at `--memory 8g --memory-swap 8g --pids-limit -1`. Each read back `memory.max=8589934592 memory.swap.max=0 pids.max=max` before its command started (`out/caps.log`); none logged `CAPS NOT READ`. `TMPDIR` was unset inside each.
  1. A failed start: `run.sh` passed a multi-line case list into `bash -c`, which stopped at a syntax error before any build or run. Fixed in `run.sh`.
  2. The 14 firmware builds (`out/fw.txt`, sha256 per ELF) and the 15 cases (`out/runs.jsonl`).
  3. `edges.py` over the raw stdout of the timer cases (`out/edges.txt`).
  4. `usb.py`, the S3 run (`out/usb.jsonl`).
- Every case uses the CLI's composition (`drive.py`, `4200`'s copy of `renode.rs`: complete `--config`, synchronous logging, `logLevel -1` per witness source). Every timer and UART case except `pwm50-host` runs with **one labelled deviation**: `log-timestamps-type = Virtual` in place of `Host`. `pwm50-host` is the composition verbatim.
- Raw stdout and the UART and USB byte captures stay in `~/.cache/aet-4300/raw`; the full `runs.jsonl` (60 101 kept lines for `pwm600k`) is `~/.cache/aet-4300/runs-full.jsonl`. The committed copy caps `kept` at 200 lines per case.
- **Third-party firmware (S3 only).** Antmicro's `nucleo_h753zi--zephyr-shell-cdc.elf`, the one `tests/platforms/nucleo_h753zi.robot` names. URL `https://dl.antmicro.com/projects/renode/nucleo_h753zi--zephyr-shell-cdc.elf-s_3030364-5067ae0e61cb460735c8e5c43f6a0000ccd17bdc`. Size 3 030 364 B and SHA-1 `5067ae0e…bdc` both match the URL; sha256 `109d26f2…cc2b`. It was downloaded on the host into `~/.cache/aet-4300/thirdparty`, not committed, and run with `--network none`.

## 3. Results beside predictions

| id | prediction | result | label | verdict |
|---|---|---|---|---|
| P1.1 | `pwm50`, CLI composition: alternating LED lines, about 8–10 in 100 ms, no timing | 9 LED lines (5 rises, 4 falls); host stamps only | measured | as predicted |
| P1.2 | Virtual stamps on a 100 µs grid; period 20.0 ± 0.1 ms, high 1.5 ± 0.1 ms | Stamps are exact to the nanosecond, 9 distinct. High **1 500.000 µs** every period; period **19 999.000 µs**, not 20 000; first rise at 19 999 µs, not at `CEN` | measured | **wrong**: there is no quantum grid, and the model's period is `ARR` ticks, not `ARR + 1` |
| P1.3 | `pwm400`: ≈ 75–80 lines; ± 0.1 ms | 79 lines; high 1 500 µs, period **2 499 µs** (silicon: 2 500), exact | measured | count as predicted; timing as P1.2 |
| P1.4 | `nomoe`: same edges; `BDTR` reads 0 | Edges identical to `pwm50` to the nanosecond; `BDTR` reads 0 in both. `pwm50`'s `MOE` write is visible only as `[WARNING] timer1: Unhandled write to offset 0x44 … Tags: Main Output Enable (MOE) (0x1).` | measured | as predicted. The twin drives an output silicon leaves dead |
| P1.5 | `cc1p`: same edges, not inverted | Identical to `pwm50`; `CCER` reads 0x1 | measured | as predicted |
| P1.6 | `nogate`: no LED line; ≥ 7 warnings | 0 LED lines; 9 warnings, `[WARNING] sysbus: Tried to write a locked peripheral: machine-0.timer1. Address …` (8 writes, 1 read) | measured | as predicted |
| P1.7 | `claim`: no LED line | 0 | measured | as predicted |
| P1.8 | `cen-off`: 0 or 1 line | 0 | measured | as predicted |
| P1.9 | `bitbang-af`: same lines and same `MODER`/`AFR` | 8 LED lines of the same form, exact virtual stamps, `MODER`/`AFRH` equal to `pwm50`'s. Every edge also gives `timer1: Channel #0 received external input when configured as output` (the stock route `8 -> timer1@00` loops the pin back), so that line does not tell a timer edge from a store. **Not** present: `timer1: IRQ pending` at each rise, and `CR1` reads 0 | measured | as predicted: the pin is forgeable |
| P1.10 | `pwm600k`: > 20 000 lines; bit time not resolvable | 60 101 kept lines in 20 ms. High **1.24–1.25 µs**, period **1.66–1.67 µs** (416/250 MHz = 1.664; silicon 417/250 MHz = 1.668), each edge at its own stamp | measured | **wrong**: DShot600 bit timing is resolvable |
| P1.11 | DShot by DMA not modelled | Read only; no DMA probe was built | read | as predicted |
| P1.12 | Timer clock is the repl's `frequency` | `PSC = 249` gave a 1 MHz tick from the fixed 250 MHz, not from this firmware's clock tree (HSI, 64 MHz on silicon) | measured | as predicted |
| P2.1 | `uart`: 19 bytes exactly; nothing logged per byte | File backend: `AET-UART\r\n 00 FE 09 FD 7F 80 FF 0D 0A`, exactly; 0 `usart2` lines at `Noisy` | measured | as predicted |
| P2.2 | 4 injected bytes echoed; record 4 | `41 00 FE 0A` follow the pattern; record 1 = 4 | measured | as predicted |
| P2.3 | `uart-sbus`: same bytes; `STOP = 10` kept; `TXINV`/`RXINV` lost; one warning | Same 23 bytes. `CR2` reads `0x2000`; `CR1` reads `0x40D` (`PCE` kept). **Two** warnings: `RXINV (0x1), TXINV (0x1)` on `CR2`, and `MO (0x1)` on `CR1`, so the 9-bit word select is a tag too | measured | as predicted, with one more tag |
| P2.4 | `uart-nogate`: nothing captured; warnings | Nothing captured; `Tried to write/read a locked peripheral: machine-0.usart2` for each access. The probe's bounded polls read 0 from the locked `ISR` about 2.7 M times in 200 ms, each with a warning, so the boot word was never written in the window | measured | as predicted; the flood is the probe's |
| P2.5 | A store cannot fill the capture | No store-only variant was run for the UART; follows from the source (bytes enter the backend only through `TransmitCharacter` from a `TDR` write with `UE` and `TE` set) | read | stands |
| P2.6 | `BRR` against the repl's 125 MHz | `BRR = 0x43D` (1 085) passed bytes; the model ignores the baud rate for timing | measured (bytes) / read (timing) | as predicted |
| P3.1 | Zephyr CDC enumerates; prompt within 5 s | 3 054 bytes to the host side within 5 s of virtual time (6.7 s wall): 30 prompts `uart:~$`, Zephyr's `usbd_init` lines, and `demo board` answered `nucleo_h753zi`. `DCFG = 0x02200030` (device address 3, the host's), `DCTL = 0`. 23 tag warnings from `usb2` (`PHYSEL`, `CSRST`, `PWRDWN`, `INEPTXSA`/`INEPTXFD`, `BVALOEN`/`BVALOVAL`, `DSPD`, endpoint interrupt flags) | measured, **third-party firmware** | as predicted |
| P3.2 | Our firmware needs a USB device stack | No USB crate is in the cache or the lock file | read | stands; not measured with our firmware |
| P3.3 | USB not gated | `usb1`/`usb2` are not registered in `rcc`; `IsPeripheralEnabled` then returns false, so `USB1OTGHSEN`/`USB2OTGFSEN` always **read 0**, whatever the firmware writes | read | as predicted, plus the read-back |
| P5.1 | Timeout; `CCCSR` 0; unhandled logs; boot written | Record 1 = timeout (`STATUS_RXNE` used as the timeout code), `CCCSR = 0`; `syscfg: Unhandled write to offset 0x20, value 0x1.` then 100 000 × `Unhandled read from offset 0x20.`; boot word `0x2100B007` | measured | as predicted |
| P5.2 | Hangs: boot 0, PC in the poll | Boot word `0x0`, PC `0x080005A4` in the poll after 10⁷ instructions; 3 332 936 unhandled-read warnings in 100 ms | measured | **as predicted: §7's second condition holds** |
| P5.3 | Speed `10`/`11` rows need the cell; no figure without it | As predicted (table in S5 below) | read | as predicted |
| P6.1 | Enable registers | As predicted (S6 below) | read | as predicted |
| P6.2 | Ungated peripherals are dropped | `nogate`, `uart-nogate`: every access refused with a `locked peripheral` warning | measured | as predicted |

## 4. Answers, by question

### S1. Timers as ESC outputs

- **Model** [read]: `Timers.STM32_Timer`, 250 MHz fixed clock. Channel outputs reach a pin through the AF route when the pin is AF and selects the timer. PWM 1 and 2, toggle and forced modes drive the output. `MOE`, `OCxPE` (preload) and output polarity are not modelled.
- **Period and duty are observable** [measured], with virtual timestamps, to the nanosecond: at 50 Hz, 400 Hz and DShot600's bit rate. With the CLI's host timestamps, only the edges' existence and order are observable.
- **Fidelity gaps** [measured]:
  - the period is `ARR` ticks where silicon is `ARR + 1`;
  - the first edge comes one period late;
  - `MOE` absent still drives the pin;
  - `CCxP` does not invert;
  - the clock is the repl's constant.

  A judge must expect the model's values, or the emitted platform must correct what it can (`frequency` per timer).
- **What it logs** [measured]: `Noisy` `IRQ pending` at each update, `Output Enable set to …` at a `CCER` write, and `output compare mode set to …` at a `CCMR` write. A `WARNING` for each tagged bit written (`OC1PE`, `MOE`). Nothing at an edge: the pin observer (`Miscellaneous.LED`) logs the edges.
- **DShot** [read]: the H7's DMA can't drive a timer in Renode 1.17.0.
  - `DIER`'s DMA enables and `CCDS` are tags; `DCR`/`DMAR` are undefined (`DMAR` read 0, measured); no timer request reaches `dmamux1`.
  - **What can be judged:**
    - the bit timing, measured, by running the timer at the DShot bit rate with a fixed duty (`pwm600k`);
    - statically: `PSC`/`ARR` against the clock tree, and the DMA stream, DMAMUX request and `CCR` address as registers.
  - A DShot frame's content cannot be witnessed.
- **What a store can forge** [measured]:
  - The pin edge itself: `bitbang-af` drives PA8 in AF1 by `BSRR`, and the observer cannot tell.
  - The timer's own `IRQ pending` line is at the identical nanosecond stamp as each rise in `pwm50` [measured: `19 999 000`, `39 998 000`, `59 997 000` ns], and `bitbang-af` has none [measured].
  - A store cannot put that line at the stamp of its own `BSRR` edge: a `UG` write is a separate instruction, ≥ 10 ns away at 100 MIPS [inferred]. The same holds with `CR1.CEN` set.
  - A firmware that runs the timer correctly *and* toggles `BSRR` adds edges the judge would see as wrong timing.
- **Recommendation:** witness timer outputs by the pin observer's edges with virtual timestamps, each rise matched to a `timer<n>: IRQ pending` at the identical stamp, and `CR1.CEN` read back. Judge period and high time against the solved values, with the model's `ARR` convention. Judge DShot by bit timing measured plus DMA configuration static. This needs the CLI's `log-timestamps-type` changed (decision D2).

### S2. UARTs

- **Model** [read]: `UART.STM32F7_USART`.
  - TX goes to anything attached: an analyzer, a file backend or another UART.
  - RX is injected by `WriteChar`.
  - The model logs nothing per byte.
- **Measured:** the file backend captures every transmitted byte, binary included, and injected bytes reach `RDR`. Virtual time per byte is not available: transmission takes no time, and the backend records no stamps.
- **Witness or claim** [measured / read]:
  - The captured bytes are a witness that the firmware drove the USART: a store cannot make them (P2.5).
  - The echo of an injected byte witnesses that the firmware read `RDR`.
  - Neither witnesses timing or framing.
- **Framing** [measured]: baud, parity, stop bits and inversion do not affect the bytes. `BRR`, `PCE`, `PS` and `STOP` are stored and can be judged as registers. `M0` (9-bit word), `TXINV` and `RXINV` are tags: not stored, visible only as the unhandled-write warning.
  - **SBUS** (100 kbaud, 8E2, inverted): its inversion and its 9-bit word can be judged only from those warning lines.
  - **CRSF** (420 kbaud, 8N1): judgeable by `BRR` and `CR1`/`CR2` reads.
  - A framing fault cannot be rendered.
- **Recommendation:** UARTs are witnessed by bytes: a fixed pattern out, an injected frame echoed. Framing is judged statically from registers, and from the tag warnings for `M0`/`TXINV`/`RXINV`. No framing fault.

### S3. USB device

- **Model** [read]: `usb1`/`usb2` are `USB.STM_USB`, a DWC2 OTG device model. `USB.USB_UART` is a CDC-ACM host that enumerates it and exposes the data endpoints as a UART.
- **Measured with third-party firmware:** a Zephyr CDC-ACM shell enumerates on `usb2`, and bytes flow both ways: prompt out, command in, answer out. Bytes reach the host side only after enumeration, once `USB_UART` has parsed CDC and CDC-Data interface descriptors, so a store cannot produce them [read]. They are a witness.
- **Our firmware** [inferred]: the generated firmware would need a USB device stack, and none is available offline today. Enumeration of *our* firmware is unmeasured.
- **Platform** [read / inferred]: the emitted platform must register `usb1`/`usb2` in `rcc` (as it already does for `spi1`), or `USBxOTGxxEN` reads 0 and the enable can't be judged.
- **Static checks a USB design needs anyway** [inferred]:
  - `D+`/`D−` on PA12/PA11, the pinout's `OTG_FS_DP`/`OTG_FS_DM` (DS12117 Rev 10, read). Their AF number and the OTG_HS pins' FS use were not read;
  - a 48 MHz kernel clock (`RCC_D2CCIP2R.USBSEL`: PLL1Q, PLL3Q or HSI48), with the frequency proven from the clock tree;
  - the 90 Ω differential pair from `4100`'s extraction;
  - VBUS sensing (PA9) or its override (`GOTGCTL.BVALOEN`/`BVALOVAL`, both tags in the model, so only static).
- **Recommendation:** USB enumeration is witnessable. The cost is a USB stack in the generated firmware. Put it after `4400`, which needs only the pins and clock to name USB in `hwdef.dat`.

### S4. Power input and regulators (proposal)

- **Parts the parts table needs** [inferred], each row cited to its datasheet:
  - **Buck from battery:** VIN min/max, VOUT, IOUT max, efficiency at load, UVLO rising/falling, switching frequency.
  - **LDO:** VIN max, VOUT, dropout at IOUT, IOUT max, PSRR at the buck's frequency.
  - **Current and voltage sense:** divider ratio and tolerance, or a sense amplifier's gain, offset and full scale, and the ADC pin's range.
- **Gates that read them** [inferred]:
  - a static check that the declared battery (cells × min/max per cell) lies inside the buck's VIN and above its UVLO;
  - the buck's VOUT minus margin ≥ the LDO's VOUT + dropout at the board's declared load;
  - the LDO's VOUT inside the MCU's VDD range;
  - the sense's full scale inside the ADC's input range.
- **The analog bridge** [read]: `--rail` solves one declared VDD droop from source resistance, load and decoupling, against BOR levels (`2301`). A regulator's input range is a datasheet comparison and needs no SPICE. A brownout margin from battery sag is a chain: battery, then buck/LDO dropout, then VDD. That chain is a later work order that feeds `--rail` from the declared parts.
- **ADC battery monitoring: out.**
  - The twin's `STM32H7_ADC` takes samples from RESD sample sources the judge supplies, so the measured voltage is the judge's own number [read]. Only the firmware's read of it could be witnessed, and nothing makes the ADC's register state a witness.
  - ArduPilot's `hwdef.dat` (`4400`) needs only the pins and scale factors, which come statically from the sense parts above.

### S5. Pin speeds and the compensation cell

- **Renode** [measured]: `SYSCFG_CCCSR` is not modelled. `EN` is dropped and `READY` reads 0. **A firmware that enables the cell and waits on `READY` hangs in the twin** (P5.2), and floods the log at about 33 M lines per simulated second. **§7's second condition holds.**
- **RM0433 §11.3.11** [read]: activation needs the CSI clock on, then `SYSCFGEN`, then `CCCSR.EN`. The probe did not enable CSI. That is irrelevant to Renode, but a silicon firmware must.
- **DS12117 Rev 10 Table 149** [read], Fmax in MHz, HSLV off:

  | OSPEEDR | 50 pF, VDD ≥ 2.7 V | 30 pF | 10 pF | 50 pF, VDD < 2.7 V | 30 pF | 10 pF | note |
  |---|---|---|---|---|---|---|---|
  | `00` | 12 | 12 | 16 | 3 | 3 | 4 | |
  | `01` | 60 | 80 | 110 | 15 | 15 | 20 | |
  | `10` | 85 | 110 | 166 | 35 | 40 | 85 | 5: compensation enabled |
  | `11` | 100 | 133 | 220 | 50 | 66 | 100 | 5: compensation enabled |

  The datasheet gives **no** figure for `10` or `11` with the cell off.
  - Note 3 defines Fmax by (tr + tf) ≤ 2/3 T, skew ≤ T/20, and duty 45–55 %.
  - The generator emits `00` for I2C and GPIO outputs and **`10` for SPI** (`board.rs` `Role::policy`), and never enables the cell.
- **Proposal** [inferred]:
  - The static check: for each line, `Fmax(OSPEEDR, VDD, C) ≥` the line's required toggle rate. For SPI SCK that is the bus clock; for a DShot output, the bit rate (600 kHz); for PWM, trivially met.
  - C is the extracted net capacitance (`4100`) plus the receiver's input capacitance, rounded **up** to the next table row (10, 30 or 50 pF). Above 50 pF the check refuses.
  - VDD is the declared rail's minimum.
  - **Speeds `10`/`11` are refused** until a ruling: the only figures for them assume the cell, and the twin cannot run a firmware that enables it.
  - The generator emits `01` for SPI, enough for every SPI part in `4200`'s table at 50 pF (60 MHz).

### S6. 2100 F24 (cold `RCC_*ENR`)

- **Enable bits** [read, `stm32h753xx.h`]:
  - **In registers already cold-written:** TIM1/8/15/16/17 and USART1/6 (`APB2ENR`); TIM2–7/12–14, USART2/3 and UART4/5/7/8 (`APB1LENR`).
  - **Outside them:** LPUART1 and SYSCFG (`APB4ENR`); OTG1/OTG2, DMA1/2 and ADC12 (`AHB1ENR`); ADC3 (`AHB4ENR`, already cold).
- **Gating** [measured]: Renode gates every peripheral registered in `rcc`, timers and USARTs included. A firmware that skips the enable gets one `Tried to write/read a locked peripheral` warning per access (`nogate`, `uart-nogate`).
- **Why F24's DMA1 case passed** [read]: `AHB1ENR` is not cold-written, so DMA1 stays enabled from Renode's reset.
- **F24's fix now:** cold-write every `RCC_*ENR` the header defines (`AHB1`–`4`, `APB1L`/`H`, `APB2`, `APB3`, `APB4`) and judge each in full. Register `usb1`/`usb2` (and any other peripheral the solver adds) in `rcc` in the emitted platform; otherwise its enable bit reads 0.
  - Size: small.
  - It belongs to the first piece that solves DMA, USB, LPUART or SYSCFG.
- **The CPU-specific `C1_*ENR` aliases:** behaviour is unread; confirm before relying on them.

### S7. The split

The schema version: `4100`'s is 2. `4200`'s build takes 3 (`spi_devices`), so a `4300` piece takes the next free one when it lands (D6).

| piece | solves | spec gains | twin witnesses / static | campaign fault | size | vs `4400` |
|---|---|---|---|---|---|---|
| **4300a — motor outputs** | timer channels on AF pins for PWM (50–490 Hz) and DShot300/600 timing; emitted `frequency` per timer from the clock tree; F24's cold `*ENR` | `motor_outputs`: label, pin (or solver-assigned), protocol `pwm`/`dshot300`/`dshot600`, rate, pulse range | **witnessed:** observer edges with virtual stamps, each rise matched to the timer's update line; period and high time. **Static:** `MOE` from its tag warning, DMA/DMAMUX for DShot, Fmax | **`motor_output_dead`**: one output's pin on the wrong AF, so no edges | M–L | before |
| **4300b — serial links** | USART/UART on AF pins; role `rc_sbus`, `rc_crsf`, `mavlink`, `gps`; baud and framing from the role | `serial_ports`: label, peripheral, TX/RX pins, role | **witnessed:** pattern bytes out; injected bytes echoed. **Static:** `BRR` from the clock tree, `PCE`/`STOP` registers, `M0`/`TXINV`/`RXINV` tag warnings | **`serial_rx_dead`**: RX on the wrong AF, so the echo is missing | M | before |
| **4300c — pin speeds** | the Fmax check; generator `OSPEEDR` from it; speeds `10`/`11` refused | none (C from extraction, VDD from the rail) | **static only** | none: a static refusal | S | before |
| **4300d — power parts** | buck, LDO and sense rows in the parts table; static input-range, dropout, VDD and full-scale checks | `power`: battery cells, buck, LDO and sense parts by name | **static only** | none | M | before; `4400` needs the sense scales |
| **4300e — USB device** | OTG FS on PA11/PA12, 48 MHz source, VBUS; CDC-ACM in the generated firmware; `usb` registered in `rcc` | `usb`: port, VBUS sense pin | **witnessed:** CDC bytes after enumeration (`USB_UART`). **Static:** the pins, the clock, the 90 Ω pair | **`usb_enum_fail`**: D+ on the wrong AF | L (a USB stack) | after; `4400` names USB from the static part alone |

- **Firmware's claim only:** none of the pieces rests on it. ADC battery monitoring would (the judge supplies the sample), so it is out, with no fault (S4).
- **Order:** `4300c` first, because it changes the SPI pins `4200` builds. Then `a` and `b` in parallel; they touch different peripherals but the same crates, so in sequence if one builder. Then `d`; `e` after `4400`.

## 5. Decisions needed

- **D1 (§7, the compensation cell).** Renode 1.17.0 has no `CCCSR`, and a firmware that waits on `READY` hangs (measured). Options:
  - **(a)** speeds `10`/`11` refused and the cell never enabled. SPI moves to `01` (60 MHz at 50 pF). No twin change. **Recommended.**
  - **(b)** enable the cell with a bounded wait, the timeout recorded. That firmware behaves differently in the twin and on silicon.
  - **(c)** model `CCCSR` in the emitted platform. `syscfg` occupies the whole page, so this means replacing `STM32_SYSCFG` (EXTI routing included) or patching Renode. Either is a model change.
- **D2 (S1, virtual time).** The CLI logs host time. Timer timing needs `log-timestamps-type = Virtual` or `Both` in the CLI's `--config`, which changes every line the witness rule compares (the timestamp prefix it strips). Change the composition, or judge timer outputs by edge count only?
- **D3 (S1, the model's conventions).** Judge period against `ARR` (the model) and say so, or against `ARR + 1` (silicon) with a known 1-tick allowance? The same question applies to the late first edge, `MOE` and `CCxP`.
- **D4 (S1, forgery).** Accept "observer edge + coincident update line + `CEN`" as the timer witness, given that a store can drive the pin (measured)?
- **D5 (S3).** USB after `4400` as `4300e`, with a USB stack in the generated firmware: vendor a crate (`usb-device` + `synopsys-usb-otg`) or hand-write a minimal CDC? The other choice is static-only USB.
- **D6 (S7).** The split as proposed, and the schema version each piece takes after `4200`'s v3.
- **D7 (S4).** ADC battery monitoring out of `4300` and its fault, as proposed; the battery-sag chain into `--rail` a later work order.
- **D8 (S2).** SBUS inversion and its 9-bit word judged only from the tag warnings; no framing fault.

## 6. What a build deletes

`spike/4300/` (`roadmap.md` §6). The tables above are the record.
