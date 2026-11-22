#!/usr/bin/env bash
# AET-WO-1216: the committed boards (PREDICTIONS.md "The universe"), one "<name>\t<path>" per line;
# the name (the file's stem, `solve-` before the solve fixtures') is every output's key.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
git ls-files '*.json' | grep -vE '^(spike|docs)/' | while read -r f; do
  python3 -I -c 2>/dev/null '
import json,sys
s=json.load(open(sys.argv[1]))
ok=isinstance(s,dict) and s.get("schema_version")==24 and isinstance(s.get("mcu"),dict) and "base" not in s
sys.exit(0 if ok else 1)' "$f" 2>/dev/null || continue
  n=$(basename "$f" .json); [ "$(basename "$(dirname "$f")")" = solve ] && n="solve-$n"
  printf '%s\t%s\n' "$n" "$f"
done
