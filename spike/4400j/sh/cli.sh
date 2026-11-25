#!/bin/bash
# AET-WO-4400j (1): one `simulate --foreign` by the measurement CLI (main + patches/measure.patch), inside the tools
# image (sh/img.sh; CPUS=0-3 pins it). Records the verdict, the kept log (AET4400J_DUMP_LOG), wall time, the host's load
# each second, and the env. Unperturbed: no capture loop, no probe. These runs carry every margin.
# Usage: cli.sh NAME WINDOW BOARD [INPUT]     BOARD s03 (main's, 97afa171) | s03h (4400h's spec 135c96fa, ELF dbf598d8)
#   | s02 (sentinel-02-own and 4400h's copy of its CI ELF 5eace4ca: CI's `positive` case, the CI/Furnace wall ratio)
#   env HOLD=1: Renode's stdin a pipe held open and never written (AET4400J_HOLDIN), in place of main's /dev/null.
#   env LIFT=1: wall 600 s, RSS 8,000,000 KiB, log 200,000,000 B (read past the bounds); unset: main's bounds.
set -u
name=$1; w=$2; board=$3; in=${4:-}
O=/cache/cli/$name; rm -rf "$O"; mkdir -p "$O"
case $board in
  s03) spec=/cache/boards/s03/sentinel-03-foreign.json; elf=/cache/boards/s03/arducopter.elf;;
  s03h) spec=/cache/boards/s03h/sentinel-03-foreign.json; elf=/cache/boards/s03h/arducopter.elf;;
  s02) spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json; elf=/cache/boards/s02/arducopter.elf;;   # CI's foreign case, for the CI ratio
  *) echo "board?" >&2; exit 64;;
esac
A=target/debug/aethelgard
G=/cache/cli/gen-$board
[ -f "$G/ok" ] || { rm -rf "$G"; $A generate --spec "$spec" --out "$G" > /dev/null && touch "$G/ok" || exit 1; }
[ "${HOLD:-0}" = 1 ] && export AET4400J_HOLDIN=1   # Renode's stdin a held-open pipe, not /dev/null (the fix, measured)
if [ "${LIFT:-0}" = 1 ]; then export AET4400J_WALL_S=600 AET4400J_RSS_KIB=8000000 AET4400J_LOG_BYTES=200000000; fi
export AET4400J_DUMP_LOG=$O/log.txt
{ env | grep '^AET4400J_' ; echo "nproc=$(nproc) affinity=$(taskset -pc $$ | sed 's/.*: //')"; } > "$O/env.txt"
( while :; do echo "$(date +%s.%N) $(cut -d' ' -f1-4 /proc/loadavg)"; sleep 1; done ) > "$O/load.txt" &
lp=$!
extra=(); [ -n "$in" ] && extra=(--input "$in")
t0=$(date +%s.%N)
$A simulate --platform "$(ls "$G"/board/*.repl | head -1)" --spec "$spec" --firmware "$elf" --foreign "$w" "${extra[@]}" \
  > "$O/verdict.json" 2> "$O/stderr.txt"
rc=$?
t1=$(date +%s.%N); kill $lp
gzip -1 -f "$O/log.txt" 2>/dev/null
python3 -I /spike/py/row.py "$O" "$rc" "$t0" "$t1" "$w" "$name" "$board" "${in:+input}" | tee "$O/row.json"
