#!/usr/bin/env bash
# Part 1: validate/solve/emit/extract over every committed spec. Outputs to scratch.
cd /home/forge/aethelgard-4300k
A=./target/debug/aethelgard
O=/home/forge/.cache/aet-4300k/blast
bash spike/4300k/universe.sh | while IFS=$'\t' read -r n f; do
  d=$O/$n; mkdir -p $d
  $A validate --file $f >$d/validate.out 2>$d/validate.err; echo $? >$d/validate.rc
  $A solve --file $f >$d/solve.out 2>$d/solve.err; echo $? >$d/solve.rc
  rm -rf $d/emit
  $A emit --file $f --out $d/emit >$d/emit.out 2>$d/emit.err; echo $? >$d/emit.rc
  pn=$(python3 -I -c 'import json,sys;print(json.load(open(sys.argv[1]))["project_name"])' $f)
  $A extract --board $d/emit/$pn.kicad_pcb --spec $f >$d/extract.out 2>$d/extract.err; echo $? >$d/extract.rc
done
