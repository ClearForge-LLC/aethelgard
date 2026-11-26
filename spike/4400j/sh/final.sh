#!/bin/bash
# AET-WO-4400j: everything the READY cites, on the branch's head, in order, one container at a time (sh/img.sh):
# every guard's mutant; fmt, clippy and the workspace suite; ci/foreign-judge.sh on main's CI ELFs; the acceptance
# runs (the spike's one-quantum copy rebuilt at this head) with their wall times; the 1 s / 5 s band, ten pairs.
set -u
K=/home/forge/aethelgard-4400j/spike/4400j/sh; B=/home/forge/aethelgard-4400j; Q=$HOME/worktrees/aethelgard/4400j-q
L=$HOME/.cache/aet-4400j/kit; H=$(git -C $B rev-parse --short=8 HEAD)
run() { REPO=$1 CPUS=${CPUS:-} MEM=$2 "$K/img.sh" tools "$3" 2>&1 | grep -v capped-run; }
echo "head $H" > $L/final-head.txt
run $B 16G 'python3 -I /spike/mutants/all.py' > $L/mutants-final.txt
run $B 32G 'cargo fmt --all -- --check; echo "fmt $?"; cargo clippy --offline --locked --workspace --all-targets -- -D warnings 2>&1 | tail -1; echo "clippy ${PIPESTATUS[0]}"; cargo build --offline --locked --manifest-path bridge/Cargo.toml --target-dir /repo/target 2>&1 | tail -1; cargo test --offline --locked --workspace --no-fail-fast > /cache/kit/test-final2.log 2>&1; echo "test $?"' > $L/gates-final.txt
CPUS=0-3 run $B 8G 'bash ci/foreign-judge.sh /cache/ci-boards/boards > /cache/kit/foreign-judge-final.log 2>&1; echo "foreign-judge $?"' >> $L/gates-final.txt
git -C $Q checkout -q -f --detach "$(git -C $B rev-parse HEAD)" && git -C $Q apply $B/spike/4400j/patches/plus-one-quantum.patch && git -C $Q diff --stat >> $L/gates-final.txt
run $Q 16G 'cargo build --offline --locked -p aethelgard-cli 2>&1 | tail -1'
for w in 11999900us 12000ms; do
  CPUS=0-3 run $B 8G "s=\$(date +%s.%N); bash /spike/sh/archive.sh G-$w-in $w /cache/in/arm12.json; echo wall \$(awk -v a=\$s -v b=\$(date +%s.%N) 'BEGIN{print b-a}')" > $L/G-$w.txt
done
CPUS=0-3 run $Q 8G "s=\$(date +%s.%N); bash /spike/sh/archive.sh G-12000100us-in-spike 12000100us /cache/in/arm12.json; echo wall \$(awk -v a=\$s -v b=\$(date +%s.%N) 'BEGIN{print b-a}')" > $L/G-12000100us.txt
CPUS=0-3 run $B 8G 'target/debug/aethelgard replay --archive "$(ls -d /cache/arch/G-12000ms-in/archive/*)" > /cache/arch/G-12000ms-in/replay.json; echo replay $?' >> $L/G-12000ms.txt
for r in 1 2 3 4 5 6 7 8 9 10; do
  for w in 1000ms 5000ms; do CPUS=0-3 run $B 8G "bash /spike/sh/cli.sh band$r-s02-${w%ms} $w s02" | tail -1 >> $L/band.jsonl; done
done
echo final-done >> $L/gates-final.txt
