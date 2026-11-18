# AET-WO-4400f FEEDBACK — the twin's timer and USART clocks

## §1.1 Read at source, before any run

**Renode (the pinned build).** Renode `renode-infrastructure` `cd4b002a` (2026-09-04), plus the three NVIC patches of `AET-WO-1605`, which touch no clock code. The files are fetched by commit to `~/.cache/aet-4400f/src`.

- **`STM32H7_RCC` already propagates a clock.**
  - It computes `sys_ck` from HSI (`HSIDIV`), CSI, HSE (a constructor default of 8 MHz) or PLL1 (`DIVM1`, `DIVN1`, `DIVP1`; `PLL1FRACEN` is a tag) (L84–130, L1514–1560).
  - It pushes `IHasFrequency.Frequency` to every peripheral registered with it, through `PropagateClock` (L1501):
    - APB1 timers TIM2–7 and TIM12–14 (L1242), and APB2 timers TIM1, TIM8 and TIM15–17 (L1255), at `sys_ck / (D1CPRE · HPRE · k)`. Here `k` is RM0433 Table 57 with `TIMPRE` (L1480–1499).
    - APB1/APB2 peripherals at `pclk1`/`pclk2`. This is "a simplification", with no kernel-clock mux; USART1 is in the APB2 list (L1286).
- **When it propagates.**
  - Only in the change callbacks of `RCC_CR`, `RCC_CFGR`, `RCC_D1CFGR` and `RCC_D2CFGR` (L206, L239, L249, L260), and in the RCC's own `Reset()` (L81).
  - The constructor runs `Reset()` before any peripheral registers (L37–39). So at load, nothing is pushed. A firmware that never writes those four registers keeps every timer at its `.repl` `frequency`.
- **The stock platform registers every timer and `usart1` with the RCC** (`stm32h7.repl` L298–L459, `rcc RegistrationPeripheralAlias.TIM1` …). The timers are 250 MHz, and `usart1`/`usart6` are 200 MHz (L295, L302).
- **Why ArduPilot's TIM5 still counts at 250 MHz:**
  - `STM32_Timer` is a `LimitTimer` (`ITimer : IHasFrequency`), so it accepts the push.
  - `LimitTimer`'s reset puts the frequency back to its constructor value (`LimitTimer.cs` L324, `frequency = initialFrequency`).
  - An RCC peripheral reset (`ResetRegisteredPeripheral`, L1377) calls the peripheral's `Reset()` and does not propagate again.
  - ChibiOS resets TIM5 through `APB1LRSTR` when it starts the system timer. The 200 MHz pushed when ArduPilot set its clocks is therefore undone [inferred; measured in P2].
  - Upstream fixed exactly this on 2026-11-05: `f2ac99657` ("Propagate clocks at peripheral reset", GH#1009), and `b14469bba` ("Propagate clock during peripheral registration"). No release carries them; the latest is v1.17.0, 2026-09-07.
- **A timer's capture/compare channels never follow.**
  - Each channel is its own `LimitTimer`, built with the constructor `frequency` (`STM32_Timer.cs` L104).
  - `STM32_Timer` does not override `Frequency`, and only `Divider` is copied to the channels (L355). This is unchanged on upstream master today.
  - So under propagation, the counter (update events, `CNT`) runs at the new clock, while every compare (a PWM fall, a `CCxIF`) still counts at 250 MHz.
- **`STM32F7_USART` cannot follow.** It is not `IHasFrequency`: `frequency` is a `readonly` constructor value (L21–26, L439), on upstream master too.
  - It is used only for `BaudRate` (L50) and the receiver timeout (L129) in this build.
  - Upstream's `88afd12bb` (2026-09-21, after the pin) begins to rate-limit received characters at `BaudRate`, so it would start to matter.

**RM0433 Rev 9 (the clock paths).**
- **Timers:**
  - TIM2–7 and TIM12–14 are clocked from `rcc_timx_ker_ck` (APB1), and TIM1, TIM8 and TIM15–17 from `rcc_timy_ker_ck` (APB2).
  - Table 57, with `TIMPRE` 0: `rcc_hclk1` when `D2PPREx` ≤ 2, else `hclk / (D2PPREx / 2)`. With `TIMPRE` 1: `hclk` up to ÷4, then `hclk / (D2PPREx / 4)`. Renode's lookup (L1480) is this table.
- **USART1/6:** the kernel clock is `D2CCIP2R.USART16SEL`. Its reset value 0 is `rcc_pclk2`; the others are `pll2_q`, `pll3_q`, `hsi_ker`, `csi_ker` and `lse`.

**What each firmware programs:**
- **The generated firmware never leaves the reset clock.** `SolvedClock::reset()`: HSI 64 MHz, every prescaler 1, `TIMPRE` 0 (`aethelgard-ir` `motor.rs` L95–150; `validate` refuses any other clock).
  - Silicon's timers and USART1 therefore run at 64 MHz, and every motor `PSC`/`ARR`/`CCR` is solved for 64 MHz (`solve.rs` L841).
  - The twin counts the same values at 250 MHz, 3.906× fast. The judge predicts at `TIMER_HZ` 250 MHz (`aethelgard-twin` `timer.rs` L31, `CYCLE_NS` 4) and so passes.
- **ArduPilot (`OSCILLATOR_HZ 0`; ChibiOS `stm32h7_mcuconf.h` in the pinned ArduPilot image):**
  - PLL1 is HSI ÷4 ×50 ÷2, so `sys_ck` = 400 MHz. `HPRE` ÷2, `D2PPRE1` and `D2PPRE2` ÷2, `TIMPRE` 1, so every timer runs at 200 MHz.
  - `USART16SEL` is `PLL2_Q`: HSI ÷8 ×45 ÷5 = 72 MHz. 72 MHz ÷ `BRR` 625 (`0x271`) = 115,200 baud, exactly.
  - Silicon: TIM5 at 200 MHz, USART1 at 72 MHz. The twin: 250 and 200.

**`TIMER_HZ`'s users:**
- `aethelgard-twin` `timer.rs` (`CYCLE_NS`, `period_ns`, `high_ns`) and its unit asserts (L375–378);
- `aethelgard-cli` `renode.rs` L1739 (a finding's text);
- `aethelgard-chaos` `campaign.rs` L2377 `TWIN_CYCLE_NS` 4 (a labelled `belt-and-suspenders` copy);
- `tests/timer_routes.rs` L81 (the stock file's `frequency` line);
- `tests/motor_platform.rs` L67 (`424`/`160` ns);
- `aethelgard-evolve` `tests/motor.rs`, and `aethelgard-chaos` `tests/serial.rs` (`639_744`).
- At 64 MHz a tick is 15.625 ns, not a whole number of nanoseconds, so `CYCLE_NS` cannot stay an integer. A DShot600 bit of 106 ticks is 1,656.25 ns.

**The Renode build `simulate` checks today:**
- It refuses any version line but `Renode v1.17.0` (`main.rs` L1710, `renode_pinned`), for `--firmware` and `--foreign` alike.
- It parses the `build:` line (`renode.rs` L3576 `build`) and records it in a campaign archive's header (`2205`). It never compares the build: any 1.17.0 build runs.

## §1.2 Predictions (committed before any measured run)

| # | Prediction |
|---|---|
| P1 | **Baseline** (pinned image, stock emission): ArduPilot's clock runs 1.25× virtual time; the witnesses are those of 4400e/1605 (last 3,610,548,200 ns); the generated verdicts equal main's byte for byte. |
| P2 | **The mechanism:** TIM5's `Frequency` reads 200,000,000 after ArduPilot's clock setup and 250,000,000 again after ChibiOS's TIM5 reset (`APB1LRSTR`). If it never reads 200 MHz, `PropagateClock` was not reached (`sys_ck` null), and P2 is wrong. |
| P3 | **(b) RCC-following, emulated** with a monitor hook that does what upstream `f2ac99657` + `b14469bba` do: propagate after load and after each RCC peripheral reset. ArduPilot: TIM5 counts at 200 MHz, so its clock runs 1.00× (±0.5 %). It still boots and passes, although TIM5's compare channel runs at 250 MHz (ChibiOS tolerates an early alarm by re-arming). The witnesses move later by 1.15–1.25×: the last about 4.2–4.5 s, so `--foreign 4000ms` fails with `spi_transaction_unwitnessed` and the bound must move (about 5 s). `usart1` stays at 200 MHz. |
| P4 | **(b) on the generated firmware:** propagation at load sets every timer to 64 MHz (HSI), but the channels stay at 250 MHz. A PWM high time becomes `CCR` ticks at 250 MHz inside a period at 64 MHz, so the motor judge's verdict for serial-dshot-9600 changes (fails, or moves to `static_only`) even with `TIMER_HZ` set to 64 MHz. Sentinel-02 (no motor) is byte-identical apart from what `usart1`/`BaudRate` touches: nothing, predicted identical. |
| P5 | **(b′)** = (b) plus the channels retuned with their timer (a Renode change that no upstream commit has): serial-dshot-9600 passes with `TIMER_HZ` 64 MHz. Its motor findings move to periods 3.906× longer (`period_ns` 1,656.25 vs 424, not whole ns). ArduPilot as P3. |
| P6 | **(a) static per board, from the solved clock** (the emission adds `frequency:` to each timer and to `usart1`, from `SolvedClock`), with the judge at the same frequency: the generated verdicts pass; the motor findings' ns move ×3.906; Sentinel-02's bytes are unchanged except any text naming 250 MHz. On ArduPilot, (a) would be 64 MHz, the wrong clock: its clock would run 0.32× and the witnesses move about 3.9× later (past 4 s, past 10 s). So (a) alone cannot serve `--foreign`. |
| P7 | **(c) a declared foreign clock** (timers 200 MHz, `usart1` 72 MHz, ArduPilot's own as read above): the same as P3 for the clock and the witnesses (1.00×, last about 4.2–4.5 s), and `BaudRate` reads 115,200. No hook is needed, and the channels are consistent because the frequency is set at construction. |
| P8 | **Wall time:** (b)/(c) on ArduPilot +10–25 % against 10–12 s (more virtual time to the last witness); (a) on the generated firmware within ±10 %. (b)'s hook adds no measurable time. |
| P9 | **Pinned build:** `simulate`'s verdict records no Renode build today (only the campaign archive's header does), and a stock 1.17.0 runs (the image before 1605). On that stock build, ArduPilot `--foreign` hard-faults (`core_in_exception`, the RETTOBASE fault) and the generated firmware's verdicts are unchanged. |
| P10 | **Recommendation (predicted):** (a) for the generated firmware plus (c) for a foreign one: a static frequency per board, set at construction. (b) needs Renode changes, two unreleased upstream commits plus a channel fix nobody has written, and still cannot follow USART16SEL; it is backlog, upstream. Pinned build: refuse by name a `build:` other than the pinned one, which `2205`'s any-build test allows; that test stays about the header's format. |

**Added before any run** (after a review of the plan; still no measured run):

| # | Prediction |
|---|---|
| P11 | **An update entry reaches the constructor.** `timerN:` / `usart1:` with `frequency: F`, appended to the emission, sets the constructor value, as `stm32h743.repl` already does for `nvic: systickFrequency: 480_000_000`. `timer5 Frequency` and the channels then read F at load, and `usart1 BaudRate` reads `F / BRR`. |
| P12 | **SysTick and DWT.** SysTick loads at 480 MHz (`stm32h743.repl`). ArduPilot's clock writes push 400 MHz into it (`UpdateSystick`, which needs no registration); ArduPilot ticks on TIM5, so nothing visible moves. The generated firmware leaves it at 480 MHz under the baseline, (a) and (c). (b) moves it to 64 MHz at load: an `INV-07` item for every candidate, and P4's "Sentinel-02 identical" holds only if the generated firmware never reads SysTick. DWT stays at 250 MHz in this build; it follows the RCC only after upstream `3423667aa`. |
| P13 | **"Static" is not static here.** Under (a) and (c) the stock RCC still pushes on `CR`, `CFGR`, `D1CFGR` and `D2CFGR` writes. On ArduPilot a timer that is not reset afterwards runs at the RCC's figure, 200 MHz, the same as declared. So for ArduPilot it is invisible, and it is read back for every timer ArduPilot uses. The generated firmware writes none of the four, so the declared value holds. |
| P14 | **`machine Reset`** (droop runs reset the machine): the RCC's `Reset()` propagates the HSI clock, and each timer's `Reset()` restores its constructor value, so the result depends on the order. Predicted: the timers read their constructor value after a reset (250 MHz on the baseline). Measured. |
| P15 | **The 4 s cap.** `--foreign` above 4000 ms is refused (`time_up_to`, `FOREIGN_MAX_VIRTUAL_NS`), so measuring the window under (b)/(c) needs that bound raised in the scratch tree: a labelled deviation. The `3500ms`/`3590ms` cases of `ci/foreign-judge.sh` move with it. |

## §2 Measured (2026-11-18, Furnace, the pinned tools image `e5fc7ddb`, build `.aet1605`)

**How it was measured.** Everything here is under `spike/4400f/`; runs and logs are in `~/.cache/aet-4400f/runs`.
- **The candidates are one CLI patch** (`patches/cli-candidates.patch`), applied to a scratch copy of this branch and never to it. Each knob is a labelled deviation, read from the environment:
  - `AET4400F_CLOCK=solved` (a): update entries from `SolvedClock`.
  - `AET4400F_CLOCK=declared:…` (c): the declared values.
  - `AET4400F_FOLLOW=b|bp` (b, b′): a monitor hook (`py/clock.py`) that does what upstream `b14469bba` and `f2ac99657` do. It propagates the RCC's clock after the load and at each timer's next `CR1` write after a reset. `bp` also retunes the timer's capture/compare channels.
  - `AET4400F_TIMER_HZ` sets the judge's frequency; `AET4400F_CEIL` see below; `AET4400F_FOREIGN_MAX_NS` raises the 4 s cap (P15).
- **The patched CLI with no knob** reproduces main byte for byte (control `ctl`; only `memory` and the input path differ).
- **Each generated firmware ELF was built once by main's CLI and reused** for every candidate.
- **Noise floor:** two baseline runs of each verdict differ only in `memory`.
- **Host load:** the `1314` builder ran beside this, at a load average of 4–26 on 20 cores, so wall times are ±30 %. One baseline `--foreign` run was killed at the RSS cap (finding F1).
- **The driver** (`drive.py`, from 4400e's) boots ArduPilot from the CLI's own captured monitor script (`sh/capture.sh`), and reads each timer's `Frequency`, the SysTick's, `usart1 BaudRate` and the RCC's registers every step.

### The mechanism (P2, P11–P14)

- **ArduPilot's TIM5 is pushed to 200 MHz, then reset to 250 MHz** (RCC log at Debug, `runs/p2/console.log`):
  - at 3.77 µs it is pushed to 32 MHz;
  - at 3.98 µs it is pushed to 200 MHz, and SysTick to 400 MHz;
  - at 0.900 ms ChibiOS resets every timer (`Resetting TIM2` … `TIM17`);
  - from 2 ms on, `timer5 Frequency` reads 250,000,000.
- **An update entry reaches the constructor:** `timer5: frequency: 200000000` reads 200 MHz at load, and so does its channel 1's compare timer. `usart1: frequency: 72000000` gives `BaudRate` 72 MHz ÷ `BRR`.
- **SysTick loads at 64 MHz, not 480 MHz.** The RCC's constructor `Reset()` pushes HSI into the `systick` it is handed, after `stm32h743.repl` sets 480 MHz. ArduPilot's writes move it to 400 MHz. DWT stays at 250 MHz.
- **`machine Reset`:** each timer reads its constructor value afterwards (250 MHz on the stock emission, the declared value under an update entry); SysTick reads 64 MHz.
- **A propagated timer's channels stay at 250 MHz** (`propagate()` then `chan_freq`: `timer2` at 64 MHz, its channel 1 at 250 MHz). `retune()` fixes them.
- **Renode realizes a timer's period in whole nanoseconds, rounded up:**
  - Virtual time's resolution is 1 ns (`TimeInterval.cs` L291, `TicksPerNanosecond = 1`).
  - At the limit the residue is dropped and the time to the next limit is rounded up (`BaseClockSource.cs` L315–L325).
  - At 250 and 200 MHz a tick is a whole number of ns, so this is exact. At 64 MHz a DShot600 period of 106 ticks (1,656.25 ns) runs as 1,657 ns. The measured mean over 1,205 periods is 1,657.004 ns.
- **Renode drops the kernel-clock selections.** ArduPilot writes `RCC_D2CCIP2R` `0xE01009` (`USART16SEL` and `USART234578SEL` = `PLL2_Q`, `I2C123SEL` = `PLL3_R`). Renode's RCC holds those fields as tags, logs "Unhandled write to offset 0x54", and reads the register back as 0.
  - The registers that set the timers' clock are modelled and read back: `PLLCKSELR` `0x408040`, `PLL1DIVR` `0x01090231`, `D1CFGR` `0x48`, `D2CFGR` `0x440`, `CFGR` `0x801B`. They decode to 400 MHz `sys_ck` and 200 MHz timers.

### ArduPilot (`simulate --foreign`; `sentinel-02-own`; CI's ELF `a799dfb6`)

| Candidate | ArduPilot µs / virtual µs | `usart1 BaudRate` | U4 identity | Barometers (U2) | U5 identity | U5 FIFO read (last) | 4000 ms | 5000 ms | Peak RSS (‰ of 1 GiB) | Wall |
|---|---|---|---|---|---|---|---|---|---|---|
| Baseline (main) | 1.25000 | 320,000 | 161,938,610 | 342,585,230 | 3,569,253,830 | 3,610,548,200 | pass | (refused) | 780–783 at 4 s | 11.3–13.7 s |
| (a) solved, 64 MHz | 0.32000 | 102,400 | 167,137,100 | 858,765,570 | not by 4 s | not by 4 s | fail | – | 704 at 4 s; **killed at the cap at 16 s** | 9.6 s |
| (b) RCC-following, channels at 250 MHz | 1.00000 | 320,000 | 162,829,790 | 387,288,980 | not by 4 s | – | fail | **killed at the cap** (twice, 5 s and 6 s) | 943 at 4 s | 13.5–15.2 s |
| (b′) (b) + channels retuned | 1.00000 | 320,000 | 161,141,150 | 385,637,340 | 4,342,643,780 | 4,393,959,380 | fail | pass | 912 at 5 s, 946 at 6 s | 12.9 s at 5 s |
| (c) declared: timers 200 MHz, `usart1` 72 MHz | 1.00000 | 115,200 | 161,141,150 | 385,637,340 | 4,342,643,780 | 4,393,959,380 | fail | pass ×2 | 771–811 at 5 s, 881 at 6 s | 12.0–12.1 s at 5 s |

- **(c) and (b′) give identical witness times to the ns.** The last witness moves from 3.611 s to 4.394 s, which is 1.217×: not the full 1.25×, because the code between delays takes the same virtual time either way.
- **(c) at 5 s keeps 1.56 MB of log** (the budget is 8 MiB); at 6 s it keeps 3.73 MB.
- **CI's other foreign cases move with the bound.** `window-before-u5` (3500 ms today) holds at 4300 ms: U5 is unwitnessed. `window-before-u5-data` (3590 ms) holds at 4370 ms: the identity is witnessed, the data read is not. `ci/foreign-judge.sh` as written fails its positive case under (c).
- **(b) alone executes 29–34 % more instructions per 100 ms than (b′):** 734–761 k against 569 k (driver, `clk-b`). TIM5's counter counts at 200 MHz while its compare channel counts at 250 MHz, so ChibiOS's tickless alarm fires early and is re-armed. Every (b) run of 5 s or more was killed at the RSS cap.

### The generated firmware (`simulate --firmware … --for 100ms`; ELFs built once by main's CLI)

| Candidate | sentinel-02-own | serial-dshot-9600 (4 × DShot600) | motors-dshot600 |
|---|---|---|---|
| Baseline | pass | pass: 4,716 rises in the 2 ms window, period 424 ns, high 160 ns, span 8 ns | pass, as serial-dshot-9600 |
| Stock 1.17.0 build (pre-1605 image) | byte-identical | byte-identical | byte-identical |
| (a) solved 64 MHz, judge at 250 MHz (main's) | pass; only `input.sha256` moves (the platform's bytes) | **fail**, `motor_timing_mismatch` ×4: span 1,486,230 ns | **fail**, the same |
| (b) / (b′), judge at 64 MHz, floor ns | pass | **fail**: (b) span 1,675 ns; (b′) span 1,211 ns | **fail**, the same |
| (b) / (b′), judge at 64 MHz, exact (ps) | pass (b′: byte-identical) | **fail**: (b) span 1,373 ns (high time at 250 MHz); (b′) span 910 ns (Renode's whole-ns rounding) | **fail**, the same |

(a) and (b′) produce the same edges: the same `rises`, `first_ns` and `last_ns` on every output. So (a) under the floor and exact judges is (b′)'s row; it was run under the 250 MHz and the rounding judges only.
| (a) and (b′), judge at 64 MHz with Renode's rounding (`AET4400F_CEIL`) | pass; (a) moves `input.sha256` only | **pass**: 1,206 rises, period 1,656 ns (judged as 1,657), high 625 ns, span 9 ns | **pass**, the same |

- **What moves under (a), byte for byte** (`py/diffv.py` against the baseline): in each motor record `twin.rises`/`falls` (4,716 → 1,206), `period_ns` (424 → 1,656), `high_ns` (160 → 625), `first_ns`, `last_ns`, `off_grid` (9,431 → 2,411) and `span_ns` (8 → 9), plus `input.sha256`. Nothing else moves: the serial port's echo, the I2C/SPI devices and every finding are unchanged.
- **The silicon block** (`kernel_hz` 64,000,000, `period_ps` 1,671,875) is already the silicon's and does not move.

### The pinned build (`1605` D1)

- **What `simulate` records today:** the verdict's `tool` is `{"name": "renode", "version": "Renode v1.17.0", "exit_code": 0}`. No build. Only a campaign archive's header names the build (`2205`).
- **Main's unmodified CLI on the image before 1605** (`598501ab`, `build: 1.17.0+20260907gitf1dd1b4af`, stock):
  - sentinel-02-own, serial-dshot-9600 and motors-dshot600 are byte-identical to the pinned build's (only `memory` and the path differ);
  - ArduPilot `--foreign 4000ms` **fails**: `core_in_exception` ("the run ended in exception 3 (ICSR 0x00000803)", a HardFault, the RETTOBASE fault) and `spi_transaction_unwitnessed`.
  - The verdict's `tool` is identical on both builds, so nothing in either verdict says which build judged it.
- **Proposal: record and refuse.**
  - Add `tool.build` to every verdict.
  - Refuse, by name, a `build:` other than the pinned one for every boot, as `renode_pinned` already refuses a version other than `Renode v1.17.0` (`main.rs` L1710). The refusal would be named like `RenodeBuildNotPinned`.
  - Keep the pinned build as one constant beside `RENODE_VERSION` (`aethelgard-twin` `platform.rs`). `twin_tier2`'s `PINNED_RENODE_BUILD` becomes a use of it, and `smoke.sh`'s literal stays tied to it by a test.
  - **What it moves:** the tier-1 stubs report main's stock build string: `boot_tier1.rs` L321, L1627, L2073 and L2350, and `campaign_tier1.rs` L283 `BUILD`. They move to the pinned string. `2205`'s any-build test keeps checking that the header names the build Renode reported, now the pinned one, and one tier-1 test refuses a stub that reports the stock build.
  - **Why not warn:** measured, a stock build changes no generated verdict but flips the foreign one to a hard-fault failure that no field explains. A warning would sit beside a `fail` that reads like the firmware's.
  - **Why not record only:** it leaves the stock build judging, and a reviewer would have to know to look.

### Predictions, scored

| # | Outcome | Score |
|---|---|---|
| P1 | 1.25000×; last witness 3,610,548,200; generated bytes equal (control and noise floor). One of two baseline `--foreign` runs was killed at the RSS cap under host load (F1) | held |
| P2 | 200 MHz pushed at 3.98 µs, ChibiOS's reset at 0.900 ms, 250 MHz again from 2 ms | held |
| P3 | 1.00000× and the bound must move (last witness 4.394 s, inside 4.2–4.5). **(b) alone does not pass:** its compare channel at 250 MHz costs 29–34 % more instructions, and every run of 5 s or more was killed at the RSS cap | partly; missed (b)'s failure |
| P4 | (b) fails the motor judge on the generated firmware (span 1,373–1,675 ns); Sentinel-02 passes | held |
| P5 | **Missed.** (b′) does not pass with the judge at 64 MHz: Renode realizes each period in whole ns rounded up (1,657 ns, not 1,656.25), so the judge must model that rounding. With it, (b′) passes | missed |
| P6 | Generated: passes with the judge at 64 MHz **plus the rounding**; ns ×3.906 (424 → 1,656); Sentinel-02 moves only the platform's sha256. ArduPilot: 0.32000×, the barometers 2.5× later, no U5 by 4 s; the run to 16 s was killed at the RSS cap, so "past 10 s" is unmeasured | held, but for the rounding |
| P7 | 1.00000×, witnesses identical to (b′), `BaudRate` 115,200, no hook | held |
| P8 | (c) at 5 s: 12.0–12.1 s against 11.3 s at 4 s (+7 %, below the predicted +10–25 %, with host-load noise ±30 %). (a) on the generated firmware: within noise. The hook costs no measurable wall time, but costs RSS: (b′) 912‰ against (c)'s 771–811‰ at 5 s | partly |
| P9 | No build in the verdict; a stock build runs; ArduPilot hard-faults; the generated verdicts are unchanged | held |
| P10 | (a) + (c), as measured | held (§3) |
| P11 | Update entries reach the constructor and the channels | held |
| P12 | **Wrong.** SysTick loads at 64 MHz, not 480 MHz: the RCC's constructor pushes HSI into it after `stm32h743.repl`'s value. ArduPilot moves it to 400 MHz. DWT 250 MHz | wrong |
| P13 | ArduPilot under (c): every timer reads 200 MHz at the end, the declared value and the RCC's both, so it is invisible as predicted | held |
| P14 | Constructor values after `machine Reset` | held |
| P15 | The cap needed raising. CI's 3500 ms and 3590 ms cases move to 4300 ms and 4370 ms | held |

## §3 For the ruling

### Recommended: (a) for the generated firmware, (c) for a foreign one: one mechanism, a `frequency` set at construction

- **The mechanism.** The emitter writes an update entry, `timerN: frequency: F` and `usartN: frequency: F`, as `stm32h743.repl` already does for `nvic`.
  - For the generated firmware, F is the solved clock's kernel frequency (`SolvedClock::timer_kernel_hz`, `serial::kernel_hz`): 64 MHz on today's reset clock, the frequency silicon runs.
  - For `--foreign`, F comes from a clock declared beside `--foreign`. Whether it is a flag or a specification field is the architect's call (Q1). For ArduPilot: timers 200 MHz, `usart1` 72 MHz.
- **No Renode change is needed** for either, and no hook: the counter and its channels are built at F.
- **Measured:**
  - ArduPilot runs 1.00000× virtual time with `BaudRate` exactly 115,200.
  - The generated firmware passes the motor judge at its silicon clock.
  - A Sentinel-02 without motors moves only its platform's bytes.
- **(b) is not recommended.**
  - It needs two upstream commits no release carries (`b14469bba`, `f2ac99657`), plus a channel fix nobody has written (§3.3).
  - It still cannot follow a USART: `STM32F7_USART` is not `IHasFrequency`, and the kernel-clock mux is tags.
  - Its partial form (the upstream commits alone) is worse than today: early compares, +29–34 % instructions, killed at the RSS cap.
  - It goes to backlog, upstream: §3.3 holds the patches.

### What the recommendation moves

- **`TIMER_HZ` stops being a constant.**
  - Each observer's frequency is its board's kernel clock: 64 MHz for every generated board today.
  - The judge predicts each edge at ⌈ticks × 10⁹ / F⌉ ns, the period Renode realizes (`BaseClockSource.cs` L315–L325). At 64 MHz a tick is 15.625 ns, so `CYCLE_NS` cannot stay an integer.
  - `TIMER_HZ`'s users (§1.1) all move: `timer.rs` `period_ns`/`high_ns` and their unit asserts, `renode.rs` L1739's text, `campaign.rs`'s labelled `TWIN_CYCLE_NS` copy, `timer_routes.rs` L81, `motor_platform.rs` L67, and `aethelgard-evolve`'s and `aethelgard-chaos`'s motor fixtures.
- **Every generated motor record:** `rises`/`falls` (4,716 → 1,206 in the 2 ms DShot window), `period_ns` 424 → 1,656 (judged as 1,657), `high_ns` 160 → 625, `first_ns`, `last_ns`, `off_grid`, `span_ns` 8 → 9.
  - At 64 MHz the 2 ms window holds 1,206 DShot600 periods, enough for the judge (≥ 2 rises).
  - **PWM (`motors-pwm`, measured):** it passes under (a).
    - The judge's PWM window is the whole boot (`MAX_VIRTUAL_NS`), so the 2 ms DShot window does not apply to it.
    - Per output: `rises` 156 → 40, `period_ns` 639,744 → 2,499,000, `high_ns` 256,000 → 1,000,000, `first_ns` 661,480 → 2,520,730, `off_grid` 311 → 79, `span_ns` 8 → 0.
    - The tick is exactly 1 µs, so there is no rounding.
    - `twin_tier2`'s `rises >= 150` floor for PWM moves with it, since 100 ms holds 40 periods.
    - Evidence: `spike/4400f/out/pwm/`.
- **Replay and the archive (F6, measured).**
  - `aethelgard-chaos`'s `TWIN_CYCLE_NS` (`campaign.rs` L2377, labelled belt-and-suspenders) refuses every 64 MHz motor record: `witness_consistent` requires `period_ns == ARR·(PSC+1)·4`. So `campaign` exits 70 ("the campaign built a run replay refuses").
  - This failed 7 tests: the DShot600 campaign, `m1`, and all 5 `attribute_seeded` tests, through the seeded `motors-pwm` scenario.
  - Replay checks an archive without the twin, so the archive must carry the clock: kind 25 (`MotorOutputSolved`) gains the kernel frequency, which is a trace-schema bump.
  - **The simpler form:** record `period_ns`/`high_ns` as Renode realizes them, ⌈ticks·10⁹/F⌉ (1,657 for DShot600 at 64 MHz). Then the judge, `witness_continuous` and replay all stay in integer ns, with one rule shared by the twin and replay. The spike's picosecond fields were measuring aids and are not needed.
- **Every emitted platform's bytes** (the update entries), and so every pinned platform hash or golden that embeds them.
- **The suite under (a), measured** (`spike/4400f/out/suite/classified.tsv`):
  - Method: `cargo test --workspace --features aethelgard-cli/with-tools` in a scratch tree with the switch compiled in.
    - An environment switch is not enough: 24 test sites `env_clear()` the CLI child, which then emits main's platform while the test process emits (a)'s. §2's runs called the CLI directly, so they are unaffected.
  - The committed `boards/sentinel-01*/…repl` were re-blessed with `AETHELGARD_BLESS`: +62 lines each, the update entries only (`out/suite/bless-a.diff`).
  - After the re-bless, 25 tests fail, every one a moved figure, none a judged behaviour:
    - 7: replay (above).
    - 6: `twin_tier2` motor-record goldens (424/160, 639,744/256,000, the PWM rise floor).
    - 6: `renode::tests` judge fixtures written at 250 MHz.
    - 3: `aethelgard-twin` platform-text goldens.
    - 2: `motor_platform.rs`.
    - 1: `order_2203.rs`'s pinned platform sha256.
  - The control (no switch) passes these targets, apart from the provenance test, which follows the re-blessed board.
  - Before the re-bless, the board goldens failed 71 more tests through `PlatformNotEmitted`, 82 in all.
- **The foreign window:** `FOREIGN_MAX_VIRTUAL_NS` from 4 s to 5 s, on the measured last witness of 4.394 s.
  - At 5 s: RSS 771–811‰ of the 1 GiB bound (78 % today at 4 s), 1.56 MB of the 8 MiB log, wall 12.0 s.
  - 6 s would cost 881‰ and 3.73 MB.
  - `ci/foreign-judge.sh`: 4000 ms → 5000 ms, 3500 ms → 4300 ms, 3590 ms → 4370 ms.
  - The witness times move: U4 0.1619 → 0.1611 s, the barometers 0.343/0.383 → 0.386/0.436 s, the ICM 3.569/3.611 → 4.343/4.394 s. So do architecture §2.27's figures.
- **`usart1`'s `BaudRate`** (not judged today: the foreign judge drops `BRR`) becomes exact: 115,200 for ArduPilot, `64 MHz / BRR` for the generated firmware.

### `INV-07`: where each candidate can still run a clock silicon would not

| Candidate | Remaining divergence |
|---|---|
| (a) | (1) Renode's whole-ns rounding: +0.75 ns per DShot600 period at 64 MHz (0.045 %). The judge must model it, so it is a model convention like `ARR` against `ARR + 1`, and the silicon block judges the real period. (2) The RCC still pushes a clock on `CR`/`CFGR`/`D1CFGR`/`D2CFGR` writes. A generated firmware that programmed its clock would get counters at the RCC's figure and channels at F: the (b) failure. Today `validate` refuses any clock but the reset one, so this is latent; a clock-programming engine must handle it. (3) DWT counts at 250 MHz (silicon 64). (4) `usart` frequency drives only `BaudRate` and the receiver timeout in this build. Upstream `88afd12bb` paces received characters at `BaudRate`, so a wrong F would start to matter at the next Renode. (5) (a) is wrong for a foreign firmware (ArduPilot 0.32×), so it must never apply to `--foreign`. |
| (b) | (1) USARTs cannot follow. (2) The RCC model's limits: HSE is a constructor default of 8 MHz unless the platform declares it; there is no fractional PLL; PLL2 and PLL3 are not computed; the kernel-clock muxes are tags. (3) Without the channel fix, compares run at the stale frequency (measured). (4) A clock is pushed only through the four registers it watches. |
| (c) | (1) **The declaration is the user's.** A wrong one runs the twin at a clock silicon would not, which is exactly `INV-07`. Proposed cross-check, measured feasible: at the run's end, decode the timers' kernel clock from the registers Renode models and reads back (`PLLCKSELR` `0x408040`, `PLL1DIVR` `0x01090231`, `D1CFGR` `0x48`, `D2CFGR` `0x440`, `CFGR` `0x801B` → 400 MHz `sys_ck`, 200 MHz timers), and refuse a mismatch by name. A USART's kernel clock cannot be read back (`D2CCIP2R` reads 0); only the RCC's "Unhandled write to offset 0x54 … 0xE01009" log line carries it, so it is either judged from that line or declared unchecked (Q2). (2) A timer not reset after an RCC push runs at the RCC's figure while its channels stay at F; the cross-check covers it. For ArduPilot both are 200 MHz. (3) At 200 MHz a tick is 5 ns, so there is no rounding. |

### §3.3 The Renode change (b) would need (the WO's §7 stop condition, for (b) only)

All three files are under `spike/4400f/patches/`.
- **`b14469bba…patch` and `f2ac99657…patch`** (upstream, 2026-11-05: propagate at registration and after a peripheral reset). Both apply to `cd4b002a` with `patch --dry-run`, offset −6 lines, despite `3423667aa` in between.
- **`stm32-timer-channels-follow-frequency.patch`** (ours; no upstream commit). `STM32_Timer` re-implements `IHasFrequency` with a `new Frequency` that sets the counter and each capture/compare channel's `LimitTimer`.
  - It applies to `cd4b002a` (dry run). It has **not been compiled**.
  - Its behaviour is what the `bp` hook does by reflection, which is measured above.
- **Not covered by any patch:** USART kernel clocks (`STM32F7_USART` keeps a `readonly` frequency), the kernel-clock muxes (tags), fractional PLLs, and PLL2/PLL3.

### Findings

| # | Finding | Since | For |
|---|---|---|---|
| F1 | The foreign judge's 1 GiB RSS cap killed one of two baseline `--foreign 4000ms` runs, and `ci/foreign-judge.sh`'s positive case in the same container. Host load was 26 on 20 cores, wall 29.7 s against 13.7 s; the run that passed peaked at 780‰. RSS grows with wall time under load, so the cap's margin is thinner than 4400e's 833 MiB suggests. | `4400e` | backlog: measure RSS against wall time, or bound it differently |
| F2 | Renode's RCC drops the D2 kernel-clock selections: `D2CCIP2R` reads 0 after ArduPilot writes `0xE01009`. A judge that reads the RCC sees the reset muxes, not the firmware's. | Renode 1.17.0 | upstream; (c)'s cross-check (Q2) |
| F3 | Renode realizes each `LimitTimer` period in whole ns, rounded up, so a timer whose tick is not a whole number of ns runs slow. Today's 250 MHz (4 ns) hid it. | Renode 1.17.0 | the judge (§3) |
| F4 | SysTick runs at 64 MHz from load, not `stm32h743.repl`'s 480 MHz: the RCC's constructor pushes HSI. It matches silicon's reset clock; nothing in the repository says so. | Renode 1.17.0 | the record |
| F5 | A timer's capture/compare channels never follow a frequency change (`STM32_Timer.cs` L104, unchanged on master). | Renode 1.17.0 | upstream (§3.3) |
| F6 | Replay's `TWIN_CYCLE_NS` copy (`aethelgard-chaos` `campaign.rs` L2377) states the 250 MHz clock as a constant. Any per-board timer frequency makes `campaign` exit 70 until kind 25 carries the clock. The archive records periods in integer ns, so the realized ⌈ticks·10⁹/F⌉ is the form that keeps replay integer. | `2302` | the (a) build: trace schema |

Reduction pass (diff): nothing to remove. The branch changes no crate; the spike's patch (`spike/4400f/patches/cli-candidates.patch`) is evidence, not code to keep.

### Questions for the architect

- **Q1:** where a foreign firmware's clock is declared: a flag beside `--foreign`, or a field of the specification (the board's `mcu`, beside `hse_hz`)?
- **Q2:** (c)'s cross-check. Refuse a declaration that the RCC registers contradict (timers, decodable today), and either judge the USART kernel clock from the RCC's unhandled-write log line or leave it declared unchecked?
- **Q3:** the window: 5 s (measured margin: RSS 771–811‰, 1.56 MB log), and should F1 be measured first?
- **Q4:** the pinned build: record and refuse (proposed), which moves five stub build strings and adds one refusal test.
