# S5: the S4 owner itself under the CLI's sandbox, unchanged: the lock-step host (libngspice in-process)
# with Renode as its child, 47k, route ii, 100 us quanta, 10 ms. Compared with the same run unsandboxed.
# Run in the image: repo at /repo, ~/.cache/aet-2101 at /cache.
L=/repo/spike/2101/lockstep; B=/cache/target-ls/release/lockstep
args="--renode-setup s4-ii.setup --reads s4.reads --quantum-ns 100000 --span-ns 10000000 --deck line-47k.cir --renode /usr/local/bin/renode"
for be in ioc lib pipe; do
  wd=$(mktemp -d /tmp/s5l.XXXX)
  extra=""; [ $be = pipe ] && extra="--rw /tmp --rw /dev/ptmx --rw /dev/pts --rw /dev/null --rw /dev/tty"
  echo "=== $be sandboxed ${extra:+(with: $extra)}"
  ( cd $L; bash /repo/spike/2101/probes/sbx.sh $wd $wd.status --ro /repo/spike/2101 --ro /cache/target-ls/release $extra -- $B $args --fault-cmd "sysbus.i2c1.sensor Fault {}" --spice $be --out $wd/sb 2>&1 | tail -3 )
  ( cd $L; $B $args --fault-cmd "sysbus.i2c1.sensor Fault {}" --spice $be --out $wd/un > /dev/null 2>&1 )
  echo "sandboxed   seq_sha256=$(sha256sum < $wd/sb.seq | cut -c1-64)"; echo "unsandboxed seq_sha256=$(sha256sum < $wd/un.seq | cut -c1-64)"
done
echo "=== pipe sandboxed with no extra grants"
wd=$(mktemp -d /tmp/s5l.XXXX); ( cd $L; bash /repo/spike/2101/probes/sbx.sh $wd $wd.status --ro /repo/spike/2101 --ro /cache/target-ls/release -- $B $args --fault-cmd "sysbus.i2c1.sensor Fault {}" --spice pipe --out $wd/sb 2>&1 | grep -vE '^\s*$' | tail -4 | cut -c1-300 )
