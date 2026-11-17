#!/usr/bin/env bash
# AET-WO-1313 §1.3 gate runner (reused by AET-WO-1313b). Usage, from a repository root that holds the CLI's sources:
#   spike/1313/run.sh <spec.json> <outdir>
# Runs validate, generate, emit, check erc, check drc --spec, drc with --schematic parity,
# check support and extract (3400's run.sh, the same steps), writing <outdir>/<step>.{json,stderr,exit}
# and <outdir>/emit.seconds (emit's wall time on the host). AET_BIN names the CLI (a release build);
# AET_ENV is passed into the CLI's environment on the host and in the container (the spike's
# placement method, e.g. AET_1313B=km). Host steps run in a capped systemd user scope; KiCad
# steps run the same binary in the pinned tools image through ~/bin/capped-run (caps read back,
# swap 0), TMPDIR unset inside, one container at a time.
set -u
SPEC=$(realpath "$1")
OUT=$(realpath -m "$2")
ROOT=$(pwd)
BIN=${AET_BIN:-$ROOT/target/release/aethelgard}
IMAGE=$(cat container/IMAGE)
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
[ -f "$SPEC" ] || { echo "no $SPEC" >&2; exit 2; }
mkdir -p "$OUT"
find "$OUT" -mindepth 1 -delete
read -r -a ENVS <<< "${AET_ENV:-}"

host() { # host <step> <args...>
  local step=$1; shift
  systemd-run --user --scope -q -p MemoryMax=16G -p MemorySwapMax=0 --expand-environment=no -- \
    env "${ENVS[@]}" "$BIN" "$@" > "$OUT/$step.json" 2> "$OUT/$step.stderr"
  echo $? > "$OUT/$step.exit"
}

box() { # box <step> <args...>; /spec and /out are the spec's directory and OUT
  local step=$1; shift
  local e=()
  for kv in "${ENVS[@]}"; do e+=(-e "$kv"); done
  "$HOME/bin/capped-run" --mem 8G -- --rm --network none "${e[@]}" \
    -v "$(dirname "$SPEC")":/spec:ro -v "$OUT":/out -v "$(dirname "$BIN")":/aet:ro -w /out \
    "$IMAGE" /aet/aethelgard "$@" > "$OUT/$step.json" 2> "$OUT/$step.stderr"
  echo $? > "$OUT/$step.exit"
}

S=/spec/$(basename "$SPEC")
host 1-validate validate --file "$SPEC"
host 2-generate generate --spec "$SPEC" --out "$OUT/gen"
t0=$(date +%s.%N)
host 3-emit emit --file "$SPEC" --out "$OUT/emit"
t1=$(date +%s.%N)
python3 -I -c 'import sys; print(f"{float(sys.argv[2]) - float(sys.argv[1]):.1f}")' "$t0" "$t1" > "$OUT/emit.seconds"
P=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["project_name"])' "$SPEC")
if [ -f "$OUT/emit/$P.kicad_pcb" ]; then
  box 4-erc check erc --schematic "/out/emit/$P.kicad_sch"
  box 5-drc check drc --board "/out/emit/$P.kicad_pcb" --spec "$S"
  box 6-drc-parity check drc --board "/out/emit/$P.kicad_pcb" --spec "$S" --schematic "/out/emit/$P.kicad_sch"
  box 7-support check support --schematic "/out/emit/$P.kicad_sch" --board "/out/emit/$P.kicad_pcb" --spec "$S"
  host 8-extract extract --board "$OUT/emit/$P.kicad_pcb" --spec "$SPEC"
fi
for f in "$OUT"/*.exit; do printf '%s=%s ' "$(basename "$f" .exit)" "$(cat "$f")"; done
echo "emit=$(cat "$OUT/emit.seconds")s"
