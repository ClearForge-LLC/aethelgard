#!/usr/bin/env bash
# AET-WO-1907: plan and run one shard of the `tools` job's tests, inside the image.
#   ci/shard.sh plan OUT N      list every test, assign them to N shards, run the guard
#   ci/shard.sh run  OUT K      run shard K's tests, then check passes against the assignment
# Run from the repository root. The cargo commands are the job's own (AET-WO-2200,
# the workspace with the tools feature); a shard adds only `-- --exact <its names>`.
set -euo pipefail
export LC_ALL=C
here=$(dirname "$0")
py=(python3 -I "$here/shard.py")
FEATURES=(--locked --offline --features aethelgard-cli/with-tools)
BRIDGE=(--manifest-path bridge/Cargo.toml --locked --offline)
# Canary files (AET-WO-1900): the tier-2 test binaries, as cargo prints them.
TIER2_RE='^tests/(with_tools|twin_tier2|board_tier2|campaign_tier2|attribute_seeded|adversarial_1301_tools|adversarial_1302_tools)\.rs$'
# Fixed work shard 0 does besides tests (bridge fmt + clippy), in seconds, measured on CI: the two steps took 25 s.
BASE0=${SHARD0_FIXED_SECONDS:-25}

cmd=${1:?plan or run}
out=${2:?output directory}
mkdir -p "$out"

case "$cmd" in
plan)
  n=${3:?number of shards}
  # stderr carries the `Running` lines the parser needs; both streams, in order.
  cargo test --workspace "${FEATURES[@]}" -- --list >"$out/list-ws.log" 2>&1 \
    || { cat "$out/list-ws.log"; echo "shard: the workspace test list failed" >&2; exit 1; }
  cargo test "${BRIDGE[@]}" -- --list >"$out/list-bridge.log" 2>&1 \
    || { cat "$out/list-bridge.log"; echo "shard: the bridge test list failed" >&2; exit 1; }
  { "${py[@]}" parse ws "$out/list-ws.log"; "${py[@]}" parse bridge "$out/list-bridge.log"; } >"$out/entries.tsv"
  "${py[@]}" assign "$n" "$here/test-weights.tsv" "$out/entries.tsv" "$out" "$BASE0" "$here/colocate.tsv"
  "${py[@]}" guard "$n" "$out/entries.tsv" "$out"
  ;;
run)
  k=${3:?shard index}
  shard="$out/shard-$k.tsv"
  cp "$shard" "$out/shard.tsv"
  rm -f "$out/test-ws.log" "$out/test-bridge.log" "$out/result.txt"
  # An empty name list would make --exact run every test: refuse it per suite.
  mapfile -t ws < <("${py[@]}" names "$shard" ws)
  mapfile -t br < <("${py[@]}" names "$shard" bridge)
  status=0
  if [ "${#ws[@]}" -gt 0 ]; then
    set +e
    cargo test --workspace "${FEATURES[@]}" -- --exact "${ws[@]}" 2>&1 | tee "$out/test-ws.log"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
    set -e
  fi
  if [ "${#br[@]}" -gt 0 ]; then
    set +e
    cargo test "${BRIDGE[@]}" -- --exact "${br[@]}" 2>&1 | tee "$out/test-bridge.log"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
    set -e
  fi
  [ "$status" -eq 0 ] || { echo "shard $k: a test run failed" >&2; exit 1; }
  "${py[@]}" result "$k" "$shard" "$out/test-ws.log" "$out/test-bridge.log" "$TIER2_RE" | tee "$out/result.txt"
  ;;
*)
  echo "usage: shard.sh plan|run" >&2
  exit 2
  ;;
esac
