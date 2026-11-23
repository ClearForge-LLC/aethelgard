# AET-WO-2309 — FEEDBACK: replay and the trace writer hold their own bounds

**Branch:** `wo/aet-2309-replay-bounds` from `main` `28fba951`; main merged in before READY (§9).
**Builder:** Claude (Opus 5.5), single-threaded: the work is gate logic in one file. Written 2026-11-23.

**Status: READY, built to the architect's ruling of 01:15Z on the §7 stop.** The stop is PR #340's record at `41a30f0a` (CI green). The ruling:
- **D1:** the full reading;
- **D2:** all three rules in one PR;
- **D3:** kinds 26, 29, 30, 34, 36 and 37 bound too;
- **D4:** booted;
- **fence:** widened to the named fixtures only.

**The answer, in six lines:**
- **(1)** `TraceWriter::append` refuses, as `ArchiveError::TraceFull`, an event that would take the trace past `MAX_TRACE_BYTES`, before it is folded. A trace of exactly 268,435,456 B is written and replays.
- **(2)** A booted run holds exactly one kind 26 per kind 24 ("a booted run's solved motor output has no witness").
- **(3)** Kinds 11, 12 and 21 (value and status) are read exactly when kind 10 is. Kinds 13 and 22 hold nothing beside an unread kind 10. An unread kind 10 holds no kind 26, 29, 30, 34, 36 or 37. Two refusals, each by name.
- **The named fixtures are reshaped to what the judge writes:** four in chaos `tests/campaign.rs`, evolve `tests/attribute.rs`'s L1581 test, and the reseal in cli `tests/campaign_tier1.rs`'s L2081 test. No other test changed.
- **No real archive moves:**
  - all 64 CLI-made archives and the flooded run replay byte-identically under the built branch;
  - the five committed archives give identical output under main's CLI and the branch's.
- **6 new tests, 20 mutants (one per guard), all killed by the new tests' named refusals.** Gates and CI in §9.

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
6. **The ruling (01:15Z), then the build:**
   - rules (1) and (2) from that patch;
   - rule (3) as measured, plus D3's kinds;
   - the named fixtures reshaped;
   - rule (3)'s tests and mutants;
   - sweeps A and B again on the built branch;
   - gates, the adversarial and reduction passes, and this file.

   The patches were then removed (§7).

## 1. The rules, as built

- **(1) `TraceWriter` refuses at `MAX_TRACE_BYTES` by name.** `append` refuses an event that would take the trace past the bound with a new `ArchiveError::TraceFull { index, bytes, limit }` (kind `TraceFull`). The check runs before the event is folded, so the writer's state hash never runs ahead of its file, and what came before commits.
  - A trace of exactly 268,435,456 B is accepted, as the reader accepts it (`>`, not `>=`).
  - A new variant, because `TraceTooLarge`'s text says "the manifest declares": that is the reader's case (R4). No crate outside `aethelgard-chaos` matches `ArchiveError` exhaustively, so no CLI edit follows.
- **(2) Exactly one kind 26 per kind 24 in a booted run.** `check_motor_joins` already refused a second witness ("… or is there twice"). It now also refuses a booted run in which a solved output has none: "a booted run's solved motor output has no witness". `booted` is `check_run`'s own (the boot word read as the firmware's), now computed before `check_joins` and passed down.
- **(3) All read or all unread across kinds 10–13, 21 and 22, and D3's kinds:** `check_read_with_the_boot`, called in `check_run` before the joins.
  - The kind 11 value, and the kind 12 and 21 value and status, are read exactly when kind 10 is.
  - A kind 13 or 22 beside an unread kind 10 holds no observed lines and no `absent` or `unrouted` evidence.
  - A read boot may hold an unread witness (R2): measured, §3.
  - Refused as "the boot's registers, identity reads and witnesses are not all read or all unread with its boot word".
  - **D3:** an unread kind 10 beside a kind 26, 29, 30, 34, 36 or 37, which the CLI writes only once Renode has read. Refused as "a boot that read nothing holds a motor, serial, pin or level witness, or a judge's stop". Kind 19 already had its rule ("a boot that read nothing holds an observation").

R1, the fence: rules (2) and (3) are archive rules, so they sit in `campaign.rs`'s `check_run` and `check_motor_joins`, beside the "at most one" they tighten. The scenario format is untouched. Beyond `archive.rs` and `campaign.rs`, the build touches:
- `src/lib.rs`: the `TraceFull` variant and its `kind()` row;
- the crate's `README.md`: the writer's refusal and the two run rules;
- `tests/common/mod.rs`: one shared helper, `unread_boot`;
- one `ci/test-weights.tsv` row, as the fence allows (§4).

The CLI reports a writer refusal as `ArchiveWriteFailed`, carrying `TraceFull`'s text, which names `MAX_TRACE_BYTES`. No CLI change was needed. `Replayer` is also driven by the writer and by non-campaign streams (golden's every-kind trace), so a fold-time rule would refuse those.

## 2. The §7 stop (the record at `41a30f0a`), and the fixtures as reshaped

Measured by sweep C (`spike/2309/out/sweep-c.txt`, the workspace; `out/sweep-c-v3.txt`, the rerun below). Each `check_run` input main's tier-1 suite builds is judged by main and by each candidate rule. Main's suite passes 2,784 tests, 0 failed.

**A gap in the first run, closed by a rerun.** The CLI's tier-1 tests spawn the CLI with `env_clear()` (`cli/tests/campaign_tier1.rs` L374), so in the first run the spawned CLI's own `check_run` calls went unlogged. The rerun (v3 of `sh/sweep-instrument.diff`) covers chaos, evolve and the four CLI tier-1 binaries that spawn `campaign`, `replay`, `attribute` or `propose` (`campaign_tier1`, `mcp_tier1`, `doctor_tier1`, `attribute_seeded`). It logs through a marker file under `$HOME`, which those tests keep. It added 26 spawned-CLI pairs. **One more full-reading fixture is among them, also outside the fence** (below).

| Reading | Accepted by main, refused by the rule | Tests |
|---|---|---|
| **full** (as the patch is built: kinds 11, 12 and 21 value and status read exactly when kind 10 is) | 44 pairs | 21: 19 in `chaos/tests/campaign.rs`, 1 in `evolve/tests/attribute.rs`, 1 in `cli/tests/campaign_tier1.rs` |
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
- `aethelgard-cli/tests/campaign_tier1.rs:2081` `a_generator_report_names_each_generator_once_in_order`: it reseals a real run (`reseal`, L1403) with `good()`'s shape, kind 12 `timeout_txis` beside a `None` value, and hands it to the spawned CLI's `attribute` and `propose`

On the branch with the full patch, the chaos suite fails 47 tests: its 19 accept-path tests, and 28 that build fixtures main already refused, which the new rule now refuses first for its own reason.

**The fixtures outside the fence.** The full reading refuses two: the CLI's `a_generator_report_names_each_generator_once_in_order`, and evolve's `a_register_with_nothing_observed_is_no_evidence`. The status reading refuses evolve's alone. `a_register_with_nothing_observed_is_no_evidence` builds a read, healthy boot and sets one register (`GPIOB.AFRL`) to unread. It pins that attribution reads that register as no evidence. That is the adversary's 5.8 shape itself, and every two-way reading refuses it before attribution sees it. Building rule (3) means changing evolve's test, and evolve is outside this work order's fence.


**Why these are fixture shapes and not the judge's.** In `cli/src/main.rs` `boot_report`:
- kind 11's `observed: observed.map(|o| word(o.registers[i]))`;
- kinds 12 and 21's `observed` and `status`: both `observed.map(..)` of the one record word pair;
- kinds 13 and 22's lines: `observed.and_then(|o| o.witnesses[i].clone())`.

The judge reads all of them in the read that reads the boot, or none. The fixtures depart from that in two ways:
- **`good()`** (`tests/campaign.rs` L901) and the helpers built on it hold a read boot with a kind 12 whose value is `None` beside a status. Read at source:
  - `good()`'s own check: `timeout_txis`;
  - `i2c_step_check` (L2538): `observed: (width > 0).then_some(expected)`, so `None` for every configure step, beside `ok`;
  - `two_devices_on_one_controller_have_two_witnesses` (L2265): a second kind 12, `None` beside `timeout_txis`.

  The judge fills every value, configure steps included.
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

**Reshaped as ruled (D1).** Each is now a shape the judge writes, and every assertion in the edited tests keeps its original outcome:
- `good()` (chaos L901): kind 12's value is `Some(0)` beside `timeout_txis`.
- `i2c_step_check` (L2538): a configure step's value is `Some(0)`.
- The two-device test (L2265): its second kind 12's value is `Some(0)`.
- `unread()` in `a_boot_that_read_nothing_names_its_bound`: unread whole, its brown-out word and flash accesses, register, record and witness lines included. Its `BOUND`, `UNBOUND` and `BOOT_WORD` assertions are unchanged.
- Evolve `a_register_with_nothing_observed_is_no_evidence`: a register holds nothing observed only in a boot that read nothing. So the test's shape is the wrong-register run killed before its reads, naming `cpu_aborted`. It now pins `I1` with no evidence, and none of its registers is evidence. That is the test's claim, on a shape the writer can produce. No other line of the file changed (3307 is editing it).
- CLI `a_generator_report_names_each_generator_once_in_order`: its reseal keeps the record's value beside `timeout_txis`, as the judge does. Attribution is `S1`, and the generator report is unchanged.

## 3. Sweeps A, B and C

The tables and every file are in `spike/2309/results.md`.

**Before the build** (the STOP):
- **A:** the four committed archives were trace 14: `UnknownVersion`, exit 3, identical.
- **B:** 64 archives made once by main's CLI, plus the flooded run, all byte-identical under the full rule.
  - The census has 0 unread boots and 1 read but unbooted run without motor outputs.
  - The flooded run is read with kinds 13 and 22 unread, so witnesses bind one way only (P8).
- **C:** rule (2) refused no accepted input.

**After the build** (`out/sweep-a-built-*.txt`, `out/sweep-b-built.txt`):
- **A** now lists five committed archives, since this branch commits the flooded run. Main's CLI and the built branch's give identical output for all five: four `UnknownVersion`, and the flooded run accepted.
- **B:** all 64 archives exit 0 with byte-identical output under main's CLI and the built branch's.
- **No real archive fails the stricter replay.**

**Unmeasured, said plainly (erratum applied):**
- No real archive of a read but unbooted run with motor outputs exists.
  - The tier-1 fixtures do hold that shape (`motor.rs` `run(.., booted = false, ..)`), and it replays.
  - So D4's booted scope is exercised on fixtures only.
- Tier 2 was not swept. A grep of `campaign_tier2`, `twin_tier2` and `attribute_seeded` finds no `reseal(` and no edit that sets a value or status to `None`.
- The tier-1 tests that spawn `verify` reach no `check_run`: they run it with `PATH=""`, so it stops at the missing tools.

## 4. Tests and mutants

**New tests** (`aethelgard-chaos`):
- `bounds.rs` `the_writer_refuses_an_event_that_would_take_the_trace_past_256_mib`:
  - it fills with 1,024-byte fault events (not records, so `MAX_RECORDS` never decides) to exactly 268,435,456 B;
  - an event too large by about 1,000 B, and one too large by 1 B, are each refused as `TraceFull` with their index and byte count;
  - the exact fill is accepted, then the smallest event is refused;
  - the result commits and replays: 259,358 events in 5.6–5.8 s in a debug build on Furnace (P9 held).
- `motor.rs` `a_booted_run_holds_exactly_one_witness_per_solved_output`: one or both witnesses dropped from a booted run are refused by name; from an unbooted one, accepted.
- `campaign.rs` `a_boots_reads_are_all_read_or_all_unread`, on `good()` plus a read SPI device (kinds 23, 21 and 22):
  - accepted as is, and with its witnesses unread (the flooded shape);
  - refused by name for each of kind 11, kind 12's value, its status, kind 21's value and its status set unread beside the read boot;
  - the same run unread whole, naming `cpu_aborted`, is accepted;
  - refused for each of nine read things put back alone: those five, kind 13's lines and `absent`, and kind 22's lines and `unrouted`.
- D3, in each kind's own fixtures (`pin_devices.rs` covers 26, 34, 36 and 37; `serial.rs` covers 29; `motor.rs`'s DShot run covers 26 and 30). Unread whole and naming its bound, the run replays without those kinds, and each one put back alone is refused by name.

**Mutants** (`spike/2309/sh/mutants.py`, one per guard, against the chaos suite; `out/mutants.txt`). **20 of 20 killed, each by its own new test's named refusal:**
- the writer: removed; `>` to `>=` (killed at the exact fill); the check after the fold (killed by the second refusal's index);
- rule (2): removed; `booted &&` dropped (also kills `serial.rs` `a2_5_…`, so "booted" is load-bearing, D4);
- rule (3): the call removed; each of the seven clauses (11; 12/21 value; 12/21 status; 13 lines; 13 `absent`; 22 lines; 22 `unrouted`); the D3 guard; and each of D3's six kinds dropped from its list.

**Test weights:** one row, `tests/bounds.rs` `the_writer_refuses_…` at 16.2 s. It is a Furnace estimate (5.78 s × 2.8, as `1310`'s was), labelled so in the file's header; the next refit replaces it. `ci/test-shard.sh` passes.

## 5. Targeted adversarial pass (§5), self-run

`spike/2309/adversarial/` holds each probe's source and output.

1. **A writer that crosses the bound in one large record.**
   - The check is the sum of the bytes written and the encoded event, compared with the bound before the fold. An event's size cannot change that comparison: the event bound is 4,096 B, so no overflow is possible.
   - The test refuses an event about 1,000 B over the remainder, and one 1 B over it.
   - The CLI names it too: `ArchiveWriteFailed` carries `TraceFull`'s text.
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
   - Rule (3) as worded listed 10–13, 21 and 22 only. **Closed by D3 as built**: the probe's run is now refused by name (the D3 tests).
   - Re-run on the build, every kind the judge reads at the boot is now bound to kind 10: 10, 11, 12, 13, 19, 21, 22, 26, 29, 30, 34, 36 and 37. The rest are decided before the boot (kinds 5, 6, 15, 16, 20, 23, 24, 25, 27, 28, 31 to 33, 35) or by ngspice and the bridge (kinds 7, 8, 9, 17, 18).
   - Not closed, by design: a run forged whole into a consistent "killed before its reads" (everything unread, a bound named) cannot be told from a real one by its records. That claim says the run never booted, which the verdict then carries.

## 6. Rulings (architect, 2026-11-23T01:15Z), as built

- **D1:** the full reading; the fixtures reshaped, the fence widened to them alone (§2).
- **D2:** rules (1), (2) and (3) in one PR.
- **D3:** kinds 26, 29, 30, 34, 36 and 37 bound; 0 fixtures moved, as measured.
- **D4:** booted; `serial.rs` untouched.

## 7. Reduction pass

`Reduction pass (diff): −2 files (the two patches), −11 lines (FEEDBACK §3 at the STOP), −49 lines (two unread-boot loops in chaos tests/campaign.rs folded into tests/common's unread_boot) · 4 duplicates removed · 0 dead paths removed · 1 kept by reason · 1 finding below`.

Hand-run with the `reduction-pass` skill's four questions over the whole diff (`crates/`, one `ci/` row, `spike/2309/`, this file):
- **Duplicates removed:**
  - `spike/2309/patches/` (both diffs), now stated by `crates/` itself;
  - the status-reading delta file;
  - the STOP's restatement of `results.md` in §3;
  - the "judge's unread boot" loops in `a_boot_that_read_nothing_names_its_bound` and `a_boots_reads_…`. Both now call `tests/common`'s `unread_boot`, the one statement of that shape for four test files.

  All 20 mutants were re-run after the fold: still all killed (`out/mutants.txt`).
- **Kept by reason:** rule (3)'s two refusals stay two. They name different things (a read set split from its boot, against Renode-read evidence beside an unread boot), and a mutant of each is killed by its own test.
- Nothing in the diff is labelled `belt-and-suspenders:`.
- **Finding, outside the fence:** evolve's attribution still has a path for a read boot's register with nothing observed. `a_register_with_nothing_observed_is_no_evidence` used to pin it, and replay now refuses that input first, so the path is unreachable through `attribute`. It is for evolve's owner to remove or label.

## 8. For the backlog

- **Tier-1 tests call `check_run` on records that the field rules would refuse** (`unread()`'s kind 10). A shared fixture constructor that runs `Record::check` would stop a check_run-only shape from standing in for a trace.
- **Sweep A is vacuous until the committed archives are current.** The flooded run added here is the first trace-18 archive; the four 3305 archives stay version 14.

## 9. Gates, merge and CI

- **Main:** `fa9d7edd` is merged in, and main has not moved since (checked before the push).
- **Gates on the pushed tree, on Furnace:**
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace`: **2,790 passed, 0 failed**. That is main's 2,784 plus the 6 new tests.
- **CI:** CI-LINE
