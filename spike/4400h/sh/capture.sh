#!/bin/bash
# AET-WO-4400h (2)-(4): one `simulate --foreign` by the patched CLI on the held ELF, with each Renode
# process that loads an ELF captured as Tviburi's spike/1150/harness/capture-foreign.sh does it (argv
# bytes, cwd, a copy of Renode's cwd, the CLI's tool scratch under HOME). Inside the pinned tools image.
# Usage: capture.sh NAME WINDOW [SPEC] [ELF]    -> /cache/cap/NAME   (default: sentinel-02-own and its CI ELF)
set -u
O=/cache/cap/$1; w=$2; rm -rf "$O"; mkdir -p "$O"
spec=${3:-crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json}
elf=${4:-/cache/boards/sentinel-02/arducopter.elf}
A=target/debug/aethelgard
(
  declare -A seen; n=0
  while :; do
    for c in /proc/[0-9]*/cmdline; do
      p=${c#/proc/}; p=${p%/cmdline}
      [ -n "${seen[$p]:-}" ] && continue
      grep -qa 'sysbus LoadELF' "$c" 2>/dev/null || continue
      grep -qa -- '--config' "$c" 2>/dev/null || continue   # Renode itself, not the CLI's reaper
      seen[$p]=1; n=$((n + 1)); mkdir -p "$O/$n"
      cp "$c" "$O/$n/argv.bin"; readlink "/proc/$p/cwd" > "$O/$n/cwd.txt"
      cp -r --no-preserve=ownership "$(readlink "/proc/$p/cwd")" "$O/$n/tool" 2> "$O/$n/cp.err"   # the CLI's tool scratch
    done
    sleep 0.02
  done
) &
wp=$!
$A generate --spec "$spec" --out "$O/gen" > "$O/generate.json"
AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 $A simulate --platform "$(ls "$O"/gen/board/*.repl | head -1)" --spec "$spec" \
  --firmware "$elf" --foreign "$w" > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?" > "$O/index.txt"
kill $wp
ls "$O" >> "$O/index.txt"
