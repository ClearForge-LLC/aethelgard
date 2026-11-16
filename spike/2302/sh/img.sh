#!/bin/bash
# AET-WO-2302 image runner: ~/bin/capped-run (scope MemoryMax=8G, MemorySwapMax=0, TasksMax=8192,
# caps read back, refuses swap != 0), podman --rm --network none, TMPDIR unset inside.
# One container at a time (flock). Mounts: $SRC at /repo, ~/.cache/aet-2302 at /cache.
# Usage: img.sh <src-dir> <command...>
set -u
CACHE=$HOME/.cache/aet-2302
SRC=$1; shift
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$HOME/aethelgard-2302/container/IMAGE")
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) cmd: $*" >> "$CACHE/caps.log"
exec ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none \
  -v "$SRC":/repo -v "$CACHE":/cache -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
