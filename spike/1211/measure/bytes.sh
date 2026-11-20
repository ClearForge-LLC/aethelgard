#!/bin/bash
# AET-WO-1211 §1.4, bytes: does a change to a specification's bytes that no field reads move anything but
# the hash lines? For each specification with PWM outputs, generate and build from the committed file and
# from a copy with one newline appended (same JSON, a different sha256). One container, as spike/1211/twin/run.sh
# (capped-run 8G, swap 0, --network none, nice -n 10); the CLI is built in the image from this worktree.
# Output: ~/.cache/aet-1211/bytes/{<spec>,<spec>.nl}/{gen,emit,fw.elf,*.json,*.err}, bytes.log.
set -u
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
CACHE=$HOME/.cache/aet-1211
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
IMAGE=$(cat "$REPO/container/IMAGE")
exec 9>"$CACHE/container.lock"; flock 9
echo "$(date -u -Is) bytes image=$IMAGE free_g=$(free -g | awk '/^Mem:/ {print $7}')" >> "$CACHE/runs/caps.log"
exec nice -n 10 ~/bin/capped-run --mem 8G --tasks 8192 -- --rm --network none -v "$REPO":/repo:ro \
  -v "$CACHE":/cache -w /repo -e CARGO_HOME=/cache/cargo-home -e CARGO_NET_OFFLINE=true \
  -e CARGO_TARGET_DIR=/cache/target-img "$IMAGE" bash -c '
unset TMPDIR; export HOME=/cache/bytes/home; mkdir -p $HOME
set -x
cargo build --locked --offline -p aethelgard-cli 2>&1 | tail -3
A=/cache/target-img/debug/aethelgard; sha256sum $A
for s in motors-pwm driver-judges-nothing; do
  for v in "" .nl; do
    d=/cache/bytes/$s$v; rm -rf $d; mkdir -p $d
    cp /repo/crates/aethelgard-cli/tests/fixtures/$s.json $d/spec.json
    [ "$v" = .nl ] && printf "\n" >> $d/spec.json
    sha256sum $d/spec.json
    $A generate --spec $d/spec.json --out $d/gen > $d/generate.json 2> $d/generate.err; echo "generate $s$v exit $?"
    $A build --crate $d/gen/board --out $d/fw.elf > $d/build.json 2> $d/build.err; echo "build $s$v exit $?"
    $A solve --file $d/spec.json > $d/solve.json 2> $d/solve.err; echo "solve $s$v exit $?"
    $A emit --file $d/spec.json --out $d/emit > $d/emit.json 2> $d/emit.err; echo "emit $s$v exit $?"
  done
done' > "$CACHE/bytes/bytes.log" 2>&1
