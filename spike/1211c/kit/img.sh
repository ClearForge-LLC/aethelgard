#!/bin/bash
# AET-WO-1211c image runner (from 1211b's kit/img.sh): ~/bin/capped-run (MEM default 8G, swap 0,
# TasksMax 8192, read back inside the scope), podman --rm --network none, one container at a time
# (flock), nice -n 10, TMPDIR unset inside. Usage: img.sh <command...>. REPO default the worktree;
# the cache is mounted at /cache.
set -u
CACHE=$HOME/.cache/aet-1211c
REPO=${REPO:-$HOME/aethelgard-1211c}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"; flock 9
echo "$(date -u -Is) image=$IMAGE repo=$REPO mem=${MEM:-8G} free_g=$(free -g | awk '/^Mem:/ {print $7}') cmd: $*" >> "$CACHE/caps.log"
CH=()
[ -n "${CARGO_HOME_DIR:-}" ] && CH=(-v "$CARGO_HOME_DIR":/cargo-home -e CARGO_HOME=/cargo-home)
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-8G}" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache "${CH[@]}" -w /repo \
  -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=${TDIR:-/cache/target-img} \
  "$IMAGE" env -u TMPDIR HOME=/cache/home "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
