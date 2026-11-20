# AET-WO-2306 — A gate that judged nothing says so

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2306-nothing-judged`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P3 (the verdict). Written 2026-11-20. **Cadence:** build. Required by P4's second-board clause: "every gate shows what it judged, and a gate that judged nothing counts for nothing".

**Grounds** (`4600` F5, #245; roadmap §5, the P4 exit gate's definition, adopted 2026-11-19):
- **On `4600`'s motor driver, three gates judged nothing and still passed:**
  - `extract` with `lines: []`;
  - `speeds` with `judged: 0`;
  - the droop observation with no device witness.
- **Where this lives in the code:**
  - `cli/src/extract.rs` about L187;
  - `cli/src/speeds.rs` about L16–17;
  - `chaos/src/campaign.rs` about L1215–1220.
- **Why it matters:** a pass that judged nothing reads like a pass that judged something. The P4 clause now says such a gate counts for nothing, so the verdict must show it.

> **What this is:**
> - each gate's verdict line carries what it judged (a count, or the named items in scope), and a gate with nothing in scope reports `pass` with `nothing_in_scope: true` and a reason;
> - `verify`'s overall verdict lists the gates that judged nothing, separately;
> - the same for `campaign` observations with no witness.
>
> It is NOT:
> - turning an empty pass into a failure. The gate's result is unchanged, and the clause's reading is the reviewer's;
> - any change to what a gate checks.

## 1. Build

1. **Inventory first.** In FEEDBACK, list every gate `verify` runs, each with:
   - what it judges;
   - where its count lives today;
   - which committed boards have it judge nothing. Run all committed specifications and record them.
2. **The field.** Each gate's verdict line gains `judged` (a count) and, when 0, `nothing_in_scope: true` with a one-line reason: no bounded line, no declared speed, no witnessed device.
   - Additive JSON only. Existing keys and values do not move.
   - Say how consumers that compare verdicts (`--against-fab`, goldens) treat the new keys.
3. **`verify`'s summary.** The summary gains a `judged_nothing` list of gate names, so the reader sees it without reading every line.
4. **The campaign.** An observation with no device witness says so, in the same form.
5. **Tests:**
   - a board where each of the three gates judges nothing: the fields appear;
   - Sentinel-01: no gate is in `judged_nothing`, or the list is exactly what the inventory found;
   - a mutant: always `nothing_in_scope: false`, and the first test fails.

## 2. Acceptance

- **The inventory** is in FEEDBACK.
- **The fields appear** exactly when a gate judged nothing. The tests and the mutant are shown.
- **The bench package:** `MANIFEST.json` binds the verdict, so expect it to move. Do not hand-edit it. Say so on the bus, and the architect dispatches `write=true`. You commit the artifact byte for byte (the fence is widened to that).
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-06`.
- **Fence:**
  - `crates/aethelgard-cli/src` (verdict lines for `extract`, `speeds`, `verify`, campaign observations);
  - `crates/aethelgard-chaos/src` (the observation's witness field only);
  - tests and goldens moved by tooling;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only.
  - **Not:** what any gate checks, the IR, the twin, workflows, or `ci/` (except a weight row P1/P2 asks for).
- **Running beside you:**
  - `2305` (BOR recovery), which touches `verify.rs`'s coverage and the campaign;
  - `3305` (#256), `1103` (#262) and `4240v` (#259), all in the bench-package queue.

  You join that queue. Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-2306/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A gate that judged something but reports 0,** for example counting the wrong collection.
- **A gate with items in scope that are all skipped:** is that "nothing judged"? Say which, and why.
- **A verdict consumer** that breaks on the new keys.

## 7. Flag-and-stop conditions

- **A gate whose "in scope" cannot be counted** without changing what it checks. Stop and name it.
