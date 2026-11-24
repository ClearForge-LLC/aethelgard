#!/bin/bash
# AET-WO-4400h (2)-(4): one `simulate --foreign` by the patched CLI on the held ELF, with each Renode
# process that loads an ELF captured as Tviburi's spike/1150/harness/capture-foreign.sh does it (argv
# bytes, cwd, a copy of the CLI's tool scratch /var/lib/aethelgard). Inside the pinned tools image.
# Usage: capture.sh NAME WINDOW    -> /cache/cap/NAME
set -u
O=/cache/cap/$1; w=$2; rm -rf "$O"; mkdir -p "$O"
spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
A=target/debug/aethelgard
(
  declare -A seen; n=0
  while :; do
    for c in /proc/[0-9]*/cmdline; do
      p=${c#/proc/}; p=${p%/cmdline}
      [ -n "${seen[$p]:-}" ] && continue
      grep -qa 'sysbus LoadELF' "$c" 2>/dev/null || continue
      grep -qa -- '--console' "$c" 2>/dev/null || continue
      seen[$p]=1; n=$((n + 1)); mkdir -p "$O/$n"
      cp "$c" "$O/$n/argv.bin"; readlink "/proc/$p/cwd" > "$O/$n/cwd.txt"
      cp -r --no-preserve=ownership /var/lib/aethelgard "$O/$n/varlib" 2> "$O/$n/cp.err"
    done
    sleep 0.02
  done
) &
wp=$!
$A generate --spec $spec --out "$O/gen" > "$O/generate.json"
AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 $A simulate --platform "$O/gen/board/sentinel-02-own.repl" --spec $spec \
  --firmware /cache/boards/sentinel-02/arducopter.elf --foreign "$w" > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?" > "$O/index.txt"
kill $wp
ls "$O" >> "$O/index.txt"
