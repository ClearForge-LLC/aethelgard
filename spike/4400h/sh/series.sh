#!/bin/bash
# AET-WO-4400h (1): the window series, one run after another inside one capped container.
# Usage: series.sh TAG  (runs: lifted bounds, log on and off, at 5/10/20/30 s)
set -u
tag=$1
for w in 5 10 20 30; do
  AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 bash /spike/sh/window.sh "$tag-L-w$w" "${w}000ms"
  AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 AET4400H_NO_USART_LOG=1 bash /spike/sh/window.sh "$tag-N-w$w" "${w}000ms"
done
