#!/bin/bash
# AET-WO-4400j (1): every table and fact RESULTS.md cites, regenerated from the records in ~/.cache/aet-4400j into
# spike/4400j/out/analysis (and out/verdicts: every CLI run's verdict, gzipped). One command, so a re-measure after
# 4411 lands is one run of the series and this. Host python3 only; no container.
set -eu
S=/home/forge/aethelgard-4400j/spike/4400j; C=$HOME/.cache/aet-4400j; A=$S/out/analysis; P="python3 -I $S/py"
mkdir -p "$A" "$S/out/verdicts"
for d in "$C"/cli/*/; do n=$(basename "$d"); [ -f "$d/verdict.json" ] && gzip -9c "$d/verdict.json" > "$S/out/verdicts/$n.json.gz"; done
$P/table.py "$C/cli" "$S/out/cli-null" L smoke > "$S/out/table-null.md"
$P/table.py "$C/cli" "$S/out/cli-held" H > "$S/out/table-held.md"
$P/table.py "$C/cli" "$S/out/cli-q" Q R > "$S/out/table-q.md"
$P/table.py "$C/cli" "$S/out/cli-main0" M0 > "$S/out/table-main0.md"
$P/table.py "$C/cli" "$S/out/cli-main1" M1 > "$S/out/table-main1.md"
$P/flood.py "$C/cli/L-s03-12000-1/log.txt.gz" 12 > "$A/flood-12.txt"
$P/flood.py "$C/cli/L-s03-12000-in-1/log.txt.gz" 16 > "$A/flood-12-in.txt"
$P/flood.py "$C/rep/stats/console.txt.gz" 4 --raw > "$A/flood-raw-12.txt"
$P/cycle.py "$C/cli/L-s03-12000-1/log.txt.gz" > "$A/cycle-12.txt"
$P/cycle.py "$C/cli/L-s03-12000-in-1/log.txt.gz" > "$A/cycle-12-in.txt"
$P/distinct.py "$C/cli/L-s03-12000-in-1/log.txt.gz" 5 > "$A/distinct-12-in.txt"
$P/residual.py "$C/cli/L-s03-12000-1/log.txt.gz" 4.65 > "$A/residual-12.txt"
$P/residual.py "$C/cli/L-s03-12000-in-1/log.txt.gz" 4.65 > "$A/residual-12-in.txt"
for r in reads reads-in; do
  OLD=1 $P/endreads.py "$C/cli/L-s03-12000${r#reads}-1/verdict.json" "$C/rep/$r/reads.txt" "$C/rep/$r/console.txt.gz" \
    5000 8000 10000 12000 > "$A/endreads-$r.txt"
  $P/facts.py vect "$C/rep/$r/console.txt.gz" "$C/rep/$r/reads.txt" > "$A/vect-$r.txt"
done
$P/facts.py cr1 "$C/rep/reads/console.txt.gz" "$C/rep/reads/reads.txt" > "$A/cr1-sbus.txt"
$P/facts.py cr2 "$C/cli/L-s03-12000-1/log.txt.gz" > "$A/cr2-edges.txt"
$P/facts.py pb13 "$C/rep/gpiob/console.txt.gz" > "$A/pb13.txt"
$P/facts.py arm "$C/rep/stats-in/console.txt.gz" > "$A/arm.txt"
$P/facts.py quantum "$C/rep/quantum/console.txt.gz" > "$A/quantum-11-12s-in.txt"
$P/facts.py ciwall "$C"/ci/*.log > "$A/ci-wall.txt"
for r in stats stats-in fullgc stats-holdin; do $P/wallrate.py "$C/rep/$r/rss.txt" 5 > "$S/out/rep/$r/wallrate.txt"; done
: > "$A/same.txt"
for w in 5000 8000 10000 12000; do for i in "" -in; do
  h=H1; [ "$w$i" = 12000 ] && h=H
  { echo "== L-s03-$w$i-1 vs $h-s03-$w$i-1"; $P/same.py "$C/cli/L-s03-$w$i-1/verdict.json" "$C/cli/$h-s03-$w$i-1/verdict.json"; } >> "$A/same.txt"
done; done
ls "$A"
