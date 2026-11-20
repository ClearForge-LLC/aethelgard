#!/usr/bin/env bash
# AET-WO-1211d §1.1: validate, solve and emit every committed specification with one aethelgard
# binary into one directory; same.sh judges two such directories byte for byte.
# Usage: sweep.sh <aethelgard binary> <repository root> <out dir>
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
  set -e
done < "$out/specs.txt"
wc -l < "$out/specs.txt"
