#!/usr/bin/env bash
# container/ardupilot-pinout.sh BIN TOPOLOGY BOARDS ALLOW: the pinout check over
# the boards the `ardupilot` and `generated` jobs built. Runs on the runner, not
# in the ArduPilot image: the check is Rust and reads bytes.
#
# - The four hand-written boards (AET-WO-4400a §1.4), judged against TOPOLOGY
#   with ALLOW. Since AET-WO-4400b the check also judges each topology pin's
#   OTYPER, OSPEEDR and PUPDR. A hand-written board carries no speed or pull
#   tokens, so ArduPilot gives every pin SPEED_MEDIUM (10) and pulls up the CS
#   and the USART: each board fails with exactly those findings beside the
#   seed's own.
# - The generated boards (AET-WO-4400b), each judged against its own topology
#   with no allow-list: each passes. Then the witness: every `define` and
#   `STM32_` line of the generated hwdef.dat reached ArduPilot's hwdef.h as
#   that exact #define, so no emitted define was dropped by ArduPilot's reader.
#
# A finding is written `check kind pin net decoded`, with `-` for no net.
set -euo pipefail
bin=$1 topology=$2 boards=$3 allow=$4
bad=0

expect() {
  local name=$1 code=$2 want=$3 topo=$4 out got rc=0
  shift 4
  out=$("$bin" check pinout --topology "$topo" --elf "$boards/$name/arducopter.elf" \
    --hwdef-h "$boards/$name/hwdef.h" "$@") || rc=$?
  echo "$name: exit $rc: $out"
  got=$(jq -r '.findings[] | "\(.check) \(.kind) \(.pin) \(.net // "-") \(.decoded)"' <<< "$out" | sort)
  if [ "$rc" -ne "$code" ] || [ "$got" != "$(sort <<< "$want")" ] \
    || [ "$(jq -r .decodes <<< "$out")" != "elf and hwdef.h agree" ]; then
    echo "::error::$name: expected exit $code with findings:"$'\n'"$want"$'\n'"got exit $rc with:"$'\n'"$got"
    bad=1
  fi
}

# ArduPilot's defaults on Sentinel-01's topology pins: SPEED_MEDIUM everywhere,
# where solve stored 00 or 01, and a pull-up on the CS and the console's pins.
defaults="forward WrongPull PA0 FLASH_CS PUPDR 01
forward WrongPull PA10 CONSOLE_RX PUPDR 01
forward WrongPull PA9 CONSOLE_TX PUPDR 01
forward WrongSpeed PA0 FLASH_CS OSPEEDR 10
forward WrongSpeed PA10 CONSOLE_RX OSPEEDR 10
forward WrongSpeed PA5 SPI_SCK OSPEEDR 10
forward WrongSpeed PA6 SPI_MISO OSPEEDR 10
forward WrongSpeed PA7 SPI_MOSI OSPEEDR 10
forward WrongSpeed PA9 CONSOLE_TX OSPEEDR 10
forward WrongSpeed PB6 I2C_SCL OSPEEDR 10
forward WrongSpeed PB7 I2C_SDA OSPEEDR 10"

expect step-g 1 "$defaults" "$topology" --allow "$allow"
expect seed-pin 1 "$defaults
forward NotAlternate PB6 I2C_SCL input
forward WrongOutputType PB6 I2C_SCL push-pull
reverse UnnamedPin PB8 - alternate AF4" "$topology" --allow "$allow"
expect seed-af 1 "$defaults
forward WrongAlternateFunction PB6 I2C_SCL alternate AF6
forward WrongAlternateFunction PB7 I2C_SDA alternate AF6" "$topology" --allow "$allow"
expect seed-pkg 1 "$defaults
reverse UnnamedPin PI0 - output
package NotInPackage PI0 - output" "$topology" --allow "$allow"

for name in sentinel-01 sentinel-02 serial-dshot-9600; do
  expect "$name" 0 "" "$boards/$name/topology.json"
  # The witness: each emitted define, exactly, in hwdef.h.
  missed=0
  while read -r line; do
    def=${line#define }
    grep -qxF "#define $def" "$boards/$name/hwdef.h" \
      || { echo "::error::$name: \`$line\` did not reach hwdef.h as \`#define $def\`"; missed=1; }
  done < <(sed -e 's/[[:space:]]*#.*$//' "$boards/$name/hwdef.dat" | grep -E '^(define |STM32_)')
  if [ "$missed" = 0 ]; then echo "$name: every define and STM32_ line reached hwdef.h"; else bad=1; fi
done
exit "$bad"
