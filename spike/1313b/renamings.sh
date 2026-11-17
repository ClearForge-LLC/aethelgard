#!/usr/bin/env bash
# AET-WO-1313b: every gate (run.sh) on 20 renamings of a specification (seeds 0-19) under one
# method (AET_1313B letters; "-" is main's placer). RENAMER=rename_perm.py keeps reference lengths
# (order only); the default rename_fixture.py is 1307's prefix renamer carried to connectors and
# test points (parts grow). Usage: spike/1313b/renamings.sh <method> <spec.json> <outdir> [base-sig]
# Prints: method, spec, fits 100x100, every gate, signatures equal to base-sig, and per seed
# outline:status:least extract slack (pF).
set -u
O=$1; SPEC=$2; R=$(realpath -m "$3"); BASE=${4:-}; HERE=$(dirname "$(realpath "$0")")
M=$O; [ "$O" = - ] && M=
mkdir -p "$R"
pass=0; fit=0; same=0; lines=""
for s in $(seq 0 19); do
  python3 -I "$HERE/${RENAMER:-rename_fixture.py}" "$SPEC" "$s" "$R/spec-$s.json"
  r=$(AET_ENV="AET_1313B=$M AET_1313B_DUMP=$R/l-$s.json" bash "$HERE/run.sh" "$R/spec-$s.json" "$R/run-$s")
  o=$(python3 -I -c 'import json,sys; o=json.load(open(sys.argv[1]))["outline"]; w,h=(o[2]-o[0])/1e6,(o[3]-o[1])/1e6; print("%.2fx%.2f%s"%(w,h,"" if w<=100 and h<=100 else "(over)"))' "$R/l-$s.json" 2>/dev/null)
  case "$o" in *over*|"") ;; *) fit=$((fit+1));; esac
  if [ -n "$BASE" ] && [ -f "$R/l-$s.json" ] && [ "$(python3 -I "$HERE/sig.py" "$R/l-$s.json")" = "$BASE" ]; then same=$((same+1)); fi
  x=$(python3 -I -c '
import json,sys
x=json.load(open(sys.argv[1]))
print("%.1f"%min((l["declared_femtofarads"]-l["c_bound_femtofarads"])/1000 for l in x["lines"]))' "$R/run-$s/8-extract.json" 2>/dev/null)
  case "$r" in *"1-validate=0 2-generate=0 3-emit=0 4-erc=0 5-drc=0 6-drc-parity=0 7-support=0 8-extract=0"*) pass=$((pass+1)); st=ok;; *) st=$(echo "$r" | grep -o '[0-9]-[a-z-]*=[1-9]' | tr '\n' ',');; esac
  lines="$lines s$s:$o:$st:$x;"
done
echo "${O} $(basename "$SPEC") fit $fit/20 all-gates $pass/20 same-signature $same/20 |$lines"
