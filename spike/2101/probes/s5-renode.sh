# S5: each S3 fault route under the CLI's sandbox, unchanged (sbx.sh), monitor commands on stdin.
# For each command file: the sandboxed run's summary HASH against the same file run unsandboxed.
# Run in the image with the repo at /repo and ~/.cache/aet-2101 at /cache.
R=/repo/spike/2101/renode
for c in "$@"; do
  wd=$(mktemp -d /tmp/s5r.XXXX); st=$wd.status
  t0=$(date +%s.%N)
  sb=$(cd $wd; bash /repo/spike/2101/probes/sbx.sh $wd $st --ro $R -- /usr/local/bin/renode --disable-gui --console --plain < $R/$c 2>&1 | tr -d '\r')
  t1=$(date +%s.%N)
  un=$(cd /opt/renode; DOTNET_EnableDiagnostics=0 renode --disable-gui --console --plain < $R/$c 2>&1 | tr -d '\r')
  hs=$(echo "$sb" | python3 -I $R/summarize.py | grep '^HASH'); hu=$(echo "$un" | python3 -I $R/summarize.py | grep '^HASH')
  echo "=== $c  sandboxed_wall_s=$(awk -v a=$t0 -v b=$t1 'BEGIN{printf "%.2f", b-a}')"
  echo "$sb" | grep -E '^SBX|There was an error|\[ERROR\]|denied|Permission|Exception|[Cc]ompil' | sort | uniq -c | head -8
  echo "sandboxed   $hs"; echo "unsandboxed $hu"; [ "$hs" = "$hu" ] && [ -n "$hs" ] && echo "MATCH" || echo "DIFFER"
  ls -la $wd | tail -n +4 | awk '{print "  workdir:", $5, $9}' | head -6
done
