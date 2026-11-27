#!/bin/bash
# AET-WO-4412 (1), from 4400j: SOS over a heap dump, inside the tools image (16 GiB cap), by the downloaded dotnet-dump 9.0.661903
# on the downloaded .NET 8.0.12 runtime, whose libcoreclr.so equals Renode's (replay.sh checks it; setclrpath gives SOS
# that runtime's DAC). Usage: analyze.sh DUMP OUT 'sos command' ['sos command' ...]
set -u
dump=$1; out=$2; shift 2
R=/cache/dl/rt/shared/Microsoft.NETCore.App/8.0.12
cmds=(-c "setclrpath $R")
for c in "$@"; do cmds+=(-c "$c"); done
cmds+=(-c exit)
DOTNET_ROOT=/cache/dl/rt /cache/dl/rt/dotnet /cache/dl/dotnet-dump/tools/net8.0/any/dotnet-dump.dll analyze "$dump" "${cmds[@]}" > "$out" 2>&1
echo "analyze exit $? -> $out ($(wc -l < "$out") lines)"
