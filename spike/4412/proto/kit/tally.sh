#!/bin/bash
# tally.sh REF RUN...: compare.py each RUN against REF (consoles under proto/rep), one summary line each, full output kept
cd $HOME/.cache/aet-4412/proto
ref=$1; shift
for r in "$@"; do
  out=cmp-r3-$ref--$r.txt
  python3 -I compare.py rep/$ref/console.txt.gz rep/$r/console.txt.gz > $out 2>&1
  d=$(grep -E "same_with_stamps=False|differing lines: [1-9]" $out | grep -v "same_without_stamps=True same_with_stamps=False" | cut -c1-60 | tr '\n' ';')
  s=$(grep -c "same_without_stamps=True same_with_stamps=False" $out)
  f=$(grep -m1 "first difference" $out | cut -c1-150)
  echo "$r: ${d:-IDENTICAL (values)}; stamp-only streams=$s; $f"
done
