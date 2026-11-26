#!/bin/bash
# AET-WO-4412 kit (from 4400j's): one command inside a pinned image under ~/bin/capped-run (MEM default 8G, swap 0,
# TasksMax 8192, read back in the scope), --rm, --network none, one container at a time (flock), nice -n 10.
# Mounts: REPO (default this worktree) at /repo (target/ inside it), ~/.cache/aet-4412 at /cache, the spike read-only.
# Usage: img.sh tools|ardupilot 'shell command'   (CPUS=0-3 pins with taskset; ROOT=1 runs as the image's root)
set -u
CACHE=$HOME/.cache/aet-4412
REPO=${REPO:-$HOME/aethelgard-4412}
SPIKE=$HOME/aethelgard-4412/spike/4412
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
case $1 in
  tools) IMAGE=$(cat "$REPO/container/IMAGE");;
  ardupilot) IMAGE=$(head -1 "$REPO/container/ARDUPILOT-IMAGE");;
  *) echo "img.sh tools|ardupilot 'cmd'" >&2; exit 64;;
esac
exec 9>"$CACHE/kit/container.lock"; flock 9
echo "$(date -u -Is) image=$IMAGE mem=${MEM:-8G} cpus=${CPUS:-all} repo=$REPO cmd: $2" >> "$CACHE/kit/caps.log"
who=(--userns=keep-id --user "$(id -u):$(id -g)"); [ "${ROOT:-0}" = 1 ] && who=()
pin=""; [ -n "${CPUS:-}" ] && pin="taskset -c $CPUS"
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-8G}" --tasks 8192 -- --rm "${who[@]}" $( [ "${NET:-0}" = 1 ] || echo --network none ) \
  -v "$REPO:/repo" -v "$CACHE:/cache" -v "$SPIKE:/spike:ro" --tmpfs "/run/user/$(id -u):rw,mode=0700,U" \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_TARGET_DIR=/repo/target -e HOME=/cache/home -w /repo \
  "$IMAGE" $pin bash -c "export PATH=/usr/local/cargo/bin:\$PATH; mkdir -p /cache/home; $2"
