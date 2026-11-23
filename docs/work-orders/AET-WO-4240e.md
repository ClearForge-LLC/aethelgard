# AET-WO-4240e — A kernel clock that actually reaches the peripheral: never switch a mux away from a dead input

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4240e-kernel-clock-switch`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before any further Sentinel-01 bench session. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `4300j`:** both touch `svd-hal`'s generator, and both move the bench package.

**Grounds:**
- **Tviburi `TVI-WO-1300` §1.1, 2026-11-23** (bus note 13:28Z; the RCC guard's measurement):
  - 9 of the 13 boards measured write `SPI123SEL` = 100 (`per_ck`, from `hsi_ker_ck`) and never start PLL1: `motors-dshot600`, `motors-pwm`, `sentinel-01`, `sentinel-01-ext-bmp581`, `sentinel-01-minimal`, `sentinel-02-bench`, `sentinel-02-shaped`, and others named in 1300's FEEDBACK.
  - **RM0433 Rev 9 pp. 371–372** ("Clock switching state after system reset"): a kernel-clock mux switches only if a clock is present on both inputs; otherwise no clock is provided to the peripheral.
  - SPI123SEL's reset input is `pll1_q_ck`. With PLL1 off, the mux cannot leave it, so **SPI1 starts with no kernel clock**.
- **The 3302 bench** (`~/.cache/aet-architect/bench-3302-results.md` B8, before the `4230` change): on silicon, `SPI123SEL` = `pll1_q_ck` with PLL1 off, no kernel clock, and SPI1 dead. The `4230` change wrote `per_ck` instead, and **no firmware with that change has run SPI1 on silicon since.**
- **The same class on another selector** (`TVI-WO-1300`, 14:42Z): `d1-drv8876` enables `ADC3EN` with `ADCSEL` = `pll2_p_ck` and PLL2 never started. It is in scope: every kernel-clock selector, not only `SPI123SEL`.
- **Review round 2, A1;** **`RUL-20`:** a fact counts as judged only by an independent rule, the twin's reaction, or the bench. The twin's stock RCC tags the selector, so nothing judged this.

> **What this is:**
> - **(1) A generator rule:** for every kernel-clock selector the firmware writes, the generated sequence makes the mux's *current* input carry a clock before it switches, and makes the new input carry one too. Two ways to satisfy it, to be chosen at the stop:
>   - (a) start PLL1 with its Q output enabled before the switch, and stop it after only if nothing else needs it;
>   - (b) leave the selector at a source that runs. On the HSE boards this is `pll1_q_ck` from the ruled H1 path; on the HSI boards PLL1 is started from HSI.
> - **(2) An independent rule in `aethelgard-ir`** (`RUL-20` (1)), sharing no code with the generator: from the solved clock tree, every enabled kernel-clocked peripheral has a running source at its start, by the p371 rule. Refused by name otherwise. Its expectations come from the specification and RM0433 Rev 9, cited by page, never from the generator's sequence.
> - **(3) The witness ledger:** the `RCC_D2CCIP1R` rows (and any other selector rows) move from unwitnessed to `independent-rule`.
> - **(4) Bench rows:** a `BENCH.md` row for the Nucleo that reads `RCC_D2CCIP1R` and confirms an SPI1 transfer with the regenerated firmware. The row is the scar's re-check; the bench result decides whether the p371 reading is literal.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - a copper change, or any pin move;
> - ArduPilot's clocking (ArduPilot is clean under the RCC guard: every kernel-clocked start finds its clock);
> - Tviburi's guard: `TVI-WO-1300` and `1300a` own that side.

## 1. Build

1. **Measure first.** Commit predictions in `spike/4240e/`, then record:
   - every committed specification's generated clock sequence;
   - which selectors each writes, in what order relative to the PLL enables and the peripheral starts;
   - the p371 outcome per peripheral;
   - which emitted and `fab/` bytes each of (a) and (b) would move.
2. **Stop with the measurement,** (a) against (b), and the rule's shape.
3. **Build to the ruling,** with goldens moved by their generators. The bench package moves by its ELF and hash lines: ask for the dispatch.
4. **Re-run the measurement:** zero peripherals start without a clock.

## 2. Acceptance

- **No committed board's generated firmware starts a kernel-clocked peripheral without a running source,** by the IR rule and by Tviburi's RCC guard query (run it in recorded mode against the regenerated firmware if `1300` has merged; otherwise say so).
- **A planted specification** whose sequence would switch away from a dead input is refused by name.
- **No pin or copper byte moves.** The bench package moves only by the firmware and its hash lines.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-svd-hal`'s clock sequence;
  - `crates/aethelgard-ir` (the independent rule, its refusal, held data and tests);
  - `crates/aethelgard-cli/src/ledger.rs` (the selector rows' class);
  - `fab/sentinel-01-bench/BENCH.md` (the new rows only);
  - goldens moved by their generators;
  - `spike/4240e/` and FEEDBACK.
- **Not:** eda, the twin's platform, chaos, workflows, `ci/`.
- **Running beside you:** the backlog in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4240e/pr-title-body.md`.

## 5. Adversarial pass, full

- **A selector switched twice,** or switched after its peripheral starts, or a PLL stopped while a peripheral still uses it.
- **An independent rule that reads the generator's sequence** instead of the specification and the manual (`INV-07`).
- **A peripheral the measurement missed:** I2C, USART, RNG, ADC and FDCAN selectors, not only SPI123SEL.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement, (a) against (b), and the rule's shape.
- **A held page contradicts the p371 reading,** or a bench record shows a switch away from a dead input working. Stop with the page or the record.
- **Any pin or copper byte would move.** Stop.
