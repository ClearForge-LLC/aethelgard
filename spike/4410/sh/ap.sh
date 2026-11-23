#!/usr/bin/env bash
# ap.sh BOARDS_DIR OUT_DIR NAME...: CI's generated-board build, in the pinned ArduPilot image, capped (capped-run), one container.
set -euo pipefail
REPO=$(git rev-parse --show-toplevel)
b=$(realpath "$1"); o=$(realpath "$2"); shift 2
mkdir -p "$o"
IMG=$(cat $REPO/container/ARDUPILOT-IMAGE)
exec nice -n 10 ~/bin/capped-run --mem 8G -- --rm --network none \
  -v $REPO/container:/container:ro -v "$b:/boards:ro" -v "$o:/out" \
  "$IMG" bash /container/ardupilot-build.sh boards "$@"
