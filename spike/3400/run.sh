#!/usr/bin/env bash
# AET-WO-3400 S2 runner. Usage, from the repository root:
#   spike/3400/run.sh <variant>        # e.g. v0; reads spike/3400/variants/<variant>.json
# Writes spike/3400/results/<variant>/: one <step>.json (the CLI's stdout line) and <step>.exit per step.
#
# The work order's five commands are steps 1-5. Steps 6-8 are extra gates, recorded in their own
# columns: drc with --schematic parity, check support, and extract.
#
# Host steps (validate, generate, emit, solve, extract) run the release CLI built from this tree
# (cargo build --release -p aethelgard-cli) under a capped systemd user scope. Steps needing KiCad run
# the same binary inside the pinned tools image (container/IMAGE) through ~/bin/capped-run, which caps
# memory with swap 0 and reads the caps back before the container starts (kickoff rule 3). TMPDIR is
# left unset inside the container. One container at a time.
set -u
V=$1
ROOT=$(pwd)
SPEC=spike/3400/variants/$V.json
OUT=spike/3400/results/$V
BIN=${AET_BIN:-$ROOT/target/release/aethelgard}
IMAGE=$(cat container/IMAGE)
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
[ -f "$SPEC" ] || { echo "no $SPEC" >&2; exit 2; }
rm -rf "$ROOT/$OUT"
mkdir -p "$OUT"

host() { # host <step> <args...>
  local step=$1; shift
  systemd-run --user --scope -q -p MemoryMax=16G -p MemorySwapMax=0 --expand-environment=no -- \
    "$BIN" "$@" > "$OUT/$step.json" 2> "$OUT/$step.stderr"
  echo $? > "$OUT/$step.exit"
}

box() { # box <step> <args...>; paths are given relative to the repository root
  local step=$1; shift
  "$HOME/bin/capped-run" --mem 8G -- --rm --network none \
    -v "$ROOT":/repo:ro -v "$ROOT/$OUT":/out -v "$(dirname "$BIN")":/aet:ro -w /repo \
    "$IMAGE" /aet/aethelgard "$@" > "$OUT/$step.json" 2> "$OUT/$step.stderr"
  echo $? > "$OUT/$step.exit"
}

host 1-validate validate --file "$SPEC"
host 2-generate generate --spec "$SPEC" --out "$OUT/gen"
host 3-emit emit --file "$SPEC" --out "$OUT/emit"
host 0-solve solve --file "$SPEC"
P=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["project_name"])' "$SPEC")
SCH=$OUT/emit/$P.kicad_sch
PCB=$OUT/emit/$P.kicad_pcb
if [ -f "$PCB" ]; then
  box 4-erc check erc --schematic "/repo/$SCH"
  box 5-drc check drc --board "/repo/$PCB" --spec "/repo/$SPEC"
  box 6-drc-parity check drc --board "/repo/$PCB" --spec "/repo/$SPEC" --schematic "/repo/$SCH"
  box 7-support check support --schematic "/repo/$SCH" --board "/repo/$PCB" --spec "/repo/$SPEC"
  host 8-extract extract --board "$PCB" --spec "$SPEC"
fi
for f in "$OUT"/*.exit; do printf '%s=%s ' "$(basename "$f" .exit)" "$(cat "$f")"; done; echo
