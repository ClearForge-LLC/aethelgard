#!/bin/bash
# AET-WO-3306, inside the tools image (img.sh, REPO = the worktree): the tier-2 tests attribution reaches,
# timed per test. Usage: tier2.sh [test-filter args for twin_tier2...]
set -u
cd /repo
cargo build -q --locked -p aethelgard-cli && cargo build -q --manifest-path bridge/Cargo.toml --locked || exit 9
T() { local t=$1; shift; RUSTC_BOOTSTRAP=1 cargo test --locked -p aethelgard-cli --features aethelgard-cli/with-tools --test "$t" -- "$@" -Z unstable-options --report-time --test-threads 1 2>&1 | grep -E '^test |test result|panicked|left|right|FAILED|finished in' ; }
T twin_tier2 a_step_sent_to_another_devices_address_that_answers_is_the_softwares a_fault_on_a_bus_with_two_devices_is_not_masked
[ "${SEEDED:-1}" = 1 ] && T attribute_seeded
echo "peak=$(cat /sys/fs/cgroup/memory.peak 2>/dev/null)"
