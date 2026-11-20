# AET-WO-1318 — eda and CI housekeeping left by 1316 and 1317: one way to read the drawing, true weights, a true README

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1318-eda-housekeeping`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (eda) and CI. Written 2026-11-20. **Cadence:** a reduction build. **After `1316`, `1317` and `4240v`** (all merged).

**Grounds** (roadmap §7.1; `1316` F5, #273; `1317` F4 and F7, #274):
- **`1316` F5.** `schematic::anchored` and `bound_holds` survive as thin readers of `schematic::Drawing`. They were kept only because neighbour branches (`1317`, `4240v`) edited their test callers, and both have merged.
- **`1317` F4.** Two CI weight rows lag what `1317` made their tests do:
  - `board_tier2::each_size_sample_…` runs about 147 s against a 1.5 s row;
  - `outline_tier1` runs 199 s against 152 (`ci/test-weights.tsv`, `ci/colocate.tsv`).
  - The planner still balanced every shard under 20 minutes, but it plans on wrong numbers.
- **`1317` F7.** The eda README's bounded-placer bullets are stale. `1317`'s FEEDBACK §6 F7 has the proposed text.

> **What this is:**
> - every caller of `anchored` and `bound_holds` reads `Drawing` directly, and both free functions are removed;
> - the two weight rows set from measured CI times, with the planner's guards passing;
> - the eda README's placer bullets made true to `pcb.rs` as it stands.
>
> It is NOT:
> - a behaviour change: no emitted byte, verdict or test outcome moves;
> - any other weight row, or the shard count.

## 1. Build

1. **The readers.** Find every caller of `schematic::anchored` and `schematic::bound_holds` (crates and tests), move each to `Drawing`, and remove the two functions. `validate`'s single `design()` call (`1316`) stays single.
2. **The weights.**
   - Measure each test's time on CI's runner from the latest green `main` run's shard logs, or by `ci/shard.sh` in the image if the logs do not give it.
   - Set the rows by the rule the planner already uses: say which rule, and cite it.
   - `ci/shard.sh plan` and its guards (P1 stale row, P2 heavy unweighted) pass.
   - Report each shard's planned and measured time before and after.
3. **The README.** Rewrite the eda README's bounded-placer bullets from `pcb.rs` as it stands: the canonical rank, the reseeded retries (`1317`), centroid targets and compaction. Start from `1317`'s proposed text and check each claim against the code.
4. **Equivalence.** `emit` of every committed specification is byte-identical to `main`, and `cargo test --workspace` gives the same pass set.

## 2. Acceptance

- Both functions are gone; every caller reads `Drawing`; one `design()` call per `validate`.
- **The two rows are measured,** the guards pass, and the plan table is in FEEDBACK.
- **The README** claims are each traced to a line.
- **No output moves,** and the bench package does not move.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` does not move.

## 3. Invariants and fences

- **Invariants:** `INV-01`.
- **Fence:**
  - `crates/aethelgard-eda/src/schematic.rs` (the two functions) and their callers' call sites;
  - `crates/aethelgard-eda/README.md`, the placer bullets;
  - `ci/test-weights.tsv` and `ci/colocate.tsv`, the two rows only;
  - FEEDBACK.
  - **Not:** `pcb.rs`'s behaviour, workflows, specifications, `boards/` or `fab/`.
- **Running beside you:**
  - `2305` (#271): chaos, `renode.rs`, `verify.rs`;
  - `4241` (#279): firmware and judge;
  - `1210` (#280): spec 20/21, CLI `validate`;
  - `4240a` (#281): IR HSE;
  - `4405` (#277): pincheck.

  `1210` and `4240a` regenerate `boards/` sha lines; you touch none. Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1318/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A caller that relied on `anchored` re-running `design()`** on a changed input.
- **A weight row set so low** that the planner packs a shard past its limit on a slow runner.

## 7. Flag-and-stop conditions

- **Any output or test outcome moves.** Stop with the diff.
