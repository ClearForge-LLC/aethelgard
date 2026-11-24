#!/bin/bash
# AET-WO-3309 §1.1, the committed tier-2 campaigns outside scenarios/ whose I2C devices have more than
# one step, or whose attribution a test pins (twin_tier2.rs): the BOR recovery fixtures (logger-4600 and
# sentinel-02-shaped carry a BMP581, ext-bmp581 its U5), imu-baro-1v8 (an ICP10111 on I2C) with the
# baro-pressure-first mutant, and 3306's committed collide (two-bme-collide.json, U2's step to 0x77).
# Same image, same $BIN and $OUT as measure.sh, whose build/camp functions it repeats.
set +e
BIN=${BIN:-/cache/target-main/debug}; OUT=${OUT:-/cache/runs-main}; A=$BIN/aethelgard
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
sc() { # file name base expect findings overrides
  printf '{"schema_version":1,"name":"%s","base":"%s","expect":"%s","expect_findings":%s,"overrides":%s}\n' "$2" "$3" "$4" "$5" "$6" > $OUT/sc/$1
}
F=crates/aethelgard-cli/tests/fixtures
build logger $F/logger-4600.json
build s02s $F/sentinel-02-shaped.json
build ext examples/sentinel-01-ext-bmp581.json
build baro $F/imu-baro-1v8.json
build baro-pf $F/imu-baro-1v8.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x63, &[0x78, 0x66], &mut buf7)' 'i2c1.transfer(&mut r, 0x63, &[0x48, 0xa3], &mut buf7)'
build two-0x77 $F/two-bme.json firmware/src/main.rs 'i2c1.transfer(&mut r, 0x76,' 'i2c1.transfer(&mut r, 0x77,'
cp $F/logger-4600.json $OUT/sc/logger-base.json
cp $F/sentinel-02-shaped.json $OUT/sc/s02s-base.json
cp $F/imu-baro-1v8.json $OUT/sc/baro-base.json
for g in logger s02s baro; do sc $g-baseline.json $g-baseline $g-base.json pass '[]' '[]'; done
sc baro-mutant.json baro-mutant baro-base.json pass '[]' '[]'
# A base must sit under the campaign's directory: the generated baselines run from $OUT/sc, the
# committed fixtures from /repo.
camp t2-logger logger $OUT/sc logger-baseline.json
camp t2-logger-bor logger /repo $F/bor-recovery/logger-droop-bor3.json
camp t2-s02s s02s $OUT/sc s02s-baseline.json
camp t2-s02s-bor s02s /repo $F/bor-recovery/s02-shaped-droop.json
camp t2-ext-bor ext /repo $F/bor-recovery/ext-bmp581-droop-bor3.json
camp t2-baro baro $OUT/sc baro-baseline.json
camp t2-baro-pf baro-pf $OUT/sc baro-mutant.json
camp t2-collide two-0x77 /repo $F/two-bme-collide.json
echo done
