# AET-WO-1313d — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read at source]** from a file and line named beside it; **[inferred]** from reasoning.

## 0. Status: READY

- **Compaction checks its own termination argument as it runs** (`aethelgard-eda` `pcb.rs`, `compact`):
  - every move must lower the moving group's potential strictly;
  - the moves may not pass a bound derived from that argument and the geometry before the first move.

  Either failure is the new refusal `CompactionUnbounded`, naming the group's IC, its potential, the move count and the bound. Nothing is written.
- **The 1313c mutant of `shift`** (the keep-out box left behind) used to run past every timeout. It is now refused by name at its first move: the eda unit tests fail three tests in **0.64 s**, against 3.4 s for the ordinary run [measured].
- **No byte moves [measured]:**
  - the 24-specification corpus is byte-identical against `main` `65622f70`'s binary (24/24, 371 files);
  - the bounded Sentinel-03 fixture's `emit` output is byte-identical;
  - its outline stays 85.25 × 80.95 mm, and its placement digest pin passes.
- **The bound on the corpus's bounded board:** 25,081 moves against the 74 compaction makes (339:1), well under 10⁶ (§2).
- **No stop:** no byte moved, and the bound needs no typed-in constant (§1.1).

## 1. What is built

### 1.1 The bound, derived where the loop is (`move_bound`)

Its doc comment holds the proof from the loop's own argument. The potential is `(|dy|, |dx|)` from the group's keep-out centre to the MCU's courtyard centre.
- **Moves in y:** one is taken only when `|dy| >= STEP`, and it takes exactly one `STEP` off. So a group makes at most `ny = |dy0| / STEP` of them.
- **Moves in x:** one leaves `|dy|` alone and takes exactly one `STEP` off `|dx|`, which is never below 0. So the moves in x number at most `|dx0| / STEP` plus every step a move in y adds to `|dx|`.
- **Bound by the sidestep:** a move in y moves sideways by at most `SIDESTEP_STEPS` steps, giving at most `|dx0| / STEP + ny * SIDESTEP_STEPS` moves in x.
- **Bound by the width:** the group's centre and the target stay inside the core's x-range, so `|dx| <= w` after every move in y, giving at most `|dx0| / STEP + ny * (w / STEP)` moves in x.
- **Per group,** the smaller of the two bounds holds: `ny + |dx0| / STEP + ny * min(SIDESTEP_STEPS, w / STEP)`. Compaction's bound is the sum over groups.

The work order's `(ny + 1) * (w / STEP + 1)` also holds; this one is tighter by the proof above. It is 5.6 times smaller on the fixture: 25,081 against 140,254, the work order's form with the width term alone [measured].

**No typed-in constant:**
- the starting potentials and `w` are read from the geometry before the first move;
- `STEP` and `SIDESTEP_STEPS` are the move rule's own constants, already there;
- nothing comes from the environment or the specification.

**Computed once, before the loop,** from values the loop does not change (§3).

### 1.2 The checks

- **Per move:** after a group moves, its potential is recomputed from where its members' keep-out boxes now are, not from the step it meant to take. A potential that is not strictly lower (lexicographically) is refused at that move: "the move did not lower its potential".
- **The count:** each move is counted, and a count past the bound is refused: "the moves passed the bound its termination argument gives". The strict-fall check catches a broken move at once; the bound catches a fall too slow for the argument, the case the work order says the bound covers.

### 1.3 The refusal

`EmitError::CompactionUnbounded { reference, dy_nm, dx_nm, moves, bound, why }` (`schematic.rs`):
- `kind()` is `"CompactionUnbounded"`, and `id()` is the group's IC reference.
- Its message reads "compaction refused at `U2`, move 1 of a bound of 9884: the move did not lower its potential; its potential is |dy| 12.125 mm, |dx| 14.385 mm".
- `place` returns it, so `emit` writes nothing, as `OutlineTooSmall` and `OutlineUnreachable` do.
- **The CLI needs no new line:** `emit`'s refusals reach JSON through `kind()` and `id()` generically [read at source, `outline_tier1.rs` L135 for `OutlineUnreachable`'s JSON].

### 1.4 The seam, and why this one

- **What it is:** a `#[cfg(test)]` thread-local mover (`tests::MOVE`, holding `shift` by default). `compact` reads it in test builds only, through two `#[cfg]` lines; release builds call `shift`. It is not a runtime switch.
- **Why this one:** every test reaches compaction through `layout_of` and then `place`.
  - A mover parameter would have to be threaded through `place` and `layout_of` and every one of their callers.
  - The thread-local adds the least code: a type alias, the static, and two `let` lines.
  - It is per thread, so the parallel test runner's other tests keep `shift`.

### 1.5 Tests (`pcb.rs`)

- **`the_move_bound_follows_the_termination_argument`** checks the bound's arithmetic on two hand-built groups:
  - group A: 4 moves in y and 2 in x, giving 166 in a 20 mm core (the sidestep caps it) and 86 in a 5 mm core (the width does);
  - group B: 10;
  - both together: 176; no groups: 0.
- **`a_compaction_whose_termination_argument_fails_is_refused_by_name`:** an MCU and one sensor under a bound, through the seam.
  - **The keep-out box left behind** (1313c's mutant of `shift`) is refused "the move did not lower its potential".
  - **A one-nanometre move** lowers the potential strictly but far too slowly, and is refused "the moves passed the bound".
  - Each refusal is asserted by kind, id (`U2`) and reason, within 1 s. Both run together in 0.01 s.

## 2. Acceptance (WO §2) [measured]

**Byte-identical:**
- **The corpus:** `corpus.sh` (`1313c`'s) compares `emit` and `generate` on every specification with a board and an MCU and no bound, `main` `65622f70`'s release binary against this branch's: 24/24, 371 files.
- **The bounded fixture:** its `emit` output is byte-identical between the two binaries (3 files).
- **Tier 1:** `outline_tier1` 8/8 holds the fixture's 85.25 × 80.95 mm outline and its placement digest pin, the size samples and the turned IMU.
- **Tier 2 in the image:** the two Sentinel-03 tests give the same answers as `1313c` (fixture 9.2 pF; samples −5.1 pF refused, 5.4 pF, `OutlineUnreachable`).
- **The bench package does not move:** it declares no bound, and compaction runs under a bound only.

**The bound against the moves made** (temporary, uncommitted instrumentation at `compact`'s return; `~/.cache/aet-1313d/bounds.txt`). Seven groups on each board.

| Board | Moves | Bound | Bound / moves |
|---|---|---|---|
| **Sentinel-03 fixture** (the corpus's one bounded specification) | **74** | **25,081** | **339** |
| size sample, 2-character references | 67 | 27,371 | 409 |
| size sample, 6 | 74 | 27,846 | 376 |
| size sample, 8 (refused later, `OutlineUnreachable`) | 74 | 28,106 | 380 |
| turned IMU, four turns (0/90/180/270) | 74 to 116 | 24,317 to 25,082 | 210 to 339 |
| 20 order-only renamings of the fixture | 74 each | 25,081 each | 339 |

- **Every bound is under 10⁶.** The ratio is dominated by the `ny * min(SIDESTEP_STEPS, w / STEP)` term: the worst case lets a full sidestep follow every move in y, which these boards never use.
- **The bound is not meaningless:** a correct compaction makes about 0.3 % of it. A fault that slows the fall, like the one-nanometre mover, is refused within the bound (in 0.01 s on the test's board).
- **Upper limit on the fixture:** at most 25,081 moves past the last correct one, about what `emit`'s compaction already costs per move [inferred].

**The `shift` mutant:** the keep-out box left behind, `1313c`'s survivor, run on the build.
- The eda unit tests fail by name in 0.64 s of test time (0.69 s wall, test binary built), against 3.4 s for the ordinary run:
  - `a_compaction_whose_termination_argument_fails_is_refused_by_name`;
  - `under_a_bound_no_reference_decides_a_placement`;
  - `under_a_bound_the_board_does_not_depend_on_the_parts_order`.
- The two board tests fail with `CompactionUnbounded { reference: "U4", moves: 1, bound: 9884, why: "the move did not lower its potential" }`.
- On `1313c` it ran past the harness's 900 s timeout.

**`emit` time unchanged:** the fixture takes 3.05 to 3.19 s on `main`'s release binary and 3.09 to 3.10 s on this branch's (three runs each).

**Gates:**
- fmt; clippy `-D warnings` with `with-tools`;
- `cargo test --workspace` 2180 passed, 0 failed;
- `ci/shard.sh plan` in the image: guard ok, 2384 tests, 188 tier 2;
- `TIER2_MIN` stays 188, since no tier-2 test is added. The two unit tests take about 0.01 s, under `ci/shard.sh`'s 0.5 s weight-row rule (`DEFAULT_WEIGHT` 0.05 s).

## 3. Adversarial pass (WO §5)

- **A bound a correct compaction can reach (a false refusal):**
  - by the proof (§1.1), no correct run can exceed the bound;
  - on every bounded board and variant, the moves are 0.3 to 0.5 % of it;
  - the mutant `bound-from-final`, which caps the bound at 1, makes the real boards refuse. Three eda tests and six of `outline_tier1`'s eight fail by name, so the test suite would catch a bound computed too small.
- **A bound computed from state the loop changes:** it is computed once, before the first move, from the groups' starting centres and the core. Neither changes inside the loop: `groups` is immutable, and `core` is a value taken before it. The count is checked against that value.
- **A potential check that a correct sidestep fails:** a move in y with a sideways offset may raise `|dx|`, but it lowers `|dy|`, which comes first lexicographically.
  - The mutant `potential-swapped` orders the potential `(|dx|, |dy|)`, and real boards' sidesteps then fail it. Eight tests fail by name: two eda unit tests and six of `outline_tier1`'s eight.
  - So the real boards do make sidesteps that raise `|dx|`, and the correct order accepts them.

## 4. Mutation (diff-sized)

8 mutants on the guards this order adds, run against the eda unit tests and `outline_tier1`: **all killed** (`~/.cache/aet-1313d/mut`).

| Mutant | Killed by |
|---|---|
| the strict-fall check off | the refusal test (the keep-out case then hits the bound, so the reason differs) |
| the strict-fall check non-strict (`>`) | the refusal test |
| the bound check off | the refusal test (the 1 nm case) |
| the per-y term dropped from the bound | the arithmetic test |
| the width term instead of the min | the arithmetic test |
| the potential's order swapped | two eda tests, six tier-1 tests |
| the bound capped at 1 | three eda tests, six tier-1 tests |
| `shift` leaving the keep-out box behind (1313c's) | three eda tests by name, 0.64 s |

## 5. Reduction pass

Reduction pass (diff): −2 net · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 1 finding below.

It was run by a fresh read-only subagent over the diff. Both changes were re-checked by the eda unit tests and their mutants (4/4 killed).
- **Duplicates removed:**
  - the inline comment restating the bound and the check, which `compact`'s doc paragraph already says;
  - the reason string copied into the error (`why` is now `&'static str`).
- The new `centre` closure replaces the inline union and centre that the loop already had, and now serves the bound too: one copy, not two.
- **Finding, resolved on review:** `crates/aethelgard-eda/README.md`'s paragraph on bounded placement named `OutlineTooSmall` and `OutlineUnreachable` but not `CompactionUnbounded`. The architect's review of `4876fdfb` on #220 extended the fence to that line, and it now names all three.

## 6. Record

- **Base:** `main` `65622f70`. Commits: the build `29dc95c0`; READY `4876fdfb`, accepted on review; the README line as asked; `main` `80e86b1c` merged (docs only: this order's work order and the roadmap); this READY commit (FEEDBACK only).
- **CI:** it runs on pull requests only (`.github/workflows/ci.yml` L3–6). The kickoff asks for the PR to open at READY, so CI's first run is on this READY head. Every gate it runs was run locally first (§2).
- **Files:**
  - `crates/aethelgard-eda/src/pcb.rs` (compaction, the bound, the seam, two tests);
  - `crates/aethelgard-eda/src/schematic.rs` (the variant and its `kind` and `id`);
  - this FEEDBACK.
  - No CLI line was needed.
- **Not built (WO "is NOT"):** no placement change, no tuning knob, no test-pad rule or capacitance-aware step.
