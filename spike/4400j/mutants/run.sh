#!/bin/bash
# AET-WO-4400j: one mutant at a time, inside the tools image at /repo (the branch). Usage (inside):
#   run.sh FILE 'python3 replacement: OLD' 'NEW' 'cargo test args'   -> prints KILLED or SURVIVED; FILE restored and touched
set -u
f=$1; old=$2; new=$3; t=$4
cp "$f" /tmp/mutant.orig
python3 -I -c 'import sys; p, a, b = sys.argv[1:4]; s = open(p).read(); assert s.count(a) == 1, "not once: " + a; open(p, "w").write(s.replace(a, b))' "$f" "$old" "$new" || { echo "MUTANT NOT APPLIED"; exit 3; }
if cargo test --offline --locked $t > /tmp/mutant.log 2>&1; then r=SURVIVED; else
  grep -q "error\[E" /tmp/mutant.log && r=UNCOMPILED || r=KILLED; fi
cp /tmp/mutant.orig "$f"; touch "$f"
echo "$r: $f :: $old -> $new"; grep -E "^test .* FAILED|panicked" /tmp/mutant.log | head -3
