# AET-WO-1912 FEEDBACK — CI's third refit, measured on `main`

## Status: READY — built. Every shard under 20 minutes on the PR's run 44762196410 (worst 17.1 min). The spread was 7.2 minutes, over §2's 6: decision needed (D1).

Builder: Claude (Opus 5.5), a fresh `1912` session, 2026-11-20, from ~13:45 UTC. Worktree `~/aethelgard-1912`, branch `wo/aet-1912-refit` from `origin/main` `472d2d84`, with `main` `a37b787f` (`4240a`, `1211`, docs) and `8fdc9422` (`4406`, `4241`) merged in. No merge touched a row of this order; `4241` added one `twin_tier2` row, which is its own.

Labels: **measured** (from CI's own logs or runs), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1912/`; the scripts are in `spike/1912/`.

## 1. The table before (§1 step 1)

**The five runs:** the five most recent green `ci.yml` runs on `main` at 14:20Z: 44747545277 (`472d2d84`, this branch's base), 44747464266 (`6c6b066b`), 44735701946 (`dc618bd8`), 44735207864 (`1ba4d031`) and 44732069104 (`a75fffcc`). The first two are the only ones planned with `1318`'s rows. Each was read with `spike/1912/fetch.sh` (jobs API, `gh run view --job ID --log`) and tabulated by `spike/1912/table.py` [measured].

**How to read it:**
- Every measurement is set against the rows at `472d2d84`, the head being re-weighed; the plan column is the plan CI printed at each run's own head.
- **job min:** GitHub's `started_at` to `completed_at`; **run step:** the `shard.sh run` step; **overhead:** the job less the run step (checkout, image pull, the shared build's download and unpack).
- **Test measurements**, one of three:
  - **`solo`**: the test was the only test of its binary in that shard, so the binary's `finished in` is its alone time.
  - **`lock`**: in a lock-serialised binary, its `ok` less the previous `ok`. Both `board_tier2` (`prove()`, `board_tier2.rs:43`) and `twin_tier2` (`RENODE_TURN`, `twin_tier2.rs:69`) take the lock per call, not per test [read]. So a `lock` time can hold another test's calls, and it is a candidate, not a weight.
  - **`ub`**: otherwise its `ok` less its binary's `Running`, an upper bound and a candidate.
- **Lagging:** a row more than 25 % under its worst measured sample. A `solo` sample settles it; a `lock` or `ub` candidate goes to the alone run (§2).

### run 44747545277 (head 472d2d84, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |
|---|---|---|---|---|---|---|---|
| 0 | 136329612929 | 16.2 | 822 | 151 | 706 | 669 | twin_tier2.rs 556/415 |
| 1 | 136329612978 | 13.8 | 665 | 164 | 706 | 706 |  |
| 2 | 136329613028 | 14.1 | 699 | 146 | 706 | 706 |  |
| 3 | 136329613012 | 14.9 | 798 | 99 | 706 | 706 | against_fab_tier2.rs 538/447 |
| 4 | 136329612963 | 14.3 | 707 | 151 | 706 | 706 | against_fab_tier2.rs 532/421 |
| 5 | 136329612946 | 14.4 | 635 | 230 | 706 | 706 |  |
| 6 | 136329613261 | 14.3 | 651 | 208 | 706 | 706 |  |
| 7 | 136329613191 | 12.3 | 641 | 100 | 706 | 706 |  |
| 8 | 136329612980 | 10.7 | 514 | 127 | 706 | 706 |  |
| 9 | 136329613027 | 8.4 | 371 | 133 | 706 | 706 |  |
| 10 | 136329613040 | 13.3 | 666 | 132 | 706 | 706 | board_tier2.rs 266/200 |
| 11 | 136329612996 | 14.0 | 741 | 99 | 706 | 706 |  |

### run 44747464266 (head 6c6b066b, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |
|---|---|---|---|---|---|---|---|
| 0 | 136329323249 | 14.7 | 715 | 166 | 706 | 669 | twin_tier2.rs 504/419 |
| 1 | 136329323316 | 15.2 | 775 | 140 | 706 | 706 | four_layer_fab_tier2.rs 703/625 |
| 2 | 136329323247 | 13.4 | 648 | 159 | 706 | 706 |  |
| 3 | 136329323360 | 12.8 | 552 | 219 | 706 | 706 |  |
| 4 | 136329323270 | 15.0 | 739 | 161 | 706 | 706 | against_fab_tier2.rs 539/421 |
| 5 | 136329323300 | 14.9 | 686 | 210 | 706 | 706 | attribute_seeded.rs 461/380 |
| 6 | 136329323266 | 13.9 | 702 | 133 | 706 | 706 |  |
| 7 | 136329323361 | 11.1 | 537 | 126 | 706 | 706 |  |
| 8 | 136329323352 | 19.9 | 1049 | 142 | 706 | 706 | supply_tier1.rs 720/302 |
| 9 | 136329323197 | 9.1 | 400 | 147 | 706 | 706 |  |
| 10 | 136329323431 | 13.0 | 646 | 134 | 706 | 706 |  |
| 11 | 136329323330 | 16.3 | 775 | 204 | 706 | 706 |  |

### run 44735701946 (head dc618bd8, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |
|---|---|---|---|---|---|---|---|
| 0 | 136289665156 | 11.9 | 577 | 137 | 684 | 647 |  |
| 1 | 136289665048 | 15.7 | 849 | 90 | 684 | 840 |  |
| 2 | 136289665069 | 15.6 | 750 | 187 | 684 | 684 | verify_2k2_tier2.rs 557/458 |
| 3 | 136289665076 | 13.4 | 594 | 211 | 684 | 684 |  |
| 4 | 136289665100 | 13.5 | 573 | 237 | 684 | 684 |  |
| 5 | 136289665266 | 11.6 | 532 | 166 | 684 | 684 |  |
| 6 | 136289665067 | 14.9 | 789 | 105 | 684 | 684 | verify_2k2_tier2.rs 516/373 |
| 7 | 136289665198 | 13.8 | 734 | 96 | 684 | 684 |  |
| 8 | 136289665103 | 23.0 | 1276 | 106 | 684 | 783 | supply_tier1.rs 716/302 |
| 9 | 136289665172 | 10.8 | 434 | 216 | 684 | 684 |  |
| 10 | 136289665037 | 19.4 | 1047 | 118 | 684 | 684 | twin_tier2.rs 535/218 |
| 11 | 136289665051 | 11.1 | 443 | 222 | 684 | 684 |  |

### run 44735207864 (head 1ba4d031, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |
|---|---|---|---|---|---|---|---|
| 0 | 136286966405 | 13.1 | 499 | 285 | 684 | 647 |  |
| 1 | 136286966742 | 16.9 | 762 | 251 | 684 | 840 |  |
| 2 | 136286966414 | 14.8 | 731 | 158 | 684 | 684 | verify_2k2_tier2.rs 524/458 |
| 3 | 136286966295 | 16.5 | 685 | 304 | 684 | 684 |  |
| 4 | 136286966345 | 13.7 | 599 | 223 | 684 | 684 |  |
| 5 | 136286966303 | 15.4 | 768 | 158 | 684 | 684 | attribute_seeded.rs 525/380 |
| 6 | 136286966338 | 12.3 | 583 | 156 | 684 | 684 |  |
| 7 | 136286966265 | 14.6 | 731 | 147 | 684 | 684 |  |
| 8 | 136286966905 | 19.3 | 944 | 216 | 684 | 783 | supply_tier1.rs 444/302 |
| 9 | 136286966446 | 12.8 | 595 | 175 | 684 | 684 |  |
| 10 | 136286966363 | 19.4 | 906 | 255 | 684 | 684 | twin_tier2.rs 483/218 |
| 11 | 136286966389 | 12.1 | 496 | 229 | 684 | 684 | board_tier2.rs 291/220 |

### run 44732069104 (head a75fffcc, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |
|---|---|---|---|---|---|---|---|
| 0 | 136276909829 | 10.9 | 475 | 178 | 684 | 647 |  |
| 1 | 136276909823 | 16.7 | 904 | 98 | 684 | 840 | four_layer_fab_tier2.rs 708/625 |
| 2 | 136276909917 | 14.4 | 582 | 282 | 684 | 684 |  |
| 3 | 136276909799 | 20.8 | 837 | 410 | 684 | 684 | against_fab_tier2.rs 581/447 |
| 4 | 136276909767 | 12.8 | 642 | 127 | 684 | 684 |  |
| 5 | 136276909777 | 12.9 | 646 | 131 | 684 | 684 | attribute_seeded.rs 444/380 |
| 6 | 136276909736 | 17.1 | 719 | 308 | 684 | 684 | verify_2k2_tier2.rs 454/373 |
| 7 | 136276909873 | 17.6 | 723 | 334 | 684 | 684 |  |
| 8 | 136276909772 | 16.5 | 788 | 203 | 684 | 783 | supply_tier1.rs 379/302 |
| 9 | 136276909785 | 17.1 | 470 | 555 | 684 | 684 |  |
| 10 | 136276909798 | 22.1 | 841 | 487 | 684 | 684 | twin_tier2.rs 448/218 |
| 11 | 136276909818 | 11.2 | 569 | 101 | 684 | 684 | board_tier2.rs 320/220 |

### colocated blocks: every `finished in` against the row

| block | row | samples (s, run/shard) | worst | row / worst |
|---|---|---|---|---|
| tests/attribute_seeded.rs | 380 | 408.70 (44747545277/5); 461.11 (44747464266/5); 372.07 (44735701946/5); 525.12 (44735207864/5); 444.20 (44732069104/5) | 525.12 (44735207864 job 136286966303) | 0.72 **lags** |
| tests/campaign_tier2.rs | 54 | 58.32 (44747545277/2); 55.59 (44747464266/2); 75.55 (44735701946/7); 82.74 (44735207864/7); 79.83 (44732069104/7) | 82.74 (44735207864 job 136286966265) | 0.65 **lags** |
| tests/four_layer_fab_tier2.rs | 625 | 605.13 (44747545277/1); 702.81 (44747464266/1); 658.53 (44735701946/1); 599.90 (44735207864/1); 708.27 (44732069104/1) | 708.27 (44732069104 job 136276909823) | 0.88 |
| tests/outline_tier1.rs | 251 | 207.70 (44747545277/10); 199.79 (44747464266/10); 277.46 (44735701946/8); 239.00 (44735207864/8); 198.53 (44732069104/8) | 277.46 (44735701946 job 136289665103) | 0.90 |
| tests/power_board_tier1.rs | 331 | 311.47 (44747545277/7); 262.20 (44747464266/7); 319.20 (44735701946/7); 311.72 (44735207864/7); 310.34 (44732069104/7) | 319.20 (44735701946 job 136289665198) | 1.04 |
| tests/supply_tier1.rs | 302 | 291.74 (44747545277/8); 720.39 (44747464266/8); 716.11 (44735701946/8); 443.66 (44735207864/8); 379.35 (44732069104/8) | 720.39 (44747464266 job 136329323352) | 0.42 **lags** |
| tests/verify_47k_tier2.rs | 220 | 277.29 (44747545277/11); 276.76 (44747464266/11); 264.91 (44735701946/10); 228.16 (44735207864/10); 210.85 (44732069104/10) | 277.29 (44747545277 job 136329612996) | 0.79 |
| tests/via_ring_tier1.rs | 61 | 62.81 (44747545277/3); 46.42 (44747464266/3); 50.14 (44735701946/4); 47.87 (44735207864/4); 51.42 (44732069104/4) | 62.81 (44747545277 job 136329613012) | 0.97 |

### tests: any sample of 30 s or more, or a row more than 25 % under its worst sample (worst of 1 s or more)

`solo` is an alone time; `lock` and `ub` are candidates only (see the header).

| binary | test | row | worst solo | worst lock/ub | samples | verdict |
|---|---|---|---|---|---|---|
| tests/against_fab_tier2.rs | verify_against_fab_names_every_changed_and_extra_file | 446.7 | 581.45 | - | 5 | holds |
| tests/verify_2k2_tier2.rs | verify_fab_out_writes_the_package_its_verdict_names_at_any_path | 457.8 | 556.51 | - | 5 | holds |
| tests/against_fab_tier2.rs | verify_against_fab_finds_the_package_it_wrote_identical | 420.7 | 538.93 | - | 5 | holds |
| tests/verify_2k2_tier2.rs | verify_binds_the_reference_and_two_runs_print_the_same_bytes | 372.6 | 515.84 | - | 5 | lags (solo) |
| tests/twin_tier2.rs | kani_proves_every_part_of_both_motor_fixtures | 169.8 | - | 447.74 | 5 | candidate: alone run |
| tests/board_tier2.rs | every_admitted_identifier_character_survives_every_output | 181.2 | - | 271.16 | 5 | candidate: alone run |
| tests/twin_tier2.rs | battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats | 288.767 | - | 251.70 | 5 | holds |
| tests/board_tier2.rs | verify_names_what_judged_nothing_on_the_motor_driver | 204.8 | - | 194.55 | 5 | holds |
| tests/escape_tier1.rs | renamings_0_to_9_route_d3_to_its_own_copper | 158.4 | 158.62 | 169.84 | 5 | holds |
| tests/escape_tier1.rs | renamings_10_to_19_route_d3_to_its_own_copper | 169.1 | 159.81 | - | 5 | holds |
| tests/twin_tier2.rs | on_a_bus_with_two_devices_each_read_is_witnessed | 216.4 | - | 155.54 | 5 | holds |
| tests/board_tier2.rs | the_generated_board_proves | 109.2 | 135.32 | 155.17 | 5 | candidate (solo holds) |
| tests/twin_tier2.rs | the_reference_firmware_boots_and_two_boots_agree | 5.4 | - | 154.77 | 5 | candidate: alone run |
| tests/board_tier2.rs | each_size_sample_passes_every_gate_or_is_refused_by_name | 156.94 | 146.51 | 141.98 | 5 | holds |
| tests/board_tier2.rs | the_power_fixture_passes_erc_drc_its_loop_rules_and_extract | 99.8 | - | 96.34 | 5 | holds |
| tests/board_tier2.rs | the_own_parts_fixture_s_transfers_prove_within_its_scripts | 77.5 | - | 84.82 | 5 | holds |
| tests/board_ui.rs | misuse_does_not_compile_and_the_driver_does | 73.6 | 81.53 | - | 5 | holds |
| tests/board_tier2.rs | every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict | 15.0 | - | 77.49 | 5 | candidate: alone run |
| tests/hot_plug_tier1.rs | the_declared_bulk_capacitor_is_c1_positive_on_vbat_on_both_power_boards | 61.2 | 61.64 | 53.28 | 5 | holds |
| unittests src/lib.rs | supply::tests::a_relieved_pad_joins_no_plane | 39.5 | - | 58.94 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv3_2_sbus_s_cr2_read_back_and_xored_fails | 6.5 | - | 50.59 | 5 | candidate: alone run |
| tests/board_tier2.rs | the_sentinel_03_fixture_passes_every_gate_inside_its_bound | 40.7 | 28.63 | 49.87 | 5 | holds |
| tests/solve_cli.rs | a_power_tree_solves_into_the_topology_and_emits_its_board | 37.6 | - | 48.09 | 5 | holds |
| tests/twin_tier2.rs | the_dshot600_fixture_s_outputs_are_witnessed_in_their_window_of_a_100_ms_boot | 1.3 | - | 47.73 | 5 | candidate: alone run |
| tests/board_tier2.rs | the_imu_fixture_s_transfers_prove_within_its_scripts | 35.0 | - | 46.70 | 5 | candidate: alone run |
| unittests src/lib.rs | supply::tests::a_through_hole_pad_carries_the_current_as_a_barrel | 26.6 | - | 46.04 | 5 | candidate: alone run |
| tests/board_tier2.rs | a_write_past_the_block_fails | 24.4 | - | 41.83 | 5 | candidate: alone run |
| tests/board_tier2.rs | the_full_power_fixture_routes_and_passes_every_gate_it_reaches | 27.9 | - | 39.48 | 5 | candidate: alone run |
| tests/escape_tier1.rs | twenty_renamings_route_d1_to_its_own_copper | 37.5 | 35.80 | - | 5 | holds |
| unittests src/lib.rs | supply::tests::a_path_through_two_vias_is_bounded_above_its_floor | 23.5 | - | 32.45 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_boot_past_the_wall_bound_fails_and_names_it | 32.4 | - | 32.30 | 5 | holds |
| tests/adversarial_1301_tools.rs | attack_wide_references_leave_the_emitted_board_clean | 33.5 | 21.17 | 32.12 | 5 | holds |
| tests/board_tier2.rs | a_wrong_alternate_function_fails | 15.7 | - | 31.95 | 5 | candidate: alone run |
| tests/adversarial_1301.rs | attack_a_wide_reference_stays_clear_of_every_other_part | 30.7 | 30.52 | 31.65 | 5 | holds |
| tests/twin_tier2.rs | an_aborted_machine_returns_inside_the_bound_as_cpu_aborted | 30.7 | - | 31.20 | 5 | holds |
| tests/escape_tier1.rs | twenty_renamings_route_the_reference_to_its_own_copper | 24.0 | - | 30.70 | 5 | holds |
| tests/battery_sag_tier1.rs | the_power_fixture_s_vdd_5v_stays_unmet_with_its_reason | - | 30.11 | - | 5 | lags (solo) |
| tests/twin_tier2.rs | adv_2_a_presc_write_in_the_bring_up_fails | 12.3 | - | 28.69 | 5 | candidate: alone run |
| tests/generate_tier1.rs | verify_against_fails_on_one_changed_byte_in_any_file_naming_it | 17.4 | 21.78 | 26.34 | 5 | candidate (solo holds) |
| tests/extract_tier1.rs | a_bound_missed_where_the_ladder_ran_states_no_cause | - | 25.39 | 15.62 | 5 | lags (solo) |
| tests/battery_sag_tier1.rs | sentinel_03_s_scenarios_cover_every_row_and_the_package_waits_on_its_ledger | - | 23.45 | 22.04 | 5 | lags (solo) |
| tests/twin_tier2.rs | the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused | 0.1 | - | 23.26 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_dshot600_fixture_s_campaign_passes_inside_its_window | 0.1 | - | 23.15 | 5 | candidate: alone run |
| tests/twin_tier2.rs | an_interrupt_armed_after_the_boot_word_fails_naming_it | 13.6 | - | 22.94 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail | 4.1 | - | 22.39 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_imu_and_barometer_fixture_runs_every_script_step_witnessed | 7.4 | - | 22.36 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_fixture_s_campaign_archives_each_configuration_and_replays | 0.1 | - | 22.19 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed | 10.7 | - | 19.70 | 5 | candidate: alone run |
| tests/twin_tier2.rs | an_open_rx_line_is_detected_by_name_and_attributed_e3 | 0.1 | - | 19.01 | 5 | candidate: alone run |
| tests/twin_tier2.rs | l1_bytes_written_before_the_frame_never_count_as_its_echo | 10.4 | - | 18.73 | 5 | candidate: alone run |
| tests/twin_tier2.rs | kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check | 1.0 | - | 18.27 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_value_one_bit_off_fails_as_an_identity_mismatch | 3.4 | - | 18.03 | 5 | candidate: alone run |
| tests/twin_tier2.rs | l1_a_pattern_sent_after_the_boot_word_never_counts_as_the_echo | 12.5 | - | 17.86 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv2_5_a_locked_tx_pin_fails | 5.3 | - | 16.52 | 5 | candidate: alone run |
| unittests src/lib.rs | supply::tests::a_knockout_beside_a_via_costs_the_plane_only_its_own_side | 8.6 | - | 15.99 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_forged_spi_record_fails_unwitnessed | 4.9 | - | 15.23 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv_2b_a_presc_write_from_main_after_the_bring_up_fails | 4.4 | - | 15.19 | 5 | candidate: alone run |
| tests/models_tier2.rs | a_big_endian_packet_with_an_invalid_gyro_slot_follows_the_datasheet | 6.6 | 4.02 | 14.34 | 5 | candidate (solo holds) |
| tests/twin_tier2.rs | adv2_2_an_open_drain_pulled_down_tx_fails | 5.0 | - | 13.88 | 5 | candidate: alone run |
| tests/models_tier2.rs | an_access_the_datasheets_forbid_warns | 9.1 | - | 13.66 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed | 7.6 | - | 13.65 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_seeded_barometer_fault_is_named_by_its_step | 7.2 | - | 13.06 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_forged_record_without_a_transaction_fails_unwitnessed | 7.7 | - | 12.79 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_droop_boot_killed_at_a_bound_is_a_verdict_and_replays | 7.1 | - | 12.62 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_tdr_store_with_ue_clear_sends_no_pattern | 4.3 | - | 11.97 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s | 4.4 | - | 11.85 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_injection_s_lines_are_read_whole_and_in_place | 7.6 | - | 11.83 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv_3_sbus_s_m0_cleared_before_ue_fails | 6.3 | - | 11.32 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv_1_a_tx_pin_routed_only_after_the_pattern_fails | 5.5 | - | 11.06 | 5 | candidate: alone run |
| unittests src/main.rs | tests::provenance_is_generated_only_for_the_generated_crate | 1.7 | - | 10.51 | 5 | candidate: alone run |
| tests/twin_tier2.rs | without_the_cited_wait_each_data_read_answers_its_reset_value | 6.9 | - | 10.48 | 5 | candidate: alone run |
| unittests src/lib.rs | flow::tests::a_via_with_its_hole_anywhere_in_tolerance_is_bounded_above_each_eccentric_floor | 7.5 | - | 10.38 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_capture_that_is_not_the_pattern_then_the_frame_fails_by_name | 6.1 | - | 9.47 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv3_5_an_rx_pin_unrouted_after_the_first_byte_fails | 4.5 | - | 9.26 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_console_too_slow_for_the_boot_is_refused_before_it | 5.4 | - | 8.96 | 5 | candidate: alone run |
| tests/board_tier2.rs | a_missing_clock_enable_fails | 6.3 | 8.95 | 8.10 | 5 | lags (solo) |
| tests/board_tier2.rs | a_vacuous_harness_fails | 6.2 | - | 8.68 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_judge_s_own_log_after_begin_is_never_the_run_s | 5.0 | - | 8.51 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_read_no_model_answers_fails_as_a_timeout_at_txis | 4.1 | - | 8.43 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv2_4_an_unaligned_store_into_afrh_fails | 5.3 | - | 8.30 | 5 | candidate: alone run |
| tests/generate_tier1.rs | two_generations_are_identical_and_a_used_directory_is_refused_untouched | 5.3 | 7.81 | 8.30 | 5 | lags (solo) |
| tests/twin_tier2.rs | adv_1b_a_tx_pin_unrouted_after_the_first_byte_fails | 5.7 | - | 8.17 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_firmware_that_skips_the_read_or_its_record_cannot_pass | 3.1 | - | 8.13 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv4_2_an_echo_that_hardly_waits_after_the_first_byte_fails | 4.1 | - | 8.11 | 5 | candidate: alone run |
| tests/twin_tier2.rs | m2_a_dma_start_of_cen_fails_closed_by_name | 3.8 | - | 8.10 | 5 | candidate: alone run |
| tests/boot_tier1.rs | an_armed_interrupt_after_the_boot_word_fails | 5.3 | - | 7.95 | 5 | candidate: alone run |
| tests/twin_tier2.rs | every_bit_the_reference_writes_is_covered_by_a_ledger_row | 4.1 | - | 7.90 | 5 | candidate: alone run |
| tests/twin_tier2.rs | an_echo_the_firmware_never_received_is_forged | 4.0 | - | 7.70 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_write_the_bring_up_never_makes_fails_and_another_source_s_does_not_count | 4.3 | - | 7.68 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_flash_controller_is_read_once_and_never_written_or_the_boot_fails | 3.6 | - | 7.25 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv2_3_a_software_reset_at_start_fails | 5.0 | - | 7.17 | 5 | candidate: alone run |
| tests/boot_tier1.rs | anything_beside_the_ports_files_in_the_capture_directory_is_forged | 5.0 | - | 6.99 | 5 | candidate: alone run |
| tests/boot_tier1.rs | two_devices_witness_their_own_reads_in_either_order | 2.0 | - | 6.95 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_record_the_firmware_never_wrote_or_wrote_oddly_fails | 4.9 | - | 6.74 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_emitted_platform_loads_and_lists_every_solved_peripheral | 0.1 | - | 6.46 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_reference_reads_u4_s_jedec_id_witnessed_behind_spi1_s_multiplexer | 3.4 | - | 6.38 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_virtual_time_is_an_integer_unit_up_to_100_ms | 3.5 | - | 6.19 | 5 | candidate: alone run |
| tests/fab_tier1.rs | a_fab_out_another_uid_could_write_is_refused | 4.3 | 5.02 | 5.74 | 5 | candidate (solo holds) |
| tests/twin_tier2.rs | renode_s_configuration_differs_from_its_default_in_synchronous_logging_and_virtual_time_alone | 3.3 | - | 5.41 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_wait_that_expired_fails_as_a_timeout_naming_the_wait | 3.1 | - | 5.38 | 5 | candidate: alone run |
| tests/boot_tier1.rs | log_text_inside_a_value_is_refused | 2.6 | - | 5.31 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_witness_is_not_judged_for_a_record_that_claims_nothing | 1.1 | - | 5.25 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_foreign_firmware_that_stores_into_its_own_flash_fails_flash_overwritten | 3.3 | - | 5.21 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_slave_mode_error_is_exempt_only_after_spi_lld_stop_s_writes | 2.2 | - | 5.13 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_four_port_fixture_s_campaign_passes_and_replays | 0.1 | - | 5.11 | 5 | candidate: alone run |
| tests/twin_tier2.rs | a_device_named_for_a_renode_instance_or_keyword_breaks_the_load | 2.1 | - | 5.10 | 5 | candidate: alone run |
| tests/boot_tier1.rs | word_one_that_is_not_the_value_the_flash_read_returned_fails_naming_both | 1.2 | - | 5.02 | 5 | candidate: alone run |
| tests/twin_tier2.rs | the_pinned_renode_reads_rettobase_0_in_a_handler_nested_over_an_external_interrupt | 3.6 | - | 4.98 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_foreign_judge_is_declared_and_its_memory_is_the_header_s_regions | 3.5 | - | 4.83 | 5 | candidate: alone run |
| tests/board_tier2.rs | verify_refuses_a_board_swapped_before_extract | 3.3 | - | 4.76 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_value_that_is_not_the_part_tables_fails_as_an_identity_mismatch | 3.3 | - | 4.67 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_core_ending_outside_the_firmware_fails | 2.6 | - | 4.56 | 5 | candidate: alone run |
| tests/adversarial_1301.rs | the_board_does_not_depend_on_key_order_locale_or_environment | 3.1 | 3.52 | 4.34 | 5 | candidate (solo holds) |
| tests/boot_tier1.rs | markers_must_appear_once_each_as_whole_lines | 2.9 | - | 4.21 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_witness_that_shows_the_read_passes_and_is_reported | 2.8 | - | 4.18 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | propose_refuses_a_run_replay_refuses_or_a_topology_the_run_did_not_have | 1.4 | - | 4.09 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_bridge_that_fails_or_answers_wrong_stops_the_call | 2.9 | - | 4.07 | 5 | candidate: alone run |
| tests/extract_tier1.rs | a_declared_bound_below_the_bound_fails_naming_each_line | 1.8 | 1.68 | 4.02 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | propose_writes_only_into_an_empty_or_absent_directory_never_through_a_symlink | 1.0 | - | 3.97 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | holding_both_lines_low_needs_no_bridge_at_all | 1.2 | - | 3.96 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_failure_for_a_superset_or_a_wrong_finding_kind_is_a_campaign_failure | 2.6 | - | 3.96 | 5 | candidate: alone run |
| tests/with_tools.rs | the_emitted_reference_schematic_passes_erc | 2.5 | 1.49 | 3.93 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | propose_on_the_47k_run_changes_only_the_pull_up_to_2700_in_identical_bytes_twice | 2.2 | - | 3.84 | 5 | candidate: alone run |
| tests/boot_tier1.rs | more_instructions_than_the_run_holds_or_another_speed_is_untrusted | 2.0 | - | 3.82 | 5 | candidate: alone run |
| tests/boot_tier1.rs | three_agreeing_witnesses_pass | 1.1 | - | 3.82 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_pin_off_its_af_does_not_count_its_bytes | 2.8 | - | 3.80 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_bridge_cannot_write_what_renode_reads | 1.1 | - | 3.73 | 5 | candidate: alone run |
| tests/fab_tier1.rs | an_unacknowledged_unwitnessed_row_is_refused_by_each_fab_flag_naming_every_one | 2.4 | 3.58 | 3.66 | 5 | lags (solo) |
| tests/board_tier2.rs | build_gives_one_sha256_at_two_paths | 0.9 | - | 3.32 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_rise_unfinished_in_the_window_or_a_line_stuck_high_is_a_fault | 1.6 | - | 3.21 | 5 | candidate: alone run |
| tests/boot_tier1.rs | the_expected_value_is_the_tables_whatever_the_record_says_it_expects | 1.0 | - | 3.19 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_register_off_the_solved_image_fails | 1.6 | - | 3.18 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | propose_refuses_a_scenario_or_base_that_is_not_the_runs_and_writes_nothing | 1.4 | - | 3.18 | 5 | candidate: alone run |
| tests/with_tools.rs | simulate_fails_a_platform_that_does_not_load | 2.3 | - | 3.16 | 5 | candidate: alone run |
| tests/escape_tier1.rs | a_via_in_a_courtyard_is_refused_but_in_the_dense_parts_own_ring | 2.0 | 3.05 | 2.94 | 5 | lags (solo) |
| tests/fab_tier1.rs | a_new_field_in_an_acknowledged_register_is_refused_on_the_fab_path | 1.8 | - | 2.84 | 5 | candidate: alone run |
| tests/extract_tier1.rs | a_board_without_test_points_states_no_probe | 1.3 | 2.80 | 1.59 | 5 | lags (solo) |
| tests/boot_tier1.rs | another_renode_is_unusable_for_a_boot | 1.1 | - | 2.74 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_run_resealed_with_the_same_header_replays_so_the_next_test_is_not_vacuous | 1.5 | - | 2.72 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_listing_without_a_solved_peripheral_is_untrusted | 2.0 | - | 2.72 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | the_floor_is_judged_at_the_buss_declared_speed_not_its_ceilings | 0.9 | - | 2.71 | 5 | candidate: alone run |
| tests/speeds_tier1.rs | the_four_layer_fixture_and_d1_hold_on_every_in_scope_line | 1.8 | 1.68 | 2.69 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | an_absolute_base_is_refused_before_a_run | - | - | 2.67 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_scenario_name_that_is_no_identifier_is_refused_before_a_run | - | - | 2.67 | 5 | candidate: alone run |
| tests/boot_tier1.rs | inputs_are_refused_before_renode_is_looked_for | 1.8 | - | 2.58 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | the_2k2_reference_scenario_passes_archives_and_replays_identically | 0.9 | - | 2.56 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_dshot_board_boots_the_whole_100_ms | 1.3 | - | 2.46 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_stray_carriage_return_around_a_line_is_tolerated | 1.0 | - | 2.45 | 5 | candidate: alone run |
| tests/support_tier1.rs | a_connector_that_cannot_program_the_part_fails | 1.5 | - | 2.39 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_touch_of_a_gated_peripheral_fails | 1.1 | - | 2.37 | 5 | candidate: alone run |
| unittests src/lib.rs | flow::tests::a_via_in_a_disc_is_bounded_above_its_shorted_floor | 1.5 | - | 2.31 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_rail_out_of_band_is_refused_by_name_and_rendered_by_exactly_its_consumers | - | - | 2.29 | 5 | candidate: alone run |
| tests/board.rs | the_firmware_follows_the_judges_record_layout | 1.6 | 1.60 | 2.21 | 5 | candidate (solo holds) |
| tests/board.rs | a_hand_edited_harness_fails_the_comparison | 1.3 | - | 2.17 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | the_47k_reference_scenario_fails_as_declared_and_replays_to_the_same_hash | 1.4 | - | 2.11 | 5 | candidate: alone run |
| tests/board.rs | the_committed_board_crate_is_exactly_the_generated_one | 1.3 | 1.68 | 2.10 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | the_deck_s_answer_is_judged_before_any_run | - | - | 2.09 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_scenario_on_a_power_tree_passes_every_check_before_its_run | - | - | 2.09 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | attribute_refuses_whatever_replay_refuses_with_the_same_bytes_and_code | 1.3 | - | 2.08 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | propose_on_a_software_run_writes_a_generator_report_and_no_specification | 0.9 | - | 2.06 | 5 | candidate: alone run |
| tests/support_tier1.rs | a_removed_per_pin_capacitor_fails | 1.2 | 1.22 | 2.05 | 5 | candidate (solo holds) |
| tests/boot_tier1.rs | the_boot_flags_come_together_or_not_at_all | - | - | 2.00 | 5 | candidate: alone run |
| unittests src/main.rs | renode::tests::each_anchor_bound_fails_by_name | 1.3 | - | 1.99 | 5 | candidate: alone run |
| tests/boot_tier1.rs | each_line_is_recorded_apart_from_the_boot_and_no_fault_leaves_the_platform_as_emitted | 1.2 | - | 1.99 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_trace_that_is_a_symlink_to_an_identical_copy_is_refused | 0.8 | - | 1.98 | 5 | candidate: alone run |
| tests/board.rs | the_firmware_follows_the_judges_pwr_boot_words | - | 1.51 | 1.94 | 5 | lags (solo) |
| tests/campaign_tier1.rs | a_run_with_a_consistently_resealed_but_changed_header_is_not_a_campaign_run | 1.1 | - | 1.93 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_passing_identity_read_passes_and_the_unmodelled_part_is_reported_as_info | 1.1 | - | 1.91 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | propose_on_a_passing_or_indeterminate_run_proposes_nothing_and_names_the_rule | 1.0 | - | 1.88 | 5 | candidate: alone run |
| tests/with_tools.rs | with_a_token_free_setup_the_project_step_is_load_bearing | 1.3 | - | 1.79 | 5 | candidate: alone run |
| tests/subgrid_tier1.rs | a_pad_the_grid_reaches_gets_no_escape | 1.3 | - | 1.78 | 5 | candidate: alone run |
| unittests src/main.rs | run::tests::the_capture_cap_holds_with_and_without_the_filter | 0.8 | - | 1.78 | 5 | candidate: alone run |
| tests/bench_tier1.rs | boot0_wired_or_floating_or_pulled_the_wrong_way_fails | 1.1 | 1.15 | 1.71 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | replay_s_serial_silicon_check_is_the_judge_s_on_a_sweep | - | - | 1.71 | 5 | candidate: alone run |
| tests/speeds_tier1.rs | the_reference_drives_sck_and_mosi_at_01_and_names_every_other_line | 1.0 | - | 1.70 | 5 | candidate: alone run |
| tests/routing_tier1.rs | the_routers_judges_refuse_orphan_and_relabelled_copper | 1.0 | 1.62 | 1.09 | 5 | lags (solo) |
| tests/extract_tier1.rs | a_bound_missed_with_no_ladder_names_the_field_that_runs_it | - | 1.60 | 1.58 | 5 | lags (solo) |
| tests/campaign_tier1.rs | an_undeclared_path_or_esc_halts_a_battery_sag_by_name | - | - | 1.57 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_passing_run_under_an_expect_fail_scenario_exits_1 | 0.9 | - | 1.57 | 5 | candidate: alone run |
| tests/support_tier1.rs | a_strap_or_a_supply_pin_off_its_net_fails | 1.0 | 1.56 | 1.08 | 5 | lags (solo) |
| tests/campaign_tier1.rs | a_droop_scenario_renders_its_reset_through_the_bridge_in_one_command_line | 0.9 | - | 1.56 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_one_byte_change_with_the_manifest_resealed_diverges_on_replay | 0.8 | - | 1.56 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | attribute_names_the_47k_run_electrical_with_its_rises_and_fault | 1.0 | - | 1.53 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | the_droop_scenarios_carry_only_identifiers_the_base_set_admits | - | - | 1.53 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_failing_run_under_an_expect_pass_scenario_exits_1_unmatched | 0.8 | - | 1.51 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_run_directory_that_is_a_symlink_to_a_real_run_is_refused | 0.7 | - | 1.49 | 5 | candidate: alone run |
| tests/routing_tier1.rs | every_pair_of_copper_items_keeps_the_clearance | 1.0 | 1.49 | 1.47 | 5 | lags (solo) |
| tests/speeds_tier1.rs | a_specification_with_no_spi_bus_judges_nothing_and_says_so | 1.0 | - | 1.44 | 5 | candidate: alone run |
| tests/bench_tier1.rs | boot0_shared_with_another_pin_fails | 0.8 | 1.21 | 1.42 | 5 | lags (solo) |
| tests/boot_tier1.rs | a_boot_word_never_written_fails | 0.9 | - | 1.42 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_boot_too_short_for_the_injection_fails_closed_on_the_echo | 1.0 | - | 1.41 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_manifest_that_is_a_fifo_is_refused_without_hanging | 0.9 | - | 1.40 | 5 | candidate: alone run |
| tests/classes_tier1.rs | extract_bounds_a_classed_line_at_its_own_width | 0.6 | 0.75 | 1.40 | 5 | candidate (solo holds) |
| tests/campaign_tier1.rs | a_held_low_sda_line_is_never_sent_to_the_bridge_and_faults_the_bus | 0.8 | - | 1.39 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_release_too_late_for_the_serial_injection_is_refused_by_name | 0.9 | - | 1.36 | 5 | candidate: alone run |
| tests/board_tier2.rs | as_root_a_third_uid_cannot_change_what_build_reads | 0.9 | - | 1.35 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_failure_for_a_subset_of_the_expected_findings_is_a_campaign_failure | 0.9 | - | 1.33 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | an_open_rx_line_on_an_unknown_port_is_refused | - | - | 1.29 | 5 | candidate: alone run |
| tests/classes_tier1.rs | the_pair_judge_refuses_each_breakage | 0.8 | - | 1.28 | 5 | candidate: alone run |
| tests/extract_tier1.rs | a_board_with_no_bounded_line_judges_nothing_and_says_so | - | - | 1.27 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | replaying_a_missing_directory_is_refused | - | - | 1.25 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | the_same_scenario_file_twice_is_a_duplicate | - | - | 1.21 | 5 | candidate: alone run |
| tests/campaign_tier1.rs | a_droop_past_a_dshot_board_s_boot_is_refused_with_its_time | 0.7 | - | 1.16 | 5 | candidate: alone run |
| tests/speeds_tier1.rs | the_motor_driver_judges_no_line_and_says_so | - | 1.13 | 1.10 | 5 | lags (solo) |
| unittests src/lib.rs | flow::tests::the_two_hole_sheet_is_bounded_and_converges_under_refinement | 0.7 | - | 1.11 | 5 | candidate: alone run |
| tests/connectors_tier1.rs | a_board_draws_exactly_its_declared_connectors | 0.7 | - | 1.09 | 5 | candidate: alone run |
| unittests src/main.rs | renode::tests::every_run_for_runs_exactly_its_nanoseconds_in_renode | - | - | 1.05 | 5 | candidate: alone run |
| tests/with_tools.rs | prove_reports_a_compile_error | - | - | 1.05 | 5 | candidate: alone run |
| tests/boot_tier1.rs | a_boot_on_another_renode_build_is_refused_by_name | 0.7 | - | 1.04 | 5 | candidate: alone run |

### 1.1 The rows named lagging (before)

- **Colocated blocks**, by their `finished in` [measured]:
  - `supply_tier1`: 302 against 720.39 (44747464266, shard 8).
  - `attribute_seeded`: 380 against 525.12.
  - `campaign_tier2`: 54 against 82.74.
  - The other five blocks sit within 25 % of their worst: `four_layer_fab_tier2` 0.88, `verify_47k_tier2` 0.79, `outline_tier1` 0.90, `power_board_tier1` 1.04, `via_ring_tier1` 0.97.
- **`supply_tier1` is bimodal:** 291.74, 379.35 and 443.66 on three runs, 716.11 and 720.39 on two, always in one process [measured]. The throwaway run prints each runner's CPU model, to test whether the runner explains it.
- **Tests:**
  - 16 rows lag a `solo` sample, an alone time on CI.
  - 172 are candidates on a `lock` or `ub` sample: 159 with no alone time yet, and 13 whose `solo` sample already holds the row.
  - All 188 go to §2's alone run. (Count corrected after the §1 commit, which said 16 and 176 because it counted the header's own lines.)
  - The largest: `twin_tier2::kani_proves_every_part_of_both_motor_fixtures` (169.8 row, 447.74 `lock`), `board_tier2::every_admitted_identifier_character_survives_every_output` (181.2, 271.16 `lock`) and `verify_2k2_tier2::verify_binds_the_reference_and_two_runs_print_the_same_bytes` (372.6, 515.84 `solo`).
- **`a_missing_clock_enable_fails`'s 62.42 s** (44712465606, shard 8, `1318` F2) is read as a lock wait [read, the job log]. Its own prove ("seed-no-clock") held the lock 10.80 s. The 51.61 s before it was `the_generated_board_proves`' `board_i2c1` prove. On the five runs it is 8.95 s `solo` at worst. §2's alone run decides its row.
- **Overhead is not in any row.** The shared build's download took 47–444 s on one run (44732069104: shard 9 444 s, shard 10 388 s, shard 3 277 s) [measured]. That is up to 9 minutes of a 20-minute job, before any test. It is outside this fence (F1).

## 2. Alone times and the new rows (§1 steps 2–4)

**The throwaway** (REFIT RULE 1):
- After a shard's own tests, `ci/shard.sh run` ran each name `spike/1912/alone.tsv` gave that shard alone: `cargo test --workspace … -- --exact NAME --test-threads=1`, one invocation each.
  - A one-test, one-thread binary's libtest `finished in` is the test's alone time.
  - `RUSTC_BOOTSTRAP` and `--report-time` were not needed. They would have made the shard compile, which `compiled_in` refuses since `1911`.
- **The names** (`spike/1912/alone.py`): every test that §1 named lagging or candidate, 188 in all. They were spread longest-first over the 12 shards, so no shard carried more than about 520 s of alone runs.
- **The first try failed** (run 44751773365, `bdf74b1d`), harmlessly: every alone invocation failed `SandboxUnavailable` [measured].
  - A loop fed by `< <(…)` left fd 63 open in cargo's tests. The CLI's descriptors sandbox refuses any descriptor inherited without close-on-exec, so INV-06 held.
  - The fix (`1d583f6e`) reads the list whole with `mapfile` first.
  - Its normal passes were green, so its block times are kept as samples.
- **The second try** (run 44755810025, `1d583f6e`) was green. It measured all 188 names, each once [measured, `spike/1912/alone_times.py`].
- Both throwaway commits are undone. `ci/shard.sh` is `main`'s byte for byte (`git diff 472d2d84 -- ci/shard.sh` is empty).

**Runners are not alike.** The throwaway printed each shard's CPU [measured]: AMD EPYC 9V45, 9V74 and 7763, Intel Xeon 6975P-C and Xeon Platinum 8573C, five models over 24 jobs. A row is CI's worst; it is not any one runner's time.

**The rule applied** (`spike/1912/refit.py`, REFIT RULE 1–3 and 5):
- **A test row** changes only when it sits more than 25 % under its worst trustworthy sample, and then it takes that sample to 3 decimals.
  - Trustworthy means an alone time: the throwaway's, or a `solo` sample of the five runs.
  - A test with no row gets one at 0.5 s or more.
- **A block row** takes the worst libtest `finished in` over the five runs and both throwaways' normal passes, rounded up.
- **No row was lowered.**
- **Of the 188 names:** 37 rows were raised, 10 added and 141 kept.
  - Of the kept, 130 had a `lock` or `ub` candidate more than 25 % above the row, but their alone time held the row. The candidate was a lock wait or an upper bound.

| binary | test | row before | row after | from (s, run, shard, job) | worst lock/ub candidate | verdict |
|---|---|---|---|---|---|---|
| tests/battery_sag_tier1.rs | sentinel_03_s_scenarios_cover_every_row_and_the_package_waits_on_its_ledger | - | 23.450 | 23.45 (44735701946, 7, job 136289665198) | 22.04 | new row |
| tests/battery_sag_tier1.rs | the_power_fixture_s_vdd_5v_stays_unmet_with_its_reason | - | 30.110 | 30.11 (44747464266, 11, job 136329323330) | - | new row |
| tests/bench_tier1.rs | boot0_shared_with_another_pin_fails | 0.8 | 1.380 | 1.38 (44755810025, 6, job 136358895954) | 1.42 | raised |
| tests/board.rs | the_firmware_follows_the_judges_pwr_boot_words | - | 1.510 | 1.51 (44735207864, 3, job 136286966295) | 1.94 | new row |
| tests/board_tier2.rs | a_missing_clock_enable_fails | 6.3 | 9.290 | 9.29 (44755810025, 6, job 136358895954) | 8.10 | raised |
| tests/board_tier2.rs | a_wrong_alternate_function_fails | 15.7 | 29.110 | 29.11 (44755810025, 9, job 136358895794) | 31.95 | raised |
| tests/board_tier2.rs | every_admitted_identifier_character_survives_every_output | 181.2 | 265.850 | 265.85 (44755810025, 2, job 136358895656) | 271.16 | raised |
| tests/board_tier2.rs | every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict | 15.0 | 49.950 | 49.95 (44755810025, 5, job 136358895693) | 77.49 | raised |
| tests/board_tier2.rs | verify_refuses_a_board_swapped_before_extract | 3.3 | 4.550 | 4.55 (44755810025, 6, job 136358895954) | 4.76 | raised |
| tests/campaign_tier1.rs | a_droop_boot_killed_at_a_bound_is_a_verdict_and_replays | 7.1 | 9.540 | 9.54 (44755810025, 9, job 136358895794) | 12.62 | raised |
| tests/campaign_tier1.rs | a_rail_out_of_band_is_refused_by_name_and_rendered_by_exactly_its_consumers | - | 1.110 | 1.11 (44755810025, 6, job 136358895954) | 2.29 | new row |
| tests/campaign_tier1.rs | a_scenario_on_a_power_tree_passes_every_check_before_its_run | - | 1.340 | 1.34 (44755810025, 7, job 136358895679) | 2.09 | new row |
| tests/classes_tier1.rs | extract_bounds_a_classed_line_at_its_own_width | 0.6 | 0.920 | 0.92 (44755810025, 5, job 136358895693) | 1.40 | raised |
| tests/escape_tier1.rs | a_via_in_a_courtyard_is_refused_but_in_the_dense_parts_own_ring | 2.0 | 3.170 | 3.17 (44755810025, 6, job 136358895954) | 2.94 | raised |
| tests/extract_tier1.rs | a_board_with_no_bounded_line_judges_nothing_and_says_so | - | 0.940 | 0.94 (44755810025, 4, job 136358895734) | 1.27 | new row |
| tests/extract_tier1.rs | a_board_without_test_points_states_no_probe | 1.3 | 2.810 | 2.81 (44755810025, 7, job 136358895679) | 1.59 | raised |
| tests/extract_tier1.rs | a_bound_missed_where_the_ladder_ran_states_no_cause | - | 25.390 | 25.39 (44735701946, 10, job 136289665037) | 15.62 | new row |
| tests/extract_tier1.rs | a_bound_missed_with_no_ladder_names_the_field_that_runs_it | - | 1.600 | 1.60 (44732069104, 1, job 136276909823) | 1.58 | new row |
| tests/fab_tier1.rs | an_unacknowledged_unwitnessed_row_is_refused_by_each_fab_flag_naming_every_one | 2.4 | 3.770 | 3.77 (44755810025, 6, job 136358895954) | 3.66 | raised |
| tests/generate_tier1.rs | two_generations_are_identical_and_a_used_directory_is_refused_untouched | 5.3 | 7.810 | 7.81 (44747545277, 3, job 136329613012) | 8.30 | raised |
| tests/generate_tier1.rs | verify_against_fails_on_one_changed_byte_in_any_file_naming_it | 17.4 | 28.170 | 28.17 (44755810025, 6, job 136358895954) | 26.34 | raised |
| tests/routing_tier1.rs | every_pair_of_copper_items_keeps_the_clearance | 1.0 | 1.490 | 1.49 (44747464266, 4, job 136329323270) | 1.47 | raised |
| tests/routing_tier1.rs | the_routers_judges_refuse_orphan_and_relabelled_copper | 1.0 | 1.620 | 1.62 (44747545277, 4, job 136329612963) | 1.09 | raised |
| tests/speeds_tier1.rs | the_motor_driver_judges_no_line_and_says_so | - | 1.130 | 1.13 (44735207864, 2, job 136286966414) | 1.10 | new row |
| tests/support_tier1.rs | a_strap_or_a_supply_pin_off_its_net_fails | 1.0 | 1.560 | 1.56 (44747545277, 4, job 136329612963) | 1.08 | raised |
| tests/twin_tier2.rs | a_device_named_for_a_renode_instance_or_keyword_breaks_the_load | 2.1 | 3.040 | 3.04 (44755810025, 10, job 136358895770) | 5.10 | raised |
| tests/twin_tier2.rs | a_firmware_that_skips_the_read_or_its_record_cannot_pass | 3.1 | 11.930 | 11.93 (44755810025, 7, job 136358895679) | 8.13 | raised |
| tests/twin_tier2.rs | a_foreign_firmware_that_stores_into_its_own_flash_fails_flash_overwritten | 3.3 | 4.660 | 4.66 (44755810025, 6, job 136358895954) | 5.21 | raised |
| tests/twin_tier2.rs | a_forged_spi_record_fails_unwitnessed | 4.9 | 7.390 | 7.39 (44755810025, 10, job 136358895770) | 15.23 | raised |
| tests/twin_tier2.rs | a_tdr_store_with_ue_clear_sends_no_pattern | 4.3 | 6.220 | 6.22 (44755810025, 5, job 136358895693) | 11.97 | raised |
| tests/twin_tier2.rs | adv4_2_an_echo_that_hardly_waits_after_the_first_byte_fails | 4.1 | 5.630 | 5.63 (44755810025, 5, job 136358895693) | 8.11 | raised |
| tests/twin_tier2.rs | adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail | 4.1 | 6.130 | 6.13 (44755810025, 11, job 136358895637) | 22.39 | raised |
| tests/twin_tier2.rs | an_open_rx_line_is_detected_by_name_and_attributed_e3 | 0.1 | 9.070 | 9.07 (44755810025, 10, job 136358895770) | 19.01 | raised |
| tests/twin_tier2.rs | kani_proves_every_part_of_both_motor_fixtures | 169.8 | 410.230 | 410.23 (44755810025, 1, job 136358895632) | 447.74 | raised |
| tests/twin_tier2.rs | kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check | 1.0 | 16.350 | 16.35 (44755810025, 7, job 136358895679) | 18.27 | raised |
| tests/twin_tier2.rs | m2_a_dma_start_of_cen_fails_closed_by_name | 3.8 | 6.100 | 6.10 (44755810025, 6, job 136358895954) | 8.10 | raised |
| tests/twin_tier2.rs | renode_s_configuration_differs_from_its_default_in_synchronous_logging_and_virtual_time_alone | 3.3 | 8.620 | 8.62 (44755810025, 5, job 136358895693) | 5.41 | raised |
| tests/twin_tier2.rs | the_dshot600_fixture_s_campaign_passes_inside_its_window | 0.1 | 13.430 | 13.43 (44755810025, 9, job 136358895794) | 23.15 | raised |
| tests/twin_tier2.rs | the_dshot600_fixture_s_outputs_are_witnessed_in_their_window_of_a_100_ms_boot | 1.3 | 4.730 | 4.73 (44755810025, 8, job 136358895641) | 47.73 | raised |
| tests/twin_tier2.rs | the_emitted_platform_loads_and_lists_every_solved_peripheral | 0.1 | 2.470 | 2.47 (44755810025, 4, job 136358895734) | 6.46 | raised |
| tests/twin_tier2.rs | the_fixture_s_campaign_archives_each_configuration_and_replays | 0.1 | 11.740 | 11.74 (44755810025, 5, job 136358895693) | 22.19 | raised |
| tests/twin_tier2.rs | the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused | 0.1 | 4.000 | 4.00 (44755810025, 10, job 136358895770) | 23.26 | raised |
| tests/twin_tier2.rs | the_four_port_fixture_s_campaign_passes_and_replays | 0.1 | 4.700 | 4.70 (44755810025, 4, job 136358895734) | 5.11 | raised |
| tests/twin_tier2.rs | the_reference_firmware_boots_and_two_boots_agree | 5.4 | 7.340 | 7.34 (44755810025, 4, job 136358895734) | 154.77 | raised |
| tests/verify_2k2_tier2.rs | verify_binds_the_reference_and_two_runs_print_the_same_bytes | 372.6 | 537.170 | 537.17 (44755810025, 0, job 136358895660) | - | raised |
| unittests src/lib.rs | supply::tests::a_relieved_pad_joins_no_plane | 39.5 | 57.430 | 57.43 (44755810025, 6, job 136358895954) | 58.94 | raised |
| unittests src/main.rs | renode::tests::every_run_for_runs_exactly_its_nanoseconds_in_renode | - | 0.760 | 0.76 (44755810025, 6, job 136358895954) | 1.05 | new row |
| tests/attribute_seeded.rs | (block) | 380.0 | 526 | 525.12 (44735207864, 5, job 136286966303) | - | raised |
| tests/campaign_tier2.rs | (block) | 54.0 | 83 | 82.74 (44735207864, 7, job 136286966265) | - | raised |
| tests/supply_tier1.rs | (block) | 302.0 | 721 | 720.39 (44747464266, 8, job 136329323352) | - | raised |

**`a_missing_clock_enable_fails` (§1 step 4):**
- Alone, it took 9.29 s (44755810025, shard 6, Xeon Platinum 8573C). Its `solo` samples on the five runs are at most 8.95 s.
- Its row rises 6.3 → 9.29 on the alone time, which is more than 25 % over the row.
- The 62.42 s of 44712465606 was a lock wait [read]. That reading was its own 10.80 s prove behind `the_generated_board_proves`' 51.61 s `board_i2c1` prove. That run is not what raised the row.

## 4. Adversarial pass, targeted (§5)

`spike/1912/adv.py`, over the five runs and both throwaways' normal passes [measured]:

- **A row raised from one outlier that would over-plan a shard every run.**
  - Every alone time is one sample, by the rule. So every raised row was checked against the test's in-shard upper bound on all seven runs: its `ok` less its binary's `Running`, which bounds any test, locked or not.
  - **6 of the 47 raised rows sit above every upper bound** over eight runs (the five, both throwaways and the PR's run 44762196410).
    - Four are within 0.11 s, rounding: `a_via_in_a_courtyard…` 3.17/3.06, `a_bound_missed_with_no_ladder…` 1.60/1.60, `an_unacknowledged_unwitnessed_row…` 3.77/3.66, `a_strap_or_a_supply_pin…` 1.56/1.56.
    - Two are material: `verify_binds_the_reference_and_two_runs_print_the_same_bytes` 537.17 against 515.84 (+4 %, alone on an EPYC 7763), and `verify_against_fails_on_one_changed_byte…` 28.17 against 26.34 (+7 %).
    - Kept at the alone times, as the rule says. Together they over-plan their shards by at most about 23 s, 3 % of a shard.
  - **The large raises are corroborated by the five runs.**
    - `kani_proves_every_part_of_both_motor_fixtures` 410.23: the `twin_tier2` binary holding it finished in 416–571 s on all seven runs.
    - `every_admitted_identifier…` 265.85 against a 271.16 `lock` candidate.
    - `verify_binds…` 537.17 against `solo` 364–516 on the five runs and 494–498 on the throwaways.
  - **`supply_tier1` at 721 is the widest:** samples 291.74–720.39, three of eight at 621 or more.
    - It is not one outlier. It is a mode, two runs of the five, and the 20-minute ceiling needs it.
    - On a fast runner it over-plans its shard by up to about 430 s. That is the spread's cost (F2).
- **A colocated block weighed from a shared-process run while it waited on a lock.**
  - Every block sample of `supply_tier1` (10 tests), `attribute_seeded` (11) and `campaign_tier2` (10), on all eight runs, ran its whole binary in one process (passed = the binary's total, 0 filtered).
  - Cargo runs a shard's binaries one after another, so no other binary's process held a lock during a block.
  - The three raised blocks were weighed only from such runs.

## 3. The plan, and the table after (§1 step 5)

**The plan** [measured, `ci/shard.sh assign 12 … 37 colocate "$LOCKED"` on CI's own list, the `shard-0` artifact's `entries.tsv`]:
- **Before:** `main` 472d2d84's rows on that run's list (44747545277) plan 706 s on every shard. That reproduces CI's plan byte for byte (`shard-0.tsv` identical).
- **After:** this order's rows on the same list plan **819 s on every shard**, `guard ok: 2642 test names, 2649 listed occurrences, 12 shards`.
- CI's own plan on the merged tree printed 819 s per shard (run 44762196410). REFIT RULE 5's clause (every shard at or under 1006 s) holds.
- **On the final tree** (`main` `8fdc9422` merged, with `4406`'s and `4241`'s tests; list from `main`'s run 44764528379, whose own plan this reproduces byte for byte at 710 s): **823 s on every shard**, `guard ok: 2676 test names, 2683 listed occurrences, 12 shards`.

**The PR's run 44762196410** (head `22f7d8f3`, `main` `a37b787f` merged), green, against the rows of this order [measured]:

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136381084692 | 9.9 | 423 | 174 | 819 | 782 |  | twin_tier2.rs 59/306 |
| 1 | 136381084560 | 10.5 | 438 | 191 | 819 | 819 |  | supply_tier1.rs 380/721 |
| 2 | 136381084554 | 17.1 | 909 | 120 | 819 | 819 | four_layer_fab_tier2.rs 741/625 |  |
| 3 | 136381084650 | 14.3 | 703 | 155 | 819 | 819 |  | verify_2k2_tier2.rs 463/537 |
| 4 | 136381084495 | 13.0 | 649 | 133 | 819 | 819 |  | attribute_seeded.rs 443/526 |
| 5 | 136381084422 | 14.9 | 732 | 163 | 819 | 819 |  | board_tier2.rs 98/162 |
| 6 | 136381084924 | 14.9 | 733 | 159 | 819 | 819 |  |  |
| 7 | 136381085061 | 15.2 | 776 | 133 | 819 | 819 | against_fab_tier2.rs 490/421 |  |
| 8 | 136381084582 | 16.9 | 916 | 100 | 819 | 819 | twin_tier2.rs 636/536 |  |
| 9 | 136381084536 | 13.5 | 658 | 152 | 819 | 819 |  |  |
| 10 | 136381084611 | 10.6 | 513 | 124 | 819 | 819 |  | board_tier2.rs 138/205; twin_tier2.rs 241/396 |
| 11 | 136381084594 | 14.6 | 762 | 116 | 819 | 819 |  | twin_tier2.rs 130/198 |

- **Every shard is under 20 minutes.** Worst: shard 2 at 17.1 min, on `four_layer_fab_tier2` 740.63 s against 625. That is a new worst, 0.84 of its row, so not lagging by the WO's 25 % (D2).
- **The spread is 7.2 minutes** (9.9 to 17.1), over §2's 6 minutes (D1).
  - The fast end is rows that ran under: `supply_tier1` 380 s against 721 (shard 1), and `twin_tier2` 59 s against 306 s of rows (shard 0).
  - The slow end is `four_layer_fab_tier2` (shard 2) and `twin_tier2` 636 s against 536 (shard 8).
- **On main before this order** the spread was 7.8, 10.7, 12.2, 7.3 and 11.3 minutes on the five runs. Main's worst shard was 23.0 min; here it is 17.1.

## 5. Gates, the reduction pass and CI

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean on `2045e7a0` and on the final tree `af964568` |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean on both |
| `cargo test --workspace` | **2,452 passed, 0 failed, 0 ignored** on the final tree `af964568` (2,436 on `2045e7a0`) |
| `bash ci/test-shard.sh` | passes on the final tree |
| `ci/shard.sh` | `main`'s byte for byte: the throwaway (`bdf74b1d`, `1d583f6e`) is undone |
| `TIER2_MIN` | unchanged; no workflow touched |
| CI | green on `22f7d8f3` (run 44762196410, every shard under 20 min). The READY head's run is the final one, reported on the bus |

- **Reduction pass (diff):** about −9 lines net in `spike/1912/` · 2 duplicates removed (the LOCKED list now read from `ci/shard.sh`; one upper-bound walker, so `over.py` folded into `adv.py`) · 2 dead paths removed (`shard()`'s unused CPU field, `alone.py`'s unused extra names) · 0 protected kept · 0 findings below. The tsv rows are data, and each fact has one row.

## 6. Findings and decisions

| # | Finding | For |
|---|---|---|
| D1 | **The spread clause (§2, under 6 min) did not hold:** 7.2 min on the PR's run. On `main` before this order it was 7.3–12.2 min on all five runs. Three causes, measured on run 44762196410 and the five (`table.py`'s over/under columns):<br>(i) `supply_tier1` runs in two modes, 292–444 s and 621–720 s. Weighed at its worst, as the 20-minute ceiling needs, its shard finishes up to about 430 s early in the fast mode. The throwaway saw 621 s on a Xeon 6975P-C and 373 s on an EPYC 9V74: two CPU-labelled samples, so the cause is **not established** [inferred, not measured].<br>(ii) **`twin_tier2`'s rows sit high:** they total 1,807–1,811 s against 1,201–1,390 s run per run. Examples (in-shard `lock` readings, which are candidates, not alone times): `on_a_bus_with_two_devices_each_read_is_witnessed` 216.4 (added by AET-WO-2204) read 7.8 s on run 44762196410; two of the three `bor_recovery::` Furnace estimates (22.9, 38.1) read 2.2 and 3.7 s. This order raises only rows that lag (fence), so none was lowered.<br>(iii) Runners differ: five CPU models over 24 throwaway jobs.<br>Options: **(a)** widen the fence to lower rows that sit high, from one more throwaway alone run of every `twin_tier2` and `board_tier2` test (the locked binaries, where no in-shard bound exists); **(b)** reword the spread clause, or REFIT RULE's 6-minute spread trigger, which fires on `main` every run; **(c)** both. | Architect (ruling) |
| D2 | **Two rows under but within 25 %** drive the slowest shard: `four_layer_fab_tier2` 625 against 740.63 (a new worst, run 44762196410) and `verify_47k_tier2` 220 against 277.29. Not raised (the WO's 25 %); raising them is the weights-only lever on the worst shard. | Architect |
| F1 | **The shared build's download is up to 9 minutes of a 20-minute job:** 47–444 s within one run (44732069104), overhead 90–555 s across the five. No row can weigh it. | Note; outside the fence (workflow) |
| F2 | **`supply_tier1`'s modes** (D1 (i)) are measured; their cause is not. One CPU-labelled run of the block on each runner type would settle it. | Backlog |
| F3 | **The throwaway's lesson:** a shell loop fed by `< <(…)` leaves its descriptor open in every command it runs. The CLI's descriptors sandbox refuses it (run 44751773365, all 188 alone runs `SandboxUnavailable`). Read such a list whole first (`mapfile`). | Note for the next refit |

## 7. Neighbours

- `4240a` (#281), `1211`, `4406` and `4241` merged into `main` while this was built. All came in by clean merges. `4241` adds one `twin_tier2` row (45 s), its own. No row of this order moved.
- `1210` (#280) is still open. Its rows are its own.
