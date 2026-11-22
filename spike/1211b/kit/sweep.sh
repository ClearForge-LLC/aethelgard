#!/bin/bash
# Inside the tools image: for every committed specification (any schema), validate, solve, generate
# and build with the CLI at $A, into /cache/$TAG/<name>/. Each step's exit and output are kept.
# Rows: spec, spec sha, exits, ELF sha. Usage: sweep.sh TAG
set -u
TAG=$1; A=${A:-$CARGO_TARGET_DIR/debug/aethelgard}
out=/cache/$TAG; mkdir -p "$out"; : > "$out/rows.tsv"
for s in $(python3 -I -c "
import json,glob
for p in sorted(glob.glob('**/*.json',recursive=True)):
    if p.startswith(('spike/','target/','fab/','docs/')) or '/target/' in p: continue
    try: d=json.load(open(p))
    except Exception: continue
    if isinstance(d,dict) and isinstance(d.get('mcu'),dict) and 'schema_version' in d: print(p)"); do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//'); d=$out/$n; rm -rf "$d"; mkdir -p "$d"
  sha=$(sha256sum "$s" | cut -c1-64)
  $A validate --file "$s" > "$d/validate.json" 2>&1; v=$?
  $A solve --file "$s" > "$d/solve.json" 2>"$d/solve.err"; so=$?
  $A generate --spec "$s" --out "$d/gen" > "$d/generate.json" 2>&1; g=$?
  b=-; elf=-
  if [ $g -eq 0 ]; then
    $A build --crate "$d/gen/board" --out "$d/fw.elf" > "$d/build.json" 2>&1; b=$?
    [ $b -eq 0 ] && elf=$(sha256sum "$d/fw.elf" | cut -c1-64)
  fi
  printf '%s\t%s\tvalidate=%s\tsolve=%s\tgenerate=%s\tbuild=%s\telf=%s\n' "$s" "$sha" "$v" "$so" "$g" "$b" "$elf" | tee -a "$out/rows.tsv"
done
