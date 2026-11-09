#!/usr/bin/env bash
# container/engine-smoke.sh: proves the engine image's aethelgard binary, and
# the analog bridge beside it, work on real inputs (INV-03), under --network
# none. Same shape as smoke.sh: each check is a command with an exact expected
# exit code; the first failure exits 1 naming the check.
#
#   docker run --rm -i --network none -v "$PWD/container:/container:ro" \
#     -v "$PWD/examples:/examples:ro" IMAGE bash /container/engine-smoke.sh
set -uo pipefail

FIX=/container/fixtures
EX=/examples
WORK=$(mktemp -d)
A=aethelgard

pass() { echo "ok      $1"; }
fail() { echo "FAIL    $1: $2" >&2; exit 1; }
expect() {
  local name=$1 want=$2
  shift 2
  "$@" > "$WORK/$name.log" 2>&1 < /dev/null
  local got=$?
  if [ "$got" -ne "$want" ]; then
    cat "$WORK/$name.log" >&2
    fail "$name" "exit $got, expected $want"
  fi
  pass "$name"
}
contains() { grep -qF -- "$2" "$WORK/$1.log" || fail "$1" "output lacks '$2'"; }

command -v "$A" > /dev/null || fail aethelgard "not on PATH"
expect version 0 "$A" --version
contains version "aethelgard"

expect validate.valid 0 "$A" validate --file "$EX/sentinel-01-minimal.json"
expect validate.invalid 2 "$A" validate --file "$EX/invalid-zero-domain.json"
contains validate.invalid "ZeroNominalVoltage"

expect drc.clean 0 "$A" check drc --board "$FIX/drc-clean.kicad_pcb"
expect drc.violation 1 "$A" check drc --board "$FIX/drc-violation.kicad_pcb"
contains drc.violation '"verdict":"fail"'

# The analog bridge (AET-WO-2200): beside the CLI, where the CLI looks for it,
# and measuring the reference line at 2.2 kOhm as AET-WO-2101 S2 did. The
# request is the one simulate --firmware writes for I2C_SCL.
[ -x "$(dirname "$(command -v "$A")")/aethelgard-bridge" ] || fail bridge "not beside $A"
expect bridge.measure 0 aethelgard-bridge "$FIX/bridge-request.json"
contains bridge.measure '"net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":186.401'

# mcp over a pipe: initialize, then tools/list; closing stdin ends the server.
{
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"engine-smoke","version":"0"}}}'
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
  sleep 2
} | "$A" mcp > "$WORK/mcp.log" 2> "$WORK/mcp.err"
got=${PIPESTATUS[1]}
[ "$got" -eq 0 ] || { cat "$WORK/mcp.err" >&2; fail mcp "exit $got, expected 0"; }
contains mcp '"serverInfo"'
for t in validate check_drc check_erc prove simulate; do
  contains mcp "\"name\":\"$t\""
done
pass mcp

echo "engine-smoke: all checks passed"
