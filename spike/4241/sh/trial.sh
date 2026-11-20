#!/bin/bash
# Inside the image: the branch with 2305 trial-merged (/cache/trial); its CLI generates and builds each group's ELF and
# runs the droop scenarios main's 2305-less judge flipped (rj: min/droop-bor3, s01/droop-bor3, dshot/motors-dshot600-droop).
set +e
T=/cache/target-trial/debug; D=/cache/trialrun; mkdir -p $D
( cd /cache/trial && CARGO_TARGET_DIR=/cache/target-trial cargo build -q --locked -p aethelgard-cli ) || { echo "trial build failed"; exit 9; }
cp /cache/target-main/debug/aethelgard-bridge $T/
cd /cache/trial
for g in min:examples/sentinel-01-minimal.json:scenarios/sentinel-01/droop-bor3.json s01:examples/sentinel-01.json:scenarios/sentinel-01-bench/droop-bor3.json dshot:crates/aethelgard-cli/tests/fixtures/motors-dshot600.json:scenarios/motors/dshot600-droop.json; do
  IFS=: read n spec sc <<< "$g"
  rm -rf $D/gen-$n $D/$n; $T/aethelgard generate --spec $spec --out $D/gen-$n > $D/$n.gen.out 2>&1
  $T/aethelgard build --crate $D/gen-$n/board --out $D/$n.elf > $D/$n.build.out 2>&1
  PATH=$T:$PATH $T/aethelgard campaign --scenarios $sc --firmware $D/$n.elf --archive $D/$n > $D/$n.campaign.out 2> $D/$n.campaign.err
  echo "$n rc=$? $(cut -c1-400 $D/$n.campaign.out | head -1)"
done
