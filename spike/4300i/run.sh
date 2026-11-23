#!/usr/bin/env bash
# AET-WO-4300i R1: validate, solve, emit (each twice) every committed board.
# usage: run.sh <aethelgard binary> <scratch dir>
set -uo pipefail
AE=$1; W=$2
here=$(cd "$(dirname "$0")" && pwd); root=$(git -C "$here" rev-parse --show-toplevel)
mkdir -p "$here/out/emit-files" "$W/solve" "$W/emit" "$W/val"
printf 'name\tpath\tvalidate_rc\tvalidate_kind\tsolve_rc\tsolve_kind\tsolve_sha256\temit_rc\temit_kind\temit_files\temit_tree_sha256\trepeat_identical\n' > "$here/out/baseline.tsv"
kind() { python3 -I -c 'import json,sys
try: print(json.load(sys.stdin).get("error",{}).get("kind","?"))
except Exception: print("?")' 2>/dev/null; }
bash "$here/universe.sh" | while IFS=$'\t' read -r n f; do
  spec="$root/$f"; same=yes
  # validate x2 (stdout+stderr combined)
  v1=$("$AE" validate --file "$spec" 2>&1); vrc=$?; v2=$("$AE" validate --file "$spec" 2>&1); vrc2=$?
  [ "$v1" = "$v2" ] && [ $vrc -eq $vrc2 ] || same=no
  vk=-; [ $vrc -ne 0 ] && vk=$(printf '%s' "$v1" | kind)
  # solve x2 (stdout only hashed)
  s1=$("$AE" solve --file "$spec" 2>"$W/val/$n.err"); src=$?; s2=$("$AE" solve --file "$spec" 2>/dev/null); src2=$?
  [ "$s1" = "$s2" ] && [ $src -eq $src2 ] || same=no
  sk=-; [ $src -ne 0 ] && sk=$( (printf '%s' "$s1"; cat "$W/val/$n.err") | kind)
  [ $src -ne 0 ] && [ "$sk" = "?" ] && sk=$(kind < "$W/val/$n.err")
  sh=$(printf '%s' "$s1" | sha256sum | cut -d' ' -f1)
  [ $src -eq 0 ] && printf '%s' "$s1" > "$W/solve/$n.json"
  # emit x2
  erc=-; ek=-; cnt=-; tree=-
  rm -rf "$W/emit/$n.a" "$W/emit/$n.b"
  e1=$(cd "$W/emit" && nice -n 10 "$AE" emit --file "$spec" --out "$n.a" 2>&1); erc=$?
  e2=$(cd "$W/emit" && nice -n 10 "$AE" emit --file "$spec" --out "$n.b" 2>&1); erc2=$?
  [ $erc -eq $erc2 ] || same=no
  if [ $erc -ne 0 ]; then ek=$(printf '%s' "$e1" | kind); fi
  : > "$here/out/emit-files/$n.sha"
  if [ -d "$W/emit/$n.a" ]; then
    (cd "$W/emit/$n.a" && find . -type f | sed 's|^\./||' | LC_ALL=C sort | while read -r p; do printf '%s  %s\n' "$(sha256sum "$p" | cut -d' ' -f1)" "$p"; done) > "$here/out/emit-files/$n.sha"
    cnt=$(wc -l < "$here/out/emit-files/$n.sha"); tree=$(sha256sum < "$here/out/emit-files/$n.sha" | cut -d' ' -f1)
    if [ -d "$W/emit/$n.b" ]; then
      (cd "$W/emit/$n.b" && find . -type f | sed 's|^\./||' | LC_ALL=C sort | while read -r p; do printf '%s  %s\n' "$(sha256sum "$p" | cut -d' ' -f1)" "$p"; done) | cmp -s - "$here/out/emit-files/$n.sha" || same=no
    else same=no; fi
    [ -d "$W/emit/$n.a" ] && mv "$W/emit/$n.a" "$W/emit/$n"
    rm -rf "$W/emit/$n.b"
  else
    cnt=0; tree=$(sha256sum < /dev/null | cut -d' ' -f1)
  fi
  [ $erc -eq 0 ] || [ $erc -ne 0 ] ; 
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$n" "$f" "$vrc" "$vk" "$src" "$sk" "$sh" "$erc" "$ek" "$cnt" "$tree" "$same" >> "$here/out/baseline.tsv"
done
