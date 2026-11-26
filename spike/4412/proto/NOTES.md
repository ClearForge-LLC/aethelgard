# AET-WO-4412 (D4 a) prototype: our own TIM1 and DMA1/DMA2, measured

PROTOTYPE, not the build. Two C# Renode 1.17.0 models, run in place of stock through the kit's replay harness
(`sh/img.sh` + `sh/replay3.sh`, unchanged), on the `arm/2` and `dshot/2` captures. Measured 2026-11-26 on Furnace, load
9-77 (other builders' runs shared the cores); every wall figure below names its load.

## The answer first

1. **Fast: dshot12 replays in 40.6 / 42.5 s with our models, against stock's 164.3 s in the same session** (load 9-12).
   That is 1.11x the arm12 replay with our models (36.5 s) and close to stock arm12 (38.2 s). After DShot starts, the cost
   is 3.8 s of wall per virtual second (stock: 24.6 s; arm with our models: 3.1 s).
2. **DShot data moves, and it decodes.** All 6,986 frames (NDTR 76, 19 bursts each) end with NDTR 0, TCIF set and EN
   cleared, 25.0-26.4 µs after EN. Every channel-frame decodes as 16 bits with a good CRC: 5,385 are packet 0 (disarmed),
   1,205 are throttle 248, and the rest are an arm ramp (54, 60, 66, 72, 78, 84, ...). Pin edges are 16 pulses per frame,
   631 ns (bit 0) and 1,261 ns (bit 1) high, 1,471 ns apart, low after each frame. ArduPilot arms over MAVLink at 8.66 s
   (`base_mode` 209), with the same bytes as stock up to 10.0 s.
3. **Equivalence with stock on arm12 holds for the final files**, but only after two rules found by measurement:
   - The model must ask the CPU to leave its translation block wherever stock does (`stockExits`). Without it, ArduPilot's
     own trajectory moves: the dma1 sequence from 4.28 s, and MAVLink bytes from 10.0 s.
   - With it, every TIM1/DMA1/DMA2/USART access (value, order, PC), every USART stamp, every LED edge and stamp, and the
     end reads are equal. Two exceptions: CCMR1/2 reads back 0x6868 against stock's 0x6060 (OCxPE with preload on;
     `preload: false` removes it), and 32 log stamps differ.
   - Our DMA alone is exactly stock, stamps included.
4. **Laziness is not proven harmless where it matters.** arm12 skips only 49 updates. On dshot12, lazy and eager runs of the
   same files differ:
   - the frame phase;
   - USART stamps from 6.186 s;
   - 4 bytes at 10.0 s;
   - the ramp samples (§5.4).

   The model's outputs are self-consistent: 0 laziness violations. The firmware's trajectory is not invariant to the
   laziness. This is the open problem.
5. **It does not fit the 8 MiB log budget as asked.** One line per burst is 7.57 MB (57 B × 132,734), plus 0.98 MB of
   per-transfer end lines, for 5.8 s of DShot. Memory: compiling the two files costs about 150 MB of RSS. Peak is
   700-797 MB against stock's 541-571 MB.

## 1. Files

| file | what | sha256 (final) |
|---|---|---|
| `models/renode/AethelgardSTM32Timer.cs` | TIM1..TIM17-shaped timer, lazy counter, UDE request, DCR/DMAR burst | `5188d4b952544f0b4e7f21fcf028b7876bed225169ba722848cf32898ff498b3` (measured as `8ae31d6a…`; the two differ in comments only, checked by stripping `//` comments and diffing; the measured copy is `~/.cache/aet-4412/proto/AethelgardSTM32Timer.measured.cs`) |
| `models/renode/AethelgardSTM32DMA.cs` | DMA1/DMA2: stock behaviour, plus request-paced streams | `79b1ebc0a6867be833eddcd5a56f6a00652afd940d4277219b551cce3785bf0a` (as measured) |

Not done (the build's): pinning in `aethelgard-twin`, `src/lib.rs` `FILES`, the README table, the emitted platform. No Renode
source is copied; stock files were read as behavioural references only.

## 2. How the models were put in the replay (exactly)

Nothing in the kit or the repo changed. Under `~/.cache/aet-4412/proto/` (mode 700; `/cache/proto` in the container):

1. `platforms/*-<variant>.repl` (`mkplat.py`): copies of Renode 1.17.0's `stm32h753.repl -> stm32h743.repl -> stm32h7.repl`
   (sha256 `13c5571e…`, `86b2c2c5…`, `b85cb4af…`, equal to the image's `/opt/renode/platforms/cpus/`), each `using` pointed
   at the next copy, and in the `stm32h7` copy only: `timer1` typed `Timers.AethelgardSTM32Timer` with
   `UpdateDmaRequest -> dmamux1@15` (and the variant's parameters); `dma1`/`dma2` typed `DMA.AethelgardSTM32DMA` with
   `dmamux: dmamux1`, `firstMuxChannel: 0|8`, `pacedRequests: "15"`. Every other attribute (interrupts, AF routes,
   the emitted platform's `frequency: 200000000`) is the stock file's. A typed re-declaration in a later file is refused by
   Renode (`E02: Variable 'timer1' was already declared`), hence the copied chain.
2. `cap/<name>/` (`mk.sh BASE NAME KIND`): a copy of the kit capture; the two `.cs` files copied into `tool/models/`;
   in `argv.bin`'s `-e`, `include @…/AethelgardSTM32Timer.cs; include @…/AethelgardSTM32DMA.cs` after the CLI's own
   `ICM42688P.cs` include, and `echo "AET-PROTO-STATS"; sysbus.timer1 Stats` before the final `peripherals; quit`;
   in `tool/platform-booted.repl` the one line `using "platforms/cpus/stm32h753.repl"` -> the variant's copy.
3. Run: `cd ~/aethelgard-4412 && CPUS=0-3 MEM=8G spike/4412/sh/img.sh tools 'PREHOOK="…" timeout 900 bash /spike/sh/replay3.sh /cache/proto/cap/<name> /cache/proto/rep/<run>'`.
   The batches are `proto/batch{1,2,3,4}.sh`; analysis `compare.py`, `decode.py`, `edges.py`, `mav.py` (ArduPilot image).

## 3. Design

### 3.1 The timer (`AethelgardSTM32Timer`)

**Two rule sets, kept apart.** The twin's committed boards are judged by stock `STM32_Timer`'s conventions
(`aethelgard-twin` `src/timer.rs`: a period is `ARR` ticks realised in whole ns rounded up, the rise at the update and the
fall `CCR` ticks later, `MOE`/`OCxPE` tags), so every register, interrupt and edge rule that exists in stock is kept as
stock behaves, and labelled `[stock]` (or `[stock != RM pN]` where RM0433 differs). Only what stock lacks is written from
RM0433: `UDE` and the update DMA request, `DCR`/`DMAR` bursts, and (switchable) `OCxPE` preload.

**Time, exactly as Renode's clock entries count.** A clock entry holds whole ticks plus an exact fraction, reaches its
limit at `ceil(((P - V)·1e9 - r) / f)` ns, drops the fraction there, and is processed at that exact nanosecond
(`BaseClockSource.Advance` splits an advance at the nearest limit, so a CPU overshooting a limit by part of an instruction
does not move it). The model keeps that arithmetic in software (`struct Entry`) for the counter (`f = 200 MHz / (PSC+1)`,
integer Hz like stock's divider) and for each channel's compare (restarted from the counter as stock's `UpdateTimer`
does; a limit write at or below the count fires at once, as a clock entry's limit setter does), and asks the clock source
for ONE event (a 1 GHz one-shot `LimitTimer`, the house pattern of `ICM42688P.cs`) at the earliest instant something
observable can happen. Limits reached at the same instant are processed as one clock-source update: all decided first,
then the counter's handler, then the channels' in index order (stock's entry order).

**What is lazy.** An update event is observable only through: the update interrupt (`UIE`); an
update DMA request not already pending (`UDE`); a channel that acts at the update or its compare (`CCxIE`, or `CCxE` with an
active compare mode 1-3/6/7 and `CCR > 0`); a pending preload. If none holds, no event is armed, and the skipped updates
are applied when the firmware next touches the timer (every bus access first catches up: `CNT`, `UIF`, `RCR`'s count,
`OPM`'s `CEN`, `ARR`'s reload all come out as the per-update processing would have left them, because that same
processing runs, late, in order, with each skipped update's own instant). The model's own claim is checked: any GPIO,
interrupt or request change produced during catch-up is counted (`Stats: violations`) and warned; it was 0 in every run.
Between DShot frames the request stays pending (RM p.695) and every `CCR` is 0, so the 680 kHz counter costs no event; inside
a frame each bit is one wake.

**Whether laziness can differ: measured, not argued** (§5.3-5.4). It does not differ in the model's own outputs. It CAN
differ in the firmware's trajectory, because Renode's CPU is affected by where it is stopped and asked to leave its blocks:
- arm12: the final files equal stock in every access, USART byte and stamp, LED edge and stamp, and end read. But only
  49-50 updates were skipped there: stock's reset read-back `CCR` = `initialLimit` (0xFFFF) keeps the channels
  "observable" (PWM, `CCxE`, `CCR` > 0) until ArduPilot writes `CCR` 0 at 5.19 s.
- dshot12 (3.8 M updates skipped): lazy and eager (`lazy: false`) runs of the same files differ (§5.4). The eager run is
  confounded three ways, each a hypothesis, not a measured cause: it adds a stop at every limit; every wake re-arm is a
  `LimitTimer` setter, so it asks for a CPU return in every event; and with a report at every update the clock-source
  instant a CPU write is taken at (stock's reference, which lags the CPU to its last report) is fresher.

**The CPU-exit rule (found by measurement, §5.3).** Stock's register writes call `LimitTimer` setters, and each setter
asks the CPU to leave its translation block (`RequestReturnOnCurrentCpu`). Where the CPU leaves its blocks changes when
ArduPilot's interrupts are taken, so a model that makes the CPU leave at other places moves the firmware's own trajectory.
`stockExits` (default true) asks for a return on exactly the writes where stock touches a clock entry (`CR1`, `SMCR`,
`DIER`, `EGR`, `CCER`, `CNT`, `ARR`, `CCRx`, `PSC` when it changes), never on a `DMAR` burst (stock never sees one). The
model's own wake re-arm also asks for a return (it is a `LimitTimer`). That is an open risk, measured once: with the
preload first initialised to 0, enabling `OCxPE` by a `CCMR` write left a preload pending, re-armed the wake on that write
(where stock asks for no return), and the arm12 run diverged from 4.28 s (`arm-aetx-eq`); starting the preload at
`initialLimit` removed it (`arm-final-eq`). Any write that moves the wake target where stock asks for no return (a `CCMR`
mode change while running, for one) can do the same.

**DMA side (RM0433).** `UDE` = 1 at an update event (and at `UG` with `URS` = 0, RM p.1589) sets `UpdateDmaRequest`, a
level held until served (RM p.641, p.695). Each `DMAR` access goes to `(DBA + index)·4` (RM p.1617), `index` runs 0..`DBL`
(RM p.1616: `DBL` = 0 is one transfer), and the access that completes the burst clears the request. One Noisy line per
completed burst: `burst 0x34: 0 6 3 6`. Clearing `UDE` withdraws an unserved request; a `DCR` write restarts the index.

**Preload (`preload`, default true).** RM p.1603: with `OCxPE` = 1 a `CCR` write goes to the preload register and is
loaded into the active one at each update event (and `UG`). This is what makes DShot's pin edges right (bit k, written at
update k, is the pulse of period k+1). `preload: false` is stock's rule (`OCxPE` a tag, a `CCR` write acts at once). Before
the first `CCR` write the preload equals the active value, stock's reset read-back `initialLimit` (`[stock != RM p.1610]`,
RM's reset value is 0).

**Left out, warned by name:** down/centre-aligned counting, slave modes, input capture (stock has capture, encoder and
gated/reset/trigger slave modes: not needed by ArduPilot's or our firmware's TIM1, but stock-equivalence for a firmware
using them is NOT established), `CCxDE`/`TDE` requests, CH5/6, break and `MOE` gating.

### 3.2 The DMA (`AethelgardSTM32DMA`)

`STM32DMA` is `sealed`, so this is a replacement, not a subclass. Per stream, decided when `EN` is written 1:

- **Paced** if memory-to-peripheral and its DMAMUX1 channel (`firstMuxChannel` + stream, RM p.696: DMA2 stream k is
  channel 8 + k) selects one of `pacedRequests` (read from `DMAMUX1_CxCR.DMAREQ_ID`, RM p.709, by a direct register read,
  so a logged DMAMUX shows nothing extra). It moves one item (`PSIZE`) at a time while the request line is high, so a
  timer burst ends when the timer drops its request (RM p.641); `NDTR` counts down per item (RM p.641, p.665); at 0 it
  sets `TCIF` and clears `EN` (RM p.654, p.665); disabled by software before 0 it sets `TCIF` (RM p.654, 15.3.16). A
  request already high at `EN` is served at `EN` (RM p.695: pending until served); a paced request arriving while the stream
  is off is held, not warned. One Noisy line per ended transfer, with the `EN` and end instants in ns.
- **Stock** otherwise: memory-to-peripheral and memory-to-memory move the whole `NDTR` at `EN` with no request, peripheral-
  to-memory one item (or the FIFO threshold) per rising request, `TCIF` at 0, `EN` left set; the same warnings. ArduPilot's
  SPI and USART streams are all here.

`pacedRequests` must list only requests a timer of ours drives (here `15`, TIM1_UP), or a stream would wait forever on a
stock timer that raises nothing. The build would emit it from Table 124 (p.697) for the timers it retypes.

### 3.3 Routing

`timer1.UpdateDmaRequest -> dmamux1@15`; Renode's `STM32_DMAMUX` forwards a level change on input 15 to every channel whose
`DMAREQ_ID` is 15, and `dmamux1 [8-15] -> dma2@[0-7]` (stock `stm32h7.repl`) makes channel 9 DMA2 stream 1. ArduPilot writes
channel 9 = 15 before `UDE` (measured: `UDE` 6.178587 s, channel 9 = 15 from 6.178563 s) and rewrites the same value per
frame. Stock DMAMUX propagates changes only, so a request asserted before its channel is pointed at it would not be seen
(not hit by ArduPilot; open, §6).

## 4. RM0433 Rev 9 citations (held text, page footers)

| behaviour | page | where |
|---|---|---|
| DMA burst mode: "multiple DMA requests upon a single event", DMAR redirected | 1586-1587 (38.3.28) | timer |
| DCR: DBA, DBL; "00000: 1 transfer" | 1615-1616 (38.4.21) | timer |
| DMAR: `(TIMx_CR1 address) + (DBA + DMA index) x 4`, index 0..DBL | 1616-1617 (38.4.22) | timer |
| CR1 ARPE | 1588 | timer ([stock] reload kept) |
| CR1 OPM, URS ("1: Only counter overflow/underflow generates an update interrupt or DMA request"), UDIS | 1589 | timer |
| DIER UDE | 1595 | timer |
| SR UIF (rc_w0; set by UG only with URS = 0) | 1596-1597 | timer ([stock != RM]: stock sets it on UG only with UIE) |
| EGR UG | 1599 | timer ([stock != RM]: stock acts on any EGR write) |
| CCMR1 OC1PE: "Read/Write operations access the preload register ... loaded in the active register at each update event" | 1603 | timer |
| PSC: "loaded in the active prescaler register at each update event" | 1609 | timer ([stock != RM]: at once) |
| CCR1 reset value 0x0000 | 1610 | timer ([stock != RM]: reads initialLimit) |
| request held until acknowledged; NDTR post-decrement | 641 (15.3.4) | DMA, timer |
| transfer completion: NDTR 0 in M2P; "EN ... cleared by Hardware" | 653-654 (15.3.15) | DMA |
| suspension: TCIF set when disabled before the end | 654 (15.3.16) | DMA |
| SxCR EN "cleared by hardware on a DMA end of transfer"; SxNDTR | 665 | DMA |
| LISR/HISR flag layout | 659-660 | DMA |
| "The DMA request is pending until served" | 695 (17.1) | DMA, timer |
| DMAMUX1 channels 8-15 to DMA2 streams 0-7 | 696 | DMA |
| Table 124: TIM1_UP = 15 (TIM2 22, TIM3 27, TIM4 32, TIM5 59, TIM8 51, TIM15 106, TIM16 110, TIM17 112) | 697 | platform |
| DMAMUX1_CxCR DMAREQ_ID[6:0] | 708-709 | DMA |

Stock conventions kept that RM0433 contradicts (each would move committed boards' witnesses if changed; none is new here):
period `ARR` ticks (RM: `ARR + 1`), so DShot600's bit at `PSC` 0x29/`ARR` 7 is 1,471 ns (680 kHz), not 1,680 ns; the
prescaler frequency is integer Hz (4,761,904 Hz); `PSC` at once; `UDIS` skips `UG` entirely; any `EGR` write is a `UG`.

## 5. Measurements

All replays: `CPUS=0-3 MEM=8G spike/4412/sh/img.sh tools '[PREHOOK="…"] timeout 900 bash /spike/sh/replay3.sh /cache/proto/cap/<cap> /cache/proto/rep/<run>'`
(the stock rows replay the kit captures `/cache/cap/{arm,dshot}/2` themselves). Wall and peak come from `replay3.sh`'s
`cost.txt`; s/vs is the wall per virtual second from 6 to 11 s (`stats.txt`). Models: the final files are batches 4-5
(`8ae31d6a…`/`79b1ebc0…`). Batches 2-3 ran `2f6e81d5…`/`79b1ebc0…`: no `inBurst` exit suppression, preload starting at 0,
`stockExits` default false. Batch 1 ran `8f8b7b34…`/`3681586d…`: no diagnostic switches, no `TraceEdges`. Model files per
capture are in `cap/<cap>/tool/models/`.

### 5.1 Cost (a), (d)

| run | models | wall s | peak KiB | load (1 min) | s/vs 6-11 |
|---|---|---|---|---|---|
| `dshot-stock-p` (batch 4) | stock | **164.27** | 540,888 | 12.1 | 24.6 |
| `dshot-final-p` (batch 4) | final | **40.55** | 796,920 | 11.1 | 3.8 |
| `dshot-final-p2` (batch 4) | final | 42.53 | 706,432 | 9.0 | 4.0 |
| `dshot-final-noexit-p` (batch 4) | final, `stockExits: false` | 40.79 | 797,172 | 8.9 | 3.8 |
| `dshot-final-eager-b` (batch 5) | final, `lazy: false`, burst log | 67.74 | 730,668 | 13.3 | — |
| `arm-final-p` (batch 4) | final | **36.51** | 703,068 | 10.9 | 3.1 |
| `arm-stock-p` (batch 3) | stock | 38.15 | 543,080 | 17.6 | 3.3 |
| `arm-inc-p` (batch 3) | stock platform, both files compiled | 38.78 | 697,320 | 22.1 | 3.4 |
| kit `dshot-p` / `arm-p` (RESULTS §1) | stock | 169.81 / 39.21 | 541,292 / 540,696 | 8.8 / 10.1 | — |
| `dshot-aetx-p` (batch 3) | `2f6e81d5…`, exits on bursts too | 362.06 | 797,280 | **76.4** | 48 |
| `dshot-aetx-s` (batch 3) | same file, TraceEdges | 43.27 | 731,184 | 31.6 | 4.2 |

- The 362 s row ran while two other builders' Renode processes shared cores 0-3 (load 76). The same file's next run took
  43 s, and the final file with and without exits takes 40.6-42.5 s. Read that row as contention, not cost.
- **Memory.** `arm-inc-p` creates no model of ours and still peaks 154 MB above `arm-stock-p` (VmHWM at t = 0: 688,256
  against 542,576 KiB). So the cost is Renode compiling two more `.cs` files, not the models running.
- **CI projection, arithmetic only.** At 1.5-3.5x slower runners, dshot12 is 61-149 s and arm12 55-134 s. Both cross 120 s
  at the slow end, and arm12 does so with stock models too. The replay is not the CLI: the CLI's own arm runs measured
  41-46 s (RESULTS §1).

### 5.2 DShot data (b): `dshot-final-b` (bursts) and `dshot-final-s` (exact edges, `TraceEdges`)

`decode.py`: frames 6,986, bursts 132,734, 19 per frame. Paced ends: all `(NDTR 0, 76 items, 19 requests, NDTR 0)`. EN->TCIF
25.0-26.4 µs. That is one burst at EN, served from the pending request, then 18 updates of 1,471 ns, at the counter's phase.

Each burst's four words are CCR1-4 (DBA 0x0D = 0x34, DBL 3). CCR values seen: 0 ×83,832, 3 ×399,352, 6 ×47,752 (bit 0 =
3 ticks, bit 1 = 6, `DSHOT_BIT_0/1_TICKS_DEFAULT`). The frame layout is 1 pre-zero, 16 bits, 2 post-zeros
(`dshot_pre`/`dshot_post`). Decoded per channel (11-bit value, telemetry bit, CRC = `(v ^ v>>4 ^ v>>8) & 0xF`):

```
0x0000 thr=0 tel=0 crc=0 ok x5385; 0x1F0E thr=248 tel=0 crc=E ok x1205; 0x06CA thr=54 crc=A ok x6; 0x078F thr=60 crc=F ok x6;
0x084C thr=66 crc=C ok x6; 0x0909 thr=72 crc=9 ok x6; 0x09C5 thr=78 crc=5 ok x6; 0x0A82 thr=84 crc=2 ok x6 ...   (malformed: 0)
```
A first frame, as logged:
```
timer1: burst 0x34: 0 0 0 0      <- at EN (the pending request)
timer1: burst 0x34: 3 3 3 3      x16  (packet 0x0000, disarmed)
timer1: burst 0x34: 0 0 0 0      x2
dma2: stream 1 paced end (NDTR 0): 76 items on 19 requests, NDTR 0, TCIF; EN … ns, end … ns
```
The first non-zero packet, 0x0642 (throttle 50), is at 10.67 s. Edges (`edges.py`, each channel): 16 pulses per frame,
high 631 ns ×99,837 / 1,261 ns ×11,938 / 630 ns ×1 (the first DShot pulse, at 6.1806 s), rises 1,471 ns apart, final
level 0. One more rise, isolated, at 0.1204 s, is the PWM output going high under stock's CCR read-back of 0xFFFF; it was
not followed further.

With stock's rule (`preload: false`), the first prototype run (code before preload existed; console since overwritten, so
no exact-edge measurement) logged each pulse's rise at the update after a non-zero CCR and its fall at the new CCR. That is
a CCR write acting at once. By that rule the rise follows the previous bit, the width follows this one, and the line is
left high after a frame's last bit. Preload is on by default for that reason. The "left high" is inferred from the rule,
not measured.

Arming: `mav.py` (pymavlink in the ArduPilot image) on `dshot-aet-p` shows `COMMAND_ACK 400 result 0` at 8.66 s and
`HEARTBEAT base_mode 209` at 8.75 s. `dshot-final-p`'s USART1 bytes equal stock `dshot-stock-p`'s up to #8059 (10.0036 s),
so they hold the same ACKs.

### 5.3 Equivalence on arm12 (c): `compare.py` over each pair; every run logs LEDs, TIM1, DMA1, DMA2 the same way

PREHOOK: `logLevel -1` on the four motor LEDs; `sysbus LogPeripheralAccess` on timer1, dma1, dma2. The CLI's own argv already
logs usart1-3.

| B against `arm-stock-eq` | TIM1/DMA/USART accesses (value, order, PC) | USART stamps | LED edges | end reads (last block, 63 lines) |
|---|---|---|---|---|
| `arm-stock-eq2` (stock again) | equal | equal | equal + stamps | equal |
| `arm-dmaonly-eq` (our DMA, stock timer) | equal, **stamps too** | equal | equal + stamps | equal |
| `arm-timonly-eq`, `arm-nopre-eq` (our timer, no exits) | **dma1 differs from #16,246 (4.28 s); USART1 bytes from #7,615 (10.00 s)** | differ from 3.92 s | same sequence, all −193 µs | instructions +223,180 |
| `arm-eager-eq` (no laziness, no exits) | differs, same point (4.28 s) | differ | same sequence, −135 µs | instructions differ |
| `arm-eagerx-eq` (no laziness, stock exits) | equal (8 TIM1 stamps) | equal | equal + stamps | equal |
| `arm-lazyx-eq` (lazy, stock exits, `preload: false`) | equal (8 TIM1 + 12+12 DMA stamps) | equal | equal + stamps | equal |
| `arm-aetx-eq` (lazy, exits, preload from 0) | **differs from 4.28 s** | differ | | CCMR 0x6868 |
| **`arm-final-nopre-eq`** (final file, `preload: false`) | equal (8 TIM1 + 12+12 DMA stamps) | equal | equal + stamps | **equal, 0 lines** |
| **`arm-final-eq`** (final file, defaults) | equal (8 TIM1 + 12+12 DMA stamps) | equal | equal + stamps | **2 lines: CCMR1, CCMR2 0x6868 against 0x6060** |

- Every "stamps differ" in the equal rows is a log stamp only. The value, order and PC are equal, the USART stamps are
  equal, and so is `cpu ExecutedInstructions` at the end. The TIM1 ones are the setup writes at 0.1004 s (up to −6.4 µs);
  the DMA ones are 12 accesses at 5.6401 s (−5.1 µs).
- **Warnings** (grep diff; `compare.py`'s own tally printed nothing even where they differ):
  - stock's 8 TIM1 lines are 2 each of "Unhandled write" for CCMR1 (OC1PE), CCMR2, BDTR (MOE) and 0x54;
  - ours has 2 "write to unmodelled offset 0x54";
  - stock also has 1 "Thread has been woken up 2 times";
  - every other warning text and count is equal.
- **Stamps.** Renode's log stamps are the time of the CPU's last report, not the access instant, so they are coarser than a
  DShot bit. The DMA end lines and `TraceEdges` therefore carry exact instants.
- **What this isolates.** Stock is deterministic, and our DMA alone is stock. With our timer, what moves ArduPilot is
  whether the CPU is asked to leave its block on TIM1 writes. Neither stops nor laziness moves it here, but only 49 updates
  were skipped (§3.1).

### 5.4 Lazy against eager on dshot12 (same final files; `dshot-final-b` against `dshot-final-eager-b`)

- USART1 and USART2 bytes are equal (8,306 and 430), but their stamps differ from 6.186 s by up to ±2.47 ms.
- USART3 differs in 4 bytes at 10.0036 s.
- `ExecutedInstructions`: 0x182F9C84 against 0x18305D92.
- Frames 6,986 against 6,986, and bursts 132,734 against 132,734, but EN->TCIF is 25.0-26.4 µs lazy against 26,470-26,478 ns
  eager.
- Decoded ramp samples: 54/60/66/72/78/84 lazy against 52/60/64/72/78/82 eager. The packet counts for 0 and 248 are equal.
- Eager's wakes: 4,069,532, all updates woken.

Against stock dshot12 (`dshot-stock-p`), the final files give the same USART bytes and stamps until 6.186 s (DShot starts
6.18 s). After that, end reads differ: DIER 0x100 (UDE read back; stock reads it as a tag, 0), CCMR1/2 0x6868, PC and
instruction count. Stock has no reference for the DShot path itself.

### 5.5 Log bytes (4)

| line | count (dshot12) | bytes |
|---|---|---|
| `timer1: burst 0x34: w w w w` (one per burst, raw words) | 132,734 | 7,565,838 (57 B) |
| `dma2: stream 1 paced end (…); EN … ns, end … ns` (one per transfer) | 6,986 | 975,854 (140 B) |
| total added (console `dshot-final-b` 149,307,166 B against `dshot-final-p` 140,765,507 B) | | 8,541,659 |

That is about 1.47 MB per virtual second of DShot, over an 8 MiB budget for the 5.8 s here, before anything else the
CLI logs. An ESTIMATE, not run: one line per transfer carrying its 76 raw words, about 210 B each, would be about 1.5 MB.
The ns fields on the end line add about 30 B of each 140.

## 6. Open problems, and what this did NOT establish

**Not established:**
- **That laziness is harmless at scale.** No stock reference skips many updates: arm12 skips 49. On dshot12, lazy and eager
  runs of the same model differ in the firmware's trajectory (§5.4). The cause is not isolated. Candidates: the stale
  clock-source instant a CPU write is taken at (stock's own reference), the CPU-return requests of the wake re-arm, and
  the stops themselves.

  A diagnostic NOT run: a scratch variant that syncs CPU time at a paced EN and re-arms through a bare `ClockEntry` (no
  return request), lazy against eager. If EN->TCIF converges, the frame-phase part is the stale write instant and is
  fixable. Whether the whole trajectory can be made independent of stops and exits in Renode 1.17.0 is open.
- **The exit mimicry.** It is per-register, read from the stock source, and checked only on arm12. Known mismatches: an
  EGR write with UDIS = 1, and an SMCR write of SMS 7 (stock asks for no return; ours does). A wake re-arm on a write where
  stock asks for none can move the firmware (measured once: the `arm-aetx-eq` CCMR case, §3.1).
- **Scope.** Only `timer1` was retyped, and `pacedRequests` is "15". TIM2-5, 8 and 15-17 were not run. Neither was a
  generated (non-foreign) committed board, nor any input-capture, encoder or slave-mode user, which stock models and ours
  does not.
- **The DMAMUX holds levels as changes.** A timer request asserted before its DMAMUX channel is pointed at it is never seen
  (stock `STM32_DMAMUX` propagates changes only). ArduPilot points channel 9 first, so it is not hit. A build might need
  its own DMAMUX or a re-assert, which would be a third model (scope).
- **Timing conventions.** DShot bit timing follows stock's convention (`ARR` ticks, 1,471 ns), not silicon's 1,680 ns. An
  ESC-timing judge would need the `ARR + 1` ruling.
- **Log budget.** The per-burst line does not fit (§5.5). The format is the architect's call.
- **Memory.** Compiling two more `.cs` files costs about 150 MB. Whether one compilation unit would cost less was not tried.
- **Trace gaps.** `TraceEdges` does not see falls made by a timer reset (Reset unsets the pins directly).
- **CI.** The projection is arithmetic, not a CI run.

**Decisions for the architect (forks, not averaged):**
- **D-a, preload.** On (RM p.1603; DShot edges right; arm12 end reads differ in CCMR's OCxPE bit) or off (stock-exact on
  arm12; DShot edges wrong).
- **D-b, log.** Per burst (asked; 7.6 MB) or per transfer (estimated 1.5 MB), and whether the end line keeps its ns fields.
- **D-c, the exit rule.** Accept mimicking stock's CPU-return pattern as part of "never different", or define equivalence
  at the level of verdicts and decoded outputs. Instruction-level identity with stock is not a property laziness can
  promise on the DShot path, as measured.
- **D-d, conventions.** Whether the DShot judge uses stock's `ARR`-tick period or silicon's `ARR + 1`.

**Reduction notes for the build:** `ckd` is stored and never read. The `lazy` switch and `TraceEdges` are prototype
evidence tools. `Stats` is likewise.
