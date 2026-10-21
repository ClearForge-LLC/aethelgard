#!/usr/bin/env bash
# AET-WO-0300a spike, S3-S5: a MEASUREMENT, not a gate. Runs the stand-in's
# attack set bare, under the real `aethelgard` Landlock trampoline, and under
# the trampoline plus the seccomp filter, then the x32 and i386 calls, then
# the filter's install time. Every exit code is captured on purpose; each
# result is an `ANSWER` line. The build half deletes this file.
#
#   bash spike/s4.sh AETHELGARD_BIN PROBES_DIR
set -u

CLI=$1
PROBES=$2
W=$(mktemp -d)
BUS=/run/user/$(id -u)/bus
TMUX=/tmp/tmux-$(id -u)/default
NAME=aet-0300a-$$
PORT=$((40000 + $$ % 20000))
SYS=/usr:/lib:/lib64:/bin:/sbin:/etc

answer() { printf 'ANSWER %s=%s\n' "$1" "$2"; }
answer env "kernel=$(uname -r) uid=$(id -u) bus_exists=$([ -S "$BUS" ] && echo yes || echo no) tmux_exists=$([ -S "$TMUX" ] && echo yes || echo no)"

"$PROBES/standin" listen "$NAME" "$PORT" 60 &
LISTENER=$!
sleep 0.5

# trampoline CMD...: the real Landlock trampoline, as the CLI calls it (one
# path per --ro).
trampoline() {
  local args=()
  local IFS=:
  for p in $SYS $PROBES; do args+=(--ro "$p"); done
  unset IFS
  "$CLI" --aethelgard-internal-confine --status "$W/status" "${args[@]}" --rw "$W" -- "$@"
}

attack=("$PROBES/standin" attack "$BUS" "$TMUX" "$NAME" "$PORT")
PROBE_MODE=bare "${attack[@]}"
PROBE_MODE=landlock trampoline "${attack[@]}"
answer landlock.status "$(head -1 "$W/status")"
PROBE_MODE=landlock+seccomp trampoline "$PROBES/seccomp-wrap" -- "${attack[@]}" 2> "$W/wrap.err"
cat "$W/wrap.err" >&2
answer landlock+seccomp.status "$(head -1 "$W/status")"
answer landlock+seccomp.install "$(grep -o 'installed.*' "$W/wrap.err" | head -1)"

for abi in x32 i386; do
  PROBE_MODE=bare "$PROBES/standin" "$abi"
  answer "bare.$abi.exit" "$?"
  PROBE_MODE=landlock+seccomp trampoline "$PROBES/seccomp-wrap" -- "$PROBES/standin" "$abi" 2> /dev/null
  rc=$?
  answer "landlock+seccomp.$abi.exit" "$rc$([ "$rc" -eq 159 ] && echo ' (128+31: SIGSYS, killed)')"
done

for i in 1 2 3 4 5; do
  "$PROBES/seccomp-wrap" -- /bin/true 2> "$W/t.err"
  answer "install_time.run$i" "$(grep -o 'in [0-9]* us' "$W/t.err")"
done

kill "$LISTENER" 2> /dev/null
wait "$LISTENER" 2> /dev/null
rm -rf "$W"
