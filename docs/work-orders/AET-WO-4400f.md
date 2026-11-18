# AET-WO-4400f — Spike: the twin's timer and USART clocks follow the clock tree, not the stock platform's fixed frequencies

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400f-twin-clocks`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1605` builder, reused).
**Phase:** P4 (the flight path after `4400e`; architecture §2.27). Written 2026-11-18. **Cadence: spike, ending in a stop.** Prototypes stay under `spike/4400f/`.

**Grounds:**
- **TIM5 and `usart1` divergences, named in architecture §2.27:**
  - every `Timers.STM32_Timer` in Renode's stock `stm32h7.repl` counts at a fixed `frequency: 250000000` (L321), whatever the firmware writes to RCC;
  - `usart1` runs at a fixed 200 MHz.
- **Measured under unmodified ArduPilot** (`4400c`, `4400e`): ArduPilot's clock runs 1.25 × virtual time, because it programs TIM5 for a 200 MHz kernel clock. Its USART1 `BRR` is `0x271`.
- **The foreign judge's 4 s window rests on it** (`4400e` D7, re-owned at #212): with TIM5 corrected, the last witness may move from 3.61 s to about 4.5 s [inferred].
- **The generated firmware's motor judge is built on the same convention** (`4300a`, `aethelgard-twin` `timer.rs`): `TIMER_HZ` = 250 MHz, a period of `ARR` ticks, and the first rise one period after `CEN`. A clock change moves its predicted edges.
- **The generated firmware runs cold on HSI** (64 MHz), and ArduPilot programs PLLs. No single static frequency is right for both.

> **What this is:** the measurement that says how the twin's timers and USARTs should get their kernel clock, from the firmware's own RCC state or from a declared or solved clock tree, and what each choice moves.
>
> It is NOT:
> - a change to any crate, model, platform or judge on the branch;
> - the battery-sag or lock-step work.

## 1. Spike

1. **Predictions first,** committed to FEEDBACK before any run.
2. **Read at source and cite:**
   - how Renode 1.17.0 (the pinned `.aet1605` build) gives `STM32_Timer` and `STM32F7_USART` their frequency;
   - whether either can follow RCC at run time;
   - what `stm32h7.repl`'s RCC model exposes;
   - RM0433's kernel-clock paths for TIM2–TIM5, TIM1/TIM8 and USART1 (`rcc_pclk1`/`pclk2`, the timer ×2 rule, `TIMPRE`).
3. **Measure the candidates**, each as a labelled deviation under `spike/4400f/`:
   - **(a)** a per-board static frequency emitted from the solved clock tree (what the generated firmware programs);
   - **(b)** a frequency that follows RCC at run time: a hook, a C# model of ours, or an upstream model if one exists;
   - **(c)** a declared clock for a foreign firmware, beside `--foreign`.

   For each, measure:
   - ArduPilot's clock against virtual time;
   - the foreign verdict's witness times and the 4 s window (re-measured);
   - the generated firmware's motor-judge verdicts (Sentinel-02 and the serial/DShot fixture), byte for byte;
   - wall time.
4. **The pinned-build question** (`1605` D1, #223): `simulate` runs whatever `renode` it finds outside the image, and a stock 1.17.0 judges nested interrupts wrongly with no warning. Measure what `simulate` records about the Renode build today. Propose: refuse, warn, or record only, given `2205`'s any-build test.

## 2. Stop

**STOP** with FEEDBACK holding:
- the predictions, scored;
- the recommended candidate, with what it moves: every verdict and golden, the 4 s window, and `TIMER_HZ`'s users;
- `INV-07`: a twin that runs a timer at a frequency silicon would not is a twin that can pass what silicon fails. Say for each candidate where it could still diverge;
- the pinned-build proposal.

Push an `AET-WO-4400f STOPPED:` commit and open a PR, merged as the record.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `spike/4400f/` and FEEDBACK.
  - Read anything. Change no crate, model, platform, workflow or `container/` file on the branch.
- **Running beside you:** `1314` (the router, `aethelgard-eda`). You share nothing with it.

  You run the tools and ArduPilot images: one container of yours at a time, under `capped-run`.
- One PR. Write `~/.cache/aet-4400f/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **A candidate needs a Renode change upstream:** stop with the patch it would need, as `1605` did.
