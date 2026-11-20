#!/bin/bash
# AET-WO-2305 §1.1, inside the tools image (sh/img.sh): one board's campaign, its undisturbed boot first,
# with the scratch CLI's full Renode log and composed commands kept (AET2305_LOG / AET2305_CMDS, a
# measurement-only patch never committed). usage: run.sh <name> <spec> <scenario...>   (paths from /repo)
set -u
B=${BIN:-/cache/target-main/debug}; A=$B/aethelgard
n=$1; spec=$2; shift 2
O=${OUT:-/cache/runs}/$n; rm -rf "$O"; mkdir -p "$O/log"
$A generate --spec "/repo/$spec" --out "$O/gen" > "$O/generate.json" 2>&1 || { echo "$n generate failed"; head -c 600 "$O/generate.json"; exit 1; }
$A build --crate "$O/gen/board" --out "$O/fw.elf" > "$O/build.json" 2>&1 || { echo "$n build failed"; head -c 600 "$O/build.json"; exit 1; }
sha256sum "$O/fw.elf" | cut -c1-64 > "$O/fw.sha256"
args=(); for s in "$@"; do args+=("/repo/$s"); done
s=$(date +%s)
AET2305_LOG=$O/log/renode AET2305_CMDS=$O/log/commands PATH=$B:/usr/local/bin:/usr/bin:/bin \
  $A campaign --scenarios "${args[@]}" --firmware "$O/fw.elf" --archive "$O/archive" > "$O/campaign.jsonl" 2> "$O/campaign.err"
echo "$n campaign exit $? wall $(( $(date +%s)-s ))s fw $(cat $O/fw.sha256 | cut -c1-8)"
