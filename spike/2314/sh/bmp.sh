#!/bin/bash
# AET-WO-2314 §1.1 (1): the BMP581 extension's undisturbed boot, as the campaign's boot(None) runs it
# (`simulate`, the board's whole MAX_VIRTUAL_NS window), N times: exit, boot word instant, findings,
# peak RSS against the cap. On the first run Renode's argv, platform and models are copied from /proc,
# and with ALONE=1 the same script is re-run by Renode alone to count its stdout bytes against the
# 8 MiB budget. Usage: bmp.sh N [CPUS]  (CPUS: a taskset list, e.g. 0-3)
set -u
cd /repo
A=/repo/target/debug/aethelgard; O=/cache/bmp; mkdir -p $O
S=examples/sentinel-01-ext-bmp581.json
[ -f $O/fw.elf ] || { $A generate --spec $S --out $O/gen > $O/generate.json && $A build --crate $O/gen/board --out $O/fw.elf > $O/build.json; } || { cat $O/generate.json $O/build.json; exit 1; }
ts=(); [ -n "${2:-}" ] && ts=(taskset -c "$2")
for i in $(seq 1 "$1"); do
  [ "$i" = 1 ] && ( for _ in $(seq 1 3000); do
      for p in /proc/[0-9]*; do c=$(tr '\0' '\n' < $p/cmdline 2>/dev/null) || continue
        case "$c" in *LoadPlatformDescription*) printf '%s' "$c" > $O/renode-argv.txt
          d=$(printf '%s' "$c" | grep -o 'LoadPlatformDescription @[^;]*' | head -1 | sed 's/LoadPlatformDescription @//')
          rm -rf $O/cap; mkdir -p $O/cap; cp "$d" $O/cap/platform.repl; cp -r "$(dirname "$d")/models" $O/cap/ 2>/dev/null; exit 0;; esac; done
      sleep 0.02; done ) &
  status=0; start=$(date +%s.%N)
  "${ts[@]}" $A simulate --spec $S --platform $O/gen/board/*.repl --firmware $O/fw.elf --for 100ms > $O/sim-$i.json || status=$?
  w=$(python3 -c "print(round($(date +%s.%N) - $start, 1))"); wait
  python3 -I spike/2314/sh/bmpline.py $O/sim-$i.json "$i" "$status" "$w"
done
[ "${ALONE:-0}" = 1 ] || exit 0
python3 -I spike/2314/sh/rescript.py $O/renode-argv.txt $O/cap $O/fw.elf $O/script.txt
mkdir -p $O/home
printf '[general]\nuse-synchronous-logging = true\nterminal = Termsharp\ncompiler-cache-enabled = False\nserialization-mode = Generated\nalways-log-machine-name = False\ncollapse-repeated-log-entries = True\nlog-timestamps-type = Virtual\nlog-history-limit = 1000\nhistory-path = %s/home/.config/renode/history\nstore-table-bits = 41\n[monitor]\nconsume-exceptions-from-command = True\nbreak-script-on-exception = True\nnumber-format = Hexadecimal\n[plugins]\nenabled-plugins = \n[translation]\nmin-tb-size = 33554432\nmax-tb-size = 536870912\n' "$O" > $O/renode.config
HOME=$O/home timeout 120 /usr/local/bin/renode --config $O/renode.config --disable-gui --console --plain -e "$(cat $O/script.txt)" > $O/out.txt 2>&1
echo "ANSWER 2314: bmp581 renode alone exit=$? stdout_bytes=$(wc -c < $O/out.txt) log_lines=$(grep -ac '^\[' $O/out.txt) budget=8388608 bootword_lines=$(grep -ac 'AETHELGARD-BOOTWORD' $O/out.txt)"
