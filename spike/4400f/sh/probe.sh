#!/bin/bash
# AET-WO-4400f: load a platform in Renode (no firmware, nothing runs) and print monitor reads.
#   probe.sh PLATFORM "cmd1" "cmd2" ...   (inside the tools image)
set -u
plat=$1; shift
h=$(mktemp -d); s=""; for m in ${MODELS:-/cache/cap/foreign/models}/*.cs; do s="$s include @$m;"; done; s="$s mach create; machine LoadPlatformDescription @$plat"
for c in "$@"; do s="$s; echo \"> $c\"; $c"; done
HOME=$h timeout 120 renode --disable-gui --console --plain -e "$s; quit" 2>&1 | grep -v '^\s*$' | grep -E '^> |^0x|^[0-9]|Error|error|rror:' | head -80
rm -rf "$h"
