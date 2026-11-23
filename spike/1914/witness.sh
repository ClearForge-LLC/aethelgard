#!/usr/bin/env bash
# AET-WO-1914 §1.1: the reconstruction against libtest's own clock. libtest prints `test X has been running for over 60
# seconds` 60 s after X starts, so that line's stamp less 60 is X's true start; P3's start is X's end less its seconds.
#   witness.sh STAMPED THREADS   ->  binary<TAB>test<TAB>true-start<TAB>P3-start<TAB>difference (s)
set -euo pipefail
export LC_ALL=C
shard="$(dirname "$0")/../../ci/shard.sh"
bash "$shard" times "$1" "$2" >"$1.times"
awk -F'\t' -v tf="$1.times" '
  FILENAME == tf { d[$1 "\t" $2] = $3; next }
  { t = $1 + 0; line = $2 }
  line ~ /^ +Running / { split(line, f, " "); b = f[2]; if (f[3] ~ /^src\//) b = f[2] " " f[3] }
  match(line, /test [A-Za-z0-9_:]+ has been running for over 60 seconds$/) { x = substr(line, RSTART + 5); sub(/ .*/, "", x); warn[b "\t" x] = t - 60 }
  match(line, /test [A-Za-z0-9_:]+( - should panic)? [.][.][.] (ok|FAILED|ignored)$/) { x = substr(line, RSTART + 5); sub(/ .*/, "", x)
    k = b "\t" x; if (k in warn) printf "%s\t%.3f\t%.3f\t%.3f\n", k, warn[k], t - d[k], t - d[k] - warn[k] }
' "$1.times" "$1"
rm -f "$1.times"
