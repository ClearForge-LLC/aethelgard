#!/bin/bash
# AET-WO-2307 §5, targeted adversarial pass. Each probe pastes Sentinel-03's `error.entries` (from
# this branch's refusal) into its specification in some way and runs `validate`, which judges the
# shape (IR) and staleness (the CLI's ledger). No gate runs.
# Usage: spike/2307/adversarial.sh <branch aethelgard>   -> spike/2307/out/adversarial.txt
set -u
cd "$(dirname "$0")/../.."
A=$(realpath "$1")
S=$HOME/.cache/aet-2307/adv; rm -rf "$S"; mkdir -p "$S/w/scen"
SEN=crates/aethelgard-cli/tests/fixtures/sentinel-03.json
cp "$SEN" "$S/w/spec.json"; cp scenarios/sentinel-03/*.json "$S/w/scen/"
sed -i 's|"base": "../../crates/aethelgard-cli/tests/fixtures/sentinel-03.json"|"base": "../spec.json"|' "$S/w/scen/"*.json
(cd "$S/w" && "$A" verify --spec spec.json --scenarios scen/*.json --fab-out ../fab > "$S/refusal.json")
python3 -I - "$SEN" "$S/refusal.json" "$S" <<'PY'
import json, sys, copy
spec, refusal, out = json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), sys.argv[3]
entries = refusal["error"]["entries"]
key = lambda a: (a["register"], a["class"])
def paste(reason_of, beside=False, drop_reason=False):
    s = copy.deepcopy(spec)
    got = {key(e): copy.deepcopy(e) for e in entries}
    held = [a for a in s["acknowledge_unwitnessed"] if beside or key(a) not in got]
    for i, e in enumerate(entries):
        e = copy.deepcopy(e)
        if e["reason"] == "":
            e["reason"] = reason_of(i, e)
        if drop_reason:
            e.pop("reason")
        held.append(e)
    s["acknowledge_unwitnessed"] = held
    return s
# The judge text of each entry's first field's row, from the reference ledger the work order names:
# no row is printed by the refusal, so a person copying it reads verify's JSON ledger. Here any
# printable string of the ledger's form stands in: the probe asks only whether validate can tell.
probes = {
    "as-printed (reason \"\")": paste(lambda i, e: ""),
    "reason one space": paste(lambda i, e: " "),
    "reason a tab": paste(lambda i, e: "\t"),
    "reason padded": paste(lambda i, e: " written "),
    "reason key absent": paste(lambda i, e: "", drop_reason=True),
    "reason 201 bytes": paste(lambda i, e: "x" * 201),
    "reason non-ASCII": paste(lambda i, e: "§ reason"),
    "completed pasted beside the held entry": paste(lambda i, e: f"reason {i}", beside=True),
    "every reason written, each its own": paste(lambda i, e: f"a person's reason {i}"),
    "every reason the same string": paste(lambda i, e: "x"),
}
for name, s in probes.items():
    json.dump(s, open(f"{out}/{name.replace(' ', '_').replace('\"', '')}.json", "w"))
PY
for f in "$S"/*.json; do
  n=$(basename "$f" .json); [ "$n" = refusal ] && continue
  r=$("$A" validate --file "$f"); rc=$?
  printf '%s\texit %s\t%s\n' "$n" "$rc" "$(python3 -I -c 'import json,sys; v=json.loads(sys.argv[1]); e=v.get("error") or {}; print(e.get("kind","ok"), e.get("path") or "", (e.get("message") or "")[:120])' "$r")"
done | sort | tee spike/2307/out/adversarial.txt
# The refusal's size on Sentinel-03, and that it prints no reason but "" (F1 as ruled).
python3 -I - "$SEN" "$S/refusal.json" <<'PY' | tee -a spike/2307/out/adversarial.txt
import json, sys
spec, v = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
held = {(a["register"], a["class"]) for a in spec["acknowledge_unwitnessed"]}
e = v["error"]["entries"]
print(f"refusal bytes {len(open(sys.argv[2]).read())}; entries {len(e)}, {sum(1 for a in e if (a['register'], a['class']) in held)} completing a held entry; non-empty reasons {sum(1 for a in e if a['reason'])} (want 0)")
PY
rm -rf "$S"
