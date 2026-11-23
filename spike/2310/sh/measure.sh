#!/bin/bash
# AET-WO-2310 §1.1 sweeps B and M, inside the tools image (img.sh, REPO = a `git archive` of main 15798f91).
# 3306's groups unchanged (every committed scenario group's campaign, the seeded set's mutants, and the
# two-device I2C cases), each run ONCE by main's CLI ($BIN), archived under $OUT. Each archive is then
# replayed on the host by main's CLI and by the branch's (sh/replay.sh): this order changes no twin,
# trace or firmware.
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

sc ext-baseline.json ext-baseline ext-base.json pass '[]' '[]'
sc ext-held-low-sda.json ext-held-low-sda ext-base.json fail '["i2c_line_held_low","i2c_timeout"]' '[{"bus":"EXT","line_held_low":"sda"}]'
for g in two; do
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
camp two-0x75 two-0x75 $OUT/sc two-mutant.json
camp two-0x77 two-0x77 $OUT/sc two-mutant.json
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

# Sweep M (item 3): each planted scenario's base, booted undisturbed by simulate for 100 ms, twice; the boot
# word's instant is the verdict's boot.boot_word_ns. The platform is the one generate emitted for the spec.
sim() { # label spec
  local l=$1 s=$2 G=$OUT/fw/gen-$1 r
  for r in 1 2; do
    $A simulate --platform $(ls $G/board/*.repl | head -1) --firmware $OUT/fw/$l.elf --spec /repo/$s --for 100ms \
      > $OUT/sim-$l-$r.json 2> $OUT/sim-$l-$r.err; echo "simulate $l run $r rc=$?"
  done
}
sim min examples/sentinel-01-minimal.json
sim s01 examples/sentinel-01.json
sim dshot crates/aethelgard-cli/tests/fixtures/motors-dshot600.json
sim s03 crates/aethelgard-cli/tests/fixtures/sentinel-03.json
sim pow crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json
echo done
