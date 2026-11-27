#!/bin/bash
# AET-WO-4412 (1), from 4400j's: replay a captured CLI Renode invocation (sh/capture.sh) in the pinned tools image, in the environment
# the CLI gives Renode (4400i's replay.sh), stepped in whole virtual seconds with a probe at each (py/stepify.py).
# These runs carry the per-second RSS and GC figures and the dumps; the margins come from the unperturbed CLI runs.
#   FULLGC=1        each probe also forces a full collection (GetTotalMemory(true))
#   PAUSE="6 9"     at those seconds: compacting GC, then a heap dump by dotnet-dump (needs ROOT=1: createdump is copied
#                   beside Renode's libcoreclr.so, after checking its libcoreclr.so equals the downloaded 8.0.12 runtime's)
#   LOGP="timer1 dmamux1"  those peripherals' accesses logged (reads dropped from the stored console)
#   GPIOB=1         GPIOB's accesses logged (reads dropped from the stored console): PB13's MODER/AFRH timeline (F4)
#   SILENCE=1       `logLevel 3` on everything the run logs at a lower level (the flood's share of RSS)
#   STEP_MS=10      the CLI's own end reads every STEP_MS from FROM_MS (default 4000): the end read at every step (F4)
#   stdin: as run.rs gives Renode now (HELD_STDIN): a pipe carrying HELD_STDIN_LINE "quit\n", held open. HOLDIN=0: /dev/null
#   GCCONF="K=V .." extra DOTNET_ settings for Renode (e.g. DOTNET_gcServer=0), a configuration lever
# Usage: replay.sh CAPDIR OUTDIR
set -u
cap=$1; O=$2; rm -rf "$O"; mkdir -p "$O"
mapfile -d '' args < "$cap/argv.bin"; args=("${args[@]:1}")
cfg=""; for i in "${!args[@]}"; do [ "${args[$i]}" = "--config" ] && cfg=${args[$((i + 1))]}; done
d=$(dirname "$cfg")
rm -rf "$d"; mkdir -p "$(dirname "$d")"; cp -r "$cap/tool" "$d"
for i in "${!args[@]}"; do
  [ "${args[$i]}" = "-e" ] || continue
  c=${args[$((i + 1))]}
  printf '%s' "$c" > "$O/e-orig.txt"
  if [ -n "${STEP_MS:-}" ]; then   # the CLI's own end reads, between its markers, for READS (py/stepify.py)
    python3 -I -c 'import sys; c = open(sys.argv[1]).read(); a = c.index("echo \"AETHELGARD-BOOT-BEGIN\"; ") + len("echo \"AETHELGARD-BOOT-BEGIN\"; "); b = c.index("; echo \"AETHELGARD-BOOT-END\""); open(sys.argv[2], "w").write(c[a:b])' "$O/e-orig.txt" "$O/reads.txt" || exit 3
    export READS=$O/reads.txt
    # READS_ONLY="0xe000ed04 0x58020400": only those reads (a per-quantum timeline of a few registers)
    if [ -n "${READS_ONLY:-}" ]; then
      python3 -I -c 'import sys; keep = sys.argv[2].split(); c = open(sys.argv[1]).read().split("; "); open(sys.argv[1], "w").write("; ".join(x for x in c if x.split()[-1] in keep))' "$O/reads.txt" "$READS_ONLY"
    fi
  fi
  tl=$(grep -o 'include @[^;]*input\.resc' <<< "$c" | head -1 | sed 's/^include @//')
  if [ -n "$tl" ] && [ -f "$tl" ]; then
    python3 -I /spike/py/stepify.py file "$tl" "$O" || exit 3
    cp "$tl" "$O/timeline-stepped.resc"
  else
    printf '%s' "$c" > "$O/e.txt"; c=$(python3 -I /spike/py/stepify.py run "$O/e.txt" "$O") || exit 3
  fi
  pre=""
  [ "${GPIOB:-0}" = 1 ] && pre="${pre}sysbus LogPeripheralAccess sysbus.gpioPortB true; "
  [ "${SILENCE:-0}" = 1 ] && pre="${pre}logLevel 3; "
  for p in ${LOGP:-}; do pre="${pre}sysbus LogPeripheralAccess sysbus.$p true; "; done
  if [ -n "$pre" ]; then
    first=$(grep -o 'python "print(.AET-STAT t=0 [^"]*"' <<< "$c" | head -1)
    [ -n "$first" ] || first=$(grep -o 'include @[^;]*\(input\|steps\)\.resc' <<< "$c" | head -1)
    c=${c/"$first"/"$pre$first"}
  fi
  args[$((i + 1))]=$c
done
printf '%s\n' "${args[@]}" > "$O/argv.txt"
envs=(HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0)
if [ -n "${PAUSE:-}" ]; then
  R=/cache/dl/rt/shared/Microsoft.NETCore.App/8.0.12
  a=$(sha256sum /opt/renode/libcoreclr.so | cut -c1-64); b=$(sha256sum $R/libcoreclr.so | cut -c1-64)
  echo "libcoreclr image=$a runtime=$b" | tee "$O/dac-match.txt"
  [ "$a" = "$b" ] || { echo "replay: the downloaded runtime is not Renode's" >&2; exit 4; }
  cp $R/createdump $R/libmscordaccore.so $R/libmscordbi.so /opt/renode/ || exit 4   # createdump loads the DAC beside libcoreclr.so
  envs=(HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=1 DOTNET_EnableWriteXorExecute=0)
fi
for kv in ${GCCONF:-}; do envs+=("$kv"); done
echo "${envs[*]}" > "$O/env.txt"; echo "nproc=$(nproc) affinity=$(taskset -pc $$ | sed 's/.*: //')" >> "$O/env.txt"
cd "$d"
start=$(date +%s.%N)
filter='\] Read\(UInt\|Byte\|Word\|DoubleWord\|QuadWord\)'
# HOLDIN=1: stdin a pipe held open and never written (Renode's console reader blocks), not /dev/null (EOF at once)
if [ "${HOLDIN:-1}" = 1 ]; then exec 7< <(printf 'quit\n'; exec sleep 100000); else exec 7< /dev/null; fi
echo "stdin=$(readlink /proc/$$/fd/7)" >> "$O/env.txt"
( echo $BASHPID > "$O/renode.pid"; exec env -i "${envs[@]}" /opt/renode/renode "${args[@]}" <&7 7<&- 2>&1 ) \
  | { if [ "${GPIOB:-0}" = 1 ] || [ -n "${LOGP:-}" ]; then grep -av "$filter"; else cat; fi; } | tee >(grep -a '^AET-STAT' > "$O/stats.txt") | gzip -1 > "$O/console.txt.gz" &
gz=$!
sleep 0.3; pid=$(cat "$O/renode.pid"); peak=0
: > "$O/rss.txt"
while kill -0 "$pid" 2>/dev/null; do
  r=$(awk '/^VmRSS:/{print $2}' /proc/$pid/status 2>/dev/null); r=${r:-0}
  echo "$(date +%s.%N) $r" >> "$O/rss.txt"; [ "$r" -gt "$peak" ] && peak=$r
  for p in "$O"/pause-*; do
    [ -e "$p" ] || continue; t=${p##*-}; [ -e "$O/go-$t" ] && continue
    echo "$(date -u -Is) dump at t=$t pid=$pid rss=$r" >> "$O/dumps.txt"
    TMPDIR="$d" DOTNET_ROOT=/cache/dl/rt /cache/dl/rt/dotnet /cache/dl/dotnet-dump/tools/net8.0/any/dotnet-dump.dll \
      collect -p "$pid" --type Heap -o "/cache/dumps/$(basename "$O")-t$t.dmp" >> "$O/dumps.txt" 2>&1
    echo "exit $?" >> "$O/dumps.txt"; touch "$O/go-$t"
  done
  sleep 0.1
done
wait "$gz"   # the pipeline's end, not a bare wait: HOLDIN's sleep never ends
echo "wall_s=$(awk -v a=$start -v b=$(date +%s.%N) 'BEGIN{printf "%.2f", b-a}') peak_vmrss_kib=$peak load=$(cut -d' ' -f1-3 /proc/loadavg)" | tee "$O/cost.txt"
