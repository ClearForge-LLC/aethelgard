#!/bin/bash
# M2 and M3, inside the tools image (img.sh): validate, solve, generate, build and simulate D1, D2 and
# D2g with the CLI at $A (main's, built at the branch's base); then verify on D1 (refused, not drawn),
# and verify --fab-out on D2g with its baseline (coverage, before any gate), and campaign on D2g's
# VMOTOR droop and rail_out_of_band. Rows to /cache/m/today.tsv; every output kept in /cache/m/<name>/.
set -u
A=${A:-/cache/target/debug/aethelgard}; M=/cache/m; cd "$M"
: > today.tsv
row() { printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$(python3 -I -c 'import json,sys
try:
    v=json.loads(open(sys.argv[1]).read().strip().splitlines()[-1]); e=v.get("error") or {}
    print(e.get("kind") or v.get("kind") or v.get("verdict"), e.get("id") or v.get("id") or "")
except Exception as x: print("unparsed", x)' "$4")" | tee -a today.tsv; }
for s in d1 d2 d2g; do
  d=$M/$s; rm -rf "$d"; mkdir -p "$d"
  $A validate --file $s.json > $d/validate.json 2>&1; row $s validate $? $d/validate.json
  $A solve --file $s.json > $d/solve.json 2>&1; row $s solve $? $d/solve.json
  $A generate --spec $s.json --out $d/gen > $d/generate.json 2>&1; g=$?; row $s generate $g $d/generate.json
  [ $g -eq 0 ] && { echo "repl: $(ls $d/gen/board/*.repl 2>/dev/null | wc -l)" >> today.tsv; }
  if [ $g -eq 0 ]; then
    $A build --crate $d/gen/board --out $d/fw.elf > $d/build.json 2>&1; row $s build $? $d/build.json
    [ -f $d/fw.elf ] && echo "elf $s $(sha256sum $d/fw.elf | cut -c1-64)" >> today.tsv
  fi
  repl=$(ls $d/gen/board/*.repl 2>/dev/null | head -1); repl=${repl:-$M/none.repl}
  elf=$d/fw.elf; [ -f $elf ] || elf=$M/none.elf
  $A simulate --platform $repl --firmware $elf --spec $s.json --for 100ms > $d/simulate.json 2>&1; row $s simulate $? $d/simulate.json
done
$A verify --spec d1.json > d1/verify.json 2>&1; row d1 verify $? d1/verify.json
rm -rf d2g/fab; $A verify --spec d2g.json --scenarios sc/d2g-baseline.json --fab-out d2g/fab > d2g/verify-fab.json 2>&1
row d2g verify-fab-out $? d2g/verify-fab.json
for sc in droop-vmotor vmotor-out-of-band; do
  rm -rf d2g/arch-$sc; $A campaign --scenarios sc/d2g-$sc.json --firmware d2g/fw.elf --archive d2g/arch-$sc > d2g/campaign-$sc.json 2>&1
  row d2g "campaign:$sc" $? d2g/campaign-$sc.json
done
