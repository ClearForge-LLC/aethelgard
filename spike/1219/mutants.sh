#!/usr/bin/env bash
# AET-WO-1219: one mutant per guard this spike adds, each killed by its own
# message (not by an unrelated failure). Run after run.sh. Scratch only.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd); repo=$(cd "$here/../.." && pwd)
S=${AET_1219_SCRATCH:-$HOME/.cache/aet-1219}/mut; rm -rf "$S"; mkdir -p "$S"
R="$here/reader.py"; D=${AET_1219_SCRATCH:-$HOME/.cache/aet-1219}/dump.jsonl
survived=0
kill() {  # name, expected message, reader args...
  local name=$1 want=$2; shift 2
  local got; got=$(python3 -I "$R" --repo "$repo" "$@" 2>&1)  # exits 1 when it refuses
  if grep -qF -- "$want" <<< "$got"; then echo "killed  $name"; else echo "SURVIVED $name"; survived=1; fi
}
# M1: a key serde writes that the source side lacks (planted in the dump).
python3 -I - "$D" "$S/dump-m1.jsonl" <<'EOF'
import json, sys
out, done = [], False
for l in open(sys.argv[1]):
    r = json.loads(l)
    if not done and r.get("dump", {}).get("schema_version") == 27:
        r["dump"]["mcu"]["core_supply"]["planted_key"] = 1; done = True
    out.append(json.dumps(r))
open(sys.argv[2], "w").write("\n".join(out) + "\n")
EOF
mkdir -p "$S/o1"; kill "M1 dump key not in source" "DUMP KEY NOT IN SOURCE" --dump "$S/dump-m1.jsonl" --out "$S/o1"
# M2: a source field renamed, so the committed files' key is no longer known.
mkdir -p "$S/src" "$S/o2"; cp "$repo"/crates/aethelgard-ir/src/*.rs "$S/src/"
python3 -I - "$S/src/lib.rs" <<'EOF'
import sys
p = sys.argv[1]; s = open(p).read()
old = "pub struct CoreSupply {\n    pub mode:"
assert old in s
open(p, "w").write(s.replace(old, "pub struct CoreSupply {\n    #[serde(rename = \"mode_planted\")]\n    pub mode:"))
EOF
kill "M2 file key not in source" "FILE KEY NOT IN SOURCE" --src "$S/src" --dump "$D" --out "$S/o2"
# M3: one entry left unclassed.
mkdir -p "$S/m3/out"; cp "$here/out/inventory.json" "$S/m3/out/"
grep -v -P "^lib:McuWire.bor_level\t" "$here/classes.tsv" > "$S/m3/classes.tsv"; cp "$here/citations.tsv" "$S/m3/"
kill "M3 unclassed entry" "UNCLASSED K lib:McuWire.bor_level" --dump "$D" --out "$S/m3/out"
# M4: a second chip named in a reason.
mkdir -p "$S/m4/out"; cp "$here/citations.tsv" "$S/m4/"
sed 's/^\(lib:McuWire.part\t[^\t]*\t[^\t]*\t[^\t]*\t[^\t]*\t\)/\1unlike the RP2350, /' "$here/classes.tsv" > "$S/m4/classes.tsv"
kill "M4 second-chip fact in a reason" "A SECOND-CHIP FACT IN A REASON: lib:McuWire.part" --dump "$D" --out "$S/m4/out"
# M5: a held ST fact loses its resolved citation.
mkdir -p "$S/m5/out"; cp "$here/classes.tsv" "$S/m5/"
grep -v -P "^H:lib:MAX_BOR_LEVEL\t" "$here/citations.tsv" > "$S/m5/citations.tsv"
kill "M5 held data with no page" "HELD DATA WITH NO PAGE (RULES §3, WO §1.2): H:lib:MAX_BOR_LEVEL" --dump "$D" --out "$S/m5/out"
# M6: a page-less row citation of a held table is left unresolved.
mkdir -p "$S/m6/out"; cp "$here/classes.tsv" "$S/m6/"
grep -v -P "^row\|power-parts.json\|MURATA_GCM188R71H104KA57\|Rated Value table, capacitance\t" "$here/citations.tsv" > "$S/m6/citations.tsv"
kill "M6 page-less row unresolved" "A PAGE-LESS ROW CITATION NOT IN citations.tsv: row|power-parts.json|MURATA_GCM188R71H104KA57" --dump "$D" --out "$S/m6/out"
exit $survived
