#!/bin/bash
# AET-WO-1606 §1.1 and §2, inside the tools image (~/.cache/aet-1606/sh/img.sh): one board's
# generate, build and campaign (archive kept), with the CLI and bridge in $BIN.
# usage: BIN=/cache/target-X/debug OUT=/cache/runs-X run.sh <name> <spec> <scenario...>   (paths from /repo)
set -u
B=${BIN:?}; A=$B/aethelgard
n=$1; spec=$2; shift 2
O=${OUT:?}/$n; rm -rf "$O"; mkdir -p "$O"
$A generate --spec "/repo/$spec" --out "$O/gen" > "$O/generate.json" 2>&1 || { echo "$n generate failed"; head -c 600 "$O/generate.json"; exit 1; }
$A build --crate "$O/gen/board" --out "$O/fw.elf" > "$O/build.json" 2>&1 || { echo "$n build failed"; head -c 600 "$O/build.json"; exit 1; }
sha256sum "$O/fw.elf" | cut -c1-64 > "$O/fw.sha256"
args=(); for s in "$@"; do args+=("/repo/$s"); done
s=$(date +%s)
PATH=$B:/usr/local/bin:/usr/bin:/bin \
  $A campaign --scenarios "${args[@]}" --firmware "$O/fw.elf" --archive "$O/archive" > "$O/campaign.jsonl" 2> "$O/campaign.err"
echo "$n campaign exit $? wall $(( $(date +%s)-s ))s fw $(cut -c1-8 "$O/fw.sha256")"
