#!/usr/bin/env bash
# Two sweep.sh directories, byte for byte. Every emitted file and exit code is compared raw; each
# `*.out` (the JSON a command prints) is compared with its own output directory's path replaced by
# `<OUT>`, since `emit` prints where it wrote. Prints each difference and exits 1 on any.
set -euo pipefail
a=$1 b=$2 bad=0
(cd "$a" && find . -type f | sort) > "${TMPDIR:-/tmp}/.same-a.$$"; (cd "$b" && find . -type f | sort) > "${TMPDIR:-/tmp}/.same-b.$$"
if ! diff -q "${TMPDIR:-/tmp}/.same-a.$$" "${TMPDIR:-/tmp}/.same-b.$$" >/dev/null; then diff "${TMPDIR:-/tmp}/.same-a.$$" "${TMPDIR:-/tmp}/.same-b.$$" || true; bad=1; fi
n=0
while read -r f; do
  [ -f "$b/$f" ] || continue
  n=$((n+1))
  case "$f" in
    *.out) cmp -s <(sed "s#$a#<OUT>#g" "$a/$f") <(sed "s#$b#<OUT>#g" "$b/$f") || { echo "differs: $f"; bad=1; } ;;
    *) cmp -s "$a/$f" "$b/$f" || { echo "differs: $f"; bad=1; } ;;
  esac
done < "${TMPDIR:-/tmp}/.same-a.$$"
rm -f "${TMPDIR:-/tmp}/.same-a.$$" "${TMPDIR:-/tmp}/.same-b.$$"
[ "$bad" = 0 ] && echo "identical: $n files"
exit "$bad"
