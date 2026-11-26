#!/bin/bash
# AET-WO-4412 (1), from 4400j's: one `simulate --foreign` by the CLI built from REPO (default this worktree: main,
# unpatched, for every "today" row), inside the tools image (sh/img.sh; CPUS=0-3 pins it). Records the verdict, wall
# time, the host's load each second and the env. Unperturbed: no capture loop, no probe.
# Usage: cli.sh NAME WINDOW SPEC ELF [INPUT]    (SPEC committed | foreign; ELF s03)
set -u
name=$1; w=$2; sp=$3; el=$4; in=${5:-}
O=/cache/cli/$name; rm -rf "$O"; mkdir -p "$O"
case $sp in
  committed) spec=examples/sentinel-03.json;;
  foreign) spec=/cache/boards/s03/sentinel-03-foreign.json;;
  *) spec=$sp;;
esac
case $el in s03) elf=/cache/boards/s03/arducopter.elf;; *) elf=$el;; esac
A=${A:-target/debug/aethelgard}
G=/cache/cli/gen-$(basename "$spec" .json)
[ -f "$G/ok" ] || { rm -rf "$G"; $A generate --spec "$spec" --out "$G" > "$O/generate.json" 2>&1 && touch "$G/ok"; }
plat=$(ls "$G"/board/*.repl 2>/dev/null | head -1); [ -n "$plat" ] || plat=/nonexistent.repl
{ echo "spec=$spec $(sha256sum "$spec" | cut -c1-16) elf=$(sha256sum "$elf" | cut -c1-16) input=${in:+$(sha256sum "$in" | cut -c1-16)}"
  echo "nproc=$(nproc) affinity=$(taskset -pc $$ | sed 's/.*: //') A=$A"; } > "$O/env.txt"
( while :; do echo "$(date +%s.%N) $(cut -d' ' -f1-4 /proc/loadavg)"; sleep 1; done ) > "$O/load.txt" &
lp=$!
extra=(); [ -n "$in" ] && extra=(--input "$in"); [ "${ARCH:-0}" = 1 ] && { mkdir -p "$O/archive"; extra+=(--archive "$O/archive"); }
t0=$(date +%s.%N)
$A simulate --platform "$plat" --spec "$spec" --firmware "$elf" --foreign "$w" "${extra[@]}" \
  > "$O/verdict.json" 2> "$O/stderr.txt"
rc=$?
t1=$(date +%s.%N); kill $lp
python3 -I /spike/py/row.py "$O" "$rc" "$t0" "$t1" "$w" "$name" "$sp" "${in:+input}" | tee "$O/row.json"
