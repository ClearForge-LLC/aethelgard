#!/bin/bash
# AET-WO-2303 image runner: ~/bin/capped-run (scope MemoryMax=8G, MemorySwapMax=0, TasksMax=8192,
# caps read back inside the scope before the container, refuses swap != 0), podman --rm
# --network none, TMPDIR unset inside. One container at a time (flock).
# Mounts: the worktree at /repo, ~/.cache/aet-2303 at /cache. Usage: img.sh <command...>
set -u
CACHE=$HOME/.cache/aet-2303
REPO=$HOME/aethelgard-2303
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) cmd: $*" >> "$CACHE/caps.log"
exec ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=/cache/target \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
