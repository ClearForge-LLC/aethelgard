#!/bin/bash
# AET-WO-1919 (1), inside the image at /repo: bench-package-s03's verify step exactly as ci.yml writes
# it (VERIFY_S03, read from the workflow so it cannot drift) with --against-fab, its verdict line to
# /repo/target/s03-<tag>.json. Run under ctl.sh, whose probe times every gate and harness.
# usage: bash verify-s03.sh <tag>
set -u
cmd=$(python3 -I -c '
import re
t = open(".github/workflows/ci.yml").read()
m = re.search(r"VERIFY_S03: >-\n((?:        .*\n)+)", t)
print(" ".join(l.strip() for l in m.group(1).splitlines()))')
echo "VERIFY $cmd --against-fab fab/sentinel-03/package" >&2
$cmd --against-fab fab/sentinel-03/package > "/repo/target/s03-$1.json"
rc=$?
echo "RESULT s03 against-fab rc=$rc $(python3 -I -c '
import json,sys
v=json.load(open(sys.argv[1]))
g=[x for x in v.get("gates",[]) if x.get("gate")=="prove"]
print("verdict", v.get("verdict"), "prove_gates", len(g), "pass", sum(1 for x in g if x.get("verdict")=="pass"))' "/repo/target/s03-$1.json" 2>&1)"
exit $rc
