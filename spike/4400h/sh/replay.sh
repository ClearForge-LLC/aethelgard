#!/bin/bash
# AET-WO-4400h (2)-(4): replay the captured CLI Renode invocation (capture.sh) in the pinned tools image,
# in the environment the CLI gives Renode (as Tviburi's spike/1150/harness/replay-ap.sh), with:
#   REQ=1     the twin's missing line: `ReceiveDmaRequest -> dmamux1@41` in usart1's block of the
#             booted platform (STM32F7_USART.cs:87, :109; RM0433 Table 121 request 41 usart1_rx_dma)
#   INJ=DIR   `emulation RunFor` replaced by DIR/inject.resc (frames.py)
#   LOG=1     `sysbus LogAllPeripheralsAccess true` before the run, collapse off, reads dropped from the
#             stored console (gzipped), and Tviburi 1100's regmap.py after it, as Tviburi (v) (replay-ap.sh `logged`)
#   NOHOOK=1  the CLI's SPI CFG2 write hooks dropped (the RSS cause, replay against replay)
#   GCLIMIT=HEX  DOTNET_GCHeapHardLimit for Renode: is the growth live memory or uncollected garbage?
#   RESETMACRO=1|2  a `reset` macro (2: without LoadELF) of the CLI's boot lines, so a firmware's SYSRESETREQ reboots it (a twin change)
#   LOGP="p ..."  only the named peripherals' accesses logged (the DShot and persistence runs: 24 s with a reboot)
#   SILENCE=1 `logLevel 3` before the run; SILENCE=2 also each device's and spi2's own level: the flood's cost on RSS
# Always: USART1's TX into OUT/tx.bin (UartFileBackend), and the storage sectors' first words read after.
# Usage: replay.sh CAPDIR OUTDIR     cost line in OUTDIR/cost.txt (tree VmRSS peak at 100 ms, as replay-ap.sh)
set -u
cap=$1; O=$2; rm -rf "$O"; mkdir -p "$O"
mapfile -d '' args < "$cap/argv.bin"; args=("${args[@]:1}")
cfg=""; for i in "${!args[@]}"; do [ "${args[$i]}" = "--config" ] && cfg=${args[$((i + 1))]}; done
d=$(dirname "$cfg"); plat=$d/platform-booted.repl
rm -rf "$d"; mkdir -p "$(dirname "$d")"; cp -r "$cap/tool" "$d"   # the captured scratch, at its own path
if [ "${REQ:-0}" = 1 ]; then
  python3 -I - "$plat" <<'PY' || { echo "replay: the request line was not added" >&2; exit 3; }
import sys, re
p = sys.argv[1]; s = open(p).read()
m = re.search(r"^usart1:[^\n]*\n((?:    [^\n]*\n)*)", s, re.M)
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
  [ "${SILENCE:-0}" = 1 ] && pre="${pre}logLevel 3; "   # (1)'s cause: global ERROR level (per-device -1 levels survive it)
  [ "${SILENCE:-0}" = 2 ] && pre="${pre}logLevel 3; logLevel 3 sysbus.spi2.spi2mux.u5; logLevel 3 sysbus.spi2.spi2mux; logLevel 3 sysbus.spi1.spi1mux.u4; logLevel 3 sysbus.spi1.spi1mux; logLevel 3 sysbus.i2c1.u7; "   # and every device line (spi2's ERROR stays: ERROR is the top level)
  # RESETMACRO=1: Renode's `reset` macro, which the platform does not register, so SYSRESETREQ resets nothing
  # (s03-dshot: "No action for reset - macro machine_0.reset is not registered"). Made of the CLI's own boot lines,
  # `sysbus LoadELF` through `cpu VectorTableOffset`: a reboot reloads the ELF's sections (bank 1) and the RCC words,
  # and keeps the rest of memory, as Renode's MappedMemory.Reset does (bank 2's storage sectors stay as written).
  if [ "${RESETMACRO:-0}" != 0 ]; then
    boot=$(grep -o 'sysbus LoadELF @[^;]*;.*cpu VectorTableOffset 0x[0-9a-fA-F]*' <<< "$c")
    # RESETMACRO=2: the same without LoadELF (memory, the ELF's sections included, survives a reset)
    [ "${RESETMACRO}" = 2 ] && boot=${boot#sysbus LoadELF @*; }
    [ -n "$boot" ] || { echo "replay: no boot lines for the reset macro" >&2; exit 3; }
    pre="${pre}macro reset \"$boot\"; "
  fi
  # LOGP="timer1 dma1 ...": only those peripherals' accesses logged (a long run's targeted log; reads kept)
  for q in ${LOGP:-}; do pre="${pre}sysbus LogPeripheralAccess sysbus.$q true; "; done
  [ "${LOG:-0}" = 1 ] && { sed -i 's/^collapse-repeated-log-entries = True/collapse-repeated-log-entries = False/' "$cfg"; pre="${pre}sysbus LogAllPeripheralsAccess true; "; }
  # NOHOOK=1: drop the CLI's two SPI write hooks (foreign.rs cfg2_hook), (1)'s cause: does RSS follow them?
  [ "${NOHOOK:-0}" = 1 ] && c=$(python3 -I -c 'import re,sys; print(re.sub(r"sysbus SetHookBeforePeripheralWrite sysbus\.spi[0-9] \"[^\"]*\"; ", "", sys.argv[1]), end="")' "$c")
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
  env -i HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0 ${GCLIMIT:+DOTNET_GCHeapHardLimit=$GCLIMIT} \
    /opt/renode/renode "${args[@]}" < /dev/null 2>&1 | grep -av '\] Read\(UInt\|Byte\|Word\|DoubleWord\|QuadWord\)' | gzip -1 > "$O/console.txt.gz" &
else
  env -i HOME="$d" TMPDIR="$d" DOTNET_EnableDiagnostics=0 DOTNET_EnableWriteXorExecute=0 ${GCLIMIT:+DOTNET_GCHeapHardLimit=$GCLIMIT} \
    /opt/renode/renode "${args[@]}" < /dev/null 2>&1 | gzip -1 > "$O/console.txt.gz" &
fi
sleep 0.05; peak=0; loads=""
while kill -0 $! 2>/dev/null; do
  s=0; for p in /proc/[0-9]*; do grep -qa 'enode' $p/cmdline 2>/dev/null || continue; r=$(awk '/^VmRSS:/{print $2}' $p/status 2>/dev/null); s=$((s + ${r:-0})); done
  [ $s -gt $peak ] && peak=$s; sleep 0.1
done
wait $!
echo "wall_s=$(awk -v a=$start -v b=$(date +%s.%N) 'BEGIN{printf "%.2f", b-a}') peak_tree_vmrss_kib=$peak load=$(cut -d' ' -f1-3 /proc/loadavg)" | tee "$O/cost.txt"
