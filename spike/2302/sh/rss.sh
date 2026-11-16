#!/bin/bash
# rss.sh <outfile> <cmd...>: run cmd, sampling the summed RSS of every Renode/dotnet process every 100 ms; peak in <outfile>.
out=$1; shift
( peak=0; while :; do r=$(ps -eo rss=,args= | awk '/[Rr]enode/ {s+=$1} END {print s+0}'); [ "$r" -gt "$peak" ] && peak=$r && echo $peak > $out; sleep 0.1; done ) &
sp=$!
"$@"; rc=$?
kill $sp 2>/dev/null; wait $sp 2>/dev/null
echo "peak_rss_kib $(cat $out 2>/dev/null)"
exit $rc
