#!/bin/bash
# AET-WO-1918, the built head, inside the image at /repo (a scratch tree): Sentinel-03's whole
# verify with its package written, Sentinel-01-bench's verify against its committed package,
# and the two twin_tier2 tests the hang guard touched. Each step prints one RESULT line.
# usage: bash built.sh <step>   (s03 | s01 | twin)
set -u
A=/repo/target/debug/aethelgard
case $1 in
  s03)
    out=/repo/target/s03-package; rm -rf "$out"
    t0=$(date +%s)
    $A verify --spec examples/sentinel-03.json --scenarios $(ls scenarios/sentinel-03/*.json | sort) --fab-out "$out" > /repo/target/s03-verify.json
    rc=$?
    echo "RESULT s03 verify rc=$rc seconds=$(( $(date +%s) - t0 )) elf=$(sha256sum "$out/sentinel-03.elf" 2>/dev/null | cut -c1-64)"
    python3 -I -c "
import json,sys
v=json.load(open('/repo/target/s03-verify.json'))
g=[x for x in v.get('gates',[]) if x.get('gate')=='prove']
print('RESULT s03 prove gates', len(g), 'pass', sum(1 for x in g if x.get('verdict')=='pass'))
print('RESULT s03 harnesses', ' '.join(x['harness'].split('::')[-1] for x in g))
"
    diff <(python3 -I -c "import json; m=json.load(open('fab/sentinel-03/package/MANIFEST.json')); print(json.dumps(m, indent=1, sort_keys=True))") \
         <(python3 -I -c "import json; m=json.load(open('$out/MANIFEST.json')); print(json.dumps(m, indent=1, sort_keys=True))") | sed 's/^/RESULT s03 manifest diff: /'
    for f in "$out"/*; do b=$(basename "$f"); [ "$b" = MANIFEST.json ] && continue; cmp -s "$f" "fab/sentinel-03/package/$b" || echo "RESULT s03 file differs: $b"; done
    ;;
  s01)
    t0=$(date +%s)
    $A verify --spec examples/sentinel-01.json --scenarios scenarios/sentinel-01-bench/pullup-3k3.json scenarios/sentinel-01-bench/droop-bor3.json \
      scenarios/sentinel-01-bench/spi-absent-u4.json scenarios/sentinel-01-bench/pullup-2k2.json scenarios/sentinel-01-bench/serial-rx-open-console.json \
      --against-fab fab/sentinel-01-bench/package > /repo/target/s01-verify.json
    echo "RESULT s01 against-fab rc=$? seconds=$(( $(date +%s) - t0 ))"
    python3 -I -c "
import json; v=json.load(open('/repo/target/s01-verify.json')); print('RESULT s01', json.dumps(v.get('against_fab', v.get('fab', {})))[:600])"
    ;;
  twin)
    cargo test --locked --offline -p aethelgard-cli --features with-tools --test twin_tier2 -- \
      an_aborted_machine_returns_inside_the_bound_as_cpu_aborted m2_a_dma_start_of_cen_fails_closed_by_name 2>&1 | tail -15
    echo "RESULT twin rc=${PIPESTATUS[0]}"
    ;;
esac
