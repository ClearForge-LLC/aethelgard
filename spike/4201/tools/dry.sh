#!/bin/bash
# usage: dry.sh <aethelgard binary> <spec> <outdir>: validate, solve, emit; one line each.
set -u
AE=$1; SPEC=$2; OUT=$3; mkdir -p "$OUT"
for step in validate solve emit; do
  case $step in
    validate) "$AE" validate < "$SPEC" > "$OUT/validate.json" 2>"$OUT/validate.err"; rc=$?;;
    solve) "$AE" solve --file "$SPEC" > "$OUT/solve.json" 2>"$OUT/solve.err"; rc=$?;;
    emit) "$AE" emit --file "$SPEC" --out "$OUT/emit" > "$OUT/emit.json" 2>"$OUT/emit.err"; rc=$?;;
  esac
  printf '%-8s rc=%s %s\n' "$step" "$rc" "$(head -c 400 "$OUT/$step.json")"
  [ "$rc" -eq 0 ] || break
done
