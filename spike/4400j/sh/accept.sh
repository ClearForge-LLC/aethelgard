#!/bin/bash
# AET-WO-4400j: the acceptance runs, on the built branch (sh/img.sh REPO=the branch) and on the spike's copy one quantum
# higher (REPO=~/worktrees/aethelgard/4400j-q, patches/plus-one-quantum.patch). Each in its own container, 4 cores.
set -u
K=/home/forge/aethelgard-4400j/spike/4400j/sh; B=/home/forge/aethelgard-4400j; Q=$HOME/worktrees/aethelgard/4400j-q
run() { REPO=$1 CPUS=0-3 MEM=8G "$K/img.sh" tools "$2" 2>&1 | grep -v capped-run; }
run $B 'cargo build --offline --locked -p aethelgard-cli 2>&1 | tail -1'
run $Q 'cargo build --offline --locked -p aethelgard-cli 2>&1 | tail -1; cargo build --offline --locked --manifest-path bridge/Cargo.toml --target-dir /repo/target 2>&1 | tail -1'
run $B 'bash /spike/sh/archive.sh F-12000-in 12000ms /cache/in/arm12.json'
run $B 'bash /spike/sh/archive.sh F-11999900us-in 11999900us /cache/in/arm12.json'
run $Q 'bash /spike/sh/archive.sh F-12000100us-in-spike 12000100us /cache/in/arm12.json'
run $B 'bash /spike/sh/cli.sh F-s03-5000-1 5000ms s03'
run $B 'target/debug/aethelgard replay --archive "$(ls -d /cache/arch/F-12000-in/archive/*)" > /cache/arch/F-12000-in/replay.json; echo replay exit $?'
echo accept-done
