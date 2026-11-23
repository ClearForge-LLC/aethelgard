#!/usr/bin/env bash
# container/test-check-pins.sh: check-pins.sh against its fixtures (AET-WO-4401).
# Each container/fixtures/check-pins/*.Dockerfile names, in `# expect: TEXT`
# lines, the refusals it must draw: check-pins.sh must exit 1 and print exactly
# as many refusals as there are expect lines, each expect matching its own one.
# So a fixture cannot pass on some other refusal. The three real Dockerfiles
# must still pass.
#
#   bash container/test-check-pins.sh [path/to/check-pins.sh]
set -euo pipefail

check=${1:-container/check-pins.sh}
fail=0
# Every fixture runs under C and, when the host has it, en_US.UTF-8, where
# [0-9a-f] and [[:space:]] reach past ASCII (AET-WO-4402, 4407). A fixture in
# a directory of its own has its own *.sources beside it.
locales=$(locale -a 2>/dev/null || true)
utf8=$(grep -ixm1 'en_US\.utf-\?8' <<< "$locales" || echo C.UTF-8)
for loc in C "$utf8"; do
echo "under LC_ALL=$loc"
for f in container/fixtures/check-pins/*.Dockerfile container/fixtures/check-pins/*/*.Dockerfile; do
  mapfile -t want < <(sed -n 's/^# expect: //p' "$f")
  [ "${#want[@]}" -gt 0 ] || { echo "FAIL $f: no expect line"; fail=1; continue; }
  set +e
  out=$(LC_ALL=$loc bash "$check" "$f" 2>&1 >/dev/null)
  rc=$?
  set -e
  mapfile -t got < <(grep '^check-pins: ' <<< "$out" || true)
  ok=1
  [ "$rc" -eq 1 ] || ok=0
  [ "${#got[@]}" -eq "${#want[@]}" ] || ok=0
  for w in "${want[@]}"; do
    n=0
    for g in "${got[@]}"; do [[ "$g" == *"$w"* ]] && n=$((n + 1)); done
    [ "$n" -eq 1 ] || ok=0
  done
  if [ "$ok" -eq 1 ]; then
    echo "ok   $f"
  else
    echo "FAIL $f: exit $rc, want ${#want[@]} refusal(s) [${want[*]}], got:"
    printf '       %s\n' "${got[@]}"
    fail=1
  fi
done
done

for f in container/Dockerfile container/engine.Dockerfile container/ardupilot.Dockerfile; do
  if bash "$check" "$f" > /dev/null; then echo "ok   $f passes"; else echo "FAIL $f is refused"; fail=1; fi
done
exit "$fail"
