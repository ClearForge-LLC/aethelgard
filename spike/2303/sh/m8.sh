#!/bin/bash
# Inside the image: the splice with per-quantum records in the campaign's time order.
set -u
L=/cache/target/release/lockstep; A=/cache/target/debug/aethelgard
cargo build --offline --release --manifest-path spike/2303/lockstep/Cargo.toml 2>&1 | grep -E "^(warning|error)|-->|Finished"
RUN=$(ls -d /cache/camp2/archive/*/ | head -1)
$L splice $RUN /cache/camp2/splice2
for v in a b; do D=$(ls -d /cache/camp2/splice2/$v/*/); $A replay --archive $D | cut -c1-600; echo " replay($v) exit=$?"; done
