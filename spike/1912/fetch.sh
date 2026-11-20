#!/usr/bin/env bash
# AET-WO-1912: fetch every tools-shard job's log and timing of the given CI runs into OUT/<run>/.
# Usage: fetch.sh OUT RUN... (read-only: gh api / gh run view)
set -euo pipefail
out=$1; shift
for run in "$@"; do
  d=$out/$run; mkdir -p "$d"
  gh api "repos/ClearForge-LLC/aethelgard/actions/runs/$run" -q '[.head_sha,.conclusion,.run_attempt] | @tsv' >"$d/run.tsv"
  gh api "repos/ClearForge-LLC/aethelgard/actions/runs/$run/jobs" --paginate \
    -q '.jobs[] | [.id,.name,.conclusion,.started_at,.completed_at] | @tsv' >"$d/jobs.tsv"
  while IFS=$'\t' read -r id name _ _ _; do
    case $name in 'tools-shard ('*')') k=${name#tools-shard (}; k=${k%)} ;; *) continue ;; esac
    [ -s "$d/shard-$k.log" ] || gh run view --job "$id" --log >"$d/shard-$k.log"
  done <"$d/jobs.tsv"
  echo "$run: $(cut -f1,2 "$d/run.tsv") $(ls "$d"/shard-*.log | wc -l) shard logs"
done
