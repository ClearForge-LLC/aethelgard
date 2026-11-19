#!/bin/bash
# AET-WO-4201: run a command in the pinned tools image, one container at a time,
# capped (capped-run: memory 8G, swap 0, read back), with a scratch tree at /repo.
# usage: TREE=<scratch tree> NET=none|default img.sh <cmd...>
set -u
export XDG_RUNTIME_DIR=/run/user/$(id -u)
exec 9>"$HOME/.cache/aet-4201/container.lock"; flock 9
echo "$(date -Is) tree=$TREE net=${NET:-none} cmd: $*" >> "$HOME/.cache/aet-4201/caps.log"
exec nice -n 19 "$HOME/bin/capped-run" --mem 8G --tasks 8192 -- --rm --network "${NET:-none}" \
  -v "$TREE":/repo -v "$HOME/.cache/aet-4201/cargo-home":/cargo-home -w /repo \
  -e CARGO_HOME=/cargo-home -e CARGO_TARGET_DIR=/repo/target \
  "$(cat $HOME/aethelgard-4201/container/IMAGE)" "$@"
