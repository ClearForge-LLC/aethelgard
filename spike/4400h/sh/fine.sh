#!/bin/bash
# AET-WO-4400h (1): the curve between 5 and 10 s at main's real cap (1,572,864 KiB), wall lifted to 600 s,
# log on (main's state) and off, then main's bounds unlifted at 5 s, then 5 s on two cores (taskset), each twice.
# Usage: fine.sh TAG
set -u
tag=$1
for w in 6 7 8 9; do
  AET4400H_WALL_S=600 bash /spike/sh/window.sh "$tag-C-w$w" "${w}000ms"
  AET4400H_WALL_S=600 AET4400H_NO_USART_LOG=1 bash /spike/sh/window.sh "$tag-CN-w$w" "${w}000ms"
done
for r in a b; do
  bash /spike/sh/window.sh "$tag-M-w5$r" 5000ms
  AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 bash /spike/sh/window.sh "$tag-T2-w5$r" 5000ms 0,1
done
