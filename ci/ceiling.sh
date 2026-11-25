#!/bin/bash
# AET-WO-1918 (1): judge a job's Kani peaks against the ceiling in the repo.
#   bash ci/ceiling.sh PROBE_LOG CEILING_FILE [--require-peak]
# Prints every harness's measured peak (ci/probe.py's `PROBE peak` lines), largest first, and
# exits 1 if any is above the ceiling (the one number in CEILING_FILE, KiB), if the probe left
# no log (it did not run, so nothing was measured), or, with --require-peak, if the log holds
# no peak at all (a job that proves harnesses must have measured one). Exit 0 otherwise.
set -u
die() { echo "ceiling: $*" >&2; exit 1; }
log=$1 file=$2 require=${3:-}
[ -r "$file" ] || die "cannot read the ceiling file $file"
ceiling=$(grep -v '^#' "$file" | tr -d '[:space:]')
case $ceiling in '' | *[!0-9]*) die "the ceiling file $file holds no number" ;; esac
[ -r "$log" ] || die "the probe left no log at $log: nothing was measured"
peaks=$(sed -n 's/^PROBE peak harness=\([^ ]*\) vmhwm_kib=\([0-9]*\) seconds=.*/\2\t\1/p' "$log" | sort -rn)
if [ -z "$peaks" ]; then
  [ "$require" = --require-peak ] && die "the probe measured no Kani harness, and this job proves some"
  echo "ceiling: no Kani harness ran (ceiling $ceiling KiB)"
  exit 0
fi
awk -F'\t' -v c="$ceiling" '{ printf "kani-driver peak %.3f GiB (%d KiB) %s%s\n", $1 / 1048576, $1, $2, ($1 > c ? "  ABOVE THE CEILING" : "") }' <<<"$peaks"
over=$(awk -F'\t' -v c="$ceiling" '$1 > c { print $2 }' <<<"$peaks" | sort -u | tr '\n' ' ')
[ -z "$over" ] || die "above the ceiling of $ceiling KiB ($file): $over"
echo "ceiling: every harness at or under $ceiling KiB"
