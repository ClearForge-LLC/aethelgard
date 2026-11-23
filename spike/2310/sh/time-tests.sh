#!/bin/bash
# AET-WO-2310, inside the tools image (img-rw.sh, REPO = a writable copy of the branch): the in-image seconds of
# the tier-2 tests this branch added, renamed or made longer, each run alone, for ci/test-weights.tsv (P3).
set -u
cd /repo
T() { RUSTC_BOOTSTRAP=1 cargo test --locked -p aethelgard-cli --features aethelgard-cli/with-tools --test "$1" -- --exact "$2" -Z unstable-options --report-time --test-threads 1 2>&1 | grep -E "^test $2 " ; }
T campaign_tier2 a_battery_sag_s_deck_answer_is_judged_after_its_boot
T campaign_tier2 the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay
T campaign_tier2 renodes_reset_recipe_is_pinned_step_by_step
T twin_tier2 m1_a_core_that_slept_and_wrote_its_boot_word_is_cpu_halted
T twin_tier2 battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats
T twin_tier2 battery_sag::the_power_fixture_s_campaign_meets_every_expectation_replays_and_repeats
