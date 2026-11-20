#!/bin/bash
# Inside the image: main's CLI (built by gen.sh). For each two-device board: the reference firmware over
# baseline + held-low-sda, and the wrong-address mutant (2204's patch) over a baseline expecting the timeout;
# campaign, then attribute on each archive.
set -u
B=/cache/target/debug; A=$B/aethelgard
export PATH=$B:$PATH
one() { # board find replace
  s=$1; O=/cache/m/$s; rm -rf $O/sc $O/fw $O/arch; mkdir -p $O/sc $O/fw
  cp /repo/crates/aethelgard-cli/tests/fixtures/$s.json $O/sc/base.json
  printf '{"schema_version":1,"name":"baseline","base":"base.json","expect":"pass","expect_findings":[],"overrides":[]}\n' > $O/sc/baseline.json
  printf '{"schema_version":1,"name":"held-low-sda","base":"base.json","expect":"fail","expect_findings":["i2c_line_held_low","i2c_timeout"],"overrides":[{"bus":"SENSORS","line_held_low":"sda"}]}\n' > $O/sc/held-low-sda.json
  printf '{"schema_version":1,"name":"wrong-address","base":"base.json","expect":"fail","expect_findings":["i2c_timeout"],"overrides":[]}\n' > $O/sc/wrong-address.json
  rm -rf $O/mut; cp -a $O/gen $O/mut
  python3 -I -c "import sys;p,o,n=sys.argv[1:4];s=open(p).read();assert s.count(o)==1,(p,o);open(p,'w').write(s.replace(o,n))" $O/mut/board/firmware/src/main.rs "$2" "$3" || exit 7
  $A build --crate $O/gen/board --out $O/fw/ref.elf > $O/fw/ref.json 2>&1; echo "$s build ref rc=$?"
  $A build --crate $O/mut/board --out $O/fw/mut.elf > $O/fw/mut.json 2>&1; echo "$s build mut rc=$?"
  cd $O/sc
  $A campaign --scenarios baseline.json held-low-sda.json --firmware $O/fw/ref.elf --archive $O/arch/ref > $O/ref.campaign.jsonl 2> $O/ref.campaign.err; echo "$s campaign ref rc=$?"
  $A campaign --scenarios wrong-address.json --firmware $O/fw/mut.elf --archive $O/arch/mut > $O/mut.campaign.jsonl 2> $O/mut.campaign.err; echo "$s campaign mut rc=$?"
  for r in $O/arch/*/*/; do echo "== attribute $r"; $A attribute --archive $r > ${r%/}.attribute.json 2>&1; echo "rc=$?"; done
}
one sentinel-03 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x77,'
one two-bme 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x75,'
