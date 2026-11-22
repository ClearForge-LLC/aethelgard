#!/usr/bin/env bash
# AET-WO-1215 §1.3 mutants: one mutant per guard this work order adds, each run against the suite it lives in.
# Each mutant is a literal (old, new) replacement in one file, applied, tested and reverted by copying the
# saved original back (no git checkout: other builders share nothing here, but the index stays untouched).
# A mutant is KILLED when its suite fails. Usage: bash spike/1215/mutants.sh REPO > spike/1215/out/mutants.txt
set -u
repo=$1
cd "$repo"
export PATH=$HOME/.cargo/bin:$PATH
V=crates/aethelgard-ir/src/versions.rs
M=crates/aethelgard-cli/src/main.rs
ir() { nice -n 10 cargo test -q -p aethelgard-ir --test versions >/dev/null 2>&1; }
cli() { nice -n 10 cargo test -q -p aethelgard-cli --test stackups_tier1 an_older >/dev/null 2>&1; }
mutant() { # name file suite old new
  local name=$1 file=$2 suite=$3 old=$4 new=$5 save
  save=$(mktemp -p "$HOME/.cache/aet-1215")
  cp -p "$file" "$save"
  python3 -I -c 'import sys; p,o,n=sys.argv[1:4]; s=open(p).read(); assert s.count(o)==1, (p, o); open(p,"w").write(s.replace(o,n))' "$file" "$old" "$new" || { echo "$name	NOT APPLIED"; rm -f "$save"; return; }
  if $suite; then echo "$name	SURVIVED"; else echo "$name	killed"; fi
  cp -p "$save" "$file"; rm -f "$save"; touch "$file"
}
rows() { # name op a [b]: drop row a, or swap rows a and b, in versions.rs's table
  local name=$1 op=$2 a=$3 b=${4:-0} save
  save=$(mktemp -p "$HOME/.cache/aet-1215")
  cp -p "$V" "$save"
  python3 -I - "$V" "$op" "$a" "$b" <<'PY'
import re, sys
p, op, a, b = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
s = open(p).read()
def block(n):
    m = re.search(r"\n    row\(\s*%d,.*?\),\n" % n, s, re.S)
    assert m, n
    return m.group(0)[1:]
if op == "drop":
    s = s.replace(block(a), "", 1)
else:
    x, y = block(a), block(b)
    s = s.replace(x, "\0", 1).replace(y, x, 1).replace("\0", y, 1)
open(p, "w").write(s)
PY
  if ir; then echo "$name	SURVIVED"; else echo "$name	killed"; fi
  cp -p "$save" "$V"; rm -f "$save"; touch "$V"
}
rows "M1 drop row 22" drop 22
rows "M2 swap rows 21 and 22" swap 21 22
rows "M2b swap rows 2 and 3" swap 2 3
mutant "M3 version 0 lists every row" $V ir 'if version == 0 {' 'if version == u32::MAX {'
mutant "M4 an older file lists its own row too" $V ir 'partition_point(|c| c.version <= version)' 'partition_point(|c| c.version < version)'
mutant "M5 the message lists no row" $V ir '        if self.0.is_empty() {
            return Ok(());
        }' '        if true {
            return Ok(());
        }'
mutant "M6 a newer file carries entries: []" $M cli '(!aethelgard_ir::versions::lacked_by(version).is_empty()).then_some(Lacking(version))' 'Some(Lacking(version))'
mutant "M7 the CLI omits entries" $M cli '(!aethelgard_ir::versions::lacked_by(version).is_empty()).then_some(Lacking(version))' 'None::<Lacking>'
mutant "M8 row 18's esc made optional" $V ir '        &["power.battery.series_milliohms", "motor_outputs[].esc"],
        &[],' '        &["power.battery.series_milliohms"],
        &["motor_outputs[].esc"],'
mutant "M9 row 20 loses its replacement" $V ir '        &[Replacement {
            path: "mcu.hse_hz",
            by: "mcu.hse",
        }],' '        &[],'
mutant "M10 a rule the prose does not name" $V ir '&["UnboundedPinCapacitance", "PartNotDrawable"]' '&["UnboundedPinCapacitance", "PartNotDrawable", "OutlineBound"]'
mutant "M11 a path the schema lacks" $V ir '&["mcu.foreign_clock"]' '&["mcu.foreign_clocks"]'
