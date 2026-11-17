#!/bin/bash
set -u
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|-->|Finished"
echo "== full setup (observers silenced), healthy 100 ms"; FULL_SETUP=1 bash /repo/spike/2303/sh/run.sh healthy 100000000 /cache/runs/healthy-100ms-full
