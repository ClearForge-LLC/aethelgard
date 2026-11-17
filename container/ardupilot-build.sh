#!/usr/bin/env bash
# container/ardupilot-build.sh: ArduPilot builds inside the aethelgard-ardupilot
# image (AET-WO-4400a), each meant to run under --network none.
#
#   smoke                 configure and build copter for MatekH743, a board
#                         ArduPilot ships; the tree, untracked files included,
#                         stays clean.
#   d4                    the D4 control: with GITHUB_ACTIONS=true, as GitHub's
#                         runner sets it, configure refuses step G's numeric
#                         APJ_BOARD_ID. Passes only if it refuses with that message.
#   boards NAME...        install /boards/NAME.hwdef.dat as the untracked board
#                         NAME (AET-WO-4400b: each board under its own name, so
#                         no two boards in one image share a board or build
#                         directory), build copter from a clean build
#                         directory, and copy the ELF and hwdef.h to /out/NAME/.
#                         NAME is letters, digits, `-` and `_`. Reusing the
#                         directory saves nothing (AET-WO-4400a FEEDBACK), so it
#                         is never reused.
#
# Every ArduPilot build runs with GITHUB_ACTIONS unset (ruling D4 on PR #130):
# ArduPilot refuses a numeric APJ_BOARD_ID when it is "true"
# (libraries/AP_HAL/hwdef/scripts/hwdef.py:129-139, running_in_CI), and a name
# would have to be added to ArduPilot's board_types.txt, which would no longer
# be unmodified ArduPilot. A container does not inherit the runner's
# environment; the unset says so explicitly.
set -euo pipefail
cd /opt/ardupilot
board=libraries/AP_HAL_ChibiOS/hwdef/Sentinel01
jobs=$(nproc)

clean_tree() {
  local dirty
  dirty=$(git status --porcelain "$@")
  [ -z "$dirty" ] || { echo "ArduPilot's tree changed:" >&2; echo "$dirty" >&2; exit 1; }
}

since() { echo "$(($(date +%s%N) / 1000000 - $1))"; }

case ${1:-} in
  smoke)
    t=$(($(date +%s%N) / 1000000))
    env -u GITHUB_ACTIONS ./waf configure --board MatekH743
    env -u GITHUB_ACTIONS ./waf copter -j"$jobs"
    clean_tree
    echo "ANSWER smoke board=MatekH743 jobs=$jobs wall_ms=$(since "$t") flash_bytes=$(stat -c %s build/MatekH743/bin/arducopter.bin)"
    ;;
  d4)
    mkdir -p "$board"
    cp /boards/step-g.hwdef.dat "$board/hwdef.dat"
    if out=$(GITHUB_ACTIONS=true ./waf configure --board Sentinel01 2>&1); then
      echo "$out" >&2
      echo "D4 control: configure accepted a numeric APJ_BOARD_ID with GITHUB_ACTIONS=true" >&2
      exit 1
    fi
    grep -F 'Numeric board ID found (7777)' <<< "$out" \
      || { echo "$out" >&2; echo "D4 control: configure failed, but not on the board ID" >&2; exit 1; }
    echo "ANSWER d4 control: refused with GITHUB_ACTIONS=true"
    ;;
  boards)
    shift
    for name in "$@"; do
      [[ $name =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || { echo "bad board name: $name" >&2; exit 64; }
      dir=libraries/AP_HAL_ChibiOS/hwdef/$name
      mkdir -p "$dir"
      cp "/boards/$name.hwdef.dat" "$dir/hwdef.dat"
      rm -rf "build/$name"
      t=$(($(date +%s%N) / 1000000))
      env -u GITHUB_ACTIONS ./waf configure --board "$name"
      env -u GITHUB_ACTIONS ./waf copter -j"$jobs"
      clean_tree --untracked-files=no
      mkdir -p "/out/$name"
      cp "build/$name/bin/arducopter" "/out/$name/arducopter.elf"
      cp "build/$name/hwdef.h" "/out/$name/hwdef.h"
      echo "ANSWER board=$name jobs=$jobs wall_ms=$(since "$t")"
    done
    ;;
  *)
    echo "usage: ardupilot-build.sh smoke | d4 | boards NAME..." >&2
    exit 64
    ;;
esac
