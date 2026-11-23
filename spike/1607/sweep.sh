#!/usr/bin/env bash
# AET-WO-1607 §1.1 and §1.3: every committed specification through one aethelgard binary, in
# the tools image (KiCad, Kani). Extends spike/1211d/measure/sweep.sh (validate, solve, emit)
# with the verdicts (6) touches: check erc, check drc with and without --spec, check support,
# generate, and prove of the first harness with and without --spec. same.sh-style comparison:
# `diff -r` of two output directories.
# Usage: sweep.sh <aethelgard binary> <a git archive of one commit> <out dir> [--no-prove]
set -euo pipefail
bin=$1 repo=$2 out=$3 prove=${4:-}
mkdir -p "$out"
cd "$repo"
python3 -I - "$repo" > "$out/specs.txt" <<'PY'
import json, sys
repo = sys.argv[1]
import os
# The tree is a `git archive` of one commit, so every file in it is committed.
files = sorted(os.path.relpath(os.path.join(r, n), repo) for r, ds, ns in os.walk(repo)
               for n in ns if n.endswith(".json"))
for f in files:
    if f.startswith(("spike/", "target/")):
        continue
    try:
        d = json.load(open(f"{repo}/{f}"))
    except (ValueError, UnicodeDecodeError):
        continue
    if isinstance(d, dict) and all(k in d for k in ("schema_version", "mcu", "motor_outputs", "project_name")):
        print(f)
PY
# Runs one command, keeping stdout, stderr and the exit code apart.
step() {
  local d=$1 name=$2; shift 2
  set +e
  "$@" > "$d/$name.out" 2> "$d/$name.err"; echo $? > "$d/$name.exit"
  set -e
}
while read -r f; do
  d="$out/${f//\//__}"
  mkdir -p "$d"
  step "$d" validate "$bin" validate --file "$f"
  step "$d" solve "$bin" solve --file "$f"
  step "$d" emit "$bin" emit --file "$f" --out "$d/emit"
  if [ "$(cat "$d/emit.exit")" = 0 ]; then
    p=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["project_name"])' "$f")
    sch="$d/emit/$p.kicad_sch" pcb="$d/emit/$p.kicad_pcb"
    step "$d" erc "$bin" check erc --schematic "$sch"
    step "$d" drc-spec "$bin" check drc --board "$pcb" --spec "$f" --schematic "$sch"
    step "$d" drc "$bin" check drc --board "$pcb"
    step "$d" support "$bin" check support --schematic "$sch" --board "$pcb" --spec "$f"
  fi
  step "$d" generate "$bin" generate --spec "$f" --out "$d/gen"
  if [ "$prove" != --no-prove ] && [ "$(cat "$d/generate.exit")" = 0 ]; then
    h=$(grep -A3 -F '#[kani::proof]' "$d/gen/board/src/proofs.rs" | grep -oE '^\s*fn [a-z0-9_]+' | head -1 | awk '{print $2}')
    step "$d" prove-spec "$bin" prove --crate "$d/gen/board" --harness "proofs::$h" --spec "$f"
    step "$d" prove "$bin" prove --crate "$d/gen/board" --harness "proofs::$h"
  fi
done < "$out/specs.txt"
wc -l < "$out/specs.txt"
