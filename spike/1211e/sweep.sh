#!/usr/bin/env bash
# AET-WO-1211e §2 (from 1211a §1.1): validate, solve, emit and generate every committed specification with one
# aethelgard binary, into one output directory. Run once with main's binary on the files as committed
# (the baseline) and once with the branch's binary on the migrated files; compare.py judges the pair.
# Usage: run.sh <aethelgard binary> <repository root> <out dir>
set -euo pipefail
bin=$1 repo=$2 out=$3
mkdir -p "$out"
cd "$repo"
python3 -I - "$repo" > "$out/specs.txt" <<'PY'
import json, subprocess, sys
repo = sys.argv[1]
for f in subprocess.run(["git", "-C", repo, "ls-files", "*.json"], capture_output=True, text=True,
                        check=True).stdout.split():
    if f.startswith("spike/"):
        continue
    try:
        d = json.load(open(f"{repo}/{f}"))
    except (ValueError, UnicodeDecodeError):
        continue
    if isinstance(d, dict) and all(k in d for k in ("schema_version", "mcu", "motor_outputs", "project_name")):
        print(f)
PY
while read -r f; do
  d="$out/${f//\//__}"
  mkdir -p "$d"
  set +e
  "$bin" validate --file "$f" > "$d/validate.out" 2>&1; echo $? > "$d/validate.exit"
  "$bin" solve --file "$f" > "$d/solve.out" 2>&1; echo $? > "$d/solve.exit"
  "$bin" emit --file "$f" --out "$d/emit" > "$d/emit.out" 2>&1; echo $? > "$d/emit.exit"
  "$bin" generate --spec "$f" --out "$d/generate" > "$d/generate.out" 2>&1; echo $? > "$d/generate.exit"
  set -e
done < "$out/specs.txt"
wc -l < "$out/specs.txt"
