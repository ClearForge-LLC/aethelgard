#!/bin/bash
# AET-WO-2301 P5: run Renode under the CLI's own sandbox as of e3e3cd2 (run.rs Invocation::start,
# main.rs simulate): reaper -> trampoline, --ro SYSTEM_DIRS + tool dirs + /proc + /dev/urandom,
# --rw workdir, env cleared to HOME/TMPDIR=workdir + DOTNET_EnableDiagnostics=0 +
# DOTNET_EnableWriteXorExecute=0, stdin null, bounds 30 s / 1 GiB for a boot. The workdir is handed over
# as hand_over() does for root: files sealed 0444 / dirs 0555 (root), the workdir root:65533 mode 1770.
# This shell is the "CLI" (the reaper checks its parent pid).
# Usage: sbx.sh <label> <virtual-ms> <quiet:0|1> [<firmware.elf>]
#   quiet=1 adds `logLevel 3 console` beside the logFile; quiet=0 adds only the logFile; quiet=2 adds neither.
set -u
LABEL=$1; MS=$2; QUIET=$3; FW=${4:-/cache/target/thumbv7em-none-eabihf/release/p5-flood}
AETH=${AETH:-/cache/target/release/aethelgard}
R=/cache/p5/$LABEL; rm -rf "$R"; mkdir -p "$R"; chmod 755 /cache/p5 "$R"
WD=$R/tool; mkdir "$WD"; chmod 700 "$WD"
cp /repo/boards/sentinel-01/sentinel-01.repl "$WD/platform-booted.repl"
cp "$FW" "$WD/firmware.elf"
{
  [ "$QUIET" != 2 ] && echo "logFile @$WD/renode.log"
  [ "$QUIET" = 1 ] && echo "logLevel 3 console"
} > "$R/pre.cmds"
PRE=$(tr '\n' ';' < "$R/pre.cmds" | sed 's/;/; /g')
NS=$(( MS * 1000000 ))
CMDS="emulation SetSeed 2100; mach create; machine LoadPlatformDescription @$WD/platform-booted.repl; sysbus LoadELF @$WD/firmware.elf; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; sysbus WriteDoubleWord 0x20000000 0x0; sysbus WriteDoubleWord 0x20000004 0xffffffff; sysbus WriteDoubleWord 0x20000008 0xffffffff; cpu VectorTableOffset 0x08000000; ${PRE}emulation RunFor \"$((NS/1000000000)).$(printf %09d $((NS%1000000000)))\"; logLevel 3; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0x58020000; sysbus ReadDoubleWord 0x58020020; sysbus ReadDoubleWord 0x58020400; sysbus ReadDoubleWord 0x58020420; sysbus ReadDoubleWord 0x580244e0; sysbus ReadDoubleWord 0x580244e8; sysbus ReadDoubleWord 0x580244f0; sysbus ReadDoubleWord 0x20000004; sysbus ReadDoubleWord 0x20000008; sysbus ReadDoubleWord 0x40005404; sysbus ReadDoubleWord 0x40005428; echo \"AETHELGARD-BOOT-END\"; peripherals; quit"
# hand over (hand_over + seal)
find "$WD" -type f -exec chmod 444 {} +
chgrp 65533 "$WD"; chmod 1770 "$WD"
: > "$R/status"
ro=(); for d in /usr /lib /lib64 /bin /sbin /etc /usr/local/bin /opt/renode /opt/renode /proc /dev/urandom; do ro+=(--ro "$d"); done
echo "$CMDS" > "$R/cmds.txt"
mkfifo "$R/fo" "$R/fe"; cat < "$R/fo" > "$R/stdout" & C1=$!; cat < "$R/fe" > "$R/stderr" & C2=$!
cd "$WD"
T0=$(date +%s%N)
if [ "${STREAM:-0}" != 0 ]; then
  echo "$CMDS" > "$R/cmds.txt"; kill $C1 $C2 2>/dev/null
  python3 -I /repo/spike/2301/p5/stream.py "$R" "$WD" "$R/cmds.txt" ${STREAM_ARGS:-} | tee "$R/stream.json"
  echo "tool files: $(ls -la "$WD" | tail -n +2 | tr '\n' '|')"; exit 0
fi
env -i HOME="$WD" TMPDIR="$WD" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0 \
  "$AETH" --aethelgard-internal-reaper --cli-pid $$ --status "$R/status" --wall-ms 30000 --rss-kib 1048576 \
  "${ro[@]}" --rw "$WD" -- /usr/local/bin/renode --disable-gui --console --plain -e "$CMDS" \
  < /dev/null > "$R/fo" 2> "$R/fe" &
PID=$!
# peak RSS of the tree, polled
PEAK=0
while kill -0 $PID 2>/dev/null; do
  S=0; for p in $(pgrep -f "[r]enode" 2>/dev/null); do v=$(awk '/VmRSS/{print $2}' /proc/$p/status 2>/dev/null); S=$((S+${v:-0})); done
  [ $S -gt $PEAK ] && PEAK=$S
  sleep 0.1
done
wait $PID; RC=$?; wait $C1 $C2
T1=$(date +%s%N)
{
 echo "reaper_exit=$RC wall_ms=$(( (T1-T0)/1000000 )) peak_renode_rss_kib=$PEAK"
 echo "status: $(tr '\n' '|' < "$R/status")"
 echo "stdout_bytes=$(stat -c %s "$R/stdout") stderr_bytes=$(stat -c %s "$R/stderr") logfile_bytes=$(stat -c %s "$WD/renode.log" 2>/dev/null || echo none)"
 [ -f "$WD/renode.log" ] && echo "logfile_lines=$(wc -l < "$WD/renode.log")"
 echo "tool files: $(ls -la "$WD" | tail -n +2 | tr '\n' '|')"
} | tee "$R/summary"
