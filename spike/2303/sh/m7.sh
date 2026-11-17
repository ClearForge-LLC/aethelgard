#!/bin/bash
# Inside the image: the splice redone on a real droop run (records beside its own RailCrossings),
# and every run's re-judge against its archived ResetRendered records.
set -u
L=/cache/target/release/lockstep; A=/cache/target/debug/aethelgard
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|-->|Finished"
rm -rf /cache/camp2; mkdir -p /cache/camp2
$A campaign --scenarios spike/2303/scenarios/s02-droop.json --firmware /cache/s02/fw.elf --archive /cache/camp2/archive > /cache/camp2/campaign.json 2>&1; echo "campaign exit=$?"
RUN=$(ls -d /cache/camp2/archive/*/ | head -1); echo "run $RUN"
$A replay --archive $RUN | cut -c1-400; echo " replay exit=$?"
$L splice $RUN /cache/camp2/splice
for v in a b; do D=$(ls -d /cache/camp2/splice/$v/*/); $A replay --archive $D; echo " replay($v) exit=$?"; done
for d in /cache/runs/*/; do [ -f $d/run.json ] && [ -s $d/run.json ] && { echo "== $d"; $L replay $d | tail -1; }; done
