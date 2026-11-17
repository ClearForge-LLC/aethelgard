#!/bin/bash
# Inside the tools image: paced injection (86806 ns a byte, 115200 baud 8N1) with the fault-status
# hook: p2 three frames (done as `paced`), p4 three frames, p2 two frames.
set -u
run() { O=/cache/runs/$1; rm -rf $O; mkdir -p $O
  PLATFORM=/cache/cand/$2.repl INJECT_AT_S=1.5 INJECT_PACE_NS=86806 INJECT_HEX=$(cat /cache/cand/$3.hex) python3 -I /repo/spike/4400c2/drive.py $O 3 500 /repo/spike/4400c2/stubs-fault-status.txt > $O/steps.jsonl 2> $O/drive.err
  echo "== $1 exit=$? $(grep -h 'AET4400C2-FAULT' $O/console.log | grep -v AddHook | cut -c1-110)"; }
run paced-p4-x3 p4-usart command_long_x3
run paced-p2-x2 p2-spi-usart command_long_x2
