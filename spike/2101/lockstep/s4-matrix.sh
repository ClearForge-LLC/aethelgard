# S4: the lock-step matrix. Route ii (C# sensor, Fault 1 = no bytes), the repeat firmware, a 10 ms span
# (boot + about 40 ID reads), quanta 1 us / 10 us / 100 us / 1 ms, backends pipe / lib / ioc, R = 2.2k and 47k,
# each run twice; the state sequence is hashed per run. Then route i (Python, withhold) once per R, twice.
# Run in the image: repo at /repo, ~/.cache/aet-2101 at /cache.
cd /repo/spike/2101/lockstep
B=/cache/target-ls/release/lockstep; O=/cache/s4; mkdir -p $O
one() { # backend quantum_ns R route run
  local tag=$4-$1-q$2-$3-r$5 setup fault
  if [ $4 = ii ]; then setup=s4-ii.setup; fault='sysbus.i2c1.sensor Fault {}'; else setup=s4-i.setup; fault='s3_fault {}'; fi
  local line; line=$(timeout 900 $B --renode-setup $setup --reads s4.reads --fault-cmd "$fault" --quantum-ns $2 --span-ns 10000000 --deck line-$3.cir --spice $1 --out $O/$tag 2>&1 | tail -1)
  echo "$tag $line seq_sha256=$(sha256sum < $O/$tag.seq | cut -c1-64)"
}
# Ordered so every backend gets 1 ms, 100 us and 10 us before any 1 us run. Pass 1 (ioc at 1 ms, 100 us
# and 10 us) ran with this loop body first; its lines are in s4-matrix-part1 of the log.
for be in ${BACKENDS:-lib pipe}; do for q in 1000000 100000 10000; do for r in 2k2 47k; do for run in 1 2; do
  one $be $q $r ii $run
done; done; done; done
for r in 2k2 47k; do for run in 1 2; do one ioc 1000 $r ii $run; done; done
one lib 1000 47k ii 1; one pipe 1000 47k ii 1
for r in 2k2 47k; do for run in 1 2; do one ioc 100000 $r i $run; done; done
