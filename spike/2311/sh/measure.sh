#!/bin/bash
# AET-WO-2311 §1.1: Renode run directly on measure.py's script, with the CLI's renode.config
# (renode.rs `config`). Inside the tools image. Usage: measure.sh NAME ELF [EXTRA_FILE]
set -u
cd /repo
O=/cache/run/$1; rm -rf $O; mkdir -p $O/home
python3 -I spike/2311/measure.py /cache/cap/base "$2" $O/script.txt ${3:-}
printf '[general]\nuse-synchronous-logging = true\nterminal = Termsharp\ncompiler-cache-enabled = False\nserialization-mode = Generated\nalways-log-machine-name = False\ncollapse-repeated-log-entries = True\nlog-timestamps-type = Virtual\nlog-history-limit = 1000\nhistory-path = %s/home/.config/renode/history\nstore-table-bits = 41\n[monitor]\nconsume-exceptions-from-command = True\nbreak-script-on-exception = True\nnumber-format = Hexadecimal\n[plugins]\nenabled-plugins = \n[translation]\nmin-tb-size = 33554432\nmax-tb-size = 536870912\n' "$O" > $O/renode.config
t0=$(date +%s%N)
HOME=$O/home timeout 120 /usr/local/bin/renode --config $O/renode.config --disable-gui --console --plain -e "$(cat $O/script.txt)" > $O/out.txt 2>&1
echo "renode exit=$? wall_ms=$((($(date +%s%N)-t0)/1000000)) bytes=$(wc -c < $O/out.txt) cr2_lines=$(grep -c AETHELGARD-CR2 $O/out.txt)"
