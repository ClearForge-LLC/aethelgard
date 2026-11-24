#!/bin/bash
# AET-WO-3309 §1.1 (3307's measure.sh), inside the tools image (img.sh, REPO = a `git archive` of the CLI's
# source, BIN = that CLI): every committed scenario, grouped by the spec it names (`base`), the seeded
# set's mutants, and 3306/3307's two-device and ext-cfg37 cases, each campaign run by $BIN and archived
# under $OUT. The count rule runs at judge time (renode.rs `attribute`), so a branch's placement needs
# its own runs: run this once per CLI. CONTROL=1 also re-runs two groups to measure run-to-run noise.
# The spike specs and the seeded manifest are read from $SPIKE (the worktree, mounted at /spike).
set +e
BIN=${BIN:-/cache/target-main/debug}; OUT=${OUT:-/cache/runs-main}; A=$BIN/aethelgard
SPIKE=${SPIKE:-/spike}
export PATH=$BIN:$PATH
cd /repo
mkdir -p $OUT/fw $OUT/sc $OUT/camp
build() { # label spec [file find replace]
  local l=$1 s=$2 G=$OUT/fw/gen-$1
  [ -f $OUT/fw/$l.elf ] && return 0
  rm -rf $G; $A generate --spec $s --out $G > $OUT/fw/$l.gen.out 2>&1 || { echo "generate $l failed"; return 1; }
  if [ $# -gt 2 ]; then
    python3 -I -c "import sys;p,o,n=sys.argv[1:4];s=open(p).read();assert s.count(o)==1,(p,o);open(p,'w').write(s.replace(o,n))" "$G/board/$3" "$4" "$5" || { echo "patch $l failed"; return 1; }
  fi
  $A build --crate $G/board --out $OUT/fw/$l.elf > $OUT/fw/$l.build.out 2>&1; echo "built $l rc=$?"
}
camp() { # label elf cwd scenarios...
  local l=$1 e=$2 c=$3; shift 3
  local O=$OUT/camp/$l; rm -rf $O; mkdir -p $O
  (cd $c && $A campaign --scenarios "$@" --firmware $OUT/fw/$e.elf --archive $O/archive > $O/campaign.out 2> $O/campaign.err; echo $? > $O/campaign.rc)
  echo "campaign $l rc=$(cat $O/campaign.rc) $(date -u +%H:%M:%S)"
}
# Every committed scenario, grouped by the spec its `base` names (resolved from the scenario's own
# directory); cases.json is the seeded manifest, not a scenario.
python3 -I -c "
import json, os, glob
by = {}
for f in sorted(glob.glob('scenarios/**/*.json', recursive=True)):
    if f.endswith('cases.json'):
        continue
    b = os.path.normpath(os.path.join(os.path.dirname(f), json.load(open(f))['base']))
    by.setdefault(b, []).append(f)
for i, (b, fs) in enumerate(sorted(by.items())):
    print('c%d' % i, b, ' '.join(fs))
" > $OUT/committed.plan
cat $OUT/committed.plan | cut -c1-160
while read -r l spec scs; do
  build $l $spec && camp $l $l /repo $scs
done < $OUT/committed.plan
# Generated scenarios for the I2C groups no committed scenario covers (3305's): baseline and held-low SDA.
sc() { # file name base expect findings overrides
  printf '{"schema_version":1,"name":"%s","base":"%s","expect":"%s","expect_findings":%s,"overrides":%s}\n' "$2" "$3" "$4" "$5" "$6" > $OUT/sc/$1
}
cp examples/sentinel-01-ext-bmp581.json $OUT/sc/ext-base.json
cp crates/aethelgard-cli/tests/fixtures/two-bme.json $OUT/sc/two-base.json
cp $SPIKE/spec/bme-bmp.json $OUT/sc/bmp-base.json
build ext examples/sentinel-01-ext-bmp581.json
build two crates/aethelgard-cli/tests/fixtures/two-bme.json
sc ext-baseline.json ext-baseline ext-base.json pass '[]' '[]'
sc ext-held-low-sda.json ext-held-low-sda ext-base.json fail '["i2c_line_held_low","i2c_timeout"]' '[{"bus":"EXT","line_held_low":"sda"}]'
for g in two bmp; do
  sc $g-baseline.json $g-baseline $g-base.json pass '[]' '[]'
  sc $g-held-low-sda.json $g-held-low-sda $g-base.json fail '["i2c_line_held_low","i2c_timeout"]' '[{"bus":"SENSORS","line_held_low":"sda"}]'
  # A mutant's run expects a pass, so the run archives whatever the judge finds (3305's collide.sh).
  sc $g-mutant.json $g-mutant $g-base.json pass '[]' '[]'
done
sc ext-mutant.json ext-mutant ext-base.json pass '[]' '[]'
camp ext ext $OUT/sc ext-baseline.json ext-held-low-sda.json
camp two two $OUT/sc two-baseline.json two-held-low-sda.json
# 3305/3306/3307's two-device cases: U2's step sent to 0x75 (no device), to 0x77 (U5, a BME280,
# answers), and on bme-bmp to 0x47 (U5, a BMP581, answers); ext-cfg37, U5's configure write sent to
# register 0x37 (3307 F2).
build two-0x75 crates/aethelgard-cli/tests/fixtures/two-bme.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x75,'
build two-0x77 crates/aethelgard-cli/tests/fixtures/two-bme.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x77,'
build bmp $SPIKE/spec/bme-bmp.json
build bmp-0x47 $SPIKE/spec/bme-bmp.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x47,'
build ext-cfg37 examples/sentinel-01-ext-bmp581.json firmware/src/main.rs '&[0x36, 0x40, 0x81]' '&[0x37, 0x40, 0x81]'
camp two-0x75 two-0x75 $OUT/sc two-mutant.json
camp two-0x77 two-0x77 $OUT/sc two-mutant.json
camp bmp bmp $OUT/sc bmp-baseline.json bmp-held-low-sda.json
camp bmp-0x47 bmp-0x47 $OUT/sc bmp-mutant.json
camp ext-cfg37 ext-cfg37 $OUT/sc ext-mutant.json
# The seeded set's mutants (scenarios/sentinel-01/seeded/cases.json), each over its own cases'
# scenarios, as attribute_seeded's tier 2 runs them (the unmutated cases are in the committed groups).
python3 -I -c "
import json
m = json.load(open('scenarios/sentinel-01/seeded/cases.json'))
by = {}
for c in m['cases']:
    if c['mutant']:
        by.setdefault(c['mutant'], set()).add(c['scenario'])
for k, scs in by.items():
    print(k, m['mutants'][k]['spec'], ' '.join(sorted(scs)))
" > $OUT/seeded.plan
while read -r k spec scs; do
  if [ ! -f $OUT/fw/seeded-$k.elf ]; then
    G=$OUT/fw/gen-seeded-$k; rm -rf $G
    $A generate --spec $spec --out $G > $OUT/fw/seeded-$k.gen.out 2>&1 || { echo "generate $k failed"; continue; }
    python3 -I -c "
import json,sys
k,g=sys.argv[1:3]; v=json.load(open('scenarios/sentinel-01/seeded/cases.json'))['mutants'][k]
p=g+'/board/'+v['file']; s=open(p).read(); assert s.count(v['find'])==1,(k,p)
open(p,'w').write(s.replace(v['find'],v['replace']))" $k $G || { echo "patch $k failed"; continue; }
    $A build --crate $G/board --out $OUT/fw/seeded-$k.elf > $OUT/fw/seeded-$k.build.out 2>&1; echo "built seeded-$k rc=$?"
  fi
  camp seeded-$k seeded-$k /repo $scs
done < $OUT/seeded.plan
# Control: the same firmware and scenarios again, to tell run-to-run noise from a placement's change.
if [ "${CONTROL:-0}" = 1 ]; then
  camp ctl-ext-cfg37 ext-cfg37 $OUT/sc ext-mutant.json
  s03=$(awk '$2 ~ /sentinel-03/ {print $1}' $OUT/committed.plan)
  camp ctl-$s03 $s03 /repo scenarios/sentinel-03/baseline.json scenarios/sentinel-03/held-low-sda.json
fi
echo done
