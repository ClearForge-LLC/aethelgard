#!/usr/bin/env bash
# container/ardupilot-pinout.sh BIN TOPOLOGY BOARDS ALLOW WORKFLOW: the pinout
# check over the boards the `ardupilot` and `generated` jobs built. Runs on the
# runner, not in the ArduPilot image: the check is Rust and reads bytes.
#
# The boards judged are WORKFLOW's own matrices, never a list kept here
# (AET-WO-1917): the `ardupilot` job's boards, each of which must be one of the
# four hand boards below, and every `generated` row without a `drop:` (a mutant
# is uploaded as `mutant-*`, out of reach). BOARDS must hold exactly those
# directories: a matrix board with no directory, a directory no matrix names,
# or an empty matrix is refused by name before any board is judged.
#   ardupilot-pinout.sh hand WORKFLOW         the `ardupilot` matrix's boards
#   ardupilot-pinout.sh generated WORKFLOW    the judged `generated` boards
#   ardupilot-pinout.sh boards WORKFLOW BOARDS  the set check alone
#
# - The four hand-written boards (AET-WO-4400a §1.4), judged against TOPOLOGY
#   with ALLOW. Since AET-WO-4400b the check also judges each topology pin's
#   OTYPER, OSPEEDR and PUPDR. A hand-written board carries no speed or pull
#   tokens, so ArduPilot gives every pin SPEED_MEDIUM (10) and pulls up the CS
#   and the USART: each board fails with exactly those findings beside the
#   seed's own.
# - The generated boards (AET-WO-4400b), each judged against its own topology
#   with no allow-list: each passes.
# - Every board also goes through the identity judge (AET-WO-4400b2): ArduPilot's
#   device tables (hwdef.h's SPI devices, PWM groups, I2C clock, barometer
#   probes and bauds, and the ELF's serial table) against the topology. A hand
#   board's SPIDEV is MODE3 at 20 MHz where mode 0 at the achieved 16 MHz was
#   solved, and its USART3 is a SERIAL1 the topology does not have.
#
# A finding is written `check kind pin net decoded`, with `-` for no net.
set -euo pipefail

die() { echo "::error::ardupilot-pinout: $*" >&2; exit 1; }

# The four hand boards (AET-WO-4400a): each one's own findings beside the
# defaults every hand board shares (below). The one list of hand boards.
declare -A OWN=(
  [step-g]=""
  [seed-pin]="forward NotAlternate PB6 I2C_SCL input
forward WrongOutputType PB6 I2C_SCL push-pull
reverse UnnamedPin PB8 - alternate AF4"
  [seed-af]="forward WrongAlternateFunction PB6 I2C_SCL alternate AF6
forward WrongAlternateFunction PB7 I2C_SDA alternate AF6
identity WrongBaroProbe HAL_BARO_PROBE1 U2 BMP280 on I2C4 at 0x76"
  [seed-pkg]="reverse UnnamedPin PI0 - output
package NotInPackage PI0 - output"
)

# hand WORKFLOW: the boards of the `ardupilot` job's `board: [...]` matrix.
hand() {
  local names
  [ -r "$1" ] || die "cannot read $1"
  names=$(awk '
    /^  [A-Za-z0-9_-]+:$/ { job = $1 }
    job == "ardupilot:" && /^        board: \[.*\]$/ { l = $0; sub(/^[^[]*\[/, "", l); sub(/\]$/, "", l); gsub(/[ ,]+/, "\n", l); print l }
  ' "$1" | sed '/^$/d')
  [ -n "$names" ] || die "$1: the ardupilot job's matrix names no board"
  printf '%s\n' "$names"
}

# generated WORKFLOW: the `generated` job's matrix rows with no `drop:`, one name per line.
generated() {
  local names
  [ -r "$1" ] || die "cannot read $1"
  names=$(awk '
    function flush() { if (row != "" && !drop) print row; row = ""; drop = 0 }
    /^  [A-Za-z0-9_-]+:$/ { flush(); job = $1; inc = 0; next }
    job != "generated:" { next }
    /^        include:$/ { inc = 1; next }
    inc && /^    [a-z]/ { flush(); inc = 0; next }
    inc && /^          - / { flush(); if ($0 !~ /^          - board: [^ ]+$/) { print "!" $0; next } row = $3; next }
    inc && /^            drop: / { drop = 1 }
    END { flush() }
  ' "$1")
  [ -n "$names" ] || die "$1: the generated job's matrix names no board"
  ! grep -q '^!' <<<"$names" || die "$1: a generated matrix row that does not open with its board: $(grep '^!' <<<"$names" | head -1 | cut -c2-)"
  printf '%s\n' "$names"
}

# boards WORKFLOW BOARDS: BOARDS holds exactly the hand and generated boards WORKFLOW names.
boards() {
  local want got n missing extra
  want=$( { hand "$1"; generated "$1"; } ) || exit 1
  for n in $(hand "$1"); do
    [[ -v OWN[$n] ]] || die "$n: in the ardupilot matrix, but this judge has no expected findings for it"
  done
  for n in $want; do [[ $n =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || die "bad board name in $1: $n"; done
  [ -d "$2" ] || die "no boards directory $2"
  got=$(find "$2" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)
  missing=$(comm -23 <(sort -u <<<"$want") <(printf '%s\n' "$got"))
  extra=$(comm -13 <(sort -u <<<"$want") <(printf '%s\n' "$got") | sed '/^$/d')
  [ -z "$missing" ] || die "in the workflow's matrix, but no board reached the judge: $(tr '\n' ' ' <<<"$missing")"
  [ -z "$extra" ] || die "a board no matrix of the workflow names: $(tr '\n' ' ' <<<"$extra")"
  echo "judged set: $(wc -l <<<"$want") boards ($(tr '\n' ' ' <<<"$want"))"
}

case ${1:-} in
  hand | generated) "$1" "${2:?WORKFLOW}"; exit ;;
  boards) boards "${2:?WORKFLOW}" "${3:?BOARDS}"; exit ;;
esac

bin=$1 topology=$2 boards=$3 allow=$4 workflow=$5
boards "$workflow" "$boards"
bad=0

expect() {
  local name=$1 code=$2 want=$3 topo=$4 out got rc=0
  shift 4
  out=$("$bin" check pinout --topology "$topo" --elf "$boards/$name/arducopter.elf" \
    --hwdef-h "$boards/$name/hwdef.h" "$@") || rc=$?
  echo "$name: exit $rc: $out"
  got=$(jq -r '.findings[] | "\(.check) \(.kind) \(.pin) \(.net // "-") \(.decoded)"' <<< "$out" | sort)
  if [ "$rc" -ne "$code" ] || [ "$got" != "$(sort <<< "$want")" ] \
    || [ "$(jq -r .decodes <<< "$out")" != "elf and hwdef.h agree" ] \
    || [ "$(jq -r .identity <<< "$out")" != "judged" ]; then
    echo "::error::$name: expected exit $code with findings:"$'\n'"$want"$'\n'"got exit $rc with:"$'\n'"$got"
    bad=1
  fi
}

# What every hand board shares: ArduPilot's defaults on Sentinel-01's topology
# pins (SPEED_MEDIUM everywhere, where solve stored 00 or 01, and a pull-up on
# the CS and the console's pins), and its own SPIDEV and USART3 (identity).
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
forward WrongSpeed PB7 I2C_SDA OSPEEDR 10
identity WrongSpiMode SPIDEV dataflash U4 MODE3
identity WrongSpiSpeed SPIDEV dataflash low U4 20000000 Hz
identity WrongSpiSpeed SPIDEV dataflash high U4 20000000 Hz
identity UnexpectedSerialPort SERIAL1 - U(S)ART3 TX PD8 RX PD9"

for name in $(hand "$workflow"); do
  expect "$name" 1 "$defaults${OWN[$name]:+$'\n'${OWN[$name]}}" "$topology" --allow "$allow"
done

for name in $(generated "$workflow"); do
  expect "$name" 0 "" "$boards/$name/topology.json"
done
exit "$bad"
