#!/bin/bash
# AET-WO-4400i (1), on the host: one injection run, three containers in turn (kit/img.sh, one at a time, capped):
#   ardupilot image: plans.py, then frames.py PLAN -> /cache/inj/NAME/{inject.resc,injected.tsv}
#   tools image:     replay.sh of capture CAP with REQ=1 (NOREQ=1 drops it), IDLE as given, INJ=that dir
#   ardupilot image: timeline.py of ArduPilot's USART1 TX with each byte's virtual time
# Usage: inject.sh NAME PLAN [CAP]
set -eu
name=$1; plan=$2; cap=${3:-s03/2}
K=$HOME/.cache/aet-4400i/kit/img.sh
$K ardupilot "mkdir -p /cache/inj/$name && python3 -I /spike/py/plans.py /cache/inj/$name && /opt/venv/bin/python -I /spike/py/frames.py /cache/inj/$name/$plan.json /cache/inj/$name" 2>&1 | grep -v '^capped-run'
req=1; [ "${NOREQ:-0}" = 1 ] && req=0
MEM=${MEM:-4G} $K tools "REQ=$req IDLE=${IDLE:-0} LOGP='${LOGP:-}' INJ=/cache/inj/$name bash /spike/sh/replay.sh /cache/cap/$cap /cache/run/$name" 2>&1 | grep -v '^capped-run'
$K ardupilot "/opt/venv/bin/python -I /spike/py/timeline.py /cache/run/$name" 2>&1 | grep -v '^capped-run' | tee ~/.cache/aet-4400i/run/$name/timeline.txt
