#!/usr/bin/env bash
# AET-WO-1919: one attempt of a CI run, read-only (gh api / gh run view): the run's head, every job
# with its runner and steps (jobs.json), and the logs of bench-package-s03, bench-package and every
# tools-shard, each in OUT/<run>-a<attempt>/. A log already fetched is kept.
# Usage: fetch.sh OUT RUN ATTEMPT
set -euo pipefail
out=$1 run=$2 n=$3
repo=ClearForge-LLC/aethelgard
d=$out/$run-a$n; mkdir -p "$d"
gh api "repos/$repo/actions/runs/$run/attempts/$n" -q '[.head_sha,.conclusion,.run_attempt,.event,.head_branch] | @tsv' >"$d/run.tsv"
gh api "repos/$repo/actions/runs/$run/attempts/$n/jobs?per_page=100" --paginate -q '.jobs[]' >"$d/jobs.json"
python3 -I -c '
import json,sys
for line in open(sys.argv[1]):
    j=json.loads(line)
    print(j["id"], j["name"], j["conclusion"], sep="\t")' "$d/jobs.json" >"$d/jobs.tsv"
while IFS=$'\t' read -r id name concl; do
  case $name in bench-package-s03) f=s03 ;; bench-package) f=s01 ;; 'tools-shard ('*')') k=${name#tools-shard (}; f=shard-${k%)} ;; *) continue ;; esac
  [ -s "$d/$f.log" ] && continue
  [ "$concl" = "None" ] || [ -z "$concl" ] && continue
  gh api "repos/$repo/actions/jobs/$id/logs" >"$d/$f.log" 2>/dev/null || rm -f "$d/$f.log"
done <"$d/jobs.tsv"
echo "$run a$n: $(cut -f1-4 "$d/run.tsv") $(ls "$d"/*.log 2>/dev/null | wc -l) logs"
