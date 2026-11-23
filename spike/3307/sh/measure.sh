#!/bin/bash
# AET-WO-3307 §1.1 (3306's measure.sh, with two groups added), inside the tools image (img.sh, REPO = a `git archive` of main 28fba951): every committed
# scenario group's campaign, the seeded set's mutants, and the two-device I2C cases, each run ONCE by
# main's CLI ($BIN), archived under $OUT. `attribute` is judged afterwards on these same archives by
# main's CLI and by the branch's (rejudge.py): this order changes no twin, trace or firmware.
set +e
BIN=${BIN:-/cache/target-main/debug}; OUT=${OUT:-/cache/runs}; A=$BIN/aethelgard
export PATH=$BIN:$PATH
cd /repo
mkdir -p $OUT/fw $OUT/sc
build() { # label spec [file find replace]
  local l=$1 s=$2 G=$OUT/fw/gen-$1
  [ -f $OUT/fw/$l.elf ] && return 0
  rm -rf $G; $A generate --spec /repo/$s --out $G > $OUT/fw/$l.gen.out 2>&1 || { echo "generate $l failed"; return 1; }
  if [ $# -gt 2 ]; then
    python3 -I -c "import sys;p,o,n=sys.argv[1:4];s=open(p).read();assert s.count(o)==1,(p,o);open(p,'w').write(s.replace(o,n))" "$G/board/$3" "$4" "$5" || { echo "patch $l failed"; return 1; }
  fi
  $A build --crate $G/board --out $OUT/fw/$l.elf > $OUT/fw/$l.build.out 2>&1; echo "built $l rc=$?"
}
camp() { # label elf cwd scenarios...
  local l=$1 e=$2 c=$3; shift 3
  local O=$OUT/camp/$l; rm -rf $O; mkdir -p $O
  (cd $c && $A campaign --scenarios "$@" --firmware $OUT/fw/$e.elf --archive $O/archive > $O/campaign.out 2> $O/campaign.err; echo $? > $O/campaign.rc)
  echo "campaign $l rc=$(cat $O/campaign.rc)"
}
# Committed scenario groups (3305's diff.sh groups, with sentinel-03 and sentinel-02-power whole).
build min examples/sentinel-01-minimal.json
build s01 examples/sentinel-01.json
build dshot crates/aethelgard-cli/tests/fixtures/motors-dshot600.json
build pwm crates/aethelgard-cli/tests/fixtures/motors-pwm.json
build serial crates/aethelgard-cli/tests/fixtures/serial-four.json
build ext examples/sentinel-01-ext-bmp581.json
build two crates/aethelgard-cli/tests/fixtures/two-bme.json
build s03 crates/aethelgard-cli/tests/fixtures/sentinel-03.json
build pow crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json
camp min min /repo $(ls scenarios/sentinel-01/*.json scenarios/sentinel-01/seeded/*.json | grep -v -e motors-pwm -e cases.json)
camp s01 s01 /repo scenarios/sentinel-01-bench/*.json
camp dshot dshot /repo scenarios/motors/*.json
camp pwm pwm /repo scenarios/sentinel-01/seeded/motors-pwm.json
camp serial serial /repo scenarios/serial/*.json
camp s03 s03 /repo scenarios/sentinel-03/*.json
camp pow pow /repo scenarios/sentinel-02-power/*.json
# Generated scenarios for the I2C groups no committed scenario covers (3305's): baseline and held-low SDA.
sc() { # file name base expect findings overrides
  printf '{"schema_version":1,"name":"%s","base":"%s","expect":"%s","expect_findings":%s,"overrides":%s}\n' "$2" "$3" "$4" "$5" "$6" > $OUT/sc/$1
}
cp examples/sentinel-01-ext-bmp581.json $OUT/sc/ext-base.json
cp crates/aethelgard-cli/tests/fixtures/two-bme.json $OUT/sc/two-base.json
cp /repo/spike/3307/spec/bme-bmp.json $OUT/sc/bmp-base.json
sc ext-baseline.json ext-baseline ext-base.json pass '[]' '[]'
sc ext-held-low-sda.json ext-held-low-sda ext-base.json fail '["i2c_line_held_low","i2c_timeout"]' '[{"bus":"EXT","line_held_low":"sda"}]'
for g in two bmp; do
  sc $g-baseline.json $g-baseline $g-base.json pass '[]' '[]'
  sc $g-held-low-sda.json $g-held-low-sda $g-base.json fail '["i2c_line_held_low","i2c_timeout"]' '[{"bus":"SENSORS","line_held_low":"sda"}]'
  # A mutant's run expects a pass, so the run archives whatever the judge finds (3305's collide.sh).
  sc $g-mutant.json $g-mutant $g-base.json pass '[]' '[]'
done
camp ext ext $OUT/sc ext-baseline.json ext-held-low-sda.json
camp two two $OUT/sc two-baseline.json two-held-low-sda.json
# The two-device cases: U2's step sent to 0x75 (no device: 3305's wrong address) and to 0x77 (U5, a
# BME280, answers: F-COLLIDE); on bme-bmp, U2's step sent to 0x47, where U5 (a BMP581) answers.
build two-0x75 crates/aethelgard-cli/tests/fixtures/two-bme.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x75,'
build two-0x77 crates/aethelgard-cli/tests/fixtures/two-bme.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x77,'
build bmp spike/3307/spec/bme-bmp.json
build bmp-0x47 spike/3307/spec/bme-bmp.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x47,'
camp two-0x75 two-0x75 $OUT/sc two-mutant.json
camp two-0x77 two-0x77 $OUT/sc two-mutant.json
camp bmp bmp $OUT/sc bmp-baseline.json bmp-held-low-sda.json
camp bmp-0x47 bmp-0x47 $OUT/sc bmp-mutant.json
# AET-WO-3307: ext-cfg37, U5's configure write sent to register 0x37 (a BMP581 witness of its own wrong
# transaction, not committed). sentinel-02-own is not here: it declares a foreign clock, so no generated
# firmware runs on it; sentinel-03 (U7) is the committed BMP581 under a fault.
sc ext-mutant.json ext-mutant ext-base.json pass '[]' '[]'
build ext-cfg37 examples/sentinel-01-ext-bmp581.json firmware/src/main.rs '&[0x36, 0x40, 0x81]' '&[0x37, 0x40, 0x81]'
camp ext-cfg37 ext-cfg37 $OUT/sc ext-mutant.json
# The seeded set's mutants (scenarios/sentinel-01/seeded/cases.json), each over its own cases'
# scenarios, as attribute_seeded's tier 2 runs them (the unmutated cases are the min group's).
python3 -I -c "
import json
m = json.load(open('/repo/scenarios/sentinel-01/seeded/cases.json'))
by = {}
for c in m['cases']:
    if c['mutant']:
        by.setdefault(c['mutant'], set()).add(c['scenario'])
for k, scs in by.items():
    print(k, m['mutants'][k]['spec'], ' '.join(sorted(scs)))
" > $OUT/seeded.plan
# The patch is read from the manifest by name (one mutant's text spans lines), applied exactly once.
while read -r k spec scs; do
  if [ ! -f $OUT/fw/seeded-$k.elf ]; then
    G=$OUT/fw/gen-seeded-$k; rm -rf $G
    $A generate --spec /repo/$spec --out $G > $OUT/fw/seeded-$k.gen.out 2>&1 || { echo "generate $k failed"; continue; }
    python3 -I -c "
import json,sys
k,g=sys.argv[1:3]; v=json.load(open('/repo/scenarios/sentinel-01/seeded/cases.json'))['mutants'][k]
p=g+'/board/'+v['file']; s=open(p).read(); assert s.count(v['find'])==1,(k,p)
open(p,'w').write(s.replace(v['find'],v['replace']))" $k $G || { echo "patch $k failed"; continue; }
    $A build --crate $G/board --out $OUT/fw/seeded-$k.elf > $OUT/fw/seeded-$k.build.out 2>&1; echo "built seeded-$k rc=$?"
  fi
  camp seeded-$k seeded-$k /repo $scs
done < $OUT/seeded.plan
echo done
