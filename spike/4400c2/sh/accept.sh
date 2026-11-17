#!/bin/bash
# Inside the tools image: §2's acceptance. The CLI built from this branch generates sentinel-02-own;
# AET-WO-4400c's own gen.sh (unchanged, on main via #202) captures the CLI's script; AET-WO-4400c's
# drive.py (unchanged) runs its r3 (3 s, 500 ms steps) on the emitted platform with NO overlay.
set -u
cargo build --locked --offline -p aethelgard-cli 2>&1 | tail -1
cargo build --offline --manifest-path bridge/Cargo.toml --locked 2>&1 | tail -1
rm -rf /cache/s02own; mkdir -p /cache/s02own
/cache/target/debug/aethelgard generate --spec /repo/crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json --out /cache/s02own/gen > /cache/s02own/generate.json 2>&1; echo "generate exit=$?"
grep -n 'DMARecieve' /cache/s02own/gen/board/sentinel-02-own.repl
bash /repo/spike/4400c/sh/gen.sh 2>&1 | grep -v cmdline | tail -4
O=/cache/runs/accept-r3; rm -rf $O; mkdir -p $O
python3 -I /repo/spike/4400c/drive.py $O 3 500 > $O/steps.jsonl 2> $O/drive.err; echo "drive exit=$?"
strings -n 6 $O/tool/capture/usart1.bin | grep -E 'SPI Flash|Init ArduCopter'
sha256sum /repo/spike/4400c/drive.py
