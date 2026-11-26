# AET-WO-4412 — P4 slice B: foreign-firmware dynamics

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4412-foreign-dynamics`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1919` builder, who knows the foreign window and the CI lanes), with a full adversarial pass.
**Phase:** P4, slice B (roadmap §5, "The remaining P4 work, as slices"). Written 2026-11-26. **Cadence:** a measurement and a stop, then the build in parts, with a stop after any part whose measurement moves a committed verdict.

**Grounds.** Roadmap §7.1, the rows tagged **P4 slice B**, cited by their text. Exit clauses 3 and 4 need ArduPilot running on Sentinel-03 in the twin as it will on the bench.
- **The board:**
  - "`simulate --foreign` refuses an HSE specification (`HseNotGenerated`), and `vos::judge_foreign` names no deviation for `PLLSRC` = HSE" (`4240a` F4).
  - "CI builds no Sentinel-03 ELF" (`4400j` F6).
- **The firmware:**
  - "Sentinel-03's hwdef emits no `MOT_PWM_TYPE`, so ArduPilot runs PWM on a board declared DShot" (`4400h` D4 b).
  - "An RC port's tolerance … is judged at the declared baud (CRSF 420000), while ArduPilot runs its own (416666)" (`4400b` F3), to triage here.
- **The motors:**
  - "DShot set over MAVLink engages timer DMA … and the twin runs out of memory in 1.65 s" (`4400h` D4 a2).
  - "DShot DMA per timer is unmodelled: the solver offers TIM12–14 … without knowing whether they raise a DMA request" (`4300h` F8).
  - "The judge stops each DShot timer at the end of its window … ArduPilot will" (`2302` D2).
- **The reboot:** "A foreign firmware cannot be rebooted in the twin: no reset macro, and with one the second boot runs out of 4 GiB" (`4400h` D4 a).
- **`4300i`'s rows R2–R8** (FEEDBACK §7), to triage here.
- `INV-03`, `INV-04`, `INV-06`, `INV-08`; `RUL-13` (as `1919` amends it).

> **What this is.** One work order for the slice, in four parts, in this order. Each part has its rule, a planted board, firmware or input that fails by name, and a mutant per branch.
> - **(1) Measure first, with predictions committed in `spike/4412/`:**
>   - for each row above, what happens today on Sentinel-03's ELF (built from the committed hwdef in the pinned `aethelgard-ardupilot` image), in the 12 s input window `4400j` admits: memory, wall time, records and the verdict;
>   - for the reboot, where the second boot's memory goes (a heap dump, as `4400j` did);
>   - for DShot DMA, which requests RM0433's DMAMUX table gives TIM1–TIM17, by page;
>   - triage R2–R8 and the RC baud row: each is in this WO, a row after P4, or closed with its reason.
>   - **Stop** with the table and the triage.
> - **(2) The board, end to end:**
>   - a foreign HSE board is accepted, with `foreign_clock` derived from the specification's crystal (USARTs at 100 MHz on Sentinel-03, not the HSI tree's 72), and `judge_foreign` judges `PLLSRC` = HSE;
>   - Sentinel-03's specification commits its derived clock;
>   - CI's `ardupilot-image` job builds Sentinel-03's ELF and checks it reproduces, as Sentinel-01's does (`4408` D1 (a): no ArduPilot bytes committed);
>   - the hwdef emits `MOT_PWM_TYPE` for a DShot board.
> - **(3) The motors over MAVLink:**
>   - DShot's timer DMA either moves data as silicon does (the request from the table, a stream that ends) or is refused by name before it exhausts memory, never a 4 GiB death;
>   - the solver offers a timer for DShot only if its DMA request exists on the table;
>   - the judge's end-of-window stop is either removed (the timer runs on to the end of the window and is read there) or named in the verdict where ArduPilot reads it.
>   - Say which, from (1).
> - **(4) The reboot:** a foreign firmware rebooted in the twin, by the reset the bench would use, with the second boot inside the cap, so that parameters set over MAVLink before a reboot are judged after it. If (1) shows the second boot can't fit the cap, stop with the dump.
>
> It is NOT:
> - flight in the twin (slice C);
> - power or coverage (slice D);
> - Tviburi's guarded twin (Aethelgard still runs stock Renode until Phase 2's seam);
> - a larger memory cap (Scotty's: 1.5 GiB stays).

## 1. Build

1. **Measure (1).** Stop with the table and the triage.
2. **Build (2), (3), (4) to the ruling, in that order, on one branch.** Stop after any part whose measurement moves a committed verdict or package. The packages are re-derived by the write dispatch on your head. Merge `main` before READY.

## 2. Acceptance

- **Sentinel-03's ELF** is built and checked in CI.
- **The foreign run:** the armed input run on Sentinel-03 arms inside the cap and the window, with DShot engaged over MAVLink and judged.
- **A planted firmware for each rule:**
  - a DShot timer with no DMA request is refused at the solver;
  - a stream that would exhaust memory is refused by name;
  - a reboot's second boot is judged.
- **Committed boards:** `INV-08`'s boards are byte-identical, or each change is named and ruled.
- **CI is green**, with `1919`'s target for `bench-package-s03` held. Every new test is in a shard at its measured weight, checked with `ci/shard.sh plan` before the push.

## 3. Fences

- **Fence:**
  - `aethelgard-cli`'s foreign path (`renode/foreign.rs`, `simulate --foreign`, the judge's window);
  - `vos`;
  - the solver's DShot timer choice;
  - the hwdef generator;
  - the specification's `mcu.foreign_clock`;
  - CI's `ardupilot-image` job;
  - planted inputs and firmware;
  - `spike/4412/`;
  - `docs/work-orders/AET-WO-4412.*`.
- **Not:**
  - the router and placer;
  - the power and coverage rules (`4310`, beside you);
  - Renode's cap;
  - the steering documents.
- **Running beside you:** `4310` (slice D) and `4411`'s merge. Merge `main` after each.
- One PR, left unmerged. Write `~/.cache/aet-4412/pr-title-body.md`.

## 5. Adversarial pass, full

- **A firmware that passes because the twin stopped it:** a timer, a stream or a window that ends before the firmware's own behaviour would.
- **A reboot that isn't one:** state carried across it that silicon would lose, or lost that silicon keeps.
- **A DMA request assumed:** a timer or stream the table does not give.
- **An ELF in CI that isn't the bench's:** the hwdef, the clock or the build options differing from what Sentinel-03 will flash.

## 7. Flag-and-stop conditions

- **After (1),** with the table and the triage.
- **A part moves a committed verdict.** Stop before building past it.
- **The second boot can't fit the cap,** or a DShot stream can't be bounded. Stop with the dump.
