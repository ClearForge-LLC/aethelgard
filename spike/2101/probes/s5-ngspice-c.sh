# S5: route c (libngspice in-process, the probe host) under the CLI's sandbox, unchanged; then route b diagnostics.
wd=/tmp/s5wd; mkdir -p $wd; cp /cache/target-ng/release/s1c $wd/../s1c
echo "== c: s1c s2 bkpt 47000 7f 1 under the sandbox (no extra grants)"
( cd $wd; bash /w/sbx.sh $wd /tmp/c1.status --ro /tmp/s1c -- /tmp/s1c s2 bkpt 47000 7f 1 < /dev/null 2>&1 | grep -vE '^\s*$' | tail -8 )
echo "== c: s1c s1 bkpt 1000 transients under the sandbox (timing)"
( cd $wd; s=$(date +%s.%N); bash /w/sbx.sh $wd /tmp/c2.status --ro /tmp/s1c -- /tmp/s1c s1 bkpt 1000 $wd/c.tsv < /dev/null 2>&1 | grep -vE '^\s*$' | tail -4; e=$(date +%s.%N); echo "wall_s=$(awk -v s=$s -v e=$e "BEGIN{print e-s}")"; wc -l < $wd/c.tsv; awk -F"\t" "NR>1{t+=\$2} END{printf \"sum_transient_ms=%.1f\\n\", t/1e6}" $wd/c.tsv; cut -f3 $wd/c.tsv | tail -n +2 | sha256sum )
echo "== c: the same 1000 unsandboxed, for the hash comparison"
( cd /tmp; /tmp/s1c s1 bkpt 1000 /tmp/c-host.tsv > /dev/null 2>&1; cut -f3 /tmp/c-host.tsv | tail -n +2 | sha256sum )
echo "== b: pty, with /tmp, /dev/ptmx, /dev/pts, /dev/null and /dev/tty granted"
( cd $wd; printf 'source /tmp/line.cir\ntran 1n 22.5u 0 5n\nprint length(time)\nquit\n' > /tmp/in.txt; cp /lockstep/line-2k2.cir /tmp/line.cir
  bash /w/sbx.sh $wd /tmp/b1.status --rw /tmp --rw /dev/ptmx --rw /dev/pts --rw /dev/null --rw /dev/tty -- /usr/bin/script -qec "ngspice -p -t dumb" /dev/null < /tmp/in.txt 2>&1 | tr -d '\r' | grep -E 'length|rror|denied|SBX|pseudo' )
echo "== b: pty, with all of /dev read-write (diagnostic)"
( cd $wd; bash /w/sbx.sh $wd /tmp/b2.status --rw /tmp --rw /dev -- /usr/bin/script -qec "ngspice -p -t dumb" /dev/null < /tmp/in.txt 2>&1 | tr -d '\r' | grep -E 'length|rror|denied|SBX|pseudo' )
