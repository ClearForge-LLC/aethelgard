#!/bin/bash
# AET-WO-1919 (2), inside the image at /repo: every platform load the CLI makes, timed.
#   1. each committed board's platform, and each example's generated platform (generate starts no
#      process), loaded by `simulate --platform` N times, one after another;
#   2. the platforms the tier-2 tests load by Renode's own name;
#   3. the six tier-2 tests that load (MCP's included), one at a time.
# Each load prints: LOAD <platform> <i> rc=<exit> wall_s=<s> verdict=<verdict or error kind>.
# Each test prints: TEST <name> rc=<exit> wall_s=<s>. Run under ctl.sh, whose probe times Renode.
# usage: bash loads.sh N
set -u
N=${1:-3}
A=/repo/target/debug/aethelgard
G=/repo/target/gen; rm -rf "$G"; mkdir -p "$G"
now() { date +%s.%N; }
plats=(boards/sentinel-01/sentinel-01.repl boards/sentinel-01-bench/sentinel-01-bench.repl)
for spec in examples/sentinel-*.json; do
  name=$(basename "$spec" .json)
  if $A generate --spec "$spec" --out "$G/$name" > "$G/$name.gen.json" 2>&1; then
    plats+=("$(ls "$G/$name"/board/*.repl)")
  else
    echo "GEN $name refused: $(head -c 300 "$G/$name.gen.json")"
  fi
done
plats+=("@platforms/boards/nucleo_h753zi.repl" "@platforms/cpus/stm32h753.repl")
for p in "${plats[@]}"; do
  for i in $(seq 1 "$N"); do
    t0=$(now); out=$($A simulate --platform "$p" 2>&1); rc=$?; t1=$(now)
    v=$(python3 -I -c 'import json,sys
t=sys.stdin.read().strip().splitlines()
try:
  d=json.loads(t[-1]); print(d.get("verdict") or d.get("error",{}).get("kind"))
except Exception: print("unparsed")' <<<"$out")
    echo "LOAD $p $i rc=$rc wall_s=$(awk "BEGIN{printf \"%.2f\", $t1-$t0}") verdict=$v"
  done
done
tests=(
  "with_tools simulate_loads_the_reference_board"
  "with_tools simulate_fails_a_platform_that_does_not_load"
  "with_tools mcp_simulate_loads_the_nucleo_board"
  "twin_tier2 the_emitted_platform_loads_and_lists_every_solved_peripheral"
  "twin_tier2 a_device_named_for_a_renode_instance_or_keyword_breaks_the_load"
  "twin_tier2 the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused"
)
for t in "${tests[@]}"; do
  set -- $t
  t0=$(now)
  cargo test --locked --offline -q -p aethelgard-cli --features with-tools --test "$1" -- "$2" --exact --test-threads=1 > "/repo/target/test-$2.log" 2>&1
  rc=$?; t1=$(now)
  echo "TEST $1::$2 rc=$rc wall_s=$(awk "BEGIN{printf \"%.2f\", $t1-$t0}") $(grep -E '^test result' "/repo/target/test-$2.log" | head -1)"
done
