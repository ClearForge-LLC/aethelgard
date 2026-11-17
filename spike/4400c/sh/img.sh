#!/bin/bash
# AET-WO-4400c image runner: ~/bin/capped-run (scope MemoryMax=8G, MemorySwapMax=0, TasksMax=8192,
# caps read back inside the scope before the container; refuses swap != 0), podman --rm
# --network none, TMPDIR unset inside, one container at a time (flock).
# Usage: img.sh tools|ardupilot <command...>   (worktree at /repo, ~/.cache/aet-4400c at /cache)
set -u
CACHE=$HOME/.cache/aet-4400c
REPO=$HOME/aethelgard-4400c
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
case $1 in
  tools) IMAGE=$(cat "$REPO/container/IMAGE");;
  ardupilot) IMAGE=$(head -1 "$REPO/container/ARDUPILOT-IMAGE");;
  *) echo "img.sh tools|ardupilot <cmd...>" >&2; exit 64;;
esac
shift
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) image=$IMAGE cmd: $*" >> "$CACHE/caps.log"
exec ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=/cache/target \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
