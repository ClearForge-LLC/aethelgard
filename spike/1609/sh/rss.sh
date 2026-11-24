#!/bin/bash
# AET-WO-1609 §1.1: Renode's peak RSS on measure.py's script, stdout discarded, with and without
# the CLI's USART access log. Inside the tools image. Usage: rss.sh NAME ELF [nolog]
set -u
cd /repo
O=/cache/rss/$1; rm -rf $O; mkdir -p $O/home
python3 -I spike/1609/measure.py /cache/cap "$2" $O/script.txt ${3:-}
printf '[general]\nuse-synchronous-logging = true\nterminal = Termsharp\ncompiler-cache-enabled = False\nserialization-mode = Generated\nalways-log-machine-name = False\ncollapse-repeated-log-entries = True\nlog-timestamps-type = Virtual\nlog-history-limit = 1000\nhistory-path = %s/home/.config/renode/history\nstore-table-bits = 41\n[monitor]\nconsume-exceptions-from-command = True\nbreak-script-on-exception = True\nnumber-format = Hexadecimal\n[plugins]\nenabled-plugins = \n[translation]\nmin-tb-size = 33554432\nmax-tb-size = 536870912\n' "$O" > $O/renode.config
HOME=$O/home timeout 300 /usr/local/bin/renode --config $O/renode.config --disable-gui --console --plain -e "$(cat $O/script.txt)" > /dev/null 2>&1 &
peak=0
while kill -0 $! 2>/dev/null; do
  for p in $(pgrep -f '[R]enode' ) ; do h=$(awk '/VmHWM/{print $2}' /proc/$p/status 2>/dev/null); [ -n "$h" ] && [ "$h" -gt "$peak" ] && peak=$h; done
  sleep 0.1
done
wait $!; echo "ANSWER 1609: rss $1 ${3:-log} exit=$? peak_vmhwm_kib=$peak"
