# S4 extras, after s4-matrix.sh: route iii (stock BME280, the fault = run-time removal, sent on transitions only),
# ioc backend, 100 us and 10 us, both R, twice; then the cost of a quantum with one read instead of twelve
# (10 ms span; 1 ms for the 1 us quantum, as in s4-part3.sh).
cd /repo/spike/2101/lockstep
B=/cache/target-ls2/release/lockstep; O=/cache/s4; mkdir -p $O
on='sysbus.i2c1 Unregister sysbus.i2c1.bme'; off='machine LoadPlatformDescriptionFromString "bme: I2C.BME280 @ i2c1 0x76"'
span=10000000
for q in 100000 10000; do for r in 2k2 47k; do for run in 1 2; do
  tag=iii-ioc-q$q-$r-r$run
  line=$($B --renode-setup s4-iii.setup --reads s4.reads --fault-on "$on" --fault-off "$off" --quantum-ns $q --span-ns $span --deck line-$r.cir --spice ioc --out $O/$tag 2>&1 | tail -1)
  echo "$tag $line seq_sha256=$(sha256sum < $O/$tag.seq | cut -c1-64)"
done; done; done
for q in 100000 10000 1000; do span=10000000; [ $q = 1000 ] && span=1000000; for r in 2k2 47k; do
  tag=min-ii-ioc-q$q-$r
  line=$($B --renode-setup s4-ii.setup --reads s4-min.reads --fault-cmd 'sysbus.i2c1.sensor Fault {}' --quantum-ns $q --span-ns $span --deck line-$r.cir --spice ioc --out $O/$tag 2>&1 | tail -1)
  echo "$tag $line"
  tag=min-ii-none-q$q-$r
  line=$($B --renode-setup s4-ii.setup --reads s4-min.reads --quantum-ns $q --span-ns $span --spice none --out $O/$tag 2>&1 | tail -1)
  echo "$tag $line"
done; done
