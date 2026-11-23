#!/bin/bash
# AET-WO-2310, inside the tools image: (a) the in-image seconds of the A8 test (its weights row); (b) Q-M (ii)'s cost
# in seconds, ruled 05:32Z: the wall time of one droop-bor3 campaign (sentinel-01-minimal) by main's CLI and by the
# branch's, three times each, on the same ELF. REPO = a writable copy of the branch; main's tree is /cache/main.
set -u
cd /repo
RUSTC_BOOTSTRAP=1 cargo test -q --locked -p aethelgard-cli --features aethelgard-cli/with-tools --test twin_tier2 -- --exact m1_a_core_that_slept_and_wrote_its_boot_word_is_cpu_halted -Z unstable-options --report-time --test-threads 1 2>&1 | grep -oE "m1_a_core_that_slept_and_wrote_its_boot_word_is_cpu_halted|ok <[0-9.]+s>|FAILED" | paste -sd' '
ELF=/cache/runs/fw/min.elf
for side in main branch; do
  if [ $side = main ]; then BIN=/cache/target-main/debug; SRC=/cache/main; else BIN=/cache/target-branch/debug; SRC=/repo; fi
  for r in 1 2 3; do
    A=/tmp/a-$side-$r; rm -rf $A
    s=$(date +%s%N); (cd $SRC && $BIN/aethelgard campaign --scenarios scenarios/sentinel-01/droop-bor3.json --firmware $ELF --archive $A > /tmp/out-$side-$r 2>&1); rc=$?; e=$(date +%s%N)
    echo "$side run $r rc=$rc wall_ms=$(( (e-s)/1000000 ))"
  done
done
