#!/bin/bash
# Inside the image (img.sh). AET-WO-4241 §1.1 runs: main's CLI (old generator, old judge) and the instrumented
# branch tree (new generator + main's judge + RAWLOG/RAWWORD + AET4241_UNMODEL lever). See spike/4241/PREDICTIONS.md.
set -u
M=/cache/target-main/debug; I=/cache/target-inst/debug; O=/cache/m; mkdir -p $O
( cd /cache/main-src && CARGO_TARGET_DIR=/cache/target-main cargo build -q --locked -p aethelgard-cli && CARGO_TARGET_DIR=/cache/target-main cargo build -q --locked --manifest-path bridge/Cargo.toml ) || { echo "main build failed"; exit 9; }
( cd /cache/inst-src && CARGO_TARGET_DIR=/cache/target-inst cargo build -q --locked -p aethelgard-cli ) || { echo "inst build failed"; exit 9; }
cp $M/aethelgard-bridge $I/
sha256sum $M/aethelgard $I/aethelgard > $O/cli.sha256
R=/cache/main-src
gen() { # gen <case> <spec> <cli-dir> <tag> [lever]
  local c=$1 spec=$2 B=$3 tag=$4 lever=${5:-}; local d=$O/$c/$tag; rm -rf $d; mkdir -p $d
  AET4241_UNMODEL=$lever $B/aethelgard generate --spec $R/$spec --out $d/gen > $d/generate.out 2>&1; echo $? > $d/generate.rc
  [ "$(cat $d/generate.rc)" = 0 ] || return 1
  $B/aethelgard build --crate $d/gen/board --out $d/fw.elf > $d/build.out 2>&1; echo $? > $d/build.rc
  sha256sum $d/fw.elf $d/gen/board/firmware/src/main.rs > $d/sha256 2>/dev/null
}
sim() { # sim <case> <spec> <tag> [lever]  -- always the instrumented judge
  local c=$1 spec=$2 tag=$3 lever=${4:-}; local d=$O/$c/$tag
  [ -f $d/fw.elf ] || return 1
  local repl=$(ls $d/gen/board/*.repl | head -1)
  AET4241_UNMODEL=$lever PATH=$I:$PATH $I/aethelgard simulate --platform $repl --firmware $d/fw.elf --spec $R/$spec --for 100ms > $d/simulate.json 2> $d/simulate.err; echo $? > $d/simulate.rc
  grep '^RAWLOG ' $d/simulate.err | sed 's/^RAWLOG //' > $d/boot.log; grep '^RAWWORD ' $d/simulate.err > $d/ram.txt
  echo "$c/$tag gen=$(cat $d/generate.rc) build=$(cat $d/build.rc) sim=$(cat $d/simulate.rc)"
}
declare -A SPEC=( [C1]=examples/sentinel-01-minimal.json [C2]=examples/sentinel-01.json [C3]=crates/aethelgard-cli/tests/fixtures/two-bme.json [C4]=examples/sentinel-01-ext-bmp581.json [C5]=crates/aethelgard-cli/tests/fixtures/sentinel-03.json [C8]=crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json [C0]=spike/4600/logger-spi-only.json [C9]=spike/4241-no-u3.json )
for c in ${CASES:-C0 C1 C2 C3 C4 C5 C8}; do
  gen $c ${SPEC[$c]} $M old; gen $c ${SPEC[$c]} $I new
  sim $c ${SPEC[$c]} old; sim $c ${SPEC[$c]} new
done
for c in ${LEVERS:-C6 C7}; do
  case $c in C6) L=W25Q64JVxIQ; S=examples/sentinel-01-minimal.json;; C7) L=BME280,W25Q64JVxIQ; S=examples/sentinel-01-minimal.json;; C10) L=W25Q64JVxIQ; S=crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json;; esac
  gen $c $S $I new $L; sim $c $S new $L
done
