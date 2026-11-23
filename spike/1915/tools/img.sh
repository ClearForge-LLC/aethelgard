#!/bin/bash
# AET-WO-1915: one measured container. The pinned tools image under
# capped-run (swap 0, read back), nice -n 10, --network none unless NET is set,
# one container at a time (flock), sampled from the host at 1 s by sample.py.
# usage: TAG=<run name> [MEM=16G] [NET=none] img.sh <tree> <cmd...>
# Writes ~/.cache/aet-1915/runs/<TAG>/: cap.log (capped-run's stderr and the tool's),
# stdout, samples.jsonl, meta.
set -u
TREE=$1; shift
MEM=${MEM:-16G}
K=$HOME/.cache/aet-1915; R=$K/runs/$TAG
[ -e "$R" ] && { echo "img.sh: $R exists" >&2; exit 2; }
mkdir -p "$R"
export XDG_RUNTIME_DIR=/run/user/$(id -u)
exec 9>"$K/container.lock"; flock 9
# Start only with room for the whole cap above Furnace's 12 GiB floor.
avail=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
need=$(( (12 + ${MEM%G}) * 1024 * 1024 ))
if [ "$avail" -lt "$need" ]; then
  echo "img.sh: MemAvailable ${avail} kB < ${need} kB (12 GiB + cap): not starting" | tee "$R/meta" >&2
  exit 3
fi
IMAGE=$(cat "$TREE/container/IMAGE")
{ echo "tag=$TAG mem=$MEM net=${NET:-none} tree=$TREE"; echo "image=$IMAGE"; echo "start=$(date -u -Is)"
  echo "host_avail_kB=$avail"; echo "cmd: $*"; } > "$R/meta"
nice -n 10 "$HOME/bin/capped-run" --mem "$MEM" --tasks 8192 -- --rm --name "aet1915-$TAG" --network "${NET:-none}" \
  -v "$TREE":/repo -v "$K/cargo-home":/cargo-home -w /repo \
  -e CARGO_HOME=/cargo-home -e CARGO_TARGET_DIR=/repo/target \
  "$IMAGE" "$@" > "$R/stdout" 2> "$R/cap.log" &
cr=$!
python3 -I "$(dirname "$(readlink -f "$0")")/sample.py" "$R/cap.log" "$R/samples.jsonl" &
sp=$!
wait $cr; rc=$?
wait $sp
# A SIGKILLed podman client leaves its record behind (measured): remove ours by name.
podman rm -f --ignore "aet1915-$TAG" >/dev/null 2>&1
{ echo "exit=$rc"; echo "end=$(date -u -Is)"; } >> "$R/meta"
exit $rc
