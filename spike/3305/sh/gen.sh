#!/bin/bash
# Inside the image: build main's CLI and the bridge, then generate Sentinel-03 and two-bme boards.
set -u
cd /repo
cargo build -q --locked -p aethelgard-cli && cargo build -q --manifest-path bridge/Cargo.toml --locked || exit 9
A=/cache/target/debug/aethelgard
for s in sentinel-03 two-bme; do
  O=/cache/m/$s; rm -rf $O; mkdir -p $O
  $A generate --spec crates/aethelgard-cli/tests/fixtures/$s.json --out $O/gen > $O/generate.json 2>&1; echo "$s generate rc=$?"
done
