#!/bin/bash
# Compare two sweeps: rows, stdout (out dir normalised) and every file's sha256. Prints the differing names only.
B=$1; A=$2
d=$(diff -rq "$B" "$A" | sed "s#$B/##; s#$A/##")
if [ -z "$d" ]; then echo "IDENTICAL $(wc -l < "$A/SHA256SUMS") files, $(wc -l < "$A/rows.txt") specifications"; else echo "$d" | head -40; echo "DIFFS $(echo "$d" | wc -l)"; fi
