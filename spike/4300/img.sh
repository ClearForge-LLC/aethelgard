#!/bin/bash
# AET-WO-4200 spike runner (2204's, with REPO/CACHE/caps log for 4200): the pinned tools image under rootless podman, as CI's tools job runs it.
# - repo at /repo (read-write: probes write spike/4200/out), caches at /cache (~/.cache/aet-4200);
# - CARGO_HOME and CARGO_TARGET_DIR under /cache, so nothing builds into the worktree;
# - TMPDIR unset inside the container;
# - --memory/--memory-swap $MEM (default 12g) and --pids-limit -1; after start, 0 is written into the
#   container's own memory.swap.max, and memory.max, memory.swap.max and pids.max are read back
#   into spike/4200/out/caps.log with the command.
# - The command inside does not start until the host has zeroed swap and read the caps back (a flag
#   file under /capflags); if that never happens it exits 99 having run nothing. (The first version
#   polled after start, and six short containers ended before the poll: FEEDBACK.)
# Usage: img.sh [--net] [--name N] [--mem 12g] -- <command...>
set -u
REPO=${REPO:-/home/forge/aethelgard-4200}
CACHE=${CACHE:-$HOME/.cache/aet-4200}
NET="--network none"; NAME="aet4200-$$-$RANDOM"; MEM=12g
while [[ $# -gt 0 && "$1" == --* ]]; do
  case "$1" in
    --net) NET="";;
    --name) NAME="$2"; shift;;
    --mem) MEM="$2"; shift;;
    --) shift; break;;
  esac
  shift
done
CAPLOG=${CAPLOG:-$REPO/spike/4200/out/caps.log}
FLAGS=$CACHE/capflags
mkdir -p "$(dirname "$CAPLOG")" "$FLAGS"
rm -f "$FLAGS/$NAME"
( for _ in $(seq 1 240); do
    p=$(podman inspect -f '{{.State.CgroupPath}}' "$NAME" 2>/dev/null) && [ -n "$p" ] \
      && [ -e "/sys/fs/cgroup$p/memory.swap.max" ] && {
      echo 0 > "/sys/fs/cgroup$p/memory.swap.max"
      echo "$(date -Is) $NAME memory.max=$(cat /sys/fs/cgroup$p/memory.max) memory.swap.max=$(cat /sys/fs/cgroup$p/memory.swap.max) pids.max=$(cat /sys/fs/cgroup$p/pids.max) cmd: $*" >> "$CAPLOG"
      touch "$FLAGS/$NAME"
      exit 0; }
    sleep 0.25
  done
  echo "$(date -Is) $NAME CAPS NOT READ cmd: $*" >> "$CAPLOG" ) &
exec podman run --rm --name "$NAME" $NET --memory "$MEM" --memory-swap "$MEM" --pids-limit -1 \
  -v "$REPO":/repo -v "$CACHE":/cache -v "$FLAGS":/capflags:ro -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_TARGET_DIR=/cache/target \
  "$(cat "$REPO/container/IMAGE")" \
  bash -c 'for _ in $(seq 1 600); do [ -e "/capflags/$0" ] && exec "$@"; sleep 0.1; done; echo "caps never read back: nothing ran" >&2; exit 99' "$NAME" "$@"
