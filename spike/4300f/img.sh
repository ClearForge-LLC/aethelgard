#!/bin/bash
# AET-WO-4300f spike runner under ~/.cache/aet-bus/RULE3.md (the architect's rule 3, 2026-11-15
# 03:40 UTC; the ruling on PR #134 withdrew the earlier swap stop):
# - the container runs INSIDE a capped systemd user scope it shares:
#   systemd-run --user --scope -p MemoryMax=$MEM -p MemorySwapMax=0 -p TasksMax=8192, and
#   podman run --rm --cgroups=disabled --network none, with no --memory, --memory-swap or
#   --pids-limit flags;
# - inside the scope, before the container, memory.max, memory.swap.max and pids.max are read at
#   /sys/fs/cgroup plus the path in /proc/self/cgroup into spike/4300f/out/caps.log, and nothing
#   runs unless memory.swap.max is 0 (exit 99). Nothing writes any cgroup file;
# - once the container is up, its PID's /proc/<pid>/cgroup is logged beside the scope's;
# - repo at /repo, cache at /cache (~/.cache/aet-4300f); TMPDIR is never passed;
# - one container of this work order at a time.
# Usage: img.sh [--mem 8G] -- <command...>
set -u
REPO=/home/forge/aethelgard-4300f
CACHE=$HOME/.cache/aet-4300f
NAME="aet4300f-$$-$RANDOM"; MEM=8G
while [[ $# -gt 0 && "$1" == --* ]]; do
  case "$1" in
    --mem) MEM="$2"; shift;;
    --) shift; break;;
  esac
  shift
done
case "$MEM" in [1-8]G) ;; *) echo "memory cap $MEM refused: at most 8G" >&2; exit 64;; esac
if podman ps --format '{{.Names}}' | grep -q '^aet4300f-'; then
  echo "another aet4300f container is running; one at a time" >&2; exit 75
fi
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
CAPLOG=$REPO/spike/4300f/out/caps.log
mkdir -p "$(dirname "$CAPLOG")"
IMG=$(cat "$REPO/container/IMAGE")
exec systemd-run --user --scope -q -p MemoryMax="$MEM" -p MemorySwapMax=0 -p TasksMax=8192 -- \
  bash -c '
    NAME=$1 CAPLOG=$2 REPO=$3 CACHE=$4 IMG=$5; shift 5
    now() { date -u +%FT%T.%N | cut -c1-23; }
    cg=$(sed -n "s/^0:://p" /proc/self/cgroup); d=/sys/fs/cgroup$cg
    v="memory.max=$(cat "$d/memory.max") memory.swap.max=$(cat "$d/memory.swap.max") pids.max=$(cat "$d/pids.max")"
    echo "$(now)Z $NAME rule3 scope $cg $v cmd: $*" >> "$CAPLOG"
    case "$v" in *"memory.swap.max=0 "*) ;; *)
      echo "$(now)Z $NAME STOPPED: memory.swap.max is not 0, nothing runs" >> "$CAPLOG"; exit 99;; esac
    podman run --rm --name "$NAME" --cgroups=disabled --network none \
      -v "$REPO":/repo -v "$CACHE":/cache -w /repo "$IMG" "$@" &
    client=$!
    ( for _ in $(seq 1 120); do
        p=$(podman inspect -f "{{.State.Pid}}" "$NAME" 2>/dev/null)
        if [ -n "$p" ] && [ "$p" != 0 ] && [ -r "/proc/$p/cgroup" ]; then
          c=$(sed -n "s/^0:://p" "/proc/$p/cgroup")
          echo "$(now)Z $NAME container pid $p cgroup $c (scope $cg; same: $([ "$c" = "$cg" ] && echo yes || echo NO))" >> "$CAPLOG"
          exit 0
        fi
        sleep 0.25
      done
      echo "$(now)Z $NAME container pid never read (it may have exited first)" >> "$CAPLOG" ) &
    wait $client; rc=$?
    echo "$(now)Z $NAME exit $rc" >> "$CAPLOG"
    exit $rc
  ' rule3 "$NAME" "$CAPLOG" "$REPO" "$CACHE" "$IMG" "$@"
