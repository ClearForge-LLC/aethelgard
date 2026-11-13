#!/bin/bash
# P1 and P3: every case 10 times, five cases at a time, inside one capped container (img.sh).
# Raw JSONL under /cache/p1 (outside the repository); the summary goes to spike/2301/out.
set -u
CASES="control machine-reset machine-reset-vtor cpu-reset held-halted held-cpuwait pvd-exti16 systick-after systick-before nvic-pend1"
mkdir -p /cache/p1
printf '%s\n' $CASES | xargs -P 5 -I{} python3 /repo/spike/2301/renode/drive.py {} 10 /cache/p1/{}.jsonl
python3 /repo/spike/2301/renode/summary.py /cache/p1 $CASES
