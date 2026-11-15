#!/bin/bash
# AET-WO-4220 spike runner: one command in the pinned tools image, through ~/bin/capped-run
# (RULE3.md as updated 2026-11-15 05:15 UTC: a capped systemd user scope the container shares,
# swap read back as 0 or nothing runs, no podman memory/pids/cgroup flags), --network none,
# TMPDIR never passed in. The repository at /repo, ~/.cache/aet-4220 at /cache.
# capped-run's read-back line (stderr) is appended to spike/4220/out/caps.log with the command.
# One container of this work order at a time.
# Usage: run.sh -- <command...>
set -u
REPO=/home/forge/aethelgard-4220
CACHE=$HOME/.cache/aet-4220
LOG=$REPO/spike/4220/out/caps.log
[ "${1:-}" = -- ] && shift
if podman ps --format '{{.Names}}' | grep -q '^aet4220-'; then
  echo "another aet4220 container is running; one at a time" >&2; exit 75
fi
mkdir -p "$(dirname "$LOG")"
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
NAME=aet4220-$$-$RANDOM
echo "$(date -Is) $NAME cmd: $*" >> "$LOG"
"$HOME/bin/capped-run" --mem 8G --tasks 8192 -- --rm --name "$NAME" --network none \
  -v "$REPO":/repo -v "$CACHE":/cache -w /repo \
  "$(cat "$REPO/container/IMAGE")" "$@" 2> >(tee -a "$LOG" >&2)
rc=$?
echo "$(date -Is) $NAME exit $rc" >> "$LOG"
exit $rc
