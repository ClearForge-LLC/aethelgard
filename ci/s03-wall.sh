#!/usr/bin/env bash
# THROWAWAY (AET-WO-4412, ruling 2026-11-26T21:47Z (c)): Sentinel-03's foreign run NAME alone, three times, each
# under `time`, with the hang guard lifted in this job's build only (the workflow patches its checkout). Reverted
# before READY. Prints one WALL4412 line per run.
set -euo pipefail
name=$1 input=$2
bin=target/debug/aethelgard
work=$(mktemp -d)
"$bin" generate --spec examples/sentinel-03.json --out "$work/gen" > /dev/null
cpu=$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')
for rep in 1 2 3; do
  status=0
  t0=$(date +%s.%N)
  "$bin" simulate --spec examples/sentinel-03.json --platform "$work/gen/board/sentinel-03.foreign.repl" \
    --firmware boards/sentinel-03/arducopter.elf --foreign 12000ms \
    --input "crates/aethelgard-cli/tests/fixtures/4412/$input.json" > "$work/$rep.json" || status=$?
  t1=$(date +%s.%N)
  python3 -I - "$work/$rep.json" "$name" "$rep" "$status" "$t0" "$t1" "$cpu" "$(nproc)" <<'PY'
import json, sys
p, name, rep, status, t0, t1, cpu, nproc = sys.argv[1:]
v = json.load(open(p))
b = v.get("boot") or {}
print(f"WALL4412 run={name} rep={rep} exit={status} wall_s={float(t1) - float(t0):.2f} verdict={v.get('verdict')} "
      f"error={(v.get('error') or {}).get('kind')} virtual_ns={b.get('virtual_time_ns')} "
      f"peak_rss_kib={(v.get('memory') or {}).get('peak_rss_kib')} cpu=\"{cpu}\" nproc={nproc}")
PY
done
