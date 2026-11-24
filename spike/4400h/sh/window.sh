#!/bin/bash
# AET-WO-4400h (1): one `simulate --foreign` of the held ELF on sentinel-02-own, by the patched CLI
# (patches/window.patch), inside the pinned tools image. Records the verdict, wall time, the host's
# load average sampled each second during the run, and the verdict's memory and log figures.
# Mounts: /repo = the patched tree (target/ built), /cache = ~/.cache/aet-4400h.
# Usage: window.sh NAME WINDOW [taskset-cpus]   env: AET4400H_WALL_S, AET4400H_RSS_KIB, AET4400H_NO_USART_LOG
set -u
name=$1; w=$2; cpus=${3:-}
O=/cache/win/$name; rm -rf "$O"; mkdir -p "$O"
spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
elf=/cache/boards/sentinel-02/arducopter.elf
A=target/debug/aethelgard
[ -f /cache/win/gen/board/sentinel-02-own.repl ] || { mkdir -p /cache/win; $A generate --spec $spec --out /cache/win/gen > /cache/win/generate.json || exit 1; }
env | grep '^AET4400H_' > "$O/env.txt"
( while :; do echo "$(date +%s.%N) $(cut -d' ' -f1-4 /proc/loadavg)"; sleep 1; done ) > "$O/load.txt" &
lp=$!
pre=(); [ -n "$cpus" ] && pre=(taskset -c "$cpus")
t0=$(date +%s.%N)
"${pre[@]}" $A simulate --platform /cache/win/gen/board/sentinel-02-own.repl --spec $spec --firmware $elf --foreign "$w" > "$O/verdict.json" 2> "$O/stderr.txt"
rc=$?
t1=$(date +%s.%N); kill $lp
python3 -I - "$O" "$rc" "$t0" "$t1" "$w" "$name" "$cpus" <<'PY'
import json, sys
o, rc, t0, t1, w, name, cpus = sys.argv[1:]
wall = float(t1) - float(t0)
try:
    v = json.load(open(o + "/verdict.json"))
except Exception as e:
    v = {}
def find(d, key, acc):
    if isinstance(d, dict):
        for k, x in d.items():
            if k == key: acc.append(x)
            find(x, key, acc)
    elif isinstance(d, list):
        for x in d: find(x, key, acc)
    return acc
mem = find(v, "memory", [])
peak = None
for m in mem:
    if isinstance(m, dict) and "peak_rss_kib" in m: peak = m["peak_rss_kib"]
loads = [float(l.split()[1]) for l in open(o + "/load.txt") if l.strip()]
kinds = sorted({f.get("kind") for f in find(v, "findings", []) for f in (f if isinstance(f, list) else []) if isinstance(f, dict) and f.get("severity", "error") != "info"})
logs = find(v, "log", [])
row = dict(name=name, window=w, exit=int(rc), wall_s=round(wall, 2), peak_rss_kib=peak,
           permille_of_1572864=(peak * 1000 // 1572864) if peak else None,
           load1_min=min(loads) if loads else None, load1_max=max(loads) if loads else None,
           cpus=cpus or "all", ok=v.get("ok"), kinds=kinds, log=logs[:1],
           stop=(v.get("error") or {}).get("kind") if isinstance(v.get("error"), dict) else v.get("error"))
print(json.dumps(row))
open(o + "/row.json", "w").write(json.dumps(row) + "\n")
PY
