#!/bin/bash
# F13: the cited measurements again, each in its own container under the gated runner, so every one has
# its caps read back with its command; then 1 µs × 100 ms lock-step with one repeated line per quantum (F15).
set -u
S=$(cd "$(dirname "$0")" && pwd)
"$S/img.sh" -- bash /repo/spike/2301/renode/all.sh > "$HOME/.cache/aet-2301/rerun-p1.txt" 2>&1
"$S/img.sh" -- bash -c 'cd spike/2301/lockstep && bash p2-matrix.sh' > "$HOME/.cache/aet-2301/rerun-p2.jsonl" 2>&1
"$S/img.sh" -- bash /repo/spike/2301/lockstep/p26-matrix.sh > "$HOME/.cache/aet-2301/rerun-p26.jsonl" 2>&1
"$S/img.sh" -- bash /repo/spike/2301/lockstep/p4-replay.sh > "$HOME/.cache/aet-2301/rerun-p4.jsonl" 2>&1
"$S/img.sh" -- bash -c 'export TMAX_NS=1000 ONE_LINE=1; /cache/target/release/lockstep lockstep bor3 1000 100000000 /cache/p26/oneline-1us-100ms' > "$HOME/.cache/aet-2301/rerun-oneline-100ms.json" 2>&1
echo rerun-done
