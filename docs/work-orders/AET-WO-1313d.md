# AET-WO-1313d — Compaction ends by a bound it can prove, and refuses by name when it does not

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1313d-compaction-bound`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4220c` builder, reused), with a targeted adversarial pass.
**Phase:** P1 board capability (roadmap §5, §7.1 row "Compaction ends by a falling potential…"). Written 2026-11-18. **Cadence: build, no stop unless §7 fires.**

**Grounds (`1313c` FEEDBACK §5, measured on the merge):**
- `compact` (`aethelgard-eda` `pcb.rs`) loops until a pass in which no group moves. Its termination argument is a comment: each group's potential, (|dy|, |dx|) from its keep-out box's centre to the MCU's courtyard centre, falls strictly in lexicographic order with every move.
- That argument rests on the code being right. The mutant of `shift` that leaves a group's keep-out box behind never terminates:
  - no assertion fails;
  - the harness's eda and tier-1 runs each hit their 900 s timeout;
  - in CI only `tools-shard`'s 45-minute hang guard would stop it.
- A loop that can run for good on a wrong edit is a hang, not a refusal (`INV-06`: refuse by name, never substitute).

> **What this is:** a bound on compaction's moves, computed from the geometry the loop starts with, so the termination argument is checked at run time. Past the bound, `emit` refuses by name with nothing written.
>
> It is NOT:
> - a change to any placement: every board, golden and the 24-specification corpus stay byte-identical, the Sentinel-03 fixture included;
> - a tuning knob or a typed-in limit;
> - the test-pad rule or a capacitance-aware step (backlog).

## 1. Build

1. **The bound, derived and stated where the loop is.** From the comment's own argument:
   - a move in y takes one `STEP` off |dy| and may set |dx| anywhere in the core's fixed x-range;
   - a move in x alone takes one `STEP` off |dx|.

   So a group makes at most `(|dy₀| / STEP + 1) × (W / STEP + 1)` moves, where `|dy₀|` is its starting |dy| and `W` is the core's width. The loop's bound is the sum over groups, plus one final pass with no move. Compute it before the loop from those values. Never type it in, and never take it from the environment or the specification. If you can prove a tighter bound, use it, with the proof in the comment.
2. **The check.**
   - The loop counts moves.
   - When the count would pass the bound, compaction returns a new `EmitError` variant (for example `CompactionUnbounded`) naming the group's IC reference, its potential at the stop and the bound.
   - `emit` writes nothing, as `OutlineTooSmall` and `OutlineUnreachable` do today.
   - The error has its `kind()` string and its CLI JSON shape, like the other two.
3. **A second, cheaper check.** Every move must lower the moving group's potential strictly (lexicographic). A move that does not is the same refusal, at the move that broke it. The bound covers what this check misses; this check finds a broken move at once.
4. **The tests.**
   - A unit test of the bound's arithmetic on a hand-built two-group case.
   - A test that drives `compact` with a move that does not lower the potential, through a test-only seam, and asserts the refusal by name in well under a second. Choose the seam to add the least code; say in FEEDBACK why you chose it. A `#[cfg(test)]` hook is acceptable; a runtime switch is not.
   - The 1313c keep-out-box mutant of `shift`, re-run on the build. It must now fail a test by name within the eda unit test run's ordinary time (about 1 s), not hang. Record the wall time.

## 2. Acceptance

- **Byte-identical:** every existing board and golden, the 24-specification corpus and the Sentinel-03 fixture's outline (85.25 × 80.95 mm). The bench package does not move.
- **On every corpus board with a declared bound, record:**
  - the computed bound;
  - the moves actually made;
  - the ratio between them.

  These figures show the bound is not loose enough to be meaningless. If one bound is over 10⁶ moves, say why.
- **The `shift` mutant is refused by name,** in about the time of the ordinary run.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`; `RUL-03`, `RUL-16`.
- **Fence:**
  - `aethelgard-eda`: `pcb.rs`, and `schematic.rs` for the error variant only;
  - the CLI's error mapping, if the new kind needs a line there;
  - its tests;
  - FEEDBACK.
- **Running beside you:** `4400e` (the foreign judge, `aethelgard-cli`'s `simulate`). You may both touch the CLI's error mapping. If main moves under you there, merge it and re-gate before READY.

  You run the tools image: one container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-1313d/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A bound that a correct compaction can reach,** on any corpus board or a constructed one: a false refusal.
- **A bound computed from state the loop changes,** which then moves under it.
- **A potential check that a correct sidestep fails:** a y-move that also moves x is still a strict fall.

## 7. Flag-and-stop conditions

- **Any byte moves:** stop, with the bytes.
- **The derived bound cannot be computed without a typed-in constant:** stop and say which.
