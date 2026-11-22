#!/bin/bash
# AET-WO-2307 §1.1: the acknowledgement refusal measured on Sentinel-03 and on 4600's board M.
# Usage: spike/2307/measure.sh <tag> <aethelgard binary> <probe2307 binary>
#   tag: `before` (the base's CLI) or `after` (this branch's CLI). Writes spike/2307/out/<tag>/.
# No container: the refusal precedes every gate. Scratch (M stripped, --fab-out dirs) under
# ~/.cache/aet-2307/measure, removed at the end.
set -eu
cd "$(dirname "$0")/../.."
TAG=$1 A=$(realpath "$2") P=$(realpath "$3")
O=spike/2307/out/$TAG
S=$HOME/.cache/aet-2307/measure/$TAG
rm -rf "$O" "$S"; mkdir -p "$O" "$S"
run() { # run <name> <spec> <scenario dir> [cwd]: the refusal's JSON and exit, wall time in ms
  local t0=$(date +%s%N) rc=0 out=$PWD/$O/$1.verify-fab.json
  (cd "${4:-.}" && "$A" verify --spec "$2" --scenarios "$3"/*.json --fab-out "$S/fab-$1") > "$out" || rc=$?
  echo "$1 exit=$rc wall_ms=$(( ($(date +%s%N) - t0) / 1000000 )) package=$([ -e "$S/fab-$1" ] && echo written || echo none)" >> "$O/runs.txt"
}
# Sentinel-03 as committed (30 entries).
SEN=crates/aethelgard-cli/tests/fixtures/sentinel-03.json
run sentinel-03 "$SEN" scenarios/sentinel-03
"$P" "$SEN" > "$O/sentinel-03.probe.json"
# M: driver-judges-nothing.json with every acknowledgement removed, beside 4600's driver scenarios.
M=crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json
mkdir -p "$S/m/scen"
python3 -I - "$M" "$S/m" <<'PY'
import json, sys
spec = json.load(open(sys.argv[1]))
acks = spec["acknowledge_unwitnessed"]
spec["acknowledge_unwitnessed"] = []
json.dump(spec, open(sys.argv[2] + "/spec.json", "w"), indent=2)
# 4600's three driver scenarios (spike/4600/scenarios/driver), each re-based on the stripped spec.
import glob, os
for f in sorted(glob.glob("spike/4600/scenarios/driver/*.json")):
    sc = json.load(open(f)); sc["base"] = "../spec.json"
    json.dump(sc, open(sys.argv[2] + "/scen/" + os.path.basename(f), "w"))
# The committed entries, reason emptied, fields sorted, in (register, class) order.
want = sorted(({"register": a["register"], "class": a["class"], "fields": sorted(a["fields"]), "reason": ""}
               for a in acks), key=lambda e: (e["register"], e["class"]))
json.dump(want, open(sys.argv[2] + "/../m-committed-emptied.json", "w"))
PY
run m-stripped spec.json scen "$S/m"
"$P" "$S/m/spec.json" > "$O/m-stripped.probe.json"
cp "$S/m-committed-emptied.json" "$O/m-committed-emptied.json"
# Compare: the probe's entries against the committed ones emptied, and (after) the refusal's.
python3 -I - "$O" <<'PY' | tee "$O/compare.txt"
import json, sys
o = sys.argv[1]
def load(n): return json.load(open(f"{o}/{n}"))
for name in ("sentinel-03", "m-stripped"):
    v, p = load(f"{name}.verify-fab.json"), load(f"{name}.probe.json")
    e = v["error"]
    msg = e["message"]
    print(f"{name}: kind={e['kind']} id={e['id']} message_rows={msg.split(' unwitnessed')[0].removeprefix('no package: ')}")
    print(f"  probe: rows={p['rows']} first={p['first']} registers={p['registers']} pairs={p['pairs']} completed={p['completed']} new={p['new']} missing_fields={p['missing_fields']} fields_in_entries={sum(len(x['fields']) for x in p['entries'])}")
    if "entries" in e:
        print(f"  refusal entries == probe entries: {e['entries'] == p['entries']} ({len(e['entries'])} entries)")
    else:
        print("  refusal has no `entries` (this CLI predates 2307)")
m = load("m-stripped.probe.json")["entries"]
c = load("m-committed-emptied.json")
print(f"M committed (reason emptied) == probe entries: {c == m} (committed {len(c)} entries / {sum(len(x['fields']) for x in c)} fields; probe {len(m)} / {sum(len(x['fields']) for x in m)})")
PY
rm -rf "$S"
