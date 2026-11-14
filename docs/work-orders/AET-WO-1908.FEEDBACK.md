# AET-WO-1908 — FEEDBACK

**Status: IN PROGRESS (rewritten at the close).** Ten shards met §3 on three runs (B, C, D) and then missed it once (E, 13:06); the plan is now eleven shards and the three runs are being re-earned (runs G to I, below). A first version of this file recorded a stop on my own judgement; the build overrules it, and Method and Findings are kept as they were.

**Branch:** `wo/aet-1908-ci-rebalance` · **Base:** `a9d94ec` (`origin/main`) · **PR:** #109 · **Builder:** Claude (Sonnet 5.5). The measurement harness was reverted: `git diff origin/main -- ci/` is empty, and `RUSTC_BOOTSTRAP`, `report-time`, `test-threads`, `MEASURE` and `WALL` do not occur in `ci/shard.sh`.

## Method, and why

Three measurement runs on the PR, each a throwaway commit to `ci/shard.sh::run`, using libtest's `--report-time`. It needs nightly's `-Z unstable-options`; **`RUSTC_BOOTSTRAP=1` in the test binary's environment enables it on the pinned stable toolchain** (no image change). Side effect: cargo rebuilds `aethelgard-cli` (about 1 minute inside the run step); the pre-1908 runs have no such compile, so setup below is read from those.

1. **Run 43529475355: the real shard plan, with `--report-time`.** All 8 shards passed (15:04 slowest). **The per-test times are not usable as weights.** `board_tier2`'s heavy tests take `ONE_AT_A_TIME` (see `AET-WO-1907` FEEDBACK), so in a shared process each reads 400 to 640 s, which is queueing behind its siblings, not its own cost. The shard sums are 1.5 to 2 times the shard's wall time.
2. **Run 43531463122: each test alone (`--exact`, one invocation per test).** My awk only matched `test NAME ... ok <t>` on one line. A test whose child process writes to the inherited stdout interleaves it, so every tier-2 test (all of the heavy ones) was lost. Only 1,120 of 1,167 tests have a time. Discarded.
3. **Run 43534530400: each test alone, tolerant parser, plus wall time per invocation.** 1,167 of 1,167 tests timed; `WALL` agrees with `report-time` within about 1 s on the heaviest. Table below. **The run went red from my harness, not a test:** every shard reported 3 to 4 more workspace passes than assigned (147 against 144 on shard 0). The cause was not diagnosed, since the harness is gone; the measured times are not affected.

All times are CI seconds on a GitHub runner, one test at a time, nothing else in the process.

## Findings

1. **Total alone work is 5,730 s** (1,167 tests; the old weights know 143 of them, the other 1,026 get the 0.2 s default; measured, those 1,026 sum to about 330 s, so the default is fair in aggregate). `board_tier2` is 3,281 s (serial by its mutex), `attribute_seeded` 1,377 s, then `boot_tier1` 153 s and `twin_tier2` 129 s.
2. **Alone time is not what runs together.** `attribute_seeded`'s 12 tests sum to 1,377 s alone, yet the block took **475, 505, 575 and 721 s** in its own process on four CI runs (m1 shard 1; 43523317353; 43526602891; 43518552803). `p3_gate_clause_2` alone is 632 s; inside the block it was about 382 s. `colocate.tsv`'s 640 sits inside that range, so the colocation is right and the 640 is a middle value.
3. **The same work varies by 1.5 times between runners** (475 to 721 s). That, not the plan, is what limits headroom.
4. **Setup is 102 to 145 s** (job start to the first test binary; pre-1908 runs 43518552803, 43523317353, 43526602891), so about 2.3 minutes, not 2.5.
5. **The block's worst sample plus setup is 721 + 131 = 852 s, 14.2 minutes.** The median sample plus setup is about 11.5 minutes. A shard holding the block alone, with nothing else, passes 13 minutes on about one run in four on these four samples. Three consecutive runs under 13 minutes is a lottery that fails more often than not.
6. **Nothing in the planning fixes it.** More shards do not shrink the block (the shard count only moves the other tests off it). Splitting the block across shards makes it worse in total: its heaviest test alone is 632 s, and §7's 12-shard bound is not the limit. Making the block faster is a test change, which §2 forbids.
7. **§7's literal condition** ("a single test exceeds about 10 minutes on CI") is met only by the alone measurement of `p3_gate_clause_2` (632 s, one sample). I did not stop on that: inside its block it is about 382 s. I stopped on finding 5.

## What was built

- `ci/test-weights.tsv`: CI seconds directly, from run 43534530400, for the 240 tests of 0.5 s or more. `CI_SCALE` is gone (not 1.0: deleted, with the multiplication). The other 927 tests get `DEFAULT_WEIGHT` 0.05 s, the measured mean of those tests (0.042 s) rounded up; before it was 0.2 s, which would have weighed them at 185 s against 39 s measured.
- `ci/colocate.tsv`: `attribute_seeded` stays whole (the measurement says splitting it costs more than it saves: 1,377 s alone against 475 to 721 s together) and is weighed at 720 s, the worst in-process sample (was 640).
- `ci/SHARDS`: **11** (10 until run E). §1.3's rule (smallest count whose planned maximum is at most 660 s) cannot be met at any count, because the block alone plans at 720 s; I used the smallest count at which every other shard plans under the 620 s that 13 minutes leaves after 2.3 minutes of setup. That is 8 (627 s) on paper and 9 (548 s) with room; **9 failed in run A** (13:27: the shards with `board_tier2`'s heavy tests ran 12 to 20% over plan), **10 plans 484 s and passed three runs.** The planner's numbers for the heavy shards run low by that 12 to 20%; the refit rule absorbs it by shard count.
- A refit rule: the comment block in `ci/shard.sh` above `DEFAULT_WEIGHT` (the five steps, including the interleaving trap that cost me run 2). No tool added.
- Not changed: `ci.yml`, the guard, the canary, `TIER2_MIN` (83), any test, crate or image.

## Result

- **Ten shards (runs B to F):** B, C and D had every shard under 13 minutes (slowest 11:44, 11:11, 12:05); E, the same plan, had shard 5 at 13:06; F was 11:48. So ten shards meet §3 on most runs, not on every run: four of five.
- **Eleven shards (runs G onward): see Acceptance runs.** The guard and canary passed on every run at either count (`tools` green), as did `gates` (fmt, clippy, workspace tests). I also ran the three gates locally at the head commit: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` rc 0 (default features; tier 2 runs only in CI), and `ci/test-shard.sh` passes.
- The first stop (this file's earlier version) was my judgement that the block's worst sample, 721 s plus 131 s of setup, would fail 13 minutes. The kickoff says to stop only on a §7 condition and that was not one, so I built it. **The block did not fail 13 minutes in any of the four plan runs** (shard 1: 10:18, 10:19, 11:11, 12:00); the worst-case reading in finding 5 was a risk, not a result. It remains a risk: one more run in four could still exceed 13 minutes.

## Adversarial pass (§5, targeted, a fresh subagent)

Run on the final `ci/` against a 1,167-entry list built from the measurements; I re-checked one finding myself (a deleted weight line: the test is still assigned, to shard 6, and the guard passes).

1. **Two heavy tests in one shard:** not found. Each `board_tier2` heavy test (200 to 516 s) sits in its own shard; ties (all weights equal), a duplicate weight line, a weight for another binary or an absent test, 1e-7 and 1e20 weights: guard ok; the plan is byte-identical across two runs and a shuffled entry order. `1e3`, a trailing space, CRLF, a negative, empty, `inf`, `nan`, `.5` and `5.` are refused with exit 1 and no shard written.
2. **A test disappears:** not found. A removed weight line gives the default; a wrong field count is refused; a `#` name is a comment (default weight); a weight for another binary is ignored. `SHARDS` 0, -1, x and empty are refused; 2000 passes `assign` and the guard refuses it ("shard 0 lists nothing").
3. **Defects: none in the changed files.** Backlog, by the scope test (a safeguard that already works; none is a failing invariant), none done:
   - A removed or stale weight line is silent: the test is planned at 0.05 s and the shard may run long (deleting the 516 s test's line leaves the plan maximum at 720 s). A warning for "a weighted test not in the list" or "a heavy test on the default" would catch it before a 13-minute shard does.
   - A duplicate weight or colocate line takes the last value silently (a second `attribute_seeded 1` weighs the block at 1 s).
   - A near-empty weights file passes the guard with a wrecked plan (the guard checks coverage, not balance).

## Reduction pass

`CI_SCALE`, its comment and the `scale` awk variable are deleted; the 143-line weights file is replaced by one of 240 (the old one's 143 tests are all in it). Nothing else in `ci/` was found to remove: the measurement harness is not in the diff, and the diff against `main` is `ci/SHARDS`, `ci/colocate.tsv`, `ci/shard.sh`, `ci/test-weights.tsv` and this file.

## For the architect

- **The measurement run went red from my harness**, not from a test (run 3 of the measurement runs: 3 to 4 more workspace passes than assigned per shard; not diagnosed, harness reverted). It is not in the diff.
- **§1.3 and §3 conflict by about 20 seconds** (11 minutes of tests plus 2.3 of setup is 13.3), and the block alone is above the 11 minutes (720 s). The shard count is a measured choice, not the rule's output. Worth a line in the roadmap if the rule is kept.
- **Twelve jobs per PR run now** (11 shards and `gates`), against eight shards before; ten shards cost eleven.
- **No test change was needed.** If the architect wants the block off the critical path for good, that is (b) in an earlier version of this file: a work order for `attribute_seeded`.

## Measurement table: tests of 5 CI seconds or more

`old` is the old weight in CI seconds (Furnace seconds times `CI_SCALE` 1.8); `default` is the 0.2 s a test without a weight line gets. The other 1,092 tests sum to 331 s, each under 5 s.

| CI s alone | old | binary | test |
|---|---|---|---|
| 632 | 554 | `tests/attribute_seeded.rs` | `tier2::p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says` |
| 516 | 446 | `tests/board_tier2.rs` | `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` |
| 453 | 443 | `tests/board_tier2.rs` | `verify_against_fab_names_every_changed_and_extra_file` |
| 416 | 443 | `tests/board_tier2.rs` | `verify_against_fab_finds_the_package_it_wrote_identical` |
| 382 | 489 | `tests/board_tier2.rs` | `verify_binds_the_reference_and_two_runs_print_the_same_bytes` |
| 251 | 446 | `tests/board_tier2.rs` | `every_admitted_identifier_character_survives_every_output` |
| 218 | 237 | `tests/board_tier2.rs` | `verify_fails_the_47k_pull_up_with_the_campaigns_findings` |
| 200 | 221 | `tests/board_tier2.rs` | `verify_fab_out_writes_nothing_when_a_gate_fails` |
| 183 | 203 | `tests/board_tier2.rs` | `the_generated_board_proves` |
| 180 | 199 | `tests/board_tier2.rs` | `a_missing_clock_enable_fails` |
| 179 | 198 | `tests/board_tier2.rs` | `a_write_past_the_block_fails` |
| 155 | 133 | `tests/attribute_seeded.rs` | `tier2::attribute_twice_is_identical_and_a_one_byte_change_is_refused_as_replay_refuses_it` |
| 151 | 129 | `tests/attribute_seeded.rs` | `tier2::p3_gate_clause_1_attribute_names_every_case_with_evidence_its_rule_allows` |
| 147 | 125 | `tests/attribute_seeded.rs` | `tier2::every_case_fails_the_way_its_manifest_says` |
| 146 | 126 | `tests/attribute_seeded.rs` | `tier2::the_trap_the_wrong_address_shares_i2c_timeout_with_47k_and_is_told_apart` |
| 146 | 125 | `tests/attribute_seeded.rs` | `tier2::oq_03_the_reference_runs_bytes_with_kind_15` |
| 127 | 137 | `tests/board_tier2.rs` | `a_wrong_alternate_function_fails` |
| 102 | 84 | `tests/board_tier2.rs` | `a_vacuous_harness_fails` |
| 78 | 91 | `tests/adversarial_1301_tools.rs` | `attack_wide_references_leave_the_emitted_board_clean` |
| 61 | 86 | `tests/adversarial_1301.rs` | `attack_a_wide_reference_stays_clear_of_every_other_part` |
| 51 | 67 | `tests/board_tier2.rs` | `every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict` |
| 44 | 42 | `tests/generate_tier1.rs` | `verify_against_fails_on_one_changed_byte_in_any_file_naming_it` |
| 42 | 2 | `tests/board_ui.rs` | `misuse_does_not_compile_and_the_driver_does` |
| 32 | 58 | `tests/boot_tier1.rs` | `a_boot_past_the_wall_bound_fails_and_names_it` |
| 32 | 28 | `tests/twin_tier2.rs` | `an_aborted_machine_returns_inside_the_bound_as_cpu_aborted` |
| 26 | 23 | `tests/twin_tier2.rs` | `p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes` |
| 23 | 23 | `tests/campaign_tier2.rs` | `the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay` |
| 17 | 23 | `tests/campaign_tier2.rs` | `p2_gate_clause_3_the_same_campaign_twice_gives_identical_hashes_per_scenario` |
| 16 | 25 | `tests/adversarial_1301_tools.rs` | `parity_fails_every_other_mismatch_with_kicads_finding` |
| 16 | 14 | `tests/board_tier2.rs` | `prove_with_spec_tells_a_hand_written_harness_from_the_generated_one` |
| 14 | 13 | `tests/campaign_tier2.rs` | `a_flooding_firmware_reaches_a_verdict_and_its_message_is_the_same_twice` |
| 14 | 12 | `tests/campaign_tier2.rs` | `p2_gate_clause_2_the_47k_trace_shows_the_rise_the_fault_and_the_timeout_as_three_events` |
| 13 | 12 | `tests/campaign_tier2.rs` | `p2_gate_clause_3_replay_twice_is_identical_and_equals_the_committed_hash` |
| 12 | 13 | `tests/campaign_tier2.rs` | `renodes_reset_recipe_is_pinned_step_by_step` |
| 12 | 11 | `tests/twin_tier2.rs` | `the_reference_firmware_boots_and_two_boots_agree` |
| 11 | 7 | `tests/boot_tier1.rs` | `an_aborted_machine_killed_at_a_bound_is_cpu_aborted` |
| 11 | 21 | `tests/with_tools.rs` | `prove_of_an_unresolvable_dependency_names_the_package` |
| 11 | 13 | `tests/generate_tier1.rs` | `two_generations_are_identical_and_a_used_directory_is_refused_untouched` |
| 10 | 9 | `tests/boot_tier1.rs` | `a_record_the_firmware_never_wrote_or_wrote_oddly_fails` |
| 9 | default | `tests/extract_tier1.rs` | `a_longer_route_raises_the_trace_term_by_the_per_length_bound` |
| 9 | default | `tests/campaign_tier1.rs` | `a_firmware_changed_while_the_campaign_runs_is_refused` |
| 9 | 12 | `tests/campaign_tier2.rs` | `p2_gate_clause_2_the_campaign_passes_2k2_and_fails_47k_as_each_expects` |
| 9 | 11 | `tests/twin_tier2.rs` | `a_firmware_that_skips_the_read_or_its_record_cannot_pass` |
| 9 | 11 | `tests/campaign_tier2.rs` | `p2_gate_clause_3_a_one_byte_change_is_refused_even_with_the_manifest_resealed` |
| 9 | 10 | `tests/with_tools.rs` | `parity_fails_a_board_that_does_not_match_its_schematic` |
| 8 | default | `tests/campaign_tier1.rs` | `a_droop_boot_killed_at_a_bound_is_a_verdict_and_replays` |
| 8 | 9 | `tests/generate_tier1.rs` | `verify_against_refuses_another_specifications_tree_and_a_hand_written_harness` |
| 8 | 8 | `tests/fab_tier1.rs` | `a_fab_out_another_uid_could_write_is_refused` |
| 8 | 7 | `tests/boot_tier1.rs` | `markers_must_appear_once_each_as_whole_lines` |
| 8 | 11 | `tests/adversarial_1301_tools.rs` | `kicad_parity_does_not_see_metadata_only_differences` |
| 7 | default | `tests/adversarial_1300.rs` | `withstood_emission_ignores_env_locale_time_cwd_and_spec_layout` |
| 7 | 9 | `tests/boot_tier1.rs` | `each_field_of_the_controller_state_fails_alone` |
| 7 | 8 | `tests/twin_tier2.rs` | `a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record` |
| 7 | 7 | `tests/boot_tier1.rs` | `a_boot_past_the_memory_cap_fails_and_names_it` |
| 7 | 10 | `tests/with_tools.rs` | `p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding` |
| 6 | default | `tests/routing_tier1.rs` | `the_routed_board_is_byte_identical_and_a_function_of_its_pads` |
| 6 | default | `tests/lifecycle_tier1.rs` | `a_guardian_removes_its_directory_though_a_writer_races_it` |
| 6 | 8 | `tests/fab_tier1.rs` | `an_unusable_fab_out_is_refused_before_any_gate_and_left_untouched` |
| 6 | 7 | `tests/adversarial_1301_tools.rs` | `a_schematic_kicad_cannot_load_is_no_verdict` |
| 6 | 6 | `tests/twin_tier2.rs` | `a_read_no_model_answers_fails_as_a_timeout_at_txis` |
| 6 | 6 | `tests/twin_tier2.rs` | `a_forged_record_without_a_transaction_fails_on_the_controller_state` |
| 6 | 5 | `tests/twin_tier2.rs` | `a_value_one_bit_off_fails_as_an_identity_mismatch` |
| 6 | 5 | `tests/twin_tier2.rs` | `a_boot_too_short_to_reach_the_fixed_point_fails` |
| 6 | 5 | `tests/boot_tier1.rs` | `the_virtual_time_is_an_integer_unit_up_to_100_ms` |
| 5 | default | `tests/strict_tier1.rs` | `every_corpus_file_is_refused_or_reaches_its_gate_as_stated` |
| 5 | default | `tests/mcp_tier1.rs` | `servers_sigkilled_while_calls_start_leave_nothing` |
| 5 | default | `tests/mcp_tier1.rs` | `a_client_that_disconnects_mid_call_leaves_nothing_after_the_grace` |
| 5 | default | `tests/campaign_tier1.rs` | `propose_never_writes_into_the_run_it_reads` |
| 5 | default | `tests/campaign_tier1.rs` | `a_failure_for_a_superset_or_a_wrong_finding_kind_is_a_campaign_failure` |
| 5 | default | `tests/board.rs` | `two_generations_are_byte_identical` |
| 5 | 8 | `tests/boot_tier1.rs` | `a_wait_that_expired_fails_as_a_timeout_naming_the_wait` |
| 5 | 8 | `tests/boot_tier1.rs` | `a_value_that_is_not_the_part_tables_fails_as_an_identity_mismatch` |
| 5 | 7 | `tests/boot_tier1.rs` | `a_bridge_that_fails_or_answers_wrong_stops_the_call` |
| 5 | 6 | `tests/twin_tier2.rs` | `a_firmware_that_writes_the_witness_registers_by_hand_passes_today_a_named_residual` |
| 5 | 5 | `tests/generate_tier1.rs` | `generate_reproduces_the_committed_board_and_emits_files_byte_for_byte` |

## Acceptance runs

Job start to job end, minutes:seconds, from the run's job list. Run A is at 9 shards and does not count (a shard passed 13 minutes); the count restarts with every change to the plan.

| run | commit | shards | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | slowest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A (43538439489) | c1a4c12 | 9 | 9:43 | 10:18 | 9:26 | 11:55 | 9:47 | **13:27** | 8:46 | 9:53 | 12:12 | | **13:27** |

Run A (nine shards; does not count): the guard and canary passed (`tools` green). Shard 1, which holds the `attribute_seeded` block (planned 720 s), took only 10:18; the shards with `board_tier2` heavy tests (3, 5, 8) took 11:55 to 13:27, planned 548 s of test time, actual about 12 to 20% more. Setup is 103 to 138 s. Next: 10 shards (planned 484 s per shard).

Run B (10 shards, `ci/SHARDS` 10, commit 4acd4f0) counts as run 1 of the three at this plan. The guard and canary passed.

| run | commit | shards | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | slowest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| B (43539995009) | 4acd4f0 | 10 | 8:17 | 10:19 | 9:49 | 11:44 | 8:24 | 11:25 | 8:00 | 8:15 | 8:43 | 8:40 | **11:44** |
| C (43541279022) | 7860e22 | 10 | 10:19 | 11:11 | 8:26 | 10:59 | 9:17 | 11:08 | 10:29 | 10:09 | 8:29 | 11:00 | **11:11** |
| D (43542481893) | 0f5700c | 10 | 8:36 | 12:00 | 11:13 | 9:01 | **12:05** | 9:49 | 7:36 | 7:33 | 8:44 | 11:54 | **12:05** |
| E (43543703305) | 211082d | 10 | 8:46 | 11:26 | 10:40 | 11:00 | 9:54 | **13:06** | 9:27 | 7:53 | 9:32 | 9:05 | **13:06** |
| F (43545027152) | c6aa0ea | 10 | 9:52 | 10:10 | 10:59 | 11:05 | **11:48** | 10:57 | 8:25 | 8:07 | 10:12 | 11:36 | **11:48** |

**Run E, on the plan of B to D, put shard 5 at 13:06, six seconds over** (guard and canary passed); shard 5 holds `board_tier2` heavy tests, which run serially and carry most of the runner-to-runner variance. F was 11:48. Four of five runs at ten shards met §3, so the acceptance was reached and then lost; I took the lever named in the first stop: an eleventh shard (planned about 427 s per shard, against 484 s at ten). The count of three consecutive runs restarts at G.

Eleven shards (`ci/SHARDS` 11). Each row's commit is the one that started the run and the next commit recorded it.

| run | commit | shards | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | slowest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
