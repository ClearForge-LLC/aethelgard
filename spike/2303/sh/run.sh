#!/bin/bash
# Inside the image: run.sh <case> <span_ns> <outdir> — one lock-step run, then replay.
set -u
L=/cache/target/release/lockstep
O=$3; rm -rf $O; mkdir -p $O
TOOL=/cache/s02/tool $L lockstep $1 $2 /cache/s02/renode-argv.txt $O > $O/run.json 2> $O/run.err
echo "lockstep exit=$?"; tail -c 3000 $O/run.err | tail -5
$L replay $O > $O/replay.jsonl 2>&1; echo "replay exit=$?"
