#!/bin/bash
# AET-WO-2310 build: the undisturbed boot word of each planted test fixture's base (the five scenario fixtures
# under crates/aethelgard-cli/tests/fixtures, and board_tier2's driver), by main's CLI in the tools image, twice,
# as sweep M measured the committed scenarios' (spike/2310/results.md). REPO = a git archive of main.
set +e
BIN=${BIN:-/cache/target-main/debug}; OUT=${OUT:-/cache/fixtures}; A=$BIN/aethelgard
export PATH=$BIN:$PATH
cd /repo
mkdir -p $OUT
one() { # label spec
  local l=$1 s=$2 G=$OUT/gen-$1
  if [ ! -f $OUT/$l.elf ]; then
    rm -rf $G; $A generate --spec /repo/$s --out $G > $OUT/$l.gen.out 2>&1 || { echo "generate $l failed"; return 1; }
    $A build --crate $G/board --out $OUT/$l.elf > $OUT/$l.build.out 2>&1 || { echo "build $l failed"; return 1; }
  fi
  for r in 1 2; do
    $A simulate --platform $(ls $G/board/*.repl | head -1) --firmware $OUT/$l.elf --spec /repo/$s --for 100ms > $OUT/sim-$l-$r.json 2> $OUT/sim-$l-$r.err
    echo "$l run $r rc=$? boot_word_ns=$(python3 -I -c "import json,sys;print(json.load(open(sys.argv[1]))['boot']['boot_word_ns'])" $OUT/sim-$l-$r.json 2>&1)"
  done
}
one ext examples/sentinel-01-ext-bmp581.json
one logger crates/aethelgard-cli/tests/fixtures/logger-4600.json
one s02shaped crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json
one fourlayer crates/aethelgard-cli/tests/fixtures/four-layer.json
one spionly crates/aethelgard-cli/tests/fixtures/spi-only.json
one driver crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json
echo done
