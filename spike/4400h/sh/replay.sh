#!/bin/bash
# AET-WO-4400h (2)-(4): replay the captured CLI Renode invocation (capture.sh) in the pinned tools image,
# in the environment the CLI gives Renode (as Tviburi's spike/1150/harness/replay-ap.sh), with:
#   REQ=1     the twin's missing line: `ReceiveDmaRequest -> dmamux1@41` in usart1's block of the
#             booted platform (STM32F7_USART.cs:87, :109; RM0433 Table 121 request 41 usart1_rx_dma)
#   INJ=DIR   `emulation RunFor` replaced by DIR/inject.resc (frames.py)
#   LOG=1     `sysbus LogAllPeripheralsAccess true` before the run, collapse off, reads dropped from the
#             stored console (gzipped), and Tviburi 1100's regmap.py after it, as Tviburi (v) (replay-ap.sh `logged`)
# Always: USART1's TX into OUT/tx.bin (UartFileBackend), and the storage sectors' first words read after.
# Usage: replay.sh CAPDIR OUTDIR     cost line in OUTDIR/cost.txt (tree VmRSS peak at 100 ms, as replay-ap.sh)
set -u
cap=$1; O=$2; rm -rf "$O"; mkdir -p "$O"
rm -rf /var/lib/aethelgard; cp -r "$cap/varlib" /var/lib/aethelgard
mapfile -d '' args < "$cap/argv.bin"; args=("${args[@]:1}")
cfg=""; for i in "${!args[@]}"; do [ "${args[$i]}" = "--config" ] && cfg=${args[$((i + 1))]}; done
d=$(dirname "$cfg"); plat=$d/platform-booted.repl
if [ "${REQ:-0}" = 1 ]; then
  python3 -I - "$plat" <<'PY'
import sys, re
p = sys.argv[1]; s = open(p).read()
m = re.search(r"^usart1: [^\n]*\n((?:    [^\n]*\n)*)", s, re.M)
assert m, "usart1 block not found"
blk = m.group(0)
assert "ReceiveDmaRequest" not in blk
new = blk.rstrip("\n") + "\n    ReceiveDmaRequest -> dmamux1@41\n"
s = s.replace(blk, new, 1); open(p, "w").write(s)
PY
fi
cp "$plat" "$O/platform-booted.repl"
for i in "${!args[@]}"; do
  [ "${args[$i]}" = "-e" ] || continue
  c=${args[$((i + 1))]}
  pre="sysbus.usart1 CreateFileBackend @$O/tx.bin true; "
  [ "${LOG:-0}" = 1 ] && { sed -i 's/^collapse-repeated-log-entries = True/collapse-repeated-log-entries = False/' "$cfg"; pre="${pre}sysbus LogAllPeripheralsAccess true; "; }
  run=$(grep -o 'emulation RunFor "[0-9.]*"' <<< "$c" | head -1)
  if [ -n "${INJ:-}" ]; then body="include @$INJ/inject.resc"; else body="$run"; fi
  post="; echo \"AET4400H-STORAGE\"; sysbus ReadDoubleWord 0x081C0000; sysbus ReadDoubleWord 0x081C0060; sysbus ReadDoubleWord 0x081E0000; sysbus ReadDoubleWord 0x081E0060"
  c=${c/"$run"/"$pre$body$post"}
  [ "${LOG:-0}" = 1 ] && c="${c%; quit}; include @/tvi/regmap.py; quit"   # Tviburi 1100's map reader, at d22dad8
  args[$((i + 1))]=$c
done
printf '%s\n' "${args[@]}" > "$O/argv.txt"
cd "$d"
start=$(date +%s.%N)
if [ "${LOG:-0}" = 1 ]; then
  env -i HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0 \
    /opt/renode/renode "${args[@]}" < /dev/null 2>&1 | grep -av '\] Read\(UInt\|Byte\|Word\|DoubleWord\|QuadWord\)' | gzip -1 > "$O/console.txt.gz" &
else
  env -i HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0 \
    /opt/renode/renode "${args[@]}" < /dev/null 2>&1 | gzip -1 > "$O/console.txt.gz" &
fi
sleep 0.05; peak=0; loads=""
while kill -0 $! 2>/dev/null; do
  s=0; for p in /proc/[0-9]*; do grep -qa 'enode' $p/cmdline 2>/dev/null || continue; r=$(awk '/^VmRSS:/{print $2}' $p/status 2>/dev/null); s=$((s + ${r:-0})); done
  [ $s -gt $peak ] && peak=$s; sleep 0.1
done
wait $!
echo "wall_s=$(awk -v a=$start -v b=$(date +%s.%N) 'BEGIN{printf "%.2f", b-a}') peak_tree_vmrss_kib=$peak load=$(cut -d' ' -f1-3 /proc/loadavg)" | tee "$O/cost.txt"
