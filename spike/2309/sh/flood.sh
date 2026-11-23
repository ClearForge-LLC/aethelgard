#!/bin/bash
# AET-WO-2309 §1.1, inside the tools image (img.sh, REPO = main's `git archive`): the one read boot the
# judge archives with unread witnesses (P8). The tier-2 flooding firmware (`tests/fixtures/flood`) is
# built from a copy under /cache, then one campaign run of it on sentinel-01-minimal, expecting a pass,
# so the run is archived whatever the judge finds (3305's collide.sh). By main's CLI.
set +e
A=${BIN:-/cache/target-main/debug}/aethelgard; export PATH=$(dirname $A):$PATH
F=/cache/flood; rm -rf $F; mkdir -p $F/sc
cp -r /repo/crates/aethelgard-cli/tests/fixtures/flood $F/src-crate
(cd $F/src-crate && cargo build --release --offline --locked --target-dir $F/target > $F/build.out 2>&1); echo "build rc=$?"
elf=$F/target/thumbv7em-none-eabihf/release/aet-flood-fixture; ls -la $elf
cp /repo/examples/sentinel-01-minimal.json $F/sc/min-base.json
printf '{"schema_version":1,"name":"flood-baseline","base":"min-base.json","expect":"pass","expect_findings":[],"overrides":[]}\n' > $F/sc/flood-baseline.json
(cd $F/sc && $A campaign --scenarios flood-baseline.json --firmware $elf --archive $F/archive > $F/campaign.out 2> $F/campaign.err; echo "campaign rc=$?")
ls $F/archive
