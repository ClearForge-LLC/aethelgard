# AET-WO-4300a FEEDBACK — motor outputs

**Builder:** Claude (strong tier), worktree `~/aethelgard-4300a`, branch `wo/aet-4300a-motor-outputs` from `main` `50d7a6f`.
**Work order:** `docs/work-orders/AET-WO-4300a.md` as on `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #136); identical to `main`'s copy at `50d7a6f`.
**Cadence:** build, with one measured stop (§1.2, pre-ruled at the `4400` ruling D1) and the off-board-net item (§1.8).

The predictions in §1 were committed before any container run, any firmware build for a probe, and any baseline run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source. Renode 1.17.0 at renode `ab721d88` / Infrastructure `066a7f13` (`~/.cache/aet-2301/src`); RM0433 Rev 9 (sha256 `672a9433…a6b0`) and DS12117 Rev 10 (`607b9ead…c032`) in `~/.cache/aet-2301/ds/`; ES0392 Rev 16 in `~/.cache/aet-ds/`; this repository at `50d7a6f`.
- **[measured]**: run in the pinned image (`container/IMAGE`) under rootless podman inside a capped user scope (RULE3, `~/.cache/aet-bus/RULE3.md`), caps read back.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the plan

Each item is **[read]**.

- **Renode has no configuration key for the quantum.** `ConfigurationManager.Instance.Get` is called for `general`, `monitor`, `plugins`, `translation`, `file-fetcher`, `termsharp` and `wireshark` keys only; none names a quantum. The quantum is `TimeSourceBase.DefaultQuantum = TimeInterval.FromMicroseconds(100)` (`TimeSourceBase.cs` L814), changed only by the monitor's `emulation SetGlobalQuantum`. D2's "the quantum stated as a `--config` key" cannot be done as written; the composition states it as a monitor command at Renode's own value instead (finding F1 below).
- **Time is counted in nanoseconds.** `TimeInterval.TicksPerNanosecond = 1` (`TimeInterval.cs` L292). The virtual log prefix is `[hh:mm:ss.nnnnnnnnn] ` (`TimeInterval.ToString`, L241–248; `LogEntry.cs` L78), 21 bytes, where the host prefix the judge strips today is 16.
- **The CPU runs at 100 MIPS** (`aethelgard_twin::boot::MIPS`), so one instruction is 10 ns. The spike's timer stamps moved in 10 ns steps although its ticks were 4 ns multiples: the stamp grain is the instruction (inferred, P7).
- **The timer model** (`Timers/STM32_Timer.cs`):
  - every channel's output is set at the update event (`LimitReached`, L50–83, PWM mode 1 `Set`) and cleared by the channel's compare timer (L105–124, PWM mode 1 `Unset`), whose limit is `CCRx`;
  - the update limit is `ARR` (L59 `Limit = autoReloadValue`), so a period is `ARR` ticks, where silicon counts `ARR + 1`;
  - `CEN` starts the counter at 0 (L143–151), so the first update, and the first rise, is a period after `CEN`;
  - `PSC` is stored as `Divider = PSC + 1` (Prescaler register), `ARR` and `CCRx` are stored;
  - tags (stored nowhere, read as reset): `CR1.CKD`; every `CR2` field but `TI1S`; `CCMRx.OCxFE`, `OCxPE`, `OCxCE`; `CCER.CCxNE`; every `BDTR` field, `MOE` included (L377–386); `DIER`'s DMA enables;
  - `OCxM` is 3 bits (`CCMRx` bits 4–6); silicon's `OCxM[3]` (bit 16 / 24) is reserved in the model;
  - `EGR.UG` (L265–295) zeroes the counter and logs `IRQ pending` when `UIE` is set and `URS` clear; it does **not** touch a channel's output;
  - `OPM` stops the counter at the first update (L57); `RCR` changes only how often the update interrupt is raised (L84–95), never the output.
- **The stock platform** wires each timer channel to its AF pins (`stm32h7.repl` L583 on, e.g. `timer1: 0 -> gpioPortA#08@01 | …`). Renode's channel `k` lists both `TIMn_CH(k+1)` and `TIMn_CH(k+1)N` pins (`timer1: 0` includes `PA7` and `PB13`, which are `TIM1_CH1N`). Every timer runs at the fixed `frequency: 250000000`.
- **The reset clock** (RM0433 Rev 9 §8.7):
  - `RCC_CR` reset `0x0000 0001` (§8.7.2): `HSION` set, `HSIDIV[1:0]` = `00`, so `hsi_ck` is 64 MHz (§8.5.2, "8, 16, 32 or 64 MHz … controlled by the HSIDIV");
  - `RCC_CFGR` reset `0x0000 0000` (§8.7.7): `SW` = `SWS` = `000` (HSI), `TIMPRE` = 0;
  - `RCC_D1CFGR` reset `0x0000 0000` (§8.7.8): `D1CPRE`, `HPRE`, `D1PPRE` = 0, each "not divided";
  - `RCC_D2CFGR` reset `0x0000 0000` (§8.7.9): `D2PPRE1`, `D2PPRE2` = 0;
  - `RCC_D3CFGR` reset `0x0000 0000` (§8.7.10): `D3PPRE` = 0;
  - Table 57 (§8.5.6, p. 351): with `D2PPRE1`/`D2PPRE2` = `0xx` and `TIMPRE` = 0, `rcc_timx_ker_ck` = `rcc_timy_ker_ck` = `Frcc_hclk1`. So every timer kernel clock at reset is 64 MHz (APB1 and APB2 alike).
- **The enable registers** (RM0433 Rev 9 Tables 66–74): `AHB3ENR` 0x0D4, `AHB1ENR` 0x0D8, `AHB2ENR` 0x0DC, `AHB4ENR` 0x0E0, `APB3ENR` 0x0E4, `APB1LENR` 0x0E8, `APB1HENR` 0x0EC, `APB2ENR` 0x0F0 reset `0x0000 0000`; **`APB4ENR` 0x0F4 resets to `0x0001 0000`** (`RTCAPBEN`, `stm32h753xx.h` `RCC_APB4ENR_RTCAPBEN_Pos (16U)`). The `RCC_C1_*ENR` aliases (0x134–0x154) are a dual-core view: the H753 header's `RCC_TypeDef` defines none of them, so the cold write never touches them.
- **Renode's RCC** (`Miscellaneous/STM32H7_RCC.cs`): `SWS` reads the `SW` it was given (L226), `HSIDIV` is a stored field (L148), `HSIDIVF` reads 1 (L158), and `HPRE`, `D1PPRE`, `D1CPRE`, `D2PPRE1`, `D2PPRE2`, `D3PPRE` are stored fields (L241–264), each starting at 0.
- **The off-board net (§1.8).** `4300f`'s S2 answer is in this builder's inbox (`~/.cache/aet-bus/4300a.md`, 03:40Z, measured on PR #134): a net of one MCU pin with a local label and no other part fails `check erc` (kicad-cli exit 5, `isolated_pin_label`, one per net), and passes DRC. A motor output is exactly such a net until a connector part exists. Per the kickoff (item 4), the board side waits for `4300f`'s build; §1.8's STOP applies only if it has not landed when the board side is reached.
- **Versions.** `main` is at specification 3, topology 4, trace 7. This work order takes specification 4, topology 5, trace 8, once each. `4210`'s build also bumps the specification and trace versions; whoever merges second renumbers (kickoff item 4).

## 1. Predictions (committed before any run)

Probe plan, so the predictions have something to point at. Probes are built `4200`'s way (the spike's `fw.py`, from `a96b977`): the committed reference firmware with raw register code inserted before the boot word, built offline in the pinned image. Runs use the CLI's composition (complete `--config`, synchronous logging, `logLevel -1` per witness source) unless a row says otherwise. The harness lives in `~/.cache/aet-4300a/probe` and is not committed.

### P1. The reset clock (§1.2, a §7 stop)

| id | probe | prediction |
|---|---|---|
| P1.1 | After the CLI's cold writes, the firmware reads `RCC_CFGR`, `RCC_D1CFGR`, `RCC_D2CFGR`, `RCC_D3CFGR` and `RCC_CR` before any timer write | `CFGR` = 0 (`SWS` = HSI, `TIMPRE` = 0); `D1CFGR` = `D2CFGR` = `D3CFGR` = 0; `RCC_CR.HSIDIV` = 0 (64 MHz). `RCC_CR` as a whole word is not predicted (Renode computes the ready flags). **§1.2 holds.** |
| P1.2 | The same reads by the monitor after a rendered `machine Reset` (the droop's reset rendering) | The same values: Renode's reset puts each stored field back to 0. |

### P2. Forgery (§2 acceptance, `4300` D4)

| id | probe | prediction |
|---|---|---|
| P2.1 | `ug-bsrr`: TIM1 configured as for PWM but `CEN` never set, `DIER.UIE` set, PA8 in AF1; in one burst of code, `EGR.UG` then a `BSRR` set of PA8, repeated with a delay loop | `[NOISY] timer1: IRQ pending` and the observer's `LED state changed to True` share one stamp each time, and every stamp is on the 100 µs grid. A store-made edge pair is indistinguishable from a timer's update edge by its stamp alone, but every one of its edges is on the grid. |
| P2.2 | `bsrr`: `BSRR` edges alone, PA8 in AF1, no timer | Edges on the grid only, no `IRQ pending`. |

### P3. The virtual-time composition (D2, a §7 stop)

| id | probe | prediction |
|---|---|---|
| P3.1 | `log-timestamps-type = Virtual` and `emulation SetGlobalQuantum "0.000100"` in the CLI's composition, everything else unchanged, against `main`'s binaries on the reference's campaign and seeded set | **Every verdict unchanged**: the judge strips the stamp as it strips the host stamp today, and no record holds one. Every trace byte-identical except where a record names the composition, if any does. Stating the quantum at Renode's default changes nothing. |
| P3.2 | Timer-caused lines under virtual stamps | Exact time in 10 ns steps, as the spike measured (`4300` P1.2). |

### P4. `mcu.hse_hz = 0` frees PH0/PH1

| id | probe | prediction |
|---|---|---|
| P4.1 | Solve Sentinel-01 and every committed fixture with PH0/PH1 free | **No solved assignment changes.** Hand pins are untouched, the solver tries pins in key order, and a chip select takes the first free pin in key order (`PA0` today); `PH0`/`PH1` sort after every pin a solved signal uses now. |

### P5. DShot in the twin

| id | probe | prediction |
|---|---|---|
| P5.1 | `dshot4`: four channels of one timer at the DShot600 solution for 64 MHz (`PSC` 0, `ARR` 106, `CCR` 40), 10 ms of virtual time | The twin's period is `106 × 4 ns` = 424 ns (`ARR` ticks at 250 MHz), high `40 × 4 ns` = 160 ns. About 23,600 periods per channel, 189,000 observer lines in 10 ms. Wall time several seconds: the timer events, not the log, dominate (the spike's one channel at 1.664 µs for 20 ms took 1.1 s above Renode's start). |
| P5.2 | The same at the boot's 100 ms | About 1.9 M lines (≈ 140 MB with the virtual prefix), far over `LOG_FILTER`'s 8 MiB, and over the 30 s wall bound. **DShot cannot run for the whole boot in the twin**; its witness needs a bounded window, decided after P5.1 (decision recorded in §4). |

### P6. 2100 F24: every `RCC_*ENR` cold

| id | probe | prediction |
|---|---|---|
| P6.1 | Cold-write all nine enable registers to their RM0433 reset values before the run, read each back after it | The reference boots as today. Each of the six registers not cold-written today reads its reset value or the bits the firmware set: `AHB1ENR`, `AHB2ENR`, `AHB3ENR`, `APB1HENR`, `APB3ENR` read 0; `APB4ENR` reads `0x0001 0000` if Renode stores `RTCAPBEN`, else 0 (not predicted which). |
| P6.2 | The reference's verdict | Gains six register rows (kind 11's image) and nothing else; every seeded case's `[index, kind]` evidence row after the image moves by the rows added (re-derived from `attribute`, §5 of this file). |

### P7. Twin edge stamps at the solved prescaler

| id | probe | prediction |
|---|---|---|
| P7.1 | `pwm400`: TIM1 CH1, `PSC` 63, `ARR` 2499, `CCR` 1000 (the 64 MHz solution for 400 Hz with a 1000 µs idle pulse) | Twin tick `64 × 4 ns` = 256 ns: period `2499 × 256` = 639,744 ns, high `1000 × 256` = 256,000 ns. Each stamp is a multiple of 10 ns; for the k-th rise `s_k`, `s_k − s_0 − k·639,744` lies in (−10, +10) ns and all such deviations, rises and falls, span less than 10 ns. Every rise after the first is off the 100 µs grid. |
| P7.2 | `pwm50`: `PSC` 63, `ARR` 19999, `CCR` 1000 | Period 5,119,744 ns, high 256,000 ns, the same rule. |

### P8. TIM register read-back

| id | probe | prediction |
|---|---|---|
| P8.1 | After `pwm400`, read `CR1`, `CCMR1`, `CCMR2`, `CCER`, `PSC`, `ARR`, `CCR1`–`CCR4`, `BDTR` | `CR1` = `0x81` (`ARPE`, `CEN` stored); `CCMR1` = `0x60` (`OC1M` 110 stored, `OC1PE` a tag, read 0); `CCER` = 1; `PSC` 63; `ARR` 2499; `CCR1` 1000; `BDTR` = 0 (`MOE` a tag). A full read-back therefore compares stored fields only, each tag named. |

### P9. The off-board net (§1.8)

- **P9.1** [read, from `4300f`'s measured S2]: a fixture whose motor nets end at one MCU pin fails `check erc` with one `isolated_pin_label` per net; DRC passes. So the fixture's ERC cannot pass honestly until a connector part carries each net (`4300f`'s build). Not re-measured here unless `4300f`'s build changes the emitter.
