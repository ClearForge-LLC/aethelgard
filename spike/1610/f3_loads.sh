#!/usr/bin/env bash
# AET-WO-1610 §1.1, F3 (power.loads[].reference): what the supply gate's
# "every drawn sink is named" rule (eda supply.rs, SupplySinkUncounted) judges
# when it runs. It runs only under a drop limit, so each committed power
# specification is given a limit on every domain, emitted and extracted.
# Usage: bash spike/1610/f3_loads.sh <aethelgard> <scratch dir>
set -uo pipefail
bin=$1; out=$2
for f in crates/aethelgard-cli/tests/fixtures/sentinel-03.json crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json; do
  n=$(basename "$f" .json); d="$out/$n"; rm -rf "$d"; mkdir -p "$d"
  python3 -I - "$f" "$d/spec.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
p = s["power"]
p["drop_limits"] = [{"domain": r["output"], "max_millivolts": 5000, "source": "AET-WO-1610 spike: a limit no drop reaches"} for r in p["regulators"]]
json.dump(s, open(sys.argv[2], "w"), indent=2)
PY
  echo "== $n"
  "$bin" validate < "$d/spec.json" | cut -c1-300
  "$bin" emit --file "$d/spec.json" --out "$d/out" | cut -c1-300
  pcb=$(ls "$d"/out/*.kicad_pcb 2>/dev/null | head -1)
  [ -n "$pcb" ] && "$bin" extract --board "$pcb" --spec "$d/spec.json" | python3 -I -c '
import json, sys
v = json.loads(sys.stdin.readline())
print(json.dumps({k: v.get(k) for k in ("ok", "verdict", "error", "ran")})[:1500])'
done
