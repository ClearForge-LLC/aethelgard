#!/bin/bash
# AET-WO-4400i: one mutant per guard this work order adds (mutants.tsv), each against the suite it lives in, inside
# the tools image. Each: the exact text replaced once (else BAD), the file touched (a restored file keeps its new
# mtime: cargo rebuilds), the tests run; KILLED when they fail, SURVIVED when they pass; then the file restored.
# Usage: run.sh [ID...]  -> one line per mutant
set -u
cd /repo
while IFS=$'\t' read -r id file old new args; do
  case $id in ''|'#'*) continue;; esac
  [ $# -gt 0 ] && ! printf '%s\n' "$@" | grep -qx "$id" && continue
  cp "$file" "/tmp/orig"
  if ! python3 -I - "$file" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:]
s = open(p).read()
if s.count(old) != 1: sys.exit(1)
open(p, "w").write(s.replace(old, new))
PY
  then echo "BAD $id: the text is not in $file once"; continue; fi
  touch "$file"
  if cargo test -q $args >/tmp/out 2>&1; then r=SURVIVED; else
    grep -q 'error\[E\|could not compile' /tmp/out && r=NOCOMPILE || r=KILLED; fi
  cp /tmp/orig "$file"; touch "$file"
  echo "$r $id $file"
done < /spike/mutants/mutants.tsv
