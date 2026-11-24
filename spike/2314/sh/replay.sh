#!/bin/bash
# AET-WO-2314 D5 (ruled 10:30Z): shard 11's failing plan (ci.yml run 45312081588's `shard.tsv`, 288 tests,
# byte-identical to the runs that failed 3 of 4) through `ci/shard.sh run`, pinned to 4 CPUs with taskset,
# until the BMP581 recovery test fails or N runs pass. Inside the tools image, after `ci/shard.sh build`.
# Each run's log is kept in /cache/replay. Usage: replay.sh [N]
set -u
cd /repo
mkdir -p target/ci-shard /cache/replay
t=bor_recovery::the_bmp581_extension_recovers_after_its_settle
for i in $(seq 1 "${1:-10}"); do
  cp /cache/shard/45312081588/shard.tsv target/ci-shard/shard-11.tsv
  s=$(date +%s)
  taskset -c 0-3 bash ci/shard.sh run target/ci-shard 11 > /cache/replay/run-$i.log 2>&1; rc=$?
  line=$(grep -a "test $t \.\.\." /cache/replay/run-$i.log | tail -1 | cut -f2)
  echo "ANSWER 2314: replay $i exit=$rc wall_s=$(( $(date +%s) - s )) cpus=$(taskset -c 0-3 nproc) load=$(cut -d' ' -f1 /proc/loadavg) :: ${line:-$t not run}"
  grep -a "test result: FAILED\|^.*---- .* stdout ----" /cache/replay/run-$i.log | cut -f2 | head -6
  if grep -aq "test $t \.\.\. FAILED" /cache/replay/run-$i.log; then
    grep -a 'PlantedEventUnplaced' /cache/replay/run-$i.log | cut -f2 | head -2
    break
  fi
done
