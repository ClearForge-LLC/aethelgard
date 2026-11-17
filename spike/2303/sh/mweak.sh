#!/bin/bash
set -u
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|Finished"
for k in 1 2; do echo "== weak run $k"; bash /repo/spike/2303/sh/run.sh weak 100000000 /cache/runs/weak-100ms-$k; done
for t in samples records events; do sha256sum /cache/runs/weak-100ms-*/trace-$t/trace.cbor | awk '{print $1}' | uniq -c | sed "s/^/weak $t /"; done
