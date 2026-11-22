#!/bin/bash
# Host: generate every committed spec with BIN into OUT, diff each tree against the baseline's
# (~/.cache/aet-1211b/base/<name>/gen). Prints one line per spec: same / DIFF / exit codes.
set -u
BIN=$1; OUT=$2; BASE=${BASE:-$HOME/.cache/aet-1211b/base}
rm -rf "$OUT"; mkdir -p "$OUT"
cut -f1 "$BASE/rows.tsv" | while read -r s; do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//')
  "$BIN" generate --spec "$s" --out "$OUT/$n" > "$OUT/$n.json" 2>&1; g=$?
  if [ -d "$BASE/$n/gen" ] || [ -d "$OUT/$n" ]; then
    if diff -r -q "$BASE/$n/gen" "$OUT/$n" > "$OUT/$n.diff" 2>&1; then echo "same $g $s"; else echo "DIFF $g $s"; fi
  else echo "none $g $s"; fi
done
