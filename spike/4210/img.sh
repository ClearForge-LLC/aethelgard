#!/bin/bash
# AET-WO-4210 spike runner: the pinned tools image, under the architect's replacement rule 3
# (~/.cache/aet-bus/RULE3.md, 2026-11-15 03:40 UTC; PR #131 comment 6888603447):
# - the container runs INSIDE a capped systemd user scope that it shares:
#   systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=8192
#   podman run --rm --cgroups=disabled --network none ...
#   with no --memory, --memory-swap or --pids-limit flags (those left swap at 8 GiB: FEEDBACK F10);
# - inside the scope, before the container, memory.max, memory.swap.max and pids.max are read back
#   at /sys/fs/cgroup plus the path in /proc/self/cgroup, into spike/4210/out/caps.log; if
#   memory.swap.max is not 0, nothing runs (exit 98). Nothing writes any cgroup file;
# - repo at /repo (read-write: probes write spike/4210/out), caches at /cache (~/.cache/aet-4210);
#   CARGO_HOME and CARGO_TARGET_DIR under /cache; TMPDIR is never passed into the container;
# - one container of this work order at a time: refuses to start while another aet4210-* runs.
# Usage: img.sh -- <command...>
set -u
REPO=/home/forge/aethelgard-4210
CACHE=$HOME/.cache/aet-4210
CAPLOG=$REPO/spike/4210/out/caps.log
if [ "${1:-}" = --in-scope ]; then
  shift; NAME=$1; shift; [ "${1:-}" = -- ] && shift
  rel=$(sed -n 's/^0:://p' /proc/self/cgroup); d=/sys/fs/cgroup$rel
  mm=$(cat "$d/memory.max"); ms=$(cat "$d/memory.swap.max"); pm=$(cat "$d/pids.max")
  if [ "$ms" != 0 ]; then
    echo "$(date -Is) $NAME SWAP NOT ZERO, nothing runs: scope=$rel memory.max=$mm memory.swap.max=$ms pids.max=$pm cmd: $*" >> "$CAPLOG"
    exit 98
  fi
  echo "$(date -Is) $NAME scope=$rel memory.max=$mm memory.swap.max=$ms pids.max=$pm cmd: $*" >> "$CAPLOG"
  exec podman run --rm --name "$NAME" --cgroups=disabled --network none \
    -v "$REPO":/repo -v "$CACHE":/cache -w /repo \
    -e CARGO_HOME=/cache/cargo-home -e CARGO_TARGET_DIR=/cache/target \
    "$(cat "$REPO/container/IMAGE")" "$@"
fi
[ "${1:-}" = -- ] && shift
if podman ps --format '{{.Names}}' | grep -q '^aet4210-'; then
  echo "another aet4210 container is running; one at a time" >&2; exit 75
fi
mkdir -p "$(dirname "$CAPLOG")"
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
exec systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=8192 \
  bash "$0" --in-scope "aet4210-$$-$RANDOM" -- "$@"
