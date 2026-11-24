#!/bin/bash
# AET-WO-4400h (1): does the growth start with the IMU (4.4 s) or at boot? 2, 3 and 4 s on main's bounds.
set -u
for w in 2 3 4; do bash /spike/sh/window.sh "$1-M-w$w" "${w}000ms"; done
