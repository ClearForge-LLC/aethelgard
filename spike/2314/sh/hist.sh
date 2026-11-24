#!/bin/bash
# AET-WO-2314 §1.1 (3): a measure.sh run's USART access lines, counted by source, op and register,
# and their bytes; the TDR writes apart. Usage: hist.sh OUT.txt
set -u
LC_ALL=C grep -a '^\[' "$1" | LC_ALL=C awk '
  match($0, /\] \[INFO\] (usart[0-9]+|uart[0-9]+): \[cpu: [^]]*\] (Read|Write)[A-Za-z0-9]+ (to|from) 0x[0-9A-F]+ \([A-Za-z0-9_+]+\)/) {
    s = substr($0, RSTART, RLENGTH); n = split(s, w, " ")
    k = w[3] " " w[6] " " w[8] " " w[9]; c[k]++; b[k] += length($0) + 1; tot++; tb += length($0) + 1; next }
  { other++; ob += length($0) + 1 }
  END { for (k in c) printf "ANSWER 2314: access %s lines=%d bytes=%d\n", k, c[k], b[k]
        printf "ANSWER 2314: access total lines=%d bytes=%d; other log lines=%d bytes=%d\n", tot, tb, other, ob }' | sort -t= -k2 -nr | head -20
