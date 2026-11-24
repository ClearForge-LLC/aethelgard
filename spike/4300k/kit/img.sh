#!/bin/bash
# AET-WO-4300k image runner (from 4240e's kit/img.sh): ~/bin/capped-run (MEM default 8G, swap 0,
# TasksMax 8192, read back inside the scope), podman --rm --network none, one container of mine at a time
# (flock), nice -n 10. Usage: img.sh <command...>. REPO (default this worktree) read-only at /repo,
# ~/.cache/aet-4300k at /cache.
set -u
CACHE=$HOME/.cache/aet-4300k
REPO=${REPO:-$HOME/aethelgard-4300k}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"; flock 9
echo "$(date -Is) image=$IMAGE repo=$REPO mem=${MEM:-8G} cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-8G}" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo:ro -v "$CACHE":/cache -v "$CACHE/cargo-home":/cargo-home -w /repo \
  -e CARGO_HOME=/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=${TDIR:-/cache/target} -e HOME=/cache/home \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
