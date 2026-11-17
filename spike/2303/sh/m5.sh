#!/bin/bash
# Inside the image: (1) the CLI's witness logging kept (FULL_SETUP), healthy 100 ms, for Renode's
# memory under DShot; (2) option (b), the weak case at 100 µs and at 1 µs quanta, against 10 µs.
set -u
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|-->|Finished"
echo "== full setup, healthy 100 ms"; FULL_SETUP=1 bash /repo/spike/2303/sh/run.sh healthy 100000000 /cache/runs/healthy-100ms-full
echo "== weak 100 ms at 100 us"; Q_NS=100000 bash /repo/spike/2303/sh/run.sh weak 100000000 /cache/runs/weak-100ms-q100us
echo "== weak 1 s at 100 us"; Q_NS=100000 bash /repo/spike/2303/sh/run.sh weak 1000000000 /cache/runs/weak-1s-q100us
echo "== weak 100 ms at 1 us"; Q_NS=1000 bash /repo/spike/2303/sh/run.sh weak 100000000 /cache/runs/weak-100ms-q1us
