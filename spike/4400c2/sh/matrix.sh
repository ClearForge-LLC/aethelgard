#!/bin/bash
# Inside the tools image: 4400c's r3 (3 s, 500 ms steps) on each candidate platform, no overlay,
# the same COMMAND_LONG injected into USART1 at 1.5 s in every run.
set -u
HEX=$(cat /cache/cand/${HEXFILE:-command_long}.hex)
for p in ${CANDS:-p0-emitted p1-spi p2-spi-usart p3-spi-usart-i2c p4-usart}; do
  O=/cache/runs/$p${SUFFIX:-}; rm -rf $O; mkdir -p $O
  PLATFORM=/cache/cand/$p.repl INJECT_AT_S=1.5 INJECT_HEX=$HEX python3 -I /repo/spike/4400c2/drive.py $O 3 500 > $O/steps.jsonl 2> $O/drive.err
  echo "== $p exit=$?"
done
