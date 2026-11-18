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
