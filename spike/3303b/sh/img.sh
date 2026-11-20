#!/bin/bash
# AET-WO-3303b image runner (from spike/4600/sh/img.sh): ~/bin/capped-run (scope MemoryMax=$MEM, default
# 16G; MemorySwapMax=0; TasksMax=8192; capped-run reads the caps back inside the scope and refuses unless
# swap is 0), podman --rm --network none, TMPDIR unset inside, nice -n 10. One container at a time
# (flock). Mounts: the tree $TREE read-only at /repo (default: this worktree), the cache
# ~/.cache/aet-3303b at /cache. Usage: [MEM=16G] [TREE=dir] [TARGET=name] img.sh <command...>
set -u
CACHE=$HOME/.cache/aet-3303b
TREE=${TREE:-$HOME/aethelgard-3303b}
MEM=${MEM:-16G}
TARGET=${TARGET:-target}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$TREE/container/IMAGE")
exec 9>"$CACHE/container.lock"
flock 9
echo "$(date -u -Is) mem=$MEM tree=$TREE cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "$MEM" --tasks 8192 -- --rm --network none \
  -v "$TREE":/repo:ro -v "$CACHE":/cache -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=/cache/$TARGET \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
