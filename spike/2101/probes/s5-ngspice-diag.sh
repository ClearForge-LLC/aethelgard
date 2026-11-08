# S5 diagnosis: is /tmp (tmpfile()) the only thing ngspice lacks under the sandbox? And the pty?
wd=/tmp/s5wd; mkdir -p $wd; cp /lockstep/line-2k2.cir $wd/line.cir
printf '%s\n' '.tran 1n 22.5u 0 5n' '.meas tran trise TRIG v(line) VAL=0.99 RISE=1 TARG v(line) VAL=2.31 RISE=1' > $wd/tran.inc
sed 's/^\.end$/.include tran.inc\n.end/' $wd/line.cir > $wd/a.cir
command -v strace >/dev/null && echo has-strace || echo no-strace
echo "== a, with /tmp granted read-write (diagnostic: too broad to rule)"
( cd $wd; bash /w/sbx.sh $wd /tmp/d1.status --rw /tmp -- /usr/bin/ngspice -b a.cir < /dev/null 2>&1 | grep -E 'trise|rror|denied|SBX' )
echo "== a, with SPICE_SCRIPTS etc unchanged, /dev/null rw only"
( cd $wd; bash /w/sbx.sh $wd /tmp/d2.status --rw /dev/null -- /usr/bin/ngspice -b a.cir < /dev/null 2>&1 | grep -E 'trise|rror|denied|SBX' )
echo "== b under pty, /tmp + /dev/ptmx + /dev/pts granted"
( cd $wd; printf 'source line.cir\ntran 1n 22.5u 0 5n\nprint length(time)\nquit\n' | bash /w/sbx.sh $wd /tmp/d3.status --rw /tmp --rw /dev/ptmx --rw /dev/pts --rw /dev/null -- /usr/bin/script -qec "ngspice -p -t dumb" /dev/null 2>&1 | tr -d '\r' | grep -E 'length|rror|denied|SBX|pseudo' )
echo "== b under pty, /dev/ptmx + /dev/pts only (no /tmp)"
( cd $wd; printf 'source line.cir\ntran 1n 22.5u 0 5n\nprint length(time)\nquit\n' | bash /w/sbx.sh $wd /tmp/d4.status --rw /dev/ptmx --rw /dev/pts -- /usr/bin/script -qec "ngspice -p -t dumb" /dev/null 2>&1 | tr -d '\r' | grep -E 'length|rror|denied|SBX|pseudo' )
