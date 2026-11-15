#!/bin/bash
# s3.sh <step>: install spike/4400/hwdef/<step>.hwdef.dat as the untracked board
# Sentinel01 in the clone, then configure and build copter (nice 10, capped, -j6).
set -u
step=$1
A=$HOME/.cache/aet-4400
B=$A/ardupilot/libraries/AP_HAL_ChibiOS/hwdef/Sentinel01
mkdir -p $B && cp "$(dirname "$0")/hwdef/$step.hwdef.dat" $B/hwdef.dat
rm -rf $A/ardupilot/build/Sentinel01
export XDG_RUNTIME_DIR=/run/user/$(id -u)
systemd-run --user --scope -q -p MemoryMax=16G -p MemorySwapMax=0 -- nice -n 10 bash -c \
 "$A/run.sh s3-$step-cfg venv314 python ./waf configure --board Sentinel01 && $A/run.sh s3-$step-copter venv314 python ./waf copter -j6"
echo "tracked changes in clone: $(git -C $A/ardupilot status --porcelain --untracked-files=no | wc -l)"
