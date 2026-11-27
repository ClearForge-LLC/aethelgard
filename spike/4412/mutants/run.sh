#!/bin/bash
# AET-WO-4412: one mutant per guard this WO adds, each against the tests it lives in (CLAUDE.md: sized to the diff).
# Usage: run.sh FILE 'python replace old' 'new' 'cargo test args...'   -> KILLED or SURVIVED; the file restored and touched.
set -u
f=$1; old=$2; new=$3; shift 3
cp "$f" "$f.orig"
python3 - "$f" "$old" "$new" <<'PY' || { mv "$f.orig" "$f"; touch "$f"; echo "MUTANT NOT APPLIED"; exit 2; }
import sys
p, a, b = sys.argv[1:]
s = open(p).read()
assert s.count(a) == 1, "old text not unique"
open(p, "w").write(s.replace(a, b))
PY
touch "$f"
if cargo test -q "$@" > /dev/null 2>&1; then r=SURVIVED; else r=KILLED; fi
mv "$f.orig" "$f"; touch "$f"
echo "$r"
