#!/bin/bash
# P3d: every committed specification's coverage before any gate, as main's CLI reports it with
# draws_a_board skipped (coverage-d1.sh's binary, so an undrawn board is reached too): each spec is
# copied byte for byte into a scratch directory beside a baseline scenario, and verify --fab-out
# refuses CampaignCoverageShort listing every unmet target with its reason (or refuses earlier).
# Rows: spec, exit, kind, the unmet list. Output ~/.cache/aet-1211c/m/rails.tsv.
set -u
K=$(cd "$(dirname "$0")" && pwd); R=$(cd "$K/../../.." && pwd); C=$HOME/.cache/aet-1211c
BIN=${BIN:-$C/cov-7fc81640-aethelgard}; W=$C/m/sweep; rm -rf "$W"; mkdir -p "$W"; : > "$C/m/rails.tsv"
cd "$R"
for s in $(python3 -I -c "
import json,glob
for p in sorted(glob.glob('**/*.json',recursive=True)):
    if p.startswith(('spike/','target/','fab/','docs/')) or '/target/' in p: continue
    try: d=json.load(open(p))
    except Exception: continue
    if isinstance(d,dict) and isinstance(d.get('mcu'),dict) and 'schema_version' in d: print(p)"); do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//'); d=$W/$n; mkdir -p "$d"; cp "$s" "$d/spec.json"
  printf '{"schema_version": 1, "name": "baseline", "base": "spec.json", "expect": "pass", "expect_findings": [], "overrides": []}\n' > "$d/baseline.json"
  (cd "$d" && "$BIN" verify --spec spec.json --scenarios baseline.json --fab-out fab > out.json 2>&1); rc=$?
  python3 -I -c '
import json,sys
v=json.loads(open(sys.argv[1]).read().strip().splitlines()[-1]); e=v.get("error") or {}
print("\t".join([sys.argv[2], sys.argv[3], e.get("kind") or v.get("verdict",""), (e.get("message") or "").replace("no package: the campaign covers no fault on ","")]))' "$d/out.json" "$s" "$rc" >> "$C/m/rails.tsv"
done
