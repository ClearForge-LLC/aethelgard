#!/bin/bash
# AET-WO-1918 (1915's tool, its paths renamed): every run's attribution into spike/1918/out/: one JSON per run
# (attribute.py's summary), runs.tsv (one row per run) and, for each verify and
# each candidate and every_valid_sequence_spi1 run, its 1 s timeline (CBMC and kani-driver side by side), and each run's meta (command, image, cap, exit). The raw samples stay in ~/.cache/aet-1918/runs.
set -u
H=$(dirname "$(readlink -f "$0")"); O=$H/../out; R=$HOME/.cache/aet-1918/runs
mkdir -p "$O/runs" "$O/timelines"
printf 'run\trc\twall_s\tinner_memory_peak_gib\tsampled_memory_peak_gib\tanon_max_gib\tfile_max_gib\tshmem_max_gib\toom_kills\ttop_at_anon_peak\tcbmc_beside_build_samples\tsamples_two_procs_over_1gib\n' > "$O/runs.tsv"
for d in "$R"/*/; do
  t=$(basename "$d")
  case "$t" in selftest-*|build-*|extra-s02-generate) continue;; esac
  [ -s "$d/samples.jsonl" ] || continue
  python3 -I "$H/attribute.py" "$d/samples.jsonl" > "$O/runs/$t.json"
  cp "$d/meta" "$O/runs/$t.meta"  # the exact command, image, cap and exit
  python3 -I "$H/attribute.py" "$d/samples.jsonl" --table >> "$O/runs.tsv"
  case "$t" in verify-*|cand-*|*every_valid_sequence_spi1*) python3 -I "$H/attribute.py" "$d/samples.jsonl" --timeline | sed -n '/^ *[0-9.]* anon=/p' > "$O/timelines/$t.txt";; esac
done
echo "summarize: $(($(wc -l < "$O/runs.tsv") - 1)) runs"
