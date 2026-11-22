#!/bin/bash
# M4g: d1-held-high.resc in the image; output ~/.cache/aet-1211c/runs/d1-held-high.log.
K=$(cd "$(dirname "$0")" && pwd)
"$K/img.sh" bash -c 'timeout 600 renode --disable-gui --console --plain -e "include @/repo/spike/1211c/kit/d1-held-high.resc; quit" </dev/null 2>&1; echo "exit $?"' > ~/.cache/aet-1211c/runs/d1-held-high.log
