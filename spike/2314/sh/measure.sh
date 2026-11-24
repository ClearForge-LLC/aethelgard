#!/bin/bash
# AET-WO-2314 §1.1 (spike/1609/sh/measure.sh, MODE and a VmHWM poll added): Renode run directly on measure.py's script, with the
# CLI's renode.config (renode.rs `config`). Inside the tools image. Usage: measure.sh NAME ELF MODE [nolog]
set -u
cd /repo
O=/cache/mrun/$1; rm -rf $O; mkdir -p $O/home
python3 -I spike/2314/measure.py /cache/cap "$2" $O/script.txt "$3" ${4:-}
printf '[general]\nuse-synchronous-logging = true\nterminal = Termsharp\ncompiler-cache-enabled = False\nserialization-mode = Generated\nalways-log-machine-name = False\ncollapse-repeated-log-entries = True\nlog-timestamps-type = Virtual\nlog-history-limit = 1000\nhistory-path = %s/home/.config/renode/history\nstore-table-bits = 41\n[monitor]\nconsume-exceptions-from-command = True\nbreak-script-on-exception = True\nnumber-format = Hexadecimal\n[plugins]\nenabled-plugins = \n[translation]\nmin-tb-size = 33554432\nmax-tb-size = 536870912\n' "$O" > $O/renode.config
start=$(date +%s.%N)
HOME=$O/home timeout ${TIMEOUT:-120} /usr/local/bin/renode --config $O/renode.config --disable-gui --console --plain -e "$(cat $O/script.txt)" > $O/out.txt 2>&1 &
pid=$!; peak=0
while kill -0 $pid 2>/dev/null; do
  for p in $(pgrep -P $pid) $pid; do h=$(awk '/VmHWM/{print $2}' /proc/$p/status 2>/dev/null); [ -n "$h" ] && [ "$h" -gt "$peak" ] && peak=$h; done
  sleep 0.1
done
wait $pid; rc=$?
echo "ANSWER 2314: measure $1 $3 ${4:-log} exit=$rc wall_s=$(python3 -c "print(round($(date +%s.%N) - $start, 1))") peak_vmhwm_kib=$peak bytes=$(wc -c < $O/out.txt)"
