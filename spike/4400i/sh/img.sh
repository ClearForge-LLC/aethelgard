#!/bin/bash
# AET-WO-4400i kit (from 4400h's): one command inside a pinned image under ~/bin/capped-run (MEM default 4G,
# swap 0, TasksMax 8192, read back in the scope), --rm --network none, one container at a time (flock), nice -n 10.
# Mounts: this worktree at /repo (target/ inside it), ~/.cache/aet-4400i at /cache, the spike read-only at /spike.
# Usage: img.sh tools|ardupilot 'shell command'
set -u
CACHE=$HOME/.cache/aet-4400i
REPO=${REPO:-/home/forge/aethelgard-4400i}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
case $1 in
  tools) IMAGE=$(cat "$REPO/container/IMAGE");;
  ardupilot) IMAGE=$(head -1 "$REPO/container/ARDUPILOT-IMAGE");;
  *) echo "img.sh tools|ardupilot 'cmd'" >&2; exit 64;;
esac
exec 9>"$CACHE/kit/container.lock"; flock 9
echo "$(date -u -Is) image=$IMAGE mem=${MEM:-4G} cmd: $2" >> "$CACHE/kit/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-4G}" --tasks 8192 -- --rm --userns=keep-id --user "$(id -u):$(id -g)" --network none \
  -v "$REPO:/repo" -v "$CACHE:/cache" -v /home/forge/aethelgard-4400i/spike/4400i:/spike:ro --tmpfs "/run/user/$(id -u):rw,mode=0700,U" \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_TARGET_DIR=/repo/target -e HOME=/cache/home -w /repo \
  "$IMAGE" bash -c "export PATH=/usr/local/cargo/bin:\$PATH; mkdir -p /cache/home; $2"
