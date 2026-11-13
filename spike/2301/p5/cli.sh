#!/bin/bash
# usage: cli.sh <for> <outfile-base> ; runs today's CLI on the flood firmware inside the image
FOR=$1; B=$2
{ echo "# cmd: spike/2301/img.sh -- /cache/target/release/aethelgard simulate --platform boards/sentinel-01/sentinel-01.repl --firmware /cache/target/thumbv7em-none-eabihf/release/p5-flood --spec examples/sentinel-01-minimal.json --for $FOR"; } > $B.txt
S=$(date +%s%N)
/cache/target/release/aethelgard simulate --platform boards/sentinel-01/sentinel-01.repl --firmware /cache/target/thumbv7em-none-eabihf/release/p5-flood --spec examples/sentinel-01-minimal.json --for $FOR > /cache/p5/cli-$FOR.out 2> /cache/p5/cli-$FOR.err
rc=$?
E=$(date +%s%N)
{ echo "exit=$rc"; echo "wall_ms=$(( (E-S)/1000000 ))"; echo "stdout_bytes=$(stat -c %s /cache/p5/cli-$FOR.out) stderr_bytes=$(stat -c %s /cache/p5/cli-$FOR.err)"; head -c 1500 /cache/p5/cli-$FOR.out; echo; head -c 1500 /cache/p5/cli-$FOR.err; } >> $B.txt
