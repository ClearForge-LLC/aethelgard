#!/bin/bash
# Inside the image, after run.sh: two-bme with U2's read sent to 0x77, where U5 answers (the colliding
# wrong address). The scenario expects a pass so the run archives whatever the judge finds.
set -u
B=/cache/target/debug; A=$B/aethelgard; export PATH=$B:$PATH
O=/cache/m/two-bme; rm -rf $O/col $O/arch/col; cp -a $O/gen $O/col
python3 -I -c "import sys;p,o,n=sys.argv[1:4];s=open(p).read();assert s.count(o)==1,(p,o);open(p,'w').write(s.replace(o,n))" $O/col/board/firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x77,' || exit 7
$A build --crate $O/col/board --out $O/fw/col.elf > $O/fw/col.json 2>&1; echo "build rc=$?"
cd $O/sc
printf '{"schema_version":1,"name":"collide","base":"base.json","expect":"pass","expect_findings":[],"overrides":[]}\n' > collide.json
$A campaign --scenarios collide.json --firmware $O/fw/col.elf --archive $O/arch/col > $O/col.campaign.jsonl 2> $O/col.campaign.err; echo "campaign rc=$?"
for r in $O/arch/col/*/; do $A attribute --archive $r > ${r%/}.attribute.json 2>&1; echo "attribute rc=$?"; done
