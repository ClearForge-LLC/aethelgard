# AET-WO-2305 — A brown-out's recovery judged by the boot the twin ran, not by a fixed instant

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2305-bor-recovery`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P2/P3 (the twin's fault judge). Written 2026-11-20. **Cadence:** build, with one measured stop. **Before `4500`'s droop scenario.**

**Grounds** (`2303` F11, #200; `4600` F1, #245; `4202` F1, #257; roadmap §7.1):
- **The judge:** a BOR droop's recovery is judged at a fixed instant, release + 100 µs (`RECOVERY_BOUND_NS`, "Measured: 20 µs" on Sentinel-01; `chaos/src/power.rs` about L87).
- **The witness rule:** the probe's witness takes every line of the first scripted device since the release, and demands exactly the identity step's one line (`cli/src/analog.rs` about L477–480; `renode.rs` about L751 and L3558–3567; `verify.rs` about L783–796).
- **Two failures follow:**
  - **(i) A board whose first device waits before the boot word fails every BOR droop** as `brownout_unrecovered`. This covers the Sentinel-02 shape (a 39 ms boot), `4600`'s logger (boot word at 19.52 ms), and since `4202` also `examples/sentinel-01-ext-bmp581.json` (the BMP581's 10 ms settle).
  - **(ii) A later probe would still fail.** The witness then holds two or three lines, not one.
- **The time base is off:** the twin runs the settle about 1.95× long (19.52 ms for 10 ms declared). So a bound in declared µs is in the wrong time base.
- **Coverage has a hole:** a droop that crosses no BOR threshold still covers the MCU rail.

> **What this is:**
> - recovery judged as the bring-up completing (the boot word, then the first identity line among the witness lines) within a bound the judge derives from the boot the twin itself ran, in the twin's time;
> - when `bor_level` > 0, the MCU rail's coverage requires a droop that crosses the BOR threshold;
> - every committed droop result re-judged, with each move explained.
>
> It is NOT:
> - a change to the twin's timing model (the 1.95× is measured and stated, not fixed here);
> - a new scenario kind or a specification field, unless a stop rules one in.

## 1. Build

1. **Measure first,** in `spike/2305/`, with predictions committed before the runs.
   - **On `main`**, run the committed droop scenarios: Sentinel-01's `droop-bor3`, the bench droops, the six committed droop scenarios, the Sentinel-02-shaped `dshot600-droop`, `4600`'s logger copied into scratch, and `sentinel-01-ext-bmp581`.
   - **For each run**, record:
     - the release instant;
     - the boot word's instant;
     - the first identity line's instant;
     - the witness's line count at the probe;
     - the verdict.
   - **Find the twin's time base** for the settle and the boot, from the same runs. Say whether the 1.95× is a constant, or depends on the board.
2. **The bound.** Derive the recovery bound from the boot the twin ran: the same board's undisturbed boot, measured in the same campaign, with a stated margin.
   - Never use a constant. Never use declared µs.
   - Where the undisturbed boot comes from: the baseline run the campaign already makes, if it does; otherwise a run the judge makes and archives.
   - Say which, and what the archive records.
3. **The witness.** Recovery holds when the boot word appears, and then the first identity line of the first scripted device appears among the witness's lines, within the bound.
   - Extra lines after it are allowed.
   - A wrong identity value is still `brownout_unrecovered`, by name.
4. **Coverage.** When `bor_level` > 0, a droop that crosses no BOR threshold no longer covers the MCU rail. Its coverage row says why, and names the crossing it lacks.
5. **Re-judge** everything in §1.1 on the branch:
   - Sentinel-01's committed results do not move. If one does, it is a stop.
   - The waiting boards now recover, or fail for a named reason.

## 2. Acceptance

- **The measurement table** is in FEEDBACK, with predictions beside it.
- **The waiting boards:** the Sentinel-02-shaped droop, the logger, and `sentinel-01-ext-bmp581` each recover, or fail by a named reason that is not the fixed instant.
- **Sentinel-01's bench droops are byte-identical, or each move is explained.** Do not overwrite the package. If its `MANIFEST.json` moves, say so on the bus, and the architect dispatches `write=true`.
- **Mutants:**
  - a constant bound fails a waiting board;
  - an exact one-line witness fails the logger;
  - a wrong identity passes, and the test fails;
  - coverage counting a non-crossing droop fails.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count.

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-05` (attribution), `INV-06`.
- **Fence:**
  - `crates/aethelgard-chaos/src` (the droop's recovery bound and coverage);
  - `crates/aethelgard-cli/src`: `analog.rs`, `renode.rs`'s recovery probe and witness, `verify.rs`'s coverage and detection lines;
  - tests and goldens moved by this, regenerated by tooling;
  - `spike/2305/`;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only, if `MANIFEST.json` moves.
  - **Not:** the twin's timing model, the IR, specifications, scenarios' expectations (except a regenerated golden), workflows, or `ci/` (except a weight row P1/P2 asks for).
- **Running beside you:**
  - `3305` (#256): `renode.rs`'s I2C witness recording and trace 16. Merge `main` after it lands.
  - `1103` (#262), a board-name test;
  - `4240v` (#259), the generated firmware's power-up and spec 19.
  - `2306` (gates that judged nothing), `cli` verdict lines.

  Merge `main` before READY, and re-run §1.1.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-2305/pr-title-body.md`.

## 5. Adversarial pass, full

- **A board that never recovers but passes:** for example, a boot word from a reset loop, or an identity line from before the droop.
- **A bound so wide it passes anything:** the margin, stated and justified.
- **A baseline boot measured under a different firmware or scenario** than the droop run.

## 7. Flag-and-stop conditions

- **A committed Sentinel-01 result moves** in a way the new rule does not explain. Stop with the diff.
- **The undisturbed boot cannot be obtained** without a new scenario kind, or a change to what the campaign archives. Stop and say what is missing.
- **The time base is not a constant per board,** so the bound cannot be derived from one baseline run. Stop with the measurements.
