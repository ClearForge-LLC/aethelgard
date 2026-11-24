#!/bin/bash
# AET-WO-2314 §1.1 (1): K copies of the BMP581 undisturbed boot (bmp.sh's) at once, all pinned to CPUS
# with taskset: each one's exit, wall time, boot word instant and peak RSS. Usage: contend.sh K CPUS
set -u
cd /repo
A=/repo/target/debug/aethelgard; O=/cache/bmp; S=examples/sentinel-01-ext-bmp581.json
echo "ANSWER 2314: contend nproc_pinned=$(taskset -c "$2" nproc)"
for i in $(seq 1 "$1"); do
  ( s=$(date +%s.%N); st=0
    taskset -c "$2" $A simulate --spec $S --platform $O/gen/board/*.repl --firmware $O/fw.elf --for 100ms > $O/c-$i.json || st=$?
    python3 -I spike/2314/sh/bmpline.py $O/c-$i.json "contend-$1x-$i" "$st" "$(python3 -c "print(round($(date +%s.%N) - $s, 1))")" ) &
done
wait
