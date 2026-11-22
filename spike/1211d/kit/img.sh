#!/bin/bash
# AET-WO-1211d image runner (from 4240c's kit/img.sh; the host-built CLI at /bin-host): ~/bin/capped-run (MEM default 8G, swap 0,
# TasksMax 8192, read back inside the scope), podman --rm --network none, one container at a time
# (flock), nice -n 10. Usage: img.sh tools|ardupilot <command...>
# Mounts: REPO (default this worktree) at /repo, ~/.cache/aet-1211d at /cache, 4400e's cargo cache.
set -u
CACHE=$HOME/.cache/aet-1211d
REPO=${REPO:-$HOME/aethelgard-1211d}
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
case $1 in
  tools) IMAGE=$(cat "$REPO/container/IMAGE");;
  ardupilot) IMAGE=$(head -1 "$REPO/container/ARDUPILOT-IMAGE");;
  *) echo "img.sh tools|ardupilot <cmd...>" >&2; exit 64;;
esac
shift
if [ -z "${NOLOCK:-}" ]; then exec 9>"$CACHE/container.lock"
flock 9; fi
echo "$(date -Is) image=$IMAGE repo=$REPO mem=${MEM:-8G} cmd: $*" >> "$CACHE/caps.log"
exec nice -n 10 ~/bin/capped-run --mem "${MEM:-8G}" --tasks 8192 -- --rm --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -v "$HOME/.cache/aet-4400e/cargo-home":/cargo-home -v "${BIN:-$CACHE/target/release}":/bin-host:ro -w /repo \
  -e CARGO_HOME=/cargo-home -e CARGO_NET_OFFLINE=true -e CARGO_TARGET_DIR=${TDIR:-/cache/target} \
  "$IMAGE" "$@" 2> >(tee -a "$CACHE/caps.log" >&2)
