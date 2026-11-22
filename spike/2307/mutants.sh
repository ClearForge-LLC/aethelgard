#!/bin/bash
# AET-WO-2307 §1.3: the three named mutants of `ledger::missing_entries`, and one for F1 as ruled
# (a completed entry keeps its held reason), one at a time, against
# ledger_tier1, fab_tier1 and battery_sag_tier1. The source is restored from a copy after each.
# Usage: spike/2307/mutants.sh   -> spike/2307/out/mutants.txt
set -u
cd "$(dirname "$0")/../.."
F=crates/aethelgard-cli/src/ledger.rs
K=$HOME/.cache/aet-2307/ledger.rs.orig
cp "$F" "$K"
mutate() { python3 -I -c '
import sys
p, old, new = sys.argv[1:4]
s = open(p).read(); assert s.count(old) == 1, old; open(p, "w").write(s.replace(old, new))' "$F" "$1" "$2"; }
run() { # run <name> <old> <new>
  mutate "$2" "$3"
  local log=$HOME/.cache/aet-2307/mut-$1.log
  nice -n 10 cargo test -p aethelgard-cli --test ledger_tier1 --test fab_tier1 --test battery_sag_tier1 --no-fail-fast > "$log" 2>&1
  local failed=$(grep -E '^test .* FAILED$' "$log" | sed 's/^test //; s/ \.\.\. FAILED$//' | tr '\n' ' ')
  printf '%s\t%s\t%s\n' "$1" "$([ -n "$failed" ] && echo killed || echo SURVIVED)" "$failed"
  cp "$K" "$F"
}
{
run drop-a-field 'fields: fields.into_iter().map(str::to_owned).collect(),' 'fields: fields.into_iter().skip(1).map(str::to_owned).collect(),'
run merge-two-classes '.entry((&r.register, class))' '.entry((&r.register, class.min(UnwitnessedClass::SameSource)))'
run copy-judge 'reason: String::new(),' 'reason: ledger.rows.iter().find(|r| r.register == register).map(|r| r.judge.clone()).unwrap_or_default(),'
run keep-held-reason 'reason: String::new(),' 'reason: held.map(|a| a.reason.clone()).unwrap_or_default(),'
} | tee spike/2307/out/mutants.txt
cmp "$F" "$K" && rm -f "$K"
# Leave no mutant built: the next use of target/debug/aethelgard must be the restored source.
nice -n 10 cargo build -q -p aethelgard-cli
