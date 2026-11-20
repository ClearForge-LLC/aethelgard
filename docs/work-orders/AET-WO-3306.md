# AET-WO-3306 — A step sent to another device's address, which answers, is attributed to the software (F-COLLIDE)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3306-collide`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P3 (attribution). Written 2026-11-20. **Cadence:** a measurement, then a build with one stop. **Before `4500`** (roadmap §7.1). **After `2305`** (#271, merged).

**Grounds** (`3305` D3 and §5, #256; roadmap §7.1, F-COLLIDE):
- **The case.** A firmware sends device A's step to device B's address, and B answers. The run fails `i2c_transaction_unwitnessed`, but `attribute` names **I4** (the bus) when the fault is the software's.
  - `3305` measured it: U2's step sent to 0x77, where U5 answers. The result is I4, with kind 13s identical to `main`'s.
- **Why per-step absence does not reach it.** B answers, so no "Unknown slave" line marks the step as absent (`3305`'s attribution).
- **What the case leaves behind.** B's witness carries A's transaction as extra lines: U5's witness gains U2's write and read.
  - Today the rule reads only a witness's first write and first read (`evolve/src/lib.rs` about L987–989, `logged(observed, "Write")` / `"Read"`).
  - So the extra lines are never judged.

> **What this is:**
> - `attribute` judges each witness's whole `observed` sequence against its `expected` steps;
> - a witness carrying a transaction its device's script does not hold names the software (the address the firmware used), not the bus;
> - every committed attribution re-judged, with each move explained.
>
> It is NOT:
> - a change to the twin, the firmware or the witness recording (trace 16 stays);
> - per-step absence (`3305`), which stays as it is.

## 1. Build

1. **Measure first,** in `spike/3306/`, with predictions committed before the runs.
   - Every committed attribution (the `attribute` goldens, `evolve`'s tests, and the campaign's seeded cases): record what `attribute` names on `main`.
   - The collide case (`3305` §5's 0x77 run): record each witness's whole `observed`.
   - **Stop and report** if more than the collide case would change attribution under the whole-sequence rule. Name each, with its records.
2. **The rule.** A witness's `observed` must equal its device's `expected` steps, in order, whole.
   - An extra transaction (a write and read the script does not hold) is software. Name the extra address and step, and the device whose step it was if the plan can tell.
   - A missing or wrong transaction keeps today's attribution.
3. **Replay** re-derives the same attribution from the archived records. No archive field is added, unless the stop rules one in.
4. **Tests:**
   - the collide case names software;
   - every committed attribution is unchanged, or each move is explained and ruled;
   - **mutants:**
     - first-write-and-read only: the collide test fails;
     - an extra transaction treated as the bus: the collide test fails.

## 2. Acceptance

- **The measurement table** is in FEEDBACK, with predictions beside it.
- **The collide case is attributed to the software,** by name.
- **No other attribution moves,** or each move is listed, explained and ruled.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count. If any committed verdict moves the bench package, say so; the architect dispatches.

## 3. Invariants and fences

- **Invariants:** `INV-05` (attribution precedes mutation), `INV-04` (replay), `INV-06`.
- **Fence:**
  - `crates/aethelgard-evolve/src` (`attribute` and its helpers);
  - `crates/aethelgard-cli/src`, only where `attribute`'s output is printed;
  - tests and goldens moved by tooling;
  - `spike/3306/`;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only, if a verdict moves.
  - **Not:** the twin, `renode.rs`'s witness recording, the trace format, the firmware, specifications, workflows, or `ci/` (except a weight row P1/P2 asks for).
- **Running beside you:**
  - `4241` (#279): unmodelled identity reads; its "Unknown slave" lines are attributed to their own records. Read its rule before yours: an unmodelled device's read is not a collide.
  - `4240a` (#281) and `1210` (#280): specification versions;
  - `4405` (#277): pincheck;
  - `1318` (#283): eda.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-3306/pr-title-body.md`.

## 5. Adversarial pass, full

- **A device whose script legitimately repeats a step** (a FIFO burst, a retry): is it named software? It must not be.
- **Two devices that share a step shape,** so an extra transaction matches the wrong one.
- **An unmodelled device's read** (`4241`) mistaken for a collide.

## 7. Flag-and-stop conditions

- **More than the collide case changes attribution.** Stop with the list.
- **The plan cannot tell whose step an extra transaction was.** Not a stop: name the address only, and say so.
