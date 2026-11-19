#!/bin/bash
# Inside the image: candidate (b), per-quantum lock-step, on Sentinel-03 (generated and built by the
# unmodified CLI, gen.sh), weak pack (1.0 ohm), each case 1 s simulated. TOOL holds the run's copies
# of the CLI's scratch files: the emitted .repl in place of platform-booted.repl (as 2303), the
# built ELF, the two C# models. Usage: runb.sh NAME I_RUN_A Q_NS [SPAN_NS]
set -u
L=/cache/target/release/lockstep
T=/cache/s03/tool
if [ ! -f $T/firmware.elf ]; then
  mkdir -p $T/models $T/capture
  cp /cache/s03/gen/board/sentinel-03.repl $T/platform-booted.repl
  cp /cache/s03/fw.elf $T/firmware.elf
  cp /repo/models/renode/BMP581.cs /repo/models/renode/ICM42688P.cs $T/models/
fi
ARGV=$(ls /cache/s03/renode-argv.* | head -1)
N=$1; I=$2; Q=$3; SPAN=${4:-1000000000}; CASE=${5:-weak}
O=/cache/runs/b-$N; rm -rf $O; mkdir -p $O
s=$(date +%s%N)
TOOL=$T I_RUN_A=$I Q_NS=$Q $L lockstep $CASE $SPAN $ARGV $O > $O/run.json 2> $O/run.err
echo "$N exit=$? wall_ms=$(( ($(date +%s%N)-s)/1000000 ))"; tail -c 600 $O/run.err | tail -3
$L replay $O > $O/replay.jsonl 2>&1; echo "  replay exit=$?"
