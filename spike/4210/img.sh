#!/bin/bash
# AET-WO-4210 spike runner: the pinned tools image under rootless podman, as 4200's
# `img.sh` (~/.cache/aet-4200/spike-copy/img.sh) with this work order's paths and caps.
# - repo at /repo (read-write: probes write spike/4210/out), caches at /cache (~/.cache/aet-4210);
# - CARGO_HOME and CARGO_TARGET_DIR under /cache, so nothing builds into the worktree;
# - TMPDIR unset inside the container (it is never passed);
# - --memory/--memory-swap $MEM (default 8g, the kickoff's ceiling) and --pids-limit -1. Nothing is
#   written into any cgroup file (Furnace hard-denies writes under /sys/fs/cgroup; kickoff rule 3):
#   with --memory-swap equal to --memory, podman is expected to set memory.swap.max to 0 itself.
#   memory.max, memory.swap.max and pids.max are read back into spike/4210/out/caps.log with the
#   command;
# - the command inside does not start until the host has read the caps back AND found
#   memory.swap.max exactly 0 (a flag file under /capflags); otherwise it exits 99 having run nothing.
# - one container of this work order at a time: refuses to start while another aet4210-* runs.
# Usage: img.sh [--name N] [--mem 8g] -- <command...>
set -u
REPO=/home/forge/aethelgard-4210
CACHE=$HOME/.cache/aet-4210
NAME="aet4210-$$-$RANDOM"; MEM=8g
while [[ $# -gt 0 && "$1" == --* ]]; do
  case "$1" in
    --name) NAME="$2"; shift;;
    --mem) MEM="$2"; shift;;
    --) shift; break;;
  esac
  shift
done
case "$MEM" in [1-8]g) ;; *) echo "memory cap $MEM refused: at most 8g" >&2; exit 64;; esac
if podman ps --format '{{.Names}}' | grep -q '^aet4210-'; then
  echo "another aet4210 container is running; one at a time" >&2; exit 75
fi
CAPLOG=$REPO/spike/4210/out/caps.log
FLAGS=$CACHE/capflags
mkdir -p "$(dirname "$CAPLOG")" "$FLAGS"
rm -f "$FLAGS/$NAME"
( for _ in $(seq 1 240); do
    p=$(podman inspect -f '{{.State.CgroupPath}}' "$NAME" 2>/dev/null) && [ -n "$p" ] \
      && [ -e "/sys/fs/cgroup$p/memory.swap.max" ] && {
      d=/sys/fs/cgroup$p
      mm=$(cat "$d/memory.max"); ms=$(cat "$d/memory.swap.max"); pm=$(cat "$d/pids.max")
      if [ "$ms" = 0 ]; then
        echo "$(date -Is) $NAME memory.max=$mm memory.swap.max=$ms pids.max=$pm cmd: $*" >> "$CAPLOG"
        touch "$FLAGS/$NAME"
      else
        echo "$(date -Is) $NAME SWAP NOT ZERO, nothing runs: memory.max=$mm memory.swap.max=$ms pids.max=$pm cmd: $*" >> "$CAPLOG"
      fi
      exit 0; }
    sleep 0.25
  done
  echo "$(date -Is) $NAME CAPS NOT READ cmd: $*" >> "$CAPLOG" ) &
exec podman run --rm --name "$NAME" --network none --memory "$MEM" --memory-swap "$MEM" --pids-limit -1 \
  -v "$REPO":/repo -v "$CACHE":/cache -v "$FLAGS":/capflags:ro -w /repo \
  -e CARGO_HOME=/cache/cargo-home -e CARGO_TARGET_DIR=/cache/target \
  "$(cat "$REPO/container/IMAGE")" \
  bash -c 'for _ in $(seq 1 600); do [ -e "/capflags/$0" ] && exec "$@"; sleep 0.1; done; echo "caps never read back: nothing ran" >&2; exit 99' "$NAME" "$@"
