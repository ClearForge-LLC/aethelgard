#!/usr/bin/env bash
# AET-WO-4410 (from 4300j): committed boards, "<name>\t<path>" per line. Schema constant read from the IR crate.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
V=$(sed -n 's/^pub const SPECIFICATION_SCHEMA_VERSION: u32 = \([0-9]*\);.*/\1/p' crates/aethelgard-ir/src/lib.rs)
[ -n "$V" ] || { echo "no schema constant" >&2; exit 1; }
git ls-files '*.json' | grep -vE '^(spike|docs)/' | while read -r f; do
  python3 -I -c 2>/dev/null '
import json,sys
s=json.load(open(sys.argv[1]))
ok=isinstance(s,dict) and s.get("schema_version")==int(sys.argv[2]) and isinstance(s.get("mcu"),dict) and "base" not in s
sys.exit(0 if ok else 1)' "$f" "$V" 2>/dev/null || continue
  n=$(basename "$f" .json); [ "$(basename "$(dirname "$f")")" = solve ] && n="solve-$n"
  printf '%s\t%s\n' "$n" "$f"
done
