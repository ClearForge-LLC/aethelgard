#!/bin/bash
# AET-WO-1303 spike: one command in the pinned tools image, by the architect's
# replacement rule 3 (~/.cache/aet-bus/RULE3.md, 2026-11-15 03:40 UTC): the
# container runs INSIDE a capped systemd user scope it shares
# (MemoryMax=8G, MemorySwapMax=0, TasksMax=8192), with podman --cgroups=disabled
# and no --memory/--memory-swap/--pids-limit flags. Inside the scope, before the
# container, memory.max, memory.swap.max and pids.max are read back from
# /sys/fs/cgroup + the path in /proc/self/cgroup into caps.log; the run stops
# (exit 99) if memory.swap.max is not 0. Nothing writes into a cgroup file.
# TMPDIR is left unset inside the container. The repository is mounted at /repo.
# --expand-environment=no: systemd 259 expands braced ${NAME} in the command
# (measured here and by 4210); this script uses none, but the flag keeps it so.
# (The first version of this script, with equal --memory/--memory-swap flags,
# read memory.swap.max = 8589934592 and stopped: caps.log, FEEDBACK D9.)
# Usage: spike/1303/img.sh <command...>
set -u
REPO=$(cd "$(dirname "$0")/../.." && pwd)
CAPLOG="$REPO/spike/1303/caps.log"
IMAGE=$(cat "$REPO/container/IMAGE")
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
exec systemd-run --user --scope -q --expand-environment=no -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=8192 \
  bash -c '
    cg=$(sed -n "s/^0:://p" /proc/self/cgroup); d=/sys/fs/cgroup$cg
    m=$(cat "$d/memory.max"); s=$(cat "$d/memory.swap.max"); k=$(cat "$d/pids.max")
    echo "$(date -Is) scope $cg memory.max=$m memory.swap.max=$s pids.max=$k" >> "$0"
    if [ "$s" != "0" ]; then echo "$(date -Is) STOP: memory.swap.max is $s, not 0" >> "$0"; exit 99; fi
    exec podman run --rm --cgroups=disabled --network none -v "$1":/repo -w /repo "$2" "${@:3}"
  ' "$CAPLOG" "$REPO" "$IMAGE" "$@"
