# AET-WO-1201 — FEEDBACK

**Branch:** `wo/aet-1201-forced-instances` · **Base:** `662f0b2` (`origin/main`, PR #46) · **Work order:** `docs/work-orders/AET-WO-1201.md` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Result:** §1 built; no §7 condition hit. Every gate passes on Furnace; CI is in the Gates section.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Findings

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **F1** | **The two reproducers no longer reach the search.** Since `AET-WO-1202` every I2C line needs a net constraint, and neither file has any, so both refuse `MissingBusConstraint` in 0 ms. I measured them on scratch copies with a 100 pF / 300 ns constraint added on each I2C line (not committed: `docs/` is outside this fence). | `docs/work-orders/AET-WO-1200.repro/*.json` | stale input | Add the constraints to the two files (the A1 history keeps the originals in git) | no |
| **F2** | **§1.3 has no committed test on the real table.** `aethelgard-ir` cannot read the real pin table, and the CLI's `solve_cli.rs` is outside the fence. The ir-side proof is a synthetic table of the same shape (six SPI instances, two sharing their only `MISO` pin, a third with a second `MISO` pin so the union still has six). The real-table proof is the measurement below. | `solve.rs` `a_shared_only_pin_is_refused_at_once_not_after_the_budget` | coverage gap | Copy the two constrained reproducers into `tests/fixtures/solve/` and assert `Infeasible` over the six SPI buses in `solve_cli.rs` | no |
| **F3** | **Two documents now say the opposite of the code.** `docs/architecture.md` §2.11 (line 202, "A known gap, `AET-WO-1201`") and `crates/aethelgard-ir/README.md` (lines 131 to 134: two checks, "neither matching check alone covers both shapes") describe the pre-check state. Both are outside the fence. | `docs/architecture.md:202`, `crates/aethelgard-ir/README.md:131` | doc drift | Close the gap paragraph and add the third check to the README list, in the architect's close-out | no |

## Build

### What was built

All in `crates/aethelgard-ir/src/solve.rs`; nothing else changed.

- **The check, `Search::forced_instances`**, called last in `consistent`. Per kind: the unplaced buses are `rest`'s buses of that kind, and the reachable instances are the union of their placeable instances. When the two counts are equal, every reachable instance is forced. It builds one row per (instance, signal), whose options are the free pins that some unplaced bus of the kind could give that signal on that instance, and runs the existing `matching` over all rows of both kinds. It answers yes or no only.
- **Why it only prunes.** The rows are a relaxation of the real choice: the real assignment gives each forced (instance, signal) a pin from that set, and the pins are distinct. The relaxation drops which bus takes which instance, nothing else.
- **The switch.** `solve_with_limit` is unchanged in signature and now calls a private `solve_inner(spec, table, hash, &mut Budget, forced)`. `Search` carries `forced`; `minimal_set` passes it on. The public API, `lib.rs` and the re-exports are untouched.
- **Tests** (both in `solve.rs`, module `differential`, since an integration test cannot reach a private switch):
  - `the_forced_instance_check_only_prunes`: 3,000 specifications from a fixed-seed xorshift64\*, random tables (4 I2C and 6 SPI instances, 6 to 14 pins, signals sometimes missing) and random buses, hand pins, peripherals and chip selects. Each is solved with and without the check at a 20,000-node limit. It asserts the answers are equal (with `BudgetExceeded` to `Infeasible` allowed), that the check never spends more nodes, and that the generator reaches ok, infeasible and other-refusal at least 50 times each.
  - `a_shared_only_pin_is_refused_at_once_not_after_the_budget`: the synthetic collision above. Between the two node counts (limit 300) the check refuses `Infeasible` and the unchecked search refuses `BudgetExceeded`.

### Differential outcomes (§1.2)

The committed run (3,000 specifications, seed `0x9E3779B97F4A7C15`, limit 20,000):

| Without the check | ok | infeasible | budget | other refusal | budget to infeasible |
|---|---|---|---|---|---|
| count | 549 | 1,087 | 0 | 1,364 | 0 |

Other refusals are the pre-search ones (no instance for a bus, pin claimed twice, unknown pin or peripheral, reserved pin, no free pin). Nodes: 152,566 without, 140,434 with.

The tables here are too small to exhaust 20,000 nodes, so the committed run shows no budget case; the budget-to-infeasible flip is the second test and the real-table reproducers.

### The reproducers (§1.3)

Release, on Furnace, wall time for one `aethelgard solve` (F1 describes the inputs). Nodes are the search plus the minimal-set searches, as `solve_with_limit` counts.

| Specification | Before | After |
|---|---|---|
| `fuzz-natural-6spi` | `BudgetExceeded` on `SPIB3`, 10,000,000 nodes, 5.5 s | `Infeasible`, all six SPI buses, 198 nodes, 4 ms |
| `core-infeasible` | `BudgetExceeded` on `SPIB3`, 10,000,000 nodes, 6.9 s | `Infeasible`, all six SPI buses, 186 nodes, 4 ms |

The four existing CLI fixtures have identical node counts before and after: stress 42, `thrash-pins` 41, `pigeonhole-spi` 308, `infeasible-shared` 44. Their golden digests (the 1200 and 1202 `solve_cli` tests) pass unmoved.

## Gates

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`: all pass on Furnace.
- Every existing solve test passes unchanged; two solves are byte-identical.
- **Reduction pass (diff-sized):** one thing cut during the pass, a `debug_assert` on signal order that restated what `Kind::signals` already fixes; nothing else in the diff is removable, since `forced` threading exists only for the differential switch.
- **CI:** see the PR.

## Adversarial pass (targeted, §5)

1. **A feasible specification the check refuses.** None found. I widened the generator (more SPI instances and buses, fewer missing signals, fewer hand pins and peripherals so that fewer specifications die before the search) and ran 150,000 specifications at each of three seeds in release (`0x9E3779B97F4A7C15`, `0x1234567`, `0xDEADBEEFCAFE`): 450,000 in all, about 83,000 of them reaching the search per seed. Zero mismatches; the check never added a node. The one-off runs used the committed test with `SPECS` and the seed edited, then restored; they are not committed.
2. **The check's cost dominating a feasible solve.** Not found. The stress fixture takes 42 nodes both ways, and 200 runs of `aethelgard solve` take 0.54 s both before and after (process start dominates). Per node the check adds one more `matching` over at most 2 x 6 x 3 rows, and only when a kind's unplaced buses equal its reachable instances.

One limit of that evidence: on a feasible specification the check could in principle let a search finish inside the budget that the unchecked search would not, which is `Ok` in place of `BudgetExceeded`. §7 names it as a stop if it ever differs. It did not occur in any run, but the committed generator cannot reach the budget, so it is also not guarded.
