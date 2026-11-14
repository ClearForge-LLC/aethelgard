#!/bin/bash
# AET-WO-4100 spike: the pinned tools image, as CI's tools job runs it (TMPDIR unset), capped:
# --memory 8g --memory-swap 8g --pids-limit -1; swap zeroed in the container's own cgroup after
# start; memory.max, memory.swap.max, pids.max read back into spike/4100/caps.log.
# Usage: img.sh [--vol HOST:CTR]... <command...>   (repo at /repo, ~/.cache/aet-4100 at /work)
REPO=/home/forge/aethelgard-4100
NAME="aet4100-$$"; EXTRA=()
while [[ "$1" == --* ]]; do case "$1" in --vol) EXTRA+=(-v "$2"); shift;; esac; shift; done
CAPLOG=$REPO/spike/4100/caps.log
( for i in $(seq 1 120); do
    p=$(podman inspect -f '{{.State.CgroupPath}}' "$NAME" 2>/dev/null) && [ -n "$p" ] && [ -e "/sys/fs/cgroup$p/memory.swap.max" ] && {
      echo 0 > "/sys/fs/cgroup$p/memory.swap.max"
      echo "$(date -Is) $NAME memory.max=$(cat /sys/fs/cgroup$p/memory.max) memory.swap.max=$(cat /sys/fs/cgroup$p/memory.swap.max) pids.max=$(cat /sys/fs/cgroup$p/pids.max)" >> "$CAPLOG"; exit 0; }
    sleep 0.5; done; echo "$(date -Is) $NAME CAPS NOT READ" >> "$CAPLOG" ) &
exec podman run --rm --name "$NAME" --network none --memory 8g --memory-swap 8g --pids-limit -1 \
  -v $REPO:/repo:ro -v /home/forge/.cache/aet-4100:/work -w /work "${EXTRA[@]}" \
  "$(cat $REPO/container/IMAGE)" "$@"
