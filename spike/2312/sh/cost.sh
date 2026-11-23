#!/bin/bash
# AET-WO-2312 §1.1: replay against attribute on the cost archives (probe `make`, K buses x 128 devices), by one
# CLI binary ($A), on the host, nice 10, one at a time; attribute capped at $CAP seconds (timeout).
# Usage: A=... CAP=900 cost.sh MADE_DIR
set -u
A=${A:?}; CAP=${CAP:-900}; M=$1
for k in 20 100 200; do for shape in big_pass big_fail; do
  d=$(find $M/k$k/$shape -name manifest.cbor -printf '%h\n'); b=$(du -sb $d | cut -f1)
  for cmd in replay attribute; do
    s=$(date +%s%N); out=$(nice -n 10 timeout $CAP $A $cmd --archive $d 2>&1); rc=$?; e=$(date +%s%N)
    echo "k=$k $shape bytes=$b $cmd rc=$rc wall_ms=$(( (e-s)/1000000 )) $(echo "$out" | head -c 160)"
  done
done; done
