#!/bin/bash
# AET-WO-1918 (1): ci/ceiling.sh fails each run it exists to fail, on planted probe logs, and
# ci.yml runs it in every job that proves Kani harnesses. Run by the `tools` job.
set -u
cd "$(dirname "$0")/.." || exit 1
t=$(mktemp -d)
trap 'rm -rf "$t"' EXIT
fail=0
check() { # NAME WANT(0|1) ARGS...
  local name=$1 want=$2; shift 2
  local got=0
  bash ci/ceiling.sh "$@" > "$t/out" 2>&1 || got=1
  if [ "$got" = "$want" ]; then echo "PASS $name"; else echo "FAIL $name (exit $got, want $want)"; cat "$t/out"; fail=1; fi
}
ceiling=$(grep -v '^#' ci/kani-ceiling | tr -d '[:space:]')
case $ceiling in '' | *[!0-9]*) echo "FAIL ci/kani-ceiling holds no number"; exit 1 ;; esac
[ "$ceiling" = 12582912 ] || { echo "FAIL the ceiling is $ceiling KiB, not the WO's 12 GiB (12582912)"; fail=1; }

printf 'PROBE peak harness=proofs::a vmhwm_kib=%s seconds=1.0\nPROBE boot seconds=3.1 comm=renode argv=renode\n' "$ceiling" > "$t/at.log"
printf 'PROBE peak harness=proofs::a vmhwm_kib=100 seconds=1.0\nPROBE peak harness=proofs::heavy vmhwm_kib=%s seconds=9.0\n' "$((ceiling + 1))" > "$t/over.log"
printf 'PROBE boot seconds=3.1 comm=renode argv=renode\n' > "$t/none.log"
check "a harness at the ceiling passes" 0 "$t/at.log" ci/kani-ceiling --require-peak
check "a planted harness 1 KiB over the ceiling fails" 1 "$t/over.log" ci/kani-ceiling
grep -q 'proofs::heavy' "$t/out" && echo "PASS the failure names the heavy harness" || { echo "FAIL the failure does not name proofs::heavy"; fail=1; }
check "a missing probe log fails (nothing was measured)" 1 "$t/missing.log" ci/kani-ceiling
check "no harness, where none is required, passes" 0 "$t/none.log" ci/kani-ceiling
check "no harness, where one is required, fails" 1 "$t/none.log" ci/kani-ceiling --require-peak
printf '# no number\n' > "$t/ceiling"
check "a ceiling file with no number fails" 1 "$t/at.log" "$t/ceiling"

# The pins: each job that proves harnesses starts the probe before its test or verify, and
# judges the ceiling after it on always(); a bench job requires a measured peak.
job() { awk -v j="  $1:" '$0 == j { on = 1; next } on && /^  [a-z][a-z0-9-]*:$/ { exit } on' .github/workflows/ci.yml; }
for spec in "tools-shard:" "bench-package:--require-peak" "bench-package-s03:--require-peak"; do
  name=${spec%%:*} req=${spec#*:}
  body=$(job "$name")
  probe=$(grep -c 'nohup python3 -I ci/probe.py "$RUNNER_TEMP/probe.log" "$RUNNER_TEMP/probe.stop"' <<<"$body")
  ceil=$(grep -c "bash ci/ceiling.sh \"\$RUNNER_TEMP/probe.log\" ci/kani-ceiling${req:+ $req}\$" <<<"$body")
  first_probe=$(grep -n 'ci/probe.py' <<<"$body" | head -1 | cut -d: -f1)
  first_run=$(grep -n -E 'ci/shard.sh run|\$VERIFY(_S03)? --' <<<"$body" | head -1 | cut -d: -f1)
  last_run=$(grep -n -E 'ci/shard.sh run|\$VERIFY(_S03)? --' <<<"$body" | tail -1 | cut -d: -f1)
  at=$(grep -n 'bash ci/ceiling.sh' <<<"$body" | head -1 | cut -d: -f1)
  always=$(awk -v at="$at" 'NR < at && /- name:/ { n = NR } END { print n }' <<<"$body")
  guarded=$(sed -n "${always},${at}p" <<<"$body" | grep -c 'if: always()')
  if [ "$probe" = 1 ] && [ "$ceil" = 1 ] && [ -n "$first_run" ] && [ "$first_probe" -lt "$first_run" ] \
    && [ "$at" -gt "$last_run" ] && [ "$guarded" = 1 ]; then
    echo "PASS $name starts the probe before its run and judges the ceiling after it${req:+ ($req)}"
  else
    echo "FAIL $name: probe=$probe ceiling=$ceil probe_line=$first_probe run=$first_run..$last_run ceiling_line=$at always=$guarded"
    fail=1
  fi
done
[ "$fail" = 0 ] && echo "test-ceiling: all passed"
exit "$fail"
