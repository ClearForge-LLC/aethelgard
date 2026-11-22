#!/usr/bin/env bash
# AET-WO-1215 §2: every committed JSON file with an `mcu` object (every specification and fixture,
# any version, outside target/ and spike/) through `validate --file` and `solve --file` with one
# binary. One line per (file, command): path, command, exit code, sha256 of stdout.
# Usage: bash spike/1215/differential.sh BINARY REPO > out/differential-<label>.txt
set -u
bin=$1 repo=$2
cd "$repo"
git ls-files '*.json' | grep -v -e '^spike/' -e '/target/' | sort | while read -r f; do
  python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if isinstance(d,dict) and isinstance(d.get("mcu"),dict) else 1)' "$f" 2>/dev/null || continue
  for sub in validate solve; do
    o=$("$bin" "$sub" --file "$f" 2>/dev/null); rc=$?
    printf '%s\t%s\t%s\t%s\n' "$f" "$sub" "$rc" "$(printf '%s' "$o" | sha256sum | cut -c1-16)"
  done
done
