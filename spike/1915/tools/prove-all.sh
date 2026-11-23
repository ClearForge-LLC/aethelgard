#!/bin/bash
# AET-WO-1915: `prove` on its own, one container per harness, so each harness
# has its own scope and its own memory.peak.
# usage: prove-all.sh <board dir> <spec> <rep> [harness...]
# With no harness named, every `#[kani::proof]` fn in <board>/src/proofs.rs.
set -u
T=$HOME/.cache/aet-1915/tree
board=$1 spec=$2 rep=$3; shift 3
hs=("$@")
[ ${#hs[@]} -gt 0 ] || mapfile -t hs < <(awk '/^#\[kani::proof\]/{p=1;next} p&&/^fn /{sub(/^fn /,"");sub(/\(.*/,"");print;p=0}' "$T/$board/src/proofs.rs")
for h in "${hs[@]}"; do
  TAG="prove-$(basename "$board")-$h-r$rep" "$(dirname "$(readlink -f "$0")")/img.sh" "$T" \
    bash /repo/spike/1915/tools/ctl.sh prove "$board" "$spec" "$h"
  rc=$?
  echo "$(date -u +%H:%M:%S) $board $h r$rep exit=$rc"
  [ $rc -eq 3 ] && { echo "prove-all: not started (memory); stopping" >&2; exit 3; }
done
exit 0
