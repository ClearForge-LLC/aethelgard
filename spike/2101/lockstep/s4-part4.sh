# S4 part 4: does a 1 us quantum's cost grow with the span? Renode alone (none) and ioc, route ii, 2.2k,
# over 1, 2 and 4 ms (1,000 / 2,000 / 4,000 quanta), one run each. Per-quantum means and the split are in each
# run's costs.csv; a rising per-quantum mean is the superlinearity that stopped the 10 ms run (s4-matrix log).
cd /repo/spike/2101/lockstep
B=/cache/target-ls2/release/lockstep; O=/cache/s4
for span in 1000000 2000000 4000000; do
  for be in none ioc; do
    tag=p4-$be-q1000-2k2-s$span
    line=$(timeout 600 $B --renode-setup s4-ii.setup --reads s4.reads --fault-cmd 'sysbus.i2c1.sensor Fault {}' --quantum-ns 1000 --span-ns $span --deck line-2k2.cir --spice $be --out $O/$tag 2>&1 | tail -1)
    echo "$tag ${line:-KILLED_AT_600s}"
  done
done
