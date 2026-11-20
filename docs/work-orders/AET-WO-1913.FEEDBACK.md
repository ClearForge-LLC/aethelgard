# AET-WO-1913 FEEDBACK — CI's fourth refit: the lock-serialised rows set to their alone times

## Status: MEASURING — §1 (the table before) committed before any row changes.

Builder: Claude (Opus 5.5), a fresh `1913` session, 2026-11-20. Worktree `~/aethelgard-1913`, branch `wo/aet-1913-refit-lower` from `origin/main` `c1012a2d`.

Labels: **measured** (from CI's own logs or runs), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1913/`; the scripts are in `spike/1913/` (1912's kit, plus `alone.py` rewritten and `locked.py` new).

## 1. The table before (§1 step 1)

**The five runs:** the five most recent green `ci.yml` runs on `main` at 17:00Z, each with 12 tools shards: 44775091139 (`c1012a2d`, this branch's base), 44774672284 (`63334d69`), 44774205804 (`08f9fb9c`, `1912`'s merge), 44768742667 (`8df353ee`) and 44764528379 (`8fdc9422`). The first three were planned with `1912`'s rows. Each was read with `spike/1913/fetch.sh` (jobs API, `gh run view --job ID --log`) and tabulated by `spike/1913/table.py` and `spike/1913/locked.py` [measured].

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
- **Outside this order's rows** (not changed here; see §6): `against_fab_tier2`'s `verify_against_fab_finds_the_package_it_wrote_identical` ran 585.18 s `solo` against its 420.7 row, more than 25 % under. `verify_2k2_tier2`'s `verify_binds_…` row of 537.17 is above every sample (323–495 s), and `lib.rs`'s `supply::` rows sum to 132 s against 38–50 s run.
- **The spread on job minutes** (slowest shard less fastest): 6.6, 7.0, 15.5, 8.1 and 5.3 min (max shard 17.3, 18.0, 27.6, 17.8, 18.3). 15.5 and 27.6 are shard 8 of 44774205804, whose run step was 851 s and its overhead 804 s.
