#!/bin/bash
# AET-WO-1919, inside the image at /repo: one command with ci/probe.py running beside it in the
# container (so it sees only this container's processes), then the scope's memory.peak.
# The probe's lines go to /repo/target/probe-<tag>.log.
# usage: bash ctl.sh <tag> <command...>
set -u
tag=$1; shift
log=/repo/target/probe-$tag.log stop=/repo/target/probe-$tag.stop
rm -f "$log" "$stop"
python3 -I ci/probe.py "$log" "$stop" &
probe=$!
t0=$(date +%s.%N)
"$@"
rc=$?
t1=$(date +%s.%N)
touch "$stop"; wait "$probe"
echo "CTL1919 tag=$tag rc=$rc wall_s=$(awk "BEGIN{print $t1-$t0}") memory.peak=$(cat /sys/fs/cgroup/memory.peak 2>/dev/null)" >&2
exit $rc
