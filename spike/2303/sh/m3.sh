#!/bin/bash
set -u
L=/cache/target/release/lockstep
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|-->|Finished"
echo "== flip"; $L flip /cache/runs/weak-100ms-1 /cache/runs/flip
echo "== synth 60 s"; $L synth 60 /cache/runs/weak-100ms-1 /cache/synth/s60
