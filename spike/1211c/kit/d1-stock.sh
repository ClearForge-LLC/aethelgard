#!/bin/bash
# M4: run d1-stock.resc in the image; output ~/.cache/aet-1211c/runs/d1-stock.log.
K=$(cd "$(dirname "$0")" && pwd)
"$K/img.sh" bash -c 'renode --version; timeout 600 renode --disable-gui --console --plain -e "include @/repo/spike/1211c/kit/d1-stock.resc; quit" </dev/null 2>&1; echo "exit $?"' > ~/.cache/aet-1211c/runs/d1-stock.log
# The twin's platform for 4600's motor driver, which D1's resc loads (written by the kit's caller):
# aethelgard generate --spec crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json --out /cache/m/djn/gen
# cp /cache/m/djn/gen/board/*.repl /cache/m/djn.repl
