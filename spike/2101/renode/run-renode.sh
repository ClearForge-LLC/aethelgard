#!/bin/bash
# Usage: run-renode.sh <cmdfile (path under spike/2101/renode)> -> raw renode output on stdout.
# Monitor commands are piped on stdin, one per line. Capped: timeout 120, 8G memory cgroup.
set -u
REPO=${REPO:-/home/forge/aethelgard-2101}
IMG=$(cat "$REPO/container/IMAGE")
export XDG_RUNTIME_DIR=/run/user/$(id -u)
timeout 120 systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 -- \
  podman run --rm -i --pids-limit=-1 -e DOTNET_EnableDiagnostics=0 \
  -v "$REPO":/repo:Z -w /opt/renode "$IMG" renode --disable-gui --console --plain < "$1" 2>&1 | tr -d '\r'
