#!/bin/bash
# AET-WO-1211f image runner (1211d's kit/img.sh, its cargo mounts dropped): the tools image under
# ~/bin/capped-run (MEM default 8G, swap 0, TasksMax 8192, read back inside the scope), podman --rm
# --network none, one container at a time (flock), nice -n 10. The host-built CLI is at /bin-host.
# Usage: img.sh <command...>. Mounts: REPO (default this worktree) at /repo, ~/.cache/aet-1211f at
# /cache, BIN (default the branch's release dir) at /bin-host.
set -u
CACHE=$HOME/.cache/aet-1211f
REPO=${REPO:-$HOME/aethelgard-1211f}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"; flock 9
echo "$(date -Is) image=$IMAGE repo=$REPO mem=${MEM:-8G} cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-8G}" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -v "${BIN:-$CACHE/target-br/release}":/bin-host:ro -w /repo \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
