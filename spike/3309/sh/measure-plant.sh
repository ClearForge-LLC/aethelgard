#!/bin/bash
# AET-WO-3309 §2 and §5, the planted firmwares (measure.sh's image, $BIN, $OUT; run after it, which
# makes $OUT/sc): each a generated board with one edit to its firmware, run once.
#   ext-zero      U5's configure write with a zero byte prepended (the acceptance of (1))
#   ext-append    the same with a zero byte appended (§5)
#   ext-idwide    U5's identity read two bytes wide (§5: the right value at another length)
#   ext-surplus   U5's configure write sent twice (the acceptance of (2): a BMP581 surplus)
#   bmp-cfg-u2    on bme-bmp, U5's (the BMP581's) configure write sent to U2's address (2): a step
#                 sent elsewhere, read in the BMP581's form
set +e
BIN=${BIN:-/cache/target-main/debug}; OUT=${OUT:-/cache/runs-main}; A=$BIN/aethelgard
SPIKE=${SPIKE:-/spike}
export PATH=$BIN:$PATH
cd /repo
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
E=examples/sentinel-01-ext-bmp581.json
M=firmware/src/main.rs
build ext-zero $E $M '&[0x36, 0x40, 0x81]' '&[0x00, 0x36, 0x40, 0x81]'
build ext-append $E $M '&[0x36, 0x40, 0x81]' '&[0x36, 0x40, 0x81, 0x00]'
build ext-idwide $E $M 'let mut buf1 = [0u8; 1];' 'let mut buf1 = [0u8; 2];'
build ext-surplus $E $M 'let mut buf2 = [0u8; 0];' 'let mut buf2 = [0u8; 0];
    let _ = i2c2.transfer(&mut r, 0x47, &[0x36, 0x40, 0x81], &mut buf2);'
build bmp-cfg-u2 $SPIKE/spec/bme-bmp.json $M 'i2c1.transfer(&mut r, 0x47, &[0x36, 0x40, 0x81]' 'i2c1.transfer(&mut r, 0x76, &[0x36, 0x40, 0x81]'
for l in ext-zero ext-append ext-idwide ext-surplus; do camp $l $l $OUT/sc ext-mutant.json; done
camp bmp-cfg-u2 bmp-cfg-u2 $OUT/sc bmp-mutant.json
echo done
