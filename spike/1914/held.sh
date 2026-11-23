#!/usr/bin/env bash
# AET-WO-1914 §1.1: a held CI run's shard logs, as `run` now stamps them, through ci/shard.sh's own P3.
#   held.sh stamp JOBLOG      the `shard.sh run` step of a GitHub job log (`gh api .../jobs/ID/logs`), each line
#                             `seconds.micro<TAB>line`. A log `run` stamped itself (AET-WO-1914 on) keeps its own stamps
#                             and only its stamped lines; an older one takes GitHub's per-line UTC time stamp.
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
case ${1:-} in
  stamp)
    t=$(mktemp); trap 'rm -f "$t"' EXIT
    awk '
      # days from 1970-01-01 to a civil date (proleptic Gregorian)
      function days(y, m, d,   era, yoe, doy, doe) {
        y -= (m <= 2); era = int(y / 400); yoe = y - era * 400
        doy = int((153 * (m + (m > 2 ? -3 : 9)) + 2) / 5) + d - 1
        doe = yoe * 365 + int(yoe / 4) - int(yoe / 100) + doy
        return era * 146097 + doe - 719468 }
      { sub(/^\357\273\277/, "") }
      /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9:.]+Z( |$)/ {
        iso = $1; rest = substr($0, length(iso) + 2)
        if (rest ~ /^##\[group\]Run .*ci\/shard\.sh run /) { on = 1; hdr = 1; next }
        if (on && rest ~ /^##\[group\]Run /) exit
        if (hdr) { if (rest ~ /^##\[endgroup\]/) hdr = 0; next }
        if (!on || rest ~ /^##\[/) next
        s = substr(iso, 18); sub(/Z$/, "", s)
        printf "%.6f\t%s\n", days(substr(iso, 1, 4) + 0, substr(iso, 6, 2) + 0, substr(iso, 9, 2) + 0) * 86400 \
          + substr(iso, 12, 2) * 3600 + substr(iso, 15, 2) * 60 + s, rest }
    ' "$2" >"$t"
    if awk -F'\t' '$2 ~ /^[0-9]+[.][0-9]+$/ { f = 1; exit } END { exit !f }' "$t"; then
      awk -F'\t' '$2 ~ /^[0-9]+[.][0-9]+$/' "$t" | cut -f2-
    else cat "$t"; fi ;;
  times)
    for f in "$2"/shard-*.log; do
      "$0" stamp "$f" >"${f%.log}.stamped"
      bash "$here/../../ci/shard.sh" times "${f%.log}.stamped" "${3:-4}" | sed "s|^|${f##*/}\t|"
    done ;;
  *) echo "usage: held.sh stamp JOBLOG | times DIR [T]" >&2; exit 2 ;;
esac
