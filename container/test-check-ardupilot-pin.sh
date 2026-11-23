#!/usr/bin/env bash
# container/test-check-ardupilot-pin.sh: check-ardupilot-pin.sh against its
# fixtures (AET-WO-4402). Each fixture under container/fixtures/ardupilot-pin/
# is named in the table below with its verdict:
#   accept   exit 0, stdout exactly the `pin` fixture's line, stderr empty;
#   missing  exit 1, stdout empty, stderr exactly "FILE missing or unreadable";
#   form     exit 1, stdout empty, stderr one line, exactly "FILE is not
#            <form>: <pin>", the pin being the file with NULs and trailing
#            newlines dropped, cut to 200 bytes and escaped by printf %q
#            (AET-WO-4407), so no line of it starts with `::` and none is long.
# So a fixture cannot pass on the other refusal. The committed pin must pass,
# and every fixture in the directory must be in the table. The check runs
# under en_US.UTF-8 when the host has it, where [0-9a-f] matches look-alikes
# unless the check sets its own locale.
#
#   bash container/test-check-ardupilot-pin.sh [path/to/check-ardupilot-pin.sh]
set -euo pipefail

check=${1:-container/check-ardupilot-pin.sh}
dir=container/fixtures/ardupilot-pin
want_pin=$(head -n 1 "$dir/pin")
want_form='ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:<64 hex>'
# No pipe: under pipefail, grep -m1 can end `locale -a` by SIGPIPE.
locales=$(locale -a 2>/dev/null || true)
loc=$(grep -ixm1 'en_US\.utf-\?8' <<< "$locales" || echo C.UTF-8)
echo "the check runs under LC_ALL=$loc"
fail=0

# quoted FILE: the refusal's view of FILE, NULs and trailing newlines dropped.
quoted() {
  local p
  p=$(LC_ALL=C tr -d '\000' < "$1")
  if [ "$(printf %s "$p" | wc -c)" -le 200 ]; then LC_ALL=C printf '%q' "$p"
  else LC_ALL=C printf '%q (the first 200 of %s bytes)' "$(printf %s "$p" | head -c 200)" "$(printf %s "$p" | wc -c)"; fi
}

run() {
  local f=$1 verdict=$2 out err rc ok=1
  set +e
  out=$(LC_ALL=$loc bash "$check" "$f" 2> "$tmp"); rc=$?
  set -e
  err=$(cat "$tmp")
  case $verdict in
    accept) [ "$rc" -eq 0 ] && [ "$out" = "$want_pin" ] && [ -z "$err" ] || ok=0 ;;
    missing) [ "$rc" -eq 1 ] && [ -z "$out" ] && [ "$err" = "$f missing or unreadable" ] || ok=0 ;;
    form) [ "$rc" -eq 1 ] && [ -z "$out" ] && [ "$err" = "$f is not $want_form: $(quoted "$f")" ] \
            && [ "$(wc -l < "$tmp")" -eq 1 ] && [ "$(wc -c < "$tmp")" -le 1024 ] || ok=0 ;;
  esac
  if [ "$ok" -eq 1 ]; then
    echo "ok   $verdict $f"
  else
    echo "FAIL $f: want $verdict, got exit $rc, stdout [${out:0:120}], stderr [${err:0:160}]"
    fail=1
  fi
}

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

# name                verdict   what it is
table='
pin                  accept    the committed pin, no newline
pin-newline          accept    one trailing newline, stripped as $(cat) did
pin-blank-lines      accept    several trailing newlines, all stripped as $(cat) did
symlink              accept    a symlink to pin-newline: it still names one digest
uppercase-hex        form      an uppercase A in the digest
hex-63               form      63 hex digits
hex-65               form      65 hex digits
tag-latest           form      :latest instead of a digest
digest-and-tag       form      :latest and a digest together
other-registry       form      docker.io instead of ghcr.io
other-repository     form      clearforge-llc/aethelgard instead of -ardupilot
sha512               form      sha512: instead of sha256:
two-lines            form      the pin twice, one per line
second-line          form      a valid first line, then a second
leading-space        form      a space before the pin
trailing-space       form      a space after the pin
empty                form      an empty file
crlf                 form      a CRLF line ending
nul-embedded         form      a NUL inside the digest (bash would drop it)
nul-trailing         form      a NUL after the newline (bash would drop it)
lookalike-cyrillic   form      a Cyrillic es in clearforge-llc
lookalike-fullwidth  form      a fullwidth 4 in the digest
lookalike-accent     form      an a with an acute accent in the digest
long-line            form      the pin followed by 65536 more hex digits
workflow-command     form      a second line ::warning::, which must not start a log line
carriage-return      form      a CR, then ::error:: on what a terminal shows as its own line
directory            missing   a directory in place of the file
symlink-dangling     missing   a symlink to nothing
does-not-exist       missing   no file at all
'

listed=()
while read -r name verdict _; do
  [ -n "$name" ] || continue
  listed+=("$name")
  run "$dir/$name" "$verdict"
done <<< "$table"

for f in "$dir"/*; do
  name=${f##*/}
  printf '%s\n' "${listed[@]}" | grep -qxF -- "$name" || { echo "FAIL $f is not in the table"; fail=1; }
done

if out=$(bash "$check" container/ARDUPILOT-IMAGE) && [ "$out" = "$(head -n 1 container/ARDUPILOT-IMAGE)" ]; then
  echo "ok   container/ARDUPILOT-IMAGE passes"
else
  echo "FAIL container/ARDUPILOT-IMAGE is refused"; fail=1
fi
exit "$fail"
