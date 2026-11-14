# AET-WO-1908 — FEEDBACK

**Status: IN PROGRESS (rewritten at the close).** Built after a first stop (see Method and Findings, kept below); the acceptance runs are logged under Acceptance runs.

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

## The shard count, had the build gone on

At the 11-minute ceiling of §1.3, test time per shard is 660 s. 5,730 s of alone work (or about 5,070 s with the block at 721 s instead of 1,377) needs 9 shards at perfect packing, 10 in practice; that is under the 12 of §7. Setup of 2.3 minutes plus 11 minutes is 13.3: **§1.3's 11 minutes and §3's 13 minutes conflict by about 20 seconds**, so 10 or 11 shards, not 9, would be the honest set. Noted for the ruling.

## Passes not run

The targeted adversarial pass (§5) and the reduction pass act on a built plan; none was built, so neither ran. The reduction pass has nothing to remove: the diff against `main` is this file only.

## Rulings asked for

- **(a)** Accept the block as the floor and change §3 to something the measurement can meet (for example, under 15 minutes on three runs, which `AET-WO-1907` run 3 already did at 13:41), with the refit still worth doing for the spread.
- **(b)** Authorise a test change to `attribute_seeded` (a work order of its own), after which this WO's weights and count follow.
- **(c)** Rule that the variance is accepted and the three runs are attempted anyway; I would then build the 10-shard refit from the table below and report what the three runs say.

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

Run A: the guard and canary passed (`tools` green). Shard 1, which holds the `attribute_seeded` block (planned 720 s), took only 10:18; the shards with `board_tier2` heavy tests (3, 5, 8) took 11:55 to 13:27, planned 548 s of test time, actual about 12 to 20% more. Setup is 103 to 138 s. Next: 10 shards (planned 484 s per shard).

Run B (10 shards, `ci/SHARDS` 10, commit 4acd4f0) counts as run 1 of the three at this plan. The guard and canary passed.

| run | commit | shards | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | slowest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| B (43539995009) | 4acd4f0 | 10 | 8:17 | 10:19 | 9:49 | 11:44 | 8:24 | 11:25 | 8:00 | 8:15 | 8:43 | 8:40 | **11:44** |
| C (43541279022) | 7860e22 | 10 | 10:19 | 11:11 | 8:26 | 10:59 | 9:17 | 11:08 | 10:29 | 10:09 | 8:29 | 11:00 | **11:11** |
