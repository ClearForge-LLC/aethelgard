#!/bin/bash
# AET-WO-1314b §1.4 (no committed artifact moves): every committed specification outside spike/
# (examples/, the test fixtures, docs repros) is emitted by the base binary and by this branch's,
# and both boards are compared; each emitted board is then extracted by both binaries against its
# own specification, and the two extract lines compared byte for byte. One step at a time, each
# under a capped scope (MemoryMax=8G, no swap) at nice -n 10.
# Usage: spike/1314b/differential.sh <base binary> <new binary>
# Output: ~/.cache/aet-1314b/diff/rows.tsv: spec, emit exits (base/new), boards equal, extract exits
# (base/new), extract equal, and when not equal whether the only difference is the `cause` field.
set -u
cd "$(dirname "$0")/../.."
B=$1; N=$2
O=$HOME/.cache/aet-1314b/diff
rm -rf "$O"; mkdir -p "$O"
export XDG_RUNTIME_DIR=/run/user/$(id -u)
cap() { systemd-run --user --scope --quiet -p MemoryMax=8G -p MemorySwapMax=0 -- nice -n 10 "$@"; }
SPECS=$(git ls-files '*.json' | grep -v '^spike/' | xargs grep -l '"schema_version"' | xargs grep -l '"project_name"' | xargs grep -l '"net_constraints"')
for s in $SPECS; do
  k=$(echo "$s" | tr '/' '_')
  mkdir -p "$O/$k/base" "$O/$k/new"
  cap "$B" emit --file "$s" --out "$O/$k/base" > "$O/$k/emit-base.json"; eb=$?
  cap "$N" emit --file "$s" --out "$O/$k/new" > "$O/$k/emit-new.json"; en=$?
  pb=$(ls "$O/$k/base"/*.kicad_pcb 2>/dev/null | head -1); pn=$(ls "$O/$k/new"/*.kicad_pcb 2>/dev/null | head -1)
  if [ -z "$pb" ] || [ -z "$pn" ]; then
    cmp -s "$O/$k/emit-base.json" "$O/$k/emit-new.json" && same=refusal-equal || same=refusal-DIFFERS
    printf '%s\t%s/%s\t%s\t-\t-\t-\n' "$s" "$eb" "$en" "$same" | tee -a "$O/rows.tsv"; continue
  fi
  cmp -s "$pb" "$pn" && boards=equal || boards=DIFFER
  cap "$B" extract --board "$pb" --spec "$s" > "$O/$k/extract-base.json"; xb=$?
  cap "$N" extract --board "$pb" --spec "$s" > "$O/$k/extract-new.json"; xn=$?
  if cmp -s "$O/$k/extract-base.json" "$O/$k/extract-new.json"; then x=equal; why=-
  else x=DIFFER
    why=$(python3 -I -c '
import json, sys
b = json.load(open(sys.argv[1])); n = json.load(open(sys.argv[2]))
causes = [f.pop("cause") for f in n.get("findings", []) if "cause" in f]
kinds = sorted({f["kind"] for f in n.get("findings", [])})
print(("only cause x%d on %s" % (len(causes), ",".join(kinds))) if b == n else "OTHER")' "$O/$k/extract-base.json" "$O/$k/extract-new.json")
  fi
  printf '%s\t%s/%s\t%s\t%s/%s\t%s\t%s\n' "$s" "$eb" "$en" "$boards" "$xb" "$xn" "$x" "$why" | tee -a "$O/rows.tsv"
done
