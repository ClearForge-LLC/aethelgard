# AET-WO-1316 — The parts table and the emitter's arms agree by test, and the drawn design is read once

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1316-parts-arms`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (eda). Written 2026-11-20. **Cadence:** build.

**Grounds** (`1209` F1, F2, #253; `1209b` F4, #270; roadmap §7.1):
- **The parts table and the emitter's per-part arms agree by hand only.**
  - `aethelgard_ir::names::VENDORED_PARTS` (`ir/src/names.rs` L32) lists six parts, and `schematic.rs` L580 imports it as `KNOWN_PARTS`.
  - The parts table holds seven rows (ICP-10111 has a row and no arm). `PartWithoutRow`'s message therefore names the wrong set as "known".
  - Nothing fails when a row is added without an arm, or an arm without a row.
- **The drawn design is computed three times.** `schematic::drawn`, `anchored` and `drawn_nets` (`schematic.rs` L1205–1250) each re-run `design()`. The CLI's `validate` calls more than one of them on the same specification.

> **What this is:**
> - a test that walks every parts-table row through `schematic::design` on a minimal fixture. Each row either draws, or is refused by its named error. The test fails when the table and the arms drift apart, in either direction;
> - `PartWithoutRow`'s message (and `KNOWN_PARTS`'s use in eda) naming the right set, from one source where the crate graph allows it;
> - one read-only view of the drawn design, computed once, that serves `drawn`, `anchored` and `drawn_nets`; the CLI's `validate` reads it once.
>
> It is NOT:
> - giving ICP-10111 an arm, or making the arms data (`4600` D4 stays its own row);
> - any change to what `validate` refuses, or to an emitted byte.

## 1. Build

1. **Inventory first,** in FEEDBACK:
   - the parts table's rows, where the table lives, and how many there are;
   - the emitter's arms, and where each is matched;
   - `VENDORED_PARTS`'s readers.
   Say which rows have no arm and which arms have no row.
2. **The agreement test** (tier 1 if it runs under a few seconds; say which tier).
   - For every row: a minimal specification that uses it, through `schematic::design`. The result is a drawing or a named refusal, and the test pins which.
   - Adding a row without an arm, or an arm without a row, fails the test with the part's name.
3. **The message.** `PartWithoutRow` and any "known parts" text name the set they mean.
   - `aethelgard-ir` cannot depend on `aethelgard-eda`. If the set must stay in the IR (`INV-01`), a test in eda pins the IR's list to the arms. Say which you chose, and why.
4. **One view.** A read-only value built by one `design()` call, from which `drawn`, `anchored` and `drawn_nets` are read.
   - The three functions keep their signatures, or their callers move with them in this PR.
   - The CLI's `validate` builds the view once per specification.
   - Measure `validate`'s wall time on Sentinel-03 and on the 52-file corpus, before and after.
5. **Corpus:** every committed specification validates and emits byte-identically to `main`.

## 2. Acceptance

- **The inventory** is in FEEDBACK.
- **The agreement test** passes on the branch. **Mutants:**
  - a row added with no arm: the test fails, naming it;
  - an arm's part removed from the table: the test fails, naming it.
- **One `design()` call** per `validate`, shown by a count or a trace, with the wall times.
- **No output moves:** `validate` on the corpus and `emit` of five specifications are byte-identical. The bench package does not move.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` moves only by your own tier-2 count, if any.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`.
- **Fence:**
  - `crates/aethelgard-eda/src/schematic.rs` (the view and the three accessors) and the parts table's reader, if the test needs one;
  - `crates/aethelgard-ir/src/names.rs` and `PartWithoutRow`'s message only;
  - `crates/aethelgard-cli/src/main.rs` (`validate`'s calls only);
  - tests;
  - FEEDBACK.
  - **Not:** the emitter's behaviour, the IR's validation rules, specifications, the parts table's contents, `boards/`, `fab/`, workflows or `ci/` (except a weight row the P2 guard asks for).
- **Running beside you:**
  - `4240v` (#259): svd-hal and spec 19;
  - `2305` (#271): chaos, `renode.rs`, `verify.rs`;
  - `2306` (#269): `cli` verdict lines;
  - `1317`: the placer in `eda/src/pcb.rs`. You share no file with it.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1316/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A row whose minimal specification is refused for a reason other than its arm,** so the test pins the wrong refusal.
- **A view that goes stale:** something read from it after the specification changed, or a caller that still calls `design()` itself.

## 7. Flag-and-stop conditions

- **A committed specification's `validate` or `emit` output moves.** Stop with the diff.
- **A row cannot be drawn by any minimal specification** without a board. Stop and name it.
