#!/bin/bash
# AET-WO-4400f, inside the tools image: build the CLI in /repo (target $CARGO_TARGET_DIR), then for
# each board solve, generate and build its firmware ONCE (/cache/fw/<board>/fw.elf, reused by every
# candidate), emit its platform with THIS CLI into /cache/fw-$TAG/<board>/gen, and run:
#   simulate --firmware fw.elf --for 100ms, and on ArduPilot simulate --foreign W for each W in
#   $FOREIGN, each REPS times (the baseline ran twice: the noise floor), and ci/foreign-judge.sh once
#   (NOJUDGE=1 skips it). The candidate's knobs are the environment's AET4400F_* (the spike patch).
# Wall time and the exit code per run go to /cache/runs/$TAG/runs.tsv.
set -u
TAG=$1; FOREIGN=${FOREIGN:-4000ms}; BOARDS=${BOARDS:-"sentinel-02-own serial-dshot-9600 motors-dshot600"}
cd /repo
cargo build -q --locked --offline -p aethelgard-cli || exit 1
cargo build -q --locked --offline --manifest-path bridge/Cargo.toml || exit 1
A=$CARGO_TARGET_DIR/debug/aethelgard; O=/cache/runs/$TAG; mkdir -p $O
t() { local name=$1; shift; local t0=$(date +%s.%N); "$@" > $O/$name.json 2> $O/$name.err; local st=$?
      echo -e "$name\t$st\t$(python3 -c "print('%.2f' % ($(date +%s.%N)-$t0))")" >> $O/runs.tsv; }
for b in $BOARDS; do
  F=/repo/crates/aethelgard-cli/tests/fixtures/$b.json
  fw=/cache/fw/$b; mkdir -p $fw
  if [ ! -f $fw/fw.elf ]; then
    $A generate --spec $F --out $fw/gen > $fw/generate.json 2>&1 || { echo "generate $b failed"; exit 1; }
    $A build --crate $fw/gen/board --out $fw/fw.elf > $fw/build.json 2>&1 || { echo "build $b failed"; exit 1; }
  fi
  g=/cache/fw-$TAG/$b; rm -rf $g; mkdir -p $g
  $A generate --spec $F --out $g/gen > $g/generate.json 2>&1 || { echo "generate($TAG) $b failed"; exit 1; }
  R=$(ls $g/gen/board/*.repl)
  for i in $(seq 1 ${REPS:-1}); do t gen-$b-$i $A simulate --platform $R --firmware $fw/fw.elf --spec $F --for 100ms; done
done
if [ -z "${NOFOREIGN:-}" ]; then
  F=/repo/crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
  R=/cache/fw-$TAG/sentinel-02-own/gen/board/sentinel-02-own.repl
  for w in $FOREIGN; do for i in $(seq 1 ${REPS:-1}); do
    t foreign-$w-$i $A simulate --platform $R --firmware /cache/boards/sentinel-02/arducopter.elf --spec $F --foreign $w; done; done
  [ -n "${NOJUDGE:-}" ] && { cat $O/runs.tsv; exit 0; }
  t0=$(date +%s.%N); bash ci/foreign-judge.sh /cache/boards > $O/foreign-judge.log 2>&1; st=$?
  echo -e "foreign-judge\t$st\t$(python3 -c "print('%.2f' % ($(date +%s.%N)-$t0))")" >> $O/runs.tsv
fi
cat $O/runs.tsv
