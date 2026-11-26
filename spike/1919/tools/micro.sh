#!/bin/bash
# AET-WO-1919 (1), inside the image at /repo: where a prove gate's fixed time and extract's time go,
# with one CLI build (target/debug or target/release), each step timed alone:
#   generate of Sentinel-03 (what `prove --spec` and `verify` each do in-process), extract on its
#   board, and prove of the lightest harness with --spec and without.
# usage: [NO_EXTRACT=1] bash micro.sh <target/debug|target/release> <tag>
# Each prove line also prints its verdict and harness_provenance.
set -u
A=/repo/$1/aethelgard tag=$2
S=examples/sentinel-03.json G=/repo/target/micro-$tag
rm -rf "$G"
t() { local n=$1; shift; local t0 t1 rc; t0=$(date +%s.%N); "$@" > "/repo/target/micro-$tag-$n.out" 2>&1; rc=$?; t1=$(date +%s.%N)
      echo "MICRO $tag $n rc=$rc wall_s=$(awk "BEGIN{printf \"%.2f\", $t1-$t0}")"; }
t generate $A generate --spec $S --out "$G"
[ -n "${NO_EXTRACT:-}" ] || t extract $A extract --board "$G/kicad/sentinel-03.kicad_pcb" --spec $S
for i in 1 2; do
  t prove-spec-$i $A prove --crate "$G/board" --harness proofs::flash_option_status_is_one_read --spec $S
  python3 -I -c 'import json,sys; v=json.loads(open(sys.argv[1]).read().strip().splitlines()[-1]); print("PROVE", sys.argv[1], v.get("verdict"), v.get("harness_provenance"))' "/repo/target/micro-$tag-prove-spec-$i.out"
  t prove-nospec-$i $A prove --crate "$G/board" --harness proofs::flash_option_status_is_one_read
done
