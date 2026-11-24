#!/bin/bash
# AET-WO-4400h (2)-(4), on the host: one injection run, three containers in turn (kit/img.sh, one at a time):
#   ardupilot image: plans.py, then frames.py PLAN -> /cache/inj/NAME/{inject.resc,injected.tsv}
#   tools image:     replay.sh of capture CAP with REQ=1 (the request line) INJ=that dir, LOG as given
#   ardupilot image: decode.py of ArduPilot's USART1 TX
# Usage: inject.sh NAME PLAN CAP [LOG]     e.g. inject.sh arm1 arm c16/1 1   (PLAN is arm|params|pad|dshot, written by py/plans.py; NOREQ=1 drops the line)
set -eu
name=$1; plan=$2; cap=$3; log=${4:-0}
K=$HOME/.cache/aet-4400h/kit/img.sh
$K ardupilot "mkdir -p /cache/inj/$name && python3 -I /spike/py/plans.py /cache/inj/$name && /opt/venv/bin/python -I /spike/py/frames.py /cache/inj/$name/$plan.json /cache/inj/$name" 2>&1 | grep -v '^capped-run'
req=1; [ "${NOREQ:-0}" = 1 ] && req=0
MEM=${MEM:-8G} $K tools "REQ=$req INJ=/cache/inj/$name LOG=$log LOGP='${LOGP:-}' RESETMACRO=${RESETMACRO:-0} bash /spike/sh/replay.sh /cache/cap/$cap /cache/run/$name" 2>&1 | grep -v '^capped-run'
$K ardupilot "/opt/venv/bin/python -I /spike/py/decode.py /cache/run/$name/tx.bin /cache/run/$name/mav.tsv" 2>&1 | grep -v '^capped-run' | tee ~/.cache/aet-4400h/run/$name/decode.txt
