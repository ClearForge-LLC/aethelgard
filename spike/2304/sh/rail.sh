#!/bin/bash
# Inside the image: the unmodified bridge's --rail on dshot600-droop's droop (VDD_3V3 to 2.4 V,
# 500 mOhm, 100 mA), at the emitter's decoupling (10.81 uF) and with U11's outputs (98.81 uF), at the
# scenario's hold (200 us) and at 600 us (above 10 tau for 98.81 uF). Output /cache/rail/*.out.
set -u
B=/cache/target/debug/aethelgard-bridge
O=/cache/rail; rm -rf $O; mkdir -p $O
for pf in 10810000 98810000; do for hold in 200000 600000; do
  f=$O/req-$pf-$hold.json
  printf '{"domain":"VDD_3V3","nominal_microvolts":3300000,"floor_microvolts":2400000,"start_ns":100000,"fall_ns":20000,"hold_ns":%s,"rise_ns":20000,"source_milliohms":500,"load_microamps":100000,"decoupling_picofarads":%s,"span_ns":100000000,"levels":[{"name":"VBOR0_falling","microvolts":1620000},{"name":"VBOR0_rising","microvolts":1670000},{"name":"VBOR3_falling","microvolts":2610000},{"name":"VBOR3_rising","microvolts":2700000}]}' $hold $pf > $f
  (cd $O && $B --rail $f > $O/out-$pf-$hold.txt 2> $O/err-$pf-$hold.txt; echo "pf=$pf hold=$hold exit=$?")
  cat $O/out-$pf-$hold.txt
done; done
