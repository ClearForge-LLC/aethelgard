#!/usr/bin/env bash
# judge.sh BIN: check pinout on every 4404 board with its topology; one line per board.
bin=$1; A=$HOME/.cache/aet-4404
top() { case $1 in fc-mav3|adv-proto-del3) echo tops/fc-mav3.top19;; eight*) echo tops/eight.top19;; *) echo tops/fc-base.top19;; esac; }
for d in $A/out/* $A/ci/out/eight $A/adv/out/*; do
  n=$(basename $d)
  out=$("$bin" check pinout --topology "$(top $n)" --elf $d/arducopter.elf --hwdef-h $d/hwdef.h); rc=$?
  echo "$n exit $rc $(jq -c '[.findings[]? | select(.check=="identity") | "\(.kind) \(.pin) [\(.expected) | \(.decoded)]"]' <<< "$out") $(jq -c '.error? // empty' <<< "$out")"
done
