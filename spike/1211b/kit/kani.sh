#!/bin/bash
# Inside the tools image: generate SPEC with the CLI at $A into /cache/$TAG/gen, then prove every
# #[kani::proof] harness of its board crate one at a time; rows: harness, exit, seconds, verdict.
# Usage: kani.sh TAG SPEC
set -u
TAG=$1; SPEC=$2; A=${A:-$CARGO_TARGET_DIR/debug/aethelgard}
out=/cache/$TAG; rm -rf "$out"; mkdir -p "$out"
$A generate --spec "$SPEC" --out "$out/gen" > "$out/generate.json" 2>&1 || { echo "generate failed"; exit 1; }
for h in $(awk '/#\[kani::proof\]/{f=1} f && /fn [a-z0-9_]+/{match($0,/fn [a-z0-9_]+/); print substr($0,RSTART+3,RLENGTH-3); f=0}' "$out/gen/board/src/proofs.rs"); do
  t0=$(date +%s)
  $A prove --crate "$out/gen/board" --harness "$h" --spec "$SPEC" > "$out/$h.json" 2>&1; rc=$?
  t1=$(date +%s)
  v=$(grep -o '"verdict":"[a-z]*"' "$out/$h.json" | head -1)
  printf '%s\t%s\t%s\t%s\n' "$h" "$rc" "$((t1-t0))" "$v" | tee -a "$out/rows.tsv"
done
