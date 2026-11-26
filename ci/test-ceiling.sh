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

# The probe itself (ci/probe.py), on planted processes: a `kani-driver` (a copy of bash, so its
# comm is kani-driver and its argv carries --harness) that grows about 60 MB and exits, a `cbmc`
# beside a second, live driver, and that driver still alive at the stop.
cp /bin/bash "$t/kani-driver"; cp /bin/bash "$t/cbmc"
python3 -I ci/probe.py "$t/probe.log" "$t/probe.stop" &
probe=$!
sleep 0.5
"$t/kani-driver" -c 'x=$(head -c 60000000 /dev/zero | tr "\0" a); sleep 1' --harness proofs::planted_grows
"$t/kani-driver" -c 'sleep 30; :' --harness proofs::planted_alive &
alive=$!
sleep 0.5
"$t/cbmc" -c 'sleep 1; :' --planted
# AET-WO-1919: a gate (an `aethelgard` subcommand) and a compile under the live driver are timed;
# the CLI's internal reaper is no gate.
cp /bin/bash "$t/aethelgard"; cp /bin/bash "$t/kani-compiler"
"$t/kani-compiler" -c 'sleep 1; :' --planted
"$t/aethelgard" -c 'sleep 1; :' prove --harness proofs::planted_gate
"$t/aethelgard" -c 'sleep 1; :' --aethelgard-internal-reaper
touch "$t/probe.stop"; wait "$probe"
kill "$alive" 2>/dev/null; wait "$alive" 2>/dev/null
grown=$(sed -n 's/^PROBE peak harness=proofs::planted_grows vmhwm_kib=\([0-9]*\) .*/\1/p' "$t/probe.log" | sort -n | tail -1)
[ -n "$grown" ] && [ "$grown" -ge 50000 ] && echo "PASS the probe measured a kani-driver's peak by its harness ($grown KiB)" \
  || { echo "FAIL the probe did not measure proofs::planted_grows"; cat "$t/probe.log"; fail=1; }
grep -q '^PROBE peak harness=proofs::planted_alive/cbmc ' "$t/probe.log" && echo "PASS a cbmc is measured under the harness its kani-driver proves" \
  || { echo "FAIL no cbmc line under proofs::planted_alive"; cat "$t/probe.log"; fail=1; }
grep -q '^PROBE peak harness=proofs::planted_alive ' "$t/probe.log" && echo "PASS a driver alive at the stop is written" \
  || { echo "FAIL the live driver was dropped at the stop"; cat "$t/probe.log"; fail=1; }
grep -q '^PROBE compile seconds=[0-9.]* start=[0-9.]* harness=proofs::planted_alive/compile$' "$t/probe.log" \
  && echo "PASS a kani-compiler is timed under the harness its kani-driver proves" \
  || { echo "FAIL no compile line under proofs::planted_alive"; cat "$t/probe.log"; fail=1; }
grep -q '^PROBE gate seconds=1\.[0-9] start=[0-9.]* argv=-c sleep 1; : prove --harness proofs::planted_gate$' "$t/probe.log" \
  && echo "PASS a gate is timed with its argv" || { echo "FAIL no gate line for proofs::planted_gate"; cat "$t/probe.log"; fail=1; }
grep -q 'aethelgard-internal' "$t/probe.log" && { echo "FAIL the internal reaper was timed as a gate"; cat "$t/probe.log"; fail=1; } \
  || echo "PASS the CLI's internal reaper is not a gate"

# The pins: each job that proves harnesses starts the probe before its test or verify, and
# judges the ceiling after it on always(); a bench job requires a measured peak.
job() { awk -v j="  $1:" '$0 == j { on = 1; next } on && /^  [a-z][a-z0-9-]*:$/ { exit } on' .github/workflows/ci.yml; }
for spec in "tools-shard:" "bench-package:--require-peak" "bench-package-s03:--require-peak"; do
  name=${spec%%:*} req=${spec#*:}
  body=$(job "$name")
  probe=$(grep -c 'run: bash ci/probe-step.sh start$' <<<"$body")
  ceil=$(grep -c "run: bash ci/probe-step.sh judge${req:+ $req}\$" <<<"$body")
  first_probe=$(grep -n 'probe-step.sh start' <<<"$body" | head -1 | cut -d: -f1)
  first_run=$(grep -n -E 'ci/shard.sh run|\$VERIFY(_S03)? --' <<<"$body" | head -1 | cut -d: -f1)
  last_run=$(grep -n -E 'ci/shard.sh run|\$VERIFY(_S03)? --' <<<"$body" | tail -1 | cut -d: -f1)
  at=$(grep -n 'probe-step.sh judge' <<<"$body" | head -1 | cut -d: -f1)
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
