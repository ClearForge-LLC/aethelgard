#!/bin/bash
# AET-WO-4400f (from 4400e's gen.sh): inside the tools image, run the unmodified CLI's
# `simulate --foreign 100ms` on CI's ArduPilot ELF while a watcher copies Renode's argv (the CLI's
# whole monitor script) and the booted platform from /proc. Output /cache/cap/$1.
set -u
cd /repo
A=$CARGO_TARGET_DIR/debug/aethelgard; O=/cache/cap/$1; rm -rf $O; mkdir -p $O
F=/repo/crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
R=/cache/fw-base/sentinel-02-own/gen/board/sentinel-02-own.repl
( for _ in $(seq 1 3000); do
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
      case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.txt
        d=$(printf '%s' "$c" | grep -o 'LoadPlatformDescription @[^;]*' | head -1 | sed 's/LoadPlatformDescription @//')
        cp "$d" $O/platform-booted.repl 2>/dev/null; cp -r "$(dirname "$d")/models" $O/ 2>/dev/null; exit 0;; esac
    done
    sleep 0.02
  done ) &
$A simulate --platform $R --firmware /cache/boards/sentinel-02/arducopter.elf --spec $F --foreign 100ms > $O/simulate.json 2>&1; echo "simulate exit=$?"
wait
tr ';' '\n' < $O/renode-argv.txt | head -60
