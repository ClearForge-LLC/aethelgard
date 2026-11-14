#!/bin/bash
# usage: spec_level.sh <bin> <label>
BIN=$1; ORD=$2; R=$HOME/.cache/aet-1307/results; F=/home/forge/aethelgard-1307
declare -A IN=([ref]=$F/examples/sentinel-01-minimal.json [D1]=$F/crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json [D3]=$F/crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json)
O=$HOME/.cache/aet-1307/out/sl-$ORD; mkdir -p $O
for k in ref D1 D3; do for s in $(seq 0 19); do
  python3 -I $R/rename_spec.py ${IN[$k]} $s $O/$k-$s.json
  rm -rf $O/o; mkdir $O/o
  $BIN emit --file $O/$k-$s.json --out $O/o > $O/o/v.json 2>$O/o/err
  res=$(python3 -I -c "import json,sys; v=json.load(open(sys.argv[1])); print('ok' if v.get('ok') else (v.get('error',{}).get('kind','')+' '+str(v.get('error',{}).get('id',''))))" $O/o/v.json)
  b=$(ls $O/o/*.kicad_pcb 2>/dev/null|head -1)
  pl=-; [ -n "$b" ] && pl=$(python3 -I $R/placement.py $b)
  printf '%s\tspec\t%s\t%s\t%s\t%s\n' $ORD $k $s "$res" "$pl"
done; done
