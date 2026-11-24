#!/bin/bash
S=/cache/sweep-img; O=$S/out; mkdir -p $O
cd /repo && cargo build -q --locked --offline -p aethelgard-cli --features with-tools || { echo BUILD FAIL; exit 1; }
A=$CARGO_TARGET_DIR/debug/aethelgard
echo "start $(date -Is)"
for f in $S/v/*.json; do
  n=$(basename $f .json); d=$O/$n; rm -rf $d; mkdir -p $d
  $A validate --file $f > $d/validate.out 2>$d/validate.err; echo $? > $d/validate.rc
  $A emit --file $f --out $d/e > $d/emit.out 2>$d/emit.err; echo $? > $d/emit.rc
  sch=$(ls $d/e/*.kicad_sch 2>/dev/null | head -1); pcb=$(ls $d/e/*.kicad_pcb 2>/dev/null | head -1)
  if [ -n "$sch" ] && [ -n "$pcb" ]; then
    $A check erc --schematic $sch > $d/erc.out 2>$d/erc.err; echo $? > $d/erc.rc
    $A check drc --board $pcb --spec $f --schematic $sch > $d/drc.out 2>$d/drc.err; echo $? > $d/drc.rc
    $A check support --schematic $sch --board $pcb --spec $f > $d/support.out 2>$d/support.err; echo $? > $d/support.rc
  fi
  rm -rf $d/e
  echo "$n done $(date -Is)"
done
echo "end $(date -Is)"
