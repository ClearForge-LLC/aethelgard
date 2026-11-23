#!/usr/bin/env bash
# container/check-ardupilot-pin.sh FILE: the ArduPilot image pin (RUL-15;
# AET-WO-4401 F5, AET-WO-4402). FILE must name exactly one
# ghcr.io/clearforge-llc/aethelgard-ardupilot digest. On success the pin goes
# to stdout; otherwise the refusal goes to stderr, by name, with exit 1
# (INV-06). The `pinned` job reads container/ARDUPILOT-IMAGE through it, and
# test-check-ardupilot-pin.sh holds the fixtures.
#
# - Trailing newlines are stripped, as the `$(cat FILE)` this replaces did.
# - LC_ALL=C: under en_US.UTF-8, [0-9a-f] also matches a fullwidth or
#   Arabic-Indic digit and an accented letter (measured, AET-WO-4402).
# - A NUL anywhere is refused: a command substitution drops NULs, so a pin
#   split by one would otherwise read as whole.
# - Only a regular file (or a symlink to one) is read.
# - A refusal quotes at most the first 200 bytes, escaped by printf %q, so a
#   newline, a CR or a `::warning::` in the file never starts a log line of
#   its own, and a long file does not flood the log (AET-WO-4407; 4402 Q4).
set -euo pipefail
export LC_ALL=C
f=${1:?usage: check-ardupilot-pin.sh FILE}
{ [ -f "$f" ] && [ -r "$f" ]; } || { echo "$f missing or unreadable" >&2; exit 1; }
pin=$(tr -d '\000' < "$f")
nul=$(tr -cd '\000' < "$f" | wc -c)
{ [ "$nul" -eq 0 ] && [[ "$pin" =~ ^ghcr\.io/clearforge-llc/aethelgard-ardupilot@sha256:[0-9a-f]{64}$ ]]; } \
  || { more=; [ "${#pin}" -le 200 ] || more=" (the first 200 of ${#pin} bytes)"
       printf '%s is not ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:<64 hex>: %q%s\n' "$f" "${pin:0:200}" "$more" >&2; exit 1; }
printf '%s\n' "$pin"
