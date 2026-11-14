#!/bin/bash
# Reproduce AET-WO-4100 S3. Usage: run.sh <workdir>   (needs cargo, python3; run inside a capped scope)
# systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 -- bash run.sh /abs/work
set -u
W=$1; HERE=$(cd "$(dirname "$0")" && pwd); REPO=${REPO:-$HERE/../../..}
mkdir -p "$W/specs" && cd "$W"
git -C "$REPO" worktree add "$W/base" HEAD --detach       # unmodified HEAD (baseline)
git -C "$REPO" worktree add "$W/probe" HEAD --detach         # patched build
(cd "$W/probe" && git apply "$HERE/router-nlayer.patch")
export CARGO_TARGET_DIR=$W/target
(cd "$W/base"  && cargo build --release -p aethelgard-cli -j 6 && cp $CARGO_TARGET_DIR/release/aethelgard $W/aethelgard-base)
(cd "$W/probe" && cargo build --release -p aethelgard-cli -j 6 && cp $CARGO_TARGET_DIR/release/aethelgard $W/aethelgard-nl)
python3 "$HERE/gen.py" "$REPO/examples/sentinel-01-minimal.json" "$W/specs"
cp "$REPO/examples/sentinel-01-minimal.json" "$W/specs/ref.json"
for sp in ref 2x5 2x5_nospi D1_24i2c D3_24i2c_4spi_x3; do
  # baseline: HEAD binary, 2 layers
  /usr/bin/time -f "base $sp %es" ./aethelgard-base emit --file specs/$sp.json --out out/base/$sp | head -c 200; echo
  for L in 2 4 6; do   # probe: AET_LAYERS selects 2/4/6; AET_STATS=1 prints vias/ripups/length per layer
    /usr/bin/time -f "$sp L=$L %es" env AET_LAYERS=$L AET_STATS=1 ./aethelgard-nl emit --file specs/$sp.json --out out/nl/${sp}_$L 2>&1 | grep -E "STATS|message|%|s$" | head -c 400; echo
  done
done
# extra knobs used in the report: AET_RIPUPS=<n> (budget), AET_STEPI=<n> (inner-layer step cost), AET_DEBUG=1 (plane-pad failures)
cmp out/base/ref/sentinel-01.kicad_pcb out/nl/ref_2/sentinel-01.kicad_pcb && echo "2-layer board byte-identical to HEAD"
