#!/bin/bash
# AET-WO-3303b: prove every harness of one board, one container per harness (sh/img.sh: capped-run at
# 16G, --network none, nice -n 10, one at a time), so each row's memory.peak is that harness's own
# scope peak; then, with VERIFY=1, `verify --spec` (no scenarios) at 16G.
# Usage: [VERIFY=1] [PROVE=0] spike/3303b/run.sh <phase> <board>   (PROVE=0: no prove step)
#   phase: before (tree ~/.cache/aet-3303b/before-tree = `git archive 3f989435`, main; its CLI in
#          target-before) or after (tree ~/.cache/aet-3303b/after-tree = `git archive` of the commit
#          named in after-tree/SHA; its CLI in target-after). Never the live worktree.
#   board: sentinel-01 | sentinel-01-bench (the committed crates under boards/, proved with --spec, so
#          the verdict's harness_provenance says whether the crate is the generator's), or
#          logger-i2c-only | logger-spi-only-cap (AET-WO-4600's specifications, re-pinned to schema 17 in
#          ~/.cache/aet-3303b/specs, nothing else changed; generated first by the phase's CLI).
# Rows: ~/.cache/aet-3303b/runs/<phase>/<board>/rows.tsv (board step exit wall_ms peak max).
set -u
cd "$(dirname "$0")/../.."
P=$1 B=$2
C=$HOME/.cache/aet-3303b
case $P in
  before) export TREE=$C/before-tree; A=/cache/target-before/debug/aethelgard ;;
  after) export TREE=$C/after-tree; A=/cache/target-after/debug/aethelgard ;;
  *) echo "phase: before|after" >&2; exit 2 ;;
esac
IMG="bash $PWD/spike/3303b/sh/img.sh"
O=/cache/runs/$P/$B
H=$C/runs/$P/$B
mkdir -p "$H" "$C/sh"; cp spike/3303b/sh/step.sh "$C/sh/step.sh"
[ -f "$TREE/SHA" ] && echo "$(date -u -Is) tree $(cat "$TREE/SHA")" >> "$H/free.log"
roomy() { local want=20 n=0
  until [ "$(free -g | awk '/^Mem:/ {print $7}')" -ge "$want" ]; do n=$((n+1)); [ $n -gt 120 ] && break; sleep 30; done
  echo "$(date -u -Is) $P $B $1 free -g available=$(free -g | awk '/^Mem:/ {print $7}')" >> "$H/free.log"; }
step() { local name=$1; shift; roomy "$name"; MEM=16G $IMG bash /cache/sh/step.sh "$B" "$name" "$O" "$@" 2>>"$H/caps.log"; }
case $B in
  sentinel-01) CRATE=/repo/boards/sentinel-01; HOSTCRATE=$TREE/boards/sentinel-01; SPEC=/repo/examples/sentinel-01-minimal.json ;;
  sentinel-01-bench) CRATE=/repo/boards/sentinel-01-bench; HOSTCRATE=$TREE/boards/sentinel-01-bench; SPEC=/repo/examples/sentinel-01.json ;;
  logger-i2c-only|logger-spi-only-cap)
    SPEC=/cache/specs/$B.json; CRATE=$O/gen/board; HOSTCRATE=$H/gen/board
    rm -rf "$H/gen"; step generate $A generate --spec $SPEC --out $O/gen ;;
  *) echo "unknown board $B" >&2; exit 2 ;;
esac
HS=$(awk '/^ *#\[kani::proof\]/{a=1;next} a&&/^ *fn /{sub(/^ *fn /,"");sub(/[^A-Za-z0-9_].*/,"");print "proofs::"$0;a=0}' "$HOSTCRATE/src/proofs.rs")
[ -n "$HS" ] || { echo "no harness in $HOSTCRATE" >&2; exit 3; }
# A split crate must not also prove the whole schedule (a peak measured beside the parts).
if grep -qx 'proofs::every_valid_sequence' <<<"$HS" && grep -q '^proofs::every_valid_sequence_' <<<"$HS"; then
  echo "$B: the whole harness and its parts are both proofs" >&2; exit 3
fi
echo "$HS" > "$H/harnesses.txt"
if [ "${PROVE:-1}" = 1 ]; then
  for h in $HS; do
    step "prove:${h#proofs::}" $A prove --crate $CRATE --harness "$h" --spec $SPEC
  done
  # The old name on a split crate: refused, not passed with no harness run.
  if ! grep -qx 'proofs::every_valid_sequence' <<<"$HS"; then
    step "prove:every_valid_sequence(absent)" $A prove --crate $CRATE --harness proofs::every_valid_sequence --spec $SPEC
  fi
fi
[ "${VERIFY:-0}" = 1 ] && step verify $A verify --spec $SPEC
exit 0
