# AET-WO-2309 — FEEDBACK: replay and the trace writer hold their own bounds (STOPPED at §7)

**Branch:** `wo/aet-2309-replay-bounds` from `main` `28fba951`. Main `fa9d7edd` was merged in before the push. It changes docs only (`docs/reviews/`, one roadmap line), so every sweep stands.
**Builder:** Claude (Opus 5.5), single-threaded: the work is gate logic in one file. Written 2026-11-23.

**Status: STOPPED at §7, "a committed archive or trace fixture fails the stricter replay".** Rule (3) as the work order states it refuses tier-1 trace fixtures that main accepts. No byte of `crates/`, `fab/`, a workflow or `ci/` changes on this branch. The only changes are `spike/2309/` and this file.

The three rules are built and measured as patches under `spike/2309/patches/`:
- `rules-1-2.diff`: rules (1) and (2) with their tests. fmt is clean, and clippy and the suite pass on `aethelgard-chaos`. The workspace suite was not run with it, but sweep C shows that rule (2) refuses no input main accepts in any crate;
- `rules-full-reading.diff`: all three rules, no new tests.

**The answer, in six lines:**
- **Rule (3) refuses fixtures main accepts, under every two-way reading.** Under the full reading it refuses 43 (input, test) pairs in 20 tests. Under the narrowest two-way reading it refuses 6 pairs in 2 tests. One of the two is outside the fence, in `aethelgard-evolve/tests/attribute.rs`. Named in §2.
- **Every refused fixture is a shape the judge never writes.** `cli/src/main.rs` `boot_report` fills every kind 11 value, kind 12 and 21 value and status, and kind 13 and 22 line set with the same `observed.map(..)`. So they are read together.
- **No real archive moves.** The four committed archives are trace 14, refused `UnknownVersion` before any rule, identically before and after. All 64 current-version archives made by main's CLI over 3306's groups replay byte-identically under the full patch. So does a flooded-log run made for this work order.
- **Rules (1) and (2) move nothing.** Rule (2) refuses no fixture main accepts in the whole tier-1 suite. Rule (1) is reached only by its own test. Both are ready to land with their tests and five mutants, all killed (§4), if the architect wants them ahead of the ruling.
- **Adversarial item 3 gets through the patch as written.** A boot read as wholly unread that still holds its motor witnesses (kind 26) replays. Kinds 26, 29, 30, 34, 36 and 37 are written only once Renode has read (`cli/src/campaign.rs`), and rule (3) does not list them (§5).
- **Rulings needed:** D1 (which reading of rule (3); its fixtures reshaped to the judge's shape; the fence widened to one evolve test, which every reading that closes 5.8 refuses), D2 (rules (1) and (2) now or with (3)), D3 (the unlisted kinds). §6.

## 0. How it was made

1. **Predictions first:** `spike/2309/predictions.md`, committed at `4a84ce75` before any sweep. It also states readings R1 to R4: fence, rule (3)'s witnesses, rule (2)'s scope, rule (1)'s refusal.
2. **The rules,** built in the worktree as the work order states them (R1–R4).
3. **Three sweeps:**
   - **A:** the committed archives, by main's CLI and the branch's;
   - **B:** 64 current-version archives made once by main's CLI in the tools image, under `capped-run` (8 GiB, swap 0, read back), replayed by both CLIs on the host;
   - **C:** every `check_run` input the tier-1 suite builds, judged by main and by each candidate rule. This is an instrumentation patch on a copy of main, so nothing panics and no fixture is masked.

   Results: `spike/2309/results.md`.
4. **The stop.** Sweep C showed the fixtures. The rule patch was saved under `spike/2309/patches/` and `crates/` restored to main's bytes, so this branch is green on main's own tests.
5. **Rules (1) and (2), tests and mutants,** were built and measured in a scratch copy and saved as `rules-1-2.diff`, so the ruling costs one apply.

## 1. The rules, as built in the patches

- **(1) `TraceWriter` refuses at `MAX_TRACE_BYTES` by name.** `append` refuses an event that would take the trace past the bound with a new `ArchiveError::TraceFull { index, bytes, limit }` (kind `TraceFull`). The check runs before the event is folded, so the writer's state hash never runs ahead of its file, and what came before commits.
  - A trace of exactly 268,435,456 B is accepted, as the reader accepts it (`>`, not `>=`).
  - A new variant, because `TraceTooLarge`'s text says "the manifest declares": that is the reader's case (R4). No crate outside `aethelgard-chaos` matches `ArchiveError` exhaustively, so no CLI edit follows.
- **(2) Exactly one kind 26 per kind 24 in a booted run.** `check_motor_joins` already refused a second witness ("… or is there twice"). It now also refuses a booted run in which a solved output has none: "a booted run's solved motor output has no witness". `booted` is `check_run`'s own (the boot word read as the firmware's), now computed before `check_joins` and passed down.
- **(3) All read or all unread across kinds 10–13, 21 and 22:** `check_read_with_the_boot`, called in `check_run` before the joins.
  - The kind 11 value, and the kind 12 and 21 value and status, are read exactly when kind 10 is.
  - A kind 13 or 22 beside an unread kind 10 holds no observed lines and no `absent` or `unrouted` evidence.
  - A read boot may hold an unread witness (R2): measured, §3.

R1, the fence: rules (2) and (3) are archive rules, so they sit in `campaign.rs`'s `check_run` and `check_motor_joins`, beside the "at most one" they tighten. The scenario format is untouched. `Replayer` is also driven by the writer and by non-campaign streams (golden's every-kind trace), so a fold-time rule would refuse those.

## 2. The §7 stop: the fixtures rule (3) refuses

Measured by sweep C (`spike/2309/out/sweep-c.txt`). Each `check_run` input main's tier-1 suite builds is judged by main and by each candidate rule. Main's suite passes 2,784 tests, 0 failed.

| Reading | Accepted by main, refused by the rule | Tests |
|---|---|---|
| **full** (as the patch is built: kinds 11, 12 and 21 value and status read exactly when kind 10 is) | 43 pairs | 20: 19 in `chaos/tests/campaign.rs`, 1 in `evolve/tests/attribute.rs` |
| **status** (kinds 12 and 21 bound both ways by their status, their value one way) | 6 pairs | 2: `chaos/tests/campaign.rs:3073` `a_boot_that_read_nothing_names_its_bound` (5) and **`evolve/tests/attribute.rs:1581` `a_register_with_nothing_observed_is_no_evidence`** (1) |

The full reading's 20 tests:
- `aethelgard-chaos/tests/campaign.rs:1064` `the_47k_shaped_run_matches_what_it_expects`
- `aethelgard-chaos/tests/campaign.rs:1073` `a_run_expecting_fewer_kinds_does_not_match`
- `aethelgard-chaos/tests/campaign.rs:1082` `a_run_expecting_more_kinds_does_not_match`
- `aethelgard-chaos/tests/campaign.rs:1089` `a_failing_run_expecting_to_pass_does_not_match`
- `aethelgard-chaos/tests/campaign.rs:1098` `info_findings_never_count_toward_a_match`
- `aethelgard-chaos/tests/campaign.rs:1273` `a_held_low_run_is_ok_and_matches`
- `aethelgard-chaos/tests/campaign.rs:1353` `the_good_run_carries_one_solved_bus_and_passes_the_joins`
- `aethelgard-chaos/tests/campaign.rs:1367` `two_solved_records_for_one_bus_or_unsorted_buses_are_refused`
- `aethelgard-chaos/tests/campaign.rs:1442` `a_witness_must_name_a_solved_device_on_its_own_controller`
- `aethelgard-chaos/tests/campaign.rs:2265` `two_devices_on_one_controller_have_two_witnesses`
- `aethelgard-chaos/tests/campaign.rs:2355` `pc_outside_firmware_is_exactly_a_pc_outside_the_archived_segments`
- `aethelgard-chaos/tests/campaign.rs:2447` `the_boot_read_is_whole_and_the_pc_findings_are_the_judges`
- `aethelgard-chaos/tests/campaign.rs:2585` `a_device_with_three_steps_round_trips_and_replays`
- `aethelgard-chaos/tests/campaign.rs:2657` `several_configurations_each_by_its_own_name_replay_in_order`
- `aethelgard-chaos/tests/campaign.rs:2893` `the_brown_out_findings_are_exactly_what_kind_10_shows`
- `aethelgard-chaos/tests/campaign.rs:3037` `a_line_bound_only_by_another_bus_or_another_line_is_refused`
- `aethelgard-chaos/tests/campaign.rs:3073` `a_boot_that_read_nothing_names_its_bound`
- `aethelgard-chaos/tests/campaign.rs:3142` `boot_word_missing_is_named_exactly_when_the_word_is_wrong`
- `aethelgard-chaos/tests/campaign.rs:3189` `a_record_judged_without_the_boot_word_is_refused`
- `aethelgard-evolve/tests/attribute.rs:1581` `a_register_with_nothing_observed_is_no_evidence`

On the branch with the full patch, the chaos suite fails 47 tests, not 20. The other 27 build fixtures main already refused, and the new rule, which runs first, now refuses them for its own reason.

**The fixture outside the fence.** `a_register_with_nothing_observed_is_no_evidence` builds a read, healthy boot and sets one register (`GPIOB.AFRL`) to unread. It pins that attribution reads that register as no evidence. That is the adversary's 5.8 shape itself, and every two-way reading refuses it before attribution sees it. Building rule (3) means changing evolve's test, and evolve is outside this work order's fence.


**Why these are fixture shapes and not the judge's.** In `cli/src/main.rs` `boot_report`:
- kind 11's `observed: observed.map(|o| word(o.registers[i]))`;
- kinds 12 and 21's `observed` and `status`: both `observed.map(..)` of the one record word pair;
- kinds 13 and 22's lines: `observed.and_then(|o| o.witnesses[i].clone())`.

The judge reads all of them in the read that reads the boot, or none. The fixtures depart from that in two ways:
- **`good()`** (`tests/campaign.rs` L901) and the step helpers (`i2c_step_check`, L2538; the two-device runs) hold a read boot with a kind 12 whose value is `None` beside a status (`timeout_txis`, and configure steps).
- **`unread()`** in `a_boot_that_read_nothing_names_its_bound` (L3074) unreads kind 10's boot word, count and `pc`, but leaves its brown-out word, kind 11 `R` and kind 12's status read.
  - Measured (`spike/2309/adversarial/probes.txt`): that kind 10 already fails its own field rule ("… not all read or all unread"), and `TraceWriter` refuses it as `InvalidEvent`.
  - So `unread()` can never be written or read as a trace. It reaches `check_run` only because tier-1 tests call `check_run` on records directly, without the field rules.

**Measured cost of reshaping, in a scratch copy, not on this branch.** `good()`'s kind 12 value set to `Some(0)`, as the judge would write it beside `timeout_txis`. Under the full reading the chaos suite then fails 5 tests, not 47:
- `a_boot_that_read_nothing_names_its_bound`;
- `a_device_with_three_steps_round_trips_and_replays`;
- `a_step_given_twice_or_out_of_order_is_refused_by_replay`;
- `several_configurations_each_by_its_own_name_replay_in_order`;
- `two_devices_on_one_controller_have_two_witnesses`.

These are the step helpers' and `unread()`'s shapes.

## 3. Sweeps A, B and C

- **A (committed archives).** Four, all under `spike/3305/out/archives`, trace version 14.
  - Main's CLI and the branch's (full patch) give byte-identical output: `UnknownVersion`, exit 3.
  - The sweep is vacuous by construction: no record rule runs on them (P1).
- **B (64 current-version archives).** 3306's groups unchanged, made once by main's CLI.
  - All replay with exit 0 and byte-identical output under both CLIs.
  - The census (`census2309`, through `replay_dir` alone):
    - 63 runs read and booted, 1 read and not booted, **0 unread**;
    - 1,890 kind 11 and every kind 12, 13, 21 and 22 record read;
    - 15 runs with 4 kinds 24 and 4 kinds 26;
    - the largest trace 9,056 B.
  - `bme-bmp` and `bme-bmp-0x47` no longer generate on main. Their specification is 3306's spike file, not a committed group.
- **The flooded log (P8).** One campaign run of the tier-2 flooding firmware, made for this work order:
  - read and booted;
  - kinds 11, 12 and 21 read;
  - **kinds 13 and 22 unread**, because the log passed the filter's budget.

  It replays identically under the full patch. So rule (3) can bind the witnesses only one way, as built. It is committed as `spike/2309/out/flood/7a624e30…`, the repository's first trace-18 archive. A later sweep A will not be vacuous.
- **C (tier-1 fixtures).** As §2. Rule (2) refuses **no** input main accepted. The two inputs it refuses, main refuses already, for other reasons ("names no solved output", "is there twice").
- **Unmeasured halves, said plainly:**
  - no unread boot exists in sweep B, so "an unread boot reads nothing" rests on `boot_report` and on the tier-1 fixtures;
  - no read but unbooted run in sweep B has a motor output, so P5's unbooted half rests on `boot_report` too.

## 4. Rules (1) and (2): tests and mutants (`patches/rules-1-2.diff`)

**Gates on the patched copy:**
- `cargo fmt --all -- --check` clean;
- clippy on `aethelgard-chaos --all-targets -D warnings` clean;
- the chaos suite: 358 passed, 0 failed.

The workspace clippy and suite were not run with the patch: it changes only `aethelgard-chaos`, and no other crate matches `ArchiveError` exhaustively. Sweep C's motor candidate covers every crate's fixtures.

**Tests:**
- `bounds.rs` `the_writer_refuses_an_event_that_would_take_the_trace_past_256_mib`:
  - fills the trace with 1,024-byte fault events (not records, so `MAX_RECORDS` never decides);
  - ends two events short and fills exactly to 268,435,456 B;
  - checks that an event too large by about 1,000 B, and one too large by 1 B, are each refused as `TraceFull` with their index and byte count;
  - checks that the exact fill is accepted and the smallest event after it refused;
  - commits and replays the result. **259,358 events, written and replayed in 5.62 s** in a debug build, under load (P9 held).
- `motor.rs` `a_booted_run_holds_exactly_one_witness_per_solved_output`: one or both witnesses dropped from a booted run, refused by name; none, one or both dropped from an unbooted one, accepted.

**Mutants** (`spike/2309/sh/mutants.py`, one per guard, against the chaos suite; `spike/2309/out/mutants-1-2.txt`). All five killed by the new tests' named refusals:
- `w-gone`, the guard removed;
- `w-at-bound`, `>` to `>=`, killed at the exact-fill append;
- `w-after-fold`, the check moved after the fold, killed by the second refusal's index;
- `m-gone`;
- `m-unbooted-too`, the `booted &&` dropped. It also kills `serial.rs` `a2_5_replay_binds_an_anchor_after_the_boot_word`, whose unbooted run holds kind 24 and no kind 26. So "booted", not "read", is what keeps that fixture (R3).

**Test weights:** not touched. `bounds.rs` gains about 5–6 s. The fence allows a test-weights row; none was written, since nothing lands.

## 5. Targeted adversarial pass (§5), self-run, probes in scratch copies

`spike/2309/adversarial/` holds each probe's source and output.

1. **A writer that crosses the bound in one large record.**
   - The check is the sum of the bytes written and the encoded event, compared with the bound before the fold. An event's size cannot change that comparison: the event bound is 4,096 B, so no overflow is possible.
   - The test refuses an event about 1,000 B over the remainder, and one 1 B over it.
   - **Held.**
2. **A booted run with kind 24 and no kind 26, or two of kind 26, that still replays.**
   - Dropped witnesses: refused by the new rule. A second witness: refused by the old one.
   - A dropped witness replaced by a duplicate of another output's: refused ("is there twice").
   - A DShot run with its witnesses and its judge's stop all dropped: refused by the new rule.
   - **Residual, the work order's scope:** a read but unbooted run may still drop its witnesses (probe: accepted). The judge writes them there too (P5), so binding them is a "read" rule. That would refuse `a2_5_replay_binds_an_anchor_after_the_boot_word`'s fixture (§4): a second §7 fixture, so not built.
3. **A mixed read/unread run that slips through a kind not listed.**
   - **Found:** under the full patch, a boot read as wholly unread (kind 10's four all `None`, the verdict naming `cpu_aborted`) that still holds its kind 26 witnesses replays as accepted. So does the same run without them.
   - The judge writes kind 26, and likewise kinds 29, 34, 36 and 37 (and kind 30's read-back), only once Renode has read: `cli/src/campaign.rs` `continue`s on an unread witness.
   - So a forger can put witness evidence beside an unread boot.
   - Rule (3) as worded lists 10–13, 21 and 22 only. This is D3.

## 6. Decisions needed

- **D1 (architect): which reading of rule (3), and its fixtures.**
  - (a) Full: value and status both ways for 11, 12 and 21; witnesses one way. It refuses 20 tests' fixtures. Building it means reshaping `good()`, the step helpers and `unread()` to the judge's shape (chaos test edits, inside the fence), and evolve's `a_register_with_nothing_observed_is_no_evidence` (outside it).
  - (b) Status: kinds 12 and 21 bound both ways by their status, their value one way. It refuses 2 tests' fixtures: `unread()` (chaos) and the evolve test (outside the fence).
  - (c) One way only (a read boot reads everything, by status). It is not "all read or all unread", and it leaves the converse open. It still refuses the evolve fixture, a read boot with a register unread: **any rule that closes 5.8 refuses that fixture, because it is 5.8's shape.** So no reading avoids an evolve test change.
  - **Lean: (a), with the fence widened to `aethelgard-evolve/tests/attribute.rs` for that one test.** It is the judge's own shape, so it is the strictest rule that no real archive breaks, and the reshape is mechanical. (b) leaves a read boot's value free beside its status, which the judge never writes. The evolve test then becomes a replay refusal, or is removed: attribution can no longer meet its input.
- **D2 (architect): rules (1) and (2) now, or with (3).** `rules-1-2.diff` is green and its mutants killed. It can land under this work order once (3) is ruled, or alone first. **Lean:** with (3), one PR, as the work order is written.
- **D3 (architect): the unlisted kinds (§5 item 3).** Add "an unread boot holds no kind 26, 29, 30, 34, 36 or 37" to rule (3). Kind 19 is already bound ("a boot that read nothing holds an observation"). Measured on kind 26; the other kinds are read at source. **Lean:** yes, in the same rule; it moves no sweep B archive, since none is unread.
- **D4 (architect, note only): "booted" or "read" for rule (2).** Built as the work order says, booted. Read would close §5 item 2's residual and is §7 on `serial.rs`'s fixture.

## 7. Reduction pass

Diff-sized, over this branch: `spike/2309/` and this file. Nothing in `crates/`.
- Removed: the status-reading delta file. It was a diff of two diffs; the one-line variant is stated in `results.md` instead.
- Kept: `sweep-instrument.diff` (both sweep C runs need it), `census/` (its own `[workspace]`, as 3306's dump), `sh/` (each script ran once and is cited).
- `rules-full-reading.diff` and `rules-1-2.diff` overlap in rules (1) and (2). Both are kept: one is what sweeps A, B and C measured, the other what would land.

## 8. For the backlog

- **Tier-1 tests call `check_run` on records that the field rules would refuse** (`unread()`'s kind 10). A shared fixture constructor that runs `Record::check` would stop a check_run-only shape from standing in for a trace.
- **Sweep A is vacuous until the committed archives are current.** The flooded run added here is the first trace-18 archive; the four 3305 archives stay version 14.
