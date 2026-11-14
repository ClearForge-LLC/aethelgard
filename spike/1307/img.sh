#!/bin/bash
# AET-WO-1307 spike: one command in the pinned tools image, capped as the kickoff rules:
# --memory 8g --memory-swap 8g --pids-limit -1, swap zeroed in the container's own cgroup after
# start, and memory.max, memory.swap.max, pids.max read back into spike/1307/caps.log. TMPDIR is
# left unset. The repo is mounted read-only at /repo, ~/.cache/aet-1307 at /work.
# Usage: img.sh <command...>
REPO=$(cd "$(dirname "$0")/../.." && pwd)
NAME="aet1307-$$"
CAPLOG=$REPO/spike/1307/caps.log
( for i in $(seq 1 120); do
    p=$(podman inspect -f '{{.State.CgroupPath}}' "$NAME" 2>/dev/null) && [ -n "$p" ] && [ -e "/sys/fs/cgroup$p/memory.swap.max" ] && {
      echo 0 > "/sys/fs/cgroup$p/memory.swap.max"
      echo "$(date -Is) $NAME memory.max=$(cat /sys/fs/cgroup$p/memory.max) memory.swap.max=$(cat /sys/fs/cgroup$p/memory.swap.max) pids.max=$(cat /sys/fs/cgroup$p/pids.max)" >> "$CAPLOG"; exit 0; }
    sleep 0.5; done; echo "$(date -Is) $NAME CAPS NOT READ" >> "$CAPLOG" ) &
exec podman run --rm --name "$NAME" --network none --memory 8g --memory-swap 8g --pids-limit -1 \
  -v "$REPO":/repo:ro -v "$HOME/.cache/aet-1307":/work -w /work \
  "$(cat "$REPO/container/IMAGE")" "$@"
