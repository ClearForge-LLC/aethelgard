#!/bin/bash
# AET-WO-4400h (3): one logged replay's console (replay.sh LOG=1) -> Tviburi's ArduPilot queue, by TVI-WO-1150 (v)'s
# own readers, unchanged, at Tviburi main d22dad8 (copied by `git show` into ~/.cache/aet-4400h/tvi; SHA256SUMS there):
#   1100's regmap.py (run in the replay), view2-writes.awk, view2-join.awk, cmsis-regs.awk, 1100's queue.tsv and
#   joined.tsv, and 1150's rank-ap.awk. View (1) for ArduPilot is every field id of 1100's joined.tsv, its first
#   15 columns, given as board "ardupilot" (results.md (v): "given to 1100's own view2-join.awk, unchanged").
# On the host (gawk). Usage: queue.sh RUN_DIR      -> RUN_DIR/{regmap,writes,view1-ap,joined-ap,only2-ap,queue-ap}.tsv
set -eu
d=$1; T=$HOME/.cache/aet-4400h/tvi
H=/home/forge/aethelgard-4400h/crates/aethelgard-svd-hal/data/st-cmsis-device-h7/stm32h753xx.h
zcat "$d/console.txt.gz" | tr -d '\r' | grep -a '^TVIMAP\|^TVIBASE' > "$d/regmap.tsv"
zcat "$d/console.txt.gz" | tr -d '\r' | gawk -f "$T/view2-writes.awk" ardupilot > "$d/writes.tsv"
gawk -F'\t' -v OFS='\t' '!seen[$2]++ { $1 = "ardupilot"; NF = 15; print }' "$T/joined.tsv" > "$d/view1-ap.tsv"
gawk -F'\t' -v OFS='\t' -v board=ardupilot -v out2="$d/only2-ap.tsv" -f "$T/view2-join.awk" \
  "$d/regmap.tsv" "$d/writes.tsv" "$d/view1-ap.tsv" > "$d/joined-ap.tsv"
awk -f "$T/cmsis-regs.awk" "$H" | LC_ALL=C sort > "$d/cmsis-regs.tsv"
gawk -F'\t' -f "$T/rank-ap.awk" "$d/cmsis-regs.tsv" "$d/joined-ap.tsv" "$d/only2-ap.tsv" "$T/queue.tsv" > "$d/queue-ap.tsv"
echo "queue: $(wc -l < "$d/queue-ap.tsv") rows; fw writes $(awk '$1=="W" && $4=="fw"' "$d/writes.tsv" | wc -l); judge (no PC) $(awk '$1=="W" && $4=="judge"' "$d/writes.tsv" | wc -l)"
