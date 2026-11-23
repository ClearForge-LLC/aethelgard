#!/bin/bash
# AET-WO-2310, inside the tools image (img-rw.sh, REPO = a writable copy of the branch): every tier-2 test this
# branch changed or whose fixture it moved, timed per test (P3: one of 10 s or more needs a test-weights row).
set -u
cd /repo
cargo build -q --locked -p aethelgard-cli --features aethelgard-cli/with-tools && cargo build -q --manifest-path bridge/Cargo.toml --locked || exit 9
T() { local t=$1; shift; RUSTC_BOOTSTRAP=1 cargo test -q --locked -p aethelgard-cli --features aethelgard-cli/with-tools --test "$t" -- "$@" -Z unstable-options --report-time --test-threads 1 2>&1 | grep -E '^test |test result|panicked|left|right|FAILED|finished in|kind|message' | cut -c1-400 ; }
T campaign_tier2 the_two_reference_droops renodes_reset_recipe a_battery_sag_s_deck_answer a_droop_on_a_board_whose_first_read_is_on_spi a_reset_on_an_instant
T twin_tier2 m1_a_core the_dshot600_fixture_s_campaign_passes battery_sag:: bor_recovery::
T board_tier2 every_admitted_identifier_character verify_names_what_judged_nothing
T four_layer_fab_tier2
cargo test -q --locked --manifest-path bridge/Cargo.toml 2>&1 | grep -E 'test result|panicked|FAILED' 
echo "peak=$(cat /sys/fs/cgroup/memory.peak 2>/dev/null)"
