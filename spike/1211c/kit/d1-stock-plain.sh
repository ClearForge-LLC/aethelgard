#!/bin/bash
# M4: run d1-stock-plain.resc in the image; output ~/.cache/aet-1211c/runs/d1-stock-plain.log.
K=$(cd "$(dirname "$0")" && pwd)
"$K/img.sh" bash -c 'renode --version; timeout 600 renode --disable-gui --console --plain -e "include @/repo/spike/1211c/kit/d1-stock-plain.resc; quit" </dev/null 2>&1; echo "exit $?"' > ~/.cache/aet-1211c/runs/d1-stock-plain.log
