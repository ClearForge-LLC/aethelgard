#!/bin/bash
# AET-WO-2302 spike driver, inside the image: m.sh <spec.json> <name> <for> ; env AET2302_* passes through.
# Generates and builds once per name, then boots with the patched CLI (spike/2302/measure.patch).
set -u
A=/cache/target-spike/debug/aethelgard
F=$1; N=$2; T=$3
O=/cache/m/$N; mkdir -p $O
if [ ! -f $O/fw.elf ]; then
  rm -rf $O/gen
  $A generate --spec $F --out $O/gen > $O/generate.json 2>&1 || { echo generate failed; head -c 800 $O/generate.json; exit 1; }
  $A build --crate $O/gen/board --out $O/fw.elf > $O/build.json 2>&1 || { echo build failed; head -c 1500 $O/build.json; exit 1; }
fi
R=$(ls $O/gen/board/*.repl | head -1)
tag=${AET2302_TAG:-run}
export AET2302_DUMP=$O/$tag.log
s=$(date +%s%N)
$A simulate --platform $R --firmware $O/fw.elf --spec $F --for $T > $O/$tag.verdict.json 2>&1
echo "exit=$? cli_wall_ms=$(( ($(date +%s%N)-s)/1000000 )) $(tr '\n' ' ' < $O/$tag.log.meta 2>/dev/null)"
python3 -I -c "import json,sys; v=json.load(open('$O/$tag.verdict.json')); print('verdict',v.get('verdict'),'findings',sorted({f['kind'] for f in v.get('findings',[])}), 'window', v.get('boot',{}).get('window'))" 2>/dev/null || head -c 600 $O/$tag.verdict.json
