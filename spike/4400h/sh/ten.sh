#!/bin/bash
# AET-WO-4400h (1): the 10 s window repeated, log on, wall lifted to 600 s.
#   ten.sh TAG lifted   twice, RSS cap lifted to 6 GiB to read the share (runs t1-L-w10a, b)
#   ten.sh TAG cap      three times at main's real cap: a memory_cap_exceeded verdict is §7's stop (runs t2-C-w10a-c)
set -u
if [ "$2" = lifted ]; then for r in a b; do AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 bash /spike/sh/window.sh "$1-L-w10$r" 10000ms; done
else for r in a b c; do AET4400H_WALL_S=600 bash /spike/sh/window.sh "$1-C-w10$r" 10000ms; done; fi
