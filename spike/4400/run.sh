#!/bin/bash
# run.sh <label> <venv> <cmd...> : run a build step in ArduPilot's clone at nice 10, timed,
# inside a capped user scope; prints the scope's memory.peak before it exits.
set -u
label=$1; venv=$2; shift 2
A=$HOME/.cache/aet-4400
export PATH=$A/toolchain/gcc-arm-none-eabi-10-2020-q4-major/bin:$A/$venv/bin:/usr/bin:/bin
cd $A/ardupilot
/usr/bin/time -v -o $A/logs/time-$label.txt "$@" > $A/logs/out-$label.txt 2>&1
rc=$?
cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
echo "rc=$rc peak=$(cat $cg/memory.peak) max=$(cat $cg/memory.max) swapmax=$(cat $cg/memory.swap.max) cg=$cg" | tee $A/logs/scope-$label.txt
grep -E "Elapsed|User time|System time|Maximum resident" $A/logs/time-$label.txt
exit $rc
