#!/bin/bash
# Inside the tools image: build sentinel-02-own's own generated Rust firmware with the unmodified
# CLI, run the unmodified `simulate --firmware` once (100 ms) while a watcher copies Renode's argv
# (the CLI's whole monitor script for this platform) from /proc.
# Output under /cache/s02own.
set -u
A=/cache/target/debug/aethelgard
F=/repo/crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
O=/cache/s02own; mkdir -p $O
[ -d $O/gen ] || $A generate --spec $F --out $O/gen > $O/generate.json 2>&1 || { echo generate failed; exit 1; }
[ -f $O/fw.elf ] || $A build --crate $O/gen/board --out $O/fw.elf > $O/build.json 2>&1 || { echo build failed; head -c 1500 $O/build.json; exit 1; }
R=$O/gen/board/sentinel-02-own.repl
rm -f $O/renode-argv.*
( for _ in $(seq 1 3000); do
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
      case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.txt; exit 0;; esac
    done
    sleep 0.02
  done ) &
$A simulate --platform $R --firmware $O/fw.elf --spec $F --for 100ms > $O/simulate-rust.json 2>&1; echo "simulate(rust) exit=$?"
wait
python3 -I -c "import json; v=json.load(open('$O/simulate-rust.json')); print('verdict', v.get('verdict'), sorted({f['kind'] for f in v.get('findings',[])}))"
