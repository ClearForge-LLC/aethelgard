#!/usr/bin/env bash
# judge.sh MAIN BRANCH: check pinout on every measured ELF with each binary
# (AET-WO-4406 §2 acceptance): one line per board and binary, then the CI
# script on the CI set. ELFs in ~/.cache/aet-4406/elves (out/SHA256SUMS);
# topologies: the 4404 boards' re-solved at spec 20 (solved 13) by MAIN into
# ~/.cache/aet-4406/tops from 4405's spec-19 files (only schema_version
# bumped; equal to 4405's but for the specification hash), the CI boards'
# solved from this tree by MAIN.
set -uo pipefail
main=$1 branch=$2
K=$HOME/.cache/aet-4406 E=$K/elves T=$HOME/.cache/aet-4406/tops
top() {
  case $1 in
    fc-mav3|adv-proto-del3) echo "$T/fc-mav3.top20" ;;
    eight*) echo "$T/eight.top20" ;;
    sentinel-01|sentinel-02|serial-dshot-9600|sentinel-02-noimu) echo "$K/ci/$1/topology.json" ;;
    step-g|seed-*) echo "$K/ci/topology.json" ;;
    *) echo "$T/fc-base.top20" ;;
  esac
}
one() { # BIN NAME ELF [--hwdef-h H] [--allow A]: the verdict on one line
  local bin=$1 name=$2 elf=$3 out rc=0
  shift 3
  out=$("$bin" check pinout --topology "$(top "$name")" --elf "$elf" "$@") || rc=$?
  echo "exit $rc $(jq -c '[.findings[]? | "\(.check) \(.kind) \(.pin) [\(.expected) | \(.decoded)]"] | sort' <<< "$out") $(jq -c '.error? // empty' <<< "$out")"
}

mkdir -p "$K/ci"
"$main" solve --file examples/sentinel-01-minimal.json > "$K/ci/topology.json"
for b in sentinel-01:examples/sentinel-01-minimal.json sentinel-02:crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json \
  serial-dshot-9600:crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json sentinel-02-noimu:crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json; do
  mkdir -p "$K/ci/${b%%:*}"
  "$main" solve --file "${b#*:}" > "$K/ci/${b%%:*}/topology.json"
done

for d in "$E"/*/; do
  n=$(basename "$d")
  allow=()
  case $n in step-g|seed-*) allow=(--allow crates/aethelgard-pincheck/fixtures/step-g.allow.json) ;; esac
  m=$(one "$main" "$n" "$d/arducopter.elf" --hwdef-h "$d/hwdef.h" "${allow[@]}")
  b=$(one "$branch" "$n" "$d/arducopter.elf" --hwdef-h "$d/hwdef.h" "${allow[@]}")
  bo=$(one "$branch" "$n" "$d/arducopter.elf" "${allow[@]}")
  if [ "$m" = "$b" ]; then echo "SAME $n $b"; else echo "MOVED $n"; echo "  main   $m"; echo "  branch $b"; fi
  echo "  elf-only branch $(cut -c1-220 <<< "$bo")"
done
for n in advnoromfs advlua; do
  d=$K/advb/out/$n
  echo "EXTRA $n main   $(one "$main" "$n" "$d/arducopter.elf" --hwdef-h "$d/hwdef.h")"
  echo "EXTRA $n branch $(one "$branch" "$n" "$d/arducopter.elf" --hwdef-h "$d/hwdef.h")"
done
# 4405 A5: fc-base with 1_PROTOCOL's and 2_PROTOCOL's offsets swapped.
d=$E/fc-base
echo "EXTRA swap main   $(one "$main" fc-base "$HOME/.cache/aet-4405/adv/swap.elf" --hwdef-h "$d/hwdef.h")"
echo "EXTRA swap branch $(one "$branch" fc-base "$HOME/.cache/aet-4405/adv/swap.elf" --hwdef-h "$d/hwdef.h")"

# The CI script itself, on the CI boards laid out as CI lays them out.
for bin in "$main" "$branch"; do
  B=$K/ci/boards; rm -rf "$B"; mkdir -p "$B"
  for n in step-g seed-pin seed-af seed-pkg sentinel-01 sentinel-02 serial-dshot-9600; do
    mkdir -p "$B/$n"; cp "$E/$n/arducopter.elf" "$E/$n/hwdef.h" "$B/$n/"
    [ -f "$K/ci/$n/topology.json" ] && cp "$K/ci/$n/topology.json" "$B/$n/"
  done
  rc=0; bash container/ardupilot-pinout.sh "$bin" "$K/ci/topology.json" "$B" crates/aethelgard-pincheck/fixtures/step-g.allow.json > "$K/ci/pinout-$(basename "$bin").log" 2>&1 || rc=$?
  echo "CI-SCRIPT $(basename "$bin") exit $rc errors $(grep -c '::error' "$K/ci/pinout-$(basename "$bin").log")"
done
