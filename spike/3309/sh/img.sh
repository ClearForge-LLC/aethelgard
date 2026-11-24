#!/bin/bash
# AET-WO-3309 image runner (3307's) (from 3305's): capped-run (MemoryMax=$MEM, default 8G, swap 0, tasks 8192,
# caps read back), podman --rm --network none, nice -n 10, one container at a time (flock).
# Mounts: $REPO (default the worktree) at /repo read-only, ~/.cache/aet-3309 at /cache, spike/3309 at /spike read-only.
# Usage: [MEM=16G] [REPO=dir] [TARGET=/cache/...] img.sh <command...>
set -u
CACHE=$HOME/.cache/aet-3309
REPO=${REPO:-$HOME/aethelgard-3309}
MEM=${MEM:-8G}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) mem=$MEM repo=$REPO cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "$MEM" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo:ro -v "$CACHE":/cache -v "$HOME/aethelgard-3309/spike/3309":/spike:ro -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=${TARGET:-/cache/target} \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
