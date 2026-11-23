#!/bin/bash
# AET-WO-2309 §1.1 sweep A: every committed archive (`git ls-files '*/trace.cbor'`), replayed by the CLI
# given, one line per archive: its directory, the exit status and the replay's whole output.
# Usage: sweep-a.sh CLI (run from the repository root)
set -u
for t in $(git ls-files '*/trace.cbor'); do
  d=$(dirname "$t"); out=$("$1" replay --archive "$d" 2>&1); echo "$d rc=$? $out"
done
