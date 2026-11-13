#!/bin/bash
# After review: (1) P2.6 again with the release nudge in lockstep and schedule (the recommended
# configuration end to end); (2) P5's three headline claims again through the gated runner.
set -u
S=$(cd "$(dirname "$0")" && pwd)
R=$HOME/.cache/aet-2301
"$S/img.sh" -- bash /repo/spike/2301/lockstep/p26-matrix.sh > "$R/rerun2-p26.jsonl" 2>&1
"$S/img.sh" -- bash -c 'cd /repo && bash spike/2301/p5/cli.sh 100ms /cache/p5/rerun2-cli-100ms && cat /cache/p5/rerun2-cli-100ms.txt' > "$R/rerun2-p5-cli.txt" 2>&1
"$S/img.sh" -- bash -c '/repo/spike/2301/p5/sbx.sh rerun2-q1-100 100 1' > "$R/rerun2-p5-file.txt" 2>&1
"$S/img.sh" -- bash -c 'STREAM=1 /repo/spike/2301/p5/sbx.sh rerun2-stream-100 100 2' > "$R/rerun2-p5-stream.txt" 2>&1
echo rerun2-done
