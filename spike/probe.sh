#!/usr/bin/env bash
# AET-WO-0200 spike probe. This is a MEASUREMENT, not a gate: it runs each tool
# on a real input and records what happened, failures included, so one CI run
# answers the work order's §0. It deliberately captures every exit code instead
# of stopping at the first nonzero one; the build half's container/smoke.sh is
# the gate and stops. Every result is one line starting `ANSWER <mode>`, so the
# FEEDBACK is filled by grepping the log.
#
# Run inside the candidate image with the spike directory mounted at /spike:
#   docker run --rm -e PROBE_MODE=net -v "$PWD/spike:/spike:ro" IMAGE bash /spike/probe.sh
set -u

MODE=${PROBE_MODE:-net}
FIX=/spike/fixtures
OUT=$(mktemp -d)

answer() { printf 'ANSWER %s %s=%s\n' "$MODE" "$1" "$2"; }

# probe NAME CMD... : run CMD in a log group, record its exit code.
probe() {
  local name=$1
  shift
  echo "::group::$name: $*"
  "$@"
  local rc=$?
  echo "::endgroup::"
  answer "$name.rc" "$rc"
}

echo "::group::os and toolchain"
cat /etc/os-release
uname -m
rustc --version
cargo --version
dpkg-query -W -f='${Package} ${Version}\n' kicad ngspice libngspice0 2>&1
echo "::endgroup::"
answer os "$(. /etc/os-release && echo "$PRETTY_NAME")"
answer rustc "$(rustc --version)"
for pkg in kicad ngspice libngspice0; do
  answer "dpkg.$pkg" "$(dpkg-query -W -f='${Version}' "$pkg" 2>/dev/null || echo absent)"
done

# --- KiCad (S2, S4) ---
probe kicad.version kicad-cli version
answer kicad.version "$(kicad-cli version 2>&1)"
echo "::group::kicad-cli pcb drc --help"
kicad-cli pcb drc --help 2>&1 | tee "$OUT/drc-help.txt"
echo "::endgroup::"
answer drc.help.exit_code_violations "$(grep -c -- '--exit-code-violations' "$OUT/drc-help.txt")"
answer drc.help.format "$(grep -c -- '--format' "$OUT/drc-help.txt")"

# drc NAME BOARD [FLAGS...] : run DRC with JSON output, record rc, print the report.
drc() {
  local name=$1 board=$2
  shift 2
  echo "::group::drc $name"
  kicad-cli pcb drc --format json -o "$OUT/$name.json" "$@" "$board"
  local rc=$?
  head -c 6000 "$OUT/$name.json" 2>/dev/null
  echo
  echo "::endgroup::"
  answer "drc.$name.rc" "$rc"
  answer "drc.$name.violations" "$(grep -o '"type": *"[a-z_]*"' "$OUT/$name.json" 2>/dev/null | sort | uniq -c | tr -s ' \n' ' ')"
}
drc violation_noflag "$FIX/drc-violation.kicad_pcb"
drc violation_flag "$FIX/drc-violation.kicad_pcb" --exit-code-violations
drc clean_noflag "$FIX/drc-clean.kicad_pcb"
drc clean_flag "$FIX/drc-clean.kicad_pcb" --exit-code-violations
# Boards KiCad itself ships, if any: proves the DRC machinery independently of
# the hand-written fixtures.
n=0
for demo in /usr/share/kicad/demos/*/*.kicad_pcb; do
  [ -e "$demo" ] || break
  n=$((n + 1))
  [ "$n" -le 3 ] || break
  base=$(basename "$demo" .kicad_pcb)
  kicad-cli pcb drc --format json -o "$OUT/demo.json" --exit-code-violations "$demo" > /dev/null 2>&1
  answer "drc.demo.$base.flag_rc" "$?"
  kicad-cli pcb drc --format json -o "$OUT/demo.json" "$demo" > /dev/null 2>&1
  answer "drc.demo.$base.noflag_rc" "$?"
done
answer drc.demos_found "$(ls /usr/share/kicad/demos/*/*.kicad_pcb 2>/dev/null | wc -l)"

# --- Renode (S2) ---
probe renode.version renode --version
answer renode.version "$(renode --version 2>&1 | head -1)"
echo "::group::renode --help"
renode --help 2>&1 | head -60
echo "::endgroup::"
nucleo='mach create; machine LoadPlatformDescription @platforms/boards/nucleo_h753zi.repl; quit'
probe renode.nucleo.disable_gui timeout 180 renode --disable-gui --console --plain -e "$nucleo"
probe renode.nucleo.disable_xwt timeout 180 renode --disable-xwt --console --plain -e "$nucleo"

# --- ngspice (S2) ---
probe ngspice.version ngspice --version
echo "::group::ngspice -b rc-step.cir"
(cd "$OUT" && ngspice -b "$FIX/rc-step.cir") 2>&1 | tee "$OUT/rc.txt"
echo "::endgroup::"
answer ngspice.rise "$(grep -i '^rise' "$OUT/rc.txt" | head -1)"
answer libngspice.ldconfig "$(ldconfig -p | grep -o '/[^ ]*libngspice[^ ]*' | tr '\n' ' ')"
answer libngspice.dpkg "$(dpkg -L libngspice0 2>/dev/null | grep '\.so' | tr '\n' ' ')"

# --- Kani (S2) ---
probe kani.version cargo kani --version
answer kani.version "$(cargo kani --version 2>&1 | tail -1)"
answer kani.home "$(ls -d "${KANI_HOME:-/nonexistent}"/* ~/.kani 2>&1 | tr '\n' ' ')"
cp -r /spike/kani-probe "$OUT/kani-probe"
cd "$OUT/kani-probe"
probe kani.pass cargo kani --harness add_in_range
probe kani.overflow cargo kani --harness add_overflows
