#!/bin/bash
# AET-WO-2311 §1.1 (from 4400f's capture.sh): inside the tools image, the unmodified CLI's
# `simulate --foreign` on the four-port ELF while a watcher copies Renode's argv (the CLI's whole
# monitor script) and the booted platform from /proc. Output /cache/cap/$1. Usage: capture.sh NAME WINDOW [ELF]
set -u
cd /repo
A=$CARGO_TARGET_DIR/debug/aethelgard; O=/cache/cap/$1; W=${2:-5000ms}; E=${3:-/cache/ap/four-port/arducopter.elf}; rm -rf $O; mkdir -p $O
python3 -I spike/2311/derive.py crates/aethelgard-cli/tests/fixtures/four-port.json \
  crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json $O/spec.json
$A solve --file $O/spec.json > $O/topology.json
$A generate --spec $O/spec.json --out $O/gen > $O/generate.json || { echo generate failed; exit 1; }
R=$(ls $O/gen/board/*.repl)
( for _ in $(seq 1 6000); do
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
      case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.txt
        d=$(printf '%s' "$c" | grep -o 'LoadPlatformDescription @[^;]*' | head -1 | sed 's/LoadPlatformDescription @//')
        cp "$d" $O/platform-booted.repl 2>/dev/null; cp -r "$(dirname "$d")/models" $O/ 2>/dev/null; exit 0;; esac
    done
    sleep 0.02
  done ) &
t0=$(date +%s%N); $A simulate --platform $R --firmware $E --spec $O/spec.json --foreign $W > $O/simulate.json 2> $O/stderr.txt; echo "simulate exit=$? wall_ms=$((($(date +%s%N)-t0)/1000000))"
wait
