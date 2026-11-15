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

**The seeded set's evidence, re-derived from `attribute`** on the image's run (`~/.cache/aet-4300a/derive.log`; every case's domain and rule unchanged). Every move is 2100 F24's: the six enable registers now judged in full are kind 11 records archived in address order, `AHB3ENR`, `AHB1ENR` and `AHB2ENR` before `AHB4ENR`, `APB3ENR` before `APB1LENR`, `APB1HENR` and `APB4ENR` among the rest, so each event after them moves by six, and an enable register by the ones now before it:

| case | before (evidence, masked) | after |
|---|---|---|
| `trap-wrong-address-47k` | [[4,7],[5,7],[6,9]] [[16,13]] | [[4,7],[5,7],[6,9]] [[22,13]] |
| `wrong-address-2k2`, `trap-wrong-address-3k3`, `wrong-register`, `wrong-nbytes` | [[4,7],[5,7],[15,13]] | [[4,7],[5,7],[21,13]] |
| `i2c-clock-skipped` | [[4,7],[5,7],[12,11]] | [[4,7],[5,7],[16,11]] (`APB1LENR`, now after four enable registers) |
| `gpiob-clock-skipped` | [[4,7],[5,7],[9,11],[10,11],[11,11]] | [[4,7],[5,7],[9,11],[10,11],[14,11]] (`AHB4ENR`, now after three) |
| `spi-absent-u4`, `cs-never` | [[17,22]] | [[23,22]] |
| `motor-output-dead` (new) | — | [[28,11],[44,26]]: `GPIOA.AFRH` and MOTOR1's witness, S4 |

**§1.8, the off-board net, measured on this work order's fixture** (`~/.cache/aet-4300a/evidence/gates-1.8`, HEAD `9fe8baa`'s binaries built in the pinned image, one container under RULE3, caps read back: memory.max 8 GiB, swap 0, pids 8192):
- `aethelgard emit --file crates/aethelgard-cli/tests/fixtures/motors-pwm.json`: exit 0. The schematic carries each motor net `MOTOR1`–`MOTOR4` as one local label on one MCU pin (PA8–PA11), and nothing else on it.
- `aethelgard check erc`: kicad-cli exit 5, verdict fail, **four findings, each `isolated_pin_label` (warning) "Label connected to only one pin"**, one per motor net. The reference (`sentinel-01-minimal.json`) passes with none.
- `aethelgard check drc --spec … --schematic …`: exit 0, pass, parity included, for both.
- This agrees with `4300f`'s S2 (measured on three SPI nets, PR #134). **The gates do not pass an off-board net honestly**: only a `no_connect` on a used pin would silence ERC, which marks a driven pin unused. By §1.8 that is the STOP branch, and the kickoff applies it only if `4300f`'s connector build has not landed when the board side is reached: at `origin/main` `cd0bd31` (2026-11-15 05:40 UTC) it has not.

## 4. Decisions taken inside the work order

- **D2's quantum (F1).** Renode 1.17.0 has no configuration key for the quantum (§0), so the composition states it by `emulation SetGlobalQuantum "0.000100000"` (`renode::QUANTUM`), Renode's own value, and the judge reads the grid from `aethelgard_twin::boot::QUANTUM_NS`. Measured to change nothing (P3.1). A deviation from D2's words, recorded for the architect.
- **DShot timing source.** No standard document defines DShot. The bit rates and duty are ArduPilot's, at the tag `4400` pinned (`Copter-4.7.2`, `fb773567`): DShot300 and DShot600 are 300,000 and 600,000 bit/s (`libraries/AP_HAL_ChibiOS/RCOutput.cpp` L2521–2524), a bit is 8 ticks and a `0` bit is high for 3 of them, a `1` for 6 (`libraries/AP_HAL/RCOutput.h` L406–408, `DSHOT_BIT_WIDTH_TICKS_DEFAULT`, `DSHOT_BIT_0_TICKS_DEFAULT`, `DSHOT_BIT_1_TICKS_DEFAULT`). [read]
- **Where a motor output lives in the topology.** Its net is its label, and its pin, timer channel (`TIMn` / `TIMn_CHk`) and AF are an entry of `assignments`, so every generator that draws or configures solved pins sees it unchanged: the schematic emitter connects it to the MCU pin as it does every solved net (`aethelgard-eda` untouched, outside this fence). `motor_outputs` adds the protocol, the declared PWM timing and the solved `PSC`, `ARR`, `CCR`; `clock` is the reset-clock fact (§1.2). `SolvedTopology::validate` checks the shape (one `TIMn_CHk` per output of a timer with channels, each channel once, `CCR` below `ARR`, one protocol, rate, `PSC` and `ARR` per timer) and refuses any clock but the reset one; it does not judge the timing, which is the static check's.
- **The rounding rule and admissible error (D3's static check, `motor::check_silicon`).** PWM takes a 1 µs tick (`PSC` 63 at 64 MHz) and the nearest whole tick count to the declared period, half up, so `ARR` is that less 1; `CCR` is the idle pulse, the range's minimum (an ESC's throttle-off pulse; the WO names the idle pulse and the range, and nothing else could be it). DShot takes `PSC` 0, the bit the nearest whole count of 64 MHz ticks to `1 / bit rate` (107 at 600 kbit/s: 1671.875 ns, 5.2 ns long), and `CCR` the nearest whole count to 3/8 of it (40: 625 ns, exactly 3/8 of the nominal bit). The check, in exact integers with silicon's `ARR + 1`: the period within half a tick of the declared one; for PWM the high time exactly the idle pulse and the range's maximum strictly inside the period; for DShot the high time within half a tick of 3/8 of the nominal bit. 490 Hz rounds to 2041 µs (0.18 µs over, inside half a tick); below 16 Hz the period passes a 16-bit `ARR` and `solve` refuses it (`MotorTiming`).
- **The twin's witness rule (D4).** The judge anchors at the first rise (the model's first edge comes one period after a `CEN` that takes effect at its quantum's start, M2), predicts every later edge from the model's conventions (`ARR` and `CCR` ticks of `PSC + 1` cycles at 250 MHz, `aethelgard_twin::timer`, each with its `STM32_Timer.cs` line), and requires: at least two rises; edges alternating from a rise; every deviation from the prediction inside one span narrower than the 10 ns stamp grain (measured spread 8 ns, P7); no two edges on one stamp (M1); and at least one edge off the 100 µs grid. A sequence that fits with every edge on the grid is `motor_witness_static_only` (info), as D4 says, never a pass of the twin. The TIM read-back judges every field Renode stores (`CR1`, `DIER`, `CCMR1`, `CCMR2`, `CCER`, `PSC`, `ARR`, each used `CCRx`) and lists `BDTR.MOE` and each `OCxPE` as unobservable. `DIER` = 0 is judged, so a firmware that arms the update interrupt the `UG` + `BSRR` forgery used fails on its register.
- **Motor pin speed.** A timer pin's `OSPEEDR` is `00`, out of the 4300c speed check's scope with its own reason: the output leaves the board, its load is off-board, and no connector part declares a capacitance for it yet (`4300f`). A net constraint on a motor net is refused by name (`UnjudgedMotorConstraint`), as 4300c refused one no check reads. A question for the architect once connectors land (§6).
- **Observer names are reserved.** The platform is one namespace, so a device reference may not be `p<port><n>obs` (`aethelgard_ir::is_observer_name`, in the reference rule).
- **The clock assertion reads `TIMPRE` too.** The WO lists `RCC_CFGR.SWS` and the `D1CFGR`/`D2CFGR` prescalers; `TIMPRE` is in the same `CFGR` read and is part of the solved fact (Table 57), so the generated check requires it 0 as well. `RCC_CR.HSIDIV`, on which the 64 MHz depends, is not asserted: it is another register, outside the WO's list, and it measured 0 (P1.1). A question for the architect (§6).
- **The motor symptoms' rule, S4 (software).** `motor_output_dead`'s evidence is the pin's `AFR` field read back other than the solved AF (kind 11) beside the output's unwitnessed edges (kind 26). The AF is the firmware's own write, and the twin models nothing electrical on a timer's pin (no copper, no connector, no load), so the evidence allows software only. A motor symptom with no wrong owned register is `I4`, never software by elimination.

## 5. Decisions needed

- **D1, DShot in the campaign's boot (blocks only the DShot fixture's campaign clause of §1.7 and §2).** Every boot runs 100 ms of virtual time (`aethelgard_twin::boot::MAX_VIRTUAL_NS`, the campaign's `VIRTUAL_NS`). Measured (P5.2, M3): four DShot600 channels for 100 ms cost 10.8 s of timer events alone and, observed, 1.89 M lines and 139.6 MB, 17× `LOG_FILTER`'s 8 MiB, so the witness would be dropped and fail closed; on CI's runners the boot would also approach the 30 s wall bound (inferred). A firmware-ended burst cannot replace it: polling the timer collapses its stamps (M1), an interrupt reopens `RUL-19` (5)'s A2, and one-pulse mode gives no second rise to witness a period. Options:
  - **(a)** a stated, shorter fixed window for a boot whose topology has a DShot output (2 ms measured: 37,748 lines, 2.80 MB, 2.64 s on Furnace), the window named in the verdict. Cost: a DShot board's identity reads and droop schedules must fit it. **Recommended.**
  - **(b)** keep 100 ms and accept the DShot fixture's campaign as failing closed (the static check alone judges DShot).
  - **(c)** raise `LOG_FILTER` and `BOOT_BOUNDS` for DShot boards (not recommended: the bounds exist for floods).
  Built as far as no ruling is needed: the solver, the static check, the firmware, the Kani harness and the three-part witness all cover DShot, and `simulate --for 2ms` witnesses the DShot fixture (tier 2). `VIRTUAL_NS` is unchanged.

## 6. Questions for the architect (not stops)

- **Q1, the quantum's key (F1).** D2 asked for the quantum as a `--config` key; Renode 1.17.0 has none, so the composition states it by `emulation SetGlobalQuantum` at Renode's own 100 µs. Acceptable as the reading of D2?
- **Q2, `RCC_CR.HSIDIV`.** The 64 MHz the timers are solved for depends on `HSIDIV = 00`, which the generated firmware does not assert (the WO's list is `SWS` and the `D1CFGR`/`D2CFGR` prescalers; `TIMPRE`, in the same read, is asserted). Add it to the assertion in a later piece?
- **Q3, a motor pin's output speed** once a connector part declares the off-board load (`4300f`): judge the timer pin by the Fmax check at the output's toggle rate (≤ 1.2 MHz for DShot600), as 4300c does SCK and MOSI?
- **Q4, the seeded manifest's `spec` key.** A mutant may now name the specification whose generated board it patches (the motor case runs on the PWM fixture, whose board is not committed). The manifest stays at schema version 2 with the key optional; a version bump instead?
