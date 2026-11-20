# AET-WO-1314b — When the capacitance ladder does not run, say so; and measure whether it would have helped

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1314b-ladder-unruled`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1/P3 (eda; `1314`'s ladder). Written 2026-11-20. **Cadence:** a measurement, then a build.

**Grounds** (`4600` F18; roadmap §7.1):
- **`1314` ruled** that the deterministic ladder of eight routings, which re-routes until every capacitance-bounded net is within its bound, runs only for boards declaring `board.stackup.fab_tolerances`. Without them, `RingRule::of` returns none (`eda/src/stack.rs` about L64–66), `ruled` is false, and no ladder runs (`eda/src/schematic.rs` about L812–817, L892).
  - That is deliberate: the via ring is sized at the fabricator's worst hole.
- **Nothing tells the author.** A specification copied from Sentinel-01-minimal (no `fab_tolerances`) is routed once, with no regard to its bounds.
  - `4600`'s `logger-spi-only-cap` routed `LOG_SCK` at 48.9 mm through 8 vias: 37.5 pF against the declared 30 pF.
  - `extract` then refuses the engine's own layout, and its message says nothing about the ladder.
- **Sentinel-01 declares bounds and no `fab_tolerances`, and it is ordered.** So a refusal at `validate` is not an option: it would refuse the ordered board.

> **What this is:**
> - a measurement: the same `logger-spi-only-cap` with `fab_tolerances` declared (cited values, Sentinel-03's), does the ladder meet 30 pF?
> - `extract`'s capacitance refusal, on a board where the ladder did not run, says so and names the field that would turn it on;
> - the same line in `verify`'s verdict for that gate.
>
> It is NOT:
> - running the ladder without `fab_tolerances`, a `validate` refusal, or any change to routing output;
> - any change to a committed board's bytes.

## 1. Build

1. **Measure first,** in `spike/1314b/`, with predictions committed before the runs:
   - `spike/4600/logger-spi-only-cap.json`, re-pinned to `main`'s schema version (change nothing else, and say so), emitted and `extract`ed as is: reproduce 37.5 pF;
   - the same file with Sentinel-03's `fab_tolerances` block copied in, and nothing else changed: the rung chosen, `LOG_SCK`'s bound, and every bounded net's margin;
   - report wall time and peak for both.
2. **The message.** When `extract` refuses a capacitance bound on a board whose routing ran no ladder (`ruled == false`), the refusal adds one sentence. It says that the router's capacitance ladder runs only when `board.stackup.fab_tolerances` is declared, and that this board declares none.
   - The refusal's kind, exit code and JSON shape are otherwise unchanged. Add a field, or a sentence in the existing message, whichever the error type allows without a breaking change. Say which.
   - **Where `extract` learns whether the ladder ran:** from the specification (`fab_tolerances` present or not), not from a guess about the board file. Cite the line.
3. **`verify`:** the extract gate's line in the verdict carries the same fact when it fails.
4. **Tests:**
   - a tier-1 test on a small fixture: no `fab_tolerances`, a bound the single routing misses, and the message names the field;
   - the same fixture with `fab_tolerances`: the message does not appear (whether it passes or fails);
   - a mutant: always print the sentence, and the second test fails;
   - every committed board's `extract` output is byte-identical (no ladder ran on any that passes today).

## 2. Acceptance

- **The measurement** is in FEEDBACK, with predictions beside results and the rung named.
- **The message** appears exactly when the ladder did not run and a bound failed. Both tests and the mutant are shown.
- **No committed artifact moves.**
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` moves only by your own tier-2 count, if any.

## 3. Invariants and fences

- **Invariants:** `INV-06` (refusals by name, and now with their cause).
- **Fence:**
  - `crates/aethelgard-eda/src` (`extract`'s error text and whatever carries `ruled`);
  - `crates/aethelgard-cli/src` (the verdict line for the extract gate only);
  - tests;
  - `spike/1314b/`;
  - FEEDBACK.
  - **Not:** routing, the ladder, `stack.rs`'s rule, the IR, `examples/`, `boards/`, workflows, `ci/` (except a weight row the P2 guard asks for) or `container/`.
- **Running beside you:** `1911` (CI), `4240v`, `2304b`, `1209` (ir validate and the CLI's `validate`), `3305` (renode/evolve), `1103` (a board-name test). Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1314b/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **The sentence printed where the ladder did run** (a board with `fab_tolerances` that still misses).
- **The sentence missing where it did not run** (a board whose `fab_tolerances` is partial: `1315` refuses that as `FabToleranceUndeclared`, so confirm it never reaches here).
- **A verdict whose JSON consumers break** on the new text.

## 7. Flag-and-stop conditions

- **With `fab_tolerances` declared, the ladder still cannot meet `LOG_SCK`'s bound.** Not a stop: report the rung and the margin. It is a finding for the placer (`4600` F18 and the capacitance-aware placement row).
- **The fact "the ladder did not run" cannot reach `extract`** without changing the routing output or the board file. Stop and say where it lives.
