#!/bin/bash
# AET-WO-1303 spike: one command in the pinned tools image, rootless podman.
# Caps: --memory and --memory-swap equal (8g), so under cgroup v2 podman sets
# memory.swap.max to 0 itself; nothing here writes under /sys/fs/cgroup
# (architect's correction to kickoff rule 3). memory.max, memory.swap.max and
# pids.max are read back into caps.log; if memory.swap.max is not 0 the
# container is killed and this script exits 99. TMPDIR is left unset inside.
# Usage: spike/1303/img.sh <command...>   (the repository is mounted at /repo)
set -u
REPO=$(cd "$(dirname "$0")/../.." && pwd)
NAME="aet1303-$$"
CAPLOG="$REPO/spike/1303/caps.log"
IMAGE=$(cat "$REPO/container/IMAGE")
(
  for _ in $(seq 1 240); do
    p=$(podman inspect -f '{{.State.CgroupPath}}' "$NAME" 2>/dev/null) || p=""
    if [ -n "$p" ] && [ -e "/sys/fs/cgroup$p/memory.swap.max" ]; then
      m=$(cat "/sys/fs/cgroup$p/memory.max"); s=$(cat "/sys/fs/cgroup$p/memory.swap.max"); k=$(cat "/sys/fs/cgroup$p/pids.max")
      echo "$(date -Is) $NAME memory.max=$m memory.swap.max=$s pids.max=$k" >> "$CAPLOG"
      if [ "$s" != "0" ]; then
        echo "$(date -Is) $NAME STOP: memory.swap.max is $s, not 0; killed" >> "$CAPLOG"
        podman kill "$NAME" >/dev/null 2>&1
      fi
      exit 0
    fi
    sleep 0.25
  done
  echo "$(date -Is) $NAME CAPS NOT READ" >> "$CAPLOG"
  podman kill "$NAME" >/dev/null 2>&1
) &
watcher=$!
podman run --rm --name "$NAME" --network none --memory 8g --memory-swap 8g --pids-limit -1 \
  -v "$REPO":/repo -w /repo "$IMAGE" "$@"
rc=$?
wait "$watcher"
if grep -q "$NAME STOP\|$NAME CAPS NOT READ" "$CAPLOG"; then exit 99; fi
exit $rc
