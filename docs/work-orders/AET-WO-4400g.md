# AET-WO-4400g — The twin's timers and USARTs at the clock silicon would run: the solved clock for the generated firmware, a declared and cross-checked clock for a foreign one, and the pinned Renode refused when absent

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400g-twin-clocks-build`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400f` builder, reused), with a full adversarial pass.
**Phase:** P4, the flight path after `4400f` (architecture §2.27). Written 2026-11-19. **Cadence: build, with one measured stop (§1.4).**

**Grounds:** the `4400f` spike, merged as the record (#229, `094d3fc7`). The ruling is comment 6996346321. FEEDBACK §2–§3 is the oracle, and `spike/4400f/` holds the evidence.
- **Today's 250 MHz is no one's choice.** Renode's RCC pushes ArduPilot's 200 MHz to TIM5. Then ChibiOS resets every timer at 0.900 ms, and `LimitTimer.Reset` restores the stock `.repl`'s 250 MHz. The result: ArduPilot runs 1.25× virtual time, and the generated firmware's motor judge is built on a clock silicon never runs.
- **A frequency set at construction fixes both, with no Renode change.** An update entry `timerN: frequency: F` / `usartN: frequency: F` builds the counter and its channels at F.
  - Under (a), the generated firmware passes the motor judge at its silicon clock of 64 MHz.
  - Under (c), ArduPilot runs at 1.00000×, its last witness at 4.394 s.
- **The suite under (a), measured:**
  - 2,405 pass and 25 fail, and no judged behaviour moves;
  - 19 are moved figures (7 replay, 12 goldens);
  - 6 are spike artifacts.

> **What this is:**
> - (a) for the generated firmware: every timer and USART at the solved clock's kernel frequency, with the judge modelling Renode's whole-ns realization;
> - (c) for a foreign firmware: a clock declared in the specification, the timers cross-checked against the RCC registers at the run's end, the USART stated unchecked;
> - the archive carrying the clock (kind 25);
> - the foreign window at 5 s, with its RSS margin measured;
> - the pinned Renode build recorded and refused when absent.
>
> It is NOT:
> - candidate (b), RCC-following. That goes to the backlog, upstream (§7.1, the patches in `spike/4400f/patches/`);
> - a clock other than the reset one for the generated firmware. `validate` still refuses it;
> - a change to `container/`, or to any Renode source;
> - the battery-sag or lock-step work.

## 1. Build

1. **(a): the generated firmware at its solved clock.**
   - The emitter writes an update entry for every timer and USART the platform carries. F is the solved clock's kernel frequency: `SolvedClock::timer_kernel_hz` and `serial::kernel_hz`, 64 MHz on the reset clock.
   - **`TIMER_HZ` and `CYCLE_NS` stop being constants.** Each observer takes its board's F.
   - **The realized period.** The judge predicts each edge at the period Renode realizes: ⌈ticks × 10⁹ / F⌉ ns (`BaseClockSource.cs` L315–L325, FEEDBACK §2, F3). Records hold integer ns.
     - Example: DShot600 at 64 MHz is 106 ticks, which Renode runs as 1,657 ns.
     - The rounding is a model convention, stated in the module documentation beside `ARR` against `ARR + 1`.
     - The silicon block judges the real period, so the model's rounding never reaches a silicon verdict.
   - **Every user of `TIMER_HZ` moves** (FEEDBACK §3):
     - `timer.rs` `period_ns`/`high_ns` and their unit asserts;
     - `renode.rs` L1739's text;
     - `timer_routes.rs` L81 and `motor_platform.rs` L67;
     - `aethelgard-evolve`'s and `aethelgard-chaos`'s motor fixtures.
   - **(a) never applies to `--foreign`.** ArduPilot runs at 0.32× under it, as measured. A foreign run that reaches (a)'s path is refused by name.
2. **(c): a foreign firmware's declared clock.**
   - **In the specification, never a flag.** The fields sit beside `mcu.hse_hz`: one kernel frequency per timer clock domain the foreign firmware uses (APB1 timers, APB2 timers), and one per USART the platform emits. Each is `{value, source}`.
     - The type lives in `aethelgard-ir` (`INV-01`).
     - `SPECIFICATION_SCHEMA_VERSION` goes from 16 to 17.
     - Propose the names in FEEDBACK. Ask only if the shape cannot be the one above.
   - **`simulate --foreign` refuses by name when the declaration is absent** (`INV-06`). A generated-firmware run refuses a specification that declares one.
   - **The timer cross-check.**
     - At the run's end, decode the timers' kernel clock from the registers Renode models and reads back: `PLLCKSELR`, `PLL1DIVR`, `D1CFGR`, `D2CFGR`, `CFGR`. ArduPilot gives `0x408040`, `0x01090231`, `0x48`, `0x440`, `0x801B`, which decode to 400 MHz `sys_ck` and 200 MHz timers.
     - A declaration they contradict fails the run by name, with both figures.
     - The decode is RM0433's, cited per register, with an exact-case test on ArduPilot's five values.
   - **The USART is declared and unchecked, and the verdict says so.** The verdict carries a named condition: the USART kernel clock is the declared one, not read back, because Renode drops `D2CCIP2R` (F2).
     - No judgement is drawn from Renode's unhandled-write log line.
     - The verdict lists which clocks were checked and which were declared only.
3. **Replay carries the clock (F6).**
   - **The trace.** Kind 25 (`MotorOutputSolved`) gains the timer's kernel frequency, and `TRACE_VERSION` goes from 13 to 14. Its periods are the realized integer ns.
   - **One function, one home.** The realized-period function is defined once, in `aethelgard-chaos`, the trace format's own crate.
     - The twin calls it. The twin gains its first dependency on `aethelgard-chaos`.
     - This puts it in `aethelgard-chaos`, not `aethelgard-ir` as the ruling first said: replay checks an archive without the twin, so the function belongs with the format.
     - If the dependency direction is refused by the workspace's layering (a cycle, or a rule in `docs/architecture.md`), stop and say which.
   - **`campaign.rs`'s `TWIN_CYCLE_NS` copy is deleted, not updated.** `witness_consistent` checks `period_ns` and `high_ns` against the function with the archive's F.
   - **Replay refuses an archive at an older trace version by name,** as it does today.
4. **The foreign window: 5 s. Measured, then STOP if it fails.**
   - `FOREIGN_MAX_VIRTUAL_NS` goes from 4 s to 5 s.
   - `ci/foreign-judge.sh`'s bounds go to 5000, 4300 and 4370 ms.
   - **F1, the RSS margin.** Run the 5 s foreign case at least three times under CI-like concurrency: the tools container alongside one other container of yours, under `capped-run`. Record each run's peak RSS against the 1 GiB cap, its wall time and the host load.
   - **STOP** with the numbers if any run passes 900‰ or is killed. The cap is a ruling of its own, never widened here.
5. **The pinned build: recorded and refused.**
   - `simulate` records the Renode build string (`tool.build`) in every verdict.
   - It refuses by name any build other than the image's pinned one: today `1.17.0+20260907gitf1dd1b4af.aet1605`, read from the image's pin, never typed in twice.
   - `2205`'s any-build test now asserts the refusal. The five stub build strings move.
6. **`INV-07`: the residues are closed or named.**
   - **The RCC push on a clock write ((a)(2)).** `validate` already refuses any clock but the reset one. Add a test whose name and message state this reason: a clock-programming firmware would get counters at the RCC's figure and channels at F. A future clock engine then trips over it instead of rediscovering (b)'s failure.
   - **DWT at 250 MHz ((a)(3)).**
     - If the platform can set DWT's frequency by an update entry, set it to F.
     - If not, state a condition in the verdict, and add a test asserting that no judge reads DWT.
   - **USART frequency and pacing ((a)(4)).** Today the USART frequency drives `BaudRate` and the receiver timeout only. Record in the module documentation that upstream `88afd12bb` paces received characters at `BaudRate`, so at the next Renode a wrong F starts to matter.
7. **Every moved figure is re-pinned from a measured run, each cited to it:**
   - the motor records;
   - the platform text and hashes, including `order_2203.rs`'s sha256;
   - the re-blessed `boards/sentinel-01*/…repl` (update entries only);
   - `twin_tier2`'s PWM rise floor (40 in 100 ms);
   - the foreign witness times, and architecture §2.27's figures (named in FEEDBACK for the docs PR; steering documents are not yours to edit).

## 2. Acceptance

- **The generated firmware's verdicts stand at 64 MHz.**
  - DShot600 (`serial-dshot-9600`, `motors-dshot600`) and PWM (`motors-pwm`) pass.
  - Sentinel-02 without motors moves only its platform's bytes.
  - No verdict flips. A flip is a stop.
- **Unmodified ArduPilot** runs at 1.00000× and passes the foreign judge at 5 s, with the timer cross-check passing and the USART condition stated.
- **Each refusal has a test and a mutant that a test kills:**
  - no declared clock under `--foreign`;
  - a declared clock on a generated run;
  - a timer declaration the RCC contradicts;
  - a non-pinned Renode build;
  - an older trace version.
- **Replay and campaign** pass on 64 MHz records, with no copy of a twin constant left in `aethelgard-chaos`.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact. The tools shards stay under 55 minutes; a shard over that is a stop.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-twin`;
  - `aethelgard-chaos` (the trace, replay and campaign);
  - `aethelgard-ir` (the declared clock and its schema version);
  - the CLI's `simulate`, `foreign.rs` and `campaign`;
  - `aethelgard-evolve`'s motor fixtures;
  - `ci/foreign-judge.sh`;
  - the committed board platforms;
  - goldens and tests;
  - FEEDBACK.
  - **Not** `container/`, workflows, or any Renode source.
- **Running beside you:** `AET-WO-1910` (CI census). Its census changes nothing, and its build touches tests across the workspace. If main moves under you in a file you touch, merge it and re-gate before READY.
  - You run the tools and ArduPilot images: one container of yours at a time under `capped-run`, except the two-container F1 measurement (§1.4).
- One PR, left unmerged. Write `~/.cache/aet-4400g/pr-title-body.md`.

## 5. Adversarial pass, full

- **A frequency typed in where a solved or declared one exists.**
- **A judge that predicts the nominal period** (1,656.25 ns) where Renode realizes 1,657 ns, or the reverse in a silicon verdict.
- **A foreign run that passes with the declaration absent, contradicted, or silently replaced by (a).**
- **A replay that accepts a record whose period disagrees with its archived F.**
- **A verdict that reads as checked** where the USART clock was only declared.
- **A non-pinned Renode that judges without refusal.**

## 7. Flag-and-stop conditions

- **§1.4:** a 5 s foreign run passes 900‰ RSS or is killed.
- **A generated-firmware verdict flips:** stop, with the bytes.
- **The layering forbids the twin calling `aethelgard-chaos`:** stop and say which rule.
- **The RCC decode cannot be cited to RM0433 for a register it reads:** stop. It is declared as a condition, or the cross-check is narrowed by ruling.
