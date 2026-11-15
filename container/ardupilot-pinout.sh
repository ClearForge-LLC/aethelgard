#!/usr/bin/env bash
# container/ardupilot-pinout.sh BIN TOPOLOGY BOARDS ALLOW: the pinout check over
# the four boards the `ardupilot` job built (AET-WO-4400a §1.4). Step G must
# pass; each seed must fail with exactly the findings the AET-WO-4400 spike
# measured, seed-pkg now named by the package check. Runs on the runner, not in
# the ArduPilot image: the check is Rust and reads bytes.
#
# A finding is written `check kind pin net decoded`, with `-` for no net.
set -euo pipefail
bin=$1 topology=$2 boards=$3 allow=$4
bad=0

expect() {
  local name=$1 code=$2 want=$3 out got rc=0
  out=$("$bin" check pinout --topology "$topology" --elf "$boards/$name/arducopter.elf" \
    --hwdef-h "$boards/$name/hwdef.h" --allow "$allow") || rc=$?
  echo "$name: exit $rc: $out"
  got=$(jq -r '.findings[] | "\(.check) \(.kind) \(.pin) \(.net // "-") \(.decoded)"' <<< "$out" | sort)
  if [ "$rc" -ne "$code" ] || [ "$got" != "$(sort <<< "$want")" ] \
    || [ "$(jq -r .decodes <<< "$out")" != "elf and hwdef.h agree" ]; then
    echo "::error::$name: expected exit $code with findings:"$'\n'"$want"$'\n'"got exit $rc with:"$'\n'"$got"
    bad=1
  fi
}

expect step-g 0 ""
expect seed-pin 1 "forward NotAlternate PB6 I2C_SCL input
reverse UnnamedPin PB8 - alternate AF4"
expect seed-af 1 "forward WrongAlternateFunction PB6 I2C_SCL alternate AF6
forward WrongAlternateFunction PB7 I2C_SDA alternate AF6"
expect seed-pkg 1 "reverse UnnamedPin PI0 - output
package NotInPackage PI0 - output"
exit "$bad"
