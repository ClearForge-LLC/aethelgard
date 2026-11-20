#!/usr/bin/env bash
# AET-WO-1317: hash everything `aethelgard generate` writes for every example,
# every top-level test fixture and the Sentinel-03 variants (dumped by the
# driver into $3), or its refusal. Usage: baseline.sh <aethelgard> <out-dir> <variants-dir>
set -u
bin=$1; out=$2; variants=$3
rm -rf "$out"; mkdir -p "$out/gen"
for spec in examples/*.json crates/aethelgard-cli/tests/fixtures/*.json "$variants"/*.json; do
  name=$(basename "$spec" .json)
  case "$spec" in "$variants"/*) name="v-$name";; examples/*) name="ex-$name";; *) name="fx-$name";; esac
  dir="$out/gen/$name"
  verdict=$("$bin" generate --spec "$spec" --out "$dir" 2>&1)
  code=$?
  if [ -d "$dir" ]; then
    (cd "$dir" && find . -type f | LC_ALL=C sort | xargs -r sha256sum) | sed "s|  \./|  $name/|"
  fi
  echo "$name exit=$code $(printf '%s' "$verdict" | sha256sum | cut -c1-16) $(printf '%s' "$verdict" | head -c 300 | tr '\n' ' ')" >> "$out/verdicts.txt"
done > "$out/files.sha256"
rm -rf "$out/gen"
