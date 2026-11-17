#!/bin/bash
# Inside the image: the short measurements again, logged for the record (m2, m3, m7, m8 printed to
# the terminal only): the flip (raw and re-sealed), synth 60 s and 44 s (summary lines), both
# campaigns with their splices and replays, and every run's re-judge.
set -u
L=/cache/target/release/lockstep; A=/cache/target/debug/aethelgard
echo "== flip"; $L flip /cache/runs/weak-100ms-1 /cache/runs/flip
for n in 60 44; do echo "== synth $n s"; $L synth $n /cache/runs/weak-100ms-1 /cache/synth/s$n | sed -E 's/,"bytes_at_each_second":\[.*\]//'; done
rm -rf /cache/synth
for sc in s02-baseline s02-droop; do
  echo "== campaign $sc"; rm -rf /cache/c9/$sc; mkdir -p /cache/c9/$sc
  $A campaign --scenarios spike/2303/scenarios/$sc.json --firmware /cache/s02/fw.elf --archive /cache/c9/$sc/archive > /cache/c9/$sc/campaign.json 2>&1; echo "campaign exit=$?"
  RUN=$(ls -d /cache/c9/$sc/archive/*/ | head -1)
  $A replay --archive $RUN; echo " replay exit=$?"
  $L splice $RUN /cache/c9/$sc/splice
  for v in a b; do D=$(ls -d /cache/c9/$sc/splice/$v/*/); $A replay --archive $D; echo " replay($v) exit=$?"; done
done
for d in /cache/runs/*/; do [ -s $d/run.json ] && { echo "== rejudge $d"; $L replay $d | tail -1; }; done
