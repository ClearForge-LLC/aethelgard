# AET-WO-1913 FEEDBACK — CI's fourth refit: the lock-serialised rows set to their alone times

## Status: READY. Every shard is under 20 minutes on the last run before READY, 44798881940 (head `e5faabd8`, `main` `adb8f61a` merged): worst 16.2 min, **spread 5.1 min**, under the 6-minute target. Tier-2 is 200 / 200 and `TIER2_MIN` is unchanged. The READY head also merges `349c75f5` (docs only), and its own run is reported on the bus. One decision for the architect: D1 (§6).

Builder: Claude (Opus 5.5), a fresh `1913` session, 2026-11-20. Worktree `~/aethelgard-1913`, branch `wo/aet-1913-refit-lower` from `origin/main` `c1012a2d`, with `main` `25203c58`/`69c77c96` (`1211a`, docs) `ba14df2c`/`adb8f61a` (`4240c`, `1213`) and `349c75f5` (docs) merged in. No merge touched `ci/`.

Labels: **measured** (from CI's own logs or runs), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1913/`; the scripts are in `spike/1913/` (`alone.py`, `alone_times.py`, `refit.py` adapted from `spike/1912/`, `locked.py` new); they read `spike/1912/table.py`, and runs are fetched with `spike/1912/fetch.sh`.

## 1. The table before (§1 step 1)

**The five runs:** the five most recent green `ci.yml` runs on `main` at 17:00Z, each with 12 tools shards: 44775091139 (`c1012a2d`, this branch's base), 44774672284 (`63334d69`), 44774205804 (`08f9fb9c`, `1912`'s merge), 44768742667 (`8df353ee`) and 44764528379 (`8fdc9422`). The first three were planned with `1912`'s rows. Each was read with `spike/1912/fetch.sh` (jobs API, `gh run view --job ID --log`) and tabulated by `spike/1912/table.py` and `spike/1913/locked.py` [measured].

**How to read it** (as in `1912` FEEDBACK §1):
- Every measurement is set against the rows at `c1012a2d`. The plan column is the plan CI printed at each run's own head.
- **job min:** GitHub's `started_at` to `completed_at`. **run step:** the `shard.sh run` step. **overhead:** the job less the run step (checkout, image pull, the shared build's download and unpack). Weights cannot move overhead, which ran 96–804 s.
- **Test samples:** `solo` (the only test of its binary in that shard: its binary's `finished in`, an alone time); `lock` (in `board_tier2` or `twin_tier2`, its `ok` less the previous `ok`: both take their lock per call, not per test, so this is not the test's cost); `ub` (its `ok` less its binary's `Running`, an upper bound).

Rows: ci/test-weights.tsv and ci/colocate.tsv at c1012a2d. Runs: 44775091139, 44774672284, 44774205804, 44768742667, 44764528379.

### run 44775091139 (head c1012a2d, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136425978460 | 11.7 | 553 | 149 | 823 | 786 |  | twin_tier2.rs 72/277 |
| 1 | 136425978404 | 15.9 | 709 | 244 | 823 | 823 |  | supply_tier1.rs 651/721 |
| 2 | 136425978348 | 17.0 | 886 | 136 | 823 | 823 | four_layer_fab_tier2.rs 733/625 |  |
| 3 | 136425978466 | 13.4 | 504 | 303 | 823 | 823 |  | verify_2k2_tier2.rs 331/537 |
| 4 | 136425978352 | 11.7 | 532 | 167 | 823 | 823 |  | attribute_seeded.rs 370/526; twin_tier2.rs 54/127 |
| 5 | 136425978472 | 15.8 | 830 | 115 | 823 | 823 | verify_2k2_tier2.rs 548/458 |  |
| 6 | 136425978335 | 14.6 | 723 | 150 | 823 | 823 |  | lib.rs 43/132 |
| 7 | 136425978453 | 17.3 | 933 | 105 | 823 | 823 | against_fab_tier2.rs 585/421 |  |
| 8 | 136425978418 | 13.4 | 637 | 166 | 823 | 823 |  | twin_tier2.rs 486/565 |
| 9 | 136425978349 | 10.7 | 484 | 156 | 823 | 823 |  | escape_tier1.rs 109/169; power_board_tier1.rs 206/331; twin_tier2.rs 47/119 |
| 10 | 136425978428 | 12.6 | 614 | 139 | 823 | 823 |  | twin_tier2.rs 256/375 |
| 11 | 136425978726 | 11.7 | 550 | 153 | 823 | 823 |  | board_tier2.rs 219/307; twin_tier2.rs 56/140 |

### run 44774672284 (head 63334d69, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136424625482 | 13.2 | 531 | 262 | 823 | 786 |  | twin_tier2.rs 69/277 |
| 1 | 136424625471 | 11.0 | 557 | 102 | 823 | 823 |  | supply_tier1.rs 477/721 |
| 2 | 136424625484 | 16.2 | 684 | 287 | 823 | 823 |  |  |
| 3 | 136424625451 | 14.4 | 735 | 131 | 823 | 823 |  |  |
| 4 | 136424625487 | 14.8 | 795 | 96 | 823 | 823 |  |  |
| 5 | 136424625501 | 13.5 | 526 | 283 | 823 | 823 |  | board_tier2.rs 64/150; verify_2k2_tier2.rs 343/458 |
| 6 | 136424625504 | 18.0 | 923 | 156 | 823 | 823 | against_fab_tier2.rs 588/447 | lib.rs 50/132 |
| 7 | 136424625775 | 16.2 | 704 | 269 | 823 | 823 |  |  |
| 8 | 136424625670 | 14.9 | 752 | 140 | 823 | 823 |  |  |
| 9 | 136424625560 | 17.3 | 713 | 327 | 823 | 823 |  |  |
| 10 | 136424625591 | 15.8 | 610 | 340 | 823 | 823 |  | twin_tier2.rs 257/375 |
| 11 | 136424625516 | 15.2 | 808 | 104 | 823 | 823 |  |  |

### run 44774205804 (head 08f9fb9c, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136422874788 | 12.2 | 326 | 409 | 823 | 786 |  | outline_tier1.rs 150/251; twin_tier2.rs 38/277 |
| 1 | 136422874833 | 12.0 | 542 | 180 | 823 | 823 |  | supply_tier1.rs 461/721 |
| 2 | 136422874310 | 14.1 | 649 | 194 | 823 | 823 |  | four_layer_fab_tier2.rs 542/625 |
| 3 | 136422874580 | 16.0 | 567 | 392 | 823 | 823 |  | verify_2k2_tier2.rs 389/537 |
| 4 | 136422874758 | 20.1 | 603 | 604 | 823 | 823 |  | attribute_seeded.rs 416/526; twin_tier2.rs 64/127 |
| 5 | 136422874952 | 18.9 | 802 | 334 | 823 | 823 |  |  |
| 6 | 136422874606 | 15.2 | 777 | 135 | 823 | 823 |  | lib.rs 38/132 |
| 7 | 136422874568 | 16.9 | 920 | 96 | 823 | 823 | against_fab_tier2.rs 576/421 |  |
| 8 | 136422874384 | 27.6 | 851 | 804 | 823 | 823 | twin_tier2.rs 629/565 |  |
| 9 | 136422874763 | 12.2 | 596 | 136 | 823 | 823 |  | power_board_tier1.rs 254/331; twin_tier2.rs 58/119 |
| 10 | 136422874510 | 17.6 | 770 | 284 | 823 | 823 |  |  |
| 11 | 136422874675 | 17.0 | 808 | 213 | 823 | 823 |  |  |

### run 44768742667 (head 8df353ee, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136403197184 | 15.7 | 802 | 142 | 710 | 925 |  | twin_tier2.rs 574/690 |
| 1 | 136403197203 | 16.4 | 720 | 267 | 710 | 754 |  |  |
| 2 | 136403197318 | 14.8 | 792 | 97 | 710 | 768 | verify_2k2_tier2.rs 531/458 |  |
| 3 | 136403197350 | 14.1 | 751 | 96 | 710 | 718 | against_fab_tier2.rs 537/447 |  |
| 4 | 136403197417 | 14.8 | 791 | 98 | 710 | 754 | against_fab_tier2.rs 546/421 |  |
| 5 | 136403197259 | 13.7 | 613 | 211 | 710 | 864 |  | attribute_seeded.rs 381/526 |
| 6 | 136403197343 | 14.7 | 696 | 188 | 710 | 898 |  | verify_2k2_tier2.rs 437/537 |
| 7 | 136403197282 | 9.7 | 408 | 173 | 710 | 770 |  | board_tier2.rs 95/207; power_board_tier1.rs 201/331 |
| 8 | 136403197326 | 17.8 | 918 | 147 | 710 | 1132 |  | supply_tier1.rs 601/721 |
| 9 | 136403197213 | 12.0 | 623 | 97 | 710 | 740 |  | twin_tier2.rs 364/440 |
| 10 | 136403197309 | 14.1 | 600 | 243 | 710 | 797 |  | outline_tier1.rs 184/251 |
| 11 | 136403197292 | 13.4 | 635 | 169 | 710 | 717 |  |  |

### run 44764528379 (head 8fdc9422, success)

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136389208386 | 15.6 | 661 | 277 | 710 | 925 |  | twin_tier2.rs 466/690 |
| 1 | 136389208543 | 18.3 | 720 | 377 | 710 | 754 |  |  |
| 2 | 136389208382 | 12.9 | 646 | 131 | 710 | 768 |  |  |
| 3 | 136389208467 | 16.3 | 784 | 192 | 710 | 718 | against_fab_tier2.rs 568/447 |  |
| 4 | 136389208317 | 16.6 | 772 | 226 | 710 | 754 | against_fab_tier2.rs 550/421 | twin_tier2.rs 90/152 |
| 5 | 136389208624 | 17.6 | 878 | 179 | 710 | 864 |  |  |
| 6 | 136389208401 | 13.3 | 525 | 274 | 710 | 898 |  | twin_tier2.rs 44/119; verify_2k2_tier2.rs 323/537 |
| 7 | 136389208314 | 13.2 | 649 | 146 | 710 | 770 |  |  |
| 8 | 136389208376 | 17.1 | 863 | 166 | 710 | 1132 |  | supply_tier1.rs 474/721 |
| 9 | 136389208357 | 13.9 | 614 | 219 | 710 | 740 |  | twin_tier2.rs 340/440 |
| 10 | 136389208408 | 18.0 | 782 | 297 | 710 | 797 |  |  |
| 11 | 136389208453 | 13.9 | 654 | 180 | 710 | 717 |  |  |

### colocated blocks: every `finished in` against the row

| block | row | samples (s, run/shard) | worst | row / worst |
|---|---|---|---|---|
| tests/attribute_seeded.rs | 526 | 370.34 (44775091139/4); 536.85 (44774672284/4); 416.12 (44774205804/4); 380.69 (44768742667/5); 560.13 (44764528379/5) | 560.13 (44764528379 job 136389208624) | 0.94 |
| tests/campaign_tier2.rs | 83 | 58.26 (44775091139/10); 59.57 (44774672284/10); 75.73 (44774205804/10); 82.06 (44768742667/2); 54.02 (44764528379/2) | 82.06 (44768742667 job 136403197318) | 1.01 |
| tests/four_layer_fab_tier2.rs | 625 | 732.88 (44775091139/2); 573.58 (44774672284/2); 542.50 (44774205804/2); 629.56 (44768742667/1); 629.96 (44764528379/1) | 732.88 (44775091139 job 136425978348) | 0.85 |
| tests/outline_tier1.rs | 251 | 256.13 (44775091139/0); 248.47 (44774672284/0); 150.30 (44774205804/0); 183.69 (44768742667/10); 247.39 (44764528379/10) | 256.13 (44775091139 job 136425978460) | 0.98 |
| tests/power_board_tier1.rs | 331 | 205.84 (44775091139/9); 307.66 (44774672284/9); 254.45 (44774205804/9); 200.89 (44768742667/7); 313.77 (44764528379/7) | 313.77 (44764528379 job 136389208314) | 1.05 |
| tests/supply_tier1.rs | 721 | 651.00 (44775091139/1); 477.40 (44774672284/1); 460.64 (44774205804/1); 600.52 (44768742667/8); 473.86 (44764528379/8) | 651.00 (44775091139 job 136425978404) | 1.11 |
| tests/verify_47k_tier2.rs | 220 | 188.42 (44775091139/11); 270.78 (44774672284/11); 268.97 (44774205804/11); 244.40 (44768742667/11); 251.46 (44764528379/11) | 270.78 (44774672284 job 136424625516) | 0.81 |
| tests/via_ring_tier1.rs | 61 | 48.33 (44775091139/3); 62.90 (44774672284/3); 49.99 (44774205804/3); 63.15 (44768742667/3); 64.45 (44764528379/3) | 64.45 (44764528379 job 136389208467) | 0.95 |

### tests: every test with a sample of 30 s or more

`solo` is an alone time; `lock` and `ub` are candidates only (see the header).

| binary | test | row | worst solo | worst lock/ub | samples | verdict |
|---|---|---|---|---|---|---|
| tests/against_fab_tier2.rs | verify_against_fab_names_every_changed_and_extra_file | 446.7 | 588.06 | - | 5 | holds |
| tests/against_fab_tier2.rs | verify_against_fab_finds_the_package_it_wrote_identical | 420.7 | 585.18 | - | 5 | lags (solo) |
| tests/verify_2k2_tier2.rs | verify_fab_out_writes_the_package_its_verdict_names_at_any_path | 457.8 | 548.29 | - | 5 | holds |
| tests/verify_2k2_tier2.rs | verify_binds_the_reference_and_two_runs_print_the_same_bytes | 537.17 | 495.21 | - | 5 | holds |
| tests/board_tier2.rs | every_admitted_identifier_character_survives_every_output | 265.85 | - | 276.53 | 5 | holds |
| tests/twin_tier2.rs | battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats | 288.767 | - | 244.16 | 5 | holds |
| tests/twin_tier2.rs | kani_proves_every_part_of_both_motor_fixtures | 410.23 | - | 213.16 | 5 | holds |
| tests/board_tier2.rs | verify_names_what_judged_nothing_on_the_motor_driver | 204.8 | - | 196.36 | 5 | holds |
| tests/escape_tier1.rs | renamings_10_to_19_route_d3_to_its_own_copper | 169.1 | 158.12 | 170.63 | 5 | holds |
| tests/escape_tier1.rs | renamings_0_to_9_route_d3_to_its_own_copper | 158.4 | 170.52 | - | 5 | holds |
| tests/twin_tier2.rs | on_a_bus_with_two_devices_each_read_is_witnessed | 216.4 | - | 162.37 | 5 | holds |
| tests/board_tier2.rs | the_generated_board_proves | 109.2 | 130.76 | 160.79 | 5 | candidate (solo holds) |
| tests/board_tier2.rs | each_size_sample_passes_every_gate_or_is_refused_by_name | 156.94 | - | 151.46 | 5 | holds |
| tests/twin_tier2.rs | the_console_sends_its_pattern_and_echoes_its_frame | 4.5 | - | 118.83 | 5 | candidate: alone run |
| tests/twin_tier2.rs | m1_a_core_that_slept_without_its_boot_word_is_cpu_halted | 14.5 | - | 115.88 | 5 | candidate: alone run |
| tests/twin_tier2.rs | motor_output_dead_leaves_its_pin_silent_and_reads_its_wrong_af | 12.3 | - | 115.09 | 5 | candidate: alone run |
| tests/twin_tier2.rs | kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check | 16.35 | - | 108.58 | 5 | candidate: alone run |
| tests/board_tier2.rs | every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict | 49.95 | - | 106.96 | 5 | candidate: alone run |
| tests/board_tier2.rs | the_power_fixture_passes_erc_drc_its_loop_rules_and_extract | 99.8 | - | 96.36 | 5 | holds |
| tests/board_ui.rs | misuse_does_not_compile_and_the_driver_does | 73.6 | 85.51 | - | 5 | holds |
| tests/board_tier2.rs | the_own_parts_fixture_s_transfers_prove_within_its_scripts | 77.5 | 77.37 | 78.22 | 5 | holds |
| tests/twin_tier2.rs | m2_a_dma_start_of_cen_fails_closed_by_name | 6.1 | - | 66.14 | 5 | candidate: alone run |
| tests/hot_plug_tier1.rs | the_declared_bulk_capacitor_is_c1_positive_on_vbat_on_both_power_boards | 61.2 | 64.97 | 49.58 | 5 | holds |
| tests/twin_tier2.rs | the_sentinel_02_shaped_board_is_witnessed_whole | 24.7 | - | 57.00 | 5 | candidate: alone run |
| unittests src/lib.rs | supply::tests::a_relieved_pad_joins_no_plane | 57.43 | - | 50.30 | 5 | holds |
| tests/twin_tier2.rs | a_firmware_that_skips_the_read_or_its_record_cannot_pass | 11.93 | - | 49.96 | 5 | candidate: alone run |
| tests/twin_tier2.rs | adv4_2_an_echo_that_hardly_waits_after_the_first_byte_fails | 5.63 | - | 48.20 | 5 | candidate: alone run |
| tests/solve_cli.rs | a_power_tree_solves_into_the_topology_and_emits_its_board | 37.6 | 45.59 | 45.37 | 5 | holds |
| tests/battery_sag_tier1.rs | the_power_fixture_s_vdd_5v_stays_unmet_with_its_reason | 30.11 | 30.21 | 45.56 | 5 | candidate (solo holds) |
| tests/board_tier2.rs | the_sentinel_03_fixture_passes_every_gate_inside_its_bound | 40.7 | - | 45.21 | 5 | holds |
| tests/board_tier2.rs | the_imu_fixture_s_transfers_prove_within_its_scripts | 35.0 | 45.06 | 42.21 | 5 | holds |
| tests/board_tier2.rs | a_write_past_the_block_fails | 24.4 | - | 43.67 | 5 | candidate: alone run |
| unittests src/lib.rs | supply::tests::a_through_hole_pad_carries_the_current_as_a_barrel | 26.6 | - | 43.61 | 5 | candidate: alone run |
| tests/twin_tier2.rs | battery_sag::the_power_fixture_s_campaign_meets_every_expectation_replays_and_repeats | 88.251 | - | 40.19 | 5 | holds |
| unittests src/lib.rs | supply::tests::a_path_through_two_vias_is_bounded_above_its_floor | 23.5 | - | 39.28 | 5 | candidate: alone run |
| tests/escape_tier1.rs | twenty_renamings_route_d1_to_its_own_copper | 37.5 | - | 38.47 | 5 | holds |
| unittests src/lib.rs | supply::tests::a_declared_dielectric_maximum_bounds_the_barrel_row | 31.8 | - | 38.26 | 5 | holds |
| tests/twin_tier2.rs | the_fixture_s_campaign_archives_each_configuration_and_replays | 11.74 | - | 35.68 | 5 | candidate: alone run |
| tests/adversarial_1301_tools.rs | attack_wide_references_leave_the_emitted_board_clean | 33.5 | 33.74 | - | 5 | holds |
| tests/board_tier2.rs | the_full_power_fixture_routes_and_passes_every_gate_it_reaches | 27.9 | - | 33.04 | 5 | holds |
| tests/board_tier2.rs | a_wrong_alternate_function_fails | 29.11 | - | 32.35 | 5 | holds |
| tests/boot_tier1.rs | a_boot_past_the_wall_bound_fails_and_names_it | 32.4 | - | 32.22 | 5 | holds |
| tests/escape_tier1.rs | twenty_renamings_route_the_reference_to_its_own_copper | 24.0 | - | 31.50 | 5 | holds |
| tests/twin_tier2.rs | an_aborted_machine_returns_inside_the_bound_as_cpu_aborted | 30.7 | - | 31.41 | 5 | holds |
| tests/adversarial_1301.rs | attack_a_wide_reference_stays_clear_of_every_other_part | 30.7 | 31.21 | - | 5 | holds |

### The two lock-serialised binaries: rows at `c1012a2d` against `finished in`, per run and shard

`spike/1913/locked.py`: for every (run, shard) that ran `twin_tier2` or `board_tier2`, the sum of the rows of the tests it ran, against the binary's libtest `finished in`. A positive difference over-weighs the shard; a negative one under-weighs it.

| run | shard | binary | tests | rows s | finished in s | rows - measured s |
|---|---|---|---|---|---|---|
| 44775091139 | 0 | board_tier2.rs | 2 | 38 | 43 | -4 |
| 44775091139 | 0 | twin_tier2.rs | 8 | 277 | 72 | +204 |
| 44775091139 | 1 | twin_tier2.rs | 4 | 23 | 21 | +2 |
| 44775091139 | 2 | board_tier2.rs | 2 | 7 | 9 | -2 |
| 44775091139 | 2 | twin_tier2.rs | 3 | 14 | 16 | -2 |
| 44775091139 | 3 | board_tier2.rs | 2 | 5 | 3 | +2 |
| 44775091139 | 3 | twin_tier2.rs | 7 | 79 | 44 | +34 |
| 44775091139 | 4 | board_tier2.rs | 1 | 35 | 33 | +2 |
| 44775091139 | 4 | twin_tier2.rs | 13 | 127 | 54 | +72 |
| 44775091139 | 5 | board_tier2.rs | 2 | 150 | 106 | +44 |
| 44775091139 | 5 | twin_tier2.rs | 7 | 49 | 52 | -3 |
| 44775091139 | 6 | board_tier2.rs | 2 | 115 | 136 | -21 |
| 44775091139 | 6 | twin_tier2.rs | 10 | 63 | 52 | +11 |
| 44775091139 | 7 | board_tier2.rs | 3 | 170 | 165 | +5 |
| 44775091139 | 7 | twin_tier2.rs | 3 | 26 | 19 | +7 |
| 44775091139 | 8 | board_tier2.rs | 1 | 1 | 1 | +0 |
| 44775091139 | 8 | twin_tier2.rs | 12 | 565 | 486 | +80 |
| 44775091139 | 9 | board_tier2.rs | 1 | 78 | 59 | +19 |
| 44775091139 | 9 | twin_tier2.rs | 11 | 119 | 47 | +71 |
| 44775091139 | 10 | board_tier2.rs | 6 | 296 | 254 | +42 |
| 44775091139 | 10 | twin_tier2.rs | 9 | 375 | 256 | +119 |
| 44775091139 | 11 | board_tier2.rs | 2 | 307 | 219 | +88 |
| 44775091139 | 11 | twin_tier2.rs | 8 | 140 | 56 | +83 |
| 44774672284 | 0 | board_tier2.rs | 2 | 38 | 39 | -1 |
| 44774672284 | 0 | twin_tier2.rs | 8 | 277 | 69 | +207 |
| 44774672284 | 1 | twin_tier2.rs | 4 | 23 | 30 | -6 |
| 44774672284 | 2 | board_tier2.rs | 2 | 7 | 6 | +0 |
| 44774672284 | 2 | twin_tier2.rs | 3 | 14 | 11 | +3 |
| 44774672284 | 3 | board_tier2.rs | 2 | 5 | 5 | +1 |
| 44774672284 | 3 | twin_tier2.rs | 7 | 79 | 68 | +10 |
| 44774672284 | 4 | board_tier2.rs | 1 | 35 | 45 | -10 |
| 44774672284 | 4 | twin_tier2.rs | 13 | 127 | 96 | +31 |
| 44774672284 | 5 | board_tier2.rs | 2 | 150 | 64 | +85 |
| 44774672284 | 5 | twin_tier2.rs | 7 | 49 | 28 | +21 |
| 44774672284 | 6 | board_tier2.rs | 2 | 115 | 167 | -52 |
| 44774672284 | 6 | twin_tier2.rs | 10 | 63 | 68 | -6 |
| 44774672284 | 7 | board_tier2.rs | 3 | 170 | 120 | +50 |
| 44774672284 | 7 | twin_tier2.rs | 3 | 26 | 13 | +13 |
| 44774672284 | 8 | board_tier2.rs | 1 | 1 | 1 | +0 |
| 44774672284 | 8 | twin_tier2.rs | 12 | 565 | 561 | +4 |
| 44774672284 | 9 | board_tier2.rs | 1 | 78 | 77 | +0 |
| 44774672284 | 9 | twin_tier2.rs | 11 | 119 | 77 | +41 |
| 44774672284 | 10 | board_tier2.rs | 6 | 296 | 250 | +47 |
| 44774672284 | 10 | twin_tier2.rs | 9 | 375 | 257 | +118 |
| 44774672284 | 11 | board_tier2.rs | 2 | 307 | 314 | -7 |
| 44774672284 | 11 | twin_tier2.rs | 8 | 140 | 95 | +45 |
| 44774205804 | 0 | board_tier2.rs | 2 | 38 | 25 | +14 |
| 44774205804 | 0 | twin_tier2.rs | 8 | 277 | 38 | +238 |
| 44774205804 | 1 | twin_tier2.rs | 4 | 23 | 31 | -8 |
| 44774205804 | 2 | board_tier2.rs | 2 | 7 | 5 | +2 |
| 44774205804 | 2 | twin_tier2.rs | 3 | 14 | 10 | +4 |
| 44774205804 | 3 | board_tier2.rs | 2 | 5 | 4 | +2 |
| 44774205804 | 3 | twin_tier2.rs | 7 | 79 | 46 | +33 |
| 44774205804 | 4 | board_tier2.rs | 1 | 35 | 33 | +2 |
| 44774205804 | 4 | twin_tier2.rs | 13 | 127 | 64 | +62 |
| 44774205804 | 5 | board_tier2.rs | 2 | 150 | 114 | +35 |
| 44774205804 | 5 | twin_tier2.rs | 7 | 49 | 48 | +1 |
| 44774205804 | 6 | board_tier2.rs | 2 | 115 | 146 | -31 |
| 44774205804 | 6 | twin_tier2.rs | 10 | 63 | 53 | +9 |
| 44774205804 | 7 | board_tier2.rs | 3 | 170 | 163 | +7 |
| 44774205804 | 7 | twin_tier2.rs | 3 | 26 | 19 | +7 |
| 44774205804 | 8 | board_tier2.rs | 1 | 1 | 1 | +0 |
| 44774205804 | 8 | twin_tier2.rs | 12 | 565 | 629 | -64 |
| 44774205804 | 9 | board_tier2.rs | 1 | 78 | 71 | +6 |
| 44774205804 | 9 | twin_tier2.rs | 11 | 119 | 58 | +61 |
| 44774205804 | 10 | board_tier2.rs | 6 | 296 | 313 | -16 |
| 44774205804 | 10 | twin_tier2.rs | 9 | 375 | 326 | +49 |
| 44774205804 | 11 | board_tier2.rs | 2 | 307 | 314 | -7 |
| 44774205804 | 11 | twin_tier2.rs | 8 | 140 | 90 | +50 |
| 44768742667 | 0 | board_tier2.rs | 3 | 137 | 162 | -25 |
| 44768742667 | 0 | twin_tier2.rs | 9 | 690 | 574 | +116 |
| 44768742667 | 1 | board_tier2.rs | 1 | 6 | 7 | -1 |
| 44768742667 | 1 | twin_tier2.rs | 6 | 40 | 33 | +7 |
| 44768742667 | 2 | board_tier2.rs | 2 | 34 | 33 | +0 |
| 44768742667 | 2 | twin_tier2.rs | 5 | 48 | 38 | +10 |
| 44768742667 | 3 | twin_tier2.rs | 8 | 48 | 53 | -5 |
| 44768742667 | 4 | board_tier2.rs | 3 | 23 | 24 | -1 |
| 44768742667 | 4 | twin_tier2.rs | 6 | 152 | 114 | +37 |
| 44768742667 | 5 | board_tier2.rs | 2 | 128 | 99 | +29 |
| 44768742667 | 5 | twin_tier2.rs | 9 | 101 | 60 | +41 |
| 44768742667 | 6 | board_tier2.rs | 1 | 109 | 131 | -22 |
| 44768742667 | 6 | twin_tier2.rs | 11 | 119 | 60 | +60 |
| 44768742667 | 7 | board_tier2.rs | 2 | 207 | 95 | +112 |
| 44768742667 | 7 | twin_tier2.rs | 8 | 91 | 35 | +56 |
| 44768742667 | 8 | board_tier2.rs | 2 | 50 | 44 | +7 |
| 44768742667 | 8 | twin_tier2.rs | 9 | 72 | 46 | +26 |
| 44768742667 | 9 | board_tier2.rs | 1 | 1 | 1 | -0 |
| 44768742667 | 9 | twin_tier2.rs | 14 | 440 | 364 | +75 |
| 44768742667 | 10 | board_tier2.rs | 3 | 279 | 225 | +54 |
| 44768742667 | 10 | twin_tier2.rs | 5 | 37 | 30 | +7 |
| 44768742667 | 11 | board_tier2.rs | 4 | 227 | 191 | +37 |
| 44768742667 | 11 | twin_tier2.rs | 5 | 18 | 20 | -2 |
| 44764528379 | 0 | board_tier2.rs | 3 | 137 | 142 | -5 |
| 44764528379 | 0 | twin_tier2.rs | 9 | 690 | 466 | +224 |
| 44764528379 | 1 | board_tier2.rs | 1 | 6 | 7 | -1 |
| 44764528379 | 1 | twin_tier2.rs | 6 | 40 | 33 | +7 |
| 44764528379 | 2 | board_tier2.rs | 2 | 34 | 27 | +6 |
| 44764528379 | 2 | twin_tier2.rs | 5 | 48 | 25 | +23 |
| 44764528379 | 3 | twin_tier2.rs | 8 | 48 | 54 | -6 |
| 44764528379 | 4 | board_tier2.rs | 3 | 23 | 24 | -1 |
| 44764528379 | 4 | twin_tier2.rs | 6 | 152 | 90 | +62 |
| 44764528379 | 5 | board_tier2.rs | 2 | 128 | 128 | -1 |
| 44764528379 | 5 | twin_tier2.rs | 9 | 101 | 93 | +8 |
| 44764528379 | 6 | board_tier2.rs | 1 | 109 | 107 | +2 |
| 44764528379 | 6 | twin_tier2.rs | 11 | 119 | 44 | +75 |
| 44764528379 | 7 | board_tier2.rs | 2 | 207 | 151 | +55 |
| 44764528379 | 7 | twin_tier2.rs | 8 | 91 | 53 | +38 |
| 44764528379 | 8 | board_tier2.rs | 2 | 50 | 53 | -3 |
| 44764528379 | 8 | twin_tier2.rs | 9 | 72 | 58 | +14 |
| 44764528379 | 9 | board_tier2.rs | 1 | 1 | 1 | -0 |
| 44764528379 | 9 | twin_tier2.rs | 14 | 440 | 340 | +100 |
| 44764528379 | 10 | board_tier2.rs | 3 | 279 | 290 | -11 |
| 44764528379 | 10 | twin_tier2.rs | 5 | 37 | 45 | -8 |
| 44764528379 | 11 | board_tier2.rs | 4 | 227 | 193 | +34 |
| 44764528379 | 11 | twin_tier2.rs | 5 | 18 | 20 | -3 |

tests/twin_tier2.rs: 60 samples; rows 9279 s against 6713 s measured; mean |error| 46 s; most under-weighed -64 s (44774205804 shard 8)
tests/board_tier2.rs: 55 samples; rows 6008 s against 5439 s measured; mean |error| 18 s; most under-weighed -52 s (44774672284 shard 6)

**What it says** [measured]:
- `twin_tier2`'s rows over-weigh it by 38 % (9,279 s of rows against 6,713 s run, over 60 shard samples). Shard 0 of the three runs on `1912`'s rows planned 277 s of `twin_tier2` and ran 38–72 s.
- `board_tier2`'s rows are 10 % over (6,008 s against 5,439 s).
- No shard ran either binary more than 64 s over its rows.
- **The two under-rows:** `four_layer_fab_tier2` ran 542.50–732.88 s against its 625 row; `verify_47k_tier2` ran 188.42–270.78 s against 220. `1912` measured 740.63 and 277.29.
- **Outside this order's rows** (moved under the architect's widening, §2): `against_fab_tier2`'s `verify_against_fab_finds_the_package_it_wrote_identical` ran 585.18 s `solo` against its 420.7 row, more than 25 % under. `verify_2k2_tier2`'s `verify_binds_…` row of 537.17 is above every sample (323–495 s), and `lib.rs`'s `supply::` rows sum to 132 s against 38–50 s run.
- **The spread on job minutes** (slowest shard less fastest): 6.6, 7.0, 15.5, 8.1 and 5.3 min (max shard 17.3, 18.0, 27.6, 17.8, 18.3). 15.5 and 27.6 are shard 8 of 44774205804, whose run step was 851 s and its overhead 804 s.

## 2. The rows set (§1 steps 2 and 3)

**The throwaway** (`c9ac0298`, reverted in `9c28088f`; `ci/shard.sh` is byte-equal to `main`'s): run 44787324803, green. After each shard's own tests, every `twin_tier2` and `board_tier2` test ran alone (one invocation, one thread, `--exact`) on **two** shards, so each row is the worse of two runners, not one. `four_layer_fab_tier2` and `verify_47k_tier2` each ran once more as a whole binary (`cargo test --workspace --test X`). That is 240 invocations, read by `spike/1913/alone_times.py`; every name gave two samples [measured]. The runners: shards 0, 3, 6 and 8 Xeon 6975P-C; 1 Xeon Platinum 8370C; 2, 5 and 10 EPYC 7763; 4 and 11 EPYC 9V45; 7 and 9 EPYC 9V74.

**The rules** (`spike/1913/refit.py`):
- **A `twin_tier2` or `board_tier2` test:** the worse of its two alone samples, 3 decimals, lowered or raised. Never from a shared-process run, because both binaries take their lock per call (WO §1 step 3).
- **The two blocks:** the worst `finished in` over the five runs, the throwaway's normal and whole-binary passes, and `1912`'s cited 740.63 and 277.29, rounded up. Never lowered.
- **Any other row** (the architect's widening, bus 2026-11-20 18:09Z): more than 25 % off its worst `solo` sample over the five runs and the throwaway's normal pass, set to it, lowered or raised. A row with no `solo` sample is not moved. `verify_2k2_tier2`'s `verify_binds_the_reference_and_two_runs_print_the_same_bytes` (537.17) is 8 % above its worst `solo` (495.21), inside 25 %, so it stays.

**Calibration** [measured]: the alone pass runs after the shard's own tests, so a warm cache could read low. Two tests also have in-shard `solo` samples. `the_generated_board_proves` read 93.26 and 127.01 s alone against a worst `solo` of 130.76; `the_imu_fixture_s_transfers_prove_within_its_scripts` read 27.76 and 39.45 against 45.06. The worse of two alone samples is 3–12 % under the worst in-shard sample, which matches runner speed (the fast samples are on Xeon 6975P-C and EPYC 9V45), not a warm cache.

**Totals:** `twin_tier2` 1,855.8 → 1,488.3 s (51 lowered, 44 raised); `board_tier2` 1,201.7 → 1,195.1 s (16 lowered, 7 raised, 1 unchanged); 28 other rows; `four_layer_fab_tier2` 625 → 741; `verify_47k_tier2` 220 → 278.

### The back-test (the §5 risk, offline)

`spike/1913/locked.py` over the six runs (the five and the throwaway's normal pass), for every (run, shard) that ran `twin_tier2` or `board_tier2`: the new rows against the binary's `finished in` [measured, then computed].

| binary | samples | rows before / measured | rows after / measured | mean abs. error before → after | most under-weighed after |
|---|---|---|---|---|---|
| `twin_tier2` | 72 | 11,135 / 7,976 s | 8,930 / 7,976 s | 47 → 23 s | −89 s (44787324803 shard 10, EPYC 7763) |
| `board_tier2` | 66 | 7,210 / 6,482 s | 7,171 / 6,482 s | 19 → 16 s | −41 s (44787324803 shard 10, EPYC 7763) |

The rows still sit above what ran, by 12 % (`twin_tier2`) and 11 % (`board_tier2`). An alone time pays work that overlaps in a shard (a test's own generate and compile run while another holds the lock). That is the WO's rule (an alone time), not an error. The worst under-weighing is 89 s, on the slowest runner, against a plan of 816 s.

### The plan

`ci/shard.sh assign 12` on the throwaway's `entries.tsv` with the new rows: every shard planned at 815–816 s (before: 823), and the guard passes (2,682 names, 2,689 occurrences).

### Every row moved

The worse sample is listed first. The two blocks' rows are their worst `finished in`; a row's job is in its citation.

| binary | test | row before | alone samples (s, run/shard, CPU) | row after | change |
|---|---|---|---|---|---|
| board_tier2.rs | a_cargo_configuration_another_uid_plants_in_tmp_is_not_read | 0.7 | 0.65 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 0.56 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 0.65 | lowered |
| board_tier2.rs | a_missing_clock_enable_fails | 9.29 | 8.80 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 5.86 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 8.8 | lowered |
| board_tier2.rs | a_vacuous_harness_fails | 6.2 | 8.93 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 7.82 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 8.93 | raised |
| board_tier2.rs | a_write_past_the_block_fails | 24.4 | 45.32 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 28.78 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 45.32 | raised |
| board_tier2.rs | a_wrong_alternate_function_fails | 29.11 | 29.11 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 27.95 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 29.11 | same |
| board_tier2.rs | as_root_a_third_uid_cannot_change_what_build_reads | 0.9 | 0.58 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 0.48 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 0.58 | lowered |
| board_tier2.rs | build_gives_one_sha256_at_two_paths | 0.9 | 0.72 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor); 0.71 (44787324803 shard 0 job 136468155823, Intel(R) Xeon(R) 6975P-C) | 0.72 | lowered |
| board_tier2.rs | connectors_pass_erc_and_drc_with_parity_at_two_and_four_layers | 7.6 | 7.37 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 6.61 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 7.37 | lowered |
| board_tier2.rs | each_size_sample_passes_every_gate_or_is_refused_by_name | 156.94 | 139.76 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 87.71 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 139.76 | lowered |
| board_tier2.rs | every_admitted_identifier_character_survives_every_output | 265.85 | 270.81 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 199.95 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 270.81 | raised |
| board_tier2.rs | every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict | 49.95 | 42.99 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 37.24 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 42.99 | lowered |
| board_tier2.rs | prove_with_spec_tells_a_hand_written_harness_from_the_generated_one | 16.8 | 15.91 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 11.65 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 15.91 | lowered |
| board_tier2.rs | the_bench_board_passes_every_gate_at_two_and_four_layers | 12.5 | 11.83 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 9.35 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 11.83 | lowered |
| board_tier2.rs | the_full_power_fixture_routes_and_passes_every_gate_it_reaches | 27.9 | 31.74 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 23.81 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 31.74 | raised |
| board_tier2.rs | the_generated_board_proves | 109.2 | 127.01 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 93.26 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 127.01 | raised |
| board_tier2.rs | the_imu_fixture_s_transfers_prove_within_its_scripts | 35 | 39.45 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 27.76 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 39.45 | raised |
| board_tier2.rs | the_ordered_board_passes_every_board_gate_with_no_finding | 5.6 | 5.52 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 4.75 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 5.52 | lowered |
| board_tier2.rs | the_own_parts_fixture_passes_erc_drc_with_parity_and_extract | 9.8 | 7.74 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 7.48 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor) | 7.74 | lowered |
| board_tier2.rs | the_own_parts_fixture_s_transfers_prove_within_its_scripts | 77.5 | 72.72 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 70.59 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 72.72 | lowered |
| board_tier2.rs | the_power_fixture_passes_erc_drc_its_loop_rules_and_extract | 99.8 | 97.06 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 76.94 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 97.06 | lowered |
| board_tier2.rs | the_reference_passes_the_support_gate_and_a_capacitor_moved_off_fails | 5.7 | 4.53 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C); 3.65 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 4.53 | lowered |
| board_tier2.rs | the_sentinel_03_fixture_passes_every_gate_inside_its_bound | 40.7 | 45.06 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 27.41 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 45.06 | raised |
| board_tier2.rs | verify_names_what_judged_nothing_on_the_motor_driver | 204.8 | 177.49 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 116.71 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 177.49 | lowered |
| board_tier2.rs | verify_refuses_a_board_swapped_before_extract | 4.55 | 3.99 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 3.79 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 3.99 | lowered |
| twin_tier2.rs | a2_3_a_9600_console_s_wire_time_is_judged_from_its_boot_word | 14.7 | 23.31 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 21.37 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 23.31 | raised |
| twin_tier2.rs | a_boot_too_short_to_reach_the_fixed_point_fails | 5.1 | 4.99 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 3.62 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 4.99 | lowered |
| twin_tier2.rs | a_core_that_leaves_the_firmware_after_the_boot_word_fails | 4.6 | 4.48 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 4.15 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 4.48 | lowered |
| twin_tier2.rs | a_device_named_for_a_renode_instance_or_keyword_breaks_the_load | 3.04 | 3.54 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 3.04 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 3.54 | raised |
| twin_tier2.rs | a_fault_on_a_bus_with_two_devices_is_not_masked_and_a_wrong_address_still_is_software | 8.4 | 11.58 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 11.37 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 11.58 | raised |
| twin_tier2.rs | a_firmware_parked_in_its_fault_handler_fails_naming_vectactive | 6 | 5.44 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 4.40 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.44 | lowered |
| twin_tier2.rs | a_firmware_that_rewrites_its_loop_in_flash_is_not_parked | 9.9 | 6.12 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C); 5.56 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 6.12 | lowered |
| twin_tier2.rs | a_firmware_that_skips_the_read_or_its_record_cannot_pass | 11.93 | 10.83 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 8.89 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 10.83 | lowered |
| twin_tier2.rs | a_firmware_that_writes_the_controller_registers_by_hand_fails_unwitnessed | 5.4 | 5.73 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 3.35 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 5.73 | raised |
| twin_tier2.rs | a_firmware_with_no_vector_table_at_the_boot_address_fails | 4.1 | 5.33 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 3.70 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.33 | raised |
| twin_tier2.rs | a_foreign_firmware_that_erases_its_own_sector_fails_though_its_hash_holds | 5.3 | 5.01 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 3.73 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 5.01 | lowered |
| twin_tier2.rs | a_foreign_firmware_that_erases_outside_its_own_flash_is_no_finding | 5.2 | 3.45 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C); 2.97 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 3.45 | lowered |
| twin_tier2.rs | a_foreign_firmware_that_stores_into_its_own_flash_fails_flash_overwritten | 4.66 | 3.77 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 3.64 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 3.77 | lowered |
| twin_tier2.rs | a_forged_record_without_a_transaction_fails_unwitnessed | 7.7 | 4.40 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 3.84 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 4.4 | lowered |
| twin_tier2.rs | a_forged_spi_record_fails_unwitnessed | 7.39 | 8.56 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 8.08 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 8.56 | raised |
| twin_tier2.rs | a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed | 7.6 | 11.15 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 8.49 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 11.15 | raised |
| twin_tier2.rs | a_locked_access_late_in_a_flood_reaches_the_judge_whole | 8.7 | 10.51 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 5.60 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 10.51 | raised |
| twin_tier2.rs | a_pattern_on_the_wrong_port_fails | 7.5 | 7.30 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 4.87 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 7.3 | lowered |
| twin_tier2.rs | a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record | 18 | 6.50 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C); 6.38 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 6.5 | lowered |
| twin_tier2.rs | a_read_no_model_answers_fails_as_a_timeout_at_txis | 4.1 | 4.45 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 4.05 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 4.45 | raised |
| twin_tier2.rs | a_seeded_barometer_fault_is_named_by_its_step | 7.2 | 9.10 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 5.43 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 9.1 | raised |
| twin_tier2.rs | a_seeded_imu_fault_is_named_by_its_step | 7.6 | 7.90 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.69 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 7.9 | raised |
| twin_tier2.rs | a_step_sent_to_another_devices_address_that_answers_is_the_softwares | 3.9 | 4.84 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 4.55 (44787324803 shard 0 job 136468155823, Intel(R) Xeon(R) 6975P-C) | 4.84 | raised |
| twin_tier2.rs | a_tdr_store_with_ue_clear_sends_no_pattern | 6.22 | 6.18 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 4.19 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 6.18 | lowered |
| twin_tier2.rs | a_value_one_bit_off_fails_as_an_identity_mismatch | 3.4 | 6.18 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 4.01 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 6.18 | raised |
| twin_tier2.rs | a_witness_line_sent_through_every_firmware_channel_never_reaches_the_judge | 4.9 | 6.05 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 3.21 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 6.05 | raised |
| twin_tier2.rs | adv2_1_a_gpio_reset_through_the_rcc_fails | 5.2 | 5.69 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 4.08 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 5.69 | raised |
| twin_tier2.rs | adv2_2_an_open_drain_pulled_down_tx_fails | 5 | 5.42 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.36 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.42 | raised |
| twin_tier2.rs | adv2_3_a_software_reset_at_start_fails | 5 | 5.18 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.15 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.18 | raised |
| twin_tier2.rs | adv2_4_an_unaligned_store_into_afrh_fails | 5.3 | 4.45 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 4.26 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 4.45 | lowered |
| twin_tier2.rs | adv2_5_a_locked_tx_pin_fails | 5.3 | 6.06 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 5.67 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor) | 6.06 | raised |
| twin_tier2.rs | adv2_6_an_open_drain_tx_read_modify_written_fails | 5.4 | 4.05 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C); 3.87 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 4.05 | lowered |
| twin_tier2.rs | adv3_1_an_apb2_prescaler_after_the_bring_up_fails | 8.1 | 4.20 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 4.02 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 4.2 | lowered |
| twin_tier2.rs | adv3_1b_a_usart_kernel_clock_selection_after_the_bring_up_fails | 5.6 | 5.71 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 4.37 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.71 | raised |
| twin_tier2.rs | adv3_2_sbus_s_cr2_read_back_and_xored_fails | 6.5 | 5.43 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 3.91 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 5.43 | lowered |
| twin_tier2.rs | adv3_4_a_tx_pin_unrouted_between_tdr_writes_fails | 5.2 | 4.52 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 4.20 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 4.52 | lowered |
| twin_tier2.rs | adv3_5_an_rx_pin_unrouted_after_the_first_byte_fails | 4.5 | 5.69 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 3.86 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.69 | raised |
| twin_tier2.rs | adv4_2_an_echo_that_hardly_waits_after_the_first_byte_fails | 5.63 | 5.86 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 3.84 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.86 | raised |
| twin_tier2.rs | adv_1_a_tx_pin_routed_only_after_the_pattern_fails | 5.5 | 5.39 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.00 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 5.39 | lowered |
| twin_tier2.rs | adv_1b_a_tx_pin_unrouted_after_the_first_byte_fails | 5.7 | 5.93 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 3.35 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 5.93 | raised |
| twin_tier2.rs | adv_2_a_presc_write_in_the_bring_up_fails | 12.3 | 4.22 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 3.86 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 4.22 | lowered |
| twin_tier2.rs | adv_2b_a_presc_write_from_main_after_the_bring_up_fails | 4.4 | 5.82 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 3.36 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 5.82 | raised |
| twin_tier2.rs | adv_3_sbus_s_m0_cleared_before_ue_fails | 6.3 | 5.28 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 4.79 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 5.28 | lowered |
| twin_tier2.rs | adv_4_sbus_s_inversion_written_after_ue_fails | 6.7 | 5.50 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 4.63 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.5 | lowered |
| twin_tier2.rs | adv_5_a_pattern_on_a_usart_no_port_names_is_forged | 7.7 | 4.01 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C); 3.89 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 4.01 | lowered |
| twin_tier2.rs | adversarial_1_a_timer_at_another_rate_with_the_claim_kept_fails | 10.3 | 5.21 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.34 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 5.21 | lowered |
| twin_tier2.rs | adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail | 6.13 | 10.89 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 7.88 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 10.89 | raised |
| twin_tier2.rs | an_aborted_machine_returns_inside_the_bound_as_cpu_aborted | 30.7 | 31.24 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 24.01 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 31.24 | raised |
| twin_tier2.rs | an_echo_the_firmware_never_received_is_forged | 4 | 5.61 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 4.23 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 5.61 | raised |
| twin_tier2.rs | an_interrupt_armed_after_the_boot_word_fails_naming_it | 13.6 | 8.11 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C); 7.65 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 8.11 | lowered |
| twin_tier2.rs | an_open_rx_line_is_detected_by_name_and_attributed_e3 | 9.07 | 10.53 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 9.70 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 10.53 | raised |
| twin_tier2.rs | battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats | 288.767 | 214.68 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 208.42 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 214.68 | lowered |
| twin_tier2.rs | battery_sag::the_power_fixture_s_campaign_meets_every_expectation_replays_and_repeats | 88.251 | 57.39 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C); 56.13 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 57.39 | lowered |
| twin_tier2.rs | bor_recovery::the_bmp581_extension_recovers_after_its_settle | 26.799 | 16.50 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 14.64 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 16.5 | lowered |
| twin_tier2.rs | bor_recovery::the_logger_recovers_with_its_first_devices_later_steps_witnessed | 22.935 | 18.09 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 15.10 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 18.09 | lowered |
| twin_tier2.rs | bor_recovery::the_sentinel_02_shape_recovers_after_its_imus_settle | 38.13 | 22.77 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 20.66 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 22.77 | lowered |
| twin_tier2.rs | every_bit_the_reference_writes_is_covered_by_a_ledger_row | 4.1 | 5.03 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 2.77 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 5.03 | raised |
| twin_tier2.rs | f2_a_late_dshot_anchor_fails_by_name | 9 | 13.45 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 9.02 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 13.45 | raised |
| twin_tier2.rs | h1a_a_pin_bit_banged_after_the_window_fails_by_name | 13.1 | 6.13 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 4.24 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 6.13 | lowered |
| twin_tier2.rs | h1b_a_16_bit_restart_after_the_window_fails_by_name | 6.4 | 6.78 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 4.54 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 6.78 | raised |
| twin_tier2.rs | h1c_a_dma_restart_after_the_window_fails_by_name | 6.2 | 5.10 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 5.08 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor) | 5.1 | lowered |
| twin_tier2.rs | kani_proves_every_part_of_both_motor_fixtures | 410.23 | 510.64 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 472.52 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 510.64 | raised |
| twin_tier2.rs | kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check | 16.35 | 12.63 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 10.00 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 12.63 | lowered |
| twin_tier2.rs | l1_a_pattern_sent_after_the_boot_word_never_counts_as_the_echo | 12.5 | 9.40 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 7.18 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 9.4 | lowered |
| twin_tier2.rs | l1_bytes_written_before_the_frame_never_count_as_its_echo | 10.4 | 8.08 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 8.07 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 8.08 | lowered |
| twin_tier2.rs | m1_a_core_that_slept_and_wrote_its_boot_word_is_the_ruled_disagreement | 5.5 | 4.67 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 4.40 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 4.67 | lowered |
| twin_tier2.rs | m1_a_core_that_slept_without_its_boot_word_is_cpu_halted | 14.5 | 12.01 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 6.73 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 12.01 | lowered |
| twin_tier2.rs | m2_a_16_bit_or_byte_store_of_cen_anchors_and_is_stopped | 21.8 | 12.82 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 12.43 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor) | 12.82 | lowered |
| twin_tier2.rs | m2_a_dma_start_of_cen_fails_closed_by_name | 6.1 | 6.24 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.43 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 6.24 | raised |
| twin_tier2.rs | motor_output_dead_leaves_its_pin_silent_and_reads_its_wrong_af | 12.3 | 4.04 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 3.20 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 4.04 | lowered |
| twin_tier2.rs | on_a_bus_with_two_devices_each_read_is_witnessed | 216.4 | 12.80 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 11.37 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 12.8 | lowered |
| twin_tier2.rs | p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes | 13.4 | 23.18 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 22.17 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor) | 23.18 | raised |
| twin_tier2.rs | renode_s_configuration_differs_from_its_default_in_synchronous_logging_and_virtual_time_alone | 8.62 | 6.22 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor); 5.93 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 6.22 | lowered |
| twin_tier2.rs | renode_s_files_are_the_pinned_ones_and_declare_the_stock_table | 0.1 | 0.00 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C); 0.00 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor) | 0 | lowered |
| twin_tier2.rs | the_console_s_rx_on_the_wrong_af_keeps_the_pattern_and_loses_the_echo | 5.4 | 5.52 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.21 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.52 | raised |
| twin_tier2.rs | the_console_sends_its_pattern_and_echoes_its_frame | 4.5 | 5.94 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 3.73 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 5.94 | raised |
| twin_tier2.rs | the_dshot600_fixture_s_campaign_passes_inside_its_window | 13.43 | 13.37 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 12.60 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 13.37 | lowered |
| twin_tier2.rs | the_dshot600_fixture_s_outputs_are_witnessed_in_their_window_of_a_100_ms_boot | 4.73 | 6.33 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 4.10 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 6.33 | raised |
| twin_tier2.rs | the_emitted_platform_loads_and_lists_every_solved_peripheral | 2.47 | 3.72 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 3.35 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 3.72 | raised |
| twin_tier2.rs | the_fixture_s_campaign_archives_each_configuration_and_replays | 11.74 | 12.82 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 10.93 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor) | 12.82 | raised |
| twin_tier2.rs | the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused | 4 | 5.83 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 4.26 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 5.83 | raised |
| twin_tier2.rs | the_four_port_fixture_s_campaign_passes_and_replays | 4.7 | 7.72 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 4.85 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 7.72 | raised |
| twin_tier2.rs | the_four_port_fixture_witnesses_every_port_and_sbus_s_tags | 11.9 | 6.88 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 4.72 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 6.88 | lowered |
| twin_tier2.rs | the_imu_and_barometer_fixture_runs_every_script_step_witnessed | 7.4 | 5.65 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 5.42 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.65 | lowered |
| twin_tier2.rs | the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s | 4.4 | 5.27 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz); 3.55 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 5.27 | raised |
| twin_tier2.rs | the_pinned_image_s_renode_is_the_build_the_rettobase_witness_measured | 0.1 | 0.05 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C); 0.04 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 0.05 | lowered |
| twin_tier2.rs | the_pinned_renode_reads_rettobase_0_in_a_handler_nested_over_an_external_interrupt | 3.6 | 3.00 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 2.85 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor) | 3 | lowered |
| twin_tier2.rs | the_pwm_fixture_s_outputs_are_witnessed_off_the_grid_and_read_back | 15.7 | 5.42 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 4.20 (44787324803 shard 7 job 136468155867, AMD EPYC 9V74 80-Core Processor) | 5.42 | lowered |
| twin_tier2.rs | the_reference_firmware_boots_and_two_boots_agree | 7.34 | 11.17 (44787324803 shard 5 job 136468155864, AMD EPYC 7763 64-Core Processor); 10.46 (44787324803 shard 1 job 136468155952, Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz) | 11.17 | raised |
| twin_tier2.rs | the_reference_reads_u4_s_jedec_id_witnessed_behind_spi1_s_multiplexer | 3.4 | 5.44 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 3.68 (44787324803 shard 8 job 136468155925, Intel(R) Xeon(R) 6975P-C) | 5.44 | raised |
| twin_tier2.rs | the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed | 10.7 | 7.86 (44787324803 shard 9 job 136468155985, AMD EPYC 9V74 80-Core Processor); 7.80 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 7.86 | lowered |
| twin_tier2.rs | the_sentinel_02_shaped_board_is_witnessed_whole | 24.7 | 8.27 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C); 8.15 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 8.27 | lowered |
| twin_tier2.rs | the_serial_dshot_9600_board_is_witnessed_in_its_100_ms_boot | 40.1 | 5.19 (44787324803 shard 4 job 136468155846, AMD EPYC 9V45 96-Core Processor); 5.01 (44787324803 shard 3 job 136468155908, Intel(R) Xeon(R) 6975P-C) | 5.19 | lowered |
| twin_tier2.rs | the_unmodelled_read_moves_each_modelled_status_write_by_at_most_32_instructions | 45 | 8.79 (44787324803 shard 10 job 136468155884, AMD EPYC 7763 64-Core Processor); 6.35 (44787324803 shard 6 job 136468155972, Intel(R) Xeon(R) 6975P-C) | 8.79 | lowered |
| twin_tier2.rs | without_the_cited_wait_each_data_read_answers_its_reset_value | 6.9 | 8.69 (44787324803 shard 2 job 136468155837, AMD EPYC 7763 64-Core Processor); 4.88 (44787324803 shard 11 job 136468155848, AMD EPYC 9V45 96-Core Processor) | 8.69 | raised |
| adversarial_1301.rs | attack_a_no_connect_pad_moved_onto_another_net_is_refused | 1.2 | 1.61 (44764528379 shard 7 job 136389208314, solo); 0.96 (44774205804 shard 0 job 136422874788, solo); 0.94 (44768742667 shard 7 job 136403197282, solo) | 1.61 | raised |
| adversarial_1302_tools.rs | kicad_does_not_see_a_floating_loop_on_a_net_no_pad_carries | 2.6 | 3.56 (44768742667 shard 3 job 136403197350, solo); 3.44 (44764528379 shard 3 job 136389208467, solo); 2.88 (44774205804 shard 11 job 136422874675, solo); 2.83 (44774672284 shard 11 job 136424625516, solo); 1.96 (44775091139 shard 11 job 136425978726, solo); 1.92 (44787324803 shard 11 job 136468155848, solo) | 3.56 | raised |
| against_fab_tier2.rs | verify_against_fab_finds_the_package_it_wrote_identical | 420.7 | 585.18 (44775091139 shard 7 job 136425978453, solo); 576.22 (44774205804 shard 7 job 136422874568, solo); 550.07 (44764528379 shard 4 job 136389208317, solo); 546.04 (44768742667 shard 4 job 136403197417, solo); 477.87 (44787324803 shard 7 job 136468155867, solo); 450.58 (44774672284 shard 7 job 136424625775, solo) | 585.18 | raised |
| bench_tier1.rs | an_unpopulated_or_overflowing_pull_down_fails | 1.8 | 1.29 (44764528379 shard 9 job 136389208357, solo); 1.23 (44768742667 shard 9 job 136403197213, solo) | 1.29 | lowered |
| connectors_tier1.rs | a_board_draws_exactly_its_declared_connectors | 0.7 | 1.10 (44774672284 shard 1 job 136424625471, solo); 1.04 (44787324803 shard 1 job 136468155952, solo); 0.86 (44764528379 shard 1 job 136389208543, solo); 0.85 (44775091139 shard 1 job 136425978404, solo); 0.75 (44768742667 shard 1 job 136403197203, solo) | 1.1 | raised |
| fab_tier1.rs | a_rail_or_bus_no_fault_kind_reaches_is_named | 4 | 3.03 (44775091139 shard 7 job 136425978453, solo); 3.03 (44764528379 shard 4 job 136389208317, solo); 3.01 (44774205804 shard 7 job 136422874568, solo); 2.81 (44768742667 shard 4 job 136403197417, solo); 2.35 (44787324803 shard 7 job 136468155867, solo); 2.18 (44774672284 shard 7 job 136424625775, solo) | 3.03 | lowered |
| fab_tier1.rs | the_acknowledged_reference_passes_the_check_before_any_gate | 2.9 | 1.78 (44774672284 shard 3 job 136424625451, solo); 1.44 (44787324803 shard 3 job 136468155908, solo); 1.43 (44774205804 shard 3 job 136422874580, solo); 1.41 (44775091139 shard 3 job 136425978466, solo) | 1.78 | lowered |
| hwdef_tier1.rs | the_board_crate_s_speeds_equal_the_stored_ones | 0.5 | 0.73 (44774672284 shard 9 job 136424625560, solo); 0.58 (44774205804 shard 9 job 136422874763, solo); 0.57 (44787324803 shard 9 job 136468155985, solo); 0.43 (44775091139 shard 9 job 136425978349, solo) | 0.73 | raised |
| ledger_tier1.rs | a_new_field_is_unacknowledged_and_an_entry_beyond_the_ledger_is_stale | 1 | 0.18 (44774205804 shard 11 job 136422874675, solo) | 0.18 | lowered |
| ledger_tier1.rs | the_families_the_work_order_names_are_classed_as_ruled | 1 | 0.10 (44774672284 shard 9 job 136424625560, solo); 0.08 (44787324803 shard 9 job 136468155985, solo); 0.08 (44774205804 shard 9 job 136422874763, solo); 0.07 (44775091139 shard 9 job 136425978349, solo) | 0.1 | lowered |
| models_tier2.rs | an_access_the_datasheets_forbid_warns | 9.1 | 6.51 (44774672284 shard 4 job 136424625487, solo); 4.54 (44787324803 shard 4 job 136468155846, solo); 4.35 (44774205804 shard 4 job 136422874758, solo); 3.63 (44775091139 shard 4 job 136425978352, solo) | 6.51 | lowered |
| models_tier2.rs | our_models_round_half_away_from_zero | 13.2 | 4.79 (44768742667 shard 6 job 136403197343, solo); 3.57 (44764528379 shard 6 job 136389208401, solo) | 4.79 | lowered |
| models_tier2.rs | the_bmp581_measures_after_its_deep_start_up_and_reads_from_its_last_write | 13.6 | 6.57 (44768742667 shard 4 job 136403197417, solo); 6.06 (44774672284 shard 6 job 136424625504, solo); 5.94 (44764528379 shard 4 job 136389208317, solo); 5.29 (44774205804 shard 6 job 136422874606, solo); 4.77 (44775091139 shard 6 job 136425978335, solo); 4.23 (44787324803 shard 6 job 136468155972, solo) | 6.57 | lowered |
| models_tier2.rs | the_fifo_timestamp_steps_as_the_datasheet_gives_it_at_1_khz | 14 | 7.16 (44787324803 shard 5 job 136468155864, solo); 6.90 (44775091139 shard 5 job 136425978472, solo); 6.16 (44774205804 shard 5 job 136422874952, solo); 5.73 (44764528379 shard 11 job 136389208453, solo); 5.61 (44768742667 shard 11 job 136403197292, solo); 3.60 (44774672284 shard 5 job 136424625501, solo) | 7.16 | lowered |
| orientation_tier1.rs | a_hand_turned_imu_footprint_is_refused_by_the_drc_gate | 5.7 | 2.91 (44775091139 shard 0 job 136425978460, solo); 2.68 (44764528379 shard 5 job 136389208624, solo); 2.61 (44774672284 shard 0 job 136424625482, solo); 2.13 (44787324803 shard 0 job 136468155823, solo); 2.13 (44768742667 shard 5 job 136403197259, solo); 1.72 (44774205804 shard 0 job 136422874788, solo) | 2.91 | lowered |
| orientation_tier1.rs | the_placed_imu_agrees_with_the_declared_orientation | 25.2 | 19.34 (44774672284 shard 9 job 136424625560, solo); 16.42 (44774205804 shard 9 job 136422874763, solo); 16.22 (44787324803 shard 9 job 136468155985, solo); 13.37 (44775091139 shard 9 job 136425978349, solo) | 19.34 | lowered |
| own_board_tier1.rs | a_declaration_below_the_parts_cited_typical_is_refused | 6.2 | 2.83 (44775091139 shard 7 job 136425978453, solo); 2.81 (44774205804 shard 7 job 136422874568, solo); 2.22 (44787324803 shard 7 job 136468155867, solo); 2.04 (44774672284 shard 7 job 136424625775, solo) | 2.83 | lowered |
| own_board_tier1.rs | extract_counts_each_declared_pin_as_declared_never_as_a_maximum | 5.9 | 2.64 (44775091139 shard 2 job 136425978348, solo); 2.60 (44787324803 shard 2 job 136468155837, solo); 2.60 (44764528379 shard 3 job 136389208467, solo); 2.59 (44768742667 shard 3 job 136403197350, solo); 1.90 (44774672284 shard 2 job 136424625484, solo); 1.80 (44774205804 shard 2 job 136422874310, solo) | 2.64 | lowered |
| own_board_tier1.rs | the_bmp581_is_wired_as_its_datasheet_says | 5.7 | 2.77 (44774205804 shard 10 job 136422874510, solo); 2.58 (44787324803 shard 10 job 136468155884, solo); 2.16 (44775091139 shard 10 job 136425978428, solo); 2.15 (44774672284 shard 10 job 136424625591, solo) | 2.77 | lowered |
| own_board_tier1.rs | the_icm_is_wired_as_ds_000347_says | 6.1 | 2.74 (44774672284 shard 1 job 136424625471, solo); 2.64 (44787324803 shard 1 job 136468155952, solo); 2.56 (44774205804 shard 1 job 136422874833, solo); 2.15 (44775091139 shard 1 job 136425978404, solo) | 2.74 | lowered |
| own_board_tier1.rs | without_its_declaration_the_bmp581_is_an_i2c_pin_at_um10204_s_ci | 5.8 | 2.68 (44774672284 shard 9 job 136424625560, solo); 2.26 (44774205804 shard 9 job 136422874763, solo); 2.24 (44764528379 shard 1 job 136389208543, solo); 2.23 (44787324803 shard 9 job 136468155985, solo); 2.04 (44768742667 shard 1 job 136403197203, solo); 1.83 (44775091139 shard 9 job 136425978349, solo) | 2.68 | lowered |
| own_board_tier1.rs | without_its_declaration_the_icm_is_refused | 5.9 | 2.78 (44774205804 shard 11 job 136422874675, solo); 2.51 (44774672284 shard 11 job 136424625516, solo); 1.67 (44775091139 shard 11 job 136425978726, solo); 1.59 (44787324803 shard 11 job 136468155848, solo) | 2.78 | lowered |
| speeds_tier1.rs | the_reference_drives_sck_and_mosi_at_01_and_names_every_other_line | 1 | 1.54 (44774672284 shard 1 job 136424625471, solo); 1.51 (44764528379 shard 11 job 136389208453, solo); 1.48 (44787324803 shard 1 job 136468155952, solo); 1.45 (44774205804 shard 1 job 136422874833, solo); 1.45 (44768742667 shard 11 job 136403197292, solo); 1.20 (44775091139 shard 1 job 136425978404, solo) | 1.54 | raised |
| supply_libm.rs | the_net_copper_through_every_libm_call_is_the_golden | 9.2 | 15.75 (44764528379 shard 11 job 136389208453, solo); 15.45 (44768742667 shard 11 job 136403197292, solo); 13.47 (44787324803 shard 8 job 136468155925, solo); 9.29 (44774205804 shard 8 job 136422874384, solo); 8.58 (44774672284 shard 8 job 136424625670, solo); 7.14 (44775091139 shard 8 job 136425978418, solo) | 15.75 | raised |
| support_tier1.rs | a_capacitor_whose_ground_side_is_on_another_net_fails | 2.4 | 1.58 (44774672284 shard 1 job 136424625471, solo); 1.51 (44787324803 shard 1 job 136468155952, solo); 1.47 (44774205804 shard 1 job 136422874833, solo); 1.23 (44775091139 shard 1 job 136425978404, solo) | 1.58 | lowered |
| support_tier1.rs | a_reset_vcap_or_debug_net_that_is_not_its_own_fails | 2.5 | 1.53 (44774672284 shard 9 job 136424625560, solo); 1.26 (44774205804 shard 9 job 136422874763, solo); 1.25 (44787324803 shard 9 job 136468155985, solo); 0.99 (44775091139 shard 9 job 136425978349, solo) | 1.53 | lowered |
| support_tier1.rs | an_unjudgeable_netlist_is_refused | 2.4 | 1.57 (44774205804 shard 10 job 136422874510, solo); 1.49 (44787324803 shard 10 job 136468155884, solo); 1.23 (44775091139 shard 10 job 136425978428, solo); 1.23 (44774672284 shard 10 job 136424625591, solo) | 1.57 | lowered |
| support_tier1.rs | one_capacitor_serves_one_pin | 3.1 | 1.22 (44764528379 shard 1 job 136389208543, solo); 1.14 (44768742667 shard 1 job 136403197203, solo) | 1.22 | lowered |
| four_layer_fab_tier2.rs | (block) | 625 | 740.63 (cited by the WO (AET-WO-1912)); 732.88 (44775091139 shard 2 job 136425978348); 695.49 (44787324803 shard 2 job 136468155837); 629.96 (44764528379 shard 1 job 136389208543); 629.56 (44768742667 shard 1 job 136403197203); 573.58 (44774672284 shard 2 job 136424625484); 542.50 (44774205804 shard 2 job 136422874310); 479.73 (44787324803 shard 0 job 136468155823 alone, Intel(R) Xeon(R) 6975P-C) | 741 | raised |
| verify_47k_tier2.rs | (block) | 220 | 277.29 (cited by the WO (AET-WO-1912)); 270.78 (44774672284 shard 11 job 136424625516); 268.97 (44774205804 shard 11 job 136422874675); 251.46 (44764528379 shard 11 job 136389208453); 244.40 (44768742667 shard 11 job 136403197292); 241.62 (44787324803 shard 7 job 136468155867 alone, AMD EPYC 9V74 80-Core Processor); 188.42 (44775091139 shard 11 job 136425978726); 187.22 (44787324803 shard 11 job 136468155848) | 278 | raised |

## 3. The PR's runs (§1 step 4, §2)

Read with `spike/1912/fetch.sh` and `spike/1912/table.py` [measured]. The spread is reported two ways: job minutes, the REFIT RULE's measure, and run-step seconds, which weights move and overhead does not.

| run | head | result | tier-2 | max shard (job min) | min | **spread (job min)** | run-step max / min s | run-step spread | planned per shard |
|---|---|---|---|---|---|---|---|---|---|
| 44794507671 | `fb5ba007` (rows, `1211a` merged) | green | 200 / 200 | 17.1 (shard 2; overhead 328 s) | 11.4 | **5.7** | 881 / 505 | 6.3 min | 816 s |
| **44798881940** | **`e5faabd8`** (the last run before READY: `1213` and `4240c` merged, re-planned) | **green** | **200 / 200** | **16.2** (shards 7 and 10) | **11.2** | **5.1** | 859 / 525 | 5.6 min | 816 s |

Before, on the five `main` runs (§1): spread 6.6, 7.0, 15.5, 8.1 and 5.3 min; max shard 17.3, 18.0, 27.6, 17.8, 18.3.

### Run 44794507671, per shard

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136494777031 | 15.4 | 577 | 348 | 816 | 779 |  | outline_tier1.rs 188/251 |
| 1 | 136494777268 | 14.5 | 770 | 100 | 816 | 816 |  |  |
| 2 | 136494777339 | 17.1 | 695 | 328 | 816 | 816 |  | supply_tier1.rs 638/721 |
| 3 | 136494777039 | 14.1 | 741 | 102 | 816 | 816 |  |  |
| 4 | 136494776982 | 11.4 | 505 | 179 | 816 | 816 |  | verify_2k2_tier2.rs 340/537 |
| 5 | 136494777142 | 13.3 | 647 | 151 | 816 | 816 |  | attribute_seeded.rs 425/526 |
| 6 | 136494777134 | 14.9 | 791 | 101 | 816 | 816 |  |  |
| 7 | 136494777646 | 12.7 | 631 | 130 | 816 | 816 |  | verify_2k2_tier2.rs 394/458 |
| 8 | 136494777124 | 16.8 | 881 | 128 | 816 | 816 | against_fab_tier2.rs 532/447 |  |
| 9 | 136494777044 | 14.9 | 609 | 284 | 816 | 816 |  | power_board_tier1.rs 252/331 |
| 10 | 136494777257 | 12.0 | 590 | 130 | 816 | 816 |  | verify_47k_tier2.rs 192/278 |
| 11 | 136494777149 | 16.3 | 842 | 135 | 816 | 816 |  |  |

### Run 44798881940 (the last run before READY), per shard

| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows | under by 60 s or more |
|---|---|---|---|---|---|---|---|---|
| 0 | 136508596716 | 13.5 | 625 | 186 | 816 | 779 |  |  |
| 1 | 136508596814 | 14.4 | 736 | 128 | 816 | 816 |  | four_layer_fab_tier2.rs 680/741 |
| 2 | 136508596650 | 14.0 | 677 | 164 | 816 | 816 |  | supply_tier1.rs 613/721 |
| 3 | 136508596768 | 14.0 | 721 | 118 | 816 | 816 |  |  |
| 4 | 136508596658 | 11.2 | 525 | 144 | 816 | 816 |  | verify_2k2_tier2.rs 361/537 |
| 5 | 136508596690 | 15.8 | 845 | 102 | 816 | 816 |  |  |
| 6 | 136508596702 | 14.4 | 699 | 165 | 816 | 816 |  | twin_tier2.rs 502/568 |
| 7 | 136508596623 | 16.2 | 859 | 115 | 816 | 816 | verify_2k2_tier2.rs 547/458 |  |
| 8 | 136508596603 | 11.8 | 566 | 142 | 816 | 816 |  | against_fab_tier2.rs 348/447 |
| 9 | 136508596697 | 14.3 | 752 | 106 | 816 | 816 |  |  |
| 10 | 136508596787 | 16.2 | 815 | 156 | 816 | 816 |  |  |
| 11 | 136508596761 | 14.6 | 732 | 144 | 816 | 816 |  |  |

Back-test on it (`locked.py`): `twin_tier2` 1,488 s of rows against 1,352 s run (most under-weighed −13 s, shard 5); `board_tier2` 1,195 against 1,094 (−34 s, shard 10). Over all eight runs: `twin_tier2` mean absolute error 21 s a shard (47 before), `board_tier2` 16 s (19 before).

## 4. The adversarial pass, targeted (WO §5)

**The attack:** a row lowered from an alone run that, colocated, waits on a lock and over-runs its shard.

- **A1, the back-test** [measured]: `spike/1913/locked.py` over every (run, shard) that ran `twin_tier2` or `board_tier2`, in the five `main` runs, the throwaway's normal pass and the PR's two runs (182 samples).
  - The new rows sit under the measured `finished in` by at most 89 s (`twin_tier2`, 44787324803 shard 10, EPYC 7763), against a shard plan of 816 s.
  - On the PR's run 44794507671 the worst is −51 s (`twin_tier2`, shard 11). That shard ran 842 s of tests, 26 s over its plan, and its job took 16.3 min.
  - On run 44798881940 the worst is −34 s (`board_tier2`, shard 10, job 16.2 min). `board_tier2`'s worst anywhere is −41 s. **No shard over-ran 20 minutes.**
- **A2, the sampling bias** [measured, then computed; the alone pass logged each runner's CPU]: 18 lowered rows took both alone samples on Xeon 6975P-C or EPYC 9V45, the models that read fastest in this pass; their rows sum to 357.9 s. Over 14 tests with one sample on each, the slowest model (EPYC 7763) ran 1.52× the fast one at the median, 1.88× at worst.
  - The largest is `battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats`, 288.767 → 214.68 (samples 208.42 and 214.68). On a slow runner it may take about 320 s, about 110 s over its row.
  - It was in shard 11 of run 44794507671, whose `twin_tier2` ran 51 s over its rows. That this test was the cause is inferred: normal runs do not log their CPU.
  - Not fixed here: the WO sets a row to its alone time. A third sample on the slowest model, or a per-model factor, is a refit-rule change (§6, F1).
- **A3, the lowered row's queue** [read]: `twin_tier2`'s `on_a_bus_with_two_devices_each_read_is_witnessed` fell furthest, 216.4 → 12.8 (samples 12.80, 11.37). Its in-shard `ok` less `Running` reached 474 s.
  - That bound is queue: its own work is one `generate` (`two_bme()`, a per-process `OnceLock`) and its Renode calls.
  - The binary-level back-test above carries it: no shard holding it was under-weighed by more than the figures in A1.
- **Mutation:** this order adds and changes no guard (weights, a spike, FEEDBACK), so there is nothing to mutate. The throwaway's `ci/shard.sh` change was reverted; it is byte-equal to `main`'s.

## 5. Gates

On `fb5ba007` (the rows with `1211a` merged), in a capped scope (16G, swap 0) [measured]:
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `bash ci/test-shard.sh`: pass.
- `cargo test --workspace`: 2,545 passed, 0 failed.

Since then, the reduction commit (`e1393137`) deleted three byte-identical copies of `spike/1912` scripts and a stray `.pyc`, repointed `spike/1913/*.py` at `spike/1912/table.py`, and edited FEEDBACK; it touches no Rust. The merge of `main` brings `1213` and `4240c`, whose own PRs passed these gates. CI ran every gate on the merged head (§3). `TIER2_MIN` is unchanged.

## 6. Findings and decisions

- **F1, backlog (the REFIT RULE):** two alone samples can land on the two fastest CPU models, and GitHub's runners span about 1.5× (A2). A rule that takes one alone sample on the slowest model seen, or scales by a measured per-model factor, would close A2. Not this order's fence (`ci/shard.sh`'s rule text).
- **F2, the rows still sit high, by design:** after the refit, `twin_tier2`'s rows are 12 % over what it ran and `board_tier2`'s 11 % (§2's back-test). An alone time pays work that overlaps in a shard: a test's own `generate`, compile and Kani run while another test holds the lock. Weighing a lock-serialised binary from in-shard lock times would track that better, but the REFIT RULE forbids it (step 2). Recorded, not changed.
- **F3 and D1, the remaining spread is `verify_2k2_tier2`'s two rows** [measured]. Both PR runs ran a wider spread in run steps than in jobs (6.3 against 5.7 min, then 5.6 against 5.1), so overhead narrowed the spread here rather than set it.
  - **The fastest shard both times was shard 4:** 11.4 and 11.2 min, with run steps of 505 and 525 s against a plan of 816. It held `verify_2k2_tier2`'s `verify_binds_the_reference_and_two_runs_print_the_same_bytes`, its only heavy test, which ran 340 and 361 s against its 537.17 row.
  - **On run 44798881940 the slowest run step was shard 7** (859 s), holding the other `verify_2k2_tier2` test, `verify_fab_out_writes_the_package_its_verdict_names_at_any_path`, at 547 against 458.
  - Both rows sit within 25 % of their worst `solo` samples over the five runs and the throwaway (537.17 is 8 % over 495.21; 457.8 is 17 % under 550.51), so the widening does not move them. Each is one verify with a wide spread of its own across runners: 323–495 s and 343–551 s over those six runs, 340–361 s and 394–547 s on the PR's two. The shape is the same as `supply_tier1`'s modes.
  - **D1 (decision):** leave them, or set `verify_binds_…` to its worst `solo` sample (537.17 → 495.21) as a further widening. That would take 42 s off a shard's plan, inside the noise of one verify. I recommend leaving them.
  - Overhead (the job less its run step: checkout, image pull, the shared build's unpack) ranged 96–804 s on the five `main` runs and 100–348 s on the PR's runs; weights cannot move it.
- **F4, `supply_tier1` stays at its worst mode** (721 s, WO §1 step 3). It ran 460–651 s on the five runs, then 696 (the throwaway), 638 and 613 s on the PR's runs. It is not the remaining spread: on run 44794507671 shard 2, which held it, was the slowest job by 0.3 min from 328 s of overhead; on run 44798881940 shard 2 ran 14.0 min, mid-pack.
- **F5, rows outside the WO's set** (the architect's widening, 18:09Z): 28 rows moved (§2). `verify_2k2_tier2`'s `verify_binds_…` stays at 537.17, 8 % above its worst `solo` sample. Three small rows with no `solo` sample were not moved (`models_tier2`'s `ardupilot_s_probe_and_init_pass_against_our_models` 13.7 against an upper bound of 9.00; two rows of 2.0 and 1.0 s).
- **F7, backlog (`lib.rs`'s `supply::` rows):** in §1 the `unittests src/lib.rs` rows sum to 132 s against 38–50 s run in the shard that held them. Its tests overlap in one process, so a per-test row counts the overlap once per test. Weighing it as a colocated block (`ci/colocate.tsv`) is a REFIT RULE 3 question for a later refit; it was not changed here (no `solo` sample, and no row is more than 25 % off one).
- **F6, process:** the first `git revert` of the throwaway was refused silently (a dirty tree, with `-q` and a `tail`); it was caught at the next diff and redone (`9c28088f`). Lesson: never pipe a revert or merge through `tail`, and check `git log -1` after.
