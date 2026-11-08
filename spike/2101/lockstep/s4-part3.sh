# S4 part 3, after s4-matrix.sh. A 1 us quantum over the full 10 ms span did not finish inside
# `timeout 900` (ii-ioc-q1000-2k2-r1, s4-matrix log), so 1 us is measured over a 1 ms span (1,000 quanta),
# with 10 us over the same span as the control, for ioc, lib and Renode alone (none). Then route i, twice per R.
cd /repo/spike/2101/lockstep
B=/cache/target-ls2/release/lockstep; O=/cache/s4; mkdir -p $O
run() { # tag span args...
  local tag=$1 span=$2; shift 2
  local line; line=$(timeout 600 $B --span-ns $span --out $O/$tag "$@" 2>&1 | tail -1)
  echo "$tag ${line:-KILLED_AT_600s} seq_sha256=$(sha256sum < $O/$tag.seq 2>/dev/null | cut -c1-64)"
}

for q in 1000 10000; do for r in 2k2 47k; do
  for be in ioc lib; do for n in 1 2; do
    run ii-$be-q$q-$r-1ms-r$n 1000000 --renode-setup s4-ii.setup --reads s4.reads --fault-cmd 'sysbus.i2c1.sensor Fault {}' --quantum-ns $q --deck line-$r.cir --spice $be
  done; done
  run ii-none-q$q-$r-1ms-r1 1000000 --renode-setup s4-ii.setup --reads s4.reads --quantum-ns $q --spice none
done; done
for r in 2k2 47k; do for n in 1 2; do
  run i-ioc-q100000-$r-r$n 10000000 --renode-setup s4-i.setup --reads s4.reads --fault-cmd 's3_fault {}' --quantum-ns 100000 --deck line-$r.cir --spice ioc
done; done
