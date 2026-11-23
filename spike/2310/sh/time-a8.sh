#!/bin/bash
# AET-WO-2310: the A8 test's in-image seconds, for its ci/test-weights.tsv row (libtest's report-time, not quiet).
cd /repo
RUSTC_BOOTSTRAP=1 cargo test --locked -p aethelgard-cli --features aethelgard-cli/with-tools --test twin_tier2 -- --exact m1_a_core_that_slept_and_wrote_its_boot_word_is_cpu_halted -Z unstable-options --report-time --test-threads 1 2>&1 | grep -oE "ok <[0-9.]+s>|FAILED|test result.*"
