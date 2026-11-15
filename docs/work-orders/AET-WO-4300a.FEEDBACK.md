# AET-WO-4300a FEEDBACK — motor outputs

**Builder:** Claude (strong tier), worktree `~/aethelgard-4300a`, branch `wo/aet-4300a-motor-outputs` from `main` `50d7a6f`.
**Work order:** `docs/work-orders/AET-WO-4300a.md` as on `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #136); identical to `main`'s copy at `50d7a6f`.
**Cadence:** build, with one measured stop (§1.2, pre-ruled at the `4400` ruling D1) and the off-board-net item (§1.8).
**Status: STOPPED at D1** (§5), on PR #145. §1.2 held (measured); §1.8 passes at a connector after `4300f` (measured); every other clause of §1.7 and §2 passes in the pinned image. The DShot600 fixture's campaign clause waits on D1: its 100 ms boot fails closed on the log budget. Adversarial pass §7, reduction pass §8, gates green.

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

## 2. Runs

- **Containers, one of mine at a time,** each through `~/.cache/aet-4300a/img.sh` (RULE3: `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=8192` around `podman run --rm --cgroups=disabled --network none`, no podman memory flags, `TMPDIR` unset inside). Every run's scope read back `memory.max=8589934592 memory.swap.max=0 pids.max=8192` before its command (`~/.cache/aet-4300a/caps.log`); none stopped at the swap guard.
  1. `main`'s binaries (`50d7a6f`): CLI with tools, bridge, reference firmware (`~/.cache/aet-4300a/baseline/main-bin`; CLI sha256 `40898e64…53fc`).
  2. The probes (§1's harness, `~/.cache/aet-4300a/probe`, kept in `~/.cache/aet-4300a/evidence/probe-final`: `runs.jsonl`, `fw.txt` with each ELF's sha256, raw stdout under `raw/`). Earlier partial runs of the same cases were superseded by the final run of all 15 cases, which is the one cited.
  3. D2 alone (commit `55690b2`): its binaries (`d2-bin`, CLI `3ae7c52b…84ea`), then `main`'s and D2's baselines: the reference campaign over every committed scenario (14), replay of every archive, and a static `simulate` (`4200`'s `run.sh`, compared by `4300c`'s keyed `compare-keyed.py`).
- Probe firmware is the committed reference firmware with raw register code inserted (`fw.py`), so every probe also performs the reference's two identity reads; every probe wrote the boot word.

## 3. Results beside predictions

| id | prediction | result | label | verdict |
|---|---|---|---|---|
| P1.1 | `CFGR`, `D1CFGR`, `D2CFGR`, `D3CFGR` = 0; `HSIDIV` = 0 | The firmware's own reads (records 2–4 of `rcc`): `CFGR` 0, `D1CFGR` 0, `D2CFGR` 0, `D3CFGR` 0, `RCC_CR` `0x25` (`HSION`, `HSIRDY`, `HSIDIVF`; `HSIDIV` = 0). The monitor's reads after the run agree. | measured | as predicted: **§1.2 holds; the §7 stop does not fire** |
| P1.2 | The same after `machine Reset` | `RCC_CR` `0x25`, the four prescaler registers 0 | measured | as predicted |
| P2.1 | `UG` + `BSRR`: `IRQ pending` and the edge share each stamp, all on the grid | Exactly: `IRQ pending` and `LED state changed to True` both at 0, 6, 12 and 18 ms; every fall at +400 µs; no stamp off the grid | measured | as predicted. **The coincident forgery is measured** (`4300` D4's open item) |
| P2.2 | `BSRR` alone: on the grid, no `IRQ pending` | The same edges, no `IRQ pending` | measured | as predicted |
| P3.1 | D2 changes no verdict | All 14 campaign lines equal field for field (keyed by scenario), replay verdicts equal, **every trace and manifest byte-identical** (28 hashes); `simulate` differs only in the firmware's path | measured | as predicted: **the second §7 stop does not fire** |
| P3.2 | Timer lines exact in 10 ns steps | Yes (P7) | measured | as predicted |
| P5.1 | 10 ms, 4 channels: ≈ 189,000 observer lines, several seconds | 400,956 lines with `timer1` also at `Noisy` (half are its `received external input` lines), 33.6 MB, 4.3 s | measured | as predicted (the line count included the timer's lines) |
| P5.2 | 100 ms: ≈ 1.9 M lines, ≈ 140 MB, over `LOG_FILTER` and over the 30 s wall bound | Observers only: **1,886,808 lines, 139.6 MB, 14.7 s** on Furnace. With no source at `Noisy`: 10.8 s, so the timer events, not the log, cost most | measured | lines and bytes as predicted; **the wall bound is wrong on Furnace** (14.7 s < 30 s). On CI's runners it would be about 3× slower (inferred from `BOOT_BOUNDS`' note: 2.16 s on Furnace against 6.84 s under CI's parallel tests), near or over 30 s |
| P6.1 | All nine enable registers cold at reset values; the reference boots; `APB4ENR` reads `0x0001 0000` or 0 | `cold9`: the reference boots (boot word, both records `STATUS_OK`); after the run `AHB1ENR`, `AHB2ENR`, `AHB3ENR`, `APB1HENR`, `APB3ENR` read 0, **`APB4ENR` reads `0x0001 0000`** (Renode stores `RTCAPBEN`), and the three registers cold today read the reference's bits. Without the six new cold writes they read Renode's every-clock-on values (`AHB1ENR` `0x8023`, `AHB3ENR` `0x4010`, `APB4ENR` `0x11E8A`, …) | measured | as predicted |
| P7.1 | `pwm400`: period 639,744 ns, high 256,000 ns, deviations in (−10, 10), span < 10, every rise after the first off the grid | 156 rises, 155 falls; deviations −6 to +2 ns (span 8); every stamp a multiple of 10 ns; **0 of 311 edges on the grid**; first rise at 639,750 ns | measured | as predicted |
| P7.2 | `pwm50`: period 5,119,744 ns | 19 rises; deviations −6 to +2 (span 8); 0 of 38 on the grid | measured | as predicted |
| P8.1 | `CR1` 0x81, `CCMR1` 0x60, `CCER` 1, `PSC` 63, `ARR` 2499, `CCR1` 1000, `BDTR` 0 | Exactly. Also: unused `CCR2`–`CCR4` read `0xFFFF` (silicon resets them to 0) | measured | as predicted |

**Measured beyond the predictions:**
- **M1, a polled timer loses its exact stamps.** A firmware that polls `TIM1_SR` (`UIF` or `CC1IF`) to count a 16-bit DShot burst saw 52–53 pulses, and every edge of the first ~38 periods carried one stamp (16.45 µs): Renode advanced the timer in one step when the core read it (`burst-cc1if`, `burst-uif-opm`, `burst-uif-force`). The WO's ground "an edge the timer causes is stamped exactly" holds only while the core does not touch the timer: a firmware that starts PWM and then runs the reference's I2C and SPI reads before idling (`pwm400-early`) keeps every stamp exact (span 8 ns, 0 of 311 on the grid). So the generated firmware writes a timer only to configure and start it, and the judge names stamps that collapse onto one instant (`motor_edges_batched`) apart from wrong timing.
- **M2, the timer's phase is the quantum's.** In every PWM probe the first rise is one period after virtual time ≈ 6 ns, although the core ran thousands of instructions before `CEN`: a store the core makes takes effect at its quantum's start. So the judge anchors at the first observed rise and never predicts an absolute stamp.
- **M4, a store can land off the grid.** In the tier-2 `UG` + `BSRR` forgery (`twin_tier2::adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail`, image run of `176c21c`), 4 of MOTOR1's 40 store-made edges (the first output the test reads) carried stamps off the 100 µs grid: a store right after a timer access (`EGR.UG`) carries the exact instant, because the access brings the clock there (as M1's polling did). So P2.1's premise, and the WO's ground that "a store can drive a pin, but only on the grid", holds only for a store not preceded by a timer access. The forgery still failed: its edges missed the model's sequence (span 5.5 ms), and the read-back named `TIM1.CR1` and `TIM1.DIER`. The witness rests on all three parts together, as D4 has it; the grid is never the witness alone, and the judge's doc comments now say so. A forger who could pace stores exactly by polling a timer would still have to leave the timer's own registers unlike the solved values, or the timer would be making the edges (open for the adversarial pass, §7).
- **M3, short DShot windows.** Four DShot600 channels with only the observers at `Noisy`: 1 ms is 18,884 lines, 1.40 MB, 2.30 s; 2 ms is 37,748 lines, 2.80 MB, 2.64 s (Renode's start alone is about 2.0 s).

**The first real boots of the fixtures through the judge** (`~/.cache/aet-4300a/evidence/trial`: this branch at `5f68264` with the board-crate generator's first three commits, built in the pinned image; `generate`, `build`, then `simulate` against each fixture's emitted platform):
- `motors-pwm`, 100 ms: **pass**. Each of MOTOR1–MOTOR4 (TIM1 CH1–CH4 on PA8–PA11) `witnessed`: 156 rises and 155 falls from a first rise at 662,750 ns, deviations spanning 8 ns, all 311 edges off the grid, none batched. Every judged TIM1 field reads the solved value (`CR1` 0x81, `DIER` 0, `CCMR1`/`CCMR2` 0x6060, `CCER` 0x1111, `PSC` 63, `ARR` 2499, `CCR1`–`CCR4` 1000), `GPIOA.AFRH` 0x1111, `RCC.APB2ENR` 0x1001; the static check holds for each.
- `motors-dshot600`, `--for 2ms`: **pass**, each output `witnessed`: 4,662 rises and falls, span 8 ns, 9,322 edges off the grid, the registers at `PSC` 0, `ARR` 106, `CCR` 40.
- `motors-dshot600`, 100 ms: **fail, closed**: `renode_log_flood` (info), then every witness unreadable (`motor_output_unwitnessed` four times, and the I2C and SPI identity witnesses with them). This is D1 (§5) measured through the CLI.

**§2's "the reference's verdicts are unchanged apart from the named consequences", measured** against `main`'s binaries at the `main` merged in (`cd0bd31`), as `4200` did (`~/.cache/aet-4300a/baseline/main2`, `head2`; `run.sh` over every committed scenario, replay of each archive, a static `simulate`; keyed by scenario):
- **All 14 shared scenarios keep their verdict and finding kinds**, field for field. They differ only in `run_id`, `run`, `state_sequence_hash`, `trace_bytes` and `event_count` (+6 each). Each difference is named: the six enable registers judged in full (§1.6, F24) are six more kind 11 events; trace version 8 (§1.4) and specification version 4 (§1.1, which moves the specification's hash, and with it the run id) change every trace's bytes. D2 itself changes nothing (P3.1).
- `simulate` differs in `boot.registers` (the same six), `boot.specification_sha256` and `input.sha256` (specification 4), and the firmware's path.
- The new seeded scenario `motors-pwm` runs here with the reference firmware, which starts no timers, and fails `motor_output_unwitnessed` and `register_mismatch`: it is a seeded case's scenario, meant for its mutant firmware on the PWM fixture's board.
- **Re-measured after the merge of `main` `091003f` (4300f's connectors)**, this head `07d0ffd` against `main`'s binaries built from `091003f` in the image (`~/.cache/aet-4300a/baseline/main3`, `head3`): the same result. All 14 shared scenarios keep their verdict and `matched`; each differs by `event_count` +6 and its hashes; every archive replays `ok` (15 on this side, the extra one `motors-pwm` as above); `simulate` passes on both with the same finding kinds, and its registers differ by the same six enable registers (`AHB1ENR`, `AHB2ENR`, `AHB3ENR`, `APB1HENR`, `APB3ENR` at 0, `APB4ENR` at 0x00010000). [measured]

**The seeded set's evidence, re-derived from `attribute`** on the image's run (`~/.cache/aet-4300a/derive.log`; every case's domain and rule unchanged). Every move is 2100 F24's: the six enable registers now judged in full are kind 11 records archived in address order, `AHB3ENR`, `AHB1ENR` and `AHB2ENR` before `AHB4ENR`, `APB3ENR` before `APB1LENR`, `APB1HENR` and `APB4ENR` among the rest, so each event after them moves by six, and an enable register by the ones now before it:

| case | before (evidence, masked) | after |
|---|---|---|
| `trap-wrong-address-47k` | [[4,7],[5,7],[6,9]] [[16,13]] | [[4,7],[5,7],[6,9]] [[22,13]] |
| `wrong-address-2k2`, `trap-wrong-address-3k3`, `wrong-register`, `wrong-nbytes` | [[4,7],[5,7],[15,13]] | [[4,7],[5,7],[21,13]] |
| `i2c-clock-skipped` | [[4,7],[5,7],[12,11]] | [[4,7],[5,7],[16,11]] (`APB1LENR`, now after four enable registers) |
| `gpiob-clock-skipped` | [[4,7],[5,7],[9,11],[10,11],[11,11]] | [[4,7],[5,7],[9,11],[10,11],[14,11]] (`AHB4ENR`, now after three) |
| `spi-absent-u4`, `cs-never` | [[17,22]] | [[23,22]] |
| `motor-output-dead` (new) | — | [[31,11],[47,26]]: `GPIOA.AFRH` and MOTOR1's witness, S4 (was [[28,11],[44,26]] before the adversarial pass judged `TIM1.CR2`, `SMCR` and `RCR`, three more kind 11 events before them; re-derived at `07d0ffd`, `attribute_seeded` 11 of 11 in the image) |

**§1.8, the off-board net, measured on this work order's fixture** (`~/.cache/aet-4300a/evidence/gates-1.8`, HEAD `9fe8baa`'s binaries built in the pinned image, one container under RULE3, caps read back: memory.max 8 GiB, swap 0, pids 8192):
- `aethelgard emit --file crates/aethelgard-cli/tests/fixtures/motors-pwm.json`: exit 0. The schematic carries each motor net `MOTOR1`–`MOTOR4` as one local label on one MCU pin (PA8–PA11), and nothing else on it.
- `aethelgard check erc`: kicad-cli exit 5, verdict fail, **four findings, each `isolated_pin_label` (warning) "Label connected to only one pin"**, one per motor net. The reference (`sentinel-01-minimal.json`) passes with none.
- `aethelgard check drc --spec … --schematic …`: exit 0, pass, parity included, for both.
- This agrees with `4300f`'s S2 (measured on three SPI nets, PR #134). **The gates do not pass an off-board net honestly**: only a `no_connect` on a used pin would silence ERC, which marks a driven pin unused. By §1.8 that is the STOP branch, and the kickoff applies it only if `4300f`'s connector build has not landed when the board side is reached: at `origin/main` `cd0bd31` (2026-11-15 05:40 UTC) it has not.

**§1.8 after `4300f` landed (`eb19c64`, 06:25 UTC, merged here at `4fd0dcf`), measured** (`~/.cache/aet-4300a/m3`, `12320c6`'s binaries built in the pinned image, one container under RULE3, caps read back: memory.max 8 GiB, swap 0, pids 8192). Each motor fixture now ends `MOTOR1`–`MOTOR4` at `J1`, an `SM06B-GHS-TB` (JST-GH 6) with an explicit map (`standard: null`; DS-009 has no motor port, `4300f` S1): pins 1–4 the motor nets, pin 5 `null`, pin 6 `GND`.
- `emit`, `check erc`, `check drc --spec --schematic` (parity included): exit 0 and `pass` with **no findings** on both fixtures and on the reference.
- **No waiver** [measured]: the PWM schematic holds 107 `no_connect`s, the reference's 110 less PA8–PA11 (now used) plus J1 pin 5 (an unused connector contact, not a used MCU pin), and 8 `MOTOR` labels (4 at the MCU, 4 at J1). The DShot600 schematic the same.
- So §1.8's first branch holds: the gates pass the off-board net honestly, at a connector. The gap the verdict names is the one `4300f` leaves: a motor pin has no off-board capacitance (D8 covers bus lines), so its speed is not judged (§4, Q3).
- `generate` and `build` exit 0 on both fixtures; `simulate`: PWM at 100 ms passes (4 × `witnessed`, 156 rises, first rise 662,750 ns at this HEAD, span 8 ns, 311 edges off the grid; 28 registers judged, none wrong); DShot600 at 2 ms passes (4 × `witnessed`, 4,662 rises); **DShot600 at 100 ms fails closed** (`renode_log_flood`, so every witness and both bus transactions `…_unwitnessed`): D1, unchanged.
- Re-measured after the merge of `1308` (specification 6, per-class DRC rules), at `fa2a572` in the image (`~/.cache/aet-4300a/m5`): `emit`, ERC and DRC pass with 0 findings on both fixtures and the reference; `twin_tier2` 31 of 31 and `campaign_tier2` pass. [measured]

## 4. Decisions taken inside the work order

- **D2's quantum (F1).** Renode 1.17.0 has no configuration key for the quantum (§0), so the composition states it by `emulation SetGlobalQuantum "0.000100000"` (`renode::QUANTUM`), Renode's own value, and the judge reads the grid from `aethelgard_twin::boot::QUANTUM_NS`. Measured to change nothing (P3.1). A deviation from D2's words, recorded for the architect.
- **DShot timing source.** No standard document defines DShot. The bit rates and duty are ArduPilot's, at the tag `4400` pinned (`Copter-4.7.2`, `fb773567`): DShot300 and DShot600 are 300,000 and 600,000 bit/s (`libraries/AP_HAL_ChibiOS/RCOutput.cpp` L2521–2524), a bit is 8 ticks and a `0` bit is high for 3 of them, a `1` for 6 (`libraries/AP_HAL/RCOutput.h` L406–408, `DSHOT_BIT_WIDTH_TICKS_DEFAULT`, `DSHOT_BIT_0_TICKS_DEFAULT`, `DSHOT_BIT_1_TICKS_DEFAULT`). [read]
- **Where a motor output lives in the topology.** Its net is its label, and its pin, timer channel (`TIMn` / `TIMn_CHk`) and AF are an entry of `assignments`, so every generator that draws or configures solved pins sees it unchanged: the schematic emitter connects it to the MCU pin as it does every solved net (`aethelgard-eda` untouched, outside this fence). `motor_outputs` adds the protocol, the declared PWM timing and the solved `PSC`, `ARR`, `CCR`; `clock` is the reset-clock fact (§1.2). `SolvedTopology::validate` checks the shape (one `TIMn_CHk` per output of a timer with channels, each channel once, `CCR` below `ARR`, one protocol, rate, `PSC` and `ARR` per timer) and refuses any clock but the reset one; it does not judge the timing, which is the static check's.
- **The rounding rule and admissible error (D3's static check, `motor::check_silicon`).** PWM takes a 1 µs tick (`PSC` 63 at 64 MHz) and the nearest whole tick count to the declared period, half up, so `ARR` is that less 1; `CCR` is the idle pulse, the range's minimum (an ESC's throttle-off pulse; the WO names the idle pulse and the range, and nothing else could be it). DShot takes `PSC` 0, the bit the nearest whole count of 64 MHz ticks to `1 / bit rate` (107 at 600 kbit/s: 1671.875 ns, 5.2 ns long), and `CCR` the nearest whole count to 3/8 of it (40: 625 ns, exactly 3/8 of the nominal bit). The check, in exact integers with silicon's `ARR + 1`: the period within half a tick of the declared one; for PWM the high time exactly the idle pulse and the range's maximum strictly inside the period; for DShot the high time within half a tick of 3/8 of the nominal bit. 490 Hz rounds to 2041 µs (0.18 µs over, inside half a tick); below 16 Hz the period passes a 16-bit `ARR` and `solve` refuses it (`MotorTiming`).
- **The twin's witness rule (D4).** The judge anchors at the first rise (the model's first edge comes one period after a `CEN` that takes effect at its quantum's start, M2), predicts every later edge from the model's conventions (`ARR` and `CCR` ticks of `PSC + 1` cycles at 250 MHz, `aethelgard_twin::timer`, each with its `STM32_Timer.cs` line), and requires: at least two rises; edges alternating from a rise; every deviation from the prediction inside one span narrower than the 10 ns stamp grain (measured spread 8 ns, P7); no two edges on one stamp (M1); and at least one edge off the 100 µs grid. A sequence that fits with every edge on the grid is `motor_witness_static_only` (info), as D4 says, never a pass of the twin. After the adversarial pass (H1) the outputs must also run to the window's end: the last rise no earlier than a period and a grain before it, or the status is `stopped` (a failure). The TIM read-back judges the fields Renode stores and acts on: `CR1`, `CR2.TI1S`, `SMCR`'s `SMS` and `TS`, `DIER`, `CCMR1`, `CCMR2`, `CCER`, `PSC`, `ARR`, `RCR` (on the timers with a break and repetition stage), each used `CCRx` (adversarial H2 added `CR2`, `SMCR` and `RCR`). It lists as unobservable each `OCxPE`, `SMCR.ECE`, a motor pin's `GPIOx.OTYPER`, and, on those timers, `BDTR.MOE` and `BDTR.BKE`: each is a tag in `STM32_Timer.cs` or `STM32_GPIOPort.cs`, so the model never acts on it. `DIER` = 0 is judged, so a firmware that arms the update interrupt the `UG` + `BSRR` forgery used fails on its register.
- **Motor pin speed.** A timer pin's `OSPEEDR` is `00`, out of the 4300c speed check's scope with its own reason: the output leaves the board and its load is off-board. After `4300f` a motor net ends at a connector pin, but `4300f`'s ruling D8 asks an off-board capacitance only of an I2C or SPI line, and a motor pin that declares one is refused (`ConnectorBudgetNotApplicable`, `aethelgard-ir/tests/motor.rs`), so no number exists for the speed check to read. A net constraint on a motor net is refused by name (`UnjudgedMotorConstraint`), as 4300c refused one no check reads. A question for the architect (§6 Q3).
- **A connector may end a motor net** (`connectors::net_kinds`, `NetKind::Motor`). `4300f`'s table of nets a connector pin may name held bus lines, chip selects, domains and `GND`; a motor output's label is a declared net too. A motor net on a DS-009 port pin is refused by that port's role, as before. Inside the fence (`aethelgard-ir`).
- **Observer names are reserved.** The platform is one namespace, so a device reference may not be `p<port><n>obs` (`aethelgard_ir::is_observer_name`, in the reference rule).
- **The clock assertion reads `TIMPRE` too.** The WO lists `RCC_CFGR.SWS` and the `D1CFGR`/`D2CFGR` prescalers; `TIMPRE` is in the same `CFGR` read and is part of the solved fact (Table 57), so the generated check requires it 0 as well. `RCC_CR.HSIDIV`, on which the 64 MHz depends, is not asserted: it is another register, outside the WO's list, and it measured 0 (P1.1). A question for the architect (§6).
- **The motor symptoms' rule, S4 (software).** `motor_output_dead`'s evidence is the pin's `AFR` field read back other than the solved AF (kind 11) beside the output's unwitnessed edges (kind 26). The AF is the firmware's own write, and the twin models nothing electrical on a timer's pin (no copper, no connector, no load), so the evidence allows software only. A motor symptom with no wrong owned register is `I4`, never software by elimination.

**The board crate and Kani** (built by an Opus subagent in its own worktree, commits cherry-picked as `c296472`, `f11df34`, `b08dde6`, touching only `aethelgard-svd-hal`; its report checked against the artifact: the reference board regenerates byte-identical, `order_2203`'s firmware digest is unchanged, and the generated firmware touches no timer after `CEN`). Kani 0.68.0 in the pinned image [measured, subagent's runs]:
- `tim1_start_writes_the_solved_values`: verified on both fixtures, about 5 s each; `reset_clock_is_ok_exactly_at_reset`: verified, about 1 s. Hand-seeded negatives each failed with its own message (CR1 0x81 → 0x01, CCMR2 and CCER swapped, the clock mask without `TIMPRE`).
- `every_valid_sequence` (unwind 77), the board's whole-schedule harness: 121 s on `boards/sentinel-01`, **not finished after 33.5 min on the PWM fixture** (symex 138 s, 286 k VCCs against 93.6 k; one solver call over 20 min). CI therefore proves a motor board by the two targeted harnesses (`twin_tier2::kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check`); `verify` on a motor board would run the whole-schedule harness and cannot fit a shard (§6 Q5).

- **Versions, after two merges of `main`.** `4300f` merged specification 4 (`connectors`) first, then `1308` specification 5 (`net_classes`, `diff_pairs`), so by the numbering rule this work order takes **specification 6** (`motor_outputs`, `mcu.hse_hz`), topology 5 and trace 8 (`4fd0dcf`, `7e75295`). Every specification carries all four keys; the motor shape is step 13b after `1308`'s 13a. `4210` also claims trace 8: whoever merges second renumbers. TIER2_MIN is 107 (`main`'s 101 and the six motor tests; the `#[test]` count of the ten tier-2 files is 107).

## 5. Decisions needed (ruled on PR #145, comment 6892698852)

**D1 ruled (a), built (§9):** a DShot board's boot runs `aethelgard_twin::boot::DSHOT_WINDOW_NS` (2 ms), named in the verdict and on each campaign line; a boot past it is refused (`BootPastWindow`), a droop past it is refused with its time, and an identity read still unset at its end is named (`identity_read_outside_window`). `VIRTUAL_NS` is 100 ms for every other board. The options as written before the ruling:


- **D1, DShot in the campaign's boot (blocks only the DShot fixture's campaign clause of §1.7 and §2).** Every boot runs 100 ms of virtual time (`aethelgard_twin::boot::MAX_VIRTUAL_NS`, the campaign's `VIRTUAL_NS`). Measured (P5.2, M3): four DShot600 channels for 100 ms cost 10.8 s of timer events alone and, observed, 1.89 M lines and 139.6 MB, 17× `LOG_FILTER`'s 8 MiB, so the witness would be dropped and fail closed; on CI's runners the boot would also approach the 30 s wall bound (inferred). A firmware-ended burst cannot replace it: polling the timer collapses its stamps (M1), an interrupt reopens `RUL-19` (5)'s A2, and one-pulse mode gives no second rise to witness a period. Options:
  - **(a)** a stated, shorter fixed window for a boot whose topology has a DShot output (2 ms measured: 37,748 lines, 2.80 MB, 2.64 s on Furnace), the window named in the verdict. Cost: a DShot board's identity reads and droop schedules must fit it. **Recommended.**
  - **(b)** keep 100 ms and accept the DShot fixture's campaign as failing closed (the static check alone judges DShot).
  - **(c)** raise `LOG_FILTER` and `BOOT_BOUNDS` for DShot boards (not recommended: the bounds exist for floods).
  Built as far as no ruling is needed: the solver, the static check, the firmware, the Kani harness and the three-part witness all cover DShot, and `simulate --for 2ms` witnesses the DShot fixture (tier 2). `VIRTUAL_NS` is unchanged.

## 6. Questions for the architect (not stops) — ruled

Ruled on PR #145 (comment 6892698852): **Q1** accepted, a deviation of form (`SetGlobalQuantum` stated); **Q2** yes, built (§9); **Q3** not here: the verdict states "motor pin speed not judged: off-board load undeclared" on each output, `UnjudgedMotorConstraint` stays, backlog owned by `4500`; **Q4** built: the seeded manifest is schema 3 and every mutant names its `spec`; **Q5** the targeted harnesses prove motor boards, and `verify` on a motor board states which ran and that the whole-schedule one did not (an info finding, never a silent skip); the split is `4500`'s backlog. The questions as asked:


- **Q1, the quantum's key (F1).** D2 asked for the quantum as a `--config` key; Renode 1.17.0 has none, so the composition states it by `emulation SetGlobalQuantum` at Renode's own 100 µs. Acceptable as the reading of D2?
- **Q2, `RCC_CR.HSIDIV`.** The 64 MHz the timers are solved for depends on `HSIDIV = 00`, which the generated firmware does not assert (the WO's list is `SWS` and the `D1CFGR`/`D2CFGR` prescalers; `TIMPRE`, in the same read, is asserted). Add it to the assertion in a later piece?
- **Q3, a motor pin's output speed.** Connectors have landed (`4300f`), but D8 gives a motor pin no off-board capacitance. Extend D8 to motor pins (an ESC's input and its cable) and judge the timer pin by the Fmax check at the output's toggle rate (≤ 1.2 MHz for DShot600), as 4300c does SCK and MOSI?
- **Q4, the seeded manifest's `spec` key.** A mutant may now name the specification whose generated board it patches (the motor case runs on the PWM fixture, whose board is not committed). The manifest stays at schema version 2 with the key optional; a version bump instead?
- **Q5, the whole-schedule Kani harness on a motor board.** `every_valid_sequence` did not finish in 33.5 min on the PWM fixture (121 s on the reference). The timer start and the clock check have their own harnesses, verified in seconds. Before Sentinel-02's `verify` (`4500`), split the schedule harness by peripheral, raise its budget, or prove motor boards by the targeted harnesses alone?

## 7. Adversarial pass (full, a fresh subagent)

A fresh Opus subagent attacked `ff06380` through the real CLI in the pinned image, with its own scripts (`~/.cache/aet-4300a/adv/`: `mutate.py` patches the PWM fixture's generated board, `exp.sh` builds, boots for 100 ms, runs the campaign and `attribute`). Its findings, each fixed at `d1d6bd7` unless said otherwise, with the test that holds it:

| # | Severity | Finding | Fix | Held by |
|---|---|---|---|---|
| H1 | high (false pass) | No coverage rule: `SMCR` trigger mode written 2 ms after `CEN` stops Renode's counter with `CEN` still reading 1; 2 rises of 156 passed (measured). A start 72 ms late passed too. | The last rise must be within a period and a grain of the window's end, or the status is `stopped` (a failure, `motor_output_mismatch`); kind 26 carries `last_ns` and `end_ns`. The late start is **not** a failure, by decision: its edges are the timer's, at the solved timing, to the window's end (43 rises), and the boot's length before `CEN` is the firmware's; the witness bounds the end, not the start. | `renode.rs` unit tests (`adversarial_h1_…`); chaos `motor.rs` |
| H2 | high (false pass) | `SMCR` (gated on an unstarted TIM15 TRGO), `CR2.TI1S` and `RCR` are stored by Renode but were not judged; FEEDBACK §4 claimed every stored field was. `SMCR.ECE` is a tag and was not listed unobservable. | Judged at reset (`CR2` 0x80, `SMCR` 0x77, `RCR` 0xFF on the break timers); `SMCR.ECE`, `BDTR.BKE` and a motor pin's `OTYPER` listed unobservable; §4 corrected. | `twin` `motor_platform.rs`; `twin_tier2` (TIM1 judges 14 registers, measured) |
| H3 | high (wrong attribution) | Another channel's wrong `CCR2` made a dead MOTOR1 software (S4): evolve owned every timer register for every output of the timer. | Each output owns its own `CCRk`, its byte of `CCMR1`/`CCMR2`, its nibble of `CCER`, and the timer-wide registers whole (`timer_bits`). | evolve `adversarial_h3_…` (mutant measured to fail it) |
| R | high or medium | Replay took kinds 25 and 26 at their word: `holds: true` beside failing figures, and `witnessed` with 0 rises, both passed. | `check_run` re-derives kind 25's figures and `holds` from kind 24 and the kernel clock, and checks kind 26's model timing, grain, quantum and that its status is one its counts allow (both labelled `belt-and-suspenders`, since replay never depends on the twin or `aethelgard-ir`, RUL-16). | chaos `replay_re_derives_…` (three mutants measured to fail it) |
| M | medium | Attribution cited a passing witness (MOTOR1 `witnessed`) as evidence for MOTOR2's wrong `CCR2`. | A witness is evidence only when it failed. | evolve `adversarial_a_wrong_channel_…` (mutant measured to fail it) |
| L | low | 13 PWM rates on 12 timers ran the solver out of budget (about 60 s) instead of naming the conflict; 33 outputs at one rate likewise. | More rates than the timers the outputs can reach is `MotorsInfeasible` before any search (13 rates: 0.0 s, all 13 claimants, measured). 33 outputs at one rate is **not** fixed: a channel–pin matching bound did not fire on it (the conflict is joint), so it still reports `BudgetExceeded` after about 80 s (measured); the change was reverted, and the case is a residual. | ir `adversarial_more_rates_than_timers_…` (fails without the check at 100 nodes, measured) |

**The adversary's mutants re-run on this HEAD, through the CLI in the image** (`12320c6`, `~/.cache/aet-4300a/adv/out`, `exp.sh`; measured):
- `unmutated`: `simulate` pass; campaign 3 of 3 matched (`motors-pwm`, the droop at 98.4 ms and the mid-window droop); `attribute` N1 on each. The coverage rule does not fail a correct board under either droop.
- `smcr-gated`: fail, `register_mismatch` `TIM1.SMCR` 0x5 (the witness still `witnessed`, as Renode keeps counting); attribute S4.
- `stop-after-2ms`: fail, `register_mismatch` `TIM1.SMCR` 0x6 and 4 × `motor_timing_mismatch` (`stopped`, last rise at 1,302,490 ns).
- `ccr2-wrong`: fail, `register_mismatch` `TIM1.CCR2` and MOTOR2 alone `mismatch` (span 264 ns); attribute S4.
- `tim1en-cleared`: fail, `locked_peripheral_access` and every TIM1 register wrong.
- `late-start`, `smcr-ece`, `moe-off`: pass, each by decision above (`SMCR.ECE` and `BDTR.MOE` are tags, listed unobservable in the verdict).

**Residuals, inferred, not fixed:** a 50 Hz board needs about 40 ms after the last reset release to show two rises, so a droop that releases late in the 100 ms window would make a correct board `unwitnessed` or `stopped`; the scenario author must leave the window room. `BDTR.MOE` cleared passes the twin by design (unobservable, and the Kani harness proves the firmware sets it).

**What held (the subagent's, measured or read):** Renode's GPIO honours the AFR number, so no other timer can drive the pin while AFR reads the solved value; a polarity flip fails `CCER`; a timer stopped and restarted or phase-shifted by `UG` fails the sequential prediction; no integer PWM rate puts the model period on the 100 µs grid; the static check and the witness are never blended; the clock assertion's masks match RM0433; `TIM1EN` cleared is caught and attributed S4; a droop at 98.4 ms on the PWM fixture passes with one `brownout_reset`.

## 8. Reduction pass and gates

Reduction pass (diff): −44 net · 4 duplicates removed · 1 dead path removed · 5 protected kept · 8 findings below. A fresh subagent read the merge-base diff (`crates`, `ci`, `.github`); each proposal was checked against the code before it was applied (`07d0ffd`), tier 1 stayed green, and the reshaped motor-findings guard's mutant still fails `chaos/tests/motor.rs`:
- the verdict serializes `motor::SiliconTiming` (with `tick_ps` skipped) instead of a field-for-field `SiliconReport`;
- the generator builds a timer pin's `PinPlan` on the common path;
- the twin carries each channel's `CCR` instead of re-scanning the topology (an `expect` gone);
- `HSE_PINS`, read only by a test of its own value, folded into `RESERVED_PINS`' doc (dead path);
- the four motor finding kinds listed once (`MOTOR_FINDINGS`).
Protected, kept (`belt-and-suspenders:`): the generator's `timer_oracle`; replay's `TWIN_CYCLE_NS`/`GRAIN`/`QUANTUM`, `silicon_check` and the observer path. Findings for the owner, not removed: (F1) the witness status strings live in four places across the CLI, chaos and evolve; (F2) `board.rs`'s `TimerShared` and duplicate-channel branches cannot fire after `validate()`; (F3) the twin's `CR1_CEN`, `CR1_ARPE`, `OCM_PWM1` and channel shifts restate the generator's under the existing `INV-07` label; (F4) `motor::has_main_output_enable` and `apb()` hold the same timer set for different hardware reasons; (F5) `TIMER_NAMES` spells `format!("timer{n}")` out because `AddressMismatch.name` is `&'static str`; (F6) the witness fields are copied three times in three shapes; (F7) `MotorProtocol::name()` restates serde's spelling; (F8) no weakened test seen.

## 9. Resume after the ruling (PR #145, comment 6892698852)

The architect ruled D1 (a), Q1 accepted (`SetGlobalQuantum` stated, a deviation of form), Q2 in this work order, Q3 stated in the verdict and owned by `4500`, Q4 a manifest version bump with `spec` required, Q5 the targeted harnesses with `verify` stating what ran. The second fresh review's findings (`~/.cache/aet-4300a/adv2-findings.md`, on `a2c4fa4`) are inside the fence: F1 (a wrong attribution in this work order's own rule) blocks READY, and F2–F6 are fixed with it. That is a stated addition to the ruling's list, not a silent one.

### Predictions (committed before any run of the resume)

- **P10, D1 window.** The DShot600 fixture's campaign at a 2 ms window passes: all four outputs `witnessed` (about 4,660 rises each, as `simulate --for 2ms` measured), the log about 2.8 MB, under the 8 MiB budget, and both identity reads (U2, U4) complete inside it. [inferred from P5.2 and §1.8's 2 ms boot]
- **P11, a late event.** A droop whose effects end after 2 ms on a DShot board is refused before any boot, `DroopRefused`, naming its end time and the window. [read: `check_droop` refuses past its span]
- **P12, a read outside the window.** A held-low SCL on the DShot fixture: the I2C wait loops run past 2 ms, so the boot word is not set and the verdict names U2's read as not completed inside the window. [inferred; the loops' length is not measured]
- **P13, a mixed board.** A PWM output at 400 Hz beside a DShot one cannot show two rises in 2 ms (its model period is 2.56 ms): the verdict names it unwitnessed with the window as the reason. This is a question for the architect, not widened here. [computed]
- **P14, Q2.** Renode's `RCC_CR` reads 0x25 (P1.1), so `HSIDIV` is 0 and the motor fixtures still boot; the Kani harness proves the check, and a copy whose check omits `HSIDIV` fails it as a failed proof. [inferred]
- **P15, Q4.** The reference mutants, generating their board from `examples/sentinel-01-minimal.json`, give the same evidence as the committed board (the regeneration guard holds them byte-identical). [inferred]
- **P16, Q5.** `verify` on the PWM fixture runs the targeted harnesses, names `every_valid_sequence` as not run in an info finding, and otherwise passes. [inferred]

### What was built (inside the fence; the commits from `cb17d54` on)

- **D1 (a).** `aethelgard_twin::boot::window(topology)` gives `DSHOT_WINDOW_NS` (2 ms) for a board with a DShot output and `MAX_VIRTUAL_NS` otherwise. The verdict's `boot.window` and each campaign line's `window` and `virtual_time_ns` name it. The campaign summary's `virtual_time_ns` is the longest window any scenario ran. `simulate --for` past the board's window is refused (`BootPastWindow`), and so is a droop whose effects end past it (`DroopRefused`, naming the time). `verify` boots for the board's window. A declared identity read whose record is still `RECORD_UNSET` when a short window ends is named (`identity_read_outside_window`). An output whose two model periods exceed its window is named as the reason it is unwitnessed. Replay derives the window from kind 24's protocols and requires every kind 26 to end at it and kind 10 to have run exactly its instructions.
- **Q2.** `RESET_CLOCK_FIELDS` gains `RCC_CR.HSIDIV`. The generated check reads four registers; the Kani harness, generated from the same table, covers it; `twin_tier2`'s Kani test proves a copy whose check leaves it out fails as a failed proof.
- **Q3.** Each motor report carries `speed`: "motor pin speed not judged: off-board load undeclared".
- **Q4.** `cases.json` is schema 3. `Mutant.spec` is required, every mutant names it, and the reference mutants generate their board from `examples/sentinel-01-minimal.json`.
- **Q5.** `verify` drops exactly `proofs::every_valid_sequence` on a board with motor outputs and states it in an info finding naming the harnesses that ran (`harness_plan`, by a subagent, `cb17d54`, reviewed). `report.ok` now ignores info findings (every other finding of `verify` is an error).
- **The second review's findings.**
  - F1: a failing static check has its own scope, which decides I4.
  - F2: a motor output named `GND` is refused (`NetShadowsDomain`). A chip select named `GND` still validates on `main`; this work order does not change it, and it is reported here.
  - F3: replay ties each witness's fields to each other and to its window.
  - F4: replay pins the reset clock's 64 MHz.
  - F5: the reason is named, as above.
  - F6: only failing records are motor evidence.

  Each new guard was mutated once against its suite, and every mutant failed it.

### Results beside the predictions (measured in the pinned image, `~/.cache/aet-4300a/m6`, `02601a8`, one container under RULE3, caps read back, swap 0)

| | Prediction | Measured |
|---|---|---|
| P10 | DShot600 campaign passes at 2 ms | **Held.** `motors-dshot600` (`scenarios/motors/dshot600.json`): pass, matched, `virtual_time_ns` 2,000,000, `window` `DSHOT_WINDOW_NS`; replay exit 0; `attribute` N1. `simulate --for 2ms`: 4 × `witnessed`. |
| P11 | late droop refused with its time | **Held** (tier 1, `campaign_tier1`): `DroopRefused`, "past the boot's 2000000 ns", no archive. `simulate --for 100ms` on the DShot board: exit 3, `BootPastWindow`, measured in the image. |
| P12 | held-low SCL: U2 named | **Held.** Findings `boot_word_missing`, `i2c_line_held_low`, `identity_read_outside_window`, `register_mismatch` (the timers were never started). Replay exit 0. **`attribute` answers I1** (the finding is unexplained by any rule): fail-closed, and a question below. |
| P13 | a 400 Hz PWM output beside DShot cannot be witnessed in 2 ms | **Wrong.** The twin witnesses the model's period, 639,744 ns at 400 Hz (Renode's 250 MHz with `PSC` 63, §0), not silicon's 2.5 ms: the mixed board passes, MOTOR1 `witnessed` with 3 rises. Only a PWM rate whose two model periods exceed the window (below about 260 Hz, e.g. 50 Hz servos) fails, closed and named. |
| P14 | HSIDIV reads 0; Kani negative fails | **Held.** Both motor fixtures boot and pass; `reset_clock_is_ok_exactly_at_reset` passes (0.8 s); the copy without HSIDIV exits 1, `proof_failed`. |
| P15 | reference mutants' evidence unchanged | **Held.** `attribute_seeded` 11 of 11 with every mutant generating its board. |
| P16 | `verify` on the PWM fixture | **Held.** Pass in 30 s: ERC, DRC, extract, five targeted `prove` gates, build, simulate; the info finding names them and `proofs::every_valid_sequence` as not run. |

`twin_tier2` 32 of 32, `campaign_tier2` and `attribute_seeded` pass. TIER2_MIN is 110 after the merge of `1309` (`main`'s 103, the six motor tests, and the DShot600 campaign test; the `#[test]` count of the ten tier-2 files is 110). `main` at `351d25c` holds an empty file named `9` at the repository root, from `1309`; outside this fence, reported.

Reduction pass (diff, the resume): −9 net · 1 duplicate removed · 2 dead paths removed · 4 protected kept · 5 findings below. A fresh subagent read the resume's own commits; each proposal was checked before it was applied, and the reshaped `harness_plan` guard's mutant still fails its tests:
- `verify`'s skipped harness is a bool, so `WHOLE_SCHEDULE` is stated once;
- a tautological test (`an_info_finding_does_not_fail_the_verdict`) was removed;
- an orphaned comment left by the removed `VIRTUAL_NS` was removed;
- in the same change, the `--for` help and the CLI README now state the DShot window, and a misplaced test doc comment was moved.

Protected, kept: replay's window, instruction and clock constants, and the selection rule (any DShot output), now named in the label. Findings, not removed:
- `docs/architecture.md` still says "a fixed 100 ms"; it is a steering document, so the finding stays here;
- `simulate` computes the window twice (in `ready` and for the boot plan), at 0 net lines;
- the 11 `spec` lines in `cases.json` are the ruling's;
- `MOTOR_SPEED_NOT_JUDGED` has one caller, kept as the ruled statement;
- `Scope::MotorStatic` and `Scope::Motor` share an I4 tail, needed for F1.

### Questions for the architect (not stops)

- **Q6, slow PWM beside DShot.** A board that mixes DShot with PWM below about 260 Hz cannot witness the PWM output in the 2 ms window: it fails closed, named. Accept that as the rule for mixed boards, or give such boards a window long enough for two of their slowest model periods?
- **Q7, attributing a read outside the window.** `identity_read_outside_window` has no rule in `aethelgard-evolve`, so `attribute` answers I1 on such a run. Add one (indeterminate, by the unset record) in a later piece?
