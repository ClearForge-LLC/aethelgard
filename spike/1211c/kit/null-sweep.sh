#!/bin/bash
# The null sweep (§2), inside the tools image (img.sh, 8G): with the CLI at $BIN, for every committed
# specification: generate (each file's sha256 kept), and where the tree holds a platform, build and
# simulate --firmware for 100 ms (and solve's bytes, AET-WO-1211c D7); then every committed scenario's campaign, on its base's ELF. Out:
# /cache/sweep/$TAG/. compare.py-style judging is null-compare.py. usage: null-sweep.sh TAG
set -u
TAG=$1; A=${BIN:-/cache/main-7fc81640-aethelgard}; O=/cache/sweep/$TAG; rm -rf "$O"; mkdir -p "$O"
cd /repo
python3 -I -c "
import json,glob
for p in sorted(glob.glob('**/*.json',recursive=True)):
    if p.startswith(('spike/','target/','fab/','docs/')) or '/target/' in p: continue
    try: d=json.load(open(p))
    except Exception: continue
    if not isinstance(d,dict): continue
    if isinstance(d.get('mcu'),dict) and 'schema_version' in d: print('spec', p)
    elif {'base','overrides','schema_version'} <= set(d): print('scenario', p)" > "$O/inputs.txt"
key() { echo "$1" | tr '/' '_' | sed 's/\.json$//'; }
while read -r t p; do
  [ "$t" = spec ] || continue
  d=$O/spec/$(key "$p"); mkdir -p "$d"
  $A solve --file "$p" > "$d/solve.json" 2>&1
  $A generate --spec "$p" --out "$d/gen" > "$d/generate.json" 2>&1; g=$?
  [ $g -eq 0 ] && (cd "$d/gen" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$d/tree.sha256"
  repl=$(ls "$d"/gen/board/*.repl 2>/dev/null | head -1)
  s=-
  if [ -n "$repl" ]; then
    $A build --crate "$d/gen/board" --out "$d/fw.elf" > "$d/build.json" 2>&1
    $A simulate --platform "$repl" --firmware "$d/fw.elf" --spec "$p" --for 100ms > "$d/simulate.json" 2>&1; s=$?
  fi
  echo "spec $p generate=$g simulate=$s" | tee -a "$O/rows.txt"
done < "$O/inputs.txt"
while read -r t p; do
  [ "$t" = scenario ] || continue
  base=$(python3 -I -c "import json,os,sys;print(os.path.normpath(os.path.join(os.path.dirname(sys.argv[1]),json.load(open(sys.argv[1]))['base'])))" "$p")
  elf=$O/spec/$(key "$base")/fw.elf; d=$O/scenario/$(key "$p"); mkdir -p "$d"
  if [ ! -f "$elf" ]; then echo "scenario $p no-elf base=$base" | tee -a "$O/rows.txt"; continue; fi
  $A campaign --scenarios "$p" --firmware "$elf" --archive "$d/archive" > "$d/campaign.jsonl" 2> "$d/campaign.err"
  echo "scenario $p campaign=$?" | tee -a "$O/rows.txt"
done < "$O/inputs.txt"
