#!/usr/bin/env bash
# container/smoke.sh: proves each tool in the image works on a real input, not
# merely that it exists (INV-03). Every check is a command with an exact
# expected exit code and, where the tool's output carries the answer, an exact
# expected string. The first failure exits 1 naming the check. Results are
# asserted by exact key, never by exclusion (ruled on PR #7).
#
# Run inside the image with container/ mounted read-only at /container:
#   docker run --rm --network none -v "$PWD/container:/container:ro" IMAGE bash /container/smoke.sh
#
# No `set -e`: each exit code is captured and compared, never ignored.
set -uo pipefail

FIX=/container/fixtures
WORK=$(mktemp -d)

pass() { echo "ok      $1"; }
fail() {
  echo "FAIL    $1: $2" >&2
  exit 1
}

# expect NAME CODE CMD... : CMD must exit with exactly CODE. Its output is kept
# in $WORK/NAME.log for contains/lacks.
expect() {
  local name=$1 want=$2
  shift 2
  "$@" > "$WORK/$name.log" 2>&1
  local got=$?
  if [ "$got" -ne "$want" ]; then
    cat "$WORK/$name.log" >&2
    fail "$name" "exit $got, expected $want"
  fi
  pass "$name"
}
contains() { grep -qF -- "$2" "$WORK/$1.log" || fail "$1" "output lacks '$2'"; }
is_line() { grep -qxF -- "$2" "$WORK/$1.log" || fail "$1" "no line is exactly '$2'"; }
lacks() { if grep -qF -- "$2" "$WORK/$1.log"; then fail "$1" "output contains '$2'"; fi; }

# 1. Each tool is present and reports its pinned version.
for tool in kicad-cli renode ngspice cargo-kani; do
  command -v "$tool" > /dev/null || fail "$tool" "not on PATH"
done
expect kicad-cli.version 0 kicad-cli version
is_line kicad-cli.version "10.0.7"
expect renode.version 0 renode --version
is_line renode.version "Renode v1.17.0"
expect ngspice.version 0 ngspice --version
contains ngspice.version "** ngspice-44.2 : "
expect kani.version 0 cargo kani --version
is_line kani.version "Kani Rust Verifier 0.68.0 (cargo plugin)"

# 2. DRC with RUL-03's mandatory flag: the clean board exits 0, the board with
# one deliberate clearance violation exits 5 and reports it.
drc() { kicad-cli pcb drc --exit-code-violations --format json -o "$WORK/$1.json" "$FIX/$1.kicad_pcb"; }
expect kicad-cli.drc.clean 0 drc drc-clean
expect kicad-cli.drc.violation 5 drc drc-violation
[ "$(grep -cE '"type": *"clearance"' "$WORK/drc-violation.json")" -eq 1 ] \
  || fail kicad-cli.drc.violation "report does not hold exactly one clearance violation"

# 3. Measurement only (architecture.md §2): the violation board without the
# flag. Its exit code is printed and deliberately not asserted.
kicad-cli pcb drc --format json -o "$WORK/noflag.json" "$FIX/drc-violation.kicad_pcb" > "$WORK/noflag.log" 2>&1
echo "measure kicad-cli.drc.violation_without_flag exit=$?"

# 4. Renode loads RUL-08's board headless and quits. A failed load prints no
# [ERROR] tag and, with stdin at EOF, still exits 0, so the exit code alone
# proves nothing: the log must list the board's CPU. The negative control, a
# nonexistent platform, must name the missing file and list no CPU.
renode_load() {
  timeout "$1" renode --disable-gui --console --plain \
    -e "mach create; machine LoadPlatformDescription @$2; peripherals; quit"
}
expect renode.nucleo_h753zi 0 renode_load 180 platforms/boards/nucleo_h753zi.repl
contains renode.nucleo_h753zi "cpu (CortexM)"
lacks renode.nucleo_h753zi "There was an error executing command"
renode_load 30 platforms/boards/does-not-exist.repl > "$WORK/renode.negative_control.log" 2>&1
echo "measure renode.missing_platform exit=$?"
contains renode.negative_control "Could not find file 'platforms/boards/does-not-exist.repl'"
lacks renode.negative_control "cpu (CortexM)"
pass renode.negative_control

# 5. ngspice in batch mode: the RC step's 10-90 % rise is within 2 % of
# 2.2*R*C = 2.2 ms.
expect ngspice.rc_step 0 ngspice -b "$FIX/rc-step.cir"
rise=$(awk '$1 == "rise" && $2 == "=" { print $3; exit }' "$WORK/ngspice.rc_step.log")
[ -n "$rise" ] || fail ngspice.rc_step "no 'rise =' measurement in the output"
awk -v r="$rise" 'BEGIN { d = (r - 2.2e-3) / 2.2e-3; exit !(d < 0.02 && d > -0.02) }' \
  || fail ngspice.rc_step "rise $rise s is not within 2 % of 2.2e-3 s"
echo "measure ngspice.rc_step rise=$rise"

# 6. Kani proves one harness and fails the seeded overflow. A failing proof is
# what shows the backend runs.
cp -r "$FIX/kani-smoke" "$WORK/kani-smoke"
kani() { (cd "$WORK/kani-smoke" && cargo kani --harness "$1"); }
expect kani.add_in_range 0 kani add_in_range
contains kani.add_in_range "VERIFICATION:- SUCCESSFUL"
expect kani.add_overflows 1 kani add_overflows
contains kani.add_overflows "attempt to add with overflow"

# 7. The libngspice shared object is at its recorded path (OQ-02's in-process
# prior) and the loader knows it.
[ -f /usr/lib/x86_64-linux-gnu/libngspice.so.0 ] || fail libngspice "missing /usr/lib/x86_64-linux-gnu/libngspice.so.0"
ldconfig -p | grep -qF "libngspice.so.0 " || fail libngspice "ldconfig does not list libngspice.so.0"
pass libngspice

echo "smoke: all checks passed"
