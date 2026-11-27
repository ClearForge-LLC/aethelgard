#!/bin/bash
# AET-WO-4412 (1), from 4400j's: one `simulate --foreign` by the CLI (REPO's; main unpatched unless A= names another), with the Renode
# process that loads the ELF captured (argv bytes, cwd, a copy of the CLI's tool scratch: platform, timeline script).
# Usage: capture.sh NAME WINDOW BOARD [INPUT]   -> /cache/cap/NAME/<n>/  (the reaper is 1, Renode 2)
set -u
name=$1; w=$2; board=$3; in=${4:-}
O=/cache/cap/$name; rm -rf "$O"; mkdir -p "$O"
case $board in
  s03) spec=/cache/boards/s03/sentinel-03-foreign.json; elf=/cache/boards/s03/arducopter.elf;;
esac
A=${A:-target/debug/aethelgard}
(
  declare -A seen; n=0
  while :; do
    for c in /proc/[0-9]*/cmdline; do
      p=${c#/proc/}; p=${p%/cmdline}
      [ -n "${seen[$p]:-}" ] && continue
      grep -qa 'sysbus LoadELF' "$c" 2>/dev/null || continue
      seen[$p]=1; n=$((n + 1)); mkdir -p "$O/$n"
      cp "$c" "$O/$n/argv.bin"; readlink "/proc/$p/cwd" > "$O/$n/cwd.txt"
      cp -r --no-preserve=ownership "$(readlink "/proc/$p/cwd")" "$O/$n/tool" 2> "$O/$n/cp.err"
    done
    sleep 0.02
  done
) &
wp=$!
G=/cache/cli/gen-sentinel-03-foreign
extra=(); [ -n "$in" ] && extra=(--input "$in")
$A simulate --platform "$(ls "$G"/board/*.repl | head -1)" --spec "$spec" --firmware "$elf" --foreign "$w" "${extra[@]}" \
  > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?" > "$O/index.txt"
kill $wp
ls "$O" >> "$O/index.txt"
