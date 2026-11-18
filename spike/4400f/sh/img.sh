#!/bin/bash
# AET-WO-4400f image runner (from 4400e's): ~/bin/capped-run (MemoryMax=8G, MemorySwapMax=0, TasksMax=8192,
# read back inside the scope), podman --rm --network none, one container at a time (flock).
# Usage: img.sh tools|ardupilot|stock <command...>
#   tools: container/IMAGE (the pinned .aet1605 build); stock: the image before 1605 (598501ab, a stock
#   1.17.0 build), for the pinned-build question only.
# Mounts: REPO (default this worktree) at /repo, ~/.cache/aet-4400f at /cache, 4400e's cargo cache at /cargo-home.
set -u
CACHE=$HOME/.cache/aet-4400f
REPO=${REPO:-$HOME/aethelgard-4400f}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
case $1 in
  tools) IMAGE=$(cat "$HOME/aethelgard-4400f/container/IMAGE");;
  ardupilot) IMAGE=$(head -1 "$HOME/aethelgard-4400f/container/ARDUPILOT-IMAGE");;
  stock) IMAGE=ghcr.io/clearforge-llc/aethelgard-tools@sha256:598501abd610a9fd84935cc32c2878e7932eae4df8f774789eaea21ec3555625;;
  *) echo "img.sh tools|ardupilot|stock <cmd...>" >&2; exit 64;;
esac
shift
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -Is) image=$IMAGE repo=$REPO cmd: $*" >> "$CACHE/caps.log"
exec ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -v "$HOME/.cache/aet-4400e/cargo-home":/cargo-home -w /repo \
  -e CARGO_HOME=/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=${TDIR:-/cache/target} \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
