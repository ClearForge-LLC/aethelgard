#!/bin/bash
# AET-WO-2314 §1.1 (spike/1609/sh/foreign.sh, paths moved): `simulate --foreign` on sentinel-02-own,
# as ci/foreign-judge.sh's positive case runs it, on one ELF, with the CLI in /repo/target (or BIN).
#   foreign.sh NAME ELF [WINDOW]   -> /cache/run/NAME/{simulate.json,exit}
set -u
cd /repo
bin=${BIN:-/repo/target/debug/aethelgard}
spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
o=/cache/run/$1; rm -rf "$o"; mkdir -p "$o"
[ -d /cache/run/gen ] || "$bin" generate --spec "$spec" --out /cache/run/gen > /cache/run/generate.json
status=0; start=$(date +%s.%N)
"$bin" simulate --spec "$spec" --platform /cache/run/gen/board/sentinel-02-own.repl \
  --firmware "$2" --foreign "${3:-5000ms}" > "$o/simulate.json" || status=$?
echo "$status" > "$o/exit"
echo "ANSWER 2314: $1 wall_s=$(python3 -c "print(round($(date +%s.%N) - $start, 1))")"
python3 -I spike/2314/sh/verdict.py "$1" "$o/simulate.json" "$status"
