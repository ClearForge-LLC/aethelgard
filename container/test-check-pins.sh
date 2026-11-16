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
for f in container/fixtures/check-pins/*.Dockerfile; do
  mapfile -t want < <(sed -n 's/^# expect: //p' "$f")
  [ "${#want[@]}" -gt 0 ] || { echo "FAIL $f: no expect line"; fail=1; continue; }
  set +e
  out=$(bash "$check" "$f" 2>&1 >/dev/null)
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

for f in container/Dockerfile container/engine.Dockerfile container/ardupilot.Dockerfile; do
  if bash "$check" "$f" > /dev/null; then echo "ok   $f passes"; else echo "FAIL $f is refused"; fail=1; fi
done
exit "$fail"
