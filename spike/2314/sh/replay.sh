#!/bin/bash
# AET-WO-2314 D5 (ruled 10:30Z): shard 11's failing plan (ci.yml run 45312081588's `shard.tsv`, 288 tests,
# byte-identical to the runs that failed 3 of 4) through `ci/shard.sh run`, pinned to 4 CPUs with taskset,
# until the BMP581 recovery test fails or N runs pass. Inside the tools image, after `ci/shard.sh build`.
# Each run's log is kept in /cache/replay (or /cache/replay-RUN). Usage: replay.sh [N [RUN]], RUN the ci.yml run whose
# shard-11 plan is replayed (default 45312081588; 45397844312 is 1610's failing one).
set -u
cd /repo
plan=${2:-45312081588}; out=/cache/replay${2:+-$2}
mkdir -p target/ci-shard $out
t=bor_recovery::the_bmp581_extension_recovers_after_its_settle
for i in $(seq 1 "${1:-10}"); do
  cp /cache/shard/$plan/shard.tsv target/ci-shard/shard-11.tsv
  s=$(date +%s)
  taskset -c 0-3 bash ci/shard.sh run target/ci-shard 11 > $out/run-$i.log 2>&1; rc=$?
  line=$(grep -a "test $t \.\.\." $out/run-$i.log | tail -1 | cut -f2)
  echo "ANSWER 2314: replay $i exit=$rc wall_s=$(( $(date +%s) - s )) cpus=$(taskset -c 0-3 nproc) load=$(cut -d' ' -f1 /proc/loadavg) :: ${line:-$t not run}"
  grep -a "test result: FAILED\|^.*---- .* stdout ----" $out/run-$i.log | cut -f2 | head -6
  if grep -aq "test $t \.\.\. FAILED" $out/run-$i.log; then
    grep -a 'PlantedEventUnplaced' $out/run-$i.log | cut -f2 | head -2
    break
  fi
done
