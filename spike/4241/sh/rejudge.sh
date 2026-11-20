#!/bin/bash
# Inside the image (MEM=16G). AET-WO-4241 §1.4 re-judge: per group, main's CLI generates/builds its ELF and runs every
# committed scenario (campaign + replay + attribute); the branch's CLI does the same with its own ELF. Then simulate per
# group spec with the instrumented branch CLI (RAWLOG) and main's instrumented CLI for modelled STEP instants.
set +e
D=/cache/rj; MB=/cache/target-main/debug; BB=/cache/target-br/debug; IB=/cache/target-inst2/debug; IM=/cache/target-inst/debug
if [ -z "${SKIP_BUILD:-}" ]; then
( cd /cache/br-src && CARGO_TARGET_DIR=/cache/target-br cargo build -q --locked -p aethelgard-cli ) || { echo "br build failed"; exit 9; }
( cd /cache/inst2-src && CARGO_TARGET_DIR=/cache/target-inst2 cargo build -q --locked -p aethelgard-cli ) || { echo "inst2 build failed"; exit 9; }
( cd /cache/main-inst-src && CARGO_TARGET_DIR=/cache/target-main-inst cargo build -q --locked -p aethelgard-cli ) || { echo "main-inst build failed"; exit 9; }
fi
for t in target-br target-inst2 target-main-inst; do cp $MB/aethelgard-bridge /cache/$t/debug/; done
declare -A SPEC=( [min]=examples/sentinel-01-minimal.json [s01]=examples/sentinel-01.json [dshot]=crates/aethelgard-cli/tests/fixtures/motors-dshot600.json [pwm]=crates/aethelgard-cli/tests/fixtures/motors-pwm.json [serial]=crates/aethelgard-cli/tests/fixtures/serial-four.json [ext]=examples/sentinel-01-ext-bmp581.json [two]=crates/aethelgard-cli/tests/fixtures/two-bme.json [s03]=crates/aethelgard-cli/tests/fixtures/sentinel-03.json [pow]=crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json [own]=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json [shaped]=crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json [spionly]=crates/aethelgard-cli/tests/fixtures/spi-only.json [imu]=crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json )
cd /repo
SC_min="$(ls scenarios/sentinel-01/*.json scenarios/sentinel-01/seeded/*.json 2>/dev/null | grep -v -e motors-pwm -e cases.json | tr '\n' ' ')"
SC_s01="$(ls scenarios/sentinel-01-bench/*.json | tr '\n' ' ')"
SC_dshot="$(ls scenarios/motors/*.json | tr '\n' ' ')"
SC_pwm="$(ls scenarios/sentinel-01/seeded/motors-pwm.json 2>/dev/null)"
SC_serial="$(ls scenarios/serial/*.json | tr '\n' ' ')"
SC_s03="$(ls scenarios/sentinel-03/*.json | tr '\n' ' ')"
SC_pow="$(ls scenarios/sentinel-02-power/*.json | tr '\n' ' ')"
mkdir -p $D/sc
for g in ext two own shaped spionly imu; do
  cp /repo/${SPEC[$g]} $D/sc/$g-base.json
  printf '{"schema_version":1,"name":"%s-baseline","base":"%s-base.json","expect":"pass","expect_findings":[],"overrides":[]}\n' $g $g > $D/sc/$g-baseline.json
done
for g in ext two; do
  printf '{"schema_version":1,"name":"%s-held-low-sda","base":"%s-base.json","expect":"fail","expect_findings":["i2c_line_held_low","i2c_timeout"],"overrides":[{"bus":"%s","line_held_low":"sda"}]}\n' $g $g $( [ $g = ext ] && echo EXT || echo SENSORS ) > $D/sc/$g-held-low-sda.json
done
SC_ext="ext-baseline.json ext-held-low-sda.json"; SC_two="two-baseline.json two-held-low-sda.json"
SC_own="own-baseline.json"; SC_shaped="shaped-baseline.json"; SC_spionly="spionly-baseline.json"; SC_imu="imu-baseline.json"
mkdir -p $D/fw
for g in ${GROUPS_TO_RUN:-min s01 dshot pwm serial ext two s03 pow own shaped spionly imu}; do
  for v in main branch; do
    [ $v = main ] && B=$MB || B=$BB
    if [ ! -f $D/fw/$g-$v.elf ]; then rm -rf $D/fw/gen-$g-$v
      $B/aethelgard generate --spec /repo/${SPEC[$g]} --out $D/fw/gen-$g-$v > $D/fw/$g-$v.gen.out 2>&1
      $B/aethelgard build --crate $D/fw/gen-$g-$v/board --out $D/fw/$g-$v.elf > $D/fw/$g-$v.build.out 2>&1; echo "built $g $v rc=$?"; fi
    # Instants: simulate with the instrumented CLI of the same side.
    [ $v = main ] && I=/cache/target-main-inst/debug || I=$IB
    repl=$(ls $D/fw/gen-$g-$v/board/*.repl | head -1); O=$D/sim/$v/$g; rm -rf $O; mkdir -p $O
    PATH=$I:$PATH $I/aethelgard simulate --platform $repl --firmware $D/fw/$g-$v.elf --spec /repo/${SPEC[$g]} --for 100ms > $O/simulate.json 2> $O/simulate.err; echo $? > $O/simulate.rc
    grep '^RAWLOG ' $O/simulate.err | sed 's/^RAWLOG //' > $O/boot.log; rm -f $O/simulate.err
    eval SCS=\$SC_$g
    [ -n "$SCS" ] || continue
    case $g in ext|two|own|shaped|spionly|imu) cd $D/sc;; *) cd /repo;; esac
    O=$D/camp/$v/$g; rm -rf $O; mkdir -p $O
    PATH=$B:$PATH $B/aethelgard campaign --scenarios $SCS --firmware $D/fw/$g-$v.elf --archive $O/archive > $O/campaign.out 2> $O/campaign.err; echo $? > $O/campaign.rc
    for r in $O/archive/*/; do n=$(basename $r); $B/aethelgard replay --archive $r > $O/replay-$n.out 2> $O/replay-$n.err; $B/aethelgard attribute --archive $r > $O/attribute-$n.out 2>&1; done
    echo "campaign $g $v rc=$(cat $O/campaign.rc) sim=$(cat $D/sim/$v/$g/simulate.rc)"
    cd /repo
  done
done
