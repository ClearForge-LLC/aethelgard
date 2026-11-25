#!/usr/bin/env bash
# AET-WO-1918 (3): spike/1912/fetch.sh's layout for one attempt of a run (fetch.sh reads only the
# latest): OUT/<run>-a<attempt>/{run.tsv,jobs.tsv,shard-K.log}, the logs by `gh run view --job ID
# --log` (job and step columns first), so spike/1916/stamp.sh and measure.py read them unchanged.
# Usage: fetch-attempt.sh OUT RUN ATTEMPT   (read-only: gh api / gh run view)
set -euo pipefail
out=$1 run=$2 n=$3
d=$out/$run-a$n; mkdir -p "$d"
gh api "repos/ClearForge-LLC/aethelgard/actions/runs/$run/attempts/$n" -q '[.head_sha,.conclusion,.run_attempt] | @tsv' >"$d/run.tsv"
gh api "repos/ClearForge-LLC/aethelgard/actions/runs/$run/attempts/$n/jobs" --paginate \
  -q '.jobs[] | [.id,.name,.conclusion,.started_at,.completed_at] | @tsv' >"$d/jobs.tsv"
while IFS=$'\t' read -r id name _ _ _; do
  case $name in 'tools-shard ('*')') k=${name#tools-shard (}; k=${k%)} ;; *) continue ;; esac
  [ -s "$d/shard-$k.log" ] && continue
  # While a later attempt runs, `gh run view` serves no log: the API's raw log, with the two
  # columns `--log` puts first (job, step), which stamp.sh cuts off.
  gh run view --job "$id" --repo ClearForge-LLC/aethelgard --log >"$d/shard-$k.log" 2>/dev/null \
    || gh api "repos/ClearForge-LLC/aethelgard/actions/jobs/$id/logs" | sed "s/^/$name\t-\t/" >"$d/shard-$k.log"
done <"$d/jobs.tsv"
echo "$run attempt $n: $(cut -f1,2 "$d/run.tsv") $(ls "$d"/shard-*.log | wc -l) shard logs"
