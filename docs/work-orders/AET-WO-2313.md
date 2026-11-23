# AET-WO-2313 — The boot's memory bound to 1.5 GiB, as ruled, with the evidence that moved it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2313-boot-memory-bound`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P2, a ruling's change. Written 2026-11-23. **Cadence:** a short measurement, then the build. No stop unless §7 fires.

**Grounds:**
- **Scotty, 2026-11-23 (the architect's recommendation):**
  - the boot's memory bound goes from 1 GiB to **1.5 GiB** (1,572,864 KiB);
  - Tviburi judges its own growth by a relative budget in its CI.
- **The evidence:** Tviburi `TVI-WO-1301` measured ArduPilot's full boot on the one-file overlay in CI.
  - Across runs of nearly the same tree, the peak spans 768,856–1,049,556 KiB. The cause is not known.
  - .NET's GC settings change nothing.
  - On Furnace the same tree peaks at about 984 MiB.
  - A 1 GiB cap on that boot passes or fails by a cause not yet found.
  - The Tviburi roadmap's memory row records the runs.
- **`RUL-13`** as amended for `simulate --firmware` (ruled on PR #56, B1). `BOOT_BOUNDS` (`crates/aethelgard-cli/src/renode.rs` L210–222) carries its reasoning: at least 2× the higher measured peak. The foreign judge's comment (`renode/foreign.rs` L24–30) says the 1 GiB bound "is a ruling of its own, never widened here". This work order is that ruling.

> **What this is:**
> - **(1)** `BOOT_BOUNDS.rss_kib` = 1,572,864. Its doc comment keeps the original measurement and adds this ruling, the date, and Tviburi's measured spread. The 30 s wall bound is unchanged.
> - **(2)** Every statement of the 1 GiB bound moves with it, each found by search, not by memory: the CLI README's bounds paragraph and its list, `foreign.rs`'s comment, the chaos README if it states the number, and any test that plants a value past the cap.
> - **(3)** The cap's own test (`boot_tier1.rs` `a_boot_past_the_memory_cap_fails_and_names_it`, which plants about 1.3 GB) plants past the new cap, so it still fails by `memory_cap_exceeded`. A second test shows that a tree under 1.5 GiB and over 1 GiB is not reaped.
> - **Each changed test comes with a mutant it kills** (the old constant restored).
>
> It is NOT:
> - the wall bound;
> - the campaign's or the foreign judge's own logic;
> - Tviburi's relative budget (Tviburi's work order);
> - the `HardFault` under `2311`'s access log that peaks at 2.59 GB (a §7.1 row; still over the new bound, still failing closed).

## 1. Build

1. **Measure first:** every place the bound is stated or relied on (`grep` for `1 GiB`, `1024 * 1024`, `1048576`, `1_048_576`, `memory_cap`), and each test's planted size. Commit the list in FEEDBACK.
2. **Build (1)–(3).** Merge `main` before READY. `AET-WO-1609` (running) also edits `foreign.rs`; take its version and re-apply the one comment.

## 2. Acceptance

- **The cap is 1,572,864 KiB** in code, and every statement of it says so.
- **A tree past the cap** fails with `memory_cap_exceeded`; **one between 1 GiB and 1.5 GiB** is not reaped.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli/src/renode.rs` (`BOOT_BOUNDS` and its comment), `renode/foreign.rs` (the comment only);
  - the READMEs' statements of the bound;
  - the tests that plant past the cap;
  - FEEDBACK.
- **Not:** any other bound, any gate's logic, the IR, generators, `fab/`.
- One PR, left unmerged. Write `~/.cache/aet-2313/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A second copy of the constant** that the search missed: a literal in a script, a workflow, or a test helper.
- **A test that passes at both caps** and so proves nothing.

## 7. Flag-and-stop conditions

- **The bound is stated in a steering document** (the architect's to change). Name it; do not edit it.
- **A test or a verdict depends on the 1 GiB value** in a way the new cap would weaken a judgement. Stop with it.
