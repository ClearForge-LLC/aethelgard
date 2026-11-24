#!/bin/bash
# AET-WO-2314 §1.1 (spike/1609/sh/capture.sh, paths moved): inside the tools image, the unmodified CLI's
# `simulate --foreign` on sentinel-02-own while a watcher copies Renode's argv (the CLI's whole
# monitor script) and the booted platform from /proc. Output /cache/cap. Usage: capture.sh ELF
set -u
cd /repo
A=/repo/target/debug/aethelgard; O=/cache/cap; rm -rf $O; mkdir -p $O /cache/run
S=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
[ -d /cache/run/gen ] || $A generate --spec $S --out /cache/run/gen > /cache/run/generate.json
( for _ in $(seq 1 6000); do
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
      case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.txt
        d=$(printf '%s' "$c" | grep -o 'LoadPlatformDescription @[^;]*' | head -1 | sed 's/LoadPlatformDescription @//')
        cp "$d" $O/platform-booted.repl 2>/dev/null; cp -r "$(dirname "$d")/models" $O/ 2>/dev/null; exit 0;; esac
    done
    sleep 0.02
  done ) &
$A simulate --spec $S --platform /cache/run/gen/board/sentinel-02-own.repl --firmware "$1" --foreign 5000ms > $O/simulate.json 2> $O/stderr.txt; echo "simulate exit=$?"
wait
python3 -I - $O <<'PY'
import sys
o = sys.argv[1]
argv = open(f"{o}/renode-argv.txt").read().split("\n")
i = argv.index("-e")
open(f"{o}/script.txt", "w").write("\n".join(c.strip() for c in argv[i + 1].split(";")))
print("ANSWER 2314: captured", len(argv[i + 1].split(";")), "commands")
PY
