#!/bin/bash
# AET-WO-4400h (1): 10 s twice, log on, cap lifted to read the share (wall 600 s): does the cap fail at 10 s off-peak?
set -u
for r in a b; do AET4400H_WALL_S=600 AET4400H_RSS_KIB=6291456 bash /spike/sh/window.sh "$1-L-w10$r" 10000ms; done
