#!/bin/bash
# Inside the tools image: p2 with three frames, twice at 1.5 s (determinism) and once at 2.5 s
# (after the dataflash erase), with fault-handler hooks; 2 s and 3 s runs.
set -u
HEX=$(cat /cache/cand/command_long_x3.hex)
run() { O=/cache/runs/$1; rm -rf $O; mkdir -p $O
  PLATFORM=/cache/cand/p2-spi-usart.repl INJECT_AT_S=$2 INJECT_HEX=$HEX python3 -I /repo/spike/4400c2/drive.py $O $3 500 /repo/spike/4400c2/stubs-fault-hooks.txt > $O/steps.jsonl 2> $O/drive.err
  echo "== $1 exit=$? $(grep -h 'AET4400C2-HOOK' $O/console.log | head -2 | tr '\n' ' ' | cut -c1-200)"; }
run crash-a 1.5 2
run crash-b 1.5 2
run crash-late 2.5 3
