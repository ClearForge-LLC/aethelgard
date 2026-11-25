#!/bin/bash
# AET-WO-1918, inside the image at /repo: every specification the repo holds, generated, and each
# board crate's `check_final_<part>` statement counts (the bound (S) would split on). A JSON
# that does not generate (not a specification, or refused) is listed with its exit code.
# usage: bash count-finals.sh   (prints `<spec>\t<part>\t<statements>`)
set -u
A=/repo/target/debug/aethelgard
O=/repo/target/gen-all; rm -rf "$O"; mkdir -p "$O"
find examples crates/*/templates crates/*/tests/fixtures -name '*.json' | sort | while read -r f; do
  n=$(echo "$f" | tr '/' '_' | sed 's/\.json$//')
  "$A" generate --spec "$f" --out "$O/$n" > /dev/null 2>&1; rc=$?
  p="$O/$n/board/src/proofs.rs"
  if [ "$rc" -ne 0 ] || [ ! -f "$p" ]; then echo -e "$f\t-\tgenerate=$rc"; continue; fi
  awk -v f="$f" '/^fn check_final_/{n=$2; sub(/\(.*/,"",n); c=0; on=1; next}
    on&&/^}/{print f "\t" n "\t" c; on=0}
    on&&/^    [^ ].*;[[:space:]]*$/{c++}
    on&&/^    [^ /].*\{[[:space:]]*$/{c++}' "$p"
done
