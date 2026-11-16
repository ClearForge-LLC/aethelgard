#!/bin/bash
# gen.sh <spec> <name>: board crate + platform via the library (as twin_tier2.rs), then build the ELF.
set -u
F=$1; N=$2; O=/cache/m/$N; mkdir -p $O; rm -rf $O/gen $O/fw.elf
cd /cache/wt && CARGO_TARGET_DIR=/cache/target-spike AET2302_SPEC=/repo/$F AET2302_OUT=$O/gen/board cargo test --locked --offline -q -p aethelgard-cli --test spike2302 > $O/gen.log 2>&1 || { tail -20 $O/gen.log; exit 1; }
/cache/target-spike/debug/aethelgard build --crate $O/gen/board --out $O/fw.elf > $O/build.json 2>&1 || { head -c 1500 $O/build.json; exit 1; }
echo "$N built"
