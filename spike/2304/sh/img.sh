#!/bin/bash
# AET-WO-2304 image runner (from spike/2303/sh/img.sh): ~/bin/capped-run (scope MemoryMax=$MEM,
# default 8G; MemorySwapMax=0; TasksMax=8192; caps read back inside the scope before the container,
# refuses swap != 0), podman --rm --network none, TMPDIR unset inside, nice -n 10. One container at
# a time (flock). Mounts: the worktree at /repo (read-only), ~/.cache/aet-2304 at /cache.
# Usage: [MEM=16G] img.sh <command...>
set -u
CACHE=$HOME/.cache/aet-2304
REPO=$HOME/aethelgard-2304
MEM=${MEM:-8G}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) mem=$MEM cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "$MEM" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo:ro -v "$CACHE":/cache -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=/cache/target \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
