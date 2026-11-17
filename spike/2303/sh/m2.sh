#!/bin/bash
# Inside the image: the flip, the campaign splice through the CLI's replay, and the synthetic
# samples-shape run to and past 256 MiB.
set -u
L=/cache/target/release/lockstep; A=/cache/target/debug/aethelgard
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|Finished"
echo "== flip"; $L flip /cache/runs/weak-100ms-1 /cache/runs/flip
echo "== campaign"; rm -rf /cache/camp; mkdir -p /cache/camp
$A campaign --scenarios spike/2303/scenarios/s02-baseline.json --firmware /cache/s02/fw.elf --archive /cache/camp/archive > /cache/camp/campaign.json 2>&1; echo "campaign exit=$?"
RUN=$(ls -d /cache/camp/archive/*/ | head -1); echo "run $RUN"
$A replay --archive $RUN; echo " replay exit=$?"
$L splice $RUN /cache/camp/splice
for v in a b; do D=$(ls -d /cache/camp/splice/$v/*/); $A replay --archive $D; echo " replay($v) exit=$?"; done
echo "== synth 60 s"; $L synth 60 /cache/runs/weak-100ms-1 /cache/synth/s60
