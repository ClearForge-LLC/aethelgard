#!/bin/bash
# AET-WO-1918 (1915's tool, its paths renamed), inside the image at /repo: one measured command, then the scope's
# memory.peak read from inside, which equals the host's (the container's cgroup
# namespace root is the capped-run scope; self-test selftest-2g).
# usage:
#   bash ctl.sh prove  <board dir> <spec> <harness name>
#   bash ctl.sh verify <spec> <fab-out dir name> <scenario...>
#   bash ctl.sh cmd    <command...>          (a candidate, from a scratch copy)
set -u
A=/repo/target/debug/aethelgard
mode=$1; shift
t0=$(date +%s.%N)
case "$mode" in
  prove)  $A prove --crate "$1" --harness "proofs::$3" --spec "$2" ;;
  verify) spec=$1; out=/repo/target/fab/$2; shift 2
          rm -rf "$out"; mkdir -p "$out"; chmod 755 "$out"
          $A verify --spec "$spec" --scenarios "$@" --fab-out "$out" ;;
  cmd)    "$@" ;;
  *) echo "ctl.sh: unknown mode $mode" >&2; exit 2 ;;
esac
rc=$?
t1=$(date +%s.%N)
echo "CTL1918 mode=$mode rc=$rc wall_s=$(echo "$t1 - $t0" | bc 2>/dev/null || awk "BEGIN{print $t1-$t0}") memory.peak=$(cat /sys/fs/cgroup/memory.peak)" >&2
exit $rc
