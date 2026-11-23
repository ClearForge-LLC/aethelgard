#!/usr/bin/env bash
# AET-WO-1914 §1.1: across held runs' times (held.sh times output, one file per run), every test with no row in WEIGHTS
# and in no binary of COLOCATE whose worst time is LO s or more: `binary<TAB>test<TAB>worst<TAB>runs-at-or-over-10<TAB>tier`.
#   band.sh WEIGHTS COLOCATE LO TIMES...
set -euo pipefail
export LC_ALL=C
w=$1 c=$2 lo=$3; shift 3
re=$(sed -n "s/^TIER2_RE='\(.*\)'$/\1/p" "$(dirname "$0")/../../ci/shard.sh")
awk -F'\t' -v wf="$w" -v cf="$c" -v lo="$lo" -v re="$re" '
  FILENAME == wf { if ($0 !~ /^#/ && NF == 3) wt[$1 "\t" $2] = 1; next }
  FILENAME == cf { if ($0 !~ /^#/ && NF == 2) blk[$1] = 1; next }
  { k = $2 "\t" $3; if ((k in wt) || ($2 in blk)) next
    if (!(k in worst) || $4 + 0 > worst[k]) worst[k] = $4 + 0
    if ($4 + 0 >= 10) over[k]++ }
  END { for (k in worst) if (worst[k] >= lo) { split(k, f, "\t")
          printf "%s\t%.3f\t%d\t%s\n", k, worst[k], over[k], (f[1] ~ re ? "tier2" : "tier1") } }
' "$w" "$c" "$@" | sort -t "$(printf '\t')" -k3,3gr
