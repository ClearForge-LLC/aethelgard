#!/usr/bin/env bash
# AET-WO-1313 §1.4 on the fixture: every gate (run.sh) on 20 renamings (rename_fixture.py, seeds
# 0-19; RENAMER=rename_perm.py for the length-keeping renaming) under one placement option.
# Usage: spike/1313/renamings.sh <opt> <spec.json> <outdir>
# Prints one line: option, spec, renamings passing all eight gates, and per seed
# outline:status:extract findings.
set -u
O=$1; SPEC=$2; R=$(realpath -m "$3"); HERE=$(dirname "$(realpath "$0")")
mkdir -p "$R"
pass=0; fit=0; lines=""
for s in $(seq 0 19); do
  python3 -I "$HERE/${RENAMER:-rename_fixture.py}" "$SPEC" "$s" "$R/spec-$s.json"
  r=$(AET_ENV="AET_1313_OPT=$O AET_1313_BOUND=100,100 AET_1313_DUMP=$R/l-$s.json AET_1313_ROUTE=$R/r-$s.json" \
      bash "$HERE/run.sh" "$R/spec-$s.json" "$R/run-$s")
  o=$(python3 -I -c 'import json,sys; o=json.load(open(sys.argv[1]))["outline"]; w,h=(o[2]-o[0])/1e6,(o[3]-o[1])/1e6; print("%.2fx%.2f%s"%(w,h,"" if w<=100 and h<=100 else "(over)"))' "$R/l-$s.json" 2>/dev/null)
  case "$o" in *over*|"") ;; *) fit=$((fit+1));; esac
  x=$(python3 -I -c 'import json,sys; x=json.load(open(sys.argv[1])); print(",".join(f["message"].split(":")[0].strip("`").split("_",1)[-1]+"="+f["message"].split("bound ")[1].split(" ")[0] for f in x.get("findings",[])))' "$R/run-$s/8-extract.json" 2>/dev/null)
  case "$r" in *"1-validate=0 2-generate=0 3-emit=0 4-erc=0 5-drc=0 6-drc-parity=0 7-support=0 8-extract=0"*) pass=$((pass+1)); st=ok;; *) st=$(echo "$r" | grep -o '[0-9]-[a-z-]*=[1-9]' | tr '\n' ',');; esac
  lines="$lines s$s:$o:$st:$x;"
done
echo "$O $(basename "$SPEC") fit-100x100 $fit/20 all-gates $pass/20 |$lines"
