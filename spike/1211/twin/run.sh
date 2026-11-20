#!/bin/bash
# AET-WO-1211 §1.5: run the three hand-written Renode scripts on the stock platform, in the pinned tools
# image (container/IMAGE), one container: capped-run (8G, swap 0, TasksMax 8192), --network none, nice -n 10,
# TMPDIR unset inside, the worktree read-only at /repo. Each script is its own Renode process, so an error in
# one does not stop the others. Output: ~/.cache/aet-1211/runs/twin.log.
set -u
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
CACHE=$HOME/.cache/aet-1211
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"; flock 9
echo "$(date -u -Is) image=$IMAGE free_g=$(free -g | awk '/^Mem:/ {print $7}')" >> "$CACHE/runs/caps.log"
exec nice -n 10 ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none -v "$REPO":/repo:ro \
  -v "$CACHE/runs":/out -w /out -e SCRIPTS="${SCRIPTS:-gpio timer adc adc23 order high}" "$IMAGE" bash -c '
unset TMPDIR; export HOME=/out/home; mkdir -p $HOME
renode --version
for s in $SCRIPTS; do
  echo "=== $s ==="
  timeout 300 renode --disable-gui --console --plain -e "include @/repo/spike/1211/twin/$s.resc; quit" </dev/null 2>&1
  echo "=== $s exit $? ==="
done' > "$CACHE/runs/twin.log" 2>>"$CACHE/runs/caps.log"
