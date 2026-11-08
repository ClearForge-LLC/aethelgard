# AET-WO-1201 — The solver's forced-instance check

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1201-forced-instances`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1, reserved at review of PR #32.

**Grounds:**
- **`AET-WO-1200` FEEDBACK A1.** Some infeasible specifications spend the whole 10 M-node budget, in 5.6 to 6.6 s release, and refuse `BudgetExceeded` naming the first bus of the colliding set rather than the collision.
  - The cause: two instances' only remaining pin for one signal is the same pin. Pin matching unions over instances, and instance matching ignores pins, so both checks pass and the search re-fails under every earlier choice.
  - The reproducers are in `docs/work-orders/AET-WO-1200.repro/`.
- **The reviewer's proposal, unmeasured.** When the buses left of a kind equal that kind's placeable instances, every instance is forced. Pin matching can then run over (instance, signal) rows.
- `docs/architecture.md` §2.11, `RUL-02`; `docs/northstar.md` §4 `INV-01`.

> **What this is:** one more pre-check in `solve.rs`'s search. It can only prune, so it never changes a feasible answer.
>
> It is NOT:
> - a change to the search order, the budget, the refusal kinds or `SolvedTopology`;
> - a change to any other crate.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **The check.** Before every node, alongside instance and pin matching:
   - for each bus kind where the number of unplaced buses equals the number of instances still placeable for it, every instance is forced;
   - run bipartite matching from (instance, signal) rows to free pins;
   - fail the node if any row is unmatched.

   It answers yes or no only.
2. **The differential test, which is the proof that it only prunes.**
   - Generate at least 2,000 specifications from a fixed seed, in the shapes of the 1200 fixtures, with buses and hand pins drawn at random. Use a deterministic PRNG written in the test, not a new dependency.
   - Solve each with the check and without it. The test-only switch is `solve_with_options`, or a `cfg(test)` flag, whichever keeps the public API unchanged.
   - The two results must be identical, except that a `BudgetExceeded` without the check may become `Infeasible` with it.
   - Record the count of each outcome in FEEDBACK.
3. **The reproducers.**
   - `fuzz-natural-6spi.json` and `core-infeasible.json` must now refuse `Infeasible`, naming the colliding SPI buses, within 1 s in release.
   - Record node counts and times before and after.
4. **The rest of the tests:**
   - every existing solve test passes unchanged;
   - two solves are byte-identical;
   - the 1200 and 1202 golden digests do not move.

## 2. Invariants and fences

- `INV-01`, `INV-06`. No `unsafe`. No `HashMap`. No new dependency.
- **Fence:** `crates/aethelgard-ir/src/solve.rs`, `crates/aethelgard-ir/tests/`, and this work order's FEEDBACK. Protected surfaces diff to empty, as in every work order.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

§1.2 to §1.4 pass in `gates` and `tools`. CI is green on the first push.

## 5. Adversarial pass — targeted

1. Find a feasible specification the check refuses. The differential test should have found it, so widen the generator until you are confident it cannot.
2. Make the check's cost dominate a feasible solve: measure the stress fixture before and after.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- Any differential mismatch other than `BudgetExceeded` becoming `Infeasible`.
- A change outside the fence.
- A test that can only go green by being weakened.

## 8. Kickoff prompt

```
/goal AET-WO-1201: §1 is built and §3 passes in gates and tools, or a §7 stop is recorded in FEEDBACK; the targeted adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1201-forced-instances.
```
