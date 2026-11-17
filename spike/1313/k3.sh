#!/usr/bin/env bash
# AET-WO-1313 §1.4: 1307's spec-level renamings (rename_spec.py, seeds 0-19) of the reference,
# D1 and D3, emitted end to end (placement and router), counted as routed of 20.
# Usage, from a repository root: spike/1313/k3.sh <label> <outdir>; AET_BIN names the CLI and
# AET_ENV the placement option's environment (e.g. "AET_1313_OPT=bc AET_1313_BOUND=100,100").
set -u
LABEL=$1; OUT=$(realpath -m "$2")
BIN=${AET_BIN:-$(pwd)/target/release/aethelgard}
HERE=$(dirname "$(realpath "$0")")
read -r -a ENVS <<< "${AET_ENV:-}"
FIX=crates/aethelgard-cli/tests/fixtures
mkdir -p "$OUT"
for k in ref D1 D3; do
  case $k in ref) IN=examples/sentinel-01-minimal.json;; D1) IN=$FIX/d1-24-i2c.json;; D3) IN=$FIX/d3-24-i2c-12-spi.json;; esac
  n=0; fails=""
  for seed in $(seq 0 19); do
    d=$OUT/$k-$seed; rm -rf "$d"; mkdir -p "$d"
    python3 -I "$HERE/rename_spec.py" "$IN" "$seed" "$d/spec.json"
    env "${ENVS[@]}" "$BIN" emit --file "$d/spec.json" --out "$d/emit" > "$d/verdict.json" 2>/dev/null
    if [ $? -eq 0 ]; then n=$((n+1)); else fails="$fails $seed"; fi
  done
  printf '%s\t%s\t%d/20\tfailed seeds:%s\n' "$LABEL" "$k" "$n" "${fails:- none}"
done
