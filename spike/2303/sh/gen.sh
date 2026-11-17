#!/bin/bash
# Inside the image: generate and build the Sentinel-02-shaped board with the unmodified CLI, then
# run the unmodified CLI's simulate once (100 ms) while a watcher copies Renode's argv (its monitor
# script) from /proc. Output under /cache/s02.
set -u
A=/cache/target/debug/aethelgard
F=/repo/crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json
O=/cache/s02; mkdir -p $O
if [ ! -f $O/fw.elf ]; then
  rm -rf $O/gen
  $A generate --spec $F --out $O/gen > $O/generate.json 2>&1 || { echo generate failed; head -c 1500 $O/generate.json; exit 1; }
  $A build --crate $O/gen/board --out $O/fw.elf > $O/build.json 2>&1 || { echo build failed; head -c 1500 $O/build.json; exit 1; }
fi
R=$(ls $O/gen/board/*.repl | head -1); echo "platform $R"
rm -f $O/renode-argv.*
( for _ in $(seq 1 3000); do
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
      case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.$(basename $p); exit 0;; esac
    done
    sleep 0.02
  done ) &
s=$(date +%s%N)
$A simulate --platform $R --firmware $O/fw.elf --spec $F --for 100ms > $O/simulate.json 2>&1
echo "simulate exit=$? wall_ms=$(( ($(date +%s%N)-s)/1000000 ))"
wait
ls -la $O/renode-argv.* 2>/dev/null
python3 -I -c "import json; v=json.load(open('$O/simulate.json')); print('verdict', v.get('verdict'), sorted({f['kind'] for f in v.get('findings',[])}))" 2>/dev/null || head -c 800 $O/simulate.json
