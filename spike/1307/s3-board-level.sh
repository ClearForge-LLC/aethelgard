#!/bin/bash
# usage: board_level.sh <bin> <order-label>  -> TSV rows level=board
BIN=$1; ORD=$2
F=/home/forge/aethelgard-1307
declare -A IN=([ref]=$F/examples/sentinel-01-minimal.json [D1]=$F/crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json [D3]=$F/crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json)
O=$HOME/.cache/aet-1307/out/bl-$ORD; mkdir -p $O
for k in ref D1 D3; do for s in id $(seq 0 19); do
  AET_RENAME_SEED=$s $BIN emit --file ${IN[$k]} --out $O/x 2>&1 >/dev/null | grep -a '^RENAMED' | sed "s/^/$ORD\tboard\t$k\t/"
done; done
